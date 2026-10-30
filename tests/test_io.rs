//! PROJ `test/unit/test_io.cpp` checks that parse WKT, PROJ strings, and PROJJSON.
//!
//! Each function is one `TEST()` name. Expected text is copied from that test.

use proj_rs::{
    create_crs_from_proj_string, Crs, DatabaseContext, GeographicCRS, UnitOfMeasure, WktConvention,
    WktDialect, WktFormatter, WktNode, WktObject, WktParser,
};

fn parse(wkt: &str) -> WktObject {
    WktParser::new().create_from_wkt(wkt).expect("wkt")
}

fn parse_db(wkt: &str) -> WktObject {
    let mut parser = WktParser::new();
    parser.attach_database(DatabaseContext::create().expect("proj.db"));
    parser.create_from_wkt(wkt).expect("wkt")
}

fn parse_loose(wkt: &str) -> WktObject {
    let mut parser = WktParser::new();
    parser.set_strictness(proj_rs::WktStrictness::Lenient);
    parser.create_from_wkt(wkt).expect("wkt")
}

fn refused(wkt: &str) {
    assert!(WktParser::new().create_from_wkt(wkt).is_err());
}

#[test]
fn io_wkt_parsing() {
    let node = WktNode::create_from("MYNODE[]").expect("node");
    assert_eq!(node.value(), "MYNODE");
    assert!(node.children().is_empty());
    assert_eq!(node.to_wkt(), "MYNODE");

    let node = WktNode::create_from("  MYNODE  [  ]  ").expect("node");
    assert_eq!(node.value(), "MYNODE");
    assert!(node.children().is_empty());

    assert!(WktNode::create_from("").is_err());
    assert!(WktNode::create_from("x").is_err());
    assert!(WktNode::create_from("x,").is_err());
    assert!(WktNode::create_from("x[").is_err());
    assert!(WktNode::create_from("[").is_err());

    let node = WktNode::create_from("MYNODE[\"x\"]").expect("node");
    assert_eq!(node.value(), "MYNODE");
    assert_eq!(node.children().len(), 1);
    assert_eq!(node.children()[0].value(), "\"x\"");
    assert_eq!(node.to_wkt(), "MYNODE[\"x\"]");

    assert!(WktNode::create_from("MYNODE[\"x\"").is_err());

    let node = WktNode::create_from("MYNODE[  \"x\"   ]").expect("node");
    assert_eq!(node.value(), "MYNODE");
    assert_eq!(node.children().len(), 1);
    assert_eq!(node.children()[0].value(), "\"x\"");

    let node = WktNode::create_from("MYNODE[\"x[\",1]").expect("node");
    assert_eq!(node.value(), "MYNODE");
    assert_eq!(node.children().len(), 2);
    assert_eq!(node.children()[0].value(), "\"x[\"");
    assert_eq!(node.children()[1].value(), "1");
    assert_eq!(node.to_wkt(), "MYNODE[\"x[\",1]");

    assert!(WktNode::create_from("MYNODE[\"x\",").is_err());

    let node = WktNode::create_from("A[B[y]]").expect("node");
    assert_eq!(node.value(), "A");
    assert_eq!(node.children().len(), 1);
    assert_eq!(node.children()[0].value(), "B");
    assert_eq!(node.children()[0].children().len(), 1);
    assert_eq!(node.children()[0].children()[0].value(), "y");
    assert_eq!(node.to_wkt(), "A[B[y]]");

    assert!(WktNode::create_from("A[B[").is_err());

    let mut nested = String::new();
    for _ in 0..17 {
        nested = format!("A[{nested}]");
    }
    assert!(WktNode::create_from(&nested).is_err());

    let wkt = "A[\"a\",B[\"b\",C[\"c\"]],D[\"d\"]]";
    assert_eq!(WktNode::create_from(wkt).expect("node").to_wkt(), wkt);
}

#[test]
fn io_wkt_parsing_with_parenthesis() {
    let node = WktNode::create_from("A(\"x\",B(\"y\"))").expect("node");
    assert_eq!(node.to_wkt(), "A[\"x\",B[\"y\"]]");
}

#[test]
fn io_wkt_parsing_with_double_quotes_inside() {
    let node = WktNode::create_from("A[\"xy\"\"z\"]").expect("node");
    assert_eq!(node.children()[0].value(), "\"xy\"z\"");
    assert_eq!(node.to_wkt(), "A[\"xy\"\"z\"]");
    assert!(WktNode::create_from("A[\"x\"").is_err());
}

#[test]
fn io_wkt_parsing_with_printed_quotes() {
    let start = "\u{201C}";
    let end = "\u{201D}";
    let node = WktNode::create_from(&format!("A[{start}x{end}]")).expect("node");
    assert_eq!(node.children()[0].value(), "\"x\"");
    assert_eq!(node.to_wkt(), "A[\"x\"]");
}

#[test]
fn wkt_parse_sphere() {
    let object = parse("ELLIPSOID[\"Sphere\",6378137,0,LENGTHUNIT[\"metre\",1]]");
    let ellipsoid = object.ellipsoid().expect("ellipsoid");
    assert!(ellipsoid.is_sphere());
}

