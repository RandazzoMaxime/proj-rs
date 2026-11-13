//! PROJ `test/unit/test_io.cpp` checks on WKT1 import (`wkt_parse.wkt1_*`).
//!
//! Each function is one `TEST()` name. Expected text is copied from that test.
//! The WKT inputs are PROJ's, byte for byte.

use proj_rs::{
    Crs, DatabaseContext, GeodeticCRS, GeographicCRS, Parameter, ParameterValue, ProjectedCRS,
    UnitOfMeasure, WktConvention, WktFormatter, WktParser, WktStrictness,
};

fn parse(wkt: &str) -> proj_rs::WktObject {
    WktParser::new().create_from_wkt(wkt).expect("wkt")
}

fn parse_db(wkt: &str) -> proj_rs::WktObject {
    let mut parser = WktParser::new();
    parser.attach_database(DatabaseContext::create().expect("proj.db"));
    parser.create_from_wkt(wkt).expect("wkt")
}

fn parse_loose(wkt: &str) -> proj_rs::WktObject {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser.create_from_wkt(wkt).expect("wkt")
}

fn refused(wkt: &str) {
    assert!(WktParser::new().create_from_wkt(wkt).is_err());
}

/// `checkEPSG_4326(crs, true, checkEPSGCodes)` of test_io.cpp, latitude first.
fn check_epsg_4326(crs: &GeographicCRS) {
    assert_eq!(crs.identifiers().len(), 1);
    assert_eq!(crs.identifiers()[0].code(), "4326");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(crs.name(), "WGS 84");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert!(axes[0].name() == "Latitude" || axes[0].name() == "Geodetic latitude");
    assert_eq!(axes[0].abbreviation().to_ascii_lowercase(), "lat");
    assert_eq!(axes[0].direction(), "north");
    assert!(axes[1].name() == "Longitude" || axes[1].name() == "Geodetic longitude");
    assert_eq!(axes[1].abbreviation().to_ascii_lowercase(), "lon");
    assert_eq!(axes[1].direction(), "east");
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "6326");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    assert_eq!(datum.name(), "World Geodetic System 1984");
    let ellipsoid = datum.ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_137.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
    assert_eq!(ellipsoid.inverse_flattening(), 298.257_223_563);
    assert_eq!(ellipsoid.identifiers().len(), 1);
    assert_eq!(ellipsoid.identifiers()[0].code(), "7030");
    assert_eq!(ellipsoid.identifiers()[0].code_space(), "EPSG");
    assert_eq!(ellipsoid.name(), "WGS 84");
}

fn check_parameter(parameter: &Parameter, name: &str, unit: &UnitOfMeasure, value: f64) {
    assert_eq!(parameter.name(), name);
    let ParameterValue::Measure(measure) = parameter.value() else {
        panic!("{name} is not a measure");
    };
    assert_eq!(measure.unit().name(), unit.name());
    assert_eq!(measure.value(), value);
}

/// `checkProjected` of test_io.cpp: the UTM zone 31N with EPSG:32631.
fn check_projected(crs: &ProjectedCRS) {
    assert_eq!(crs.name(), "WGS 84 / UTM zone 31N");
    assert_eq!(crs.identifiers().len(), 1);
    assert_eq!(crs.identifiers()[0].code(), "32631");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    let GeodeticCRS::Geographic(base) = crs.base_crs() else {
        panic!("base CRS is not geographic");
    };
    check_epsg_4326(base);
    let conversion = crs.deriving_conversion();
    assert_eq!(conversion.name(), "UTM zone 31N");
    assert_eq!(conversion.method().name(), "Transverse Mercator");
    let parameters = conversion.parameters();
    assert_eq!(parameters.len(), 5);
    check_parameter(
        &parameters[0],
        "Latitude of natural origin",
        &UnitOfMeasure::degree(),
        0.0,
    );
    check_parameter(
        &parameters[1],
        "Longitude of natural origin",
        &UnitOfMeasure::degree(),
        3.0,
    );
    check_parameter(
        &parameters[2],
        "Scale factor at natural origin",
        &UnitOfMeasure::unity(),
        0.9996,
    );
    check_parameter(
        &parameters[3],
        "False easting",
        &UnitOfMeasure::metre(),
        500_000.0,
    );
    check_parameter(
        &parameters[4],
        "False northing",
        &UnitOfMeasure::metre(),
        0.0,
    );
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].name(), "Easting");
    assert_eq!(axes[0].abbreviation(), "E");
    assert_eq!(axes[0].direction(), "east");
    assert_eq!(axes[1].name(), "Northing");
    assert_eq!(axes[1].abbreviation(), "N");
    assert_eq!(axes[1].direction(), "north");
}

