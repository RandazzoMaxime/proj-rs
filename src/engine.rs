#![allow(
    clippy::pedantic,
    reason = "parameter rules and formulae follow PROJ's proj_create and fwd.cpp"
)]

//! PROJ strings, the shared frame (`lam0`, false offsets, metres) and the
//! operation a pipeline step calls.
//!
//! A classic projection kernel returns coordinates in units of the semi-major
//! axis. The frame then applies `a`, the false origin and `to_meter`, which is
//! what `src/fwd.cpp` does for `PJ_IO_UNITS_CLASSIC`.

use crate::angle::adjlon;

use crate::tmerc_approx::ApproxTmerc;
use crate::{
    Cone, Ellipsoid, EllipsoidError, Geographic, Hemisphere, Lcc, Mercator, Projected,
    ProjectionError, Tmerc, UtmZone,
};
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;
use std::sync::Mutex;

#[path = "projections/datum_ops.rs"]
mod datum_ops;

// NKG URNs are database coordinate operations, not `+proj` names.
#[path = "projections/nkg.rs"]
mod nkg;

// `+towgs84`, `+nadgrids`, `+geoidgrids`, and `+axis` around a classic step.
#[path = "projections/legacy.rs"]
mod legacy;

const EPS_LAT: f64 = 1e-12;

/// Four-dimensional coordinate. Angular axes are radians inside an operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coord {
    /// First axis.
    pub x: f64,
    /// Second axis.
    pub y: f64,
    /// Third axis. Zero when the caller gave only two dimensions.
    pub z: f64,
    /// Time, in years when a test supplies it.
    pub t: f64,
}

impl Coord {
    /// Two-dimensional coordinate, height and time zero.
    #[must_use]
    pub fn xy(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            z: 0.0,
            t: 0.0,
        }
    }
}

/// The operation string names something this crate does not build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreateError {
    /// `+proj` is not a ported operation.
    UnknownProj(String),
    /// A parameter is not accepted by this operation.
    UnknownParameter(String),
    /// A parameter value is outside the domain PROJ rejects at creation.
    IllegalValue(&'static str),
    /// A required parameter is absent.
    MissingArgument(&'static str),
    /// The operation needs a grid, a datum shift, or another deferred piece.
    Unsupported(&'static str),
}

impl fmt::Display for CreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProj(name) => write!(f, "unknown +proj={name}"),
            Self::UnknownParameter(name) => write!(f, "unknown parameter +{name}"),
            Self::IllegalValue(name) => write!(f, "illegal value for +{name}"),
            Self::MissingArgument(name) => write!(f, "missing +{name}"),
            Self::Unsupported(what) => write!(f, "{what} is not built"),
        }
    }
}

impl std::error::Error for CreateError {}

/// A coordinate transform failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformError {
    /// PROJ would return `HUGE_VAL` and `coord_transfm_outside_projection_domain`.
    OutsideDomain,
    /// PROJ `coord_transfm_outside_grid`: the point is not covered by a shift grid.
    OutsideGrid,
    /// PROJ `PROJ_ERR_COORD_TRANSFM_MISSING_TIME`.
    MissingTime,
    /// This direction is not defined.
    NoInverse,
    /// Latitude or longitude is not usable.
    InvalidCoord,
    /// A coordinate was not finite.
    NonFinite,
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutsideDomain => "point is outside the projection domain",
            Self::OutsideGrid => "point is outside the grid",
            Self::MissingTime => "coordinate is missing a time",
            Self::NoInverse => "inverse is not available",
            Self::InvalidCoord => "coordinate is invalid",
            Self::NonFinite => "coordinate is not finite",
        })
    }
}

impl std::error::Error for TransformError {}

impl From<ProjectionError> for TransformError {
    fn from(error: ProjectionError) -> Self {
        match error {
            ProjectionError::OutsideDomain
            | ProjectionError::InvalidLatitude
            | ProjectionError::InvalidParallel
            | ProjectionError::DegenerateCone => Self::OutsideDomain,
            ProjectionError::NonFinite => Self::NonFinite,
            ProjectionError::InvalidScale | ProjectionError::InvalidZone => Self::InvalidCoord,
        }
    }
}

/// Parsed `+key=value` list. The first occurrence of a key wins, matching PROJ.
#[derive(Clone, Debug)]
pub(crate) struct Params {
    items: Vec<(String, Option<String>)>,
}

impl Params {
    /// Parse a PROJ string. `+` is optional. `#` starts a comment.
    ///
    /// # Errors
    ///
    /// [`CreateError::IllegalValue`] when a token is not `key` or `key=value`.
    pub(crate) fn parse(input: &str) -> Result<Self, CreateError> {
        let input = shrink_equals(input);
        let mut items = Vec::new();
        for raw_line in input.lines() {
            let line = raw_line.split('#').next().unwrap_or("");
            for token in line.split_whitespace() {
                let token = token.trim_start_matches('+');
                if token.is_empty() {
                    continue;
                }
                if let Some((key, value)) = token.split_once('=') {
                    if key.is_empty() {
                        return Err(CreateError::IllegalValue("parameter"));
                    }
                    push_first(&mut items, key, Some(value.to_owned()));
                } else if comma_continuation(&items, token) {
                    // `towgs84 = 1, 2, 3` survives as separate tokens after `=`.
                    if let Some((_, Some(value))) = items.last_mut() {
                        value.push_str(token);
                    }
                } else {
                    push_first(&mut items, token, None);
                }
            }
        }
        Ok(Self { items })
    }

    fn get(&self, key: &str) -> Option<&Option<String>> {
        self.items
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// Keys in first-seen order.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.items
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_deref()))
    }

    fn insert_missing(&mut self, key: &str, value: Option<String>) {
        if !self.flag(key) {
            self.items.push((key.to_owned(), value));
        }
    }

    fn remove(&mut self, key: &str) {
        self.items.retain(|(name, _)| name != key);
    }

    /// The key is present, with or without a value.
    #[must_use]
    pub(crate) fn flag(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Raw text after `=`.
    #[must_use]
    pub(crate) fn text(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|value| value.as_deref())
    }

    /// Numeric parameter, underscores allowed.
    ///
    /// # Errors
    ///
    /// [`CreateError::IllegalValue`] when the text is not a number.
    pub(crate) fn number(&self, key: &str) -> Result<Option<f64>, CreateError> {
        let Some(text) = self.text(key) else {
            return Ok(None);
        };
        proj_number(text)
            .map(Some)
            .ok_or(CreateError::IllegalValue(leak_key(key)))
    }

    /// Angle parameter, decimal degrees or DMS, returned in radians.
    ///
    /// # Errors
    ///
    /// [`CreateError::IllegalValue`] when the text is not an angle.
    pub(crate) fn radians(&self, key: &str) -> Result<Option<f64>, CreateError> {
        let Some(text) = self.text(key) else {
            return Ok(None);
        };
        let text = text.trim();
        // PROJ `r` suffix: the number is already in radians.
        if let Some(stem) = text.strip_suffix(['r', 'R']) {
            return proj_number(stem.trim())
                .map(Some)
                .ok_or(CreateError::IllegalValue(leak_key(key)));
        }
        parse_dms_degrees(text)
            .map(|deg| Some(deg.to_radians()))
            .ok_or(CreateError::IllegalValue(leak_key(key)))
    }

    /// Integer parameter.
    ///
    /// # Errors
    ///
    /// [`CreateError::IllegalValue`] when the text is not an integer.
    pub(crate) fn integer(&self, key: &str) -> Result<Option<i32>, CreateError> {
        let Some(text) = self.text(key) else {
            return Ok(None);
        };
        if !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'+' || byte == b'-')
        {
            return Err(CreateError::IllegalValue(leak_key(key)));
        }
        let Some(value) = self.number(key)? else {
            return Err(CreateError::IllegalValue(leak_key(key)));
        };
        if value.fract() != 0.0 || value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
            return Err(CreateError::IllegalValue(leak_key(key)));
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::as_conversions,
            reason = "the range was checked against i32"
        )]
        Ok(Some(value as i32))
    }

    fn keys(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|(key, _)| key.as_str())
    }
}

