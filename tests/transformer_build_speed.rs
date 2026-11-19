//! Building a `Transformer` between `EPSG:6364` and `EPSG:8801` with the grids of the parity corpus
//! in the search path. The selection finds 34 candidates, most of them on the NAD83 and geoid grids
//! of tens of megabytes. Built at once, they took 11.8 s in a dev build under the default policy
//! and 5.9 s under `only_best` off, almost all of it decoding grids: a candidate is now built at
//! its first use, and the build reads none of them.
//!
//! The expected values are PROJ 9.9.0's: `cs2cs -d 12 EPSG:6364 +to EPSG:8801` on
//! `33.917795307 -99.466886035 3768.919` prints `3797.495905946656` with `only_best` off
//! (`tests/parity/grids/expected.jsonl`, `cs2cs-grid-out-geoid_like-0024`).
//!
//! The limit is a regression guard, not a benchmark: the builds take about 0.5 s in a dev build.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use proj_rs::{Context, Crs, Network, OnlyBest, Point, Transformer};

/// The longest build accepted, in a dev build.
const BUILD_LIMIT: Duration = Duration::from_secs(3);

fn grids_dir() -> PathBuf {
    let dir = std::env::var("PARITY_GRIDS_DIR").expect(
        "PARITY_GRIDS_DIR absent : tools/parity/fetch-grids.sh <dossier>, puis \
         PARITY_GRIDS_DIR=<dossier> cargo test --test transformer_build_speed -- --ignored",
    );
    PathBuf::from(dir)
}

/// The transformer of the pair under `only_best`, and the time its build took.
fn timed_build(ctx: &Context, only_best: OnlyBest) -> (Transformer, Duration) {
    let source = Crs::from_user_input(ctx, "EPSG:6364").expect("source");
    let target = Crs::from_user_input(ctx, "EPSG:8801").expect("target");
    let started = Instant::now();
    let transformer = Transformer::builder(&source, &target)
        .only_best(only_best)
        .build()
        .expect("the pair builds");
    (transformer, started.elapsed())
}

#[test]
#[ignore = "corpus avec grilles : PARITY_GRIDS_DIR=... cargo test --test transformer_build_speed -- --ignored"]
fn building_a_pair_with_geoid_grids_takes_seconds_and_gives_proj_values() {
    let ctx = Context::builder()
        .search_path(grids_dir())
        .network(Network::Disabled)
        .build();

    for only_best in [OnlyBest::Default, OnlyBest::No] {
        let (_, elapsed) = timed_build(&ctx, only_best);
        assert!(
            elapsed < BUILD_LIMIT,
            "the build under {only_best:?} took {elapsed:?}, the limit is {BUILD_LIMIT:?}"
        );
    }

    let (transformer, _) = timed_build(&ctx, OnlyBest::No);
    // The first point reads the grid of the candidate it uses, after the build.
    let p = transformer
        .forward(Point::xyz(33.917_795_307, -99.466_886_035, 3_768.919))
        .expect("a point of the grids");
    assert!((p.x + 1_065_171.068_292_7).abs() < 5e-4, "x {}", p.x);
    assert!((p.y - 463_942.607_341_9).abs() < 5e-4, "y {}", p.y);
    assert!(
        (p.z - 3_797.495_905_946_656).abs() < 5e-4,
        "z {} (PROJ 9.9.0: 3797.495905946656)",
        p.z
    );
}
