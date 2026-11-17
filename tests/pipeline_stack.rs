//! The coordinate stack of `push` and `pop`. The transcripts in `tests/pipeline_stack/golden.txt`
//! are the bits of every value and the text of every error of pipelines that keep a stack, in
//! PROJ's way: one stack in the pipeline object, kept from one point to the next. They cover a
//! failing point in the middle of a sequence, nested and inverted steps, `omit_fwd` and `omit_inv`,
//! and chains that pop before they push, whose later points read what the earlier ones left on
//! the stack.
//!
//! Set `PIPELINE_STACK_CAPTURE=<file>` to write the transcripts instead of checking them.

use proj_rs::Proj;

const GOLDEN: &str = include_str!("pipeline_stack/golden.txt");

/// `ortho` refuses the far side of the globe: a point there fails after the `push` and before
/// the `pop`.
const FAR: f64 = 2.5;

fn points() -> Vec<(f64, f64, f64, f64)> {
    let ok = |i: f64| (0.01 * i, 0.4 + 0.02 * i, 100.0 + i, 2000.0 + 0.5 * i);
    vec![
        ok(1.0),
        (FAR, 0.2, 7.0, 8.0),
        ok(2.0),
        (-FAR, 0.3, 70.0, 80.0),
        (FAR, 0.4, 700.0, 800.0),
        ok(3.0),
        ok(4.0),
    ]
}