#[test]
fn wkt_parse_datum_with_anchor() {
    let object = parse(
        "DATUM[\"WGS_1984 with anchor\",\n\
         ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
             LENGTHUNIT[\"metre\",1,\n\
                 ID[\"EPSG\",9001]],\n\
             ID[\"EPSG\",7030]],\n\
         ANCHOR[\"My anchor\"]]",
    );
    let datum = object.geodetic_datum().expect("datum");
    assert_eq!(datum.ellipsoid().celestial_body(), "Earth");
    assert_eq!(datum.prime_meridian().name(), "Greenwich");
    assert_eq!(datum.anchor(), Some("My anchor"));
    assert_eq!(datum.anchor_epoch(), None);
}

#[test]
fn wkt_parse_datum_with_anchorepoch() {
    let object = parse(
        "DATUM[\"my_datum\",\n\
         ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
             LENGTHUNIT[\"metre\",1],\n\
             ID[\"EPSG\",7030]],\n\
         ANCHOREPOCH[2002.5]]",
    );
    let datum = object.geodetic_datum().expect("datum");
    assert_eq!(datum.anchor_epoch(), Some(2002.5));
    assert_eq!(datum.anchor(), None);
}

#[test]
fn wkt_parse_datum_with_invalid_anchorepoch() {
    refused(
        "DATUM[\"my_datum\",\n\
         ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
             LENGTHUNIT[\"metre\",1],\n\
             ID[\"EPSG\",7030]],\n\
         ANCHOREPOCH[invalid]]",
    );
}

#[test]
fn wkt_parse_datum_with_invalid_anchorepoch_too_many_children() {
    refused(
        "DATUM[\"my_datum\",\n\
         ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
             LENGTHUNIT[\"metre\",1],\n\
             ID[\"EPSG\",7030]],\n\
         ANCHOREPOCH[2002.5,invalid]]",
    );
}

#[test]
fn wkt_parse_datum_with_pm() {
    let wkt = "DATUM[\"Nouvelle Triangulation Francaise (Paris)\",\n    \
ELLIPSOID[\"Clarke 1880 (IGN)\",6378249.2,293.466021293627,\n        \
LENGTHUNIT[\"metre\",1]],\n    \
ID[\"EPSG\",6807]],\n\
PRIMEM[\"Paris\",2.5969213,\n    \
ANGLEUNIT[\"grad\",0.0157079632679489],\n    \
ID[\"EPSG\",8903]]";
    let datum = parse(wkt).geodetic_datum().expect("datum").clone();
    assert_eq!(datum.prime_meridian().name(), "Paris");
    let exported = datum
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("export");
    assert_eq!(exported, wkt);
}

#[test]
fn wkt_parse_datum_no_pm_not_earth() {
    let datum = parse(
        "DATUM[\"unnamed\",\n\
         ELLIPSOID[\"unnamed\",1,0,\n\
             LENGTHUNIT[\"metre\",1]]]",
    )
    .geodetic_datum()
    .expect("datum")
    .clone();
    assert_eq!(datum.ellipsoid().celestial_body(), "Non-Earth body");
    assert_eq!(datum.prime_meridian().name(), "Reference meridian");
}

#[test]
fn wkt_parse_guess_celestial_body_from_ellipsoid_name() {
    let datum = parse_db(
        "DATUM[\"unnamed\",\n\
         ELLIPSOID[\"Ananke\",10000,0,\n\
             LENGTHUNIT[\"metre\",1]]]",
    )
    .geodetic_datum()
    .expect("datum")
    .clone();
    assert_eq!(datum.ellipsoid().celestial_body(), "Ananke");
}

#[test]
fn wkt_parse_guess_celestial_body_from_ellipsoid_name_false_positive() {
    let datum = parse_db(
        "DATUM[\"unnamed\",\n\
         ELLIPSOID[\"Ananke\",999999,0,\n\
             LENGTHUNIT[\"metre\",1]]]",
    )
    .geodetic_datum()
    .expect("datum")
    .clone();
    assert_eq!(datum.ellipsoid().celestial_body(), "Non-Earth body");
}

#[test]
fn wkt_parse_dynamic_geodetic_reference_frame() {
    let crs = parse(
        "GEOGCRS[\"WGS 84 (G1762)\",\
         DYNAMIC[FRAMEEPOCH[2005.0]],\
         TRF[\"World Geodetic System 1984 (G1762)\",\
             ELLIPSOID[\"WGS 84\",6378137,298.257223563],\
             ANCHOR[\"My anchor\"]],\
         CS[ellipsoidal,3],\
             AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"ellipsoidal height (h)\",up,LENGTHUNIT[\"metre\",1.0]]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.anchor(), Some("My anchor"));
    assert_eq!(datum.frame_reference_epoch(), Some(2005.0));
    assert_eq!(datum.deformation_model(), None);
}

#[test]
fn wkt_parse_dynamic_geodetic_reference_frame_with_model() {
    let datum = parse(
        "GEOGCRS[\"WGS 84 (G1762)\",\
         DYNAMIC[FRAMEEPOCH[2005.0],MODEL[\"my_model\"]],\
         TRF[\"World Geodetic System 1984 (G1762)\",\
             ELLIPSOID[\"WGS 84\",6378137,298.257223563],\
             ANCHOR[\"My anchor\"]],\
         CS[ellipsoidal,3],\
             AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"ellipsoidal height (h)\",up,LENGTHUNIT[\"metre\",1.0]]]",
    )
    .geographic()
    .expect("crs")
    .datum()
    .expect("datum")
    .clone();
    assert_eq!(datum.anchor(), Some("My anchor"));
    assert_eq!(datum.frame_reference_epoch(), Some(2005.0));
    assert_eq!(datum.deformation_model(), Some("my_model"));
}

#[test]
fn wkt_parse_dynamic_geodetic_reference_frame_with_velocitygrid() {
    let datum = parse(
        "GEOGCRS[\"WGS 84 (G1762)\",\
         DYNAMIC[FRAMEEPOCH[2005.0],VELOCITYGRID[\"my_model\"]],\
         TRF[\"World Geodetic System 1984 (G1762)\",\
             ELLIPSOID[\"WGS 84\",6378137,298.257223563],\
             ANCHOR[\"My anchor\"]],\
         CS[ellipsoidal,3],\
             AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"ellipsoidal height (h)\",up,LENGTHUNIT[\"metre\",1.0]]]",
    )
    .geographic()
    .expect("crs")
    .datum()
    .expect("datum")
    .clone();
    assert_eq!(datum.deformation_model(), Some("my_model"));
    assert_eq!(datum.frame_reference_epoch(), Some(2005.0));
}

#[test]
fn wkt_parse_geogcrs_with_ensemble() {
    let crs = parse(
        "GEOGCRS[\"WGS 84\",\
         ENSEMBLE[\"WGS 84 ensemble\",\
             MEMBER[\"WGS 84 (TRANSIT)\"],\
             MEMBER[\"WGS 84 (G730)\"],\
             MEMBER[\"WGS 84 (G834)\"],\
             MEMBER[\"WGS 84 (G1150)\"],\
             MEMBER[\"WGS 84 (G1674)\"],\
             MEMBER[\"WGS 84 (G1762)\"],\
             ELLIPSOID[\"WGS 84\",6378137,298.2572236,LENGTHUNIT[\"metre\",1.0]],\
             ENSEMBLEACCURACY[2]],\
         CS[ellipsoidal,3],\
             AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"ellipsoidal height (h)\",up,LENGTHUNIT[\"metre\",1.0]]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    assert!(crs.datum().is_none());
    assert_eq!(crs.datum_ensemble().expect("ensemble").members().len(), 6);
}

#[test]
fn wkt_parse_invalid_geogcrs_with_ensemble() {
    refused(
        "GEOGCRS[\"WGS 84\",\
         ENSEMBLE[\"WGS 84 ensemble\",\
             MEMBER[\"WGS 84 (TRANSIT)\"],\
             MEMBER[\"WGS 84 (G730)\"],\
             MEMBER[\"WGS 84 (G834)\"],\
             MEMBER[\"WGS 84 (G1150)\"],\
             MEMBER[\"WGS 84 (G1674)\"],\
             MEMBER[\"WGS 84 (G1762)\"],\
             ENSEMBLEACCURACY[2]],\
         CS[ellipsoidal,3],\
             AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"ellipsoidal height (h)\",up,LENGTHUNIT[\"metre\",1.0]]]",
    );
}

#[test]
fn wkt_parse_ensemble_without_members() {
    let crs = parse_db(
        "GEOGCRS[\"WGS 84\",\
            ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
                    LENGTHUNIT[\"metre\",1]],\
                ENSEMBLEACCURACY[2]],\
         CS[ellipsoidal,2],\
             AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    assert!(crs.datum_ensemble().expect("ensemble").members().len() >= 2);
}

#[test]
fn wkt_parse_ensemble_without_members_no_db() {
    refused(
        "GEOGCRS[\"WGS 84\",\
            ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
                    LENGTHUNIT[\"metre\",1]],\
                ENSEMBLEACCURACY[2]],\
         CS[ellipsoidal,2],\
             AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
             AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    );
}

#[test]
fn wkt_parse_ensemble_without_members_unknown_name() {
    let mut parser = WktParser::new();
    parser.attach_database(DatabaseContext::create().expect("proj.db"));
    assert!(parser
        .create_from_wkt(
            "GEOGCRS[\"WGS 84\",\
                ENSEMBLE[\"i do not exist\",\
                    ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
                        LENGTHUNIT[\"metre\",1]],\
                    ENSEMBLEACCURACY[2]],\
             CS[ellipsoidal,2],\
                 AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
                 AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]]]",
        )
        .is_err());
}

enum AxisOrder {
    LatLong,
    LongLat,
}

enum CodeCheck {
    Yes,
    No,
}

fn check_epsg_4326(crs: &proj_rs::GeographicCRS, order: AxisOrder, codes: CodeCheck) {
    if matches!(codes, CodeCheck::Yes) {
        assert_eq!(crs.identifiers().len(), 1);
        assert_eq!(crs.identifiers()[0].code(), "4326");
        assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    }
    assert_eq!(crs.name(), "WGS 84");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    if matches!(order, AxisOrder::LatLong) {
        assert!(axes[0].name() == "Latitude" || axes[0].name() == "Geodetic latitude");
        assert_eq!(axes[0].abbreviation().to_ascii_lowercase(), "lat");
        assert_eq!(axes[0].direction(), "north");
        assert!(axes[1].name() == "Longitude" || axes[1].name() == "Geodetic longitude");
        assert_eq!(axes[1].abbreviation().to_ascii_lowercase(), "lon");
        assert_eq!(axes[1].direction(), "east");
    } else {
        assert_eq!(axes[0].name(), "Longitude");
        assert_eq!(axes[0].abbreviation(), "lon");
        assert_eq!(axes[0].direction(), "east");
        assert_eq!(axes[1].name(), "Latitude");
        assert_eq!(axes[1].abbreviation(), "lat");
        assert_eq!(axes[1].direction(), "north");
    }
    let datum = crs.datum().expect("datum");
    if matches!(codes, CodeCheck::Yes) {
        assert_eq!(datum.identifiers().len(), 1);
        assert_eq!(datum.identifiers()[0].code(), "6326");
        assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    }
    assert_eq!(datum.name(), "World Geodetic System 1984");
    let ellipsoid = datum.ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_137.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
    assert_eq!(ellipsoid.inverse_flattening(), 298.257_223_563);
    if matches!(codes, CodeCheck::Yes) {
        assert_eq!(ellipsoid.identifiers().len(), 1);
        assert_eq!(ellipsoid.identifiers()[0].code(), "7030");
        assert_eq!(ellipsoid.identifiers()[0].code_space(), "EPSG");
    }
    assert_eq!(ellipsoid.name(), "WGS 84");
}

#[test]
fn wkt_parse_wkt1_epsg_4326() {
    let crs = parse(
        "GEOGCS[\"WGS 84\",\
            DATUM[\"WGS_1984\",\
                SPHEROID[\"WGS 84\",6378137,298.257223563,\
                    AUTHORITY[\"EPSG\",\"7030\"]],\
                AUTHORITY[\"EPSG\",\"6326\"]],\
            PRIMEM[\"Greenwich\",0,\
                AUTHORITY[\"EPSG\",\"8901\"]],\
            UNIT[\"degree\",0.0174532925199433,\
                AUTHORITY[\"EPSG\",\"9122\"]],\
            AUTHORITY[\"EPSG\",\"4326\"]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    check_epsg_4326(&crs, AxisOrder::LongLat, CodeCheck::Yes);
}

#[test]
fn wkt_parse_wkt1_epsg_4267() {
    let crs = parse_db(
        "GEOGCS[\"NAD27\",\
            DATUM[\"North_American_Datum_1927\",\
                SPHEROID[\"Clarke 1866\",6378206.4,294.978698213898,\
                    AUTHORITY[\"EPSG\",\"7008\"]],\
                AUTHORITY[\"EPSG\",\"6267\"]],\
            PRIMEM[\"Greenwich\",0,\
                AUTHORITY[\"EPSG\",\"8901\"]],\
            UNIT[\"degree\",0.0174532925199433,\
                AUTHORITY[\"EPSG\",\"9122\"]],\
            AUTHORITY[\"EPSG\",\"4267\"]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "6267");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    assert_eq!(datum.name(), "North American Datum 1927");
}

#[test]
fn wkt_parse_wkt1_epsg_4807_grad_mess() {
    let crs = parse(
        "GEOGCS[\"NTF (Paris)\",\n\
            DATUM[\"Nouvelle_Triangulation_Francaise_Paris\",\n\
                SPHEROID[\"Clarke 1880 (IGN)\",6378249.2,293.466021293627,\n\
                    AUTHORITY[\"EPSG\",\"6807\"]],\n\
                AUTHORITY[\"EPSG\",\"6807\"]],\n\
            PRIMEM[\"Paris\",2.33722917,\n\
                AUTHORITY[\"EPSG\",\"8903\"]],\n\
            UNIT[\"grad\",0.015707963267949,\n\
                AUTHORITY[\"EPSG\",\"9105\"]],\n\
            AXIS[\"latitude\",NORTH],\n\
            AXIS[\"longitude\",EAST],\n\
            AUTHORITY[\"EPSG\",\"4807\"]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    let meridian = crs.datum().expect("datum").prime_meridian();
    assert_eq!(meridian.longitude().unit().name(), "grad");
    assert_eq!(meridian.longitude().value(), 2.596_921_3);
    let _ = UnitOfMeasure::grad();
}

#[test]
fn wkt_parse_wkt2_geodcrs_epsg_4326() {
    let crs = parse(
        "GEODCRS[\"WGS 84\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"latitude\",north],\n\
                AXIS[\"longitude\",east],\n\
                UNIT[\"degree\",0.0174532925199433],\n\
            ID[\"EPSG\",4326]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    check_epsg_4326(&crs, AxisOrder::LatLong, CodeCheck::No);
}

#[test]
fn wkt_parse_wkt2_long_geodeticcrs_epsg_4326() {
    let crs = parse(
        "GEODETICCRS[\"WGS 84\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"latitude\",north],\n\
                AXIS[\"longitude\",east],\n\
                UNIT[\"degree\",0.0174532925199433],\n\
            ID[\"EPSG\",4326]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    check_epsg_4326(&crs, AxisOrder::LatLong, CodeCheck::No);
}

#[test]
fn wkt_parse_wkt2_2019_geogcrs_epsg_4326() {
    let crs = parse(
        "GEOGCRS[\"WGS 84\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"latitude\",north],\n\
                AXIS[\"longitude\",east],\n\
                UNIT[\"degree\",0.0174532925199433],\n\
            ID[\"EPSG\",4326]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    check_epsg_4326(&crs, AxisOrder::LatLong, CodeCheck::No);
}

#[test]
fn wkt_parse_wkt2_2019_long_geographiccrs_epsg_4326() {
    let crs = parse(
        "GEOGRAPHICCRS[\"WGS 84\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"latitude\",north],\n\
                AXIS[\"longitude\",east],\n\
                UNIT[\"degree\",0.0174532925199433],\n\
            ID[\"EPSG\",4326]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    check_epsg_4326(&crs, AxisOrder::LatLong, CodeCheck::No);
}

#[test]
fn wkt_parse_wkt2_geodeticdatum() {
    let crs = parse(
        "GEODCRS[\"WGS 84\",\n\
            GEODETICDATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"(lat)\",north],\n\
                AXIS[\"(lon)\",east],\n\
                UNIT[\"degree\",0.0174532925199433],\n\
            ID[\"EPSG\",4326]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    check_epsg_4326(&crs, AxisOrder::LatLong, CodeCheck::No);
}

#[test]
fn wkt_parse_wkt2_trf() {
    let crs = parse(
        "GEODCRS[\"WGS 84\",\n\
            TRF[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"latitude\",north],\n\
                AXIS[\"longitude\",east],\n\
                UNIT[\"degree\",0.0174532925199433],\n\
            ID[\"EPSG\",4326]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    check_epsg_4326(&crs, AxisOrder::LatLong, CodeCheck::No);
}

#[test]
fn wkt_parse_vertcrs_wkt2() {
    let crs = parse(
        "VERTCRS[\"ODN height\",\n\
            VDATUM[\"Ordnance Datum Newlyn\"],\n\
            CS[vertical,1],\n\
                AXIS[\"gravity-related height (H)\",up,\n\
                    LENGTHUNIT[\"metre\",1]],\n\
            ID[\"EPSG\",5701]]",
    )
    .vertical()
    .expect("vertical")
    .clone();
    assert_eq!(crs.name(), "ODN height");
    assert_eq!(crs.identifiers().len(), 1);
    assert_eq!(crs.identifiers()[0].code(), "5701");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.name(), "Ordnance Datum Newlyn");
    let axis = crs.coordinate_system().axis();
    assert_eq!(axis.name(), "Gravity-related height");
    assert_eq!(axis.abbreviation(), "H");
    assert_eq!(axis.direction(), "up");
}

#[test]
fn wkt_parse_engineering_crs_wkt2_affine_cs() {
    let wkt = "ENGCRS[\"Engineering CRS\",\n    \
EDATUM[\"Engineering datum\"],\n    \
CS[affine,2],\n        \
AXIS[\"(E)\",east,\n            \
ORDER[1],\n            \
LENGTHUNIT[\"metre\",1,\n                \
ID[\"EPSG\",9001]]],\n        \
AXIS[\"(N)\",north,\n            \
ORDER[2],\n            \
LENGTHUNIT[\"metre\",1,\n                \
ID[\"EPSG\",9001]]]]";
    let crs = parse(wkt).crs().expect("crs").clone();
    assert_eq!(wkt2_2015(&crs), wkt);
}

fn wkt2_2015(crs: &proj_rs::Crs) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("export")
}

#[test]
fn io_guess_dialect() {
    assert_eq!(
        WktParser::guess_dialect("LOCAL_CS[\"foo\"]"),
        WktDialect::Wkt1Gdal
    );
    assert_eq!(
        WktParser::guess_dialect(
            "GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]"
        ),
        WktDialect::Wkt1Esri
    );
    assert_eq!(
        WktParser::guess_dialect(
            "GEOGCRS[\"WGS 84\",\n\
                DATUM[\"World Geodetic System 1984\",\n\
                    ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
                CS[ellipsoidal,2],\n\
                    AXIS[\"geodetic latitude (Lat)\",north],\n\
                    AXIS[\"geodetic longitude (Lon)\",east],\n\
                    UNIT[\"degree\",0.0174532925199433]]"
        ),
        WktDialect::Wkt2_2019
    );
    assert_eq!(
        WktParser::guess_dialect(
            "TIMECRS[\"Temporal CRS\",\n\
                TDATUM[\"Gregorian calendar\",\n\
                    CALENDAR[\"proleptic Gregorian\"],\n\
                    TIMEORIGIN[0000-01-01]],\n\
                CS[TemporalDateTime,1],\n\
                    AXIS[\"time (T)\",future]]"
        ),
        WktDialect::Wkt2_2019
    );
    assert_eq!(
        WktParser::guess_dialect(
            "GEODCRS[\"WGS 84\",\n\
                DATUM[\"World Geodetic System 1984\",\n\
                    ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
                CS[ellipsoidal,2],\n\
                    AXIS[\"geodetic latitude (Lat)\",north],\n\
                    AXIS[\"geodetic longitude (Lon)\",east],\n\
                    UNIT[\"degree\",0.0174532925199433]]"
        ),
        WktDialect::Wkt2_2015
    );
    assert_eq!(WktParser::guess_dialect("foo"), WktDialect::NotWkt);
    assert_eq!(WktParser::guess_dialect("ID74"), WktDialect::NotWkt);
}

#[test]
fn wkt_parse_wkt1_geographic_missing_unit_and_axis() {
    let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]]]]";
    refused(wkt);
    let crs = parse_loose(wkt).geographic().expect("crs").clone();
    assert_eq!(crs.coordinate_system().axes().len(), 2);
    assert_eq!(
        crs.coordinate_system().axes()[0].unit().name(),
        UnitOfMeasure::degree().name()
    );
}

#[test]
fn wkt_parse_esri_geogcs_datum_spheroid_name_hardcoded_substitution() {
    let crs = parse(
        "GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",\
         SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
         PRIMEM[\"Greenwich\",0.0],\
         UNIT[\"Degree\",0.0174532925199433]]",
    )
    .geographic()
    .expect("crs")
    .clone();
    assert_eq!(crs.name(), "WGS 84");
    assert_eq!(
        crs.datum().expect("datum").name(),
        "World Geodetic System 1984"
    );
    assert_eq!(crs.ellipsoid().expect("ellipsoid").name(), "WGS 84");
}

#[test]
fn wkt_parse_invalid() {
    refused("");
    refused("A");
    refused("UNKNOWN[\"foo\"]");
    refused("INVALID[");
    refused("INVALID[]");
}

#[test]
fn wkt_parse_invalid_spheroid() {
    parse("SPHEROID[\"x\",1,0.5]");
    refused("SPHEROID[\"x\"]");
    refused("SPHEROID[\"x\",\"1\",0.5]");
    refused("SPHEROID[\"x\",1,\"0.5\"]");
}

#[test]
fn wkt_parse_invalid_datum() {
    parse("DATUM[\"x\",SPHEROID[\"x\",1,0.5]]");
    refused("DATUM[\"x\"]");
    refused("DATUM[\"x\",FOO[]]");
}

#[test]
fn wkt_parse_invalid_ensemble() {
    refused("ENSEMBLE[]");
    refused("ENSEMBLE[\"x\"]");
    refused("ENSEMBLE[\"x\",MEMBER[\"vdatum1\"],MEMBER[\"vdatum2\"]]");
    refused("ENSEMBLE[\"x\",MEMBER[],MEMBER[\"vdatum2\"],ENSEMBLEACCURACY[\"100\"]]");
    refused("ENSEMBLE[\"x\",MEMBER[\"vdatum1\"],MEMBER[\"vdatum2\"],ENSEMBLEACCURACY[]]");
    refused("ENSEMBLE[\"x\",ENSEMBLEACCURACY[\"100\"]]");
}

const BBOX_CRS_HEAD: &str = "GEOGCRS[\"x\",DATUM[\"x\",ELLIPSOID[\"x\",1,0.5,\
LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]]],\
PRIMEM[\"x\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],\
CS[ellipsoidal,2],AXIS[\"longitude\",east,ORDER[1],\
ANGLEUNIT[\"degree\",0.0174532925199433]],\
AXIS[\"latitude\",north,ORDER[2],\
ANGLEUNIT[\"degree\",0.0174532925199433]],";

#[test]
fn wkt_parse_invalid_bbox() {
    parse(&format!(
        "{BBOX_CRS_HEAD}USAGE[SCOPE[\"unknown\"],BBOX[1,2,3,4]]]"
    ));
    refused(&format!(
        "{BBOX_CRS_HEAD}USAGE[SCOPE[\"unknown\"],BBOX[1,2,3]]]"
    ));
    refused(&format!(
        "{BBOX_CRS_HEAD}USAGE[SCOPE[\"unknown\"],BBOX[1,2,3,a]]]"
    ));
    refused(&format!(
        "{BBOX_CRS_HEAD}USAGE[SCOPE[\"unknown\"],BBOX[1,2,-1,4]]]"
    ));
}

#[test]
fn wkt_parse_invalid_geogcs() {
    let datum = "DATUM[\"x\",SPHEROID[\"x\",1,0.5]]";
    let unit = "UNIT[\"degree\",0.0174532925199433]";
    parse(&format!("GEOGCS[\"x\",{datum},PRIMEM[\"x\",0],{unit}]"));

    refused(&format!("GEOGCS[\"x\",{datum},{unit}]"));
    refused(&format!("GEOGCS[\"x\",{datum},PRIMEM[\"x\",0]]"));
    refused("GEOGCS[\"x\"]");
    refused("GEOGCS[\"x\",FOO[]]");
    refused(&format!(
        "GEOGCS[\"x\",DATUM[\"x\"],PRIMEM[\"x\",0],{unit}]"
    ));
    refused(&format!(
        "GEOGCS[\"x\",{datum},PRIMEM[\"x\",0],{unit},AUTHORITY[\"x\"]]"
    ));

    let mut lenient = WktParser::new();
    lenient.set_strictness(proj_rs::WktStrictness::Lenient);
    lenient
        .create_from_wkt(&format!(
            "GEOGCS[\"x\",{datum},PRIMEM[\"x\",0],{unit},AUTHORITY[\"x\"]]"
        ))
        .expect("AUTHORITY is ignored when not strict");

    refused(&format!("GEOGCS[\"x\",{datum},PRIMEM[\"x\",\"a\"],{unit}]"));
    refused(&format!("GEOGCS[\"x\",{datum},PRIMEM[\"x\"],{unit}]"));

    parse(&format!(
        "GEOGCS[\"x\",{datum},PRIMEM[\"x\",0],{unit},AXIS[\"latitude\",NORTH],AXIS[\"longitude\",EAST]]"
    ));

    refused(&format!(
        "GEOGCS[\"x\",{datum},PRIMEM[\"x\",0],{unit},AXIS[\"latitude\",NORTH]]"
    ));
    refused(&format!(
        "GEOGCS[\"x\",{datum},PRIMEM[\"x\",0],{unit},AXIS[\"latitude\",NORTH],AXIS[\"longitude\"]]"
    ));
}

#[test]
fn wkt_parse_invalid_unit() {
    let start = "GEODCRS[\"x\",DATUM[\"x\",SPHEROID[\"x\",1,0.5]],CS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east],";
    parse(&format!("{start}UNIT[\"degree\",0.0174532925199433]]"));
    refused(&format!("{start}UNIT[\"x\"]]]"));
    refused(&format!("{start}UNIT[\"x\",\"invalid\"]]]"));
    refused(&format!("{start}UNIT[\"x\",1,ID[\"x\"]]]]"));
}

#[test]
fn wkt_parse_invalid_geoccs() {
    let datum = "DATUM[\"x\",SPHEROID[\"x\",1,0.5]]";
    parse(&format!(
        "GEOCCS[\"x\",{datum},PRIMEM[\"x\",0],UNIT[\"metre\",1]]"
    ));

    refused(&format!("GEOCCS[\"x\",{datum},UNIT[\"metre\",1]]"));
    refused(&format!("GEOCCS[\"x\",{datum},PRIMEM[\"x\",0]]"));
    refused(&format!(
        "GEOCCS[\"x\",{datum},PRIMEM[\"x\",0],UNIT[\"degree\",0.0174532925199433],AXIS[\"latitude\",NORTH],AXIS[\"longitude\",EAST]]"
    ));
    refused(
        "GEOCCS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",\
ELLIPSOID[\"WGS 84\",6378274,298.257223564,LENGTHUNIT[\"metre\",1]]],\
CS[ellipsoidal,2],AXIS[\"geodetic latitude (Lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
AXIS[\"geodetic longitude (Lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    );
    refused(&format!(
        "GEOCCS[\"x\",{datum},PRIMEM[\"x\",0],UNIT[\"metre\",1],AXIS[\"Geocentric X\",OTHER],AXIS[\"Geocentric Y\",OTHER]]"
    ));
}

#[test]
fn wkt_parse_invalid_cs_of_geodcrs() {
    let start = "GEODCRS[\"x\",DATUM[\"x\",SPHEROID[\"x\",1,0.5]]";
    refused(&format!("{start}]"));
    refused(&format!("{start},CS[x]]"));
    refused(&format!("{start},CS[x,2]]"));
    refused(&format!("{start},CS[ellipsoidal,0]]"));
    refused(&format!("{start},CS[ellipsoidal,\"x\"]]"));
    refused(&format!(
        "{start},CS[invalid,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east]]"
    ));
    parse(&format!(
        "{start},CS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east],UNIT[\"degree\",0.0174532925199433]]"
    ));
    refused(&format!(
        "{start},CS[Cartesian,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east],UNIT[\"degree\",0.0174532925199433]]"
    ));
    refused(&format!(
        "{start},CS[ellipsoidal,2],AXIS[\"latitude\",north],UNIT[\"degree\",0.0174532925199433]]"
    ));
    refused(&format!(
        "{start},CS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\"],UNIT[\"degree\",0.0174532925199433]]"
    ));
    refused(&format!(
        "{start},CS[ellipsoidal,2],AXIS[\"latitude\",north,ORDER[]],AXIS[\"longitude\",east],UNIT[\"degree\",0.0174532925199433]]"
    ));
    refused(&format!(
        "{start},CS[ellipsoidal,2],AXIS[\"latitude\",north,ORDER[\"x\"]],AXIS[\"longitude\",east],UNIT[\"degree\",0.0174532925199433]]"
    ));
    refused(&format!(
        "{start},CS[ellipsoidal,2],AXIS[\"latitude\",north,ORDER[2]],AXIS[\"longitude\",east],UNIT[\"degree\",0.0174532925199433]]"
    ));
    refused(&format!(
        "{start},CS[vertical,1],\n        AXIS[\"gravity-related height (H)\",up],\n        UNIT[\"metre\",1]]"
    ));
}

#[test]
fn wkt_parse_invalid_cs_of_geographiccrs() {
    refused(
        "GEOGRAPHICCRS[\"x\",DATUM[\"x\",SPHEROID[\"x\",1,0.5]],\
CS[Cartesian,3],AXIS[\"(X)\",geocentricX],AXIS[\"(Y)\",geocentricY],AXIS[\"(Z)\",geocentricZ]]",
    );
}

#[test]
fn wkt_parse_invalid_dynamic() {
    let prefix = "GEOGCRS[\"WGS 84 (G1762)\",";
    let suffix = "TRF[\"World Geodetic System 1984 (G1762)\",\
ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\
CS[ellipsoidal,3],\
AXIS[\"(lat)\",north,ANGLEUNIT[\"degree\",0.0174532925199433]],\
AXIS[\"(lon)\",east,ANGLEUNIT[\"degree\",0.0174532925199433]],\
AXIS[\"ellipsoidal height (h)\",up,LENGTHUNIT[\"metre\",1.0]]]";
    parse(&format!("{prefix}DYNAMIC[FRAMEEPOCH[2015]],{suffix}"));
    refused(&format!("{prefix}DYNAMIC[],{suffix}"));
    refused(&format!("{prefix}DYNAMIC[FRAMEEPOCH[]],{suffix}"));
    refused(&format!(
        "{prefix}DYNAMIC[FRAMEEPOCH[\"invalid\"]],{suffix}"
    ));
}

#[test]
fn wkt_parse_invalid_projcrs() {
    refused("PROJCRS[\"WGS 84 / UTM zone 31N\"]");

    let start = "PROJCRS[\"WGS 84 / UTM zone 31N\",BASEGEOGCRS[\"WGS 84\",DATUM[\"WGS_1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],UNIT[\"degree\",0.0174532925199433]]";
    refused(&format!("{start}]"));
    refused(&format!("{start},CONVERSION[\"x\"]]"));
    refused(&format!("{start},CONVERSION[\"x\",METHOD[]]]"));
    refused(&format!(
        "{start},CONVERSION[\"x\",METHOD[\"y\"],PARAMETER[\"z\"]]]"
    ));
    refused(&format!(
        "{start},CONVERSION[\"x\",METHOD[\"y\"],PARAMETER[\"z\",\"foo\"]]]"
    ));
    refused(&format!("{start},CONVERSION[\"x\",METHOD[\"y\"]]]"));
    refused(&format!(
        "{start},CONVERSION[\"x\",METHOD[\"y\"]],CS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east]]"
    ));
    refused(&format!(
        "{start},CONVERSION[\"x\",METHOD[\"y\"]],CS[Cartesian,2],AXIS[\"easting (X)\",south,MERIDIAN[90]],AXIS[\"northing (Y)\",south]]"
    ));
    refused(&format!(
        "{start},CONVERSION[\"x\",METHOD[\"y\"]],CS[Cartesian,2],AXIS[\"easting (X)\",south,MERIDIAN[\"x\",UNIT[\"degree\",0.0174532925199433]]],AXIS[\"northing (Y)\",south]]"
    ));
}

#[test]
fn wkt_parse_invalid_projcs() {
    let start = "PROJCS[\"WGS 84 / UTM zone 31N\",\n\
    GEOGCS[\"WGS 84\",\n\
        DATUM[\"WGS_1984\",\n\
            SPHEROID[\"WGS 84\",6378137,298.257223563,\n\
                AUTHORITY[\"EPSG\",\"7030\"]],\n\
            AUTHORITY[\"EPSG\",\"6326\"]],\n\
        PRIMEM[\"x\",0],\n\
        UNIT[\"degree\",0.0174532925199433,\n\
            AUTHORITY[\"EPSG\",\"9122\"]],\n\
        AXIS[\"latitude\",NORTH],\n\
        AXIS[\"longitude\",EAST],\n\
        AUTHORITY[\"EPSG\",\"4326\"]]\n";
    refused(&format!("{start}]"));
    refused(&format!("{start},PROJECTION[],UNIT[\"metre\",1]]"));
    refused(&format!(
        "{start},PROJECTION[\"x\"],PARAMETER[\"z\"],UNIT[\"metre\",1]]"
    ));
    parse(&format!(
        "{start},PROJECTION[\"x\"],PARAMETER[\"z\",1],UNIT[\"metre\",1]]"
    ));
    refused(&format!("{start},PROJECTION[\"x\"],PARAMETER[\"z\",1]]"));
}

#[test]
fn wkt_parse_invalid_vertcrs() {
    refused("VERTCRS[\"foo\",CS[vertical,1],AXIS[\"x\",up]]");
    refused("VERTCRS[\"foo\",VDATUM[\"bar\"]]");
    refused(
        "VERTCRS[\"foo\",VDATUM[\"bar\"],CS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east]]",
    );
    refused(
        "VERTCRS[\"foo\",VDATUM[\"bar\"],CS[vertical,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east]]",
    );
}

#[test]
fn wkt_parse_invalid_esri_vertcs() {
    refused(
        "GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
VERTCS[\"EGM96_Geoid\",VDATUM,PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]",
    );
}

#[test]
fn wkt_parse_invalid_vert_cs() {
    parse("VERT_CS[\"x\",VERT_DATUM[\"y\",2005],UNIT[\"metre\",1]]");
    refused("VERT_CS[\"x\",UNIT[\"metre\",1]]");
    refused("VERT_CS[\"x\",VERT_DATUM[\"y\",2005]]");
}

fn wkt2_of(crs: &proj_rs::Crs) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt")
}

fn wkt2_without_ids(crs: &proj_rs::Crs) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.hide_identifiers().expect("hide identifiers");
    crs.export_to_wkt(&mut formatter).expect("wkt")
}

#[test]
fn wkt_parse_invalid_coordinateoperation() {
    let src = wkt2_without_ids(&proj_rs::Crs::Geographic(
        proj_rs::GeographicCRS::epsg_4326(),
    ));
    let dst = wkt2_without_ids(&proj_rs::Crs::Geographic(
        proj_rs::GeographicCRS::epsg_4807(),
    ));
    let tail = "METHOD[\"operationMethodName\"],PARAMETERFILE[\"paramName\",\"foo.bin\"]]";
    parse(&format!(
        "COORDINATEOPERATION[\"transformationName\",SOURCECRS[{src}],TARGETCRS[{dst}],{tail}"
    ));
    refused(&format!(
        "COORDINATEOPERATION[\"transformationName\",TARGETCRS[{dst}],{tail}"
    ));
    refused(&format!(
        "COORDINATEOPERATION[\"transformationName\",SOURCECRS[FOO],TARGETCRS[{dst}],{tail}"
    ));
    refused(&format!(
        "COORDINATEOPERATION[\"transformationName\",SOURCECRS[{src}],{tail}"
    ));
    refused(&format!(
        "COORDINATEOPERATION[\"transformationName\",SOURCECRS[{src}],TARGETCRS[FOO],{tail}"
    ));
    refused(&format!(
        "COORDINATEOPERATION[\"transformationName\",SOURCECRS[{src}],TARGETCRS[{dst}]]"
    ));
    refused(&format!(
        "COORDINATEOPERATION[\"transformationName\",SOURCECRS[{src}],TARGETCRS[{dst}],METHOD[],PARAMETERFILE[\"paramName\",\"foo.bin\"]]"
    ));
}

#[test]
fn wkt_parse_invalid_concatenatedoperation() {
    refused("CONCATENATEDOPERATION[\"name\"]");

    let src = wkt2_of(&proj_rs::Crs::Geographic(
        proj_rs::GeographicCRS::epsg_4326(),
    ));
    let dst = wkt2_of(&proj_rs::Crs::Geographic(
        proj_rs::GeographicCRS::epsg_4807(),
    ));
    let step = format!(
        "COORDINATEOPERATION[\"transf_1\",SOURCECRS[{src}],TARGETCRS[{dst}],METHOD[\"operationMethodName\"],PARAMETERFILE[\"paramName\",\"foo.bin\"]]"
    );
    let tail = "ID[\"codeSpace\",\"code\"],REMARK[\"my remarks\"]]";
    refused(&format!(
        "CONCATENATEDOPERATION[\"name\",SOURCECRS[{src}],TARGETCRS[{dst}],STEP[{step}],{tail}"
    ));
    refused(&format!(
        "CONCATENATEDOPERATION[\"name\",SOURCECRS[{src}],TARGETCRS[{dst}],STEP[],STEP[],{tail}"
    ));
    refused(&format!(
        "CONCATENATEDOPERATION[\"name\",SOURCECRS[{src}],TARGETCRS[{dst}],STEP[SOURCECRS[{src}]],STEP[SOURCECRS[{src}]],{tail}"
    ));
}

#[test]
fn wkt_parse_invalid_boundcrs() {
    let projcrs = proj_rs::ProjectedCRS::create(
        "my PROJCRS",
        proj_rs::GeodeticCRS::Geographic(
            proj_rs::GeographicCRS::create(
                "my GEOGCRS",
                Some(proj_rs::GeodeticReferenceFrame::epsg_6326()),
                None,
                proj_rs::EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
            )
            .expect("geographic"),
        ),
        proj_rs::Conversion::create_utm_north(&proj_rs::OpProps::new(), 31).expect("utm"),
        proj_rs::CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let proj = wkt2_of(&proj_rs::Crs::Projected(projcrs));
    let geog = wkt2_of(&proj_rs::Crs::Geographic(
        proj_rs::GeographicCRS::epsg_4326(),
    ));
    let transf = "ABRIDGEDTRANSFORMATION[\"foo\",METHOD[\"bar\"],PARAMETER[\"foo\",1.0]]]";
    parse(&format!(
        "BOUNDCRS[SOURCECRS[{proj}],\nTARGETCRS[{geog}],\n{transf}"
    ));
    refused(&format!("BOUNDCRS[TARGETCRS[{geog}],\n{transf}"));
    refused(&format!(
        "BOUNDCRS[SOURCECRS[foo], TARGETCRS[{geog}],\n{transf}"
    ));
    refused(&format!(
        "BOUNDCRS[SOURCECRS[{proj}],TARGETCRS[\"foo\"],\n{transf}"
    ));
    refused(&format!("BOUNDCRS[SOURCECRS[{proj}],\nTARGETCRS[{geog}]]"));
    refused(&format!(
        "BOUNDCRS[SOURCECRS[{proj}],\nTARGETCRS[{geog}],ABRIDGEDTRANSFORMATION[\"foo\"],PARAMETER[\"foo\",1.0]]"
    ));
    refused(&format!(
        "BOUNDCRS[SOURCECRS[{proj}],\nTARGETCRS[{geog}],ABRIDGEDTRANSFORMATION[\"foo\",METHOD[],PARAMETER[\"foo\",1.0]]]"
    ));
}

#[test]
fn wkt_parse_invalid_towgs84() {
    refused(
        "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563],TOWGS84[0]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]",
    );
    refused(
        "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563],TOWGS84[0,0,0,0,0,0,\"foo\"]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]",
    );
}

#[test]
fn wkt_parse_invalid_local_cs() {
    refused(
        "LOCAL_CS[\"name\",LOCAL_DATUM[\"name\",1234],AXIS[\"Geodetic latitude\",NORTH],AXIS[\"Geodetic longitude\",EAST],AXIS[\"Ellipsoidal height\",UP]]",
    );
}

// `PROJStringParser().createFromPROJString` for `+proj=longlat`, exported to WKT2.

fn proj_crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("PROJ string")
}

fn proj_geographic(definition: &str) -> GeographicCRS {
    match proj_crs(definition) {
        Crs::Geographic(crs) => crs,
        _ => panic!("not a geographic CRS: {definition}"),
    }
}

fn proj_wkt2(definition: &str) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    proj_crs(definition)
        .export_to_wkt(&mut formatter)
        .expect("WKT2")
}

fn proj_wkt2_simul(definition: &str) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.simul_cur_node_has_id();
    proj_crs(definition)
        .export_to_wkt(&mut formatter)
        .expect("WKT2")
}

fn lines(parts: &[&str]) -> String {
    parts.join("\n")
}

#[test]
fn io_projparse_longlat() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"World Geodetic System 1984\",",
        "        ELLIPSOID[\"WGS 84\",6378137,298.257223563,",
        "            LENGTHUNIT[\"metre\",1]],",
        "        ID[\"EPSG\",6326]],",
        "    PRIMEM[\"Greenwich\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433],",
        "        ID[\"EPSG\",8901]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]]]",
    ]);
    for definition in [
        "+proj=longlat +type=crs",
        "+proj=longlat +datum=WGS84 +type=crs",
    ] {
        assert_eq!(proj_wkt2(definition), expected, "{definition}");
    }
}

#[test]
fn io_projparse_longlat_datum_nad83() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"North American Datum 1983\",",
        "        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,",
        "            LENGTHUNIT[\"metre\",1]],",
        "        ID[\"EPSG\",6269]],",
        "    PRIMEM[\"Greenwich\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433],",
        "        ID[\"EPSG\",8901]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]]]",
    ]);
    assert_eq!(proj_wkt2("+proj=longlat +datum=NAD83 +type=crs"), expected);
}

#[test]
fn io_projparse_longlat_datum_nad27() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"North American Datum 1927\",",
        "        ELLIPSOID[\"Clarke 1866\",6378206.4,294.978698213898,",
        "            LENGTHUNIT[\"metre\",1]],",
        "        ID[\"EPSG\",6267]],",
        "    PRIMEM[\"Greenwich\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433],",
        "        ID[\"EPSG\",8901]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]]]",
    ]);
    assert_eq!(proj_wkt2("+proj=longlat +datum=NAD27 +type=crs"), expected);
}

#[test]
fn io_projparse_longlat_datum_other() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"Carthage\",",
        "        ELLIPSOID[\"Clarke 1880 (IGN)\",6378249.2,293.4660213,",
        "            LENGTHUNIT[\"metre\",1]],",
        "        ID[\"EPSG\",6223]],",
        "    PRIMEM[\"Greenwich\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433],",
        "        ID[\"EPSG\",8901]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433,",
        "                ID[\"EPSG\",9122]]]]",
    ]);
    assert_eq!(
        proj_wkt2("+proj=longlat +datum=carthage +type=crs"),
        expected
    );
}

#[test]
fn io_projparse_longlat_ellps_wgs84() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"Unknown based on WGS 84 ellipsoid\",",
        "        ELLIPSOID[\"WGS 84\",6378137,298.257223563,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Greenwich\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +ellps=WGS84 +type=crs"),
        expected
    );
}

