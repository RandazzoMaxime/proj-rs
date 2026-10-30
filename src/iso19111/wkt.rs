//! WKT1 and WKT2 export.
//!
//! Node order, identifier suppression, and the simplified omissions follow
//! `WKTFormatter` in PROJ `src/iso19111/io.cpp`.

use super::factory::{AuthorityFactory, DatabaseContext};
use super::wkt_import::WktParser;
/// Whether the node being opened carries an authority identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeId {
    Present,
    Absent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AxisAbbrev {
    Show,
    Hide,
}

fn node_from_ids(identifiers: &[Identifier]) -> NodeId {
    if identifiers.is_empty() {
        NodeId::Absent
    } else {
        NodeId::Present
    }
}

fn node_from_text(text: &str) -> NodeId {
    if text.is_empty() {
        NodeId::Absent
    } else {
        NodeId::Present
    }
}

use super::model::{
    ensemble_name_as_datum, Axis, BoundCRS, CompoundCRS, Conversion, CoordinateSystem, Crs, Datum,
    DatumEnsemble, Ellipsoid, EngineeringCRS, EngineeringDatum, FactoryError, GeocentricCRS,
    GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identified, Identifier, Measure,
    OperationMethod, Parameter, ParameterValue, PrimeMeridian, ProjectedCRS, Transformation,
    UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame,
};

/// WKT dialect.
///
/// `Wkt2_2015` is `GEODCRS`. `Wkt2_2019` is `GEOGCRS` for a geographic CRS.
/// The simplified forms omit Greenwich, metre on the ellipsoid, axis order,
/// and every identifier except the top object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WktConvention {
    /// ISO 19162:2015 full form.
    Wkt2_2015,
    /// ISO 19162:2015 simplified form.
    Wkt2_2015Simplified,
    /// ISO 19162:2019 full form.
    Wkt2_2019,
    /// ISO 19162:2019 simplified form.
    Wkt2_2019Simplified,
    /// WKT1 as written by GDAL.
    Wkt1Gdal,
    /// WKT1 as written by Esri.
    Wkt1Esri,
}

/// Line breaks in WKT text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WktLines {
    /// One node per line, indented.
    Multi,
    /// A single line.
    Single,
}

/// Whether WKT1 rejects constructs the dialect does not define.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WktStrictness {
    /// Reject a geographic 3D CRS in WKT1.
    Strict,
    /// Skip that rejection. The export may still fail for other reasons.
    Lenient,
}

/// WKT1 treatment of ellipsoidal height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EllipsoidalHeightPolicy {
    /// Do not build a compound CRS from ellipsoidal height.
    Reject,
    /// Write a WKT1 `COMPD_CS` whose vertical CRS is the ellipsoidal height.
    AllowAsVertical,
}

/// `LINUNIT` on Esri geographic 3D WKT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinunitPolicy {
    /// Refuse geographic 3D Esri WKT without another representation.
    Reject,
    /// Write `LINUNIT` for the ellipsoidal height unit.
    Allow,
}

/// Whether `AXIS` nodes are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputAxisRule {
    /// Always write axes.
    Yes,
    /// Never write axes.
    No,
    /// WKT1 GDAL: axes on a projected CRS only when the order is east then north.
    Wkt1GdalEpsgStyle,
}

/// Mutable WKT writer. One formatter is used for one export.
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one WKTFormatter::Params switch from PROJ"
)]
pub struct WktFormatter {
    convention: WktConvention,
    wkt2: bool,
    keywords_2019: bool,
    esri: bool,
    multi_line: bool,
    strict: bool,
    indent_width: usize,
    id_on_top_only: bool,
    output_axis_order: bool,
    omit_greenwich: bool,
    omit_ellipsoid_metre: bool,
    omit_parameter_unit_if_axis: bool,
    force_unit_keyword: bool,
    cs_unit_once: bool,
    prime_meridian_degree: bool,
    output_axis: OutputAxisRule,
    allow_ellipsoidal_height: bool,
    allow_linunit: bool,
    result: String,
    indent_level: i32,
    stack_has_child: Vec<bool>,
    stack_has_id: Vec<bool>,
    stack_empty: Vec<bool>,
    output_id: Vec<bool>,
    output_unit: Vec<bool>,
    axis_linear: Vec<UnitOfMeasure>,
    axis_angular: Vec<UnitOfMeasure>,
    abridged: bool,
    /// Nesting of `pushDisableUsage`: while positive, no `USAGE` or `REMARK` is written.
    usage_disabled: usize,
    db: Option<DatabaseContext>,
}

impl WktFormatter {
    /// Formatter for `convention`, with PROJ's default axis and multiline rules.
    #[must_use]
    pub fn create(convention: WktConvention) -> Self {
        let (wkt2, keywords_2019, omissions) = match convention {
            WktConvention::Wkt2_2015 => (true, false, false),
            WktConvention::Wkt2_2015Simplified => (true, false, true),
            WktConvention::Wkt2_2019 => (true, true, false),
            WktConvention::Wkt2_2019Simplified => (true, true, true),
            WktConvention::Wkt1Gdal | WktConvention::Wkt1Esri => (false, false, false),
        };
        let esri = convention == WktConvention::Wkt1Esri;
        Self {
            convention,
            wkt2,
            keywords_2019,
            esri,
            multi_line: !esri,
            strict: true,
            indent_width: 4,
            id_on_top_only: omissions,
            output_axis_order: wkt2 && !omissions,
            omit_greenwich: omissions,
            omit_ellipsoid_metre: omissions,
            omit_parameter_unit_if_axis: omissions,
            force_unit_keyword: omissions || !wkt2,
            cs_unit_once: omissions,
            prime_meridian_degree: !wkt2,
            output_axis: match convention {
                WktConvention::Wkt1Gdal => OutputAxisRule::Wkt1GdalEpsgStyle,
                WktConvention::Wkt1Esri => OutputAxisRule::No,
                _ => OutputAxisRule::Yes,
            },
            allow_ellipsoidal_height: false,
            allow_linunit: esri,
            result: String::new(),
            indent_level: 0,
            stack_has_child: Vec::new(),
            stack_has_id: vec![false],
            stack_empty: Vec::new(),
            output_id: vec![true],
            output_unit: vec![true],
            axis_linear: vec![UnitOfMeasure::metre()],
            axis_angular: vec![UnitOfMeasure::degree()],
            abridged: false,
            usage_disabled: 0,
            db: None,
        }
    }

    /// Attach `proj.db` so Esri aliases can be read while exporting.
    #[must_use]
    pub fn with_database(mut self, db: DatabaseContext) -> Self {
        self.db = Some(db);
        self
    }

    /// Multi-line or single-line output. Esri WKT1 defaults to one line.
    pub fn set_lines(&mut self, lines: WktLines) -> &mut Self {
        self.multi_line = lines == WktLines::Multi;
        self
    }

    /// Axis policy. WKT1 GDAL defaults to east/north only.
    pub fn set_output_axis(&mut self, rule: OutputAxisRule) -> &mut Self {
        self.output_axis = rule;
        self
    }

    /// Strict WKT1 rejects a geographic 3D CRS. Lenient mode does not add that check.
    pub fn set_strictness(&mut self, strictness: WktStrictness) -> &mut Self {
        self.strict = strictness == WktStrictness::Strict;
        self
    }

    /// Whether a 3D geographic CRS may be considered for a WKT1 compound export.
    pub fn set_ellipsoidal_height(&mut self, policy: EllipsoidalHeightPolicy) -> &mut Self {
        self.allow_ellipsoidal_height = policy == EllipsoidalHeightPolicy::AllowAsVertical;
        self
    }

    /// Whether Esri geographic 3D output may contain `LINUNIT`.
    pub fn set_linunit(&mut self, policy: LinunitPolicy) -> &mut Self {
        self.allow_linunit = policy == LinunitPolicy::Allow;
        self
    }

    /// Spaces per indentation level. PROJ's default is 4.
    pub fn set_indentation_width(&mut self, width: usize) -> &mut Self {
        self.indent_width = width;
        self
    }

    /// Text written so far.
    #[must_use]
    pub fn to_wkt(&self) -> &str {
        &self.result
    }

    /// Suppress identifiers below the current node, as `simulCurNodeHasId` does.
    pub fn simul_cur_node_has_id(&mut self) -> &mut Self {
        if let Some(flag) = self.stack_has_id.last_mut() {
            *flag = true;
        }
        self
    }

    /// Hide authority nodes. Only valid before the first node.
    ///
    /// # Errors
    ///
    /// A node has already been opened.
    pub fn hide_identifiers(&mut self) -> Result<&mut Self, FactoryError> {
        if self.indent_level != 0 {
            return Err(FactoryError::Invalid(
                "setOutputId() shall only be called when the stack state is empty".to_owned(),
            ));
        }
        if let Some(flag) = self.output_id.first_mut() {
            *flag = false;
        }
        Ok(self)
    }

    /// Convention this formatter was created with.
    #[must_use]
    pub fn convention(&self) -> WktConvention {
        self.convention
    }

    fn output_id_now(&self) -> bool {
        !self.esri && *self.output_id.last().unwrap_or(&false)
    }

    fn output_unit_now(&self) -> bool {
        *self.output_unit.last().unwrap_or(&false)
    }

