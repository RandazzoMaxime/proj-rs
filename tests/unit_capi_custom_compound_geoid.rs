//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: a custom compound CRS whose vertical part is a bound
//! vertical CRS on the `egm96_15.gtx` geoid grid, to `WGS 84 (G1762)` (`EPSG:7665`).
//!
//! The compound is given to the pipeline selection as its WKT2 text, since the facade takes CRS
//! text and not objects. The C test passes the compound object to `proj_create_crs_to_crs_from_pj`.
//! The `t` of each input is not used by these static CRS.

mod common;

use proj_rs::iso19111::{
    best_pipeline, AuthorityFactory, BoundCRS, CompoundCRS, CoordinateOperation, Crs,
    DatabaseContext, OpProps, OperationParameter, ParameterValue, Transformation, UnitOfMeasure,
    VerticalCRS, VerticalCS, VerticalReferenceFrame, WktConvention, WktFormatter,
};
use proj_rs::Proj;

fn near(got: f64, expect: f64, tol: f64) {
    assert!((got - expect).abs() <= tol, "{got} vs {expect} (tol {tol})");
}

/// `proj_crs_create_bound_vertical_crs(vertical, hub, grid)`: the bound CRS of a vertical CRS to
/// `hub`, through the geoid model file `grid`.
fn bound_vertical_crs(vertical: VerticalCRS, hub: Crs, grid: &str) -> Crs {
    let transformation = Transformation::create(
        &OpProps::new().name("unknown to WGS 84 ellipsoidal height"),
        Crs::Vertical(vertical.clone()),
        hub.clone(),
        None,
        "GravityRelatedHeight to Geographic3D",
        vec![OperationParameter::create(
            "Geoid (height correction) model file",
        )],
        vec![ParameterValue::filename(grid)],
        None,
    )
    .expect("geoid transformation");
    Crs::Bound(BoundCRS::create(
        Crs::Vertical(vertical),
        hub,
        CoordinateOperation::Transformation(transformation),
    ))
}

fn dummy_vertical_crs() -> VerticalCRS {
    VerticalCRS::create(
        "VerticalDummyCrs",
        VerticalReferenceFrame::named("DummyDatum"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
}

/// The pipeline `cs2cs` takes for the pair, read back as a transform. Its axes are the CRS axes
/// as they are, not normalized for visualization: `EPSG:7665` is latitude first.
fn pipeline_for(source: &str) -> Proj {
    let (definition, _lat_first) = best_pipeline(source, "EPSG:7665").expect("pipeline");
    Proj::new(&definition).expect("pipeline text")
}

/// The compound CRS as WKT2 text.
fn compound_text(compound: &CompoundCRS, db: &DatabaseContext) -> String {
    Crs::Compound(compound.clone())
        .export_to_wkt(
            &mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db.clone()),
        )
        .expect("WKT2 of the compound")
}

/// `TEST_F(CApi, proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_WGS84_to_WGS84_G1762)`
///
/// NAD83(2011) / UTM zone 11N (EPSG:6340) with the EGM96 geoid height, referenced against
/// WGS 84. PROJ finds a transformation from NAD83(2011) to WGS 84 for the horizontal part, which
/// has six candidates in US subzones and a ballpark one. The point is in California.
#[test]
fn capi_proj_create_crs_to_crs_custom_compound_geoidgrid_ref_against_wgs84() {
    common::use_fixture_search_path();
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let in_crs_h = epsg
        .create_coordinate_reference_system("6340")
        .expect("6340");
    let crs_4979 = epsg
        .create_coordinate_reference_system("4979")
        .expect("4979");
    let in_crs_v = bound_vertical_crs(dummy_vertical_crs(), crs_4979, "egm96_15.gtx");
    let in_compound = CompoundCRS::create("Compound", vec![in_crs_h, in_crs_v]).expect("compound");

    let proj = pipeline_for(&compound_text(&in_compound, &db));
    let (x, y, z) = proj
        .convert_xyz(350_499.911, 3_884_807.956, 150.072)
        .expect("point");
    near(x, 35.094_993_072_71, 1e-9);
    near(y, -118.640_148_689_21, 1e-9);
    near(z, 117.655, 1e-3);
}
