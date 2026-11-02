//! PROJ `test/unit/test_io.cpp` checks on WKT2 and vertical, compound and bound
//! CRS import (`wkt_parse.*` in the `core` class of `docs/UNIT-TRIAGE.md`).
//!
//! Each function is one `TEST()` name. Expected text is copied from that test.
//! The WKT inputs are PROJ's, byte for byte, including the line breaks of the
//! tests that compare an export to the input.

use proj_rs::{
    create_crs_from_proj_string, Crs, DatabaseContext, GeodeticCRS, GeographicCRS, LinunitPolicy,
    Parameter, ParameterValue, ProjectedCRS, UnitOfMeasure, WktConvention, WktDialect,
    WktFormatter, WktParser, WktStrictness,
};

fn parse(wkt: &str) -> proj_rs::WktObject {
    WktParser::new().create_from_wkt(wkt).expect("wkt")
}

fn parse_db(wkt: &str) -> proj_rs::WktObject {
    let mut parser = WktParser::new();
    parser.attach_database(DatabaseContext::create().expect("proj.db"));
    parser.create_from_wkt(wkt).expect("wkt")
}

#[expect(
    dead_code,
    reason = "ESRI PROJCS cases in this lot still parse through the lenient parser"
)]
fn parse_loose(wkt: &str) -> proj_rs::WktObject {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser.create_from_wkt(wkt).expect("wkt")
}

fn refused(wkt: &str) {
    assert!(WktParser::new().create_from_wkt(wkt).is_err());
}

fn export(crs: &Crs, convention: WktConvention) -> Result<String, String> {
    crs.export_to_wkt(&mut WktFormatter::create(convention))
        .map_err(|e| format!("{e:?}"))
}

/// `WKTFormatter::create(convention, dbContext)` of test_io.cpp.
fn export_db(crs: &Crs, convention: WktConvention, db: &DatabaseContext) -> Result<String, String> {
    crs.export_to_wkt(&mut WktFormatter::create(convention).with_database(db.clone()))
        .map_err(|e| format!("{e:?}"))
}

/// `BoundCRS::transformation` of a bound CRS built from a transformation.
fn transformation_of(bound: &proj_rs::BoundCRS) -> &proj_rs::Transformation {
    let proj_rs::CoordinateOperation::Transformation(transformation) = bound.transformation()
    else {
        panic!("not a transformation");
    };
    transformation
}

/// The bound CRS of a `COMPD_CS` whose horizontal part carries TOWGS84.
fn bound_base_geographic(crs: &proj_rs::BoundCRS) -> &GeographicCRS {
    let Crs::Geographic(base) = crs.source_crs() else {
        panic!("base is not geographic");
    };
    base
}

#[expect(
    dead_code,
    reason = "parameter checks for the remaining ESRI projection table"
)]
fn check_parameter(parameter: &Parameter, name: &str, unit: &UnitOfMeasure, value: f64) {
    assert_eq!(parameter.name(), name);
    let ParameterValue::Measure(measure) = parameter.value() else {
        panic!("{name} is not a measure");
    };
    assert_eq!(measure.unit().name(), unit.name());
    assert_eq!(measure.value(), value);
}

/// `checkEPSG_4326(crs, true, false)` of test_io.cpp, latitude first: no EPSG codes.
#[expect(
    dead_code,
    reason = "EPSG:4326 check used by the remaining ESRI and axis cases"
)]
fn check_epsg_4326(crs: &GeographicCRS) {
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
    assert_eq!(datum.name(), "World Geodetic System 1984");
    let ellipsoid = datum.ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_137.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
    assert_eq!(ellipsoid.inverse_flattening(), 298.257_223_563);
    assert_eq!(ellipsoid.name(), "WGS 84");
}

/// `checkEPSG_4979` of test_io.cpp.
#[expect(
    dead_code,
    reason = "EPSG:4979 check used by the remaining compound cases"
)]
fn check_epsg_4979(crs: &GeographicCRS) {
    assert_eq!(crs.identifiers().len(), 1);
    assert_eq!(crs.identifiers()[0].code(), "4979");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(crs.name(), "WGS 84");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert_eq!(axes[0].name(), "Latitude");
    assert_eq!(axes[0].abbreviation(), "lat");
    assert_eq!(axes[0].direction(), "north");
    assert_eq!(axes[1].name(), "Longitude");
    assert_eq!(axes[1].abbreviation(), "lon");
    assert_eq!(axes[1].direction(), "east");
    assert_eq!(axes[2].name(), "Ellipsoidal height");
    assert_eq!(axes[2].abbreviation(), "h");
    assert_eq!(axes[2].direction(), "up");
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.name(), "World Geodetic System 1984");
    let ellipsoid = datum.ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_137.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
    assert_eq!(ellipsoid.inverse_flattening(), 298.257_223_563);
    assert_eq!(ellipsoid.name(), "WGS 84");
}

/// `checkProjected(crs, false)` of test_io.cpp: the UTM zone 31N with EPSG:32631.
#[expect(
    dead_code,
    reason = "UTM zone 31N check used by the remaining projected cases"
)]
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
fn wkt_parse_wkt2_spherical_planetocentric() {
    let wkt = r#"GEODCRS["Mercury (2015) / Ocentric",
    DATUM["Mercury (2015)",
        ELLIPSOID["Mercury (2015)",2440530,1075.12334801762,
            LENGTHUNIT["metre",1]],
        ANCHOR["Hun Kal: 20.0"]],
    PRIMEM["Reference Meridian",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[spherical,2],
        AXIS["planetocentric latitude (U)",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433]],
        AXIS["planetocentric longitude (V)",east,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433]],
    ID["IAU",19902,2015],
    REMARK["Source of IAU Coordinate systems: doi://10.1007/s10569-017-9805-5"]]"#;
    let crs = parse(wkt).geodetic_crs().expect("crs").clone();
    assert!(matches!(crs, GeodeticCRS::Spherical(_)));
    assert_eq!(
        export(&Crs::Geodetic(crs), WktConvention::Wkt2_2019).expect("WKT2"),
        wkt
    );
}