    fn start_node(&mut self, keyword: &str, id: NodeId) {
        let has_id = id == NodeId::Present;
        if let Some(has_child) = self.stack_has_child.last_mut() {
            if *has_child {
                self.result.push(',');
            }
            *has_child = true;
        } else if !self.result.is_empty() {
            self.result.push(',');
            if self.multi_line && !keyword.is_empty() {
                self.result.push('\n');
            }
        }
        if self.multi_line && (self.indent_level != 0) && !keyword.is_empty() {
            if !self.result.is_empty() {
                self.result.push('\n');
            }
            self.write_indent();
        }
        if !keyword.is_empty() {
            self.result.push_str(keyword);
            self.result.push('[');
        }
        self.indent_level += 1;
        self.stack_has_child.push(false);
        self.stack_empty.push(keyword.is_empty());
        let parent_has_id = *self.stack_has_id.last().unwrap_or(&false);
        let top_id = self.output_id.first().copied().unwrap_or(true);
        let next_id = if !self.id_on_top_only
            && self.indent_level >= 2
            && self.wkt2
            && (keyword == "METHOD" || keyword == "PARAMETER")
        {
            top_id
        } else if self.indent_level >= 2 && self.wkt2 {
            top_id && !parent_has_id
        } else {
            self.output_id_now() || self.esri && top_id
        };
        // `output_id_now` is false for Esri, and the non-WKT2 branch must keep
        // the previous flag. Esri never prints identifiers.
        let next_id = if self.esri { false } else { next_id };
        self.output_id.push(next_id);
        self.stack_has_id.push(has_id || parent_has_id);
    }

    fn end_node(&mut self) {
        self.stack_has_id.pop();
        self.output_id.pop();
        self.indent_level -= 1;
        let empty = self.stack_empty.pop().unwrap_or(false);
        self.stack_has_child.pop();
        if !empty {
            self.result.push(']');
        }
    }

    fn write_indent(&mut self) {
        let width = self
            .indent_width
            .saturating_mul(usize::try_from(self.indent_level).unwrap_or(0));
        self.result.extend(std::iter::repeat_n(' ', width));
    }

    fn start_child(&mut self) {
        if let Some(has_child) = self.stack_has_child.last_mut() {
            if *has_child {
                self.result.push(',');
            }
            *has_child = true;
        }
    }

    fn add_quoted(&mut self, text: &str) {
        self.start_child();
        self.result.push('"');
        for ch in text.chars() {
            if ch == '"' {
                self.result.push('"');
            }
            self.result.push(ch);
        }
        self.result.push('"');
    }

    fn add_raw(&mut self, text: &str) {
        self.start_child();
        self.result.push_str(text);
    }

    fn add_number(&mut self, value: f64) {
        self.start_child();
        self.result.push_str(&format_number(value, self));
    }

    fn format_ids(&mut self, identifiers: &[Identifier]) {
        if !self.output_id_now() {
            return;
        }
        for identifier in identifiers {
            let (code_space, version) = self.split_authority(&identifier.code_space);
            self.start_node(if self.wkt2 { "ID" } else { "AUTHORITY" }, NodeId::Absent);
            self.add_quoted(&code_space);
            if self.wkt2 && is_integer_code(&identifier.code) {
                self.add_raw(&identifier.code);
            } else {
                self.add_quoted(&identifier.code);
            }
            if let Some(version) = version.filter(|_| self.wkt2) {
                if is_number_token(&version) {
                    self.add_raw(&version);
                } else {
                    self.add_quoted(&version);
                }
            }
            self.end_node();
            if !self.wkt2 {
                break;
            }
        }
    }

    /// `IAU_2015` becomes authority `IAU` and version `2015` when `proj.db` says so.
    fn split_authority(&self, code_space: &str) -> (String, Option<String>) {
        if let Some(db) = &self.db {
            if let Some((authority, version)) = db.authority_and_version(code_space) {
                return (authority, Some(version));
            }
        }
        (code_space.to_owned(), None)
    }

    fn export_unit(&mut self, unit: &UnitOfMeasure, keyword: Option<&str>) {
        let has_id = !unit.code_space.is_empty();
        let node = if let Some(keyword) = keyword {
            keyword
        } else if self.force_unit_keyword && unit.kind != UnitKind::None {
            "UNIT"
        } else {
            match (self.wkt2, unit.kind) {
                (true, UnitKind::Linear) => "LENGTHUNIT",
                (true, UnitKind::Angular) => "ANGLEUNIT",
                (true, UnitKind::Scale) => "SCALEUNIT",
                (true, UnitKind::Time) => "TIMEUNIT",
                _ => "UNIT",
            }
        };
        self.start_node(node, node_from_text(&unit.code_space));
        self.add_quoted(&esri_unit_name(&unit.name, self));
        if !(self.wkt2 && unit.kind == UnitKind::Time && unit.factor == 0.0) {
            self.add_number(unit.factor);
        }
        if has_id && self.output_id_now() {
            self.start_node(if self.wkt2 { "ID" } else { "AUTHORITY" }, NodeId::Absent);
            self.add_quoted(&unit.code_space);
            if self.wkt2 && is_integer_code(&unit.code) {
                self.add_raw(&unit.code);
            } else {
                self.add_quoted(&unit.code);
            }
            self.end_node();
        }
        self.end_node();
    }

    fn base_usage(&mut self, identified: &Identified) {
        let usage = self.wkt2 && self.output_id_now() && self.usage_disabled == 0;
        if usage {
            if self.keywords_2019 {
                for domain in &identified.domains {
                    self.start_node("USAGE", NodeId::Absent);
                    self.export_domain(domain);
                    self.end_node();
                }
            } else if let Some(domain) = identified.domains.first() {
                self.export_domain(domain);
            }
        }
        self.format_ids(&identified.identifiers);
        if usage && !identified.remarks.is_empty() {
            self.start_node("REMARK", NodeId::Absent);
            self.add_quoted(&identified.remarks);
            self.end_node();
        }
    }

    fn export_domain(&mut self, domain: &super::model::ObjectDomain) {
        self.start_node("SCOPE", NodeId::Absent);
        self.add_quoted(domain.scope.as_deref().unwrap_or("unknown"));
        self.end_node();
        if let Some(area) = &domain.area {
            self.start_node("AREA", NodeId::Absent);
            self.add_quoted(area);
            self.end_node();
        }
        if let (Some(south), Some(west), Some(north), Some(east)) =
            (domain.south, domain.west, domain.north, domain.east)
        {
            self.start_node("BBOX", NodeId::Absent);
            self.add_number(south);
            self.add_number(west);
            self.add_number(north);
            self.add_number(east);
            self.end_node();
        }
    }
}

impl Conversion {
    /// Write this conversion into `formatter` and return the WKT.
    ///
    /// # Errors
    ///
    /// WKT1 has no spelling for the method, or a parameter cannot be written.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_conversion(self, formatter, BaseShape::NotSphere)?;
        Ok(std::mem::take(&mut formatter.result))
    }
}

impl super::model::CoordinateOperation {
    /// Write this operation into `formatter` and return the WKT. A concatenated
    /// operation has no single-node spelling here and is refused.
    ///
    /// # Errors
    ///
    /// The operation cannot be written in the formatter's convention.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_operation(self, formatter)?;
        Ok(std::mem::take(&mut formatter.result))
    }
}

impl Crs {
    /// Write this CRS into `formatter` and return the WKT.
    ///
    /// # Errors
    ///
    /// Strict WKT1 refuses a geographic 3D CRS, and WKT1 refuses a projected
    /// 3D CRS. WKT2:2019 accepts both.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_crs(self, formatter)?;
        Ok(std::mem::take(&mut formatter.result))
    }
}

pub(crate) fn export_crs(crs: &Crs, formatter: &mut WktFormatter) -> Result<(), FactoryError> {
    match crs {
        Crs::Geographic(crs) => export_geographic(crs, formatter)?,
        Crs::Geodetic(GeodeticCRS::Geographic(crs)) => export_geographic(crs, formatter)?,
        Crs::Geodetic(GeodeticCRS::Geocentric(crs)) => export_geocentric(crs, formatter)?,
        Crs::Geodetic(GeodeticCRS::Spherical(crs)) => export_spherical(
            &crs.identified,
            crs.datum.as_ref(),
            crs.ensemble.as_ref(),
            &crs.coordinate_system,
            formatter,
        )?,
        Crs::Projected(crs) => export_projected(crs, formatter)?,
        Crs::Vertical(crs) => export_vertical(crs, formatter)?,
        Crs::Compound(crs) => export_compound(crs, formatter)?,
        Crs::Bound(crs) => export_bound(crs, formatter)?,
        Crs::Engineering(crs) => export_engineering(crs, formatter)?,
        Crs::DerivedProjected(crs) => {
            return Err(FactoryError::Format(format!(
                "derived projected CRS {} has no WKT export in this build",
                crs.name()
            )));
        }
    }
    Ok(())
}

fn export_ellipsoidal_height_compound(
    crs: &GeographicCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let flat = horizontal_for_compound(crs, formatter);
    let Some(height) = crs.coordinate_system.axes.get(2) else {
        return Err(FactoryError::Format(
            "a geographic 3D CRS has no ellipsoidal height axis".to_owned(),
        ));
    };
    let vertical_name = format!("Ellipsoid ({})", height.unit().name());
    let compound_name = format!("{} + {vertical_name}", crs.name());
    let vertical = VerticalCRS {
        identified: Identified::named(vertical_name),
        datum: Some(VerticalReferenceFrame {
            identified: Identified::named("Ellipsoid"),
            wkt1_type: "2002".to_owned(),
            frame_epoch: None,
            deformation_model: None,
        }),
        ensemble: None,
        coordinate_system: VerticalCS {
            identified: Identified::default(),
            axis: height.clone(),
        },
        geoid_models: Vec::new(),
    };
    formatter.start_node("COMPD_CS", NodeId::Absent);
    formatter.add_quoted(&compound_name);
    export_geographic(&flat, formatter)?;
    export_vertical(&vertical, formatter)?;
    formatter.end_node();
    Ok(())
}

/// PROJ demotes a geographic 3D CRS to the database 2D CRS of the same name
/// when that 2D CRS is the horizontal part. Otherwise the copy has no authority.
fn horizontal_for_compound(crs: &GeographicCRS, formatter: &WktFormatter) -> GeographicCRS {
    if let (Some(db), Some(identifier)) = (&formatter.db, crs.identifiers().first()) {
        if let Ok(Some(found)) = AuthorityFactory::create(db, identifier.code_space())
            .create_geographic_2d_crs_from_name(crs.name())
        {
            if found.is_2d_part_of_3d(crs) {
                return found;
            }
        }
    }
    let mut flat = crs.clone();
    flat.coordinate_system.axes.truncate(2);
    flat.identified.identifiers.clear();
    flat
}

