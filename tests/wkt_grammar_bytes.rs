//! The context window of a WKT grammar error is 40 bytes and can end in the middle of a UTF-8
//! character. PROJ writes those bytes as they are, in the strict-mode failure of
//! `proj_create_from_wkt`, in its log and in the `Grammar error:` lines of `projinfo`. The
//! expected bytes are the output of PROJ 9.9.0 for the input of `tests/data/utf8_cut`.

use proj_rs::projinfo::run;
use proj_rs::WktParser;
use std::fs;
use std::path::Path;

fn fixture(name: &str) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/utf8_cut");
    fs::read(dir.join(name)).expect("fixture")
}

fn input() -> String {
    String::from_utf8(fixture("input.wkt"))
        .expect("UTF-8 input")
        .trim_end()
        .to_owned()
}

#[test]
fn strict_failure_keeps_the_cut_character() {
    let expected = fixture("strict.expected");
    assert!(
        std::str::from_utf8(&expected).is_err(),
        "the fixture cuts a character"
    );
    let mut parser = WktParser::new();
    let error = parser
        .create_from_wkt(&input())
        .expect_err("strict parse fails");
    assert_eq!(parser.failure_bytes(), Some(expected.as_slice()));
    assert!(error.to_string().contains('\u{fffd}'));
}

#[test]
fn projinfo_writes_the_cut_character() {
    let expected = fixture("projinfo_stderr.expected");
    let report = run(&["-o", "PROJ", &input()]);
    assert!(
        report.warning_bytes.contains(&expected),
        "{:?}",
        report.warning_bytes
    );
    assert_eq!(report.warning_bytes.len(), report.warnings.len());
}
