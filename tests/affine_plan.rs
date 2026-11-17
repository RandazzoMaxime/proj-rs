//! Runs of axis swaps and unit conversions are folded into one step when a pipeline is built
//! (`docs/PORTING-affine-plan.md`). A chain has to give what the steps gave one after the other
//! before they were folded: the same errors, the same text of the definition, the values to the
//! last bits of a product of two factors. `tests/affine_plan/golden.txt` holds what the code that
//! ran every step gave for a few hundred chains, forward and inverted.
//!
//! Set `AFFINE_PLAN_CAPTURE=<file>` to write the transcript instead of checking it.

use proj_rs::Proj;

/// One step of a chain: its text, with `+inv` and `omit_*` flags as the pipeline carries them.
#[derive(Clone)]
struct Step {
    text: &'static str,
    inverted: bool,
    omit_fwd: bool,
    omit_inv: bool,
}

/// Axis swaps and unit conversions the plan folds, and steps it must leave alone: a time unit,
/// a height finalised by `+z_0`, a step that is not affine (`noop`).
const POOL: &[&str] = &[
    "+proj=axisswap +order=2,1",
    "+proj=axisswap +order=-2,1",
    "+proj=axisswap +order=2,-1",
    "+proj=axisswap +order=3,1,2",
    "+proj=axisswap +order=2,-1,-3",
    "+proj=axisswap +order=4,3,2,1",
    "+proj=axisswap +order=-1,2,3,4",
    "+proj=axisswap +order=1,2,-3,4",
    "+proj=axisswap +axis=wsu",
    "+proj=unitconvert +xy_in=m +xy_out=ft",
    "+proj=unitconvert +xy_in=ft +xy_out=us-ft",
    "+proj=unitconvert +xy_in=km +xy_out=m +z_in=m +z_out=cm",
    "+proj=unitconvert +z_in=ft +z_out=m",
    "+proj=unitconvert +xy_in=deg +xy_out=grad",
    "+proj=unitconvert +xy_in=grad +xy_out=deg",
    "+proj=unitconvert +xy_in=deg +xy_out=rad",
    "+proj=unitconvert +xy_in=rad +xy_out=deg",
    "+proj=unitconvert +xy_in=rad +xy_out=rad +z_in=m +z_out=km",
    "+proj=unitconvert +t_in=decimalyear +t_out=mjd",
    "+proj=unitconvert +xy_in=deg +xy_out=rad +t_in=mjd +t_out=decimalyear",
    "+proj=unitconvert +xy_in=deg +xy_out=rad +z_0=3 +vto_meter=2",
    "+proj=noop",
];

/// Whether `+inv` stands on the header: the pipeline then runs the chain the other way.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Header {
    Plain,
    Inverted,
}

fn definition(steps: &[Step], header: Header) -> String {
    let mut text = String::from("+proj=pipeline");
    if header == Header::Inverted {
        text.push_str(" +inv");
    }
    for step in steps {
        text.push_str(" +step ");
        text.push_str(step.text);
        for (flag, on) in [
            ("+inv", step.inverted),
            ("+omit_fwd", step.omit_fwd),
            ("+omit_inv", step.omit_inv),
        ] {
            if on {
                text.push(' ');
                text.push_str(flag);
            }
        }
    }
    text
}

type Point = (f64, f64, f64, f64);

fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn chain(state: &mut u64) -> Vec<Step> {
    let length = 1 + usize::try_from(next(state) % 5).expect("small");
    (0..length)
        .map(|_| {
            let pick = usize::try_from(next(state)).expect("fits") % POOL.len();
            let flags = next(state) % 16;
            Step {
                text: POOL[pick],
                inverted: flags & 1 == 1,
                omit_fwd: flags == 6,
                omit_inv: flags == 10,
            }
        })
        .collect()
}

const SAMPLE: [Point; 5] = [
    (0.5, 0.25, 100.0, 2020.5),
    (-1.25, 2.0, -7.5, 1999.0),
    (200.0, -80.0, 0.0, 0.0),
    (3.0, 4.0, 1.0e5, 59_000.0),
    (50.0, 120.0, 3.0, 4.0),
];

