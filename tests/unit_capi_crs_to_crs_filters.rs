//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: the candidate filters of `proj_create_crs_to_crs_from_pj`
//! (`ACCURACY`, `ALLOW_BALLPARK`) and the ballpark-only pair `EPSG:25831+5773` to `EPSG:7665`.
//!
//! The `ACCURACY` and `ALLOW_BALLPARK` options are checked on the ISO 19111 selection
//! ([`best_pipeline_filtered`]): [`Proj::create_crs_to_crs_from_pj`] refuses every option,
//! so a null result from it would not test the filter. The call without options goes through
//! [`Proj::create_crs_to_crs_from_pj`], as the C test does.

mod common;

use proj_rs::iso19111::{best_pipeline_filtered, Ballpark, CandidateFilter, OnlyBest};
use proj_rs::Proj;

fn near(got: f64, expect: f64, tol: f64) {
    assert!((got - expect).abs() <= tol, "{got} vs {expect} (tol {tol})");
}

/// `TEST_F(CApi, proj_create_crs_to_crs_from_pj_accuracy_filter)`
///
/// `EPSG:4326` to `EPSG:4258` has a transformation, and no options keep it. With
/// `ACCURACY=0.05` no candidate is known to be that accurate, so the pair has no transformation.
#[test]
fn capi_proj_create_crs_to_crs_from_pj_accuracy_filter() {
    let src = Proj::new("EPSG:4326").expect("EPSG:4326");
    let dst = Proj::new("EPSG:4258").expect("EPSG:4258");
    assert!(src.create_crs_to_crs_from_pj(&dst, None, None).is_ok());

    let filter = CandidateFilter {
        accuracy: Some(0.05),
        ballpark: Ballpark::Allowed,
    };
    assert!(
        best_pipeline_filtered("EPSG:4326", "EPSG:4258", OnlyBest::Off, filter).is_err(),
        "ACCURACY=0.05 keeps a candidate"
    );
}

/// `TEST_F(CApi, proj_create_crs_to_crs_from_pj_ballpark_filter)`
///
/// `EPSG:4267` (NAD 27) to `EPSG:4258` has a transformation. `ALLOW_BALLPARK=NO` drops the
/// ballpark candidates, and none is left.
#[test]
fn capi_proj_create_crs_to_crs_from_pj_ballpark_filter() {
    let src = Proj::new("EPSG:4267").expect("EPSG:4267");
    let dst = Proj::new("EPSG:4258").expect("EPSG:4258");
    assert!(src.create_crs_to_crs_from_pj(&dst, None, None).is_ok());

    let filter = CandidateFilter {
        accuracy: None,
        ballpark: Ballpark::Forbidden,
    };
    assert!(
        best_pipeline_filtered("EPSG:4267", "EPSG:4258", OnlyBest::Off, filter).is_err(),
        "ALLOW_BALLPARK=NO keeps a candidate"
    );
}

/// `TEST_F(CApi, proj_create_crs_to_crs_with_only_ballpark_transformations)`
///
/// `ETRS89 / UTM zone 31N` with `EGM96 height` to `WGS 84 (G1762)`. The result is in degrees,
/// longitude first, and the height goes through the EGM96 geoid grid.
#[test]
fn capi_proj_create_crs_to_crs_with_only_ballpark_transformations() {
    common::use_fixture_search_path();
    let proj = Proj::new_known_crs("EPSG:25831+5773", "EPSG:7665", None).expect("pair");
    let (x, y, z) = proj
        .convert_xyz(500_000.0, 4_500_000.0, 0.0)
        .expect("point");
    near(x, 3.0, 1e-9);
    near(y, 40.650_856_516_605_55, 1e-9);
    near(z, 47.726_000_236_085_7, 1e-3);
}
