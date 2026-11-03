//! PROJ 9.9.0 `test/unit/test_crs.cpp` core cases.
//!
//! Each function is one `TEST()` name. Expected text is copied from that test.

use proj_rs::{
    create_crs_from_proj_string, parametric_unit, AlternativeGridNames, AuthorityFactory, Axis,
    BoundCRS, CartesianCS, CompoundCRS, Conversion, CoordinateOperation,
    CoordinateOperationFactory, CoordinateSystem, Crs, DatabaseContext, Datum, DatumEnsemble,
    DerivedCRS, DerivedKind, DerivedProjectedCRS, EllipsoidalCS, Equivalence, GeodeticCRS,
    GeodeticReferenceFrame, GeographicCRS, OpProps, ParameterLinearUnit, ParameterValue,
    ParametricCRS, ParametricCS, ParametricDatum, ProjectedCRS, Transformation, UnitKind,
    UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame, Wgs84Dynamic, WktConvention,
    WktFormatter, WktLines, WktParser, WktStrictness,
};

fn wkt(crs: &Crs, convention: WktConvention) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention))
        .expect("wkt")
}

fn wkt_single(crs: &Crs, convention: WktConvention) -> String {
    let mut formatter = WktFormatter::create(convention);
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("wkt")
}

fn compact(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn parse(text: &str) -> Crs {
    WktParser::new()
        .create_from_wkt(text)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

fn parse_db(text: &str, db: &DatabaseContext) -> Crs {
    WktParser::new()
        .attach_database(db.clone())
        .create_from_wkt(text)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

fn parse_lenient(text: &str, db: &DatabaseContext) -> Crs {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser.attach_database(db.clone());
    parser
        .create_from_wkt(text)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

fn epsg_code(crs: &Crs) -> &str {
    crs.authority_code("EPSG").expect("EPSG identifier").code()
}

fn my_unit() -> UnitOfMeasure {
    UnitOfMeasure::new("my unit", 2.0, UnitKind::Unknown)
}

fn utm_31n() -> ProjectedCRS {
    ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631")
}

fn odn() -> VerticalCRS {
    VerticalCRS::create(
        "ODN height",
        VerticalReferenceFrame::named("Ordnance Datum Newlyn").with_identifier("EPSG", "5101"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "5701")
}

fn compound() -> CompoundCRS {
    CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn())],
    )
    .expect("compound")
    .with_identifier("codespace", "code")
}

fn unimplemented_conversion(name: &str) -> Conversion {
    Conversion::create_with_method("PROJ unimplemented", "", "", Vec::new()).with_name(name)
}

fn derived_projected() -> DerivedProjectedCRS {
    DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
}

fn derived_projected_northing_easting() -> DerivedProjectedCRS {
    DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_northing_easting(UnitOfMeasure::foot())),
    )
}

fn derived_vertical() -> Crs {
    Crs::Derived(
        DerivedCRS::create(
            "Derived vertCRS",
            Crs::Vertical(odn()),
            unimplemented_conversion("unnamed"),
            CoordinateSystem::Vertical(VerticalCS::create_gravity_related_height(
                UnitOfMeasure::metre(),
            )),
        )
        .expect("derived vertical"),
    )
}

fn derived_geographic() -> Crs {
    let conversion = Conversion::create_with_method(
        "Pole rotation",
        "",
        "",
        vec![
            Conversion::parameter(
                "Latitude of rotated pole",
                "",
                "",
                ParameterValue::angle(52.0),
            ),
            Conversion::parameter(
                "Longitude of rotated pole",
                "",
                "",
                ParameterValue::angle(-30.0),
            ),
            Conversion::parameter("Axis rotation", "", "", ParameterValue::angle(-25.0)),
        ],
    )
    .with_name("Atlantic pole");
    Crs::Derived(
        DerivedCRS::create(
            "WMO Atlantic Pole",
            Crs::Geographic(GeographicCRS::epsg_4326()),
            conversion,
            CoordinateSystem::Ellipsoidal(EllipsoidalCS::create_latitude_longitude(
                UnitOfMeasure::degree(),
            )),
        )
        .expect("derived geographic"),
    )
}

