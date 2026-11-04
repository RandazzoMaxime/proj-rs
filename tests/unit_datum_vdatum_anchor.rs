//! PROJ 9.9.0 `test/unit/test_crs.cpp`, `VerticalReferenceFrame` with an anchor and a
//! realization method.

use proj_rs::{
    Datum, GeodeticReferenceFrame, RealizationMethod, VerticalReferenceFrame, WktConvention,
    WktFormatter,
};

/// `TEST(datum, vdatum_with_anchor)`
#[test]
fn datum_vdatum_with_anchor() {
    let vdatum = VerticalReferenceFrame::named("Ordnance Datum Newlyn")
        .with_identifier("EPSG", "5101")
        .with_anchor("my anchor")
        .with_realization_method(RealizationMethod::Levelling);
    assert_eq!(
        vdatum.realization_method(),
        Some(RealizationMethod::Levelling)
    );

    let expected =
        "VDATUM[\"Ordnance Datum Newlyn\",\n    ANCHOR[\"my anchor\"],\n    ID[\"EPSG\",5101]]";
    let wkt = Datum::Vertical(vdatum.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt");
    assert_eq!(wkt, expected);

    assert!(vdatum.is_equivalent_to(&vdatum));
    assert!(!vdatum.is_equivalent_to_datum(&Datum::Geodetic(GeodeticReferenceFrame::epsg_6326())));
}
