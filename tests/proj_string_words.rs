//! The words of a PROJ string, as PROJ 9.9.0's `pj_shrink` and `pj_trim_argv` cut them: a `+` is
//! optional before a word, kept inside a number (`1e+06`), and blanks around `=` and `,` vanish.
//! Expected strings: `projinfo -o PROJ -q "<text>"` of PROJ 9.9.0.

use proj_rs::{create_from_user_input, DatabaseContext};

fn proj_of(text: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    create_from_user_input(text, &db)
        .unwrap_or_else(|error| panic!("{text}: {error}"))
        .export_to_proj_string()
        .expect("PROJ string")
}

#[test]
fn words_without_plus_build_the_same_crs() {
    let expected = "+proj=longlat +datum=WGS84 +no_defs +type=crs";
    assert_eq!(proj_of("proj=lonlat type=crs"), expected);
    assert_eq!(proj_of("+proj=lonlat +type=crs"), expected);
}

#[test]
fn plus_inside_a_number_stays() {
    let expected = "+proj=utm +zone=31 +x_0=1e+06 +ellps=GRS80 +type=crs";
    assert_eq!(
        proj_of("proj=utm zone=31 x_0=1e+06 ellps=GRS80 type=crs"),
        expected
    );
    assert_eq!(
        proj_of("+proj=utm +zone=31 +x_0 = 1e+06 +ellps=GRS80 +type=crs"),
        expected
    );
}

#[test]
fn title_with_blanks_is_one_word() {
    assert_eq!(
        proj_of("+title=Mon CRS +proj=lonlat +type=crs"),
        "+proj=longlat +datum=WGS84 +no_defs +type=crs"
    );
}

#[test]
fn blanks_after_commas_vanish() {
    assert_eq!(
        proj_of("+proj=lonlat +towgs84=1, 2, 3 +ellps=GRS80 +type=crs"),
        "+proj=longlat +ellps=GRS80 +towgs84=1,2,3,0,0,0,0 +no_defs +type=crs"
    );
}
