//! PROJ 9.9.0 `test/unit/test_datum.cpp`, the 22 `TEST(datum, ...)` cases. Expected values are the
//! ones the C++ asserts. The gap rows and the justified divergences are in `docs/AUDIT-unit-core.md`.
//!
//! Two C++ checks are not ported here, both recorded in the audit: the `createUnrelatedObject()`
//! argument (a Rust `is_equivalent_to` takes the same type, so that call does not compile), and the
//! `semiMedianAxis()` assertions (PROJ 9.9.0 never writes that field).

use proj_rs::iso19111::{Ellipsoid, EllipsoidShape, JsonFormat};
use proj_rs::{
    DatabaseContext, Datum, DatumEnsemble, EngineeringDatum, Equivalence, FactoryError,
    GeodeticReferenceFrame, Measure, ParametricDatum, PrimeMeridian, TemporalDatum, UnitKind,
    UnitOfMeasure, VerticalReferenceFrame, WktConvention, WktFormatter,
    CALENDAR_PROLEPTIC_GREGORIAN,
};

const WGS84_ELLIPSOID_WKT: &str = "ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n        LENGTHUNIT[\"metre\",1],\n        ID[\"EPSG\",7030]]";

fn wkt2_2019(datum: &Datum) -> String {
    datum
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2:2019")
}

fn sphere(name: &str, radius: f64, body: &str) -> Ellipsoid {
    Ellipsoid::create(name, Measure::length(radius), EllipsoidShape::Sphere, body)
}

/// `TEST(datum, ellipsoid_from_sphere)`
#[test]
fn ellipsoid_from_sphere() {
    let ellipsoid = sphere("", 6_378_137.0, "Earth");
    assert_eq!(ellipsoid.declared_inverse_flattening(), None);
    assert!(ellipsoid.semi_minor_axis().is_none());
    assert!(ellipsoid.is_sphere());
    assert_eq!(ellipsoid.semi_major_axis(), &Measure::length(6_378_137.0));
    assert_eq!(ellipsoid.celestial_body(), "Earth");

    assert_eq!(
        ellipsoid.computed_semi_minor_axis(),
        Measure::length(6_378_137.0)
    );
    assert_eq!(ellipsoid.inverse_flattening(), 0.0);

    assert_eq!(ellipsoid.export_to_proj_string(), "+R=6378137");

    assert!(ellipsoid.is_equivalent_to(&ellipsoid));
}

/// `TEST(datum, ellipsoid_non_earth)`
#[test]
fn ellipsoid_non_earth() {
    let ellipsoid = sphere("", 1.0, "Unity sphere");
    assert_eq!(ellipsoid.celestial_body(), "Unity sphere");
}

/// `TEST(datum, ellipsoid_from_inverse_flattening)`
#[test]
fn ellipsoid_from_inverse_flattening() {
    let ellipsoid = Ellipsoid::create(
        "",
        Measure::length(6_378_137.0),
        EllipsoidShape::InverseFlattening(298.257_223_563),
        "Earth",
    );
    assert_eq!(
        ellipsoid.declared_inverse_flattening(),
        Some(298.257_223_563)
    );
    assert!(ellipsoid.semi_minor_axis().is_none());
    assert!(!ellipsoid.is_sphere());
    assert_eq!(ellipsoid.semi_major_axis(), &Measure::length(6_378_137.0));

    assert_eq!(
        ellipsoid.computed_semi_minor_axis().unit(),
        ellipsoid.semi_major_axis().unit()
    );
    assert!(
        (ellipsoid.computed_semi_minor_axis().value() - 6_356_752.314_245_18).abs() < 1e-9,
        "computed semi-minor axis"
    );
    assert_eq!(ellipsoid.inverse_flattening(), 298.257_223_563);

    assert_eq!(ellipsoid.export_to_proj_string(), "+ellps=WGS84");

    assert!(ellipsoid.is_equivalent_to(&ellipsoid));

    let two_axis = Ellipsoid::create(
        "",
        Measure::length(6_378_137.0),
        EllipsoidShape::SemiMinorAxis(Measure::length(6_356_752.314_245_18)),
        "Earth",
    );
    assert!(!ellipsoid.is_equivalent_to(&two_axis));
    assert!(ellipsoid.is_equivalent_to_with(&two_axis, Equivalence::Equivalent));

    assert!(!Ellipsoid::wgs84().is_equivalent_to_with(&Ellipsoid::grs80(), Equivalence::Equivalent));

    let sphere = sphere("", 6_378_137.0, "Earth");
    assert!(!Ellipsoid::wgs84().is_equivalent_to_with(&sphere, Equivalence::Equivalent));
    assert!(!sphere.is_equivalent_to_with(&Ellipsoid::wgs84(), Equivalence::Equivalent));
}