#[test]
fn io_projparse_longlat_ellps_grs80() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"Unknown based on GRS 1980 ellipsoid\",",
        "        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Greenwich\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +ellps=GRS80 +type=crs"),
        expected
    );
}

#[test]
fn io_projparse_longlat_a_b() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"unknown\",",
        "        ELLIPSOID[\"unknown\",2,4,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Reference meridian\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +a=2 +b=1.5 +type=crs"),
        expected
    );
    let crs = proj_geographic("+proj=longlat +a=2 +b=1.5 +type=crs");
    let ellipsoid = crs.datum().expect("datum").ellipsoid();
    assert_eq!(ellipsoid.celestial_body(), "Non-Earth body");
}

#[test]
fn io_projparse_longlat_a_rf_wgs84() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"Unknown based on WGS 84 ellipsoid\",",
        "        ELLIPSOID[\"WGS 84\",6378137,298.257223563,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Greenwich\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +a=6378137 +rf=298.257223563 +type=crs"),
        expected
    );
    let crs = proj_geographic("+proj=longlat +a=6378137 +rf=298.257223563 +type=crs");
    assert_eq!(
        crs.datum().expect("datum").ellipsoid().celestial_body(),
        "Earth"
    );
}

#[test]
fn io_projparse_longlat_a_rf() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"unknown\",",
        "        ELLIPSOID[\"unknown\",2,4,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Reference meridian\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +a=2 +rf=4 +type=crs"),
        expected
    );
}

