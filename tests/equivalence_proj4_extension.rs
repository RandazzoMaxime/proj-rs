//! `SingleCRS::baseIsEquivalentTo` (crs.cpp:1700-1724) compares the PROJ4 extensions last: a CRS
//! with one and a CRS without one are not equivalent, under every criterion. Two CRS that both
//! have one are equivalent whatever they hold. GDAL's `test_osr_ct_lon_wrap` relies on it: an
//! equivalent target makes the transformation a no-op, and `+lon_wrap=180` is lost. Expected
//! values: C PROJ 9.9.0 `proj_is_equivalent_to`.

use proj_rs::iso19111::{create_crs_from_proj_string, AuthorityFactory, Crs, Equivalence};
use proj_rs::DatabaseContext;

const CRITERIA: [Equivalence; 3] = [
    Equivalence::Strict,
    Equivalence::Equivalent,
    Equivalence::ExceptAxisOrder,
];

fn crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("proj string")
}

fn equivalent_under_all(left: &str, right: &str) -> [bool; 3] {
    let (left, right) = (crs(left), crs(right));
    CRITERIA.map(|criterion| left.is_equivalent_to_with(&right, criterion))
}

const PLAIN: &str = "+proj=longlat +ellps=GRS80 +type=crs";
const WRAP_180: &str = "+proj=longlat +ellps=GRS80 +lon_wrap=180 +type=crs";

#[test]
fn one_extension_is_not_equivalent() {
    assert_eq!(equivalent_under_all(PLAIN, WRAP_180), [false; 3]);
    assert_eq!(equivalent_under_all(WRAP_180, PLAIN), [false; 3]);
    assert_eq!(
        equivalent_under_all(PLAIN, "+proj=longlat +ellps=GRS80 +over +type=crs"),
        [false; 3]
    );
    assert_eq!(
        equivalent_under_all(
            "+proj=geocent +ellps=GRS80 +type=crs",
            "+proj=geocent +ellps=GRS80 +lon_wrap=180 +type=crs"
        ),
        [false; 3]
    );
}

#[test]
fn two_extensions_are_equivalent_whatever_they_hold() {
    assert_eq!(equivalent_under_all(WRAP_180, WRAP_180), [true; 3]);
    assert_eq!(
        equivalent_under_all(WRAP_180, "+proj=longlat +ellps=GRS80 +lon_wrap=0 +type=crs"),
        [true; 3]
    );
}

/// `proj_is_equivalent_to_with_ctx` with a database, the path GDAL's `IsSame` takes, reaches
/// `baseIsEquivalentTo` too.
#[test]
fn the_database_path_compares_the_extension() {
    let db = DatabaseContext::create().expect("proj.db");
    let equivalent = |left: &str, right: &str| crs(left).is_equivalent_to_db(&crs(right), &db);
    assert!(!equivalent(PLAIN, WRAP_180));
    assert!(!equivalent(WRAP_180, PLAIN));
    assert!(equivalent(WRAP_180, WRAP_180));
    assert!(equivalent(PLAIN, PLAIN));
}

const WGS84_WRAP: &str = "+proj=longlat +datum=WGS84 +lon_wrap=180 +type=crs";

/// Under `EQUIVALENT_EXCEPT_AXIS_ORDER_GEOGCRS`, PROJ retries with the first CRS rebuilt on
/// swapped axes, without its extension (crs.cpp:3264-3290). A longitude-first CRS with an
/// extension then matches a latitude-first CRS without one, in that order only.
#[test]
fn the_axis_swap_retry_drops_the_extension_of_the_first_crs() {
    let e4326 = create_crs_from_proj_string("+proj=latlong +datum=WGS84 +axis=neu +type=crs")
        .expect("lat first");
    let wrap = crs(WGS84_WRAP);
    let wrap_lat_first = crs("+proj=latlong +datum=WGS84 +axis=neu +lon_wrap=180 +type=crs");
    let plain = crs("+proj=longlat +datum=WGS84 +type=crs");
    let except = |left: &Crs, right: &Crs| {
        CRITERIA.map(|criterion| left.is_equivalent_to_with(right, criterion))
    };
    assert_eq!(except(&wrap, &e4326), [false, false, true]);
    assert_eq!(except(&e4326, &wrap), [false; 3]);
    assert_eq!(except(&wrap, &plain), [false; 3]);
    assert_eq!(except(&wrap_lat_first, &plain), [false, false, true]);
    assert_eq!(except(&plain, &wrap_lat_first), [false; 3]);
}

/// `proj_identify` on a longitude-first CRS with `+lon_wrap`: only the latitude-first EPSG:4326
/// matches, through the axis swap, at 70 %, as C PROJ 9.9.0 gives it.
#[test]
fn identify_keeps_only_the_axis_swap_match() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "");
    let found = crs(WGS84_WRAP).identify_with(Some(&factory));
    let found: Vec<(String, String, i32)> = found
        .iter()
        .map(|(crs, confidence)| {
            let id = &crs.identifiers()[0];
            (
                id.code_space().to_owned(),
                id.code().to_owned(),
                *confidence,
            )
        })
        .collect();
    assert_eq!(found, [("EPSG".to_owned(), "4326".to_owned(), 70)]);
}
