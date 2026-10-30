//! PROJ strings for the geographic CRS the unit tests export.
//!
//! Datum codes 6326, 6267, and 6269 become `+datum=`. Anything else is
//! `+ellps` plus `+pm` when the prime meridian is not Greenwich.

use super::bound::hub_is_wgs84;
use super::model::{
    Axis, BoundCRS, CartesianCS, Conversion, CoordinateOperation, Crs, Datum, DatumEnsemble,
    Ellipsoid, EllipsoidalCS, FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Identified, Identifier, ParameterValue, PrimeMeridian, ProjectedCRS, UnitKind,
    UnitOfMeasure, VerticalCRS,
};
use super::parammap::{MethodMap, OTHER_METHODS, PROJECTION_METHODS};
use super::pipeline::proj_number;
use super::wkt::format_g;
use crate::ellipsoid::PJ_ELLPS;
use crate::projections::{angular_proj_unit, linear_proj_unit};

/// What a bound CRS adds to the PROJ string of its base CRS.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ProjExtras<'a> {
    /// `+geoidgrids=`, written with `+geoid_crs=WGS84`, for a vertical base CRS.
    geoid_grid: Option<&'a str>,
    /// `+towgs84=`, for a horizontal base CRS bound to WGS 84.
    towgs84: Option<[f64; 7]>,
}

impl Ellipsoid {
    /// `exportToPROJString` for an ellipsoid: `+ellps=` for a known name,
    /// otherwise `+a` with `+rf` or `+b`, or `+R` for a sphere.
    #[must_use]
    pub fn export_to_proj_string(&self) -> String {
        ellipsoid_proj(self)
    }
}

impl GeographicCRS {
    /// `exportToPROJString` for a geographic CRS.
    #[must_use]
    pub fn export_to_proj_string(&self) -> String {
        geographic_proj(self, &ProjExtras::default())
    }
}

impl GeodeticCRS {
    /// `exportToPROJString` for a geographic or metre geocentric CRS.
    ///
    /// # Errors
    ///
    /// The CRS is spherical, or geocentric in a unit other than the metre.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        self.export_with(&ProjExtras::default())
    }

    fn export_with(&self, extras: &ProjExtras<'_>) -> Result<String, FactoryError> {
        match self {
            Self::Geographic(crs) => Ok(geographic_proj(crs, extras)),
            Self::Geocentric(crs) if extras.towgs84.is_none() => crs.export_to_proj_string(),
            Self::Geocentric(_) => Err(FactoryError::Format(
                "a bound geocentric CRS has no PROJ string in this build".to_owned(),
            )),
            Self::Spherical(_) => Err(FactoryError::Format(
                "a spherical CRS has no PROJ string in this build".to_owned(),
            )),
        }
    }
}

impl GeographicCRS {
    /// `normalizeForVisualization`: latitude/longitude becomes longitude/latitude.
    #[must_use]
    pub fn normalize_for_visualization(&self) -> Self {
        let axes = self.coordinate_system().axes();
        if axes.len() < 2 || axes[0].direction() != "north" || axes[1].direction() != "east" {
            return self.clone();
        }
        let mut crs = self.clone();
        crs.coordinate_system.axes.swap(0, 1);
        apply_axis_reversal(&mut crs.identified);
        crs
    }
}

impl ProjectedCRS {
    /// `normalizeForVisualization` for a projected CRS whose axes are not east-then-north.
    #[must_use]
    pub fn normalize_for_visualization(&self) -> Self {
        if projected_axis_order(self).is_none() {
            return self.clone();
        }
        let mut crs = self.clone();
        crs.coordinate_system.axes.swap(0, 1);
        apply_axis_reversal(&mut crs.identified);
        crs
    }
}