/// `TEST(datum, ellipsoid_from_null_inverse_flattening)`: a zero inverse flattening is a sphere.
#[test]
fn ellipsoid_from_null_inverse_flattening() {
    let ellipsoid = Ellipsoid::create(
        "",
        Measure::length(6_378_137.0),
        EllipsoidShape::InverseFlattening(0.0),
        "Earth",
    );
    assert_eq!(ellipsoid.declared_inverse_flattening(), None);
    assert!(ellipsoid.semi_minor_axis().is_none());
    assert!(ellipsoid.is_sphere());
}

/// `TEST(datum, ellipsoid_from_semi_minor_axis)`
#[test]
fn ellipsoid_from_semi_minor_axis() {
    let ellipsoid = Ellipsoid::create(
        "",
        Measure::length(6_378_137.0),
        EllipsoidShape::SemiMinorAxis(Measure::length(6_356_752.314_245_18)),
        "Earth",
    );
    assert_eq!(ellipsoid.declared_inverse_flattening(), None);
    assert!(ellipsoid.semi_minor_axis().is_some());
    assert!(!ellipsoid.is_sphere());
    assert_eq!(ellipsoid.semi_major_axis(), &Measure::length(6_378_137.0));
    assert_eq!(
        ellipsoid.semi_minor_axis(),
        Some(&Measure::length(6_356_752.314_245_18))
    );

    assert_eq!(
        ellipsoid.computed_semi_minor_axis(),
        Measure::length(6_356_752.314_245_18)
    );
    assert!((ellipsoid.inverse_flattening() - 298.257_223_563).abs() < 1e-10);

    assert_eq!(ellipsoid.export_to_proj_string(), "+ellps=WGS84");

    assert!(ellipsoid.is_equivalent_to(&ellipsoid));
    let flattened = Ellipsoid::create(
        "",
        Measure::length(6_378_137.0),
        EllipsoidShape::InverseFlattening(298.257_223_563),
        "Earth",
    );
    assert!(!ellipsoid.is_equivalent_to(&flattened));
    assert!(ellipsoid.is_equivalent_to_with(&flattened, Equivalence::Equivalent));
}

/// `TEST(datum, prime_meridian_to_PROJString)`
#[test]
fn prime_meridian_to_proj_string() {
    assert_eq!(
        PrimeMeridian::greenwich().export_to_proj_string(),
        "+proj=noop"
    );
    assert_eq!(PrimeMeridian::paris().export_to_proj_string(), "+pm=paris");
    assert_eq!(
        PrimeMeridian::create("", Measure::angle(3.5)).export_to_proj_string(),
        "+pm=3.5"
    );
    assert_eq!(
        PrimeMeridian::create("", Measure::new(100.0, UnitOfMeasure::grad()))
            .export_to_proj_string(),
        "+pm=90"
    );
    assert_eq!(
        PrimeMeridian::create("Origin meridian", Measure::angle(0.0)).export_to_proj_string(),
        "+proj=noop"
    );

    assert!(PrimeMeridian::greenwich().is_equivalent_to(&PrimeMeridian::greenwich()));
}

