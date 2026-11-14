//! PROJ `test/unit/test_tinshift.cpp` (`tinshift.basic`) through the public
//! `proj_rs::tinshift` API: the file parser and its accessors, and the
//! evaluator. Values, tolerances and fixtures are PROJ 9.9.0's.

use proj_rs::tinshift::{FallbackStrategy, TinShiftEvaluator, TinShiftFile};

/// The C++ `getMinValidContent()` as JSON text.
fn min_valid() -> String {
    r#"{
        "file_type": "triangulation_file",
        "format_version": "1.0",
        "input_crs": "EPSG:2393",
        "output_crs": "EPSG:3067",
        "transformed_components": ["horizontal"],
        "vertices_columns": ["source_x", "source_y", "target_x", "target_y"],
        "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
        "vertices": [[0, 0, 101, 101], [0, 1, 100, 101], [1, 1, 100, 100]],
        "triangles": [[0, 1, 2]]
    }"#
    .to_owned()
}

fn parse(text: &str) -> TinShiftFile {
    TinShiftFile::parse(text).expect("valid file")
}

#[test]
fn tinshift_basic_parse_errors() {
    assert!(TinShiftFile::parse("foo").is_err());
    assert!(TinShiftFile::parse("null").is_err());
    assert!(TinShiftFile::parse("{}").is_err());
}

#[test]
fn tinshift_basic_accessors() {
    let file = parse(&min_valid());
    assert_eq!(file.file_type(), "triangulation_file");
    assert_eq!(file.format_version(), "1.0");
    assert_eq!(file.input_crs(), "EPSG:2393");
    assert_eq!(file.output_crs(), "EPSG:3067");
    assert_eq!(file.fallback_strategy(), FallbackStrategy::None);
}

#[test]
fn tinshift_basic_horizontal_evaluator() {
    let file = parse(&min_valid());
    let eval = TinShiftEvaluator::new(file);
    assert_eq!(eval.forward(-0.1, 0.0, 1000.0), None);
    let cases = [
        ((0.0, 0.0), (101.0, 101.0)),
        ((0.0, 1.0), (100.0, 101.0)),
        ((1.0, 1.0), (100.0, 100.0)),
        ((0.0, 0.5), (100.5, 101.0)),
        ((0.5, 0.5), (100.5, 100.5)),
        ((0.5, 0.75), (100.25, 100.5)),
    ];
    for ((x, y), (x_out, y_out)) in cases {
        assert_eq!(eval.forward(x, y, 1000.0), Some([x_out, y_out, 1000.0]));
    }
    assert_eq!(
        eval.inverse(100.25, 100.5, 1000.0),
        Some([0.5, 0.75, 1000.0])
    );
}

#[test]
fn tinshift_basic_vertical_source_z_and_target_z() {
    let text = r#"{
        "file_type": "triangulation_file", "format_version": "1.0",
        "input_crs": "EPSG:2393", "output_crs": "EPSG:3067",
        "transformed_components": ["vertical"],
        "vertices_columns": ["source_x", "source_y", "source_z", "target_z"],
        "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
        "vertices": [[0, 0, 10.5, 10.6], [0, 1, 15.0, 15.2], [1, 1, 17.5, 18.0]],
        "triangles": [[0, 1, 2]]
    }"#;
    let eval = TinShiftEvaluator::new(parse(text));
    assert_eq!(eval.forward(0.0, 0.0, 1000.0), Some([0.0, 0.0, 1000.1]));
    assert_eq!(eval.forward(0.5, 0.75, 1000.0), Some([0.5, 0.75, 1000.325]));
    assert_eq!(eval.inverse(0.5, 0.75, 1000.325), Some([0.5, 0.75, 1000.0]));
}