impl Crs {
    /// `exportToPROJString` for the CRS kinds PROJ writes as one string: geographic,
    /// geocentric, and projected.
    ///
    /// # Errors
    ///
    /// Any other kind, or a CRS whose parts cannot be written as PROJ text.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        self.export_with(&ProjExtras::default())
    }

    fn export_with(&self, extras: &ProjExtras<'_>) -> Result<String, FactoryError> {
        match self {
            Self::Geographic(crs) => Ok(geographic_proj(crs, extras)),
            Self::Geodetic(crs) => crs.export_with(extras),
            Self::Projected(crs) => crs.export_with(extras),
            Self::Vertical(crs) => Ok(vertical_proj(crs, extras)),
            Self::Bound(crs) => export_bound(crs),
            _ => Err(FactoryError::Format(
                "this CRS kind has no PROJ string export".to_owned(),
            )),
        }
    }

    /// `normalizeForVisualization` for a geographic or projected CRS.
    #[must_use]
    pub fn normalize_for_visualization(&self) -> Self {
        match self {
            Self::Geographic(crs) => Self::Geographic(crs.normalize_for_visualization()),
            Self::Projected(crs) => Self::Projected(crs.normalize_for_visualization()),
            other => other.clone(),
        }
    }

    /// Remarks recorded on this CRS.
    #[must_use]
    pub fn remarks(&self) -> &str {
        match self {
            Self::Geographic(crs) => &crs.identified.remarks,
            Self::Geodetic(GeodeticCRS::Geographic(crs)) => &crs.identified.remarks,
            Self::Geodetic(GeodeticCRS::Geocentric(crs)) => &crs.identified.remarks,
            Self::Geodetic(GeodeticCRS::Spherical(crs)) => &crs.identified.remarks,
            Self::Projected(crs) => &crs.identified.remarks,
            Self::Vertical(crs) => &crs.identified.remarks,
            Self::Compound(crs) => &crs.identified.remarks,
            Self::Bound(crs) => &crs.identified.remarks,
            Self::Engineering(crs) => &crs.identified.remarks,
            Self::DerivedProjected(crs) => &crs.identified.remarks,
        }
    }
}

fn apply_axis_reversal(identified: &mut Identified) {
    const SUFFIX: &str = " (with axis order normalized for visualization)";
    if !identified.name.ends_with(SUFFIX) {
        identified.name.push_str(SUFFIX);
    }
    let prefix = "Axis order reversed compared to ";
    if identified.identifiers.len() == 1 && !identified.remarks.starts_with(prefix) {
        let identifier = &identified.identifiers[0];
        let mut remarks = format!("{prefix}{}:{}", identifier.code_space, identifier.code);
        if !identified.remarks.is_empty() {
            remarks.push_str(". ");
            remarks.push_str(&identified.remarks);
        }
        identified.remarks = remarks;
    }
}

impl ProjectedCRS {
    /// `exportToPROJString` for a projected CRS: the conversion, the base datum, and the linear unit.
    ///
    /// # Errors
    ///
    /// The deriving conversion cannot be written as a PROJ string.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        self.export_with(&ProjExtras::default())
    }

    fn export_with(&self, extras: &ProjExtras<'_>) -> Result<String, FactoryError> {
        let mut text = self.deriving_conversion().export_to_proj_string()?;
        text.push(' ');
        text.push_str(&crs_datum_clause(self.base_crs(), extras.towgs84.as_ref()));
        if let Some(unit) = self.coordinate_system().axes().first().map(Axis::unit) {
            if let Some(clause) = linear_unit_clause(unit) {
                text.push(' ');
                text.push_str(&clause);
            }
        }
        text.push_str(" +no_defs +type=crs");
        Ok(text)
    }
}

/// PROJ string of one operation from `source` to `target`.
///
/// This is the no-database path of `CoordinateOperationFactory::createOperation`.
/// A projected target whose base is `source` becomes a pipeline of axis order,
/// degree-to-radian conversion, and the deriving conversion with `+ellps`.
/// A geocentric target on the same datum becomes `+proj=cart` plus a linear
/// `unitconvert` when the cartesian unit is not the metre.
///
/// # Errors
///
/// `target` is neither of those CRS, or a step cannot be written.
pub(crate) fn operation_proj_string(source: &Crs, target: &Crs) -> Result<String, FactoryError> {
    if let Crs::Geodetic(GeodeticCRS::Geocentric(geocentric)) = target {
        return geocentric_operation(source, geocentric);
    }
    let Crs::Projected(projected) = target else {
        return Err(FactoryError::Format(
            "this operation is not a projection of the source CRS".to_owned(),
        ));
    };
    let Some(geographic) = geographic_of(source) else {
        return Err(FactoryError::Format(
            "the source CRS is not geographic".to_owned(),
        ));
    };
    let GeodeticCRS::Geographic(base) = projected.base_crs() else {
        return Err(FactoryError::Format(
            "the projected CRS base is not geographic".to_owned(),
        ));
    };
    if !super::equiv::same_geographic_base(geographic, base) {
        return Err(FactoryError::Format(
            "the source CRS is not the base of the projected CRS".to_owned(),
        ));
    }
    let mut steps = Vec::new();
    if let Some(order) = geographic_axis_order(geographic) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    if let Some(step) = angular_to_radians(geographic) {
        steps.push(step.to_owned());
    }
    let mut projection = projected.deriving_conversion().export_to_proj_string()?;
    if let Some(ellipsoid) = base.ellipsoid() {
        projection.push(' ');
        projection.push_str(&ellipsoid_proj(ellipsoid));
    }
    steps.push(projection);
    if let Some(order) = projected_axis_order(projected) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    Ok(pipeline(&steps))
}

