//! A CRS-to-CRS pair whose two ends carry a coordinate epoch. PROJ's `createOperations` changes
//! the epoch only through a point motion operation of the CRS: without one, the operation runs
//! at the source epoch, and the null geographic offset of one geographic CRS stays `proj=noop`.
//! `tests/proj_meta_facade.rs` covers two CRS. Expected values: `proj_create_crs_to_crs_from_pj`
//! of C PROJ 9.9.0 on `proj_coordinate_metadata_create` objects.

use proj_rs::{Proj, ProjCreateError};

fn at(code: &str, epoch: f64) -> Proj {
    Proj::new(code)
        .expect("CRS")
        .coordinate_metadata_create(epoch)
        .expect("coordinate metadata")
}

/// GDAL's `test_osr_ct_take_into_account_srs_coordinate_epoch`: ITRF2014 from 2020 to 2030 has
/// no point motion operation, so PROJ gives `+proj=noop`.
#[test]
fn two_epochs_without_point_motion_build_the_plain_pair() {
    for (source, target) in [
        (at("EPSG:9000", 2020.0), at("EPSG:9000", 2030.0)),
        (at("EPSG:9000", 2020.0), at("EPSG:9000", 2020.0)),
        (at("EPSG:4326", 2020.0), at("EPSG:4326", 2030.0)),
    ] {
        let transform = source
            .create_crs_to_crs_from_pj(&target, None, None)
            .expect("created");
        assert_eq!(
            transform.def().expect("definition"),
            "proj=noop ellps=GRS80"
        );
        let (x, y): (f64, f64) = transform.convert((150.0, -30.0)).expect("converted");
        assert!(
            (x - 150.0).abs() < 1e-12 && (y + 30.0).abs() < 1e-12,
            "{x} {y}"
        );
    }
}

/// NAD83(CSRS)v7 has a point motion operation: that change of epoch is not built.
#[test]
fn two_epochs_with_point_motion_stay_refused() {
    let result =
        at("EPSG:8254", 2010.0).create_crs_to_crs_from_pj(&at("EPSG:8254", 2020.0), None, None);
    assert!(matches!(result, Err(ProjCreateError::ProjError(_))));
}