fn comma_continuation(items: &[(String, Option<String>)], token: &str) -> bool {
    let Some((_, Some(value))) = items.last() else {
        return false;
    };
    if !value.ends_with(',') {
        return false;
    }
    token.starts_with('-')
        || token.starts_with('+')
        || token.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn shrink_equals(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_whitespace() || ch == ';' {
            while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
                chars.next();
            }
            let around_equals = chars.peek() == Some(&'=') || out.ends_with('=');
            if around_equals {
                continue;
            }
            if !out.is_empty() && chars.peek().is_some() {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn push_first(items: &mut Vec<(String, Option<String>)>, key: &str, value: Option<String>) {
    if items.iter().any(|(name, _)| name == key) {
        return;
    }
    items.push((key.to_owned(), value));
}

fn leak_key(key: &str) -> &'static str {
    match key {
        "lat_0" => "lat_0",
        "lat_1" => "lat_1",
        "lat_2" => "lat_2",
        "lon_0" => "lon_0",
        "k" | "k_0" => "k_0",
        "x_0" => "x_0",
        "y_0" => "y_0",
        "a" | "R" => "a",
        "b" => "b",
        "rf" => "rf",
        "f" => "f",
        "es" => "es",
        "e" => "e",
        "ellps" => "ellps",
        "zone" => "zone",
        "pm" => "pm",
        "units" => "units",
        "to_meter" => "to_meter",
        _ => "parameter",
    }
}

/// Ellipsoid plus the offsets applied around every classic kernel.
#[derive(Clone, Debug)]
pub(crate) struct Frame {
    /// Ellipsoid used by the kernel and by angular tolerances.
    pub earth: Ellipsoid,
    /// Central meridian, radians.
    pub lam0: f64,
    /// Latitude of origin, radians.
    pub phi0: f64,
    /// False easting, metres, added before the unit conversion.
    pub x0: f64,
    /// False northing, metres.
    pub y0: f64,
    /// False height.
    pub z0: f64,
    /// Scale factor at the origin.
    pub k0: f64,
    /// Metres per horizontal unit.
    pub to_meter: f64,
    /// Inverse of [`Self::to_meter`].
    pub fr_meter: f64,
    /// Metres per vertical unit.
    pub vto_meter: f64,
    /// Inverse of [`Self::vto_meter`].
    pub vfr_meter: f64,
    /// Keep longitudes outside ]−π, π].
    pub over: bool,
    /// Input latitude is geocentric.
    pub geoc: bool,
    /// Prime meridian, radians east of Greenwich.
    pub from_greenwich: f64,
    /// Longitude wrap centre, radians. `None` keeps the principal range.
    pub lon_wrap: Option<f64>,
}

impl Frame {
    fn from_params(params: &Params) -> Result<Self, CreateError> {
        let earth = earth_from_params(params)?;
        let lam0 = params.radians("lon_0")?.unwrap_or(0.0);
        let phi0 = params.radians("lat_0")?.unwrap_or(0.0);
        if phi0.abs() > FRAC_PI_2 {
            return Err(CreateError::IllegalValue("lat_0"));
        }
        let k0 = if params.flag("k_0") {
            params.number("k_0")?.unwrap_or(1.0)
        } else if params.flag("k") {
            params.number("k")?.unwrap_or(1.0)
        } else {
            1.0
        };
        if k0 <= 0.0 {
            return Err(CreateError::IllegalValue("k_0"));
        }
        let to_meter = linear_unit(params, "units", "to_meter")?;
        let (vto_meter, vfr_meter) = if params.flag("vunits") || params.flag("vto_meter") {
            let v = linear_unit(params, "vunits", "vto_meter")?;
            (v, 1.0 / v)
        } else {
            (to_meter, 1.0 / to_meter)
        };
        let from_greenwich = prime_meridian(params)?;
        let earth = spherify(params, earth, phi0)?;
        let lon_wrap = if params.flag("lon_wrap") {
            let center = params.radians("lon_wrap")?.unwrap_or(0.0);
            // init.cpp: `!(fabs(center) < 10 * 2 * pi)`, which is also false for NaN.
            if !(center.abs() < 20.0 * PI) {
                return Err(CreateError::IllegalValue("lon_wrap"));
            }
            Some(center)
        } else {
            None
        };
        Ok(Self {
            earth,
            lam0,
            phi0,
            x0: params.number("x_0")?.unwrap_or(0.0),
            y0: params.number("y_0")?.unwrap_or(0.0),
            z0: params.number("z_0")?.unwrap_or(0.0),
            k0,
            to_meter,
            fr_meter: 1.0 / to_meter,
            vto_meter,
            vfr_meter,
            over: params.flag("over"),
            geoc: params.flag("geoc"),
            from_greenwich,
            lon_wrap,
        })
    }
}

fn earth_from_params(params: &Params) -> Result<Ellipsoid, CreateError> {
    if let Some(name) = params.text("datum") {
        return datum_ellipsoid(name);
    }
    let mut earth = if let Some(name) = params.text("ellps") {
        named_ellipsoid(name)?
    } else if !has_size(params) {
        Ellipsoid::grs80()
    } else {
        Ellipsoid::try_from_a_es(6_378_137.0, 0.0).expect("unit sphere axes are valid")
    };
    if params.flag("R") {
        let radius = params.number("R")?.unwrap_or(0.0);
        earth = Ellipsoid::try_from_a_es(radius, 0.0).map_err(ell_err)?;
    } else if params.flag("a") {
        let a = params.number("a")?.unwrap_or(0.0);
        let es = shape_es(params, earth.es(), a)?;
        earth = Ellipsoid::try_from_a_es(a, es).map_err(ell_err)?;
    } else if has_shape(params) {
        let es = shape_es(params, earth.es(), earth.a())?;
        earth = Ellipsoid::try_from_a_es(earth.a(), es).map_err(ell_err)?;
    }
    if earth.a() <= 0.0 {
        return Err(CreateError::IllegalValue("a"));
    }
    Ok(earth)
}

fn shape_in_range(value: f64) -> bool {
    #[expect(
        clippy::manual_range_contains,
        reason = "zero is a valid sphere flattening and must be included"
    )]
    {
        value >= 0.0 && value < 1.0
    }
}

fn has_size(params: &Params) -> bool {
    params.flag("R") || params.flag("a") || params.flag("ellps") || params.flag("b")
}

fn has_shape(params: &Params) -> bool {
    params.flag("rf")
        || params.flag("f")
        || params.flag("es")
        || params.flag("e")
        || params.flag("b")
}

fn shape_es(params: &Params, previous_es: f64, previous_a: f64) -> Result<f64, CreateError> {
    if let Some(rf) = params.number("rf")? {
        if rf <= 0.0 {
            return Err(CreateError::IllegalValue("rf"));
        }
        let f = 1.0 / rf;
        return Ok(f * (2.0 - f));
    }
    if let Some(f) = params.number("f")? {
        if !shape_in_range(f) {
            return Err(CreateError::IllegalValue("f"));
        }
        return Ok(f * (2.0 - f));
    }
    if let Some(es) = params.number("es")? {
        if !shape_in_range(es) {
            return Err(CreateError::IllegalValue("es"));
        }
        return Ok(es);
    }
    if let Some(e) = params.number("e")? {
        if !shape_in_range(e) {
            return Err(CreateError::IllegalValue("e"));
        }
        return Ok(e * e);
    }
    let previous = previous_a;
    if let Some(b) = params.number("b")? {
        let a = params.number("a")?.unwrap_or(previous);
        if b <= 0.0 || a <= 0.0 {
            return Err(CreateError::IllegalValue("b"));
        }
        if b == a {
            return Ok(0.0);
        }
        let f = (a - b) / a;
        return Ok(f * (2.0 - f));
    }
    Ok(previous_es)
}

fn ell_err(error: EllipsoidError) -> CreateError {
    match error {
        EllipsoidError::NonFiniteParameter
        | EllipsoidError::NonPositiveSemiMajorAxis
        | EllipsoidError::InvalidFlattening => CreateError::IllegalValue("ellps"),
    }
}

fn spherify(params: &Params, earth: Ellipsoid, phi0: f64) -> Result<Ellipsoid, CreateError> {
    // `+r_a=<radius>` is the sphere radius the healpix tests pass explicitly.
    if let Some(text) = params.text("r_a") {
        let radius = proj_number(text).ok_or(CreateError::IllegalValue("r_a"))?;
        return Ellipsoid::try_from_a_es(radius, 0.0).map_err(ell_err);
    }
    let es = earth.es();
    let a = earth.a();
    let b = earth.b();
    let radius = if params.flag("R_A") {
        a * (1.0 - es * (1.0 / 6.0 + es * (17.0 / 360.0 + es * 67.0 / 3024.0)))
    } else if params.flag("R_V") {
        a * (1.0 - es * (1.0 / 6.0 + es * (5.0 / 72.0 + es * 55.0 / 1296.0)))
    } else if params.flag("R_a") {
        (a + b) / 2.0
    } else if params.flag("R_g") {
        (a * b).sqrt()
    } else if params.flag("R_h") {
        2.0 * a * b / (a + b)
    } else if params.flag("R_C") {
        // PROJ spherifies before it reads lat_0, so the conformal radius is
        // taken at latitude 0 and equals the semi-minor axis.
        let _ = phi0;
        a * (1.0 - es).sqrt()
    } else if params.flag("R_lat_a") || params.flag("R_lat_g") {
        // `ellps_spherification` walks R_A, R_V, R_a, R_g, R_h, R_lat_a,
        // R_lat_g, R_C and keeps the first hit. R_lat_a wins over R_lat_g.
        let (key, kind) = if params.flag("R_lat_a") {
            ("R_lat_a", LatitudeMean::Arithmetic)
        } else {
            ("R_lat_g", LatitudeMean::Geometric)
        };
        latitude_radius(params, a, es, key, kind)?
    } else {
        return Ok(earth);
    };
    Ellipsoid::try_from_a_es(radius, 0.0).map_err(ell_err)
}

/// Which mean of the meridional and prime-vertical radii `R_lat_*` asks for.
enum LatitudeMean {
    Arithmetic,
    Geometric,
}

/// Sphere radius at one latitude. PROJ `ellps_spherification` cases 5 and 6.
fn latitude_radius(
    params: &Params,
    a: f64,
    es: f64,
    key: &'static str,
    kind: LatitudeMean,
) -> Result<f64, CreateError> {
    let phi = match params.radians(key) {
        Ok(Some(phi)) => phi,
        // A bare flag has no `=`. PROJ then runs dmstor on the key text, which
        // returns 0.
        Ok(None) => 0.0,
        Err(_) => return Err(CreateError::IllegalValue(key)),
    };
    // Same test as `fabs(t) > M_HALFPI`. NaN fails it in C++ and falls through;
    // a non-finite radius is rejected below.
    if phi.abs() > FRAC_PI_2 {
        return Err(CreateError::IllegalValue(key));
    }
    let sinphi = phi.sin();
    let t = 1.0 - es * sinphi * sinphi;
    if t == 0.0 {
        return Err(CreateError::IllegalValue(key));
    }
    let radius = match kind {
        // (M + N) / 2, with M the meridional radius and N the prime vertical.
        LatitudeMean::Arithmetic => a * (1.0 - es + t) / (2.0 * t * t.sqrt()),
        // sqrt(M * N).
        LatitudeMean::Geometric => a * (1.0 - es).sqrt() / t,
    };
    if !radius.is_finite() || radius <= 0.0 {
        return Err(CreateError::IllegalValue(key));
    }
    Ok(radius)
}