fn cell(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn forward(proj: &Proj) -> Vec<String> {
    points()
        .into_iter()
        .map(|(x, y, z, t)| match proj.convert_xyzt(x, y, z, t) {
            Ok((x, y, z, t)) => format!("ok {} {} {} {}", cell(x), cell(y), cell(z), cell(t)),
            Err(error) => format!("err {error}"),
        })
        .collect()
}

fn inverse(proj: &Proj) -> Vec<String> {
    points()
        .into_iter()
        .map(|(x, y, ..)| match proj.project((x, y), true) {
            Ok((x, y)) => format!("ok {} {}", cell(x), cell(y)),
            Err(error) => format!("err {error}"),
        })
        .collect()
}

/// The inverse of a projection reads metres: scale the sample so that it is inside the world.
fn inverse_metres(proj: &Proj) -> Vec<String> {
    let metres = [
        (1.0e5, 2.0e6),
        (3.0e5, 1.0e30),
        (-2.0e5, 3.0e6),
        (7.0e6, 1.0),
        (5.0e5, -4.0e6),
    ];
    metres
        .into_iter()
        .map(|(x, y)| match proj.project((x, y), true) {
            Ok((x, y)) => format!("ok {} {}", cell(x), cell(y)),
            Err(error) => format!("err {error}"),
        })
        .collect()
}

fn pipeline(definition: &str) -> Proj {
    Proj::new(definition).unwrap_or_else(|error| panic!("{definition}: {error}"))
}

fn known(from: &str, to: &str, input: &[(f64, f64)]) -> Vec<String> {
    let proj = Proj::new_known_crs(from, to, None).expect("pair");
    input
        .iter()
        .map(|&point| match proj.convert(point) {
            Ok((x, y)) => format!("ok {} {}", cell(x), cell(y)),
            Err(error) => format!("err {error}"),
        })
        .collect()
}

fn scenarios() -> Vec<(&'static str, Vec<String>)> {
    let mut out: Vec<(&'static str, Vec<String>)> = Vec::new();
    let merc = "+proj=ortho +ellps=GRS80";

    let balanced = pipeline(&format!(
        "+proj=pipeline +step +proj=push +v_3 +v_4 +step {merc} +step +proj=pop +v_3 +v_4"
    ));
    out.push(("balanced forward", forward(&balanced)));
    out.push(("balanced inverse", inverse_metres(&balanced)));

    let nested = pipeline(&format!(
        "+proj=pipeline +step +proj=push +v_3 +step +proj=push +v_4 \
         +step +proj=axisswap +order=1,2,4,3 +step +proj=push +v_3 +v_4 \
         +step +proj=set +v_3=5 +v_4=6 +step {merc} +step +proj=pop +v_4 +v_3 \
         +step +proj=pop +v_3 +step +proj=pop +v_4"
    ));
    out.push(("nested forward", forward(&nested)));
    out.push(("nested inverse", inverse_metres(&nested)));

    let lifo = pipeline(&format!(
        "+proj=pipeline +step +proj=push +v_3 +step +proj=set +v_3=5 +step +proj=push +v_3 \
         +step +proj=set +v_3=9 +step +proj=pop +v_3 +step +proj=axisswap +order=1,2,4,3 \
         +step {merc} +step +proj=pop +v_3"
    ));
    out.push(("last in first out forward", forward(&lifo)));
    out.push(("last in first out inverse", inverse_metres(&lifo)));

    let inverted_steps = pipeline(&format!(
        "+proj=pipeline +step +inv +proj=pop +v_3 +step {merc} +step +inv +proj=push +v_3"
    ));
    out.push(("inverted steps forward", forward(&inverted_steps)));
    out.push(("inverted steps inverse", inverse_metres(&inverted_steps)));

    let whole_inverted = pipeline(&format!(
        "+proj=pipeline +inv +step +proj=push +v_4 +step {merc} +step +proj=pop +v_4"
    ));
    out.push(("inverted pipeline forward", forward(&whole_inverted)));
    out.push(("inverted pipeline inverse", inverse(&whole_inverted)));

    let two_axes = pipeline(&format!(
        "+proj=pipeline +step +proj=push +v_1 +v_2 +step {merc} \
         +step +proj=set +v_1=1 +step +proj=pop +v_2 +step +proj=pop +v_1"
    ));
    out.push(("one axis popped by two steps forward", forward(&two_axes)));

    // Unbalanced: the stack of the object is read by later points.
    let pop_first = pipeline(
        "+proj=pipeline +step +proj=pop +v_3 +step +proj=axisswap +order=1,2,-3 \
         +step +proj=push +v_3",
    );
    out.push(("pop before push forward", forward(&pop_first)));
    out.push(("pop before push inverse", inverse(&pop_first)));

    let leftover = pipeline(&format!(
        "+proj=pipeline +step +proj=pop +v_3 +step +proj=push +v_3 +step {merc}"
    ));
    out.push((
        "a failing point leaves its push forward",
        forward(&leftover),
    ));

    let push_only = pipeline(&format!(
        "+proj=pipeline +step +proj=push +v_3 +step {merc} +step +proj=pop +v_3 +omit_fwd"
    ));
    out.push(("push only forward", forward(&push_only)));
    out.push(("push only inverse", inverse_metres(&push_only)));

    let balanced_forward_only = pipeline(&format!(
        "+proj=pipeline +step +proj=push +v_3 +step {merc} +step +proj=pop +v_3 +omit_inv"
    ));
    out.push((
        "balanced forward only, forward",
        forward(&balanced_forward_only),
    ));
    out.push((
        "balanced forward only, inverse",
        inverse_metres(&balanced_forward_only),
    ));
    out.push((
        "balanced forward only, forward again",
        forward(&balanced_forward_only),
    ));

    let deep = pipeline(&format!(
        "+proj=pipeline +step +proj=push +v_3 +step +proj=set +v_3=1 +step +proj=push +v_3 \
         +step +proj=set +v_3=2 +step +proj=push +v_3 +step +proj=set +v_3=3 +step +proj=push +v_3 \
         +step +proj=set +v_3=4 +step +proj=push +v_3 +step +proj=set +v_3=5 +step +proj=push +v_3 \
         +step {merc} +step +proj=pop +v_3 +step +proj=pop +v_3 +step +proj=pop +v_3 \
         +step +proj=pop +v_3 +step +proj=pop +v_3 +step +proj=pop +v_3"
    ));
    out.push(("six deep forward", forward(&deep)));

    let sweep: Vec<(f64, f64)> = vec![
        (52.1, 5.3),
        (91.0, 5.3),
        (52.2, 5.4),
        (f64::NAN, 5.3),
        (-89.9, 179.9),
        (52.3, 5.5),
    ];
    out.push((
        "EPSG:4326 to EPSG:28992",
        known("EPSG:4326", "EPSG:28992", &sweep),
    ));
    out.push((
        "EPSG:4326 to EPSG:27700",
        known("EPSG:4326", "EPSG:27700", &sweep),
    ));
    let projected: Vec<(f64, f64)> = vec![
        (155_000.0, 463_000.0),
        (1.0e30, 463_000.0),
        (156_000.0, 464_000.0),
        (f64::INFINITY, 1.0),
        (157_000.0, 465_000.0),
    ];
    out.push((
        "EPSG:28992 to EPSG:4326",
        known("EPSG:28992", "EPSG:4326", &projected),
    ));
    out
}

fn golden_section(name: &str) -> Vec<&'static str> {
    let header = format!("== {name}");
    let mut lines = GOLDEN.lines().skip_while(|line| *line != header);
    assert!(lines.next().is_some(), "no golden section {name}");
    lines.take_while(|line| !line.starts_with("== ")).collect()
}

#[test]
fn the_stack_of_a_pipeline_gives_the_same_points_and_errors() {
    let all = scenarios();
    if let Ok(file) = std::env::var("PIPELINE_STACK_CAPTURE") {
        let mut text = String::new();
        for (name, lines) in &all {
            text.push_str(&format!("== {name}\n"));
            for line in lines {
                text.push_str(line);
                text.push('\n');
            }
        }
        std::fs::write(file, text).expect("write the capture");
        return;
    }
    for (name, lines) in &all {
        assert_eq!(
            golden_section(name),
            lines.iter().map(String::as_str).collect::<Vec<_>>(),
            "{name}"
        );
    }
}

#[test]
fn proj_stays_send() {
    // `Step: Send + Sync` is a bound of the crate: a pipeline that keeps a stack must keep both,
    // and it only compiles while it does. `Proj` itself is `Send` and not `Sync`.
    fn is_send<T: Send>() {}
    is_send::<Proj>();
}

#[test]
fn a_pipeline_moves_to_another_thread_with_its_stack() {
    let proj = pipeline(
        "+proj=pipeline +step +proj=push +v_3 +v_4 +step +proj=ortho +ellps=GRS80 \
         +step +proj=pop +v_3 +v_4",
    );
    let want = forward(&proj);
    let got = std::thread::spawn(move || forward(&proj))
        .join()
        .expect("thread");
    assert_eq!(got, want);
}