/// The `unitconvert` step that turns the geographic axes into radians, or
/// `None` when they already are.
fn angular_to_radians(geographic: &GeographicCRS) -> Option<&'static str> {
    match geographic.coordinate_system().axes().first()?.unit().name() {
        "degree" => Some("+proj=unitconvert +xy_in=deg +xy_out=rad"),
        "grad" => Some("+proj=unitconvert +xy_in=grad +xy_out=rad"),
        _ => None,
    }
}

/// Geographic source to a geocentric target: axis swap, degrees to radians,
/// `+proj=cart`, then metres into the target linear unit.
fn geocentric_operation(source: &Crs, target: &GeocentricCRS) -> Result<String, FactoryError> {
    let Some(geographic) = geographic_of(source) else {
        return Err(FactoryError::Format(
            "the source CRS is not geographic".to_owned(),
        ));
    };
    let same_datum = match (geographic.datum(), target.datum()) {
        (Some(left), Some(right)) => left.is_equivalent_to(right),
        _ => false,
    };
    if !same_datum {
        return Err(FactoryError::Format(
            "the geocentric datum is not the source datum".to_owned(),
        ));
    }
    let mut steps = Vec::new();
    if let Some(order) = geographic_axis_order(geographic) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    if let Some(step) = angular_to_radians(geographic) {
        steps.push(step.to_owned());
    }
    let mut cart = String::from("+proj=cart");
    if let Some(datum) = target.datum() {
        cart.push(' ');
        cart.push_str(&ellipsoid_proj(datum.ellipsoid()));
        if let Some(clause) = prime_meridian_proj(datum.prime_meridian()) {
            cart.push(' ');
            cart.push_str(&clause);
        }
    }
    steps.push(cart);
    if let Some(unit) = target.coordinate_system.axes().first().map(Axis::unit) {
        if unit.kind() == UnitKind::Linear && (unit.conversion_to_si() - 1.0).abs() > 1e-10 {
            let out = crate::projections::linear_proj_unit(unit.conversion_to_si())
                .map(str::to_owned)
                .unwrap_or_else(|| super::wkt::format_g(unit.conversion_to_si(), 15));
            steps.push(format!(
                "+proj=unitconvert +xy_in=m +z_in=m +xy_out={out} +z_out={out}"
            ));
        }
    }
    Ok(pipeline(&steps))
}

