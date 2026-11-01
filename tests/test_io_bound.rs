//! PROJ `test/unit/test_io.cpp` checks that import `+towgs84`, `+nadgrids` and
//! `+geoidgrids` from a PROJ string as a bound CRS, or as a compound CRS with a
//! bound vertical CRS, and the vertical units (`+vunits`, `+vto_meter`).
//!
//! Each function is one `TEST()` name. Expected text is copied from that test.

use proj_rs::{
    create_crs_from_proj_string, create_from_user_input, Crs, DatabaseContext, WktConvention,
    WktFormatter, WktLines,
};

fn proj_crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("PROJ string")
}

/// `crs->exportToWKT` with `simulCurNodeHasId()`, on one line when `lines` says so.
fn wkt2(crs: &Crs, lines: WktLines) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.simul_cur_node_has_id();
    formatter.set_lines(lines);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

fn contains(wkt: &str, expected: &str) {
    assert!(wkt.contains(expected), "{expected} not in {wkt}");
}

fn bound(crs: Crs) -> proj_rs::BoundCRS {
    match crs {
        Crs::Bound(bound) => bound,
        _ => panic!("not a bound CRS"),
    }
}

fn compound(crs: Crs) -> proj_rs::CompoundCRS {
    match crs {
        Crs::Compound(compound) => compound,
        _ => panic!("not a compound CRS"),
    }
}

#[test]
fn io_projparse_longlat_towgs84_3_terms() {
    let crs = proj_crs("+proj=longlat +ellps=GRS80 +towgs84=1.2,2,3 +type=crs");
    assert!(matches!(crs, Crs::Bound(_)));
    let wkt = wkt2(&crs, WktLines::Multi);
    contains(&wkt, "METHOD[\"Geocentric translations");
    contains(&wkt, "PARAMETER[\"X-axis translation\",1.2");
    contains(&wkt, "PARAMETER[\"Y-axis translation\",2");
    contains(&wkt, "PARAMETER[\"Z-axis translation\",3");
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=longlat +ellps=GRS80 +towgs84=1.2,2,3,0,0,0,0 +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_longlat_towgs84_7_terms() {
    let crs = proj_crs("+proj=longlat +ellps=GRS80 +towgs84=1.2,2,3,4,5,6,7 +type=crs");
    assert!(matches!(crs, Crs::Bound(_)));
    let wkt = wkt2(&crs, WktLines::Multi);
    contains(&wkt, "METHOD[\"Position Vector transformation");
    contains(&wkt, "PARAMETER[\"X-axis translation\",1.2");
    contains(&wkt, "PARAMETER[\"Y-axis translation\",2");
    contains(&wkt, "PARAMETER[\"Z-axis translation\",3");
    contains(&wkt, "PARAMETER[\"Scale difference\",1.000007");
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=longlat +ellps=GRS80 +towgs84=1.2,2,3,4,5,6,7 +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_longlat_towgs84_7_terms_autocorrect() {
    // Auto-correct wrong sign for rotation terms
    // Cf https://github.com/OSGeo/PROJ/issues/4170
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input(
        "+proj=lcc +lat_0=90 +lon_0=4.36748666666667 +lat_1=51.1666672333333 \
         +lat_2=49.8333339 +x_0=150000.013 +y_0=5400088.438 +ellps=intl \
         +towgs84=-106.8686,52.2978,-103.7239,-0.3366,0.457,-1.8422,-1.2747 \
         +units=m +no_defs +type=crs",
        &db,
    )
    .expect("PROJ string");
    assert!(matches!(crs, Crs::Bound(_)));
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=lcc +lat_0=90 +lon_0=4.36748666666667 +lat_1=51.1666672333333 \
         +lat_2=49.8333339 +x_0=150000.013 +y_0=5400088.438 +ellps=intl \
         +towgs84=-106.8686,52.2978,-103.7239,0.3366,-0.457,1.8422,-1.2747 \
         +units=m +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_longlat_nadgrids() {
    let crs = proj_crs("+proj=longlat +ellps=GRS80 +nadgrids=foo.gsb +type=crs");
    assert!(matches!(crs, Crs::Bound(_)));
    let wkt = wkt2(&crs, WktLines::Multi);
    contains(&wkt, "METHOD[\"NTv2\"");
    contains(
        &wkt,
        "PARAMETERFILE[\"Latitude and longitude difference file\",\"foo.gsb\"]",
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=longlat +ellps=GRS80 +nadgrids=foo.gsb +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_longlat_nadgrids_towgs84_ignored() {
    let crs = bound(proj_crs(
        "+proj=longlat +ellps=GRS80 +towgs84=1,2,3 +nadgrids=foo.gsb +type=crs",
    ));
    assert!(matches!(crs.source_crs(), Crs::Geographic(_)));
}

#[test]
fn io_projparse_longlat_geoidgrids() {
    let crs = proj_crs("+proj=longlat +ellps=GRS80 +geoidgrids=foo.gtx +type=crs");
    assert!(matches!(crs, Crs::Compound(_)));
    let wkt = wkt2(&crs, WktLines::Multi);
    contains(
        &wkt,
        "ABRIDGEDTRANSFORMATION[\"unknown to WGS 84 ellipsoidal height\"",
    );
    contains(
        &wkt,
        "PARAMETERFILE[\"Geoid (height correction) model file\",\"foo.gtx\"]",
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=longlat +ellps=GRS80 +geoidgrids=foo.gtx +geoid_crs=WGS84 \
         +vunits=m +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_longlat_geoidgrids_vunits() {
    let crs = proj_crs("+proj=longlat +ellps=GRS80 +geoidgrids=foo.gtx +vunits=ft +type=crs");
    assert_eq!(compound(crs.clone()).components().len(), 2);
    let wkt = wkt2(&crs, WktLines::Single);
    contains(
        &wkt,
        "AXIS[\"gravity-related height (H)\",up,LENGTHUNIT[\"foot\",0.3048]",
    );
}

#[test]
fn io_projparse_longlat_vunits() {
    let crs = proj_crs("+proj=longlat +ellps=GRS80 +vunits=ft +type=crs");
    assert!(matches!(crs, Crs::Geographic(_)));
    let wkt = wkt2(&crs, WktLines::Single);
    contains(
        &wkt,
        "AXIS[\"ellipsoidal height (h)\",up,ORDER[3],LENGTHUNIT[\"foot\",0.3048]",
    );
}

#[test]
fn io_projparse_vunits() {
    let crs = proj_crs("+vunits=ft +type=crs");
    assert!(matches!(crs, Crs::Vertical(_)));
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+vunits=ft +no_defs +type=crs"
    );
}

#[test]
fn io_projparse_vto_meter() {
    let crs = proj_crs("+vto_meter=2 +type=crs");
    assert!(matches!(crs, Crs::Vertical(_)));
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+vto_meter=2 +no_defs +type=crs"
    );
}