enum DatumShift {
    /// Ellipsoid only. WGS84 and NAD83 stay this way: a zero `towgs84` on
    /// GRS80 would otherwise run a cartesian conversion.
    None,
    Towgs84(&'static str),
    Nadgrids(&'static str),
}

fn datum_record(name: &str) -> Option<(&'static str, DatumShift)> {
    Some(match name {
        "WGS84" => ("WGS84", DatumShift::None),
        "NAD83" | "GRS80" => ("GRS80", DatumShift::None),
        "GGRS87" => ("GRS80", DatumShift::Towgs84("-199.87,74.79,246.62")),
        "NAD27" => (
            "clrk66",
            DatumShift::Nadgrids("@conus,@alaska,@ntv2_0.gsb,@ntv1_can.dat"),
        ),
        "potsdam" => ("bessel", DatumShift::Nadgrids("@BETA2007.gsb")),
        "carthage" => ("clrk80ign", DatumShift::Towgs84("-263.0,6.0,431.0")),
        "hermannskogel" => (
            "bessel",
            DatumShift::Towgs84("577.326,90.129,463.919,5.137,1.474,5.297,2.4232"),
        ),
        "ire65" => (
            "mod_airy",
            DatumShift::Towgs84("482.530,-130.596,564.557,-1.042,-0.214,-0.631,8.15"),
        ),
        "nzgd49" => (
            "intl",
            DatumShift::Towgs84("59.47,-5.04,187.44,0.47,-0.1,1.024,-4.5993"),
        ),
        "OSGB36" => (
            "airy",
            DatumShift::Towgs84("446.448,-125.157,542.060,0.1502,0.2470,0.8421,-20.4894"),
        ),
        _ => return None,
    })
}

fn datum_ellipsoid(name: &str) -> Result<Ellipsoid, CreateError> {
    let Some((ellps, _)) = datum_record(name) else {
        return Err(CreateError::IllegalValue("datum"));
    };
    named_ellipsoid(ellps)
}

fn prepare_params(params: &mut Params) -> Result<(), CreateError> {
    expand_init(params)?;
    expand_datum(params)?;
    Ok(())
}

fn expand_init(params: &mut Params) -> Result<(), CreateError> {
    let Some(spec) = params.text("init").map(str::to_owned) else {
        return Ok(());
    };
    let definition = crate::crs::init_definition(&spec)?;
    let extra = Params::parse(&definition)?;
    for (key, value) in extra.items {
        if key == "no_defs" {
            continue;
        }
        params.insert_missing(&key, value);
    }
    params.remove("init");
    Ok(())
}

fn expand_datum(params: &mut Params) -> Result<(), CreateError> {
    let Some(name) = params.text("datum").map(str::to_owned) else {
        return Ok(());
    };
    let Some((ellps, shift)) = datum_record(&name) else {
        return Err(CreateError::IllegalValue("datum"));
    };
    if !params.flag("ellps") && !params.flag("a") && !params.flag("R") {
        params.insert_missing("ellps", Some(ellps.to_owned()));
    }
    match shift {
        DatumShift::None => {}
        DatumShift::Towgs84(text) if !params.flag("towgs84") && !params.flag("nadgrids") => {
            params.insert_missing("towgs84", Some(text.to_owned()));
        }
        DatumShift::Nadgrids(text) if !params.flag("nadgrids") && !params.flag("towgs84") => {
            params.insert_missing("nadgrids", Some(text.to_owned()));
        }
        DatumShift::Towgs84(_) | DatumShift::Nadgrids(_) => {}
    }
    Ok(())
}

fn named_ellipsoid(name: &str) -> Result<Ellipsoid, CreateError> {
    const TABLE: &[(&str, f64, f64, bool)] = &[
        ("MERIT", 6_378_137.0, 298.257, true),
        ("SGS85", 6_378_136.0, 298.257, true),
        ("GRS80", 6_378_137.0, 298.257_222_101, true),
        ("IAU76", 6_378_140.0, 298.257, true),
        ("airy", 6_377_563.396, 299.324_964_6, true),
        ("APL4.9", 6_378_137.0, 298.25, true),
        ("NWL9D", 6_378_145.0, 298.25, true),
        ("mod_airy", 6_377_340.189, 6_356_034.446, false),
        ("andrae", 6_377_104.43, 300.0, true),
        ("danish", 6_377_019.256_3, 300.0, true),
        ("aust_SA", 6_378_160.0, 298.25, true),
        ("GRS67", 6_378_160.0, 298.247_167_427, true),
        ("GSK2011", 6_378_136.5, 298.256_415_1, true),
        ("bessel", 6_377_397.155, 299.152_812_8, true),
        ("bess_nam", 6_377_483.865, 299.152_812_8, true),
        ("clrk66", 6_378_206.4, 6_356_583.8, false),
        ("clrk80", 6_378_249.145, 293.4663, true),
        ("clrk80ign", 6_378_249.2, 293.466_021_293_626_9, true),
        ("CPM", 6_375_738.7, 334.29, true),
        ("delmbr", 6_376_428.0, 311.5, true),
        ("engelis", 6_378_136.05, 298.2566, true),
        ("evrst30", 6_377_276.345, 300.8017, true),
        ("evrst48", 6_377_304.063, 300.8017, true),
        ("evrst56", 6_377_301.243, 300.8017, true),
        ("evrst69", 6_377_295.664, 300.8017, true),
        ("evrstSS", 6_377_298.556, 300.8017, true),
        ("fschr60", 6_378_166.0, 298.3, true),
        ("fschr60m", 6_378_155.0, 298.3, true),
        ("fschr68", 6_378_150.0, 298.3, true),
        ("helmert", 6_378_200.0, 298.3, true),
        ("hough", 6_378_270.0, 297.0, true),
        ("intl", 6_378_388.0, 297.0, true),
        ("krass", 6_378_245.0, 298.3, true),
        ("kaula", 6_378_163.0, 298.24, true),
        ("lerch", 6_378_139.0, 298.257, true),
        ("mprts", 6_397_300.0, 191.0, true),
        ("new_intl", 6_378_157.5, 6_356_772.2, false),
        ("plessis", 6_376_523.0, 6_355_863.0, false),
        ("PZ90", 6_378_136.0, 298.257_84, true),
        ("SEasia", 6_378_155.0, 6_356_773.320_5, false),
        ("walbeck", 6_376_896.0, 6_355_834.846_7, false),
        ("WGS60", 6_378_165.0, 298.3, true),
        ("WGS66", 6_378_145.0, 298.25, true),
        ("WGS72", 6_378_135.0, 298.26, true),
        ("WGS84", 6_378_137.0, 298.257_223_563, true),
        ("sphere", 6_370_997.0, 6_370_997.0, false),
    ];
    let Some((_, first, second, rf)) = TABLE.iter().find(|(id, ..)| *id == name) else {
        return Err(CreateError::IllegalValue("ellps"));
    };
    let built = if *rf {
        let f = 1.0 / second;
        Ellipsoid::try_from_a_es(*first, f * (2.0 - f))
    } else if *first == *second {
        Ellipsoid::try_from_a_es(*first, 0.0)
    } else {
        let f = (first - second) / first;
        Ellipsoid::try_from_a_es(*first, f * (2.0 - f))
    };
    built.map_err(ell_err)
}

fn linear_unit(params: &Params, units_key: &str, meter_key: &str) -> Result<f64, CreateError> {
    const UNITS: &[(&str, f64)] = &[
        ("km", 1000.0),
        ("m", 1.0),
        ("dm", 0.1),
        ("cm", 0.01),
        ("mm", 0.001),
        ("kmi", 1852.0),
        ("in", 0.0254),
        ("ft", 0.3048),
        ("yd", 0.9144),
        ("mi", 1609.344),
        ("fath", 1.8288),
        ("ch", 20.1168),
        ("link", 0.201_168),
        ("us-in", 100.0 / 3937.0),
        ("us-ft", 1200.0 / 3937.0),
        ("us-yd", 3600.0 / 3937.0),
        ("us-ch", 79200.0 / 3937.0),
        ("us-mi", 6_336_000.0 / 3937.0),
        ("ind-yd", 0.914_395_23),
        ("ind-ft", 0.304_798_41),
        ("ind-ch", 20.116_695_06),
    ];
    let text = if let Some(name) = params.text(units_key) {
        UNITS
            .iter()
            .find(|(id, _)| *id == name)
            .map(|(_, factor)| *factor)
            .ok_or(CreateError::IllegalValue("units"))?
    } else if let Some(raw) = params.text(meter_key) {
        parse_ratio(raw).ok_or(CreateError::IllegalValue("to_meter"))?
    } else {
        1.0
    };
    if text <= 0.0 {
        return Err(CreateError::IllegalValue("to_meter"));
    }
    Ok(text)
}

fn parse_ratio(text: &str) -> Option<f64> {
    let (num, den) = text.split_once('/').unwrap_or((text, ""));
    let value = proj_number(num)?;
    if den.is_empty() {
        Some(value)
    } else {
        let denom = proj_number(den)?;
        if denom == 0.0 {
            None
        } else {
            Some(value / denom)
        }
    }
}

fn prime_meridian(params: &Params) -> Result<f64, CreateError> {
    let Some(name) = params.text("pm") else {
        return Ok(0.0);
    };
    const MERIDIANS: &[(&str, &str)] = &[
        ("greenwich", "0dE"),
        ("lisbon", "9d07'54.862\"W"),
        ("paris", "2d20'14.025\"E"),
        ("bogota", "74d04'51.3\"W"),
        ("madrid", "3d41'16.58\"W"),
        ("rome", "12d27'8.4\"E"),
        ("bern", "7d26'22.5\"E"),
        ("jakarta", "106d48'27.79\"E"),
        ("ferro", "17d40'W"),
        ("brussels", "4d22'4.71\"E"),
        ("stockholm", "18d3'29.8\"E"),
        ("athens", "23d42'58.815\"E"),
        ("oslo", "10d43'22.5\"E"),
        ("copenhagen", "12d34'40.35\"E"),
    ];
    let text = MERIDIANS
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, def)| *def)
        .unwrap_or(name);
    parse_dms_degrees(text)
        .map(|deg| deg.to_radians())
        .ok_or(CreateError::IllegalValue("pm"))
}

