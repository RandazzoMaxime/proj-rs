//! PROJ 9.9.0 `test/unit/test_io.cpp`, a WKT1 Lambert-93 CRS from the Oracle table, identified
//! against all authorities of the database.

use proj_rs::iso19111::{AuthorityFactory, Crs, DatabaseContext, WktObject, WktParser};

/// `TEST(wkt_parse, wkt1_oracle)`
#[test]
fn wkt_parse_wkt1_oracle() {
    let wkt = "PROJCS[\"RGF93 / Lambert-93\", GEOGCS [ \"RGF93\", DATUM [\"Reseau Geodesique Francais 1993 (EPSG ID 6171)\", SPHEROID [\"GRS 1980 (EPSG ID 7019)\", 6378137.0, 298.257222101]], PRIMEM [ \"Greenwich\", 0.000000000 ], UNIT [\"Decimal Degree\", 0.0174532925199433]], PROJECTION [\"Lambert Conformal Conic\"], PARAMETER [\"Latitude_Of_Origin\", 46.5], PARAMETER [\"Central_Meridian\", 3.0], PARAMETER [\"Standard_Parallel_1\", 49.0], PARAMETER [\"Standard_Parallel_2\", 44.0], PARAMETER [\"False_Easting\", 700000.0], PARAMETER [\"False_Northing\", 6600000.0], UNIT [\"Meter\", 1.0]]";
    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let WktObject::Crs(Crs::Projected(crs)) = parser.create_from_wkt(wkt).expect("wkt") else {
        panic!("expected a projected CRS");
    };
    let datum = crs.base_crs().datum().expect("datum");
    assert_eq!(datum.name(), "Reseau Geodesique Francais 1993");
    assert_eq!(datum.identifiers()[0].code(), "6171");
    assert_eq!(
        crs.deriving_conversion().method().name(),
        "Lambert Conic Conformal (2SP)"
    );
    let factory = AuthorityFactory::create(&db, "");
    let found = Crs::Projected(crs).identify_with(Some(&factory));
    assert!(!found.is_empty());
    assert_eq!(found[0].0.identifiers()[0].code(), "2154");
    assert_eq!(found[0].1, 90);
}
