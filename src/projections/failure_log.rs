//! The line PROJ logs when `fwd_prepare` refuses a point (`fwd.cpp:39-80`): `<step>: Invalid
//! latitude` or `<step>: Invalid longitude`, where `<step>` is the short name of the operation
//! that read the angles. Only an operation that is run forward, does not skip `fwd_prepare`
//! (`unitconvert`, `axisswap`, a pipeline) and reads radians on its left is checked. The line is
//! worked out after a failure, by running the steps again, so a point that succeeds costs nothing.

use super::{Coord, IoKind, PipeStep, PipelineStack, Step, EPS_LAT};
use std::f64::consts::{FRAC_PI_2, PI};

/// What `fwd_prepare` does with a point.
enum Gate {
    /// The point goes on.
    Pass,
    /// The point is refused with this text logged at error level.
    Logged(&'static str),
    /// The point is refused without a log line (`HUGE_VAL` already in the point).
    Silent,
}

/// The checks of `fwd_prepare` on radians, in its order.
fn gate(coord: Coord) -> Gate {
    if coord.y.abs() > FRAC_PI_2 {
        if coord.x == f64::INFINITY || coord.y == f64::INFINITY {
            return Gate::Silent;
        }
        let beyond = if coord.y > 0.0 {
            coord.y - FRAC_PI_2 > EPS_LAT
        } else {
            coord.y - FRAC_PI_2 < -EPS_LAT
        };
        if beyond {
            return Gate::Logged("Invalid latitude");
        }
    }
    if coord.x.abs() > PI && coord.x.abs() > 10.0 {
        return Gate::Logged("Invalid longitude");
    }
    if coord.z == f64::INFINITY {
        return Gate::Silent;
    }
    Gate::Pass
}

/// The operations whose `skip_fwd_prepare` is set (`unitconvert.cpp`, `axisswap.cpp`,
/// `pipeline.cpp`).
fn skips_prepare(name: &str) -> bool {
    matches!(name, "unitconvert" | "axisswap" | "pipeline")
}

/// The left units of each step once a pipeline has replaced `PJ_IO_UNITS_WHATEVER` with those of
/// its neighbours (`pipeline.cpp:581-610`): a step whose two sides are `Any` takes the left side
/// of the step after it, then the right side of the step before it.
fn resolved_lefts(steps: &[PipeStep]) -> Vec<IoKind> {
    let mut units: Vec<(IoKind, IoKind)> = steps
        .iter()
        .map(|step| (step.inner.input_kind(), step.inner.output_kind()))
        .collect();
    let left = |unit: (IoKind, IoKind), inverted: bool| if inverted { unit.1 } else { unit.0 };
    let right = |unit: (IoKind, IoKind), inverted: bool| if inverted { unit.0 } else { unit.1 };
    let whatever = |unit: (IoKind, IoKind)| unit.0 == IoKind::Any && unit.1 == IoKind::Any;
    for index in (0..steps.len().saturating_sub(1)).rev() {
        if whatever(units[index]) {
            let next = units[index + 1];
            let (next_left, next_right) = (
                left(next, steps[index + 1].inverted),
                right(next, steps[index + 1].inverted),
            );
            if next_left != next_right || next_left != IoKind::Any {
                units[index] = (next_left, next_left);
            }
        }
    }
    for index in 1..steps.len() {
        if whatever(units[index]) {
            let previous = units[index - 1];
            let (previous_left, previous_right) = (
                left(previous, steps[index - 1].inverted),
                right(previous, steps[index - 1].inverted),
            );
            if previous_left != previous_right || previous_right != IoKind::Any {
                units[index] = (previous_right, previous_right);
            }
        }
    }
    units.into_iter().map(|unit| unit.0).collect()
}

/// The line of a pipeline whose run refused `coord`: the steps run again one by one, each first
/// through `fwd_prepare` when it applies. `None` when no step logs.
pub(super) fn pipeline_failure(
    steps: &[PipeStep],
    coord: Coord,
    inverse_chain: bool,
) -> Option<String> {
    let lefts = resolved_lefts(steps);
    let order: Vec<usize> = if inverse_chain {
        (0..steps.len())
            .rev()
            .filter(|index| !steps[*index].omit_inv)
            .collect()
    } else {
        (0..steps.len())
            .filter(|index| !steps[*index].omit_fwd)
            .collect()
    };
    let mut stack = PipelineStack::new();
    let mut coord = coord;
    for index in order {
        let step = &steps[index];
        let inverse = step.inverted != inverse_chain;
        if !inverse && !skips_prepare(&step.name) && lefts[index] == IoKind::Angular {
            match gate(coord) {
                Gate::Pass => {}
                Gate::Logged(text) => return Some(format!("{}: {text}", step.name)),
                Gate::Silent => return None,
            }
        }
        coord = if inverse {
            step.inner.inverse_in_pipeline(coord, &mut stack)
        } else {
            step.inner.forward_in_pipeline(coord, &mut stack)
        }
        .ok()?;
    }
    None
}

/// The line of a single operation named `name` that refused `coord` in its forward direction.
pub(super) fn operation_failure(name: &str, step: &dyn Step, coord: Coord) -> Option<String> {
    if skips_prepare(name) || step.input_kind() != IoKind::Angular {
        return None;
    }
    match gate(coord) {
        Gate::Logged(text) => Some(format!("{name}: {text}")),
        Gate::Pass | Gate::Silent => None,
    }
}
