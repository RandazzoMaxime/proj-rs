//! The accuracy of a concatenated operation built by the factory: the sum of the accuracies of its
//! steps, with the ETRS89 ensemble case, rounded as PROJ stores it (`getAccuracy`,
//! `oputils.cpp:560-636`, and `ConcatenatedOperation::createComputeMetadata`,
//! `concatenatedoperation.cpp:754-756`, PROJ 9.9.0). The expected values are the ones
//! `projinfo --summary` 9.9.0 prints for the pair; they are PROJ's values, not this port's.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

/// The candidates of `projinfo -s source -t target --summary --spatial-test intersects`: the
/// partial intersection criterion, grids not checked.
fn candidates(source: &str, target: &str) -> Vec<CoordinateOperation> {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "EPSG");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
}

/// The accuracy of the candidate called `name`, and that name's presence.
fn accuracy_of(operations: &[CoordinateOperation], name: &str) -> Option<f64> {
    let operation = operations
        .iter()
        .find(|operation| operation.name() == name)
        .unwrap_or_else(|| panic!("no candidate named {name}"));
    operation.accuracy()
}

/// `projinfo -s EPSG:4746 -t EPSG:4230 --summary` (PROJ 9.9.0): `PD/83 to WGS 84 (1) + Inverse of
/// ED50 to WGS 84 (2)` is 7 m (the sum of 1 m and 6 m), and `(1)` with `Inverse of ED50 to WGS 84
/// (1)` is 11 m. The port used to keep the largest step, 6 m.
#[test]
fn concatenation_accuracy_is_the_sum_of_its_steps() {
    let operations = candidates("EPSG:4746", "EPSG:4230");
    assert_eq!(
        accuracy_of(
            &operations,
            "PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (2)"
        ),
        Some(7.0)
    );
    assert_eq!(
        accuracy_of(
            &operations,
            "PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (1)"
        ),
        Some(11.0)
    );
}

/// `projinfo -s EPSG:4312 -t EPSG:4230 --summary` (PROJ 9.9.0): the MGI to WGS 84 and ED50 to WGS
/// 84 legs add up (7.5 m, 6.5 m, 10.5 m, 3.5 m, 2.5 m), and the ETRS89 pair is 2.14 m, which is
/// stored as the 15-digit text PROJ writes, so it compares equal to the literal 2.14.
#[test]
fn mgi_to_ed50_candidates_carry_the_summed_accuracies() {
    let operations = candidates("EPSG:4312", "EPSG:4230");
    let expected = [
        ("MGI to WGS 84 (3) + Inverse of ED50 to WGS 84 (2)", 7.5),
        ("MGI to WGS 84 (3) + Inverse of ED50 to WGS 84 (1)", 11.5),
        ("MGI to WGS 84 (8) + Inverse of ED50 to WGS 84 (2)", 6.5),
        ("MGI to WGS 84 (8) + Inverse of ED50 to WGS 84 (1)", 10.5),
        ("MGI to WGS 84 (3) + Inverse of ED50 to WGS 84 (17)", 3.5),
        ("MGI to WGS 84 (3) + Inverse of ED50 to WGS 84 (18)", 2.5),
        ("MGI to ETRS89 (8) + Inverse of ED50 to ETRS89 (10)", 2.14),
    ];
    for (name, metres) in expected {
        assert_eq!(accuracy_of(&operations, name), Some(metres), "{name}");
    }
}

/// `projinfo -s EPSG:11108 -t EPSG:11128 --summary` (PROJ 9.9.0): `Inverse of ETRS89 to ETRS89-PRT
/// [1995] + ETRS89 to ETRS89-ESP [ERGNSS]` is 0.1 m, not 0.2 m. Its two legs are 0.1 m each,
/// both go through the ETRS89 CRS, and one of them is a no-op, so the ETRS89 case counts the
/// larger of the two once (`oputils.cpp:600-636`).
#[test]
fn two_etrs89_legs_through_the_ensemble_count_once() {
    let db = DatabaseContext::create().expect("database");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let source = epsg
        .create_coordinate_reference_system("11108")
        .expect("11108");
    let target = epsg
        .create_coordinate_reference_system("11128")
        .expect("11128");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert_eq!(
        accuracy_of(
            &operations,
            "Inverse of ETRS89 to ETRS89-PRT [1995] + ETRS89 to ETRS89-ESP [ERGNSS]"
        ),
        Some(0.1)
    );
}
