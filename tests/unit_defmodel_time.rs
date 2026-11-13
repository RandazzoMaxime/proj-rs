//! `test/unit/test_defmodel.cpp`: `defmodel.evaluate_constant`,
//! `evaluate_velocity`, `evaluate_step`, `evaluate_reverse_step`,
//! `evaluate_piecewise` and `evaluate_exponential`. The time functions are
//! evaluated through `TimeFunction::evaluate_at`, as `ComponentEx` does.
//! Expected values are PROJ 9.9.0's.

#[path = "common/defmodel_json.rs"]
#[expect(
    dead_code,
    reason = "this file does not use the removal and key helpers"
)]
mod defmodel_json;

use defmodel_json::{
    arr, full_valid, n, obj, s, set, Json, IDX_CONSTANT, IDX_EXPONENTIAL, IDX_PIECEWISE,
    IDX_REVERSE_STEP, IDX_STEP, IDX_VELOCITY,
};
use proj_rs::defmodel::MasterFile;

fn parse(j: &Json) -> MasterFile {
    MasterFile::parse(&j.dump()).expect("valid model")
}

/// `components[index].time_function.parameters[key] = value`, where the
/// parameter of the piecewise function is `key`.
fn piecewise_param(j: &mut Json, key: &str, value: Json) {
    let path = ["components", "4", "time_function", "parameters", key];
    set(j, &path, value);
}

#[test]
fn defmodel_evaluate_constant() {
    let mf = parse(&full_valid());
    let time_function = mf.components()[IDX_CONSTANT].time_function();
    assert_eq!(time_function.evaluate_at(1999.0), 1.0);
    assert_eq!(time_function.evaluate_at(2000.0), 1.0);
    assert_eq!(time_function.evaluate_at(2001.0), 1.0);
}

#[test]
fn defmodel_evaluate_velocity() {
    let mf = parse(&full_valid());
    let time_function = mf.components()[IDX_VELOCITY].time_function();
    assert_eq!(time_function.evaluate_at(1999.0), -1.0);
    assert_eq!(time_function.evaluate_at(2000.0), 0.0);
    assert_eq!(time_function.evaluate_at(2001.0), 1.0);
}

#[test]
fn defmodel_evaluate_step() {
    let mf = parse(&full_valid());
    let time_function = mf.components()[IDX_STEP].time_function();
    assert_eq!(time_function.evaluate_at(1999.99), 0.0);
    assert_eq!(time_function.evaluate_at(2000.00), 1.0);
    assert_eq!(time_function.evaluate_at(2000.01), 1.0);
}

#[test]
fn defmodel_evaluate_reverse_step() {
    let mf = parse(&full_valid());
    let time_function = mf.components()[IDX_REVERSE_STEP].time_function();
    assert_eq!(time_function.evaluate_at(1999.99), -1.0);
    assert_eq!(time_function.evaluate_at(2000.00), 0.0);
    assert_eq!(time_function.evaluate_at(2000.01), 0.0);
}

#[test]
fn defmodel_evaluate_piecewise() {
    let j_full_valid = full_valid();
    {
        let mf = parse(&j_full_valid);
        let time_function = mf.components()[IDX_PIECEWISE].time_function();
        assert_eq!(time_function.evaluate_at(2015.99), 0.0);
        assert_eq!(time_function.evaluate_at(2016.00), 0.5);
        assert_eq!(time_function.evaluate_at(2016.5), 0.75);
        assert!((time_function.evaluate_at(2017.0 - 1e-9) - 1.0).abs() < 1e-9);
        assert_eq!(time_function.evaluate_at(2017.0), 2.0);
        assert_eq!(time_function.evaluate_at(2017.5), 1.5);
        assert_eq!(time_function.evaluate_at(2018.0), 1.0);
        assert_eq!(time_function.evaluate_at(2019.0), 1.0);
    }

    // before_first
    for (before, t, expected) in [
        ("zero", 2015.5, 0.0),
        ("constant", 2015.5, 0.5),
        ("linear", 2015.5, 0.25),
    ] {
        let mut j = j_full_valid.clone();
        piecewise_param(&mut j, "before_first", s(before));
        let mf = parse(&j);
        assert_eq!(
            mf.components()[IDX_PIECEWISE]
                .time_function()
                .evaluate_at(t),
            expected,
            "before_first = {before}"
        );
    }

    // after_last
    for (after, t, expected) in [
        ("zero", 2018.5, 0.0),
        ("constant", 2018.5, 1.0),
        ("linear", 2018.5, 0.5),
    ] {
        let mut j = j_full_valid.clone();
        piecewise_param(&mut j, "after_last", s(after));
        let mf = parse(&j);
        assert_eq!(
            mf.components()[IDX_PIECEWISE]
                .time_function()
                .evaluate_at(t),
            expected,
            "after_last = {after}"
        );
    }

    // No epoch
    {
        let mut j = j_full_valid.clone();
        piecewise_param(&mut j, "model", arr(vec![]));
        let mf = parse(&j);
        assert_eq!(
            mf.components()[IDX_PIECEWISE]
                .time_function()
                .evaluate_at(2015.5),
            0.0
        );
    }

    // Just one epoch
    {
        let mut j = j_full_valid.clone();
        piecewise_param(
            &mut j,
            "model",
            arr(vec![obj(vec![
                ("epoch", s("2016-01-01T00:00:00Z")),
                ("scale_factor", n(0.5)),
            ])]),
        );
        piecewise_param(&mut j, "before_first", s("linear"));
        piecewise_param(&mut j, "after_last", s("linear"));
        let mf = parse(&j);
        let time_function = mf.components()[IDX_PIECEWISE].time_function();
        assert_eq!(time_function.evaluate_at(2015.5), 0.5);
        assert_eq!(time_function.evaluate_at(2016.5), 0.5);
    }

    // Two identical epochs
    {
        let mut j = j_full_valid.clone();
        piecewise_param(
            &mut j,
            "model",
            arr(vec![
                obj(vec![
                    ("epoch", s("2016-01-01T00:00:00Z")),
                    ("scale_factor", n(0.5)),
                ]),
                obj(vec![
                    ("epoch", s("2016-01-01T00:00:00Z")),
                    ("scale_factor", n(1.0)),
                ]),
            ]),
        );
        piecewise_param(&mut j, "before_first", s("linear"));
        piecewise_param(&mut j, "after_last", s("linear"));
        let mf = parse(&j);
        let time_function = mf.components()[IDX_PIECEWISE].time_function();
        assert_eq!(time_function.evaluate_at(2015.5), 0.5);
        assert_eq!(time_function.evaluate_at(2016.5), 1.0);
    }
}

#[test]
fn defmodel_evaluate_exponential() {
    let mf = parse(&full_valid());
    let time_function = mf.components()[IDX_EXPONENTIAL].time_function();
    assert_eq!(time_function.evaluate_at(1999.99), 0.0);
    assert_eq!(time_function.evaluate_at(2000.00), 1.0);
    assert_eq!(
        time_function.evaluate_at(2000.50),
        1.0 + (3.0 - 1.0) * (1.0 - (-(2000.50 - 2000.00) / 2.0_f64).exp())
    );
    assert_eq!(
        time_function.evaluate_at(2001.00),
        1.0 + (3.0 - 1.0) * (1.0 - (-(2001.00 - 2000.00) / 2.0_f64).exp())
    );
    assert_eq!(
        time_function.evaluate_at(2002.00),
        1.0 + (3.0 - 1.0) * (1.0 - (-(2001.00 - 2000.00) / 2.0_f64).exp())
    );
}