/// `TEST(datum, prime_meridian_to_JSON)`
#[test]
fn prime_meridian_to_json() {
    let db = DatabaseContext::create().expect("proj.db");
    let format = JsonFormat::default().with_schema("");
    assert_eq!(
        PrimeMeridian::greenwich()
            .export_to_json(&db, &format)
            .expect("PROJJSON"),
        concat!(
            "{\n",
            "  \"type\": \"PrimeMeridian\",\n",
            "  \"name\": \"Greenwich\",\n",
            "  \"longitude\": 0,\n",
            "  \"id\": {\n",
            "    \"authority\": \"EPSG\",\n",
            "    \"code\": 8901\n",
            "  }\n",
            "}"
        )
    );

    assert_eq!(
        PrimeMeridian::paris()
            .export_to_json(&db, &format)
            .expect("PROJJSON"),
        concat!(
            "{\n",
            "  \"type\": \"PrimeMeridian\",\n",
            "  \"name\": \"Paris\",\n",
            "  \"longitude\": {\n",
            "    \"value\": 2.5969213,\n",
            "    \"unit\": {\n",
            "      \"type\": \"AngularUnit\",\n",
            "      \"name\": \"grad\",\n",
            "      \"conversion_factor\": 0.015707963267949\n",
            "    }\n",
            "  },\n",
            "  \"id\": {\n",
            "    \"authority\": \"EPSG\",\n",
            "    \"code\": 8903\n",
            "  }\n",
            "}"
        )
    );
}

/// `TEST(datum, datum_with_ANCHOR)`
#[test]
fn datum_with_anchor() {
    let datum = GeodeticReferenceFrame::create(
        "WGS_1984 with anchor",
        Ellipsoid::wgs84(),
        PrimeMeridian::greenwich(),
    )
    .with_anchor("My anchor");

    assert_eq!(datum.anchor(), Some("My anchor"));
    assert_eq!(datum.anchor_epoch(), None);

    let expected = "DATUM[\"WGS_1984 with anchor\",\n    ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n        LENGTHUNIT[\"metre\",1],\n        ID[\"EPSG\",7030]],\n    ANCHOR[\"My anchor\"]]";
    assert_eq!(
        datum
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2:2015"),
        expected
    );
}

/// `TEST(datum, datum_with_ANCHOREPOCH)`
#[test]
fn datum_with_anchor_epoch() {
    let datum =
        GeodeticReferenceFrame::create("my_datum", Ellipsoid::wgs84(), PrimeMeridian::greenwich())
            .with_anchor_epoch(2002.5);

    assert_eq!(datum.anchor(), None);
    assert!((datum.anchor_epoch().expect("anchor epoch") - 2002.5).abs() < 1e-8);

    let expected = "DATUM[\"my_datum\",\n    ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n        LENGTHUNIT[\"metre\",1],\n        ID[\"EPSG\",7030]],\n    ANCHOREPOCH[2002.5]]";
    assert_eq!(
        datum
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
            .expect("WKT2:2019"),
        expected
    );
}

/// `TEST(datum, unknown_datum)`: `unknown` matches any datum under the `EQUIVALENT` criterion only.
#[test]
fn unknown_datum() {
    let datum =
        GeodeticReferenceFrame::create("my_datum", Ellipsoid::grs80(), PrimeMeridian::greenwich());
    let unknown =
        GeodeticReferenceFrame::create("unknown", Ellipsoid::grs80(), PrimeMeridian::greenwich());

    assert!(!datum.is_equivalent_to(&unknown));
    assert!(datum.is_equivalent_to_with(&unknown, Equivalence::Equivalent));
    assert!(!unknown.is_equivalent_to(&datum));
    assert!(unknown.is_equivalent_to_with(&datum, Equivalence::Equivalent));
}