/// What a step reads or writes. Pipelines refuse to join different kinds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IoKind {
    /// Radians. `gie` converts degrees on this side.
    Angular,
    /// Degrees. `gie` leaves the numbers alone. Not the same as [`Self::Angular`].
    Degrees,
    Projected,
    Cartesian,
    /// Same unit as the neighbouring step. Axis swap uses this.
    Any,
}

/// Four independent coordinate stacks owned by one pipeline.
///
/// PROJ keeps these on the pipeline object (`pipeline.cpp`, `stack[4]`). A
/// push or pop with no pipeline parent does not see this and leaves the
/// coordinate unchanged.
#[derive(Debug)]
pub(crate) struct PipelineStack {
    axes: [Vec<f64>; 4],
}

impl PipelineStack {
    fn new() -> Self {
        Self {
            axes: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        }
    }

    /// Save one axis. `axis` is 0..4.
    pub(crate) fn push_axis(&mut self, axis: usize, value: f64) {
        self.axes[axis].push(value);
    }

    /// Pop one axis. An empty axis returns [`None`] and the coordinate stays.
    pub(crate) fn pop_axis(&mut self, axis: usize) -> Option<f64> {
        self.axes[axis].pop()
    }
}

/// One forward or inverse step.
pub(crate) trait Step: Send + Sync {
    /// Forward transform. Angular axes are radians.
    ///
    /// # Errors
    ///
    /// [`TransformError`] when the point is outside the domain.
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError>;

    /// Inverse transform.
    ///
    /// # Errors
    ///
    /// [`TransformError`] when the point is outside the domain or the inverse
    /// does not exist.
    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError>;

    /// Forward input is longitude / latitude.
    fn angular_input_forward(&self) -> bool {
        self.input_kind() == IoKind::Angular
    }

    /// Forward output is longitude / latitude.
    fn angular_output_forward(&self) -> bool {
        self.output_kind() == IoKind::Angular
    }

    /// Forward input is degrees. `gie` does not convert this side.
    fn degree_input_forward(&self) -> bool {
        self.input_kind() == IoKind::Degrees
    }

    /// Forward output is degrees. `gie` converts this side before a geodesic distance.
    fn degree_output_forward(&self) -> bool {
        self.output_kind() == IoKind::Degrees
    }

    /// Unit of the forward input.
    fn input_kind(&self) -> IoKind;

    /// Unit of the forward output.
    fn output_kind(&self) -> IoKind;

    /// Semi-major axis and `e²` used when an angular error is turned into metres.
    fn distance_ellipsoid(&self) -> (f64, f64);

    /// `true` when this step reads the pipeline coordinate stack.
    fn needs_pipeline_stack(&self) -> bool {
        false
    }

    /// Forward while a pipeline holds the coordinate stack.
    ///
    /// # Errors
    ///
    /// [`TransformError`] from [`Self::forward`] when this step ignores the stack.
    fn forward_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut PipelineStack,
    ) -> Result<Coord, TransformError> {
        let _ = stack;
        self.forward(coord)
    }

    /// Inverse while a pipeline holds the coordinate stack.
    ///
    /// # Errors
    ///
    /// [`TransformError`] from [`Self::inverse`] when this step ignores the stack.
    fn inverse_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut PipelineStack,
    ) -> Result<Coord, TransformError> {
        let _ = stack;
        self.inverse(coord)
    }
}

/// A built operation.
#[derive(Clone)]
pub struct Operation {
    step: std::sync::Arc<dyn Step>,
}

impl fmt::Debug for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Operation")
    }
}

impl Operation {
    /// Build an operation from a PROJ string.
    ///
    /// # Errors
    ///
    /// [`CreateError`] when `+proj` or a parameter is unknown or illegal.
    /// Nothing is returned that would transform a coordinate anyway.
    ///
    /// `urn:ogc:def:coordinateOperation:NKG::` is read from `proj.db`. A step
    /// that needs a grid is [`CreateError::Unsupported`], not a coordinate.
    pub fn create(definition: &str) -> Result<Self, CreateError> {
        if let Some(step) = nkg::from_definition(definition)? {
            return Ok(Self {
                step: std::sync::Arc::from(step),
            });
        }
        let mut params = Params::parse(definition)?;
        // `+init=epsg:CODE` expands to a proj string before the name is required.
        expand_init(&mut params)?;
        let name = params.text("proj").unwrap_or("").to_owned();
        if name.is_empty() {
            return Err(if definition.contains("proj=") {
                CreateError::MissingArgument("proj")
            } else {
                CreateError::IllegalValue("proj")
            });
        }
        if name == "pipeline" {
            return Ok(Self {
                step: std::sync::Arc::from(build_pipeline(definition)?),
            });
        }
        prepare_params(&mut params)?;
        let name = params.text("proj").unwrap_or("").to_owned();
        if name.is_empty() {
            return Err(CreateError::MissingArgument("proj"));
        }
        let frame = Frame::from_params(&params)?;
        let step = finish_step(&name, &params, &frame)?;
        Ok(Self { step })
    }

    /// Forward transform.
    ///
    /// # Errors
    ///
    /// [`TransformError`] from the underlying step.
    pub fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.step.forward(coord)
    }

    /// Inverse transform.
    ///
    /// # Errors
    ///
    /// [`TransformError`] from the underlying step.
    pub fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.step.inverse(coord)
    }

    /// Whether this direction reads longitude and latitude.
    #[must_use]
    pub fn angular_input(&self, inverse: bool) -> bool {
        if inverse {
            self.step.angular_output_forward()
        } else {
            self.step.angular_input_forward()
        }
    }

    /// Whether this direction writes longitude and latitude.
    #[must_use]
    pub fn angular_output(&self, inverse: bool) -> bool {
        if inverse {
            self.step.angular_input_forward()
        } else {
            self.step.angular_output_forward()
        }
    }

    /// Whether this direction writes degrees rather than radians.
    #[must_use]
    pub fn degree_output(&self, inverse: bool) -> bool {
        if inverse {
            self.step.degree_input_forward()
        } else {
            self.step.degree_output_forward()
        }
    }

    /// Ellipsoid used to turn an angular miss into metres.
    #[must_use]
    pub fn distance_ellipsoid(&self) -> (f64, f64) {
        self.step.distance_ellipsoid()
    }
}

/// Transform between two EPSG CRS, in the axis order stored in `proj.db`.
///
/// Built by [`CrsPipeline::create`]. Inputs are degrees or metres. A degree
/// output is still degrees, so a geodesic check converts it first.
#[derive(Clone, Debug)]
pub struct CrsPipeline {
    /// Pipeline. Every step is an operation this crate already builds.
    pub operation: Operation,
    /// Destination first axis is latitude or northing.
    pub lat_or_northing_first: bool,
}

impl CrsPipeline {
    /// Look the pair up in `proj.db` and emit a pipeline.
    ///
    /// # Errors
    ///
    /// [`CreateError::Unsupported`] when a step is a grid, a deformation
    /// model, or a method this crate does not build. No coordinate is returned
    /// for that pair.
    pub fn create(source: &str, target: &str) -> Result<Self, CreateError> {
        let (definition, lat_or_northing_first) = crate::crs::epsg_pipeline(source, target)?;
        let operation = Operation::create(&definition)?;
        Ok(Self {
            operation,
            lat_or_northing_first,
        })
    }
}

type BuildFn = fn(&Params, &Frame) -> Result<Box<dyn Step>, CreateError>;

struct Built {
    name: &'static str,
    keys: &'static [&'static str],
    build: BuildFn,
}

fn catalogue() -> &'static [Built] {
    CATALOGUE
}

type SharedStep = std::sync::Arc<dyn Step>;

fn finish_step(name: &str, params: &Params, frame: &Frame) -> Result<SharedStep, CreateError> {
    let inner = build_step(name, params, frame)?;
    check_keys(name, params)?;
    if legacy::applies(name, params) {
        legacy::wrap(inner, params, frame)
    } else {
        Ok(inner)
    }
}

fn build_step(name: &str, params: &Params, frame: &Frame) -> Result<SharedStep, CreateError> {
    if let Some(spec) = catalogue().iter().find(|spec| spec.name == name) {
        return Ok(std::sync::Arc::from((spec.build)(params, frame)?));
    }
    if let Some(step) = datum_ops::build(name, params, frame)? {
        return Ok(std::sync::Arc::from(step));
    }
    if let Some(step) = crate::projections::build(name, params, frame)? {
        return Ok(std::sync::Arc::from(step));
    }
    Err(CreateError::UnknownProj(name.to_owned()))
}

fn check_keys(name: &str, params: &Params) -> Result<(), CreateError> {
    let extra = catalogue()
        .iter()
        .find(|spec| spec.name == name)
        .map(|spec| spec.keys)
        .or_else(|| datum_ops::extra_keys(name))
        .or_else(|| crate::projections::extra_keys(name));
    let Some(extra) = extra else {
        return Ok(());
    };
    for key in params.keys() {
        if GLOBAL_KEYS.contains(&key) || extra.contains(&key) {
            continue;
        }
        return Err(CreateError::UnknownParameter(key.to_owned()));
    }
    Ok(())
}

const GLOBAL_KEYS: &[&str] = &[
    "proj",
    "ellps",
    "datum",
    "a",
    "b",
    "rf",
    "f",
    "es",
    "e",
    "R",
    "r_a",
    "R_A",
    "R_V",
    "R_a",
    "R_g",
    "R_h",
    "R_lat_a",
    "R_lat_g",
    "R_C",
    "lon_0",
    "lat_0",
    "x_0",
    "y_0",
    "z_0",
    "t_0",
    "k",
    "k_0",
    "units",
    "to_meter",
    "vunits",
    "vto_meter",
    "no_defs",
    "wktext",
    "type",
    "pm",
    "axis",
    "over",
    "geoc",
    "lon_wrap",
    "inv",
    "omit_fwd",
    "omit_inv",
    "step",
    "towgs84",
    "nadgrids",
    "geoidgrids",
];

