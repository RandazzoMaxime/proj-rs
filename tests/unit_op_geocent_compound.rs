//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: geocentric CRS to a bound compound CRS.

use proj_rs::{
    create_crs_from_proj_string, AuthorityFactory, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, ProjLines,
};

const TARGET: &str = "+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb +geoidgrids=@foo.gtx \
                      +geoid_crs=horizontal_crs +type=crs";
const EXPECTED: &str = "+proj=pipeline +step +inv +proj=cart +ellps=WGS84 \
    +step +inv +proj=hgridshift +grids=@foo.gsb \
    +step +inv +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
    +step +proj=unitconvert +xy_in=rad +xy_out=deg";

/// `TEST(operation, geocent_to_compoundCRS)`
#[test]
fn operation_geocent_to_compound_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let source =
        create_crs_from_proj_string("+proj=geocent +datum=WGS84 +units=m +type=crs").expect("src");
    let target = create_crs_from_proj_string(TARGET).expect("dst");
    let op = CoordinateOperationFactory::create()
        .create_operation(&source, &target)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        EXPECTED
    );
}

/// `TEST(operation, geocent_to_compoundCRS_context)`
#[test]
fn operation_geocent_to_compound_crs_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4978")
        .expect("4978");
    let target = create_crs_from_proj_string(TARGET).expect("dst");
    let context = CoordinateOperationContext::create(&db, "EPSG");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        EXPECTED
    );
}
