//! A derived projected CRS whose deriving conversion is a similarity transformation (EPSG:9621), as
//! GDAL's `test_osr_ct_fix_9732` builds it on EPSG:6453. Expected values: C PROJ 9.9.0
//! (`proj_create_crs_to_crs_from_pj`, `proj_trans`, `proj_as_wkt`).

use proj_rs::iso19111::{
    create_from_user_input, create_object_from_json, select_alternatives, BestPolicy,
    CandidateFilter, JsonObject, Selection,
};
use proj_rs::unstable::Crs;
use proj_rs::{
    CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext, ProjLines,
    WktConvention, WktFormatter, WktLines, Xyzt,
};

const GROUND: &str = include_str!("data/derived_projected/ground_idaho_west.wkt");
const GROUND_ROTATED: &str = include_str!("data/derived_projected/ground_idaho_west_rotated.wkt");
const GROUND_JSON: &str = include_str!("data/derived_projected/ground_idaho_west.json");

fn first_operation(source: &str, target: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    operations
        .first()
        .expect("one operation")
        .export_to_proj_string_with_database(&db, ProjLines::Single)
        .expect("PROJ string")
}

/// The similarity is an affine step in the unit of the grid (US survey feet), with no unit
/// conversion around it, and the projection of the base cancels out.
#[test]
fn similarity_is_an_affine_step_in_the_grid_unit() {
    assert_eq!(
        first_operation("EPSG:6453", GROUND),
        "+proj=affine +xoff=1000 +s11=1 +s12=0 +yoff=0 +s21=0 +s22=1"
    );
    assert_eq!(
        first_operation("EPSG:6453", GROUND_ROTATED),
        "+proj=affine +xoff=1234.5 +s11=0.984906233787509 +s12=0.173665542484697 +yoff=-321 \
         +s21=-0.173665542484697 +s22=0.984906233787509"
    );
}

/// GDAL hands the target over as PROJJSON: a `DerivedProjectedCRS` object reads back to the WKT
/// PROJ gives for it.
#[test]
fn derived_projected_projjson_is_read() {
    let db = DatabaseContext::create().expect("proj.db");
    let Ok(JsonObject::Crs(crs @ Crs::DerivedProjected(_))) =
        create_object_from_json(GROUND_JSON, Some(&db))
    else {
        panic!("a derived projected CRS");
    };
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.set_lines(WktLines::Single);
    assert_eq!(
        crs.export_to_wkt(&mut formatter).expect("WKT2_2019"),
        r#"DERIVEDPROJCRS["Ground for NAD83(2011) / Idaho West (ftUS)",BASEPROJCRS["NAD83(2011) / Idaho West (ftUS)",BASEGEOGCRS["NAD83(2011)",DATUM["NAD83 (National Spatial Reference System 2011)",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]]],CONVERSION["SPCS83 Idaho West zone (US Survey feet)",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Latitude of natural origin",41.6666666666667,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8801]],PARAMETER["Longitude of natural origin",-115.75,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["Scale factor at natural origin",0.999933333,SCALEUNIT["unity",1],ID["EPSG",8805]],PARAMETER["False easting",2624666.667,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8807]]],ID["EPSG",6453]],DERIVINGCONVERSION["Grid to ground",METHOD["Similarity transformation",ID["EPSG",9621]],PARAMETER["Ordinate 1 of evaluation point in target CRS",1000,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8621]],PARAMETER["Ordinate 2 of evaluation point in target CRS",0,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8622]],PARAMETER["Scale factor for source CRS axes",1,SCALEUNIT["unity",1],ID["EPSG",1061]],PARAMETER["Rotation angle of source CRS axes",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8614]]],CS[Cartesian,2],AXIS["easting (X)",east,ORDER[1],LENGTHUNIT["US survey foot",0.304800609601219]],AXIS["northing (Y)",north,ORDER[2],LENGTHUNIT["US survey foot",0.304800609601219]]]"#
    );
}

/// The candidates of the derived projected CRS to WGS 84 are prepared on the geographic base of
/// its projected base, as `extractGeodeticCRS` gives it.
#[test]
fn derived_projected_source_reaches_wgs84() {
    let selection =
        select_alternatives(GROUND, "EPSG:4326", BestPolicy::Warn, CandidateFilter::NONE)
            .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("several candidates");
    };
    let point = Xyzt {
        x: 2_301_000.0,
        y: 2_000_000.0,
        z: 0.0,
        t: 0.0,
    };
    let done = alternatives.forward(point).expect("transformed");
    assert!((done.x - 47.145_560).abs() < 1e-6, "lat {}", done.x);
    assert!((done.y + 117.054_771).abs() < 1e-6, "lon {}", done.y);
}
