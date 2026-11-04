//! PROJ 9.9.0 `test/unit/test_c_api_990.cpp`, `proj_create_operations_with_pivot`: ED50 (EPSG:4230)
//! to ETRS89-FRA [RGF93 v1] (EPSG:4171) has no direct transformation. The default context pivots
//! through ETRS89, a `NEVER` context does not, and a pivot list is honoured under `ALWAYS`.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    IntermediateCrsUse,
};

fn names(
    authority: &str,
    use_pivot: Option<IntermediateCrsUse>,
    pivots: &[(&str, &str)],
) -> Vec<String> {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let source = epsg
        .create_coordinate_reference_system("4230")
        .expect("4230");
    let target = epsg
        .create_coordinate_reference_system("4171")
        .expect("4171");
    let mut ctxt = CoordinateOperationContext::create(&db, authority);
    if let Some(policy) = use_pivot {
        ctxt.set_intermediate_crs_use(policy);
    }
    ctxt.set_intermediate_crs(
        pivots
            .iter()
            .map(|(auth, code)| (auth.to_string(), code.to_string()))
            .collect(),
    );
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &ctxt)
        .expect("operations")
        .iter()
        .map(|op| op.name().to_owned())
        .collect()
}

/// `TEST_F(CApi, proj_create_operations_with_pivot)`
#[test]
fn capi_proj_create_operations_with_pivot() {
    assert_eq!(
        names("", None, &[]),
        ["ED50 to ETRS89 (10) + ETRS89 to ETRS89-FRA [RGF93 v1]"]
    );
    assert_eq!(
        names("", Some(IntermediateCrsUse::Never), &[]),
        ["Ballpark geographic offset from ED50 to ETRS89-FRA [RGF93 v1]"]
    );
    assert_eq!(
        names("", None, &[("EPSG", "4258")]),
        ["ED50 to ETRS89 (10) + ETRS89 to ETRS89-FRA [RGF93 v1]"]
    );
    assert_eq!(
        names("any", Some(IntermediateCrsUse::Always), &[("EPSG", "4267")]),
        ["Ballpark geographic offset from ED50 to ETRS89-FRA [RGF93 v1]"]
    );
}
