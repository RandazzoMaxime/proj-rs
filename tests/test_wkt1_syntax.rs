//! WKT1 syntax and lax-mode warnings, as PROJ 9.9.0 writes them.
//!
//! The expected texts are the `Warning:` and `Grammar error:` lines that PROJ 9.9.0's
//! `projinfo` writes to stderr for each WKT1 text (projinfo_lib.cpp:389-406), copied
//! from the captured stderr of that run. The parser is lax, as projinfo sets it.

use proj_rs::{FactoryError, WktParser, WktStrictness};

fn lax(wkt: &str) -> WktParser {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    let object = parser.create_from_wkt(wkt);
    assert!(object.is_ok(), "lax parse failed: {object:?}");
    parser
}

/// test_projinfo.yaml#23: GEOGCS without PRIMEM; `,` is not expected after DATUM.
#[test]
fn wkt1_missing_primem_is_a_warning_and_a_grammar_error() {
    let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],UNIT[\"degree\",0.0174532925199433]]";
    let parser = lax(wkt);
    let expected_warnings: &[&str] = &["GEOGCS should have a PRIMEM node"];
    assert_eq!(parser.warnings(), expected_warnings);
    let expected_grammar: &[&str] = &["Parsing error : syntax error, unexpected UNIT, expecting PRIMEM. Error occurred around:\nHEROID[\"WGS 84\",6378137,298.257223563]],UNIT[\"degree\",0.0174532925199433]]\n                                        ^"];
    assert_eq!(parser.grammar_errors(), expected_grammar);
}

/// Two commas before UNIT: PROJ's own test text has one (test_io.cpp).
#[test]
fn wkt1_double_comma_before_unit() {
    let wkt = "PROJCS[\"unknown\",GEOGCS[\"unknown\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY[\"EPSG\",\"6326\"]],PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]]],PROJECTION[\"Polar_Stereographic\"],PARAMETER[\"latitude_of_origin\",90],PARAMETER[\"central_meridian\",2],PARAMETER[\"scale_factor\",0.99],PARAMETER[\"false_easting\",3],PARAMETER[\"false_northing\",4],,UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]]]";
    let parser = lax(wkt);
    let expected_warnings: &[&str] = &[];
    assert_eq!(parser.warnings(), expected_warnings);
    let expected_grammar: &[&str] = &["Parsing error : syntax error, unexpected ',', expecting UNIT or PARAMETER. Error occurred around:\nsting\",3],PARAMETER[\"false_northing\",4],,UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]\n                                        ^"];
    assert_eq!(parser.grammar_errors(), expected_grammar);
}

/// A bare identifier after the root node: expected end of string.
#[test]
fn wkt1_text_after_the_root_node() {
    let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]] x";
    let parser = lax(wkt);
    let expected_warnings: &[&str] = &[];
    assert_eq!(parser.warnings(), expected_warnings);
    let expected_grammar: &[&str] = &["Parsing error : syntax error, unexpected identifier, expecting end of string. Error occurred around:\n\",0],UNIT[\"degree\",0.0174532925199433]] x\n                                        ^"];
    assert_eq!(parser.grammar_errors(), expected_grammar);
}

/// An unterminated quoted string is the end of the input.
#[test]
fn wkt1_unterminated_string() {
    let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],AUTHORITY[\"EPSG,\"4326\"]";
    let parser = lax(wkt);
    let expected_warnings: &[&str] = &[];
    assert_eq!(parser.warnings(), expected_warnings);
    let expected_grammar: &[&str] = &["Parsing error : syntax error, unexpected AUTHORITY, expecting VERTCS. Error occurred around:\n\",0],UNIT[\"degree\",0.0174532925199433]],AUTHORITY[\"EPSG,\"4326\"]\n                                        ^"];
    assert_eq!(parser.grammar_errors(), expected_grammar);
}

/// FOO[...] where UNIT is expected, and the missing-UNIT warning for GEOGCS.
#[test]
fn wkt1_unknown_node_is_an_identifier() {
    let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],FOO[\"degree\",0.0174532925199433]]";
    let parser = lax(wkt);
    let expected_warnings: &[&str] = &["buildCS: missing UNIT in GEOGCS. Assuming degree"];
    assert_eq!(parser.warnings(), expected_warnings);
    let expected_grammar: &[&str] = &["Parsing error : syntax error, unexpected identifier, expecting UNIT. Error occurred around:\n7,298.257223563]],PRIMEM[\"Greenwich\",0],FOO[\"degree\",0.0174532925199433]]\n                                        ^"];
    assert_eq!(parser.grammar_errors(), expected_grammar);
}

#[test]
fn wkt1_lowercase_keywords_are_accepted() {
    let wkt = "geogcs[\"WGS 84\",datum[\"WGS_1984\",spheroid[\"WGS 84\",6378137,298.257223563]],primem[\"Greenwich\",0],unit[\"degree\",0.0174532925199433]]";
    let parser = lax(wkt);
    let expected: &[&str] = &[];
    assert_eq!(parser.warnings(), expected);
    assert!(parser.grammar_errors().is_empty());
}

#[test]
fn wkt1_esri_geogcs_is_accepted() {
    let wkt = "GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]";
    let parser = lax(wkt);
    let expected: &[&str] = &[];
    assert_eq!(parser.warnings(), expected);
    assert!(parser.grammar_errors().is_empty());
}

#[test]
fn wkt1_non_ascii_byte_reads_as_end_of_input() {
    let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]é";
    let parser = lax(wkt);
    let expected: &[&str] = &[];
    assert_eq!(parser.warnings(), expected);
    assert!(parser.grammar_errors().is_empty());
}

/// Strict mode, which PROJ's default parser uses. The object is built before the grammar
/// is checked (io.cpp:8506-8520), so a build error comes first: for case 23 that is the
/// missing PRIMEM. A text that builds fine fails on its grammar message.
#[test]
fn wkt1_grammar_error_is_an_error_in_strict_mode() {
    let primem = WktParser::new()
        .create_from_wkt("GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],UNIT[\"degree\",0.0174532925199433]]")
        .expect_err("strict");
    assert!(
        matches!(&primem, FactoryError::Invalid(message) if message == "GEOGCS should have a PRIMEM node")
    );
    let wkt = "PROJCS[\"unknown\",GEOGCS[\"unknown\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY[\"EPSG\",\"6326\"]],PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]]],PROJECTION[\"Polar_Stereographic\"],PARAMETER[\"latitude_of_origin\",90],PARAMETER[\"central_meridian\",2],PARAMETER[\"scale_factor\",0.99],PARAMETER[\"false_easting\",3],PARAMETER[\"false_northing\",4],,UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]]]";
    let error = WktParser::new().create_from_wkt(wkt).expect_err("strict");
    let FactoryError::Invalid(message) = error else {
        panic!("unexpected error kind: {error:?}");
    };
    assert_eq!(message, "Parsing error : syntax error, unexpected ',', expecting UNIT or PARAMETER. Error occurred around:\nsting\",3],PARAMETER[\"false_northing\",4],,UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]\n                                        ^");
}
