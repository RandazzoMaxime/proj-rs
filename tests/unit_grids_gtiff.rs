//! PROJ `test/unit/test_grids.cpp` cases for GeoTIFF shift grids: pixels,
//! values in blocks, metadata and subgrids. Expected values are PROJ 9.9.0's.
//!
//! The C++ `float` literals are written with the fewest digits that give the
//! same `f32`; the `double` ones with the shortest round-trip form.

use std::f64::consts::PI;

use proj_rs::grids::{GenericShiftGridSet, HorizontalShiftGridSet};

mod common;

#[test]
fn horizontal_shift_grid_set_gtiff() {
    common::use_fixture_search_path();
    let mut set = HorizontalShiftGridSet::open("tests/test_hgrid.tif").expect("gtiff set");
    assert_eq!(set.format(), "gtiff");
    assert!(
        set.name().contains("tests/test_hgrid.tif"),
        "{}",
        set.name()
    );
    assert_eq!(set.grids().len(), 1);
    assert!(set.grid_at(-100.0, -100.0).is_none());
    let grid = set
        .grid_at(5.5 / 180.0 * PI, 53.5 / 180.0 * PI)
        .expect("grid");
    assert_eq!(grid.width(), 4);
    assert_eq!(grid.height(), 4);
    assert_eq!(grid.extent().west, 4.0 / 180.0 * PI);
    assert!(!grid.is_null_grid());
    assert!(!grid.has_changed());
    let (lon, lat) = grid.horizontal_shift(0, 3).expect("shift");
    // C++: `static_cast<float>(14400.0 / 3600. / 180 * M_PI)` and the 900 arc-second one.
    assert_eq!(lon, 0.069_813_17_f32);
    assert_eq!(lat, 0.004_363_323_f32);
    set.reopen().expect("reopen");
    assert!(set.grid_at(5.5 / 180.0 * PI, 53.5 / 180.0 * PI).is_some());
}

#[test]
fn generic_shift_grid_set_gtiff() {
    common::use_fixture_search_path();
    assert!(GenericShiftGridSet::open("foobar").is_err());
    let mut set =
        GenericShiftGridSet::open("tests/nkgrf03vel_realigned_extract.tif").expect("gtiff set");
    assert!(set.grid_at(-100.0, -100.0).is_none());
    let grid = set
        .grid_at(21.3333333 / 180.0 * PI, 63.0 / 180.0 * PI)
        .expect("grid");
    assert_eq!(grid.width(), 5);
    assert_eq!(grid.height(), 5);
    assert!(grid.extent().is_geographic);
    assert_eq!(grid.extent().west, 21.0 / 180.0 * PI);
    assert!(!grid.is_null_grid());
    assert!(!grid.has_changed());
    // A sample past `samples_per_pixel` is an error, not a value.
    assert_eq!(grid.value_at(0, 0, grid.samples_per_pixel()), None);
    assert_eq!(grid.value_at(0, 0, 0), Some(0.207_838_91_f32));
    assert_eq!(grid.value_at(1, 0, 0), Some(0.224_270_36_f32));
    assert_eq!(grid.value_at(0, 1, 0), Some(0.197_180_2_f32));
    let mut one = [0.0_f32; 1];
    assert!(grid.values_at(0, 0, 1, 1, &[0], &mut one).is_some());
    assert_eq!(one[0], 0.207_838_91_f32);
    let mut block = [0.0_f32; 2 * 4 * 3];
    assert!(grid.values_at(2, 1, 2, 4, &[0, 1, 2], &mut block).is_some());
    let mut idx = 0;
    for y in 0..4 {
        for x in 0..2 {
            for band in 0..3 {
                assert_eq!(grid.value_at(2 + x, 1 + y, band), Some(block[idx]));
                idx += 1;
            }
        }
    }
    assert_eq!(
        grid.metadata_item("area_of_use"),
        Some("Nordic and Baltic countries")
    );
    assert_eq!(grid.metadata_item("non_existing"), None);
    assert_eq!(grid.metadata_item_for_sample("non_existing", 1), None);
    assert_eq!(grid.metadata_item_for_sample("non_existing", 10), None);
    set.reopen().expect("reopen");
    assert!(set
        .grid_at(21.3333333 / 180.0 * PI, 63.0 / 180.0 * PI)
        .is_some());
}

