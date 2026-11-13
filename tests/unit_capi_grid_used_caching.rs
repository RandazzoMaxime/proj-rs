//! PROJ 9.9.0 `test/unit/test_c_api.cpp`,
//! `TEST_F(CApi, proj_coordoperation_get_grid_used_fullname_caching)` (lines 1775-1801).
//!
//! The C++ test checks a regression: the full name of a grid returned on the second pass of a
//! loop, after the first object was destroyed, must still be the file. The port builds the
//! object twice and reads its grids each time. The fixture grid is on the search path of the
//! test fixtures (`data/tests`).

mod common;

use proj_rs::Proj;

#[test]
fn capi_proj_coordoperation_get_grid_used_fullname_caching() {
    common::use_fixture_search_path();
    for _ in 0..2 {
        let proj =
            Proj::new("proj=vgridshift grids=tests/test_vgrid_int16.tif").expect("vgridshift");
        let grids = proj.grids_needed().expect("grids");
        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].short_name, "tests/test_vgrid_int16.tif");
        assert!(
            grids[0]
                .info
                .full_name
                .contains("tests/test_vgrid_int16.tif"),
            "{}",
            grids[0].info.full_name
        );
    }
}
