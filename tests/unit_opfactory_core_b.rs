//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, second half of the `core` class:
//! the compound, vertical and bound CRS cases of `createOperation(s)`.
//!
//! Each function keeps the C++ `TEST()` name. The expected PROJ strings and names are
//! copied from the C++ file.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, AuthorityFactory, CompoundCRS,
    CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory, Crs,
    DatabaseContext, ProjLines, SpatialCriterion,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg_crs(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn parse(db: &DatabaseContext, text: &str) -> Crs {
    create_from_user_input(text, db).expect(text)
}

fn compound(name: &str, components: Vec<Crs>) -> Crs {
    Crs::Compound(CompoundCRS::create(name, components).expect("compound"))
}

fn proj_string(operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.export_to_proj_string(),
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
        }
    }
    .expect("PROJ string")
}

/// `createOperations` with an EPSG-or-any-authority context, partial intersection.
fn list(
    db: &DatabaseContext,
    authority: &str,
    source: &Crs,
    target: &Crs,
) -> Vec<CoordinateOperation> {
    let mut context = CoordinateOperationContext::create(db, authority);
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    CoordinateOperationFactory::create()
        .create_operations(source, target, &context)
        .expect("operations")
}

/// `createOperation(source, target)`: the first operation of the default context.
fn first(db: &DatabaseContext, source: &Crs, target: &Crs) -> CoordinateOperation {
    let context = CoordinateOperationContext::create(db, "");
    CoordinateOperationFactory::create()
        .create_operations(source, target, &context)
        .expect("operations")
        .into_iter()
        .next()
        .expect("an operation")
}

const ODN_HEIGHT: &str = r#"VERTCRS["ODN height",VDATUM["Ordnance Datum Newlyn",ID["EPSG",5101]],CS[vertical,1],AXIS["gravity-related height (H)",up,LENGTHUNIT["metre",1]],ID["EPSG",5701]]"#;

fn bound_vertical_wkt() -> String {
    format!(
        r#"BOUNDCRS[SOURCECRS[{ODN_HEIGHT}],TARGETCRS[GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,3],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]],AXIS["ellipsoidal height (h)",up,ORDER[3],LENGTHUNIT["metre",1]],ID["EPSG",4979]]],ABRIDGEDTRANSFORMATION["ODN height to WGS 84",METHOD["GravityRelatedHeight to Geographic3D"],PARAMETERFILE["Geoid (height correction) model file","us_nga_egm08_25.tif"]]]"#
    )
}

