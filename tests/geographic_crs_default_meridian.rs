//! `proj_create_geographic_crs` with no prime meridian name and a zero offset: PROJ 9.9.0's
//! `createGeodeticReferenceFrame` names it Greenwich on the Earth and the reference meridian on
//! another body, whose name `Ellipsoid::guessBodyName` reads from the database. GDAL's `SetGeogCS`
//! builds a CRS this way. Expected values: the same call on PROJ 9.9.0's C library.

use proj_rs::{DatabaseContext, EllipsoidalCS, GeodeticComponents, UnitOfMeasure};

fn meridian_and_body(semi_major_metre: f64, inverse_flattening: f64) -> (String, String) {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = db.create_geographic_crs(
        &GeodeticComponents {
            crs_name: None,
            datum_name: None,
            ellipsoid_name: None,
            semi_major_metre,
            inverse_flattening,
            prime_meridian_name: None,
            prime_meridian_offset: 0.0,
            angular_unit_name: None,
            angular_unit_to_radian: 0.0,
        },
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    );
    let datum = crs.datum().expect("datum");
    (
        datum.prime_meridian().name().to_owned(),
        datum.ellipsoid().celestial_body().to_owned(),
    )
}

#[test]
fn an_unnamed_meridian_follows_the_body() {
    assert_eq!(
        meridian_and_body(1.0, 2.0),
        ("Reference meridian".to_owned(), "Non-Earth body".to_owned())
    );
    assert_eq!(
        meridian_and_body(6_378_137.0, 298.257_223_563),
        ("Greenwich".to_owned(), "Earth".to_owned())
    );
    assert_eq!(
        meridian_and_body(3_396_190.0, 0.0),
        ("Reference meridian".to_owned(), "Mars".to_owned())
    );
}