#[test]
fn wkt_parse_wkt1_esri_epsg_4901_grad() {
    let crs = parse_db(r#"GEOGCS["GCS_ATF_Paris",DATUM["D_ATF",SPHEROID["Plessis_1817",6376523.0,308.64]],PRIMEM["Paris_RGS",2.33720833333333],UNIT["Grad",0.0157079632679489]]"#).geographic().expect("crs").clone();
    let primem = crs.datum().expect("datum").prime_meridian();
    assert_eq!(primem.name(), "Paris RGS");
    // The PRIMEM is really in degree
    assert_eq!(
        primem.longitude().unit().name(),
        UnitOfMeasure::degree().name()
    );
    assert!((primem.longitude().value() - 2.337_208_333_333_33).abs() < 1e-14);
}

#[test]
fn wkt_parse_wkt1_esri_linunit() {
    let crs = parse_db(r#"GEOGCS["WGS_1984_3D",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433],LINUNIT["Meter",1.0]]"#).geographic().expect("crs").clone();
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert!((axes[0].unit().conversion_to_si() - 0.0174532925199433).abs() < 1e-15);
    assert!((axes[1].unit().conversion_to_si() - 0.0174532925199433).abs() < 1e-15);
    assert_eq!(axes[2].unit().name(), UnitOfMeasure::metre().name());
}

#[test]
fn wkt_parse_wkt1_geographic_old_datum_name_from_epsg_code() {
    let crs = parse_db(
        r#"GEOGCS["S-JTSK (Ferro)",
    DATUM["System_Jednotne_Trigonometricke_Site_Katastralni_Ferro",
        SPHEROID["Bessel 1841",6377397.155,299.1528128,
            AUTHORITY["EPSG","7004"]],
        AUTHORITY["EPSG","6818"]],
    PRIMEM["Ferro",-17.66666666666667,
       AUTHORITY["EPSG","8909"]],
    UNIT["degree",0.0174532925199433,
        AUTHORITY["EPSG","9122"]],
    AUTHORITY["EPSG","4818"]]"#,
    )
    .geographic()
    .expect("crs")
    .clone();
    assert_eq!(
        crs.datum().expect("datum").name(),
        "System of the Unified Trigonometrical Cadastral Network (Ferro)"
    );
}

#[test]
fn wkt_parse_wkt1_geographic_old_datum_name_without_epsg_code() {
    let crs = parse_db(
        r#"GEOGCS["S-JTSK (Ferro)",
    DATUM["System_Jednotne_Trigonometricke_Site_Katastralni_Ferro",
        SPHEROID["Bessel 1841",6377397.155,299.1528128]],
    PRIMEM["Ferro",-17.66666666666667],
    UNIT["degree",0.0174532925199433]]"#,
    )
    .geographic()
    .expect("crs")
    .clone();
    assert_eq!(
        crs.datum().expect("datum").name(),
        "System of the Unified Trigonometrical Cadastral Network (Ferro)"
    );
}