#[test]
fn io_projparse_longlat_a_f_zero() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"unknown\",",
        "        ELLIPSOID[\"unknown\",2,0,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Reference meridian\",0,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +a=2 +f=0 +type=crs"),
        expected
    );
}

#[test]
fn io_projparse_longlat_a_f_non_zero() {
    let crs = proj_geographic("+proj=longlat +a=2 +f=0.5 +type=crs");
    let ellipsoid = crs.datum().expect("datum").ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
    assert_eq!(ellipsoid.inverse_flattening(), 2.0);
}

#[test]
fn io_projparse_longlat_a_e() {
    let crs = proj_geographic("+proj=longlat +a=2 +e=0.5 +type=crs");
    let ellipsoid = crs.datum().expect("datum").ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
    let rf = ellipsoid.inverse_flattening();
    assert!((rf - 7.46410161513775).abs() < 1e-14, "{rf}");
}

#[test]
fn io_projparse_longlat_a_es() {
    let crs = proj_geographic("+proj=longlat +a=2 +es=0.5 +type=crs");
    let ellipsoid = crs.datum().expect("datum").ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
    let rf = ellipsoid.inverse_flattening();
    assert!((rf - 3.414213562373096).abs() < 1e-14, "{rf}");
}

#[test]
fn io_projparse_longlat_r() {
    let crs = proj_geographic("+proj=longlat +R=2 +type=crs");
    let ellipsoid = crs.datum().expect("datum").ellipsoid();
    assert!(ellipsoid.is_sphere());
    assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
}

