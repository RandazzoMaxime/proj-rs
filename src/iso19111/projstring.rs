//! PROJ strings for the geographic CRS the unit tests export.
//!
//! Datum codes 6326, 6267, and 6269 become `+datum=`. Anything else is
//! `+ellps` plus `+pm` when the prime meridian is not Greenwich.

use super::bound_export::{export_bound, export_compound};
use super::bound_import;
use super::factory::DatabaseContext;
use super::model::{
    Axis, BoundCRS, CartesianCS, Conversion, CoordinateOperation, CoordinateSystem, Crs, Datum,
    DatumEnsemble, DerivedProjectedCRS, Ellipsoid, EllipsoidalCS, FactoryError, GeocentricCRS,
    GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identified, Identifier, Measure,
    ParameterValue, PrimeMeridian, ProjectedCRS, SphericalCS, SphericalGeodeticCRS, UnitKind,
    UnitOfMeasure, VerticalCRS,
};
use super::parammap::{MethodMap, ParamMap, OTHER_METHODS, PROJECTION_METHODS};
use super::pipeline::proj_number;
use super::proj_crs;
use super::projfam;
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
    /// `+geoidgrids=`, written with `+geoid_crs=`, for a vertical base CRS.
    pub(super) geoid_grid: Option<&'a str>,
    /// The `+geoid_crs` value that goes with `geoid_grid`.
    pub(super) geoid_crs: &'a str,
    /// `+towgs84=`, for a horizontal base CRS bound to WGS 84.
    pub(super) towgs84: Option<[f64; 7]>,
    /// `+nadgrids=`, for a horizontal base CRS bound to WGS 84 by a grid.
    pub(super) nadgrids: Option<&'a str>,
}

impl ProjExtras<'_> {
    /// A datum shift is written, so the datum token gives way to the ellipsoid.
    fn has_shift(&self) -> bool {
        self.towgs84.is_some() || self.nadgrids.is_some()
    }
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
            Self::Geocentric(crs) => crs.export_with(extras),
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

