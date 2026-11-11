//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: a compound CRS with a non-metre height given
//! to a PROJ string CRS, and the IGNF `init=` pair under the PROJ 4 rules. Expected values are
//! copied from that file; none is taken from this crate's output.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, create_from_user_input_with_init_rules,
    CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory, Crs,
    DatabaseContext, InitRules, ProjLines, SpatialCriterion,
};

/// `createOperations` with the partial-intersection criterion, as the C++ tests set it.
fn partial_operations(
    db: &DatabaseContext,
    authority: &str,
    source: &Crs,
    target: &Crs,
) -> Vec<CoordinateOperation> {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    CoordinateOperationFactory::create()
        .create_operations(source, target, &ctxt)
        .expect("operations")
}

/// `exportToPROJString` of an operation.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_to_proj_string_with_non_metre_height)`
///
/// `EPSG:6318+5703` (NAD83(2011) + NAVD88 height in US survey feet) to a PROJ string
/// geographic CRS with `+vunits=us-ft` and no identifier. The first operation must write the
/// vertical unit change once, in the last `unitconvert`.
#[test]
fn operation_compound_to_proj_string_with_non_metre_height() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input("EPSG:6318+5703", &db).expect("EPSG:6318+5703");
    let dst = create_crs_from_proj_string("+proj=longlat +ellps=GRS80 +vunits=us-ft +type=crs")
        .expect("PROJ string CRS");

    let list = partial_operations(&db, "EPSG", &src, &dst);
    assert!(list.len() > 1, "{} operations", list.len());
    // What is important to check here is the vertical unit conversion
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_noaa_g2018u0.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=us-ft"
    );
}

/// `TEST(operation, geogCRS_to_geogCRS_init_IGNF_to_init_IGNF_context)`
///
/// `+init=IGNF:NTFG` to `+init=IGNF:RGF93G` under the PROJ 4 init rules
/// (`PROJStringParser().setUsePROJ4InitRules(true)`, here `create_from_user_input_with_init_rules`
/// with `InitRules::Proj4`), operations with an empty authority name.
#[test]
fn operation_geog_crs_to_geog_crs_init_ignf_to_init_ignf_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input_with_init_rules("+init=IGNF:NTFG", &db, InitRules::Proj4)
        .expect("+init=IGNF:NTFG");
    let dst = create_from_user_input_with_init_rules("+init=IGNF:RGF93G", &db, InitRules::Proj4)
        .expect("+init=IGNF:RGF93G");

    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 2, "{} operations", list.len());

    assert_eq!(
        list[0].name(),
        "NOUVELLE TRIANGULATION DE LA FRANCE (NTF) vers RGF93 (ETRS89)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=hgridshift +grids=fr_ign_ntf_r93.tif +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}
