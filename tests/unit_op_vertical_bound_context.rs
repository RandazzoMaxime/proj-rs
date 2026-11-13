//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, two tests that the C++ file no longer
//! compiles for the first one, and that the second one asserts with a bound CRS:
//!
//! - `TEST(operation, geog3DCRS_to_geog2DCRS_plus_vertCRS_context)` (lines 1053-1097), inside
//!   `#if no_longer_work_since_epsg_13_001`. PROJ 9.9.0 does not compile it. The expected values
//!   below are the ones the 9.9.0 library gives for the same calls (names, inverse names, PROJ
//!   strings), measured with the C++ API of `libproj.so` 9.9.0 built from the tag. The C++ names
//!   of the 2019 test (`ETRS89 to Baltic 1957 height (2)`) are the names of older EPSG releases.
//! - `TEST(operation, boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not)` (lines
//!   3423-3459): a bound CRS whose hub is the target but whose base is not, a US-foot height, grids
//!   ignored. The horizontal `deg` terms of the first leg must stay in the operation.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GeographicCRS, GridAvailabilityUse,
    ProjLines, SpatialCriterion,
};

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, geog3DCRS_to_geog2DCRS_plus_vertCRS_context)`, values of PROJ 9.9.0.
/// The context is every authority, partial intersection, grids not filtered.
#[test]
fn operation_geog3d_crs_to_geog2d_crs_plus_vert_crs_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let source = epsg(&db, "4937");
    let target = epsg(&db, "8360");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");

    assert!(list.len() >= 2, "{} candidates", list.len());

    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift +grids=cz_cuzk_CR-2005.tif +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        list[0].name(),
        "ETRS89-CZE [2007] to Baltic 1957 height (2)"
    );
    assert_eq!(
        list[0].inverse().name(),
        "Inverse of ETRS89-CZE [2007] to Baltic 1957 height (2)"
    );

    assert_eq!(
        proj(&db, &list[1]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=vgridshift +grids=sk_gku_Slovakia_ETRS89h_to_Baltic1957.tif \
         +multiplier=1 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        list[1].name(),
        "'ETRS89-SVK [SKTRF09] to Baltic 1957 height \
         (from ETRS89-SVK [SKTRF09] to ETRS89-SVK [SKTRF09] + Baltic 1957 height (1))'"
    );
    assert_eq!(
        list[1].inverse().name(),
        "Inverse of 'ETRS89-SVK [SKTRF09] to Baltic 1957 height \
         (from ETRS89-SVK [SKTRF09] to ETRS89-SVK [SKTRF09] + Baltic 1957 height (1))'"
    );
}

/// `TEST(operation, boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not)`.
#[test]
fn operation_bound_crs_to_geog_crs_hub_and_target_same_but_base_not() {
    const WKT: &str = r#"COMPD_CS["NAD83 + Ellipsoid (US Feet)",
        GEOGCS["NAD83",
            DATUM["North_American_Datum_1983",
                SPHEROID["GRS 1980",6378137,298.257222101,
                    AUTHORITY["EPSG","7019"]],
                TOWGS84[0,0,0,0,0,0,0],
                AUTHORITY["EPSG","6269"]],
            PRIMEM["Greenwich",0,
                AUTHORITY["EPSG","8901"]],
            UNIT["degree",0.0174532925199433,
                AUTHORITY["EPSG","9122"]],
            AUTHORITY["EPSG","4269"]],
        VERT_CS["Ellipsoid (US Feet)",
            VERT_DATUM["Ellipsoid",2002],
            UNIT["US survey foot",0.304800609601219,
                AUTHORITY["EPSG","9003"]],
            AXIS["Up",UP]]]"#;

    let db = DatabaseContext::create().expect("proj.db");
    let source = create_from_user_input(WKT, &db).expect("source");
    let mut context = CoordinateOperationContext::create(&db, "EPSG");
    context.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let target = Crs::Geographic(GeographicCRS::epsg_4979());
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");

    assert_eq!(list.len(), 1);
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m"
    );
}
