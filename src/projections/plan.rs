//! The execution plan of a pipeline: the order its steps run in, with each run of affine steps
//! folded into one (`docs/PORTING-affine-plan.md`). The steps themselves, which the definition
//! and the units of the pipeline read, are not touched.

use super::{Coord, PipeStep, PipelineStack, Step, TransformError};
use std::sync::Arc;

/// A signed, scaled permutation of the four axes: `out[k] = in[src[k]] * mul[k]`.
///
/// What an axis swap and a unit conversion without time are, and any chain of them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Affine {
    src: [usize; 4],
    mul: [f64; 4],
    shape: Shape,
}

/// The two permutations that make nearly every chain, which `apply` takes without indexing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    InPlace,
    SwapXy,
    Any,
}

impl Affine {
    const IDENTITY: Self = Self {
        src: [0, 1, 2, 3],
        mul: [1.0; 4],
        shape: Shape::InPlace,
    };

    /// `out[k] = in[src[k]] * mul[k]`. Every `src[k]` is below 4.
    pub(crate) fn new(src: [usize; 4], mul: [f64; 4]) -> Self {
        debug_assert!(src.iter().all(|index| *index < 4));
        let shape = match src {
            [0, 1, 2, 3] => Shape::InPlace,
            [1, 0, 2, 3] => Shape::SwapXy,
            _ => Shape::Any,
        };
        Self { src, mul, shape }
    }

    /// `x` and `y` times `xy`, `z` times `z`, the time as it is.
    pub(crate) fn scale(xy: f64, z: f64) -> Self {
        Self::new([0, 1, 2, 3], [xy, xy, z, 1.0])
    }

    /// `self` first, then `next`. The factors of the two multiply: for a single factor and
    /// signs this is exact, for two factors the product may differ from the two products in
    /// the last bit.
    fn then(self, next: Self) -> Self {
        let mut src = [0; 4];
        let mut mul = [0.0; 4];
        for axis in 0..4 {
            let from = next.src[axis];
            src[axis] = self.src[from];
            mul[axis] = self.mul[from] * next.mul[axis];
        }
        Self::new(src, mul)
    }

    #[inline]
    pub(crate) fn apply(&self, coord: Coord) -> Coord {
        let [mx, my, mz, mt] = self.mul;
        match self.shape {
            Shape::InPlace => Coord {
                x: coord.x * mx,
                y: coord.y * my,
                z: coord.z * mz,
                t: coord.t * mt,
            },
            Shape::SwapXy => Coord {
                x: coord.y * mx,
                y: coord.x * my,
                z: coord.z * mz,
                t: coord.t * mt,
            },
            Shape::Any => {
                let values = [coord.x, coord.y, coord.z, coord.t];
                Coord {
                    x: values[self.src[0] & 3] * mx,
                    y: values[self.src[1] & 3] * my,
                    z: values[self.src[2] & 3] * mz,
                    t: values[self.src[3] & 3] * mt,
                }
            }
        }
    }
}

/// One thing the plan does to a point. The explicit tag spares the loop a niche decode.
#[repr(u8)]
pub(super) enum Exec {
    Forward(Arc<dyn Step>),
    Inverse(Arc<dyn Step>),
    Affine(Affine),
}

/// The plan of one direction: `inverse_chain` runs the steps from the last, each the other way
/// round. A step that carries `omit_fwd` (`omit_inv` for the inverse chain) does not run, and
/// does not stop the steps around it from folding together.
pub(super) fn plan(steps: &[PipeStep], inverse_chain: bool) -> Vec<Exec> {
    let ordered: Box<dyn Iterator<Item = &PipeStep>> = if inverse_chain {
        Box::new(steps.iter().rev().filter(|step| !step.omit_inv))
    } else {
        Box::new(steps.iter().filter(|step| !step.omit_fwd))
    };
    let mut out = Vec::new();
    let mut pending: Option<Affine> = None;
    for step in ordered {
        let inverse = step.inverted != inverse_chain;
        if let Some(affine) = step.inner.affine(inverse) {
            pending = Some(pending.map_or(affine, |earlier| earlier.then(affine)));
            continue;
        }
        flush(&mut out, pending.take());
        out.push(if inverse {
            Exec::Inverse(Arc::clone(&step.inner))
        } else {
            Exec::Forward(Arc::clone(&step.inner))
        });
    }
    flush(&mut out, pending);
    out
}

fn flush(out: &mut Vec<Exec>, pending: Option<Affine>) {
    if let Some(affine) = pending.filter(|affine| *affine != Affine::IDENTITY) {
        out.push(Exec::Affine(affine));
    }
}

