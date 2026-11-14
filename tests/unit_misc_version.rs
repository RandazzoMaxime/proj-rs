//! PROJ `test/unit/test_misc.cpp` (`misc.version`) through `proj_rs::version`:
//! the version constants and the `PROJ_COMPUTE_VERSION` / `PROJ_AT_LEAST_VERSION`
//! macros of `proj.h` as `const fn`s. Expected values are PROJ 9.9.0's.

use proj_rs::version::{at_least, compute, MAJOR, MINOR, NUMBER, PATCH};

#[test]
fn misc_version() {
    assert_eq!(compute(2200, 98, 76), 22_009_876);
    assert_eq!(NUMBER, MAJOR * 10_000 + MINOR * 100 + PATCH);
    assert!(at_least(MAJOR, MINOR, PATCH));
    assert!(at_least(MAJOR - 1, MINOR, PATCH));
    assert!(!at_least(MAJOR, MINOR, PATCH + 1));
}

#[test]
fn version_constants_are_proj_9_9_0() {
    assert_eq!((MAJOR, MINOR, PATCH), (9, 9, 0));
    assert_eq!(NUMBER, 90_900);
}
