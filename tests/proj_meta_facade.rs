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
fn metadata_object_as_source_or_target_is_refused() {
    let plain = Proj::new("EPSG:4326").expect("CRS");
    let dated = plain.coordinate_metadata_create(2021.3).expect("object");
    let target = Proj::new("EPSG:3857").expect("CRS");
    assert!(dated
        .create_crs_to_crs_from_pj(&target, None, None)
        .is_err());
    assert!(plain.create_crs_to_crs_from_pj(&dated, None, None).is_err());
    let builder = ProjBuilder::new().proj_create_crs_to_crs_from_pj(&dated, &target, None, None);
    assert!(builder.is_err());
}

#[test]
fn metadata_object_does_not_convert() {
    let dated = Proj::new("EPSG:4326")
        .expect("CRS")
        .coordinate_metadata_create(2021.3)
        .expect("object");
    assert!(dated.convert((2.0_f64, 49.0_f64)).is_err());
}
