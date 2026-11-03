//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-b`: `createOperations` between
//! a projected or compound CRS with bound vertical components and a 3D geographic CRS.
//!
//! Each function keeps the C++ `TEST()` name in snake case. Expected names and PROJ strings are
//! copied from the C++ file.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
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

/// `exportToPROJString` of an operation.
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

/// `createOperations` over the EPSG authority, with the grid availability ignored and partial
/// intersection, as the C++ `ctxt` of these tests.
fn epsg_operations(db: &DatabaseContext, source: &Crs, target: &Crs) -> Vec<CoordinateOperation> {
    let mut context = CoordinateOperationContext::create(db, "EPSG");
    context
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_spatial_criterion(SpatialCriterion::PartialIntersection);
    CoordinateOperationFactory::create()
        .create_operations(source, target, &context)
        .expect("operations")
}

#[test]
fn operation_compound_crs_with_bound_of_projected_and_bound_of_vertical_to_geog3_d() {
    let db = database();
    let source = parse(
        &db,
        r#"COMPD_CS["TempTM + CGVD28 height - HT2_0",
    PROJCS["Custom",
        GEOGCS["NAD83(CSRS)",
            DATUM["NAD83_Canadian_Spatial_Reference_System",
                SPHEROID["GRS 1980",6378137,298.257222101,
                    AUTHORITY["EPSG","7019"]],
                TOWGS84[0,0,0,0,0,0,0],
                AUTHORITY["EPSG","6140"]],
            PRIMEM["Greenwich",0,
                AUTHORITY["EPSG","8901"]],
            UNIT["degree",0.0174532925199433,
                AUTHORITY["EPSG","9122"]],
            AUTHORITY["EPSG","4617"]],
        PROJECTION["Transverse_Mercator"],
        PARAMETER["latitude_of_origin",49.351346659616],
        PARAMETER["central_meridian",-123.20266499149],
        PARAMETER["scale_factor",1],
        PARAMETER["false_easting",15307.188],
        PARAMETER["false_northing",6540.975],
        UNIT["Meters",1],
        AXIS["Easting",EAST],
        AXIS["Northing",NORTH]],
    VERT_CS["CGVD28 height - HT2_0",
        VERT_DATUM["Canadian Geodetic Vertical Datum of 1928",2005,
            EXTENSION["PROJ4_GRIDS","HT2_0.gtx"],
            AUTHORITY["EPSG","5114"]],
        UNIT["metre",1,
            AUTHORITY["EPSG","9001"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","5713"]]]"#,
    );
    // NAD83(CSRS) 3D
    let target = epsg_crs(&db, "4955");
    let list = epsg_operations(&db, &source, &target);
    assert_eq!(list.len(), 1);
    assert_eq!(
        proj_string(&list[0]),
        "+proj=pipeline \
         +step +inv +proj=tmerc +lat_0=49.351346659616 \
         +lon_0=-123.20266499149 +k=1 \
         +x_0=15307.188 +y_0=6540.975 +ellps=GRS80 \
         +step +proj=vgridshift +grids=HT2_0.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_compound_crs_to_geog_crs_3_d_with_same_geog_src_target_interp_context() {
    let db = database();
    // Tests a mix of Datum and DatumEnsemble regarding WGS 84 when we compare the datums used in the
    // source -> interpolation_crs and interpolation_crs -> target transformations.
    let target = parse(
        &db,
        r#"COMPOUNDCRS["WGS 84 + my_height",
    GEOGCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        CS[ellipsoidal,2],
            AXIS["geodetic latitude (Lat)",north,
                ORDER[1],
                ANGLEUNIT["degree",0.0174532925199433]],
            AXIS["geodetic longitude (Lon)",east,
                ORDER[2],
                ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4326]],
    BOUNDCRS[
        SOURCECRS[
            VERTCRS["my_height",
                VDATUM["my_height"],
                CS[vertical,1],
                    AXIS["up",up,
                        LENGTHUNIT["metre",1,
                            ID["EPSG",9001]]]]],
        TARGETCRS[
            GEOGCRS["WGS 84",
                DATUM["World Geodetic System 1984",
                    ELLIPSOID["WGS 84",6378137,298.257223563,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                CS[ellipsoidal,3],
                    AXIS["geodetic latitude (Lat)",north,
                        ORDER[1],
                        ANGLEUNIT["degree",0.0174532925199433]],
                    AXIS["geodetic longitude (Lon)",east,
                        ORDER[2],
                        ANGLEUNIT["degree",0.0174532925199433]],
                    AXIS["ellipsoidal height (h)",up,
                        ORDER[3],
                        LENGTHUNIT["metre",1]],
                ID["EPSG",4979]]],
        ABRIDGEDTRANSFORMATION["my_height to WGS84 ellipsoidal height",
            METHOD["GravityRelatedHeight to Geographic3D"],
            PARAMETERFILE["Geoid (height correction) model file","fake.gtx",
                ID["EPSG",8666]]]]]"#,
    );
    let list = epsg_operations(&db, &epsg_crs(&db, "4979"), &target);
    assert_eq!(list.len(), 1);
    assert_eq!(
        proj_string(&list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift +grids=fake.gtx +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
