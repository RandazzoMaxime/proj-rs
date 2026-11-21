//! Operations through a listed intermediate CRS: `findsOpsInRegistryWithIntermediate` hands the
//! list to `createFromCRSCodesWithIntermediates`, which joins every registered operation of each
//! leg (coordinateoperationfactory.cpp:2254-2283). Expected names and order from
//! `proj_create_operations` of PROJ 9.9.0: `EPSG:4275` to `EPSG:4312`, authority `any`, CRS
//! extents not used, grids ignored, intermediate CRS `EPSG:4326` (pyproj's
//! test_transformer_group_grid_check_with_pivot_discard_missing).

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory, CrsExtentUse,
    DatabaseContext, GridAvailabilityUse,
};

#[test]
fn ntf_to_mgi_through_wgs84() {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input("EPSG:4275", &db).expect("source");
    let target = create_from_user_input("EPSG:4312", &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "any");
    context
        .set_crs_extent_use(CrsExtentUse::Ignore)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_intermediate_crs(vec![("EPSG".to_owned(), "4326".to_owned())]);
    let names: Vec<String> = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
        .iter()
        .map(|operation| operation.name().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "Inverse of WGS 84 to NTF (3) + Inverse of MGI_To_WGS_1984_2",
            "Inverse of WGS 84 to NTF (3) + Inverse of MGI to WGS 84 (3)",
            "NTF to WGS 84 (1) + Inverse of MGI_To_WGS_1984_2",
            "NTF to WGS 84 (1) + Inverse of MGI to WGS 84 (3)",
            "Inverse of WGS 84 to NTF (3) + Inverse of MGI to WGS 84 (8)",
            "Inverse of WGS 84 to NTF (3) + Inverse of MGI_To_WGS_1984_Serbia",
            "NTF to WGS 84 (1) + Inverse of MGI to WGS 84 (8)",
            "NTF to WGS 84 (1) + Inverse of MGI_To_WGS_1984_Serbia",
            "Ballpark geographic offset from NTF to MGI",
        ]
    );
}