impl DerivedProjectedCRS {
    /// `normalizeForVisualization` when the first axes are north then east.
    #[must_use]
    pub fn normalize_for_visualization(&self) -> Self {
        let axes = self.coordinate_system.axes();
        if axes.len() < 2 || axes[0].direction() != "north" || axes[1].direction() != "east" {
            return self.clone();
        }
        let mut crs = self.clone();
        if let CoordinateSystem::Cartesian(cs) = &mut crs.coordinate_system {
            cs.axes.swap(0, 1);
        }
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

    /// `exportToPROJString` with a database: a `+nadgrids` name that the database renames
    /// (`grid_alternatives`) is written under its PROJ name. Other kinds are as
    /// [`Self::export_to_proj_string`].
    ///
    /// # Errors
    ///
    /// As [`Self::export_to_proj_string`], or the database cannot be read.
    pub fn export_to_proj_string_with_database(
        &self,
        db: &DatabaseContext,
    ) -> Result<String, FactoryError> {
        match self {
            Self::Bound(crs) => export_bound(crs, None, Some(db)),
            Self::Compound(crs) => export_compound(crs, Some(db)),
            other => other.export_to_proj_string(),
        }
    }

    pub(super) fn export_with(&self, extras: &ProjExtras<'_>) -> Result<String, FactoryError> {
        match self {
            Self::Geographic(crs) => Ok(geographic_proj(crs, extras)),
            Self::Geodetic(crs) => crs.export_with(extras),
            Self::Projected(crs) => crs.export_with(extras),
            Self::Vertical(crs) => Ok(vertical_proj(crs, extras)),
            Self::Bound(crs) => export_bound(crs, None, None),
            Self::Compound(crs) => export_compound(crs, None),
            Self::Derived(crs) => derived_geographic_proj(crs),
            _ => Err(FactoryError::Format(
                "this CRS kind has no PROJ string export".to_owned(),
            )),
        }
    }

    /// `normalizeForVisualization` for a geographic, projected, or bound CRS. A bound CRS
    /// normalizes its base and keeps its hub and transformation.
    #[must_use]
    pub fn normalize_for_visualization(&self) -> Self {
        match self {
            Self::Geographic(crs) => Self::Geographic(crs.normalize_for_visualization()),
            Self::Projected(crs) => Self::Projected(crs.normalize_for_visualization()),
            Self::DerivedProjected(crs) => {
                Self::DerivedProjected(crs.normalize_for_visualization())
            }
            Self::Bound(crs) => Self::Bound(BoundCRS::create(
                crs.source_crs().normalize_for_visualization(),
                crs.target_crs().clone(),
                crs.transformation().clone(),
            )),
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
            Self::Temporal(crs) => &crs.identified.remarks,
            Self::Parametric(crs) => &crs.identified.remarks,
            Self::Derived(crs) => &crs.identified.remarks,
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
    identified.identifiers.clear();
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
        if let Some(text) = self.pseudo_mercator_proj().filter(|_| !extras.has_shift()) {
            return Ok(text);
        }
        if let Some(text) = self
            .identified
            .extension_proj4
            .clone()
            .filter(|_| !extras.has_shift())
        {
            // `ingestPROJString` drops `+type=crs` and the CRS formatter puts it back.
            if text.contains("+type=crs") {
                return Ok(text);
            }
            return Ok(format!("{text} +type=crs"));
        }
        let mut text = match self.conversion.identified.proj_method_override.as_deref() {
            Some("tmerc approx" | "utm approx") => self
                .deriving_conversion()
                .export_to_proj_string_approx_tmerc()?,
            _ => self.deriving_conversion().export_to_proj_string()?,
        };
        if self.has_west_south_axes() {
            text = text.replacen("+axis=swu", "+czech", 1);
        }
        text.push(' ');
        text.push_str(&crs_datum_clause(self.base_crs(), extras));
        if let Some(unit) = self.coordinate_system().axes().first().map(Axis::unit) {
            if let Some(clause) = linear_unit_clause(unit) {
                text.push(' ');
                text.push_str(&clause);
            }
        }
        text.push_str(" +no_defs +type=crs");
        Ok(text)
    }

    /// Westing then southing axes, which `+czech` names on a Krovak method.
    fn has_west_south_axes(&self) -> bool {
        matches!(self.deriving_conversion().method().code(), "9819" | "1042")
            && matches!(
                self.coordinate_system().axes(),
                [first, second] if first.direction() == "west" && second.direction() == "south"
            )
    }

    /// The pseudo-Mercator method (EPSG 1024) as PROJ writes it: `+proj=merc` on
    /// a sphere of the base's semi-major axis, with `+lat_ts=0`, `+k=1` and
    /// `+nadgrids=@null`, and no datum. `None` for any other method.
    fn pseudo_mercator_proj(&self) -> Option<String> {
        let conversion = self.deriving_conversion();
        let method = conversion.method();
        let pseudo = method.code_space() == "EPSG" && method.code() == "1024";
        if !pseudo
            && !conversion
                .name()
                .eq_ignore_ascii_case("Popular Visualisation Mercator")
        {
            return None;
        }
        let radius = format_g(
            self.base_crs().ellipsoid()?.semi_major_axis().si_value(),
            15,
        );
        let mut text = format!(
            "+proj=merc +a={radius} +b={radius} +lat_ts=0 +lon_0={} +x_0={} +y_0={} +k=1",
            format_g(conversion.numeric_degrees(8802), 15),
            format_g(conversion.numeric_si(8806), 15),
            format_g(conversion.numeric_si(8807), 15),
        );
        if let Some(axis) = self.coordinate_system().axes().first() {
            if let Some(clause) = linear_unit_clause(axis.unit()) {
                text.push(' ');
                text.push_str(&clause);
            }
        }
        text.push_str(" +nadgrids=@null +wktext +no_defs +type=crs");
        Some(text)
    }

    /// `Conversion::addWKTExtensionNode`: the PROJ.4 string that a WKT1 `EXTENSION` node
    /// carries for a method WKT1 cannot spell, or `None`.
    pub(super) fn wkt1_extension_proj4(&self) -> Option<String> {
        let suffix = " +type=crs";
        if let Some(text) = self.pseudo_mercator_proj() {
            return text.strip_suffix(suffix).map(str::to_owned);
        }
        if self.deriving_conversion().method().name() == "Geostationary Satellite (Sweep X)" {
            let text = self.export_with(&ProjExtras::default()).ok()?;
            return text.strip_suffix(suffix).map(str::to_owned);
        }
        None
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
    let steps = projection_steps(source, target)?;
    // Geographic steps cancel inverse unit conversions and fold axis swaps.
    // A projected or geocentric pipeline is already the text PROJ prints.
    if geographic_of(target).is_some() {
        let mut formatter = super::pipeline::PipelineFormatter::default();
        for step in &steps {
            formatter.ingest_proj_string(step);
        }
        return Ok(formatter.finish(super::pipeline::ProjLines::Single));
    }
    Ok(pipeline(&steps))
}

/// `createOperationsFromProj4Ext`: when a CRS, or the base of a bound CRS, carries a PROJ
/// string extension (`+over`, `+nadgrids`, ...), no registered operation is looked for. The
/// operation is one PROJ-based step: the source written inverted, then the target.
///
/// # Errors
///
/// An end cannot be written as a PROJ string.
pub(crate) fn operation_from_proj_extensions(
    source: &Crs,
    target: &Crs,
) -> Result<Option<CoordinateOperation>, FactoryError> {
    if proj_extension(source).is_none() && proj_extension(target).is_none() {
        return Ok(None);
    }
    let mut formatter = super::pipeline::PipelineFormatter::default();
    formatter.start_inversion();
    write_proj_extension_end(source, &mut formatter)?;
    formatter.stop_inversion();
    write_proj_extension_end(target, &mut formatter)?;
    let text = formatter.finish(super::pipeline::ProjLines::Single);
    let props = super::buildop::OpProps::new().name(format!(
        "Transformation from {} to {}",
        source.name(),
        target.name()
    ));
    let operation = Conversion::create_proj_based(&props, &text)?
        .with_endpoints(source.clone(), target.clone());
    Ok(Some(CoordinateOperation::Conversion(operation)))
}

/// One end of [`operation_from_proj_extensions`]. A plain geographic CRS writes no step of
/// its own: only its angular unit conversion and axis order, as PROJ does.
fn write_proj_extension_end(
    crs: &Crs,
    formatter: &mut super::pipeline::PipelineFormatter,
) -> Result<(), FactoryError> {
    if let Some(text) = proj_extension(crs) {
        formatter.ingest_proj_string(text);
    } else if !matches!(
        crs,
        Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_))
    ) {
        // The CRS-level tokens belong to the top-level export, not to a step.
        let text = crs.export_to_proj_string()?;
        let body = text
            .split_whitespace()
            .filter(|token| !matches!(*token, "+type=crs" | "+units=m" | "+no_defs"))
            .collect::<Vec<_>>()
            .join(" ");
        formatter.ingest_proj_string(&body);
    }
    if let Some(geographic) = geographic_of(base_of(crs)) {
        let mut angular = super::pipeline::PipelineFormatter::default();
        super::transform_export::angular_unit_and_axis_swap(
            geographic.coordinate_system().axes(),
            &mut angular,
        )?;
        formatter.ingest_proj_string(&angular.finish(super::pipeline::ProjLines::Single));
    }
    Ok(())
}

/// The CRS a bound CRS is bound to, or the CRS itself.
fn base_of(crs: &Crs) -> &Crs {
    match crs {
        Crs::Bound(bound) => &bound.source,
        other => other,
    }
}

/// The PROJ string extension of the CRS, or of the base of a bound CRS.
fn proj_extension(crs: &Crs) -> Option<&str> {
    let identified = match base_of(crs) {
        Crs::Geographic(crs) => &crs.identified,
        Crs::Geodetic(GeodeticCRS::Geographic(crs)) => &crs.identified,
        Crs::Geodetic(GeodeticCRS::Geocentric(crs)) => &crs.identified,
        Crs::Projected(crs) => &crs.identified,
        _ => return None,
    };
    identified.extension_proj4.as_deref()
}

/// The steps of [`operation_proj_string`], one PROJ string each, in pipeline order.
///
/// `ConcatenatedOperation` writes a conversion step from these, between its own
/// `startInversion` and `stopInversion` when the step is reversed.
///
/// # Errors
///
/// As [`operation_proj_string`].
pub(crate) fn projection_steps(source: &Crs, target: &Crs) -> Result<Vec<String>, FactoryError> {
    if let Crs::Geodetic(GeodeticCRS::Geocentric(geocentric)) = target {
        return geocentric_steps(source, geocentric);
    }
    if let Some(target_geographic) = geographic_of(target) {
        let Some(source_geographic) = geographic_of(source) else {
            return Err(FactoryError::Format(
                "the source CRS is not geographic".to_owned(),
            ));
        };
        return Ok(geographic_steps(source_geographic, target_geographic));
    }
    let Crs::Projected(projected) = target else {
        return Err(FactoryError::Format(
            "this operation is not a projection of the source CRS".to_owned(),
        ));
    };
    if let Some(steps) = topocentric_steps(source, projected)? {
        return Ok(steps);
    }
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
    let mut steps = Vec::new();
    if let Some(order) = geographic_axis_order(geographic) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    if let Some(step) = angular_unit_step(geographic) {
        steps.push(step);
    }
    if let Some(ob_tran) = ob_tran_geographic_step(projected, geographic, base) {
        steps.push(ob_tran);
        steps.push("+proj=unitconvert +xy_in=rad +xy_out=deg".to_owned());
        steps.push("+proj=axisswap +order=2,1".to_owned());
        return Ok(steps);
    }
    let mut projection = projected.deriving_conversion().export_to_proj_string()?;
    append_ellipsoid_and_meridian(&mut projection, geographic, base);
    steps.push(projection);
    if let Some(step) = projected_unit_step(projected) {
        steps.push(step);
    }
    if let Some(order) = projected_axis_order(projected) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    Ok(steps)
}

/// Axis order and angular unit from `source` into the axes of `target`.
///
/// PROJ prints this with no datum shift: the caller asked for the conversion
/// the projected or geographic definition names, not a transformation.
fn geographic_steps(source: &GeographicCRS, target: &GeographicCRS) -> Vec<String> {
    let mut steps = Vec::new();
    if let Some(order) = geographic_axis_order(source) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    if let Some(step) = angular_to_radians(source) {
        steps.push(step.to_owned());
    }
    if let Some(step) = radians_to_angular(target) {
        steps.push(step.to_owned());
    }
    if let Some(order) = geographic_axis_order(target) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    steps
}

/// `+ellps` and `+pm` of the projected base, appended to a conversion step.
///
/// The source longitude is already relative to its prime meridian, and the
/// engine subtracts `+pm`. The clause is written only when that meridian is
/// not the base meridian (Greenwich input into a Paris projection).
fn append_ellipsoid_and_meridian(
    projection: &mut String,
    source: &GeographicCRS,
    base: &GeographicCRS,
) {
    if let Some(ellipsoid) = base.ellipsoid() {
        projection.push(' ');
        projection.push_str(&ellipsoid_proj(ellipsoid));
    }
    if same_meridian_angle(source, base) {
        return;
    }
    if let Some(clause) = meridian_of(base).and_then(prime_meridian_proj) {
        projection.push(' ');
        projection.push_str(&clause);
    }
}

fn same_meridian_angle(left: &GeographicCRS, right: &GeographicCRS) -> bool {
    let left = meridian_of(left)
        .map(|meridian| meridian.longitude().si_value())
        .unwrap_or(0.0);
    let right = meridian_of(right)
        .map(|meridian| meridian.longitude().si_value())
        .unwrap_or(0.0);
    (left - right).abs() <= 1e-9
}

/// A geographic `ob_tran` step: the stored definition, then the base ellipsoid.
fn ob_tran_geographic_step(
    projected: &ProjectedCRS,
    source: &GeographicCRS,
    base: &GeographicCRS,
) -> Option<String> {
    let name = projected.deriving_conversion().method().name();
    const PREFIXES: &[&str] = &[
        "PROJ ob_tran o_proj=longlat",
        "PROJ ob_tran o_proj=lonlat",
        "PROJ ob_tran o_proj=latlong",
        "PROJ ob_tran o_proj=latlon",
    ];
    if !PREFIXES.iter().any(|prefix| name.starts_with(prefix)) {
        return None;
    }
    let mut text = projected.conversion.identified.extension_proj4.clone()?;
    append_ellipsoid_and_meridian(&mut text, source, base);
    Some(text)
}

/// The geocentric/topocentric (EPSG:9836) and geographic/topocentric (EPSG:9837)
/// conversions of a projected CRS, as `Conversion::_exportToPROJString` writes them:
/// `+proj=topocentric` with the base ellipsoid, after `+proj=cart` for a geographic source.
/// `None` for any other method.
fn topocentric_steps(
    source: &Crs,
    projected: &ProjectedCRS,
) -> Result<Option<Vec<String>>, FactoryError> {
    let conversion = projected.deriving_conversion();
    let method = conversion.method();
    let code = if method.code_space() == "EPSG" {
        method.code()
    } else {
        ""
    };
    let ellipsoid = projected.base_crs().ellipsoid().map(ellipsoid_proj);
    let mut topocentric = String::from("+proj=topocentric");
    let mut steps = Vec::new();
    match code {
        "9836" => {
            topocentric = conversion.export_to_proj_string()?;
        }
        "9837" => {
            let Some(geographic) = geographic_of(source) else {
                return Err(FactoryError::Format(
                    "Export of Geographic/Topocentric conversion to a PROJ string requires an \
                     input geographic CRS"
                        .to_owned(),
                ));
            };
            if let Some(order) = geographic_axis_order(geographic) {
                steps.push(format!("+proj=axisswap +order={order}"));
            }
            let degrees = geographic
                .coordinate_system()
                .axes()
                .first()
                .is_some_and(|axis| axis.unit().name() == "degree");
            if degrees {
                let height = if geographic.coordinate_system().axes().len() == 3 {
                    " +z_in=m"
                } else {
                    ""
                };
                let height_out = if height.is_empty() { "" } else { " +z_out=m" };
                steps.push(format!(
                    "+proj=unitconvert +xy_in=deg{height} +xy_out=rad{height_out}"
                ));
            }
            let mut cart = String::from("+proj=cart");
            if let Some(ellipsoid) = &ellipsoid {
                cart.push(' ');
                cart.push_str(ellipsoid);
            }
            steps.push(cart);
            for (key, parameter, unit) in [
                ("lat_0", 8834, UnitOfMeasure::degree()),
                ("lon_0", 8835, UnitOfMeasure::degree()),
                ("h_0", 8836, UnitOfMeasure::metre()),
            ] {
                let value = conversion
                    .measure_of_code(parameter)
                    .map_or(0.0, |measure| measure.convert_to(&unit));
                topocentric.push_str(&format!(" +{key}={}", proj_number(value)));
            }
        }
        _ => return Ok(None),
    }
    if let Some(ellipsoid) = ellipsoid {
        topocentric.push(' ');
        topocentric.push_str(&ellipsoid);
    }
    steps.push(topocentric);
    Ok(Some(steps))
}

/// `addUnitConvertAndAxisSwap` for the unit: the metre that the projection writes becomes
/// the linear unit of the projected CRS, such as the US survey foot. A third axis is
/// included when it is not the metre, even if easting and northing already are.
/// `None` when every axis is the metre.
fn projected_unit_step(crs: &ProjectedCRS) -> Option<String> {
    let axes = crs.coordinate_system().axes();
    let unit = axes.first()?.unit();
    let height = axes.get(2).map(Axis::unit);
    let metric = |unit: &UnitOfMeasure| {
        unit.kind() == UnitKind::Linear && (unit.conversion_to_si() - 1.0).abs() <= 1e-10
    };
    if metric(unit) && height.is_none_or(metric) {
        return None;
    }
    let xy_out = linear_out_name(unit);
    match height {
        Some(height) => Some(format!(
            "+proj=unitconvert +xy_in=m +z_in=m +xy_out={xy_out} +z_out={}",
            linear_out_name(height)
        )),
        None => Some(format!("+proj=unitconvert +xy_in=m +xy_out={xy_out}")),
    }
}

/// The PROJ name of a linear unit, or its factor when PROJ has no name for it.
fn linear_out_name(unit: &UnitOfMeasure) -> String {
    linear_proj_unit(unit.conversion_to_si())
        .map(str::to_owned)
        .unwrap_or_else(|| super::wkt::format_g(unit.conversion_to_si(), 15))
}

/// The `unitconvert` step that turns the geographic axes into radians, or
/// `None` when they already are. A third axis is converted from its unit into
/// metres, which is the height the projection consumes.
fn angular_unit_step(geographic: &GeographicCRS) -> Option<String> {
    let axes = geographic.coordinate_system().axes();
    let xy_in = match axes.first()?.unit().name() {
        "degree" => "deg",
        "grad" => "grad",
        _ => return None,
    };
    match axes.get(2) {
        Some(height) => Some(format!(
            "+proj=unitconvert +xy_in={xy_in} +z_in={} +xy_out=rad +z_out=m",
            linear_out_name(height.unit())
        )),
        None => Some(format!("+proj=unitconvert +xy_in={xy_in} +xy_out=rad")),
    }
}

/// The `unitconvert` step that turns the geographic axes into radians, or
/// `None` when they already are. The geocentric path leaves a height axis to
/// the `cart` step.
fn angular_to_radians(geographic: &GeographicCRS) -> Option<&'static str> {
    match geographic.coordinate_system().axes().first()?.unit().name() {
        "degree" => Some("+proj=unitconvert +xy_in=deg +xy_out=rad"),
        "grad" => Some("+proj=unitconvert +xy_in=grad +xy_out=rad"),
        _ => None,
    }
}