fn export_geographic(
    crs: &GeographicCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let axes = &crs.coordinate_system.axes;
    let three_d = axes.len() == 3;
    if !formatter.wkt2
        && !formatter.esri
        && formatter.strict
        && three_d
        && formatter.output_axis != OutputAxisRule::No
    {
        if formatter.allow_ellipsoidal_height {
            return export_ellipsoidal_height_compound(crs, formatter);
        }
        return Err(FactoryError::Format(
            "WKT1 does not support a geographic 3D CRS".to_owned(),
        ));
    }
    if !formatter.wkt2 && formatter.esri && three_d && !formatter.allow_linunit {
        return export_esri_geographic_3d_as_vertcs(crs, formatter);
    }
    let keyword = if formatter.wkt2 {
        if formatter.keywords_2019 {
            "GEOGCRS"
        } else {
            "GEODCRS"
        }
    } else {
        "GEOGCS"
    };
    let mut name = crs.identified.name.clone();
    if formatter.esri {
        name = esri_geographic_name(&crs.identified, axes.len(), formatter);
    } else if !formatter.wkt2 && crs.identified.deprecated {
        name.push_str(" (deprecated)");
    }
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&name);
    if let Some(unit) = axes.first().map(Axis::unit) {
        formatter.axis_angular.push(unit.clone());
    }
    export_geodetic_anchor(crs.datum.as_ref(), crs.ensemble.as_ref(), formatter);
    if let Some(meridian) = geodetic_meridian(crs.datum.as_ref(), crs.ensemble.as_ref()) {
        export_prime_meridian(meridian, formatter);
    }
    formatter.axis_angular.pop();
    if !formatter.wkt2 {
        if let Some(unit) = axes.first().map(Axis::unit) {
            formatter.export_unit(unit, None);
        }
    }
    if three_d && formatter.esri {
        if let Some(axis) = axes.get(2) {
            formatter.export_unit(axis.unit(), Some("LINUNIT"));
        }
    }
    export_cs(
        &CoordinateSystem::Ellipsoidal(crs.coordinate_system.clone()),
        formatter,
    );
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

/// ESRI has no `LINUNIT`. A geographic 3D CRS is `GEOGCS` plus `VERTCS`.
fn export_esri_geographic_3d_as_vertcs(
    crs: &GeographicCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let mut flat = crs.clone();
    flat.coordinate_system.axes.truncate(2);
    export_geographic(&flat, formatter)?;
    let Some(height) = crs.coordinate_system.axes.get(2) else {
        return Err(FactoryError::Format(
            "a geographic 3D CRS has no ellipsoidal height axis".to_owned(),
        ));
    };
    let mut vert_name = if let Some(datum) = crs.datum.as_ref() {
        wkt1_datum_name(&datum.identified, formatter)
    } else if let Some(ensemble) = crs.ensemble.as_ref() {
        wkt1_datum_name(&ensemble.identified, formatter)
    } else {
        crs.name().to_owned()
    };
    if let Some(rest) = vert_name.strip_prefix("GCS_") {
        vert_name = rest.to_owned();
    }
    formatter.start_node("VERTCS", NodeId::Absent);
    formatter.add_quoted(&vert_name);
    if let Some(datum) = crs.datum.as_ref() {
        export_geodetic_datum(datum, formatter);
    } else if let Some(ensemble) = crs.ensemble.as_ref() {
        export_ensemble_as_datum(ensemble, formatter);
    }
    formatter.start_node("PARAMETER", NodeId::Absent);
    formatter.add_quoted("Vertical_Shift");
    formatter.add_number(0.0);
    formatter.end_node();
    formatter.start_node("PARAMETER", NodeId::Absent);
    formatter.add_quoted("Direction");
    formatter.add_number(if height.direction() == "down" {
        -1.0
    } else {
        1.0
    });
    formatter.end_node();
    formatter.export_unit(height.unit(), None);
    formatter.end_node();
    Ok(())
}

fn export_geocentric(
    crs: &GeocentricCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    if formatter.esri {
        return Err(FactoryError::Format(
            "a geocentric CRS is not supported in WKT1 Esri".to_owned(),
        ));
    }
    let keyword = if formatter.wkt2 { "GEODCRS" } else { "GEOCCS" };
    let mut name = crs.identified.name.clone();
    if !formatter.wkt2 && crs.identified.deprecated {
        name.push_str(" (deprecated)");
    }
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&name);
    if let Some(unit) = crs.coordinate_system.axes.first().map(Axis::unit) {
        formatter.axis_angular.push(UnitOfMeasure::degree());
        let _ = unit;
    }
    export_geodetic_anchor(crs.datum.as_ref(), crs.ensemble.as_ref(), formatter);
    if let Some(meridian) = geodetic_meridian(crs.datum.as_ref(), crs.ensemble.as_ref()) {
        export_prime_meridian(meridian, formatter);
    }
    formatter.axis_angular.pop();
    if !formatter.wkt2 {
        if let Some(unit) = crs.coordinate_system.axes.first().map(Axis::unit) {
            formatter.export_unit(unit, None);
        }
    }
    let previous = formatter.output_axis;
    if previous == OutputAxisRule::Wkt1GdalEpsgStyle {
        formatter.output_axis = OutputAxisRule::Yes;
    }
    export_cs(
        &CoordinateSystem::Cartesian(crs.coordinate_system.clone()),
        formatter,
    );
    formatter.output_axis = previous;
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

fn export_spherical(
    identified: &Identified,
    datum: Option<&GeodeticReferenceFrame>,
    ensemble: Option<&DatumEnsemble>,
    cs: &super::model::SphericalCS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let keyword = if formatter.wkt2 { "GEODCRS" } else { "GEOGCS" };
    formatter.start_node(keyword, node_from_ids(&identified.identifiers));
    formatter.add_quoted(&identified.name);
    export_geodetic_anchor(datum, ensemble, formatter);
    if let Some(meridian) = geodetic_meridian(datum, ensemble) {
        export_prime_meridian(meridian, formatter);
    }
    export_cs(&CoordinateSystem::Spherical(cs.clone()), formatter);
    formatter.base_usage(identified);
    formatter.end_node();
    Ok(())
}

fn export_projected(crs: &ProjectedCRS, formatter: &mut WktFormatter) -> Result<(), FactoryError> {
    if let Some(text) = esri_projected_text(crs, formatter) {
        formatter.result.push_str(&text);
        return Ok(());
    }
    let axes = &crs.coordinate_system.axes;
    if axes.len() == 3 && !(formatter.wkt2 && formatter.keywords_2019) {
        if !formatter.wkt2 && !formatter.esri && formatter.allow_ellipsoidal_height {
            return export_projected_ellipsoidal_height(crs, formatter);
        }
        return Err(FactoryError::Format(
            "a projected 3D CRS can only be exported as WKT2:2019".to_owned(),
        ));
    }
    let keyword = if formatter.wkt2 { "PROJCRS" } else { "PROJCS" };
    let mut name = crs.identified.name.clone();
    if formatter.esri {
        name = esri_projected_name(&crs.identified, formatter);
    } else if !formatter.wkt2 && crs.identified.deprecated {
        name.push_str(" (deprecated)");
    }
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&name);
    let angular = base_angular_unit(&crs.base);
    if formatter.wkt2 {
        let base_keyword =
            if formatter.keywords_2019 && matches!(crs.base, GeodeticCRS::Geographic(_)) {
                "BASEGEOGCRS"
            } else {
                "BASEGEODCRS"
            };
        let base_has_id = formatter.keywords_2019 && !crs.base.identifiers().is_empty();
        formatter.start_node(
            base_keyword,
            if base_has_id {
                NodeId::Present
            } else {
                NodeId::Absent
            },
        );
        formatter.add_quoted(crs.base.name());
        export_base_datum(&crs.base, formatter);
        if formatter.omit_parameter_unit_if_axis {
            formatter.export_unit(&angular, None);
        }
        if let Some(meridian) = crs.base.prime_meridian() {
            formatter.axis_angular.push(angular.clone());
            export_prime_meridian(meridian, formatter);
            formatter.axis_angular.pop();
        }
        if formatter.keywords_2019 && !(formatter.id_on_top_only && top_has_id(formatter)) {
            formatter.output_id.push(true);
            formatter.format_ids(crs.base.identifiers());
            formatter.output_id.pop();
        }
        formatter.end_node();
    } else {
        let previous = formatter.output_axis;
        formatter.output_axis = OutputAxisRule::No;
        match &crs.base {
            GeodeticCRS::Geographic(base) => export_geographic(base, formatter)?,
            GeodeticCRS::Geocentric(base) => export_geocentric(base, formatter)?,
            GeodeticCRS::Spherical(crs) => export_spherical(
                &crs.identified,
                crs.datum.as_ref(),
                crs.ensemble.as_ref(),
                &crs.coordinate_system,
                formatter,
            )?,
        }
        formatter.output_axis = previous;
    }
    if let Some(linear) = axes.first().map(Axis::unit) {
        formatter.axis_linear.push(linear.clone());
    }
    formatter.axis_angular.push(angular);
    let base_shape = if crs.base.ellipsoid().is_some_and(Ellipsoid::is_sphere) {
        BaseShape::Sphere
    } else {
        BaseShape::NotSphere
    };
    export_conversion(&crs.conversion, formatter, base_shape)?;
    formatter.axis_angular.pop();
    formatter.axis_linear.pop();
    if !formatter.wkt2 {
        if let Some(unit) = axes.first().map(Axis::unit) {
            formatter.export_unit(unit, None);
        }
    }
    let previous = formatter.output_axis;
    if previous == OutputAxisRule::Wkt1GdalEpsgStyle {
        let east_north =
            axes.len() >= 2 && axes[0].direction() == "east" && axes[1].direction() == "north";
        if east_north {
            formatter.output_axis = OutputAxisRule::Yes;
        }
    }
    export_cs(
        &CoordinateSystem::Cartesian(crs.coordinate_system.clone()),
        formatter,
    );
    formatter.output_axis = previous;
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

