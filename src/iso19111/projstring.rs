//! PROJ strings for the geographic CRS the unit tests export.
//!
//! Datum codes 6326, 6267, and 6269 become `+datum=`. Anything else is
//! `+ellps` plus `+pm` when the prime meridian is not Greenwich.

use super::bound::hub_is_wgs84;
use super::model::{
    Axis, BoundCRS, CartesianCS, Conversion, CoordinateOperation, Crs, Datum, DatumEnsemble,
    Ellipsoid, EllipsoidalCS, FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Identified, Identifier, Measure, ParameterValue, PrimeMeridian, ProjectedCRS,
    UnitKind, UnitOfMeasure, VerticalCRS,
};
use super::parammap::{MethodMap, ParamMap, OTHER_METHODS, PROJECTION_METHODS};
use super::pipeline::proj_number;
use super::wkt::format_g;
use super::wkt_import::{BODY_RELATIVE, EARTH_RADIUS};
use crate::ellipsoid::PJ_ELLPS;
use crate::projections::{angular_proj_unit, linear_proj_unit};

/// PROJ's `proj_list_prime_meridians`, in degrees, for `+pm=NAME`.
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

/// One row of PROJ's `datumDescs`, for `+datum=NAME`. The `gcsName` and
/// `gcsCode` columns are not used by the PROJ string parser.
struct DatumDesc {
    proj_name: &'static str,
    datum_name: &'static str,
    datum_code: &'static str,
    ellipsoid_name: &'static str,
    ellipsoid_code: &'static str,
    a: f64,
    inverse_flattening: f64,
}

const DATUM_DESCS: &[DatumDesc] = &[
    DatumDesc {
        proj_name: "GGRS87",
        datum_name: "Greek Geodetic Reference System 1987",
        datum_code: "6121",
        ellipsoid_name: "GRS 1980",
        ellipsoid_code: "7019",
        a: 6_378_137.0,
        inverse_flattening: 298.257_222_101,
    },
    DatumDesc {
        proj_name: "potsdam",
        datum_name: "Deutsches Hauptdreiecksnetz",
        datum_code: "6314",
        ellipsoid_name: "Bessel 1841",
        ellipsoid_code: "7004",
        a: 6_377_397.155,
        inverse_flattening: 299.152_812_8,
    },
    DatumDesc {
        proj_name: "carthage",
        datum_name: "Carthage",
        datum_code: "6223",
        ellipsoid_name: "Clarke 1880 (IGN)",
        ellipsoid_code: "7011",
        a: 6_378_249.2,
        inverse_flattening: 293.466_021_3,
    },
    DatumDesc {
        proj_name: "hermannskogel",
        datum_name: "Militar-Geographische Institut",
        datum_code: "6312",
        ellipsoid_name: "Bessel 1841",
        ellipsoid_code: "7004",
        a: 6_377_397.155,
        inverse_flattening: 299.152_812_8,
    },
    DatumDesc {
        proj_name: "ire65",
        datum_name: "TM65",
        datum_code: "6299",
        ellipsoid_name: "Airy Modified 1849",
        ellipsoid_code: "7002",
        a: 6_377_340.189,
        inverse_flattening: 299.324_964_6,
    },
    DatumDesc {
        proj_name: "nzgd49",
        datum_name: "New Zealand Geodetic Datum 1949",
        datum_code: "6272",
        ellipsoid_name: "International 1924",
        ellipsoid_code: "7022",
        a: 6_378_388.0,
        inverse_flattening: 297.0,
    },
    DatumDesc {
        proj_name: "OSGB36",
        datum_name: "OSGB 1936",
        datum_code: "6277",
        ellipsoid_name: "Airy 1830",
        ellipsoid_code: "7001",
        a: 6_377_563.396,
        inverse_flattening: 299.324_964_6,
    },
];

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
    well_known_ellps_entry(major, inverse, minor).map(|(id, ..)| *id)
}

