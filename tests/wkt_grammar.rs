//! The WKT1 and WKT2 grammar checks that `WKTParser::createFromWKT` runs once the object is
//! built (`pj_wkt1_parse`, `pj_wkt2_parse`).
//!
//! The expected messages are the grammar errors that PROJ 9.9.0's `proj_create_from_wkt` gives
//! for each text (capi/tests/wkt_warnings.c, `check` mode, STRICT=NO), copied from the output
//! of the reference libproj, and for the multi-line texts the `Grammar error:` lines of its
//! `projinfo`. The strict-mode messages come from the same libproj with STRICT=YES.

use proj_rs::{WktParser, WktStrictness};

fn lax(wkt: &str) -> WktParser {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    let object = parser.create_from_wkt(wkt);
    assert!(object.is_ok(), "lax parse failed: {object:?}");
    parser
}

#[test]
fn wkt2_one_expected_token_end_of_string() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ']', expecting end of string. Error occurred around:
,ANGLEUNIT["degree",0.0174532925199433]]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_two_expected_tokens() {
    let wkt = r#"PROJCRS["WGS 84 / UTM zone 31N",,BASEGEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],ANGLEUNIT["degree",0.0174532925199433]],CONVERSION["UTM zone 31N",METHOD["Transverse Mercator"],PARAMETER["Latitude of natural origin",0],PARAMETER["Longitude of natural origin",3],PARAMETER["Scale factor at natural origin",0.9996],PARAMETER["False easting",500000],PARAMETER["False northing",0]],CS[Cartesian,2],AXIS["easting",east],AXIS["northing",north],LENGTHUNIT["metre",1]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ',', expecting BASEGEODCRS or BASEGEOGCRS. Error occurred around:
PROJCRS["WGS 84 / UTM zone 31N",,BASEGEOGCRS["WGS 84",DATUM["World Geode
                                ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_three_expected_tokens() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ',', expecting UNIT or LENGTHUNIT or ID. Error occurred around:
LLIPSOID["WGS 84",6378137,298.257223563,,LENGTHUNIT["metre",1]]],PRIMEM["Greenwi
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_four_expected_tokens() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433],USAGE[SCOPE["s"],,AREA["a"]]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ',', expecting AREA or BBOX or VERTICALEXTENT or TIMEEXTENT. Error occurred around:
e",0.0174532925199433],USAGE[SCOPE["s"],,AREA["a"]]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_more_than_four_expected_tokens_lists_none() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north,,ORDER[1]],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ','. Error occurred around:
CS[ellipsoidal,2],AXIS["latitude",north,,ORDER[1]],AXIS["longitude",east],ANGLEU
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_end_of_string_from_a_byte_above_0x7f() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433],REMARK[é]]"#;
    let expected = r#"Parsing error : syntax error, unexpected end of string, expecting string. Error occurred around:
NIT["degree",0.0174532925199433],REMARK[é]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_printed_quotes_are_a_string() {
    let wkt = r#"GEOGCRS[“WGS 84”,DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ']', expecting end of string. Error occurred around:
,ANGLEUNIT["degree",0.0174532925199433]]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_doubled_quote_in_a_string() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433],REMARK["a""b"]]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ']', expecting end of string. Error occurred around:
ree",0.0174532925199433],REMARK["a""b"]]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_undefined_token() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2]FOO,AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]"#;
    let expected = r#"Parsing error : syntax error, unexpected $undefined, expecting ','. Error occurred around:
,PRIMEM["Greenwich",0],CS[ellipsoidal,2]FOO,AXIS["latitude",north],AXIS["longitu
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_digits_one_two_three_are_their_own_tokens() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ',', expecting '1' or '2' or '3'. Error occurred around:
]],PRIMEM["Greenwich",0],CS[ellipsoidal,,2],AXIS["latitude",north],AXIS["longitu
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt2_2015_dialect_uses_the_wkt2_grammar() {
    let wkt = r#"GEODCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ']', expecting end of string. Error occurred around:
,ANGLEUNIT["degree",0.0174532925199433]]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt1_number_where_a_string_is_expected() {
    let wkt = r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],AXIS["Lat",NORTH],AXIS["Lon",EAST],AUTHORITY["EPSG",4326]]"#;
    let expected = r#"Parsing error : syntax error, unexpected number, expecting string. Error occurred around:
ORTH],AXIS["Lon",EAST],AUTHORITY["EPSG",4326]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt1_two_expected_tokens() {
    let wkt = r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],AXIS["Lat",NORTH],AXIS["Lon",EAST],AUTHORITY["EPSG","4326"],é]"#;
    let expected = r#"Parsing error : syntax error, unexpected ',', expecting ']' or ')'. Error occurred around:
