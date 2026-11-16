//! The C-API calls of `test/unit/gie_self_tests.cpp` that have no method yet:
//! `proj_roundtrip`, `proj_trans_array`, `proj_xy_dist`, and the `proj_list_*`
//! enumerations. Each follows PROJ 9.9.0 (`trans.cpp`, `dist.cpp`, `apps` lists).

use crate::angle::adjlon;
use crate::engine::{
    geocentric_to_geographic, lpz_dist, xyz_dist, Coord, Operation, TransformError, UnitProjection,
};
use crate::iso19111::{create_from_user_input, Crs, DatabaseContext};
use crate::proj::{Direction, ProjError};
use crate::projmath::aasin;
use std::f64::consts::FRAC_PI_2;
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

/// `PJ_FACTORS`: the cartographic characteristics of a projection at one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Factors {
    /// Meridian scale `h`.
    pub meridional_scale: f64,
    /// Parallel scale `k`.
    pub parallel_scale: f64,
    /// Areal scale `s`.
    pub areal_scale: f64,
    /// Angular distortion `omega`, in radians.
    pub angular_distortion: f64,
    /// Meridian-parallel angle `theta'`, in radians.
    pub meridian_parallel_angle: f64,
    /// Meridian convergence, in radians.
    pub meridian_convergence: f64,
    /// Tissot ellipse semi-major axis.
    pub tissot_semimajor: f64,
    /// Tissot ellipse semi-minor axis.
    pub tissot_semiminor: f64,
    /// Derivative of x with respect to the longitude.
    pub dx_dlam: f64,
    /// Derivative of x with respect to the latitude.
    pub dx_dphi: f64,
    /// Derivative of y with respect to the longitude.
    pub dy_dlam: f64,
    /// Derivative of y with respect to the latitude.
    pub dy_dphi: f64,
}

/// `DEFAULT_H` of `factors.cpp`: the step of the numerical derivatives, in radians.
const DEFAULT_STEP: f64 = 1e-5;
/// `EPS` of `factors.cpp`.
const FACTOR_EPS: f64 = 1e-12;

impl Operation {
    /// `proj_factors` of a projection: the numerical derivatives of the forward transform
    /// (`pj_deriv`, central differences of step `1e-5` rad) and the scale factors of
    /// `pj_factors`. Input is longitude and latitude in radians.
    ///
    /// A step with a `P->fwd` (a projection, `cart`, `geocent`) is differentiated as PROJ
    /// differentiates that raw forward: no unit conversion and no semi-major axis. Any other
    /// operation is differentiated in metres, divided by the semi-major axis.
    ///
    /// # Errors
    ///
    /// [`TransformError::InvalidCoord`] when the latitude or longitude is out of
    /// range, and [`TransformError::OutsideDomain`] when a stencil point has no image.
    pub fn factors(&self, lon: f64, lat: f64) -> Result<Factors, TransformError> {
        match self.unit_projection() {
            Some(unit) => unit_factors(unit, lon, lat),
            None => self.factors_in_metres(lon, lat, self.output_to_metre()),
        }
    }

    /// [`Self::factors`] when one unit of the forward output is `to_metre` metres.
    pub(crate) fn factors_in_metres(
        &self,
        lon: f64,
        lat: f64,
        to_metre: f64,
    ) -> Result<Factors, TransformError> {
        check_factor_range(lon, lat)?;
        let (a, es) = self.distance_ellipsoid();
        let lat = inside_pole(lat, DEFAULT_STEP);
        let point = |l: f64, p: f64| -> Result<(f64, f64), TransformError> {
            let out = self.forward(Coord {
                x: l,
                y: p,
                z: 0.0,
                t: 0.0,
            })?;
            Ok((out.x * to_metre / a, out.y * to_metre / a))
        };
        let [x_l, y_p, x_p, y_l] = pj_deriv(lon, lat, DEFAULT_STEP, point)?;
        factors_from_derivatives(x_l, y_p, x_p, y_l, lat, es, 1.0 - es)
    }
}

/// `pj_factors` of `factors.cpp` for a projection with a unit-ellipsoid forward. The geocentric
/// latitude is converted and the central meridian removed, then `pj_deriv` differentiates the
/// unit forward itself. The factors need neither `to_meter` nor the semi-major axis.
fn unit_factors(unit: UnitProjection<'_>, lon: f64, lat: f64) -> Result<Factors, TransformError> {
    check_factor_range(lon, lat)?;
    let frame = unit.frame;
    let lat = if frame.geoc {
        geocentric_to_geographic(lat, frame.earth.one_es())
    } else {
        lat
    };
    let lat = inside_pole(lat, DEFAULT_STEP);
    let mut lam = lon - unit.forward.lam0();
    if !frame.over {
        lam = adjlon(lam);
    }
    let [x_l, y_p, x_p, y_l] = pj_deriv(lam, lat, DEFAULT_STEP, |l, p| {
        unit.forward.unit_forward(l, p)
    })?;
    factors_from_derivatives(
        x_l,
        y_p,
        x_p,
        y_l,
        lat,
        frame.factor_es,
        frame.factor_one_es,
    )
}

fn check_factor_range(lon: f64, lat: f64) -> Result<(), TransformError> {
    if (lat.abs() - FRAC_PI_2) > FACTOR_EPS || lon.abs() > 10.0 {
        return Err(TransformError::InvalidCoord);
    }
    Ok(())
}

/// `pj_factors` keeps a latitude beyond one step from the pole inside it, so the stencil exists.
fn inside_pole(lat: f64, h: f64) -> f64 {
    let limit = FRAC_PI_2 - h;
    if lat.abs() <= limit {
        lat
    } else if lat < 0.0 {
        -limit
    } else {
        limit
    }
}