/// [`well_known_ellps`] with the whole `proj_ellps` row, so the name is available.
fn well_known_ellps_entry(
    major: f64,
    inverse: Option<f64>,
    minor: Option<f64>,
) -> Option<&'static (&'static str, &'static str, f64, f64, bool)> {
    let inverse = inverse.filter(|value| *value != 0.0);
    let minor = minor.or_else(|| inverse.map(|value| major * (1.0 - 1.0 / value)));
    let inverse = inverse.or_else(|| minor.filter(|b| *b != major).map(|b| major / (major - b)));
    PJ_ELLPS
        .iter()
        .find(|(_, _, table_major, second, is_inverse)| {
            if (major - table_major).abs() >= 1e-10 * table_major.abs() {
                return false;
            }
            if *is_inverse {
                inverse.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            } else {
                minor.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            }
        })
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
        .split('+')
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(|raw| {
            let (key, value) = raw.split_once('=').unwrap_or((raw, ""));
            (key.trim().to_owned(), value.trim().to_owned())
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

/// `PROJStringParser::Private::buildDatum`. `+R` takes precedence. Then
/// `+datum=` or `+ellps=` gives a base datum, and `+a`, `+b`, `+rf`, `+f`, `+e`,
/// and `+es` override its numbers. With no base, the datum is WGS 84.
fn datum_from_proj(tokens: &[(String, String)]) -> Result<GeodeticReferenceFrame, FactoryError> {
    let meridian = prime_meridian_from_proj(tokens)?;
    let radius = token(tokens, "R");
    if !radius.is_empty() {
        let radius = parse_proj_number(radius, "Invalid R value")?;
        return Ok(frame_from_ellipsoid(sphere("unknown", radius), &meridian));
    }
    let datum = token(tokens, "datum");
    let ellps = token(tokens, "ellps");
    let a_text = token(tokens, "a");
    let b_text = token(tokens, "b");
    let rf_text = token(tokens, "rf");
    let f_text = token(tokens, "f");
    let e_text = token(tokens, "e");
    let es_text = token(tokens, "es");
    let numeric = [a_text, b_text, rf_text, f_text, e_text, es_text]
        .iter()
        .any(|text| !text.is_empty());

    let mut a = -1.0;
    let mut b = -1.0;
    let mut rf = -1.0;
    if !datum.is_empty() || !ellps.is_empty() {
        let base = if datum.is_empty() {
            named_ellipsoid_datum(ellps, &meridian)?
        } else {
            named_datum(datum, &meridian)?
        };
        if !numeric {
            return Ok(base);
        }
        let base_ellipsoid = base.ellipsoid();
        a = base_ellipsoid.semi_major_axis().si_value();
        if !datum.is_empty() {
            rf = base_ellipsoid.inverse_flattening();
        } else if let Some(minor) = base_ellipsoid.semi_minor_axis() {
            b = minor.si_value();
        } else {
            rf = base_ellipsoid.inverse_flattening();
        }
    }
    if !a_text.is_empty() {
        a = parse_proj_number(a_text, "Invalid a value")?;
    }

    if a > 0.0 && (b > 0.0 || !b_text.is_empty()) {
        if !b_text.is_empty() {
            b = parse_proj_number(b_text, "Invalid b value")?;
        }
        return Ok(frame_from_ellipsoid(
            identified_ellipsoid(a, Shape::TwoAxis(b)),
            &meridian,
        ));
    }
    if a > 0.0 && (rf >= 0.0 || !rf_text.is_empty()) {
        if !rf_text.is_empty() {
            rf = parse_proj_number(rf_text, "Invalid rf value")?;
        }
        return Ok(frame_from_ellipsoid(
            identified_ellipsoid(a, Shape::Flattened(rf)),
            &meridian,
        ));
    }
    if a > 0.0 && !f_text.is_empty() {
        let f = parse_proj_number(f_text, "Invalid f value")?;
        return Ok(frame_from_ellipsoid(
            identified_ellipsoid(a, Shape::Flattened(inverse_flattening_of(f))),
            &meridian,
        ));
    }
    if a > 0.0 && !e_text.is_empty() {
        let e = parse_proj_number(e_text, "Invalid e value")?;
        let f = 1.0 - e.asin().cos();
        return Ok(frame_from_ellipsoid(
            identified_ellipsoid(a, Shape::Flattened(inverse_flattening_of(f))),
            &meridian,
        ));
    }
    if a > 0.0 && !es_text.is_empty() {
        let es = parse_proj_number(es_text, "Invalid es value")?;
        let f = 1.0 - (1.0 - es).sqrt();
        return Ok(frame_from_ellipsoid(
            identified_ellipsoid(a, Shape::Flattened(inverse_flattening_of(f))),
            &meridian,
        ));
    }
    if a > 0.0 && b_text.is_empty() && rf_text.is_empty() && e_text.is_empty() && es_text.is_empty()
    {
        return Ok(frame_from_ellipsoid(sphere("unknown", a), &meridian));
    }
    for (text, key) in [
        (b_text, "b"),
        (rf_text, "rf"),
        (f_text, "f"),
        (e_text, "e"),
        (es_text, "es"),
    ] {
        if !text.is_empty() && a_text.is_empty() {
            return Err(FactoryError::Invalid(format!("{key} found, but a missing")));
        }
    }
    Ok(override_meridian(
        GeodeticReferenceFrame::epsg_6326(),
        &meridian,
    ))
}

fn parse_proj_number(text: &str, message: &str) -> Result<f64, FactoryError> {
    text.parse::<f64>()
        .map_err(|_| FactoryError::Invalid(message.to_owned()))
}

/// `+f` and `+e`/`+es` give a flattening. A zero flattening is a sphere, which
/// PROJ stores with an inverse flattening of zero.
fn inverse_flattening_of(flattening: f64) -> f64 {
    if flattening == 0.0 {
        0.0
    } else {
        1.0 / flattening
    }
}

/// `buildPrimeMeridian`. `None` is Greenwich, the default.
fn prime_meridian_from_proj(
    tokens: &[(String, String)],
) -> Result<Option<PrimeMeridian>, FactoryError> {
    let text = token(tokens, "pm");
    if text.is_empty() {
        return Ok(None);
    }
    if let Ok(degrees) = text.parse::<f64>() {
        return Ok(Some(prime_meridian_named("unknown", degrees)));
    }
    if text == "paris" {
        return Ok(Some(PrimeMeridian::paris()));
    }
    let (_, degrees) = PRIME_MERIDIANS
        .iter()
        .find(|(id, _)| *id == text)
        .ok_or_else(|| FactoryError::Invalid(format!("unknown pm {text}")))?;
    let mut name = text.to_owned();
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    Ok(Some(prime_meridian_named(&name, *degrees)))
}

fn prime_meridian_named(name: &str, degrees: f64) -> PrimeMeridian {
    PrimeMeridian {
        identified: Identified::named(name),
        longitude: Measure {
            value: degrees,
            unit: UnitOfMeasure::degree(),
        },
    }
}

fn meridian_or_greenwich(meridian: &Option<PrimeMeridian>) -> PrimeMeridian {
    meridian.clone().unwrap_or_else(PrimeMeridian::greenwich)
}

/// `fixupPrimeMeridan`: a Greenwich meridian on a body other than Earth is the
/// reference meridian.
fn meridian_for(ellipsoid: &Ellipsoid, meridian: &Option<PrimeMeridian>) -> PrimeMeridian {
    match meridian {
        Some(meridian) => meridian.clone(),
        None if ellipsoid.celestial_body() != "Earth" => {
            prime_meridian_named("Reference meridian", 0.0)
        }
        None => PrimeMeridian::greenwich(),
    }
}

/// `overridePmIfNeeded`: a datum named by its code keeps its ellipsoid, and a
/// given prime meridian replaces its own. The datum is renamed after the
/// ellipsoid.
fn override_meridian(
    frame: GeodeticReferenceFrame,
    meridian: &Option<PrimeMeridian>,
) -> GeodeticReferenceFrame {
    match meridian {
        None => frame,
        Some(meridian) => {
            let ellipsoid = frame.ellipsoid().clone();
            GeodeticReferenceFrame::create(
                format!("Unknown based on {} ellipsoid", ellipsoid.name()),
                ellipsoid,
                meridian.clone(),
            )
        }
    }
}

/// `createGRF`: an ellipsoid with the name `unknown` gives an `unknown` datum.
fn frame_from_ellipsoid(
    ellipsoid: Ellipsoid,
    meridian: &Option<PrimeMeridian>,
) -> GeodeticReferenceFrame {
    let name = if ellipsoid.name() == "unknown" {
        "unknown".to_owned()
    } else {
        format!("Unknown based on {} ellipsoid", ellipsoid.name())
    };
    let meridian = meridian_for(&ellipsoid, meridian);
    GeodeticReferenceFrame::create(name, ellipsoid, meridian)
}

/// `Ellipsoid::guessBodyName` without a database: Earth when `a` is close to
/// the mean Earth radius, otherwise a non-Earth body.
fn guess_body(semi_major: f64) -> String {
    if (semi_major - EARTH_RADIUS).abs() < BODY_RELATIVE * EARTH_RADIUS {
        "Earth".to_owned()
    } else {
        "Non-Earth body".to_owned()
    }
}

/// A sphere of radius `radius`, on the body `guess_body` names.
fn sphere(name: &str, radius: f64) -> Ellipsoid {
    let mut ellipsoid = Ellipsoid::create_flattened_sphere(name, radius, 0.0);
    ellipsoid.body = guess_body(radius);
    ellipsoid
}

/// The shape of an ellipsoid given by two numbers after its semi-major axis.
#[derive(Clone, Copy)]
enum Shape {
    /// Inverse flattening.
    Flattened(f64),
    /// Semi-minor axis.
    TwoAxis(f64),
}

/// `->identify()` on a two-axis or flattened ellipsoid. A match in `proj_ellps`
/// takes its name and values. Otherwise the ellipsoid stays `unknown` on the
/// body `guess_body` names.
fn identified_ellipsoid(major: f64, shape: Shape) -> Ellipsoid {
    let entry = match shape {
        Shape::Flattened(inverse) => well_known_ellps_entry(major, Some(inverse), None),
        Shape::TwoAxis(minor) => well_known_ellps_entry(major, None, Some(minor)),
    };
    let mut ellipsoid = match (entry, shape) {
        (Some((_, name, table_major, second, true)), _) => {
            Ellipsoid::create_flattened_sphere(*name, *table_major, *second)
        }
        (Some((_, name, table_major, second, false)), _) => {
            Ellipsoid::create_two_axis(*name, *table_major, *second)
        }
        (None, Shape::Flattened(inverse)) => {
            Ellipsoid::create_flattened_sphere("unknown", major, inverse)
        }
        (None, Shape::TwoAxis(minor)) => Ellipsoid::create_two_axis("unknown", major, minor),
    };
    if entry.is_none() {
        ellipsoid.body = guess_body(major);
    }
    ellipsoid
}

/// `buildDatum` for `+datum=`. `WGS84`, `NAD83`, and `NAD27` are the EPSG frames.
/// Other names come from PROJ's `datumDescs` table, with their EPSG codes.
fn named_datum(
    datum: &str,
    meridian: &Option<PrimeMeridian>,
) -> Result<GeodeticReferenceFrame, FactoryError> {
    let frame = match datum {
        "WGS84" => GeodeticReferenceFrame::epsg_6326(),
        "NAD83" => GeodeticReferenceFrame::epsg_6269(),
        "NAD27" => GeodeticReferenceFrame::epsg_6267(),
        other => {
            let desc = DATUM_DESCS
                .iter()
                .find(|desc| desc.proj_name == other)
                .ok_or_else(|| FactoryError::Invalid(format!("unknown datum {other}")))?;
            let mut ellipsoid = Ellipsoid::create_flattened_sphere(
                desc.ellipsoid_name,
                desc.a,
                desc.inverse_flattening,
            );
            ellipsoid.identified =
                Identified::named(desc.ellipsoid_name).with_id("EPSG", desc.ellipsoid_code);
            let mut frame = GeodeticReferenceFrame::create(
                desc.datum_name,
                ellipsoid,
                meridian_or_greenwich(meridian),
            );
            frame.identified = Identified::named(desc.datum_name).with_id("EPSG", desc.datum_code);
            return Ok(frame);
        }
    };
    Ok(override_meridian(frame, meridian))
}

/// `buildDatum` for `+ellps=`. `WGS84` and `GRS80` are named ellipsoids. Other
/// names come from PROJ's `proj_ellps` table.
fn named_ellipsoid_datum(
    ellps: &str,
    meridian: &Option<PrimeMeridian>,
) -> Result<GeodeticReferenceFrame, FactoryError> {
    let (name, ellipsoid) = match ellps {
        "WGS84" => ("WGS 84", Ellipsoid::wgs84()),
        "GRS80" => ("GRS 1980", Ellipsoid::grs80()),
        _ => {
            let (_, name, major, second, is_inverse) = PJ_ELLPS
                .iter()
                .find(|(id, ..)| *id == ellps)
                .ok_or_else(|| FactoryError::Invalid(format!("unknown ellipsoid {ellps}")))?;
            let ellipsoid = if *is_inverse {
                Ellipsoid::create_flattened_sphere(*name, *major, *second)
            } else {
                Ellipsoid::create_two_axis(*name, *major, *second)
            };
            (*name, ellipsoid)
        }
    };
    Ok(GeodeticReferenceFrame::create(
        format!("Unknown based on {name} ellipsoid"),
        ellipsoid,
        meridian_or_greenwich(meridian),
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

/// Keys of a PROJ string that name the datum, the units, the title, or the
/// output, not a parameter of a projection method.
const NON_METHOD_KEYS: &[&str] = &[
    "proj", "datum", "ellps", "a", "b", "rf", "f", "R", "towgs84", "units", "no_defs", "type",
    "axis", "title",
];

/// `+k` and `+k_0` name the same parameter.
fn same_param(given: &str, proj_name: &str) -> bool {
    given == proj_name || (is_scale_key(given) && is_scale_key(proj_name))
}

fn is_scale_key(key: &str) -> bool {
    key == "k" || key == "k_0"
}

/// The value of the first token that names the PROJ parameter `key`.
fn param_token<'a>(tokens: &'a [(String, String)], key: &str) -> Option<&'a str> {
    tokens
        .iter()
        .find(|(name, _)| same_param(name, key))
        .map(|(_, value)| value.as_str())
}

/// The EPSG method that `buildProjectedCRS` forces when the PROJ name alone
/// does not decide it. `None` keeps the first method whose parameters cover
/// every key. Pseudo-Mercator, the no-rotation and two-point variants of
/// `omerc`, and non-polar `stere` are not decided here.
fn forced_method_code(proj: &str, tokens: &[(String, String)]) -> Option<i32> {
    let present = |key: &str| param_token(tokens, key).is_some();
    let number = |key: &str| param_token(tokens, key).and_then(|text| text.parse::<f64>().ok());
    let k = number("k");
    match proj {
        "lcc" if !present("lat_2") && present("lat_0") && present("lat_1") => {
            Some(if number("lat_0") == number("lat_1") {
                9801
            } else {
                1102
            })
        }
        "lcc" if k.is_some_and(|k| k != 1.0) => Some(1051),
        "lcc" => Some(9802),
        "omerc" if present("no_rot") => None,
        "omerc" if present("no_uoff") || present("no_off") => Some(9812),
        "omerc" if present("lat_1") && present("lon_1") && present("lat_2") && present("lon_2") => {
            None
        }
        "omerc" => Some(9815),
        "merc" if present("lat_ts") => Some(9805),
        "merc" => Some(9804),
        "stere" => {
            let polar = number("lat_0").is_some_and(|lat| (lat.abs() - 90.0).abs() < 1e-10);
            if !polar {
                return None;
            }
            if present("lat_ts") && k.is_none_or(|k| (k - 1.0).abs() < 1e-10) {
                Some(9829)
            } else {
                Some(9810)
            }
        }
        "krovak" | "mod_krovak" => {
            let swu = param_token(tokens, "axis") == Some("swu");
            let czech = present("czech");
            ((swu && !czech) || (czech && !present("axis"))).then_some(if proj == "krovak" {
                9819
            } else {
                1042
            })
        }
        _ => None,
    }
}

/// The value PROJ uses when a parameter is not given: 1 for a scale, 0 for an
/// angle or length. `krovak` and `omerc` have their own defaults.
fn parameter_default(proj: &str, param: &ParamMap, tokens: &[(String, String)]) -> f64 {
    if proj == "omerc" && param.proj_name == Some("gamma") {
        return param_token(tokens, "alpha")
            .and_then(|text| text.parse().ok())
            .unwrap_or(0.0);
    }
    if matches!(proj, "krovak" | "mod_krovak") {
        match param.epsg_code {
            8811 => return 49.5,
            8833 => return 24.833_333_333_333_333,
            1036 => return 30.288_139_752_777_777_76,
            8818 => return 78.5,
            8819 => return 0.9999,
            _ => {}
        }
    }
    if param.unit == UnitKind::Scale {
        1.0
    } else {
        0.0
    }
}

/// `PROJStringParser` for a projected CRS with `+proj=` naming a method. The
/// method is the one `forced_method_code` selects, else the first one whose
/// PROJ parameters cover every key. Each parameter takes the key's value, or
/// `parameter_default`. The CRS is named by `+title=`, or `unknown`, on the
/// `+datum`/`+ellps` geographic CRS, with easting and northing in metres.
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
                    .any(|param| param.proj_name.is_some_and(|name| same_param(key, name)))
        })
    };
    let forced = forced_method_code(proj, tokens).and_then(|code| {
        methods
            .iter()
            .copied()
            .find(|method| method.epsg_code == code)
    });
    let Some(method) = forced.or_else(|| methods.iter().copied().find(|method| covers(method)))
    else {
        return Err(FactoryError::Invalid(format!(
            "unsupported +proj={proj} for a CRS"
        )));
    };
    let mut parameters = Vec::new();
    for param in method.params {
        let key = param.proj_name.unwrap_or("");
        let value = match param_token(tokens, key) {
            Some(text) => text
                .parse::<f64>()
                .map_err(|_| FactoryError::Invalid(format!("+{key} is not a number: {text}")))?,
            None => parameter_default(proj, param, tokens),
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
    let krovak_default = matches!(proj, "krovak" | "mod_krovak")
        && ["datum", "ellps", "a", "rf"]
            .iter()
            .all(|key| token(tokens, key).is_empty());
    let datum = if krovak_default {
        named_ellipsoid_datum("bessel", &None)?
    } else {
        datum_from_proj(tokens)?
    };
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
    let name = param_token(tokens, "title")
        .filter(|title| !title.is_empty())
        .unwrap_or("unknown");
    Ok(Crs::Projected(ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(base),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )))
}