XIS["Lon",EAST],AUTHORITY["EPSG","4326"],é]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt1_vert_cs_two_expected_tokens() {
    let wkt = r#"VERT_CS["NAVD88 height",VERT_DATUM["North American Vertical Datum 1988",2005],UNIT["metre",1],,AXIS["Gravity-related height",UP]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ',', expecting AUTHORITY or AXIS. Error occurred around:
tical Datum 1988",2005],UNIT["metre",1],,AXIS["Gravity-related height",UP]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn wkt1_one_expected_token_end_of_string() {
    let wkt = r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],AXIS["Lat",NORTH],AXIS["Lon",EAST]]]"#;
    let expected = r#"Parsing error : syntax error, unexpected ']', expecting end of string. Error occurred around:
433],AXIS["Lat",NORTH],AXIS["Lon",EAST]]]
                                        ^"#;
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn context_starts_after_the_previous_line_break() {
    let wkt = "GEOGCRS[\"WGS 84\",\n  DATUM[\"World Geodetic System 1984\",\n    ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n  CS[ellipsoidal,2],\n    AXIS[\"latitude\",north],\n    AXIS[\"longitude\",east],\n    ANGLEUNIT[\"degree\",0.0174532925199433],,\n  REMARK[\"r\"]]";
    let expected = "Parsing error : syntax error, unexpected ','. Error occurred around:\n ANGLEUNIT[\"degree\",0.0174532925199433],,\n                                        ^";
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn caret_at_the_start_of_a_line() {
    let wkt = "GEOGCRS[\"WGS 84\",\nDATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\nCS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east],ANGLEUNIT[\"degree\",0.0174532925199433],\n,REMARK[\"r\"]]";
    let expected =
        "Parsing error : syntax error, unexpected ','. Error occurred around:\n,REMARK[\"r\"]]\n^";
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

#[test]
fn context_stops_at_the_next_line_break() {
    let wkt = "GEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],CS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east],ANGLEUNIT[\"degree\",0.0174532925199433]],\n]";
    let expected = "Parsing error : syntax error, unexpected ',', expecting end of string. Error occurred around:\n,ANGLEUNIT[\"degree\",0.0174532925199433]],\n                                        ^";
    assert_eq!(lax(wkt).grammar_errors(), [expected]);
}

/// The 40 bytes before the error start inside a two-byte character: PROJ keeps the cut bytes.
#[test]
fn context_window_may_cut_a_utf8_sequence() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433],REMARK["ééééééééééééééééééééé"]]]"#;
    let expected: &[u8] = b"Parsing error : syntax error, unexpected ']', expecting end of string. Error occurred around:\n\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\"]]]\n                                        ^";
    let parser = lax(wkt);
    assert_eq!(parser.grammar_error_bytes(), [expected]);
    assert_eq!(parser.grammar_errors(), [String::from_utf8_lossy(expected)]);
}

#[test]
fn strict_mode_fails_with_the_grammar_error() {
    let wkt = r#"PROJCRS["WGS 84 / UTM zone 31N",,BASEGEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],ANGLEUNIT["degree",0.0174532925199433]],CONVERSION["UTM zone 31N",METHOD["Transverse Mercator"],PARAMETER["Latitude of natural origin",0],PARAMETER["Longitude of natural origin",3],PARAMETER["Scale factor at natural origin",0.9996],PARAMETER["False easting",500000],PARAMETER["False northing",0]],CS[Cartesian,2],AXIS["easting",east],AXIS["northing",north],LENGTHUNIT["metre",1]]"#;
    let expected = "Parsing error : syntax error, unexpected ',', expecting BASEGEODCRS or BASEGEOGCRS. Error occurred around:\nPROJCRS[\"WGS 84 / UTM zone 31N\",,BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geode\n                                ^";
    let error = WktParser::new()
        .create_from_wkt(wkt)
        .expect_err("strict mode refuses a grammar error");
    assert_eq!(error.to_string(), expected);
}

#[test]
fn valid_wkt2_has_no_grammar_error() {
    let wkt = r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0],CS[ellipsoidal,2],AXIS["latitude",north,ORDER[1]],AXIS["longitude",east,ORDER[2]],ANGLEUNIT["degree",0.0174532925199433],USAGE[SCOPE["s"],AREA["a"],BBOX[-90,-180,90,180]],ID["EPSG",4326]]"#;
    assert!(lax(wkt).grammar_errors().is_empty());
}