fn agree(left: f64, right: f64) -> bool {
    left == right
        || (left.is_nan() && right.is_nan())
        || (left - right).abs() <= 1e-13 * left.abs().max(right.abs()).max(1e-300)
}

/// Chains whose result a step that the plan keeps decides: a longitude past π that the inverse of
/// `rad` to `deg` wraps, a time unit between two folded steps, a height finalised by `+z_0`, a
/// projection that refuses a point between two of them.
const FIXED: &[&str] = &[
    "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg \
     +xy_out=rad +step +proj=ortho +ellps=GRS80 +step +proj=unitconvert +xy_in=m +xy_out=km \
     +step +proj=axisswap +order=2,-1",
    "+proj=pipeline +inv +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg \
     +xy_out=rad +step +proj=ortho +ellps=GRS80 +step +proj=unitconvert +xy_in=m +xy_out=km \
     +step +proj=axisswap +order=2,-1",
    "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=rad +xy_out=deg \
     +step +proj=unitconvert +t_in=decimalyear +t_out=mjd +step +proj=axisswap +order=1,-2",
    "+proj=pipeline +inv +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=rad \
     +xy_out=deg +step +proj=unitconvert +t_in=decimalyear +t_out=mjd +step +proj=axisswap \
     +order=1,-2",
    "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +z_0=3 +vto_meter=2 \
     +step +proj=axisswap +order=2,1",
    "+proj=pipeline +inv +step +proj=unitconvert +xy_in=deg +xy_out=rad +z_0=3 +vto_meter=2 \
     +step +proj=axisswap +order=2,1",
    "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg \
     +xy_out=rad +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1",
    "+proj=pipeline +step +proj=unitconvert +xy_in=ft +xy_out=us-ft +inv \
     +step +proj=unitconvert +xy_in=ft +xy_out=us-ft",
    "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=push +v_3 \
     +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=axisswap +order=2,1 \
     +step +proj=pop +v_3 +step +proj=unitconvert +xy_in=rad +xy_out=deg",
];

fn definitions() -> Vec<String> {
    let mut out: Vec<String> = FIXED.iter().map(|text| (*text).to_owned()).collect();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    for _ in 0..200 {
        let steps = chain(&mut state);
        out.push(definition(&steps, Header::Plain));
        out.push(definition(&steps, Header::Inverted));
    }
    out
}

fn cell(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn transcript() -> Vec<String> {
    let mut lines = Vec::new();
    for text in definitions() {
        let Ok(pipeline) = Proj::new(&text) else {
            lines.push("new err".to_owned());
            continue;
        };
        for (x, y, z, t) in SAMPLE {
            lines.push(match pipeline.convert_xyzt(x, y, z, t) {
                Ok((x, y, z, t)) => format!("ok {} {} {} {}", cell(x), cell(y), cell(z), cell(t)),
                Err(error) => format!("err {error}"),
            });
        }
    }
    lines
}

fn bits(text: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(text, 16).expect("hex"))
}

fn same_line(got: &str, want: &str) -> bool {
    let (mut got_words, mut want_words) = (got.split(' '), want.split(' '));
    if got_words.next() != Some("ok") || want_words.next() != Some("ok") {
        return got == want;
    }
    got_words
        .zip(want_words)
        .all(|(got, want)| agree(bits(got), bits(want)))
}

#[test]
fn a_chain_gives_what_its_steps_gave_one_after_the_other() {
    let got = transcript();
    if let Ok(file) = std::env::var("AFFINE_PLAN_CAPTURE") {
        std::fs::write(file, format!("{}\n", got.join("\n"))).expect("write the capture");
        return;
    }
    let want: Vec<&str> = include_str!("affine_plan/golden.txt").lines().collect();
    assert_eq!(got.len(), want.len());
    let mut identical = 0;
    for (index, (got, want)) in got.iter().zip(&want).enumerate() {
        assert!(
            same_line(got, want),
            "line {index}\n got  {got}\n want {want}"
        );
        identical += usize::from(got == want);
    }
    // A product of two factors may move the last bit; a chain of one factor and signs may not.
    assert!(
        identical * 10 > want.len() * 9,
        "{identical} of {} identical",
        want.len()
    );
}

