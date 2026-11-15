//! ITRF2020 (EPSG:9988) to WGS 84 (EPSG:4978): PROJ 9.9.0 leads with the datum pivot (2,01 m),
//! not EPSG:10608 alone (0,01 m), because the ensemble step is skipped after the intermediate
//! search (`bTriedThroughIntermediateCRS`, `coordinateoperationfactory.cpp:4502`, `:4524`).
//! Expected values: PROJ 9.9.0 `projinfo --summary` and `cs2cs`, not this port.

use std::io::Write;
use std::process::{Command, Stdio};

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, SpatialCriterion,
};

const PIVOT_NAME: &str =
    "Inverse of WGS 84 (G2296) to ITRF2020 (1) + Conversion from WGS 84 (G2296) \
     (geocentric) to WGS 84 (G2296) (geog2D) + Inverse of WGS 84 to WGS 84 (G2296) + \
     Conversion from WGS 84 (geog2D) to WGS 84 (geocentric)";

/// `cs2cs -d 10 EPSG:9988 +to EPSG:4978` (PROJ 9.9.0) for `4200985 168325 4780236`.
const PROJ_OUTPUT: [f64; 3] = [4200985.3753091125, 168324.5839754438, 4780235.670799215];
const TOLERANCE: f64 = 1e-6;

#[test]
fn itrf2020_to_wgs84_ensemble_leads_with_the_pivot_not_epsg_10608() {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input("EPSG:9988", &db).expect("source");
    let target = create_from_user_input("EPSG:4978", &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert_eq!(operations.len(), 40);
    assert_eq!(operations[0].name(), PIVOT_NAME);
    assert!(operations
        .iter()
        .all(|operation| operation.name() != "Inverse of WGS 84 (G2296) to ITRF2020 (1)"));
}

#[test]
fn itrf2020_to_wgs84_ensemble_cs2cs_matches_proj_values() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cs2cs"))
        .args(["-d", "10", "EPSG:9988", "+to", "EPSG:4978"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cs2cs");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"4200985 168325 4780236\n")
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let fields: Vec<f64> = stdout
        .lines()
        .next()
        .expect("one output line")
        .split_whitespace()
        .map(|field| field.parse::<f64>().expect("number"))
        .collect();
    assert_eq!(fields.len(), 3, "{stdout}");
    for (got, expected) in fields.iter().zip(PROJ_OUTPUT) {
        assert!(
            (got - expected).abs() <= TOLERANCE,
            "got {fields:?}, PROJ {PROJ_OUTPUT:?}"
        );
    }
}