#[test]
fn wkt_parse_wkt1_non_conformant_inf_inverse_flattening() {
    let crs = parse_loose(r#"GEOGCS["GCS_sphere",DATUM["D_unknown",SPHEROID["Spherical_Earth",6370997,"inf"]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]]"#).geographic().expect("crs").clone();
    assert_eq!(
        crs.ellipsoid().expect("ellipsoid").inverse_flattening(),
        0.0
    );
}

#[test]
fn wkt_parse_wkt1_esri_gcs_unknown_d_unknown() {
    let crs = parse(r#"GEOGCS["GCS_unknown",DATUM["D_unknown",SPHEROID["unknown",6370997,0]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]]"#).geographic().expect("crs").clone();
    assert_eq!(crs.name(), "unknown");
    assert_eq!(crs.datum().expect("datum").name(), "unknown");
}

#[test]
fn wkt_parse_wkt1_esri_gcs_unknown_d_unknown_based_on_wgs_84_ellipsoid() {
    let crs = parse(r#"GEOGCS["GCS_unknown",DATUM["D_Unknown_based_on_WGS_84_ellipsoid",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#).geographic().expect("crs").clone();
    assert_eq!(crs.name(), "unknown");
    assert_eq!(
        crs.datum().expect("datum").name(),
        "Unknown based on WGS 84 ellipsoid"
    );
}

#[test]
fn wkt_parse_wkt1_projected() {
    let crs = parse_db(
        r#"PROJCS["WGS 84 / UTM zone 31N",
    GEOGCS["WGS 84",
        DATUM["WGS_1984",
            SPHEROID["WGS 84",6378137,298.257223563,
                AUTHORITY["EPSG","7030"]],
            AUTHORITY["EPSG","6326"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4326"]],
    PROJECTION["Transverse_Mercator"],
    PARAMETER["latitude_of_origin",0],
    PARAMETER["central_meridian",3],
    PARAMETER["scale_factor",0.9996],
    PARAMETER["false_easting",500000],
    PARAMETER["false_northing",0],
    UNIT["metre",1,
        AUTHORITY["EPSG","9001"]],
    AXIS["(E)",East],
    AXIS["(N)",NORTH],
    AUTHORITY["EPSG","32631"]]"#,
    )
    .projected()
    .expect("crs")
    .clone();
    check_projected(&crs);
    assert!(!crs.base_crs().identifiers().is_empty());
}

#[test]
fn wkt_parse_wkt1_projected_no_axis() {
    let crs = parse(
        r#"PROJCS["WGS 84 / UTM zone 31N",
    GEOGCS["WGS 84",
        DATUM["WGS_1984",
            SPHEROID["WGS 84",6378137,298.257223563,
                AUTHORITY["EPSG","7030"]],
            AUTHORITY["EPSG","6326"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AXIS["latitude",NORTH],
        AXIS["longitude",EAST],
        AUTHORITY["EPSG","4326"]],
    PROJECTION["Transverse_Mercator"],
    PARAMETER["latitude_of_origin",0],
    PARAMETER["central_meridian",3],
    PARAMETER["scale_factor",0.9996],
    PARAMETER["false_easting",500000],
    PARAMETER["false_northing",0],
    UNIT["metre",1,
        AUTHORITY["EPSG","9001"]],
    AUTHORITY["EPSG","32631"]]"#,
    )
    .projected()
    .expect("crs")
    .clone();
    check_projected(&crs);
}

#[test]
fn wkt_parse_wkt1_projected_missing_unit_and_axis() {
    let wkt = r#"PROJCS["WGS 84 / UTM zone 31N",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4326"]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",3],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0]]"#;
    // Missing UNIT[] is illegal in strict mode
    refused(wkt);
    let crs = parse_loose(wkt).projected().expect("crs").clone();
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].unit().name(), UnitOfMeasure::metre().name());
}

#[test]
fn wkt_parse_wkt1_projected_wrong_angular_unit() {
    let wkt = r#"PROJCS["Merchich / Nord Maroc",    GEOGCS["Merchich",        DATUM["Merchich",            SPHEROID["Clarke 1880 (IGN)",6378249.2,293.466021293627]],        PRIMEM["Greenwich",0],        UNIT["grad",0.015707963267949,            AUTHORITY["EPSG","9105"]],        AUTHORITY["EPSG","4261"]],    PROJECTION["Lambert_Conformal_Conic_1SP"],    PARAMETER["latitude_of_origin",37],    PARAMETER["central_meridian",-6],    PARAMETER["scale_factor",0.999625769],    PARAMETER["false_easting",500000],    PARAMETER["false_northing",300000],    UNIT["metre",1,        AUTHORITY["EPSG","9001"]],    AXIS["Easting",EAST],    AXIS["Northing",NORTH]]"#;
    {
        let mut parser = WktParser::new();
        parser.set_strictness(WktStrictness::Lenient);
        parser.attach_database(DatabaseContext::create().expect("proj.db"));
        let obj = parser.create_from_wkt(wkt).expect("wkt");
        assert!(!parser.warnings().is_empty());
        let crs = obj.projected().expect("crs").clone();
        // No base CRS identifiers
        assert!(crs.base_crs().identifiers().is_empty());
        let base_cs = crs.base_crs().coordinate_system();
        let axes = base_cs.axes();
        assert_eq!(axes.len(), 2);
        assert!(
            (axes[0].unit().conversion_to_si() - UnitOfMeasure::grad().conversion_to_si()).abs()
                < 1e-10
        );
    }
    {
        let mut parser = WktParser::new();
        parser.keep_incompatible_identifiers();
        parser.attach_database(DatabaseContext::create().expect("proj.db"));
        let obj = parser.create_from_wkt(wkt).expect("wkt");
        assert!(parser.warnings().is_empty());
        let crs = obj.projected().expect("crs").clone();
        // Base CRS identifier preserved
        assert!(!crs.base_crs().identifiers().is_empty());
        let base_cs = crs.base_crs().coordinate_system();
        let axes = base_cs.axes();
        assert_eq!(axes.len(), 2);
        assert!(
            (axes[0].unit().conversion_to_si() - UnitOfMeasure::grad().conversion_to_si()).abs()
                < 1e-10
        );
    }
}

#[test]
fn wkt_parse_wkt1_projected_wrong_axis_geogcs() {
    let wkt = r#"PROJCS["WGS 84 / UTM zone 31N",
    GEOGCS["WGS 84",
        DATUM["WGS_1984",
            SPHEROID["WGS 84",6378137,298.257223563,
                AUTHORITY["EPSG","7030"]],
            AUTHORITY["EPSG","6326"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AXIS["longitude",EAST],
        AXIS["latitude",NORTH],
        AUTHORITY["EPSG","4326"]],
    PROJECTION["Transverse_Mercator"],
    PARAMETER["latitude_of_origin",0],
    PARAMETER["central_meridian",3],
    PARAMETER["scale_factor",0.9996],
    PARAMETER["false_easting",500000],
    PARAMETER["false_northing",0],
    UNIT["metre",1,
        AUTHORITY["EPSG","9001"]],
    AUTHORITY["EPSG","32631"]]"#;
    {
        let mut parser = WktParser::new();
        parser.set_strictness(WktStrictness::Lenient);
        parser.attach_database(DatabaseContext::create().expect("proj.db"));
        let obj = parser.create_from_wkt(wkt).expect("wkt");
        assert!(!parser.warnings().is_empty());
        let crs = obj.projected().expect("crs").clone();
        assert!(crs.base_crs().identifiers().is_empty());
        let base_cs = crs.base_crs().coordinate_system();
        let axes = base_cs.axes();
        assert_eq!(axes.len(), 2);
        assert_eq!(axes[0].direction(), "east");
        assert_eq!(axes[1].direction(), "north");
    }
    {
        let mut parser = WktParser::new();
        parser.set_strictness(WktStrictness::Lenient);
        parser.keep_incompatible_identifiers();
        parser.attach_database(DatabaseContext::create().expect("proj.db"));
        let obj = parser.create_from_wkt(wkt).expect("wkt");
        assert!(parser.warnings().is_empty());
        let crs = obj.projected().expect("crs").clone();
        assert!(!crs.base_crs().identifiers().is_empty());
    }
}

#[test]
fn wkt_parse_wkt1_hotine_oblique_mercator_without_rectified_grid_angle() {
    let crs = parse(r#"PROJCS["NAD_1983_Michigan_GeoRef_Meters",GEOGCS["NAD83(1986)",DATUM["North_American_Datum_1983",SPHEROID["GRS_1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]],PROJECTION["Hotine_Oblique_Mercator"],PARAMETER["false_easting",2546731.496],PARAMETER["false_northing",-4354009.816],PARAMETER["latitude_of_center",45.30916666666666],PARAMETER["longitude_of_center",-86],PARAMETER["azimuth",-22.74444],PARAMETER["scale_factor",0.9996],UNIT["Meter",1]]"#).projected().expect("crs").clone();
    // Check that we have added automatically rectified_grid_angle
    let got = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("WKT1");
    assert!(
        got.contains("PARAMETER[\"rectified_grid_angle\",-22.74444]"),
        "{got}"
    );
}

#[test]
fn wkt_parse_wkt1_hotine_oblique_mercator_with_rectified_grid_angle() {
    let crs = parse(r#"PROJCS["NAD_1983_Michigan_GeoRef_Meters",GEOGCS["NAD83(1986)",DATUM["North_American_Datum_1983",SPHEROID["GRS_1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]],PROJECTION["Hotine_Oblique_Mercator"],PARAMETER["false_easting",2546731.496],PARAMETER["false_northing",-4354009.816],PARAMETER["latitude_of_center",45.30916666666666],PARAMETER["longitude_of_center",-86],PARAMETER["azimuth",-22.74444],PARAMETER["rectified_grid_angle",-23],PARAMETER["scale_factor",0.9996],UNIT["Meter",1]]"#).projected().expect("crs").clone();
    // Check that we have not overridden rectified_grid_angle
    let got = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("WKT1");
    assert!(
        got.contains("PARAMETER[\"rectified_grid_angle\",-23]"),
        "{got}"
    );
}

#[test]
fn wkt_parse_wkt1_hotine_oblique_mercator_azimuth_center_with_rectified_grid_angle() {
    let crs = parse(r#"PROJCS["unknown",GEOGCS["unknown",    DATUM["WGS_1984",        SPHEROID["WGS 84",6378137,298.257223563]],    PRIMEM["Greenwich",0],    UNIT["degree",0.0174532925199433]],PROJECTION["Hotine_Oblique_Mercator_Azimuth_Center"],PARAMETER["latitude_of_center",0],PARAMETER["longitude_of_center",0],PARAMETER["azimuth",30],PARAMETER["rectified_grid_angle",0],PARAMETER["scale_factor",1],PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["metre",1]]"#).projected().expect("crs").clone();
    // Check that we have not overridden rectified_grid_angle
    let got = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("WKT1");
    assert!(
        got.contains("PARAMETER[\"rectified_grid_angle\",0]"),
        "{got}"
    );
}

#[test]
fn wkt_parse_wkt1_geographic_deprecated() {
    let crs = parse_db(
        r#"GEOGCS["SAD69 (deprecated)",
    DATUM["South_American_Datum_1969",
        SPHEROID["GRS 1967",6378160,298.247167427,
            AUTHORITY["EPSG","7036"]],
        AUTHORITY["EPSG","6291"]],
    PRIMEM["Greenwich",0,
        AUTHORITY["EPSG","8901"]],
    UNIT["degree",0.0174532925199433,
        AUTHORITY["EPSG","9108"]],
    AUTHORITY["EPSG","4291"]]"#,
    )
    .geographic()
    .expect("crs")
    .clone();
    assert_eq!(crs.name(), "SAD69");
    assert!(Crs::Geographic(crs).is_deprecated());
}

#[test]
fn wkt_parse_wkt1_geographic_with_proj4_extension() {
    let wkt = r#"GEOGCS["WGS 84",
    DATUM["unknown",
        SPHEROID["WGS84",6378137,298.257223563]],
    PRIMEM["Greenwich",0],
    UNIT["degree",0.0174532925199433],
    EXTENSION["PROJ4","+proj=longlat +foo=bar +wktext"]]"#;
    let crs = Crs::Geographic(parse(wkt).geographic().expect("crs").clone());
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("WKT1"),
        wkt
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ"),
        "+proj=longlat +foo=bar +wktext +type=crs"
    );
    assert!(!crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2")
        .contains("EXTENSION"));
    assert!(!crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri))
        .expect("ESRI")
        .contains("EXTENSION"));
}

#[test]
fn wkt_parse_wkt1_geocentric_with_proj4_extension() {
    let wkt = r#"GEOCCS["WGS 84",
    DATUM["unknown",
        SPHEROID["WGS84",6378137,298.257223563]],
    PRIMEM["Greenwich",0],
    UNIT["Meter",1],
    AXIS["Geocentric X",OTHER],
    AXIS["Geocentric Y",OTHER],
    AXIS["Geocentric Z",NORTH],
    EXTENSION["PROJ4","+proj=geocent +foo=bar +wktext"]]"#;
    let crs = Crs::Geodetic(parse(wkt).geodetic_crs().expect("crs").clone());
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("WKT1"),
        wkt
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ"),
        "+proj=geocent +foo=bar +wktext +type=crs"
    );
    assert!(!crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2")
        .contains("EXTENSION"));
}

#[test]
fn wkt_parse_wkt1_projected_with_proj4_extension() {
    let wkt = r#"PROJCS["unnamed",
    GEOGCS["WGS 84",
        DATUM["unknown",
            SPHEROID["WGS84",6378137,298.257223563]],
        PRIMEM["Greenwich",0],
        UNIT["degree",0.0174532925199433]],
    PROJECTION["Mercator_1SP"],
    PARAMETER["central_meridian",0],
    PARAMETER["scale_factor",1],
    PARAMETER["false_easting",0],
    PARAMETER["false_northing",0],
    UNIT["Meter",1],
    AXIS["Easting",EAST],
    AXIS["Northing",NORTH],
    EXTENSION["PROJ4","+proj=merc +wktext"]]"#;
    let crs = Crs::Projected(parse(wkt).projected().expect("crs").clone());
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("WKT1"),
        wkt
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ"),
        "+proj=merc +wktext +type=crs"
    );
    assert!(!crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2")
        .contains("EXTENSION"));
    assert!(!crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri))
        .expect("ESRI")
        .contains("EXTENSION"));
}

