//! `proj-rs-bench --billion`: wall-clock time to convert a billion points, EPSG:4326 to 2154.
//!
//! A billion points are 16 GB: they go through a buffer of [`CHUNK`] points, refilled from one
//! source chunk shifted east by a micro-degree per pass, so no two passes convert the same
//! points. Both sides run the same passes, one point at a time and as a slice.

use super::{facade_inputs, Proj};
use std::hint::black_box;
use std::time::Instant;

/// Points converted by each case.
pub(crate) const TOTAL: usize = 1_000_000_000;
/// Points per pass; 16 MB of `(f64, f64)`.
pub(crate) const CHUNK: usize = 1_000_000;

/// Fills `buffer` with pass `pass` of the billion: the source shifted east.
fn refill(buffer: &mut [(f64, f64)], source: &[(f64, f64)], pass: usize) {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "the pass is below 1000 and exact in f64"
    )]
    let shift = pass as f64 * 1e-6;
    for (slot, &(lon, lat)) in buffer.iter_mut().zip(source) {
        *slot = (lon + shift, lat);
    }
}

/// One million source points in France, cycled from the facade grid.
fn source() -> Vec<(f64, f64)> {
    let (geographic, _) = facade_inputs();
    geographic.iter().copied().cycle().take(CHUNK).collect()
}

/// Seconds for `pass` over the whole billion, buffer refill included.
fn seconds(source: &[(f64, f64)], mut pass: impl FnMut(&mut [(f64, f64)])) -> f64 {
    let mut buffer = source.to_vec();
    let start = Instant::now();
    for index in 0..TOTAL / CHUNK {
        refill(&mut buffer, source, index);
        pass(&mut buffer);
        black_box(&buffer);
    }
    start.elapsed().as_secs_f64()
}

/// proj-rs seconds for (`convert`, `convert_array`).
pub(crate) fn proj_rs() -> (f64, f64) {
    let points = source();
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("4326 to 2154");
    let one_by_one = seconds(&points, |buffer| {
        for point in buffer.iter_mut() {
            *point = proj.convert(black_box(*point)).unwrap_or((0.0, 0.0));
        }
    });
    let array = seconds(&points, |buffer| {
        black_box(proj.convert_array(buffer).map(|done| done.len()).ok());
    });
    (one_by_one, array)
}

/// libproj seconds for (`proj_trans`, `proj_trans_generic`), with the transform the facade
/// bench builds, and the first pass checked against proj-rs.
#[cfg(link_libproj)]
pub(crate) fn libproj() -> Option<(f64, f64)> {
    use super::{
        check_same_output, proj_context_create, proj_context_destroy, proj_create_crs_to_crs,
        proj_destroy, proj_log_level, proj_normalize_for_visualization, proj_trans,
        proj_trans_generic, PjCoord,
    };
    use std::ffi::CString;
    let points = source();
    let (from, to) = (
        CString::new("EPSG:4326").ok()?,
        CString::new("EPSG:2154").ok()?,
    );
    // SAFETY: libproj creates one context. It is destroyed before return.
    let ctx = unsafe { proj_context_create() };
    assert!(!ctx.is_null(), "proj_context_create");
    // SAFETY: `ctx` is live. Level 0 is PJ_LOG_NONE.
    unsafe { proj_log_level(ctx, 0) };
    // SAFETY: both strings are nul-terminated and `ctx` is live. A null area is allowed.
    let raw =
        unsafe { proj_create_crs_to_crs(ctx, from.as_ptr(), to.as_ptr(), std::ptr::null_mut()) };
    assert!(!raw.is_null(), "proj_create_crs_to_crs");
    // SAFETY: `raw` is a live transform of `ctx`.
    let pj = unsafe { proj_normalize_for_visualization(ctx, raw) };
    assert!(!pj.is_null(), "proj_normalize_for_visualization");
    // SAFETY: `raw` is not used again.
    unsafe { proj_destroy(raw) };
    let one_by_one = seconds(&points, |buffer| {
        for point in buffer.iter_mut() {
            let coord = PjCoord {
                x: point.0,
                y: point.1,
                z: 0.0,
                t: f64::INFINITY,
            };
            // SAFETY: `pj` is live and `coord` is a 32-byte `PJ_COORD`.
            let out = unsafe { proj_trans(pj, 1, black_box(coord)) };
            *point = (out.x, out.y);
        }
    });
    let mut first = Vec::new();
    let array = seconds(&points, |buffer| {
        let count = buffer.len();
        let x = buffer.as_mut_ptr().cast::<f64>();
        let y = x.wrapping_add(1);
        // SAFETY: `buffer` holds `count` interleaved (x, y) pairs of 16 bytes, so `y` starts
        // one `f64` after `x`. Null z and t with a zero count make libproj use 0 and no time.
        let done = unsafe {
            proj_trans_generic(
                pj,
                1,
                x,
                16,
                count,
                y,
                16,
                count,
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null_mut(),
                0,
                0,
            )
        };
        black_box(done);
        if first.is_empty() {
            first = buffer.to_vec();
        }
    });
    check_same_output(
        "billion_4326_2154",
        &first,
        &points,
        ("EPSG:4326", "EPSG:2154"),
    );
    // SAFETY: `pj` was created above and is not used again.
    unsafe { proj_destroy(pj) };
    // SAFETY: `ctx` was created above and is not used again.
    unsafe { proj_context_destroy(ctx) };
    Some((one_by_one, array))
}

#[cfg(not(link_libproj))]
pub(crate) fn libproj() -> Option<(f64, f64)> {
    None
}