/// `pj_deriv` of `deriv.cpp`: central differences over the stencil (+,+), (+,-), (-,-), (-,+) in
/// (lam, phi). The steps are taken in PROJ's order so that the rounding is PROJ's too. Returns
/// `[x_lam, y_phi, x_phi, y_lam]`.
fn pj_deriv(
    lam: f64,
    phi: f64,
    h: f64,
    mut forward: impl FnMut(f64, f64) -> Result<(f64, f64), TransformError>,
) -> Result<[f64; 4], TransformError> {
    let (lam, phi) = (lam + h, phi + h);
    if phi.abs() > FRAC_PI_2 {
        return Err(TransformError::OutsideDomain);
    }
    let step = 2.0 * h;
    let (x, y) = forward(lam, phi)?;
    let (mut x_l, mut y_p, mut x_p, mut y_l) = (x, y, x, y);
    let phi = phi - step;
    if phi.abs() > FRAC_PI_2 {
        return Err(TransformError::OutsideDomain);
    }
    let (x, y) = forward(lam, phi)?;
    x_l += x;
    y_p -= y;
    x_p -= x;
    y_l += y;
    let lam = lam - step;
    let (x, y) = forward(lam, phi)?;
    x_l -= x;
    y_p -= y;
    x_p -= x;
    y_l -= y;
    let phi = phi + step;
    let (x, y) = forward(lam, phi)?;
    x_l -= x;
    y_p += y;
    x_p += x;
    y_l -= y;
    let divisor = 2.0 * step;
    Ok([x_l / divisor, y_p / divisor, x_p / divisor, y_l / divisor])
}

/// The scale factors of `pj_factors` from the derivatives of the forward transform, at a
/// latitude already clamped inside the pole, on an ellipsoid of squared eccentricity `es`.
fn factors_from_derivatives(
    x_l: f64,
    y_p: f64,
    x_p: f64,
    y_l: f64,
    lat: f64,
    es: f64,
    one_es: f64,
) -> Result<Factors, TransformError> {
    let cosphi = lat.cos();
    let mut hs = x_p.hypot(y_p);
    let mut k = x_l.hypot(y_l) / cosphi;
    let r = if es == 0.0 {
        1.0
    } else {
        let mut t = lat.sin();
        t = 1.0 - es * t * t;
        let n = t.sqrt();
        hs *= t * n / one_es;
        k *= n;
        t * t / one_es
    };
    let conv = -x_p.atan2(y_p);
    let s = (y_p * x_l - x_p * y_l) * r / cosphi;
    let thetap = aasin(s / (hs * k)).map_err(|()| TransformError::OutsideDomain)?;
    let mut t = k * k + hs * hs;
    let a_axis = (t + 2.0 * s).sqrt();
    t -= 2.0 * s;
    let t = if t > 0.0 { t.sqrt() } else { 0.0 };
    let b = 0.5 * (a_axis - t);
    let a_axis = 0.5 * (a_axis + t);
    let omega =
        2.0 * aasin((a_axis - b) / (a_axis + b)).map_err(|()| TransformError::OutsideDomain)?;
    Ok(Factors {
        meridional_scale: hs,
        parallel_scale: k,
        areal_scale: s,
        angular_distortion: omega,
        meridian_parallel_angle: thetap,
        meridian_convergence: conv,
        tissot_semimajor: a_axis,
        tissot_semiminor: b,
        dx_dlam: x_l,
        dx_dphi: x_p,
        dy_dlam: y_l,
        dy_dphi: y_p,
    })
}

/// `proj_factors` of a single CRS, which [`crate::Proj::new`] keeps as its text: an `EPSG:`
/// code, WKT, or PROJJSON. PROJ's `proj_factors` (`factors.cpp`) takes the projection of the
/// CRS: the conversion of a projected CRS, the horizontal CRS of a compound CRS, and the base
/// projection with the deriving affine of a derived projected CRS. A geographic CRS has no
/// projection: `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE`, which is [`ProjError::NotATransform`].
///
/// # Errors
///
/// [`ProjError::Definition`] when the text does not read as a CRS, [`ProjError::NotATransform`]
/// for a CRS without a projection, and [`ProjError::InvalidCoordinate`] for a latitude or
/// longitude that `proj_factors` rejects.
pub(crate) fn crs_factors(text: &str, lon: f64, lat: f64) -> Result<Factors, ProjError> {
    let db = DatabaseContext::create().map_err(|_| ProjError::Definition)?;
    let crs = create_from_user_input(text, &db).map_err(|_| ProjError::Definition)?;
    crs_factors_of(&crs, lon, lat)
}

fn crs_factors_of(crs: &Crs, lon: f64, lat: f64) -> Result<Factors, ProjError> {
    let factors = match crs {
        // `proj_crs_get_sub_crs(ctx, pj, 0)`: the first component, the horizontal CRS.
        Crs::Compound(compound) => {
            let first = compound
                .components()
                .first()
                .ok_or(ProjError::NotATransform)?;
            return crs_factors_of(first, lon, lat);
        }
        Crs::Projected(projected) => {
            let definition = Crs::Projected(projected.clone())
                .export_to_proj_string()
                .map_err(|_| ProjError::Definition)?;
            let op =
                Operation::create_projection(&definition).map_err(|_| ProjError::Definition)?;
            op.factors(lon, lat)
        }
        Crs::DerivedProjected(derived) => {
            let (definition, unit) = crate::crs::derived_projected_pipeline(derived)
                .map_err(|_| ProjError::Definition)?;
            let op =
                Operation::create_projection(&definition).map_err(|_| ProjError::Definition)?;
            op.factors_in_metres(lon, lat, unit)
        }
        _ => return Err(ProjError::NotATransform),
    };
    factors.map_err(|_| ProjError::InvalidCoordinate)
}
