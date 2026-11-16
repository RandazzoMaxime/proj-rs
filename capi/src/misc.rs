//! Angles, distances, factors, library information and the small utilities of `proj.h`
//! (`coordinates.cpp`, `dist.cpp`, `factors.cpp`, `info.cpp`, `trans.cpp`).

use std::ffi::{c_char, c_int};

use proj_rs::dms::{self, DmsFormat, DmsSign};
use proj_rs::geod::Geod;
use proj_rs::unstable::{Direction, Proj};

use crate::context::context;
use crate::ffi::*;
use crate::pj::{pj, Pj};
use crate::trans::{components, coord_error, coord_of, direction_of, reverse, trans_one};
use crate::util::{self, guard};

fn operation_input_is_angular(object: &Pj, direction: PJ_DIRECTION) -> bool {
    let Some(operation) = object.proj.as_ref().and_then(Proj::native_operation) else {
        return false;
    };
    match direction_of(direction) {
        Some(Direction::Forward) => operation.angular_input(false),
        Some(Direction::Inverse) => operation.angular_input(true),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_angular_input(p: *mut PJ, dir: PJ_DIRECTION) -> c_int {
    guard(0, || {
        pj(p).map_or(0, |object| {
            c_int::from(operation_input_is_angular(object, dir))
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_angular_output(p: *mut PJ, dir: PJ_DIRECTION) -> c_int {
    guard(0, || {
        pj(p).map_or(0, |object| {
            c_int::from(operation_input_is_angular(object, reverse(dir)))
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_degree_input(p: *mut PJ, dir: PJ_DIRECTION) -> c_int {
    guard(0, || {
        let Some(object) = pj(p) else {
            return 0;
        };
        direction_of(dir).map_or(0, |way| c_int::from(object.degree_input(way)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_degree_output(p: *mut PJ, dir: PJ_DIRECTION) -> c_int {
    proj_degree_input(p, reverse(dir))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_torad(angle_in_degrees: f64) -> f64 {
    angle_in_degrees.to_radians()
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_todeg(angle_in_radians: f64) -> f64 {
    angle_in_radians.to_degrees()
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_dmstor(is: *const c_char, rs: *mut *mut c_char) -> f64 {
    guard(f64::INFINITY, || {
        let Some(text) = util::bytes(is) else {
            return f64::INFINITY;
        };
        let (value, used) = dms::dmstor_with_end(text);
        if !rs.is_null() {
            util::put(rs, is.wrapping_add(used).cast_mut());
        }
        value
    })
}

fn rtodms(s: *mut c_char, size: usize, r: f64, pos: c_int, neg: c_int) -> *mut c_char {
    if s.is_null() || size == 0 {
        return std::ptr::null_mut();
    }
    let sign = if pos == 0 {
        DmsSign::Minus
    } else {
        DmsSign::Hemisphere {
            positive: u8::try_from(pos).map_or('\0', char::from),
            negative: u8::try_from(neg).map_or('\0', char::from),
        }
    };
    let text = dms::rtodms(r, sign, DmsFormat::default(), size - 1);
    let out = util::slice_mut(s.cast::<u8>(), size);
    let bytes = text.as_bytes();
    let count = bytes.len().min(size - 1);
    out[..count].copy_from_slice(&bytes[..count]);
    out[count] = 0;
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_rtodms(s: *mut c_char, r: f64, pos: c_int, neg: c_int) -> *mut c_char {
    guard(std::ptr::null_mut(), || rtodms(s, 40, r, pos, neg))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_rtodms2(
    s: *mut c_char,
    sizeof_s: usize,
    r: f64,
    pos: c_int,
    neg: c_int,
) -> *mut c_char {
    guard(std::ptr::null_mut(), || rtodms(s, sizeof_s, r, pos, neg))
}

// ---------------------------------------------------------------------------
// Distances (`dist.cpp`).

fn xy(coord: PJ_COORD) -> (f64, f64) {
    let [x, y, _, _] = components(coord);
    (x, y)
}

fn geodesic_of(object: &Pj) -> Option<Geod> {
    let (a, es) = object
        .proj
        .as_ref()?
        .native_operation()?
        .distance_ellipsoid();
    Some(Geod::new(a, 1.0 - (1.0 - es).sqrt()))
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_xy_dist(a: PJ_COORD, b: PJ_COORD) -> f64 {
    let ((ax, ay), (bx, by)) = (xy(a), xy(b));
    (ax - bx).hypot(ay - by)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_xyz_dist(a: PJ_COORD, b: PJ_COORD) -> f64 {
    let [_, _, az, _] = components(a);
    let [_, _, bz, _] = components(b);
    proj_xy_dist(a, b).hypot(az - bz)
}

fn lp_dist(object: &Pj, a: PJ_COORD, b: PJ_COORD) -> f64 {
    let Some(geod) = geodesic_of(object) else {
        return f64::INFINITY;
    };
    let ([lam1, phi1, _, _], [lam2, phi2, _, _]) = (components(a), components(b));
    geod.inverse(
        phi1.to_degrees(),
        lam1.to_degrees(),
        phi2.to_degrees(),
        lam2.to_degrees(),
    )
    .distance
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_lp_dist(p: *const PJ, a: PJ_COORD, b: PJ_COORD) -> f64 {
    guard(f64::INFINITY, || {
        pj(p).map_or(f64::INFINITY, |object| lp_dist(object, a, b))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_lpz_dist(p: *const PJ, a: PJ_COORD, b: PJ_COORD) -> f64 {
    guard(f64::INFINITY, || {
        let Some(object) = pj(p) else {
            return f64::INFINITY;
        };
        let ([lam1, _, z1, _], [lam2, _, z2, _]) = (components(a), components(b));
        if lam1 == f64::INFINITY || lam2 == f64::INFINITY {
            return f64::INFINITY;
        }
        lp_dist(object, a, b).hypot(z1 - z2)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_geod(p: *const PJ, a: PJ_COORD, b: PJ_COORD) -> PJ_COORD {
    guard(coord_error(), || {
        let Some(geod) = pj(p).and_then(|object| geodesic_of(object)) else {
            return coord_error();
        };
        let ([lam1, phi1, _, _], [lam2, phi2, _, _]) = (components(a), components(b));
        let found = geod.inverse(
            phi1.to_degrees(),
            lam1.to_degrees(),
            phi2.to_degrees(),
            lam2.to_degrees(),
        );
        coord_of([found.distance, found.azimuth1, found.azimuth2, 0.0])
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_geod_direct(
    p: *const PJ,
    a: PJ_COORD,
    azimuth: f64,
    distance: f64,
) -> PJ_COORD {
    guard(coord_error(), || {
        let Some(geod) = pj(p).and_then(|object| geodesic_of(object)) else {
            return coord_error();
        };
        let [lam, phi, _, _] = components(a);
        let (lat, lon, azi) = geod
            .line(phi.to_degrees(), lam.to_degrees(), azimuth.to_degrees())
            .position(distance);
        coord_of([lon.to_radians(), lat.to_radians(), azi.to_radians(), 0.0])
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_roundtrip(
    p: *mut PJ,
    direction: PJ_DIRECTION,
    n: c_int,
    coord: *mut PJ_COORD,
) -> f64 {
    guard(f64::INFINITY, || {
        let Some(object) = pj(p) else {
            return f64::INFINITY;
        };
        if n < 1 {
            object
                .context()
                .fail(PROJ_ERR_OTHER_API_MISUSE as c_int, "n should be >= 1");
            return f64::INFINITY;
        }
        let Some(start) = util::get(coord.cast_const()) else {
            return f64::INFINITY;
        };
        let mut current = start;
        for _ in 0..n - 1 {
            let back = trans_one(object, reverse(direction), current);
            current = trans_one(object, direction, back);
        }
        current = trans_one(object, reverse(direction), current);
        let from = components(start);
        let to = components(current);
        if from.iter().any(|v| v.is_nan()) && to.iter().all(|v| v.is_nan()) {
            return 0.0;
        }
        if operation_input_is_angular(object, direction) {
            proj_lpz_dist(p, start, current)
        } else {
            proj_xyz_dist(start, current)
        }
    })
}

// ---------------------------------------------------------------------------
// Factors (`factors.cpp`).

#[unsafe(no_mangle)]
pub extern "C" fn proj_factors(p: *mut PJ, lp: PJ_COORD) -> PJ_FACTORS {
    guard(zero_factors(), || {
        let Some(object) = pj(p) else {
            return zero_factors();
        };
        let [lam, phi, _, _] = components(lp);
        let Some(proj) = object.proj.as_ref() else {
            return zero_factors();
        };
        match proj.factors(lam, phi) {
            Ok(found) => PJ_FACTORS {
                meridional_scale: found.meridional_scale,
                parallel_scale: found.parallel_scale,
                areal_scale: found.areal_scale,
                angular_distortion: found.angular_distortion,
                meridian_parallel_angle: found.meridian_parallel_angle,
                meridian_convergence: found.meridian_convergence,
                tissot_semimajor: found.tissot_semimajor,
                tissot_semiminor: found.tissot_semiminor,
                dx_dlam: found.dx_dlam,
                dx_dphi: found.dx_dphi,
                dy_dlam: found.dy_dlam,
                dy_dphi: found.dy_dphi,
            },
            Err(error) => {
                object.context().fail(error.code(), "proj_factors failed");
                zero_factors()
            }
        }
    })
}

fn zero_factors() -> PJ_FACTORS {
    PJ_FACTORS {
        meridional_scale: 0.0,
        parallel_scale: 0.0,
        areal_scale: 0.0,
        angular_distortion: 0.0,
        meridian_parallel_angle: 0.0,
        meridian_convergence: 0.0,
        tissot_semimajor: 0.0,
        tissot_semiminor: 0.0,
        dx_dlam: 0.0,
        dx_dphi: 0.0,
        dy_dlam: 0.0,
        dy_dphi: 0.0,
    }
}

// ---------------------------------------------------------------------------
// Utilities.

#[unsafe(no_mangle)]
pub extern "C" fn proj_cleanup() {
    guard((), || {
        let ctx = context(std::ptr::null());
        ctx.database = None;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_string_list_destroy(list: PROJ_STRING_LIST) {
    guard((), || util::free_string_list(list));
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_string_destroy(str_: *mut c_char) {
    guard((), || util::take_string(str_));
}
