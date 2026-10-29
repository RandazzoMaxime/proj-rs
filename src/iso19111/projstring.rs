//! PROJ strings for the geographic CRS the unit tests export.
//!
//! Datum codes 6326, 6267, and 6269 become `+datum=`. Anything else is
//! `+ellps` plus `+pm` when the prime meridian is not Greenwich.

use super::buildop::OpProps;
use super::factory::DatabaseContext;
use super::model::{
    Axis, BoundCRS, CartesianCS, CompoundCRS, Conversion, CoordinateOperation, Crs, Datum,
    DatumEnsemble, Ellipsoid, EllipsoidalCS, Transformation,
    FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identified,
    Identifier, Measure, OperationMethod, Parameter, ParameterValue, PrimeMeridian, ProjectedCRS,
    UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame,
};

impl GeographicCRS {
    /// `exportToPROJString` for a geographic CRS.
    #[must_use]
    pub fn export_to_proj_string(&self) -> String {
        geographic_proj(self)
    }
}

impl GeodeticCRS {
    /// `exportToPROJString` for a geographic or metre geocentric CRS.
    ///
    /// # Errors
    ///
    /// The CRS is spherical, or geocentric in a unit other than the metre.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        match self {
            Self::Geographic(crs) => Ok(crs.export_to_proj_string()),
            Self::Geocentric(crs) => crs.export_to_proj_string(),
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
            Self::DerivedVertical(crs) => &crs.identified.remarks,
            Self::DerivedGeographic(crs) => &crs.identified.remarks,
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

impl CompoundCRS {
    /// `exportToPROJString` for a horizontal CRS plus a vertical CRS.
    ///
    /// The vertical unit is `+vunits` and is written before `+no_defs`.
    ///
    /// # Errors
    ///
    /// The horizontal component cannot be written as a PROJ string.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        let mut text = String::new();
        for component in &self.components {
            match component {
                Crs::Projected(crs) => push_clause(&mut text, &crs.export_to_proj_string()?),
                Crs::Geographic(crs) => push_clause(&mut text, &crs.export_to_proj_string()),
                Crs::Geodetic(crs) => push_clause(&mut text, &crs.export_to_proj_string()?),
                Crs::Vertical(crs) => {
                    insert_vertical_unit(&mut text, crs.coordinate_system().axis().unit());
                }
                Crs::DerivedVertical(crs) => {
                    insert_vertical_unit(&mut text, crs.coordinate_system().axis().unit());
                }
                _ => {}
            }
        }
        if !text.contains("+type=crs") {
            push_clause(&mut text, "+no_defs +type=crs");
        }
        Ok(text)
    }
}

fn push_clause(text: &mut String, clause: &str) {
    if clause.is_empty() {
        return;
    }
    if !text.is_empty() && !text.ends_with(' ') {
        text.push(' ');
    }
    text.push_str(clause);
}

fn insert_vertical_unit(text: &mut String, unit: &UnitOfMeasure) {
    let Some(clause) = vertical_unit_clause(unit) else {
        return;
    };
    if let Some(index) = text.find(" +no_defs") {
        text.insert_str(index, &format!(" {clause}"));
    } else {
        push_clause(text, &clause);
    }
}

fn vertical_unit_clause(unit: &UnitOfMeasure) -> Option<String> {
    match unit.name() {
        "metre" => Some("+vunits=m".to_owned()),
        "foot" | "Foot" => Some("+vunits=ft".to_owned()),
        "US survey foot" => Some("+vunits=us-ft".to_owned()),
        _ if unit.kind() == UnitKind::Linear => {
            Some(format!("+vto_meter={}", trim_proj_number(unit.conversion_to_si())))
        }
        _ => None,
    }
}

