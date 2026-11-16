//! `createCoordinateOperation` of an `other_transformation` row (`factory.cpp:6872`): the grid file
//! parameter comes after the numeric parameters, where `grid_transformation` puts it first.
//!
//! Expected values: PROJ 9.9.0 `projinfo -k operation EPSG:10813 -o WKT2:2019`, whose parameters
//! run from `X-axis translation` to `Target epoch`, then `Point motion velocity grid file`.

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, DatabaseContext,
};

#[test]
fn operation_10813_grid_file_is_the_last_parameter() {
    let db = DatabaseContext::create().expect("proj.db");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("10813", AlternativeGridNames::Official)
        .expect("10813");
    let CoordinateOperation::Transformation(transformation) = operation else {
        panic!("10813 is not a transformation");
    };
    let names: Vec<&str> = transformation
        .parameters()
        .iter()
        .map(|parameter| parameter.name())
        .collect();
    assert_eq!(
        names,
        [
            "X-axis translation",
            "Y-axis translation",
            "Z-axis translation",
            "X-axis rotation",
            "Y-axis rotation",
            "Z-axis rotation",
            "Scale difference",
            "Source epoch",
            "Target epoch",
            "Point motion velocity grid file",
        ]
    );
}
