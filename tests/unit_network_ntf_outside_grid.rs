//! `cs2cs -d 10 EPSG:4807 +to EPSG:4171` at `60 10`, network on, PROJ 9.9.0: the point is outside
//! the grids of NTF to RGF93, which the network makes available. PROJ then takes the candidate
//! `Transformation from NTF (Paris) to NTF (Paris) altered to use prime meridian of ETRS89-FRA
//! [RGF93 v1] + Ballpark geographic offset ...`, with the world as area of use, and prints
//! `54.0000000000 11.3372291667`. The port listed only the two grid candidates and printed `* * inf`.
//!
//! The reference value is `cs2cs` 9.9.0 with `PROJ_NETWORK=ON` and `PROJ_DATA` set to the database
//! directory only (no local `ntf_r93.gsb`). It needs the CDN, as `tests/net_cdn`.

use std::env;

#[path = "net_cdn/mod.rs"]
mod net_cdn;
use net_cdn::cdn_reachable;

use proj_rs::iso19111::{select_alternatives, BestPolicy, CandidateFilter, Selection};
use proj_rs::Xyzt;

#[test]
fn ntf_to_rgf93_outside_the_grids_takes_the_ballpark_with_network() {
    if !cdn_reachable() {
        eprintln!(
            "ntf_to_rgf93_outside_the_grids: the CDN does not answer, the case is not checked"
        );
        return;
    }
    env::set_var("PROJ_DATA", concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
    env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    env::set_var("PROJ_NETWORK", "ON");
    env::set_var(
        "PROJ_USER_WRITABLE_DIRECTORY",
        env::temp_dir().join(format!("proj-rs-ntf-outside-{}", std::process::id())),
    );
    let selection = select_alternatives(
        "EPSG:4807",
        "EPSG:4171",
        BestPolicy::Warn,
        CandidateFilter::NONE,
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("EPSG:4807 to EPSG:4171 has several candidates under the cs2cs default");
    };
    let coord = alternatives
        .forward(Xyzt {
            x: 60.0,
            y: 10.0,
            z: 0.0,
            t: 0.0,
        })
        .expect("the world ballpark candidate serves a point outside the grids");
    assert!((coord.x - 54.0).abs() < 1e-9, "lat {}", coord.x);
    assert!((coord.y - 11.337_229_166_7).abs() < 1e-9, "lon {}", coord.y);
}
