//! PROJ `test/unit/test_operation.cpp` transformation constructors: the Helmert
//! family, the Molodensky pair, and the Transformation inverse and WKT.
//!
//! Each function is one `TEST()` name. Expected PROJ strings, WKT and parameter
//! values are copied from PROJ 9.9.0 unchanged. Multi-line WKT is built with
//! `concat!` and `\n`, so the text does not depend on the line endings of this
//! file. The expected values are compared exactly, as PROJ does.

use proj_rs::iso19111::{
    AuthorityFactory, CartesianCS, CoordinateSystem, Crs, DatabaseContext, GeodeticCRS, ProjLines,
};
use proj_rs::{OpProps, Transformation, UnitKind, UnitOfMeasure};

fn props() -> OpProps {
    OpProps::new()
}

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

/// A CRS of the EPSG authority, as PROJ's `GeographicCRS::EPSG_*` constants.
fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

/// PROJ string of a transformation, on one line as the PROJ tests expect.
fn proj(transformation: &Transformation) -> String {
    transformation
        .export_to_proj_string(ProjLines::Single)
        .expect("proj")
}

#[test]
fn operation_transformation_create_geocentric_translations() {
    let db = database();
    let transf = Transformation::create_geocentric_translations(
        &props(),
        epsg(&db, "4269"),
        epsg(&db, "4326"),
        1.0,
        2.0,
        3.0,
        None,
    )
    .expect("geocentric translations");
    assert_eq!(
        transf.towgs84_parameters().expect("towgs84"),
        Some([1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0])
    );

    let inv_transf = transf.inverse();
    assert_eq!(
        transf.source_crs().map(Crs::name),
        inv_transf.target_crs().map(Crs::name)
    );
    assert_eq!(
        transf.target_crs().map(Crs::name),
        inv_transf.source_crs().map(Crs::name)
    );
    assert_eq!(
        inv_transf.towgs84_parameters().expect("towgs84"),
        Some([-1.0, -2.0, -3.0, 0.0, 0.0, 0.0, 0.0])
    );

    assert_eq!(
        proj(&transf),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 +step +proj=helmert +x=1 +y=2 \
         +z=3 +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg +step \
         +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_create_geocentric_translations_null() {
    let db = database();
    let transf = Transformation::create_geocentric_translations(
        &props(),
        epsg(&db, "4328"),
        epsg(&db, "4328"),
        0.0,
        0.0,
        0.0,
        None,
    )
    .expect("geocentric translations");
    assert_eq!(proj(&transf.inverse()), "+proj=noop");
}

#[test]
fn operation_transformation_create_geocentric_translations_neg_zero() {
    let db = database();
    let transf = Transformation::create_geocentric_translations(
        &props(),
        epsg(&db, "4328"),
        epsg(&db, "4328"),
        1.0,
        -0.0,
        0.0,
        None,
    )
    .expect("geocentric translations");
    assert_eq!(proj(&transf.inverse()), "+proj=helmert +x=-1 +y=0 +z=0");
}

/// A geocentric CRS on WGS 84 in kilometres, PROJ's `createGeocentricKM()`.
fn geocentric_km(db: &DatabaseContext) -> Crs {
    let datum = AuthorityFactory::create(db, "EPSG")
        .create_geodetic_datum("6326")
        .expect("6326");
    let cs =
        CartesianCS::create_geocentric(UnitOfMeasure::new("kilometre", 1000.0, UnitKind::Linear));
    Crs::Geodetic(
        GeodeticCRS::create(
            "Based on WGS 84",
            Some(datum),
            None,
            CoordinateSystem::Cartesian(cs),
        )
        .expect("geocentric km"),
    )
}

#[test]
fn operation_transformation_create_geocentric_translations_between_geocentric_crs() {
    let db = database();
    let transf1 = Transformation::create_geocentric_translations(
        &props(),
        epsg(&db, "4328"),
        geocentric_km(&db),
        1.0,
        2.0,
        3.0,
        None,
    )
    .expect("transf1");
    assert_eq!(
        proj(&transf1),
        "+proj=pipeline +step +proj=helmert +x=1 +y=2 +z=3 +step \
         +proj=unitconvert +xy_in=m +z_in=m +xy_out=km +z_out=km"
    );

    let transf2 = Transformation::create_geocentric_translations(
        &props(),
        geocentric_km(&db),
        epsg(&db, "4328"),
        1.0,
        2.0,
        3.0,
        None,
    )
    .expect("transf2");
    assert_eq!(
        proj(&transf2),
        "+proj=pipeline +step +proj=unitconvert +xy_in=km +z_in=km \
         +xy_out=m +z_out=m +step +proj=helmert +x=1 +y=2 +z=3"
    );

    let transf3 = Transformation::create_geocentric_translations(
        &props(),
        geocentric_km(&db),
        geocentric_km(&db),
        1.0,
        2.0,
        3.0,
        None,
    )
    .expect("transf3");
    assert_eq!(
        proj(&transf3),
        "+proj=pipeline +step +proj=unitconvert +xy_in=km +z_in=km \
         +xy_out=m +z_out=m +step +proj=helmert +x=1 +y=2 +z=3 +step \
         +proj=unitconvert +xy_in=m +z_in=m +xy_out=km +z_out=km"
    );
}

#[test]
fn operation_transformation_create_position_vector() {
    let db = database();
    let transf = Transformation::create_position_vector(
        &props(),
        epsg(&db, "4269"),
        epsg(&db, "4326"),
        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        Some(100.0),
    )
    .expect("position vector");
    assert_eq!(transf.accuracy(), Some(100.0));
    assert_eq!(
        transf.towgs84_parameters().expect("towgs84"),
        Some([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0])
    );
    assert_eq!(
        proj(&transf),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 +step +proj=helmert +x=1 +y=2 \
         +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector +step \
         +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );

    let inv_transf = transf.inverse();
    assert_eq!(inv_transf.accuracy(), Some(100.0));
    assert_eq!(
        proj(&inv_transf),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 +step \
         +proj=cart +ellps=WGS84 +step +inv +proj=helmert +x=1 +y=2 +z=3 +rx=4 \
         +ry=5 +rz=6 +s=7 +convention=position_vector +step +inv +proj=cart \
         +ellps=GRS80 +step +proj=pop +v_3 +step +proj=unitconvert +xy_in=rad \
         +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn operation_transformation_create_coordinate_frame_rotation() {
    let db = database();
    let transf = Transformation::create_coordinate_frame_rotation(
        &props(),
        epsg(&db, "4269"),
        epsg(&db, "4326"),
        [1.0, 2.0, 3.0, -4.0, -5.0, -6.0, 7.0],
        None,
    )
    .expect("coordinate frame rotation");
    assert_eq!(
        transf.towgs84_parameters().expect("towgs84"),
        Some([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0])
    );
    assert_eq!(
        proj(&transf),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 +step +proj=helmert +x=1 +y=2 \
         +z=3 +rx=-4 +ry=-5 +rz=-6 +s=7 +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 +step \
         +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap \
         +order=2,1"
    );
    let inv_transf = transf.inverse();
    assert_eq!(inv_transf.accuracy(), None);
    assert_eq!(
        proj(&inv_transf),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 +step \
         +proj=cart +ellps=WGS84 +step +inv +proj=helmert +x=1 +y=2 +z=3 \
         +rx=-4 +ry=-5 +rz=-6 +s=7 +convention=coordinate_frame +step +inv \
         +proj=cart +ellps=GRS80 +step +proj=pop +v_3 +step +proj=unitconvert \
         +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}
