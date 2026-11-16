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
        for x in [1e300, -1e300, f64::INFINITY, f64::NEG_INFINITY, 1e15, f64::MAX] {
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
    std::fs::write(&path, include_bytes!("fuzz_regressions/geotiff_empty_sample_format.tif"))
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
    for text in ["geoc-prok-\u{fffd}ch", "urn:ogc:def:é", "urn:ogc:de\u{fffd}f:x", "é"] {
        assert!(Operation::create(text).is_err(), "{text}");
        assert!(proj_rs::Proj::new(text).is_err(), "{text}");
    }
}

/// Found by `tools/parity/fuzz_diff.py --seed 1`: the `proj` application applied `+axis`, which
/// PROJ's `pj_fwd` and `pj_inv` do not (the pipeline path, `Operation::create`, does).
#[test]
fn projection_alone_ignores_the_axis_order() {
    let plain = Operation::create_projection("+proj=merc +ellps=WGS84").expect("merc");
    let swapped =
        Operation::create_projection("+proj=merc +ellps=WGS84 +axis=wnu").expect("merc wnu");
    let coord = proj_rs::Xyzt::xy(-0.29, 1.23);
    let (a, b) = (plain.forward(coord).expect("a"), swapped.forward(coord).expect("b"));
    assert_eq!((a.x, a.y), (b.x, b.y));
    let (back_a, back_b) = (plain.inverse(a).expect("a"), swapped.inverse(b).expect("b"));
    assert_eq!((back_a.x, back_a.y), (back_b.x, back_b.y));
    let piped = Operation::create("+proj=merc +ellps=WGS84 +axis=wnu").expect("merc wnu");
    let c = piped.forward(coord).expect("c");
    assert_eq!((c.x, c.y), (-a.x, a.y));
}
