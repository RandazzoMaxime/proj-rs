//! The PROJ log lines of a transformation under the warn policy (`PROJ_DEBUG=2`). The expected
//! text is the stderr of PROJ 9.9.0's `cs2cs` on the same pair with every grid missing, the setup
//! of the `test_cs2cs_various.yaml` cases named in each test (`tmpdir` copies `proj.db` only). It
//! is never the output of the code under test.

use std::fs;
use std::path::PathBuf;

use proj_rs::iso19111::{
    best_pipeline_logged, select_alternatives, BestOperation, BestPolicy, CandidateFilter, LogLine,
    OnlyBest, Selection,
};
use proj_rs::Xyzt;

fn point(x: f64, y: f64, z: f64) -> Xyzt {
    Xyzt { x, y, z, t: 0.0 }
}

/// A data search path holding the database and no grid, so that every grid of the pairs below is
/// missing, as in the yaml cases. Set up once per test process: the tests run in parallel, and on
/// Windows a second copy over a `proj.db` another test has open fails with a sharing violation.
fn use_grid_free_search_path() {
    static SETUP: std::sync::Once = std::sync::Once::new();
    SETUP.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("proj-rs-proj-debug-log-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("scratch directory");
        let database = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/proj.db");
        fs::copy(&database, dir.join("proj.db")).expect("copy proj.db");
        std::env::set_var("PROJ_DATA", &dir);
        std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    });
}

/// `test_cs2cs_various.yaml#126`: a compound pair whose first candidate lacks its grids. PROJ
/// logs the failure once, at creation (`crs_to_crs.cpp:650`), and then the candidate its first use
/// takes (`trans.cpp:269-276`), which is the ballpark candidate.
#[test]
fn compound_pair_logs_the_creation_failure_and_the_first_use() {
    use_grid_free_search_path();
    let (best, log) = best_pipeline_logged(
        "EPSG:4326+5773",
        "EPSG:4326+5782",
        OnlyBest::Unset,
        CandidateFilter::NONE,
    )
    .expect("the pair has a best operation");
    assert!(
        matches!(best, BestOperation::Pipeline { .. }),
        "the ballpark candidate is the best one"
    );
    assert_eq!(
        log.creation,
        vec![LogLine::debug(
            "Attempt to use coordinate operation Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP [REGENTE] to Alicante height (1) using ETRS89-ESP [REGENTE] to WGS 84 (1) failed. Grid es_ign_egm08-rednap.tif is not available. Consult https://proj.org/resource_files.html for guidance. Grid us_nga_egm96_15.tif is not available. Consult https://proj.org/resource_files.html for guidance. This might become an error in a future PROJ major release. Set the ONLY_BEST option to YES or NO. This warning will no longer be emitted (for the current transformation instance).",
        )]
    );
    assert_eq!(
        log.first_use,
        Some(LogLine::debug(
            "Using coordinate operation Transformation from EGM96 height to Alicante height (ballpark vertical transformation)",
        ))
    );
}

/// `ONLY_BEST=YES` fails the creation through its own path, and the warning of the warn policy is
/// not logged at all.
#[test]
fn only_best_logs_no_creation_line() {
    use_grid_free_search_path();
    let (_, log) = best_pipeline_logged(
        "EPSG:4326+5773",
        "EPSG:4326+5782",
        OnlyBest::On,
        CandidateFilter::NONE,
    )
    .expect("the pair has an operation");
    assert!(log.creation.is_empty(), "creation {:?}", log.creation);
    assert!(log.first_use.is_none(), "first use {:?}", log.first_use);
}

/// `NAD27 to NAD83` at 40N 100W, grids missing. PROJ's first candidate is `NAD27 to NAD83 (7)`,
/// which fails: its retry lines and its warning come from `trans.cpp:257-298`. The warning is
/// logged once per instance, so the second point logs nothing, and the same candidate is used.
#[test]
fn per_point_retry_logs_once_per_instance() {
    use_grid_free_search_path();
    let selection = select_alternatives("NAD27", "NAD83", BestPolicy::Warn, CandidateFilter::NONE)
        .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("NAD27 to NAD83 has several candidates");
    };
    let mut first = Vec::new();
    let coord = alternatives
        .forward_logged(point(40.0, -100.0, 0.0), &mut first)
        .expect("a retry gives the point");
    assert!(coord.x.is_finite());
    assert_eq!(
        first,
        vec![
            LogLine::debug("Using coordinate operation NAD27 to NAD83 (7)"),
            LogLine::debug(
                "Attempt to use coordinate operation NAD27 to NAD83 (7) failed. Grid us_noaa_nadcon5_nad27_nad83_1986_conus.tif is not available. Consult https://proj.org/resource_files.html for guidance. This might become an error in a future PROJ major release. Set the ONLY_BEST option to YES or NO. This warning will no longer be emitted (for the current transformation instance).",
            ),
            LogLine::debug("No inverse operation"),
            LogLine::debug(
                "Did not result in valid result. Attempting a retry with another operation.",
            ),
            LogLine::debug(
                "Using coordinate operation NAD27 to WGS 84 (6) + Inverse of NAD83 to WGS 84 (1)",
            ),
        ]
    );
    let mut second = Vec::new();
    alternatives
        .forward_logged(point(40.0, -100.0, 0.0), &mut second)
        .expect("the same candidate gives the point again");
    assert!(second.is_empty(), "second point logged {second:?}");
}