/// Degrees or grads into radians, with the metre height PROJ's cart step expects.
fn angular_z_to_radians(geographic: &GeographicCRS) -> Option<&'static str> {
    match geographic.coordinate_system().axes().first()?.unit().name() {
        "degree" => Some("+proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m"),
        "grad" => Some("+proj=unitconvert +xy_in=grad +z_in=m +xy_out=rad +z_out=m"),
        _ => None,
    }
}

/// Radians into the angular unit of `geographic`. A radian axis needs no step.
fn radians_to_angular(geographic: &GeographicCRS) -> Option<&'static str> {
    match geographic.coordinate_system().axes().first()?.unit().name() {
        "degree" => Some("+proj=unitconvert +xy_in=rad +xy_out=deg"),
        "grad" => Some("+proj=unitconvert +xy_in=rad +xy_out=grad"),
        _ => None,
    }
}

/// Geographic source to a geocentric target: axis swap, degrees to radians,
/// `+proj=cart`, then metres into the target linear unit.
fn geocentric_steps(source: &Crs, target: &GeocentricCRS) -> Result<Vec<String>, FactoryError> {
    let Some(geographic) = geographic_of(source) else {
        return Err(FactoryError::Format(
            "the source CRS is not geographic".to_owned(),
        ));
    };
    let same_datum = match (geographic.datum(), target.datum()) {
        (Some(left), Some(right)) => same_ellipsoid_and_meridian(left, right),
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
    if let Some(step) = angular_z_to_radians(geographic) {
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
            let out = linear_out_name(unit);
            steps.push(format!(
                "+proj=unitconvert +xy_in=m +z_in=m +xy_out={out} +z_out={out}"
            ));
        }
    }
    Ok(steps)
}

