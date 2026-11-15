//! `cs2cs EPSG:6364 +to EPSG:8801` at `33.917795307 -99.466886035` (Texas), with the grids of
//! `tests/parity/grids/grids.txt` in `PARITY_GRIDS_DIR`. The reference reads `proj.ini`
//! (`only_best_default = off`): it drops the candidates whose grids are missing and prints
//! `3797.495905946656` with `us_noaa_g2018u0.tif` (`tests/parity/grids/expected.jsonl`,
//! `cs2cs-grid-out-geoid_like-0024`). Under PROJ's built-in Warn policy it prints `3797.540472221235`.

use proj_rs::iso19111::{select_alternatives, BestPolicy, CandidateFilter, Selection};
use proj_rs::Xyzt;

fn point(x: f64, y: f64, z: f64) -> Xyzt {
    Xyzt { x, y, z, t: 0.0 }
}

#[test]
#[ignore = "corpus avec grilles : PARITY_GRIDS_DIR=... cargo test --test cs2cs_grid_geoid_tie -- --ignored"]
fn geoid_tie_takes_the_g2018_candidate_as_proj_does() {
    let grids = std::env::var("PARITY_GRIDS_DIR")
        .expect("PARITY_GRIDS_DIR absent : tools/parity/fetch-grids.sh <dossier>");
    std::env::set_var(
        "PROJ_DATA",
        format!(
            "{}/data:{}/data/tests:{grids}",
            env!("CARGO_MANIFEST_DIR"),
            env!("CARGO_MANIFEST_DIR")
        ),
    );
    std::env::set_var("PROJ_NETWORK", "OFF");
    std::env::set_var("PROJ_ONLY_BEST_DEFAULT", "NO");
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    let selection = select_alternatives(
        "EPSG:6364",
        "EPSG:8801",
        BestPolicy::Discard,
        CandidateFilter::NONE,
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("EPSG:6364 to EPSG:8801 has several candidates under the cs2cs default");
    };
    let coord = alternatives
        .forward(point(33.917_795_307, -99.466_886_035, 3_768.919))
        .expect("a candidate serves the point");
    assert!(
        (coord.x + 1_065_171.068_292_7).abs() < 5e-4,
        "x {}",
        coord.x
    );
    assert!((coord.y - 463_942.607_341_9).abs() < 5e-4, "y {}", coord.y);
    assert!(
        (coord.z - 3_797.495_906).abs() < 5e-4,
        "z {} (Warn policy: 3797.540472221235)",
        coord.z
    );
}
