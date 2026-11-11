//! PROJ 9.9.0 `test/unit/test_operation.cpp`: `TEST(operation, normalizeForVisualization)`.
//!
//! The blocks follow the C++ test in order. Expected PROJ strings and the WKT2_2019 round trip are
//! copied from that test. Not in this file, see `docs/PORTING-visualization-alternatives.md`:
//! the compound source block (case 5, grid order), and the bound source block (case 6), whose
//! PROJ string depends on the export of a transformation with a bound source, not checked.

use proj_rs::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, CoordinateOperationFactory, Crs,
    DatabaseContext, ProjLines, WktConvention, WktFormatter, WktObject, WktParser,
};

/// `PROJStringFormatter::create()` of an operation, as one line.
fn proj_string(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ")
}

/// `CoordinateOperationFactory::create()->createOperation(source, target)`, which must exist.
fn operation_between(source: &Crs, target: &Crs) -> CoordinateOperation {
    CoordinateOperationFactory::create()
        .create_operation(source, target)
        .expect("operation")
        .expect("an operation")
}

/// `checkThroughWKTRoundtrip`: the operation written in WKT2_2019 and read back is equivalent,
/// and has the same PROJ string.
fn check_through_wkt_round_trip(db: &DatabaseContext, operation: &CoordinateOperation) {
    let wkt = operation
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2_2019");
    let read = match WktParser::new().create_from_wkt(&wkt).expect("WKT") {
        WktObject::Concatenated(read) => CoordinateOperation::Concatenated(read),
        WktObject::Transformation(read) => CoordinateOperation::Transformation(read),
        WktObject::Conversion(read) => CoordinateOperation::Conversion(read),
        other => panic!("expected an operation: {other:?}"),
    };
    assert!(operation.is_equivalent_to(&read));
    assert_eq!(proj_string(db, &read), proj_string(db, operation));
}

/// `TEST(operation, normalizeForVisualization)`, without its compound source block.
#[test]
fn operation_normalize_for_visualization() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");

    // Source(geographic) must be inverted
    {
        let src = factory
            .create_coordinate_reference_system("4326")
            .expect("EPSG:4326");
        let dst = factory
            .create_coordinate_reference_system("32631")
            .expect("EPSG:32631");
        let op = operation_between(&src, &dst);
        let normalized = op.normalize_for_visualization().expect("normalized");
        assert!(!normalized.is_equivalent_to(&op));
        assert_eq!(
            proj_string(&db, &normalized),
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=utm +zone=31 +ellps=WGS84"
        );
        check_through_wkt_round_trip(&db, &normalized);
    }

    // Target(geographic) must be inverted
    {
        let src = factory
            .create_coordinate_reference_system("32631")
            .expect("EPSG:32631");
        let dst = factory
            .create_coordinate_reference_system("4326")
            .expect("EPSG:4326");
        let op = operation_between(&src, &dst);
        let normalized = op.normalize_for_visualization().expect("normalized");
        assert!(!normalized.is_equivalent_to(&op));
        assert_eq!(
            proj_string(&db, &normalized),
            "+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=WGS84 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg"
        );
        check_through_wkt_round_trip(&db, &normalized);
    }

    // Source(geographic) and target(projected) must be inverted
    {
        let src = factory
            .create_coordinate_reference_system("4326")
            .expect("EPSG:4326");
        let dst = factory
            .create_coordinate_reference_system("3040")
            .expect("EPSG:3040");
        let op = operation_between(&src, &dst);
        let normalized = op.normalize_for_visualization().expect("normalized");
        assert!(!normalized.is_equivalent_to(&op));
        assert_eq!(
            proj_string(&db, &normalized),
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=utm +zone=28 +ellps=GRS80"
        );
        check_through_wkt_round_trip(&db, &normalized);
    }

    // No inversion
    {
        let src = factory
            .create_coordinate_reference_system("32631")
            .expect("EPSG:32631");
        let dst = factory
            .create_coordinate_reference_system("32632")
            .expect("EPSG:32632");
        let op = operation_between(&src, &dst);
        let normalized = op.normalize_for_visualization().expect("normalized");
        assert!(normalized.is_equivalent_to(&op));
    }

    // Test normalizeForVisualization on a concatenated operation with
    // non-overlapping sub-operation extents (EPSG:8047 = ED50 -> ED87 -> WGS84)
    {
        let op = factory
            .create_coordinate_operation("8047", AlternativeGridNames::Official)
            .expect("EPSG:8047");
        assert!(matches!(op, CoordinateOperation::Concatenated(_)));
        let normalized = op.normalize_for_visualization().expect("normalized");
        assert!(!normalized.is_equivalent_to(&op));
        assert!(!proj_string(&db, &normalized).is_empty());
        assert_eq!(normalized.accuracy(), op.accuracy());
        assert_eq!(normalized.remarks(), op.remarks());
        assert_eq!(
            normalized.name(),
            format!(
                "{} (with axis order normalized for visualization)",
                op.name()
            )
        );
        assert_eq!(normalized.domains().len(), 1);
    }
}