#[test]
fn wkt_parse_wkt1_krovak_north_oriented() {
    let obj = WktParser::new().create_from_wkt(r#"PROJCS["S-JTSK / Krovak East North",    GEOGCS["S-JTSK",        DATUM["System_Jednotne_Trigonometricke_Site_Katastralni",            SPHEROID["Bessel 1841",6377397.155,299.1528128,                AUTHORITY["EPSG","7004"]],            AUTHORITY["EPSG","6156"]],        PRIMEM["Greenwich",0,            AUTHORITY["EPSG","8901"]],        UNIT["degree",0.0174532925199433,            AUTHORITY["EPSG","9122"]],        AUTHORITY["EPSG","4156"]],    PROJECTION["Krovak"],    PARAMETER["latitude_of_center",49.5],    PARAMETER["longitude_of_center",24.83333333333333],    PARAMETER["azimuth",30.2881397527778],    PARAMETER["pseudo_standard_parallel_1",78.5],    PARAMETER["scale_factor",0.9999],    PARAMETER["false_easting",0],    PARAMETER["false_northing",0],    UNIT["metre",1,        AUTHORITY["EPSG","9001"]],    AXIS["X",EAST],    AXIS["Y",NORTH],    AUTHORITY["EPSG","5514"]]"#).expect("wkt");
    let crs = obj.projected().expect("crs").clone();
    assert_eq!(
        crs.deriving_conversion().method().name(),
        "Krovak (North Orientated)"
    );
    assert_eq!(
        Crs::Projected(crs.clone())
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2"),
        r#"PROJCRS["S-JTSK / Krovak East North",
    BASEGEODCRS["S-JTSK",
        DATUM["System_Jednotne_Trigonometricke_Site_Katastralni",
            ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]]],
    CONVERSION["unknown",
        METHOD["Krovak (North Orientated)",
            ID["EPSG",1041]],
        PARAMETER["Latitude of projection centre",49.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8811]],
        PARAMETER["Longitude of origin",24.8333333333333,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8833]],
        PARAMETER["Co-latitude of cone axis",30.2881397527778,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",1036]],
        PARAMETER["Latitude of pseudo standard parallel",78.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8818]],
        PARAMETER["Scale factor on pseudo standard parallel",0.9999,
            SCALEUNIT["unity",1],
            ID["EPSG",8819]],
        PARAMETER["False easting",0,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",0,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]]],
    CS[Cartesian,2],
        AXIS["x",east,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["y",north,
            ORDER[2],
            LENGTHUNIT["metre",1]],
    ID["EPSG",5514]]"#
    );
    assert_eq!(
        Crs::Projected(crs).export_to_proj_string().expect("PROJ"),
        "+proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +units=m +no_defs +type=crs"
    );
}