const CATALOGUE: &[Built] = &[
    Built {
        name: "tmerc",
        keys: &["approx", "algo"],
        build: build_tmerc,
    },
    Built {
        name: "etmerc",
        keys: &[],
        build: build_etmerc,
    },
    Built {
        name: "utm",
        keys: &["zone", "south", "approx", "algo"],
        build: build_utm,
    },
    Built {
        name: "lcc",
        keys: &["lat_1", "lat_2"],
        build: build_lcc,
    },
    Built {
        name: "merc",
        keys: &["lat_ts"],
        build: build_merc,
    },
    Built {
        name: "helmert",
        keys: datum_ops::HELMERT_KEYS,
        build: build_helmert,
    },
    Built {
        name: "cart",
        keys: &[],
        build: build_cart,
    },
    Built {
        name: "geocent",
        keys: &[],
        build: build_cart,
    },
    Built {
        name: "longlat",
        keys: &[],
        build: build_longlat,
    },
    Built {
        name: "latlong",
        keys: &[],
        build: build_longlat,
    },
    Built {
        name: "latlon",
        keys: &[],
        build: build_longlat,
    },
    Built {
        name: "lonlat",
        keys: &[],
        build: build_longlat,
    },
];

#[derive(Clone, Copy)]
struct SphereTmerc {
    a: f64,
    k0: f64,
    lam0: f64,
    phi0: f64,
    x0: f64,
    y0: f64,
}

fn sphere_tmerc_forward(
    sphere: &SphereTmerc,
    lon: f64,
    lat: f64,
) -> Result<Projected, ProjectionError> {
    let SphereTmerc {
        a,
        k0,
        lam0,
        phi0,
        x0,
        y0,
    } = *sphere;
    const EPS10: f64 = 1e-10;
    let lam = adjlon(adjlon(lon) - lam0);
    let phi = lat.clamp(-FRAC_PI_2, FRAC_PI_2);
    let cosphi = phi.cos();
    let b = cosphi * lam.sin();
    if (b.abs() - 1.0).abs() <= EPS10 {
        return Err(ProjectionError::OutsideDomain);
    }
    let x = 0.5 * k0 * ((1.0 + b) / (1.0 - b)).ln();
    let mut angle = if cosphi == 1.0 {
        if lam < -FRAC_PI_2 || lam > FRAC_PI_2 {
            PI
        } else {
            0.0
        }
    } else {
        let y = cosphi * lam.cos() / (1.0 - b * b).sqrt();
        if y.abs() >= 1.0 {
            if y.abs() - 1.0 > EPS10 {
                return Err(ProjectionError::OutsideDomain);
            }
            0.0
        } else {
            y.acos()
        }
    };
    if phi < 0.0 {
        angle = -angle;
    }
    angle = k0 * (angle - phi0);
    Ok(Projected {
        east: a * x + x0,
        north: a * angle + y0,
    })
}

fn sphere_tmerc_inverse(
    sphere: &SphereTmerc,
    xy: Projected,
) -> Result<Geographic, ProjectionError> {
    let SphereTmerc {
        a,
        k0,
        lam0,
        phi0,
        x0,
        y0,
    } = *sphere;
    let x = (xy.east - x0) / a;
    let y = (xy.north - y0) / a;
    let h = (x / k0).exp();
    if h == 0.0 {
        return Err(ProjectionError::OutsideDomain);
    }
    let g = 0.5 * (h - 1.0 / h);
    let d = phi0 + y / k0;
    let h = d.cos();
    let mut phi = ((1.0 - h * h) / (1.0 + g * g)).sqrt().asin();
    phi = phi.copysign(d);
    let lam = if g != 0.0 || h != 0.0 {
        g.atan2(h)
    } else {
        0.0
    };
    Ok(Geographic {
        lon_deg: adjlon(lam + lam0).to_degrees(),
        lat_deg: phi.to_degrees(),
    })
}

enum TmercAlgo {
    Exact,
    Evenden,
    Auto,
}

fn tmerc_algo(params: &Params) -> Result<TmercAlgo, CreateError> {
    if params.flag("approx") {
        return Ok(TmercAlgo::Evenden);
    }
    match params.text("algo") {
        None => Ok(TmercAlgo::Exact),
        Some("evenden_snyder") => Ok(TmercAlgo::Evenden),
        Some("poder_engsager") => Ok(TmercAlgo::Exact),
        Some("auto") => Ok(TmercAlgo::Auto),
        Some(_) => Err(CreateError::IllegalValue("algo")),
    }
}

fn auto_is_exact_only(es: f64, phi0: f64, k0: f64) -> bool {
    es > 0.1 || phi0 != 0.0 || (k0 - 1.0).abs() > 0.01
}

fn native_exact(frame: &Frame, tmerc: Tmerc) -> Box<dyn Step> {
    let inverse = tmerc.clone();
    Box::new(NativeProjected::new(
        frame,
        move |lon, lat| tmerc.forward_rad(lon, lat),
        move |xy| inverse.inverse(xy),
    ))
}

fn native_approx(frame: &Frame, approx: ApproxTmerc) -> Box<dyn Step> {
    Box::new(NativeProjected::new(
        frame,
        move |lon, lat| approx.forward(lon, lat),
        move |xy| approx.inverse(xy),
    ))
}

fn native_auto(
    frame: &Frame,
    exact: Tmerc,
    approx: ApproxTmerc,
    lam0: f64,
    x0: f64,
    y0: f64,
    ra: f64,
) -> Box<dyn Step> {
    let inverse_exact = exact.clone();
    Box::new(NativeProjected::new(
        frame,
        move |lon, lat| {
            let lam = adjlon(adjlon(lon) - lam0);
            if lam.abs() > 3.0_f64.to_radians() {
                exact.forward_rad(lon, lat)
            } else {
                approx.forward(lon, lat)
            }
        },
        move |xy| {
            let x = (xy.east - x0) * ra;
            let y = (xy.north - y0) * ra;
            if x.abs() > 0.053 - 0.022 * y * y {
                inverse_exact.inverse(xy)
            } else {
                approx.inverse(xy)
            }
        },
    ))
}

fn sphere_tmerc_step(frame: &Frame) -> Box<dyn Step> {
    let sphere = SphereTmerc {
        a: frame.earth.a(),
        k0: frame.k0,
        lam0: frame.lam0,
        phi0: frame.phi0,
        x0: frame.x0,
        y0: frame.y0,
    };
    let inverse = sphere;
    Box::new(NativeProjected::new(
        frame,
        move |lon, lat| sphere_tmerc_forward(&sphere, lon, lat),
        move |xy| sphere_tmerc_inverse(&inverse, xy),
    ))
}

fn exact_from_frame(frame: &Frame) -> Result<Tmerc, CreateError> {
    Tmerc::try_new(
        frame.earth,
        frame.lam0.to_degrees(),
        frame.phi0.to_degrees(),
        frame.k0,
        frame.x0,
        frame.y0,
    )
    .map_err(|_| CreateError::IllegalValue("tmerc"))
}

fn approx_from_degrees(
    frame: &Frame,
    lon0_deg: f64,
    lat0_deg: f64,
    k0: f64,
    x0: f64,
    y0: f64,
) -> Result<ApproxTmerc, CreateError> {
    ApproxTmerc::try_new(frame.earth, lon0_deg, lat0_deg, k0, x0, y0)
        .map_err(|_| CreateError::IllegalValue("tmerc"))
}

fn build_tmerc(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let algo = tmerc_algo(params)?;
    if frame.earth.es() == 0.0 {
        return Ok(sphere_tmerc_step(frame));
    }
    match algo {
        TmercAlgo::Evenden => Ok(native_approx(
            frame,
            approx_from_degrees(
                frame,
                frame.lam0.to_degrees(),
                frame.phi0.to_degrees(),
                frame.k0,
                frame.x0,
                frame.y0,
            )?,
        )),
        TmercAlgo::Exact => Ok(native_exact(frame, exact_from_frame(frame)?)),
        TmercAlgo::Auto if auto_is_exact_only(frame.earth.es(), frame.phi0, frame.k0) => {
            Ok(native_exact(frame, exact_from_frame(frame)?))
        }
        TmercAlgo::Auto => {
            let exact = exact_from_frame(frame)?;
            let approx = approx_from_degrees(
                frame,
                frame.lam0.to_degrees(),
                frame.phi0.to_degrees(),
                frame.k0,
                frame.x0,
                frame.y0,
            )?;
            Ok(native_auto(
                frame,
                exact,
                approx,
                frame.lam0,
                frame.x0,
                frame.y0,
                frame.earth.ra(),
            ))
        }
    }
}

fn build_etmerc(_params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    if frame.earth.es() == 0.0 {
        return Err(CreateError::IllegalValue("ellps"));
    }
    Ok(native_exact(frame, exact_from_frame(frame)?))
}

fn build_utm(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    if frame.earth.es() == 0.0 {
        return Err(CreateError::IllegalValue("ellps"));
    }
    let zone = params
        .integer("zone")?
        .ok_or(CreateError::MissingArgument("zone"))?;
    let zone = u32::try_from(zone).map_err(|_| CreateError::IllegalValue("zone"))?;
    let zone = UtmZone::try_new(zone).map_err(|_| CreateError::IllegalValue("zone"))?;
    let hemisphere = if params.flag("south") {
        Hemisphere::South
    } else {
        Hemisphere::North
    };
    let algo = tmerc_algo(params)?;
    let lon0 = (f64::from(zone.number()) - 0.5) * 6.0 - 180.0;
    let y0 = match hemisphere {
        Hemisphere::North => 0.0,
        Hemisphere::South => 10_000_000.0,
    };
    let lam0 = lon0.to_radians();
    let k0 = 0.9996;
    let exact =
        || Tmerc::utm(zone, hemisphere, frame.earth).map_err(|_| CreateError::IllegalValue("zone"));
    let approx = || approx_from_degrees(frame, lon0, 0.0, k0, 500_000.0, y0);
    match algo {
        TmercAlgo::Evenden => Ok(native_approx(frame, approx()?)),
        TmercAlgo::Exact => Ok(native_exact(frame, exact()?)),
        TmercAlgo::Auto if auto_is_exact_only(frame.earth.es(), 0.0, k0) => {
            Ok(native_exact(frame, exact()?))
        }
        TmercAlgo::Auto => Ok(native_auto(
            frame,
            exact()?,
            approx()?,
            lam0,
            500_000.0,
            y0,
            frame.earth.ra(),
        )),
    }
}