/// WKT1 GDAL writes a projected 3D CRS as `COMPD_CS` plus `VERT_CS["Ellipsoid"]`.
fn export_projected_ellipsoidal_height(
    crs: &ProjectedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let mut flat = crs.demote_to_2d(formatter.db.as_ref());
    if let Some(db) = &formatter.db {
        let found = flat.identify_with(Some(&AuthorityFactory::create(db, "EPSG")));
        if found.len() == 1 && found[0].1 == 100 {
            if let Crs::Projected(official) = &found[0].0 {
                flat = official.clone();
            }
        }
    }
    let Some(height) = crs.coordinate_system.axes.get(2) else {
        return Err(FactoryError::Format(
            "a projected 3D CRS has no ellipsoidal height axis".to_owned(),
        ));
    };
    let vertical_name = format!("Ellipsoid ({})", height.unit().name());
    let compound_name = format!("{} + {vertical_name}", flat.name());
    let vertical = VerticalCRS {
        identified: Identified::named(vertical_name),
        datum: Some(VerticalReferenceFrame {
            identified: Identified::named("Ellipsoid"),
            wkt1_type: "2002".to_owned(),
            frame_epoch: None,
            deformation_model: None,
        }),
        ensemble: None,
        coordinate_system: VerticalCS {
            identified: Identified::default(),
            axis: height.clone(),
        },
        geoid_models: Vec::new(),
    };
    formatter.start_node("COMPD_CS", NodeId::Absent);
    formatter.add_quoted(&compound_name);
    export_projected(&flat, formatter)?;
    export_vertical(&vertical, formatter)?;
    formatter.end_node();
    Ok(())
}

fn export_vertical(crs: &VerticalCRS, formatter: &mut WktFormatter) -> Result<(), FactoryError> {
    let keyword = if formatter.wkt2 {
        "VERTCRS"
    } else if formatter.esri {
        "VERTCS"
    } else {
        "VERT_CS"
    };
    let mut name = crs.identified.name.clone();
    if formatter.esri {
        if let Some(alias) = esri_alias(&crs.identified, "vertical_crs", formatter) {
            name = alias;
        } else {
            name = morph_name_to_esri(&name);
        }
    }
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&name);
    if let Some(datum) = &crs.datum {
        export_vertical_datum(datum, formatter);
    } else if let Some(ensemble) = &crs.ensemble {
        export_vertical_ensemble(ensemble, formatter);
    }
    if formatter.esri {
        formatter.start_node("PARAMETER", NodeId::Absent);
        formatter.add_quoted("Vertical_Shift");
        formatter.add_number(0.0);
        formatter.end_node();
        formatter.start_node("PARAMETER", NodeId::Absent);
        formatter.add_quoted("Direction");
        let up = crs.coordinate_system.axis.direction() == "up";
        formatter.add_number(if up { 1.0 } else { -1.0 });
        formatter.end_node();
    }
    if !formatter.wkt2 {
        formatter.export_unit(crs.coordinate_system.axis.unit(), None);
    }
    let previous = formatter.output_axis;
    if previous == OutputAxisRule::Wkt1GdalEpsgStyle {
        formatter.output_axis = OutputAxisRule::Yes;
    }
    export_cs(
        &CoordinateSystem::Vertical(crs.coordinate_system.clone()),
        formatter,
    );
    formatter.output_axis = previous;
    if formatter.wkt2 && formatter.keywords_2019 {
        for model in &crs.geoid_models {
            formatter.start_node("GEOIDMODEL", NodeId::Absent);
            formatter.add_quoted(&model.name);
            formatter.format_ids(&model.identifiers);
            formatter.end_node();
        }
    }
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

fn export_compound(crs: &CompoundCRS, formatter: &mut WktFormatter) -> Result<(), FactoryError> {
    if !formatter.wkt2 && formatter.esri && crs.components.len() == 2 {
        export_crs(&crs.components[0], formatter)?;
        export_crs(&crs.components[1], formatter)?;
        return Ok(());
    }
    let keyword = if formatter.wkt2 {
        "COMPOUNDCRS"
    } else {
        "COMPD_CS"
    };
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(crs.name());
    for component in &crs.components {
        export_crs(component, formatter)?;
    }
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

fn export_bound(crs: &BoundCRS, formatter: &mut WktFormatter) -> Result<(), FactoryError> {
    if formatter.wkt2 {
        formatter.start_node("BOUNDCRS", NodeId::Absent);
        formatter.start_node("SOURCECRS", NodeId::Absent);
        export_crs(&crs.source, formatter)?;
        formatter.end_node();
        formatter.start_node("TARGETCRS", NodeId::Absent);
        export_crs(&crs.target, formatter)?;
        formatter.end_node();
        formatter.abridged = true;
        export_operation(&crs.transformation, formatter)?;
        formatter.abridged = false;
        formatter.base_usage(&crs.identified);
        formatter.end_node();
        return Ok(());
    }
    Err(FactoryError::Format(
        "WKT1 bound CRS export requires a WGS 84 hub and TOWGS84 parameters".to_owned(),
    ))
}

fn export_engineering(
    crs: &EngineeringCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let keyword = if formatter.wkt2 { "ENGCRS" } else { "LOCAL_CS" };
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(crs.name());
    if formatter.wkt2 || !crs.datum.name().is_empty() {
        export_engineering_datum(&crs.datum, formatter);
    }
    if !formatter.wkt2 {
        if let Some(axis) = crs.coordinate_system.axes().first() {
            formatter.export_unit(axis.unit(), None);
        }
    }
    let previous = formatter.output_axis;
    formatter.output_axis = OutputAxisRule::Yes;
    export_cs(&crs.coordinate_system, formatter);
    formatter.output_axis = previous;
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

fn export_geodetic_anchor(
    datum: Option<&GeodeticReferenceFrame>,
    ensemble: Option<&DatumEnsemble>,
    formatter: &mut WktFormatter,
) {
    if let Some(datum) = datum {
        export_geodetic_datum(datum, formatter);
    } else if let Some(ensemble) = ensemble {
        if !formatter.wkt2 || !formatter.keywords_2019 {
            export_ensemble_as_datum(ensemble, formatter);
        } else {
            export_ensemble(ensemble, formatter);
        }
    }
}

fn export_base_datum(crs: &GeodeticCRS, formatter: &mut WktFormatter) {
    export_geodetic_anchor(crs.datum(), crs.datum_ensemble(), formatter);
}

pub(crate) fn export_datum_standalone(
    datum: &GeodeticReferenceFrame,
    formatter: &mut WktFormatter,
) -> Result<String, FactoryError> {
    export_geodetic_datum(datum, formatter);
    if datum.prime_meridian.name() != "Greenwich" {
        export_prime_meridian(&datum.prime_meridian, formatter);
    }
    Ok(std::mem::take(&mut formatter.result))
}

impl GeodeticReferenceFrame {
    /// Write this datum. A non-Greenwich prime meridian follows the datum node.
    ///
    /// # Errors
    ///
    /// The formatter cannot represent a value on the datum.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_datum_standalone(self, formatter)
    }
}

impl Datum {
    /// Write this datum, of any kind, into `formatter` and return the WKT.
    ///
    /// # Errors
    ///
    /// The formatter cannot represent a value on the datum.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        match self {
            Self::Geodetic(datum) => datum.export_to_wkt(formatter),
            Self::Vertical(datum) => {
                export_vertical_datum(datum, formatter);
                Ok(std::mem::take(&mut formatter.result))
            }
            Self::Engineering(datum) => {
                export_engineering_datum(datum, formatter);
                Ok(std::mem::take(&mut formatter.result))
            }
        }
    }
}

impl DatumEnsemble {
    /// Write this datum ensemble into `formatter` and return the WKT.
    ///
    /// # Errors
    ///
    /// The formatter cannot represent a value on the ensemble.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_ensemble(self, formatter);
        Ok(std::mem::take(&mut formatter.result))
    }
}

impl Ellipsoid {
    /// Write this ellipsoid into `formatter` and return the WKT.
    ///
    /// # Errors
    ///
    /// The formatter cannot represent a value on the ellipsoid.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_ellipsoid(self, formatter);
        Ok(std::mem::take(&mut formatter.result))
    }
}

fn export_geodetic_datum(datum: &GeodeticReferenceFrame, formatter: &mut WktFormatter) {
    if formatter.wkt2 && formatter.keywords_2019 {
        if let Some(epoch) = datum.frame_epoch {
            formatter.start_node("DYNAMIC", NodeId::Absent);
            formatter.start_node("FRAMEEPOCH", NodeId::Absent);
            formatter.add_number(epoch);
            formatter.end_node();
            if let Some(model) = datum.deformation_model.as_deref() {
                if !model.is_empty() {
                    formatter.start_node("MODEL", NodeId::Absent);
                    formatter.add_quoted(model);
                    formatter.end_node();
                }
            }
            formatter.end_node();
        }
    }
    let mut name = if datum.identified.name.is_empty() {
        "unnamed".to_owned()
    } else {
        datum.identified.name.clone()
    };
    if !formatter.wkt2 {
        name = wkt1_datum_name(&datum.identified, formatter);
    }
    formatter.start_node("DATUM", node_from_ids(&datum.identified.identifiers));
    formatter.add_quoted(&name);
    export_ellipsoid(&datum.ellipsoid, formatter);
    if formatter.wkt2 {
        if let Some(anchor) = &datum.identified.anchor {
            formatter.start_node("ANCHOR", NodeId::Absent);
            formatter.add_quoted(anchor);
            formatter.end_node();
        }
        if formatter.keywords_2019 {
            if let Some(epoch) = datum.identified.anchor_epoch {
                formatter.start_node("ANCHOREPOCH", NodeId::Absent);
                formatter.add_number(epoch);
                formatter.end_node();
            }
        }
    }
    formatter.format_ids(&datum.identified.identifiers);
    formatter.end_node();
}