#[test]
fn io_projparse_longlat_a() {
    let crs = proj_geographic("+proj=longlat +a=2 +type=crs");
    let ellipsoid = crs.datum().expect("datum").ellipsoid();
    assert!(ellipsoid.is_sphere());
    assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
}

#[test]
fn io_projparse_longlat_a_override_ellps() {
    let crs = proj_geographic("+proj=longlat +a=2 +ellps=WGS84 +type=crs");
    let ellipsoid = crs.datum().expect("datum").ellipsoid();
    assert!(!ellipsoid.is_sphere());
    assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
    assert_eq!(ellipsoid.inverse_flattening(), 298.257223563);
}

#[test]
fn io_projparse_longlat_pm_paris() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"Unknown based on WGS 84 ellipsoid\",",
        "        ELLIPSOID[\"WGS 84\",6378137,298.257223563,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Paris\",2.5969213,",
        "        ANGLEUNIT[\"grad\",0.015707963267949]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +pm=paris +type=crs"),
        expected
    );
}

#[test]
fn io_projparse_longlat_pm_ferro() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"Unknown based on Bessel 1841 ellipsoid\",",
        "        ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"Ferro\",-17.6666666666667,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(
        proj_wkt2_simul("+proj=longlat +ellps=bessel +pm=ferro +type=crs"),
        expected
    );
}