#[test]
fn wkt_parse_wkt2_krovak_modified_south_west() {
    let wkt = r#"PROJCRS["S-JTSK/05 / Modified Krovak",
    BASEGEOGCRS["S-JTSK/05",
        DATUM["System of the Unified Trigonometrical Cadastral Network/05",
            ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",5228]],
    CONVERSION["Modified Krovak (Greenwich)",
        METHOD["Krovak Modified",
            ID["EPSG",1042]],
        PARAMETER["Latitude of projection centre",49.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8811]],
        PARAMETER["Longitude of origin",24.8333333333333,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8833]],
        PARAMETER["Co-latitude of cone axis",30.2881397222222,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",1036]],
        PARAMETER["Latitude of pseudo standard parallel",78.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8818]],
        PARAMETER["Scale factor on pseudo standard parallel",0.9999,
            SCALEUNIT["unity",1],
            ID["EPSG",8819]],
        PARAMETER["False easting",5000000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",5000000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]]],
    CS[Cartesian,2],
        AXIS["southing (X)",south,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["westing (Y)",west,
            ORDER[2],
            LENGTHUNIT["metre",1]],
    USAGE[
        SCOPE["Engineering survey, topographic mapping."],
        AREA["Czechia."],
        BBOX[48.58,12.09,51.06,18.86]],
    ID["EPSG",5515]]"#;
    let crs = parse(wkt).projected().expect("crs").clone();
    assert_eq!(crs.deriving_conversion().method().name(), "Krovak Modified");
    assert_eq!(
        export(&Crs::Projected(crs.clone()), WktConvention::Wkt2_2019).expect("WKT2"),
        wkt
    );
    let expected_proj = "+proj=mod_krovak +axis=swu +lat_0=49.5 +lon_0=24.8333333333333 \
        +alpha=30.2881397222222 +k=0.9999 +x_0=5000000 +y_0=5000000 \
        +ellps=bessel +units=m +no_defs +type=crs";
    let proj = Crs::Projected(crs).export_to_proj_string().expect("PROJ");
    assert_eq!(proj, expected_proj);

    let Crs::Projected(crs2) = create_crs_from_proj_string(&proj).expect("import") else {
        panic!("not projected");
    };
    let wkt2 = export(&Crs::Projected(crs2.clone()), WktConvention::Wkt2_2019).expect("WKT2");
    assert!(wkt2.contains("METHOD[\"Krovak Modified\""), "{wkt2}");
    assert!(
        wkt2.contains("PARAMETER[\"Latitude of pseudo standard parallel\",78.5,"),
        "{wkt2}"
    );
    assert!(
        wkt2.contains("PARAMETER[\"Co-latitude of cone axis\",30.2881397222222,"),
        "{wkt2}"
    );
    assert_eq!(
        Crs::Projected(crs2).export_to_proj_string().expect("PROJ"),
        expected_proj
    );

    let Crs::Projected(crs3) = create_crs_from_proj_string(
        "+type=crs +proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=mod_krovak +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397222222 +k=0.9999 +x_0=5000000 +y_0=5000000 +ellps=bessel \
         +step +proj=axisswap +order=-2,-1",
    )
    .expect("import") else {
        panic!("not projected");
    };
    let wkt3 = export(&Crs::Projected(crs3), WktConvention::Wkt2_2019).expect("WKT2");
    assert!(wkt3.contains("METHOD[\"Krovak Modified\""), "{wkt3}");
}

#[test]
fn wkt_parse_wkt2_krovak_modified_east_north() {
    let wkt = r#"PROJCRS["S-JTSK/05 / Modified Krovak East North",
    BASEGEOGCRS["S-JTSK/05",
        DATUM["System of the Unified Trigonometrical Cadastral Network/05",
            ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",5228]],
    CONVERSION["Modified Krovak East North (Greenwich)",
        METHOD["Krovak Modified (North Orientated)",
            ID["EPSG",1043]],
        PARAMETER["Latitude of projection centre",49.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8811]],
        PARAMETER["Longitude of origin",24.8333333333333,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8833]],
        PARAMETER["Co-latitude of cone axis",30.2881397222222,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",1036]],
        PARAMETER["Latitude of pseudo standard parallel",78.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8818]],
        PARAMETER["Scale factor on pseudo standard parallel",0.9999,
            SCALEUNIT["unity",1],
            ID["EPSG",8819]],
        PARAMETER["False easting",5000000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",5000000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]]],
    CS[Cartesian,2],
        AXIS["easting (X)",east,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["northing (Y)",north,
            ORDER[2],
            LENGTHUNIT["metre",1]],
    USAGE[
        SCOPE["GIS."],
        AREA["Czechia."],
        BBOX[48.58,12.09,51.06,18.86]],
    ID["EPSG",5516]]"#;
    let crs = parse(wkt).projected().expect("crs").clone();
    assert_eq!(
        crs.deriving_conversion().method().name(),
        "Krovak Modified (North Orientated)"
    );
    assert_eq!(
        export(&Crs::Projected(crs.clone()), WktConvention::Wkt2_2019).expect("WKT2"),
        wkt
    );
    let expected_proj = "+proj=mod_krovak +lat_0=49.5 +lon_0=24.8333333333333 \
        +alpha=30.2881397222222 +k=0.9999 +x_0=5000000 +y_0=5000000 \
        +ellps=bessel +units=m +no_defs +type=crs";
    let proj = Crs::Projected(crs).export_to_proj_string().expect("PROJ");
    assert_eq!(proj, expected_proj);

    let Crs::Projected(crs2) = create_crs_from_proj_string(&proj).expect("import") else {
        panic!("not projected");
    };
    let wkt2 = export(&Crs::Projected(crs2.clone()), WktConvention::Wkt2_2019).expect("WKT2");
    assert!(
        wkt2.contains("METHOD[\"Krovak Modified (North Orientated)\""),
        "{wkt2}"
    );
    assert!(
        wkt2.contains("PARAMETER[\"Latitude of pseudo standard parallel\",78.5,"),
        "{wkt2}"
    );
    assert!(
        wkt2.contains("PARAMETER[\"Co-latitude of cone axis\",30.2881397222222,"),
        "{wkt2}"
    );
    assert_eq!(
        Crs::Projected(crs2).export_to_proj_string().expect("PROJ"),
        expected_proj
    );

    let Crs::Projected(crs3) = create_crs_from_proj_string(
        "+type=crs +proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=mod_krovak +lat_0=49.5 +lon_0=24.8333333333333 \
         +alpha=30.2881397222222 +k=0.9999 +x_0=5000000 +y_0=5000000 +ellps=bessel",
    )
    .expect("import") else {
        panic!("not projected");
    };
    let wkt3 = export(&Crs::Projected(crs3), WktConvention::Wkt2_2019).expect("WKT2");
    assert!(
        wkt3.contains("METHOD[\"Krovak Modified (North Orientated)\""),
        "{wkt3}"
    );
}

#[test]
fn wkt_parse_wkt2_2019_projected_3d() {
    let wkt = r#"PROJCRS["WGS 84 (G1762) / UTM zone 31N 3D",BASEGEOGCRS["WGS 84",DATUM["World Geodetic System of 1984 (G1762)",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1.0]]]],CONVERSION["Some conversion 3D",METHOD["Transverse Mercator (3D)"],PARAMETER["Latitude of origin",0.0,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Longitude of origin",3.0,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Scale factor",1,SCALEUNIT["unity",1.0]],PARAMETER["False easting",0.0,LENGTHUNIT["metre",1.0]],PARAMETER["False northing",0.0,LENGTHUNIT["metre",1.0]]],CS[Cartesian,3],AXIS["(E)",east,ORDER[1]],AXIS["(N)",north,ORDER[2]],AXIS["ellipsoidal height (h)",up,ORDER[3]],LENGTHUNIT["metre",1.0]]"#;
    let crs = Crs::Projected(parse(wkt).projected().expect("crs").clone());
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ"),
        "+proj=tmerc +lat_0=0 +lon_0=3 +k=1 +x_0=0 +y_0=0 +ellps=WGS84 +units=m +no_defs +type=crs"
    );
    assert!(export(&crs, WktConvention::Wkt2_2015).is_err());
    assert!(export(&crs, WktConvention::Wkt2_2019).is_ok());
}

#[test]
fn wkt_parse_wkt2_2019_projected_utm_3d() {
    let wkt = r#"PROJCRS["WGS 84 (G1762) / UTM zone 31N 3D",BASEGEOGCRS["WGS 84",DATUM["World Geodetic System of 1984 (G1762)",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1.0]]]],CONVERSION["UTM zone 31N 3D",METHOD["Transverse Mercator (3D)"],PARAMETER["Latitude of origin",0.0,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Longitude of origin",3.0,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Scale factor",0.9996,SCALEUNIT["unity",1.0]],PARAMETER["False easting",500000.0,LENGTHUNIT["metre",1.0]],PARAMETER["False northing",0.0,LENGTHUNIT["metre",1.0]]],CS[Cartesian,3],AXIS["(E)",east,ORDER[1]],AXIS["(N)",north,ORDER[2]],AXIS["ellipsoidal height (h)",up,ORDER[3]],LENGTHUNIT["metre",1.0]]"#;
    let crs = Crs::Projected(parse(wkt).projected().expect("crs").clone());
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ"),
        "+proj=utm +zone=31 +ellps=WGS84 +units=m +no_defs +type=crs"
    );
    assert!(export(&crs, WktConvention::Wkt2_2015).is_err());
    assert!(export(&crs, WktConvention::Wkt2_2019).is_ok());
}

#[test]
fn wkt_parse_wkt2_2019_projected_with_base_geocentric() {
    let wkt = r#"PROJCRS["EPSG topocentric example B",
    BASEGEODCRS["WGS 84",
        ENSEMBLE["World Geodetic System 1984 ensemble",
            MEMBER["World Geodetic System 1984 (Transit)"],
            MEMBER["World Geodetic System 1984 (G730)"],
            MEMBER["World Geodetic System 1984 (G873)"],
            MEMBER["World Geodetic System 1984 (G1150)"],
            MEMBER["World Geodetic System 1984 (G1674)"],
            MEMBER["World Geodetic System 1984 (G1762)"],
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]],
            ENSEMBLEACCURACY[2.0]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4978]],
    CONVERSION["EPSG topocentric example B",
        METHOD["Geocentric/topocentric conversions",
            ID["EPSG",9836]],
        PARAMETER["Geocentric X of topocentric origin",3771793.97,
            LENGTHUNIT["metre",1],
            ID["EPSG",8837]],
        PARAMETER["Geocentric Y of topocentric origin",140253.34,
            LENGTHUNIT["metre",1],
            ID["EPSG",8838]],
        PARAMETER["Geocentric Z of topocentric origin",5124304.35,
            LENGTHUNIT["metre",1],
            ID["EPSG",8839]]],
    CS[Cartesian,3],
        AXIS["topocentric East (U)",east,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["topocentric North (V)",north,
            ORDER[2],
            LENGTHUNIT["metre",1]],
        AXIS["topocentric height (W)",up,
            ORDER[3],
            LENGTHUNIT["metre",1]],
    USAGE[
        SCOPE["Example only (fictitious)."],
        AREA["Description of the extent of the CRS."],
        BBOX[-90,-180,90,180]],
    ID["EPSG",5820]]"#;
    // Need a database so that EPSG:4978 is resolved
    let obj = parse_db(wkt);
    let crs = obj.projected().expect("crs").clone();
    assert!(matches!(crs.base_crs(), GeodeticCRS::Geocentric(_)));
}

#[test]
fn wkt_parse_wkt2_2019_eqdc_non_epsg() {
    let crs = parse(
        r#"PROJCRS["unknown",
    BASEGEOGCRS["unknown",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]],
            ID["EPSG",6326]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8901]]],
    CONVERSION["unknown",
        METHOD["Equidistant Conic"],
        PARAMETER["Latitude of natural origin",1,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8801]],
        PARAMETER["Longitude of natural origin",2,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8802]],
        PARAMETER["Latitude of 1st standard parallel",3,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8823]],
        PARAMETER["Latitude of 2nd standard parallel",4,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8824]],
        PARAMETER["False easting",5,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",6,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]]],
    CS[Cartesian,2],
        AXIS["(E)",east,
            ORDER[1],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["(N)",north,
            ORDER[2],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#,
    )
    .projected()
    .expect("crs")
    .clone();
    assert_eq!(
        Crs::Projected(crs).export_to_proj_string().expect("PROJ"),
        "+proj=eqdc +lat_0=1 +lon_0=2 +lat_1=3 +lat_2=4 +x_0=5 +y_0=6 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn wkt_parse_vertcs_wkt1_esri_context() {
    let wkt = r#"VERTCS["EGM2008_Geoid",VDATUM["EGM2008_Geoid"],PARAMETER["Vertical_Shift",0.0],PARAMETER["Direction",1.0],UNIT["Meter",1.0]]"#;
    let crs = parse_db(wkt).vertical().expect("crs").clone();
    assert_eq!(crs.name(), "EGM2008 height");
    assert_eq!(crs.datum().expect("datum").name(), "EGM2008 geoid");
    assert_eq!(crs.coordinate_system().axis().direction(), "up");
    assert_eq!(WktParser::guess_dialect(wkt), WktDialect::Wkt1Esri);
}

#[test]
fn wkt_parse_vertcrs_wkt1_las_ftus() {
    let wkt = r#"VERT_CS["NAVD88 - Geoid03 (Feet)",    VERT_DATUM["unknown",2005],    UNIT["US survey foot",0.3048006096012192,        AUTHORITY["EPSG","9003"]],    AXIS["Up",UP]]"#;
    let crs = parse(wkt).vertical().expect("crs").clone();
    assert_eq!(crs.name(), "NAVD88 height (ftUS)");
    assert_eq!(crs.identifiers().len(), 1);
    assert_eq!(crs.identifiers()[0].code(), "6360");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    let geoid_models = crs.geoid_models();
    assert!(!geoid_models.is_empty());
    assert_eq!(geoid_models[0].name(), "GEOID03");
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.name(), "North American Vertical Datum 1988");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "5103");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    let axis = crs.coordinate_system().axis();
    assert_eq!(axis.direction(), "up");
    assert_eq!(axis.unit().name(), "US survey foot");
    assert!((axis.unit().conversion_to_si() - 0.3048006096012192).abs() < 1e-16);
}