fn my_geog() -> GeographicCRS {
    GeographicCRS::create(
        "my GEOGCRS",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("geographic")
}

fn my_projected() -> ProjectedCRS {
    ProjectedCRS::create(
        "my PROJCRS",
        GeodeticCRS::Geographic(my_geog()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
}

fn bound_towgs84(source: Crs) -> BoundCRS {
    BoundCRS::from_towgs84(source, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]).expect("towgs84")
}

enum UtmHemisphere {
    North,
    South,
}

fn utm_named(
    name: &str,
    base: GeographicCRS,
    zone: i32,
    hemisphere: UtmHemisphere,
) -> ProjectedCRS {
    let conversion = match hemisphere {
        UtmHemisphere::North => Conversion::create_utm_north(&OpProps::new(), zone).expect("utm"),
        UtmHemisphere::South => Conversion::create_utm_south(&OpProps::new(), zone).expect("utm"),
    };
    ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(base),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
}

fn geodetic_4979() -> GeodeticCRS {
    GeodeticCRS::Geographic(GeographicCRS::epsg_4979())
}

fn measure_of(parameter: &proj_rs::Parameter) -> &proj_rs::Measure {
    match parameter.value() {
        ParameterValue::Measure(measure) => measure,
        other => panic!("expected a measure, got {other:?}"),
    }
}

fn axis_count_geographic(crs: &GeographicCRS) -> usize {
    crs.coordinate_system().axes().len()
}

fn axis_count_geodetic(crs: &GeodeticCRS) -> usize {
    crs.coordinate_system().axes().len()
}

/// `crs.verticalCRS_datum_ensemble`
#[test]
fn crs_vertical_crs_datum_ensemble() {
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Vertical(VerticalReferenceFrame::named("vdatum1")),
            Datum::Vertical(VerticalReferenceFrame::named("vdatum2")),
        ],
        100.0,
    )
    .expect("ensemble");
    let crs = VerticalCRS::create_ensemble(
        "unnamed",
        ensemble,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .expect("vertical ensemble");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.simul_cur_node_has_id();
    let got = Crs::Vertical(crs)
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert_eq!(
        got,
        "\
VERTCRS[\"unnamed\",
    ENSEMBLE[\"unnamed\",
        MEMBER[\"vdatum1\"],
        MEMBER[\"vdatum2\"],
        ENSEMBLEACCURACY[100]],
    CS[vertical,1],
        AXIS[\"gravity-related height (H)\",up,
            LENGTHUNIT[\"metre\",1]]]"
    );
}

/// `crs.compoundCRS_valid`
#[test]
fn crs_compound_crs_valid() {
    CompoundCRS::create(
        "",
        vec![
            Crs::Geographic(GeographicCRS::epsg_4326()),
            Crs::Vertical(odn()),
        ],
    )
    .expect("geographic + vertical");
    CompoundCRS::create("", vec![Crs::Projected(utm_31n()), Crs::Vertical(odn())])
        .expect("projected + vertical");
    CompoundCRS::create(
        "",
        vec![
            Crs::DerivedProjected(derived_projected()),
            Crs::Vertical(odn()),
        ],
    )
    .expect("derived projected + vertical");
    CompoundCRS::create(
        "",
        vec![
            Crs::DerivedProjected(derived_projected()),
            derived_vertical(),
        ],
    )
    .expect("derived projected + derived vertical");
}

/// `crs.compoundCRS_as_WKT1_GDAL`
#[test]
fn crs_compound_crs_as_wkt1_gdal() {
    assert_eq!(
        wkt(&Crs::Compound(compound()), WktConvention::Wkt1Gdal),
        "\
COMPD_CS[\"horizontal + vertical\",
    PROJCS[\"WGS 84 / UTM zone 31N\",
        GEOGCS[\"WGS 84\",
            DATUM[\"WGS_1984\",
                SPHEROID[\"WGS 84\",6378137,298.257223563,
                    AUTHORITY[\"EPSG\",\"7030\"]],
                AUTHORITY[\"EPSG\",\"6326\"]],
            PRIMEM[\"Greenwich\",0,
                AUTHORITY[\"EPSG\",\"8901\"]],
            UNIT[\"degree\",0.0174532925199433,
                AUTHORITY[\"EPSG\",\"9122\"]],
            AUTHORITY[\"EPSG\",\"4326\"]],
        PROJECTION[\"Transverse_Mercator\"],
        PARAMETER[\"latitude_of_origin\",0],
        PARAMETER[\"central_meridian\",3],
        PARAMETER[\"scale_factor\",0.9996],
        PARAMETER[\"false_easting\",500000],
        PARAMETER[\"false_northing\",0],
        UNIT[\"metre\",1,
            AUTHORITY[\"EPSG\",\"9001\"]],
        AXIS[\"Easting\",EAST],
        AXIS[\"Northing\",NORTH],
        AUTHORITY[\"EPSG\",\"32631\"]],
    VERT_CS[\"ODN height\",
        VERT_DATUM[\"Ordnance Datum Newlyn\",2005,
            AUTHORITY[\"EPSG\",\"5101\"]],
        UNIT[\"metre\",1,
            AUTHORITY[\"EPSG\",\"9001\"]],
        AXIS[\"Gravity-related height\",UP],
        AUTHORITY[\"EPSG\",\"5701\"]],
    AUTHORITY[\"codespace\",\"code\"]]"
    );
}

/// `crs.compoundCRS_as_PROJ_string`
#[test]
fn crs_compound_crs_as_proj_string() {
    assert_eq!(
        Crs::Compound(compound())
            .export_to_proj_string()
            .expect("proj"),
        "+proj=utm +zone=31 +datum=WGS84 +units=m +vunits=m +no_defs +type=crs"
    );
}

/// `crs.boundCRS_to_WKT2`
#[test]
fn crs_bound_crs_to_wkt2() {
    let projected = my_projected();
    let bound = bound_towgs84(Crs::Projected(projected.clone()));
    assert_eq!(bound.source_crs().name(), projected.name());
    assert_eq!(bound.target_crs().name(), GeographicCRS::epsg_4326().name());
    let CoordinateOperation::Transformation(transformation) = bound.transformation() else {
        panic!("expected a transformation");
    };
    assert_eq!(
        transformation.source_crs().expect("source").name(),
        my_geog().name()
    );
    assert_eq!(
        transformation.target_crs().expect("target").name(),
        GeographicCRS::epsg_4326().name()
    );
    let parameters = transformation.parameters();
    assert_eq!(parameters.len(), 7);
    let expected = [
        ("8605", "X-axis translation", 1.0, "metre"),
        ("8606", "Y-axis translation", 2.0, "metre"),
        ("8607", "Z-axis translation", 3.0, "metre"),
        ("8608", "X-axis rotation", 4.0, "arc-second"),
        ("8609", "Y-axis rotation", 5.0, "arc-second"),
        ("8610", "Z-axis rotation", 6.0, "arc-second"),
        ("8611", "Scale difference", 7.0, "parts per million"),
    ];
    for (parameter, (code, name, value, unit)) in parameters.iter().zip(expected) {
        assert_eq!(parameter.code(), code);
        assert_eq!(parameter.name(), name);
        let measure = measure_of(parameter);
        assert_eq!(measure.value(), value);
        assert_eq!(measure.unit().name(), unit);
    }
    let source = wkt(&Crs::Projected(projected), WktConvention::Wkt2_2019);
    let target = wkt(
        &Crs::Geographic(GeographicCRS::epsg_4326()),
        WktConvention::Wkt2_2019,
    );
    let expected_wkt = format!(
        "BOUNDCRS[SOURCECRS[{source}],\nTARGETCRS[{target}],\n    \
ABRIDGEDTRANSFORMATION[\"Transformation from my GEOGCRS to WGS84\",\n        \
METHOD[\"Position Vector transformation (geog2D domain)\",\n            \
ID[\"EPSG\",9606]],\n        \
PARAMETER[\"X-axis translation\",1,\n            ID[\"EPSG\",8605]],\n        \
PARAMETER[\"Y-axis translation\",2,\n            ID[\"EPSG\",8606]],\n        \
PARAMETER[\"Z-axis translation\",3,\n            ID[\"EPSG\",8607]],\n        \
PARAMETER[\"X-axis rotation\",4,\n            ID[\"EPSG\",8608]],\n        \
PARAMETER[\"Y-axis rotation\",5,\n            ID[\"EPSG\",8609]],\n        \
PARAMETER[\"Z-axis rotation\",6,\n            ID[\"EPSG\",8610]],\n        \
PARAMETER[\"Scale difference\",1.000007,\n            ID[\"EPSG\",8611]]]]"
    );
    let got = wkt(&Crs::Bound(bound.clone()), WktConvention::Wkt2_2019);
    assert_eq!(compact(&got), compact(&expected_wkt), "{got}");
    let wrapped = Crs::Bound(bound);
    assert!(wrapped.is_equivalent_to(&wrapped));
    assert!(wrapped.is_equivalent_to(&wrapped.clone()));
    assert!(!wrapped.is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
}

/// `crs.boundCRS_to_WKT1`
#[test]
fn crs_bound_crs_to_wkt1() {
    let bound = bound_towgs84(Crs::Projected(my_projected()));
    assert_eq!(
        wkt(&Crs::Bound(bound), WktConvention::Wkt1Gdal),
        "\
PROJCS[\"my PROJCRS\",
    GEOGCS[\"my GEOGCRS\",
        DATUM[\"WGS_1984\",
            SPHEROID[\"WGS 84\",6378137,298.257223563,
                AUTHORITY[\"EPSG\",\"7030\"]],
            TOWGS84[1,2,3,4,5,6,7],
            AUTHORITY[\"EPSG\",\"6326\"]],
        PRIMEM[\"Greenwich\",0,
            AUTHORITY[\"EPSG\",\"8901\"]],
        UNIT[\"degree\",0.0174532925199433,
            AUTHORITY[\"EPSG\",\"9122\"]]],
    PROJECTION[\"Transverse_Mercator\"],
    PARAMETER[\"latitude_of_origin\",0],
    PARAMETER[\"central_meridian\",3],
    PARAMETER[\"scale_factor\",0.9996],
    PARAMETER[\"false_easting\",500000],
    PARAMETER[\"false_northing\",0],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Easting\",EAST],
    AXIS[\"Northing\",NORTH]]"
    );
}

/// `crs.boundCRS_geographicCRS_to_PROJ_string`
#[test]
fn crs_bound_crs_geographic_crs_to_proj_string() {
    let bound = bound_towgs84(Crs::Geographic(my_geog()));
    assert_eq!(
        Crs::Bound(bound).export_to_proj_string().expect("proj"),
        "+proj=longlat +ellps=WGS84 +towgs84=1,2,3,4,5,6,7 +no_defs +type=crs"
    );
}

/// `crs.boundCRS_projectedCRS_to_PROJ_string`
#[test]
fn crs_bound_crs_projected_crs_to_proj_string() {
    let bound = bound_towgs84(Crs::Projected(my_projected()));
    assert_eq!(
        Crs::Bound(bound).export_to_proj_string().expect("proj"),
        "+proj=utm +zone=31 +ellps=WGS84 +towgs84=1,2,3,4,5,6,7 +units=m +no_defs +type=crs"
    );
}

/// `crs.boundCRS_nadcon_to_PROJ_string_usePROJAlternativeGridNames_true`
#[test]
fn crs_bound_crs_nadcon_to_proj_string_use_proj_alternative_grid_names_true() {
    let db = DatabaseContext::create().expect("proj.db");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("15851", AlternativeGridNames::Proj)
        .expect("EPSG:15851");
    let bound = BoundCRS::create(
        Crs::Geographic(GeographicCRS::epsg_4267()),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        operation,
    );
    assert_eq!(
        Crs::Bound(bound).export_to_proj_string().expect("proj"),
        "+proj=longlat +ellps=clrk66 +nadgrids=us_noaa_conus.tif +no_defs +type=crs"
    );
}

/// `crs.incompatible_boundCRS_hubCRS_to_WKT1`
#[test]
fn crs_incompatible_bound_crs_hub_crs_to_wkt1() {
    let transformation = Transformation::create_geocentric_translations(
        &OpProps::new(),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        Crs::Geographic(GeographicCRS::epsg_4807()),
        1.0,
        2.0,
        3.0,
        None,
    )
    .expect("helmert");
    let bound = BoundCRS::create(
        Crs::Geographic(GeographicCRS::epsg_4326()),
        Crs::Geographic(GeographicCRS::epsg_4807()),
        CoordinateOperation::Transformation(transformation),
    );
    assert!(Crs::Bound(bound)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

/// `crs.incompatible_boundCRS_transformation_to_WKT1`
#[test]
fn crs_incompatible_bound_crs_transformation_to_wkt1() {
    let transformation = Transformation::create(
        &OpProps::new(),
        Crs::Geographic(GeographicCRS::epsg_4807()),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        None,
        "",
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("transformation");
    let bound = BoundCRS::create(
        Crs::Geographic(GeographicCRS::epsg_4807()),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        CoordinateOperation::Transformation(transformation),
    );
    assert!(Crs::Bound(bound)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

/// `crs.extractGeographicCRS`
#[test]
fn crs_extract_geographic_crs() {
    let geographic = Crs::Geographic(GeographicCRS::epsg_4326());
    assert!(geographic
        .extract_geographic_crs()
        .expect("geographic")
        .is_equivalent_to(&GeographicCRS::epsg_4326()));
    assert!(Crs::Projected(utm_31n())
        .extract_geographic_crs()
        .expect("projected base")
        .is_equivalent_to(&GeographicCRS::epsg_4326()));
    let compound = CompoundCRS::create(
        "",
        vec![
            Crs::Geographic(GeographicCRS::epsg_4326()),
            Crs::Vertical(odn()),
        ],
    )
    .expect("compound");
    assert!(Crs::Compound(compound)
        .extract_geographic_crs()
        .expect("compound geographic")
        .is_equivalent_to(&GeographicCRS::epsg_4326()));
}

/// `crs.extractVerticalCRS`
#[test]
fn crs_extract_vertical_crs() {
    assert!(Crs::Geographic(GeographicCRS::epsg_4326())
        .extract_vertical_crs()
        .is_none());
    let vertical = Crs::Compound(compound())
        .extract_vertical_crs()
        .expect("vertical");
    assert!(vertical.is_equivalent_to(&odn()));
}

/// `crs.default_identify_method`
#[test]
fn crs_default_identify_method() {
    let crs = ParametricCRS::create(
        "Parametric CRS",
        ParametricDatum::create("Parametric datum"),
        ParametricCS::create(Axis::create(
            "pressure",
            "hPa",
            "up",
            parametric_unit("HectoPascal", 100.0),
        )),
    );
    assert!(Crs::Parametric(crs).identify().is_empty());
}

/// `crs.projectedCRS_identify_no_db`
#[test]
fn crs_projected_crs_identify_no_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let unnamed = utm_named("", GeographicCRS::epsg_4326(), 1, UtmHemisphere::North);
    let hits = unnamed.identify_with(None);
    assert_eq!(hits.len(), 1);
    assert_eq!(epsg_code(&hits[0].0), "32601");
    assert_eq!(hits[0].1, 70);
    let Crs::Projected(found) = &hits[0].0 else {
        panic!("projected");
    };
    assert!(found.is_equivalent_to_with(
        &factory.create_projected_crs("32601").expect("32601"),
        Equivalence::Equivalent
    ));

    let named = utm_named(
        "WGS 84 / UTM zone 60S",
        GeographicCRS::epsg_4326(),
        60,
        UtmHemisphere::South,
    );
    let hits = Crs::Projected(named).identify();
    assert_eq!(hits.len(), 1);
    assert_eq!(epsg_code(&hits[0].0), "32760");
    assert_eq!(hits[0].1, 100);
    let Crs::Projected(found) = &hits[0].0 else {
        panic!("projected");
    };
    assert!(found.is_equivalent_to_with(
        &factory.create_projected_crs("32760").expect("32760"),
        Equivalence::Equivalent
    ));

    let nad27 = utm_named(
        "NAD27_UTM_zone_11N",
        GeographicCRS::epsg_4267(),
        11,
        UtmHemisphere::North,
    );
    let hits = nad27.identify_with(None);
    assert_eq!(hits.len(), 1);
    assert_eq!(epsg_code(&hits[0].0), "26711");
    assert_eq!(hits[0].1, 90);
    let Crs::Projected(found) = &hits[0].0 else {
        panic!("projected");
    };
    assert!(found.is_equivalent_to_with(
        &factory.create_projected_crs("26711").expect("26711"),
        Equivalence::Equivalent
    ));

    let nad83 = utm_named("", GeographicCRS::epsg_4269(), 11, UtmHemisphere::North);
    let hits = nad83.identify_with(None);
    assert_eq!(hits.len(), 1);
    assert_eq!(epsg_code(&hits[0].0), "26911");
    assert_eq!(hits[0].1, 70);
    let Crs::Projected(found) = &hits[0].0 else {
        panic!("projected");
    };
    assert!(found.is_equivalent_to_with(
        &factory.create_projected_crs("26911").expect("26911"),
        Equivalence::Equivalent
    ));

    let parsed =
        create_crs_from_proj_string("+proj=utm +zone=31 +datum=WGS84 +type=crs").expect("proj");
    let Crs::Projected(parsed_projected) = &parsed else {
        panic!("projected");
    };
    let hits = parsed_projected.identify_with(None);
    assert_eq!(hits.len(), 1);
    assert_eq!(epsg_code(&hits[0].0), "32631");
    assert_eq!(hits[0].1, 70);
    let Crs::Projected(found) = &hits[0].0 else {
        panic!("projected");
    };
    assert!(found.is_equivalent_to_with(parsed_projected, Equivalence::ExceptAxisOrder));
}

/// `crs.crs_stripVerticalComponent`
#[test]
fn crs_crs_strip_vertical_component() {
    let stripped = Crs::Geographic(GeographicCRS::epsg_4979()).strip_vertical_component();
    let Crs::Geographic(geographic) = &stripped else {
        panic!("geographic");
    };
    assert_eq!(axis_count_geographic(geographic), 2);

    assert!(Crs::Geographic(GeographicCRS::epsg_4326())
        .strip_vertical_component()
        .is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));

    let cartesian = CartesianCS::create(vec![
        Axis::create("Easting", "E", "east", UnitOfMeasure::metre()),
        Axis::create("Northing", "N", "north", UnitOfMeasure::metre()),
        Axis::create("Height", "z", "up", UnitOfMeasure::metre()),
    ])
    .expect("cartesian");
    let projected = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        cartesian,
    );
    let stripped = Crs::Projected(projected).strip_vertical_component();
    let Crs::Projected(projected) = stripped else {
        panic!("projected");
    };
    assert_eq!(projected.coordinate_system().axes().len(), 2);

    let promoted = Crs::DerivedProjected(derived_projected()).promote_to_3d(None);
    let Crs::DerivedProjected(promoted) = &promoted else {
        panic!("derived projected");
    };
    assert_eq!(promoted.coordinate_system().axes().len(), 3);
    let stripped = Crs::DerivedProjected(promoted.clone()).strip_vertical_component();
    let Crs::DerivedProjected(stripped) = stripped else {
        panic!("derived projected");
    };
    assert_eq!(stripped.coordinate_system().axes().len(), 2);
}

/// `crs.crs_alterGeodeticCRS`
#[test]
fn crs_crs_alter_geodetic_crs() {
    let altered = Crs::Geographic(GeographicCRS::epsg_4326()).alter_geodetic_crs(&geodetic_4979());
    assert!(altered.is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4979())));

    let altered = Crs::Projected(utm_31n()).alter_geodetic_crs(&geodetic_4979());
    let Crs::Projected(projected) = altered else {
        panic!("projected");
    };
    assert!(projected.base_crs().is_equivalent_to(&geodetic_4979()));

    let altered = Crs::Compound(compound()).alter_geodetic_crs(&geodetic_4979());
    let Crs::Compound(compound) = altered else {
        panic!("compound");
    };
    assert!(compound.components()[0]
        .extract_geographic_crs()
        .expect("geographic")
        .is_equivalent_to(&GeographicCRS::epsg_4979()));

    let vertical = Crs::Vertical(odn());
    assert!(vertical
        .alter_geodetic_crs(&geodetic_4979())
        .is_equivalent_to(&vertical));

    let altered = Crs::DerivedProjected(derived_projected()).alter_geodetic_crs(&geodetic_4979());
    let Crs::DerivedProjected(derived) = altered else {
        panic!("derived projected");
    };
    assert!(derived
        .base_crs()
        .base_crs()
        .is_equivalent_to(&geodetic_4979()));
}

/// `crs.crs_alterCSLinearUnit`
#[test]
fn crs_crs_alter_cs_linear_unit() {
    let unit = my_unit();
    let altered = Crs::Projected(utm_31n()).alter_cs_linear_unit(&unit);
    let Crs::Projected(projected) = altered else {
        panic!("projected");
    };
    assert_eq!(projected.coordinate_system().axes().len(), 2);
    for axis in projected.coordinate_system().axes() {
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }

    let altered = Crs::DerivedProjected(derived_projected()).alter_cs_linear_unit(&unit);
    let Crs::DerivedProjected(derived) = altered else {
        panic!("derived projected");
    };
    assert_eq!(derived.coordinate_system().axes().len(), 2);
    for axis in derived.coordinate_system().axes() {
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }

    let altered =
        Crs::DerivedProjected(derived_projected_northing_easting()).alter_cs_linear_unit(&unit);
    let Crs::DerivedProjected(derived) = altered else {
        panic!("derived projected");
    };
    let axes = derived.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].unit().name(), "my unit");
    assert_eq!(axes[0].direction(), "north");
    assert_eq!(axes[0].unit().conversion_to_si(), 2.0);
    assert_eq!(axes[1].unit().name(), "my unit");
    assert_eq!(axes[1].direction(), "east");
    assert_eq!(axes[1].unit().conversion_to_si(), 2.0);

    let altered = Crs::Geodetic(GeodeticCRS::epsg_4978()).alter_cs_linear_unit(&unit);
    let Crs::Geodetic(geodetic) = altered else {
        panic!("geodetic");
    };
    assert_eq!(axis_count_geodetic(&geodetic), 3);
    for axis in geodetic.coordinate_system().axes() {
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }

    let altered = Crs::Geographic(GeographicCRS::epsg_4979()).alter_cs_linear_unit(&unit);
    let Crs::Geographic(geographic) = altered else {
        panic!("geographic");
    };
    let axes = geographic.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert_ne!(axes[0].unit().name(), "my unit");
    assert_ne!(axes[0].unit().conversion_to_si(), 2.0);
    assert_ne!(axes[1].unit().name(), "my unit");
    assert_ne!(axes[1].unit().conversion_to_si(), 2.0);
    assert_eq!(axes[2].unit().name(), "my unit");
    assert_eq!(axes[2].unit().conversion_to_si(), 2.0);

    let altered = Crs::Vertical(odn()).alter_cs_linear_unit(&unit);
    let Crs::Vertical(vertical) = altered else {
        panic!("vertical");
    };
    let axis = vertical.coordinate_system().axis();
    assert_eq!(axis.unit().name(), "my unit");
    assert_eq!(axis.unit().conversion_to_si(), 2.0);

    let engineering = parse("LOCAL_CS[\"foo\"]");
    let altered = engineering.alter_cs_linear_unit(&unit);
    assert_eq!(
        wkt_single(&altered, WktConvention::Wkt1Gdal),
        "LOCAL_CS[\"foo\",UNIT[\"my unit\",2],AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH]]"
    );

    let altered = Crs::Compound(compound()).alter_cs_linear_unit(&unit);
    let Crs::Compound(compound) = altered else {
        panic!("compound");
    };
    assert_eq!(compound.components().len(), 2);
    for component in compound.components() {
        let axis = match component {
            Crs::Projected(crs) => &crs.coordinate_system().axes()[0],
            Crs::Vertical(crs) => crs.coordinate_system().axis(),
            other => panic!("unexpected component {other:?}"),
        };
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }

    let bound = bound_towgs84(Crs::Projected(utm_31n()));
    let altered = Crs::Bound(bound).alter_cs_linear_unit(&unit);
    let Crs::Bound(bound) = altered else {
        panic!("bound");
    };
    let Crs::Projected(projected) = bound.source_crs() else {
        panic!("projected source");
    };
    let axes = projected.coordinate_system().axes();
    assert_eq!(axes[0].unit().name(), "my unit");
    assert_eq!(axes[0].unit().conversion_to_si(), 2.0);
    assert_eq!(axes[1].unit().name(), "my unit");
    assert_eq!(axes[1].unit().conversion_to_si(), 2.0);

    let parametric = Crs::Parametric(ParametricCRS::create(
        "Parametric CRS",
        ParametricDatum::create("Parametric datum"),
        ParametricCS::create(Axis::create(
            "pressure",
            "hPa",
            "up",
            parametric_unit("HectoPascal", 100.0),
        )),
    ));
    assert!(parametric
        .alter_cs_linear_unit(&unit)
        .is_equivalent_to(&parametric));
}

/// `crs.alterParametersLinearUnit`
#[test]
fn crs_alter_parameters_linear_unit() {
    let projected = utm_31n();
    let relabeled = projected.alter_parameters_linear_unit(ParameterLinearUnit::Relabel(my_unit()));
    let text = wkt_single(&Crs::Projected(relabeled), WktConvention::Wkt2_2015);
    assert!(
        text.contains("PARAMETER[\"Longitude of natural origin\",3"),
        "{text}"
    );
    assert!(
        text.contains("PARAMETER[\"False easting\",500000,UNIT[\"my unit\",2]"),
        "{text}"
    );

    let converted = utm_31n().alter_parameters_linear_unit(ParameterLinearUnit::Convert(my_unit()));
    let text = wkt_single(&Crs::Projected(converted), WktConvention::Wkt2_2015);
    assert!(
        text.contains("PARAMETER[\"False easting\",250000,UNIT[\"my unit\",2]"),
        "{text}"
    );
}

fn wkt_db(crs: &Crs, convention: WktConvention, db: &DatabaseContext) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention).with_database(db.clone()))
        .expect("wkt")
}

/// C++ `datum()` synthesizes a datum from an ensemble (`datumNonNull`).
fn datum_name(crs: &GeodeticCRS) -> String {
    if let Some(datum) = crs.datum() {
        return datum.name().to_owned();
    }
    match crs.datum_ensemble().and_then(DatumEnsemble::as_datum) {
        Some(Datum::Geodetic(frame)) => frame.name().to_owned(),
        other => panic!("no geodetic datum: {other:?}"),
    }
}

fn axes_match_loose(left: &CoordinateSystem, right: &CoordinateSystem) -> bool {
    let left = left.axes();
    let right = right.axes();
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            let (left_si, right_si) = (
                left.unit().conversion_to_si(),
                right.unit().conversion_to_si(),
            );
            left.direction() == right.direction()
                && (left_si - right_si).abs() <= 1e-10 * left_si.abs().max(1.0)
        })
}

/// `isEquivalentTo` with `EQUIVALENT`: a database demote may return the WGS 84
/// ensemble where the input held datum EPSG:6326.
fn equivalent_roundtrip(left: &Crs, right: &Crs) -> bool {
    match (left, right) {
        (Crs::Derived(left), Crs::Derived(right)) => {
            left.kind() == right.kind()
                && left.name() == right.name()
                && match (left.base_crs(), right.base_crs()) {
                    (Crs::Geographic(left_base), Crs::Geographic(right_base)) => {
                        GeodeticCRS::Geographic(left_base.clone()).is_equivalent_to_with(
                            &GeodeticCRS::Geographic(right_base.clone()),
                            Equivalence::Equivalent,
                        )
                    }
                    _ => false,
                }
                && left
                    .deriving_conversion()
                    .is_equivalent_to(right.deriving_conversion())
                && axes_match_loose(left.coordinate_system(), right.coordinate_system())
        }
        (Crs::DerivedProjected(left), Crs::DerivedProjected(right)) => {
            left.name() == right.name()
                && left
                    .base_crs()
                    .is_equivalent_to_with(right.base_crs(), Equivalence::Equivalent)
                && left
                    .deriving_conversion()
                    .is_equivalent_to(right.deriving_conversion())
                && axes_match_loose(left.coordinate_system(), right.coordinate_system())
        }
        _ => false,
    }
}

/// `crs.getNonDeprecated`
#[test]
fn crs_get_non_deprecated() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let unnamed = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    assert!(Crs::Projected(unnamed)
        .non_deprecated(&db)
        .expect("list")
        .is_empty());
    assert!(
        Crs::Geodetic(factory.create_geodetic_crs("4326").expect("4326"))
            .non_deprecated(&db)
            .expect("list")
            .is_empty()
    );
    assert!(Crs::Bound(bound_towgs84(Crs::Projected(utm_31n())))
        .non_deprecated(&db)
        .expect("list")
        .is_empty());
    assert_eq!(
        Crs::Geodetic(factory.create_geodetic_crs("4226").expect("4226"))
            .non_deprecated(&db)
            .expect("list")
            .len(),
        2
    );
    assert_eq!(
        Crs::Projected(factory.create_projected_crs("26591").expect("26591"))
            .non_deprecated(&db)
            .expect("list")
            .len(),
        1
    );
    assert_eq!(
        Crs::Vertical(factory.create_vertical_crs("5704").expect("5704"))
            .non_deprecated(&db)
            .expect("list")
            .len(),
        1
    );
    assert_eq!(
        Crs::Compound(factory.create_compound_crs("7401").expect("7401"))
            .non_deprecated(&db)
            .expect("list")
            .len(),
        1
    );
}

/// `crs.is_dynamic`
#[test]
fn crs_is_dynamic() {
    let wgs84 = Crs::Geographic(GeographicCRS::epsg_4326());
    assert!(!wgs84.is_dynamic());
    assert!(wgs84.is_dynamic_with(Wgs84Dynamic::TreatAsDynamic));

    let db = DatabaseContext::create().expect("proj.db");
    let from_db = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4326")
        .expect("4326");
    assert!(!from_db.is_dynamic());
    assert!(from_db.is_dynamic_with(Wgs84Dynamic::TreatAsDynamic));

    let dynamic = GeodeticReferenceFrame::epsg_6326().with_frame_epoch(2018.5);
    let geographic = GeographicCRS::create(
        "test",
        Some(dynamic),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("geographic");
    assert!(Crs::Geographic(geographic).is_dynamic());

    assert!(!Crs::Vertical(odn()).is_dynamic());
    let vertical = VerticalCRS::create(
        "",
        VerticalReferenceFrame::named("test").with_frame_epoch(2018.5),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    );
    assert!(Crs::Vertical(vertical).is_dynamic());
    assert!(!Crs::Compound(compound()).is_dynamic());
}

fn target_unchanged(projected: &ProjectedCRS) {
    assert!(std::ptr::eq(
        projected.deriving_operation().target_crs(),
        projected
    ));
}

/// `crs.projected_normalizeForVisualization_do_not_mess_deriving_conversion`
#[test]
fn crs_projected_normalize_for_visualization_do_not_mess_deriving_conversion() {
    let db = DatabaseContext::create().expect("proj.db");
    let projected = AuthorityFactory::create(&db, "EPSG")
        .create_projected_crs("3035")
        .expect("3035");
    let operation = CoordinateOperationFactory::create()
        .create_operation(
            &Crs::Geographic(GeographicCRS::epsg_4326()),
            &Crs::Projected(projected.clone()),
        )
        .expect("operation");
    let operation = operation.expect("EPSG:4326 to EPSG:3035");
    let _ = operation.normalize_for_visualization();
    target_unchanged(&projected);
}

/// `crs.projected_promoteTo3D_do_not_mess_deriving_conversion`
#[test]
fn crs_projected_promote_to_3d_do_not_mess_deriving_conversion() {
    let projected = utm_31n();
    let _ = Crs::Projected(projected.clone()).promote_to_3d(None);
    target_unchanged(&projected);
}

/// `crs.projected_demoteTo2D_do_not_mess_deriving_conversion`
#[test]
fn crs_projected_demote_to_2d_do_not_mess_deriving_conversion() {
    let promoted = Crs::Projected(utm_31n()).promote_to_3d(None);
    let Crs::Projected(projected) = promoted else {
        panic!("projected");
    };
    let _ = projected.demote_to_2d(None);
    target_unchanged(&projected);
}

/// `crs.projected_alterGeodeticCRS_do_not_mess_deriving_conversion`
#[test]
fn crs_projected_alter_geodetic_crs_do_not_mess_deriving_conversion() {
    let projected = utm_31n();
    let geographic = Crs::Projected(projected.clone())
        .extract_geographic_crs()
        .expect("geographic");
    let _ =
        Crs::Projected(projected.clone()).alter_geodetic_crs(&GeodeticCRS::Geographic(geographic));
    target_unchanged(&projected);
}

/// `crs.projected_alterCSLinearUnit_do_not_mess_deriving_conversion`
#[test]
fn crs_projected_alter_cs_linear_unit_do_not_mess_deriving_conversion() {
    let projected = utm_31n();
    let _ = Crs::Projected(projected.clone()).alter_cs_linear_unit(&my_unit());
    target_unchanged(&projected);
}

/// `crs.projected_alterParametersLinearUnit_do_not_mess_deriving_conversion`
#[test]
fn crs_projected_alter_parameters_linear_unit_do_not_mess_deriving_conversion() {
    let projected = utm_31n();
    let _ = projected
        .alter_parameters_linear_unit(ParameterLinearUnit::Relabel(UnitOfMeasure::metre()));
    target_unchanged(&projected);
}

/// `crs.promoteTo3D_and_demoteTo2D`
#[test]
fn crs_promote_to_3d_and_demote_to_2d() {
    let db = DatabaseContext::create().expect("proj.db");
    let promoted = Crs::Geographic(GeographicCRS::epsg_4326()).promote_to_3d(None);
    let Crs::Geographic(geographic) = &promoted else {
        panic!("geographic");
    };
    assert_eq!(axis_count_geographic(geographic), 3);
    assert!(promoted.promote_to_3d(None).is_equivalent_to(&promoted));

    let promoted = Crs::Geographic(GeographicCRS::epsg_4326()).promote_to_3d(Some(&db));
    let Crs::Geographic(geographic) = &promoted else {
        panic!("geographic");
    };
    assert_eq!(axis_count_geographic(geographic), 3);
    assert!(!geographic.identifiers().is_empty());
    let demoted = promoted.demote_to_2d(Some(&db));
    let Crs::Geographic(geographic) = &demoted else {
        panic!("geographic");
    };
    assert_eq!(axis_count_geographic(geographic), 2);
    assert!(!geographic.identifiers().is_empty());

    let promoted = Crs::Projected(utm_31n()).promote_to_3d(None);
    let Crs::Projected(projected) = &promoted else {
        panic!("projected");
    };
    assert_eq!(projected.coordinate_system().axes().len(), 3);
    assert_eq!(axis_count_geodetic(projected.base_crs()), 3);
    let text = wkt(&promoted, WktConvention::Wkt2_2019);
    let parsed = parse(&text);
    let Crs::Projected(parsed) = &parsed else {
        panic!("{text}");
    };
    assert_eq!(parsed.coordinate_system().axes().len(), 3);
    assert_eq!(axis_count_geodetic(parsed.base_crs()), 3);
    assert!(promoted.promote_to_3d(None).is_equivalent_to(&promoted));
    let demoted = projected.demote_to_2d(None);
    assert_eq!(demoted.coordinate_system().axes().len(), 2);
    assert_eq!(axis_count_geodetic(demoted.base_crs()), 2);

    let promoted = Crs::Projected(utm_31n()).promote_to_3d(Some(&db));
    let Crs::Projected(projected) = &promoted else {
        panic!("projected");
    };
    assert_eq!(projected.coordinate_system().axes().len(), 3);
    assert_eq!(axis_count_geodetic(projected.base_crs()), 3);
    let base = match projected.base_crs() {
        GeodeticCRS::Geographic(crs) => crs,
        other => panic!("expected a geographic base, got {other:?}"),
    };
    assert!(!base.identifiers().is_empty());
    let demoted = projected.demote_to_2d(Some(&db));
    assert_eq!(demoted.coordinate_system().axes().len(), 2);
    assert_eq!(axis_count_geodetic(demoted.base_crs()), 2);
    let GeodeticCRS::Geographic(base) = demoted.base_crs() else {
        panic!("geographic base");
    };
    assert!(!base.identifiers().is_empty());

    let promoted = Crs::Bound(bound_towgs84(Crs::Projected(utm_31n()))).promote_to_3d(Some(&db));
    let Crs::Bound(bound) = &promoted else {
        panic!("bound");
    };
    let Crs::Projected(base) = bound.source_crs() else {
        panic!("projected source");
    };
    assert_eq!(base.coordinate_system().axes().len(), 3);
    let Crs::Geographic(hub) = bound.target_crs() else {
        panic!("geographic hub");
    };
    assert_eq!(axis_count_geographic(hub), 3);
    let CoordinateOperation::Transformation(transformation) = bound.transformation() else {
        panic!("transformation");
    };
    let Crs::Geographic(source) = transformation.source_crs().expect("source") else {
        panic!("geographic source");
    };
    assert_eq!(axis_count_geographic(source), 3);
    let Crs::Geographic(target) = transformation.target_crs().expect("target") else {
        panic!("geographic target");
    };
    assert_eq!(axis_count_geographic(target), 3);
    let demoted = promoted.demote_to_2d(None);
    let Crs::Bound(bound) = &demoted else {
        panic!("bound");
    };
    let Crs::Projected(base) = bound.source_crs() else {
        panic!("projected source");
    };
    assert_eq!(base.coordinate_system().axes().len(), 2);
    let Crs::Geographic(hub) = bound.target_crs() else {
        panic!("geographic hub");
    };
    assert_eq!(axis_count_geographic(hub), 2);
    let CoordinateOperation::Transformation(transformation) = bound.transformation() else {
        panic!("transformation");
    };
    let Crs::Geographic(source) = transformation.source_crs().expect("source") else {
        panic!("geographic source");
    };
    assert_eq!(axis_count_geographic(source), 2);
    let Crs::Geographic(target) = transformation.target_crs().expect("target") else {
        panic!("geographic target");
    };
    assert_eq!(axis_count_geographic(target), 2);

    let demoted = Crs::Compound(compound()).demote_to_2d(None);
    assert!(matches!(demoted, Crs::Projected(_)));

    let derived = derived_geographic();
    let promoted = derived.promote_to_3d(Some(&db));
    let Crs::Derived(derived_3d) = &promoted else {
        panic!("derived geographic");
    };
    assert_eq!(derived_3d.kind(), DerivedKind::Geographic);
    let Crs::Geographic(base) = derived_3d.base_crs() else {
        panic!("geographic base");
    };
    assert_eq!(axis_count_geographic(base), 3);
    assert_eq!(derived_3d.coordinate_system().axes().len(), 3);
    assert!(promoted.promote_to_3d(None).is_equivalent_to(&promoted));
    let demoted = promoted.demote_to_2d(Some(&db));
    let Crs::Derived(demoted_derived) = &demoted else {
        panic!("derived geographic");
    };
    let Crs::Geographic(base) = demoted_derived.base_crs() else {
        panic!("geographic base");
    };
    assert_eq!(axis_count_geographic(base), 2);
    assert_eq!(demoted_derived.coordinate_system().axes().len(), 2);
    assert!(
        equivalent_roundtrip(&demoted, &derived),
        "demoted {}\noriginal {}",
        wkt(&demoted, WktConvention::Wkt2_2019),
        wkt(&derived, WktConvention::Wkt2_2019)
    );
    assert!(demoted.demote_to_2d(None).is_equivalent_to(&demoted));

    let derived = Crs::DerivedProjected(derived_projected());
    let promoted = derived.promote_to_3d(Some(&db));
    let Crs::DerivedProjected(derived_3d) = &promoted else {
        panic!("derived projected");
    };
    assert_eq!(derived_3d.base_crs().coordinate_system().axes().len(), 3);
    assert_eq!(derived_3d.coordinate_system().axes().len(), 3);
    assert!(promoted.promote_to_3d(None).is_equivalent_to(&promoted));
    let text = wkt(&promoted, WktConvention::Wkt2_2019);
    let parsed = parse(&text);
    let Crs::DerivedProjected(parsed) = &parsed else {
        panic!("{text}");
    };
    assert_eq!(parsed.coordinate_system().axes().len(), 3);
    assert_eq!(parsed.base_crs().coordinate_system().axes().len(), 3);
    let demoted = promoted.demote_to_2d(Some(&db));
    let Crs::DerivedProjected(demoted_derived) = &demoted else {
        panic!("derived projected");
    };
    assert_eq!(
        demoted_derived.base_crs().coordinate_system().axes().len(),
        2
    );
    assert_eq!(demoted_derived.coordinate_system().axes().len(), 2);
    assert!(
        equivalent_roundtrip(&demoted, &derived),
        "demoted {}\noriginal {}",
        wkt(&demoted, WktConvention::Wkt2_2019),
        wkt(&derived, WktConvention::Wkt2_2019)
    );
    assert!(demoted.demote_to_2d(None).is_equivalent_to(&demoted));
}

/// `crs.projectedCRS_from_WKT1_ESRI_ETRS89_with_db`
#[test]
fn crs_projected_crs_from_wkt1_esri_etrs89_with_db() {
    let wkt1 = "PROJCS[\"ETRS89 / ETRS-LAEA\",\
GEOGCS[\"ETRS89\",\
DATUM[\"European_Terrestrial_Reference_System_1989\",\
SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.01745329251994328]],\
PROJECTION[\"Lambert_Azimuthal_Equal_Area\"],\
PARAMETER[\"latitude_of_center\",52],\
PARAMETER[\"longitude_of_center\",10],\
PARAMETER[\"false_easting\",4321000],\
PARAMETER[\"false_northing\",3210000],\
UNIT[\"metre\",1]]";
    let db = DatabaseContext::create().expect("proj.db");
    let crs = parse_db(wkt1, &db);
    let Crs::Projected(projected) = &crs else {
        panic!("{}", wkt_single(&crs, WktConvention::Wkt2_2019));
    };
    assert_eq!(
        datum_name(projected.base_crs()),
        "European Terrestrial Reference System 1989"
    );
}

/// `crs.projectedCRS_3D_is_WKT1_equivalent_to_WKT2`
#[test]
fn crs_projected_crs_3d_is_wkt1_equivalent_to_wkt2() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt1 = "PROJCS[\"WGS 84 / UTM zone 16N [EGM08-1]\",\
GEOGCS[\"WGS 84 / UTM zone 16N [EGM08-1]\",\
DATUM[\"WGS84\",SPHEROID[\"WGS84\",6378137.000,298.257223563,\
AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY[\"EPSG\",\"6326\"]],\
PRIMEM[\"Greenwich\",0.0000000000000000,\
AUTHORITY[\"EPSG\",\"8901\"]],\
UNIT[\"Degree\",0.01745329251994329547,\
AUTHORITY[\"EPSG\",\"9102\"]],AUTHORITY[\"EPSG\",\"32616\"]],\
UNIT[\"Meter\",1.00000000000000000000,\
AUTHORITY[\"EPSG\",\"9001\"]],\
PROJECTION[\"Transverse_Mercator\"],\
PARAMETER[\"latitude_of_origin\",0.0000000000000000],\
PARAMETER[\"central_meridian\",-87.0000000002777938],\
PARAMETER[\"scale_factor\",0.9996000000000000],\
PARAMETER[\"false_easting\",500000.000],\
PARAMETER[\"false_northing\",0.000],\
AXIS[\"Easting\",EAST],\
AXIS[\"Northing\",NORTH],\
AXIS[\"Height\",UP],\
AUTHORITY[\"EPSG\",\"32616\"]]";
    let crs = parse_lenient(wkt1, &db);
    let Crs::Projected(projected) = &crs else {
        panic!("{}", wkt_single(&crs, WktConvention::Wkt2_2019));
    };
    let exported = wkt(&crs, WktConvention::Wkt2_2019);
    let parsed = parse_db(&exported, &db);
    let Crs::Projected(again) = &parsed else {
        panic!("{exported}");
    };
    assert!(projected.is_equivalent_to_with(again, Equivalence::ExceptAxisOrder));
}

/// `crs.identical_after_import_from_proj4_and_wkt1_export`
#[test]
fn crs_identical_after_import_from_proj4_and_wkt1_export() {
    let crs = create_crs_from_proj_string(
        "+proj=lcc +lat_0=25 +lon_0=-95 +lat_1=25 +lat_2=25 +a=6371229 \
+b=6371229 +units=m +no_defs +type=crs",
    )
    .expect("lcc");
    let Crs::Projected(_) = &crs else {
        panic!("{}", wkt_single(&crs, WktConvention::Wkt2_2019));
    };
    let wkt1 = wkt(&crs, WktConvention::Wkt1Gdal);
    let again = parse(&wkt1);
    assert_eq!(
        wkt(&crs, WktConvention::Wkt2_2019),
        wkt(&again, WktConvention::Wkt2_2019)
    );
}

/// `crs.verticalCRS_ESRI_115834_as_WKT1_ESRI_with_database`
#[test]
fn crs_vertical_crs_esri_115834_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "ESRI")
        .create_coordinate_reference_system("115834")
        .expect("115834");
    assert_eq!(
        wkt_db(&crs, WktConvention::Wkt1Esri, &db),
        "VERTCS[\"NAD83(CSRS)v5\",\
DATUM[\"North_American_Datum_of_1983_(CSRS)_version_5\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PARAMETER[\"Vertical_Shift\",0.0],\
PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

fn require_bound(crs: &Crs) {
    if !matches!(crs, Crs::Bound(_)) {
        panic!(
            "expected a bound CRS, got {}",
            wkt_single(crs, WktConvention::Wkt2_2019)
        );
    }
}

/// `crs.WKT1_DATUM_EXTENSION_to_WKT1_and_PROJ_string`
#[test]
fn crs_wkt1_datum_extension_to_wkt1_and_proj_string() {
    let wkt1 = "\
PROJCS[\"unnamed\",
    GEOGCS[\"International 1909 (Hayford)\",
        DATUM[\"unknown\",
            SPHEROID[\"intl\",6378388,297],
            EXTENSION[\"PROJ4_GRIDS\",\"nzgd2kgrid0005.gsb\"]],
        PRIMEM[\"Greenwich\",0],
        UNIT[\"degree\",0.0174532925199433]],
    PROJECTION[\"New_Zealand_Map_Grid\"],
    PARAMETER[\"latitude_of_origin\",-41],
    PARAMETER[\"central_meridian\",173],
    PARAMETER[\"false_easting\",2510000],
    PARAMETER[\"false_northing\",6023150],
    UNIT[\"Meter\",1],
    AXIS[\"Easting\",EAST],
    AXIS[\"Northing\",NORTH]]";
    let crs = parse(wkt1);
    require_bound(&crs);
    assert_eq!(wkt(&crs, WktConvention::Wkt1Gdal), wkt1);
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=nzmg +lat_0=-41 +lon_0=173 +x_0=2510000 +y_0=6023150 \
+ellps=intl +nadgrids=nzgd2kgrid0005.gsb +units=m +no_defs +type=crs"
    );
}

/// `crs.WKT1_VERT_DATUM_EXTENSION_to_WKT1`
#[test]
fn crs_wkt1_vert_datum_extension_to_wkt1() {
    let wkt1 = "\
VERT_CS[\"EGM2008 geoid height\",
    VERT_DATUM[\"EGM2008 geoid\",2005,
        EXTENSION[\"PROJ4_GRIDS\",\"egm08_25.gtx\"],
        AUTHORITY[\"EPSG\",\"1027\"]],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Gravity-related height\",UP],
    AUTHORITY[\"EPSG\",\"3855\"]]";
    let crs = parse(wkt1);
    require_bound(&crs);
    assert_eq!(wkt(&crs, WktConvention::Wkt1Gdal), wkt1);
}

/// `crs.WKT1_VERT_DATUM_EXTENSION_to_WKT2`
#[test]
fn crs_wkt1_vert_datum_extension_to_wkt2() {
    let wkt1 = "\
VERT_CS[\"EGM2008 geoid height\",
    VERT_DATUM[\"EGM2008 geoid\",2005,
        EXTENSION[\"PROJ4_GRIDS\",\"egm08_25.gtx\"],
        AUTHORITY[\"EPSG\",\"1027\"]],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Up\",UP],
    AUTHORITY[\"EPSG\",\"3855\"]]";
    let crs = parse(wkt1);
    require_bound(&crs);
    let expected = "\
BOUNDCRS[
    SOURCECRS[
        VERTCRS[\"EGM2008 geoid height\",
            VDATUM[\"EGM2008 geoid\"],
            CS[vertical,1],
                AXIS[\"up\",up,
                    LENGTHUNIT[\"metre\",1]],
            ID[\"EPSG\",3855]]],
    TARGETCRS[
        GEODCRS[\"WGS 84\",
            DATUM[\"World Geodetic System 1984\",
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                    LENGTHUNIT[\"metre\",1]]],
            PRIMEM[\"Greenwich\",0,
                ANGLEUNIT[\"degree\",0.0174532925199433]],
            CS[ellipsoidal,3],
                AXIS[\"latitude\",north,
                    ORDER[1],
                    ANGLEUNIT[\"degree\",0.0174532925199433]],
                AXIS[\"longitude\",east,
                    ORDER[2],
                    ANGLEUNIT[\"degree\",0.0174532925199433]],
                AXIS[\"ellipsoidal height\",up,
                    ORDER[3],
                    LENGTHUNIT[\"metre\",1]],
            ID[\"EPSG\",4979]]],
    ABRIDGEDTRANSFORMATION[\"EGM2008 geoid height to WGS 84 ellipsoidal height\",
        METHOD[\"GravityRelatedHeight to Geographic3D\"],
        PARAMETERFILE[\"Geoid (height correction) model file\",\"egm08_25.gtx\",
            ID[\"EPSG\",8666]]]]";
    assert_eq!(wkt(&crs, WktConvention::Wkt2_2015), expected);
}

/// `crs.WKT1_VERT_DATUM_EXTENSION_to_PROJ_string`
#[test]
fn crs_wkt1_vert_datum_extension_to_proj_string() {
    let wkt1 = "\
VERT_CS[\"EGM2008 geoid height\",
    VERT_DATUM[\"EGM2008 geoid\",2005,
        EXTENSION[\"PROJ4_GRIDS\",\"egm08_25.gtx\"],
        AUTHORITY[\"EPSG\",\"1027\"]],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Up\",UP],
    AUTHORITY[\"EPSG\",\"3855\"]]";
    let crs = parse(wkt1);
    require_bound(&crs);
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+geoidgrids=egm08_25.gtx +geoid_crs=WGS84 +vunits=m +no_defs +type=crs"
    );
}
