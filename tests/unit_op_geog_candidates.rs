//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the candidate list between geographic CRS
//! and projected CRS: the registered CRS between two geographic CRS read with
//! `IntermediateCRSUse::ALWAYS`, and the operations that follow from it.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The expected names and PROJ
//! strings are copied from that file. The C++ `ComparePROJString` helper is mirrored by
//! [`assert_proj_eq`], except that a token with no `=` is compared exactly.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, GridAvailabilityUse, IntermediateCrsUse, ProjLines,
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

/// `TEST(operation, geogCRS_to_geogCRS_CH1903_to_CH1903plus_context)`
#[test]
fn operation_geog_crs_to_geog_crs_ch1903_to_ch1903plus_context() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
    ctx.set_intermediate_crs_use(IntermediateCrsUse::Always)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = operations(&epsg(&db, "4149"), &epsg(&db, "4150"), &ctx);
    assert_eq!(list.len(), 1);

    assert_eq!(list[0].name(), "CH1903 to CH1903+ (1)");
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline +step +proj=axisswap +order=2,1 ",
            "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
            "+step +proj=hgridshift +grids=ch_swisstopo_CHENyx06a.tif ",
            "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
            "+step +proj=axisswap +order=2,1"
        ),
    );
}

/// `TEST(operation, geogCRS_different_from_baseCRS_to_projCRS_context_compatible_area)`
#[test]
fn operation_geog_crs_different_from_base_crs_to_proj_crs_context_compatible_area() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
    ctx.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_intermediate_crs_use(IntermediateCrsUse::Always);
    // NTF(Paris) to UTM31 WGS84
    let list = operations(&epsg(&db, "4807"), &epsg(&db, "32631"), &ctx);
    assert_eq!(list.len(), 4);
    assert_eq!(
        list[0].name(),
        "NTF (Paris) to NTF (1) + Inverse of WGS 84 to NTF (3) + UTM zone 31N"
    );
    // PROJ's `coordinateOperationAccuracies()[0]->value()` is "1". The accessor
    // `accuracy_text` keeps only the text the database wrote, so the value is read here.
    assert_eq!(list[0].accuracy(), Some(1.0));
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline +step +proj=axisswap +order=2,1 +step ",
            "+proj=unitconvert +xy_in=grad +xy_out=rad +step +inv ",
            "+proj=longlat +ellps=clrk80ign +pm=paris +step +proj=hgridshift ",
            "+grids=fr_ign_ntf_r93.tif +step +proj=utm +zone=31 +ellps=WGS84"
        ),
    );
}

/// `TEST(operation, IGNF_LAMB1_TO_EPSG_4326)`
#[test]
fn operation_ignf_lamb1_to_epsg_4326() {
    let db = database();
    let mut ctx = CoordinateOperationContext::create(&db, "");
    ctx.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_intermediate_crs_use(IntermediateCrsUse::Always);
    let ignf_lamb1 = AuthorityFactory::create(&db, "IGNF")
        .create_coordinate_reference_system("LAMB1")
        .expect("IGNF:LAMB1");
    let list = operations(&ignf_lamb1, &epsg(&db, "4326"), &ctx);
    assert_eq!(list.len(), 2);

    assert!(!list[0].has_ballpark_transformation());
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline +step +inv +proj=lcc +lat_1=49.5 +lat_0=49.5 ",
            "+lon_0=0 +k_0=0.99987734 +x_0=600000 +y_0=200000 ",
            "+ellps=clrk80ign +pm=paris +step +proj=hgridshift ",
            "+grids=fr_ign_ntf_r93.tif +step +proj=unitconvert +xy_in=rad ",
            "+xy_out=deg +step +proj=axisswap +order=2,1"
        ),
    );

    assert!(!list[1].has_ballpark_transformation());
    assert_proj_eq(
        &proj(&db, &list[1]),
        concat!(
            "+proj=pipeline +step +inv +proj=lcc +lat_1=49.5 +lat_0=49.5 ",
            "+lon_0=0 +k_0=0.99987734 +x_0=600000 +y_0=200000 ",
            "+ellps=clrk80ign +pm=paris +step +proj=push +v_3 +step ",
            "+proj=cart +ellps=clrk80ign +step +proj=helmert +x=-168 +y=-60 ",
            "+z=320 +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 ",
            "+step +proj=unitconvert +xy_in=rad +xy_out=deg +step ",
            "+proj=axisswap +order=2,1"
        ),
    );

    // NTF (Paris) / Lambert Nord France equivalent to IGNF:LAMB1
    let list2 = operations(&epsg(&db, "27561"), &epsg(&db, "4326"), &ctx);
    assert!(list2.len() >= 3);

    // The second entry in list2 (list2[1]) uses the
    // weird +pm=2.33720833333333 from "NTF (Paris) to NTF (2)"
    // so skip to the 3th method
    assert_proj_eq(
        &proj(&db, &list2[0]).replace("0.999877341", "0.99987734"),
        &proj(&db, &list[0]),
    );
    assert_proj_eq(
        &proj(&db, &list2[2]).replace("0.999877341", "0.99987734"),
        &proj(&db, &list[1]),
    );
}
