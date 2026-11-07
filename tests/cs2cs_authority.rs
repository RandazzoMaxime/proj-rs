//! `cs2cs --authority` against PROJ 9.9.0 (`cs2cs.cpp:502-509` and `965-972`,
//! `crs_to_crs.cpp:505-508`, `c_api.cpp:8049-8072`). The option restricts the candidate
//! operations to one authority's registry. `PROJ` has no operation between ITRF96 and ITRF2014,
//! so the answer is the null offset (`+proj=noop`), not the EPSG:8076 operation.
//!
//! The expected values are the output of the PROJ 9.9.0 `cs2cs` binary, run with `-f %.9f`.
//! The authority name is compared without case, as `AuthorityFactory::create` does
//! (`factory.cpp:4400-4417`): `epsg` is `EPSG`.

use std::io::Write;
use std::process::{Command, Stdio};

use proj_rs::iso19111::{best_pipeline_with_authority, BestOperation, OnlyBest};

const FORMAT: &[&str] = &["-f", "%.9f"];

/// The three fields of the one output line of `cs2cs` for `input`.
fn run(args: &[&str], input: &str) -> [f64; 3] {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cs2cs"));
    command
        .args(args)
        .args(FORMAT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn cs2cs");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let fields: Vec<f64> = stdout
        .lines()
        .next()
        .expect("one output line")
        .split_whitespace()
        .map(|field| field.parse::<f64>().expect("number"))
        .collect();
    assert_eq!(fields.len(), 3, "three output fields for {args:?}");
    [fields[0], fields[1], fields[2]]
}

/// Each axis is within 1e-8 of PROJ's value, the print precision of `-f %.9f` on degrees.
fn assert_point(args: &[&str], input: &str, expected: [f64; 3]) {
    let got = run(args, input);
    for axis in 0..3 {
        assert!(
            (got[axis] - expected[axis]).abs() <= 1e-8,
            "axis {axis} of {args:?}: got {}, PROJ {}",
            got[axis],
            expected[axis]
        );
    }
}

/// No option: the EPSG:8076 operation, as PROJ picks it.
#[test]
fn itrf96_without_authority_uses_the_epsg_operation() {
    assert_point(
        &["ITRF96", "ITRF2014"],
        "49 2\n",
        [49.000_000_421, 1.999_999_938, 0.018_364_660],
    );
}

/// `--authority PROJ`: no PROJ operation links the two frames, and the null offset is the answer.
#[test]
fn authority_proj_has_no_operation_so_the_offset_is_null() {
    assert_point(
        &["--authority", "PROJ", "ITRF96", "ITRF2014"],
        "49 2\n",
        [49.0, 2.0, 0.0],
    );
}

/// `--authority NONEXISTENT`: no authority of that name holds an operation either.
#[test]
fn unknown_authority_leaves_no_candidate() {
    assert_point(
        &["--authority", "NONEXISTENT", "ITRF96", "ITRF2014"],
        "49 2\n",
        [49.0, 2.0, 0.0],
    );
}

/// `--authority EPSG` and `--authority any` both keep the EPSG candidates.
#[test]
fn authority_epsg_and_any_keep_the_epsg_operation() {
    let expected = [49.000_000_421, 1.999_999_938, 0.018_364_660];
    assert_point(
        &["--authority", "EPSG", "ITRF96", "ITRF2014"],
        "49 2\n",
        expected,
    );
    assert_point(
        &["--authority", "any", "ITRF96", "ITRF2014"],
        "49 2\n",
        expected,
    );
}

/// The authority name is spelled as PROJ spells it: `epsg` is `EPSG`.
#[test]
fn authority_name_is_case_insensitive() {
    assert_point(
        &["--authority", "epsg", "ITRF96", "ITRF2014"],
        "49 2\n",
        [49.000_000_421, 1.999_999_938, 0.018_364_660],
    );
}

/// ED50 to ETRS89: the EPSG operation is a shift of about 0.9 m, and `PROJ` keeps the input.
#[test]
fn authority_proj_keeps_the_input_of_ed50_to_etrs89() {
    assert_point(
        &["EPSG:4230", "EPSG:4258"],
        "50 4\n",
        [49.999_138_141, 3.998_732_117, 0.0],
    );
    assert_point(
        &["--authority", "PROJ", "EPSG:4230", "EPSG:4258"],
        "50 4\n",
        [50.0, 4.0, 0.0],
    );
    assert_point(
        &["--authority", "EPSG", "EPSG:4230", "EPSG:4258"],
        "50 4\n",
        [49.999_138_141, 3.998_732_117, 0.0],
    );
}

/// Belgian Lambert 72 to WGS 84: the `PROJ` authority gives a different operation from the
/// EPSG one, and `epsg` gives the EPSG one.
#[test]
fn authority_chooses_between_the_registries_for_belgian_lambert() {
    assert_point(
        &["EPSG:31370", "EPSG:4326"],
        "150000 170000\n",
        [50.840_411_283, 4.368_752_141, 0.0],
    );
    assert_point(
        &["--authority", "PROJ", "EPSG:31370", "EPSG:4326"],
        "150000 170000\n",
        [50.840_956_191, 4.367_486_482, 0.0],
    );
    assert_point(
        &["--authority", "epsg", "EPSG:31370", "EPSG:4326"],
        "150000 170000\n",
        [50.840_411_283, 4.368_752_141, 0.0],
    );
}

/// The library selection takes the same name as `cs2cs`: `epsg` and `EPSG` choose the same
/// operation, and `PROJ` chooses another one.
#[test]
fn library_selection_canonicalizes_the_authority_name() {
    let lower = best_pipeline_with_authority("EPSG:31370", "EPSG:4326", OnlyBest::Off, "epsg")
        .expect("epsg");
    let upper = best_pipeline_with_authority("EPSG:31370", "EPSG:4326", OnlyBest::Off, "EPSG")
        .expect("EPSG");
    assert_eq!(lower, upper);
    let proj = best_pipeline_with_authority("EPSG:31370", "EPSG:4326", OnlyBest::Off, "PROJ")
        .expect("PROJ");
    assert_ne!(proj, upper);
    assert!(matches!(proj, BestOperation::Pipeline { .. }));
}