fn pipeline(steps: &[String]) -> String {
    let body = steps
        .iter()
        .map(|step| format!("+step {step}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("+proj=pipeline {body}")
}

pub(crate) fn geographic_of(crs: &Crs) -> Option<&GeographicCRS> {
    match crs {
        Crs::Geographic(crs) => Some(crs),
        Crs::Geodetic(GeodeticCRS::Geographic(crs)) => Some(crs),
        _ => None,
    }
}

/// `2,1` when the first axis is north and the second is east. East-then-north needs no swap.
fn geographic_axis_order(crs: &GeographicCRS) -> Option<String> {
    let axes = crs.coordinate_system().axes();
    if axes.len() < 2 {
        return None;
    }
    let first = axis_slot(axes[0].direction())?;
    let second = axis_slot(axes[1].direction())?;
    if first == "1" && second == "2" {
        None
    } else {
        Some(format!("{first},{second}"))
    }
}

/// Trailing axis swap of `ProjectedCRS::addUnitConvertAndAxisSwap`.
///
/// East-then-north needs none. A polar CRS whose axes are both north or both
/// south, named northing then easting, swaps as `2,1` (UPS North and South).
pub(crate) fn projected_axis_order(crs: &ProjectedCRS) -> Option<String> {
    let axes = crs.coordinate_system().axes();
    if axes.len() < 2 {
        return None;
    }
    let first = axes[0].direction();
    let second = axes[1].direction();
    if first == "east" && second == "north" {
        return None;
    }
    if first != second {
        let left = axis_slot(first)?;
        let right = axis_slot(second)?;
        return Some(format!("{left},{right}"));
    }
    let northing_easting = axes[0].name().to_ascii_lowercase().starts_with("northing")
        && axes[1].name().to_ascii_lowercase().starts_with("easting");
    if northing_easting && (first == "north" || first == "south") {
        Some("2,1".to_owned())
    } else {
        None
    }
}

fn axis_slot(direction: &str) -> Option<&'static str> {
    match direction {
        "east" => Some("1"),
        "west" => Some("-1"),
        "north" => Some("2"),
        "south" => Some("-2"),
        _ => None,
    }
}

/// `addDatumInfoToPROJString`: a datum token, unless `+towgs84` must be written,
/// else the ellipsoid and prime meridian, then `+towgs84`.
fn crs_datum_clause(crs: &GeodeticCRS, towgs84: Option<&[f64; 7]>) -> String {
    if let (GeodeticCRS::Geographic(geographic), None) = (crs, towgs84) {
        if let Some(token) = datum_token(geographic) {
            return format!("+datum={token}");
        }
    }
    let mut parts = Vec::new();
    if let Some(ellipsoid) = crs.ellipsoid() {
        parts.push(ellipsoid_proj(ellipsoid));
    }
    if let Some(meridian) = crs.prime_meridian() {
        if let Some(clause) = prime_meridian_proj(meridian) {
            parts.push(clause);
        }
    }
    if let Some(values) = towgs84 {
        parts.push(towgs84_clause(values));
    }
    parts.join(" ")
}

fn towgs84_clause(values: &[f64; 7]) -> String {
    let numbers: Vec<String> = values.iter().map(|value| format_g(*value, 15)).collect();
    format!("+towgs84={}", numbers.join(","))
}

fn linear_unit_clause(unit: &UnitOfMeasure) -> Option<String> {
    match unit.name() {
        "metre" => Some("+units=m".to_owned()),
        "foot" | "Foot" => Some("+units=ft".to_owned()),
        "US survey foot" => Some("+units=us-ft".to_owned()),
        _ => None,
    }
}

impl GeocentricCRS {
    /// `+proj=geocent` when the cartesian unit is the metre.
    ///
    /// # Errors
    ///
    /// The unit is not the metre. PROJ builds that case only as a pipeline step.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        let unit = self
            .coordinate_system
            .axes()
            .first()
            .map(Axis::unit)
            .ok_or_else(|| FactoryError::Format("a geocentric CRS has no axis".to_owned()))?;
        if unit.name() != "metre" {
            return Err(FactoryError::Format(
                "geocentric CRS PROJ export requires a metre unit".to_owned(),
            ));
        }
        let mut parts = vec!["+proj=geocent".to_owned()];
        if let Some(token) = self.datum().and_then(datum_code_token) {
            parts.push(format!("+datum={token}"));
        } else if let Some(datum) = self.datum() {
            parts.push(ellipsoid_proj(datum.ellipsoid()));
        }
        parts.push("+units=m".to_owned());
        parts.push("+no_defs".to_owned());
        parts.push("+type=crs".to_owned());
        Ok(parts.join(" "))
    }
}

fn geographic_proj(crs: &GeographicCRS, extras: &ProjExtras<'_>) -> String {
    let mut parts = vec!["+proj=longlat".to_owned()];
    match datum_token(crs).filter(|_| extras.towgs84.is_none()) {
        Some(token) => parts.push(format!("+datum={token}")),
        None => {
            if let Some(ellipsoid) = ellipsoid_of(crs) {
                parts.push(ellipsoid_proj(ellipsoid));
            }
            if let Some(meridian) = meridian_of(crs) {
                if let Some(clause) = prime_meridian_proj(meridian) {
                    parts.push(clause);
                }
            }
        }
    }
    if let Some(values) = extras.towgs84.as_ref() {
        parts.push(towgs84_clause(values));
    }
    parts.push("+no_defs".to_owned());
    parts.push("+type=crs".to_owned());
    parts.join(" ")
}

/// `VerticalCRS::_exportToPROJString`: the geoid grid of a bound CRS, then the
/// vertical unit.
fn vertical_proj(crs: &VerticalCRS, extras: &ProjExtras<'_>) -> String {
    let mut parts = Vec::new();
    if let Some(grid) = extras.geoid_grid {
        parts.push(format!("+geoidgrids={grid}"));
        parts.push("+geoid_crs=WGS84".to_owned());
    }
    let unit = crs.coordinate_system().axis().unit();
    parts.push(match proj_unit_name(unit) {
        Some(name) => format!("+vunits={name}"),
        None => format!("+vto_meter={}", format_g(unit.conversion_to_si(), 15)),
    });
    parts.push("+no_defs".to_owned());
    parts.push("+type=crs".to_owned());
    parts.join(" ")
}

