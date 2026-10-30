//! PROJ `test/unit/test_io.cpp` checks that parse WKT, PROJ strings, and PROJJSON.
//!
//! Each function is one `TEST()` name. Expected text is copied from that test.

use proj_rs::{
    DatabaseContext, UnitOfMeasure, WktConvention, WktDialect, WktFormatter, WktNode, WktObject,
    WktParser,
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
