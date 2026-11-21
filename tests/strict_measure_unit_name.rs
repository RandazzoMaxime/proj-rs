//! Under the strict criterion PROJ compares a measure by its value and the name of its unit, not
//! the factor (`Measure::operator==`, `UnitOfMeasure::operator==`, common.cpp:305, 423). A
//! transformation read back from its WKT2 is then strictly equivalent to the database one, though
//! the milliarc-second factor is written with 15 significant digits (PROJ 9.9.0:
//! `proj_is_equivalent_to(..., PJ_COMP_STRICT)` is 1).

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, DatabaseContext,
};
use proj_rs::{WktConvention, WktFormatter, WktObject, WktParser};

fn read_back(code: &str) -> (CoordinateOperation, CoordinateOperation) {
    let db = DatabaseContext::create().expect("database");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation(code, AlternativeGridNames::Official)
        .expect("operation");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    let wkt = operation.export_to_wkt(&mut formatter).expect("WKT");
    let WktObject::Transformation(back) = WktParser::new().create_from_wkt(&wkt).expect("read")
    else {
        panic!("a transformation: {wkt}");
    };
    (operation, CoordinateOperation::Transformation(back))
}

#[test]
fn gda94_to_gda2020_reads_back_strictly_equivalent() {
    let (operation, back) = read_back("8048");
    assert!(operation.is_equivalent_to(&back));
    assert!(back.is_equivalent_to(&operation));
}

#[test]
fn a_different_unit_name_is_not_strictly_equivalent() {
    let (operation, _) = read_back("8048");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    let wkt = operation
        .export_to_wkt(&mut formatter)
        .expect("WKT")
        .replace("\"milliarc-second\"", "\"other unit\"");
    let WktObject::Transformation(other) = WktParser::new().create_from_wkt(&wkt).expect("read")
    else {
        panic!("a transformation");
    };
    assert!(!operation.is_equivalent_to(&CoordinateOperation::Transformation(other)));
}
