//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`:
//! `TEST(operation, createOperation_bound_of_spherical_ocentric_to_same_type)`.
//!
//! Two bound geocentric-latitude spherical-based CRS (GRS80 with `towgs84=1,2,3`, clrk66 with
//! `towgs84=4,5,6`) give one Helmert `x=-3 y=-3 z=-3`, the difference of the two TOWGS84 vectors,
//! between the `geoc` legs. The expected PROJ string is copied from the C++ test.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperationFactory, DatabaseContext, ProjLines,
};

#[test]
fn operation_bound_of_spherical_ocentric_to_same_type() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input(
        "+proj=longlat +geoc +ellps=GRS80 +towgs84=1,2,3 +type=crs",
        &db,
    )
    .expect("source");
    let dest = create_from_user_input(
        "+proj=longlat +geoc +ellps=clrk66 +towgs84=4,5,6 +type=crs",
        &db,
    )
    .expect("target");

    let operation = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("a bound CRS to a bound CRS of the same type");
    assert_eq!(
        operation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=geoc +ellps=GRS80 \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=-3 +y=-3 +z=-3 \
         +step +inv +proj=cart +ellps=clrk66 \
         +step +proj=pop +v_3 \
         +step +proj=geoc +ellps=clrk66 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}
