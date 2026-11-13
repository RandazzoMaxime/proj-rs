//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: `TEST_F(CApi, proj_clone_of_obj_with_alternative_operations)`.
//!
//! NAD27 (EPSG:4267) to NAD83 (EPSG:4269) has several candidates, all with a missing grid in
//! this environment: the rerun with DISCARD gives the WGS 84 pivots. A point picks one, the last
//! used operation is that candidate, a clone keeps the candidate list, and the clone transforms
//! the same point to the same result after the original is dropped.
//!
//! The C++ test passes (40.5, -60), latitude first, as EPSG:4267 is written. `Proj::new_known_crs`
//! normalizes the pair to longitude, latitude, so the same point is (-60, 40.5) here.

use proj_rs::Proj;

#[test]
fn capi_proj_clone_of_obj_with_alternative_operations() {
    let obj = Proj::new_known_crs("EPSG:4267", "EPSG:4269", None).expect("pair");
    let (lon_ref, lat_ref, _, _) = obj.convert_xyzt(-60.0, 40.5, 0.0, 2021.0).expect("forward");
    assert!(lon_ref != -60.0);
    assert!((lon_ref - -60.0).abs() < 1e-3, "lon = {lon_ref}");
    assert!((lat_ref - 40.5).abs() < 1e-3, "lat = {lat_ref}");

    let used = obj
        .last_used_operation()
        .expect("the operation the point used");
    drop(used);

    let clone = obj.clone();
    assert!(obj.is_equivalent_to(&clone));

    drop(obj);
    let (lon, lat, _, _) = clone
        .convert_xyzt(-60.0, 40.5, 0.0, 2021.0)
        .expect("forward, clone");
    assert_eq!(lon, lon_ref);
    assert_eq!(lat, lat_ref);
}
