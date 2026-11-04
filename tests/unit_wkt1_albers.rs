//! PROJ 9.9.0 `test/unit/test_io.cpp`, a WKT1 projected CRS from the Oracle table, identified
//! against all authorities of the database.

use proj_rs::iso19111::{AuthorityFactory, Crs, DatabaseContext, WktObject, WktParser};

/// `TEST(wkt_parse, wkt1_oracle_albers_conical_equal_area)`
#[test]
fn wkt_parse_wkt1_oracle_albers_conical_equal_area() {
    let wkt = "PROJCS[\"NAD83 / BC Albers\",GEOGCS[\"NAD83\",DATUM[\"North_American_Datum_1983\",SPHEROID[\"GRS 1980\",6378137,298.257222101,AUTHORITY[\"EPSG\",\"7019\"]],AUTHORITY[\"EPSG\",\"6269\"]],PRIMEM[\"Greenwich\",0],UNIT[\"Decimal Degree\",0.0174532925199433]],PROJECTION[\"Albers_Conical_Equal_Area\"],PARAMETER[\"Latitude_Of_Origin\",45],PARAMETER[\"Central_Meridian\",-126],PARAMETER[\"Standard_Parallel_1\",50],PARAMETER[\"Standard_Parallel_2\",58.5],PARAMETER[\"False_Easting\",1000000],PARAMETER[\"False_Northing\",0],UNIT[\"Meter\",1],AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH]]";
    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let WktObject::Crs(Crs::Projected(crs)) = parser.create_from_wkt(wkt).expect("wkt") else {
        panic!("expected a projected CRS");
    };
    let factory = AuthorityFactory::create(&db, "");
    let found = Crs::Projected(crs).identify_with(Some(&factory));
    assert!(!found.is_empty());
    let (front, confidence) = &found[0];
    assert_eq!(front.identifiers()[0].code(), "3005");
    assert_eq!(*confidence, 100);
}
