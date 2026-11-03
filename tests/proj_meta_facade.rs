//! Compound CRS names in the facade, and coordinate metadata objects (`src/proj_meta.rs`).
//!
//! The EGM2008 geoid grid is not on disk in this suite, so a conversion that needs it must
//! fail. A two-dimensional source has no height, so a compound target is refused in the
//! forward direction whatever the grids.

use proj_rs::{Proj, ProjBuilder, ProjCreateError, ProjError};

#[test]
fn compound_target_creates_and_refuses_a_forward_conversion() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:4326+3855", None)
        .expect("creation must not need the geoid grid");
    let error = proj
        .convert((2.0_f64, 49.0_f64))
        .expect_err("a height from a two-dimensional source is not built");
    assert!(matches!(error, ProjError::Conversion(_)), "{error:?}");
}

#[test]
fn compound_source_to_two_dimensional_target_drops_the_height() {
    let proj = Proj::new_known_crs("EPSG:4326+3855", "EPSG:4326", None).expect("creation");
    let out = proj.convert((2.0_f64, 49.0_f64)).expect("horizontal part");
    assert_eq!(out, (2.0, 49.0));
}

#[test]
fn compound_source_to_two_dimensional_target_through_the_builder() {
    let proj = ProjBuilder::new()
        .proj_known_crs("EPSG:4326+3855", "EPSG:4326", None)
        .expect("creation");
    let out = proj.convert((2.0_f64, 49.0_f64)).expect("horizontal part");
    assert_eq!(out, (2.0, 49.0));
}

#[test]
fn compound_target_refusal_is_not_taken_by_the_inverse() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:4326+3855", None).expect("creation");
    // Inverse: the compound coordinate back to the two-dimensional source. The height is
    // dropped, which takes no value from the geoid.
    let out = proj
        .project((2.0_f64, 49.0_f64), true)
        .expect("inverse keeps the horizontal part");
    assert_eq!(out, (2.0, 49.0));
}

#[test]
fn metadata_object_keeps_the_epoch() {
    let proj = Proj::new("EPSG:4326").expect("CRS");
    let object = proj.coordinate_metadata_create(2021.3).expect("object");
    assert_eq!(object.coordinate_metadata_get_epoch(), 2021.3);
    // A CRS without an object has no epoch.
    assert!(proj.coordinate_metadata_get_epoch().is_nan());
}

#[test]
fn metadata_object_needs_a_single_crs() {
    let transform = Proj::new_known_crs("EPSG:4326", "EPSG:3857", None).expect("transform");
    assert!(matches!(
        transform.coordinate_metadata_create(2021.3),
        Err(ProjCreateError::MetadataObjectCreation)
    ));
}

#[test]
fn an_epoch_moves_itrf2014_to_itrf2008() {
    // PROJ 9.8.1 `proj_trans` of EPSG:7789 to EPSG:5332 (pyproj 3.8.0's libproj).
    // The input is geocentric metres. No epoch passes time 0. An epoch inserts
    // `proj=set v_4=<year>` and the helmert rates move the point.
    let source = Proj::new("EPSG:7789").expect("ITRF2014");
    let target = Proj::new("EPSG:5332").expect("ITRF2008");
    let plain = source
        .create_crs_to_crs_from_pj(&target, None, None)
        .expect("undated");
    let (x, y, z) = plain
        .convert_xyz(4_027_893.0, 307_045.0, 4_919_474.0)
        .expect("time 0");
    near(x, 4_027_892.758_637_508_4);
    near(y, 307_044.983_379_046_55);
    near(z, 4_919_473.906_657_333_5);

    let at_2010 = source
        .coordinate_metadata_create(2010.0)
        .expect("epoch")
        .create_crs_to_crs_from_pj(&target, None, None)
        .expect("2010");
    let (x, y, z) = at_2010
        .convert_xyz(4_027_893.0, 307_045.0, 4_919_474.0)
        .expect("2010");
    near(x, 4_027_893.001_519_442);
    near(y, 307_045.001_893_859_07);
    near(z, 4_919_474.002_301_61);
    let definition = at_2010.def().expect("definition");
    assert!(definition.contains("proj=set v_4=2010"), "{definition}");

    let at_2020 = source
        .coordinate_metadata_create(2020.0)
        .expect("epoch")
        .create_crs_to_crs_from_pj(&target, None, None)
        .expect("2020");
    let (x, y, z) = at_2020
        .convert_xyz(4_027_893.0, 307_045.0, 4_919_474.0)
        .expect("2020");
    near(x, 4_027_893.002_727_810_3);
    near(y, 307_045.001_985_972_6);
    near(z, 4_919_474.002_777_453_5);
}

#[test]
fn two_epochs_are_refused() {
    let source = Proj::new("EPSG:7789")
        .expect("source")
        .coordinate_metadata_create(2010.0)
        .expect("source epoch");
    let target = Proj::new("EPSG:5332")
        .expect("target")
        .coordinate_metadata_create(2020.0)
        .expect("target epoch");
    assert!(source
        .create_crs_to_crs_from_pj(&target, None, None)
        .is_err());
    let builder = ProjBuilder::new().proj_create_crs_to_crs_from_pj(&source, &target, None, None);
    assert!(builder.is_err());
}

#[test]
fn an_epoch_on_a_static_projected_pair_keeps_the_undated_point() {
    // Web Mercator ignores time. PROJ 9.8.1 `Transformer.from_crs(4326, 3857)`
    // of (2°, 49°) is these metres. An epoch must not move them.
    let dated = Proj::new("EPSG:4326")
        .expect("source")
        .coordinate_metadata_create(2021.3)
        .expect("epoch")
        .create_crs_to_crs_from_pj(&Proj::new("EPSG:3857").expect("target"), None, None)
        .expect("dated");
    let (x, y) = dated.convert((2.0_f64, 49.0_f64)).expect("convert");
    near(x, 222_638.981_586_547_13);
    near(y, 6_274_861.394_006_575);
}

fn near(got: f64, expect: f64) {
    assert!((got - expect).abs() <= 1e-6, "{got} vs {expect}");
}

#[test]
fn metadata_object_does_not_convert() {
    let dated = Proj::new("EPSG:4326")
        .expect("CRS")
        .coordinate_metadata_create(2021.3)
        .expect("object");
    assert!(dated.convert((2.0_f64, 49.0_f64)).is_err());
}