fn trim_proj_number(value: f64) -> String {
    let mut text = format!("{value}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

impl BoundCRS {
    /// `exportToPROJString` for a bound CRS whose hub is WGS 84.
    ///
    /// A Helmert transformation is `+ellps` plus `+towgs84`, not `+datum`.
    ///
    /// # Errors
    ///
    /// The source CRS has no PROJ string, or the transformation is not TOWGS84.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        self.proj_string(None)
    }

    /// `exportToPROJString` with a formatter that carries `proj.db`.
    ///
    /// A NADCON latitude file is replaced by its PROJ grid alternative before
    /// `+nadgrids` is written.
    ///
    /// # Errors
    ///
    /// The source CRS has no PROJ string.
    pub fn export_to_proj_string_with_database(
        &self,
        db: &DatabaseContext,
    ) -> Result<String, FactoryError> {
        self.proj_string(Some(db))
    }

    fn proj_string(&self, db: Option<&DatabaseContext>) -> Result<String, FactoryError> {
        let towgs84 = super::wkt::wkt1_towgs84(self);
        let nadgrids = nadgrids_name(self, db);
        match self.source_crs() {
            Crs::Geographic(crs) => Ok(geographic_bound_proj(
                crs,
                towgs84.as_deref(),
                nadgrids.as_deref(),
            )),
            Crs::Projected(crs) => {
                projected_bound_proj(crs, towgs84.as_deref(), nadgrids.as_deref())
            }
            Crs::Geodetic(crs) => crs.export_to_proj_string(),
            Crs::Vertical(crs) => vertical_bound_proj(crs, bound_geoid_grids(self).as_deref()),
            _ => Err(FactoryError::Format(
                "this bound CRS has no PROJ string in this build".to_owned(),
            )),
        }
    }
}

pub(crate) fn bound_geoid_grids(crs: &BoundCRS) -> Option<String> {
    if !matches!(crs.source_crs(), Crs::Vertical(_)) {
        return None;
    }
    if !crs.target_crs().name().eq_ignore_ascii_case("WGS 84") {
        return None;
    }
    let CoordinateOperation::Transformation(transformation) = crs.transformation() else {
        return None;
    };
    filename_parameter(transformation, "8666")
}

pub(crate) fn bound_nadgrids(crs: &BoundCRS, db: Option<&DatabaseContext>) -> Option<String> {
    nadgrids_name(crs, db)
}

fn nadgrids_name(crs: &BoundCRS, db: Option<&DatabaseContext>) -> Option<String> {
    if !crs.target_crs().name().eq_ignore_ascii_case("WGS 84") {
        return None;
    }
    let CoordinateOperation::Transformation(transformation) = crs.transformation() else {
        return None;
    };
    let official = filename_parameter(transformation, "8657")
        .or_else(|| filename_parameter(transformation, "8656"))?;
    let Some(db) = db else {
        return Some(official);
    };
    let Some(alternative) = db.grid_alternative(&official) else {
        return Some(official);
    };
    let usable = matches!(
        alternative.format.as_str(),
        "GTiff" | "NTv2" | "NTv1" | "CTable2"
    ) && !alternative.inverse;
    if usable {
        Some(alternative.proj_grid_name)
    } else {
        Some(official)
    }
}

fn filename_parameter(transformation: &Transformation, code: &str) -> Option<String> {
    transformation.parameters().iter().find_map(|parameter| {
        if parameter.code() != code {
            return None;
        }
        match parameter.value() {
            ParameterValue::Filename(name) if !name.is_empty() => Some(name.clone()),
            _ => None,
        }
    })
}

fn geographic_bound_proj(
    crs: &GeographicCRS,
    towgs84: Option<&[f64]>,
    nadgrids: Option<&str>,
) -> String {
    if towgs84.is_none() && nadgrids.is_none() {
        return crs.export_to_proj_string();
    }
    let mut parts = vec!["+proj=longlat".to_owned()];
    if let Some(ellipsoid) = ellipsoid_of(crs) {
        parts.push(ellipsoid_proj(ellipsoid));
    }
    if let Some(meridian) = meridian_of(crs) {
        if let Some(clause) = prime_meridian_proj(meridian) {
            parts.push(clause);
        }
    }
    if let Some(values) = towgs84 {
        parts.push(format!("+towgs84={}", format_towgs84(values)));
    }
    if let Some(grid) = nadgrids {
        parts.push(format!("+nadgrids={grid}"));
    }
    parts.push("+no_defs".to_owned());
    parts.push("+type=crs".to_owned());
    parts.join(" ")
}

fn vertical_bound_proj(crs: &VerticalCRS, grid: Option<&str>) -> Result<String, FactoryError> {
    let Some(grid) = grid else {
        return Err(FactoryError::Format(
            "this bound CRS has no PROJ string in this build".to_owned(),
        ));
    };
    let mut text = format!("+geoidgrids={grid} +geoid_crs=WGS84");
    if let Some(clause) = vertical_unit_clause(crs.coordinate_system().axis().unit()) {
        text.push(' ');
        text.push_str(&clause);
    }
    text.push_str(" +no_defs +type=crs");
    Ok(text)
}

fn projected_bound_proj(
    crs: &ProjectedCRS,
    towgs84: Option<&[f64]>,
    nadgrids: Option<&str>,
) -> Result<String, FactoryError> {
    if towgs84.is_none() && nadgrids.is_none() {
        return crs.export_to_proj_string();
    }
    let mut text = crs.deriving_conversion().export_to_proj_string()?;
    text.push(' ');
    text.push_str(&ellipsoid_datum_clause(crs.base_crs()));
    if let Some(grid) = nadgrids {
        text.push_str(" +nadgrids=");
        text.push_str(grid);
    } else if let Some(values) = towgs84 {
        text.push_str(" +towgs84=");
        text.push_str(&format_towgs84(values));
    }
    if let Some(unit) = crs.coordinate_system().axes().first().map(Axis::unit) {
        if let Some(clause) = linear_unit_clause(unit) {
            text.push(' ');
            text.push_str(&clause);
        }
    }
    text.push_str(" +no_defs +type=crs");
    Ok(text)
}

fn ellipsoid_datum_clause(crs: &GeodeticCRS) -> String {
    let mut parts = Vec::new();
    if let Some(ellipsoid) = crs.ellipsoid() {
        parts.push(ellipsoid_proj(ellipsoid));
    }
    if let Some(meridian) = crs.prime_meridian() {
        if let Some(clause) = prime_meridian_proj(meridian) {
            parts.push(clause);
        }
    }
    parts.join(" ")
}

fn format_towgs84(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| trim_proj_number(*value))
        .collect::<Vec<_>>()
        .join(",")
}

