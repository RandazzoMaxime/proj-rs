//! Creation of a CRS-to-CRS transform, and the transformation functions of `proj.h`
//! (`trans.cpp`, `trans_bounds.cpp`, `crs_to_crs.cpp`).

use std::ffi::{c_char, c_int};

use proj_rs::unstable::{Direction, Proj, ProjError, StepError, Xyzt};

use crate::context::{context, Ctx};
use crate::ffi::*;
use crate::pj::{area, creation_failure, hand_over, pj, Pj};
use crate::util::{self, guard};

/// The four components of a `PJ_COORD`.
pub(crate) fn components(coord: PJ_COORD) -> [f64; 4] {
    // SAFETY: every member of the union is made of `double` values and `v` covers all four,
    // so reading `v` is valid whichever member was written.
    unsafe { coord.v }
}

pub(crate) fn coord_of(values: [f64; 4]) -> PJ_COORD {
    PJ_COORD { v: values }
}

pub(crate) fn xyzt(coord: PJ_COORD) -> Xyzt {
    let [x, y, z, t] = components(coord);
    Xyzt { x, y, z, t }
}

pub(crate) fn coord_from(value: Xyzt) -> PJ_COORD {
    coord_of([value.x, value.y, value.z, value.t])
}

/// `proj_coord_error()`: every component `HUGE_VAL`.
pub(crate) fn coord_error() -> PJ_COORD {
    coord_of([f64::INFINITY; 4])
}

/// The PROJ error code of a failed step.
pub(crate) fn step_errno(error: StepError) -> c_int {
    (match error {
        StepError::OutsideDomain => PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN,
        StepError::OutsideGrid => PROJ_ERR_COORD_TRANSFM_OUTSIDE_GRID,
        StepError::MissingTime => PROJ_ERR_COORD_TRANSFM_MISSING_TIME,
        StepError::NoInverse => PROJ_ERR_OTHER_NO_INVERSE_OP,
        StepError::InvalidCoord | StepError::NonFinite => PROJ_ERR_COORD_TRANSFM_INVALID_COORD,
        StepError::NetworkUnavailable => PROJ_ERR_OTHER_NETWORK_ERROR,
    }) as c_int
}

pub(crate) fn direction_of(direction: PJ_DIRECTION) -> Option<Direction> {
    match direction {
        1 => Some(Direction::Forward),
        -1 => Some(Direction::Inverse),
        _ => None,
    }
}

fn opposite(direction: PJ_DIRECTION) -> PJ_DIRECTION {
    -direction
}

impl Pj {
    /// The axes the caller's order puts first, for a pair that has not been normalised: the
    /// engine transforms longitude or easting first, the caller gives the order of the CRS.
    fn swaps(&self, direction: Direction) -> (bool, bool) {
        let swaps = self.proj.as_ref().and_then(Proj::pair_axis_swaps);
        let (source, target) = match (self.normalized, swaps) {
            (false, Some(swaps)) => swaps,
            _ => (false, false),
        };
        match direction {
            Direction::Forward => (source, target),
            Direction::Inverse => (target, source),
        }
    }

    /// One coordinate in `direction`, in the axis order the caller uses.
    pub(crate) fn transform(&self, coord: Xyzt, direction: Direction) -> Result<Xyzt, StepError> {
        let (swap_in, swap_out) = self.swaps(direction);
        let given = if swap_in { swapped(coord) } else { coord };
        let proj = self.proj.as_ref().ok_or(StepError::NoInverse)?;
        let done = proj.trans_coord(given, direction)?;
        Ok(if swap_out { swapped(done) } else { done })
    }

    /// Whether the operation reads angles in degrees (`proj_degree_input`).
    pub(crate) fn degree_input(&self, direction: Direction) -> bool {
        self.proj
            .as_ref()
            .is_some_and(|proj| proj.degree_input(direction))
    }
}

fn swapped(coord: Xyzt) -> Xyzt {
    Xyzt {
        x: coord.y,
        y: coord.x,
        ..coord
    }
}

