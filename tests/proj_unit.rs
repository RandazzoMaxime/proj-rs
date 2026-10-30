//! PROJ `test/unit` cases that build an [`Operation`](proj_rs::Operation) or a [`Proj`](proj_rs::Proj).
//!
//! Each test is named for the C++ `TEST()`. Object-model checks that this crate
//! does not expose (WKT identity, NameFactory, grid pixel iterators) are noted
//! in `PORT-NOTES-tests.md` and `docs/UNPORTED-UNIT.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use proj_rs::{CreateError, Operation, Proj, ProjBuilder, UtmZone, Xyzt};

fn near(got: f64, expect: f64, tol: f64) {
    assert!((got - expect).abs() <= tol, "{got} vs {expect} (tol {tol})");
}

fn rad(deg: f64) -> f64 {
    deg.to_radians()
}

fn scratch(name: &str, body: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "proj-rs-{name}-{}-{nanos}.json",
        std::process::id()
    ));
    fs::write(&path, body).expect("write model");
    path
}

fn op_file(prefix: &str, path: &Path) -> Operation {
    let text = path.to_string_lossy().replace('\\', "/");
    Operation::create(&format!(
        "+proj={prefix} +{key}={text}",
        key = if prefix == "defmodel" {
            "model"
        } else {
            "file"
        }
    ))
    .unwrap_or_else(|error| panic!("create {prefix}: {error}"))
}

fn grid_tif() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/tests/simple_model_degree_3d_grid.tif")
        .to_string_lossy()
        .replace('\\', "/")
}

fn min_model(time_function: &str) -> String {
    let grid = grid_tif();
    format!(
        r#"{{
  "file_type": "deformation_model_master_file",
  "format_version": "1.0",
  "source_crs": "EPSG:4326",
  "target_crs": "EPSG:7907",
  "definition_crs": "EPSG:4326",
  "horizontal_offset_method": "addition",
  "horizontal_offset_unit": "degree",
  "extent": {{"type": "bbox", "parameters": {{"bbox": [-180, -90, 180, 90]}}}},
  "time_extent": {{"first": "1900-01-01T00:00:00Z", "last": "2050-01-01T00:00:00Z"}},
  "components": [{{
    "displacement_type": "horizontal",
    "uncertainty_type": "none",
    "extent": {{"type": "bbox", "parameters": {{"bbox": [-180, -90, 180, 90]}}}},
    "spatial_model": {{
      "type": "GeoTIFF",
      "interpolation_method": "bilinear",
      "filename": "{grid}"
    }},
    "time_function": {time_function}
  }}]
}}"#
    )
}

/// Longitude and latitude shift, in degrees, at 2°E 49°N.
fn degree_shift(model: &str, time: f64) -> (f64, f64) {
    let path = scratch("defmodel", model);
    let operation = op_file("defmodel", &path);
    let input = Xyzt {
        x: rad(2.0),
        y: rad(49.0),
        z: 0.0,
        t: time,
    };
    let got = operation.forward(input).expect("forward");
    let _ = fs::remove_file(&path);
    (
        (got.x - input.x).to_degrees(),
        (got.y - input.y).to_degrees(),
    )
}

fn assert_scale(shift: (f64, f64), scale: f64) {
    near(shift.0, scale, 1e-6);
    near(shift.1, 2.0 * scale, 1e-6);
}

#[test]
fn angular_units_basic() {
    let operation = Operation::create("+proj=latlong").expect("latlong");
    assert!(operation.angular_input(false));
    assert!(operation.angular_output(false));
    assert!(operation.angular_input(true));
    assert!(operation.angular_output(true));
}

#[test]
fn angular_units_pipelines() {
    let operation = Operation::create(
        "proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=latlong +step +proj=axisswap +order=2,1",
    )
    .expect("pipeline");
    assert!(operation.angular_input(false));
    assert!(operation.angular_output(false));
    assert!(operation.angular_input(true));
    assert!(operation.angular_output(true));
}

#[test]
fn angular_units_pipelines2() {
    let operation = Operation::create(
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=tmerc +lat_0=0 +lon_0=-81 +k=0.9996 +x_0=500000.001016002 +y_0=0 +ellps=WGS84 +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=us-ft +z_out=us-ft",
    )
    .expect("pipeline");
    assert!(!operation.angular_input(false));
    assert!(!operation.angular_output(false));
}