impl ProjectedCRS {
    /// `exportToPROJString` for a projected CRS: the conversion, the base datum, and the linear unit.
    ///
    /// # Errors
    ///
    /// The deriving conversion cannot be written as a PROJ string.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        let mut text = self.deriving_conversion().export_to_proj_string()?;
        text.push(' ');
        text.push_str(&crs_datum_clause(self.base_crs()));
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
    if geographic
        .coordinate_system()
        .axes()
        .first()
        .is_some_and(|axis| axis.unit().name() == "degree")
    {
        steps.push("+proj=unitconvert +xy_in=deg +xy_out=rad".to_owned());
    }
    // A non-Greenwich source is inverted `longlat` before the projection.
    // Greenwich omits that step (`omitProjLongLatIfPossible`).
    if let Some(step) = inverse_longlat(base) {
        steps.push(step);
    }
    let mut projection = projected.deriving_conversion().export_to_proj_string()?;
    append_ellps_pm(&mut projection, base);
    steps.push(projection);
    if let Some(order) = projected_axis_order(projected) {
        steps.push(format!("+proj=axisswap +order={order}"));
    }
    Ok(pipeline(&steps))
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
    if geographic
        .coordinate_system()
        .axes()
        .first()
        .is_some_and(|axis| axis.unit().name() == "degree")
    {
        steps.push("+proj=unitconvert +xy_in=deg +xy_out=rad".to_owned());
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

fn geographic_of(crs: &Crs) -> Option<&GeographicCRS> {
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
fn projected_axis_order(crs: &ProjectedCRS) -> Option<String> {
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

fn crs_datum_clause(crs: &GeodeticCRS) -> String {
    if let GeodeticCRS::Geographic(geographic) = crs {
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
    parts.join(" ")
}

fn linear_unit_clause(unit: &UnitOfMeasure) -> Option<String> {
    match unit.name() {
        "metre" | "meter" | "Meter" => Some("+units=m".to_owned()),
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

fn geographic_proj(crs: &GeographicCRS) -> String {
    let mut parts = vec!["+proj=longlat".to_owned()];
    match datum_token(crs) {
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

fn ellipsoid_of(crs: &GeographicCRS) -> Option<&Ellipsoid> {
    if let Some(datum) = crs.datum() {
        return Some(datum.ellipsoid());
    }
    geodetic_member(crs.datum_ensemble()?).map(super::model::GeodeticReferenceFrame::ellipsoid)
}

fn meridian_of(crs: &GeographicCRS) -> Option<&PrimeMeridian> {
    if let Some(datum) = crs.datum() {
        return Some(datum.prime_meridian());
    }
    geodetic_member(crs.datum_ensemble()?).map(super::model::GeodeticReferenceFrame::prime_meridian)
}

fn geodetic_member(ensemble: &DatumEnsemble) -> Option<&super::model::GeodeticReferenceFrame> {
    ensemble.members().iter().find_map(|member| match member {
        Datum::Geodetic(frame) => Some(frame),
        _ => None,
    })
}

fn prime_meridian_proj(meridian: &PrimeMeridian) -> Option<String> {
    if meridian.name().eq_ignore_ascii_case("paris")
        || epsg_code(meridian.identifiers()) == Some("8903")
    {
        return Some("+pm=paris".to_owned());
    }
    let degrees = meridian.longitude().convert_to(&UnitOfMeasure::degree());
    if degrees == 0.0 {
        return None;
    }
    // PROJ names a meridian when its longitude matches `proj_list_prime_meridians`
    // within 1e-10 radian. Ferro is 17°40' west.
    let ferro: f64 = -(17.0 + 40.0 / 60.0);
    let radians = degrees.to_radians();
    if meridian.name().eq_ignore_ascii_case("ferro")
        || epsg_code(meridian.identifiers()) == Some("8902")
        || (radians - ferro.to_radians()).abs() < 1e-10
    {
        return Some("+pm=ferro".to_owned());
    }
    Some(format!("+pm={degrees}"))
}

/// Inverted geographic step emitted when the source prime meridian is not Greenwich.
fn inverse_longlat(geographic: &GeographicCRS) -> Option<String> {
    let meridian = meridian_of(geographic)?;
    prime_meridian_proj(meridian)?;
    let mut step = String::from("+inv +proj=longlat");
    append_ellps_pm(&mut step, geographic);
    Some(step)
}

fn append_ellps_pm(step: &mut String, geographic: &GeographicCRS) {
    if let Some(ellipsoid) = geographic.ellipsoid() {
        step.push(' ');
        step.push_str(&ellipsoid_proj(ellipsoid));
    }
    if let Some(meridian) = meridian_of(geographic) {
        if let Some(clause) = prime_meridian_proj(meridian) {
            step.push(' ');
            step.push_str(&clause);
        }
    }
}

fn ellipsoid_proj(ellipsoid: &Ellipsoid) -> String {
    let metres = ellipsoid.semi_major_axis().si_value();
    let inverse = ellipsoid.inverse_flattening();
    let flattening = (inverse != 0.0).then_some(inverse);
    let minor = ellipsoid
        .semi_minor_axis()
        .map(super::model::Measure::si_value);
    if let Some(name) = well_known_ellps(metres, flattening, minor) {
        return format!("+ellps={name}");
    }
    if let Some(inverse) = flattening {
        format!("+a={metres} +rf={inverse}")
    } else if let Some(minor) = minor {
        format!("+a={metres} +b={minor}")
    } else {
        format!("+R={metres}")
    }
}

fn well_known_ellps(major: f64, inverse: Option<f64>, minor: Option<f64>) -> Option<&'static str> {
    const TABLE: &[(&str, f64, f64, bool)] = &[
        ("bessel", 6_377_397.155, 299.152_812_8, true),
        ("intl", 6_378_388.0, 297.0, true),
        ("GRS80", 6_378_137.0, 298.257_222_101, true),
        ("clrk66", 6_378_206.4, 6_356_583.8, false),
        ("clrk80", 6_378_249.145, 293.4663, true),
        ("clrk80ign", 6_378_249.2, 293.466_021_293_626_9, true),
        ("WGS84", 6_378_137.0, 298.257_223_563, true),
        ("sphere", 6_370_997.0, 6_370_997.0, false),
    ];
    let minor = minor.or_else(|| {
        inverse
            .filter(|value| *value != 0.0)
            .map(|value| major * (1.0 - 1.0 / value))
    });
    TABLE
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

/// `PROJStringParser::createFromPROJString` for one step.
///
/// `+proj=longlat` (or `lonlat` / `latlong`) builds a geographic CRS named
/// `unknown`. `+datum=WGS84`, `NAD83`, and `NAD27` use the built-in frames.
/// `+ellps=GRS80`, `WGS84`, or `bessel` builds an `Unknown based on` datum.
/// `+a` and `+rf` build an unnamed ellipsoid. The coordinate system is
/// implicit, so identification accepts a latitude/longitude swap. The default
/// axis order is longitude then latitude; `+axis=` replaces the first two
/// letters.
///
/// Any other single `+proj=` that has a parameter map builds a projected CRS
/// on that geographic base. `+proj=krovak` without `+axis=swu` is Krovak north
/// orientated. A missing Krovak parameter uses the `krovak.cpp` default.
///
/// # Errors
///
/// The projection is unknown, a datum or ellipsoid is unknown, or a number
/// does not parse.
pub fn create_crs_from_proj_string(definition: &str) -> Result<Crs, FactoryError> {
    let tokens = proj_tokens(definition);
    let proj = token(&tokens, "proj");
    if proj == "pipeline" {
        return crs_from_pipeline(&tokens);
    }
    let mut crs = if matches!(proj, "longlat" | "lonlat" | "latlong") {
        geographic_from_proj(&tokens)?
    } else if proj == "utm" {
        utm_from_proj(&tokens)?
    } else {
        projected_from_proj(&tokens, proj)?
    };
    if has_token(&tokens, "towgs84") {
        let values = towgs84_values(token(&tokens, "towgs84"))?;
        crs = Crs::Bound(BoundCRS::create_from_towgs84(crs, &values)?);
    }
    if has_token(&tokens, "geoidgrids") {
        return compound_with_geoid(&tokens, crs);
    }
    Ok(crs)
}

fn towgs84_values(raw: &str) -> Result<Vec<f64>, FactoryError> {
    if raw.is_empty() {
        return Err(FactoryError::Invalid(
            "Invalid number of elements in TOWGS84Parameters".to_owned(),
        ));
    }
    raw.split(',')
        .map(|part| {
            part.parse::<f64>().map_err(|_| {
                FactoryError::Invalid("Invalid number of elements in TOWGS84Parameters".to_owned())
            })
        })
        .collect()
}

fn crs_from_pipeline(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    let mut steps = Vec::new();
    let mut current = Vec::new();
    let mut started = false;
    for (key, value) in tokens {
        if key == "step" {
            if started && !current.is_empty() {
                steps.push(std::mem::take(&mut current));
            }
            started = true;
            continue;
        }
        if started {
            current.push((key.clone(), value.clone()));
        }
    }
    if !current.is_empty() {
        steps.push(current);
    }
    let step = steps.into_iter().find(|step| {
        let proj = token(step, "proj");
        !matches!(
            proj,
            "axisswap" | "unitconvert" | "push" | "pop" | "longlat" | "lonlat" | "latlong" | ""
        )
    });
    let Some(step) = step else {
        return Err(FactoryError::Invalid(
            "a pipeline CRS needs a projection step".to_owned(),
        ));
    };
    projected_from_proj(&step, token(&step, "proj"))
}

fn utm_from_proj(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    let zone = token(tokens, "zone")
        .parse::<i32>()
        .map_err(|_| FactoryError::Invalid("invalid UTM zone".to_owned()))?;
    let conversion = if has_token(tokens, "south") {
        Conversion::create_utm_south(&OpProps::new(), zone)
    } else {
        Conversion::create_utm_north(&OpProps::new(), zone)
    }?;
    let geographic = geographic_base(tokens)?;
    Ok(Crs::Projected(ProjectedCRS::create(
        "unknown",
        GeodeticCRS::Geographic(geographic),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )))
}

fn geographic_for_projected(
    tokens: &[(String, String)],
    proj: &str,
) -> Result<GeographicCRS, FactoryError> {
    Ok(GeographicCRS {
        identified: Identified::named("unknown"),
        datum: Some(datum_for_projected(tokens, proj)?),
        ensemble: None,
        coordinate_system: EllipsoidalCS {
            identified: Identified::default(),
            axes: vec![
                geographic_axis('e', UnitOfMeasure::degree())?,
                geographic_axis('n', UnitOfMeasure::degree())?,
            ],
        },
        implicit_cs: true,
    })
}

fn datum_for_projected(
    tokens: &[(String, String)],
    proj: &str,
) -> Result<GeodeticReferenceFrame, FactoryError> {
    let bare = token(tokens, "datum").is_empty()
        && token(tokens, "ellps").is_empty()
        && token(tokens, "a").is_empty()
        && token(tokens, "b").is_empty()
        && token(tokens, "rf").is_empty();
    if bare && (proj == "krovak" || proj == "mod_krovak") {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on Bessel 1841 ellipsoid",
            Ellipsoid::create_flattened_sphere("Bessel 1841", 6_377_397.155, 299.152_812_8),
            PrimeMeridian::greenwich(),
        ));
    }
    match datum_from_proj(tokens) {
        Ok(datum) => Ok(datum),
        Err(_) if bare => Ok(GeodeticReferenceFrame::epsg_6326()),
        Err(error) => Err(error),
    }
}

fn geographic_base(tokens: &[(String, String)]) -> Result<GeographicCRS, FactoryError> {
    Ok(GeographicCRS {
        identified: Identified::named("unknown"),
        datum: Some(datum_from_proj(tokens)?),
        ensemble: None,
        coordinate_system: EllipsoidalCS {
            identified: Identified::default(),
            axes: vec![
                geographic_axis('e', UnitOfMeasure::degree())?,
                geographic_axis('n', UnitOfMeasure::degree())?,
            ],
        },
        implicit_cs: true,
    })
}

fn has_token(tokens: &[(String, String)], key: &str) -> bool {
    tokens.iter().any(|(name, _)| name == key)
}

fn compound_with_geoid(tokens: &[(String, String)], horizontal: Crs) -> Result<Crs, FactoryError> {
    let grids = token(tokens, "geoidgrids");
    let vertical = VerticalCRS::create(
        "unknown",
        VerticalReferenceFrame::named(format!("unknown using geoidgrids={grids}")),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    );
    Ok(Crs::Compound(CompoundCRS::create(
        "unknown",
        vec![horizontal, Crs::Vertical(vertical)],
    )?))
}

fn geographic_from_proj(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    let datum = datum_from_proj(tokens)?;
    let axis = token(tokens, "axis");
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

fn projected_from_proj(tokens: &[(String, String)], proj: &str) -> Result<Crs, FactoryError> {
    let method = projection_method(proj, tokens).ok_or_else(|| {
        FactoryError::Invalid(format!("unknown +proj={proj} cannot be imported as a CRS"))
    })?;
    let geographic = geographic_for_projected(tokens, proj)?;
    let linear = linear_unit_from_proj(tokens);
    let mut parameters = Vec::new();
    for param in method.params {
        if let Some(parameter) = proj_parameter(proj, param, tokens, &linear)? {
            parameters.push(parameter);
        }
    }
    let code = if method.epsg_code == 0 {
        String::new()
    } else {
        method.epsg_code.to_string()
    };
    let conversion = Conversion {
        identified: Identified::named("unknown"),
        method: OperationMethod {
            name: method.wkt2_name.to_owned(),
            code_space: if code.is_empty() {
                String::new()
            } else {
                "EPSG".to_owned()
            },
            code,
            parameters: Vec::new(),
        },
        parameters,
        version: None,
    };
    let title = token(tokens, "title");
    let name = if title.is_empty() { "unknown" } else { title };
    let mut crs = ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(geographic),
        conversion,
        CartesianCS::create_easting_northing(linear),
    );
    crs.implicit_cs = true;
    Ok(Crs::Projected(crs))
}

fn projection_method(
    proj: &str,
    tokens: &[(String, String)],
) -> Option<&'static super::parammap::MethodMap> {
    let axis = token(tokens, "axis");
    if proj == "krovak" && axis == "swu" {
        return super::parammap::method_by_code(9819);
    }
    if proj == "mod_krovak" && axis == "swu" {
        return super::parammap::method_by_code(1042);
    }
    if proj == "omerc" {
        if has_token(tokens, "no_uoff") || has_token(tokens, "no_off") {
            return super::parammap::method_by_code(9812);
        }
        return super::parammap::method_by_code(9815);
    }
    if proj == "stere" {
        let latitude = token(tokens, "lat_0").parse::<f64>().unwrap_or(0.0);
        if (latitude.abs() - 90.0).abs() < 1e-10 {
            let scale = {
                let k = token(tokens, "k");
                if k.is_empty() { token(tokens, "k_0") } else { k }
            };
            let parallel = token(tokens, "lat_ts");
            if !parallel.is_empty() && (scale.is_empty() || scale.parse::<f64>().ok() == Some(1.0)) {
                return super::parammap::method_by_code(9829);
            }
            return super::parammap::method_by_code(9810);
        }
    }
    if proj == "lcc" {
        let lat_0 = token(tokens, "lat_0");
        let lat_1 = token(tokens, "lat_1");
        let lat_2 = token(tokens, "lat_2");
        if lat_2.is_empty() && !lat_0.is_empty() && !lat_1.is_empty() {
            if lat_0 == lat_1 {
                return super::parammap::method_by_code(9801);
            }
            return super::parammap::method_by_code(1102);
        }
        let mut scale = token(tokens, "k");
        if scale.is_empty() {
            scale = token(tokens, "k_0");
        }
        if !scale.is_empty() && scale.parse::<f64>().ok() != Some(1.0) {
            return super::parammap::method_by_code(1051);
        }
        return super::parammap::method_by_code(9802);
    }
    super::parammap::PROJECTION_METHODS
        .iter()
        .find(|method| method.proj_name == Some(proj))
}

fn linear_unit_from_proj(tokens: &[(String, String)]) -> UnitOfMeasure {
    let to_meter = token(tokens, "to_meter");
    if !to_meter.is_empty() {
        if let Ok(factor) = to_meter.parse::<f64>() {
            return UnitOfMeasure::new("unknown", factor, UnitKind::Linear);
        }
    }
    match token(tokens, "units") {
        "ft" | "foot" => UnitOfMeasure::foot(),
        "us-ft" | "us-foot" => UnitOfMeasure::new("US survey foot", 12.0 / 39.37, UnitKind::Linear),
        _ => UnitOfMeasure::metre(),
    }
}

fn proj_parameter(
    proj: &str,
    param: &super::parammap::ParamMap,
    tokens: &[(String, String)],
    linear: &UnitOfMeasure,
) -> Result<Option<Parameter>, FactoryError> {
    let Some(mut value) = proj_param_value(proj, param, tokens)? else {
        return Ok(None);
    };
    let unit = match param.unit {
        UnitKind::Angular => UnitOfMeasure::degree(),
        UnitKind::Linear => {
            if linear.conversion_to_si() != 0.0 {
                value /= linear.conversion_to_si();
            }
            linear.clone()
        }
        UnitKind::Scale => UnitOfMeasure::unity(),
        _ => UnitOfMeasure::unity(),
    };
    let (code_space, code) = if param.epsg_code == 0 {
        (String::new(), String::new())
    } else {
        ("EPSG".to_owned(), param.epsg_code.to_string())
    };
    Ok(Some(Parameter {
        name: param.wkt2_name.to_owned(),
        code_space,
        code,
        value: ParameterValue::Measure(Measure { value, unit }),
    }))
}

fn proj_param_value(
    proj: &str,
    param: &super::parammap::ParamMap,
    tokens: &[(String, String)],
) -> Result<Option<f64>, FactoryError> {
    if let Some(proj_name) = param.proj_name {
        let raw = if proj_name == "k" || proj_name == "k_0" {
            let scale = token(tokens, "k");
            if scale.is_empty() {
                token(tokens, "k_0")
            } else {
                scale
            }
        } else {
            token(tokens, proj_name)
        };
        if !raw.is_empty() {
            return raw
                .parse::<f64>()
                .map(Some)
                .map_err(|_| FactoryError::Invalid(format!("invalid value for {proj_name}")));
        }
    }
    if proj == "omerc" && param.proj_name == Some("gamma") {
        let alpha = token(tokens, "alpha");
        if !alpha.is_empty() {
            return alpha
                .parse::<f64>()
                .map(Some)
                .map_err(|_| FactoryError::Invalid("invalid value for alpha".to_owned()));
        }
    }
    if proj == "krovak" || proj == "mod_krovak" {
        if let Some(value) = krovak_default(param.epsg_code) {
            return Ok(Some(value));
        }
    }
    if param.unit == UnitKind::Scale {
        return Ok(Some(1.0));
    }
    Ok(Some(0.0))
}

/// Defaults from `krovak.cpp`, used when the PROJ string omits the parameter.
fn krovak_default(epsg_code: i32) -> Option<f64> {
    match epsg_code {
        8811 => Some(49.5),
        8833 => Some(24.833_333_333_333_333),
        1036 => Some(30.288_139_752_777_778),
        8818 => Some(78.5),
        8819 => Some(0.9999),
        _ => None,
    }
}

fn proj_tokens(definition: &str) -> Vec<(String, String)> {
    let definition = quote_title(definition);
    let chars: Vec<char> = definition.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }
        if chars[index] == '+' {
            index += 1;
        }
        let start = index;
        while index < chars.len() && chars[index] != '=' && !chars[index].is_whitespace() {
            index += 1;
        }
        let key: String = chars[start..index].iter().collect();
        if key.is_empty() {
            continue;
        }
        if index < chars.len() && chars[index] == '=' {
            index += 1;
            let value = if index < chars.len() && chars[index] == '"' {
                index += 1;
                let value_start = index;
                while index < chars.len() && chars[index] != '"' {
                    index += 1;
                }
                let value: String = chars[value_start..index].iter().collect();
                if index < chars.len() && chars[index] == '"' {
                    index += 1;
                }
                value
            } else {
                let value_start = index;
                while index < chars.len() && !chars[index].is_whitespace() {
                    index += 1;
                }
                chars[value_start..index].iter().collect()
            };
            tokens.push((key, value));
        } else {
            tokens.push((key, String::new()));
        }
    }
    tokens
}

/// `+title=my Plate carree +proj=eqc` keeps the words up to the next `+`.
fn quote_title(definition: &str) -> String {
    let trimmed = definition.trim_start();
    let Some(after) = trimmed.strip_prefix("+title=") else {
        return definition.to_owned();
    };
    if after.starts_with('"') {
        return definition.to_owned();
    }
    let Some(plus) = after.find(" +") else {
        return definition.to_owned();
    };
    if !after[..plus].contains(' ') {
        return definition.to_owned();
    }
    let title = &after[..plus];
    let tail = &after[plus..];
    format!("+title=\"{title}\"{tail}")
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
    let meridian = prime_meridian_from_token(token(tokens, "pm"))?;
    let ellps = token(tokens, "ellps");
    if ellps == "WGS84" {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on WGS 84 ellipsoid",
            Ellipsoid::wgs84(),
            meridian,
        ));
    }
    if ellps == "GRS80" {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on GRS 1980 ellipsoid",
            Ellipsoid::grs80(),
            meridian,
        ));
    }
    if ellps == "bessel" {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on Bessel 1841 ellipsoid",
            Ellipsoid::create_flattened_sphere("Bessel 1841", 6_377_397.155, 299.152_812_8),
            meridian,
        ));
    }
    if ellps == "krass" {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on Krassovsky, 1940 ellipsoid",
            Ellipsoid::create_flattened_sphere("Krassovsky, 1940", 6_378_245.0, 298.3),
            meridian,
        ));
    }
    if ellps == "clrk80" {
        return Ok(GeodeticReferenceFrame::create(
            "Unknown based on Clarke 1880 mod. ellipsoid",
            Ellipsoid::create_flattened_sphere("Clarke 1880 mod.", 6_378_249.145, 293.4663),
            meridian,
        ));
    }
    let major = token(tokens, "a");
    let minor = token(tokens, "b");
    if !major.is_empty() && !minor.is_empty() {
        let major = major
            .parse::<f64>()
            .map_err(|_| FactoryError::Invalid("Invalid a value".to_owned()))?;
        let minor = minor
            .parse::<f64>()
            .map_err(|_| FactoryError::Invalid("Invalid b value".to_owned()))?;
        return Ok(GeodeticReferenceFrame::create(
            "unknown",
            Ellipsoid {
                identified: Identified::named("unknown"),
                semi_major: Measure {
                    value: major,
                    unit: UnitOfMeasure::metre(),
                },
                inverse_flattening: None,
                semi_minor: Some(Measure {
                    value: minor,
                    unit: UnitOfMeasure::metre(),
                }),
                body: "Earth".to_owned(),
            },
            meridian,
        ));
    }
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
            meridian,
        ));
    }
    Err(FactoryError::Invalid(
        "a longlat CRS needs +datum, +ellps, or +a and +rf".to_owned(),
    ))
}

fn prime_meridian_from_token(pm: &str) -> Result<PrimeMeridian, FactoryError> {
    if pm.is_empty() || pm.eq_ignore_ascii_case("greenwich") {
        return Ok(PrimeMeridian::greenwich());
    }
    if pm.eq_ignore_ascii_case("ferro") {
        return Ok(PrimeMeridian {
            identified: Identified::named("Ferro"),
            longitude: Measure {
                value: -(17.0 + 40.0 / 60.0),
                unit: UnitOfMeasure::degree(),
            },
        });
    }
    if pm.eq_ignore_ascii_case("paris") {
        return Ok(PrimeMeridian::paris());
    }
    let degrees = pm
        .parse::<f64>()
        .map_err(|_| FactoryError::Invalid(format!("unknown prime meridian {pm}")))?;
    Ok(PrimeMeridian {
        identified: Identified::named("unknown"),
        longitude: Measure {
            value: degrees,
            unit: UnitOfMeasure::degree(),
        },
    })
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
