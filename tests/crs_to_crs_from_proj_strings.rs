//! `create_crs_to_crs_from_pj` between CRS written as PROJ strings with `+type=crs`. PROJ 9.9.0's
//! `proj_create` reads such a string as a CRS, and `proj_create_crs_to_crs_from_pj` takes it;
//! pyproj builds its `Transformer.from_proj` this way. Expected values: the same calls on PROJ
//! 9.9.0's C library.

use proj_rs::Proj;

fn convert(source: &str, target: &str) -> (f64, f64) {
    let from = Proj::new(source).expect(source);
    let to = Proj::new(target).expect(target);
    let transform = from
        .create_crs_to_crs_from_pj(&to, None, None)
        .unwrap_or_else(|error| panic!("{source} -> {target}: {error}"));
    transform.convert((2.0, 49.0)).expect("point")
}

fn assert_near(got: (f64, f64), expected: (f64, f64)) {
    assert!(
        (got.0 - expected.0).abs() <= 1e-6 && (got.1 - expected.1).abs() <= 1e-6,
        "{got:?} vs {expected:?}"
    );
}

#[test]
fn proj_string_crs_on_both_sides() {
    assert_near(
        convert(
            "+proj=longlat +datum=WGS84 +type=crs",
            "+proj=utm +zone=31 +ellps=GRS80 +towgs84=0,0,0 +type=crs",
        ),
        (426_857.987_717, 5_427_937.523_342),
    );
    assert_near(
        convert(
            "+proj=longlat +datum=WGS84 +type=crs",
            "+proj=utm +zone=31 +ellps=intl +towgs84=-87,-98,-121 +type=crs",
        ),
        (426_950.721_789, 5_428_145.637_260),
    );
}

#[test]
fn an_authority_crs_and_a_proj_string_crs() {
    let from = Proj::new("EPSG:32631").expect("EPSG:32631");
    let to = Proj::new("+proj=longlat +datum=WGS84 +type=crs").expect("longlat");
    let transform = from
        .create_crs_to_crs_from_pj(&to, None, None)
        .expect("EPSG:32631 -> longlat");
    let (lon, lat): (f64, f64) = transform
        .convert((426_857.987_717, 5_427_937.523_342))
        .expect("point");
    assert!(
        (lon - 2.000_000_000_018).abs() <= 1e-11 && (lat - 48.999_999_998_894).abs() <= 1e-11,
        "{lon} {lat}"
    );
}

#[test]
fn a_proj_string_without_type_crs_is_still_an_operation() {
    let from = Proj::new("+proj=longlat +datum=WGS84").expect("operation");
    let to = Proj::new("EPSG:4326").expect("crs");
    assert!(from.create_crs_to_crs_from_pj(&to, None, None).is_err());
}
