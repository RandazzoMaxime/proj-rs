//! PROJ 9.9.0 `test/unit/test_c_api.cpp`, `proj_create_cartesian_2D_cs`: each of the five
//! 2D cartesian axis orders is created in metres.

use proj_rs::{CartesianCS, UnitOfMeasure};

/// `TEST_F(CApi, proj_create_cartesian_2D_cs)`
#[test]
fn capi_create_cartesian_2d_cs() {
    let metre = UnitOfMeasure::metre;
    for cs in [
        CartesianCS::create_easting_northing(metre()),
        CartesianCS::create_northing_easting(metre()),
        CartesianCS::create_north_pole_easting_south_northing_south(metre()),
        CartesianCS::create_south_pole_easting_north_northing_north(metre()),
        CartesianCS::create_westing_southing(metre()),
    ] {
        assert_eq!(cs.axes().len(), 2);
    }
}
