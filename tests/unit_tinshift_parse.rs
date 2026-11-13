//! PROJ `test/unit/test_tinshift.cpp` (`tinshift.basic`): the parse errors and
//! the horizontal evaluator, through `+proj=tinshift` operations. Values are
//! PROJ 9.9.0's; the C++ internal evaluator is reached through the operation
//! that wraps it.

use std::fs;
use std::path::{Path, PathBuf};

use proj_rs::{Operation, Xyzt};

fn write_file(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "proj-rs-tinshift-{name}-{}.json",
        std::process::id()
    ));
    fs::write(&path, body).expect("write tin file");
    path
}

fn tinshift(path: &Path) -> Result<Operation, proj_rs::CreateError> {
    let text = path.to_string_lossy().replace('\\', "/");
    Operation::create(&format!("+proj=tinshift +file={text}"))
}

/// The C++ `getMinValidContent()`, as JSON text.
const MIN_VALID: &str = r#"{
    "file_type": "triangulation_file",
    "format_version": "1.0",
    "input_crs": "EPSG:2393",
    "output_crs": "EPSG:3067",
    "transformed_components": ["horizontal"],
    "vertices_columns": ["source_x", "source_y", "target_x", "target_y"],
    "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
    "vertices": [[0, 0, 101, 101], [0, 1, 100, 101], [1, 1, 100, 100]],
    "triangles": [[0, 1, 2]]
}"#;

#[test]
fn tinshift_basic_parse_errors() {
    // `TINShiftJSONFile::parse` throws ParsingException for these.
    for (name, body) in [("foo", "foo"), ("null", "null"), ("empty", "{}")] {
        let path = write_file(name, body);
        assert!(tinshift(&path).is_err(), "{body} must not parse");
        let _ = fs::remove_file(&path);
    }
}

#[test]
fn tinshift_basic_horizontal_evaluator() {
    let path = write_file("min", MIN_VALID);
    let operation = tinshift(&path).expect("minimal valid file");
    let _ = fs::remove_file(&path);

    // Outside the triangle: no value (the C++ `forward` returns false).
    assert!(operation.forward(Xyzt::xy(-0.1, 0.0)).is_err());

    let cases = [
        ((0.0, 0.0), (101.0, 101.0)),
        ((0.0, 1.0), (100.0, 101.0)),
        ((1.0, 1.0), (100.0, 100.0)),
        ((0.0, 0.5), (100.5, 101.0)),
        ((0.5, 0.5), (100.5, 100.5)),
        ((0.5, 0.75), (100.25, 100.5)),
    ];
    for ((x, y), (x_out, y_out)) in cases {
        let input = Xyzt {
            x,
            y,
            z: 1000.0,
            t: 0.0,
        };
        let got = operation.forward(input).expect("inside the triangle");
        assert_eq!(got.x, x_out, "forward x at ({x}, {y})");
        assert_eq!(got.y, y_out, "forward y at ({x}, {y})");
        assert_eq!(got.z, 1000.0);
    }

    let back = operation
        .inverse(Xyzt {
            x: 100.25,
            y: 100.5,
            z: 1000.0,
            t: 0.0,
        })
        .expect("inverse inside the target triangle");
    assert_eq!(back.x, 0.5);
    assert_eq!(back.y, 0.75);
    assert_eq!(back.z, 1000.0);
}
