//! `+proj=cea` with a scale `k` and no `+lat_ts`: PROJ 9.9.0 computes the standard parallel with
//! the closed form of io.cpp, from the squared eccentricity of the ellipsoid even under `+R_A`.
//! Expected values from `projinfo -o WKT2_2019` of PROJ 9.9.0.

use proj_rs::{create_crs_from_proj_string, WktConvention, WktFormatter, WktLines};

fn standard_parallel(definition: &str) -> String {
    let crs = create_crs_from_proj_string(definition).expect("PROJ string");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.set_lines(WktLines::Single);
    let wkt = crs.export_to_wkt(&mut formatter).expect("WKT2");
    let key = "PARAMETER[\"Latitude of 1st standard parallel\",";
    let start = wkt.find(key).expect("standard parallel") + key.len();
    let value = &wkt[start..];
    value[..value.find(',').expect("unit")].to_owned()
}

#[test]
fn scale_one_is_the_equator() {
    assert_eq!(
        standard_parallel("+proj=cea +k=1 +ellps=GRS80 +type=crs"),
        "0"
    );
    assert_eq!(
        standard_parallel("+proj=cea +k=1 +R=6371000 +type=crs"),
        "0"
    );
}

#[test]
fn scale_below_one_on_the_ellipsoid() {
    assert_eq!(
        standard_parallel("+proj=cea +k_0=0.99 +ellps=GRS80 +type=crs"),
        "8.13653121990374"
    );
    assert_eq!(
        standard_parallel("+proj=cea +k=0.9 +ellps=GRS80 +type=crs"),
        "25.9174996921819"
    );
}

#[test]
fn authalic_sphere_keeps_the_ellipsoid_eccentricity() {
    assert_eq!(
        standard_parallel("+proj=cea +k=0.9 +R_A +ellps=GRS80 +type=crs"),
        "25.9174996921819"
    );
}

#[test]
fn scale_above_one_is_refused() {
    let error = create_crs_from_proj_string("+proj=cea +k=2 +ellps=GRS80 +type=crs")
        .expect_err("k above 1");
    assert!(
        error.to_string().contains("k/k_0 should be in [0,1]"),
        "{error}"
    );
}
