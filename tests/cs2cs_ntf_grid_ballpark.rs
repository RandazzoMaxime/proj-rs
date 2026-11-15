//! NTF (Paris) to RGF93 with the NTv2 grid `data/tests/ntf_r93.gsb` on the search path, PROJ 9.9.0
//! (`cs2cs EPSG:4807 +to EPSG:2154`). PROJ names the grid `fr_ign_ntf_r93.tif`, absent here, and
//! finds `ntf_r93.gsb` through `grid_alternatives`. Its own binary: the search path is process
//! wide, and the other `cs2cs` candidate tests run with the database alone.

use proj_rs::iso19111::{select_alternatives, BestPolicy, CandidateFilter, Selection};
use proj_rs::Xyzt;

fn point(x: f64, y: f64, z: f64) -> Xyzt {
    Xyzt { x, y, z, t: 0.0 }
}

/// The grid covers 41 to 52 degrees north, so the point `43.2999763919 3.0333132353` (grads, about
/// 38.97 degrees north) lies outside it. PROJ then takes the candidate whose area of use is the
/// world, `Inverse of Transformation from ETRS89-FRA [RGF93 v1] to NTF (Paris) altered to use prime
/// meridian of ETRS89-FRA [RGF93 v1] + Ballpark geographic offset from NTF (Paris) altered ...`,
/// and prints `880453.0554161093 5764316.2342027659`. The port listed only the two France
/// candidates and returned `* * inf`.
#[test]
fn ntf_to_rgf93_takes_the_prime_meridian_ballpark_outside_the_grid() {
    std::env::set_var(
        "PROJ_DATA",
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data:",
            env!("CARGO_MANIFEST_DIR"),
            "/data/tests"
        ),
    );
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    let selection = select_alternatives(
        "EPSG:4807",
        "EPSG:2154",
        BestPolicy::Warn,
        CandidateFilter::NONE,
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("EPSG:4807 to EPSG:2154 has several candidates under the cs2cs default");
    };
    let coord = alternatives
        .forward(point(43.299_976_391_9, 3.033_313_235_3, 0.0))
        .expect("the world ballpark candidate serves a point outside the grid");
    assert!(
        (coord.x - 880_453.055_416_109_3).abs() < 1e-6,
        "x {}",
        coord.x
    );
    assert!(
        (coord.y - 5_764_316.234_202_77).abs() < 1e-6,
        "y {}",
        coord.y
    );
}
