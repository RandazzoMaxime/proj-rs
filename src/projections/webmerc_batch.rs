//! `webmerc` par lots : le noyau sphérique de [`Mercator`] en vecteur, derrière l'étape scalaire.
//!
//! `classic_step` n'ajoute pas de chemin vectoriel, et `Classic::with_batch` reste privé à
//! `engine`. L'étape est donc enveloppée : elle délègue tout à l'étape scalaire, et ne prend en
//! charge que `forward_batch` et `inverse_batch`, par la même suite d'opérations que `Classic`.
//! Un point que le noyau refuse, ou dont une valeur n'est pas finie, repasse par le scalaire.

use crate::angle::adjlon;
use crate::engine::{
    geographic_to_geocentric, prepare_forward, Coord, CreateError, Frame, IoKind, PipelineStack,
    Step, TransformError, UnitProjection,
};
use crate::merc::Mercator;

/// Points par tour de boucle : la taille de `engine::BATCH_CHUNK`, pour les mêmes tableaux.
const CHUNK: usize = 256;

/// `scalar` enveloppé d'un chemin par lots, sauf sous `+over` : la longitude y reste non ramenée
/// côté scalaire, ce que le noyau vectoriel ne fait pas (comme `Classic::with_batch`).
pub(crate) fn wrap(frame: &Frame, scalar: Box<dyn Step>) -> Result<Box<dyn Step>, CreateError> {
    if frame.over {
        return Ok(scalar);
    }
    let merc = Mercator::try_sphere(frame.earth.a(), 0.0, 1.0, 0.0, 0.0)
        .map_err(|_| CreateError::IllegalValue("a"))?;
    Ok(Box::new(WebMercStep {
        scalar,
        frame: frame.clone(),
        merc,
    }))
}

struct WebMercStep {
    scalar: Box<dyn Step>,
    frame: Frame,
    merc: Mercator,
}

impl WebMercStep {
    fn forward_chunk(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        let count = coords.len();
        let frame = &self.frame;
        let (mut lam, mut phi) = ([0.0; CHUNK], [0.0; CHUNK]);
        let (mut east, mut north) = ([0.0; CHUNK], [0.0; CHUNK]);
        let mut use_scalar = [false; CHUNK];
        for (slot, (coord, failed)) in coords.iter().zip(failed.iter_mut()).enumerate() {
            if failed.is_some() {
                continue;
            }
            if coord.x.is_nan() || coord.y.is_nan() {
                use_scalar[slot] = true;
                continue;
            }
            match prepare_forward(frame, coord.x, coord.y) {
                Ok((l, p)) => (lam[slot], phi[slot]) = (l, p),
                Err(error) => *failed = Some(error),
            }
        }
        self.merc.forward_rad_batch(
            &lam[..count],
            &phi[..count],
            &mut east[..count],
            &mut north[..count],
        );
        for (slot, (coord, failed)) in coords.iter_mut().zip(failed.iter_mut()).enumerate() {
            if failed.is_some() {
                continue;
            }
            if use_scalar[slot] || !east[slot].is_finite() || !north[slot].is_finite() {
                match self.scalar.forward(*coord) {
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

    fn inverse_chunk(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        let count = coords.len();
        let frame = &self.frame;
        let ra = frame.earth.ra();
        let (mut x, mut y) = ([0.0; CHUNK], [0.0; CHUNK]);
        let (mut lam, mut phi) = ([0.0; CHUNK], [0.0; CHUNK]);
        for (slot, coord) in coords.iter().enumerate() {
            x[slot] = (frame.to_meter * coord.x - frame.x0) * ra;
            y[slot] = (frame.to_meter * coord.y - frame.y0) * ra;
        }
        self.merc.inverse_unit_batch(
            &x[..count],
            &y[..count],
            &mut lam[..count],
            &mut phi[..count],
        );
        for (slot, (coord, failed)) in coords.iter_mut().zip(failed.iter_mut()).enumerate() {
            if failed.is_some() {
                continue;
            }
            if !lam[slot].is_finite() || !phi[slot].is_finite() {
                match self.scalar.inverse(*coord) {
                    Ok(out) => *coord = out,
                    Err(error) => *failed = Some(error),
                }
                continue;
            }
            let mut latitude = phi[slot];
            if frame.geoc {
                latitude = geographic_to_geocentric(latitude, frame.earth.one_es());
            }
            *coord = Coord {
                x: adjlon(lam[slot] + frame.from_greenwich + frame.lam0),
                y: latitude,
                z: frame.vto_meter * coord.z - frame.z0,
                t: coord.t,
            };
        }
    }
}

impl Step for WebMercStep {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.scalar.forward(coord)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.scalar.inverse(coord)
    }

    fn angular_input_forward(&self) -> bool {
        self.scalar.angular_input_forward()
    }

    fn angular_output_forward(&self) -> bool {
        self.scalar.angular_output_forward()
    }

    fn degree_input_forward(&self) -> bool {
        self.scalar.degree_input_forward()
    }

    fn degree_output_forward(&self) -> bool {
        self.scalar.degree_output_forward()
    }

    fn input_kind(&self) -> IoKind {
        self.scalar.input_kind()
    }

    fn output_kind(&self) -> IoKind {
        self.scalar.output_kind()
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        self.scalar.distance_ellipsoid()
    }

    fn output_to_metre(&self) -> f64 {
        self.scalar.output_to_metre()
    }

    fn unit_projection(&self) -> Option<UnitProjection<'_>> {
        self.scalar.unit_projection()
    }

    fn computes_height(&self) -> bool {
        self.scalar.computes_height()
    }

    fn needs_pipeline_stack(&self) -> bool {
        self.scalar.needs_pipeline_stack()
    }

    fn forward_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut PipelineStack,
    ) -> Result<Coord, TransformError> {
        self.scalar.forward_in_pipeline(coord, stack)
    }

    fn inverse_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut PipelineStack,
    ) -> Result<Coord, TransformError> {
        self.scalar.inverse_in_pipeline(coord, stack)
    }

