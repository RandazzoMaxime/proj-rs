//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: NAD83(CSRS) MTM zone 7 with HT2 heights
//! to UTM zone 19 with CGVD2013a heights, at the 1997, 2002 and 2010 epochs.

use proj_rs::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion,
};

fn check(year: &str, grid: &str) {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "NRCAN");
    let source = factory
        .create_coordinate_metadata(&format!("NAD83_CSRS_{year}_MTM7_HT2_{year}"))
        .expect("source");
    let target = factory
        .create_coordinate_metadata(&format!("NAD83_CSRS_{year}_UTM19_CGVD2013_{year}"))
        .expect("target");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let factory_ops = CoordinateOperationFactory::create();
    let proj = |operation: &proj_rs::CoordinateOperation| {
        operation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string")
    };

    let list = factory_ops
        .create_operations_between_metadata(&source, &target, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        proj(&list[0]),
        format!(
            "+proj=pipeline +step +inv +proj=tmerc +lat_0=0 +lon_0=-70.5 +k=0.9999 \
             +x_0=304800 +y_0=0 +ellps=GRS80 \
             +step +proj=vgridshift +grids={grid} +multiplier=-1 \
             +step +proj=utm +zone=19 +ellps=GRS80"
        )
    );

    let list = factory_ops
        .create_operations_between_metadata(&target, &source, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        proj(&list[0]),
        format!(
            "+proj=pipeline +step +inv +proj=utm +zone=19 +ellps=GRS80 \
             +step +inv +proj=vgridshift +grids={grid} +multiplier=-1 \
             +step +proj=tmerc +lat_0=0 +lon_0=-70.5 +k=0.9999 \
             +x_0=304800 +y_0=0 +ellps=GRS80"
        )
    );
}

/// `TEST(operation, createOperation_compound_to_compound_with_epoch_HT2_1997_CGG2013a)`
#[test]
fn operation_compound_to_compound_with_epoch_ht2_1997() {
    check("1997", "ca_nrc_HT2_1997_CGG2013a.tif");
}

/// `TEST(operation, createOperation_compound_to_compound_with_epoch_HT2_2002_CGG2013a)`
#[test]
fn operation_compound_to_compound_with_epoch_ht2_2002() {
    check("2002", "ca_nrc_HT2_2002v70_CGG2013a.tif");
}

/// `TEST(operation, createOperation_compound_to_compound_with_epoch_HT2_2010_CGG2013a)`
#[test]
fn operation_compound_to_compound_with_epoch_ht2_2010() {
    check("2010", "ca_nrc_HT2_2010v70_CGG2013a.tif");
}