/// `TEST(datum, compare_with_D_prefixing)`: an Esri `D_` prefix is ignored under `EQUIVALENT`.
#[test]
fn compare_with_d_prefixing() {
    let my_datum =
        GeodeticReferenceFrame::create("my_datum", Ellipsoid::grs80(), PrimeMeridian::greenwich());
    let d_my_datum = GeodeticReferenceFrame::create(
        "D_my_datum",
        Ellipsoid::grs80(),
        PrimeMeridian::greenwich(),
    );

    assert!(!my_datum.is_equivalent_to(&d_my_datum));
    assert!(my_datum.is_equivalent_to_with(&d_my_datum, Equivalence::Equivalent));
    assert!(!d_my_datum.is_equivalent_to(&my_datum));
    assert!(d_my_datum.is_equivalent_to_with(&my_datum, Equivalence::Equivalent));

    let d_other_datum = GeodeticReferenceFrame::create(
        "D_other_datum",
        Ellipsoid::grs80(),
        PrimeMeridian::greenwich(),
    );
    assert!(!my_datum.is_equivalent_to_with(&d_other_datum, Equivalence::Equivalent));
    assert!(!d_my_datum.is_equivalent_to_with(&d_other_datum, Equivalence::Equivalent));
    assert!(!d_other_datum.is_equivalent_to_with(&my_datum, Equivalence::Equivalent));
}

/// `TEST(datum, dynamic_geodetic_reference_frame)`: a dynamic frame is a geodetic frame with a frame
/// reference epoch and a deformation model.
#[test]
fn dynamic_geodetic_reference_frame() {
    let drf =
        GeodeticReferenceFrame::create("test", Ellipsoid::wgs84(), PrimeMeridian::greenwich())
            .with_anchor("My anchor")
            .with_frame_epoch(2018.5)
            .with_deformation_model("My model");

    let expected = "DATUM[\"test\",\n    ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n        LENGTHUNIT[\"metre\",1],\n        ID[\"EPSG\",7030]],\n    ANCHOR[\"My anchor\"]]";
    assert_eq!(
        Datum::Geodetic(drf.clone())
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2:2015"),
        expected
    );

    let expected_2019 = "DYNAMIC[\n    FRAMEEPOCH[2018.5],\n    MODEL[\"My model\"]],\nDATUM[\"test\",\n    ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n        LENGTHUNIT[\"metre\",1],\n        ID[\"EPSG\",7030]],\n    ANCHOR[\"My anchor\"]]";
    assert_eq!(wkt2_2019(&Datum::Geodetic(drf.clone())), expected_2019);

    assert!(drf.is_equivalent_to(&drf));
    assert!(drf.is_equivalent_to_with(&drf, Equivalence::Equivalent));

    // "Same" datum, except that it is a non-dynamic one.
    let datum =
        GeodeticReferenceFrame::create("test", Ellipsoid::wgs84(), PrimeMeridian::greenwich())
            .with_anchor("My anchor");
    assert!(!datum.is_equivalent_to(&drf));
    assert!(!drf.is_equivalent_to(&datum));
    assert!(datum.is_equivalent_to_with(&drf, Equivalence::Equivalent));
    assert!(drf.is_equivalent_to_with(&datum, Equivalence::Equivalent));

    let unrelated =
        GeodeticReferenceFrame::create("test2", Ellipsoid::wgs84(), PrimeMeridian::greenwich())
            .with_anchor("My anchor");
    assert!(!unrelated.is_equivalent_to(&drf));
    assert!(!drf.is_equivalent_to(&unrelated));
    assert!(!unrelated.is_equivalent_to_with(&drf, Equivalence::Equivalent));
    assert!(!drf.is_equivalent_to_with(&unrelated, Equivalence::Equivalent));
}

/// `TEST(datum, ellipsoid_to_PROJString)`
#[test]
fn ellipsoid_to_proj_string() {
    assert_eq!(Ellipsoid::wgs84().export_to_proj_string(), "+ellps=WGS84");
    assert_eq!(Ellipsoid::grs80().export_to_proj_string(), "+ellps=GRS80");

    let km = UnitOfMeasure::new("km", 1000.0, UnitKind::Linear);
    assert_eq!(
        Ellipsoid::create(
            "",
            Measure::new(10.0, km.clone()),
            EllipsoidShape::InverseFlattening(0.5),
            "Earth",
        )
        .export_to_proj_string(),
        "+a=10000 +rf=0.5"
    );
    assert_eq!(
        Ellipsoid::create(
            "",
            Measure::new(10.0, km.clone()),
            EllipsoidShape::SemiMinorAxis(Measure::new(5.0, km)),
            "Earth",
        )
        .export_to_proj_string(),
        "+a=10000 +b=5000"
    );
}