    fn forward_batch(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        assert_eq!(coords.len(), failed.len(), "one slot per point");
        for (coords, failed) in coords.chunks_mut(CHUNK).zip(failed.chunks_mut(CHUNK)) {
            self.forward_chunk(coords, failed);
        }
    }

    fn inverse_batch(&self, coords: &mut [Coord], failed: &mut [Option<TransformError>]) {
        assert_eq!(coords.len(), failed.len(), "one slot per point");
        for (coords, failed) in coords.chunks_mut(CHUNK).zip(failed.chunks_mut(CHUNK)) {
            self.inverse_chunk(coords, failed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angle::adjlon;
    use crate::engine::Operation;
    use crate::Ellipsoid;
    use std::f64::consts::{FRAC_PI_2, PI};

    /// Tolérance de `docs/PARITY.md` : 1e-6 m et 1e-13 relatif pour une distance, 1e-11 degré
    /// (1,75e-13 rad) pour une longitude ou une latitude.
    const METRES: f64 = 1e-6;
    const RADIANS: f64 = 1.75e-13;

    /// Plain, with the frame offsets (the longitude wrap of the inverse depends on `lon_0`), and
    /// `+over`, which keeps no vector path.
    const DEFINITIONS: [&str; 3] = [
        "+proj=webmerc +ellps=WGS84",
        "+proj=webmerc +ellps=WGS84 +lon_0=-170 +x_0=1000 +y_0=-2000",
        "+proj=webmerc +ellps=WGS84 +over",
    ];

    fn operation(definition: &str) -> Operation {
        Operation::create(definition).expect("webmerc")
    }

    fn close(got: f64, want: f64, floor: f64, relative: f64) -> bool {
        if got.is_nan() || want.is_nan() {
            return got.is_nan() && want.is_nan();
        }
        if got.is_infinite() || want.is_infinite() {
            return got == want;
        }
        (got - want).abs() <= floor.max(relative * want.abs())
    }

    fn coords(pairs: &[(f64, f64)]) -> Vec<Coord> {
        pairs
            .iter()
            .map(|&(x, y)| Coord {
                x,
                y,
                z: 0.0,
                t: 0.0,
            })
            .collect()
    }

    /// Points de la grille (bords compris, valeurs refusées et non finies) et `count` points tirés.
    fn grid(count: usize, lon: (f64, f64), lat: (f64, f64)) -> Vec<(f64, f64)> {
        let mut state = 0xDEAD_BEEF_u64;
        let mut unit = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[expect(
                clippy::cast_precision_loss,
                clippy::as_conversions,
                reason = "the top 53 bits of the generator are exact in f64"
            )]
            let u = ((state >> 11) as f64) / 9_007_199_254_740_992.0;
            u
        };
        let mut points: Vec<(f64, f64)> = (0..count)
            .map(|_| {
                (
                    lon.0 + unit() * (lon.1 - lon.0),
                    lat.0 + unit() * (lat.1 - lat.0),
                )
            })
            .collect();
        points.extend([
            (lon.0, lat.0),
            (lon.1, lat.1),
            (0.0, 0.0),
            (f64::NAN, 0.3),
            (0.3, f64::NAN),
            (f64::INFINITY, 0.3),
            (0.3, f64::NEG_INFINITY),
        ]);
        points
    }

    #[derive(Clone, Copy)]
    enum Way {
        Forward,
        Inverse,
    }

    /// Batch and one-by-one must agree; the failing slots must be the failing points. Returns
    /// how many outputs differ bit for bit from the one-by-one value (the vector path at work).
    fn assert_agrees(
        op: &Operation,
        points: &[Coord],
        way: Way,
        floor: f64,
        relative: f64,
    ) -> usize {
        let mut batch = points.to_vec();
        let mut failed = vec![None; points.len()];
        match way {
            Way::Forward => op.forward_batch(&mut batch, &mut failed),
            Way::Inverse => op.inverse_batch(&mut batch, &mut failed),
        }
        let mut differing = 0;
        for (slot, point) in points.iter().enumerate() {
            let one = match way {
                Way::Forward => op.forward(*point),
                Way::Inverse => op.inverse(*point),
            };
            match one {
                Ok(want) => {
                    assert_eq!(
                        failed[slot], None,
                        "slot {slot} {point:?}: batch fails alone"
                    );
                    let got = batch[slot];
                    assert!(
                        close(got.x, want.x, floor, relative)
                            && close(got.y, want.y, floor, relative),
                        "slot {slot} {point:?}: batch {got:?}, one by one {want:?}"
                    );
                    if got.x.to_bits() != want.x.to_bits() || got.y.to_bits() != want.y.to_bits() {
                        differing += 1;
                    }
                }
                Err(error) => assert_eq!(failed[slot], Some(error), "slot {slot} {point:?}"),
            }
        }
        differing
    }

    #[test]
    fn forward_batch_agrees_with_one_by_one() {
        let lon = (-4.0, 4.0);
        let lat = (-FRAC_PI_2 - 1e-6, FRAC_PI_2 + 1e-6);
        let mut pairs = grid(3_001, lon, lat);
        pairs.extend(
            [-PI, -PI + 1e-9, PI - 1e-9, PI, 9.9, 10.5, 1e300]
                .iter()
                .flat_map(|&x| [(x, 0.0), (x, FRAC_PI_2), (x, -FRAC_PI_2)]),
        );
        pairs.extend(
            [
                FRAC_PI_2,
                FRAC_PI_2 + 1e-11,
                -FRAC_PI_2 - 1e-11,
                FRAC_PI_2 + 1e-5,
                1e300,
            ]
            .iter()
            .map(|&y| (0.5, y)),
        );
        let points = coords(&pairs);
        for definition in DEFINITIONS {
            let differing =
                assert_agrees(&operation(definition), &points, Way::Forward, METRES, 1e-13);
            assert_vector_taken(definition, differing);
        }
    }

    #[test]
    fn inverse_batch_agrees_with_one_by_one() {
        let edge = 20_037_508.342_789_244;
        let mut pairs = grid(3_001, (-2.1 * edge, 2.1 * edge), (-2.1 * edge, 2.1 * edge));
        pairs.extend(
            [edge, -edge, edge + 1e-6, -edge - 1e-6, 1e300, -1e300, 1e308]
                .iter()
                .flat_map(|&x| [(x, 0.0), (x, 1e7), (x, -1e7)]),
        );
        let points = coords(&pairs);
        for definition in DEFINITIONS {
            let differing =
                assert_agrees(&operation(definition), &points, Way::Inverse, RADIANS, 0.0);
            assert_vector_taken(definition, differing);
        }
    }

    /// Under `+over` there is no vector path: nothing differs from the one-by-one values.
    fn assert_vector_taken(definition: &str, differing: usize) {
        if crate::simd::batch_path() == "avx2" && !definition.contains("+over") {
            assert!(
                differing > 0,
                "{definition}: no value differs bit for bit, the vector path was not taken"
            );
        }
    }

    /// The portable path (`simd::run` without AVX2) is `Lanes::point`, which is the scalar
    /// `Mercator` of the same formulas: the one-by-one values of this step, bit for bit.
    #[test]
    fn the_scalar_mercator_is_the_one_by_one_step_bit_for_bit() {
        let a = Ellipsoid::wgs84().a();
        let merc = Mercator::try_sphere(a, 0.0, 1.0, 0.0, 0.0).expect("sphere");
        let op = operation(DEFINITIONS[0]);
        for (lon, lat) in grid(2_001, (-PI, PI), (-FRAC_PI_2, FRAC_PI_2)) {
            if !(lon.is_finite() && lat.is_finite()) {
                continue;
            }
            let want = op.forward(Coord {
                x: lon,
                y: lat,
                z: 0.0,
                t: 0.0,
            });
            if let Ok(want) = want {
                let got = merc.forward_rad(lon, lat).expect("in domain");
                assert_eq!(
                    (got.east.to_bits(), got.north.to_bits()),
                    (want.x.to_bits(), want.y.to_bits()),
                    "{lon} {lat}"
                );
            }
        }
        let ra = Ellipsoid::wgs84().ra();
        for (x, y) in grid(2_001, (-2.1 * a, 2.1 * a), (-2.1 * a, 2.1 * a)) {
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            let want = op.inverse(Coord {
                x,
                y,
                z: 0.0,
                t: 0.0,
            });
            if let Ok(want) = want {
                let (lam, phi) = merc.inv_unit(x * ra, y * ra).expect("sphere");
                assert_eq!(
                    (adjlon(lam).to_bits(), phi.to_bits()),
                    (want.x.to_bits(), want.y.to_bits()),
                    "{x} {y}"
                );
            }
        }
    }
}
