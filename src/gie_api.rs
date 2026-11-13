//! The C-API calls of `test/unit/gie_self_tests.cpp` that have no method yet:
//! `proj_roundtrip`, `proj_trans_array`, `proj_xy_dist`, and the `proj_list_*`
//! enumerations. Each follows PROJ 9.9.0 (`trans.cpp`, `dist.cpp`, `apps` lists).

use crate::engine::{lpz_dist, xyz_dist, Coord, Operation, TransformError};
use crate::proj::Direction;
use std::num::NonZeroUsize;

/// The error of [`Operation::transform_array`], as `proj_trans_array` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchError {
    /// Every failed point failed for this one reason.
    Uniform(TransformError),
    /// The failed points failed for different reasons: `PROJ_ERR_COORD_TRANSFM`.
    Mixed,
}

impl Operation {
    /// `proj_roundtrip`: transform `coord` in `direction`, then take `steps - 1` full
    /// round trips and one half step back. Returns the end point and the distance to
    /// the start: a geodesic and height distance when `direction` reads angles, else
    /// Euclidean. A start with a NaN component must end all NaN, which is distance 0.
    ///
    /// # Errors
    ///
    /// The first [`TransformError`] of any step.
    pub fn roundtrip(
        &self,
        direction: Direction,
        steps: NonZeroUsize,
        coord: Coord,
    ) -> Result<(Coord, f64), TransformError> {
        let inverse = direction == Direction::Inverse;
        let back = |c: Coord| {
            if inverse {
                self.forward(c)
            } else {
                self.inverse(c)
            }
        };
        let step = |c: Coord| {
            if inverse {
                self.inverse(c)
            } else {
                self.forward(c)
            }
        };
        let origin = coord;
        let mut t = step(origin)?;
        for _ in 0..steps.get() - 1 {
            t = step(back(t)?)?;
        }
        t = back(t)?;
        let all_nan = t.x.is_nan() && t.y.is_nan() && t.z.is_nan() && t.t.is_nan();
        let origin_nan = origin.x.is_nan() || origin.y.is_nan() || origin.z.is_nan();
        if origin_nan && all_nan {
            return Ok((t, 0.0));
        }
        let distance = if self.angular_input(inverse) {
            let (a, es) = self.distance_ellipsoid();
            lpz_dist(a, es, origin, t)
        } else {
            xyz_dist(origin, t)
        };
        Ok((t, distance))
    }

    /// `proj_trans_array`: transform every point, even after a failure. A failed point
    /// has all four components set to `+inf` (`HUGE_VAL`).
    ///
    /// # Errors
    ///
    /// [`BatchError::Uniform`] when every failure has one reason, [`BatchError::Mixed`]
    /// when two failures differ. `Ok` when no point fails.
    pub fn transform_array(
        &self,
        direction: Direction,
        points: &mut [Coord],
    ) -> Result<(), BatchError> {
        let mut first: Option<TransformError> = None;
        let mut uniform = true;
        for point in points.iter_mut() {
            let result = match direction {
                Direction::Forward => self.forward(*point),
                Direction::Inverse => self.inverse(*point),
            };
            match result {
                Ok(value) => *point = value,
                Err(error) => {
                    *point = Coord {
                        x: f64::INFINITY,
                        y: f64::INFINITY,
                        z: f64::INFINITY,
                        t: f64::INFINITY,
                    };
                    match first {
                        None => first = Some(error),
                        Some(seen) if seen != error => uniform = false,
                        Some(_) => {}
                    }
                }
            }
        }
        match first {
            None => Ok(()),
            Some(error) if uniform => Err(BatchError::Uniform(error)),
            Some(_) => Err(BatchError::Mixed),
        }
    }
}

