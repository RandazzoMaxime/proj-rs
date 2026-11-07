//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS to a 3D geographic CRS through
//! `createOperationsCompoundToGeog` (the derived horizontal part, the interpolation pairs and the
//! only-vertical-ballpark intermediate). The C++ `TEST()` names are kept in the doc comments and
//! the expected names and PROJ strings are copied from that file.

use proj_rs::iso19111::{
    create_from_user_input, BallparkUse, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion, UnitOfMeasure, WktObject, WktParser,
};

fn ctx_partial(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt
}

fn ctx_partial_ignore(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = ctx_partial(db, authority);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
}

/// `createFromUserInput("EPSG:<code>")`, which also reads compound codes such as `4150+5773`.
fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    create_from_user_input(&format!("EPSG:{code}"), db).expect(code)
}

fn wkt_crs(db: &DatabaseContext, text: &str) -> Crs {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    match parser.create_from_wkt(text).expect("WKT") {
        WktObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    }
}

fn operations(
    source: &Crs,
    target: &Crs,
    ctxt: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, ctxt)
        .expect("operations")
}

/// `exportToPROJString` with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_ftUS_to_geogCRS_ft)`
#[test]
fn operation_compound_ftus_to_geog_ft() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    // NAD83(2011) + NAVD88 height (ftUS)
    let src = epsg(&db, "6318+6360");
    // NAD83(2011) with foot
    let dst = epsg(&db, "6319").alter_cs_linear_unit(&UnitOfMeasure::foot());
    let res = operations(&src, &dst, &ctxt);
    assert!(!res.is_empty());
    assert_eq!(
        proj(&db, &res[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=rad +z_out=m \
         +step +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=ft \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj(&db, res.last().expect("last")),
        "+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=ft"
    );
    let res_inv = operations(&dst, &src, &ctxt);
    assert!(!res_inv.is_empty());
    assert_eq!(
        proj(&db, &res_inv[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=ft +xy_out=rad +z_out=m \
         +step +inv +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=us-ft \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj(&db, res_inv.last().expect("last")),
        "+proj=unitconvert +xy_in=deg +z_in=ft +xy_out=deg +z_out=us-ft"
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_with_vertical_unit_change_and_complex_horizontal_change)`
#[test]
fn operation_compound_to_geog_with_vertical_unit_change_and_complex_horizontal_change() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    // NAD83(2011) + NAVD88 height (ftUS)
    let src = epsg(&db, "6318+6360");
    // WGS84(G1762) 3D
    let dst = epsg(&db, "7665");
    let list_ftus = operations(&src, &dst, &ctxt);
    // NAD83(2011) + NAVD88 height
    let src_metre = epsg(&db, "6318+5703");
    let list_metre = operations(&src_metre, &dst, &ctxt);
    assert_eq!(list_ftus.len(), list_metre.len());
    assert!(!list_ftus.is_empty());
    assert_eq!(
        list_ftus[0].name(),
        format!(
            "Conversion from NAVD88 height (ftUS) to NAVD88 height + {}",
            list_metre[0].name()
        )
    );
    assert_eq!(
        proj(&db, &list_ftus[0]),
        proj(&db, &list_metre[0]).replace(
            "+step +proj=unitconvert +xy_in=deg +xy_out=rad",
            "+step +proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=rad +z_out=m"
        )
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_2D_promote_to_3D_context)`
#[test]
fn operation_compound_to_geog_2d_promote_to_3d_context() {
    let db = DatabaseContext::create().expect("proj.db");
    // NAD83 + NAVD88 height
    let src = epsg(&db, "4269+5703");
    let dst = epsg(&db, "4269").promote_to_3d(Some(&db));
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    let list = operations(&src, &dst, &ctxt);
    assert_eq!(list.len(), 133);
    let reverse = operations(&dst, &src, &ctxt);
    assert_eq!(reverse.len(), list.len());
}

/// `TEST(operation, compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS)`
#[test]
fn operation_compound_of_vert_crs_with_geoid_model_to_geog() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    let src = wkt_crs(&db, GEOIDMODEL_NAME_SRC);
    let dst = epsg(&db, "4269").promote_to_3d(Some(&db));
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of SPCS83 Pennsylvania South zone (meters) + \
         Ballpark geographic offset from NAD83 to NAD83(2011) + \
         Inverse of NAD83(2011) to NAVD88 height (1) + \
         Ballpark geographic offset from NAD83(2011) to NAD83"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=lcc +lat_0=39.3333333333333 +lon_0=-77.75 \
         +lat_1=40.9666666666667 +lat_2=39.9333333333333 +x_0=600000 \
         +y_0=0 +ellps=GRS80 \
         +step +proj=vgridshift +grids=us_noaa_g2012bu0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_3D_Amersfoort_NAP_height_to_Amersfoort)`
#[test]
fn operation_compound_to_geog_3d_amersfoort_nap_height_to_amersfoort() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut ctxt = ctx_partial_ignore(&db, "EPSG");
    ctxt.set_ballpark_use(BallparkUse::Forbid);
    // Amersfoort / RD + NAP height
    let src = epsg(&db, "7415");
    // Amersfoort promoted to 3D
    let dst = epsg(&db, "4289").promote_to_3d(Some(&db));
    let list = operations(&src, &dst, &ctxt);
    assert_eq!(list.len(), 1);
    // I'm not sure this is absolutely correct, but we do not certainly
    // a purely 2D grid-based horizontal transformation to be used when
    // going back from ETRS89 to Amersfoort.
    assert_eq!(
        list[0].name(),
        "Inverse of RD + \
         Inverse of ETRS89-NLD [AGRS2010] to NAP height (2) using Amersfoort to \
         ETRS89-NLD [AGRS2010] (8)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=sterea +lat_0=52.1561605555556 \
         +lon_0=5.38763888888889 +k=0.9999079 +x_0=155000 +y_0=463000 \
         +ellps=bessel \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1 \
         +step +proj=push +v_1 +v_2 \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=565.7381 +y=50.4018 +z=465.2904 \
         +rx=0.395025981036064 +ry=-0.330772431242031 +rz=1.87607329462821 \
         +s=4.07244 +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=vgridshift +grids=nl_nsgi_nlgeo2018.tif +multiplier=1 \
         +step +proj=cart +ellps=GRS80 \
         +step +inv +proj=helmert +x=565.7381 +y=50.4018 +z=465.2904 \
         +rx=0.395025981036064 +ry=-0.330772431242031 +rz=1.87607329462821 \
         +s=4.07244 +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=bessel \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1 \
         +step +proj=pop +v_1 +v_2"
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_3D_using_intermediate_of_horizontal_transform)`
#[test]
fn operation_compound_to_geog_3d_using_intermediate_of_horizontal_transform() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut ctxt = ctx_partial(&db, "");
    ctxt.set_grid_availability_use(GridAvailabilityUse::KnownAvailable);
    // "PNG94 / PNGMG94 zone 54 + Kumul 34 height"
    let src = epsg(&db, "5550+7651");
    // WGS 84 (G2139) (3D)
    let list = operations(&src, &epsg(&db, "9754"), &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of Papua New Guinea Map Grid 1994 zone 54 + \
         PNG94 to WGS 84 (1) + \
         Inverse of EGM96 height to Kumul 34 height (1) + \
         Inverse of WGS 84 to EGM96 height (1) + \
         WGS 84 to WGS 84 (G2139)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=54 +south +ellps=GRS80 \
         +step +proj=geogoffset +dh=0.87 \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}

/// `GEOIDMODEL_NAME_SRC`: the C++ string literal, concatenated.
const GEOIDMODEL_NAME_SRC: &str = r#"COMPOUNDCRS["NAD83 / Pennsylvania South + NAVD88 height",
    PROJCRS["NAD83 / Pennsylvania South",
        BASEGEOGCRS["NAD83",
            DATUM["North American Datum 1983",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["SPCS83 Pennsylvania South zone (meters)",
            METHOD["Lambert Conic Conformal (2SP)",
                ID["EPSG",9802]],
            PARAMETER["Latitude of false origin",39.3333333333333,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8821]],
            PARAMETER["Longitude of false origin",-77.75,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8822]],
            PARAMETER["Latitude of 1st standard parallel",40.9666666666667,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8823]],
            PARAMETER["Latitude of 2nd standard parallel",39.9333333333333,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8824]],
            PARAMETER["Easting at false origin",600000,
                LENGTHUNIT["metre",1],
                ID["EPSG",8826]],
            PARAMETER["Northing at false origin",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8827]]],
        CS[Cartesian,2],
            AXIS["easting (X)",east,
                ORDER[1],
                LENGTHUNIT["metre",1]],
            AXIS["northing (Y)",north,
                ORDER[2],
                LENGTHUNIT["metre",1]]],
    VERTCRS["NAVD88 height",
        VDATUM["North American Vertical Datum 1988"],
        CS[vertical,1],
            AXIS["gravity-related height (H)",up,
                LENGTHUNIT["metre",1]],
        GEOIDMODEL["GEOID12B"]]]"#;
