//! The vector path of a [`Classic`] step: `fwd_prepare` and `fwd_finalize` stay the scalar ones,
//! point by point, and only the kernel runs over a chunk ([`ProjectedBatch`], zero origin). A
//! point the kernel refuses (`NaN`) is run again through the scalar step, so its value or its
//! error is the scalar one. See `docs/PORTING-batch-api.md`.

use super::{
    adjlon, geographic_to_geocentric, prepare_forward, Classic, Coord, Kernel, ProjectedBatch,
    Step, TransformError, BATCH_CHUNK,
};

/// The forward of `step` over `coords`, through its vector path when it has one.
pub(super) fn forward<K: Kernel + 'static>(
    step: &Classic<K>,
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
) {
    assert_eq!(coords.len(), failed.len(), "one slot per point");
    let Some(batch) = &step.batch else {
        one_by_one(coords, failed, |coord| step.forward(coord));
        return;
    };
    for (coords, failed) in coords
        .chunks_mut(BATCH_CHUNK)
        .zip(failed.chunks_mut(BATCH_CHUNK))
    {
        forward_chunk(step, batch.as_ref(), coords, failed);
    }
}

fn forward_chunk<K: Kernel + 'static>(
    step: &Classic<K>,
    batch: &dyn ProjectedBatch,
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
) {
    let count = coords.len();
    let frame = &step.frame;
    let (mut lam, mut phi) = ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
    let (mut east, mut north) = ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
    let mut scalar = [false; BATCH_CHUNK];
    for (slot, (coord, failed)) in coords.iter().zip(failed.iter_mut()).enumerate() {
        if failed.is_some() {
            continue;
        }
        // A NaN input has its own answer in the scalar step.
        if coord.x.is_nan() || coord.y.is_nan() {
            scalar[slot] = true;
            continue;
        }
        match prepare_forward(frame, coord.x, coord.y) {
            Ok((l, p)) => (lam[slot], phi[slot]) = (l, p),
            Err(error) => *failed = Some(error),
        }
    }
    batch.forward(
        &lam[..count],
        &phi[..count],
        &mut east[..count],
        &mut north[..count],
    );
    for (slot, (coord, failed)) in coords.iter_mut().zip(failed.iter_mut()).enumerate() {
        if failed.is_some() {
            continue;
        }
        if scalar[slot] || !east[slot].is_finite() || !north[slot].is_finite() {
            match step.forward(*coord) {
                Ok(out) => *coord = out,
                Err(error) => *failed = Some(error),
            }
            continue;
        }
        *coord = Coord {
            x: frame.fr_meter * (east[slot] + frame.x0),
            y: frame.fr_meter * (north[slot] + frame.y0),
            z: frame.vfr_meter * (coord.z + frame.z0),
            t: coord.t,
        };
    }
}

/// The inverse of `step` over `coords`, through its vector path when it has one.
pub(super) fn inverse<K: Kernel + 'static>(
    step: &Classic<K>,
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
) {
    assert_eq!(coords.len(), failed.len(), "one slot per point");
    let Some(batch) = &step.batch else {
        one_by_one(coords, failed, |coord| step.inverse(coord));
        return;
    };
    let frame = &step.frame;
    for (coords, failed) in coords
        .chunks_mut(BATCH_CHUNK)
        .zip(failed.chunks_mut(BATCH_CHUNK))
    {
        let count = coords.len();
        let (mut east, mut north) = ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
        let (mut lon, mut lat) = ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
        for (slot, coord) in coords.iter().enumerate() {
            east[slot] = frame.to_meter * coord.x - frame.x0;
            north[slot] = frame.to_meter * coord.y - frame.y0;
        }
        batch.inverse(
            &east[..count],
            &north[..count],
            &mut lon[..count],
            &mut lat[..count],
        );
        for (slot, (coord, failed)) in coords.iter_mut().zip(failed.iter_mut()).enumerate() {
            if failed.is_some() {
                continue;
            }
            if !lon[slot].is_finite() || !lat[slot].is_finite() {
                match step.inverse(*coord) {
                    Ok(out) => *coord = out,
                    Err(error) => *failed = Some(error),
                }
                continue;
            }
            // `with_batch` keeps no vector path under `+over`: the longitude is wrapped.
            let lam = adjlon(lon[slot].to_radians() + frame.from_greenwich + frame.lam0);
            let mut phi = lat[slot].to_radians();
            if frame.geoc {
                phi = geographic_to_geocentric(phi, frame.earth.one_es());
            }
            *coord = Coord {
                x: lam,
                y: phi,
                z: frame.vto_meter * coord.z - frame.z0,
                t: coord.t,
            };
        }
    }
}

fn one_by_one(
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
    step: impl Fn(Coord) -> Result<Coord, TransformError>,
) {
    for (coord, failed) in coords.iter_mut().zip(failed) {
        if failed.is_none() {
            match step(*coord) {
                Ok(out) => *coord = out,
                Err(error) => *failed = Some(error),
            }
        }
    }
}