/// `proj_trans` on a loaded object: the coordinate, or the error coordinate with the error set
/// on the context (`pj_fwd4d` resets the error on entry and restores it on success).
pub(crate) fn trans_one(object: &mut Pj, direction: PJ_DIRECTION, coord: PJ_COORD) -> PJ_COORD {
    let Some(way) = direction_of(direction) else {
        return coord;
    };
    let ctx = object.context();
    if !object.is_transform() {
        ctx.fail(
            PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE as c_int,
            "Object is not a coordinate operation",
        );
        return coord_error();
    }
    let values = components(coord);
    if values.iter().any(|value| value.is_nan()) {
        return coord_of([f64::NAN; 4]);
    }
    object.transformed = true;
    let previous = std::mem::replace(&mut ctx.errno, 0);
    match object.transform(xyzt(coord), way) {
        Ok(done) => {
            ctx.errno = previous;
            coord_from(done)
        }
        Err(error) => {
            ctx.errno = step_errno(error);
            coord_error()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_trans(p: *mut PJ, direction: PJ_DIRECTION, coord: PJ_COORD) -> PJ_COORD {
    guard(coord_error(), || match pj(p) {
        Some(object) => trans_one(object, direction, coord),
        None => coord,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_trans_array(
    p: *mut PJ,
    direction: PJ_DIRECTION,
    n: usize,
    coord: *mut PJ_COORD,
) -> c_int {
    guard(PROJ_ERR_COORD_TRANSFM as c_int, || {
        let Some(object) = pj(p) else {
            return 0;
        };
        let mut result = 0;
        let mut seen = false;
        let mut same = true;
        for slot in util::slice_mut(coord, n) {
            object.context().errno = 0;
            *slot = trans_one(object, direction, *slot);
            let code = object.context().errno;
            if code != 0 {
                if !seen {
                    result = code;
                    seen = true;
                } else if same && result != code {
                    same = false;
                    result = PROJ_ERR_COORD_TRANSFM as c_int;
                }
            }
        }
        object.context().errno = result;
        result
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_trans_generic(
    p: *mut PJ,
    direction: PJ_DIRECTION,
    x: *mut f64,
    sx: usize,
    nx: usize,
    y: *mut f64,
    sy: usize,
    ny: usize,
    z: *mut f64,
    sz: usize,
    nz: usize,
    t: *mut f64,
    st: usize,
    nt: usize,
) -> usize {
    guard(0, || {
        let Some(object) = pj(p) else {
            return 0;
        };
        let mut zero = 0.0_f64;
        let mut invalid_time = f64::INFINITY;
        let mut axes = [
            Axis::new(x, sx, nx, &mut zero),
            Axis::new(y, sy, ny, &mut zero),
            Axis::new(z, sz, nz, &mut zero),
            Axis::new(t, st, nt, &mut invalid_time),
        ];
        if axes.iter().all(|axis| axis.count == 0) {
            return 0;
        }
        let longest = axes
            .iter()
            .map(|axis| axis.count)
            .find(|count| *count > 1)
            .unwrap_or(1);
        let shortest = axes
            .iter()
            .map(|axis| axis.count)
            .filter(|count| *count > 1)
            .fold(longest, usize::min);
        if direction == 0 {
            return shortest;
        }
        let mut done = 0;
        while done < shortest {
            let values = [
                axes[0].read(),
                axes[1].read(),
                axes[2].read(),
                axes[3].read(),
            ];
            let out = components(trans_one(object, direction, coord_of(values)));
            for (axis, value) in axes.iter_mut().zip(out) {
                axis.write_and_step(value);
            }
            done += 1;
        }
        for axis in &mut axes {
            axis.finish();
        }
        done
    })
}

/// One strided array of `proj_trans_generic`.
struct Axis {
    base: *mut f64,
    stride: usize,
    count: usize,
    current: *mut f64,
    last: f64,
}

impl Axis {
    fn new(array: *mut f64, stride: usize, count: usize, fallback: &mut f64) -> Self {
        if array.is_null() || count == 0 {
            return Self {
                base: std::ptr::from_mut(fallback),
                stride: 0,
                count: 0,
                current: std::ptr::from_mut(fallback),
                last: *fallback,
            };
        }
        Self {
            base: array,
            stride,
            count,
            current: array,
            last: 0.0,
        }
    }

    fn read(&self) -> f64 {
        util::get(self.current.cast_const()).unwrap_or(0.0)
    }

    fn write_and_step(&mut self, value: f64) {
        self.last = value;
        if self.count > 1 {
            util::put(self.current, value);
            self.current = self
                .current
                .cast::<u8>()
                .wrapping_add(self.stride)
                .cast::<f64>();
        }
    }

    fn finish(&mut self) {
        if self.count == 1 {
            util::put(self.base, self.last);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_trans_get_last_used_operation(p: *mut PJ) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some(object) = pj(p) else {
            return std::ptr::null_mut();
        };
        match object.proj.as_ref().and_then(Proj::last_used_operation) {
            Some(operation) => {
                let mut copy = object.copy_in(object.ctx);
                copy.proj = Some(operation);
                hand_over(copy)
            }
            None if object.transformed => hand_over(object.copy_in(object.ctx)),
            None => std::ptr::null_mut(),
        }
    })
}

// ---------------------------------------------------------------------------
// CRS to CRS.

fn bbox_of(area_pointer: *const PJ_AREA) -> Option<proj_rs::unstable::Area> {
    area(area_pointer).and_then(|found| found.bbox)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_crs_to_crs(
    ctx: *mut PJ_CONTEXT,
    source_crs: *const c_char,
    target_crs: *const c_char,
    area_pointer: *mut PJ_AREA,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let (Some(source), Some(target)) = (util::text(source_crs), util::text(target_crs)) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_create_crs_to_crs: missing CRS",
            );
            return std::ptr::null_mut();
        };
        if source.contains('@') || target.contains('@') {
            // A coordinate epoch: each side is an object of its own, as in PROJ.
            let from = crate::pj::create(ctx, source, "proj_create_crs_to_crs");
            let to = crate::pj::create(ctx, target, "proj_create_crs_to_crs");
            let made =
                proj_create_crs_to_crs_from_pj(ctx, from, to, area_pointer, std::ptr::null());
            crate::pj::proj_destroy(from);
            crate::pj::proj_destroy(to);
            return made;
        }
        match context
            .builder()
            .proj_known_crs(source, target, bbox_of(area_pointer))
        {
            Ok(proj) => hand_over(Pj::new(std::ptr::from_mut(context), proj)),
            Err(error) => {
                creation_failure(context, "proj_create_crs_to_crs", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_crs_to_crs_from_pj(
    ctx: *mut PJ_CONTEXT,
    source_crs: *const PJ,
    target_crs: *const PJ,
    area_pointer: *mut PJ_AREA,
    options: *const *const c_char,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let (Some(source), Some(target)) = (pj(source_crs), pj(target_crs)) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_create_crs_to_crs_from_pj: missing CRS",
            );
            return std::ptr::null_mut();
        };
        let (Some(source_proj), Some(target_proj)) = (source.proj.as_ref(), target.proj.as_ref())
        else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_create_crs_to_crs_from_pj: not a CRS",
            );
            return std::ptr::null_mut();
        };
        let mut list = Vec::new();
        if !options.is_null() {
            let mut index = 0;
            while let Some(item) = util::get(options.wrapping_add(index)) {
                let Some(text) = util::text(item) else {
                    break;
                };
                list.push(text);
                index += 1;
            }
        }
        let options = (!list.is_empty()).then_some(list);
        match context.builder().proj_create_crs_to_crs_from_pj(
            source_proj,
            target_proj,
            bbox_of(area_pointer),
            options,
        ) {
            Ok(proj) => hand_over(Pj::new(std::ptr::from_mut(context), proj)),
            Err(error) => {
                creation_failure(context, "proj_create_crs_to_crs_from_pj", &error);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_normalize_for_visualization(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            return std::ptr::null_mut();
        };
        if object
            .proj
            .as_ref()
            .and_then(Proj::pair_axis_swaps)
            .is_none()
        {
            let is_proj_string = object
                .proj
                .as_ref()
                .is_some_and(|proj| proj.native_operation().is_some());
            let normalized = if is_proj_string {
                None
            } else {
                crate::iso::normalized_for_visualization(object)
            };
            return normalized.unwrap_or_else(|| {
                context.fail(
                    PROJ_ERR_OTHER_API_MISUSE as c_int,
                    "proj_normalize_for_visualization: the object is not a CRS or a CRS pair",
                );
                std::ptr::null_mut()
            });
        }
        let mut copy = object.copy_in(std::ptr::from_mut(context));
        copy.normalized = true;
        hand_over(copy)
    })
}

// ---------------------------------------------------------------------------
// Bounds.

fn swap_box(values: [f64; 4]) -> [f64; 4] {
    [values[1], values[0], values[3], values[2]]
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_trans_bounds(
    ctx: *mut PJ_CONTEXT,
    p: *mut PJ,
    direction: PJ_DIRECTION,
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
    out_xmin: *mut f64,
    out_ymin: *mut f64,
    out_xmax: *mut f64,
    out_ymax: *mut f64,
    densify_pts: c_int,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(object) = pj(p) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_trans_bounds: null PJ",
            );
            return 0;
        };
        if proj_id(object).is_none() {
            context.fail(
                PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE as c_int,
                "NULL transformation not allowed,",
            );
            return 0;
        }
        if direction == 0 || is_noop(object) {
            for (slot, value) in [
                (out_xmin, xmin),
                (out_ymin, ymin),
                (out_xmax, xmax),
                (out_ymax, ymax),
            ] {
                util::put(slot, value);
            }
            return 1;
        }
        let Some(way) = direction_of(direction) else {
            return 0;
        };
        if !(0..=10_000).contains(&densify_pts) {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "densify_pts must be between 0-10000.",
            );
            return 0;
        }
        if xmin.is_nan() || ymin.is_nan() || xmax.is_nan() || ymax.is_nan() {
            context.fail(
                PROJ_ERR_COORD_TRANSFM_INVALID_COORD as c_int,
                "xmin, ymin, xmax or ymax is NaN",
            );
            return 0;
        }
        if !object.is_transform() {
            context.fail(
                PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE as c_int,
                "Object is not a coordinate operation",
            );
            return 0;
        }
        let (swap_in, swap_out) = object.swaps(way);
        let given = [xmin, ymin, xmax, ymax];
        let given = if swap_in { swap_box(given) } else { given };
        let Some(proj) = object.proj.as_ref() else {
            return 0;
        };
        match proj.transform_bounds_in(way, given[0], given[1], given[2], given[3], densify_pts) {
            Ok(found) => {
                let found = if swap_out { swap_box(found) } else { found };
                util::put(out_xmin, found[0]);
                util::put(out_ymin, found[1]);
                util::put(out_xmax, found[2]);
                util::put(out_ymax, found[3]);
                1
            }
            Err(error) => {
                bounds_failure(context, &error);
                0
            }
        }
    })
}

fn proj_id(object: &Pj) -> Option<String> {
    object.proj.as_ref().and_then(|proj| proj.proj_info().id)
}

/// An operation `proj_pj_info` names `noop`: the bounds are returned as they came.
fn is_noop(object: &Pj) -> bool {
    proj_id(object).as_deref() == Some("noop")
}

fn bounds_failure(context: &mut Ctx, error: &ProjError) {
    context.fail(error.code(), &format!("proj_trans_bounds: {error}"));
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_trans_bounds_3D(
    ctx: *mut PJ_CONTEXT,
    p: *mut PJ,
    direction: PJ_DIRECTION,
    xmin: f64,
    ymin: f64,
    zmin: f64,
    xmax: f64,
    ymax: f64,
    zmax: f64,
    out_xmin: *mut f64,
    out_ymin: *mut f64,
    out_zmin: *mut f64,
    out_xmax: *mut f64,
    out_ymax: *mut f64,
    out_zmax: *mut f64,
    densify_pts: c_int,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some(object) = pj(p) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_trans_bounds_3D: null PJ",
            );
            return 0;
        };
        if proj_id(object).is_none() {
            context.fail(
                PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE as c_int,
                "NULL transformation not allowed,",
            );
            return 0;
        }
        if direction == 0 || is_noop(object) {
            for (slot, value) in [
                (out_xmin, xmin),
                (out_ymin, ymin),
                (out_zmin, zmin),
                (out_xmax, xmax),
                (out_ymax, ymax),
                (out_zmax, zmax),
            ] {
                util::put(slot, value);
            }
            return 1;
        }
        let Some(way) = direction_of(direction) else {
            return 0;
        };
        if !(0..=10_000).contains(&densify_pts) {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "densify_pts must be between 0-10000.",
            );
            return 0;
        }
        if [xmin, ymin, zmin, xmax, ymax, zmax]
            .iter()
            .any(|v| v.is_nan())
        {
            context.fail(
                PROJ_ERR_COORD_TRANSFM_INVALID_COORD as c_int,
                "a bound is NaN",
            );
            return 0;
        }
        if !object.is_transform() {
            context.fail(
                PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE as c_int,
                "Object is not a coordinate operation",
            );
            return 0;
        }
        let (swap_in, swap_out) = object.swaps(way);
        let (mut low, mut high) = ([xmin, ymin, zmin], [xmax, ymax, zmax]);
        if swap_in {
            low.swap(0, 1);
            high.swap(0, 1);
        }
        let Some(proj) = object.proj.as_ref() else {
            return 0;
        };
        match proj.transform_bounds_3d(way, low, high, densify_pts) {
            Ok((mut low, mut high)) => {
                if swap_out {
                    low.swap(0, 1);
                    high.swap(0, 1);
                }
                util::put(out_xmin, low[0]);
                util::put(out_ymin, low[1]);
                util::put(out_zmin, low[2]);
                util::put(out_xmax, high[0]);
                util::put(out_ymax, high[1]);
                util::put(out_zmax, high[2]);
                1
            }
            Err(error) => {
                bounds_failure(context, &error);
                0
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coord(x: f64, y: f64, z: f64, t: f64) -> PJ_COORD {
    coord_of([x, y, z, t])
}

/// The opposite direction of a direction flag, for the round trips of `dist.rs`.
pub(crate) fn reverse(direction: PJ_DIRECTION) -> PJ_DIRECTION {
    opposite(direction)
}
