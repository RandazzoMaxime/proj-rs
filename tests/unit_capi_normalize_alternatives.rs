//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: `proj_normalize_for_visualization` for a pair, for a
//! pair with alternative operations (forward and reverse), and the degree queries
//! `proj_degree_input` and `proj_degree_output` on the normalized objects.
//!
//! Expected values are copied from the C++ tests without change. The check of the
//! `proj_pj_info` definition after `proj_trans` in `..._with_alternatives` is not here: the
//! candidate that `proj_trans` selects is not recorded by this facade (see the sheet).

use proj_rs::iso19111::{AuthorityFactory, CoordinateOperationFactory, DatabaseContext, ProjLines};
use proj_rs::{Direction, Proj};

/// `TEST_F(CApi, proj_normalize_for_visualization)`, the second block: EPSG:4326 to EPSG:32631.
///
/// The C++ test writes the normalized object with `proj_as_proj_string`. This facade has no such
/// export, so the normalized operation is written instead, which is the object the facade
/// transforms with. The first block (a PROJ string `+proj=utm`, normalized to a null object) has
/// no counterpart: this facade normalizes CRS pairs at creation and has no normalize call on a
/// single operation.
#[test]
fn capi_proj_normalize_for_visualization() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let source = factory
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326");
    let target = factory
        .create_coordinate_reference_system("32631")
        .expect("EPSG:32631");
    let operation = CoordinateOperationFactory::create()
        .create_operation(&source, &target)
        .expect("operation")
        .expect("EPSG:4326 to EPSG:32631");
    let normalized = operation.normalize_for_visualization().expect("normalized");
    assert_eq!(
        normalized
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=utm +zone=31 +ellps=WGS84"
    );

    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32631", None).expect("pair");
    assert!(proj.degree_input(Direction::Forward));
    assert!(!proj.degree_output(Direction::Forward));
}

/// `TEST_F(CApi, proj_normalize_for_visualization_with_alternatives)`: EPSG:4326 to EPSG:3003,
/// forward, then the inverse of the Roma point. "Approximately Roma" is `(12.5, 42)`.
#[test]
fn capi_proj_normalize_for_visualization_with_alternatives() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:3003", None).expect("pair");
    assert!(proj.degree_input(Direction::Forward));
    assert!(!proj.degree_output(Direction::Forward));

    let (x, y) = proj
        .convert((12.5_f64, 42.0_f64))
        .expect("forward, approximately Roma");
    assert!((x - 1_789_912.462_647_830_37).abs() < 1e-8, "x = {x}");
    assert!((y - 4_655_716.254_025_768_49).abs() < 1e-8, "y = {y}");

    let (lon, lat) = proj
        .project(
            (1_789_912.462_647_830_37_f64, 4_655_716.254_025_768_49_f64),
            true,
        )
        .expect("inverse, approximately Roma");
    assert!((lon - 12.5).abs() < 1e-8, "lon = {lon}");
    assert!((lat - 42.0).abs() < 1e-8, "lat = {lat}");
}

/// `TEST_F(CApi, proj_normalize_for_visualization_with_alternatives_reverse)`: EPSG:3003 to
/// EPSG:4326, the Roma point in, degrees out.
#[test]
fn capi_proj_normalize_for_visualization_with_alternatives_reverse() {
    let proj = Proj::new_known_crs("EPSG:3003", "EPSG:4326", None).expect("pair");
    assert!(!proj.degree_input(Direction::Forward));
    assert!(proj.degree_output(Direction::Forward));

    let (lon, lat) = proj
        .convert((1_789_912.462_647_830_37_f64, 4_655_716.254_025_768_49_f64))
        .expect("forward, approximately Roma");
    assert!((lon - 12.5).abs() < 1e-8, "lon = {lon}");
    assert!((lat - 42.0).abs() < 1e-8, "lat = {lat}");
}
