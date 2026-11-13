//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: `proj_normalize_for_visualization` for a pair, for a
//! pair with alternative operations (forward and reverse), and the degree queries
//! `proj_degree_input` and `proj_degree_output` on the normalized objects.
//!
//! Expected values are copied from the C++ tests without change. The `proj_pj_info` definition
//! after `proj_trans` in `..._with_alternatives` is the candidate the point used (`Proj::proj_info`).

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
    // Before any point, PROJ has no chosen candidate (`info.cpp` 178-186).
    assert_eq!(
        proj.proj_info().definition.as_deref(),
        Some("unavailable until proj_trans is called")
    );

    let (x, y) = proj
        .convert((12.5_f64, 42.0_f64))
        .expect("forward, approximately Roma");
    assert!((x - 1_789_912.462_647_830_4).abs() < 1e-8, "x = {x}");
    assert!((y - 4_655_716.254_025_768_5).abs() < 1e-8, "y = {y}");
    // The candidate the point used is the one `proj_pj_info` reports (`info.cpp` 170-171).
    assert_eq!(
        proj.proj_info().definition.as_deref(),
        Some(
            "proj=pipeline step proj=unitconvert xy_in=deg xy_out=rad \
             step proj=push v_3 step proj=cart ellps=WGS84 \
             step inv proj=helmert x=-104.1 y=-49.1 z=-9.9 rx=0.971 \
             ry=-2.917 rz=0.714 s=-11.68 convention=position_vector \
             step inv proj=cart ellps=intl step proj=pop v_3 \
             step proj=tmerc lat_0=0 lon_0=9 k=0.9996 x_0=1500000 \
             y_0=0 ellps=intl"
        )
    );

    let (lon, lat) = proj
        .project(
            (1_789_912.462_647_830_4_f64, 4_655_716.254_025_768_5_f64),
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
        .convert((1_789_912.462_647_830_4_f64, 4_655_716.254_025_768_5_f64))
        .expect("forward, approximately Roma");
    assert!((lon - 12.5).abs() < 1e-8, "lon = {lon}");
    assert!((lat - 42.0).abs() < 1e-8, "lat = {lat}");
}
