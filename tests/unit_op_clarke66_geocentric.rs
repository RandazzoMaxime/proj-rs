//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, bound CRS on Clarke 1866 with a grid to NAD83,
//! and a geographic 3D CRS with a TOWGS84 datum to a geocentric CRS in both directions.

use proj_rs::iso19111::{
    create_crs_from_proj_string, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, ProjLines, WktParser,
};

/// `TEST(operation, boundCRS_of_clrk_66_geogCRS_to_nad83_geogCRS)`
///
/// The query counter check of the C++ test (two extra queries for the conus grid) is not
/// ported: this crate does not count database queries.
#[test]
fn operation_bound_crs_of_clrk_66_geog_crs_to_nad83_geog_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_crs_from_proj_string(
        "+proj=latlong +ellps=clrk66 +nadgrids=ntv1_can.dat,conus +type=crs",
    )
    .expect("source");
    let dest = create_crs_from_proj_string("+proj=latlong +datum=NAD83 +type=crs").expect("dest");
    let expected = "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
                    +step +proj=hgridshift +grids=ntv1_can.dat,conus \
                    +step +proj=unitconvert +xy_in=rad +xy_out=deg";

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        expected
    );

    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dest, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert!(list[0].is_equivalent_to(&op));
}

/// `TEST(operation, boundCRS_of_clrk_66_projCRS_to_nad83_geogCRS)`
#[test]
fn operation_bound_crs_of_clrk_66_proj_crs_to_nad83_geog_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_crs_from_proj_string(
        "+proj=utm +zone=17 +ellps=clrk66 +nadgrids=ntv1_can.dat,conus +type=crs",
    )
    .expect("source");
    let dest = create_crs_from_proj_string("+proj=latlong +datum=NAD83 +type=crs").expect("dest");

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=utm +zone=17 +ellps=clrk66 \
         +step +proj=hgridshift +grids=ntv1_can.dat,conus \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation, geogCRS_with_towgs84_to_geocentric)`
#[test]
fn operation_geog_crs_with_towgs84_to_geocentric() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let geog = WktParser::new()
        .attach_database(db.clone())
        .create_from_wkt(
            "GEOGCS[\"WGS84 Coordinate System\",DATUM[\"WGS_1984\",\
             SPHEROID[\"WGS 1984\",6378137,298.257223563],\
             TOWGS84[0,0,0,0,0,0,0],AUTHORITY[\"EPSG\",\"6326\"]],\
             PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433],\
             AXIS[\"Latitude\",NORTH],AXIS[\"Longitude\",EAST],\
             AUTHORITY[\"EPSG\",\"4326\"]]",
        )
        .expect("geographic CRS")
        .crs()
        .expect("CRS")
        .clone();
    let src3d = geog.promote_to_3d(Some(&db));
    let dst = WktParser::new()
        .attach_database(db.clone())
        .create_from_wkt(
            "GEOCCS[\"WGS 84\",DATUM[\"WGS_1984\",\
             SPHEROID[\"WGS 84\",6378137,298.257223563,\
             AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY[\"EPSG\",\"6326\"]],\
             PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],\
             UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]],\
             AXIS[\"Geocentric X\",OTHER],AXIS[\"Geocentric Y\",OTHER],\
             AXIS[\"Geocentric Z\",NORTH],AUTHORITY[\"EPSG\",\"4978\"]]",
        )
        .expect("geocentric CRS")
        .crs()
        .expect("CRS")
        .clone();
    let factory = CoordinateOperationFactory::create();

    let list = factory
        .create_operations(&src3d, &dst, &ctxt)
        .expect("forward operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=WGS84"
    );

    let list = factory
        .create_operations(&dst, &src3d, &ctxt)
        .expect("reverse operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}
