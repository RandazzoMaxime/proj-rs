//! PROJ 9.9.0 `test/unit/test_c_api.cpp` (`CApi.*`) cases of the operation and factory
//! core lot. Each function is one `TEST_F`, with the C++ name in its doc comment.
//! Expected names, counts, EPSG codes and coordinates are those of PROJ 9.9.0. The C
//! context, list, and pointer handles of the C tests are not reproduced; the observable
//! result is checked through the Rust API. See `docs/PORTING-capi-operation-factory-core.md`.

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, BallparkUse, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion, WktConvention, WktFormatter, WktParser,
};
use proj_rs::{Direction, Proj};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg_crs(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

/// `proj_create_operation_factory_context` followed by the spatial criterion and the
/// grid availability the C tests set: partial intersection, grids ignored.
fn context(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut context = CoordinateOperationContext::create(db, "EPSG");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    context
}

fn operations(
    db: &DatabaseContext,
    source: &str,
    target: &str,
    context: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(&epsg_crs(db, source), &epsg_crs(db, target), context)
        .expect("operations")
}

/// `CApi.proj_create_from_database_grid_alternative_null_old_proj_grid_name`
///
/// EPSG:9484 uses `href2008a.bin`. Its PROJ grid alternative `no_kv_href2008a.tif` must be
/// the grid written in the operation when the PROJ alternative names are used.
#[test]
fn capi_proj_create_from_database_grid_alternative_null_old_proj_grid_name() {
    let db = database();
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("9484", AlternativeGridNames::Proj)
        .expect("EPSG:9484");
    let definition = operation
        .export_to_proj_string_with_database(&db, ProjLines::Single)
        .expect("PROJ string");
    assert!(
        definition.contains("no_kv_href2008a.tif"),
        "expected CDN grid name in {definition}"
    );
}

/// `CApi.proj_create_operations`
///
/// NAD27 (EPSG:4267) to NAD83 (EPSG:4269), partial intersection, grids ignored: 10
/// operations, the first is `NAD27 to NAD83 (4)` and is not ballpark. The coordinate
/// (lat 40, lon -100) is suggested `NAD27 to NAD83 (7)`, which is the NADCON5 for the USA.
#[test]
fn capi_proj_create_operations() {
    let db = database();
    let list = operations(&db, "4267", "4269", &context(&db));
    assert_eq!(list.len(), 10);
    assert!(list.get(list.len()).is_none());
    let first = &list[0];
    assert!(!first.has_ballpark_transformation());
    assert_eq!(first.name(), "NAD27 to NAD83 (4)");

    let suggested = CoordinateOperationFactory::create()
        .suggested_operation(&list, &epsg_crs(&db, "4267"), (40.0, -100.0))
        .expect("suggestion")
        .expect("an operation covers (40, -100)");
    assert_eq!(list[suggested].name(), "NAD27 to NAD83 (7)");

    // A point outside every area of use is not an error in PROJ; it has no suggestion.
    assert!(CoordinateOperationFactory::create()
        .suggested_operation(&list, &epsg_crs(&db, "4267"), (40.0, 10.0))
        .is_ok());
}

/// `CApi.proj_trans_bounds_to_compound_crs`
///
/// EPSG:4326 to EPSG:9707 (WGS 84 + EGM96 height) is a no-op pipeline, so PROJ returns the
/// box unchanged, without the densification check. The C box `(40, -120, 64, -80)` in
/// latitude-first order is `(-120, 40, -80, 64)` in the longitude-first order of this API.
#[test]
fn capi_proj_trans_bounds_to_compound_crs() {
    let transform = Proj::new_known_crs("EPSG:4326", "EPSG:9707", None).expect("EPSG:9707");
    let bounds: [f64; 4] = transform
        .transform_bounds_in(Direction::Forward, -120.0, 40.0, -80.0, 64.0, 0)
        .expect("bounds");
    let expected = [-120.0, 40.0, -80.0, 64.0];
    for (got, want) in bounds.iter().zip(expected) {
        assert!((got - want).abs() <= 1e-8, "{bounds:?} != {expected:?}");
    }
}

/// `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type`
///
/// The deriving conversion of a projected CRS is a conversion, not a CRS: the WKT parses to
/// an object that has no CRS, so there is no CRS to bind to WGS 84. The C fixture is
/// represented here by EPSG:32631.
#[test]
fn capi_proj_crs_create_bound_crs_to_wgs84_on_invalid_type() {
    let db = database();
    let projected = epsg_crs(&db, "32631");
    let Crs::Projected(projected) = projected else {
        panic!("EPSG:32631 is a projected CRS");
    };
    let wkt = projected
        .deriving_conversion()
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("conversion WKT");
    let parsed = WktParser::new().create_from_wkt(&wkt).expect("WKT parses");
    assert!(parsed.crs().is_none(), "a conversion is not a CRS");
}

/// `CApi.proj_create_operations_allow_ballpark_transformations`
///
/// NAD27 (EPSG:4267) to ETRS89 (EPSG:4258): the ballpark transformation is allowed by
/// default and when set explicitly, which gives one operation; forbidden, none.
#[test]
fn capi_proj_create_operations_allow_ballpark_transformations() {
    let db = database();
    let mut context = context(&db);
    assert_eq!(operations(&db, "4267", "4258", &context).len(), 1);

    context.set_ballpark_use(BallparkUse::Allow);
    assert_eq!(operations(&db, "4267", "4258", &context).len(), 1);

    context.set_ballpark_use(BallparkUse::Forbid);
    assert_eq!(operations(&db, "4267", "4258", &context).len(), 0);
}
