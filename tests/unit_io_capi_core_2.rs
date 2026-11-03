//! Lot `io-capi-core-2`: WKT1 ESRI projections, one concatenated round-trip,
//! longlat axis pipelines, and the engineering CRS checks from `test_io.cpp`.
//!
//! Each function is one C++ `TEST()` name. Expected text is copied from PROJ 9.9.0.

use proj_rs::{
    create_crs_from_proj_string, AuthorityFactory, ConcatenatedOperation, CoordinateOperation,
    CoordinateOperationFactory, Crs, DatabaseContext, GeographicCRS, OpProps, OperationParameter,
    ParameterValue, ProjLines, Transformation, WktConvention, WktFormatter, WktLines, WktObject,
    WktParser,
};

fn parse(wkt: &str) -> proj_rs::WktObject {
    WktParser::new().create_from_wkt(wkt).expect("wkt")
}

fn parse_db(wkt: &str) -> proj_rs::WktObject {
    let mut parser = WktParser::new();
    parser.attach_database(DatabaseContext::create().expect("proj.db"));
    parser.create_from_wkt(wkt).expect("wkt")
}

fn export(crs: &Crs, convention: WktConvention) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention))
        .expect("wkt export")
}

fn export_db(crs: &Crs, convention: WktConvention, db: &DatabaseContext) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention).with_database(db.clone()))
        .expect("wkt export")
}

fn number_token(value: f64) -> String {
    let text = format!("{value}");
    if text == "-0" {
        "0".to_owned()
    } else {
        text
    }
}

/// `wkt_parse.esri_projcs`.
#[test]
fn wkt_parse_esri_projcs() {
    for case in ESRI_PROJCS {
        let crs_name = if case.esri == "Plate_Carree" {
            "Plate Carree"
        } else {
            "unnamed"
        };
        let mut wkt = format!(
            "PROJCS[\"{crs_name}\",GEOGCS[\"unnamed\",DATUM[\"unnamed\",SPHEROID[\"unnamed\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"{}\"],",
            case.esri
        );
        for (name, value) in case.esri_params {
            wkt.push_str(&format!("PARAMETER[\"{name}\",{}],", number_token(*value)));
        }
        wkt.push_str("UNIT[\"Meter\",1.0]]");
        let crs = parse(&wkt)
            .projected()
            .unwrap_or_else(|| panic!("not projected: {wkt}"))
            .clone();
        let conversion = crs.deriving_conversion();
        assert_eq!(conversion.method().name(), case.wkt2, "{wkt}");
        let got = conversion.parameters();
        assert_eq!(
            got.len(),
            case.wkt2_params.len(),
            "{wkt}\nmethod {}",
            case.wkt2
        );
        for (parameter, (name, value)) in got.iter().zip(case.wkt2_params.iter()) {
            assert_eq!(parameter.name(), *name, "{wkt}");
            let ParameterValue::Measure(measure) = parameter.value() else {
                panic!("{name} is not a measure in {wkt}");
            };
            assert_eq!(measure.value(), *value, "{name} in {wkt}");
        }
        let expected_projection = match case.esri {
            "Transverse_Mercator_Complex" => "Transverse_Mercator",
            "Equidistant_Cylindrical_Ellipsoidal" => "Equidistant_Cylindrical",
            "Mercator_Variant_C" => "Mercator",
            "Gnomonic_Ellipsoidal" => "Gnomonic",
            other => other,
        };
        let exported = export(&Crs::Projected(crs), WktConvention::Wkt1Esri);
        let needle = format!("PROJECTION[\"{expected_projection}\"");
        assert!(
            exported.contains(&needle),
            "input: {wkt}\noutput: {exported}\nmissing {needle}"
        );
    }
}

