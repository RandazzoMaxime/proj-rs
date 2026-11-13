//! PROJ `test/unit/proj_errno_string_test.cpp` through `proj_rs::errno`.
//! The strings and codes are PROJ 9.9.0's (`src/strerrno.cpp`, `proj.h`).

use proj_rs::errno::{error_string, PROJ_ERR_COORD_TRANSFM, PROJ_ERR_INVALID_OP};
use proj_rs::errno::{PROJ_ERR_COORD_TRANSFM_OUTSIDE_GRID, PROJ_ERR_INVALID_OP_WRONG_SYNTAX};

#[test]
fn proj_errno_string_no_error() {
    assert_eq!(error_string(0), None);
}

#[test]
fn proj_errno_string_proj_errnos() {
    assert_eq!(error_string(-1).as_deref(), Some("Unknown error (code -1)"));
    assert_eq!(
        error_string(PROJ_ERR_INVALID_OP_WRONG_SYNTAX).as_deref(),
        Some("Invalid PROJ string syntax")
    );
    assert_eq!(
        error_string(PROJ_ERR_INVALID_OP).as_deref(),
        Some("Unspecified error related to coordinate operation initialization")
    );
    assert_eq!(
        error_string(PROJ_ERR_COORD_TRANSFM).as_deref(),
        Some("Unspecified error related to coordinate transformation")
    );
}

#[test]
fn proj_errno_string_listed_codes() {
    assert_eq!(
        error_string(PROJ_ERR_COORD_TRANSFM_OUTSIDE_GRID).as_deref(),
        Some("Coordinate to transform falls outside grid")
    );
}

#[test]
fn proj_context_errno_string_unknown_code() {
    // The context variant of the C++ test: an unlisted positive code is reported
    // with its number.
    assert_eq!(
        error_string(-999).as_deref(),
        Some("Unknown error (code -999)")
    );
}
