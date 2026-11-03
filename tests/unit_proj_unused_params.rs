//! PROJ 9.9.0 `src/iso19111/io.cpp`: `PROJStringParser::Private::hasUnusedParameters`
//! and its callers `buildGeodeticCRS`, `buildGeocentricCRS`, and `buildProjectedCRS`.
//! A single-step string whose CRS builder does not read a key keeps the whole
//! string as `EXTENSION_PROJ4`. The expected extension is the string as written,
//! with `+type=crs` at the end.

use proj_rs::{create_crs_from_proj_string, Crs, WktConvention, WktFormatter};

fn proj_crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("PROJ string")
}

fn wkt1(crs: &Crs) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    crs.export_to_wkt(&mut formatter).expect("WKT1")
}

/// The extension text `export_to_proj_string` returns for a CRS with an extension.
fn extension_of(definition: &str) -> String {
    proj_crs(definition).export_to_proj_string().expect("PROJ")
}

/// `buildGeodeticCRS`: `+lon_wrap` is an operation parameter the CRS builder does
/// not read, so the string is kept (`hasUnusedParameters`).
#[test]
fn longlat_lon_wrap_is_kept_as_extension() {
    let input = "+proj=longlat +ellps=WGS84 +lon_wrap=180 +type=crs";
    assert_eq!(extension_of(input), input);
    assert!(wkt1(&proj_crs(input))
        .contains("EXTENSION[\"PROJ4\",\"+proj=longlat +ellps=WGS84 +lon_wrap=180\"]"));
}

/// `buildGeodeticCRS`: `+lon_0` other than zero sets `EXTENSION_PROJ4` on a
/// geographic step, even when every key is read.
#[test]
fn longlat_nonzero_lon_0_is_kept_as_extension() {
    let input = "+proj=longlat +lon_0=10 +datum=WGS84 +type=crs";
    assert_eq!(extension_of(input), input);
}

/// `buildGeodeticCRS`: a zero `+lon_0` does not trigger the rule by itself.
#[test]
fn longlat_zero_lon_0_is_not_kept() {
    let input = "+proj=longlat +lon_0=0 +datum=WGS84 +type=crs";
    assert!(!wkt1(&proj_crs(input)).contains("EXTENSION"));
}

/// `hasUnusedParameters` runs on the keys `pj` recognizes. `+foo` is not one, so
/// it is dropped and does not make an extension (PROJ `io.projparse_longlat_wktext`).
#[test]
fn longlat_unrecognized_key_is_dropped() {
    let input = "+proj=longlat +foo=bar +datum=WGS84 +type=crs";
    assert!(!wkt1(&proj_crs(input)).contains("EXTENSION"));
    assert_eq!(
        extension_of("+proj=longlat +foo +wktext +type=crs"),
        "+proj=longlat +datum=WGS84 +no_defs +type=crs"
    );
}

/// `buildGeodeticCRS`: `+datum`, `+units`, and `+type` are all read, so the CRS
/// has no extension.
#[test]
fn longlat_used_keys_are_not_kept() {
    let input = "+proj=longlat +datum=WGS84 +units=m +no_defs +type=crs";
    assert!(!wkt1(&proj_crs(input)).contains("EXTENSION"));
}

/// `buildGeocentricCRS`: `+over` is recognized and not read, so the string is kept.
#[test]
fn geocent_over_is_kept_as_extension() {
    let input = "+proj=geocent +over +datum=WGS84 +type=crs";
    assert_eq!(extension_of(input), input);
}

/// `buildGeocentricCRS`: `+foo=bar` is dropped, so the export is the conversion
/// (PROJ `io.projparse_geocent_wktext`).
#[test]
fn geocent_unrecognized_key_is_dropped() {
    assert_eq!(
        extension_of("+proj=geocent +foo=bar +datum=WGS84 +type=crs"),
        "+proj=geocent +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

/// `buildGeocentricCRS` reads only the datum and `+units`. `+axis` is not read,
/// so the string is kept.
#[test]
fn geocent_axis_is_kept_as_extension() {
    let input = "+proj=geocent +axis=neu +datum=WGS84 +type=crs";
    assert_eq!(extension_of(input), input);
}

/// `buildProjectedCRS` reads the `tmerc` parameters and `+k`, so the string is not kept.
#[test]
fn tmerc_used_keys_are_not_kept() {
    let input =
        "+proj=tmerc +lat_0=0 +lon_0=3 +k=1 +x_0=0 +y_0=0 +ellps=WGS84 +units=m +no_defs +type=crs";
    assert!(!wkt1(&proj_crs(input)).contains("EXTENSION"));
}

/// `buildProjectedCRS`: `+lon_wrap` is recognized by PROJ and not read by the
/// `tmerc` builder, so the string is kept.
#[test]
fn tmerc_unread_recognized_key_is_kept_as_extension() {
    let input = "+proj=tmerc +lat_0=0 +lon_0=3 +k=1 +x_0=0 +y_0=0 +ellps=WGS84 +lon_wrap=180 +units=m +type=crs";
    assert_eq!(extension_of(input), input);
}

/// `buildProjectedCRS`: `+proj=peirce_q +shape=square` reads `shape` as the
/// flag of the method, so the string is not kept (PROJ `io.projparse_peirce_q_square`).
#[test]
fn peirce_q_shape_is_read_by_the_method() {
    let crs = proj_crs("+proj=peirce_q +shape=square +type=crs");
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ"),
        "+proj=peirce_q +shape=square +lat_0=90 +lon_0=0 +k_0=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

/// `buildProjectedCRS`: a polar `stere` reads `+k` in its polar branch.
#[test]
fn polar_stere_k_is_read() {
    let crs = proj_crs("+proj=stere +lat_0=90 +lat_ts=90 +k=1 +type=crs");
    assert!(!wkt1(&crs).contains("EXTENSION"));
}

/// `buildProjectedCRS`: `+k` is not read by the `utm` branch, which calls
/// `createUTM` and does not read the method parameters.
#[test]
fn utm_scale_is_kept_as_extension() {
    let input = "+proj=utm +zone=31 +k=0.9996 +ellps=WGS84 +units=m +type=crs";
    assert_eq!(extension_of(input), input);
}

/// `hasUnusedParameters`: `+over` is not read by the builder.
#[test]
fn over_is_kept_as_extension() {
    let input = "+proj=longlat +over +datum=WGS84 +type=crs";
    assert_eq!(extension_of(input), input);
}

/// `hasUnusedParameters` ignores `no_defs`.
#[test]
fn no_defs_alone_is_not_kept() {
    let input = "+proj=longlat +datum=WGS84 +no_defs +type=crs";
    assert!(!wkt1(&proj_crs(input)).contains("EXTENSION"));
}

/// `PROJStringParser`: a repeated key is removed before the check, so the second
/// `+datum` does not make an extension.
#[test]
fn duplicate_key_is_dropped() {
    let input = "+proj=longlat +datum=WGS84 +datum=WGS84 +type=crs";
    assert!(!wkt1(&proj_crs(input)).contains("EXTENSION"));
}