fn export_ensemble(ensemble: &DatumEnsemble, formatter: &mut WktFormatter) {
    formatter.start_node("ENSEMBLE", NodeId::Absent);
    let name = if ensemble.identified.name.is_empty() {
        "unnamed"
    } else {
        ensemble.identified.name.as_str()
    };
    formatter.add_quoted(name);
    for member in &ensemble.members {
        formatter.start_node("MEMBER", node_from_ids(member.identifiers()));
        let member_name = if member.name().is_empty() {
            "unnamed"
        } else {
            member.name()
        };
        formatter.add_quoted(member_name);
        formatter.format_ids(member.identifiers());
        formatter.end_node();
    }
    if let Some(Datum::Geodetic(frame)) = ensemble.members.first() {
        export_ellipsoid(frame.ellipsoid(), formatter);
    }
    formatter.start_node("ENSEMBLEACCURACY", NodeId::Absent);
    match &ensemble.accuracy_text {
        Some(text) => formatter.add_raw(text),
        None => formatter.add_number(ensemble.accuracy),
    }
    formatter.end_node();
    formatter.format_ids(&ensemble.identified.identifiers);
    formatter.end_node();
}

fn export_ensemble_as_datum(ensemble: &DatumEnsemble, formatter: &mut WktFormatter) {
    let name = ensemble_name_as_datum(ensemble.name()).unwrap_or(ensemble.name());
    let mut identified = ensemble.identified.clone();
    identified.name = name.to_owned();
    if let Some(Datum::Geodetic(frame)) = ensemble.members.first() {
        let synthetic = GeodeticReferenceFrame {
            identified,
            ellipsoid: frame.ellipsoid.clone(),
            prime_meridian: frame.prime_meridian.clone(),
            frame_epoch: None,
            deformation_model: None,
        };
        export_geodetic_datum(&synthetic, formatter);
    }
}

fn export_vertical_datum(datum: &VerticalReferenceFrame, formatter: &mut WktFormatter) {
    if formatter.wkt2 && formatter.keywords_2019 {
        if let Some(epoch) = datum.frame_epoch {
            formatter.start_node("DYNAMIC", NodeId::Absent);
            formatter.start_node("FRAMEEPOCH", NodeId::Absent);
            formatter.add_number(epoch);
            formatter.end_node();
            if let Some(model) = datum.deformation_model.as_deref() {
                if !model.is_empty() {
                    formatter.start_node("MODEL", NodeId::Absent);
                    formatter.add_quoted(model);
                    formatter.end_node();
                }
            }
            formatter.end_node();
        }
    }
    let keyword = if formatter.wkt2 || formatter.esri {
        "VDATUM"
    } else {
        "VERT_DATUM"
    };
    let mut name = if datum.identified.name.is_empty() {
        "unnamed".to_owned()
    } else {
        datum.identified.name.clone()
    };
    if !formatter.wkt2 && formatter.esri {
        if let Some(alias) = esri_alias(&datum.identified, "vertical_datum", formatter) {
            name = alias;
        } else {
            name = morph_name_to_esri(&name);
        }
    }
    formatter.start_node(keyword, node_from_ids(&datum.identified.identifiers));
    formatter.add_quoted(&name);
    if formatter.wkt2 {
        if let Some(anchor) = &datum.identified.anchor {
            formatter.start_node("ANCHOR", NodeId::Absent);
            formatter.add_quoted(anchor);
            formatter.end_node();
        }
    } else if !formatter.esri {
        formatter.add_raw(&datum.wkt1_type);
    }
    formatter.format_ids(&datum.identified.identifiers);
    formatter.end_node();
}

fn export_vertical_ensemble(ensemble: &DatumEnsemble, formatter: &mut WktFormatter) {
    if !formatter.wkt2 || !formatter.keywords_2019 {
        let name = ensemble_name_as_datum(ensemble.name()).unwrap_or(ensemble.name());
        let mut identified = ensemble.identified.clone();
        identified.name = name.to_owned();
        let datum = VerticalReferenceFrame {
            identified,
            wkt1_type: "2005".to_owned(),
            frame_epoch: None,
            deformation_model: None,
        };
        export_vertical_datum(&datum, formatter);
    } else {
        export_ensemble(ensemble, formatter);
    }
}

fn export_engineering_datum(datum: &EngineeringDatum, formatter: &mut WktFormatter) {
    let keyword = if formatter.wkt2 {
        "EDATUM"
    } else {
        "LOCAL_DATUM"
    };
    formatter.start_node(keyword, node_from_ids(&datum.identified.identifiers));
    formatter.add_quoted(if datum.name().is_empty() {
        "unnamed"
    } else {
        datum.name()
    });
    if !formatter.wkt2 {
        formatter.add_raw("32767");
    }
    formatter.format_ids(&datum.identified.identifiers);
    formatter.end_node();
}

fn export_ellipsoid(ellipsoid: &Ellipsoid, formatter: &mut WktFormatter) {
    let keyword = if formatter.wkt2 {
        "ELLIPSOID"
    } else {
        "SPHEROID"
    };
    formatter.start_node(keyword, node_from_ids(&ellipsoid.identified.identifiers));
    let mut name = if ellipsoid.name().is_empty() {
        "unnamed".to_owned()
    } else {
        ellipsoid.name().to_owned()
    };
    if formatter.esri {
        if name == "WGS 84" {
            name = "WGS_1984".to_owned();
        } else if let Some(alias) = esri_alias(&ellipsoid.identified, "ellipsoid", formatter) {
            name = alias;
        } else if !esri_official_name(&name, "ellipsoid", formatter) {
            name = morph_name_to_esri(&name);
        }
    }
    formatter.add_quoted(&name);
    if formatter.wkt2 {
        formatter.add_number(ellipsoid.semi_major.value);
    } else {
        formatter.add_number(ellipsoid.semi_major.si_value());
    }
    formatter.add_number(ellipsoid.inverse_flattening());
    let unit = &ellipsoid.semi_major.unit;
    if formatter.wkt2 && !(formatter.omit_ellipsoid_metre && unit.name() == "metre") {
        formatter.export_unit(unit, Some("LENGTHUNIT"));
    }
    formatter.format_ids(&ellipsoid.identified.identifiers);
    formatter.end_node();
}

fn export_prime_meridian(meridian: &PrimeMeridian, formatter: &mut WktFormatter) {
    let mut name = if meridian.name().is_empty() {
        "Greenwich".to_owned()
    } else {
        meridian.name().to_owned()
    };
    if formatter.wkt2 && formatter.omit_greenwich && name == "Greenwich" {
        return;
    }
    if formatter.esri {
        if let Some(alias) = esri_alias(&meridian.identified, "prime_meridian", formatter) {
            name = alias;
        } else {
            name = morph_name_to_esri(&name);
        }
    }
    formatter.start_node("PRIMEM", node_from_ids(&meridian.identified.identifiers));
    formatter.add_quoted(&name);
    let longitude = if formatter.prime_meridian_degree {
        meridian.longitude.convert_to(&UnitOfMeasure::degree())
    } else {
        meridian.longitude.value
    };
    formatter.add_number(longitude);
    if formatter.wkt2 {
        let axis_unit = formatter
            .axis_angular
            .last()
            .cloned()
            .unwrap_or_else(UnitOfMeasure::degree);
        let omit =
            formatter.omit_parameter_unit_if_axis && meridian.longitude.unit.same_name(&axis_unit);
        if !omit {
            formatter.export_unit(&meridian.longitude.unit, Some("ANGLEUNIT"));
        }
    }
    formatter.format_ids(&meridian.identified.identifiers);
    formatter.end_node();
}

fn export_cs(cs: &CoordinateSystem, formatter: &mut WktFormatter) {
    if formatter.output_axis != OutputAxisRule::Yes {
        return;
    }
    let axes = cs.axes();
    if formatter.wkt2 {
        formatter.start_node("CS", node_from_ids(&cs.identified().identifiers));
        formatter.add_raw(cs.wkt2_type());
        formatter.add_raw(&axes.len().to_string());
        formatter.end_node();
        formatter.start_node("", NodeId::Absent);
    }
    let same_unit = axes
        .first()
        .map(|axis| axes.iter().all(|other| other.unit().same_name(axis.unit())))
        .unwrap_or(true);
    let emit_each = formatter.wkt2 && (!same_unit || !formatter.cs_unit_once);
    formatter.output_unit.push(emit_each);
    let suppress_abbrev = axes.len() == 3
        && axes.first().is_some_and(|axis| axis.name() == "Latitude")
        && axes.get(1).is_some_and(|axis| axis.name() == "Longitude")
        && axes
            .get(2)
            .is_some_and(|axis| axis.name() == "Ellipsoidal height");
    for (index, axis) in axes.iter().enumerate() {
        let order = if formatter.wkt2 && axes.len() > 1 {
            index.saturating_add(1)
        } else {
            0
        };
        let abbrev = if suppress_abbrev {
            AxisAbbrev::Hide
        } else {
            AxisAbbrev::Show
        };
        export_axis(axis, order, abbrev, formatter);
    }
    if formatter.wkt2 && !axes.is_empty() && same_unit && formatter.cs_unit_once {
        formatter.export_unit(axes[0].unit(), None);
    }
    formatter.output_unit.pop();
    if formatter.wkt2 {
        formatter.end_node();
    }
}