#[test]
fn operation_compound_crs_with_bound_vertical_crs_to_geog_crs() {
    let db = database();
    let bound = parse(&db, &bound_vertical_wkt());
    let source = compound("", vec![epsg_crs(&db, "4326"), bound]);
    let op = first(&db, &source, &epsg_crs(&db, "4979"));
    assert_eq!(
        proj_string(&op),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=vgridshift \
         +grids=us_nga_egm08_25.tif +multiplier=1 +step +proj=unitconvert \
         +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

const GEOG_UNKNOWN_TOWGS84: &str = r#"GEOGCS["unknown",DATUM["unknown",SPHEROID["WGS 84",6378137,298.257223563],TOWGS84[1,2,3,4,5,6,7]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],AXIS["Latitude",NORTH],AXIS["Longitude",EAST]]"#;

const NTF_PARIS_TOWGS84: &str = r#"GEOGCS["NTF (Paris)",DATUM["Nouvelle_Triangulation_Francaise_Paris",SPHEROID["Clarke 1880 (IGN)",6378249.2,293.4660212936269],TOWGS84[1,2,3,4,5,6,7]],PRIMEM["Paris",2.5969213],UNIT["grad",0.0157079632679489],AXIS["Latitude",NORTH],AXIS["Longitude",EAST],AUTHORITY["EPSG","4807"]]"#;

#[test]
fn operation_compound_crs_with_bound_geog_crs_to_geog_crs() {
    let db = database();
    let vertical = parse(&db, ODN_HEIGHT);
    let source = compound("", vec![parse(&db, GEOG_UNKNOWN_TOWGS84), vertical]);
    // `createOperation`, with no database context: no registered operation.
    let op = CoordinateOperationFactory::create()
        .create_operation(&source, &epsg_crs(&db, "4979"))
        .expect("operation")
        .expect("an operation");
    assert_eq!(
        proj_string(&op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_compound_crs_with_bound_geog_crs_and_bound_vertical_crs_to_geog_crs() {
    let db = database();
    let source = compound(
        "",
        vec![
            parse(&db, NTF_PARIS_TOWGS84),
            parse(&db, &bound_vertical_wkt()),
        ],
    );
    let op = first(&db, &source, &epsg_crs(&db, "4979"));
    assert_eq!(
        proj_string(&op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=grad +xy_out=rad \
         +step +inv +proj=longlat +ellps=clrk80ign +pm=paris \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=clrk80ign \
         +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=pop +v_3 \
         +step +proj=vgridshift +grids=us_nga_egm08_25.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_compound_crs_with_bound_vertical_crs_from_geoidgrids_with_m_to_geog_crs() {
    let db = database();
    let source = parse(
        &db,
        "+proj=longlat +datum=WGS84 +geoidgrids=@foo.gtx +type=crs",
    );
    let op = first(&db, &source, &epsg_crs(&db, "4979"));
    assert_eq!(
        proj_string(&op),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_compound_crs_with_bound_vertical_crs_from_geoidgrids_with_ftus_to_geog_crs() {
    let db = database();
    let source = parse(
        &db,
        "+proj=longlat +datum=WGS84 +geoidgrids=@foo.gtx +vunits=us-ft +type=crs",
    );
    let op = first(&db, &source, &epsg_crs(&db, "4979"));
    assert_eq!(
        proj_string(&op),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=rad +z_out=m \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_compound_crs_with_bound_vertical_crs_from_grids_to_geog_crs_with_ftus_ctxt() {
    let db = database();
    let source = parse(
        &db,
        r#"COMPD_CS["NAD83 + NAVD88 height - Geoid12B (Meters)",
    GEOGCS["NAD83",
        DATUM["North_American_Datum_1983",
            SPHEROID["GRS 1980",6378137,298.257222101,
                AUTHORITY["EPSG","7019"]],
            AUTHORITY["EPSG","6269"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4269"]],
    VERT_CS["NAVD88 height - Geoid12B (Meters)",
        VERT_DATUM["North American Vertical Datum 1988",2005,
            EXTENSION["PROJ4_GRIDS","@foo.gtx"],
            AUTHORITY["EPSG","5103"]],
        UNIT["metre",1.0,
            AUTHORITY["EPSG","9001"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","5703"]]]"#,
    );
    let target = parse(
        &db,
        r#"COMPD_CS["NAD83 + Ellipsoid (US Feet)",
    GEOGCS["NAD83",
        DATUM["North_American_Datum_1983",
            SPHEROID["GRS 1980",6378137,298.257222101,
                AUTHORITY["EPSG","7019"]],
            AUTHORITY["EPSG","6269"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4269"]],
    VERT_CS["Ellipsoid (US Feet)",
        VERT_DATUM["Ellipsoid",2002],
        UNIT["US survey foot",0.304800609601219,
            AUTHORITY["EPSG","9003"]],
        AXIS["Up",UP]]]"#,
    );
    assert!(matches!(target, Crs::Geographic(_)));
    let ops = list(&db, "EPSG", &source, &target);
    assert!(!ops.is_empty());
    assert_eq!(proj_string(&ops[0]), NAD83_NAVD88_TO_US_FT);
}

const NAD83_NAVD88_TO_US_FT: &str = "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=us-ft \
         +step +proj=axisswap +order=2,1";

/// Whether the NAD83 `GEOGCS` carries a zero `TOWGS84` binding.
enum Nad83Binding {
    Plain,
    Towgs84,
}

fn nad83_datum(binding: Nad83Binding) -> &'static str {
    match binding {
        Nad83Binding::Towgs84 => {
            r#"DATUM["North_American_Datum_1983",
            SPHEROID["GRS 1980",6378137,298.257222101,
                AUTHORITY["EPSG","7019"]],
            TOWGS84[0,0,0,0,0,0,0],
            AUTHORITY["EPSG","6269"]]"#
        }
        Nad83Binding::Plain => {
            r#"DATUM["North_American_Datum_1983",
            SPHEROID["GRS 1980",6378137,298.257222101,
                AUTHORITY["EPSG","7019"]],
            AUTHORITY["EPSG","6269"]]"#
        }
    }
}

fn nad83_navd88_metres(binding: Nad83Binding) -> String {
    let datum = nad83_datum(binding);
    format!(
        r#"COMPD_CS["NAD83 + NAVD88 height - Geoid12B (Meters)",
    GEOGCS["NAD83",
        {datum},
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4269"]],
    VERT_CS["NAVD88 height - Geoid12B (Meters)",
        VERT_DATUM["North American Vertical Datum 1988",2005,
            EXTENSION["PROJ4_GRIDS","@foo.gtx"],
            AUTHORITY["EPSG","5103"]],
        UNIT["metre",1.0,
            AUTHORITY["EPSG","9001"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","5703"]]]"#
    )
}

fn nad83_ellipsoid_us_feet(binding: Nad83Binding) -> String {
    let datum = nad83_datum(binding);
    format!(
        r#"COMPD_CS["NAD83 + Ellipsoid (US Feet)",
    GEOGCS["NAD83",
        {datum},
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4269"]],
    VERT_CS["Ellipsoid (US Feet)",
        VERT_DATUM["Ellipsoid",2002],
        UNIT["US survey foot",0.304800609601219,
            AUTHORITY["EPSG","9003"]],
        AXIS["Up",UP]]]"#
    )
}

#[test]
fn operation_compound_crs_with_bound_geog_crs_bound_vertical_crs_from_grids_to_bound_geog_crs_with_ftus_ctxt(
) {
    let db = database();
    let source = parse(&db, &nad83_navd88_metres(Nad83Binding::Towgs84));
    let target = parse(&db, &nad83_ellipsoid_us_feet(Nad83Binding::Towgs84));
    assert!(matches!(target, Crs::Bound(_)));
    let ops = list(&db, "EPSG", &source, &target);
    assert_eq!(ops.len(), 1);
    assert_eq!(proj_string(&ops[0]), NAD83_NAVD88_TO_US_FT);
}

#[test]
fn operation_compound_crs_with_bound_vertical_crs_from_grids_to_bound_geog_crs_with_ftus_ctxt() {
    let db = database();
    let source = parse(&db, &nad83_navd88_metres(Nad83Binding::Plain));
    let target = parse(&db, &nad83_ellipsoid_us_feet(Nad83Binding::Towgs84));
    assert!(matches!(target, Crs::Bound(_)));
    let ops = list(&db, "EPSG", &source, &target);
    assert!(!ops.is_empty());
    assert_eq!(proj_string(&ops[0]), NAD83_NAVD88_TO_US_FT);
}

#[test]
fn operation_vert_crs_to_vert_crs() {
    let db = database();
    let metre = create_crs_from_proj_string("+vunits=m").expect("+vunits=m");
    let foot = create_crs_from_proj_string("+vunits=ft").expect("+vunits=ft");
    let us_foot = create_crs_from_proj_string("+vunits=us-ft").expect("+vunits=us-ft");

    let metres_to_feet = first(&db, &metre, &foot);
    assert_eq!(
        proj_string(&metres_to_feet),
        "+proj=unitconvert +z_in=m +z_out=ft"
    );
    assert_eq!(
        proj_string(&metres_to_feet.inverse()),
        "+proj=unitconvert +z_in=ft +z_out=m"
    );
    assert_eq!(
        proj_string(&first(&db, &foot, &metre)),
        "+proj=unitconvert +z_in=ft +z_out=m"
    );
    assert_eq!(
        proj_string(&first(&db, &foot, &us_foot)),
        "+proj=unitconvert +z_in=ft +z_out=us-ft"
    );
    assert_eq!(
        proj_string(&first(&db, &us_foot, &foot)),
        "+proj=unitconvert +z_in=us-ft +z_out=ft"
    );

    let height = parse(
        &db,
        r#"VERTCRS["my height",VDATUM["my datum"],CS[vertical,1],AXIS["gravity-related height (H)",up,LENGTHUNIT["metre",1]]]"#,
    );
    let depth = parse(
        &db,
        r#"VERTCRS["my depth",VDATUM["my datum"],CS[vertical,1],AXIS["depth (D)",down,LENGTHUNIT["metre",1]]]"#,
    );
    let depth_us = parse(
        &db,
        r#"VERTCRS["my depth (ftUS)",VDATUM["my datum"],CS[vertical,1],AXIS["depth (D)",down,LENGTHUNIT["US survey foot",0.304800609601219]]]"#,
    );
    assert_eq!(
        proj_string(&first(&db, &height, &depth)),
        "+proj=axisswap +order=1,2,-3"
    );
    assert_eq!(
        proj_string(&first(&db, &height, &depth_us)),
        "+proj=affine +s33=-3.28083333333333"
    );
}

#[test]
fn operation_proj_crs_3_d_to_geog_crs_3_d() {
    let db = database();
    let projected = create_crs_from_proj_string("+proj=merc +vunits=ft +type=crs").expect("merc");
    let geographic =
        create_crs_from_proj_string("+proj=longlat +vunits=m +type=crs").expect("longlat");

    let forward = first(&db, &projected, &geographic);
    assert!(!forward.has_ballpark_transformation());
    assert_eq!(
        proj_string(&forward),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=m +z_in=ft +xy_out=m +z_out=m \
         +step +inv +proj=merc +lon_0=0 +k=1 +x_0=0 +y_0=0 +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m"
    );

    let reverse = first(&db, &geographic, &projected);
    assert!(!reverse.has_ballpark_transformation());
    assert_eq!(
        proj_string(&reverse),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=merc +lon_0=0 +k=1 +x_0=0 +y_0=0 +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=m +z_out=ft"
    );
}

#[test]
fn operation_compound_crs_to_geog_crs_3_d() {
    let db = database();
    let compound = parse(
        &db,
        r#"COMPOUNDCRS["unknown",
    PROJCRS["unknown",
        BASEGEOGCRS["unknown",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]],
                ID["EPSG",6326]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8901]]],
        CONVERSION["unknown",
            METHOD["Mercator (variant A)",
                ID["EPSG",9804]],
            PARAMETER["Latitude of natural origin",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",1,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8807]]],
        CS[Cartesian,2],
            AXIS["(E)",east,
                ORDER[1],
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]],
            AXIS["(N)",north,
                ORDER[2],
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]]],
    VERTCRS["unknown",
        VDATUM["unknown"],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["foot",0.3048,
                    ID["EPSG",9002]]]]]"#,
    );
    let geographic =
        create_crs_from_proj_string("+proj=longlat +vunits=m +type=crs").expect("longlat");

    let forward = first(&db, &compound, &geographic);
    assert!(forward.has_ballpark_transformation());
    assert_eq!(
        proj_string(&forward),
        "+proj=pipeline +step +inv +proj=merc +lon_0=0 +k=1 +x_0=0 \
         +y_0=0 +ellps=WGS84 +step +proj=unitconvert +xy_in=rad \
         +z_in=ft +xy_out=deg +z_out=m"
    );

    let reverse = first(&db, &geographic, &compound);
    assert!(reverse.has_ballpark_transformation());
    assert_eq!(
        proj_string(&reverse),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +z_in=m \
         +xy_out=rad +z_out=ft +step +proj=merc +lon_0=0 +k=1 +x_0=0 \
         +y_0=0 +ellps=WGS84"
    );
}
