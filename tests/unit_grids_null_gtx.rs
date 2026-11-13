//! PROJ `test/unit/test_grids.cpp` cases for the null grids and the GTX
//! vertical grid. Expected values are PROJ 9.9.0's.
//!
//! The C++ `reassign_context` calls have no counterpart here: a grid set owns
//! its bytes and holds no context. `reopen` is ported as a re-read.

use std::f64::consts::PI;

use proj_rs::grids::{GenericShiftGridSet, HorizontalShiftGridSet, VerticalShiftGridSet};

mod common;

#[test]
fn vertical_shift_grid_set_null() {
    common::use_fixture_search_path();
    let mut set = VerticalShiftGridSet::open("null").expect("null set");
    let grid = set.grid_at(0.0, 0.0).expect("grid");
    assert_eq!(grid.width(), 3);
    assert_eq!(grid.height(), 3);
    assert_eq!(grid.extent().west, -PI);
    assert!(grid.is_null_grid());
    assert!(!grid.has_changed());
    assert_eq!(grid.value_at(0, 0, 0), Some(0.0));
    assert!(!grid.is_nodata(0.0, 0.0));
    set.reopen().expect("reopen");
    assert!(set.grid_at(0.0, 0.0).is_some());
}

#[test]
fn vertical_shift_grid_set_gtx() {
    common::use_fixture_search_path();
    assert!(VerticalShiftGridSet::open("foobar").is_err());
    let mut set = VerticalShiftGridSet::open("tests/test_nodata.gtx").expect("gtx set");
    assert!(set.grid_at(-100.0, -100.0).is_none());
    let grid = set
        .grid_at(4.15 / 180.0 * PI, 52.15 / 180.0 * PI)
        .expect("grid");
    assert!(grid.is_nodata(-88.8888_f32, 1.0));
    set.reopen().expect("reopen");
    assert!(set.grid_at(4.15 / 180.0 * PI, 52.15 / 180.0 * PI).is_some());
}

#[test]
fn horizontal_shift_grid_set_null() {
    common::use_fixture_search_path();
    let mut set = HorizontalShiftGridSet::open("null").expect("null set");
    let grid = set.grid_at(0.0, 0.0).expect("grid");
    assert_eq!(grid.width(), 3);
    assert_eq!(grid.height(), 3);
    assert_eq!(grid.extent().west, -PI);
    assert!(grid.is_null_grid());
    assert!(!grid.has_changed());
    assert_eq!(grid.horizontal_shift(0, 0), Some((0.0, 0.0)));
    set.reopen().expect("reopen");
    assert!(set.grid_at(0.0, 0.0).is_some());
}

#[test]
fn generic_shift_grid_set_null() {
    common::use_fixture_search_path();
    let mut set = GenericShiftGridSet::open("null").expect("null set");
    let grid = set.grid_at(0.0, 0.0).expect("grid");
    assert_eq!(grid.width(), 3);
    assert_eq!(grid.height(), 3);
    assert_eq!(grid.extent().west, -PI);
    assert!(grid.is_null_grid());
    assert!(!grid.has_changed());
    assert_eq!(grid.value_at(0, 0, 0), Some(0.0));
    assert_eq!(grid.unit(0), "");
    assert_eq!(grid.description(0), "");
    assert_eq!(grid.metadata_item("foo"), None);
    assert_eq!(grid.samples_per_pixel(), 0);
    set.reopen().expect("reopen");
    assert!(set.grid_at(0.0, 0.0).is_some());
}