#[test]
fn angular_units_pipelines3() {
    let operation = Operation::create(
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=tmerc +lat_0=0 +lon_0=-81 +k=0.9996 +x_0=500000.001016002 +y_0=0 +ellps=WGS84 +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=us-ft +z_out=us-ft",
    )
    .expect("pipeline");
    assert!(operation.angular_input(false));
    assert!(!operation.angular_output(false));
}

#[test]
fn angular_units_degrees() {
    let operation = Operation::create(
        "+proj=pipeline +step +inv +proj=utm +zone=32 +ellps=GRS80 +step +proj=unitconvert +xy_in=rad +xy_out=deg",
    )
    .expect("pipeline");
    // `degree_output(true)` is the forward degree input (the inverse reads it).
    assert!(!operation.degree_output(true));
    assert!(operation.degree_output(false));
    assert!(operation.degree_output(false));
    assert!(!operation.degree_output(true));
}

#[test]
fn proj_context_init_epsg_and_unknown_dictionary() {
    let operation = Operation::create("+init=epsg:32631").expect("init");
    let got = operation
        .forward(Xyzt::xy(rad(3.0), rad(46.5)))
        .expect("forward");
    assert!(got.x.is_finite() && got.y.is_finite());
    // No file finder: a dictionary name is an error, not a coordinate.
    assert!(Operation::create("+init=temp_proj_dic1:MY_PIPELINE").is_err());
}

#[test]
fn proj_context_set_search_paths() {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/tests");
    let mut builder = ProjBuilder::new();
    builder.set_search_paths(&data).expect("search path");
    let info = builder.lib_info().expect("info");
    assert!(info.searchpath.contains("data"), "{}", info.searchpath);
    builder
        .proj("+proj=hgridshift +grids=test_hgrid.tif")
        .expect("grid on the added path");
}