#[test]
fn io_projparse_longlat_pm_numeric() {
    let expected = lines(&[
        "GEODCRS[\"unknown\",",
        "    DATUM[\"Unknown based on WGS 84 ellipsoid\",",
        "        ELLIPSOID[\"WGS 84\",6378137,298.257223563,",
        "            LENGTHUNIT[\"metre\",1]]],",
        "    PRIMEM[\"unknown\",2.5,",
        "        ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "    CS[ellipsoidal,2],",
        "        AXIS[\"longitude\",east,",
        "            ORDER[1],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "        AXIS[\"latitude\",north,",
        "            ORDER[2],",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
    ]);
    assert_eq!(proj_wkt2_simul("+proj=longlat +pm=2.5 +type=crs"), expected);
}

#[test]
fn io_projparse_longlat_pm_overriding_datum() {
    // The ellipsoid of a datum named by its code is kept, and the given prime
    // meridian replaces the datum's own. This is a GDAL regression case.
    let crs = proj_geographic("+proj=longlat +datum=WGS84 +pm=ferro +type=crs");
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.name(), "Unknown based on WGS 84 ellipsoid");
    assert_eq!(datum.prime_meridian().name(), "Ferro");
}

// `WKTParser` on WKT1 `PROJCS` with a single projection: the PROJ string it
// exports, and the axis names and directions it gets.