#[test]
fn wkt_parse_vertcrs_wkt1_las_metre() {
    let wkt = r#"VERT_CS["NAVD88 via Geoid09",    VERT_DATUM["unknown",2005],    UNIT["metre",1.0,        AUTHORITY["EPSG","9001"]],    AXIS["Up",UP]]"#;
    let crs = parse(wkt).vertical().expect("crs").clone();
    assert_eq!(crs.name(), "NAVD88 height");
    assert_eq!(crs.identifiers().len(), 1);
    assert_eq!(crs.identifiers()[0].code(), "5703");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    let geoid_models = crs.geoid_models();
    assert!(!geoid_models.is_empty());
    assert_eq!(geoid_models[0].name(), "GEOID09");
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.name(), "North American Vertical Datum 1988");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "5103");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    let axis = crs.coordinate_system().axis();
    assert_eq!(axis.direction(), "up");
    assert_eq!(axis.unit().name(), UnitOfMeasure::metre().name());
}

#[test]
fn wkt_parse_compoundcrs_wkt1_las_only_geoid_name() {
    // Yes, this WKT is quite odd...
    let wkt = r#"COMPD_CS["NAD83 / NAD83 / South Dakota South / Geoid 2012A (ftUS)",PROJCS["NAD83 / NAD83 / South Dakota South / Geoid 2012A (ftUS)",GEOGCS["NAD83 / NAD83 / South Dakota South / Geoid 2012A (ftUS)",DATUM["NAD83",SPHEROID["GRS80",6378137.000,298.257222100,AUTHORITY["EPSG","0"]],AUTHORITY["EPSG","0"]],PRIMEM["Greenwich",0.0000000000000000,AUTHORITY["EPSG","8901"]],UNIT["US survey foot",0.30480060960121918567,AUTHORITY["EPSG","9003"]],AUTHORITY["EPSG","0"]],PROJECTION["Lambert_Conformal_Conic_2SP",AUTHORITY["EPSG","9802"]],PARAMETER["standard_parallel_1",44.4000000000000057],PARAMETER["standard_parallel_2",42.8333333333333357],PARAMETER["latitude_of_origin",42.3333333333333499],PARAMETER["central_meridian",-100.3333333333333428],PARAMETER["false_easting",1968500.000],PARAMETER["false_northing",0.000],UNIT["US survey foot",0.30480060960121918567,AUTHORITY["EPSG","9003"]],AXIS["Easting",EAST],AXIS["Northing",NORTH],AUTHORITY["EPSG","0"]],VERT_CS["Geoid 2012A",VERT_DATUM["Geoid 2012A",2005],UNIT["US survey foot",0.30480060960121918567,AUTHORITY["EPSG","9003"]],AXIS["Gravity-related height",UP],AUTHORITY["EPSG","0"]],AUTHORITY["EPSG","0"]]"#;
    let compound = parse(wkt).compound().expect("crs").clone();
    let Crs::Vertical(vcrs) = &compound.components()[1] else {
        panic!("component 1 is not vertical");
    };
    assert_eq!(vcrs.name(), "NAVD88 height (ftUS)");
    assert_eq!(vcrs.identifiers().len(), 1);
    assert_eq!(vcrs.identifiers()[0].code_space(), "EPSG");
    let geoid_models = vcrs.geoid_models();
    assert!(!geoid_models.is_empty());
    assert_eq!(geoid_models[0].name(), "GEOID12A");
    let datum = vcrs.datum().expect("datum");
    assert_eq!(datum.name(), "North American Vertical Datum 1988");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "5103");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    let axis = vcrs.coordinate_system().axis();
    assert_eq!(axis.direction(), "up");
    assert_eq!(axis.unit().name(), "US survey foot");
    assert!((axis.unit().conversion_to_si() - 0.3048006096012192).abs() < 1e-16);
}

