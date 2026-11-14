//! PROJ 9.9.0 `test/unit/test_coordinates.cpp`,
//! `TEST(coordinateMetadata, crs_with_point_motion_operation_and_promote_to_3D)`.
//!
//! EPSG:8255 (NAD83(CSRS)v7) is static but has point motion operations, so it takes an epoch
//! with a database and no epoch without one. EPSG:4267 (NAD27) has none. Both promote to 3D
//! through their CRS, and the promoted object keeps the epoch.

use proj_rs::iso19111::{CoordinateMetadata, Crs, Equivalence, GeodeticCRS};
use proj_rs::{AuthorityFactory, DatabaseContext};

fn geographic(factory: &AuthorityFactory, code: &str) -> Crs {
    Crs::Geodetic(GeodeticCRS::Geographic(
        factory.create_geographic_crs(code).expect(code),
    ))
}

#[test]
fn crs_with_point_motion_operation_and_promote_to_3d() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");

    // "NAD83(CSRS)v7": an epoch needs the database to find the point motion operation.
    let csrs_v7 = geographic(&factory, "8255");
    assert!(CoordinateMetadata::create_without_database(&csrs_v7, 2023.5).is_err());
    let with_epoch = CoordinateMetadata::create(&csrs_v7, 2023.5, &db).expect("epoch");
    let promoted = with_epoch.promote_to_3d(Some(&db));
    assert!(promoted
        .crs()
        .is_equivalent_to_with(&csrs_v7.promote_to_3d(Some(&db)), Equivalence::Strict));
    let epoch = promoted.coordinate_epoch().expect("the epoch is kept");
    assert!((epoch - 2023.5).abs() <= 1e-10, "{epoch}");

    // NAD27: no point motion operation, so an epoch is refused, and none is kept.
    let nad27 = geographic(&factory, "4267");
    assert!(CoordinateMetadata::create(&nad27, 2023.5, &db).is_err());
    let without_epoch = CoordinateMetadata::create_without_epoch(&nad27)
        .expect("a static CRS takes no epoch")
        .promote_to_3d(Some(&db));
    assert!(without_epoch
        .crs()
        .is_equivalent_to_with(&nad27.promote_to_3d(Some(&db)), Equivalence::Strict));
    assert_eq!(without_epoch.coordinate_epoch(), None);
}