fn build_lcc(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let lat1 = params.radians("lat_1")?.unwrap_or(frame.phi0);
    let lat2 = params.radians("lat_2")?.unwrap_or(lat1);
    let cone = Cone {
        lon0_deg: frame.lam0.to_degrees(),
        lat0_deg: frame.phi0.to_degrees(),
        lat1_deg: lat1.to_degrees(),
        lat2_deg: lat2.to_degrees(),
        x0: frame.x0,
        y0: frame.y0,
        k0: frame.k0,
    };
    let lcc = Lcc::try_new(frame.earth, cone).map_err(|_| CreateError::IllegalValue("lat_1"))?;
    let inverse = lcc.clone();
    Ok(Box::new(NativeProjected::new(
        frame,
        move |lon, lat| lcc.forward(lon.to_degrees(), lat.to_degrees()),
        move |xy| inverse.inverse(xy),
    )))
}

fn build_merc(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let mut k0 = frame.k0;
    if let Some(lat_ts) = params.radians("lat_ts")? {
        if lat_ts.abs() >= FRAC_PI_2 {
            return Err(CreateError::IllegalValue("lat_ts"));
        }
        k0 = if frame.earth.es() == 0.0 {
            lat_ts.cos()
        } else {
            crate::projmath::msfn(lat_ts.sin(), lat_ts.cos(), frame.earth.es())
        };
    }
    let merc = if frame.earth.es() == 0.0 {
        Mercator::try_sphere(
            frame.earth.a(),
            frame.lam0.to_degrees(),
            k0,
            frame.x0,
            frame.y0,
        )
        .map_err(|_| CreateError::IllegalValue("k_0"))?
    } else {
        Mercator::try_ellipsoid(frame.earth, frame.lam0.to_degrees(), k0, frame.x0, frame.y0)
            .map_err(|_| CreateError::IllegalValue("k_0"))?
    };
    Ok(Box::new(NativeProjected::new(
        frame,
        move |lon, lat| merc.forward(lon.to_degrees(), lat.to_degrees()),
        move |xy| merc.inverse(xy),
    )))
}

fn build_helmert(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    datum_ops::build("helmert", params, frame)?.ok_or(CreateError::IllegalValue("helmert"))
}

fn build_cart(_params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    Ok(Box::new(CartStep {
        frame: frame.clone(),
    }))
}

fn build_longlat(_params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    Ok(Box::new(LongLat {
        frame: frame.clone(),
    }))
}

/// Kernel whose `(x, y)` are in units of the semi-major axis.
pub(crate) trait Kernel: Send + Sync {
    /// Forward. `lam` is already relative to the central meridian.
    ///
    /// # Errors
    ///
    /// [`TransformError::OutsideDomain`] when the point has no image.
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError>;

    /// Inverse, arguments in units of the semi-major axis.
    ///
    /// # Errors
    ///
    /// [`TransformError::OutsideDomain`] or [`TransformError::NoInverse`].
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError>;
}

struct Classic<K> {
    frame: Frame,
    kernel: K,
}

impl<K: Kernel> Classic<K> {
    fn new(frame: Frame, kernel: K) -> Self {
        Self { frame, kernel }
    }
}

