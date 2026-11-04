//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c` (bound CRS to bound CRS):
//! the `createOperation` case that this crate reproduces.
//!
//! The function keeps the C++ `TEST()` name in its doc comment. The expected PROJ string is
//! copied from that file. The porting notes are in `docs/PORTING-lotc-bound-bound.md`.

use proj_rs::iso19111::{
    create_crs_from_proj_string, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, ProjLines,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn proj_crs(text: &str) -> Crs {
    create_crs_from_proj_string(text).expect(text)
}

/// `exportToPROJString(PROJStringFormatter::create())`, without database.
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

/// Operation from `source` to `target` with the default context, the first one.
fn first_operation(db: &DatabaseContext, source: &Crs, target: &Crs) -> CoordinateOperation {
    let ctx = CoordinateOperationContext::create(db, "");
    CoordinateOperationFactory::create()
        .create_operations(source, target, &ctx)
        .expect("operations")
        .into_iter()
        .next()
        .expect("operation")
}

/// TEST(operation, createOperation_on_crs_with_bound_crs_and_wktext)
#[test]
fn operation_create_operation_on_crs_with_bound_crs_and_wktext() {
    let db = database();
    let src = proj_crs(
        "+proj=utm +zone=55 +south +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 \
         +units=m +no_defs +nadgrids=@GDA94_GDA2020_conformal.gsb +ignored1 \
         +ignored2=val +wktext +type=crs",
    );
    let dst = proj_crs(
        "+proj=utm +zone=55 +south +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 \
         +units=m +no_defs +type=crs",
    );
    let op = first_operation(&db, &src, &dst);
    assert_eq!(
        plain(&op),
        "+proj=pipeline \
         +step +inv +proj=utm +zone=55 +south +ellps=GRS80 \
         +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb \
         +step +proj=utm +zone=55 +south +ellps=GRS80"
    );
}