/// `BoundCRS::_exportToPROJString`: the base CRS, carrying the geoid grid of a
/// vertical base bound to a WGS 84 hub, or else the `+towgs84` of a Helmert
/// transformation to a WGS 84 hub.
fn export_bound(bound: &BoundCRS) -> Result<String, FactoryError> {
    let hub = bound.target_crs();
    let transformation = match bound.transformation() {
        CoordinateOperation::Transformation(transformation) => Some(transformation),
        _ => None,
    };
    let geoid_grid = match (bound.source_crs(), transformation) {
        (Crs::Vertical(_), Some(transformation)) if hub.name().eq_ignore_ascii_case("WGS 84") => {
            transformation.height_to_geographic_3d_filename()
        }
        _ => None,
    };
    let towgs84 = match transformation {
        Some(transformation) if geoid_grid.is_none() && hub_is_wgs84(hub) => {
            transformation.towgs84_strict()?
        }
        _ => None,
    };
    bound.source_crs().export_with(&ProjExtras {
        geoid_grid,
        towgs84,
    })
}

fn datum_token(crs: &GeographicCRS) -> Option<&'static str> {
    crs.datum()
        .and_then(datum_code_token)
        .or_else(|| crs.datum_ensemble().and_then(ensemble_code_token))
}

fn datum_code_token(datum: &GeodeticReferenceFrame) -> Option<&'static str> {
    if let Some(token) = epsg_code(datum.identifiers()).and_then(proj_datum_name) {
        return Some(token);
    }
    // `addDatumInfoToPROJString` matches EPSG:6326, 6267, and 6269 by
    // equivalence. Without an identifier, the official name and ellipsoid do.
    let ellps = well_known_ellps(
        datum.ellipsoid().semi_major_axis().si_value(),
        (datum.ellipsoid().inverse_flattening() != 0.0)
            .then_some(datum.ellipsoid().inverse_flattening()),
        datum
            .ellipsoid()
            .semi_minor_axis()
            .map(super::model::Measure::si_value),
    )?;
    if datum.prime_meridian().longitude().si_value().abs() >= 1e-12 {
        return None;
    }
    match (datum.name(), ellps) {
        ("World Geodetic System 1984", "WGS84") => Some("WGS84"),
        ("North American Datum 1927", "clrk66") => Some("NAD27"),
        ("North American Datum 1983", "GRS80") => Some("NAD83"),
        _ => None,
    }
}

fn ensemble_code_token(ensemble: &DatumEnsemble) -> Option<&'static str> {
    epsg_code(ensemble.identifiers()).and_then(proj_datum_name)
}

fn proj_datum_name(code: &str) -> Option<&'static str> {
    match code {
        "6326" => Some("WGS84"),
        "6267" => Some("NAD27"),
        "6269" => Some("NAD83"),
        _ => None,
    }
}

fn epsg_code(identifiers: &[Identifier]) -> Option<&str> {
    identifiers
        .iter()
        .find(|identifier| identifier.code_space() == "EPSG")
        .map(Identifier::code)
}

pub(crate) fn ellipsoid_of(crs: &GeographicCRS) -> Option<&Ellipsoid> {
    if let Some(datum) = crs.datum() {
        return Some(datum.ellipsoid());
    }
    geodetic_member(crs.datum_ensemble()?).map(super::model::GeodeticReferenceFrame::ellipsoid)
}

pub(crate) fn meridian_of(crs: &GeographicCRS) -> Option<&PrimeMeridian> {
    if let Some(datum) = crs.datum() {
        return Some(datum.prime_meridian());
    }
    geodetic_member(crs.datum_ensemble()?).map(super::model::GeodeticReferenceFrame::prime_meridian)
}

pub(crate) fn geodetic_member(
    ensemble: &DatumEnsemble,
) -> Option<&super::model::GeodeticReferenceFrame> {
    ensemble.members().iter().find_map(|member| match member {
        Datum::Geodetic(frame) => Some(frame),
        _ => None,
    })
}

fn prime_meridian_proj(meridian: &PrimeMeridian) -> Option<String> {
    prime_meridian_params(meridian).map(|(key, value)| format!("+{key}={value}"))
}