#[test]
fn wkt_parse_compd_cs_non_conformant_horizontal_plus_horizontal_as_in_las() {
    let obj = parse(
        r#"COMPD_CS["horizontal + vertical",
    PROJCS["WGS 84 / UTM zone 31N",
        GEOGCS["WGS 84",
            DATUM["World Geodetic System 1984",
                SPHEROID["WGS 84",6378137,298.257223563,
                    AUTHORITY["EPSG","7030"]],
                AUTHORITY["EPSG","6326"]],
            PRIMEM["Greenwich",0,
                AUTHORITY["EPSG","8901"]],
            UNIT["degree",0.0174532925199433,
                AUTHORITY["EPSG","9122"]],
            AXIS["Latitude",NORTH],
            AXIS["Longitude",EAST],
            AUTHORITY["EPSG","4326"]],
        PROJECTION["Transverse_Mercator"],
        PARAMETER["latitude_of_origin",0],
        PARAMETER["central_meridian",3],
        PARAMETER["scale_factor",0.9996],
        PARAMETER["false_easting",500000],
        PARAMETER["false_northing",0],
        UNIT["metre",1,
            AUTHORITY["EPSG","9001"]],
        AXIS["Easting",EAST],
        AXIS["Northing",NORTH],
        AUTHORITY["EPSG","32631"]],
    GEOGCS["WGS 84",
        DATUM["World Geodetic System 1984",
            SPHEROID["WGS 84",6378137,298.257223563,
                AUTHORITY["EPSG","7030"]],
            AUTHORITY["EPSG","6326"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AXIS["Latitude",NORTH],
        AXIS["Longitude",EAST],
        AUTHORITY["EPSG","4326"]]]"#,
    );
    let crs = obj.projected().expect("crs").clone();
    assert_eq!(crs.name(), "WGS 84 / UTM zone 31N");
    assert_eq!(crs.coordinate_system().axes().len(), 3);
}

#[test]
fn wkt_parse_compd_cs_non_conformant_horizontal_towgs84_plus_horizontal_as_in_las() {
    let wkt = r#"COMPD_CS["WGS 84 + WGS 84",
    GEOGCS["WGS 84",
        DATUM["WGS_1984",
            SPHEROID["WGS 84",6378137,298.257223563,
                AUTHORITY["EPSG","7030"]],
            TOWGS84[0,0,0,0,0,0,0],
            AUTHORITY["EPSG","6326"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4326"]],
    GEOGCS["WGS 84",
        DATUM["WGS_1984",
            SPHEROID["WGS 84",6378137,298.257223563,
                AUTHORITY["EPSG","7030"]],
            AUTHORITY["EPSG","6326"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4326"]]]"#;
    let dbcontext = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(dbcontext.clone());
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let crs = obj.bound().expect("crs").clone();
    let base = bound_base_geographic(&crs);
    assert_eq!(base.name(), "WGS 84");
    assert_eq!(base.coordinate_system().axes().len(), 3);
    assert_eq!(
        export_db(&Crs::Bound(crs), WktConvention::Wkt1Gdal, &dbcontext).expect("WKT1"),
        wkt
    );
}

#[test]
fn wkt_parse_compd_cs_horizontal_bound_geog_plus_vertical_ellipsoidal_height() {
    // See https://github.com/OSGeo/PROJ/issues/2228
    let wkt = r#"COMPD_CS["NAD83 + Ellipsoid (Meters)",
    GEOGCS["NAD83",
        DATUM["North_American_Datum_1983",
            SPHEROID["GRS 1980",6378137,298.257222101,
                AUTHORITY["EPSG","7019"]],
            TOWGS84[0,0,0,0,0,0,0],
            AUTHORITY["EPSG","6269"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4269"]],
    VERT_CS["Ellipsoid (Meters)",
        VERT_DATUM["Ellipsoid",2002],
        UNIT["metre",1,
            AUTHORITY["EPSG","9001"]],
        AXIS["Up",UP]]]"#;
    let dbcontext = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(dbcontext.clone());
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let crs = obj.bound().expect("crs").clone();
    let base = bound_base_geographic(&crs);
    assert_eq!(base.name(), "NAD83");
    assert_eq!(base.coordinate_system().axes().len(), 3);
    assert_eq!(
        export_db(&Crs::Bound(crs), WktConvention::Wkt1Gdal, &dbcontext)
            .expect("WKT1")
            .replace("ellipsoidal height", "Up"),
        wkt
    );
}

#[test]
fn wkt_parse_compd_cs_horizontal_projected_plus_vertical_ellipsoidal_height() {
    // Variant of above
    let wkt = r#"COMPD_CS["WGS 84 / UTM zone 31N + Ellipsoid (Meters)",
    PROJCS["WGS 84 / UTM zone 31N",
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
        AXIS["Easting",EAST],
        AXIS["Northing",NORTH],
        AUTHORITY["EPSG","32631"]],
    VERT_CS["Ellipsoid (Meters)",
        VERT_DATUM["Ellipsoid",2002],
        UNIT["metre",1,
            AUTHORITY["EPSG","9001"]],
        AXIS["Up",UP]]]"#;
    let dbcontext = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(dbcontext.clone());
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let crs = obj.projected().expect("crs").clone();
    assert_eq!(crs.name(), "WGS 84 / UTM zone 31N");
    assert_eq!(crs.coordinate_system().axes().len(), 3);
    assert_eq!(
        export_db(&Crs::Projected(crs), WktConvention::Wkt1Gdal, &dbcontext)
            .expect("WKT1")
            .replace("ellipsoidal height", "Up"),
        wkt
    );
}

#[test]
fn wkt_parse_compd_cs_horizontal_geog_plus_vertical_ellipsoidal_height_non_metre() {
    // See https://github.com/OSGeo/PROJ/issues/2232
    let wkt = r#"COMPD_CS["NAD83 + Ellipsoid (US Feet)",
    GEOGCS["NAD83",
        DATUM["North_American_Datum_1983",
            SPHEROID["GRS 1980",6378137,298.257222101,
                AUTHORITY["EPSG","7019"]],
            AUTHORITY["EPSG","6269"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4269"]],
    VERT_CS["Ellipsoid (US Feet)",
        VERT_DATUM["Ellipsoid",2002],
        UNIT["US survey foot",0.304800609601219,
            AUTHORITY["EPSG","9003"]],
        AXIS["Up",UP]]]"#;
    let dbcontext = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(dbcontext.clone());
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let crs = obj.geographic().expect("crs").clone();
    assert_eq!(crs.name(), "NAD83 (Ellipsoid (US Feet))");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert!((axes[2].unit().conversion_to_si() - 0.304_800_609_601_219).abs() < 1e-15);
    assert_eq!(
        export_db(&Crs::Geographic(crs), WktConvention::Wkt1Gdal, &dbcontext)
            .expect("WKT1")
            .replace("ellipsoidal height", "Up"),
        wkt
    );
}

#[test]
fn wkt_parse_implicit_compound_crs_geographic_with_ellipsoidal_height_esri() {
    let wkt = r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],VERTCS["WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PARAMETER["Vertical_Shift",0.0],PARAMETER["Direction",1.0],UNIT["Meter",1.0]]"#;
    let dbcontext = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(dbcontext.clone());
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let crs = obj.geographic().expect("crs").clone();
    assert_eq!(crs.coordinate_system().axes().len(), 3);
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Esri).with_database(dbcontext);
    formatter.set_linunit(LinunitPolicy::Reject);
    assert_eq!(
        Crs::Geographic(crs)
            .export_to_wkt(&mut formatter)
            .expect("WKT1"),
        wkt
    );
}

#[test]
fn wkt_parse_implicit_compound_crs_projected_with_ellipsoidal_height_esri() {
    let wkt = r#"PROJCS["WGS_1984_UTM_Zone_31N",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",3.0],PARAMETER["Scale_Factor",0.9996],PARAMETER["Latitude_Of_Origin",0.0],UNIT["Meter",1.0]],VERTCS["WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PARAMETER["Vertical_Shift",0.0],PARAMETER["Direction",1.0],UNIT["Meter",1.0]]"#;
    let dbcontext = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(dbcontext.clone());
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let crs = obj.projected().expect("crs").clone();
    assert_eq!(crs.coordinate_system().axes().len(), 3);
    assert_eq!(
        Crs::Projected(crs)
            .export_to_wkt(
                &mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(dbcontext)
            )
            .expect("WKT1"),
        wkt
    );
}

#[test]
fn wkt_parse_boundcrs_transformation_from_names() {
    let wkt = r#"BOUNDCRS[SOURCECRS[PROJCRS["my PROJCRS",BASEGEOGCRS["my GEOGCRS",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]]],CONVERSION["UTM zone 31S",METHOD["Transverse Mercator"],PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Longitude of natural origin",3,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Scale factor at natural origin",0.9996,SCALEUNIT["unity",1]],PARAMETER["False easting",500000,LENGTHUNIT["metre",1]],PARAMETER["False northing",10000000,LENGTHUNIT["metre",1]]],CS[Cartesian,2],AXIS["easting (E)",east,ORDER[1],LENGTHUNIT["metre",1]],AXIS["northing (N)",north,ORDER[2],LENGTHUNIT["metre",1]]]],TARGETCRS[GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",4326]]],ABRIDGEDTRANSFORMATION["Transformation to WGS84",METHOD["Coordinate Frame"],PARAMETER["X-axis translation",1],PARAMETER["Y-axis translation",2],PARAMETER["Z-axis translation",3],PARAMETER["X-axis rotation",-4],PARAMETER["Y-axis rotation",-5],PARAMETER["Z-axis rotation",-6],PARAMETER["Scale difference",1.000007]]]"#;
    let crs = parse(wkt).bound().expect("crs").clone();
    assert_eq!(crs.source_crs().name(), "my PROJCRS");
    assert_eq!(crs.target_crs().name(), "WGS 84");
    // The seven TOWGS84 values, through getTOWGS84Parameters(true), are the
    // open part: Transformation exposes no such accessor.
}

#[test]
fn wkt_parse_boundcrs_transformation_from_codes() {
    let wkt = r#"BOUNDCRS[SOURCECRS[PROJCRS["my PROJCRS",BASEGEOGCRS["my GEOGCRS",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]]],CONVERSION["UTM zone 31S",METHOD["Transverse Mercator"],PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Longitude of natural origin",3,ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Scale factor at natural origin",0.9996,SCALEUNIT["unity",1]],PARAMETER["False easting",500000,LENGTHUNIT["metre",1]],PARAMETER["False northing",10000000,LENGTHUNIT["metre",1]]],CS[Cartesian,2],AXIS["easting (E)",east,ORDER[1],LENGTHUNIT["metre",1]],AXIS["northing (N)",north,ORDER[2],LENGTHUNIT["metre",1]]]],TARGETCRS[GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["geodetic longitude (Lon)",east,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",4326]]],ABRIDGEDTRANSFORMATION["Transformation to WGS84",METHOD["bla",ID["EPSG",1032]],PARAMETER["tx",1,ID["EPSG",8605]],PARAMETER["ty",2,ID["EPSG",8606]],PARAMETER["tz",3,ID["EPSG",8607]],PARAMETER["rotx",-4,ID["EPSG",8608]],PARAMETER["roty",-5,ID["EPSG",8609]],PARAMETER["rotz",-6,ID["EPSG",8610]],PARAMETER["scale",1.000007,ID["EPSG",8611]]]]"#;
    let crs = parse(wkt).bound().expect("crs").clone();
    assert_eq!(crs.source_crs().name(), "my PROJCRS");
    assert_eq!(crs.target_crs().name(), "WGS 84");
    // The seven TOWGS84 values, through getTOWGS84Parameters(true), are the
    // open part: Transformation exposes no such accessor.
}

#[test]
fn wkt_parse_boundcrs_with_interpolation_as_parameter() {
    let wkt = r#"BOUNDCRS[
    SOURCECRS[
        VERTCRS["unknown",
            VDATUM["unknown using geoidgrids=@foo.gtx"],
            CS[vertical,1],
                AXIS["gravity-related height (H)",up,
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]]]],
    TARGETCRS[
        GEOGCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            CS[ellipsoidal,3],
                AXIS["latitude",north,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["longitude",east,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["ellipsoidal height",up,
                    ORDER[3],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",4979]]],
    ABRIDGEDTRANSFORMATION["unknown to WGS84 ellipsoidal height",
        METHOD["GravityRelatedHeight to Geographic3D"],
        PARAMETERFILE["Geoid (height correction) model file","@foo.gtx",
            ID["EPSG",8666]],
        PARAMETER["EPSG code for Interpolation CRS",7886]]]"#;
    {
        let bound = parse(wkt).bound().expect("crs").clone();
        let transformation = transformation_of(&bound);
        assert!(transformation.interpolation_crs().is_none());
        assert_eq!(transformation.parameters().len(), 2);
    }
    {
        // Need a database so that the interpolation CRS EPSG:7886 is resolved
        let bound = parse_db(wkt).bound().expect("crs").clone();
        let transformation = transformation_of(&bound);
        assert!(transformation.interpolation_crs().is_some());
        assert_eq!(transformation.parameters().len(), 1);
        let exported = export(&Crs::Bound(bound), WktConvention::Wkt2_2019).expect("WKT2");
        assert!(
            exported.contains("PARAMETER[\"EPSG code for Interpolation CRS\",7886,"),
            "{exported}"
        );
    }
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
fn wkt_parse_wkt1_geographic_epsg_org_api_4326() {
    let wkt = r#"GEOGCS["WGS 84",DATUM["World Geodetic System 1984 ensemble",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree (supplier to define representation)",0.0174532925199433,AUTHORITY["EPSG","9122"]],AXIS["Lat",north],AXIS["Lon",east],AUTHORITY["EPSG","4326"]]"#;
    let object = parse(wkt);
    let crs = object.geographic().expect("crs");
    assert_eq!(
        crs.datum().expect("datum").name(),
        "World Geodetic System 1984"
    );
}

#[test]
fn wkt_parse_wkt1_geographic_epsg_org_api_4258() {
    let wkt = r#"GEOGCS["ETRS89",DATUM["European Terrestrial Reference System 1989 ensemble",SPHEROID["GRS 1980",6378137,298.257222101,AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","6258"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree (supplier to define representation)",0.0174532925199433,AUTHORITY["EPSG","9122"]],AXIS["Lat",north],AXIS["Lon",east],AUTHORITY["EPSG","4258"]]"#;
    let object = parse(wkt);
    let crs = object.geographic().expect("crs");
    assert_eq!(
        crs.datum().expect("datum").name(),
        "European Terrestrial Reference System 1989"
    );
}

#[test]
fn crs_projected_angular_unit_from_primem() {
    let wkt = r#"PROJCRS["NTF (Paris) / Lambert Nord France",
  BASEGEODCRS["NTF (Paris)",
    DATUM["Nouvelle Triangulation Francaise (Paris)",
      ELLIPSOID["Clarke 1880 (IGN)",6378249.2,293.4660213,LENGTHUNIT["metre",1.0]]],
    PRIMEM["Paris",2.5969213,ANGLEUNIT["grad",0.015707963268]]],
  CONVERSION["Lambert Nord France",
    METHOD["Lambert Conic Conformal (1SP)",ID["EPSG",9801]],
    PARAMETER["Latitude of natural origin",55,ANGLEUNIT["grad",0.015707963268]],
    PARAMETER["Longitude of natural origin",0,ANGLEUNIT["grad",0.015707963268]],
    PARAMETER["Scale factor at natural origin",0.999877341,SCALEUNIT["unity",1.0]],
    PARAMETER["False easting",600000,LENGTHUNIT["metre",1.0]],
    PARAMETER["False northing",200000,LENGTHUNIT["metre",1.0]]],
  CS[cartesian,2],
    AXIS["easting (X)",east,ORDER[1]],
    AXIS["northing (Y)",north,ORDER[2]],
    LENGTHUNIT["metre",1.0],
  ID["EPSG",27561]]"#;
    let object = parse(wkt);
    let crs = object.projected().expect("projected");
    let GeodeticCRS::Geographic(base) = crs.base_crs() else {
        panic!("base is not geographic");
    };
    let unit = base.coordinate_system().axes()[0].unit();
    assert_eq!(unit.name(), UnitOfMeasure::grad().name());
}

#[test]
fn wkt_parse_vdatum_with_anchor() {
    let object = parse(
        r#"VDATUM["Ordnance Datum Newlyn",
    ANCHOR["my anchor"],
    ID["EPSG",5101]]"#,
    );
    let datum = object.vertical_datum().expect("datum");
    assert_eq!(datum.anchor(), Some("my anchor"));
    assert_eq!(datum.anchor_epoch(), None);
}

#[test]
fn wkt_parse_vdatum_with_anchorepoch() {
    let object = parse(
        r#"VDATUM["my_datum",
    ANCHOREPOCH[2002.5]]"#,
    );
    let datum = object.vertical_datum().expect("datum");
    assert_eq!(datum.anchor_epoch(), Some(2002.5));
    assert_eq!(datum.anchor(), None);
}

#[test]
fn wkt_parse_dynamic_vertical_reference_frame() {
    let object = parse(
        r#"VERTCRS["RH2000",  DYNAMIC[FRAMEEPOCH[2000.0],MODEL["NKG2016LU"]],  VDATUM["Rikets Hojdsystem 2000",ANCHOR["my anchor"]],  CS[vertical,1],    AXIS["gravity-related height (H)",up],    LENGTHUNIT["metre",1.0]]"#,
    );
    let datum = object.vertical().expect("vertical").datum().expect("datum");
    assert_eq!(datum.anchor(), Some("my anchor"));
    assert_eq!(datum.frame_reference_epoch(), Some(2000.0));
    assert_eq!(datum.deformation_model(), Some("NKG2016LU"));
}

#[test]
fn wkt_parse_conversion_proj_based() {
    let wkt = r#"CONVERSION["PROJ-based coordinate operation",
    METHOD["PROJ-based operation method: +proj=merc"]]"#;
    let object = parse(wkt);
    let conversion = object.conversion().expect("conversion");
    assert_eq!(
        conversion.export_to_proj_string().expect("PROJ"),
        "+proj=merc"
    );
}

#[test]
fn wkt_parse_conversion_utm_zone_south_wrong_id() {
    let wkt = r#"CONVERSION["UTM zone 55S",    METHOD["Transverse Mercator",        ID["EPSG",9807]],    PARAMETER["Latitude of natural origin",0,        ANGLEUNIT["Degree",0.0174532925199433],        ID["EPSG",8801]],    PARAMETER["Longitude of natural origin",147,        ANGLEUNIT["Degree",0.0174532925199433],        ID["EPSG",8802]],    PARAMETER["Scale factor at natural origin",0.9996,        SCALEUNIT["unity",1],        ID["EPSG",8805]],    PARAMETER["False easting",500000,        LENGTHUNIT["metre",1],        ID["EPSG",8806]],    PARAMETER["False northing",10000000,        LENGTHUNIT["metre",1],        ID["EPSG",8807]],    ID["EPSG",17055]]"#;
    let object = parse(wkt);
    let conversion = object.conversion().expect("conversion");
    let ids = conversion.identifiers();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].code_space(), "EPSG");
    assert_eq!(ids[0].code(), "16155");
}

#[test]
fn wkt_parse_edatum() {
    let object = parse(
        r#"EDATUM["Engineering datum",
    ANCHOR["my anchor"]]"#,
    );
    let proj_rs::WktObject::EngineeringDatum(datum) = object else {
        panic!("not an engineering datum");
    };
    assert_eq!(datum.name(), "Engineering datum");
    assert_eq!(datum.anchor(), Some("my anchor"));
}

#[test]
fn wkt_parse_engcrs() {
    let wkt = r#"ENGCRS["Engineering CRS",
    EDATUM["Engineering datum"],
    CS[Cartesian,2],
        AXIS["(E)",east,
            ORDER[1],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["(N)",north,
            ORDER[2],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#;
    let object = parse(wkt);
    let crs = object.engineering().expect("engineering");
    assert_eq!(crs.name(), "Engineering CRS");
    assert_eq!(crs.datum().name(), "Engineering datum");
    assert_eq!(crs.coordinate_system().axes().len(), 2);
}

#[test]
fn wkt_parse_engcrs_unknown_unit() {
    let wkt = r#"ENGCRS["Undefined Cartesian SRS with unknown unit",
    EDATUM["Unknown engineering datum"],
    CS[Cartesian,2],
        AXIS["X",unspecified,
            ORDER[1],
            LENGTHUNIT["unknown",0]],
        AXIS["Y",unspecified,
            ORDER[2],
            LENGTHUNIT["unknown",0]]]"#;
    let object = parse(wkt);
    let crs = object.engineering().expect("engineering");
    assert_eq!(crs.name(), "Undefined Cartesian SRS with unknown unit");
    assert_eq!(crs.datum().name(), "Unknown engineering datum");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    for (axis, name) in axes.iter().zip(["X", "Y"]) {
        assert_eq!(axis.name(), name);
        assert_eq!(axis.direction(), "unspecified");
        assert_eq!(axis.unit().name(), "unknown");
        assert_eq!(axis.unit().conversion_to_si(), 0.0);
    }
}

#[test]
fn wkt_parse_pdatum() {
    let object = parse(
        r#"PDATUM["Parametric datum",
    ANCHOR["my anchor"]]"#,
    );
    let proj_rs::WktObject::ParametricDatum(datum) = object else {
        panic!("not a parametric datum");
    };
    assert_eq!(datum.name(), "Parametric datum");
    assert_eq!(datum.anchor(), Some("my anchor"));
}

#[test]
fn wkt_parse_ensemble() {
    let wkt = r#"ENSEMBLE["test",
    MEMBER["World Geodetic System 1984",
        ID["EPSG",6326]],
    MEMBER["other datum"],
    ELLIPSOID["WGS 84",6378137,298.257223563,
        LENGTHUNIT["metre",1],
        ID["EPSG",7030]],
    ENSEMBLEACCURACY[100]]"#;
    let object = parse(wkt);
    let ensemble = object.ensemble().expect("ensemble");
    assert_eq!(ensemble.members().len(), 2);
    let proj_rs::Datum::Geodetic(first) = &ensemble.members()[0] else {
        panic!("first member is not a geodetic datum");
    };
    assert_eq!(first.name(), "World Geodetic System 1984");
    assert_eq!(first.identifiers().len(), 1);
    assert_eq!(first.identifiers()[0].code(), "6326");
    assert_eq!(first.identifiers()[0].code_space(), "EPSG");
    assert_eq!(first.ellipsoid().name(), "WGS 84");
    assert_eq!(ensemble.positional_accuracy(), 100.0);
}

#[test]
fn wkt_parse_ensemble_vdatum() {
    let wkt = r#"ENSEMBLE["unnamed",
    MEMBER["vdatum1"],
    MEMBER["vdatum2"],
    ENSEMBLEACCURACY[100]]"#;
    let object = parse(wkt);
    let ensemble = object.ensemble().expect("ensemble");
    assert_eq!(ensemble.members().len(), 2);
    let proj_rs::Datum::Vertical(first) = &ensemble.members()[0] else {
        panic!("first member is not a vertical datum");
    };
    assert_eq!(first.name(), "vdatum1");
    assert_eq!(ensemble.positional_accuracy(), 100.0);
}

#[test]
#[ignore = "open: buildCS has no temporal coordinate system, CS[temporal,1] in an ENGCRS is refused (wkt_import.rs build_cs)"]
fn wkt_parse_invalid_engineeingcrs() {
    assert!(WktParser::new()
        .create_from_wkt(
            r#"ENGCRS["name",
    EDATUM["name"],
    CS[temporal,1],
        AXIS["time (T)",future]]"#
        )
        .is_ok());
    // Missing EDATUM
    refused(
        r#"ENGCRS["name",
    CS[temporal,1],
        AXIS["time (T)",future]]"#,
    );
    // Missing CS
    refused(
        r#"ENGCRS["name",
    EDATUM["name"]]"#,
    );
}