/// `wkt_parse.wkt1_esri_krovak_south_west`.
#[test]
fn wkt_parse_wkt1_esri_krovak_south_west() {
    let wkt = r#"PROJCS["S-JTSK_Krovak",GEOGCS["GCS_S_JTSK",DATUM["D_S_JTSK",SPHEROID["Bessel_1841",6377397.155,299.1528128]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Krovak"],PARAMETER["False_Easting",0.0],PARAMETER["False_Northing",0.0],PARAMETER["Pseudo_Standard_Parallel_1",78.5],PARAMETER["Scale_Factor",0.9999],PARAMETER["Azimuth",30.28813975277778],PARAMETER["Longitude_Of_Center",24.83333333333333],PARAMETER["Latitude_Of_Center",49.5],PARAMETER["X_Scale",1.0],PARAMETER["Y_Scale",1.0],PARAMETER["XY_Plane_Rotation",0.0],UNIT["Meter",1.0]]"#;
    let crs = parse_db(wkt).projected().expect("projected").clone();
    assert_eq!(crs.deriving_conversion().method().name(), "Krovak");
    let expected = r#"PROJCRS["S-JTSK / Krovak",
    BASEGEODCRS["S-JTSK",
        DATUM["System of the Unified Trigonometrical Cadastral Network",
            ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                LENGTHUNIT["metre",1]],
            ID["EPSG",6156]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["Degree",0.0174532925199433]]],
    CONVERSION["unknown",
        METHOD["Krovak",
            ID["EPSG",9819]],
        PARAMETER["Latitude of projection centre",49.5,
            ANGLEUNIT["Degree",0.0174532925199433],
            ID["EPSG",8811]],
        PARAMETER["Longitude of origin",24.8333333333333,
            ANGLEUNIT["Degree",0.0174532925199433],
            ID["EPSG",8833]],
        PARAMETER["Co-latitude of cone axis",30.2881397527778,
            ANGLEUNIT["Degree",0.0174532925199433],
            ID["EPSG",1036]],
        PARAMETER["Latitude of pseudo standard parallel",78.5,
            ANGLEUNIT["Degree",0.0174532925199433],
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
        AXIS["southing",south,
            ORDER[1],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["westing",west,
            ORDER[2],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#;
    assert_eq!(
        export(&Crs::Projected(crs), WktConvention::Wkt2_2015),
        expected
    );
}

/// `wkt_parse.wkt1_esri_krovak_east_north_non_standard_likely_from_GDAL_wkt1`.
#[test]
fn wkt_parse_wkt1_esri_krovak_east_north_non_standard_likely_from_gdal_wkt1() {
    let wkt = r#"PROJCS["S_JTSK_Krovak_East_North",GEOGCS["GCS_S-JTSK",DATUM["D_S_JTSK",SPHEROID["Bessel_1841",6377397.155,299.1528128]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]],PROJECTION["Krovak"],PARAMETER["latitude_of_center",49.5],PARAMETER["longitude_of_center",24.83333333333333],PARAMETER["azimuth",30.2881397527778],PARAMETER["pseudo_standard_parallel_1",78.5],PARAMETER["scale_factor",0.9999],PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["Meter",1]]"#;
    let crs = parse_db(wkt).projected().expect("projected").clone();
    assert_eq!(
        crs.deriving_conversion().method().name(),
        "Krovak (North Orientated)"
    );
    let expected = r#"PROJCRS["S_JTSK_Krovak_East_North",
    BASEGEODCRS["GCS_S-JTSK",
        DATUM["System of the Unified Trigonometrical Cadastral Network",
            ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                LENGTHUNIT["metre",1]],
            ID["EPSG",6156]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["Degree",0.0174532925199433]]],
    CONVERSION["unknown",
        METHOD["Krovak (North Orientated)",
            ID["EPSG",1041]],
        PARAMETER["Latitude of projection centre",49.5,
            ANGLEUNIT["Degree",0.0174532925199433],
            ID["EPSG",8811]],
        PARAMETER["Longitude of origin",24.8333333333333,
            ANGLEUNIT["Degree",0.0174532925199433],
            ID["EPSG",8833]],
        PARAMETER["Co-latitude of cone axis",30.2881397527778,
            ANGLEUNIT["Degree",0.0174532925199433],
            ID["EPSG",1036]],
        PARAMETER["Latitude of pseudo standard parallel",78.5,
            ANGLEUNIT["Degree",0.0174532925199433],
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
        AXIS["(E)",east,
            ORDER[1],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["(N)",north,
            ORDER[2],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#;
    assert_eq!(
        export(&Crs::Projected(crs), WktConvention::Wkt2_2015),
        expected
    );
}

fn ups_axes(wkt: &str, name: &str, direction: &str) {
    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .projected()
        .expect("projected")
        .clone();
    assert_eq!(crs.name(), name);
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes[0].direction(), direction);
    assert_eq!(axes[0].abbreviation(), "E");
    assert_eq!(axes[1].direction(), direction);
    assert_eq!(axes[1].abbreviation(), "N");
    assert_eq!(
        export_db(&Crs::Projected(crs), WktConvention::Wkt1Esri, &db),
        wkt
    );
}

/// `wkt_parse.wkt1_esri_ups_north`.
#[test]
fn wkt_parse_wkt1_esri_ups_north() {
    let wkt = r#"PROJCS["UPS_North",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Stereographic"],PARAMETER["False_Easting",2000000.0],PARAMETER["False_Northing",2000000.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Scale_Factor",0.994],PARAMETER["Latitude_Of_Origin",90.0],UNIT["Meter",1.0]]"#;
    ups_axes(wkt, "WGS 84 / UPS North (N,E)", "south");
}

/// `wkt_parse.wkt1_esri_ups_south`.
#[test]
fn wkt_parse_wkt1_esri_ups_south() {
    let wkt = r#"PROJCS["UPS_South",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Stereographic"],PARAMETER["False_Easting",2000000.0],PARAMETER["False_Northing",2000000.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Scale_Factor",0.994],PARAMETER["Latitude_Of_Origin",-90.0],UNIT["Meter",1.0]]"#;
    ups_axes(wkt, "WGS 84 / UPS South (N,E)", "north");
}

/// `wkt_parse.wkt1_esri_wgs_1984_ups_north_E_N`.
#[test]
fn wkt_parse_wkt1_esri_wgs_1984_ups_north_e_n() {
    let wkt = r#"PROJCS["WGS_1984_UPS_North_(E-N)",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Polar_Stereographic_Variant_A"],PARAMETER["False_Easting",2000000.0],PARAMETER["False_Northing",2000000.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Scale_Factor",0.994],PARAMETER["Latitude_Of_Origin",90.0],UNIT["Meter",1.0]]"#;
    ups_axes(wkt, "WGS 84 / UPS North (E,N)", "south");
}

/// `wkt_parse.wkt1_esri_wgs_1984_ups_south_E_N`.
#[test]
fn wkt_parse_wkt1_esri_wgs_1984_ups_south_e_n() {
    let wkt = r#"PROJCS["WGS_1984_UPS_South_(E-N)",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Polar_Stereographic_Variant_A"],PARAMETER["False_Easting",2000000.0],PARAMETER["False_Northing",2000000.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Scale_Factor",0.994],PARAMETER["Latitude_Of_Origin",-90.0],UNIT["Meter",1.0]]"#;
    ups_axes(wkt, "WGS 84 / UPS South (E,N)", "north");
}

/// `wkt_parse.wkt1_esri_gauss_kruger`.
#[test]
fn wkt_parse_wkt1_esri_gauss_kruger() {
    let wkt = r#"PROJCS["ETRS_1989_UWPP_2000_PAS_8",GEOGCS["GCS_ETRS_1989",DATUM["D_ETRS_1989",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Gauss_Kruger"],PARAMETER["False_Easting",8500000.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",24.0],PARAMETER["Scale_Factor",0.999923],PARAMETER["Latitude_Of_Origin",0.0],UNIT["Meter",1.0]]"#;
    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .projected()
        .expect("projected")
        .clone();
    assert_eq!(
        export_db(&Crs::Projected(crs), WktConvention::Wkt1Esri, &db),
        wkt
    );
    let from_db = AuthorityFactory::create(&db, "ESRI")
        .create_projected_crs("102177")
        .expect("ESRI:102177");
    assert_eq!(
        export_db(&Crs::Projected(from_db), WktConvention::Wkt1Esri, &db),
        wkt
    );
}

/// `wkt_parse.wkt1_esri_goode_homolosine_without_option_0`.
#[test]
fn wkt_parse_wkt1_esri_goode_homolosine_without_option_0() {
    let wkt = r#"PROJCS["unknown",GEOGCS["GCS_unknown",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Goode_Homolosine"],PARAMETER["False_Easting",0.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",0.0],UNIT["Meter",1.0]]"#;
    let crs = parse_db(wkt).projected().expect("projected").clone();
    assert_eq!(
        crs.deriving_conversion().method().name(),
        "Goode Homolosine"
    );
}

/// `wkt_parse.wkt1_lcc_1sp_without_1sp_suffix`.
#[test]
fn wkt_parse_wkt1_lcc_1sp_without_1sp_suffix() {
    let wkt = r#"PROJCS["TWM-Madison Co LDP",GEOGCS["WGS 1984",DATUM["WGS 1984",SPHEROID["World Geodetic System 1984",6378137,298.257223563]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["Degree",0.01745329251994,AUTHORITY["EPSG","9102"]],AXIS["Long",EAST],AXIS["Lat",NORTH]],PROJECTION["Lambert_Conformal_Conic"],PARAMETER["False_Easting",103000.0000035],PARAMETER["False_Northing",79000.00007055],PARAMETER["Latitude_Of_Origin",38.83333333333],PARAMETER["Central_Meridian",-89.93333333333],PARAMETER["Scale_Factor",1.000019129],UNIT["Foot_US",0.3048006096012,AUTHORITY["EPSG","9003"]],AXIS["East",EAST],AXIS["North",NORTH]]"#;
    let crs = parse_db(wkt).projected().expect("projected").clone();
    assert_eq!(
        crs.deriving_conversion().method().name(),
        "Lambert Conic Conformal (1SP)"
    );
}

/// `wkt_parse.wkt1_pseudo_wkt1_gdal_pseudo_wkt1_esri`.
#[test]
fn wkt_parse_wkt1_pseudo_wkt1_gdal_pseudo_wkt1_esri() {
    let wkt = r#"PROJCS["Equidistant_Cylindrical",GEOGCS["WGS 84",DATUM["wgs_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9102"]],AUTHORITY["EPSG","4326"]],PROJECTION["Equidistant_Cylindrical"],PARAMETER["false_easting",0],PARAMETER["false_northing",0],PARAMETER["central_meridian",0],PARAMETER["standard_parallel_1",37],UNIT["Meter",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#;
    let crs = parse_db(wkt).projected().expect("projected").clone();
    assert_eq!(
        crs.deriving_conversion().method().name(),
        "Equidistant Cylindrical"
    );
    assert_eq!(crs.deriving_conversion().method().code(), "1028");
}

/// `wkt_parse.CONCATENATEDOPERATION`.
#[test]
fn wkt_parse_concatenatedoperation() {
    let transf = |name: &str, source: Crs, target: Crs| {
        Transformation::create(
            &OpProps::new().name(name),
            source,
            target,
            None,
            "operationMethodName",
            vec![OperationParameter::create("paramName")],
            vec![ParameterValue::filename("foo.bin")],
            None,
        )
        .expect(name)
    };
    let transf_1 = transf(
        "transf_1",
        Crs::Geographic(GeographicCRS::epsg_4326()),
        Crs::Geographic(GeographicCRS::epsg_4807()),
    );
    let transf_2 = transf(
        "transf_2",
        Crs::Geographic(GeographicCRS::epsg_4807()),
        Crs::Geographic(GeographicCRS::epsg_4979()),
    );
    let concat = ConcatenatedOperation::create(
        &OpProps::new()
            .identifier("codeSpace", "code")
            .name("name")
            .remarks("my remarks"),
        vec![
            CoordinateOperation::Transformation(transf_1.clone()),
            CoordinateOperation::Transformation(transf_2.clone()),
        ],
        Some(0.1),
    )
    .expect("concat");
    let wkt = concat
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt");
    let WktObject::Concatenated(parsed) = WktParser::new().create_from_wkt(&wkt).expect("parse")
    else {
        panic!("not a concatenated operation");
    };
    assert_eq!(parsed.name(), "name");
    assert!(parsed.version().is_none());
    let ids = parsed.identifiers();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].code(), "code");
    assert_eq!(ids[0].code_space(), "codeSpace");
    assert_eq!(parsed.steps().len(), 2);
    assert_eq!(parsed.steps()[0].operation().name(), transf_1.name());
    assert_eq!(parsed.steps()[1].operation().name(), transf_2.name());
    assert_eq!(
        parsed.source_crs().map(Crs::name),
        transf_1.source_crs().map(Crs::name)
    );
    assert_eq!(
        parsed.target_crs().map(Crs::name),
        transf_2.target_crs().map(Crs::name)
    );
}

fn longlat_axis(definition: &str) -> (String, String) {
    let crs = create_crs_from_proj_string(definition).expect("crs");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.set_lines(WktLines::Single);
    let wkt = crs.export_to_wkt(&mut formatter).expect("wkt");
    let source = Crs::Geographic(GeographicCRS::epsg_4326());
    let operation = CoordinateOperationFactory::create()
        .create_operation(&source, &crs)
        .expect("operation")
        .expect("an operation");
    let proj = match &operation {
        CoordinateOperation::Conversion(conversion) => {
            conversion.export_to_proj_string().expect("proj")
        }
        CoordinateOperation::Transformation(transformation) => transformation
            .export_to_proj_string(ProjLines::Single)
            .expect("proj"),
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .export_to_proj_string(ProjLines::Single)
            .expect("proj"),
    };
    (wkt, proj)
}

/// `io.projparse_longlat_complex`.
#[test]
fn io_projparse_longlat_complex() {
    let input = "+step +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=unitconvert +xy_in=rad +xy_out=grad +step +proj=axisswap +order=2,1";
    let crs =
        create_crs_from_proj_string(&format!("+type=crs +proj=pipeline {input}")).expect("crs");
    let source = Crs::Geographic(GeographicCRS::epsg_4326());
    let operation = CoordinateOperationFactory::create()
        .create_operation(&source, &crs)
        .expect("operation")
        .expect("an operation");
    let proj = match &operation {
        CoordinateOperation::Conversion(conversion) => {
            conversion.export_to_proj_string().expect("proj")
        }
        CoordinateOperation::Transformation(transformation) => transformation
            .export_to_proj_string(ProjLines::Single)
            .expect("proj"),
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .export_to_proj_string(ProjLines::Single)
            .expect("proj"),
    };
    assert_eq!(
        proj,
        format!(
            "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad {input}"
        )
    );
}

/// `io.projparse_longlat_axis_enu`.
#[test]
fn io_projparse_longlat_axis_enu() {
    let (wkt, proj) = longlat_axis("+proj=longlat +ellps=GRS80 +axis=enu +type=crs");
    assert!(wkt.contains("AXIS[\"longitude\",east,ORDER[1]"), "{wkt}");
    assert!(wkt.contains("AXIS[\"latitude\",north,ORDER[2]"), "{wkt}");
    assert_eq!(proj, "+proj=axisswap +order=2,1");
}

/// `io.projparse_longlat_axis_neu`.
#[test]
fn io_projparse_longlat_axis_neu() {
    let (wkt, proj) = longlat_axis("+proj=longlat +ellps=GRS80 +axis=neu +type=crs");
    assert!(wkt.contains("AXIS[\"latitude\",north,ORDER[1]"), "{wkt}");
    assert!(wkt.contains("AXIS[\"longitude\",east,ORDER[2]"), "{wkt}");
    assert_eq!(proj, "+proj=noop");
}

/// `io.projparse_longlat_axis_swu`.
#[test]
fn io_projparse_longlat_axis_swu() {
    let (wkt, proj) = longlat_axis("+proj=longlat +ellps=GRS80 +axis=swu +type=crs");
    assert!(wkt.contains("AXIS[\"latitude\",south,ORDER[1]"), "{wkt}");
    assert!(wkt.contains("AXIS[\"longitude\",west,ORDER[2]"), "{wkt}");
    assert_eq!(
        proj,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=axisswap +order=-2,-1"
    );
}

/// `wkt_parse.invalid_EngineeingCRS` (the C++ name spells Engineering that way).
#[test]
fn wkt_parse_invalid_engineeingcrs() {
    let ok = "ENGCRS[\"name\",\n    EDATUM[\"name\"],\n    CS[temporal,1],\n        AXIS[\"time (T)\",future]]";
    parse(ok).engineering().expect("engineering CRS");
    assert!(WktParser::new()
        .create_from_wkt(
            "ENGCRS[\"name\",\n    CS[temporal,1],\n        AXIS[\"time (T)\",future]]"
        )
        .is_err());
    assert!(WktParser::new()
        .create_from_wkt("ENGCRS[\"name\",\n    EDATUM[\"name\"]]")
        .is_err());
}
struct EsriProjCase {
    esri: &'static str,
    esri_params: &'static [(&'static str, f64)],
    wkt2: &'static str,
    wkt2_params: &'static [(&'static str, f64)],
}

