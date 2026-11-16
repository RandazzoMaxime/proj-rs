//! Reproductions of the crashes `fuzz/` found (docs/FUZZ.md). Each test names its cause, and
//! returns instead of panicking, hanging or exhausting memory.

mod common;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use proj_rs::Operation;

/// Run `work` on a thread and fail when it does not return in ten seconds: a loop that cannot end
/// would otherwise stall the whole suite.
fn within_ten_seconds<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("the call did not return in ten seconds")
}

/// `Evaluator::forward` wrapped a longitude into the model's range one turn at a time: a
/// longitude of 1e300 radians, or infinity, never reached it.
#[test]
fn defmodel_forward_returns_for_huge_longitude() {
    common::use_fixture_search_path();
    within_ten_seconds(|| {
        let op = Operation::create("+proj=defmodel +model=tests/simple_model_polar.json")
            .expect("the polar test model builds");
        for x in [
            1e300,
            -1e300,
            f64::INFINITY,
            f64::NEG_INFINITY,
            1e15,
            f64::MAX,
        ] {
            let coord = proj_rs::Xyzt {
                x,
                y: -1.55,
                z: 0.0,
                t: 2020.0,
            };
            assert!(op.forward(coord).is_err(), "forward of {x}");
            assert!(op.inverse(coord).is_err(), "inverse of {x}");
        }
    });
}

/// A TIFF whose `SampleFormat` tag holds no value: `tiff` 0.10.3 indexed `sample_format[0]` and
/// panicked while the grid was opened.
#[test]
fn geotiff_with_empty_sample_format_is_an_error() {
    common::use_fixture_search_path();
    let path = std::env::temp_dir().join(format!("proj-rs-fuzz-sample-{}.tif", std::process::id()));
    std::fs::write(
        &path,
        include_bytes!("fuzz_regressions/geotiff_empty_sample_format.tif"),
    )
    .expect("write the fixture");
    let name = path.display().to_string();
    let horizontal = proj_rs::grids::HorizontalShiftGridSet::open(&name);
    let vertical = proj_rs::grids::VerticalShiftGridSet::open(&name);
    let _ = std::fs::remove_file(&path);
    assert!(horizontal.is_err());
    assert!(vertical.is_err());
}

/// `strip_urn_prefix` cut the text at the prefix length: a multi-byte character straddling it
/// ("geoc-prok-" then U+FFFD) is not a char boundary, and the slice panicked.
#[test]
fn proj_string_with_multibyte_character_at_the_urn_prefix_length() {
    common::use_fixture_search_path();
    for text in [
        "geoc-prok-\u{fffd}ch",
        "urn:ogc:def:é",
        "urn:ogc:de\u{fffd}f:x",
        "é",
    ] {
        assert!(Operation::create(text).is_err(), "{text}");
        assert!(proj_rs::Proj::new(text).is_err(), "{text}");
    }
}

/// Found by `fuzz_diff.py --seed 20261119`: without `+lat_1`, PROJ's `lcc` takes 0 (not
/// `lat_0`), and with `+lat_1` alone and no `+lat_0` the origin latitude is `lat_1`. Values from
/// PROJ 9.9.0 `cct` on WGS72.
#[test]
fn lcc_defaults_of_lat_1_and_lat_0_follow_proj() {
    let at = |definition: &str, lon: f64, lat: f64| {
        let op = Operation::create(&format!("+proj=lcc +ellps=WGS72 {definition}")).expect("lcc");
        let out = op
            .forward(proj_rs::Xyzt::xy(lon.to_radians(), lat.to_radians()))
            .expect("forward");
        (out.x, out.y)
    };
    let (x, y) = at("+lat_0=-2.2 +lat_2=-40.6", 10.0, 20.0);
    assert!((x - 1_261_387.548_701).abs() < 1e-3 && (y - 2_609_023.785_546).abs() < 1e-3);
    let (x, y) = at("+lat_1=30", 10.0, 20.0);
    assert!((x - 1_060_664.030_917).abs() < 1e-3 && (y + 1_066_935.623_583).abs() < 1e-3);
}

/// A CTable2 header of 2^31-1 by 2^31-1 nodes: `count * 8` overflowed in the size check.
#[test]
fn ctable2_with_huge_dimensions_is_an_error() {
    common::use_fixture_search_path();
    let mut bytes = vec![0_u8; 200];
    bytes[..16].copy_from_slice(b"CTABLE V2.0     ");
    bytes[96..104].copy_from_slice(&0.0_f64.to_le_bytes());
    bytes[104..112].copy_from_slice(&0.0_f64.to_le_bytes());
    bytes[112..120].copy_from_slice(&0.01_f64.to_le_bytes());
    bytes[120..128].copy_from_slice(&0.01_f64.to_le_bytes());
    bytes[128..132].copy_from_slice(&i32::MAX.to_le_bytes());
    bytes[132..136].copy_from_slice(&i32::MAX.to_le_bytes());
    let path = std::env::temp_dir().join(format!("proj-rs-fuzz-ct2-{}.ct2", std::process::id()));
    std::fs::write(&path, &bytes).expect("write the fixture");
    let result = proj_rs::grids::HorizontalShiftGridSet::open(&path.display().to_string());
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

/// Found by `tools/parity/fuzz_diff.py --seed 1`: the `proj` application applied `+axis`, which
/// PROJ's `pj_fwd` and `pj_inv` do not (the pipeline path, `Operation::create`, does).
#[test]
fn projection_alone_ignores_the_axis_order() {
    let plain = Operation::create_projection("+proj=merc +ellps=WGS84").expect("merc");
    let swapped =
        Operation::create_projection("+proj=merc +ellps=WGS84 +axis=wnu").expect("merc wnu");
    let coord = proj_rs::Xyzt::xy(-0.29, 1.23);
    let (a, b) = (
        plain.forward(coord).expect("a"),
        swapped.forward(coord).expect("b"),
    );
    assert_eq!((a.x, a.y), (b.x, b.y));
    let (back_a, back_b) = (plain.inverse(a).expect("a"), swapped.inverse(b).expect("b"));
    assert_eq!((back_a.x, back_a.y), (back_b.x, back_b.y));
    let piped = Operation::create("+proj=merc +ellps=WGS84 +axis=wnu").expect("merc wnu");
    let c = piped.forward(coord).expect("c");
    assert_eq!((c.x, c.y), (-a.x, a.y));
}