/// `TEST(datum, temporal_datum_WKT2)`
#[test]
fn temporal_datum_wkt2() {
    let datum = TemporalDatum::create(
        "Gregorian calendar",
        "0000-01-01",
        CALENDAR_PROLEPTIC_GREGORIAN,
    );

    let expected = "TDATUM[\"Gregorian calendar\",\n    TIMEORIGIN[0000-01-01]]";
    assert_eq!(
        datum
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2:2015"),
        expected
    );

    assert!(matches!(
        datum.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal)),
        Err(FactoryError::Format(_))
    ));

    assert!(datum.is_equivalent_to(&datum));
}

/// `TEST(datum, temporal_datum_time_origin_non_ISO8601)`: a time origin that is not ISO 8601 is quoted.
#[test]
fn temporal_datum_time_origin_non_iso8601() {
    let datum = TemporalDatum::create(
        "Gregorian calendar",
        "0001 January 1st",
        CALENDAR_PROLEPTIC_GREGORIAN,
    );

    let expected = "TDATUM[\"Gregorian calendar\",\n    TIMEORIGIN[\"0001 January 1st\"]]";
    assert_eq!(
        datum
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2:2015"),
        expected
    );
}

/// `TEST(datum, temporal_datum_WKT2_2019)`
#[test]
fn temporal_datum_wkt2_2019() {
    let datum = TemporalDatum::create(
        "Gregorian calendar",
        "0000-01-01",
        CALENDAR_PROLEPTIC_GREGORIAN,
    );

    let expected = "TDATUM[\"Gregorian calendar\",\n    CALENDAR[\"proleptic Gregorian\"],\n    TIMEORIGIN[0000-01-01]]";
    assert_eq!(
        datum
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
            .expect("WKT2:2019"),
        expected
    );
}

/// `TEST(datum, dynamic_vertical_reference_frame)`
#[test]
fn dynamic_vertical_reference_frame() {
    let drf = VerticalReferenceFrame::named("test")
        .with_anchor("My anchor")
        .with_frame_epoch(2018.5)
        .with_deformation_model("My model");

    let expected = "VDATUM[\"test\",\n    ANCHOR[\"My anchor\"]]";
    assert_eq!(
        Datum::Vertical(drf.clone())
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2:2015"),
        expected
    );

    let expected_2019 = "DYNAMIC[\n    FRAMEEPOCH[2018.5],\n    MODEL[\"My model\"]],\nVDATUM[\"test\",\n    ANCHOR[\"My anchor\"]]";
    assert_eq!(wkt2_2019(&Datum::Vertical(drf.clone())), expected_2019);

    assert!(drf.is_equivalent_to(&drf));
    assert!(drf.is_equivalent_to_with(&drf, Equivalence::Equivalent));

    // "Same" datum, except that it is a non-dynamic one.
    let datum = VerticalReferenceFrame::named("test").with_anchor("My anchor");
    assert!(!datum.is_equivalent_to(&drf));
    assert!(!drf.is_equivalent_to(&datum));
    assert!(datum.is_equivalent_to_with(&drf, Equivalence::Equivalent));
    assert!(drf.is_equivalent_to_with(&datum, Equivalence::Equivalent));

    let unrelated = VerticalReferenceFrame::named("test2").with_anchor("My anchor");
    assert!(!unrelated.is_equivalent_to(&drf));
    assert!(!drf.is_equivalent_to(&unrelated));
    assert!(!unrelated.is_equivalent_to_with(&drf, Equivalence::Equivalent));
    assert!(!drf.is_equivalent_to_with(&unrelated, Equivalence::Equivalent));
}