fn export_axis(axis: &Axis, order: usize, abbrev: AxisAbbrev, formatter: &mut WktFormatter) {
    let suppress_abbrev = abbrev == AxisAbbrev::Hide;
    formatter.start_node("AXIS", NodeId::Absent);
    let mut designation = String::new();
    if !axis.name().is_empty() {
        if formatter.wkt2 {
            let mut chars = axis.name().chars();
            if let Some(first) = chars.next() {
                designation.push(first.to_ascii_lowercase());
                designation.push_str(chars.as_str());
            }
        } else if axis.name() == "Geodetic latitude" {
            designation.push_str("Latitude");
        } else if axis.name() == "Geodetic longitude" {
            designation.push_str("Longitude");
        } else {
            designation.push_str(axis.name());
        }
    }
    let geocentric = matches!(
        axis.direction(),
        "geocentricX" | "geocentricY" | "geocentricZ"
    );
    let easting_northing = (order == 1 && axis.name() == "Easting" && axis.abbreviation() == "E")
        || (order == 2 && axis.name() == "Northing" && axis.abbreviation() == "N");
    if formatter.wkt2 && !suppress_abbrev && !axis.abbreviation().is_empty() {
        let latitude = axis.name() == "Latitude" || axis.name() == "Longitude";
        if !latitude {
            if geocentric || easting_northing {
                designation = format!("({})", axis.abbreviation());
            } else if !designation.is_empty() {
                designation.push(' ');
                designation.push('(');
                designation.push_str(axis.abbreviation());
                designation.push(')');
            }
        }
    }
    formatter.add_quoted(&designation);
    let mut direction = axis.direction().to_owned();
    if !formatter.wkt2 {
        direction.make_ascii_uppercase();
        if axis.direction() == "geocentricZ" {
            direction = "NORTH".to_owned();
        } else if !matches!(
            direction.as_str(),
            "NORTH" | "SOUTH" | "EAST" | "WEST" | "UP" | "DOWN" | "OTHER"
        ) {
            direction = "OTHER".to_owned();
        }
    }
    formatter.add_raw(&direction);
    if formatter.wkt2 {
        if let Some(meridian) = &axis.meridian {
            formatter.start_node("MERIDIAN", NodeId::Absent);
            formatter.add_number(meridian.longitude.value);
            formatter.export_unit(&meridian.longitude.unit, Some("ANGLEUNIT"));
            formatter.end_node();
        }
    }
    if formatter.output_axis_order && order > 0 {
        formatter.start_node("ORDER", NodeId::Absent);
        formatter.add_raw(&order.to_string());
        formatter.end_node();
    }
    if formatter.output_unit_now() && axis.unit().kind() != UnitKind::None {
        formatter.export_unit(axis.unit(), None);
    }
    if formatter.wkt2 && formatter.keywords_2019 {
        if let Some(minimum) = axis.minimum {
            formatter.start_node("AXISMINVALUE", NodeId::Absent);
            formatter.add_number(minimum);
            formatter.end_node();
        }
        if let Some(maximum) = axis.maximum {
            formatter.start_node("AXISMAXVALUE", NodeId::Absent);
            formatter.add_number(maximum);
            formatter.end_node();
        }
        if axis.minimum.is_some() && axis.maximum.is_some() {
            if let Some(meaning) = &axis.range_meaning {
                formatter.start_node("RANGEMEANING", NodeId::Absent);
                formatter.add_raw(meaning);
                formatter.end_node();
            }
        }
    }
    formatter.end_node();
}

/// Whether the projected base ellipsoid is a sphere.
///
/// A bool parameter is rejected by `max-fn-params-bools`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BaseShape {
    Sphere,
    NotSphere,
}

fn export_conversion(
    conversion: &Conversion,
    formatter: &mut WktFormatter,
    base_shape: BaseShape,
) -> Result<(), FactoryError> {
    if !formatter.wkt2 && formatter.esri {
        export_conversion_wkt1(conversion, formatter, base_shape)?;
        return Ok(());
    }
    if formatter.wkt2 {
        formatter.start_node(
            "CONVERSION",
            node_from_ids(&conversion.identified.identifiers),
        );
        formatter.add_quoted(conversion.name());
        export_method(&conversion.method, formatter)?;
        for parameter in &conversion.parameters {
            if omitted_zero_parameter(conversion, parameter) {
                continue;
            }
            export_parameter(parameter, formatter)?;
        }
        formatter.format_ids(&conversion.identified.identifiers);
        formatter.end_node();
        return Ok(());
    }
    export_conversion_wkt1(conversion, formatter, base_shape)
}

/// PROJ leaves out a zero latitude of origin in Equidistant Cylindrical, and a
/// zero false easting or northing in Vertical Perspective, because EPSG does not
/// list them for those methods.
fn omitted_zero_parameter(conversion: &Conversion, parameter: &Parameter) -> bool {
    let zero =
        matches!(&parameter.value, ParameterValue::Measure(measure) if measure.si_value() == 0.0);
    match (conversion.method.code.as_str(), parameter.code.as_str()) {
        ("1028" | "1029", "8801") => zero,
        ("9838", "8806" | "8807") => zero,
        _ => false,
    }
}

fn export_conversion_wkt1(
    conversion: &Conversion,
    formatter: &mut WktFormatter,
    base_shape: BaseShape,
) -> Result<(), FactoryError> {
    formatter.output_unit.push(false);
    formatter.output_id.push(false);
    if conversion.method.code == "1024" {
        formatter.start_node("PROJECTION", NodeId::Absent);
        formatter.add_quoted("Mercator_1SP");
        formatter.end_node();
        write_wkt1_parameter(
            formatter,
            "central_meridian",
            parameter_number(conversion, "8802")?,
        )?;
        formatter.start_node("PARAMETER", NodeId::Absent);
        formatter.add_quoted("scale_factor");
        formatter.add_number(1.0);
        formatter.end_node();
        write_wkt1_parameter(
            formatter,
            "false_easting",
            parameter_number(conversion, "8806")?,
        )?;
        write_wkt1_parameter(
            formatter,
            "false_northing",
            parameter_number(conversion, "8807")?,
        )?;
    } else {
        if formatter.esri && conversion.method.code == "9807" {
            formatter.start_node("PROJECTION", NodeId::Absent);
            formatter.add_quoted("Transverse_Mercator");
            formatter.end_node();
            for (name, code) in [
                ("False_Easting", "8806"),
                ("False_Northing", "8807"),
                ("Central_Meridian", "8802"),
                ("Scale_Factor", "8805"),
                ("Latitude_Of_Origin", "8801"),
            ] {
                write_wkt1_parameter(formatter, name, parameter_number(conversion, code)?)?;
            }
            formatter.output_id.pop();
            formatter.output_unit.pop();
            return Ok(());
        }
        if formatter.esri && conversion.method.code == "9801" {
            formatter.start_node("PROJECTION", NodeId::Absent);
            formatter.add_quoted("Lambert_Conformal_Conic");
            formatter.end_node();
            for (name, code, wkt2_name) in [
                ("False_Easting", "8806", "False easting"),
                ("False_Northing", "8807", "False northing"),
                ("Central_Meridian", "8802", "Longitude of natural origin"),
                ("Standard_Parallel_1", "8801", "Latitude of natural origin"),
                ("Scale_Factor", "8805", "Scale factor at natural origin"),
                ("Latitude_Of_Origin", "8801", "Latitude of natural origin"),
            ] {
                write_wkt1_parameter(
                    formatter,
                    name,
                    parameter_on_axis(conversion, code, wkt2_name, formatter)?,
                )?;
            }
            formatter.output_id.pop();
            formatter.output_unit.pop();
            return Ok(());
        }
        if conversion.method.code == "9804"
            || (base_shape == BaseShape::Sphere && conversion.method.code == "1026")
        {
            let latitude = parameter_number(conversion, "8801").unwrap_or(0.0);
            if latitude != 0.0 {
                return Err(FactoryError::Format(
                    "Unsupported value for Latitude of natural origin".to_owned(),
                ));
            }
        }
        if !formatter.esri && base_shape == BaseShape::Sphere && conversion.method.code == "1026" {
            formatter.start_node("PROJECTION", NodeId::Absent);
            formatter.add_quoted("Mercator_1SP");
            formatter.end_node();
            let mercator_a = super::parammap::method_by_code(9804);
            for parameter in &conversion.parameters {
                if parameter.code == "8801" {
                    continue;
                }
                let Some(name) = wkt1_parameter_name(mercator_a, parameter) else {
                    continue;
                };
                formatter.start_node("PARAMETER", NodeId::Absent);
                formatter.add_quoted(&name);
                add_parameter_value(parameter, formatter)?;
                formatter.end_node();
            }
            formatter.output_id.pop();
            formatter.output_unit.pop();
            return Ok(());
        }
        formatter.start_node("PROJECTION", NodeId::Absent);
        formatter.add_quoted(&wkt1_method_name(&conversion.method)?);
        formatter.end_node();
        let mapped = lookup_method(&conversion.method);
        for parameter in &conversion.parameters {
            if omitted_zero_parameter(conversion, parameter) {
                continue;
            }
            let Some(mut name) = wkt1_parameter_name(mapped, parameter) else {
                continue;
            };
            if formatter.esri {
                name = esri_wkt1_parameter_name(&name);
            }
            formatter.start_node("PARAMETER", NodeId::Absent);
            formatter.add_quoted(&name);
            add_parameter_value(parameter, formatter)?;
            formatter.end_node();
        }
    }
    formatter.output_id.pop();
    formatter.output_unit.pop();
    Ok(())
}

