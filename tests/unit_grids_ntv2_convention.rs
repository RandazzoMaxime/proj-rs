//! PROJ `grids.cpp` `valueAt(x, y, compensateNTConvention, lon, lat)`:
//! for NTv2 the flag negates the longitude shift and leaves the latitude
//! (`NTv2Grid::valueAt`, `grids.cpp` line 2173). The GTiff reader ignores the
//! flag (`GTiffHGrid::valueAt`, `bool` unnamed).

use proj_rs::grids::{HorizontalShiftGridSet, NtConvention};

mod common;

#[test]
fn ntv2_compensated_negates_longitude_only() {
    common::use_fixture_search_path();
    let set = HorizontalShiftGridSet::open("tests/test_hgrid_big_endian.gsb").expect("ntv2 set");
    assert_eq!(set.format(), "ntv2");
    let extent = set.grids()[0].extent();
    let lon = (extent.west + extent.east) / 2.0;
    let lat = (extent.south + extent.north) / 2.0;
    let grid = set.grid_at(lon, lat).expect("grid at the centre");
    let (w, h) = (grid.width(), grid.height());
    for (x, y) in [(0, 0), (w - 1, h - 1), (w / 2, h / 2)] {
        let plain = grid
            .horizontal_value_at(x, y, NtConvention::Plain)
            .expect("plain value");
        let compensated = grid
            .horizontal_value_at(x, y, NtConvention::Compensated)
            .expect("compensated value");
        assert_eq!(compensated.0, -plain.0, "longitude at ({x}, {y})");
        assert_eq!(compensated.1, plain.1, "latitude at ({x}, {y})");
        assert_eq!(plain, grid.horizontal_shift(x, y).expect("shift"));
    }
}

#[test]
fn gtiff_ignores_the_convention() {
    common::use_fixture_search_path();
    let set = HorizontalShiftGridSet::open("tests/test_hgrid.tif").expect("gtiff set");
    let grid = set
        .grid_at(
            5.5 / 180.0 * std::f64::consts::PI,
            53.5 / 180.0 * std::f64::consts::PI,
        )
        .expect("grid");
    let plain = grid
        .horizontal_value_at(0, 3, NtConvention::Plain)
        .expect("plain");
    let compensated = grid
        .horizontal_value_at(0, 3, NtConvention::Compensated)
        .expect("compensated");
    assert_eq!(plain, compensated);
}
