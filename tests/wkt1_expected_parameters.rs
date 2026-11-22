//! A WKT1 `PROJCS` whose projection lacks a parameter: the warnings of `validateParameters` and
//! the `CONVERSION` that PROJ 9.9.0 builds.
//!
//! PROJ names a Transverse Mercator `UTM zone N` only when its four UTM parameters are all there
//! (`Conversion::isUTM`, conversion.cpp:4499-4570), and `identify()` only renames it. A WKT1
//! `Polar_Stereographic` gets variant A or B with every parameter of the variant, a missing one
//! being 0 without a unit (io.cpp:4253-4300). Each `.expected` file is the output of PROJ 9.9.0:
//! `proj_create_from_wkt` with `STRICT=NO` for the warnings, and the `CONVERSION` node of
//! `proj_as_wkt` (WKT2:2019, one line). The warnings are those of the parser, then those of
//! `validateParameters`, as `proj_create_from_wkt` lists them.

use proj_rs::iso19111::Crs;
use proj_rs::{WktConvention, WktFormatter, WktLines, WktObject, WktParser, WktStrictness};
use std::fs;
use std::path::Path;

fn conversion_node(wkt2: &str) -> String {
    let start = wkt2.find("CONVERSION[").expect("CONVERSION node");
    let mut depth = 0;
    for (offset, ch) in wkt2[start..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return wkt2[start..=start + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced WKT2");
}

fn ours(wkt: &str) -> String {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    let object = parser.create_from_wkt(wkt).expect("object");
    let WktObject::Crs(crs) = object else {
        panic!("not a CRS");
    };
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.set_lines(WktLines::Single);
    let wkt2 = crs.export_to_wkt(&mut formatter).expect("WKT2");
    let Crs::Projected(projected) = &crs else {
        panic!("not a projected CRS");
    };
    let mut warnings = parser.warnings().to_vec();
    warnings.extend(projected.deriving_conversion().validate_parameters());
    let mut lines: Vec<String> = warnings
        .iter()
        .map(|warning| format!("warning: {warning}"))
        .collect();
    lines.push(format!("conversion: {}", conversion_node(&wkt2)));
    lines.join("\n") + "\n"
}

#[test]
fn missing_parameters_are_those_of_proj() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/wkt1_expected_parameters");
    let mut names: Vec<_> = fs::read_dir(&dir)
        .expect("fixtures")
        .filter_map(|entry| entry.ok()?.path().file_stem()?.to_str().map(str::to_owned))
        .collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 10);
    let mut failures = Vec::new();
    for name in &names {
        let wkt = fs::read_to_string(dir.join(format!("{name}.wkt"))).expect("wkt");
        let expected = fs::read_to_string(dir.join(format!("{name}.expected"))).expect("expected");
        let got = ours(wkt.trim());
        if got != expected {
            failures.push(format!("{name}\n  PROJ: {expected}  ours: {got}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
