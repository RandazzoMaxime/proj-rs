//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c`, bound CRS cases
//! and the spherical ellipsoidal CRS export.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected PROJ strings
//! are copied from that file.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationFactory, DatabaseContext, ProjLines,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

/// `exportToPROJString(PROJStringFormatter::create().get())`, without database.
fn plain(operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.export_to_proj_string(),
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
        }
    }
    .expect("PROJ string")
}

/// TEST(operation, createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ocentric)
#[test]
fn operation_create_operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ocentric() {
    let db = database();
    let authority = AuthorityFactory::create(&db, "IAU_2015");
    let source = authority
        .create_coordinate_reference_system("19902")
        .expect("19902");
    let target = authority
        .create_coordinate_reference_system("19912")
        .expect("19912");

    let op = CoordinateOperationFactory::create()
        .create_operation(&source, &target)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=geoc +a=2440530 +b=2438260 \
         +step +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260"
    );

    let inverse = CoordinateOperationFactory::create()
        .create_operation(&target, &source)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        plain(&inverse),
        "+proj=pipeline \
         +step +inv +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260 \
         +step +proj=geoc +a=2440530 +b=2438260 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
