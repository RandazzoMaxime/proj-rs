//! `proj_is_equivalent_to_with_ctx` with `PJ_COMP_EQUIVALENT` and a database: a vertical or a
//! compound CRS is equivalent to itself, as with every other criterion. The database path compared
//! geodetic and projected CRS only and answered false for the others, so pyproj's
//! `CRS.from_epsg(5703) == CRS.from_epsg(5703)` was false. Expected values: C PROJ 9.9.0.

use proj_rs::iso19111::create_from_user_input;
use proj_rs::DatabaseContext;

fn equivalent_db(left: &str, right: &str) -> bool {
    let db = DatabaseContext::create().expect("proj.db");
    let left = create_from_user_input(left, &db).expect("left");
    let right = create_from_user_input(right, &db).expect("right");
    left.is_equivalent_to_db(&right, &db)
}

#[test]
fn vertical_and_compound_crs_are_equivalent_to_themselves() {
    for code in [
        "EPSG:5703",
        "EPSG:5941",
        "EPSG:2272+5703",
        "EPSG:9518",
        "EPSG:4326",
        "EPSG:4979",
    ] {
        assert!(equivalent_db(code, code), "{code}");
    }
}

#[test]
fn different_vertical_crs_stay_different() {
    assert!(!equivalent_db("EPSG:5703", "EPSG:5941"));
    assert!(!equivalent_db("EPSG:2272+5703", "EPSG:2272+5941"));
}
