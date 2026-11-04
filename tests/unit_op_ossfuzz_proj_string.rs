//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, fuzzer inputs written as PROJ strings. The
//! operation factory must return, without a panic or an infinite recursion.

use proj_rs::iso19111::{create_crs_from_proj_string, CoordinateOperationFactory};

/// `TEST(operation, createOperation_ossfuzz_47873)`
#[test]
fn operation_create_operation_ossfuzz_47873() {
    let src = create_crs_from_proj_string(
        "+proj=ob_tran +o_proj=longlat +o_lat_1=1 +o_lat_2=2 +datum=WGS84 \
         +geoidgrids=@x +geoid_crs=horizontal_crs +type=crs",
    )
    .expect("source PROJ string");
    let dst = create_crs_from_proj_string("+proj=longlat +datum=WGS84 +geoidgrids=@y +type=crs")
        .expect("target PROJ string");
    let _ = CoordinateOperationFactory::create().create_operation(&src, &dst);
}
