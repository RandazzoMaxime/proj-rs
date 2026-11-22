//! An operation read from the WKT that PROJ writes, whose grid parameter has no EPSG identifier
//! (`PARAMETERFILE["Vertical offset file","es_2019z.asc"]`), exports to the PROJ string of
//! PROJ 9.9.0: `parameterValue(code)` finds such a parameter by its name, through
//! `OperationParameter::getEPSGCode` (singleoperation.cpp:1210-1233). One case per method family;
//! the expected strings come from `proj_as_proj_string` of libproj 9.9.0.

use proj_rs::iso19111::{create_user_object, DatabaseContext, InitRules, ProjLines, UserObject};

#[test]
fn wkt_operations_with_unidentified_grid_parameters_export_like_proj() {
    let db = DatabaseContext::create().expect("database");
    let cases = include_str!("data/wkt_operation_proj/cases.tsv");
    let mut count = 0;
    for line in cases.lines() {
        let (wkt, expected) = line.split_once('\t').expect("wkt<TAB>expected");
        let UserObject::Operation(operation) =
            create_user_object(wkt, Some(&db), InitRules::default()).expect("WKT reads")
        else {
            panic!("not an operation: {}", &wkt[..60]);
        };
        let text = operation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .unwrap_or_else(|error| panic!("{}: {error}", operation.name()));
        assert_eq!(text, expected, "{}", operation.name());
        count += 1;
    }
    assert_eq!(count, 7);
}