const WKT1_WGS84_GEOGCS: &str = "GEOGCS[\"unknown\",\
    DATUM[\"WGS_1984\",\
        SPHEROID[\"WGS 84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],\
        AUTHORITY[\"EPSG\",\"6326\"]],\
    PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],\
    UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]]]";

fn wkt1_polar_stereographic(parameters: &str) -> String {
    format!(
        "PROJCS[\"unknown\",{WKT1_WGS84_GEOGCS},\
         PROJECTION[\"Polar_Stereographic\"],{parameters},\
         UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]]]"
    )
}

fn projected_from_wkt1(wkt: &str) -> proj_rs::ProjectedCRS {
    parse(wkt).projected().expect("crs").clone()
}

fn projected_proj_string(crs: &proj_rs::ProjectedCRS) -> String {
    crs.export_to_proj_string().expect("PROJ string")
}

#[test]
fn wkt_parse_wkt1_mercator_1sp_with_latitude_origin_0() {
    let crs = projected_from_wkt1(
        "PROJCS[\"unnamed\",\
            GEOGCS[\"WGS 84\",\
                DATUM[\"unknown\",SPHEROID[\"WGS84\",6378137,298.257223563]],\
                PRIMEM[\"Greenwich\",0],\
                UNIT[\"degree\",0.0174532925199433]],\
            PROJECTION[\"Mercator_1SP\"],\
            PARAMETER[\"latitude_of_origin\",0],\
            PARAMETER[\"central_meridian\",0],\
            PARAMETER[\"scale_factor\",1],\
            PARAMETER[\"false_easting\",0],\
            PARAMETER[\"false_northing\",0],\
            UNIT[\"Meter\",1],\
            AXIS[\"Easting\",EAST],\
            AXIS[\"Northing\",NORTH]]",
    );
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    let wkt = Crs::Projected(crs)
        .export_to_wkt(&mut formatter)
        .expect("WKT1");
    assert!(wkt.contains("Mercator_1SP"), "{wkt}");
}

#[test]
fn wkt_parse_wkt1_mercator_1sp_without_scale_factor() {
    // See https://github.com/OSGeo/PROJ/issues/1700
    let wkt = "PROJCS[\"unnamed\",\
        GEOGCS[\"WGS 84\",\
            DATUM[\"unknown\",SPHEROID[\"WGS84\",6378137,298.257223563]],\
            PRIMEM[\"Greenwich\",0],\
            UNIT[\"degree\",0.0174532925199433]],\
        PROJECTION[\"Mercator_1SP\"],\
        PARAMETER[\"central_meridian\",0],\
        PARAMETER[\"false_easting\",0],\
        PARAMETER[\"false_northing\",0],\
        UNIT[\"Meter\",1],\
        AXIS[\"Easting\",EAST],\
        AXIS[\"Northing\",NORTH]]";
    let mut parser = WktParser::new();
    parser
        .set_strictness(proj_rs::WktStrictness::Lenient)
        .attach_database(DatabaseContext::create().expect("proj.db"));
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    assert!(!parser.warnings().is_empty());

    let crs = obj.projected().expect("crs").clone();
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    let got = Crs::Projected(crs.clone())
        .export_to_wkt(&mut formatter)
        .expect("WKT1");
    assert!(got.contains("PARAMETER[\"scale_factor\",1]"), "{got}");
    assert_eq!(
        projected_proj_string(&crs),
        "+proj=merc +lon_0=0 +k=1 +x_0=0 +y_0=0 +ellps=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn wkt_parse_wkt1_polar_stereographic_latitude_of_origin_70() {
    let crs = projected_from_wkt1(&wkt1_polar_stereographic(
        "PARAMETER[\"latitude_of_origin\",70],\
         PARAMETER[\"central_meridian\",2],\
         PARAMETER[\"false_easting\",3],\
         PARAMETER[\"false_northing\",4],",
    ));
    assert_eq!(
        projected_proj_string(&crs),
        "+proj=stere +lat_0=90 +lat_ts=70 +lon_0=2 +x_0=3 +y_0=4 +datum=WGS84 +units=m +no_defs +type=crs"
    );

    let axes = crs.coordinate_system().axes();
    assert_eq!(axes[0].name(), "Easting");
    assert_eq!(axes[0].direction(), "south");
    assert_eq!(axes[1].name(), "Northing");
    assert_eq!(axes[1].direction(), "south");
}

#[test]
fn wkt_parse_wkt1_polar_stereographic_latitude_of_origin_minus_70() {
    let crs = projected_from_wkt1(&wkt1_polar_stereographic(
        "PARAMETER[\"latitude_of_origin\",-70],\
         PARAMETER[\"central_meridian\",2],\
         PARAMETER[\"false_easting\",3],\
         PARAMETER[\"false_northing\",4],",
    ));
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes[0].name(), "Easting");
    assert_eq!(axes[0].direction(), "north");
    assert_eq!(axes[1].name(), "Northing");
    assert_eq!(axes[1].direction(), "north");
}

