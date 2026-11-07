//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, projected CRS to projected CRS operations
//! that a grid offset or a unit change makes a no-op.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The expected names and PROJ
//! strings are copied from that file. The C++ `ComparePROJString` helper is mirrored by
//! [`assert_proj_eq`], except that a token with no `=` is compared exactly.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, GridAvailabilityUse, ProjLines, SpatialCriterion,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn operations(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, context)
        .expect("operations")
}

fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_none() || conversion.target_crs().is_none() =>
        {
            conversion.export_to_proj_string()
        }
        _ => operation.export_to_proj_string_with_database(db, ProjLines::Single),
    }
    .expect("PROJ string")
}

/// PROJ's `ComparePROJString`: tokens split on spaces, numeric `key=value` terms compared to a
/// relative 1e-14, other terms compared as text. Tokens without `=` are compared exactly here.
fn assert_proj_eq(actual: &str, expected: &str) {
    let actual_tokens: Vec<&str> = actual.split(' ').collect();
    let expected_tokens: Vec<&str> = expected.split(' ').collect();
    assert_eq!(actual_tokens.len(), expected_tokens.len(), "{actual}");
    for (a, e) in actual_tokens.iter().zip(&expected_tokens) {
        match (a.split_once('='), e.split_once('=')) {
            (Some((ak, av)), Some((ek, ev))) => {
                assert_eq!(ak, ek, "{actual}");
                match (av.parse::<f64>(), ev.parse::<f64>()) {
                    (Ok(x), Ok(y)) => assert!((x - y).abs() <= 1e-14 * x.abs(), "{a} vs {e}"),
                    (Err(_), Err(_)) => assert_eq!(av, ev, "{actual}"),
                    _ => panic!("{a} vs {e}"),
                }
            }
            _ => assert_eq!(a, e, "{actual}"),
        }
    }
}

/// `TEST(operation, projCRS_to_projCRS_context_grid_offsets)`
#[test]
fn operation_proj_crs_to_proj_crs_context_grid_offsets() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    {
        // Karbala 1979 / UTM zone 38N to IGRS / UTM zone 38N
        let list = operations(&epsg(&db, "3392"), &epsg(&db, "3891"), &ctx);
        assert_eq!(list.len(), 2);
        assert_eq!(
            list[1].name(),
            "Karbala 1979 / UTM zone 38N to IGRS / UTM zone 38N (1)"
        );
        assert_proj_eq(
            &proj(&db, &list[1]),
            "+proj=affine +xoff=-287.54 +yoff=278.25",
        );
    }
    {
        // IGRS / UTM zone 38N to Karbala 1979 / UTM zone 38N
        let list = operations(&epsg(&db, "3891"), &epsg(&db, "3392"), &ctx);
        assert_eq!(list.len(), 2);
        assert_eq!(
            list[1].name(),
            "Inverse of Karbala 1979 / UTM zone 38N to IGRS / UTM zone 38N (1)"
        );
        assert_proj_eq(
            &proj(&db, &list[1]),
            "+proj=affine +xoff=287.54 +yoff=-278.25",
        );
    }
}

/// `TEST(operation, projCRS_to_projCRS_context_grid_offsets_non_metre_unit_noop)`
#[test]
fn operation_proj_crs_to_proj_crs_context_grid_offsets_non_metre_unit_noop() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    // NAD83(2011) / Adjusted Jackson (ftUS) to NAD83(HARN) / WISCRS Jackson (ftUS)
    let list = operations(&epsg(&db, "10516"), &epsg(&db, "8162"), &ctx);
    assert!(list.len() >= 2);
    assert_eq!(
        list[0].name(),
        "NAD83(2011) / Adjusted Jackson (ftUS) to NAD83(HARN) / WISCRS Jackson (ftUS) (1)"
    );
    assert_proj_eq(&proj(&db, &list[0]), "+proj=noop");
}
