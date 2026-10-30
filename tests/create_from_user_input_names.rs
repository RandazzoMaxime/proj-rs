//! `createFromUserInput` for names PROJ matches approximately, and for a
//! non-EPSG authority code that `cs2cs` takes.

use proj_rs::{create_from_user_input, DatabaseContext};

#[test]
fn itrf1993_matches_itrf93_through_the_19dd_rule() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input("ITRF1993", &db).expect("ITRF1993");
    assert_eq!(crs.name(), "ITRF93");
    assert_eq!(
        crs.authority_code("EPSG").map(|id| id.code().to_owned()),
        Some("8993".to_owned())
    );
}

#[test]
fn approximate_name_longer_than_twice_the_text_is_rejected() {
    let db = DatabaseContext::create().expect("proj.db");
    assert!(create_from_user_input("EGM2008", &db).is_err());
}

#[test]
fn iau_authority_code_exports_its_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input("IAU_2015:49935", &db).expect("IAU_2015:49935");
    let text = crs.export_to_proj_string().expect("PROJ string");
    assert!(text.starts_with("+proj=stere +lat_0=-90 "), "{text}");
    assert!(text.contains("+R=3396190"), "{text}");
}
