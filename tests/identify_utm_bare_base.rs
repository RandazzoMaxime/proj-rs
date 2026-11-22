//! The UTM branch of `ProjectedCRS::identify` identifies a new latitude-longitude base CRS made of
//! the name and datum only (crs.cpp:4876-4885), so a `+proj=utm` CRS has one base candidate and
//! the hardcoded zone CRS, at 70 when its name is not equivalent. Expected values from
//! `ProjectedCRS::identify` of PROJ 9.9.0, for the authority `""` and `EPSG`.

use proj_rs::{create_crs_from_proj_string, AuthorityFactory, DatabaseContext};

fn identified(definition: &str, authority: &str) -> Vec<(String, String, i32)> {
    let db = DatabaseContext::create().expect("database");
    let factory = AuthorityFactory::create(&db, authority);
    let crs = create_crs_from_proj_string(definition).expect("PROJ string");
    crs.identify_with(Some(&factory))
        .into_iter()
        .map(|(found, confidence)| {
            let id = found.identifiers().first().expect("identifier");
            (found.name().to_owned(), id.code().to_owned(), confidence)
        })
        .collect()
}

fn single(name: &str, code: &str) -> Vec<(String, String, i32)> {
    vec![(name.to_owned(), code.to_owned(), 70)]
}

#[test]
fn utm_zones_on_the_hardcoded_datums() {
    for authority in ["", "EPSG"] {
        assert_eq!(
            identified("+proj=utm +zone=15 +type=crs", authority),
            single("WGS 84 / UTM zone 15N", "32615")
        );
        assert_eq!(
            identified("+proj=utm +zone=15 +datum=NAD83 +type=crs", authority),
            single("NAD83 / UTM zone 15N", "26915")
        );
        assert_eq!(
            identified("+proj=utm +zone=15 +datum=NAD27 +type=crs", authority),
            single("NAD27 / UTM zone 15N", "26715")
        );
        assert_eq!(
            identified(
                "+proj=utm +zone=31 +south +datum=WGS84 +type=crs",
                authority
            ),
            single("WGS 84 / UTM zone 31S", "32731")
        );
    }
}
