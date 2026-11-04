//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, compound CRS to compound CRS through the
//! horizontal and vertical parts of each: the registry pieces for a datum change under a
//! height, and the intermediate 3D geographic CRS when the vertical operation is ballpark.
//!
//! The C++ `TEST()` names are kept in the doc comments. The expected names and PROJ strings
//! are copied from that file.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
};

fn context(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, "EPSG");
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt
}

fn parse(db: &DatabaseContext, text: &str) -> Crs {
    create_from_user_input(text, db).expect(text)
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn operations(
    source: &Crs,
    target: &Crs,
    ctxt: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, ctxt)
        .expect("operations")
}

/// `exportToPROJString(PROJStringFormatter::create())`, with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// TEST(operation, compoundCRS_to_compoundCRS_context_helmert_noop): WGS 84 + EGM96 to
/// ETRS89 + EGM96 is a no-op, the horizontal datum change being a null one.
#[test]
fn operation_compound_to_compound_context_helmert_noop() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = parse(&db, "EPSG:4326+5773");
    let dst = parse(&db, "EPSG:4258+5773");
    let list = operations(&src, &dst, &context(&db));
    assert!(!list.is_empty());
    assert_eq!(proj(&db, &list[0]), "+proj=noop");
}

/// TEST(operation, compoundCRS_to_compoundCRS_context_helmert): GDA94 + AHD height to
/// GDA2020 + AHD height goes through the 3D geographic CRS, with the vertical part unchanged.
#[test]
fn operation_compound_to_compound_context_helmert() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = parse(&db, "EPSG:9464");
    let dst = parse(&db, "EPSG:9463");
    let list = operations(&src, &dst, &context(&db));
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_GDA94_AHD_to_GDA2020_AVWS): GDA94 + AHD height
/// to GDA2020 + AVWS height, the vertical grids of both sides around the datum change.
#[test]
fn operation_compound_to_compound_gda94_ahd_to_gda2020_avws() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = epsg(&db, "9464");
    let dst = epsg(&db, "9462");
    let list = operations(&src, &dst, &context(&db));
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of 'GDA94 to GDA94 + AHD height (1)' + \
         GDA94 to GDA2020 (1) + \
         GDA2020 to AVWS height (2)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=au_ga_AUSGeoid09_V1.01.tif +multiplier=1 \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=0.06155 +y=-0.01087 +z=-0.04019 \
         +rx=-0.0394924 +ry=-0.0327221 +rz=-0.0328979 +s=-0.009994 \
         +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +inv +proj=vgridshift +grids=au_ga_AGQG_20201120.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}

/// TEST(operation, compoundCRS_to_compoundCRS_WGS84_EGM96_to_ETRS89_Belfast): EGM96 has a
/// geoid model referenced to WGS 84, and Belfast height one referenced to ETRS89.
#[test]
fn operation_compound_to_compound_wgs84_egm96_to_etrs89_belfast() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = parse(&db, "EPSG:4326+5773");
    let dst = parse(&db, "EPSG:4173+5732");
    let list = operations(&src, &dst, &context(&db));
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of WGS 84 to EGM96 height (1) + \
         Inverse of ETRS89-IRE [ETRF2000] to WGS 84 (1) + \
         ETRS89-IRE [ETRF2000] to Belfast height (2)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +inv +proj=vgridshift +grids=uk_os_OSGM15_Belfast.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
