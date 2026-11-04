//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: NAD27 to WGS 84 (G1762), and ETRS89 to a
//! WGS 84 datum ensemble read from WKT.

use proj_rs::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion, WktObject, WktParser,
};

/// `TEST(operation, geogCRS_to_geogCRS_context_NAD27_to_WGS84_G1762)`
#[test]
fn operation_geog_to_geog_context_nad27_to_wgs84_g1762() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let mut context = CoordinateOperationContext::create(&db, "");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &factory
                .create_coordinate_reference_system("4267")
                .expect("4267"),
            &factory
                .create_coordinate_reference_system("9057")
                .expect("9057"),
            &context,
        )
        .expect("operations");
    assert!(list.len() >= 78, "{}", list.len());
    assert_eq!(
        list[0].name(),
        "NAD27 to WGS 84 (33) + WGS 84 to WGS 84 (G1762)"
    );
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=hgridshift +grids=ca_nrc_ntv2_0.tif \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        list[1].name(),
        "NAD27 to WGS 84 (3) + WGS 84 to WGS 84 (G1762)"
    );
    assert_eq!(
        list[2].name(),
        "NAD27 to WGS 84 (79) + WGS 84 to WGS 84 (G1762)"
    );
    assert_eq!(
        list[3].name(),
        "NAD27 to WGS 84 (4) + WGS 84 to WGS 84 (G1762)"
    );
}

/// `TEST(operation, geogCRS_to_geogCRS_context_datum_ensemble)`
#[test]
fn operation_geog_to_geog_context_datum_ensemble() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let context = CoordinateOperationContext::create(&db, "EPSG");
    let wkt = "GEOGCRS[\"unknown\",\
        ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
        MEMBER[\"World Geodetic System 1984 (Transit)\",ID[\"EPSG\",1166]],\
        MEMBER[\"World Geodetic System 1984 (G730)\",ID[\"EPSG\",1152]],\
        MEMBER[\"World Geodetic System 1984 (G873)\",ID[\"EPSG\",1153]],\
        MEMBER[\"World Geodetic System 1984 (G1150)\",ID[\"EPSG\",1154]],\
        MEMBER[\"World Geodetic System 1984 (G1674)\",ID[\"EPSG\",1155]],\
        MEMBER[\"World Geodetic System 1984 (G1762)\",ID[\"EPSG\",1156]],\
        ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
        LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]],ID[\"EPSG\",7030]],\
        ENSEMBLEACCURACY[2]],\
        PRIMEM[\"Greenwich\",0,ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9102]],\
        ID[\"EPSG\",8901]],\
        CS[ellipsoidal,2,ID[\"EPSG\",6422]],\
        AXIS[\"Geodetic latitude (Lat)\",north,ORDER[1]],\
        AXIS[\"Geodetic longitude (Lon)\",east,ORDER[2]],\
        ANGLEUNIT[\"degree (supplier to define representation)\",\
        0.0174532925199433,ID[\"EPSG\",9122]]]";
    let target = match WktParser::new().create_from_wkt(wkt).expect("WKT") {
        WktObject::Crs(crs) => crs,
        other => panic!("expected a CRS: {other:?}"),
    };
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &factory
                .create_coordinate_reference_system("4258")
                .expect("4258"),
            &target,
            &context,
        )
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name(), "ETRS89 to WGS 84 (1)");
}