const ESRI_PROJCS: &[EsriProjCase] = &[
    EsriProjCase {
        esri: "Plate_Carree",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Equidistant Cylindrical",
        wkt2_params: &[
            ("Latitude of 1st standard parallel", 0.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Equidistant_Cylindrical",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Equidistant Cylindrical",
        wkt2_params: &[
            ("Latitude of 1st standard parallel", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Miller_Cylindrical",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Miller Cylindrical",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Mercator",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Mercator (variant B)",
        wkt2_params: &[
            ("Latitude of 1st standard parallel", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Gauss_Kruger",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
        ],
        wkt2: "Transverse Mercator",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Transverse_Mercator",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
        ],
        wkt2: "Transverse Mercator",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Transverse_Mercator_Complex",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
        ],
        wkt2: "Transverse Mercator",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Albers",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
            ("Standard_Parallel_2", 5.0),
            ("Latitude_Of_Origin", 6.0),
        ],
        wkt2: "Albers Equal Area",
        wkt2_params: &[
            ("Latitude of false origin", 6.0),
            ("Longitude of false origin", 3.0),
            ("Latitude of 1st standard parallel", 4.0),
            ("Latitude of 2nd standard parallel", 5.0),
            ("Easting at false origin", 1.0),
            ("Northing at false origin", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Sinusoidal",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Sinusoidal",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Mollweide",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Mollweide",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Eckert_I",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Eckert I",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Gall_Stereographic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Gall Stereographic",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Patterson",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Patterson",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Natural_Earth",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Natural Earth",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Natural_Earth_II",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Natural Earth II",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Compact_Miller",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Compact Miller",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Times",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Times",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Flat_Polar_Quartic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Flat Polar Quartic",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Behrmann",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Lambert Cylindrical Equal Area",
        wkt2_params: &[
            ("Latitude of 1st standard parallel", 30.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Winkel_I",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Winkel I",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("Latitude of 1st standard parallel", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Winkel_II",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Winkel II",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("Latitude of 1st standard parallel", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Lambert_Conformal_Conic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
            ("Scale_Factor", 5.0),
            ("Latitude_Of_Origin", 4.0),
        ],
        wkt2: "Lambert Conic Conformal (1SP)",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 5.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Lambert_Conformal_Conic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
            ("Standard_Parallel_2", 5.0),
            ("Latitude_Of_Origin", 6.0),
        ],
        wkt2: "Lambert Conic Conformal (2SP)",
        wkt2_params: &[
            ("Latitude of false origin", 6.0),
            ("Longitude of false origin", 3.0),
            ("Latitude of 1st standard parallel", 4.0),
            ("Latitude of 2nd standard parallel", 5.0),
            ("Easting at false origin", 1.0),
            ("Northing at false origin", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Lambert_Conformal_Conic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
            ("Standard_Parallel_2", 5.0),
            ("Scale_Factor", 1.0),
            ("Latitude_Of_Origin", 6.0),
        ],
        wkt2: "Lambert Conic Conformal (2SP)",
        wkt2_params: &[
            ("Latitude of false origin", 6.0),
            ("Longitude of false origin", 3.0),
            ("Latitude of 1st standard parallel", 4.0),
            ("Latitude of 2nd standard parallel", 5.0),
            ("Easting at false origin", 1.0),
            ("Northing at false origin", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Polyconic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Latitude_Of_Origin", 4.0),
        ],
        wkt2: "American Polyconic",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Quartic_Authalic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Quartic Authalic",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Loximuthal",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Central_Parallel", 4.0),
        ],
        wkt2: "Loximuthal",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Bonne",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Bonne",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Hotine_Oblique_Mercator_Two_Point_Natural_Origin",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Latitude_Of_1st_Point", 3.0),
            ("Latitude_Of_2nd_Point", 4.0),
            ("Scale_Factor", 5.0),
            ("Longitude_Of_1st_Point", 6.0),
            ("Longitude_Of_2nd_Point", 7.0),
            ("Latitude_Of_Center", 8.0),
        ],
        wkt2: "Hotine Oblique Mercator Two Point Natural Origin",
        wkt2_params: &[
            ("Latitude of projection centre", 8.0),
            ("Latitude of 1st point", 3.0),
            ("Longitude of 1st point", 6.0),
            ("Latitude of 2nd point", 4.0),
            ("Longitude of 2nd point", 7.0),
            ("Scale factor at projection centre", 5.0),
            ("Easting at projection centre", 1.0),
            ("Northing at projection centre", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Stereographic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
        ],
        wkt2: "Stereographic",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Stereographic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("longitude_of_center", 3.0),
            ("Scale_Factor", 4.0),
            ("latitude_of_center", 5.0),
        ],
        wkt2: "Stereographic",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Stereographic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 90.0),
        ],
        wkt2: "Polar Stereographic (variant A)",
        wkt2_params: &[
            ("Latitude of natural origin", 90.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Stereographic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", -90.0),
        ],
        wkt2: "Polar Stereographic (variant A)",
        wkt2_params: &[
            ("Latitude of natural origin", -90.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Equidistant_Conic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
            ("Standard_Parallel_2", 5.0),
            ("Latitude_Of_Origin", 6.0),
        ],
        wkt2: "Equidistant Conic",
        wkt2_params: &[
            ("Latitude of false origin", 6.0),
            ("Longitude of false origin", 3.0),
            ("Latitude of 1st standard parallel", 4.0),
            ("Latitude of 2nd standard parallel", 5.0),
            ("Easting at false origin", 1.0),
            ("Northing at false origin", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Cassini",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 1.0),
            ("Latitude_Of_Origin", 4.0),
        ],
        wkt2: "Cassini-Soldner",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Van_der_Grinten_I",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Van Der Grinten",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Robinson",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Robinson",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Two_Point_Equidistant",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Latitude_Of_1st_Point", 3.0),
            ("Latitude_Of_2nd_Point", 4.0),
            ("Longitude_Of_1st_Point", 5.0),
            ("Longitude_Of_2nd_Point", 6.0),
        ],
        wkt2: "Two Point Equidistant",
        wkt2_params: &[
            ("Latitude of 1st point", 3.0),
            ("Longitude of 1st point", 5.0),
            ("Latitude of 2nd point", 4.0),
            ("Longitude of 2nd point", 6.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Azimuthal_Equidistant",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Latitude_Of_Origin", 4.0),
        ],
        wkt2: "Azimuthal Equidistant",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Lambert_Azimuthal_Equal_Area",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Latitude_Of_Origin", 4.0),
        ],
        wkt2: "Lambert Azimuthal Equal Area",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Cylindrical_Equal_Area",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Lambert Cylindrical Equal Area",
        wkt2_params: &[
            ("Latitude of 1st standard parallel", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Hotine_Oblique_Mercator_Azimuth_Natural_Origin",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Scale_Factor", 3.0),
            ("Azimuth", 4.0),
            ("Longitude_Of_Center", 5.0),
            ("Latitude_Of_Center", 6.0),
        ],
        wkt2: "Hotine Oblique Mercator (variant A)",
        wkt2_params: &[
            ("Latitude of projection centre", 6.0),
            ("Longitude of projection centre", 5.0),
            ("Azimuth at projection centre", 4.0),
            ("Angle from Rectified to Skew Grid", 4.0),
            ("Scale factor at projection centre", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Hotine_Oblique_Mercator_Azimuth_Center",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Scale_Factor", 3.0),
            ("Azimuth", 4.0),
            ("Longitude_Of_Center", 5.0),
            ("Latitude_Of_Center", 6.0),
        ],
        wkt2: "Hotine Oblique Mercator (variant B)",
        wkt2_params: &[
            ("Latitude of projection centre", 6.0),
            ("Longitude of projection centre", 5.0),
            ("Azimuth at projection centre", 4.0),
            ("Angle from Rectified to Skew Grid", 4.0),
            ("Scale factor at projection centre", 3.0),
            ("Easting at projection centre", 1.0),
            ("Northing at projection centre", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Double_Stereographic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
        ],
        wkt2: "Oblique Stereographic",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Krovak",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Pseudo_Standard_Parallel_1", 3.0),
            ("Scale_Factor", 4.0),
            ("Azimuth", 5.0),
            ("Longitude_Of_Center", 6.0),
            ("Latitude_Of_Center", 7.0),
            ("X_Scale", 1.0),
            ("Y_Scale", 1.0),
            ("XY_Plane_Rotation", 0.0),
        ],
        wkt2: "Krovak",
        wkt2_params: &[
            ("Latitude of projection centre", 7.0),
            ("Longitude of origin", 6.0),
            ("Co-latitude of cone axis", 5.0),
            ("Latitude of pseudo standard parallel", 3.0),
            ("Scale factor on pseudo standard parallel", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Krovak",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Pseudo_Standard_Parallel_1", 3.0),
            ("Scale_Factor", 4.0),
            ("Azimuth", 5.0),
            ("Longitude_Of_Center", 6.0),
            ("Latitude_Of_Center", 7.0),
            ("X_Scale", -1.0),
            ("Y_Scale", 1.0),
            ("XY_Plane_Rotation", 90.0),
        ],
        wkt2: "Krovak (North Orientated)",
        wkt2_params: &[
            ("Latitude of projection centre", 7.0),
            ("Longitude of origin", 6.0),
            ("Co-latitude of cone axis", 5.0),
            ("Latitude of pseudo standard parallel", 3.0),
            ("Scale factor on pseudo standard parallel", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "New_Zealand_Map_Grid",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Origin", 3.0),
            ("Latitude_Of_Origin", 4.0),
        ],
        wkt2: "New Zealand Map Grid",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Orthographic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Center", 3.0),
            ("Latitude_Of_Center", 4.0),
        ],
        wkt2: "Orthographic (Spherical)",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Local",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Scale_Factor", 1.25),
            ("Azimuth", 15.0),
            ("Longitude_Of_Center", 3.0),
            ("Latitude_Of_Center", 4.0),
        ],
        wkt2: "Local Orthographic",
        wkt2_params: &[
            ("Latitude of projection centre", 4.0),
            ("Longitude of projection centre", 3.0),
            ("Azimuth at projection centre", 15.0),
            ("Scale factor at projection centre", 1.25),
            ("Easting at projection centre", 1.0),
            ("Northing at projection centre", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Winkel_Tripel",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Winkel Tripel",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("Latitude of 1st standard parallel", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Aitoff",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Aitoff",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Craster_Parabolic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Craster Parabolic",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Gnomonic",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Center", 3.0),
            ("Latitude_Of_Center", 4.0),
        ],
        wkt2: "Gnomonic",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Stereographic_North_Pole",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Polar Stereographic (variant B)",
        wkt2_params: &[
            ("Latitude of standard parallel", 4.0),
            ("Longitude of origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Stereographic_South_Pole",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", -4.0),
        ],
        wkt2: "Polar Stereographic (variant B)",
        wkt2_params: &[
            ("Latitude of standard parallel", -4.0),
            ("Longitude of origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Rectified_Skew_Orthomorphic_Natural_Origin",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Scale_Factor", 3.0),
            ("Azimuth", 4.0),
            ("Longitude_Of_Center", 5.0),
            ("Latitude_Of_Center", 6.0),
            ("XY_Plane_Rotation", 7.0),
        ],
        wkt2: "Hotine Oblique Mercator (variant A)",
        wkt2_params: &[
            ("Latitude of projection centre", 6.0),
            ("Longitude of projection centre", 5.0),
            ("Azimuth at projection centre", 4.0),
            ("Angle from Rectified to Skew Grid", 7.0),
            ("Scale factor at projection centre", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Rectified_Skew_Orthomorphic_Center",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Scale_Factor", 3.0),
            ("Azimuth", 4.0),
            ("Longitude_Of_Center", 5.0),
            ("Latitude_Of_Center", 6.0),
            ("XY_Plane_Rotation", 7.0),
        ],
        wkt2: "Hotine Oblique Mercator (variant B)",
        wkt2_params: &[
            ("Latitude of projection centre", 6.0),
            ("Longitude of projection centre", 5.0),
            ("Azimuth at projection centre", 4.0),
            ("Angle from Rectified to Skew Grid", 7.0),
            ("Scale factor at projection centre", 3.0),
            ("Easting at projection centre", 1.0),
            ("Northing at projection centre", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Goode_Homolosine",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Option", 0.0),
        ],
        wkt2: "Goode Homolosine",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Goode_Homolosine",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Option", 1.0),
        ],
        wkt2: "Interrupted Goode Homolosine",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Goode_Homolosine",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Option", 2.0),
        ],
        wkt2: "Interrupted Goode Homolosine Ocean",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Equidistant_Cylindrical_Ellipsoidal",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
        ],
        wkt2: "Equidistant Cylindrical",
        wkt2_params: &[
            ("Latitude of 1st standard parallel", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Laborde_Oblique_Mercator",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Scale_Factor", 3.0),
            ("Azimuth", 4.0),
            ("Longitude_Of_Center", 5.0),
            ("Latitude_Of_Center", 6.0),
        ],
        wkt2: "Laborde Oblique Mercator",
        wkt2_params: &[
            ("Latitude of projection centre", 6.0),
            ("Longitude of projection centre", 5.0),
            ("Azimuth at projection centre", 4.0),
            ("Scale factor at projection centre", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Mercator_Variant_A",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Scale_Factor", 3.0),
            ("Central_Meridian", 4.0),
        ],
        wkt2: "Mercator (variant A)",
        wkt2_params: &[
            ("Longitude of natural origin", 4.0),
            ("Scale factor at natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Mercator_Variant_C",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Standard_Parallel_1", 3.0),
            ("Central_Meridian", 4.0),
        ],
        wkt2: "Mercator (variant B)",
        wkt2_params: &[
            ("Latitude of 1st standard parallel", 3.0),
            ("Longitude of natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Transverse_Cylindrical_Equal_Area",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
        ],
        wkt2: "Transverse Cylindrical Equal Area",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Gnomonic_Ellipsoidal",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Center", 3.0),
            ("Latitude_Of_Center", 4.0),
        ],
        wkt2: "Gnomonic",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Wagner_IV",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Latitude_Of_Center", 0.0),
        ],
        wkt2: "Wagner IV",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Wagner_V",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Wagner V",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Wagner_VII",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Wagner VII",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Geostationary_Satellite",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Center", 3.0),
            ("Height", 4.0),
            ("Option", 0.0),
        ],
        wkt2: "Geostationary Satellite (Sweep Y)",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("Satellite Height", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Mercator_Auxiliary_Sphere",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Standard_Parallel_1", 4.0),
            ("Auxiliary_Sphere_Type", 0.0),
        ],
        wkt2: "Popular Visualisation Pseudo Mercator",
        wkt2_params: &[
            ("Latitude of natural origin", 4.0),
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Vertical_Near_Side_Perspective",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Center", 3.0),
            ("Latitude_Of_Center", 4.0),
            ("Height", 5.0),
        ],
        wkt2: "Vertical Perspective",
        wkt2_params: &[
            ("Latitude of topocentric origin", 4.0),
            ("Longitude of topocentric origin", 3.0),
            ("Viewpoint height", 5.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Equal_Earth",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
        ],
        wkt2: "Equal Earth",
        wkt2_params: &[
            ("Longitude of natural origin", 3.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Peirce_Quincuncial",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
            ("Option", 0.0),
        ],
        wkt2: "Peirce Quincuncial (Square)",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Peirce_Quincuncial",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Central_Meridian", 3.0),
            ("Scale_Factor", 4.0),
            ("Latitude_Of_Origin", 5.0),
            ("Option", 1.0),
        ],
        wkt2: "Peirce Quincuncial (Diamond)",
        wkt2_params: &[
            ("Latitude of natural origin", 5.0),
            ("Longitude of natural origin", 3.0),
            ("Scale factor at natural origin", 4.0),
            ("False easting", 1.0),
            ("False northing", 2.0),
        ],
    },
    EsriProjCase {
        esri: "Unknown_Method",
        esri_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Origin", 3.0),
            ("Latitude_Of_Origin", 4.0),
        ],
        wkt2: "Unknown_Method",
        wkt2_params: &[
            ("False_Easting", 1.0),
            ("False_Northing", 2.0),
            ("Longitude_Of_Origin", 3.0),
            ("Latitude_Of_Origin", 4.0),
        ],
    },
];