/// The point through the plan, with no stack.
#[inline(always)]
pub(super) fn run(plan: &[Exec], mut coord: Coord) -> Result<Coord, TransformError> {
    for exec in plan {
        coord = match exec {
            Exec::Forward(step) => step.forward(coord)?,
            Exec::Inverse(step) => step.inverse(coord)?,
            Exec::Affine(affine) => affine.apply(coord),
        };
    }
    Ok(coord)
}

/// The point through the plan, a step that reads the stack of the pipeline getting it.
#[inline(always)]
pub(super) fn run_in_pipeline(
    plan: &[Exec],
    mut coord: Coord,
    stack: &mut PipelineStack,
) -> Result<Coord, TransformError> {
    for exec in plan {
        coord = match exec {
            Exec::Forward(step) => step.forward_in_pipeline(coord, stack)?,
            Exec::Inverse(step) => step.inverse_in_pipeline(coord, stack)?,
            Exec::Affine(affine) => affine.apply(coord),
        };
    }
    Ok(coord)
}

/// A push or pop of the pipeline stack: the axes it moves, and which way.
#[derive(Clone, Copy)]
pub(crate) struct StackOp {
    axes: [bool; 4],
    push: bool,
}

impl StackOp {
    pub(crate) fn new(axes: [bool; 4], push: bool) -> Self {
        Self { axes, push }
    }
}

/// The stack of a pipeline over a slice: one entry per point, so a push saves the whole slice
/// and the matching pop gives each point its own value back. A point that has failed keeps its
/// coordinate, and its slot stays so that the others line up.
#[derive(Default)]
struct BatchStack {
    axes: [Vec<f64>; 4],
}

impl BatchStack {
    fn apply(&mut self, op: StackOp, coords: &mut [Coord], failed: &[Option<TransformError>]) {
        let count = coords.len();
        for (index, saved) in self.axes.iter_mut().enumerate() {
            if !op.axes[index] {
                continue;
            }
            if op.push {
                saved.extend(coords.iter().map(|coord| axis_of(coord, index)));
            } else if saved.len() >= count {
                let from = saved.len() - count;
                for ((coord, failed), value) in
                    coords.iter_mut().zip(failed).zip(saved.drain(from..))
                {
                    if failed.is_none() {
                        set_axis(coord, index, value);
                    }
                }
            }
        }
    }
}

fn axis_of(coord: &Coord, index: usize) -> f64 {
    match index {
        0 => coord.x,
        1 => coord.y,
        2 => coord.z,
        _ => coord.t,
    }
}

fn set_axis(coord: &mut Coord, index: usize, value: f64) {
    match index {
        0 => coord.x = value,
        1 => coord.y = value,
        2 => coord.z = value,
        _ => coord.t = value,
    }
}

/// Whether a slice can go through `plan` at once: every step that reads the stack is a push or a
/// pop, and on each axis the pops never outnumber the pushes before them and the two end equal.
/// An unbalanced pipeline keeps values from one point to the next, and only the order of the
/// points one at a time gives that.
pub(super) fn stack_balanced(plan: &[Exec]) -> bool {
    let mut depth = [0_i64; 4];
    for exec in plan {
        let (step, inverse) = match exec {
            Exec::Forward(step) => (step, false),
            Exec::Inverse(step) => (step, true),
            Exec::Affine(_) => continue,
        };
        if !step.needs_pipeline_stack() {
            continue;
        }
        let Some(op) = step.stack_op(inverse) else {
            return false;
        };
        for (axis, selected) in op.axes.iter().enumerate() {
            if *selected {
                depth[axis] += if op.push { 1 } else { -1 };
                if depth[axis] < 0 {
                    return false;
                }
            }
        }
    }
    depth == [0; 4]
}

/// The slice through the plan, one step at a time over every point. A point that has failed
/// keeps its input.
pub(super) fn run_batch(
    plan: &[Exec],
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
) {
    let mut stack = BatchStack::default();
    for exec in plan {
        let (step, inverse) = match exec {
            Exec::Forward(step) => (Some(step), false),
            Exec::Inverse(step) => (Some(step), true),
            Exec::Affine(_) => (None, false),
        };
        if let Some(op) = step.and_then(|step| step.stack_op(inverse)) {
            stack.apply(op, coords, failed);
            continue;
        }
        match exec {
            Exec::Forward(step) => step.forward_batch(coords, failed),
            Exec::Inverse(step) => step.inverse_batch(coords, failed),
            Exec::Affine(affine) => {
                for (coord, failed) in coords.iter_mut().zip(failed.iter()) {
                    if failed.is_none() {
                        *coord = affine.apply(*coord);
                    }
                }
            }
        }
    }
}