/// `PrimeMeridian::_exportToPROJString`: the well-known name when the longitude
/// is one, otherwise the longitude in degrees. Greenwich writes nothing.
pub(crate) fn prime_meridian_params(meridian: &PrimeMeridian) -> Option<(&'static str, String)> {
    prime_meridian_radians_params(meridian.longitude().si_value())
}

/// [`prime_meridian_params`] for a longitude in radians.
pub(crate) fn prime_meridian_radians_params(radians: f64) -> Option<(&'static str, String)> {
    if radians == 0.0 {
        return None;
    }
    if let Some(name) = well_known_pm_name(radians) {
        return Some(("pm", name.to_owned()));
    }
    Some((
        "pm",
        proj_number(radians / UnitOfMeasure::degree().conversion_to_si()),
    ))
}

/// `PrimeMeridian::getPROJStringWellKnownName`: the name of the prime meridian
/// of `datums.cpp` at `radians`, within `1e-10` radian.
pub(crate) fn well_known_pm_name(radians: f64) -> Option<&'static str> {
    const PRIME_MERIDIANS: &[(&str, f64)] = &[
        ("greenwich", 0.0),
        ("lisbon", -(9.0 + 7.0 / 60.0 + 54.862 / 3600.0)),
        ("paris", 2.0 + 20.0 / 60.0 + 14.025 / 3600.0),
        ("bogota", -(74.0 + 4.0 / 60.0 + 51.3 / 3600.0)),
        ("madrid", -(3.0 + 41.0 / 60.0 + 16.58 / 3600.0)),
        ("rome", 12.0 + 27.0 / 60.0 + 8.4 / 3600.0),
        ("bern", 7.0 + 26.0 / 60.0 + 22.5 / 3600.0),
        ("jakarta", 106.0 + 48.0 / 60.0 + 27.79 / 3600.0),
        ("ferro", -17.0 - 40.0 / 60.0),
        ("brussels", 4.0 + 22.0 / 60.0 + 4.71 / 3600.0),
        ("stockholm", 18.0 + 3.0 / 60.0 + 29.8 / 3600.0),
        ("athens", 23.0 + 42.0 / 60.0 + 58.815 / 3600.0),
        ("oslo", 10.0 + 43.0 / 60.0 + 22.5 / 3600.0),
        ("copenhagen", 12.0 + 34.0 / 60.0 + 40.35 / 3600.0),
    ];
    PRIME_MERIDIANS
        .iter()
        .find(|(_, degrees)| (radians - degrees.to_radians()).abs() < 1e-10)
        .map(|(name, _)| *name)
}