#[test]
fn proj_context_set_user_writable_directory() {
    let dir = std::env::temp_dir().join(format!("proj-rs-writable-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    ProjBuilder::new()
        .set_user_writable_directory(&dir)
        .expect("create writable dir");
    assert!(dir.is_dir());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn proj_context_ca_bundle_is_not_a_coordinate_path() {
    // `proj_context_set_ca_bundle_path` is not a public setter. Network stays
    // off unless the caller or PROJ_NETWORK turns it on, and the endpoint is
    // still the documented default.
    let builder = ProjBuilder::new();
    assert!(!builder.network_enabled());
    let endpoint = builder.get_url_endpoint().expect("endpoint");
    assert!(endpoint.starts_with("https://"), "{endpoint}");
}

#[test]
fn proj_errno_string_no_error_and_known_failures() {
    assert!(Operation::create("+proj=utm +zone=31 +ellps=WGS84").is_ok());
    let syntax = Operation::create("+proj=").expect_err("empty proj");
    assert!(syntax.to_string().contains("proj"), "{syntax}");
    let unknown = Operation::create("+proj=not_a_real_operation").expect_err("unknown");
    assert!(matches!(unknown, CreateError::UnknownProj(_)));
    assert!(unknown.to_string().contains("not_a_real_operation"));
    let outside = Operation::create("+proj=utm +zone=31 +ellps=WGS84")
        .expect("utm")
        .forward(Xyzt::xy(rad(3.0), rad(95.0)));
    assert!(outside.is_err());
}

#[test]
fn grids_null_shift_is_identity() {
    let vertical = Operation::create("+proj=vgridshift +grids=null").expect("null vgrid");
    let got = vertical
        .forward(Xyzt {
            x: rad(4.0),
            y: rad(52.0),
            z: 12.5,
            t: 0.0,
        })
        .expect("vertical");
    near(got.z, 12.5, 1e-9);
    let horizontal = Operation::create("+proj=hgridshift +grids=null").expect("null hgrid");
    let input = Xyzt::xy(rad(5.5), rad(53.5));
    let got = horizontal.forward(input).expect("horizontal");
    near(got.x, input.x, 1e-12);
    near(got.y, input.y, 1e-12);
    assert!(Operation::create("+proj=vgridshift +grids=foobar").is_err());
    assert!(Operation::create("+proj=hgridshift +grids=foobar").is_err());
}

#[test]
fn grids_gtiff_subgrid_matches_gie_samples() {
    let grid = Operation::create("+proj=gridshift +grids=tests/test_hgrid_with_subgrid.tif")
        .expect("subgrid");
    let cases = [
        (-115.5416667, 51.1666667, -115.5427092888, 51.1666899972),
        (-80.5041667, 44.5458333, -80.50401615833, 44.5458827236),
    ];
    for (lon, lat, east, north) in cases {
        let got = grid.forward(Xyzt::xy(rad(lon), rad(lat))).expect("in grid");
        near(got.x.to_degrees(), east, 1e-8);
        near(got.y.to_degrees(), north, 1e-8);
    }
    assert!(grid.forward(Xyzt::xy(rad(179.799), rad(54.5))).is_err());
    let named = Operation::create(
        "+proj=gridshift +grids=tests/test_hgrid_with_two_level_of_subgrids_no_grid_name.tif",
    )
    .expect("two level");
    assert!(named.forward(Xyzt::xy(rad(-45.5), rad(22.5))).is_ok());
    // `tests/test_3d_grid_projected.tif` is a projected GeoTIFF. Opening it by
    // the gie-relative name is an error when the file is not a geographic shift,
    // which is still not a coordinate.
    assert!(Operation::create("+proj=gridshift +grids=tests/test_3d_grid_projected.tif").is_err());
}

#[test]
fn defmodel_basic_and_rejected_json() {
    for text in ["foo", "null", "{}", ""] {
        let path = scratch("bad", text);
        let text_path = path.to_string_lossy().replace('\\', "/");
        assert!(
            Operation::create(&format!("+proj=defmodel +model={text_path}")).is_err(),
            "{text}"
        );
        let _ = fs::remove_file(&path);
    }
    let path = scratch("min", &min_model(r#"{"type": "constant"}"#));
    let _ = op_file("defmodel", &path);
    let _ = fs::remove_file(&path);
    let mut broken = min_model(r#"{"type": "constant"}"#);
    broken = broken.replace("\"EPSG:4326\"", "\"EPSG:4978\"");
    // source and definition were both replaced; put source back so they differ.
    let broken = broken.replacen("EPSG:4978", "EPSG:4326", 1);
    let path = scratch("mismatch", &broken);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(Operation::create(&format!("+proj=defmodel +model={text_path}")).is_err());
    let _ = fs::remove_file(&path);
}

#[test]
fn defmodel_error_cases() {
    let bad_method = min_model(r#"{"type": "constant"}"#).replace("addition", "unsupported");
    let path = scratch("method", &bad_method);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(Operation::create(&format!("+proj=defmodel +model={text_path}")).is_err());
    let _ = fs::remove_file(&path);

    let bad_interp = min_model(r#"{"type": "constant"}"#).replace("bilinear", "unsupported");
    let path = scratch("interp", &bad_interp);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(Operation::create(&format!("+proj=defmodel +model={text_path}")).is_err());
    let _ = fs::remove_file(&path);

    let bad_unit = min_model(
        r#"{"type": "velocity", "parameters": {"reference_epoch": "2000-01-01T00:00:00Z"}}"#,
    )
    .replace("bilinear", "geocentric_bilinear");
    let path = scratch("combo", &bad_unit);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(
        Operation::create(&format!("+proj=defmodel +model={text_path}")).is_err(),
        "degree unit rejects geocentric bilinear"
    );
    let _ = fs::remove_file(&path);

    let bad_time = min_model(r#"{"type": "unknown"}"#);
    let path = scratch("time", &bad_time);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(Operation::create(&format!("+proj=defmodel +model={text_path}")).is_err());
    let _ = fs::remove_file(&path);

    let bad_date = min_model(
        r#"{"type": "velocity", "parameters": {"reference_epoch": "2001-02-29T00:00:00Z"}}"#,
    );
    let path = scratch("date", &bad_date);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(Operation::create(&format!("+proj=defmodel +model={text_path}")).is_err());
    let _ = fs::remove_file(&path);
}

#[test]
fn defmodel_iso8601_and_time_functions() {
    let constant = min_model(r#"{"type": "constant"}"#);
    assert_scale(degree_shift(&constant, 1999.0), 1.0);
    assert_scale(degree_shift(&constant, 2000.0), 1.0);
    assert_scale(degree_shift(&constant, 2001.0), 1.0);

    let velocity = min_model(
        r#"{"type": "velocity", "parameters": {"reference_epoch": "2000-01-01T00:00:00Z"}}"#,
    );
    assert_scale(degree_shift(&velocity, 1999.0), -1.0);
    assert_scale(degree_shift(&velocity, 2000.0), 0.0);
    assert_scale(degree_shift(&velocity, 2001.0), 1.0);
    let leap = 2000.0 + ((31.0 + 28.0) * 86_400.0 + 12.0 * 3_600.0) / (366.0 * 86_400.0);
    let leap_model = min_model(
        r#"{"type": "velocity", "parameters": {"reference_epoch": "2000-02-29T12:00:00Z"}}"#,
    );
    assert_scale(degree_shift(&leap_model, leap), 0.0);

    let step =
        min_model(r#"{"type": "step", "parameters": {"step_epoch": "2000-01-01T00:00:00Z"}}"#);
    assert_scale(degree_shift(&step, 1999.99), 0.0);
    assert_scale(degree_shift(&step, 2000.0), 1.0);
    assert_scale(degree_shift(&step, 2000.01), 1.0);

    let reverse = min_model(
        r#"{"type": "reverse_step", "parameters": {"step_epoch": "2000-01-01T00:00:00Z"}}"#,
    );
    assert_scale(degree_shift(&reverse, 1999.99), -1.0);
    assert_scale(degree_shift(&reverse, 2000.0), 0.0);
    assert_scale(degree_shift(&reverse, 2000.01), 0.0);

    let piecewise = min_model(
        r#"{"type": "piecewise", "parameters": {"before_first": "zero", "after_last": "constant", "model": [
            {"epoch": "2016-01-01T00:00:00Z", "scale_factor": 0.5},
            {"epoch": "2017-01-01T00:00:00Z", "scale_factor": 1.0},
            {"epoch": "2017-01-01T00:00:00Z", "scale_factor": 2.0},
            {"epoch": "2018-01-01T00:00:00Z", "scale_factor": 1.0}
        ]}}"#,
    );
    assert_scale(degree_shift(&piecewise, 2015.99), 0.0);
    assert_scale(degree_shift(&piecewise, 2016.0), 0.5);
    assert_scale(degree_shift(&piecewise, 2016.5), 0.75);
    assert_scale(degree_shift(&piecewise, 2017.0), 2.0);
    assert_scale(degree_shift(&piecewise, 2017.5), 1.5);
    assert_scale(degree_shift(&piecewise, 2018.0), 1.0);
    assert_scale(degree_shift(&piecewise, 2019.0), 1.0);

    let linear_before =
        piecewise.replace("\"before_first\": \"zero\"", "\"before_first\": \"linear\"");
    assert_scale(degree_shift(&linear_before, 2015.5), 0.25);
    let constant_before = piecewise.replace(
        "\"before_first\": \"zero\"",
        "\"before_first\": \"constant\"",
    );
    assert_scale(degree_shift(&constant_before, 2015.5), 0.5);
    let zero_after = piecewise.replace("\"after_last\": \"constant\"", "\"after_last\": \"zero\"");
    assert_scale(degree_shift(&zero_after, 2018.5), 0.0);
    let linear_after =
        piecewise.replace("\"after_last\": \"constant\"", "\"after_last\": \"linear\"");
    assert_scale(degree_shift(&linear_after, 2018.5), 0.5);

    let exp = min_model(
        r#"{"type": "exponential", "parameters": {
            "reference_epoch": "2000-01-01T00:00:00Z",
            "end_epoch": "2001-01-01T00:00:00Z",
            "relaxation_constant": 2.0,
            "before_scale_factor": 0.0,
            "initial_scale_factor": 1.0,
            "final_scale_factor": 3.0
        }}"#,
    );
    let exp_at = |time: f64| -> f64 {
        if time < 2000.0 {
            0.0
        } else {
            let time = time.min(2001.0);
            1.0 + (3.0 - 1.0) * (1.0 - (-(time - 2000.0) / 2.0).exp())
        }
    };
    for time in [1999.99, 2000.0, 2000.5, 2001.0, 2002.0] {
        assert_scale(degree_shift(&exp, time), exp_at(time));
    }
}

#[test]
fn tinshift_basic_triangles() {
    let horizontal = r#"{
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
    let path = scratch("tin", horizontal);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(Operation::create("+proj=tinshift +file=foo").is_err());
    assert!(Operation::create(&format!("+proj=tinshift +file={text_path}.missing")).is_err());
    let tin = op_file("tinshift", &path);
    assert!(tin
        .forward(Xyzt {
            x: -0.1,
            y: 0.0,
            z: 1000.0,
            t: 0.0,
        })
        .is_err());
    let check = |x, y, ex, ey| {
        let got = tin
            .forward(Xyzt {
                x,
                y,
                z: 1000.0,
                t: 0.0,
            })
            .expect("inside");
        near(got.x, ex, 1e-9);
        near(got.y, ey, 1e-9);
        near(got.z, 1000.0, 1e-9);
    };
    check(0.0, 0.0, 101.0, 101.0);
    check(0.0, 1.0, 100.0, 101.0);
    check(1.0, 1.0, 100.0, 100.0);
    check(0.0, 0.5, 100.5, 101.0);
    check(0.5, 0.5, 100.5, 100.5);
    check(0.5, 0.75, 100.25, 100.5);
    let back = tin
        .inverse(Xyzt {
            x: 100.25,
            y: 100.5,
            z: 1000.0,
            t: 0.0,
        })
        .expect("inverse");
    near(back.x, 0.5, 1e-9);
    near(back.y, 0.75, 1e-9);
    near(back.z, 1000.0, 1e-9);
    let _ = fs::remove_file(&path);

    let vertical = r#"{
        "file_type": "triangulation_file",
        "format_version": "1.0",
        "input_crs": "EPSG:2393",
        "output_crs": "EPSG:3067",
        "transformed_components": ["vertical"],
        "vertices_columns": ["source_x", "source_y", "source_z", "target_z"],
        "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
        "vertices": [[0, 0, 10.5, 10.6], [0, 1, 15.0, 15.2], [1, 1, 17.5, 18.0]],
        "triangles": [[0, 1, 2]]
    }"#;
    let path = scratch("tinv", vertical);
    let tin = op_file("tinshift", &path);
    let got = tin
        .forward(Xyzt {
            x: 0.5,
            y: 0.75,
            z: 1000.0,
            t: 0.0,
        })
        .expect("vertical");
    near(got.x, 0.5, 1e-9);
    near(got.y, 0.75, 1e-9);
    near(got.z, 1000.325, 1e-9);
    let _ = fs::remove_file(&path);

    let both = r#"{
        "file_type": "triangulation_file",
        "format_version": "1.0",
        "input_crs": "EPSG:2393",
        "output_crs": "EPSG:3067",
        "transformed_components": ["horizontal", "vertical"],
        "vertices_columns": ["source_x", "source_y", "target_x", "target_y", "offset_z"],
        "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
        "vertices": [[0, 0, 101, 101, 0.1], [0, 1, 100, 101, 0.2], [1, 1, 100, 100, 0.5]],
        "triangles": [[0, 1, 2]]
    }"#;
    let path = scratch("tinhv", both);
    let tin = op_file("tinshift", &path);
    let got = tin
        .forward(Xyzt {
            x: 0.5,
            y: 0.75,
            z: 1000.0,
            t: 0.0,
        })
        .expect("both");
    near(got.x, 100.25, 1e-9);
    near(got.y, 100.5, 1e-9);
    near(got.z, 1000.325, 1e-9);
    let _ = fs::remove_file(&path);

    let illegal = horizontal.replace(
        "\"format_version\": \"1.0\"",
        "\"format_version\": \"1.0\", \"fallback_strategy\": \"none\"",
    );
    let path = scratch("tinbad", &illegal);
    let text_path = path.to_string_lossy().replace('\\', "/");
    assert!(Operation::create(&format!("+proj=tinshift +file={text_path}")).is_err());
    let _ = fs::remove_file(&path);

    let empty = r#"{
        "file_type": "triangulation_file",
        "format_version": "1.1",
        "input_crs": "EPSG:2393",
        "output_crs": "EPSG:3067",
        "transformed_components": ["horizontal"],
        "fallback_strategy": "nearest_side",
        "vertices_columns": ["source_x", "source_y", "target_x", "target_y"],
        "triangles_columns": ["idx_vertex1", "idx_vertex2", "idx_vertex3"],
        "vertices": [[0, 0, 101, 101], [0, 1, 100, 101], [1, 1, 100, 100]],
        "triangles": []
    }"#;
    let path = scratch("tinempty", empty);
    let tin = op_file("tinshift", &path);
    assert!(tin.forward(Xyzt::xy(1.0, 1.0)).is_err());
    let _ = fs::remove_file(&path);
}

#[test]
fn polyhedral_roundtrip_and_offsets() {
    let configs = [
        ("+proj=tsea +R=1", 1e-13),
        ("+proj=tsea +ellps=WGS84", 1e-13),
        ("+proj=tsea +R=1 +orient_lat=90 +orient_lon=0 +azi=0", 1e-13),
        ("+proj=tsea +R=1 +lat_0=10 +lon_0=20", 1e-11),
        ("+proj=tsea +R=1 +orient_lat=45 +orient_lon=30 +azi=60 +lat_0=15 +lon_0=-25 +x_0=1000 +y_0=-2000", 1e-11),
        ("+proj=dsea +R=1", 1e-13),
        ("+proj=dsea +ellps=WGS84", 1e-13),
        ("+proj=dsea +R=1 +lat_0=10 +lon_0=20", 1e-11),
        ("+proj=isea +R=1", 1e-13),
        ("+proj=isea +ellps=WGS84", 1e-13),
        ("+proj=isea +R=1 +lat_0=10 +lon_0=20", 1e-11),
        ("+proj=ivea +R=1", 1e-13),
        ("+proj=ivea +ellps=WGS84", 1e-13),
        ("+proj=ivea +R=1 +lat_0=10 +lon_0=20", 1e-11),
    ];
    let points = [
        (0.0, 0.0),
        (45.0, 90.0),
        (-45.0, -90.0),
        (89.0, 0.0),
        (-89.0, 170.0),
        (30.0, 120.0),
        (-60.0, -150.0),
        (15.0, 45.0),
        (-30.0, -60.0),
        (70.0, -170.0),
    ];
    for (definition, tol) in configs {
        let operation =
            Operation::create(definition).unwrap_or_else(|error| panic!("{definition}: {error}"));
        for (lat, lon) in points {
            let input = Xyzt::xy(rad(lon), rad(lat));
            let flat = operation
                .forward(input)
                .unwrap_or_else(|error| panic!("{definition} {lat},{lon}: {error}"));
            let back = operation.inverse(flat).expect("inverse");
            assert!((back.x - input.x).abs() <= tol, "{definition} lon {lon}");
            assert!((back.y - input.y).abs() <= tol, "{definition} lat {lat}");
        }
    }

    let origin = Operation::create("+proj=tsea +R=1 +lat_0=10 +lon_0=20").expect("origin");
    let got = origin
        .forward(Xyzt::xy(rad(20.0), rad(10.0)))
        .expect("forward");
    near(got.x, 0.0, 1e-13);
    near(got.y, 0.0, 1e-13);

    assert!(Operation::create("+proj=dsea +R=1 +net=does_not_exist").is_err());
    for net in ["dsea", "a5", "crescent", "flower"] {
        Operation::create(&format!("+proj=dsea +R=1 +net={net}"))
            .unwrap_or_else(|error| panic!("{net}: {error}"));
    }

    let stacked =
        Operation::create("+proj=tsea +R=1 +lat_0=10 +lon_0=20 +x_0=7 +y_0=-3").expect("stacked");
    let got = stacked
        .forward(Xyzt::xy(rad(20.0), rad(10.0)))
        .expect("forward");
    near(got.x, 7.0, 1e-13);
    near(got.y, -3.0, 1e-13);
}

fn tmerc_zone(target: &str, lat: f64, lon0: f64, east0: f64, north: f64) {
    let proj = Proj::new_known_crs("EPSG:4326", target, None)
        .unwrap_or_else(|error| panic!("{target}: {error}"));
    for zone in 1..61 {
        let lon = lon0 + f64::from(zone - 1) * 6.0;
        let (east, got_north) = proj
            .convert((lon, lat))
            .unwrap_or_else(|error| panic!("zone {zone}: {error}"));
        near(east, east0 + f64::from(zone) * 1_000_000.0, 1e-4);
        near(got_north, north, 1e-4);
        let (back_lon, back_lat) = proj
            .project((east, got_north), true)
            .unwrap_or_else(|error| panic!("inverse {zone}: {error}"));
        near(back_lon, lon, 1e-8);
        near(back_lat, lat, 1e-8);
    }
}

#[test]
fn tmerczoned_west_boundary_equator_32600() {
    tmerc_zone("EPSG:32600", 0.0, -180.0, 166_021.443_1, 0.0);
}

#[test]
fn tmerczoned_west_boundary_high_latitude_32600() {
    tmerc_zone("EPSG:32600", 80.0, -180.0, 441_867.784_9, 8_883_084.955_9);
}

#[test]
fn tmerczoned_east_boundary_equator_32600() {
    tmerc_zone("EPSG:32600", 0.0, -174.000_000_001, 833_978.556_8, 0.0);
}

#[test]
fn tmerczoned_east_boundary_high_latitude_32600() {
    tmerc_zone(
        "EPSG:32600",
        80.0,
        -174.000_000_001,
        558_132.215_1,
        8_883_084.955_9,
    );
}

#[test]
fn tmerczoned_west_boundary_equator_32700() {
    tmerc_zone("EPSG:32700", 0.0, -180.0, 166_021.443_1, 10_000_000.0);
}

#[test]
fn tmerczoned_west_boundary_low_latitude_32700() {
    tmerc_zone("EPSG:32700", -80.0, -180.0, 441_867.784_9, 1_116_915.044_1);
}

#[test]
fn tmerczoned_east_boundary_equator_32700() {
    tmerc_zone(
        "EPSG:32700",
        0.0,
        -174.000_000_001,
        833_978.556_8,
        10_000_000.0,
    );
}

#[test]
fn tmerczoned_east_boundary_low_latitude_32700() {
    tmerc_zone(
        "EPSG:32700",
        -80.0,
        -174.000_000_001,
        558_132.215_1,
        1_116_915.044_1,
    );
}

#[test]
fn misc_version() {
    let info = ProjBuilder::new().lib_info().expect("info");
    assert_eq!((info.major, info.minor, info.patch), (9, 9, 0));
    assert_eq!(info.version, "9.9.0");
    assert_eq!(info.release, "9.9.0");
    let number = info.major * 10_000 + info.minor * 100 + info.patch;
    assert_eq!(number, 90_900);
    let at_least =
        |major, minor, patch| (info.major, info.minor, info.patch) >= (major, minor, patch);
    assert!(at_least(9, 9, 0));
    assert!(at_least(8, 9, 0));
    assert!(!at_least(9, 9, 1));
}

#[test]
fn util_name_factory_is_not_a_projection() {
    // ISO19111 NameFactory accepts "foo" and "bar/foo". Those strings are not
    // operations: creation fails instead of returning a coordinate.
    assert!(matches!(
        Operation::create("foo"),
        Err(CreateError::IllegalValue(_))
    ));
    assert!(Proj::new("foo").is_err());
    assert!(Proj::new("bar/foo").is_err());
    let crs = Proj::new("EPSG:4326").expect("epsg");
    let definition = crs.def().expect("definition");
    assert!(definition.contains("4326"), "{definition}");
}

#[test]
fn c_api_proj_create_rejects_unknown_and_builds_epsg() {
    assert!(Operation::create("invalid").is_err());
    assert!(Proj::new("invalid").is_err());
    assert!(Proj::new("EPSG:4326").is_ok());
    assert!(Proj::new("EPSG:i_do_not_exist").is_err());
}

#[test]
fn c_api_utm_zone_31_forward_and_invalid_zone() {
    assert!(UtmZone::try_new(0).is_err());
    assert!(Operation::create("+proj=utm +zone=0").is_err());
    let utm = Operation::create("+proj=utm +zone=1 +ellps=WGS84").expect("zone 1");
    assert!(utm.forward(Xyzt::xy(rad(-177.0), 0.0)).is_ok());
    let zone31 = Operation::create("+proj=utm +zone=31 +ellps=WGS84").expect("zone 31");
    let got = zone31.forward(Xyzt::xy(rad(3.0), 0.0)).expect("forward");
    near(got.x, 500_000.0, 1e-6);
}

#[test]
fn c_api_normalize_visualization_is_lon_lat_degrees() {
    assert!(Operation::create("+proj=utm +zone=31 +ellps=WGS84").is_ok());
    let pipeline = Operation::create(
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=utm +zone=31 +ellps=WGS84",
    )
    .expect("pipeline");
    assert!(pipeline.degree_output(true));
    assert!(!pipeline.degree_output(false));
    let known = Proj::new_known_crs("EPSG:4326", "EPSG:32631", None).expect("pair");
    let (east, _north) = known.convert((3.0, 0.0)).expect("convert");
    near(east, 500_000.0, 1e-4);
    let from = Proj::new("EPSG:4326").expect("source");
    let to = Proj::new("EPSG:32631").expect("target");
    let built = from
        .create_crs_to_crs_from_pj(&to, None, None)
        .expect("from pj");
    let (east, _) = built.convert((3.0, 0.0)).expect("built");
    near(east, 500_000.0, 1e-4);
    assert!(from
        .create_crs_to_crs_from_pj(&to, None, Some(vec!["ACCURACY=0.05"]))
        .is_err());
}

#[test]
fn c_api_laea_bounds() {
    let proj = Proj::new_known_crs(
        "EPSG:4326",
        "+proj=laea +lat_0=45 +lon_0=-100 +x_0=0 +y_0=0 +a=6370997 +b=6370997 +units=m +no_defs",
        None,
    )
    .expect("laea");
    let corners = proj
        .transform_bounds(-120.0, 40.0, -80.0, 64.0, 0)
        .expect("densify 0");
    near(corners[0], -1_684_649.413_38, 1.0);
    near(corners[1], -350_356.813_77, 1.0);
    near(corners[2], 1_684_649.413_38, 1.0);
    near(corners[3], 2_234_551.185_59, 1.0);
    let dense = proj
        .transform_bounds(-120.0, 40.0, -80.0, 64.0, 100)
        .expect("densify 100");
    near(dense[0], -1_684_649.413_38, 1.0);
    near(dense[1], -555_777.792_10, 1.0);
    near(dense[2], 1_684_649.413_38, 1.0);
    near(dense[3], 2_234_551.185_59, 1.0);
}

#[test]
fn c_api_geocentric_bounds_corners() {
    let cart = Operation::create("+proj=cart +ellps=WGS84").expect("cart");
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for lon in [2.0, 3.0] {
        for lat in [49.0, 50.0] {
            for height in [0.0, 100.0] {
                let got = cart
                    .forward(Xyzt {
                        x: rad(lon),
                        y: rad(lat),
                        z: height,
                        t: 0.0,
                    })
                    .expect("forward");
                for (slot, value) in [got.x, got.y, got.z].into_iter().enumerate() {
                    min[slot] = min[slot].min(value);
                    max[slot] = max[slot].max(value);
                }
            }
        }
    }
    near(min[0], 4_102_234.41, 1.0);
    near(min[1], 143_362.39, 1.0);
    near(max[0], 4_189_946.59, 1.0);
    near(max[1], 219_418.53, 1.0);
    near(min[2], 4_790_558.75, 1.0);
    near(max[2], 4_862_865.64, 1.0);
}

#[test]
fn c_api_time_dependent_operations() {
    let rates = Operation::create("+proj=helmert +dx=1 +t_epoch=2010").expect("rates");
    let at = rates
        .forward(Xyzt {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 2010.0,
        })
        .expect("epoch");
    let later = rates
        .forward(Xyzt {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 2011.0,
        })
        .expect("later");
    near(later.x - at.x, 1.0, 1e-9);
    let plain = Operation::create("+proj=helmert +x=1").expect("plain");
    let still = plain.forward(Xyzt::xy(0.0, 0.0)).expect("no time");
    near(still.x, 1.0, 1e-9);
    let deformation = Operation::create(
        "+proj=deformation +t_epoch=2010 +grids=tests/nkgrf03vel_realigned_extract.tif +ellps=GRS80",
    )
    .expect("deformation");
    assert!(deformation.forward(Xyzt::xy(rad(21.5), rad(63.0))).is_err());
    let dated = Operation::create(
        "+proj=deformation +dt=1 +grids=tests/nkgrf03vel_realigned_extract.tif +ellps=GRS80",
    )
    .expect("dt");
    let dated_result = dated.forward(Xyzt {
        x: rad(21.5),
        y: rad(63.0),
        z: 0.0,
        t: f64::NAN,
    });
    assert!(!matches!(
        dated_result,
        Err(proj_rs::StepError::MissingTime)
    ));
    let model = Operation::create("+proj=defmodel +model=tests/simple_model_degree_3d.json")
        .expect("defmodel");
    assert!(model
        .forward(Xyzt {
            x: rad(2.0),
            y: rad(49.0),
            z: 0.0,
            t: f64::NAN,
        })
        .is_err());
    assert!(model
        .forward(Xyzt {
            x: rad(2.0),
            y: rad(49.0),
            z: 0.0,
            t: 2020.0,
        })
        .is_ok());
}

#[test]
fn sch_matches_cs2cs_and_round_trips() {
    // cs2cs 9.9.0: `+proj=latlong +datum=WGS84 +to +proj=sch +plat_0=30
    // +plon_0=45 +phdg_0=-12 +ellps=WGS84 -f %.9f`, input `10 60 100`.
    let op =
        Operation::create("+proj=sch +plat_0=30 +plon_0=45 +phdg_0=-12 +ellps=WGS84").expect("sch");
    let input = Xyzt {
        x: rad(10.0),
        y: rad(60.0),
        z: 100.0,
        t: 0.0,
    };
    let got = op.forward(input).expect("forward");
    near(got.x, 4_111_770.028_648_399, 1e-6);
    near(got.y, 1_076_239.118_408_572, 1e-6);
    near(got.z, 3_769.576_684_201, 1e-6);
    let back = op.inverse(got).expect("inverse");
    near(back.x, input.x, 1e-12);
    near(back.y, input.y, 1e-12);
    near(back.z, input.z, 1e-6);
    assert!(Operation::create("+proj=sch +plat_0=30 +plon_0=45 +ellps=WGS84").is_err());
}
