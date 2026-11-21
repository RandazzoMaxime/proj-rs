//! The `ONLY_BEST` and `AUTHORITY` options of `proj_create_crs_to_crs`
//! (`crs_to_crs.cpp:505-541`, `trans.cpp:285-291`). Expected values: `proj_create_crs_to_crs_from_pj`
//! and `proj_trans` of C PROJ 9.9.0 on the same search path, where `fr_ign_gr3df97a.tif` is missing.

mod common;

use proj_rs::unstable::{Direction, PairError, Proj, ProjCreateError};
use proj_rs::Xyzt;

const NTF_POINT: Xyzt = Xyzt {
    x: 2.0,
    y: 49.0,
    z: 0.0,
    t: 0.0,
};

fn ntf_to_rgf93(options: &[&str]) -> Result<Proj, ProjCreateError> {
    common::use_fixture_search_path();
    let source = Proj::new("EPSG:4275").expect("NTF");
    let target = Proj::new("EPSG:4171").expect("RGF93");
    source.create_crs_to_crs_from_pj(&target, None, Some(options.to_vec()))
}

/// `ONLY_BEST=YES`: the best operation needs the missing grid, so the point fails with
/// `PROJ_ERR_COORD_TRANSFM_NO_OPERATION`, and no other operation is tried.
#[test]
fn only_best_yes_fails_on_the_missing_grid() {
    let transform = ntf_to_rgf93(&["ONLY_BEST=YES"]).expect("created");
    let mut log = Vec::new();
    let result = transform.trans_coord_logged(NTF_POINT, Direction::Forward, &mut log);
    assert_eq!(result, Err(PairError::NoOperation));
    let texts: Vec<&str> = log.iter().map(|line| line.text.as_str()).collect();
    assert!(
        texts.contains(
            &"Attempt to use coordinate operation NTF to ETRS89-FRA [RGF93 v1] (1) failed. Grid \
              fr_ign_gr3df97a.tif is not available. Consult \
              https://proj.org/resource_files.html for guidance."
        ),
        "{texts:?}"
    );
}

/// `ONLY_BEST=NO` and `AUTHORITY=EPSG`: an operation without the grid gives the point.
#[test]
fn only_best_no_and_authority_use_another_operation() {
    for options in [&["ONLY_BEST=NO"][..], &["AUTHORITY=EPSG"][..]] {
        let transform = ntf_to_rgf93(options).expect("created");
        let done = transform
            .trans_coord(NTF_POINT, Direction::Forward)
            .expect("transformed");
        assert!(
            (done.x - 1.999_277_684_775).abs() < 1e-9,
            "{options:?} x {}",
            done.x
        );
        assert!(
            (done.y - 48.999_932_260_000).abs() < 1e-9,
            "{options:?} y {}",
            done.y
        );
    }
}

#[test]
fn bad_options_are_refused_with_proj_messages() {
    for (options, message) in [
        (
            &["ONLY_BEST=maybe"][..],
            "Invalid value for ONLY_BEST option.",
        ),
        (&["FOO=1"][..], "Unknown option :FOO=1"),
    ] {
        match ntf_to_rgf93(options) {
            Err(ProjCreateError::ProjError(text)) => assert_eq!(text, message),
            Err(other) => panic!("{options:?}: {other:?}"),
            Ok(_) => panic!("{options:?} accepted"),
        }
    }
}