fn pipeline(steps: &[String]) -> String {
    if let [single] = steps {
        return single.clone();
    }
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

/// `DerivedGeographicCRS::_exportToPROJString`: a pole-rotation conversion, then
/// the base datum, `+no_defs` and `+type=crs`. Other derived kinds stay unexported.
fn derived_geographic_proj(crs: &super::niche::DerivedCRS) -> Result<String, FactoryError> {
    if crs.kind() != super::niche::DerivedKind::Geographic {
        return Err(FactoryError::Format(
            "this CRS kind has no PROJ string export".to_owned(),
        ));
    }
    let Some(step) = crs.deriving_conversion().pole_rotation_proj_string() else {
        return Err(FactoryError::Format(
            "DerivedGeographicCRS cannot be exported to PROJ string".to_owned(),
        ));
    };
    let base = crs.geodetic_base().ok_or_else(|| {
        FactoryError::Format("DerivedGeographicCRS cannot be exported to PROJ string".to_owned())
    })?;
    let mut text = step;
    let datum = crs_datum_clause(&base, &ProjExtras::default());
    if !datum.is_empty() {
        text.push(' ');
        text.push_str(&datum);
    }
    text.push_str(" +no_defs +type=crs");
    Ok(text)
}

/// `addDatumInfoToPROJString`: a datum token, unless `+towgs84` must be written,
/// else the ellipsoid and prime meridian, then `+towgs84`.
fn crs_datum_clause(crs: &GeodeticCRS, extras: &ProjExtras<'_>) -> String {
    if let (GeodeticCRS::Geographic(geographic), false) = (crs, extras.has_shift()) {
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
    if let Some(values) = extras.towgs84.as_ref() {
        parts.push(towgs84_clause(values));
    }
    if let Some(grids) = extras.nadgrids {
        parts.push(format!("+nadgrids={grids}"));
    }
    parts.join(" ")
}

fn towgs84_clause(values: &[f64; 7]) -> String {
    let numbers: Vec<String> = values.iter().map(|value| format_g(*value, 15)).collect();
    format!("+towgs84={}", numbers.join(","))
}

fn linear_unit_clause(unit: &UnitOfMeasure) -> Option<String> {
    linear_proj_unit(unit.conversion_to_si()).map(|id| format!("+units={id}"))
}

impl GeocentricCRS {
    /// `+proj=geocent` for a geocentric CRS bound to WGS 84 (`+towgs84`). The
    /// ellipsoid is written, not the datum name, as `BoundCRS` does.
    ///
    /// # Errors
    ///
    /// As [`Self::export_to_proj_string`].
    fn export_with(&self, extras: &ProjExtras<'_>) -> Result<String, FactoryError> {
        let Some(values) = extras.towgs84.as_ref() else {
            return self.export_to_proj_string();
        };
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
        if let Some(datum) = self.datum() {
            parts.push(ellipsoid_proj(datum.ellipsoid()));
        }
        parts.push(towgs84_clause(values));
        parts.push("+units=m".to_owned());
        parts.push("+no_defs".to_owned());
        parts.push("+type=crs".to_owned());
        Ok(parts.join(" "))
    }

    /// `+proj=geocent` when the cartesian unit is the metre.
    ///
    /// # Errors
    ///
    /// The unit is not the metre. PROJ builds that case only as a pipeline step.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        if let Some(text) = extension_proj(&self.identified) {
            return Ok(text);
        }
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

/// The `EXTENSION["PROJ4", ...]` text of a WKT1 CRS, as the string PROJ writes for it.
fn extension_proj(identified: &Identified) -> Option<String> {
    let text = identified
        .extension_proj4
        .as_deref()?
        .replace(" +type=crs", "");
    (!text.is_empty()).then(|| format!("{text} +type=crs"))
}

fn geographic_proj(crs: &GeographicCRS, extras: &ProjExtras<'_>) -> String {
    if let Some(text) = extension_proj(&crs.identified) {
        return text;
    }
    let mut parts = vec!["+proj=longlat".to_owned()];
    match datum_token(crs).filter(|_| !extras.has_shift()) {
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
    if let Some(grids) = extras.nadgrids {
        parts.push(format!("+nadgrids={grids}"));
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
        if !extras.geoid_crs.is_empty() {
            parts.push(format!("+geoid_crs={}", extras.geoid_crs));
        }
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
/// The CRS is named by `+title=`, or `unknown`. `+datum=WGS84`, `NAD83`, and `NAD27` use the
/// built-in frames. `+ellps=GRS80` or `WGS84` builds an `Unknown based on`
/// datum. `+a` and `+rf` build an unnamed ellipsoid. The coordinate system is
/// implicit, so identification accepts a latitude/longitude swap. The default
/// axis order is longitude then latitude; `+axis=` replaces the first two
/// letters.
///
/// `+nadgrids` (which wins), `+towgs84` and `+geoidgrids` give a bound CRS, or a
/// compound CRS with a bound vertical CRS, and `+vunits` or `+vto_meter` the
/// vertical unit (`buildBoundOrCompoundCRSIfNeeded`). A string with only
/// `+vunits` or `+vto_meter` is a vertical CRS. Without a database, seven
/// `+towgs84` terms are taken as they are.
///
/// # Errors
///
/// The string is not one longlat step, a datum or ellipsoid is unknown, a
/// number does not parse, or a `+towgs84` does not have 3 or 7 terms.
pub fn create_crs_from_proj_string(definition: &str) -> Result<Crs, FactoryError> {
    crs_from_proj_string(definition, None)
}

/// [`create_crs_from_proj_string`] with a database, as
/// `PROJStringParser::attachDatabaseContext` does. Seven `+towgs84` terms whose
/// rotation signs are the opposite of the EPSG position vector row they match
/// are repaired.
///
/// # Errors
///
/// As [`create_crs_from_proj_string`], or the database rejects the lookup.
pub fn create_crs_from_proj_string_with_database(
    definition: &str,
    db: &DatabaseContext,
) -> Result<Crs, FactoryError> {
    crs_from_proj_string(definition, Some(db))
}

/// A `krovak` or `mod_krovak` pipeline whose only other steps are a degree-to-radian
/// `unitconvert` and an optional `axisswap` of order `-2,-1`.
///
/// The axis swap is the south-west orientation (`+axis=swu`). Any other pipeline
/// shape is left for the caller.
fn krovak_from_pipeline(tokens: &[(String, String)]) -> Result<Option<Crs>, FactoryError> {
    let mut steps: Vec<Vec<(String, String)>> = Vec::new();
    let mut current: Option<Vec<(String, String)>> = None;
    for (key, value) in tokens {
        if key == "step" {
            if let Some(step) = current.take() {
                steps.push(step);
            }
            current = Some(Vec::new());
            continue;
        }
        if let Some(step) = current.as_mut() {
            step.push((key.clone(), value.clone()));
        }
    }
    if let Some(step) = current {
        steps.push(step);
    }
    if steps.is_empty() {
        return Ok(None);
    }
    let mut projection: Option<Vec<(String, String)>> = None;
    let mut swap_to_south_west = false;
    for step in &steps {
        let name = token(step, "proj");
        if is_degree_radian_convert(step) {
            continue;
        }
        if is_south_west_swap(step) {
            if swap_to_south_west {
                return Ok(None);
            }
            swap_to_south_west = true;
            continue;
        }
        if matches!(name, "krovak" | "mod_krovak") {
            if projection.is_some() {
                return Ok(None);
            }
            projection = Some(step.clone());
            continue;
        }
        return Ok(None);
    }
    let Some(mut projection) = projection else {
        return Ok(None);
    };
    if swap_to_south_west && token(&projection, "axis").is_empty() {
        projection.push(("axis".to_owned(), "swu".to_owned()));
    }
    Ok(Some(projected_from_proj_string(&projection)?))
}

fn is_degree_radian_convert(step: &[(String, String)]) -> bool {
    token(step, "proj") == "unitconvert"
        && token(step, "xy_in") == "deg"
        && token(step, "xy_out") == "rad"
        && step
            .iter()
            .all(|(key, _)| matches!(key.as_str(), "proj" | "xy_in" | "xy_out"))
}

fn is_south_west_swap(step: &[(String, String)]) -> bool {
    token(step, "proj") == "axisswap"
        && token(step, "order") == "-2,-1"
        && step
            .iter()
            .all(|(key, _)| matches!(key.as_str(), "proj" | "order"))
}

fn crs_from_proj_string(
    definition: &str,
    db: Option<&DatabaseContext>,
) -> Result<Crs, FactoryError> {
    let tokens = proj_tokens(definition);
    // A Krovak pipeline is a projected CRS (`+axis=swu`), not the generic pipeline CRS.
    if definition.split_whitespace().any(|word| {
        let word = word.strip_prefix('+').unwrap_or(word);
        word == "proj=pipeline"
    }) {
        match krovak_from_pipeline(&tokens)? {
            Some(crs) => return bound_import::bind_if_needed(&tokens, crs, db),
            None => return pipeline_crs(definition),
        }
    }
    if let Some(vertical) = bound_import::vertical_from_units(&tokens)? {
        return Ok(vertical);
    }
    let proj = token(&tokens, "proj");
    let mut crs = if matches!(proj, "longlat" | "lonlat" | "latlong") {
        geographic_from_proj_string(&tokens)?
    } else if proj == "geocent" {
        proj_crs::geocentric_from_proj_string(&tokens)?
    } else if proj == "pipeline" {
        match krovak_from_pipeline(&tokens)? {
            Some(crs) => crs,
            None => projected_from_proj_string(&tokens)?,
        }
    } else {
        projected_from_proj_string(&tokens)?
    };
    // `+over` is recognized by PROJ and unused by the CRS builder, so the
    // original string is kept and repeated in the remarks.
    if tokens.iter().any(|(key, _)| key == "over") {
        attach_over_extension(&mut crs, &tokens);
    }
    bound_import::bind_if_needed(&tokens, crs, db)
}

/// `CRS::setProperties` for a single-step string that still has `+over`.
fn attach_over_extension(crs: &mut Crs, tokens: &[(String, String)]) {
    let identified = match crs {
        Crs::Projected(projected) => &mut projected.identified,
        Crs::Geographic(geographic) => &mut geographic.identified,
        _ => return,
    };
    let bare = cleaned_proj_string(tokens).replace(" +type=crs", "");
    const PREFIX: &str = "PROJ CRS string: ";
    if !identified.remarks.contains(PREFIX) {
        identified.remarks = if identified.remarks.is_empty() {
            format!("{PREFIX}{bare}")
        } else {
            format!("{PREFIX}{bare}. {}", identified.remarks)
        };
    }
    identified.extension_proj4 = Some(bare);
}

fn geographic_from_proj_string(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    if tokens.iter().any(|(key, _)| key == "geoc") {
        return spherical_from_proj_string(tokens);
    }
    let datum = proj_crs::titled_datum(datum_from_proj(tokens)?, tokens);
    let axis = token(tokens, "axis");
    let mut axes = if axis.is_empty() {
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
    axes.extend(bound_import::ellipsoidal_height_axis(tokens)?);
    Ok(Crs::Geographic(GeographicCRS {
        identified: Identified::named(proj_crs::title_or_unknown(tokens)),
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

pub(super) fn token<'a>(tokens: &'a [(String, String)], key: &str) -> &'a str {
    tokens
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
        .unwrap_or("")
}

/// `PROJStringParser::Private::buildDatum`. `+R` takes precedence. Then
/// `+datum=` or `+ellps=` gives a base datum, and `+a`, `+b`, `+rf`, `+f`, `+e`,
/// and `+es` override its numbers. With no base, the datum is WGS 84.
pub(super) fn datum_from_proj(
    tokens: &[(String, String)],
) -> Result<GeodeticReferenceFrame, FactoryError> {
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
pub(super) fn guess_body(semi_major: f64) -> String {
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

pub(super) fn geographic_axis(letter: char, unit: UnitOfMeasure) -> Result<Axis, FactoryError> {
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

/// Keys of a PROJ string that name the datum, the units, the title, the grids,
/// or the output, not a parameter of a projection method.
const NON_METHOD_KEYS: &[&str] = &[
    "proj",
    "datum",
    "ellps",
    "a",
    "b",
    "rf",
    "f",
    "R",
    "towgs84",
    "units",
    "no_defs",
    "type",
    "axis",
    "title",
    "to_meter",
    "vunits",
    "vto_meter",
    "nadgrids",
    "geoidgrids",
    "geoid_crs",
    "wktext",
    "approx",
    "pm",
    "geoc",
    "over",
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

/// Whether a method's `proj_aux` is in the PROJ string: a flag such as `+guam`
/// or `+R_A`, or a `key=value` such as `+sweep=x` or `+axis=swu`.
fn aux_selects(method: &MethodMap, tokens: &[(String, String)]) -> bool {
    match method.proj_aux {
        None => false,
        Some(aux) => match aux.split_once('=') {
            Some((key, value)) => param_token(tokens, key) == Some(value),
            None => param_token(tokens, aux).is_some(),
        },
    }
}

/// The standard parallel `phi` with `cos(phi) / sqrt(1 - e^2 sin^2(phi)) = k0`,
/// found by bisection on `[0, pi/2]`, where the scale falls as `phi` rises.
fn standard_parallel_of_scale(k0: f64, ellipsoid: &Ellipsoid) -> f64 {
    let inverse_flattening = ellipsoid.inverse_flattening();
    let flattening = if inverse_flattening != 0.0 {
        1.0 / inverse_flattening
    } else {
        0.0
    };
    let squared_e = flattening * (2.0 - flattening);
    let (mut low, mut high) = (0.0_f64, std::f64::consts::FRAC_PI_2);
    for _ in 0..100 {
        let mid = 0.5 * (low + high);
        let scale = mid.cos() / (1.0 - squared_e * mid.sin().powi(2)).sqrt();
        if scale > k0 {
            low = mid;
        } else {
            high = mid;
        }
    }
    0.5 * (low + high)
}

/// The projection name and keys PROJ rewrites before it picks a method.
/// `+proj=somerc` is Hotine oblique Mercator variant B with azimuth and
/// rectified-grid angle 90, centred on `+lon_0`. On `+proj=cea` without `+lat_ts`, a `+k_0` is the
/// scale factor of the standard parallel: `cos(lat_ts)` on a sphere
/// (`+R_A`), else the ellipsoidal form of `standard_parallel_of_scale`.
fn method_spelling(
    proj: &str,
    tokens: &[(String, String)],
    ellipsoid: &Ellipsoid,
) -> (String, Vec<(String, String)>) {
    let mut tokens = tokens.to_vec();
    let mut proj = proj.to_owned();
    if proj == "etmerc" {
        // Exact transverse Mercator is the same EPSG method, written as `tmerc`.
        proj = "tmerc".to_owned();
    }
    if proj == "somerc" {
        proj = "omerc".to_owned();
        let oblique = ["alpha", "gamma", "lonc"]
            .iter()
            .any(|key| param_token(&tokens, key).is_some());
        if !oblique {
            tokens.push(("alpha".to_owned(), "90".to_owned()));
            tokens.push(("gamma".to_owned(), "90".to_owned()));
            if let Some(lon_0) = param_token(&tokens, "lon_0").map(str::to_owned) {
                tokens.push(("lonc".to_owned(), lon_0));
            }
        }
    }
    if proj == "cea" && param_token(&tokens, "lat_ts").is_none() {
        let k0 = param_token(&tokens, "k").and_then(|text| text.parse::<f64>().ok());
        if let Some(k0) = k0 {
            let lat_ts = if param_token(&tokens, "R_A").is_some() {
                k0.acos()
            } else {
                standard_parallel_of_scale(k0, ellipsoid)
            };
            tokens.retain(|(key, _)| !is_scale_key(key));
            tokens.push(("lat_ts".to_owned(), lat_ts.to_degrees().to_string()));
        }
    }
    (proj, tokens)
}

/// The EPSG method that `buildProjectedCRS` forces when the PROJ name alone
/// does not decide it. `None` leaves the choice to `projected_from_proj_string`.
/// Pseudo-Mercator and the no-rotation and two-point variants of `omerc` are
/// not decided here.
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
    if let Some(value) = projfam::default_value(proj, param) {
        return value;
    }
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
/// `+datum`/`+ellps` geographic CRS. Easting and northing are in the linear unit
/// of `+units` or `+to_meter` (`buildUnit`), and a linear parameter is stored in
/// that unit. A `+vunits` adds the ellipsoidal height axis to both CRSs.
/// `+proj=topocentric +type=crs`: a projected CRS over a geocentric base, with the
/// topocentric East, North and Up axes.
fn topocentric_crs(
    name: &str,
    base: GeographicCRS,
    conversion: Conversion,
    unit: UnitOfMeasure,
) -> Result<ProjectedCRS, FactoryError> {
    let geocentric = GeodeticCRS::Geocentric(GeocentricCRS {
        identified: base.identified.clone(),
        datum: base.datum,
        ensemble: None,
        coordinate_system: CartesianCS::create_geocentric(unit.clone()),
    });
    let coordinate_system = CartesianCS::create_allowing_mixed_units(vec![
        Axis::new("topocentric East", "U", "east", unit.clone()),
        Axis::new("topocentric North", "V", "north", unit.clone()),
        Axis::new("topocentric Up", "W", "up", unit),
    ])?;
    Ok(ProjectedCRS::create(
        name,
        geocentric,
        conversion,
        coordinate_system,
    ))
}

pub(super) fn projected_from_proj_string(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    if token(tokens, "proj") == "utm" {
        return projfam::utm_projected(tokens);
    }
    let unit = bound_import::build_unit(tokens, "units", "to_meter")?;
    let height = bound_import::ellipsoidal_height_axis(tokens)?;
    let krovak_default = matches!(token(tokens, "proj"), "krovak" | "mod_krovak")
        && ["datum", "ellps", "a", "rf"]
            .iter()
            .all(|key| token(tokens, key).is_empty());
    let datum = if krovak_default {
        named_ellipsoid_datum("bessel", &None)?
    } else {
        datum_from_proj(tokens)?
    };
    let datum = proj_crs::grid_datum(datum, tokens);
    let pseudo = proj_crs::pseudo_mercator(tokens);
    let (proj, spelled) = method_spelling(token(tokens, "proj"), tokens, datum.ellipsoid());
    let proj = proj.as_str();
    let tokens = spelled.as_slice();
    // PROJ refuses a polar `stere` that gives both `+lat_ts` and a scale other
    // than 1: the two describe the same parameter.
    let lat_0 = param_token(tokens, "lat_0").and_then(|text| text.parse::<f64>().ok());
    let scale = param_token(tokens, "k").and_then(|text| text.parse::<f64>().ok());
    if proj == "stere"
        && lat_0.is_some_and(|lat| (lat.abs() - 90.0).abs() < 1e-10)
        && param_token(tokens, "lat_ts").is_some()
        && scale.is_some_and(|k| (k - 1.0).abs() > 1e-10)
    {
        return Err(FactoryError::Invalid(
            "+lat_ts and +k are both given for a polar stereographic".to_owned(),
        ));
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
    // A method named by its flag or `key=value` wins. Then the EPSG code the
    // PROJ name forces. Then the non-polar `stere`, which has no EPSG code.
    // Otherwise the first method without a flag whose parameters cover the keys.
    let by_aux = methods
        .iter()
        .copied()
        .find(|method| aux_selects(method, tokens));
    // The pseudo-Mercator method (1024) is not a `merc` method by PROJ name.
    let pseudo_method = pseudo.and_then(|_| {
        PROJECTION_METHODS
            .iter()
            .chain(OTHER_METHODS)
            .find(|method| method.epsg_code == 1024)
    });
    let forced = pseudo_method
        .or(by_aux)
        .or_else(|| {
            projfam::spherical_method_code(proj, datum.ellipsoid().inverse_flattening())
                .or_else(|| forced_method_code(proj, tokens))
                .and_then(|code| {
                    methods
                        .iter()
                        .copied()
                        .find(|method| method.epsg_code == code)
                })
        })
        .or_else(|| {
            methods
                .iter()
                .copied()
                .find(|method| proj == "stere" && method.wkt2_name == "Stereographic")
        });
    let Some(method) = forced.or_else(|| {
        methods
            .iter()
            .copied()
            .find(|method| method.proj_aux.is_none() && covers(method))
    }) else {
        // `+proj=unknown` and a name that is not an identifier are refused.
        // Other unmapped names (`mbt_s`, `peirce_q`, `ob_tran`) stay PROJ-based
        // CRSs when `+type=crs` is set.
        if token(tokens, "type") == "crs" && proj_identifier(proj) {
            return proj_based_projected(tokens);
        }
        return Err(FactoryError::Invalid(format!(
            "unsupported +proj={proj} for a CRS"
        )));
    };
    // The web Mercator CRS keeps metre parameters whatever `+units` says.
    let web = pseudo == Some(proj_crs::PseudoMercator::Web);
    let parameter_unit = if web {
        UnitOfMeasure::metre()
    } else {
        unit.clone()
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
            // PROJ gives a linear value in metres, and stores it in the CRS unit.
            _ => ParameterValue::Measure(Measure {
                value: value / parameter_unit.conversion_to_si(),
                unit: parameter_unit.clone(),
            }),
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
    if tokens.iter().any(|(key, _)| key == "approx") && matches!(proj, "tmerc" | "utm") {
        conversion.identified.proj_method_override = Some(format!("{proj} approx"));
    }
    let base = GeographicCRS {
        identified: Identified::named("unknown"),
        datum: Some(datum),
        ensemble: None,
        coordinate_system: EllipsoidalCS {
            identified: Identified::default(),
            axes: vec![
                geographic_axis('e', UnitOfMeasure::degree())?,
                geographic_axis('n', UnitOfMeasure::degree())?,
            ]
            .into_iter()
            .chain(height.clone())
            .collect(),
        },
        implicit_cs: true,
    };
    let name = param_token(tokens, "title")
        .filter(|title| !title.is_empty())
        .unwrap_or("unknown");
    if proj == "topocentric" {
        return Ok(Crs::Projected(topocentric_crs(
            name, base, conversion, unit,
        )?));
    }
    let coordinate_system = match height {
        None => projfam::projected_cs(proj, tokens, unit)?,
        Some(height) => CartesianCS::create_allowing_mixed_units(vec![
            Axis::new("Easting", "E", "east", unit.clone()),
            Axis::new("Northing", "N", "north", unit),
            height,
        ])?,
    };
    if web {
        // `createPseudoMercator`: the WGS 84 base, named with the `+units` suffix.
        let units = token(tokens, "units");
        let name = if units.is_empty() || units == "m" {
            "WGS 84 / Pseudo-Mercator".to_owned()
        } else {
            format!("WGS 84 / Pseudo-Mercator (unit {units})")
        };
        return Ok(Crs::Projected(ProjectedCRS::create(
            name,
            GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
            conversion,
            coordinate_system,
        )));
    }
    Ok(Crs::Projected(ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(base),
        conversion,
        coordinate_system,
    )))
}

/// Ellipsoid numbers and prime-meridian longitude, ignoring the datum name.
///
/// `+ellps=WGS84` names the frame `Unknown based on WGS 84 ellipsoid`. That is
/// the same ellipsoid and meridian as EPSG:6326, which a cart pipeline from
/// EPSG:4326 has to accept.
fn same_ellipsoid_and_meridian(
    left: &GeodeticReferenceFrame,
    right: &GeodeticReferenceFrame,
) -> bool {
    let major = (left.ellipsoid().semi_major_axis().si_value()
        - right.ellipsoid().semi_major_axis().si_value())
    .abs()
        <= 1e-6;
    let flattening =
        (left.ellipsoid().inverse_flattening() - right.ellipsoid().inverse_flattening()).abs()
            <= 1e-8;
    let meridian = (left.prime_meridian().longitude().si_value()
        - right.prime_meridian().longitude().si_value())
    .abs()
        <= 1e-12;
    major && flattening && meridian
}

/// `+proj=longlat +geoc`: a spherical planetocentric CRS. This build has no
/// PROJ string for it, which is what `io.projparse_geoc` asserts.
fn spherical_from_proj_string(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    let datum = proj_crs::titled_datum(datum_from_proj(tokens)?, tokens);
    let degree = UnitOfMeasure::degree();
    Ok(Crs::Geodetic(GeodeticCRS::Spherical(
        SphericalGeodeticCRS {
            identified: Identified::named(proj_crs::title_or_unknown(tokens)),
            datum: Some(datum),
            ensemble: None,
            coordinate_system: SphericalCS {
                identified: Identified::default(),
                axes: vec![
                    Axis::new("Planetocentric longitude", "V", "east", degree.clone()),
                    Axis::new("Planetocentric latitude", "U", "north", degree),
                ],
            },
        },
    )))
}

/// A `+proj=` with no EPSG method (`peirce_q` horizontal, `ob_tran`, `mbt_s`).
///
/// The conversion is `PROJ {name}` plus the numeric parameters. Flags PROJ
/// itself does not use are dropped, and the cleaned string is stored so export
/// matches `exportToPROJString` instead of expanding defaults.
/// A PROJ method name: letters, digits, and underscores, and not `unknown`.
fn proj_identifier(proj: &str) -> bool {
    !proj.is_empty()
        && proj != "unknown"
        && proj
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn proj_based_projected(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    let mut crs = proj_based_shell(tokens)?;
    crs.identified.extension_proj4 = Some(cleaned_proj_string(tokens));
    if geographic_ob_tran(&crs.conversion.method.name) {
        crs.conversion.identified.extension_proj4 = Some(operation_body(tokens));
    }
    Ok(Crs::Projected(crs))
}

fn proj_based_shell(tokens: &[(String, String)]) -> Result<ProjectedCRS, FactoryError> {
    let datum = datum_from_proj(tokens)?;
    let unit = bound_import::build_unit(tokens, "units", "to_meter")?;
    let (method_name, parameters) = proj_based_method(tokens);
    let mut conversion = Conversion::create_with_method(&method_name, "", "", parameters);
    conversion.identified = Identified::named("unknown");
    let name = param_token(tokens, "title")
        .filter(|title| !title.is_empty())
        .unwrap_or("unknown");
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
    Ok(ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(base),
        conversion,
        CartesianCS::create_easting_northing(unit),
    ))
}

/// Keys `buildProjectedCRS` does not turn into a parameter of an unmapped method.
const PROJ_BASED_SKIP: &[&str] = &[
    "proj",
    "wktext",
    "no_defs",
    "datum",
    "ellps",
    "a",
    "b",
    "R",
    "f",
    "rf",
    "towgs84",
    "nadgrids",
    "geoidgrids",
    "units",
    "to_meter",
    "vunits",
    "vto_meter",
    "type",
];

/// Empty-value tokens a PROJ method may actually read. Any other flag is the
/// `unused_flag` case: `pj` never marks it used, so the CRS string drops it.
const KNOWN_PROJ_FLAGS: &[&str] = &[
    "no_defs", "wktext", "approx", "over", "czech", "guam", "R_A", "no_rot", "no_uoff", "no_off",
    "south", "geoc",
];

fn proj_based_method(tokens: &[(String, String)]) -> (String, Vec<super::model::Parameter>) {
    let proj = token(tokens, "proj");
    let mut method_name = format!("PROJ {proj}");
    let mut parameters = Vec::new();
    for (key, value) in tokens {
        if PROJ_BASED_SKIP.contains(&key.as_str()) {
            continue;
        }
        if value.is_empty() && !KNOWN_PROJ_FLAGS.contains(&key.as_str()) {
            continue;
        }
        if value.is_empty() {
            method_name.push(' ');
            method_name.push_str(key);
            continue;
        }
        if value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
            || key == "gores"
        {
            method_name.push(' ');
            method_name.push_str(key);
            method_name.push('=');
            method_name.push_str(value);
            continue;
        }
        let Ok(number) = value.parse::<f64>() else {
            continue;
        };
        let measure = if matches!(key.as_str(), "x_0" | "y_0" | "h" | "h_0") {
            ParameterValue::length(number)
        } else if matches!(
            key.as_str(),
            "k" | "k_0"
                | "north_square"
                | "south_square"
                | "n"
                | "m"
                | "q"
                | "path"
                | "lsat"
                | "W"
                | "M"
        ) {
            ParameterValue::scale(number)
        } else {
            ParameterValue::angle(number)
        };
        parameters.push(Conversion::parameter(key, "", "", measure));
    }
    (method_name, parameters)
}

fn cleaned_proj_string(tokens: &[(String, String)]) -> String {
    let mut parts = Vec::new();
    for (key, value) in tokens {
        if value.is_empty() && !KNOWN_PROJ_FLAGS.contains(&key.as_str()) {
            continue;
        }
        if value.is_empty() {
            parts.push(format!("+{key}"));
        } else {
            parts.push(format!("+{key}={value}"));
        }
    }
    parts.join(" ")
}

/// The `ob_tran` step without the datum clause. `projection_steps` appends `+ellps`.
/// `proj` stays: the step is `+proj=ob_tran`, not a bare parameter list.
fn operation_body(tokens: &[(String, String)]) -> String {
    let mut parts = Vec::new();
    for (key, value) in tokens {
        if key != "proj" && (PROJ_BASED_SKIP.contains(&key.as_str()) || key == "pm") {
            continue;
        }
        if value.is_empty() {
            parts.push(format!("+{key}"));
        } else {
            parts.push(format!("+{key}={value}"));
        }
    }
    parts.join(" ")
}

fn geographic_ob_tran(method_name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "PROJ ob_tran o_proj=longlat",
        "PROJ ob_tran o_proj=lonlat",
        "PROJ ob_tran o_proj=latlong",
        "PROJ ob_tran o_proj=latlon",
    ];
    PREFIXES
        .iter()
        .any(|prefix| method_name.starts_with(prefix))
}

struct PipeStep {
    name: String,
    inverted: bool,
    params: Vec<(String, String)>,
}

struct ParsedPipeline {
    steps: Vec<PipeStep>,
    global_type_crs: bool,
}

/// `+proj=pipeline` steps, keeping a leading `+type=crs`.
fn parse_pipeline(definition: &str) -> Option<ParsedPipeline> {
    let words: Vec<&str> = definition
        .split_whitespace()
        .map(|word| word.strip_prefix('+').unwrap_or(word))
        .collect();
    if !words.contains(&"proj=pipeline") {
        return None;
    }
    let mut steps = Vec::new();
    let mut global_type_crs = false;
    let mut seen_step = false;
    for word in words {
        if word == "proj=pipeline" {
            continue;
        }
        if word == "step" {
            steps.push(PipeStep {
                name: String::new(),
                inverted: false,
                params: Vec::new(),
            });
            seen_step = true;
            continue;
        }
        if word == "inv" {
            if let Some(step) = steps.last_mut() {
                step.inverted = true;
            }
            continue;
        }
        if let Some(name) = word.strip_prefix("proj=") {
            if let Some(step) = steps.last_mut().filter(|step| step.name.is_empty()) {
                step.name = name.to_owned();
                continue;
            }
        }
        let (key, value) = word.split_once('=').unwrap_or((word, ""));
        if !seen_step {
            if key == "type" && value == "crs" {
                global_type_crs = true;
            }
            continue;
        }
        if let Some(step) = steps.last_mut() {
            step.params.push((key.to_owned(), value.to_owned()));
        }
    }
    Some(ParsedPipeline {
        steps,
        global_type_crs,
    })
}

fn step_value<'a>(step: &'a PipeStep, key: &str) -> &'a str {
    step.params
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
        .unwrap_or("")
}

fn step_tokens(step: &PipeStep) -> Vec<(String, String)> {
    let mut tokens = vec![("proj".to_owned(), step.name.clone())];
    tokens.extend(step.params.iter().cloned());
    tokens
}

fn is_geographic_name(name: &str) -> bool {
    matches!(name, "longlat" | "lonlat" | "latlong" | "latlon")
}

/// `PROJStringParser::createFromPROJString` for `+proj=pipeline`.
///
/// A recognized shape is a geographic, projected, or geocentric CRS. Anything
/// else is a PROJ-based operation, which this function refuses: the caller
/// asked for a CRS.
fn pipeline_crs(definition: &str) -> Result<Crs, FactoryError> {
    let parsed = parse_pipeline(definition)
        .ok_or_else(|| FactoryError::Invalid("unsupported +proj=pipeline for a CRS".to_owned()))?;
    let steps = parsed.steps;
    if steps.is_empty() {
        return Err(FactoryError::Invalid(
            "unsupported +proj=pipeline for a CRS".to_owned(),
        ));
    }
    let geocentric = !steps[0].inverted
        && matches!(steps[0].name.as_str(), "cart" | "geocent")
        && ((steps.len() == 1 && step_value(&steps[0], "type") == "crs")
            || (steps.len() == 2 && steps[1].name == "unitconvert"));
    if geocentric {
        return geocentric_from_pipeline(&steps);
    }

    let mut geog = Vec::new();
    let mut units = Vec::new();
    let mut swaps = Vec::new();
    let mut proj = None;
    let mut unexpected = false;
    for (index, step) in steps.iter().enumerate() {
        if is_geographic_name(&step.name) {
            geog.push(index);
        } else if step.name == "unitconvert" {
            units.push(index);
        } else if step.name == "axisswap" {
            swaps.push(index);
        } else if proj.is_none() {
            proj = Some(index);
        } else {
            unexpected = true;
            break;
        }
    }
    if geog.len() > 2 || units.len() > 2 || swaps.len() > 2 {
        unexpected = true;
    }
    let type_ok = if steps.len() == 1 {
        step_value(&steps[0], "type") == "crs"
    } else {
        parsed.global_type_crs
    };
    if !type_ok {
        unexpected = true;
    }
    if unexpected {
        return Err(FactoryError::Invalid(
            "unsupported +proj=pipeline for a CRS".to_owned(),
        ));
    }
    let i_geog = geog.first().copied();
    let i_unit = units.first().copied();
    let i_unit2 = units.get(1).copied();
    let i_swap = swaps.first().copied();
    let i_swap2 = swaps.get(1).copied();
    if i_geog == Some(0)
        && geog.len() == 1
        && proj.is_none()
        && units.len() <= 1
        && swaps.len() <= 1
        && !steps[0].inverted
    {
        return geographic_from_pipeline(&steps, i_unit, i_swap);
    }
    if let Some(i_proj) = proj.filter(|index| !steps[*index].inverted) {
        if geog.len() < 2 && i_geog.is_none_or(|index| index + 1 == i_proj) {
            let anchor = i_geog.unwrap_or(i_proj);
            return projected_from_pipeline(
                &steps,
                PipelineProj {
                    proj: i_proj,
                    anchor,
                    unit: i_unit,
                    unit2: i_unit2,
                    swap: i_swap,
                    swap2: i_swap2,
                },
            );
        }
    }
    Err(FactoryError::Invalid(
        "unsupported +proj=pipeline for a CRS".to_owned(),
    ))
}

struct PipelineProj {
    proj: usize,
    anchor: usize,
    unit: Option<usize>,
    unit2: Option<usize>,
    swap: Option<usize>,
    swap2: Option<usize>,
}

fn geocentric_from_pipeline(steps: &[PipeStep]) -> Result<Crs, FactoryError> {
    let tokens = step_tokens(&steps[0]);
    let datum = datum_from_proj(&tokens)?;
    let mut unit = UnitOfMeasure::metre();
    if steps.len() == 2 {
        let xy_in = step_value(&steps[1], "xy_in");
        let xy_out = step_value(&steps[1], "xy_out");
        let z_in = step_value(&steps[1], "z_in");
        let z_out = step_value(&steps[1], "z_out");
        if xy_in != "m" || z_in != "m" || xy_out.is_empty() || xy_out != z_out {
            return Err(FactoryError::Invalid(
                "unhandled values for xy_in, z_in, xy_out or z_out".to_owned(),
            ));
        }
        unit = unit_from_xy_out(xy_out)?;
    }
    Ok(Crs::Geodetic(GeodeticCRS::Geocentric(GeocentricCRS {
        identified: Identified::named("unknown"),
        datum: Some(datum),
        ensemble: None,
        coordinate_system: CartesianCS::create_geocentric(unit),
    })))
}

fn geographic_from_pipeline(
    steps: &[PipeStep],
    i_unit: Option<usize>,
    i_swap: Option<usize>,
) -> Result<Crs, FactoryError> {
    let step = &steps[0];
    let tokens = step_tokens(step);
    let datum = proj_crs::titled_datum(datum_from_proj(&tokens)?, &tokens);
    let unit = angular_unit(steps, 0, i_unit)?;
    let order = swap_order(steps, i_swap)?;
    let axes = axis_pair(order.as_deref(), &unit, AxisKind::Geographic)?;
    if step.params.iter().any(|(key, _)| key == "geoc") {
        return Ok(Crs::Geodetic(GeodeticCRS::Spherical(
            SphericalGeodeticCRS {
                identified: Identified::named(proj_crs::title_or_unknown(&tokens)),
                datum: Some(datum),
                ensemble: None,
                coordinate_system: SphericalCS {
                    identified: Identified::default(),
                    axes,
                },
            },
        )));
    }
    Ok(Crs::Geographic(GeographicCRS {
        identified: Identified::named(proj_crs::title_or_unknown(&tokens)),
        datum: Some(datum),
        ensemble: None,
        coordinate_system: EllipsoidalCS {
            identified: Identified::default(),
            axes,
        },
        implicit_cs: true,
    }))
}

fn projected_from_pipeline(steps: &[PipeStep], hits: PipelineProj) -> Result<Crs, FactoryError> {
    let step = &steps[hits.proj];
    // A prime meridian on only the projection step is that step's `+pm`.
    // A leading geographic step with a different `+pm` (including one side empty) is refused.
    if hits.anchor != hits.proj {
        let geog_pm = step_value(&steps[hits.anchor], "pm");
        let proj_pm = step_value(step, "pm");
        if !geog_pm.eq_ignore_ascii_case(proj_pm) {
            return Err(FactoryError::Invalid(
                "inconsistent pm values between geogCRS and projectedCRS".to_owned(),
            ));
        }
    }
    if step.name == "ob_tran" && is_geographic_name(step_value(step, "o_proj")) {
        let tokens = step_tokens(step);
        let mut crs = proj_based_shell(&tokens)?;
        crs.conversion.identified.extension_proj4 = Some(operation_body(&tokens));
        return Ok(Crs::Projected(crs));
    }
    let mut tokens = step_tokens(step);
    let proj_swap = projected_index(hits.swap, hits.swap2, hits.anchor);
    let applied_wsu = step.name == "tmerc"
        && proj_swap.is_some_and(|index| step_value(&steps[index], "order") == "-1,-2");
    if applied_wsu {
        tokens.push(("axis".to_owned(), "wsu".to_owned()));
    }
    let mut crs = match projected_from_proj_string(&tokens) {
        Ok(crs) => crs,
        Err(error) => {
            if let FactoryError::Invalid(text) = &error {
                if text.starts_with("unsupported +proj=") {
                    return proj_based_projected(&tokens);
                }
            }
            return Err(error);
        }
    };
    let proj_unit = projected_index(hits.unit, hits.unit2, hits.anchor);
    if let Some(index) = proj_unit {
        apply_projected_unit(&mut crs, &steps[index])?;
    }
    if !applied_wsu {
        if let Some(index) = proj_swap {
            apply_projected_swap(&mut crs, &steps[index])?;
        }
    }
    Ok(crs)
}

/// The unitconvert or axisswap that belongs to the projected CRS, not its base.
fn projected_index(first: Option<usize>, second: Option<usize>, anchor: usize) -> Option<usize> {
    if first.is_some_and(|index| index < anchor) {
        second
    } else {
        first
    }
}

fn apply_projected_unit(crs: &mut Crs, step: &PipeStep) -> Result<(), FactoryError> {
    let xy_in = step_value(step, "xy_in");
    let xy_out = step_value(step, "xy_out");
    if xy_in != "m" || xy_out.is_empty() {
        return Err(FactoryError::Invalid(
            "unhandled values for xy_in and/or xy_out".to_owned(),
        ));
    }
    let unit = unit_from_xy_out(xy_out)?;
    let Crs::Projected(projected) = crs else {
        return Ok(());
    };
    for axis in &mut projected.coordinate_system.axes {
        if axis.unit.kind == UnitKind::Linear {
            axis.unit = unit.clone();
        }
    }
    Ok(())
}

fn apply_projected_swap(crs: &mut Crs, step: &PipeStep) -> Result<(), FactoryError> {
    let order = swap_order(std::slice::from_ref(step), Some(0))?;
    let Crs::Projected(projected) = crs else {
        return Ok(());
    };
    let unit = projected
        .coordinate_system
        .axes
        .first()
        .map(Axis::unit)
        .cloned()
        .unwrap_or_else(UnitOfMeasure::metre);
    projected.coordinate_system =
        CartesianCS::create(axis_pair(order.as_deref(), &unit, AxisKind::Projected)?)?;
    Ok(())
}

fn angular_unit(
    steps: &[PipeStep],
    i_step: usize,
    i_unit: Option<usize>,
) -> Result<UnitOfMeasure, FactoryError> {
    let Some(i_unit) = i_unit else {
        return Ok(UnitOfMeasure::degree());
    };
    let step = &steps[i_unit];
    let mut xy_in = step_value(step, "xy_in").to_owned();
    let mut xy_out = step_value(step, "xy_out").to_owned();
    if step.inverted {
        std::mem::swap(&mut xy_in, &mut xy_out);
    }
    if i_unit < i_step {
        std::mem::swap(&mut xy_in, &mut xy_out);
    }
    if xy_in != "rad" || !matches!(xy_out.as_str(), "rad" | "deg" | "grad") {
        return Err(FactoryError::Invalid(
            "unhandled values for xy_in and/or xy_out".to_owned(),
        ));
    }
    Ok(match xy_out.as_str() {
        "rad" => UnitOfMeasure::new("radian", 1.0, UnitKind::Angular),
        "grad" => UnitOfMeasure::grad(),
        _ => UnitOfMeasure::degree(),
    })
}

fn swap_order(steps: &[PipeStep], index: Option<usize>) -> Result<Option<String>, FactoryError> {
    let Some(index) = index else {
        return Ok(None);
    };
    let step = &steps[index];
    if step.inverted {
        return Err(FactoryError::Invalid(
            "Unhandled +inv for +proj=axisswap".to_owned(),
        ));
    }
    let order = step_value(step, "order");
    if order.split(',').count() < 2 {
        return Err(FactoryError::Invalid(format!("Unhandled order={order}")));
    }
    Ok(Some(order.to_owned()))
}

/// Geographic axes are longitude and latitude. Projected axes are easting and northing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AxisKind {
    Geographic,
    Projected,
}

fn axis_pair(
    order: Option<&str>,
    unit: &UnitOfMeasure,
    kind: AxisKind,
) -> Result<Vec<Axis>, FactoryError> {
    let slots: Vec<&str> = match order {
        None => vec!["1", "2"],
        Some(order) => order.split(',').collect(),
    };
    if slots.len() < 2 {
        return Err(FactoryError::Invalid(format!(
            "Unhandled order={}",
            order.unwrap_or("")
        )));
    }
    slots
        .into_iter()
        .take(2)
        .map(|slot| axis_for_slot(slot, unit, kind))
        .collect()
}

fn axis_for_slot(slot: &str, unit: &UnitOfMeasure, kind: AxisKind) -> Result<Axis, FactoryError> {
    let (name, abbreviation, direction) = match (slot, kind) {
        ("1", AxisKind::Geographic) => ("Longitude", "lon", "east"),
        ("-1", AxisKind::Geographic) => ("Longitude", "lon", "west"),
        ("2", AxisKind::Geographic) => ("Latitude", "lat", "north"),
        ("-2", AxisKind::Geographic) => ("Latitude", "lat", "south"),
        ("1", AxisKind::Projected) => ("Easting", "E", "east"),
        ("-1", AxisKind::Projected) => ("Westing", "", "west"),
        ("2", AxisKind::Projected) => ("Northing", "N", "north"),
        ("-2", AxisKind::Projected) => ("Southing", "", "south"),
        _ => {
            return Err(FactoryError::Invalid(format!("Unhandled order={slot}")));
        }
    };
    Ok(Axis::new(name, abbreviation, direction, unit.clone()))
}

fn unit_from_xy_out(xy_out: &str) -> Result<UnitOfMeasure, FactoryError> {
    let named = match xy_out {
        "km" => Some(("kilometre", 1000.0)),
        "m" => Some(("metre", 1.0)),
        "ft" => Some(("foot", 0.3048)),
        "us-ft" => Some(("US survey foot", 0.304_800_609_601_219)),
        _ => None,
    };
    if let Some((name, factor)) = named {
        return Ok(UnitOfMeasure::new(name, factor, UnitKind::Linear));
    }
    let factor = xy_out.parse::<f64>().map_err(|_| {
        FactoryError::Invalid("unhandled values for xy_in and/or xy_out".to_owned())
    })?;
    Ok(UnitOfMeasure::new(xy_out, factor, UnitKind::Linear))
}