fn ellipsoid_proj(ellipsoid: &Ellipsoid) -> String {
    ellipsoid_params(ellipsoid)
        .iter()
        .map(|(key, value)| format!("+{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `Ellipsoid::_exportToPROJString`: `ellps` for a known ellipsoid, otherwise
/// `R` for a sphere, or `a` with `rf` or `b`.
pub(crate) fn ellipsoid_params(ellipsoid: &Ellipsoid) -> Vec<(&'static str, String)> {
    let metres = ellipsoid.semi_major_axis().si_value();
    let inverse = ellipsoid.inverse_flattening();
    let flattening = (inverse != 0.0).then_some(inverse);
    let minor = ellipsoid
        .semi_minor_axis()
        .map(super::model::Measure::si_value);
    if let Some(name) = well_known_ellps(metres, flattening, minor) {
        return vec![("ellps", name.to_owned())];
    }
    if ellipsoid.is_sphere() {
        vec![("R", proj_number(metres))]
    } else if let (Some(inverse), Some(_)) = (flattening, ellipsoid.declared_inverse_flattening()) {
        vec![("a", proj_number(metres)), ("rf", proj_number(inverse))]
    } else if let Some(minor) = minor {
        vec![("a", proj_number(metres)), ("b", proj_number(minor))]
    } else {
        vec![("R", proj_number(metres))]
    }
}

/// Linear or angular PROJ unit name of `unit`, or `None` when PROJ has no name for it.
pub(crate) fn proj_unit_name(unit: &UnitOfMeasure) -> Option<&'static str> {
    match unit.kind() {
        UnitKind::Linear => linear_proj_unit(unit.conversion_to_si()),
        UnitKind::Angular => angular_proj_unit(unit.conversion_to_si()),
        _ => None,
    }
}

/// `Ellipsoid::lookForProjWellKnownEllps`: the first named ellipsoid of PROJ's
/// table with the same semi-major axis and the same inverse flattening or minor
/// axis. A missing one is derived from the other.
fn well_known_ellps(major: f64, inverse: Option<f64>, minor: Option<f64>) -> Option<&'static str> {
    let inverse = inverse.filter(|value| *value != 0.0);
    let minor = minor.or_else(|| inverse.map(|value| major * (1.0 - 1.0 / value)));
    let inverse = inverse.or_else(|| minor.filter(|b| *b != major).map(|b| major / (major - b)));
    PJ_ELLPS
        .iter()
        .find(|(_, table_major, second, is_inverse)| {
            if (major - table_major).abs() >= 1e-10 * table_major.abs() {
                return false;
            }
            if *is_inverse {
                inverse.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            } else {
                minor.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            }
        })
        .map(|(name, _, _, _)| *name)
}

/// `PROJStringParser::createFromPROJString` for a single `+proj=longlat` CRS.
///
/// The CRS name is `unknown`. `+datum=WGS84`, `NAD83`, and `NAD27` use the
/// built-in frames. `+ellps=GRS80` or `WGS84` builds an `Unknown based on`
/// datum. `+a` and `+rf` build an unnamed ellipsoid. The coordinate system is
/// implicit, so identification accepts a latitude/longitude swap. The default
/// axis order is longitude then latitude; `+axis=` replaces the first two
/// letters.
///
/// # Errors
///
/// The string is not one longlat step, a datum or ellipsoid is unknown, or a
/// number does not parse.
pub fn create_crs_from_proj_string(definition: &str) -> Result<Crs, FactoryError> {
    let tokens = proj_tokens(definition);
    let proj = token(&tokens, "proj");
    if !matches!(proj, "longlat" | "lonlat" | "latlong") {
        return projected_from_proj_string(&tokens);
    }
    let datum = datum_from_proj(&tokens)?;
    let axis = token(&tokens, "axis");
    let axes = if axis.is_empty() {
        vec![
            geographic_axis('e', UnitOfMeasure::degree())?,
            geographic_axis('n', UnitOfMeasure::degree())?,
        ]
    } else if axis.len() == 3 {
        let mut letters = axis.chars();
        let first = letters.next().unwrap_or('e');
        let second = letters.next().unwrap_or('n');
        vec![
            geographic_axis(first, UnitOfMeasure::degree())?,
            geographic_axis(second, UnitOfMeasure::degree())?,
        ]
    } else {
        return Err(FactoryError::Invalid(format!("Unhandled axis={axis}")));
    };
    Ok(Crs::Geographic(GeographicCRS {
        identified: Identified::named("unknown"),
        datum: Some(datum),
        ensemble: None,
        coordinate_system: EllipsoidalCS {
            identified: Identified::default(),
            axes,
        },
        implicit_cs: true,
    }))
}

fn proj_tokens(definition: &str) -> Vec<(String, String)> {
    definition
        .split_whitespace()
        .filter_map(|raw| {
            let raw = raw.trim_start_matches('+');
            if raw.is_empty() {
                return None;
            }
            let (key, value) = raw.split_once('=').unwrap_or((raw, ""));
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn token<'a>(tokens: &'a [(String, String)], key: &str) -> &'a str {
    tokens
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
        .unwrap_or("")
}

fn datum_from_proj(tokens: &[(String, String)]) -> Result<GeodeticReferenceFrame, FactoryError> {
    let datum = token(tokens, "datum");
    if !datum.is_empty() {
        return match datum {
            "WGS84" => Ok(GeodeticReferenceFrame::epsg_6326()),
            "NAD83" => Ok(GeodeticReferenceFrame::epsg_6269()),
            "NAD27" => Ok(GeodeticReferenceFrame::epsg_6267()),
            other => Err(FactoryError::Invalid(format!("unknown datum {other}"))),
        };
    }
    let ellps = token(tokens, "ellps");
    if ellps == "WGS84" {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on WGS 84 ellipsoid",
            Ellipsoid::wgs84(),
            PrimeMeridian::greenwich(),
        ));
    }
    if ellps == "GRS80" {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on GRS 1980 ellipsoid",
            Ellipsoid::grs80(),
            PrimeMeridian::greenwich(),
        ));
    }
    let major = token(tokens, "a");
    let flattening = token(tokens, "rf");
    if !major.is_empty() && !flattening.is_empty() {
        let major = major
            .parse::<f64>()
            .map_err(|_| FactoryError::Invalid("Invalid a value".to_owned()))?;
        let flattening = flattening
            .parse::<f64>()
            .map_err(|_| FactoryError::Invalid("Invalid rf value".to_owned()))?;
        return Ok(GeodeticReferenceFrame::create(
            "unknown",
            Ellipsoid::create_flattened_sphere("unknown", major, flattening),
            PrimeMeridian::greenwich(),
        ));
    }
    Err(FactoryError::Invalid(
        "a longlat CRS needs +datum, +ellps, or +a and +rf".to_owned(),
    ))
}