fn export_method(
    method: &OperationMethod,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let keyword = if formatter.wkt2 {
        "METHOD"
    } else {
        "PROJECTION"
    };
    formatter.start_node(keyword, node_from_text(&method.code));
    let name = if formatter.wkt2 {
        method.name.clone()
    } else {
        wkt1_method_name(method)?
    };
    formatter.add_quoted(&name);
    if formatter.output_id_now() && !method.code.is_empty() {
        formatter.start_node("ID", NodeId::Absent);
        formatter.add_quoted(&method.code_space);
        if is_integer_code(&method.code) {
            formatter.add_raw(&method.code);
        } else {
            formatter.add_quoted(&method.code);
        }
        formatter.end_node();
    }
    formatter.end_node();
    Ok(())
}

fn export_parameter(
    parameter: &Parameter,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let keyword = if formatter.wkt2 && matches!(parameter.value, ParameterValue::Filename(_)) {
        "PARAMETERFILE"
    } else {
        "PARAMETER"
    };
    formatter.start_node(keyword, node_from_text(&parameter.code));
    formatter.add_quoted(&parameter.name);
    add_parameter_value(parameter, formatter)?;
    if formatter.output_id_now() && !parameter.code.is_empty() {
        formatter.start_node("ID", NodeId::Absent);
        formatter.add_quoted(&parameter.code_space);
        if is_integer_code(&parameter.code) {
            formatter.add_raw(&parameter.code);
        } else {
            formatter.add_quoted(&parameter.code);
        }
        formatter.end_node();
    }
    formatter.end_node();
    Ok(())
}

fn add_parameter_value(
    parameter: &Parameter,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    match &parameter.value {
        ParameterValue::Measure(measure) => add_measure(measure, formatter),
        ParameterValue::String(text) | ParameterValue::Filename(text) => {
            formatter.add_quoted(text);
            Ok(())
        }
        ParameterValue::Integer(value) => {
            formatter.add_raw(&value.to_string());
            Ok(())
        }
        ParameterValue::Boolean(_) => Err(FactoryError::Format(
            "boolean parameter value not handled".to_owned(),
        )),
    }
}

fn add_measure(measure: &Measure, formatter: &mut WktFormatter) -> Result<(), FactoryError> {
    if formatter.abridged {
        match measure.unit.kind {
            UnitKind::Linear => formatter.add_number(measure.si_value()),
            UnitKind::Angular => {
                let seconds = UnitOfMeasure {
                    name: "arc-second".to_owned(),
                    factor: UnitOfMeasure::degree().factor / 3600.0,
                    kind: UnitKind::Angular,
                    code_space: String::new(),
                    code: String::new(),
                };
                formatter.add_number(measure.convert_to(&seconds));
            }
            _ => formatter.add_number(measure.value),
        }
        return Ok(());
    }
    if formatter.wkt2 {
        formatter.add_number(measure.value);
        if measure.unit.kind != UnitKind::None {
            let linear = formatter
                .axis_linear
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::metre);
            let angular = formatter
                .axis_angular
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::degree);
            let omit = formatter.omit_parameter_unit_if_axis
                && (measure.unit.name() == "unity"
                    || measure.unit.same_name(&linear)
                    || measure.unit.same_name(&angular));
            if !omit {
                formatter.export_unit(&measure.unit, None);
            }
        }
        return Ok(());
    }
    match measure.unit.kind {
        UnitKind::Linear => {
            let target = formatter
                .axis_linear
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::metre);
            if target.factor == 0.0 {
                return Err(FactoryError::Format(
                    "cannot convert a parameter into the target linear unit".to_owned(),
                ));
            }
            formatter.add_number(measure.convert_to(&target));
        }
        UnitKind::Angular => {
            let target = formatter
                .axis_angular
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::degree);
            if target.factor == 0.0 {
                return Err(FactoryError::Format(
                    "cannot convert a parameter into the target angular unit".to_owned(),
                ));
            }
            formatter.add_number(measure.convert_to(&target));
        }
        _ => formatter.add_number(measure.si_value()),
    }
    Ok(())
}

fn export_operation(
    operation: &super::model::CoordinateOperation,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    match operation {
        super::model::CoordinateOperation::Conversion(conversion) => {
            export_conversion(conversion, formatter, BaseShape::NotSphere)
        }
        super::model::CoordinateOperation::Transformation(transformation) => {
            export_transformation(transformation, formatter)
        }
        super::model::CoordinateOperation::Concatenated(_) => Err(FactoryError::Format(
            "a concatenated operation is not exported as a single WKT node here".to_owned(),
        )),
    }
}

fn export_transformation_crs(
    transformation: &Transformation,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    if let Some(source) = &transformation.source {
        formatter.start_node("SOURCECRS", NodeId::Absent);
        export_crs(source, formatter)?;
        formatter.end_node();
    }
    if let Some(target) = &transformation.target {
        formatter.start_node("TARGETCRS", NodeId::Absent);
        export_crs(target, formatter)?;
        formatter.end_node();
    }
    Ok(())
}

fn export_transformation(
    transformation: &Transformation,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    if !formatter.wkt2 {
        return Err(FactoryError::Format(
            "a transformation can only be exported to WKT2".to_owned(),
        ));
    }
    let keyword = if formatter.abridged {
        "ABRIDGEDTRANSFORMATION"
    } else if transformation.point_motion {
        "POINTMOTIONOPERATION"
    } else {
        "COORDINATEOPERATION"
    };
    formatter.start_node(
        keyword,
        node_from_ids(&transformation.identified.identifiers),
    );
    formatter.add_quoted(transformation.name());
    if formatter.keywords_2019 {
        if let Some(version) = &transformation.version {
            formatter.start_node("VERSION", NodeId::Absent);
            formatter.add_quoted(version);
            formatter.end_node();
        }
    }
    if !formatter.abridged {
        let disabled = usize::from(!transformation.identified.domains.is_empty());
        formatter.usage_disabled += disabled;
        let exported = export_transformation_crs(transformation, formatter);
        formatter.usage_disabled -= disabled;
        exported?;
    }
    export_method(&transformation.method, formatter)?;
    for parameter in &transformation.parameters {
        export_parameter(parameter, formatter)?;
    }
    if !formatter.abridged {
        if let Some(accuracy) = transformation.accuracy {
            formatter.start_node("OPERATIONACCURACY", NodeId::Absent);
            formatter.add_number(accuracy);
            formatter.end_node();
        }
    }
    formatter.base_usage(&transformation.identified);
    formatter.end_node();
    Ok(())
}

fn geodetic_meridian<'a>(
    datum: Option<&'a GeodeticReferenceFrame>,
    ensemble: Option<&'a DatumEnsemble>,
) -> Option<&'a PrimeMeridian> {
    if let Some(datum) = datum {
        return Some(datum.prime_meridian());
    }
    ensemble.and_then(|ensemble| {
        ensemble.members().iter().find_map(|member| match member {
            Datum::Geodetic(frame) => Some(frame.prime_meridian()),
            _ => None,
        })
    })
}

fn base_angular_unit(crs: &GeodeticCRS) -> UnitOfMeasure {
    let axes = match crs {
        GeodeticCRS::Geographic(crs) => crs.coordinate_system.axes(),
        GeodeticCRS::Geocentric(crs) => crs.coordinate_system.axes(),
        GeodeticCRS::Spherical(crs) => crs.coordinate_system.axes(),
    };
    axes.first()
        .map(|axis| axis.unit().clone())
        .unwrap_or_else(UnitOfMeasure::degree)
}

fn top_has_id(formatter: &WktFormatter) -> bool {
    formatter.stack_has_id.get(1).copied().unwrap_or(false)
}

fn esri_geographic_name(
    identified: &Identified,
    axis_count: usize,
    formatter: &WktFormatter,
) -> String {
    if identified.name == "WGS 84" {
        return if axis_count == 3 {
            "WGS_1984_3D".to_owned()
        } else {
            "GCS_WGS_1984".to_owned()
        };
    }
    let table = if axis_count == 3 {
        "geographic_3D_crs"
    } else {
        "geodetic_crs"
    };
    if let Some(alias) = esri_alias(identified, table, formatter) {
        return alias;
    }
    if esri_official_name(&identified.name, table, formatter) {
        return identified.name.clone();
    }
    let mut name = morph_name_to_esri(&identified.name);
    if !name.starts_with("GCS_") {
        name.insert_str(0, "GCS_");
    }
    name
}

/// ESRI WKT1 of an object whose database row is a `PROJCS` `text_definition`.
fn esri_projected_text(crs: &ProjectedCRS, formatter: &WktFormatter) -> Option<String> {
    if !formatter.esri || formatter.wkt2 {
        return None;
    }
    let db = formatter.db.as_ref()?;
    let identifier = crs
        .identifiers()
        .iter()
        .find(|identifier| identifier.code_space() == "ESRI")?;
    let text = db.text_definition("projected_crs", "ESRI", identifier.code())?;
    if !text.starts_with("PROJCS") {
        return None;
    }
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let Ok(object) = parser.create_from_wkt(&text) else {
        return None;
    };
    let Some(Crs::Projected(parsed)) = object.crs().cloned() else {
        return None;
    };
    if crs.is_equivalent_to(&parsed) {
        Some(text)
    } else {
        None
    }
}

fn esri_projected_name(identified: &Identified, formatter: &WktFormatter) -> String {
    if let Some(alias) = esri_alias(identified, "projected_crs", formatter) {
        return alias;
    }
    morph_name_to_esri(&identified.name)
}

fn esri_official_name(name: &str, table: &str, formatter: &WktFormatter) -> bool {
    formatter
        .db
        .as_ref()
        .is_some_and(|db| db.has_single_esri_name(table, name))
}

fn esri_alias(identified: &Identified, table: &str, formatter: &WktFormatter) -> Option<String> {
    let db = formatter.db.as_ref()?;
    if let Some(identifier) = identified.identifiers.first() {
        if let Ok(aliases) = db.aliases(&identifier.code_space, &identifier.code, table, "ESRI") {
            if aliases.len() == 1 {
                return Some(aliases[0].clone());
            }
        }
    }
    db.alias_from_official_name(&identified.name, table, "ESRI")
        .ok()
        .filter(|alias| !alias.is_empty())
}