#[test]
fn wkt_parse_wkt1_polar_stereographic_latitude_of_origin_90() {
    let crs = projected_from_wkt1(&wkt1_polar_stereographic(
        "PARAMETER[\"latitude_of_origin\",90],\
         PARAMETER[\"central_meridian\",2],\
         PARAMETER[\"false_easting\",3],\
         PARAMETER[\"false_northing\",4],",
    ));
    assert_eq!(
        projected_proj_string(&crs),
        "+proj=stere +lat_0=90 +lat_ts=90 +lon_0=2 +x_0=3 +y_0=4 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn wkt_parse_wkt1_polar_stereographic_latitude_of_origin_90_scale_factor_1() {
    let crs = projected_from_wkt1(&wkt1_polar_stereographic(
        "PARAMETER[\"latitude_of_origin\",90],\
         PARAMETER[\"central_meridian\",2],\
         PARAMETER[\"scale_factor\",1],\
         PARAMETER[\"false_easting\",3],\
         PARAMETER[\"false_northing\",4],",
    ));
    assert_eq!(
        projected_proj_string(&crs),
        "+proj=stere +lat_0=90 +lat_ts=90 +lon_0=2 +x_0=3 +y_0=4 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn wkt_parse_wkt1_polar_stereographic_scale_factor() {
    let crs = projected_from_wkt1(&wkt1_polar_stereographic(
        "PARAMETER[\"latitude_of_origin\",90],\
         PARAMETER[\"central_meridian\",2],\
         PARAMETER[\"scale_factor\",0.99],\
         PARAMETER[\"false_easting\",3],\
         PARAMETER[\"false_northing\",4],",
    ));
    assert_eq!(
        projected_proj_string(&crs),
        "+proj=stere +lat_0=90 +lon_0=2 +k=0.99 +x_0=3 +y_0=4 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn wkt_parse_wkt1_spherical_cross_track_height() {
    let crs = projected_from_wkt1(
        "PROJCS[\"unknown\",\
            GEOGCS[\"unknown\",\
                DATUM[\"WGS_1984\",\
                    SPHEROID[\"WGS 84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],\
                    AUTHORITY[\"EPSG\",\"6326\"]],\
                PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],\
                UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]]],\
            PROJECTION[\"Spherical_Cross_Track_Height\"],\
            PARAMETER[\"peg_point_latitude\",1],\
            PARAMETER[\"peg_point_longitude\",2],\
            PARAMETER[\"peg_point_heading\",3],\
            PARAMETER[\"peg_point_height\",4],\
            UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]]]",
    );
    assert_eq!(
        projected_proj_string(&crs),
        "+proj=sch +plat_0=1 +plon_0=2 +phdg_0=3 +h_0=4 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

fn geocentric_crs(object: &WktObject) -> &proj_rs::GeocentricCRS {
    match object.geodetic_crs() {
        Some(proj_rs::GeodeticCRS::Geocentric(crs)) => crs,
        _ => panic!("expected a geocentric CRS"),
    }
}

fn check_geocentric(crs: &proj_rs::GeocentricCRS) {
    assert_eq!(crs.name(), "WGS 84 (geocentric)");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    let expected = [
        ("Geocentric X", "X", "geocentricX"),
        ("Geocentric Y", "Y", "geocentricY"),
        ("Geocentric Z", "Z", "geocentricZ"),
    ];
    for (axis, (name, abbreviation, direction)) in axes.iter().zip(expected) {
        assert_eq!(axis.name(), name);
        assert_eq!(axis.abbreviation(), abbreviation);
        assert_eq!(axis.direction(), direction);
    }
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.name(), "World Geodetic System 1984");
    let ellipsoid = datum.ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_137.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
    assert_eq!(ellipsoid.inverse_flattening(), 298.257_223_563);
    assert_eq!(ellipsoid.name(), "WGS 84");
    assert_eq!(datum.prime_meridian().longitude().unit().name(), "degree");
}

#[test]
fn wkt_parse_wkt2_geocentric() {
    let object = parse(
        "GEODCRS[\"WGS 84 (geocentric)\",\
            DATUM[\"World Geodetic System 1984\",\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
                    LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]],\
                    ID[\"EPSG\",7030]],\
                ID[\"EPSG\",6326]],\
            PRIMEM[\"Greenwich\",0,\
                ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9122]],\
                ID[\"EPSG\",8901]],\
            CS[Cartesian,3],\
                AXIS[\"(X)\",GeocentricX,ORDER[1],LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],\
                AXIS[\"(Y)\",geocentricY,ORDER[2],LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],\
                AXIS[\"(Z)\",geocentricZ,ORDER[3],LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],\
            ID[\"EPSG\",4328]]",
    );
    check_geocentric(geocentric_crs(&object));
}

#[test]
fn wkt_parse_wkt2_simplified_geocentric() {
    let object = parse(
        "GEODCRS[\"WGS 84 (geocentric)\",\
            DATUM[\"World Geodetic System 1984\",\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\
            CS[Cartesian,3],\
                AXIS[\"(X)\",geocentricX],\
                AXIS[\"(Y)\",geocentricY],\
                AXIS[\"(Z)\",geocentricZ],\
                UNIT[\"metre\",1],\
            ID[\"EPSG\",4328]]",
    );
    check_geocentric(geocentric_crs(&object));
}

#[test]
fn wkt_parse_wkt1_geocentric() {
    let object = parse(
        "GEOCCS[\"WGS 84 (geocentric)\",\
            DATUM[\"WGS_1984\",\
                SPHEROID[\"WGS 84\",6378137,298.257223563,\
                    AUTHORITY[\"EPSG\",\"7030\"]],\
                AUTHORITY[\"EPSG\",\"6326\"]],\
            PRIMEM[\"Greenwich\",0,\
                AUTHORITY[\"EPSG\",\"8901\"]],\
            UNIT[\"metre\",1,\
                AUTHORITY[\"EPSG\",\"9001\"]],\
            AXIS[\"Geocentric X\",OTHER],\
            AXIS[\"Geocentric Y\",OTHER],\
            AXIS[\"Geocentric Z\",NORTH],\
            AUTHORITY[\"EPSG\",\"4328\"]]",
    );
    check_geocentric(geocentric_crs(&object));
}

#[test]
fn wkt_parse_wkt1_geocentric_with_z_other() {
    let object = parse(
        "GEOCCS[\"WGS 84 (geocentric)\",\
            DATUM[\"WGS_1984\",\
                SPHEROID[\"WGS 84\",6378137,298.257223563,\
                    AUTHORITY[\"EPSG\",\"7030\"]],\
                AUTHORITY[\"EPSG\",\"6326\"]],\
            PRIMEM[\"Greenwich\",0,\
                AUTHORITY[\"EPSG\",\"8901\"]],\
            UNIT[\"metre\",1,\
                AUTHORITY[\"EPSG\",\"9001\"]],\
            AXIS[\"Geocentric X\",OTHER],\
            AXIS[\"Geocentric Y\",OTHER],\
            AXIS[\"Geocentric Z\",OTHER],\
            AUTHORITY[\"EPSG\",\"4328\"]]",
    );
    check_geocentric(geocentric_crs(&object));
}

#[test]
fn wkt_parse_wkt1_geocentric_missing_unit_and_axis() {
    let wkt = "GEOCCS[\"WGS 84\",DATUM[\"WGS_1984\",\
        SPHEROID[\"WGS 84\",6378137,298.257223563]]]]";
    refused(wkt);
    let object = parse_loose(wkt);
    let crs = geocentric_crs(&object);
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert_eq!(axes[0].unit().name(), "metre");
}
