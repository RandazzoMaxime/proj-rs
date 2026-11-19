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
/// bench builds, and the first pass checked against proj-rs. The passes run in
/// `libproj_timer.c`, from the same source points and with the same refill.
#[cfg(link_libproj)]
pub(crate) fn libproj() -> Option<(f64, f64)> {
    use super::{check_same_output, libproj_timer::Timer};
    let points = source();
    let mut timer = Timer::start();
    let (one_by_one, array, first) = timer.billion("EPSG:4326", "EPSG:2154", TOTAL, &points);
    check_same_output(
        "billion_4326_2154",
        &first,
        &points,
        ("EPSG:4326", "EPSG:2154"),
    );
    Some((one_by_one, array))
}

#[cfg(not(link_libproj))]
pub(crate) fn libproj() -> Option<(f64, f64)> {
    None
}
