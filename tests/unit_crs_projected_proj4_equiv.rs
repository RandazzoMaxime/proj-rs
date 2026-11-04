//! PROJ 9.9.0 `test/unit/test_crs.cpp`, equivalence of projected CRS read from PROJ strings
//! with a PROJ4 extension, through their WKT.

use proj_rs::iso19111::{
    create_crs_from_proj_string, Crs, Equivalence, WktConvention, WktFormatter, WktObject,
    WktParser,
};

fn projected(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("proj string")
}

/// `TEST(crs, projected_is_equivalent_to_with_proj4_extension)`
#[test]
fn crs_projected_is_equivalent_to_with_proj4_extension() {
    let crs1 = projected(
        "+proj=omerc +lat_0=50 +alpha=50.0 +no_rot +a=6378144.0 +b=6356759.0 +lon_0=8.0 +type=crs",
    );
    let wkt = crs1
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt");
    let WktObject::Crs(crs_from_wkt) = WktParser::new().create_from_wkt(&wkt).expect("wkt") else {
        panic!("expected a CRS");
    };
    assert!(crs1.is_equivalent_to_with(&crs_from_wkt, Equivalence::Equivalent));
    let crs2 = projected(
        "+proj=omerc +lat_0=50 +no_rot +alpha=50.0 +a=6378144.0 +b=6356759.0 +lon_0=8.0 +type=crs",
    );
    assert!(crs1.is_equivalent_to_with(&crs2, Equivalence::Equivalent));
    let no_rot = projected(
        "+proj=omerc +lat_0=50 +alpha=50.0 +a=6378144.0 +b=6356759.0 +lon_0=8.0 +type=crs",
    );
    assert!(!crs1.is_equivalent_to_with(&no_rot, Equivalence::Equivalent));
    assert!(!no_rot.is_equivalent_to_with(&crs1, Equivalence::Equivalent));
    let different = projected(
        "+proj=omerc +lat_0=50 +alpha=49.0 +no_rot +a=6378144.0 +b=6356759.0 +lon_0=8.0 +type=crs",
    );
    assert!(!crs1.is_equivalent_to_with(&different, Equivalence::Equivalent));
}