/// `TEST(datum, datum_ensemble)`
#[test]
fn datum_ensemble() {
    let other_datum = GeodeticReferenceFrame::create(
        "other datum",
        Ellipsoid::wgs84(),
        PrimeMeridian::greenwich(),
    );
    let ensemble = DatumEnsemble::create(
        "test",
        vec![
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
            Datum::Geodetic(other_datum),
        ],
        100.0,
    )
    .expect("ensemble");
    assert_eq!(ensemble.members().len(), 2);
    assert_eq!(ensemble.positional_accuracy(), 100.0);

    let wkt = DatumEnsemble::export_to_wkt(
        &ensemble,
        &mut WktFormatter::create(WktConvention::Wkt2_2019),
    )
    .expect("WKT2:2019");
    assert_eq!(
        wkt,
        "ENSEMBLE[\"test\",\n    MEMBER[\"World Geodetic System 1984\",\n        ID[\"EPSG\",6326]],\n    MEMBER[\"other datum\"],\n    ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n        LENGTHUNIT[\"metre\",1],\n        ID[\"EPSG\",7030]],\n    ENSEMBLEACCURACY[100]]"
    );

    let wkt_2015 = DatumEnsemble::export_to_wkt(
        &ensemble,
        &mut WktFormatter::create(WktConvention::Wkt2_2015),
    )
    .expect("WKT2:2015");
    assert_eq!(
        wkt_2015,
        format!("DATUM[\"test\",\n    {WGS84_ELLIPSOID_WKT}]")
    );
}

/// `TEST(datum, datum_ensemble_vertical)`
#[test]
fn datum_ensemble_vertical() {
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Vertical(VerticalReferenceFrame::named("vdatum1")),
            Datum::Vertical(VerticalReferenceFrame::named("vdatum2")),
        ],
        100.0,
    )
    .expect("ensemble");

    let wkt = DatumEnsemble::export_to_wkt(
        &ensemble,
        &mut WktFormatter::create(WktConvention::Wkt2_2019),
    )
    .expect("WKT2:2019");
    assert_eq!(
        wkt,
        "ENSEMBLE[\"unnamed\",\n    MEMBER[\"vdatum1\"],\n    MEMBER[\"vdatum2\"],\n    ENSEMBLEACCURACY[100]]"
    );
}

/// `TEST(datum, datum_ensemble_exceptions)`: an ensemble needs two members of one kind, with the same
/// ellipsoid and prime meridian.
#[test]
fn datum_ensemble_exceptions() {
    let wgs84 = || Datum::Geodetic(GeodeticReferenceFrame::epsg_6326());
    let vdatum = Datum::Vertical(VerticalReferenceFrame::named("vdatum1"));

    // No datum.
    assert!(DatumEnsemble::create("", Vec::new(), 100.0).is_err());

    // Single datum.
    assert!(DatumEnsemble::create("", vec![wgs84()], 100.0).is_err());

    // Different datum type, both orders.
    assert!(DatumEnsemble::create("", vec![wgs84(), vdatum.clone()], 100.0).is_err());
    assert!(DatumEnsemble::create("", vec![vdatum, wgs84()], 100.0).is_err());

    // Different ellipsoid.
    assert!(DatumEnsemble::create(
        "",
        vec![
            wgs84(),
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6267()),
        ],
        100.0,
    )
    .is_err());

    // Different prime meridian.
    assert!(DatumEnsemble::create(
        "",
        vec![
            wgs84(),
            Datum::Geodetic(GeodeticReferenceFrame::create(
                "other datum",
                Ellipsoid::wgs84(),
                PrimeMeridian::paris(),
            )),
        ],
        100.0,
    )
    .is_err());
}

/// `TEST(datum, edatum)`
#[test]
fn edatum() {
    let datum = EngineeringDatum::create("Engineering datum").with_anchor("my anchor");
    let expected = "EDATUM[\"Engineering datum\",\n    ANCHOR[\"my anchor\"]]";
    assert_eq!(
        Datum::Engineering(datum)
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2:2015"),
        expected
    );
}

/// `TEST(datum, pdatum)`
#[test]
fn pdatum() {
    let datum = ParametricDatum::create("Parametric datum").with_anchor("my anchor");
    let expected = "PDATUM[\"Parametric datum\",\n    ANCHOR[\"my anchor\"]]";
    assert_eq!(
        datum
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2:2015"),
        expected
    );
}