#[test]
fn generic_shift_grid_set_gtiff_valuesat_tiled_optim() {
    common::use_fixture_search_path();
    let set = GenericShiftGridSet::open("tests/nkgrf03vel_realigned_extract_tiled_256x256.tif")
        .expect("tiled gtiff set");
    assert!(set.grid_at(-100.0, -100.0).is_none());
    let grid = set
        .grid_at(21.3333333 / 180.0 * PI, 63.0 / 180.0 * PI)
        .expect("grid");
    assert_eq!(grid.width(), 5);
    assert_eq!(grid.height(), 5);
    assert!(grid.extent().is_geographic);
    assert_eq!(grid.extent().west, 21.0 / 180.0 * PI);
    assert!(!grid.is_null_grid());
    assert!(!grid.has_changed());
    assert_eq!(grid.value_at(0, 0, grid.samples_per_pixel()), None);
    assert_eq!(grid.value_at(0, 0, 0), Some(0.207_838_91_f32));
    assert_eq!(grid.value_at(1, 0, 0), Some(0.224_270_36_f32));
    assert_eq!(grid.value_at(0, 1, 0), Some(0.197_180_2_f32));
    let mut one = [0.0_f32; 1];
    assert!(grid.values_at(0, 0, 1, 1, &[0], &mut one).is_some());
    assert_eq!(one[0], 0.207_838_91_f32);

    // Three bands, then two bands from the same origin: every value matches
    // the single-pixel reads, which is the check the C++ test makes.
    for bands in [&[0_usize, 1, 2][..], &[0, 1][..]] {
        let count = 2 * 4 * bands.len();
        let mut values = vec![0.0_f32; count];
        assert!(grid.values_at(2, 1, 2, 4, bands, &mut values).is_some());
        let mut idx = 0;
        for y in 0..4 {
            for x in 0..2 {
                for &band in bands {
                    assert_eq!(grid.value_at(2 + x, 1 + y, band), Some(values[idx]));
                    idx += 1;
                }
            }
        }
    }
}

#[test]
fn generic_shift_grid_set_gtiff_with_subgrid() {
    common::use_fixture_search_path();
    let set = GenericShiftGridSet::open("tests/test_hgrid_with_subgrid.tif").expect("gtiff set");
    assert!(set.grid_at(-100.0, -100.0).is_none());
    let grid = set
        .grid_at(-115.5416667 / 180.0 * PI, 51.1666667 / 180.0 * PI)
        .expect("grid");
    assert_eq!(grid.width(), 11);
    assert_eq!(grid.height(), 21);
    assert_eq!(grid.metadata_item("grid_name"), Some("ALbanff"));
}

#[test]
fn generic_shift_grid_set_gtiff_with_two_level_of_subgrids_no_grid_name() {
    common::use_fixture_search_path();
    let set =
        GenericShiftGridSet::open("tests/test_hgrid_with_two_level_of_subgrids_no_grid_name.tif")
            .expect("gtiff set");
    assert!(set.grid_at(-100.0, -100.0).is_none());
    let grid = set
        .grid_at(-45.5 / 180.0 * PI, 22.5 / 180.0 * PI)
        .expect("grid");
    assert_eq!(grid.width(), 8);
    assert_eq!(grid.height(), 8);
}

#[test]
fn generic_shift_grid_set_gtiff_projected() {
    common::use_fixture_search_path();
    let set = GenericShiftGridSet::open("tests/test_3d_grid_projected.tif").expect("gtiff set");
    assert!(set.grid_at(-1000.0, -1000.0).is_none());
    let grid = set.grid_at(1_500_300.0, 5_400_300.0).expect("grid");
    assert_eq!(grid.width(), 2);
    assert_eq!(grid.height(), 2);
    let extent = grid.extent();
    assert!(!extent.is_geographic);
    assert_eq!(extent.west, 1_500_000.0);
    assert_eq!(extent.east, 1_501_000.0);
    assert_eq!(extent.south, 5_400_000.0);
    assert_eq!(extent.north, 5_401_000.0);
    assert_eq!(extent.res_x, 1000.0);
    assert_eq!(extent.res_y, 1000.0);
}