const ITRF2014_GEOGCRS: &str = r#"GEOGCRS["ITRF2014",
        DYNAMIC[
            FRAMEEPOCH[2010]],
        DATUM["International Terrestrial Reference Frame 2014",
            ELLIPSOID["GRS 1980",6378137,298.257222101,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        CS[ellipsoidal,2],
            AXIS["geodetic latitude (Lat)",north,
                ORDER[1],
                ANGLEUNIT["degree",0.0174532925199433]],
            AXIS["geodetic longitude (Lon)",east,
                ORDER[2],
                ANGLEUNIT["degree",0.0174532925199433]],
        USAGE[
            SCOPE["Geodesy."],
            AREA["World."],
            BBOX[-90,-180,90,180]],
        ID["EPSG",9000]]"#;

#[test]
fn wkt_parse_invalid_coordinatemetadata() {
    refused("COORDINATEMETADATA[]");
    refused(r#"COORDINATEMETADATA[ELLIPSOID["GRS 1980",6378137,298.257222101]]"#);
    // Empty epoch
    refused(&format!(
        "COORDINATEMETADATA[\n    {ITRF2014_GEOGCRS},\n    EPOCH[]]"
    ));
    // Invalid epoch
    refused(&format!(
        "COORDINATEMETADATA[\n    {ITRF2014_GEOGCRS},\n    EPOCH[invalid]]"
    ));
}

#[test]
fn wkt_parse_esri_geogcs_datum_spheroid_name_from_db_substitution() {
    let wkt = r#"GEOGCS["GCS_WGS_1966",DATUM["D_WGS_1966",SPHEROID["WGS_1966",6378145.0,298.25]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#;
    let object = parse_db(wkt);
    let crs = object.geographic().expect("geographic");
    assert_eq!(crs.name(), "WGS 66");
    assert_eq!(
        crs.datum().expect("datum").name(),
        "World Geodetic System 1966"
    );
    assert_eq!(crs.ellipsoid().expect("ellipsoid").name(), "WGS_1966");
}

#[test]
fn wkt_parse_esri_datum_name_with_prime_meridian() {
    let wkt = r#"GEOGCS["GCS_NTF_Paris",DATUM["D_NTF",SPHEROID["Clarke_1880_IGN",6378249.2,293.4660212936265]],PRIMEM["Paris",2.337229166666667],UNIT["Grad",0.01570796326794897]]"#;
    let object = parse_db(wkt);
    let crs = object.geographic().expect("geographic");
    assert_eq!(crs.name(), "NTF (Paris)");
    assert_eq!(
        crs.datum().expect("datum").name(),
        "Nouvelle Triangulation Francaise (Paris)"
    );
    assert_eq!(
        crs.ellipsoid().expect("ellipsoid").name(),
        "Clarke 1880 (IGN)"
    );
}

const ESRI_UTM_31N: &str = r#"PROJCS["WGS_1984_UTM_Zone_31N",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["transverse_mercator"],PARAMETER["false_easting",500000.0],PARAMETER["false_northing",0.0],PARAMETER["central_meridian",3.0],PARAMETER["scale_factor",0.9996],PARAMETER["latitude_of_origin",0.0],UNIT["Meter",1.0]]"#;

#[test]
fn wkt_parse_wkt1_esri_case_insensitive_names() {
    let object = parse(ESRI_UTM_31N);
    let crs = object.projected().expect("projected");
    assert_eq!(
        crs.deriving_conversion().is_utm(),
        Some((31, proj_rs::Hemisphere::North))
    );
}

#[test]
fn wkt_parse_wkt1_esri_non_expected_param_name() {
    // We try to be lax on parameter names.
    let wkt = ESRI_UTM_31N.replace("central_meridian", "longitude_of_center");
    let object = parse(&wkt);
    let crs = object.projected().expect("projected");
    assert_eq!(
        crs.deriving_conversion().is_utm(),
        Some((31, proj_rs::Hemisphere::North))
    );
}

#[test]
fn wkt_parse_wkt1_esri_normalize_unit() {
    let wkt = r#"PROJCS["Accra_Ghana_Grid",GEOGCS["GCS_Accra",DATUM["D_Accra",SPHEROID["War_Office",6378300.0,296.0]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",900000.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",-1.0],PARAMETER["Scale_Factor",0.99975],PARAMETER["Latitude_Of_Origin",4.666666666666667],UNIT["Foot_Gold_Coast",0.3047997101815088]]"#;
    let object = parse_db(wkt);
    let crs = object.projected().expect("projected");
    assert_eq!(
        crs.coordinate_system().axes()[0].unit().name(),
        "Gold Coast foot"
    );
}

fn assert_towgs84(transformation: &proj_rs::Transformation, expected: [f64; 7]) {
    let params = transformation
        .towgs84_parameters()
        .expect("helmert")
        .expect("seven terms");
    for (got, want) in params.iter().zip(expected) {
        assert!((got - want).abs() < 1e-10, "{got} != {want}");
    }
}

#[test]
fn wkt_parse_wkt1_krovak_south_west() {
    let wkt = r#"PROJCS["S-JTSK / Krovak",GEOGCS["S-JTSK",DATUM["System_Jednotne_Trigonometricke_Site_Katastralni",SPHEROID["Bessel 1841",6377397.155,299.1528128,AUTHORITY["EPSG","7004"]],AUTHORITY["EPSG","6156"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4156"]],PROJECTION["Krovak"],PARAMETER["latitude_of_center",49.5],PARAMETER["longitude_of_center",24.83333333333333],PARAMETER["azimuth",30.2881397527778],PARAMETER["pseudo_standard_parallel_1",78.5],PARAMETER["scale_factor",0.9999],PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["X",SOUTH],AXIS["Y",WEST],AUTHORITY["EPSG","5513"]]"#;
    let crs = parse(wkt).projected().expect("crs").clone();
    assert_eq!(crs.deriving_conversion().method().name(), "Krovak");
    let expected_wkt2 = r#"PROJCRS["S-JTSK / Krovak",
    BASEGEODCRS["S-JTSK",
        DATUM["System_Jednotne_Trigonometricke_Site_Katastralni",
            ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]]],
    CONVERSION["unknown",
        METHOD["Krovak",
            ID["EPSG",9819]],
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
        AXIS["x",south,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["y",west,
            ORDER[2],
            LENGTHUNIT["metre",1]],
    ID["EPSG",5513]]"#;
    assert_eq!(
        export(&Crs::Projected(crs.clone()), WktConvention::Wkt2_2015).expect("WKT2"),
        expected_wkt2
    );
    let expected_proj = "+proj=krovak +axis=swu +lat_0=49.5 +lon_0=24.8333333333333 \
        +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +units=m \
        +no_defs +type=crs";
    let proj = Crs::Projected(crs).export_to_proj_string().expect("PROJ");
    assert_eq!(proj, expected_proj);

    let Crs::Projected(crs2) = create_crs_from_proj_string(&proj).expect("import") else {
        panic!("not projected");
    };
    let wkt2 = export(&Crs::Projected(crs2.clone()), WktConvention::Wkt2_2015).expect("WKT2");
    assert!(wkt2.contains("METHOD[\"Krovak\""), "{wkt2}");
    assert!(
        wkt2.contains("PARAMETER[\"Latitude of pseudo standard parallel\",78.5,"),
        "{wkt2}"
    );
    assert!(
        wkt2.contains("PARAMETER[\"Co-latitude of cone axis\",30.2881397527778,"),
        "{wkt2}"
    );
    assert_eq!(
        Crs::Projected(crs2).export_to_proj_string().expect("PROJ"),
        expected_proj
    );

    let Crs::Projected(crs3) = create_crs_from_proj_string(
        "+type=crs +proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 +alpha=30.2881397527778 \
         +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +step +proj=axisswap +order=-2,-1",
    )
    .expect("import") else {
        panic!("not projected");
    };
    let wkt3 = export(&Crs::Projected(crs3), WktConvention::Wkt2_2015).expect("WKT2");
    assert!(wkt3.contains("METHOD[\"Krovak\""), "{wkt3}");
}

#[test]
fn proj_export_wkt2_hotine_oblique_mercator_without_rectified_grid_angle() {
    let wkt = "PROJCRS[\"NAD_1983_Michigan_GeoRef_Meters\",\n\
        BASEGEOGCRS[\"NAD83(1986)\",\n\
            DATUM[\"North American Datum 1983\",\n\
                ELLIPSOID[\"GRS_1980\",6378137,298.257222101,\n\
                    LENGTHUNIT[\"metre\",1]],\n\
                ID[\"EPSG\",6269]],\n\
            PRIMEM[\"Greenwich\",0,\n\
                ANGLEUNIT[\"Degree\",0.0174532925199433]]],\n\
        CONVERSION[\"unknown\",\n\
            METHOD[\"Hotine Oblique Mercator (variant A)\",\n\
                ID[\"EPSG\",9812]],\n\
            PARAMETER[\"False easting\",2546731.496,\n\
                LENGTHUNIT[\"Meter\",1],\n\
                ID[\"EPSG\",8806]],\n\
            PARAMETER[\"False northing\",-4354009.816,\n\
                LENGTHUNIT[\"Meter\",1],\n\
                ID[\"EPSG\",8807]],\n\
            PARAMETER[\"Latitude of projection centre\",45.3091666666667,\n\
                ANGLEUNIT[\"Degree\",0.0174532925199433],\n\
                ID[\"EPSG\",8811]],\n\
            PARAMETER[\"Longitude of projection centre\",-86,\n\
                ANGLEUNIT[\"Degree\",0.0174532925199433],\n\
                ID[\"EPSG\",8812]],\n\
            PARAMETER[\"Azimuth of initial line\",-22.74444,\n\
                ANGLEUNIT[\"Degree\",0.0174532925199433],\n\
                ID[\"EPSG\",8813]],\n\
            PARAMETER[\"Scale factor on initial line\",0.9996,\n\
                SCALEUNIT[\"unity\",1],\n\
                ID[\"EPSG\",8815]]],\n\
        CS[Cartesian,2],\n\
            AXIS[\"(E)\",east,\n\
                ORDER[1],\n\
                LENGTHUNIT[\"Meter\",1]],\n\
            AXIS[\"(N)\",north,\n\
                ORDER[2],\n\
                LENGTHUNIT[\"Meter\",1]]]";
    let crs = parse(wkt).projected().expect("crs").clone();
    let expected = "+proj=omerc +no_uoff +lat_0=45.3091666666667 +lonc=-86 +alpha=-22.74444 \
        +k=0.9996 +x_0=2546731.496 +y_0=-4354009.816 +datum=NAD83 +units=m +no_defs +type=crs";
    let proj = Crs::Projected(crs).export_to_proj_string().expect("PROJ");
    assert_eq!(proj, expected);
    assert!(!proj.contains("gamma"));
}

#[test]
fn wkt_parse_geogcs_towgs84_3terms() {
    let wkt = "GEOGCS[\"my GEOGCRS\",\n\
        DATUM[\"WGS_1984\",\n\
            SPHEROID[\"WGS 84\",6378137,298.257223563],\n\
            TOWGS84[1,2,3]],\n\
        PRIMEM[\"Greenwich\",0],\n\
        UNIT[\"degree\",0.0174532925199433]]";
    let object = parse(wkt);
    let crs = object.bound().expect("bound");
    assert_eq!(crs.source_crs().name(), "my GEOGCRS");
    assert_eq!(crs.target_crs().name(), GeographicCRS::epsg_4326().name());
    let transformation = transformation_of(crs);
    let source = transformation.source_crs().expect("source");
    assert_eq!(source.name(), "my GEOGCRS");
    assert_towgs84(transformation, [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn wkt_parse_projcs_towgs84_7terms() {
    let wkt = "PROJCS[\"my PROJCRS\",\n\
        GEOGCS[\"my GEOGCRS\",\n\
            DATUM[\"WGS_1984\",\n\
                SPHEROID[\"WGS 84\",6378137,298.257223563,\n\
                    AUTHORITY[\"EPSG\",\"7030\"]],\n\
                TOWGS84[1,2,3,4,5,6,7],\n\
                AUTHORITY[\"EPSG\",\"6326\"]],\n\
            PRIMEM[\"Greenwich\",0,\n\
                AUTHORITY[\"EPSG\",\"8901\"]],\n\
            UNIT[\"degree\",0.0174532925199433,\n\
                AUTHORITY[\"EPSG\",\"9122\"]],\n\
            AXIS[\"Latitude\",NORTH],\n\
            AXIS[\"Longitude\",EAST]],\n\
        PROJECTION[\"Transverse_Mercator\"],\n\
        PARAMETER[\"latitude_of_origin\",0],\n\
        PARAMETER[\"central_meridian\",3],\n\
        PARAMETER[\"scale_factor\",0.9996],\n\
        PARAMETER[\"false_easting\",500000],\n\
        PARAMETER[\"false_northing\",0],\n\
        UNIT[\"metre\",1,\n\
            AUTHORITY[\"EPSG\",\"9001\"]],\n\
        AXIS[\"Easting\",EAST],\n\
        AXIS[\"Northing\",NORTH]]";
    let object = parse(wkt);
    let crs = object.bound().expect("bound");
    assert_eq!(crs.source_crs().name(), "my PROJCRS");
    assert_eq!(crs.target_crs().name(), GeographicCRS::epsg_4326().name());
    let transformation = transformation_of(crs);
    assert_eq!(
        transformation.source_crs().expect("source").name(),
        "my GEOGCRS"
    );
    assert_towgs84(transformation, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
}

#[test]
fn io_projcs_towgs84_7terms_autocorrect() {
    let wkt = "PROJCS[\"BD72 / Belgian Lambert 72\",\n\
        GEOGCS[\"BD72\",\n\
            DATUM[\"Reseau_National_Belge_1972\",\n\
                SPHEROID[\"International 1924\",6378388,297],\n\
                TOWGS84[-106.8686,52.2978,-103.7239,-0.3366,0.457,-1.8422,-1.2747]],\n\
            PRIMEM[\"Greenwich\",0,\n\
                AUTHORITY[\"EPSG\",\"8901\"]],\n\
            UNIT[\"degree\",0.0174532925199433,\n\
                AUTHORITY[\"EPSG\",\"9122\"]],\n\
            AUTHORITY[\"EPSG\",\"4313\"]],\n\
        PROJECTION[\"Lambert_Conformal_Conic_2SP\"],\n\
        PARAMETER[\"latitude_of_origin\",90],\n\
        PARAMETER[\"central_meridian\",4.36748666666667],\n\
        PARAMETER[\"standard_parallel_1\",51.1666672333333],\n\
        PARAMETER[\"standard_parallel_2\",49.8333339],\n\
        PARAMETER[\"false_easting\",150000.013],\n\
        PARAMETER[\"false_northing\",5400088.438],\n\
        UNIT[\"metre\",1,\n\
            AUTHORITY[\"EPSG\",\"9001\"]],\n\
        AXIS[\"Easting\",EAST],\n\
        AXIS[\"Northing\",NORTH],\n\
        AUTHORITY[\"EPSG\",\"31370\"]]";
    let object = parse_db(wkt);
    let crs = object.bound().expect("bound");
    assert_towgs84(
        transformation_of(crs),
        [
            -106.8686, 52.2978, -103.7239, 0.3366, -0.457, 1.8422, -1.2747,
        ],
    );
}

#[test]
fn wkt_parse_wkt1_vert_datum_extension() {
    let wkt = "VERT_CS[\"EGM2008 geoid height\",\n\
        VERT_DATUM[\"EGM2008 geoid\",2005,\n\
            EXTENSION[\"PROJ4_GRIDS\",\"egm08_25.gtx\"],\n\
            AUTHORITY[\"EPSG\",\"1027\"]],\n\
        UNIT[\"metre\",1,\n\
            AUTHORITY[\"EPSG\",\"9001\"]],\n\
        AXIS[\"Up\",UP],\n\
        AUTHORITY[\"EPSG\",\"3855\"]]";
    let object = parse(wkt);
    let crs = object.bound().expect("bound");
    assert_eq!(crs.source_crs().name(), "EGM2008 geoid height");
    assert_eq!(crs.target_crs().name(), GeographicCRS::epsg_4979().name());
    let transformation = transformation_of(crs);
    assert_eq!(
        transformation.source_crs().expect("source").name(),
        crs.source_crs().name()
    );
    assert_eq!(
        transformation.target_crs().expect("target").name(),
        crs.target_crs().name()
    );
    assert_eq!(
        transformation.name(),
        "EGM2008 geoid height to WGS 84 ellipsoidal height"
    );
    assert_eq!(
        transformation.method().name(),
        "GravityRelatedHeight to Geographic3D"
    );
    let parameters = transformation.parameters();
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0].code(), "8666");
    assert_eq!(parameters[0].name(), "Geoid (height correction) model file");
    let ParameterValue::Filename(file) = parameters[0].value() else {
        panic!("not a filename");
    };
    assert_eq!(file, "egm08_25.gtx");
}

#[test]
fn wkt_parse_wkt1_vert_datum_extension_units_ftus() {
    let wkt = "VERT_CS[\"NAVD88 height (ftUS)\",\
         VERT_DATUM[\"North American Vertical Datum 1988\",2005,\
             EXTENSION[\"PROJ4_GRIDS\",\"foo.gtx\"],\
             AUTHORITY[\"EPSG\",\"5103\"]],\
         UNIT[\"US survey foot\",0.304800609601219,\
             AUTHORITY[\"EPSG\",\"9003\"]],\
         AXIS[\"Gravity-related height\",UP],\
         AUTHORITY[\"EPSG\",\"6360\"]]";
    let object = parse(wkt);
    let crs = object.bound().expect("bound");
    let transformation = transformation_of(crs);
    assert_eq!(
        transformation.name(),
        "NAVD88 height to WGS 84 ellipsoidal height"
    );
    let Crs::Vertical(source) = transformation.source_crs().expect("source") else {
        panic!("source is not vertical");
    };
    assert_eq!(source.coordinate_system().axis().unit().name(), "metre");
}

#[test]
fn wkt_parse_wkt1_compd_cs_vert_datum_extension() {
    let wkt = r#"COMPD_CS["NAD83 + NAVD88 height",
    GEOGCS["NAD83",
        DATUM["North_American_Datum_1983",
            SPHEROID["GRS 1980",6378137,298.257222101,
                AUTHORITY["EPSG","7019"]],
            AUTHORITY["EPSG","6269"]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]],
        AUTHORITY["EPSG","4269"]],
    VERT_CS["NAVD88 height",
        VERT_DATUM["North American Vertical Datum 1988",2005,
            EXTENSION["PROJ4_GRIDS","@foo.gtx"],
            AUTHORITY["EPSG","5103"]],
        UNIT["metre",1,
            AUTHORITY["EPSG","9001"]],
        AXIS["Gravity-related height",UP],
        AUTHORITY["EPSG","5703"]]]"#;
    let object = parse(wkt);
    let crs = object.compound().expect("compound");
    let Crs::Bound(vertical) = &crs.components()[1] else {
        panic!("vertical component is not bound");
    };
    assert_eq!(
        transformation_of(vertical).name(),
        "NAVD88 height to NAD83 ellipsoidal height"
    );
    assert_eq!(
        export(&Crs::Compound(crs.clone()), WktConvention::Wkt1Gdal).expect("WKT1"),
        wkt
    );
}

#[test]
fn wkt_parse_wkt1_datum_extension() {
    let wkt = "PROJCS[\"unnamed\",\n\
    GEOGCS[\"International 1909 (Hayford)\",\n\
        DATUM[\"unknown\",\n\
            SPHEROID[\"intl\",6378388,297],\n\
            EXTENSION[\"PROJ4_GRIDS\",\"nzgd2kgrid0005.gsb\"]],\n\
        PRIMEM[\"Greenwich\",0],\n\
        UNIT[\"degree\",0.0174532925199433]],\n\
    PROJECTION[\"New_Zealand_Map_Grid\"],\n\
    PARAMETER[\"latitude_of_origin\",-41],\n\
    PARAMETER[\"central_meridian\",173],\n\
    PARAMETER[\"false_easting\",2510000],\n\
    PARAMETER[\"false_northing\",6023150],\n\
    UNIT[\"Meter\",1]]";
    let object = parse(wkt);
    let crs = object.bound().expect("bound");
    assert_eq!(crs.source_crs().name(), "unnamed");
    assert_eq!(crs.target_crs().name(), GeographicCRS::epsg_4326().name());
    let transformation = transformation_of(crs);
    assert_eq!(
        transformation.source_crs().expect("source").name(),
        "International 1909 (Hayford)"
    );
    assert_eq!(
        transformation.target_crs().expect("target").name(),
        crs.target_crs().name()
    );
    assert_eq!(
        transformation.name(),
        "International 1909 (Hayford) to WGS84"
    );
    assert_eq!(transformation.method().name(), "NTv2");
    let parameters = transformation.parameters();
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0].code(), "8656");
    assert_eq!(
        parameters[0].name(),
        "Latitude and longitude difference file"
    );
    let ParameterValue::Filename(file) = parameters[0].value() else {
        panic!("not a filename");
    };
    assert_eq!(file, "nzgd2kgrid0005.gsb");
}

