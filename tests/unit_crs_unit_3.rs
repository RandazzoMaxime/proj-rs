//! PROJ 9.9.0 `test/unit/test_crs.cpp`, lot 3: `boundCRS_crs_link` and
//! `VerticalCRS_ensemble_exception_in_create`, and `normalizeForVisualization_derivedprojected_operation`
//! (ignored: the derived projected export is not ported). Expected values are the C++ literals.
//! The sheet is `docs/PORTING-crs-unit-3.md`.

use proj_rs::iso19111::{
    BoundCRS, CartesianCS, Conversion, CoordinateOperationFactory, CoordinateSystem, Crs,
    DatabaseContext, Datum, DatumEnsemble, DerivedProjectedCRS, Equivalence, GeodeticCRS,
    GeodeticReferenceFrame, GeographicCRS, OpProps, ProjLines, ProjectedCRS, UnitOfMeasure,
    VerticalCRS, VerticalCS,
};

/// `createProjected()` of test_crs.cpp:2164: "WGS 84 / UTM zone 31N", EPSG:32631.
fn create_projected() -> ProjectedCRS {
    let conversion = Conversion::create_utm_north(&OpProps::new(), 31).expect("utm 31");
    ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631")
}

/// `TEST(crs, boundCRS_crs_link)` (test_crs.cpp:5409). The `use_count()` and `expired()`
/// assertions of the C++ test check shared-pointer lifetimes, which have no counterpart here:
/// those blocks are not ported. The values they check are below.
#[test]
fn crs_bound_crs_link() {
    let db = DatabaseContext::create().expect("proj.db");
    let towgs84 = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let epsg_4267 = Crs::Geographic(GeographicCRS::epsg_4267());

    // test_crs.cpp:5427-5439: the base of a bound CRS is EPSG:4267, with no canonical bound.
    let bound = BoundCRS::from_towgs84(epsg_4267.clone(), &towgs84).expect("bound CRS");
    let base = bound.source_crs();
    assert!(base.is_equivalent_to_with(&epsg_4267, Equivalence::Equivalent));
    assert!(base.canonical_bound_crs().is_none());

    // test_crs.cpp:5441-5458: the base with its canonical bound CRS.
    let base = BoundCRS::from_towgs84(epsg_4267.clone(), &towgs84)
        .expect("bound CRS")
        .base_crs_with_canonical_bound_crs();
    assert!(base.is_equivalent_to_with(&epsg_4267, Equivalence::Equivalent));
    let canonical = base.canonical_bound_crs().expect("canonical bound CRS");
    let to_wgs84 = base.create_bound_crs_to_wgs84_if_possible(&db);
    assert!(to_wgs84.is_equivalent_to_with(&Crs::Bound(canonical.clone()), Equivalence::Equivalent));

    // test_crs.cpp:5460-5475: the exterior bound CRS of a geographic base keeps EPSG:4267.
    let exterior = BoundCRS::from_towgs84(epsg_4267.clone(), &towgs84)
        .expect("bound CRS")
        .base_crs_with_canonical_bound_crs();
    let exterior = exterior
        .canonical_bound_crs()
        .expect("canonical bound CRS")
        .clone();
    assert!(exterior
        .source_crs()
        .is_equivalent_to_with(&epsg_4267, Equivalence::Equivalent));

    // test_crs.cpp:5477-5497: the same with a projected base.
    let projected = Crs::Projected(create_projected());
    let exterior = BoundCRS::from_towgs84(projected.clone(), &towgs84)
        .expect("bound CRS")
        .base_crs_with_canonical_bound_crs();
    let exterior = exterior
        .canonical_bound_crs()
        .expect("canonical bound CRS")
        .clone();
    assert!(exterior
        .source_crs()
        .is_equivalent_to_with(&projected, Equivalence::Equivalent));
}

/// `TEST(crs, normalizeForVisualization_derivedprojected_operation)` (test_crs.cpp:7681).
/// The derived projected CRS of `createDerivedProjectedCRSNorthingEasting` (test_crs.cpp:4817)
/// has a PROJ unimplemented conversion and northing/easting axes in feet.
///
/// Ignored: the operation from EPSG:4326 to the derived CRS is an inverted concatenation whose
/// conversion step runs from the derived CRS. `conversion_export.rs` writes a projected
/// source only, so the export fails with "the source CRS is not geographic". See the sheet.
#[test]
#[ignore = "derived projected conversion export not ported: see docs/PORTING-crs-unit-3.md"]
fn crs_normalize_for_visualization_derivedprojected_operation() {
    let db = DatabaseContext::create().expect("proj.db");
    let derived = DerivedProjectedCRS::create(
        "derived projectedCRS",
        create_projected(),
        Conversion::create_with_method("PROJ unimplemented", "", "", Vec::new())
            .with_name("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_northing_easting(UnitOfMeasure::foot())),
    );
    let op = CoordinateOperationFactory::create()
        .create_operation(
            &Crs::Geographic(GeographicCRS::epsg_4326()),
            &Crs::DerivedProjected(derived),
        )
        .expect("operations")
        .expect("operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
+proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=utm +zone=31 \
+ellps=WGS84 +step +proj=unimplemented +step +proj=unitconvert \
+xy_in=m +xy_out=ft +step +proj=axisswap +order=2,1"
    );

    let normalized = op
        .normalize_for_visualization()
        .expect("normalized operation");
    assert_eq!(
        normalized
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step \
+proj=utm +zone=31 +ellps=WGS84 +step +proj=unimplemented +step \
+proj=unitconvert +xy_in=m +xy_out=ft"
    );
}

/// `TEST(crs, VerticalCRS_ensemble_exception_in_create)` (test_crs.cpp:4749). The C++ first
/// case passes neither a datum nor an ensemble. `VerticalCRS::create` takes a datum in Rust,
/// so that case is not expressible and is not tested. The ensemble case: an ensemble of
/// geodetic datums is refused.
#[test]
fn crs_vertical_crs_ensemble_exception_in_create() {
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
        ],
        100.0,
    )
    .expect("ensemble");
    assert!(VerticalCRS::create_ensemble(
        "",
        ensemble,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .is_err());
}
