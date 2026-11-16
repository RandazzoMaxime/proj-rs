//! Characterization of the engine behaviours the C API relies on (`capi/tests/pending.txt`
//! lists the C cases they close). Each expected value is what PROJ 9.9.0 gives.

use proj_rs::unstable::{Proj, ProjBuilder};

const ITRF2014_WKT: &str = r#"GEOGCRS["ITRF2014",DYNAMIC[FRAMEEPOCH[2010]],DATUM["International Terrestrial Reference Frame 2014",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",9000]]"#;

#[test]
fn a_dynamic_geographic_crs_given_as_wkt_is_a_crs() {
    let crs = Proj::new(ITRF2014_WKT);
    assert!(crs.is_ok(), "{:?}", crs.err());
}

#[test]
fn a_crs_paired_with_itself_is_the_noop() {
    let pair = ProjBuilder::new()
        .proj_known_crs("EPSG:4284", "EPSG:4284", None)
        .expect("pair");
    assert_eq!(pair.proj_info().id.as_deref(), Some("noop"));
}

#[test]
fn a_compound_crs_with_a_bound_vertical_crs_given_as_wkt_is_a_crs() {
    let wkt = r#"COMPOUNDCRS["Compound",GEOGCRS["NAD83(2011)",DATUM["NAD83 (National Spatial Reference System 2011)",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",6318]],VERTCRS["VerticalDummyCrs",VDATUM["DummyDatum"],CS[vertical,1],AXIS["gravity-related height (H)",up,LENGTHUNIT["metre",1]]]]"#;
    assert!(Proj::new(wkt).is_ok());
}

#[test]
fn a_projected_crs_can_be_written_with_approx() {
    use proj_rs::iso19111::{create_from_user_input, Crs, DatabaseContext};
    let db = DatabaseContext::create().expect("db");
    let Crs::Projected(crs) = create_from_user_input("+proj=tmerc +type=crs", &db).expect("crs")
    else {
        panic!("not a projected CRS");
    };
    let text = crs
        .with_approx_tmerc()
        .export_to_proj_string()
        .expect("string");
    assert!(text.starts_with("+proj=tmerc +approx "), "{text}");
}

#[test]
fn a_proj_based_helmert_with_an_epoch_needs_the_time_of_each_point() {
    use proj_rs::iso19111::proj_based_operation_of_text;
    let timed = proj_based_operation_of_text("+proj=helmert +t_epoch=2010")
        .expect("operation")
        .expect("read");
    assert!(timed.requires_per_coordinate_input_time());
    let fixed = proj_based_operation_of_text("+proj=helmert")
        .expect("operation")
        .expect("read");
    assert!(!fixed.requires_per_coordinate_input_time());
}