/// `proj_xy_dist`: the Euclidean distance of the first two components, ignoring
/// height and time.
#[must_use]
pub fn xy_dist(a: Coord, b: Coord) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
/// The projection ids of `proj -l` in PROJ 9.9.0, in its order.
const OPERATIONS: &[&str] = &[
    "adams_hemi",
    "adams_ws1",
    "adams_ws2",
    "aea",
    "aeqd",
    "affine",
    "airy",
    "aitoff",
    "alsk",
    "apian",
    "august",
    "axisswap",
    "bacon",
    "bertin1953",
    "bipc",
    "boggs",
    "bonne",
    "calcofi",
    "cart",
    "cass",
    "cc",
    "ccon",
    "cea",
    "chamb",
    "collg",
    "col_urban",
    "comill",
    "crast",
    "defmodel",
    "deformation",
    "denoy",
    "dsea",
    "airocean",
    "eck1",
    "eck2",
    "eck3",
    "eck4",
    "eck5",
    "eck6",
    "eqearth",
    "eqc",
    "eqdc",
    "euler",
    "etmerc",
    "fahey",
    "fouc",
    "fouc_s",
    "gall",
    "geoc",
    "geogoffset",
    "geos",
    "gins8",
    "gn_sinu",
    "gnom",
    "goode",
    "gridshift",
    "gs48",
    "gs50",
    "guyou",
    "hammer",
    "hatano",
    "healpix",
    "rhealpix",
    "helmert",
    "hgridshift",
    "horner",
    "hourglass",
    "igh",
    "igh_o",
    "imoll",
    "imoll_o",
    "imw_p",
    "interrupted",
    "isea",
    "ivea",
    "kav5",
    "kav7",
    "krovak",
    "labrd",
    "laea",
    "lagrng",
    "larr",
    "lask",
    "lonlat",
    "latlon",
    "lcc",
    "lcca",
    "leac",
    "lee_os",
    "loxim",
    "lsat",
    "mbt_s",
    "mbt_fps",
    "mbtfpp",
    "mbtfpq",
    "mbtfps",
    "merc",
    "mil_os",
    "mill",
    "misrsom",
    "mod_krovak",
    "moll",
    "molobadekas",
    "molodensky",
    "murd1",
    "murd2",
    "murd3",
    "natearth",
    "natearth2",
    "nell",
    "nell_h",
    "nicol",
    "nsper",
    "nzmg",
    "noop",
    "ob_tran",
    "ocea",
    "oea",
    "omerc",
    "ortel",
    "ortho",
    "pconic",
    "patterson",
    "peirce_q",
    "pipeline",
    "poly",
    "pop",
    "push",
    "putp1",
    "putp2",
    "putp3",
    "putp3p",
    "putp4p",
    "putp5",
    "putp5p",
    "putp6",
    "putp6p",
    "qua_aut",
    "qsc",
    "robin",
    "rouss",
    "rpoly",
    "s2",
    "sch",
    "set",
    "sinu",
    "som",
    "somerc",
    "spilhaus",
    "stere",
    "sterea",
    "gstmerc",
    "tcc",
    "tcea",
    "times",
    "tinshift",
    "tissot",
    "tmerc",
    "tmerczoned",
    "tobmerc",
    "topocentric",
    "tpeqd",
    "tpers",
    "tsea",
    "unitconvert",
    "ups",
    "urm5",
    "urmfps",
    "utm",
    "vandg",
    "vandg2",
    "vandg3",
    "vandg4",
    "vertoffset",
    "vitk1",
    "vgridshift",
    "wag1",
    "wag2",
    "wag3",
    "wag4",
    "wag5",
    "wag6",
    "wag7",
    "webmerc",
    "weren",
    "wink1",
    "wink2",
    "wintri",
    "xyzgridshift",
];

/// `proj_list_operations`: the projection ids PROJ 9.9.0 builds.
#[must_use]
pub fn list_operations() -> &'static [&'static str] {
    OPERATIONS
}

/// `proj_list_ellps`: the ellipsoid ids of the built-in table.
#[must_use]
pub fn list_ellps() -> Vec<&'static str> {
    crate::ellipsoid::PJ_ELLPS.iter().map(|row| row.0).collect()
}

/// `proj_list_prime_meridians`: the prime meridian ids and their longitudes in degrees.
#[must_use]
pub fn list_prime_meridians() -> &'static [(&'static str, f64)] {
    crate::iso19111::PRIME_MERIDIANS
}
