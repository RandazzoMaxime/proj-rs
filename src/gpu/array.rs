//! `Proj::convert_array_gpu`: a slice through the GPU, block by block, with the CPU path as the
//! fallback of every block the GPU cannot guarantee.

use super::device::Block;
use super::GpuContext;
use crate::proj::{Coord, CoordinateType, Direction, ProjError};
use crate::Proj;

/// Why part of a call ran on the CPU. The first reason of the call.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Fallback {
    /// Fewer points than [`GpuOptions::min_points`](super::GpuOptions::min_points).
    TooFewPoints,
    /// A step of the transform has no GPU kernel: a grid shift, a datum shift, a projection
    /// that is not covered, a transform that picks its operation per point.
    Unsupported,
    /// A block held a point the GPU does not decide: a latitude beyond the pole, a longitude
    /// far outside the range, a non-finite value, a point outside the domain of the projection.
    /// The CPU path then runs that block and reports what it reports.
    Flagged,
    /// The device failed. The message is the driver's.
    Device(String),
}

/// What [`Proj::convert_array_gpu`] did.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GpuReport {
    /// Points the GPU converted.
    pub gpu_points: usize,
    /// Points the CPU converted.
    pub cpu_points: usize,
    /// Why the CPU converted any, when it did.
    pub fallback: Option<Fallback>,
}

impl Proj {
    /// [`Self::convert_array`], on the GPU when every step of the transform has a GPU kernel.
    ///
    /// The kernels cover the axis swaps and unit conversions, the exact transverse Mercator
    /// (UTM and Gauss-Krüger), the Lambert conformal conic, and the spherical Mercator (Web
    /// Mercator). Any other transform, a call below [`GpuOptions::min_points`], and any block
    /// with a point the GPU does not decide run on the CPU path, with the result and the error
    /// [`Self::convert_array`] gives. The report says which.
    ///
    /// The GPU result agrees with the CPU to a micrometre, not bit for bit: its sine, its
    /// logarithm and the rest are its own.
    ///
    /// # Errors
    ///
    /// [`ProjError`] as [`Self::convert_array`]. A device failure is not an error: that block
    /// and the rest of the call run on the CPU.
    ///
    /// [`GpuOptions::min_points`]: super::GpuOptions::min_points
    pub fn convert_array_gpu<'a, C, F>(
        &self,
        points: &'a mut [C],
        gpu: &GpuContext,
    ) -> Result<(&'a mut [C], GpuReport), ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        self.convert_array_gpu_in(points, Direction::Forward, gpu)
    }

    /// Seconds per dispatch of this transform over the first `chunk_points` of `points`, the data
    /// uploaded once and nothing read back: the compute alone, for the benchmark. Also the number
    /// of points of one dispatch. `None` when the transform has no GPU program or the device
    /// refuses it.
    #[doc(hidden)]
    pub fn gpu_dispatch_seconds(
        &self,
        points: &[(f64, f64)],
        direction: Direction,
        gpu: &GpuContext,
        repeats: u32,
    ) -> Option<(usize, f64)> {
        let program = self.gpu_program(direction)?;
        let count = points.len().min(gpu.chunk_points());
        let block: Vec<[f64; 2]> = points[..count].iter().map(|&(x, y)| [x, y]).collect();
        let mut session = gpu.session(&program.encode(), count).ok()?;
        // One dispatch to warm the pipeline, then the measure.
        session.time_dispatches(&block, 1).ok()?;
        let seconds = session.time_dispatches(&block, repeats).ok()?;
        Some((count, seconds / f64::from(repeats)))
    }

    /// [`Self::convert_array_gpu`] in `direction`.
    ///
    /// # Errors
    ///
    /// [`ProjError`] as [`Self::convert_array_gpu`].
    pub fn convert_array_gpu_in<'a, C, F>(
        &self,
        points: &'a mut [C],
        direction: Direction,
        gpu: &GpuContext,
    ) -> Result<(&'a mut [C], GpuReport), ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        let total = points.len();
        let mut report = GpuReport::default();
        if total < gpu.options().min_points {
            return all_on_cpu(self, points, direction, report, Fallback::TooFewPoints);
        }
        let program = match self.gpu_program(direction) {
            Some(program) if program.has_projection() => program,
            _ => return all_on_cpu(self, points, direction, report, Fallback::Unsupported),
        };
        // As `fill_array`: a coordinate that is not an `f64` is refused first, slice untouched.
        for point in &*points {
            point.x().into_f64()?;
            point.y().into_f64()?;
        }
        let encoded = program.encode();
        let chunk = gpu.chunk_points().min(total);
        let mut session = match gpu.session(&encoded, chunk) {
            Ok(session) => session,
            Err(why) => {
                return all_on_cpu(self, points, direction, report, Fallback::Device(why));
            }
        };
        let mut block: Vec<[f64; 2]> = Vec::with_capacity(chunk);
        let mut narrowing = None;
        let mut start = 0;
        while start < total {
            let end = (start + chunk).min(total);
            let window = &mut points[start..end];
            block.clear();
            for point in &*window {
                block.push([point.x().into_f64()?, point.y().into_f64()?]);
            }
            match session.run(&mut block) {
                Ok(Block::Done) => {
                    for (point, value) in window.iter_mut().zip(&block) {
                        match (F::from_f64(value[0]), F::from_f64(value[1])) {
                            (Ok(x), Ok(y)) => *point = C::from_xy(x, y),
                            (Err(error), _) | (_, Err(error)) => {
                                narrowing.get_or_insert(error);
                            }
                        }
                    }
                    report.gpu_points += window.len();
                }
                Ok(Block::Flagged) => {
                    cpu_block(self, window, direction, &mut narrowing)?;
                    report.cpu_points += end - start;
                    report.fallback.get_or_insert(Fallback::Flagged);
                }
                Err(why) => {
                    // The device is not trusted for the rest of the call.
                    drop(session);
                    let rest = &mut points[start..];
                    cpu_block(self, rest, direction, &mut narrowing)?;
                    report.cpu_points += total - start;
                    report.fallback.get_or_insert(Fallback::Device(why));
                    return match narrowing {
                        Some(error) => Err(error),
                        None => Ok((points, report)),
                    };
                }
            }
            start = end;
        }
        match narrowing {
            Some(error) => Err(error),
            None => Ok((points, report)),
        }
    }
}

