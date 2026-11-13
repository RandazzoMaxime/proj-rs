//! PROJ 9.9.0 `test/unit/test_crs.cpp`, `crs.scope_area_bbox_remark`.
//!
//! The WKT2 text parsed here carries SCOPE, AREA, BBOX, VERTICALEXTENT,
//! TIMEEXTENT, ID and REMARK. The expected export is copied from the C++ test.

use proj_rs::{Crs, GeodeticCRS, WktConvention, WktFormatter, WktParser};

const IN_WKT: &str = "GEODETICCRS[\"JGD2000\",\
DATUM[\"Japanese Geodetic Datum 2000\",\
  ELLIPSOID[\"GRS 1980\",6378137,298.257222101]],\
CS[Cartesian,3],\
  AXIS[\"(X)\",geocentricX],\
  AXIS[\"(Y)\",geocentricY],\
  AXIS[\"(Z)\",geocentricZ],\
  LENGTHUNIT[\"metre\",1.0],\
SCOPE[\"Geodesy, topographic mapping and cadastre\"],\
AREA[\"Japan\"],\
BBOX[17.09,122.38,46.05,157.64],\
VERTICALEXTENT[-10000,10000],\
TIMEEXTENT[2002-04-01,2011-10-21],\
ID[\"EPSG\",4946],\
REMARK[\"some_remark\"]]";

const EXPECTED_WKT: &str = r#"GEODCRS["JGD2000",
    DATUM["Japanese Geodetic Datum 2000",
        ELLIPSOID["GRS 1980",6378137,298.257222101,
            LENGTHUNIT["metre",1]]],
    PRIMEM["Greenwich",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[Cartesian,3],
        AXIS["(X)",geocentricX,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["(Y)",geocentricY,
            ORDER[2],
            LENGTHUNIT["metre",1]],
        AXIS["(Z)",geocentricZ,
            ORDER[3],
            LENGTHUNIT["metre",1]],
    SCOPE["Geodesy, topographic mapping and cadastre"],
    AREA["Japan"],
    BBOX[17.09,122.38,46.05,157.64],
    VERTICALEXTENT[-10000,10000,
        LENGTHUNIT["metre",1]],
    TIMEEXTENT[2002-04-01,2011-10-21],
    ID["EPSG",4946],
    REMARK["some_remark"]]"#;

fn parse(text: &str) -> Crs {
    WktParser::new()
        .create_from_wkt(text)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

/// `TEST(crs, scope_area_bbox_remark)`
#[test]
fn crs_scope_area_bbox_remark() {
    let crs = parse(IN_WKT);
    let Crs::Geodetic(geodetic) = &crs else {
        panic!("expected a geodetic CRS, got {crs:?}");
    };
    let GeodeticCRS::Geocentric(_) = geodetic else {
        panic!("expected a geocentric CRS, got {geodetic:?}");
    };
    assert_eq!(geodetic.domains().len(), 1);
    let domain = &geodetic.domains()[0];
    assert_eq!(
        domain.scope(),
        Some("Geodesy, topographic mapping and cadastre")
    );
    assert_eq!(domain.area(), Some("Japan"));
    assert_eq!(domain.south_bound_latitude(), Some(17.09));
    assert_eq!(domain.west_bound_longitude(), Some(122.38));
    assert_eq!(domain.north_bound_latitude(), Some(46.05));
    assert_eq!(domain.east_bound_longitude(), Some(157.64));

    let vertical = domain.vertical_extent().expect("vertical extent");
    assert_eq!(vertical.minimum(), -10000.0);
    assert_eq!(vertical.maximum(), 10000.0);
    assert_eq!(vertical.unit().name(), "metre");

    let temporal = domain.temporal_extent().expect("temporal extent");
    assert_eq!(temporal.start(), "2002-04-01");
    assert_eq!(temporal.stop(), "2011-10-21");

    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.set_indentation_width(4);
    let got = crs.export_to_wkt(&mut formatter).expect("export");
    assert_eq!(got, EXPECTED_WKT);
}
