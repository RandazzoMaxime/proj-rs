//! PROJ 9.9.0 `test/unit/test_c_api.cpp`, `proj_create_crs_to_crs` with a coordinate epoch on one
//! side: ITRF2014 at epoch 2025.0 against GDA2020, both directions.
//!
//! `ITRF2014@2025.0` is the 2D geographic ITRF2014, `EPSG:9000`, with the epoch set. Coordinates
//! are longitude, latitude here, where the C++ test passes latitude, longitude.

use proj_rs::iso19111::CoordinateMetadata;
use proj_rs::{AuthorityFactory, Crs, DatabaseContext, GeodeticCRS, Proj};

fn near(got: f64, expect: f64, tol: f64) {
    assert!((got - expect).abs() <= tol, "{got} vs {expect} (tol {tol})");
}

/// `TEST_F(CApi, proj_create_crs_to_crs_coordinate_metadata_in_src)`
#[test]
fn capi_create_crs_to_crs_coordinate_metadata_in_src() {
    let source = Proj::new("EPSG:9000")
        .expect("ITRF2014")
        .coordinate_metadata_create(2025.0)
        .expect("epoch");
    let target = Proj::new("EPSG:7844").expect("GDA2020");
    let proj = source
        .create_crs_to_crs_from_pj(&target, None, None)
        .expect("pair");
    let (lon, lat, _) = proj.convert_xyz(130.0, -30.0, 0.0).expect("forward");
    near(lat, -30.000_002_665_5, 1e-10);
    near(lon, 129.999_998_371_2, 1e-10);
}

/// `TEST_F(CApi, proj_create_crs_to_crs_coordinate_metadata_in_target)`
#[test]
fn capi_create_crs_to_crs_coordinate_metadata_in_target() {
    let source = Proj::new("EPSG:7844").expect("GDA2020");
    let target = Proj::new("EPSG:9000")
        .expect("ITRF2014")
        .coordinate_metadata_create(2025.0)
        .expect("epoch");
    let proj = source
        .create_crs_to_crs_from_pj(&target, None, None)
        .expect("pair");
    let (lon, lat, _) = proj
        .convert_xyz(129.999_998_371_2, -30.000_002_665_5, 0.0)
        .expect("forward");
    near(lon, 130.0, 1e-10);
    near(lat, -30.0, 1e-10);
}

/// `TEST_F(CApi, proj_crs_has_point_motion_operation)`
///
/// Checked through `CoordinateMetadata::create`, which asks the same question of a static CRS:
/// it takes an epoch only when the CRS has a point motion operation.
#[test]
fn capi_crs_has_point_motion_operation() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let nad27 = Crs::Geodetic(GeodeticCRS::Geographic(
        factory.create_geographic_crs("4267").expect("NAD27"),
    ));
    assert!(CoordinateMetadata::create(&nad27, 2010.0, &db).is_err());
    let csrs_v7 = Crs::Geodetic(GeodeticCRS::Geographic(
        factory
            .create_geographic_crs("8255")
            .expect("NAD83(CSRS)v7"),
    ));
    assert!(CoordinateMetadata::create(&csrs_v7, 2010.0, &db).is_ok());
}

/// `TEST_F(CApi, proj_crs_promote_to_3D_on_coordinate_metadata)`
///
/// The coordinate metadata is promoted through its CRS, as `proj_crs_promote_to_3D` does for a
/// coordinate metadata object: the epoch is kept and the CRS is the 3D ITRF2014, `EPSG:7912`.
#[test]
fn capi_crs_promote_to_3d_on_coordinate_metadata() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let itrf2d = Crs::Geodetic(GeodeticCRS::Geographic(
        factory.create_geographic_crs("9000").expect("ITRF2014 2D"),
    ));
    let cm_2d = CoordinateMetadata::create(&itrf2d, 2010.0, &db).expect("epoch");
    let promoted = cm_2d.crs().promote_to_3d(Some(&db));
    let cm_3d = CoordinateMetadata::create(&promoted, 2010.0, &db).expect("epoch");
    near(cm_3d.coordinate_epoch().expect("epoch"), 2010.0, 1e-10);
    let Crs::Geographic(geographic) = cm_3d.crs() else {
        panic!("promoted CRS is not geographic");
    };
    assert_eq!(geographic.coordinate_system().axes().len(), 3);
    let code = cm_3d
        .crs()
        .authority_code("EPSG")
        .map(|identifier| identifier.code().to_owned());
    assert_eq!(code.as_deref(), Some("7912"));
}