/// The whole slice on the CPU path.
fn all_on_cpu<'a, C, F>(
    proj: &Proj,
    points: &'a mut [C],
    direction: Direction,
    report: GpuReport,
    why: Fallback,
) -> Result<(&'a mut [C], GpuReport), ProjError>
where
    C: Coord<F>,
    F: CoordinateType,
{
    let count = points.len();
    let points = proj.fill_array(points, direction)?;
    Ok((
        points,
        GpuReport {
            cpu_points: report.cpu_points + count,
            fallback: report.fallback.or(Some(why)),
            ..report
        },
    ))
}

/// A block on the CPU path. A value that does not fit the slice type is the same deferred error
/// as on the GPU path; any other error ends the call.
fn cpu_block<C, F>(
    proj: &Proj,
    window: &mut [C],
    direction: Direction,
    narrowing: &mut Option<ProjError>,
) -> Result<(), ProjError>
where
    C: Coord<F>,
    F: CoordinateType,
{
    match proj.fill_array(window, direction) {
        Ok(_) => Ok(()),
        Err(error @ ProjError::FloatConversion) => {
            narrowing.get_or_insert(error);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    //! What each transform lowers to. No device is needed: these run on every machine.

    use super::super::Opcode;
    use crate::proj::Direction;
    use crate::Proj;

    fn opcodes(proj: &Proj, way: Direction) -> Option<Vec<u32>> {
        proj.gpu_program(way)
            .map(|program| program.encode().instructions.iter().map(|i| i[0]).collect())
    }

    /// `needle` appears in `haystack` as consecutive items.
    fn contains_run(haystack: &[u32], needle: &[Opcode]) -> bool {
        let needle: Vec<u32> = needle.iter().map(|op| op.code()).collect();
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn pair(from: &str, to: &str) -> Proj {
        Proj::new_known_crs(from, to, None).expect("pair")
    }

    #[test]
    fn utm_lowers_to_prepare_kernel_finalize() {
        let utm = pair("EPSG:4326", "EPSG:32631");
        let forward = opcodes(&utm, Direction::Forward).expect("forward");
        assert!(contains_run(
            &forward,
            &[Opcode::PrepFwd, Opcode::TmercFwd, Opcode::FinishFwd]
        ));
        let inverse = opcodes(&utm, Direction::Inverse).expect("inverse");
        assert!(contains_run(
            &inverse,
            &[Opcode::PrepInv, Opcode::TmercInv, Opcode::FinishInv]
        ));
    }

    #[test]
    fn web_mercator_and_lambert_have_their_kernels() {
        let web = opcodes(&pair("EPSG:4326", "EPSG:3857"), Direction::Forward).expect("web");
        assert!(web.contains(&Opcode::MercFwd.code()), "{web:?}");
        let lambert = opcodes(&pair("EPSG:4326", "EPSG:2154"), Direction::Forward).expect("lcc");
        assert!(lambert.contains(&Opcode::LccFwd.code()), "{lambert:?}");
    }

    #[test]
    fn two_projections_make_one_program() {
        let both = opcodes(&pair("EPSG:2154", "EPSG:32631"), Direction::Forward).expect("both");
        let lcc = both
            .iter()
            .position(|&c| c == Opcode::LccInv.code())
            .expect("lcc");
        let tmerc = both
            .iter()
            .position(|&c| c == Opcode::TmercFwd.code())
            .expect("tmerc");
        assert!(lcc < tmerc, "{both:?}");
    }

    #[test]
    fn a_step_without_a_kernel_has_no_program() {
        // Grids and a per-point choice of operation.
        assert!(opcodes(&pair("EPSG:4267", "EPSG:4326"), Direction::Forward).is_none());
        for definition in [
            "+proj=merc +ellps=WGS84",
            "+proj=utm +zone=31 +ellps=WGS84 +approx",
            "+proj=stere +lat_0=90 +ellps=WGS84",
            "+proj=utm +zone=31 +ellps=WGS84 +over",
            "+proj=pipeline +step +proj=push +v_3 +step +proj=utm +zone=31 +ellps=WGS84 \
             +step +proj=pop +v_3",
            "+proj=pipeline +step +proj=cart +ellps=WGS84 +step +proj=helmert +x=1",
        ] {
            let proj = Proj::new(definition).expect(definition);
            assert!(
                opcodes(&proj, Direction::Forward).is_none(),
                "{definition} must stay on the CPU"
            );
        }
    }

    #[test]
    fn an_axis_swap_alone_is_not_worth_a_trip() {
        let swap = Proj::new("+proj=pipeline +step +proj=axisswap +order=2,1").expect("axisswap");
        let program = swap.gpu_program(Direction::Forward).expect("lowers");
        assert!(!program.has_projection());
    }

    #[test]
    fn a_geocentric_latitude_stays_on_the_cpu() {
        let proj = Proj::new("+proj=utm +zone=31 +ellps=WGS84 +geoc").expect("geoc");
        assert!(opcodes(&proj, Direction::Forward).is_none());
    }
}
