//! PROJ `test/unit/test_io.cpp` `io.projparse_*` tests for projection families:
//! UTM and transverse Mercator, Lambert azimuthal equal area, equidistant
//! cylindrical, orthographic, Peirce quincuncial. Lot io-6. The PROJ strings and
//! expected text are copied from PROJ 9.9.0, without change.

use proj_rs::{create_crs_from_proj_string, Crs, WktConvention, WktFormatter, WktLines};

fn proj_crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("PROJ string")
}

/// WKT2 on one line with identifiers simulated, as the PROJ test formats it.
fn wkt2_single(crs: &Crs) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.simul_cur_node_has_id();
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

fn proj_string(crs: &Crs) -> String {
    crs.export_to_proj_string().expect("PROJ string")
}

/// The EPSG code of the first `METHOD[...]` in `wkt`, if it has one.
fn method_epsg_code(wkt: &str) -> Option<i32> {
    let rest = &wkt[wkt.find("METHOD[")?..];
    let id = "ID[\"EPSG\",";
    let digits = &rest[rest.find(id)? + id.len()..];
    let digits: String = digits.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[test]
fn io_projparse_utm() {
    let wkt = wkt2_single(&proj_crs("+proj=utm +zone=1 +type=crs"));
    assert!(
        wkt.contains(
            "CONVERSION[\"UTM zone 1N\",METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]]"
        ),
        "{wkt}"
    );
    assert!(
        wkt.contains("\"Longitude of natural origin\",-177,"),
        "{wkt}"
    );
    assert!(wkt.contains("\"False northing\",0,"), "{wkt}");
}

#[test]
fn io_projparse_utm_south() {
    let wkt = wkt2_single(&proj_crs("+proj=utm +zone=1 +south +type=crs"));
    assert!(
        wkt.contains(
            "CONVERSION[\"UTM zone 1S\",METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]]"
        ),
        "{wkt}"
    );
    assert!(
        wkt.contains("\"Longitude of natural origin\",-177,"),
        "{wkt}"
    );
    assert!(wkt.contains("\"False northing\",10000000,"), "{wkt}");
}

#[test]
fn io_projparse_etmerc() {
    let crs = proj_crs("+proj=etmerc +type=crs");
    let wkt = wkt2_single(&crs);
    assert!(
        wkt.contains("METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]]"),
        "{wkt}"
    );
    assert_eq!(
        proj_string(&crs),
        "+proj=tmerc +lat_0=0 +lon_0=0 +k=1 +x_0=0 +y_0=0 \
         +datum=WGS84 +units=m +no_defs +type=crs"
    );
    let mut wkt1 = WktFormatter::create(WktConvention::Wkt1Gdal);
    let wkt1 = crs.export_to_wkt(&mut wkt1).expect("WKT1");
    assert!(!wkt1.contains("EXTENSION[\"PROJ4\""), "{wkt1}");
}

#[test]
fn io_projparse_laea_north_pole() {
    let wkt = wkt2_single(&proj_crs("+proj=laea +lat_0=90 +type=crs"));
    assert!(wkt.contains("AXIS[\"(E)\",south"), "{wkt}");
    assert!(wkt.contains("AXIS[\"(N)\",south"), "{wkt}");
}

#[test]
fn io_projparse_laea_south_pole() {
    let wkt = wkt2_single(&proj_crs("+proj=laea +lat_0=-90 +type=crs"));
    assert!(wkt.contains("AXIS[\"(E)\",north"), "{wkt}");
    assert!(wkt.contains("AXIS[\"(N)\",north"), "{wkt}");
}

#[test]
fn io_projparse_laea_ellipsoidal() {
    let wkt = wkt2_single(&proj_crs("+proj=laea +ellps=WGS84 +type=crs"));
    assert_eq!(method_epsg_code(&wkt), Some(9820), "{wkt}");
}

#[test]
fn io_projparse_laea_spherical_on_ellipsoid() {
    let input = "+proj=laea +R_A +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
                 +ellps=WGS84 +units=m +no_defs +type=crs";
    let crs = proj_crs(input);
    let wkt = wkt2_single(&crs);
    assert_eq!(method_epsg_code(&wkt), Some(1027), "{wkt}");
    assert_eq!(proj_string(&crs), input);
}

/// Not a PROJ `TEST()`: the method choice that `io.projparse_laea_spherical` and
/// `io.projparse_eqc_spherical` check before their equivalence assertions. A
/// sphere (`+R=`) takes the spherical EPSG method; an ellipsoid does not.
#[test]
fn io_projfam_sphere_selects_spherical_method() {
    let laea = wkt2_single(&proj_crs("+proj=laea +R=6371228 +type=crs"));
    assert_eq!(method_epsg_code(&laea), Some(1027), "{laea}");
    let eqc = wkt2_single(&proj_crs("+proj=eqc +R=6371228 +type=crs"));
    assert_eq!(method_epsg_code(&eqc), Some(1029), "{eqc}");
    let laea_ellipsoid = wkt2_single(&proj_crs("+proj=laea +ellps=WGS84 +type=crs"));
    assert_eq!(method_epsg_code(&laea_ellipsoid), Some(9820));
}

#[test]
fn io_projparse_eqc_ellipsoidal() {
    let wkt = wkt2_single(&proj_crs("+proj=eqc +ellps=WGS84 +type=crs"));
    assert_eq!(method_epsg_code(&wkt), Some(1028), "{wkt}");
}

#[test]
fn io_projparse_ortho_ellipsoidal() {
    let input = "+proj=ortho +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
                 +ellps=WGS84 +units=m +no_defs +type=crs";
    let crs = proj_crs(input);
    let wkt = wkt2_single(&crs);
    assert_eq!(method_epsg_code(&wkt), Some(9840), "{wkt}");
    assert_eq!(proj_string(&crs), input);
}

#[test]
fn io_projparse_ortho_with_alpha() {
    let input = "+proj=ortho +lat_0=0 +lon_0=0 +alpha=12 +k=1 +x_0=0 \
                 +y_0=0 +ellps=WGS84 +units=m +no_defs +type=crs";
    let crs = proj_crs(input);
    let wkt = wkt2_single(&crs);
    // EPSG_CODE_METHOD_LOCAL_ORTHOGRAPHIC: EPSG 1130 "Local Orthographic".
    assert_eq!(method_epsg_code(&wkt), Some(1130), "{wkt}");
    assert_eq!(proj_string(&crs), input);
}

#[test]
fn io_projparse_ortho_with_scale() {
    let input = "+proj=ortho +lat_0=0 +lon_0=0 +alpha=0 +k=0.9 +x_0=0 \
                 +y_0=0 +ellps=WGS84 +units=m +no_defs +type=crs";
    let crs = proj_crs(input);
    let wkt = wkt2_single(&crs);
    assert_eq!(method_epsg_code(&wkt), Some(1130), "{wkt}");
    assert_eq!(proj_string(&crs), input);
}

#[test]
fn io_projparse_ortho_spherical_on_ellipsoid() {
    let input = "+proj=ortho +f=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
                 +ellps=WGS84 +units=m +no_defs +type=crs";
    let crs = proj_crs(input);
    let wkt = wkt2_single(&crs);
    assert!(
        wkt.contains("METHOD[\"Orthographic (Spherical)\"]"),
        "{wkt}"
    );
    assert_eq!(proj_string(&crs), input);
}

#[test]
fn io_projparse_ortho_spherical_on_sphere() {
    let input = "+proj=ortho +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
                 +R=6378137 +units=m +no_defs +type=crs";
    assert_eq!(proj_string(&proj_crs(input)), input);
}

#[test]
fn io_projparse_peirce_q_square() {
    let crs = proj_crs("+proj=peirce_q +shape=square +type=crs");
    assert_eq!(
        proj_string(&crs),
        "+proj=peirce_q +shape=square +lat_0=90 +lon_0=0 +k_0=1 +x_0=0 +y_0=0 \
         +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_peirce_q_diamond() {
    let crs = proj_crs("+proj=peirce_q +shape=diamond +type=crs");
    assert_eq!(
        proj_string(&crs),
        "+proj=peirce_q +shape=diamond +lat_0=90 +lon_0=0 +k_0=1 +x_0=0 +y_0=0 \
         +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_peirce_q_invalid_lat_0() {
    let crs = proj_crs("+proj=peirce_q +lat_0=0 +shape=square +type=crs");
    assert!(crs.export_to_proj_string().is_err());
}

#[test]
fn io_projparse_peirce_q_invalid_k_0() {
    let crs = proj_crs("+proj=peirce_q +k_0=0.5 +shape=square +type=crs");
    assert!(crs.export_to_proj_string().is_err());
}
