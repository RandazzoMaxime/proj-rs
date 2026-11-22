//! A PROJ string CRS whose scale factor is not positive, or whose `lat_0` is beyond the pole, is
//! refused at creation: PROJ's `createFromPROJString` instantiates the string (io.cpp:12799) and
//! `pj_init` refuses it (init.cpp:653-678). Expected texts: `projinfo -o PROJ` of PROJ 9.9.0.

use proj_rs::{create_from_user_input, DatabaseContext};

fn refusal(text: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    match create_from_user_input(text, &db) {
        Ok(_) => format!("accepted: {text}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn a_scale_factor_not_above_zero_is_refused() {
    for (text, step) in [
        ("+proj=tmerc +k=0 +type=crs", "tmerc"),
        ("+proj=tmerc +k_0=-2 +type=crs", "tmerc"),
        ("+proj=utm +zone=31 +k=0 +type=crs", "utm"),
        ("+proj=longlat +k=0 +type=crs", "longlat"),
    ] {
        assert_eq!(
            refusal(text),
            format!(
                "Error 1027 (Invalid value for an argument): {step}: Invalid value for k/k_0: it should be > 0"
            ),
            "{text}"
        );
    }
}

#[test]
fn a_latitude_of_origin_beyond_the_pole_is_refused() {
    assert_eq!(
        refusal("+proj=tmerc +lat_0=100 +type=crs"),
        "Error 1027 (Invalid value for an argument): tmerc: Invalid value for lat_0: |lat_0| should be <= 90°"
    );
}

#[test]
fn a_projection_check_of_lat_0_keeps_its_own_text() {
    // `+proj=omerc +R=1 +alpha=0 +lat_0=90`: omerc refuses `|lat_0| = 90`, not `pj_init`.
    let text = refusal("+proj=omerc +R=1 +alpha=0 +lat_0=90 +type=crs");
    assert!(!text.contains("|lat_0| should be <= 90°"), "{text}");
}

#[test]
fn a_valid_scale_factor_is_kept() {
    let db = DatabaseContext::create().expect("proj.db");
    for text in [
        "+proj=tmerc +k=0.9996 +type=crs",
        "+proj=tmerc +lat_0=90 +type=crs",
        "+proj=longlat +type=crs",
    ] {
        assert!(create_from_user_input(text, &db).is_ok(), "{text}");
    }
}