fn hidden_wkt(crs: &Crs, convention: WktConvention) -> String {
    let mut formatter = WktFormatter::create(convention);
    formatter.hide_identifiers().expect("setOutputId");
    crs.export_to_wkt(&mut formatter).expect("wkt")
}

fn compact(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn coordinate_operation_wkt(convention: WktConvention, version: Option<&str>) -> String {
    let src = hidden_wkt(&Crs::Geographic(GeographicCRS::epsg_4326()), convention);
    let dst = hidden_wkt(&Crs::Geographic(GeographicCRS::epsg_4807()), convention);
    let interpolation = hidden_wkt(&Crs::Geographic(GeographicCRS::epsg_4979()), convention);
    let version = version
        .map(|version| format!("    VERSION[\"{version}\"],\n"))
        .unwrap_or_default();
    format!(
        "COORDINATEOPERATION[\"transformationName\",\n\
         {version}\
         \x20   SOURCECRS[{src}],\n\
         \x20   TARGETCRS[{dst}],\n\
         \x20   METHOD[\"operationMethodName\",\n\
         \x20       ID[\"codeSpaceOperationMethod\",\"codeOperationMethod\"]],\n\
         \x20   PARAMETERFILE[\"paramName\",\"foo.bin\"],\n\
         \x20   INTERPOLATIONCRS[{interpolation}],\n\
         \x20   OPERATIONACCURACY[0.1],\n\
         \x20   ID[\"codeSpaceTransformation\",\"codeTransformation\"],\n\
         \x20   REMARK[\"my remarks\"]]"
    )
}

#[test]
fn wkt_parse_coordinateoperation() {
    let wkt = coordinate_operation_wkt(WktConvention::Wkt2_2015, None);
    let transformation = parse(&wkt)
        .transformation()
        .expect("transformation")
        .clone();
    assert_eq!(transformation.name(), "transformationName");
    assert_eq!(transformation.identifiers().len(), 1);
    assert_eq!(transformation.identifiers()[0].code(), "codeTransformation");
    assert_eq!(
        transformation.identifiers()[0].code_space(),
        "codeSpaceTransformation"
    );
    let accuracy = transformation.accuracy().expect("accuracy");
    assert!((accuracy - 0.1).abs() < 1e-12);
    assert_eq!(
        transformation.source_crs().expect("source").name(),
        GeographicCRS::epsg_4326().name()
    );
    assert_eq!(
        transformation.target_crs().expect("target").name(),
        GeographicCRS::epsg_4807().name()
    );
    assert_eq!(
        transformation
            .interpolation_crs()
            .expect("interpolation")
            .name(),
        GeographicCRS::epsg_4979().name()
    );
    assert_eq!(transformation.method().name(), "operationMethodName");
    assert_eq!(transformation.parameters().len(), 1);
    let out = proj_rs::CoordinateOperation::Transformation(transformation)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("WKT");
    assert_eq!(compact(&out), compact(&wkt));
}

#[test]
fn wkt_parse_coordinateoperation_with_interpolation_as_parameter() {
    let wkt = r#"COORDINATEOPERATION["SHGD2015 to SHGD2015 + SHVD2015 height (1)",
    VERSION["ENRD-Shn Hel"],
    SOURCECRS[
        GEOGCRS["SHGD2015",
            DATUM["St. Helena Geodetic Datum 2015",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            CS[ellipsoidal,3],
                AXIS["latitude",north,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["longitude",east,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["ellipsoidal height",up,
                    ORDER[3],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",7885]]],
    TARGETCRS[
        COMPOUNDCRS["SHMG2015 + SHVD2015 height",
            PROJCRS["SHMG2015",
                BASEGEOGCRS["SHGD2015",
                    DATUM["St. Helena Geodetic Datum 2015",
                        ELLIPSOID["GRS 1980",6378137,298.257222101,
                            LENGTHUNIT["metre",1]]],
                    PRIMEM["Greenwich",0,
                        ANGLEUNIT["degree",0.0174532925199433]],
                    ID["EPSG",7886]],
                CONVERSION["UTM zone 30S",
                    METHOD["Transverse Mercator",
                        ID["EPSG",9807]],
                    PARAMETER["Latitude of natural origin",0,
                        ANGLEUNIT["degree",0.0174532925199433],
                        ID["EPSG",8801]],
                    PARAMETER["Longitude of natural origin",-3,
                        ANGLEUNIT["degree",0.0174532925199433],
                        ID["EPSG",8802]],
                    PARAMETER["Scale factor at natural origin",0.9996,
                        SCALEUNIT["unity",1],
                        ID["EPSG",8805]],
                    PARAMETER["False easting",500000,
                        LENGTHUNIT["metre",1],
                        ID["EPSG",8806]],
                    PARAMETER["False northing",10000000,
                        LENGTHUNIT["metre",1],
                        ID["EPSG",8807]]],
                CS[Cartesian,2],
                    AXIS["(E)",east,
                        ORDER[1],
                        LENGTHUNIT["metre",1]],
                    AXIS["(N)",north,
                        ORDER[2],
                        LENGTHUNIT["metre",1]]],
            VERTCRS["SHVD2015 height",
                VDATUM["St. Helena Vertical Datum 2015"],
                CS[vertical,1],
                    AXIS["gravity-related height (H)",up,
                        LENGTHUNIT["metre",1]]],
            ID["EPSG",7956]]],
    METHOD["Geog3D to Geog2D+GravityRelatedHeight (EGM2008)",
        ID["EPSG",1092]],
    PARAMETERFILE["Geoid (height correction) model file","Und_min2.5x2.5_egm2008_isw=82_WGS84_TideFree.gz"],
    PARAMETER["EPSG code for Interpolation CRS",7886,
        ID["EPSG",1048]],
    OPERATIONACCURACY[0],
    ID["EPSG",9617]]"#;
    {
        let object = parse(wkt);
        let transformation = object.transformation().expect("transformation");
        assert!(transformation.interpolation_crs().is_none());
        assert_eq!(transformation.parameters().len(), 2);
    }
    {
        let object = parse_db(wkt);
        let transformation = object.transformation().expect("transformation");
        assert!(transformation.interpolation_crs().is_some());
        assert_eq!(transformation.parameters().len(), 1);
    }
}

#[test]
fn wkt_parse_coordinateoperation_wkt2_2019() {
    let wkt = coordinate_operation_wkt(WktConvention::Wkt2_2019, Some("my version"));
    let transformation = parse(&wkt)
        .transformation()
        .expect("transformation")
        .clone();
    assert_eq!(transformation.name(), "transformationName");
    assert_eq!(transformation.version(), Some("my version"));
    assert_eq!(transformation.identifiers().len(), 1);
    assert_eq!(transformation.identifiers()[0].code(), "codeTransformation");
    assert_eq!(
        transformation.identifiers()[0].code_space(),
        "codeSpaceTransformation"
    );
    let accuracy = transformation.accuracy().expect("accuracy");
    assert!((accuracy - 0.1).abs() < 1e-12);
    assert_eq!(
        transformation.source_crs().expect("source").name(),
        GeographicCRS::epsg_4326().name()
    );
    assert_eq!(
        transformation.target_crs().expect("target").name(),
        GeographicCRS::epsg_4807().name()
    );
    assert_eq!(
        transformation
            .interpolation_crs()
            .expect("interpolation")
            .name(),
        GeographicCRS::epsg_4979().name()
    );
    assert_eq!(transformation.method().name(), "operationMethodName");
    assert_eq!(transformation.parameters().len(), 1);
    let out = proj_rs::CoordinateOperation::Transformation(transformation.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT");
    assert_eq!(compact(&out), compact(&wkt));
    let wkt2015 = proj_rs::CoordinateOperation::Transformation(transformation)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("WKT");
    assert!(!wkt2015.contains("VERSION[\"my version\"],"));
}
