//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: a vertical CRS with a geoid model (`GEOID_MODEL`) and
//! an accuracy, compounded with a horizontal CRS and used to find the operations to a 3D
//! geographic CRS. The C++ `TEST()` names are kept in the doc comments; the names, PROJ strings
//! and values are copied from that file.

mod common;

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CompoundCRS, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext, GeographicCRS,
    GeoidModel, GridAvailabilityUse, ProjLines, SpatialCriterion, UnitKind, UnitOfMeasure,
    VerticalCRS, VerticalCS, VerticalReferenceFrame,
};

/// The operation context of the three tests: no authority (`nullptr`), grid availability
/// ignored, and the partial intersection criterion.
fn context(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
}

/// `"US survey foot"` with the factor of the C test.
fn us_survey_foot() -> UnitOfMeasure {
    UnitOfMeasure::new("US survey foot", 0.304_800_609_601_219, UnitKind::Linear)
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn geographic(crs: Crs) -> GeographicCRS {
    match crs {
        Crs::Geographic(geog) => geog,
        _ => panic!("not a geographic CRS"),
    }
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

/// `proj_as_proj_string(PJ_PROJ_5)`: the single-line PROJ string, with the database for the
/// grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// The pipeline of the first two tests, from NAD83(2011) / UTM zone 11N and the vertical CRS in
/// US survey foot to NAD83(2011) 3D.
const PIPELINE_UTM11_FTUS_GEOID: &str = "+proj=pipeline \
    +step +inv +proj=utm +zone=11 +ellps=GRS80 \
    +step +proj=unitconvert +z_in=us-ft +z_out=m \
    +step +proj=vgridshift +grids=@foo.gtx +multiplier=1 \
    +step +proj=unitconvert +xy_in=rad +xy_out=deg \
    +step +proj=axisswap +order=2,1";

/// `TEST_F(CApi, proj_create_vertical_crs_ex)`: `myVertCRS (ftUS)` on `myVertDatum`, with the
/// geoid model `PROJ @foo.gtx` and `ACCURACY=123`, compounded with EPSG:6340, and the operations
/// to NAD83(2011) 3D (EPSG:6319).
#[test]
fn capi_proj_create_vertical_crs_ex() {
    common::use_fixture_search_path();
    let db = DatabaseContext::create().expect("proj.db");
    let horiz_crs = epsg(&db, "6340");
    let vert_crs = VerticalCRS::create(
        "myVertCRS (ftUS)",
        VerticalReferenceFrame::named("myVertDatum"),
        VerticalCS::create_gravity_related_height(us_survey_foot()),
    )
    .with_geoid_model(GeoidModel::create("PROJ @foo.gtx").with_accuracy(123.0));
    let compound = Crs::Compound(
        CompoundCRS::create("Compound", vec![horiz_crs, Crs::Vertical(vert_crs)])
            .expect("compound"),
    );
    let geog_crs = epsg(&db, "6319");

    let ops = operations(&compound, &geog_crs, &context(&db));
    assert!(!ops.is_empty());
    let operation = &ops[0];
    assert_eq!(
        operation.name(),
        "Inverse of UTM zone 11N + \
         Conversion from myVertCRS (ftUS) to myVertCRS + \
         Transformation from myVertCRS to NAD83(2011)"
    );
    assert_eq!(proj(&db, operation), PIPELINE_UTM11_FTUS_GEOID);
    assert_eq!(operation.accuracy(), Some(123.0));
}

/// `TEST_F(CApi, proj_create_vertical_crs_ex_with_geog_crs)`: the same vertical CRS, named
/// `myVertCRS`, with the geographic CRS WGS 84 (EPSG:4979) as the interpolation CRS of its
/// geoid model. The compound is also exported to PROJJSON and read back.
#[test]
fn capi_proj_create_vertical_crs_ex_with_geog_crs() {
    common::use_fixture_search_path();
    let db = DatabaseContext::create().expect("proj.db");
    let horiz_crs = epsg(&db, "6340");
    let wgs84 = geographic(epsg(&db, "4979"));
    let vert_crs = VerticalCRS::create(
        "myVertCRS",
        VerticalReferenceFrame::named("myVertDatum"),
        VerticalCS::create_gravity_related_height(us_survey_foot()),
    )
    .with_geoid_model(GeoidModel::create("PROJ @foo.gtx").with_interpolation_crs(wgs84));
    let compound = Crs::Compound(
        CompoundCRS::create("Compound", vec![horiz_crs, Crs::Vertical(vert_crs)])
            .expect("compound"),
    );
    let geog_crs = epsg(&db, "6319");
    let ctxt = context(&db);

    let ops = operations(&compound, &geog_crs, &ctxt);
    assert!(!ops.is_empty());
    let operation = &ops[0];
    let name = operation.name().to_owned();
    assert_eq!(
        name,
        "Inverse of UTM zone 11N + \
         Conversion from myVertCRS to myVertCRS (metre) + \
         Transformation from myVertCRS (metre) to WGS 84 using NAD83(2011) to WGS 84 (1)"
    );
    let proj_string = proj(&db, operation);
    assert_eq!(proj_string, PIPELINE_UTM11_FTUS_GEOID);

    // The same results after an export of the compound CRS to PROJJSON and a re-import.
    let projjson = compound.export_to_json(&db).expect("PROJJSON");
    let compound_from_projjson = create_from_user_input(&projjson, &db).expect("PROJJSON import");
    let ops_again = operations(&compound_from_projjson, &geog_crs, &ctxt);
    assert!(!ops_again.is_empty());
    assert_eq!(ops_again[0].name(), name);
    assert_eq!(proj(&db, &ops_again[0]), proj_string);
}

/// `TEST_F(CApi, proj_create_vertical_crs_ex_implied_accuracy)`: the AHD height (EPSG:5711) with
/// the geoid model `PROJ au_ga_AUSGeoid09_V1.01.tif`, whose interpolation CRS is GDA94 3D
/// (EPSG:4939), compounded with GDA94 (EPSG:4283). No accuracy is given: the one of the
/// operations registered for the grid, 0.15 m (EPSG:5656 / 5657), is implied.
#[test]
fn capi_proj_create_vertical_crs_ex_implied_accuracy() {
    common::use_fixture_search_path();
    let db = DatabaseContext::create().expect("proj.db");
    let crs_h = epsg(&db, "4283");
    let Crs::Vertical(crs_v) = epsg(&db, "5711") else {
        panic!("EPSG:5711 is not a vertical CRS");
    };
    // `proj_crs_get_datum` with its name and its first authority identifier.
    let datum = crs_v.datum().expect("vertical datum");
    let mut frame = VerticalReferenceFrame::named(datum.name());
    if let Some(id) = datum.identifiers().first() {
        frame = frame.with_identifier(id.code_space(), id.code());
    }
    let geog_3d = geographic(epsg(&db, "4939"));
    let vert_crs = VerticalCRS::create(
        "Vertical",
        frame,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .with_geoid_model(
        GeoidModel::create("PROJ au_ga_AUSGeoid09_V1.01.tif")
            .with_interpolation_crs(geog_3d.clone()),
    );
    let compound = Crs::Compound(
        CompoundCRS::create("Compound with geoid", vec![crs_h, Crs::Vertical(vert_crs)])
            .expect("compound"),
    );

    let ops = operations(&compound, &Crs::Geographic(geog_3d), &context(&db));
    assert!(!ops.is_empty());
    let operation = &ops[0];
    // This is the accuracy of operations EPSG:5656 / 5657.
    let accuracy = operation.accuracy().expect("accuracy");
    assert!((accuracy - 0.15).abs() <= 1e-10, "accuracy {accuracy}");
    // The operation has an associated area of use.
    let domain = operation.domains().first().expect("area of use");
    let north = domain.north_bound_latitude().expect("north bound");
    let west = domain.west_bound_longitude().expect("west bound");
    assert!(north <= -10.0, "north {north}");
    assert!(west >= 110.0, "west {west}");
}
