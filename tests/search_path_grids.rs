//! The grids of an operation are looked for on the search paths of the context.
//!
//! PROJ 9.9.0 `proj_coordoperation_get_grid_used` and `proj_coordoperation_is_instantiable`
//! (c_api.cpp:7552, 7925) open the grid files through the context, whose search paths
//! `proj_context_set_search_paths` replaces (pyproj's `append_data_dir`). Measured with libproj
//! 9.9.0 and the paths `[<proj.db dir>, data/tests]`, no `PROJ_DATA`: `EPSG:1312` (NAD27 to NAD83
//! (3)) uses `ca_nrc_ntv1_can.tif`, available as `data/tests/ntv1_can.dat`, and is instantiable
//! (`capi/tests/run.sh search_path_grids`).

use std::path::PathBuf;

use proj_rs::iso19111::{AlternativeGridNames, AuthorityFactory, DatabaseContext};
use proj_rs::{GridAvailabilityUse, ProjBuilder};

fn builder(path: PathBuf) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder.replace_search_paths(vec![path]);
    builder
}

#[test]
fn grids_are_looked_for_on_the_search_paths_of_the_builder() {
    let db = DatabaseContext::create().expect("proj.db");
    let op = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("1312", AlternativeGridNames::Proj)
        .expect("EPSG:1312");
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/tests");
    let found = builder(tests);
    let grids = found
        .grids_needed_at(&op, &db, GridAvailabilityUse::UseForSorting, "")
        .expect("grids");
    assert_eq!(grids.len(), 1);
    assert_eq!(grids[0].short_name, "ca_nrc_ntv1_can.tif");
    assert!(grids[0].info.available, "{:?}", grids[0].info);
    assert!(
        grids[0].info.full_name.ends_with("/ntv1_can.dat"),
        "{}",
        grids[0].info.full_name
    );
    assert!(found.is_proj_instantiable(&op, &db, GridAvailabilityUse::UseForSorting));

    let empty = std::env::temp_dir().join(format!("proj-rs-no-grids-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("empty directory");
    let missing = builder(empty.clone());
    let grids = missing
        .grids_needed_at(&op, &db, GridAvailabilityUse::UseForSorting, "")
        .expect("grids");
    assert!(!grids[0].info.available, "{:?}", grids[0].info);
    assert!(!missing.is_proj_instantiable(&op, &db, GridAvailabilityUse::UseForSorting));
    let _gone = std::fs::remove_dir(&empty);
}