impl<K: Kernel + 'static> Step for Classic<K> {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        // PROJ returns NaN coordinates for a NaN input. It does not set errno.
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        let (x, y) = self.kernel.forward(lam, phi)?;
        Ok(Coord {
            x: self.frame.fr_meter * (self.frame.earth.a() * x + self.frame.x0),
            y: self.frame.fr_meter * (self.frame.earth.a() * y + self.frame.y0),
            z: self.frame.vfr_meter * (coord.z + self.frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let x = (self.frame.to_meter * coord.x - self.frame.x0) * self.frame.earth.ra();
        let y = (self.frame.to_meter * coord.y - self.frame.y0) * self.frame.earth.ra();
        let (mut lam, mut phi) = self.kernel.inverse(x, y)?;
        lam = lam + self.frame.from_greenwich + self.frame.lam0;
        if !self.frame.over {
            lam = adjlon(lam);
        }
        if self.frame.geoc {
            phi = geographic_to_geocentric(phi, self.frame.earth.one_es());
        }
        Ok(Coord {
            x: lam,
            y: phi,
            z: self.frame.vto_meter * coord.z - self.frame.z0,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Projected
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

/// Angular forward prepare: latitude gate, `lon_0`, prime meridian, geocentric latitude.
pub(crate) fn prepare_forward(
    frame: &Frame,
    mut lon: f64,
    mut lat: f64,
) -> Result<(f64, f64), TransformError> {
    if !lon.is_finite() || !lat.is_finite() {
        return Err(TransformError::NonFinite);
    }
    if lat.abs() > FRAC_PI_2 {
        if lat > 0.0 {
            if lat - FRAC_PI_2 > EPS_LAT {
                return Err(TransformError::InvalidCoord);
            }
            lat = FRAC_PI_2;
        } else if lat - FRAC_PI_2 < -EPS_LAT {
            return Err(TransformError::InvalidCoord);
        } else {
            lat = -FRAC_PI_2;
        }
    }
    if lon.abs() > PI {
        if lon.abs() > 10.0 {
            return Err(TransformError::InvalidCoord);
        }
        if !frame.over {
            lon = adjlon(lon);
        }
    }
    if frame.geoc {
        lat = geocentric_to_geographic(lat, frame.earth.one_es());
    }
    lon = (lon - frame.from_greenwich) - frame.lam0;
    if !frame.over {
        lon = adjlon(lon);
    }
    Ok((lon, lat))
}

/// `fwd_finalize` for radian output: `center + adjlon(lam - center)`.
pub(crate) fn wrap_longitude(center: Option<f64>, lam: f64) -> f64 {
    let Some(center) = center else {
        return lam;
    };
    if !lam.is_finite() {
        return lam;
    }
    center + adjlon(lam - center)
}

/// Inverse finalize for a radian output: put `lon_0` and the prime meridian back.
pub(crate) fn finish_angular(frame: &Frame, mut lam: f64, mut phi: f64) -> (f64, f64) {
    lam = lam + frame.from_greenwich + frame.lam0;
    if !frame.over {
        lam = adjlon(lam);
    }
    if frame.geoc {
        phi = geographic_to_geocentric(phi, frame.earth.one_es());
    }
    (lam, phi)
}

/// Geographic latitude to geocentric: `atan((1 - e^2) tan phi)`.
pub(crate) fn geographic_to_geocentric(phi: f64, one_es: f64) -> f64 {
    let limit = FRAC_PI_2 - 1e-9;
    if phi > limit || phi < -limit || one_es == 1.0 {
        phi
    } else {
        (one_es * phi.tan()).atan()
    }
}

pub(crate) fn geocentric_to_geographic(phi: f64, one_es: f64) -> f64 {
    let limit = FRAC_PI_2 - 1e-9;
    if phi > limit || phi < -limit || one_es == 1.0 {
        phi
    } else {
        (phi.tan() / one_es).atan()
    }
}

struct NativeProjected<F, I> {
    frame: Frame,
    forward_xy: F,
    inverse_xy: I,
}

impl<F, I> NativeProjected<F, I>
where
    F: Fn(f64, f64) -> Result<Projected, ProjectionError> + Send + Sync,
    I: Fn(Projected) -> Result<Geographic, ProjectionError> + Send + Sync,
{
    fn new(frame: &Frame, forward_xy: F, inverse_xy: I) -> Self {
        Self {
            frame: frame.clone(),
            forward_xy,
            inverse_xy,
        }
    }
}

impl<F, I> Step for NativeProjected<F, I>
where
    F: Fn(f64, f64) -> Result<Projected, ProjectionError> + Send + Sync,
    I: Fn(Projected) -> Result<Geographic, ProjectionError> + Send + Sync,
{
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        // `pm=` is subtracted before the kernel, as in `fwd_prepare`.
        let projected = (self.forward_xy)(coord.x - self.frame.from_greenwich, coord.y)?;
        Ok(Coord {
            x: self.frame.fr_meter * projected.east,
            y: self.frame.fr_meter * projected.north,
            z: self.frame.vfr_meter * (coord.z + self.frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let geographic = (self.inverse_xy)(Projected {
            east: self.frame.to_meter * coord.x,
            north: self.frame.to_meter * coord.y,
        })?;
        Ok(Coord {
            x: geographic.lon_deg.to_radians() + self.frame.from_greenwich,
            y: geographic.lat_deg.to_radians(),
            z: self.frame.vto_meter * coord.z - self.frame.z0,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Projected
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

struct CartStep {
    frame: Frame,
}

impl Step for CartStep {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        let (x, y, z) = geodetic_to_cartesian(self.frame.earth, lam, phi, coord.z);
        Ok(Coord {
            x: self.frame.fr_meter * x,
            y: self.frame.fr_meter * y,
            z: self.frame.fr_meter * z,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (mut lam, mut phi, z) = cartesian_to_geodetic(
            self.frame.earth,
            self.frame.to_meter * coord.x,
            self.frame.to_meter * coord.y,
            self.frame.to_meter * coord.z,
        );
        lam = lam + self.frame.from_greenwich + self.frame.lam0;
        if !self.frame.over {
            lam = adjlon(lam);
        }
        if self.frame.geoc {
            phi = geographic_to_geocentric(phi, self.frame.earth.one_es());
        }
        Ok(Coord {
            x: lam,
            y: phi,
            z,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

pub(crate) fn geodetic_to_cartesian(
    earth: Ellipsoid,
    lam: f64,
    phi: f64,
    height: f64,
) -> (f64, f64, f64) {
    let sinphi = phi.sin();
    let cosphi = phi.cos();
    let n = if earth.es() == 0.0 {
        earth.a()
    } else {
        earth.a() / (1.0 - earth.es() * sinphi * sinphi).sqrt()
    };
    let x = (n + height) * cosphi * lam.cos();
    let y = (n + height) * cosphi * lam.sin();
    let z = (n * (1.0 - earth.es()) + height) * sinphi;
    (x, y, z)
}

pub(crate) fn cartesian_to_geodetic(earth: Ellipsoid, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let x_div = x * earth.ra();
    let y_div = y * earth.ra();
    let z_div = z * earth.ra();
    let p_div = (x_div * x_div + y_div * y_div).sqrt();
    let b_div = 1.0 - earth.flattening();
    let p_b = p_div * b_div;
    let norm = (z_div * z_div + p_b * p_b).sqrt();
    let (c, s) = if norm != 0.0 {
        (p_b / norm, z_div / norm)
    } else {
        (1.0, 0.0)
    };
    let y_phi = z_div + earth.e2s() * b_div * s * s * s;
    let x_phi = p_div - earth.es() * c * c * c;
    let norm_phi = (y_phi * y_phi + x_phi * x_phi).sqrt();
    let (cosphi, sinphi, phi) = if x_phi <= 0.0 {
        let sign = if z >= 0.0 { 1.0 } else { -1.0 };
        (0.0, sign, sign * FRAC_PI_2)
    } else if norm_phi != 0.0 {
        let cosphi = x_phi / norm_phi;
        let sinphi = y_phi / norm_phi;
        (cosphi, sinphi, (y_phi / x_phi).atan())
    } else {
        (1.0, 0.0, 0.0)
    };
    let lam = y_div.atan2(x_div);
    let height = if cosphi < 1e-6 {
        let r = geocentric_radius(earth.a(), b_div, cosphi, sinphi);
        z.abs() - r
    } else {
        let n = if earth.es() == 0.0 {
            earth.a()
        } else {
            earth.a() / (1.0 - earth.es() * sinphi * sinphi).sqrt()
        };
        earth.a() * p_div / cosphi - n
    };
    (lam, phi, height)
}

fn geocentric_radius(a: f64, b_div_a: f64, cosphi: f64, sinphi: f64) -> f64 {
    let cos2 = cosphi * cosphi;
    let sin2 = sinphi * sinphi;
    let b2 = b_div_a * b_div_a;
    let b2sin2 = b2 * sin2;
    a * ((cos2 + b2 * b2sin2) / (cos2 + b2sin2)).sqrt()
}

struct LongLat {
    frame: Frame,
}

impl Step for LongLat {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        Ok(Coord {
            x: wrap_longitude(self.frame.lon_wrap, lam),
            y: phi,
            z: self.frame.vfr_meter * (coord.z + self.frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let mut phi = coord.y;
        if self.frame.geoc {
            phi = geographic_to_geocentric(phi, self.frame.earth.one_es());
        }
        let mut lam = coord.x + self.frame.from_greenwich + self.frame.lam0;
        if !self.frame.over {
            lam = adjlon(lam);
        }
        Ok(Coord {
            x: lam,
            y: phi,
            z: self.frame.vto_meter * coord.z - self.frame.z0,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

struct PipeStep {
    inverted: bool,
    omit_fwd: bool,
    omit_inv: bool,
    inner: std::sync::Arc<dyn Step>,
}

struct Pipeline {
    steps: Vec<PipeStep>,
    stack: Mutex<PipelineStack>,
    uses_stack: bool,
    a: f64,
    es: f64,
}

impl Pipeline {
    fn lock_stack(&self) -> std::sync::MutexGuard<'_, PipelineStack> {
        self.stack
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Step for Pipeline {
    fn forward(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        if !self.uses_stack {
            for step in &self.steps {
                if step.omit_fwd {
                    continue;
                }
                coord = if step.inverted {
                    step.inner.inverse(coord)?
                } else {
                    step.inner.forward(coord)?
                };
            }
            return Ok(coord);
        }
        let mut stack = self.lock_stack();
        for step in &self.steps {
            if step.omit_fwd {
                continue;
            }
            coord = if step.inverted {
                step.inner.inverse_in_pipeline(coord, &mut stack)?
            } else {
                step.inner.forward_in_pipeline(coord, &mut stack)?
            };
        }
        Ok(coord)
    }

    fn inverse(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        if !self.uses_stack {
            for step in self.steps.iter().rev() {
                if step.omit_inv {
                    continue;
                }
                coord = if step.inverted {
                    step.inner.forward(coord)?
                } else {
                    step.inner.inverse(coord)?
                };
            }
            return Ok(coord);
        }
        let mut stack = self.lock_stack();
        for step in self.steps.iter().rev() {
            if step.omit_inv {
                continue;
            }
            coord = if step.inverted {
                step.inner.forward_in_pipeline(coord, &mut stack)?
            } else {
                step.inner.inverse_in_pipeline(coord, &mut stack)?
            };
        }
        Ok(coord)
    }

    fn input_kind(&self) -> IoKind {
        self.steps
            .iter()
            .map(|step| kind_in(step.inverted, step.inner.as_ref()))
            .find(|kind| *kind != IoKind::Any)
            .unwrap_or(IoKind::Projected)
    }

    fn output_kind(&self) -> IoKind {
        self.steps
            .iter()
            .rev()
            .map(|step| kind_out(step.inverted, step.inner.as_ref()))
            .find(|kind| *kind != IoKind::Any)
            .unwrap_or(IoKind::Projected)
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

fn build_pipeline(definition: &str) -> Result<Pipeline, CreateError> {
    let parts = split_pipeline(definition);
    if parts.len() < 2 || !parts[0].contains("proj=pipeline") || step_precedes_pipeline(definition)
    {
        return Err(CreateError::IllegalValue("pipeline"));
    }
    if parts[0].matches("proj=").count() > 1 {
        return Err(CreateError::IllegalValue("pipeline"));
    }
    let header_params = Params::parse(&parts[0])?;
    let frame = Frame::from_params(&header_params).unwrap_or_else(|_| {
        Frame::from_params(&Params::parse("+proj=pipeline +ellps=GRS80").expect("GRS80 pipeline"))
            .expect("GRS80 frame")
    });
    let mut steps = Vec::new();
    for chunk in parts.iter().skip(1) {
        let mut step_params = Params::parse(chunk)?;
        for (key, value) in &header_params.items {
            if key == "proj" || key == "step" || key == "inv" {
                continue;
            }
            if !step_params.flag(key) {
                step_params.items.push((key.clone(), value.clone()));
            }
        }
        prepare_params(&mut step_params)?;
        let inverted = step_params.flag("inv");
        // Presence, not the value. PROJ's `bomit_fwd` is the same test.
        let omit_fwd = step_params.flag("omit_fwd");
        let omit_inv = step_params.flag("omit_inv");
        let name = step_params.text("proj").unwrap_or("").to_owned();
        if name == "pipeline" {
            return Err(CreateError::IllegalValue("pipeline"));
        }
        if name.is_empty() {
            return Err(CreateError::MissingArgument("proj"));
        }
        let step_frame = Frame::from_params(&step_params)?;
        let step = finish_step(&name, &step_params, &step_frame)?;
        steps.push(PipeStep {
            inverted,
            omit_fwd,
            omit_inv,
            inner: step,
        });
    }
    // `+inv` on the pipeline inverts the whole chain: reverse order and toggle
    // each step. A step that already carries `+inv` is toggled back.
    // `omit_fwd` / `omit_inv` name a direction of that chain, so they swap.
    if header_params.flag("inv") {
        steps.reverse();
        for step in &mut steps {
            step.inverted = !step.inverted;
            std::mem::swap(&mut step.omit_fwd, &mut step.omit_inv);
        }
    }
    if steps.is_empty() {
        return Err(CreateError::IllegalValue("pipeline"));
    }
    let uses_stack = steps.iter().any(|step| step.inner.needs_pipeline_stack());
    for pair in steps.windows(2) {
        if !kinds_compatible(
            kind_out(pair[0].inverted, pair[0].inner.as_ref()),
            kind_in(pair[1].inverted, pair[1].inner.as_ref()),
        ) {
            return Err(CreateError::IllegalValue("pipeline"));
        }
    }
    Ok(Pipeline {
        steps,
        stack: Mutex::new(PipelineStack::new()),
        uses_stack,
        a: frame.earth.a(),
        es: frame.earth.es(),
    })
}

fn step_precedes_pipeline(definition: &str) -> bool {
    let shrunk = shrink_equals(definition);
    let tokens: Vec<&str> = shrunk
        .split_whitespace()
        .map(|token| token.trim_start_matches('+'))
        .collect();
    let step = tokens.iter().position(|token| *token == "step");
    let pipeline = tokens
        .iter()
        .position(|token| token.starts_with("proj=pipeline"));
    match (step, pipeline) {
        (Some(step_at), Some(pipeline_at)) => step_at < pipeline_at,
        (Some(_), None) => true,
        _ => false,
    }
}

fn kinds_compatible(left: IoKind, right: IoKind) -> bool {
    left == IoKind::Any || right == IoKind::Any || left == right
}

fn kind_in(inverted: bool, step: &dyn Step) -> IoKind {
    if inverted {
        step.output_kind()
    } else {
        step.input_kind()
    }
}

fn kind_out(inverted: bool, step: &dyn Step) -> IoKind {
    if inverted {
        step.input_kind()
    } else {
        step.output_kind()
    }
}

fn split_pipeline(definition: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    for token in shrink_equals(definition).split_whitespace() {
        if token.trim_start_matches('+') == "step" {
            parts.push(String::new());
            continue;
        }
        let last = parts
            .last_mut()
            .expect("pipeline parts always has one string");
        if !last.is_empty() {
            last.push(' ');
        }
        last.push_str(token);
    }
    parts.retain(|part| !part.is_empty());
    parts
}

/// Geodesic-scale distance between two radian positions, plus height.
#[must_use]
pub fn lpz_dist(a: f64, es: f64, left: Coord, right: Coord) -> f64 {
    if !left.x.is_finite() || !right.x.is_finite() {
        return f64::INFINITY;
    }
    let phi = 0.5 * (left.y + right.y);
    let sinphi = phi.sin();
    let one = 1.0 - es * sinphi * sinphi;
    let meridional = a * (1.0 - es) / (one * one.sqrt());
    let parallel = a * phi.cos() / one.sqrt();
    let east = adjlon(right.x - left.x) * parallel;
    let north = (right.y - left.y) * meridional;
    (east * east + north * north + (right.z - left.z).powi(2)).sqrt()
}

/// Euclidean distance, ignoring time.
#[must_use]
pub fn xyz_dist(left: Coord, right: Coord) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Read a PROJ number. Underscores between digits are ignored.
#[must_use]
pub(crate) fn proj_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut filtered = String::new();
    for ch in text.chars() {
        if ch != '_' {
            filtered.push(ch);
        }
    }
    let mut end = 0;
    let bytes = filtered.as_bytes();
    if bytes.first() == Some(&b'+') || bytes.first() == Some(&b'-') {
        end = 1;
    }
    let mut saw_digit = false;
    let mut saw_dot = false;
    while end < bytes.len() {
        match bytes[end] {
            b'0'..=b'9' => {
                saw_digit = true;
                end += 1;
            }
            b'.' if !saw_dot => {
                saw_dot = true;
                end += 1;
            }
            b'e' | b'E' if saw_digit => {
                end += 1;
                if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
                    end += 1;
                }
                let exp = end;
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                if end == exp {
                    end -= 1;
                    if bytes.get(end) == Some(&b'+') || bytes.get(end) == Some(&b'-') {
                        end -= 1;
                    }
                }
                break;
            }
            _ => break,
        }
    }
    if !saw_digit {
        return None;
    }
    filtered[..end].parse().ok()
}

/// Decimal degrees, including a DMS suffix. The sign of a trailing hemisphere wins.
#[must_use]
pub(crate) fn parse_dms_degrees(text: &str) -> Option<f64> {
    assemble_dms(text.trim())
}

fn assemble_dms(text: &str) -> Option<f64> {
    let (signed, rest) = take_number(text.trim())?;
    let sign = if signed.is_sign_negative() { -1.0 } else { 1.0 };
    let mut degrees = signed.abs();
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some(signed);
    }
    let bytes = rest.as_bytes();
    if bytes[0] != b'd' && bytes[0] != b'D' {
        return hemisphere_only(signed, rest);
    }
    let mut cursor = rest[1..].trim_start();
    if let Some((minutes, next)) = take_number(cursor) {
        degrees += minutes.abs() / 60.0;
        cursor = next.trim_start();
        if cursor.starts_with('\'') || cursor.starts_with('m') || cursor.starts_with('M') {
            cursor = cursor[1..].trim_start();
        }
        if let Some((seconds, next)) = take_number(cursor) {
            degrees += seconds.abs() / 3600.0;
            cursor = next.trim_start();
            // A lone `S` is south, not the seconds marker. `45.6S` is the gie case.
            if cursor.starts_with('"')
                || ((cursor.starts_with('s') || cursor.starts_with('S'))
                    && hemisphere(cursor).is_none())
            {
                cursor = cursor[1..].trim_start();
            }
        }
    }
    if let Some(hemi) = hemisphere(cursor) {
        return Some(degrees.abs() * hemi);
    }
    if !cursor.is_empty() {
        return None;
    }
    Some(degrees * sign)
}

fn hemisphere_only(signed: f64, rest: &str) -> Option<f64> {
    hemisphere(rest).map(|hemi| signed.abs() * hemi)
}

fn hemisphere(text: &str) -> Option<f64> {
    match text.trim() {
        "" => None,
        "N" | "n" | "E" | "e" => Some(1.0),
        "S" | "s" | "W" | "w" => Some(-1.0),
        _ => None,
    }
}

fn take_number(text: &str) -> Option<(f64, &str)> {
    let trimmed = text.trim_start();
    let offset = text.len() - trimmed.len();
    let mut end = 0;
    let bytes = trimmed.as_bytes();
    if bytes.first() == Some(&b'+') || bytes.first() == Some(&b'-') {
        end = 1;
    }
    let mut saw = false;
    let mut dot = false;
    while end < bytes.len() {
        match bytes[end] {
            b'0'..=b'9' => {
                saw = true;
                end += 1;
            }
            b'_' => end += 1,
            b'.' if !dot => {
                dot = true;
                end += 1;
            }
            b'e' | b'E' if saw => {
                let mark = end;
                end += 1;
                if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
                    end += 1;
                }
                let exp = end;
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                if end == exp {
                    end = mark;
                }
                break;
            }
            _ => break,
        }
    }
    if !saw {
        return None;
    }
    let value = proj_number(&trimmed[..end])?;
    Some((value, &text[offset + end..]))
}

/// Parse one gie coordinate token list into up to four values.
///
/// # Errors
///
/// Returns nothing when fewer than two numbers are present. Callers treat that
/// as a malformed expectation.
pub fn parse_gie_coord(text: &str) -> Option<(Coord, usize)> {
    let mut coord = Coord {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        t: 0.0,
    };
    let mut count = 0;
    for token in text.split_whitespace() {
        if count == 4 {
            break;
        }
        let value = if token == "HUGE_VAL" {
            f64::INFINITY
        } else if token.eq_ignore_ascii_case("nan") {
            f64::NAN
        } else if token.contains('d')
            || token.contains('D')
            || token.contains('\'')
            || token.contains('"')
        {
            let Some(parsed) = assemble_dms(token) else {
                break;
            };
            parsed
        } else {
            let Some(parsed) = proj_number(token) else {
                break;
            };
            parsed
        };
        match count {
            0 => coord.x = value,
            1 => coord.y = value,
            2 => coord.z = value,
            _ => coord.t = value,
        }
        count += 1;
    }
    if count < 2 {
        None
    } else {
        Some((coord, count))
    }
}

/// Build a classic step from a kernel. Projection modules use this.
pub(crate) fn classic_step<K: Kernel + 'static>(frame: &Frame, kernel: K) -> Box<dyn Step> {
    Box::new(Classic::new(frame.clone(), kernel))
}

/// Shared authalic helpers re-exported for projection modules.
pub(crate) use crate::projmath::{
    authalic_inverse as authalic_lat_inverse, authalic_q, msfn as proj_msfn,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aea_grs80_matches_the_first_builtins_point() {
        let op = Operation::create("+proj=aea +ellps=GRS80 +lat_1=0 +lat_2=2").expect("aea");
        let got = op
            .forward(Coord::xy(2.0_f64.to_radians(), 1.0_f64.to_radians()))
            .expect("forward");
        assert!((got.x - 222_571.608_757_106).abs() < 1e-6, "{}", got.x);
        assert!((got.y - 110_653.326_743_030).abs() < 1e-6, "{}", got.y);
    }

    #[test]
    fn unknown_proj_is_an_error() {
        let error = Operation::create("+proj=not_a_projection").expect_err("unknown");
        assert!(matches!(error, CreateError::UnknownProj(_)));
    }

    #[test]
    fn dms_and_underscores() {
        assert!((parse_dms_degrees("17d40'W").expect("ferro") + 17.0 + 40.0 / 60.0).abs() < 1e-12);
        assert!((proj_number("10_018_754.1714").expect("num") - 10_018_754.171_4).abs() < 1e-6);
    }

    #[test]
    fn r_lat_matches_ellipsoid_gie_and_r_c_ignores_lat_0() {
        let point = Coord::xy(12.0_f64.to_radians(), 55.0_f64.to_radians());
        let arithmetic = Operation::create("+proj=merc +ellps=GRS80 +R_lat_a=60").expect("R_lat_a");
        let got = arithmetic.forward(point).expect("forward");
        assert!((got.x - 1_338_073.743_626_891_9).abs() < 1e-8, "{}", got.x);
        assert!((got.y - 7_374_210.092_480_332_6).abs() < 1e-8, "{}", got.y);
        let geometric = Operation::create("+proj=merc +ellps=GRS80 +R_lat_g=60").expect("R_lat_g");
        let got = geometric.forward(point).expect("forward");
        assert!((got.x - 1_338_073.269_610_159_3).abs() < 1e-8, "{}", got.x);
        assert!((got.y - 7_374_207.480_143_763_1).abs() < 1e-8, "{}", got.y);
        let collapsed = Operation::create("+proj=merc +a=1E77 +R_lat_a=90 +b=1");
        assert!(matches!(collapsed, Err(CreateError::IllegalValue(_))));
        let at_origin = Operation::create("+proj=merc +R_C +ellps=WGS84").expect("R_C");
        let at_45 = Operation::create("+proj=merc +R_C +ellps=WGS84 +lat_0=45").expect("R_C lat");
        let sample = Coord::xy(2.0_f64.to_radians(), 49.0_f64.to_radians());
        let origin = at_origin.forward(sample).expect("origin");
        let shifted = at_45.forward(sample).expect("lat_0");
        assert!((origin.x - shifted.x).abs() < 1e-6);
        assert!((origin.y - shifted.y).abs() < 1e-6);
    }

    #[test]
    fn pipeline_omit_skips_only_the_named_direction() {
        let merc = Operation::create("+proj=merc +ellps=WGS84").expect("merc");
        let input = Coord::xy(0.2, 0.3);
        let projected = merc.forward(input).expect("merc forward");
        let omit_fwd = Operation::create("+proj=pipeline +step +proj=merc +ellps=WGS84 +omit_fwd")
            .expect("omit_fwd");
        let skipped = omit_fwd.forward(input).expect("omit forward");
        assert!((skipped.x - input.x).abs() < 1e-15);
        assert!((skipped.y - input.y).abs() < 1e-15);
        let back = omit_fwd.inverse(projected).expect("omit_fwd inverse");
        assert!((back.x - input.x).abs() < 1e-9);
        assert!((back.y - input.y).abs() < 1e-9);
    }
}