#[test]
fn tinshift_basic_vertical_offset_z() {
    let text = r#"{
        "file_type": "triangulation_file", "format_version": "1.0",
        "input_crs": "EPSG:2393", "output_crs": "EPSG:3067",
        "transformed_components": ["vertical"],
        "vertices_columns": ["source_x", "source_y", "offset_z"],
        "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
        "vertices": [[0, 0, 0.1], [0, 1, 0.2], [1, 1, 0.5]],
        "triangles": [[0, 1, 2]]
    }"#;
    let eval = TinShiftEvaluator::new(parse(text));
    assert_eq!(eval.forward(0.0, 0.0, 1000.0), Some([0.0, 0.0, 1000.1]));
    assert_eq!(eval.forward(0.5, 0.75, 1000.0), Some([0.5, 0.75, 1000.325]));
    assert_eq!(eval.inverse(0.5, 0.75, 1000.325), Some([0.5, 0.75, 1000.0]));
}

#[test]
fn tinshift_basic_horizontal_and_vertical() {
    let text = r#"{
        "file_type": "triangulation_file", "format_version": "1.0",
        "input_crs": "EPSG:2393", "output_crs": "EPSG:3067",
        "transformed_components": ["horizontal", "vertical"],
        "vertices_columns": ["source_x", "source_y", "target_x", "target_y", "offset_z"],
        "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
        "vertices": [[0, 0, 101, 101, 0.1], [0, 1, 100, 101, 0.2], [1, 1, 100, 100, 0.5]],
        "triangles": [[0, 1, 2]]
    }"#;
    let eval = TinShiftEvaluator::new(parse(text));
    assert_eq!(eval.forward(0.0, 0.0, 1000.0), Some([101.0, 101.0, 1000.1]));
    assert_eq!(
        eval.forward(0.5, 0.75, 1000.0),
        Some([100.25, 100.5, 1000.325])
    );
    assert_eq!(
        eval.inverse(100.25, 100.5, 1000.325),
        Some([0.5, 0.75, 1000.0])
    );
}

#[test]
fn tinshift_basic_fallback_strategies() {
    let min = min_valid();
    // The fallback strategy needs format 1.1, and "none" is rejected in 1.0.
    assert!(TinShiftFile::parse(&min.replace(
        "\"triangles\"",
        "\"fallback_strategy\": \"none\", \"triangles\""
    ))
    .is_err());
    let invalid = min.replace("\"1.0\"", "\"1.1\"").replace(
        "\"triangles\"",
        "\"fallback_strategy\": \"invalid\", \"triangles\"",
    );
    assert!(TinShiftFile::parse(&invalid).is_err());

    // Nearest side with no triangle: no value.
    let empty = min.replace("\"1.0\"", "\"1.1\"").replace(
        "\"triangles\": [[0, 1, 2]]",
        "\"fallback_strategy\": \"nearest_side\", \"triangles\": []",
    );
    let file = parse(&empty);
    assert_eq!(file.fallback_strategy(), FallbackStrategy::NearestSide);
    assert_eq!(TinShiftEvaluator::new(file).forward(1.0, 1.0, 1.0), None);

    // Nearest side with a degenerate triangle (two identical vertices).
    let degenerate = min
        .replace("\"1.0\"", "\"1.1\"")
        .replace(
            "[[0, 0, 101, 101], [0, 1, 100, 101], [1, 1, 100, 100]]",
            "[[0, 0, 101, 101], [0, 1, 100, 101], [0, 1, 100, 100]]",
        )
        .replace(
            "\"triangles\"",
            "\"fallback_strategy\": \"nearest_side\", \"triangles\"",
        );
    assert_eq!(
        TinShiftEvaluator::new(parse(&degenerate)).forward(1.0, 1.0, 1.0),
        None
    );

    // Only degenerate triangles (one side of length zero).
    let flat = min
        .replace("\"1.0\"", "\"1.1\"")
        .replace(
            "[[0, 0, 101, 101], [0, 1, 100, 101], [1, 1, 100, 100]]",
            "[[0, 0, 101, 101], [0, 0.5, 100, 101], [0, 1, 100, 100]]",
        )
        .replace(
            "\"triangles\"",
            "\"fallback_strategy\": \"nearest_side\", \"triangles\"",
        );
    assert_eq!(
        TinShiftEvaluator::new(parse(&flat)).forward(1.0, 1.0, 1.0),
        None
    );
}