#[test]
fn the_definition_of_a_pair_is_the_text_of_its_steps() {
    let defined = |from: &str, to: &str| {
        let proj = Proj::new_known_crs(from, to, None).expect("pair");
        (
            proj.def().expect("definition"),
            proj.definition_in_authority_order()
                .expect("authority order"),
        )
    };
    let (def, authority) = defined("EPSG:4326", "EPSG:2154");
    assert_eq!(
        def,
        "proj=pipeline step proj=unitconvert xy_in=deg xy_out=rad step proj=lcc lat_0=46.5 \
         lon_0=3 lat_1=49 lat_2=44 x_0=700000 y_0=6600000 ellps=GRS80"
    );
    assert_eq!(
        authority,
        "proj=pipeline step proj=axisswap order=2,1 step proj=unitconvert xy_in=deg xy_out=rad \
         step proj=lcc lat_0=46.5 lon_0=3 lat_1=49 lat_2=44 x_0=700000 y_0=6600000 ellps=GRS80"
    );
    let (def, authority) = defined("EPSG:2154", "EPSG:4326");
    assert_eq!(
        def,
        "proj=pipeline step inv proj=lcc lat_0=46.5 lon_0=3 lat_1=49 lat_2=44 x_0=700000 \
         y_0=6600000 ellps=GRS80 step proj=unitconvert xy_in=rad xy_out=deg"
    );
    assert_eq!(authority, format!("{def} step proj=axisswap order=2,1"));
    let text = "+proj=pipeline +step +proj=axisswap +order=2,1 \
                +step +proj=unitconvert +xy_in=deg +xy_out=rad \
                +step +proj=unitconvert +xy_in=rad +xy_out=deg \
                +step +proj=axisswap +order=2,1";
    assert_eq!(
        Proj::new(text)
            .expect("pipeline")
            .def()
            .expect("definition"),
        "proj=pipeline step proj=axisswap order=2,1 step proj=unitconvert xy_in=deg xy_out=rad \
         step proj=unitconvert xy_in=rad xy_out=deg step proj=axisswap order=2,1"
    );
}

/// The vector path of a step (`ortho` has none, the others may) is not the scalar one to the bit.
fn close(left: (f64, f64), right: (f64, f64)) -> bool {
    [(left.0, right.0), (left.1, right.1)]
        .iter()
        .all(|(a, b)| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0))
}

#[test]
fn a_slice_through_a_chain_gives_what_the_points_one_by_one_give() {
    let text = "+proj=pipeline +step +proj=axisswap +order=2,1 \
                +step +proj=unitconvert +xy_in=deg +xy_out=rad \
                +step +proj=ortho +ellps=GRS80 \
                +step +proj=unitconvert +xy_in=m +xy_out=ft \
                +step +proj=axisswap +order=2,-1";
    let proj = Proj::new(text).expect("pipeline");
    let points: Vec<(f64, f64)> = (0..37)
        .map(|index| {
            let index = f64::from(index);
            (10.0 + 0.7 * index, -20.0 + 1.1 * index)
        })
        .collect();
    let mut sliced = points.clone();
    proj.project_array(&mut sliced, false).expect("slice");
    for (point, got) in points.iter().zip(&sliced) {
        let want = proj.project(*point, false).expect("point");
        assert!(close(*got, want), "{got:?} {want:?}");
    }
    let mut back = sliced.clone();
    proj.project_array(&mut back, true).expect("inverse slice");
    for (point, got) in sliced.iter().zip(&back) {
        let want = proj.project(*point, true).expect("point");
        assert!(close(*got, want), "{got:?} {want:?}");
    }
}
