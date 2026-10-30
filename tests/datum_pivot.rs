//! Operations between geographic CRS of two datums, linked through their geocentric CRS.
//!
//! `EPSG:9000` (ITRF2014, geographic 2D) to `EPSG:7844` (GDA2020, geographic 2D) has no
//! registered operation. PROJ links the two geocentric CRS with `ITRF2014 to GDA2020 (1)`,
//! and the `ESRI` null operation between the geographic CRS is not read for an `EPSG` pair.

use proj_rs::iso19111::{
    best_pipeline, create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext,
};

#[test]
fn datum_pivot_links_geographic_crs_through_geocentric() {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input("EPSG:9000", &db).expect("source");
    let target = create_from_user_input("EPSG:7844", &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert!(operations
        .iter()
        .all(|operation| !operation.name().contains("ITRF_2014_To_GDA2020_Null")));
    assert!(operations
        .iter()
        .any(|operation| operation.name().contains("ITRF2014 to GDA2020 (1)")));
    let (definition, _) = best_pipeline("EPSG:9000", "EPSG:7844").expect("pipeline");
    assert!(definition.contains("+t_epoch=2020"), "{definition}");
}