fn wkt1_datum_name(identified: &Identified, formatter: &WktFormatter) -> String {
    if formatter.esri {
        if identified.name == "World Geodetic System 1984" {
            return "D_WGS_1984".to_owned();
        }
        if let Some(alias) = esri_alias(identified, "geodetic_datum", formatter) {
            return alias;
        }
        if esri_official_name(&identified.name, "geodetic_datum", formatter) {
            return identified.name.clone();
        }
        let mut name = morph_name_to_esri(&identified.name);
        if !name.starts_with("D_") {
            name.insert_str(0, "D_");
        }
        return name;
    }
    let mut name = identified.name.clone();
    let epsg = identified
        .identifiers
        .first()
        .is_some_and(|identifier| identifier.code_space == "EPSG");
    let matched = identified.identifiers.is_empty()
        && formatter
            .db
            .as_ref()
            .is_some_and(|db| db.single_epsg_geodetic_datum(&name));
    if epsg || matched {
        name = morph_name_to_esri(&name);
    }
    if name == "World_Geodetic_System_1984" {
        name = "WGS_1984".to_owned();
    }
    name
}

fn esri_unit_name(name: &str, formatter: &WktFormatter) -> String {
    if !formatter.esri {
        return name.to_owned();
    }
    if name.eq_ignore_ascii_case("degree") {
        "Degree".to_owned()
    } else if name.eq_ignore_ascii_case("grad") {
        "Grad".to_owned()
    } else if name.eq_ignore_ascii_case("metre") {
        "Meter".to_owned()
    } else {
        name.to_owned()
    }
}

fn lookup_method(method: &OperationMethod) -> Option<&'static super::parammap::MethodMap> {
    let code = method.code.parse::<i32>().unwrap_or(0);
    if code != 0 {
        let named = super::parammap::PROJECTION_METHODS
            .iter()
            .chain(super::parammap::OTHER_METHODS)
            .find(|mapped| mapped.epsg_code == code && names_match(mapped.wkt2_name, &method.name));
        if named.is_some() {
            return named;
        }
        if let Some(mapped) = super::parammap::method_by_code(code) {
            return Some(mapped);
        }
    }
    super::parammap::method_by_name(&method.name)
}

fn names_match(left: &str, right: &str) -> bool {
    canonical_name(left) == canonical_name(right)
}

fn canonical_name(name: &str) -> String {
    name.chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '/' | '{' | '}'))
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn esri_wkt1_parameter_name(name: &str) -> String {
    match name {
        "latitude_of_origin" => "Latitude_Of_Origin",
        "central_meridian" => "Central_Meridian",
        "scale_factor" => "Scale_Factor",
        "false_easting" => "False_Easting",
        "false_northing" => "False_Northing",
        "standard_parallel_1" => "Standard_Parallel_1",
        "standard_parallel_2" => "Standard_Parallel_2",
        "azimuth" => "Azimuth",
        "rectified_grid_angle" => "Rectified_Grid_Angle",
        _ => return name.to_owned(),
    }
    .to_owned()
}

fn wkt1_method_name(method: &OperationMethod) -> Result<String, FactoryError> {
    if method.name == "Geostationary Satellite (Sweep X)" {
        return Ok("Geostationary_Satellite".to_owned());
    }
    if let Some(mapped) = lookup_method(method) {
        return mapped.wkt1_name.map(str::to_owned).ok_or_else(|| {
            FactoryError::Format(format!(
                "Unsupported conversion method: {}",
                mapped.wkt2_name
            ))
        });
    }
    Ok(method.name.replace(' ', "_"))
}

fn wkt1_parameter_name(
    method: Option<&super::parammap::MethodMap>,
    parameter: &Parameter,
) -> Option<String> {
    if let Some(method) = method {
        let code = parameter.code.parse::<i32>().unwrap_or(0);
        let found = if code != 0 {
            method
                .params
                .iter()
                .find(|param| param.epsg_code == code)
                .or_else(|| {
                    method
                        .params
                        .iter()
                        .find(|param| names_match(param.wkt2_name, &parameter.name))
                })
        } else {
            method
                .params
                .iter()
                .find(|param| names_match(param.wkt2_name, &parameter.name))
        };
        if let Some(param) = found {
            return param.wkt1_name.map(str::to_owned);
        }
    }
    Some(match parameter.code.as_str() {
        "8801" | "8811" | "8821" => "latitude_of_origin".to_owned(),
        "8802" | "8812" | "8822" => "central_meridian".to_owned(),
        "8805" | "8815" => "scale_factor".to_owned(),
        "8806" | "8816" | "8826" => "false_easting".to_owned(),
        "8807" | "8817" | "8827" => "false_northing".to_owned(),
        "8823" => "standard_parallel_1".to_owned(),
        "8824" => "standard_parallel_2".to_owned(),
        "8813" => "azimuth".to_owned(),
        "8814" => "rectified_grid_angle".to_owned(),
        _ => parameter.name.clone(),
    })
}

fn parameter_number(conversion: &Conversion, code: &str) -> Result<f64, FactoryError> {
    conversion
        .parameters
        .iter()
        .find(|parameter| parameter.code == code)
        .and_then(|parameter| match &parameter.value {
            ParameterValue::Measure(measure) => Some(match measure.unit.kind {
                UnitKind::Angular => measure.convert_to(&UnitOfMeasure::degree()),
                UnitKind::Linear => measure.si_value(),
                _ => measure.value,
            }),
            _ => None,
        })
        .ok_or_else(|| FactoryError::Format(format!("missing conversion parameter {code}")))
}

fn parameter_on_axis(
    conversion: &Conversion,
    code: &str,
    wkt2_name: &str,
    formatter: &WktFormatter,
) -> Result<f64, FactoryError> {
    let measure = conversion
        .parameters
        .iter()
        .find(|parameter| parameter.code == code || names_match(&parameter.name, wkt2_name))
        .and_then(|parameter| match &parameter.value {
            ParameterValue::Measure(measure) => Some(measure),
            _ => None,
        })
        .ok_or_else(|| FactoryError::Format(format!("missing conversion parameter {wkt2_name}")))?;
    match measure.unit.kind {
        UnitKind::Linear => {
            let target = formatter
                .axis_linear
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::metre);
            Ok(measure.convert_to(&target))
        }
        UnitKind::Angular => {
            let target = formatter
                .axis_angular
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::degree);
            Ok(measure.convert_to(&target))
        }
        _ => Ok(measure.si_value()),
    }
}

fn write_wkt1_parameter(
    formatter: &mut WktFormatter,
    name: &str,
    value: f64,
) -> Result<(), FactoryError> {
    formatter.start_node("PARAMETER", NodeId::Absent);
    formatter.add_quoted(name);
    formatter.add_number(value);
    formatter.end_node();
    Ok(())
}

fn morph_name_to_esri(name: &str) -> String {
    for suffix in ["(m)", "(ftUS)", "(E-N)", "(N-E)"] {
        if let Some(prefix) = name.strip_suffix(suffix) {
            let mut morphed = morph_name_to_esri(prefix);
            morphed.push_str(suffix);
            return morphed;
        }
    }
    let mut out = String::new();
    let mut underscore = false;
    for ch in name.chars() {
        if ch == '+' || ch == '-' || ch.is_ascii_alphanumeric() {
            if underscore && !out.is_empty() {
                out.push('_');
            }
            out.push(ch);
            underscore = false;
        } else {
            underscore = true;
        }
    }
    out
}

fn is_integer_code(code: &str) -> bool {
    !code.is_empty() && code.chars().all(|ch| ch.is_ascii_digit())
}

fn is_number_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+')
}

fn format_number(value: f64, formatter: &WktFormatter) -> String {
    let esri = formatter.esri;
    if value == 0.0 {
        return if esri {
            "0.0".to_owned()
        } else {
            "0".to_owned()
        };
    }
    let mut text = format_g(value, 15);
    if text.contains("9999999999") {
        text = format_g(value, 14);
    }
    text = text.replace('e', "E");
    if esri && !text.contains('.') && !text.contains('E') {
        text.push_str(".0");
    }
    text
}

/// `printf("%.Pg")`: `precision` significant digits, trailing zeros removed.
pub(crate) fn format_g(value: f64, precision: usize) -> String {
    let precision = precision.max(1);
    let negative = value.is_sign_negative();
    let abs = value.abs();
    let exp_digits = precision - 1;
    let scientific = format!("{abs:.exp_digits$e}");
    let exponent = scientific
        .split('e')
        .nth(1)
        .and_then(|text| text.parse::<i32>().ok())
        .unwrap_or(0);
    let mut body = if exponent < -4 || exponent >= i32::try_from(precision).unwrap_or(i32::MAX) {
        c_exponent(&scientific)
    } else {
        let fraction = (i32::try_from(precision).unwrap_or(15) - 1 - exponent).max(0);
        let fraction = usize::try_from(fraction).unwrap_or(0);
        format!("{abs:.fraction$}")
    };
    strip_trailing_zeros(&mut body);
    if negative {
        let mut signed = String::from("-");
        signed.push_str(&body);
        signed
    } else {
        body
    }
}

/// Rust writes `e-5`; C writes the sign and at least two digits, `e-05`.
fn c_exponent(scientific: &str) -> String {
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return scientific.to_owned();
    };
    let exponent = exponent.parse::<i32>().unwrap_or(0);
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}e{sign}{:02}", exponent.abs())
}

fn strip_trailing_zeros(text: &mut String) {
    let split = text.find(['e', 'E']);
    if let Some(index) = split {
        let exponent = text[index..].to_owned();
        let mut mantissa = text[..index].to_owned();
        strip_fixed(&mut mantissa);
        text.clear();
        text.push_str(&mantissa);
        text.push_str(&exponent);
    } else {
        strip_fixed(text);
    }
}

fn strip_fixed(text: &mut String) {
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
}
