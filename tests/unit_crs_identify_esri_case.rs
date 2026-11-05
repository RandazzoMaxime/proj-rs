//! PROJ 9.9.0 `test/unit/test_crs_990.cpp`, `projectedCRS_identify_wrong_auth_name_case`: a
//! projected CRS whose authority is written `Esri` is identified as `ESRI:54034`.

use proj_rs::{AuthorityFactory, Crs, DatabaseContext, WktObject, WktParser, WktStrictness};

/// `TEST(crs, projectedCRS_identify_wrong_auth_name_case)`
#[test]
fn crs_projected_crs_identify_wrong_auth_name_case() {
    let db = DatabaseContext::create().expect("proj.db");
    let anonymous = AuthorityFactory::create(&db, "");
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser.attach_database(db);
    let wkt = "PROJCS[\"World_Cylindrical_Equal_Area\",\
        GEOGCS[\"GCS_WGS_1984\",\
        DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
        PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
        PROJECTION[\"Cylindrical_Equal_Area\"],\
        PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],\
        PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",0.0],\
        UNIT[\"Meter\",1.0],AUTHORITY[\"Esri\",54034]]";
    let crs = match parser.create_from_wkt(wkt).expect("WKT") {
        WktObject::Crs(crs @ Crs::Projected(_)) => crs,
        other => panic!("expected a projected CRS: {other:?}"),
    };
    let res = crs.identify_with(Some(&anonymous));
    assert_eq!(res.len(), 1);
    let ids = res[0].0.identifiers();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].code_space(), "ESRI");
    assert_eq!(ids[0].code(), "54034");
}