fn geographic_axis(letter: char, unit: UnitOfMeasure) -> Result<Axis, FactoryError> {
    match letter {
        'n' => Ok(Axis::new("Latitude", "lat", "north", unit)),
        's' => Ok(Axis::new("Latitude", "lat", "south", unit)),
        'e' => Ok(Axis::new("Longitude", "lon", "east", unit)),
        'w' => Ok(Axis::new("Longitude", "lon", "west", unit)),
        _ => Err(FactoryError::Invalid(format!(
            "Unhandled axis letter {letter}"
        ))),
    }
}

/// Keys of a PROJ string that name the datum, the units, or the output, not
/// a parameter of a projection method.
const NON_METHOD_KEYS: &[&str] = &[
    "proj", "datum", "ellps", "a", "b", "rf", "f", "R", "towgs84", "units", "no_defs", "type",
    "axis",
];

/// `PROJStringParser` for a projected CRS with `+proj=` naming a method. The
/// method is the first one whose PROJ parameters cover every key; each of its
/// parameters takes the key's value, or PROJ's default (1 for a scale, 0 for
/// an angle or length). The CRS is `unknown`, on the `+datum`/`+ellps`
/// geographic CRS, with easting and northing in metres.
fn projected_from_proj_string(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    let proj = token(tokens, "proj");
    let units = token(tokens, "units");
    if !units.is_empty() && units != "m" {
        return Err(FactoryError::Invalid(format!("unsupported +units={units}")));
    }
    let methods: Vec<&MethodMap> = PROJECTION_METHODS
        .iter()
        .chain(OTHER_METHODS)
        .filter(|method| method.proj_name == Some(proj))
        .collect();
    let covers = |method: &MethodMap| {
        tokens.iter().all(|(key, _)| {
            NON_METHOD_KEYS.contains(&key.as_str())
                || method
                    .params
                    .iter()
                    .any(|param| param.proj_name == Some(key.as_str()))
                || (key == "k_0" && method.params.iter().any(|p| p.proj_name == Some("k")))
        })
    };
    let Some(method) = methods.iter().copied().find(|method| covers(method)) else {
        return Err(FactoryError::Invalid(format!(
            "unsupported +proj={proj} for a CRS"
        )));
    };
    let mut parameters = Vec::new();
    for param in method.params {
        let key = param.proj_name.unwrap_or("");
        let given = tokens
            .iter()
            .find(|(name, _)| name == key || (key == "k" && name == "k_0"))
            .map(|(_, value)| value.as_str());
        let default = if param.unit == UnitKind::Scale {
            1.0
        } else {
            0.0
        };
        let value = match given {
            Some(text) => text
                .parse::<f64>()
                .map_err(|_| FactoryError::Invalid(format!("+{key} is not a number: {text}")))?,
            None => default,
        };
        let value = match param.unit {
            UnitKind::Angular => ParameterValue::angle(value),
            UnitKind::Scale => ParameterValue::scale(value),
            _ => ParameterValue::length(value),
        };
        parameters.push(Conversion::parameter(
            param.wkt2_name,
            "EPSG",
            &param.epsg_code.to_string(),
            value,
        ));
    }
    let mut conversion = Conversion::create_with_method(
        method.wkt2_name,
        "EPSG",
        &method.epsg_code.to_string(),
        parameters,
    );
    conversion.identified = Identified::named("unknown");
    let datum = datum_from_proj(tokens)?;
    let base = GeographicCRS {
        identified: Identified::named("unknown"),
        datum: Some(datum),
        ensemble: None,
        coordinate_system: EllipsoidalCS {
            identified: Identified::default(),
            axes: vec![
                geographic_axis('e', UnitOfMeasure::degree())?,
                geographic_axis('n', UnitOfMeasure::degree())?,
            ],
        },
        implicit_cs: true,
    };
    Ok(Crs::Projected(ProjectedCRS::create(
        "unknown",
        GeodeticCRS::Geographic(base),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )))
}
