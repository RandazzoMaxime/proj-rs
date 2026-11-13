//! WKT1 and WKT2 export.
//!
//! Node order, identifier suppression, and the simplified omissions follow
//! `WKTFormatter` in PROJ `src/iso19111/io.cpp`.

use super::bound::hub_is_wgs84;
use super::esri_parammap::{
    esri_method, find_parameter, mercator_variant_b, squared_eccentricity, EsriChoice, EsriParam,
};
use super::factory::{AuthorityFactory, AuthorityObject, DatabaseContext, NameMatch, ObjectType};
use super::wkt_import::WktParser;

mod niche;
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
    ensemble_name_as_datum, Axis, BoundCRS, CompoundCRS, Conversion, CoordinateOperation,
    CoordinateSystem, Crs, Datum, DatumEnsemble, Ellipsoid, EngineeringCRS, EngineeringDatum,
    FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identified,
    Identifier, Measure, OperationMethod, Parameter, ParameterValue, PrimeMeridian, ProjectedCRS,
    Transformation, UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame,
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
    /// Seven TOWGS84 values written into the next WKT1 datum.
    towgs84: Option<[f64; 7]>,
    /// `PROJ4_GRIDS` written into the next WKT1 vertical datum.
    vdatum_grid: Option<String>,
    /// `PROJ4_GRIDS` written into the next WKT1 geodetic datum.
    hdatum_grid: Option<String>,
    /// Geographic CRS name of a compound being exported, so a vertical bound
    /// whose hub is that CRS is written back as `EXTENSION["PROJ4_GRIDS"]`.
    compound_geog_name: Option<String>,
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
    /// `setUseDerivingConversion`: the next conversion is a `DERIVINGCONVERSION`.
    use_deriving_conversion: bool,
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
            towgs84: None,
            vdatum_grid: None,
            hdatum_grid: None,
            compound_geog_name: None,
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
            use_deriving_conversion: false,
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

    /// `pushHasId(false)` on the current node: identifiers below it are written even when an
    /// ancestor has one.
    fn simul_cur_node_has_id_cleared(&mut self) {
        if let Some(flag) = self.stack_has_id.last_mut() {
            *flag = false;
        }
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
            let (code_space, version) = if let Some(version) = &identifier.version {
                (identifier.code_space.clone(), Some(version.clone()))
            } else {
                self.split_authority(&identifier.code_space)
            };
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
                (true, UnitKind::Parametric) => "PARAMETRICUNIT",
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
        if let Some(vertical) = &domain.vertical {
            self.start_node("VERTICALEXTENT", NodeId::Absent);
            self.add_number(vertical.minimum);
            self.add_number(vertical.maximum);
            self.export_unit(&vertical.unit, None);
            self.end_node();
        }
        if let Some(temporal) = &domain.temporal {
            self.start_node("TIMEEXTENT", NodeId::Absent);
            self.add_raw(&temporal.start);
            self.add_raw(&temporal.stop);
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
        export_conversion(self, formatter, BaseShape::NotSphere, None)?;
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

impl Transformation {
    /// Write this transformation into `formatter` and return the WKT.
    ///
    /// # Errors
    ///
    /// The formatter is not a WKT2 one, or a parameter cannot be written.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_transformation(self, formatter)?;
        Ok(std::mem::take(&mut formatter.result))
    }
}

impl super::model::ConcatenatedOperation {
    /// Write this concatenated operation into `formatter` and return the WKT.
    ///
    /// # Errors
    ///
    /// The formatter is not WKT2:2019, or a step cannot be written.
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> Result<String, FactoryError> {
        export_concatenated(self, formatter)?;
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
        Crs::DerivedProjected(crs) => niche::export_derived_projected(crs, formatter)?,
        Crs::Temporal(crs) => niche::export_temporal(crs, formatter)?,
        Crs::Parametric(crs) => niche::export_parametric(crs, formatter)?,
        Crs::Derived(crs) => niche::export_derived(crs, formatter)?,
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
            realization_method: None,
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

/// WKT1 GDAL `EXTENSION["PROJ4", ...]` of a GEOGCS, GEOCCS or PROJCS.
fn export_extension_proj4(identified: &Identified, formatter: &mut WktFormatter) {
    let Some(extension) = identified.extension_proj4.as_deref() else {
        return;
    };
    if extension.is_empty() || formatter.wkt2 || formatter.esri {
        return;
    }
    formatter.start_node("EXTENSION", NodeId::Absent);
    formatter.add_quoted("PROJ4");
    formatter.add_quoted(extension);
    formatter.end_node();
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
        if crs.identified.allow_non_conformant_wkt1 {
            return export_nonconformant_geographic(crs, formatter);
        }
        if let Some(compound) = &crs.identified.original_compound {
            return export_compound(compound, formatter);
        }
        if formatter.allow_ellipsoidal_height {
            return export_ellipsoidal_height_compound(crs, formatter);
        }
        return Err(FactoryError::Format(
            "WKT1 does not support Geographic 3D CRS.".to_owned(),
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
    export_extension_proj4(&crs.identified, formatter);
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
    if let Some(vertical) = esri_ellipsoidal_vertical(crs.datum.as_ref(), formatter) {
        return export_vertical(&vertical, formatter);
    }
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
    export_extension_proj4(&crs.identified, formatter);
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
        if !formatter.wkt2 && !formatter.esri {
            if crs.identified.allow_non_conformant_wkt1 {
                return export_nonconformant_projected(crs, formatter);
            }
            if let Some(compound) = &crs.identified.original_compound {
                return export_compound(compound, formatter);
            }
            if formatter.allow_ellipsoidal_height {
                return export_projected_ellipsoidal_height(crs, formatter);
            }
        }
        if formatter.esri && export_esri_projected_3d(crs, formatter)? {
            return Ok(());
        }
        return Err(FactoryError::Format(
            "a projected 3D CRS can only be exported as WKT2:2019".to_owned(),
        ));
    }
    if !formatter.wkt2
        && !formatter.esri
        && crs
            .identified
            .name
            .starts_with("Popular Visualisation CRS / Mercator")
    {
        return export_popular_visualisation_projected(crs, formatter);
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
        // The base of a projected CRS has no CS in WKT2, 2D or 3D (crs.cpp:4541-4557).
        // The importer promotes a 2D base of a 3D projected CRS (io.cpp:4870-4874).
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
    let owner = OwnerCrs {
        name: &crs.identified.name,
        squared_eccentricity: crs.base.ellipsoid().map(squared_eccentricity),
    };
    export_conversion(&crs.conversion, formatter, base_shape, Some(owner))?;
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
    if !formatter.wkt2 && !formatter.esri {
        if let Some(extension) =
            projected_proj4_extension(crs).or_else(|| crs.wkt1_extension_proj4())
        {
            formatter.start_node("EXTENSION", NodeId::Absent);
            formatter.add_quoted("PROJ4");
            formatter.add_quoted(&extension);
            formatter.end_node();
        }
    }
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

/// The deprecated EPSG:3785 in WKT1 GDAL: the sphere datum with a zero `TOWGS84`, and
/// `Mercator_1SP` with no latitude of origin.
fn export_popular_visualisation_projected(
    crs: &ProjectedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    formatter.start_node("PROJCS", node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&crs.identified.name);
    formatter.towgs84 = Some([0.0; 7]);
    let previous = formatter.output_axis;
    formatter.output_axis = OutputAxisRule::No;
    let base = match &crs.base {
        GeodeticCRS::Geographic(base) => export_geographic(base, formatter),
        GeodeticCRS::Geocentric(base) => export_geocentric(base, formatter),
        GeodeticCRS::Spherical(base) => export_spherical(
            &base.identified,
            base.datum.as_ref(),
            base.ensemble.as_ref(),
            &base.coordinate_system,
            formatter,
        ),
    };
    formatter.output_axis = previous;
    formatter.towgs84 = None;
    base?;
    formatter.start_node("PROJECTION", NodeId::Absent);
    formatter.add_quoted("Mercator_1SP");
    formatter.end_node();
    for (name, value) in [
        ("central_meridian", 0.0),
        ("scale_factor", 1.0),
        ("false_easting", 0.0),
        ("false_northing", 0.0),
    ] {
        formatter.start_node("PARAMETER", NodeId::Absent);
        formatter.add_quoted(name);
        formatter.add_number(value);
        formatter.end_node();
    }
    let axes = &crs.coordinate_system.axes;
    if let Some(unit) = axes.first().map(Axis::unit) {
        formatter.export_unit(unit, None);
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
    if let Some(text) = projected_proj4_extension(crs).or_else(|| crs.wkt1_extension_proj4()) {
        formatter.start_node("EXTENSION", NodeId::Absent);
        formatter.add_quoted("PROJ4");
        formatter.add_quoted(&text);
        formatter.end_node();
    }
    formatter.base_usage(&crs.identified);
    formatter.end_node();
    Ok(())
}

/// WKT1 `EXTENSION["PROJ4", ...]` for a PROJ string the conversion cannot carry.
fn projected_proj4_extension(crs: &ProjectedCRS) -> Option<String> {
    let approx = matches!(
        crs.conversion.identified.proj_method_override.as_deref(),
        Some("tmerc approx" | "utm approx")
    );
    if crs.identified.extension_proj4.is_none() && !approx {
        return None;
    }
    let text = crs.export_to_proj_string().ok()?;
    Some(text.replace(" +type=crs", ""))
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
            realization_method: None,
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

/// WKT1 of two equivalent geographic CRS: `COMPD_CS[name + name, GEOGCS, GEOGCS]`.
///
/// The second copy omits `TOWGS84`. The formatter's parameters are restored afterwards.
fn export_nonconformant_geographic(
    crs: &GeographicCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let flat = geographic_without_height(crs);
    let compound_name = format!("{} + {}", crs.name(), crs.name());
    formatter.start_node("COMPD_CS", NodeId::Absent);
    formatter.add_quoted(&compound_name);
    export_geographic(&flat, formatter)?;
    let saved = formatter.towgs84.take();
    export_geographic(&flat, formatter)?;
    formatter.towgs84 = saved;
    formatter.end_node();
    Ok(())
}

/// WKT1 of a projected CRS stacked on an equivalent geographic CRS.
fn export_nonconformant_projected(
    crs: &ProjectedCRS,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let mut flat = crs.clone();
    flat.coordinate_system.axes.truncate(2);
    flat.identified.allow_non_conformant_wkt1 = false;
    if let GeodeticCRS::Geographic(base) = &mut flat.base {
        *base = geographic_without_height(base);
    }
    let compound_name = format!("{} + {}", crs.name(), crs.base.name());
    formatter.start_node("COMPD_CS", NodeId::Absent);
    formatter.add_quoted(&compound_name);
    export_projected(&flat, formatter)?;
    match &flat.base {
        GeodeticCRS::Geographic(base) => export_geographic(base, formatter)?,
        GeodeticCRS::Geocentric(base) => export_geocentric(base, formatter)?,
        GeodeticCRS::Spherical(base) => export_spherical(
            &base.identified,
            base.datum.as_ref(),
            base.ensemble.as_ref(),
            &base.coordinate_system,
            formatter,
        )?,
    }
    formatter.end_node();
    Ok(())
}

fn geographic_without_height(crs: &GeographicCRS) -> GeographicCRS {
    let mut flat = crs.clone();
    flat.coordinate_system.axes.truncate(2);
    flat.identified.allow_non_conformant_wkt1 = false;
    flat.identified.original_compound = None;
    flat
}

/// `PROJCS,VERTCS` for an ESRI projected 3D CRS whose datum has one ellipsoidal-height CRS.
fn export_esri_projected_3d(
    crs: &ProjectedCRS,
    formatter: &mut WktFormatter,
) -> Result<bool, FactoryError> {
    let Some(vertical) = esri_ellipsoidal_vertical(crs.base.datum(), formatter) else {
        return Ok(false);
    };
    let mut flat = crs.clone();
    flat.coordinate_system.axes.truncate(2);
    if let GeodeticCRS::Geographic(base) = &mut flat.base {
        *base = geographic_without_height(base);
    }
    export_projected(&flat, formatter)?;
    export_vertical(&vertical, formatter)?;
    Ok(true)
}

/// ESRI vertical CRS `from_geogdatum_{authority}_{code}` for this geodetic datum.
fn esri_ellipsoidal_vertical(
    datum: Option<&GeodeticReferenceFrame>,
    formatter: &WktFormatter,
) -> Option<VerticalCRS> {
    let db = formatter.db.as_ref()?;
    let datum = datum?;
    let identifier = datum
        .identifiers()
        .iter()
        .find(|identifier| identifier.code_space.eq_ignore_ascii_case("EPSG"))
        .or_else(|| datum.identifiers().first())?;
    let code = db.esri_vertical_code_for_geodetic(identifier.code_space(), identifier.code())?;
    AuthorityFactory::create(db, "ESRI")
        .create_vertical_crs(&code)
        .ok()
}

/// ESRI type `2002` is a geodetic datum (`DATUM`/`SPHEROID`), not a `VDATUM`.
///
/// The vertical datum code is `from_geogdatum_{authority}_{code}`. A code that
/// does not parse, or a datum the database does not have, leaves the caller to
/// write the vertical datum instead.
fn export_esri_type_2002_datum(
    datum: &VerticalReferenceFrame,
    formatter: &mut WktFormatter,
) -> bool {
    let Some(code) = datum.identifiers().first().map(Identifier::code) else {
        return false;
    };
    let Some(rest) = code.strip_prefix("from_geogdatum_") else {
        return false;
    };
    let Some((authority, geodetic_code)) = rest.rsplit_once('_') else {
        return false;
    };
    let Some(db) = formatter.db.as_ref() else {
        return false;
    };
    if let Ok(geodetic) =
        AuthorityFactory::create(db, authority).create_geodetic_datum(geodetic_code)
    {
        export_geodetic_datum(&geodetic, formatter);
        return true;
    }
    false
}

/// Type `2002` whose WKT named the geodetic datum but carried no `from_geogdatum_` code.
fn export_esri_type_2002_by_name(
    datum: &VerticalReferenceFrame,
    formatter: &mut WktFormatter,
) -> bool {
    let Some(db) = formatter.db.as_ref() else {
        return false;
    };
    let Ok(found) =
        AuthorityFactory::create(db, "EPSG").create_geodetic_datums_by_name(datum.name())
    else {
        return false;
    };
    let Some(geodetic) = found.into_iter().next() else {
        return false;
    };
    export_geodetic_datum(&geodetic, formatter);
    true
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
        } else if !esri_official_name(&name, "vertical_crs", formatter) {
            name = morph_name_to_esri(&name);
        }
    }
    formatter.start_node(keyword, node_from_ids(&crs.identified.identifiers));
    formatter.add_quoted(&name);
    if let Some(datum) = &crs.datum {
        if !(formatter.esri
            && datum.wkt1_type == "2002"
            && (export_esri_type_2002_datum(datum, formatter)
                || export_esri_type_2002_by_name(datum, formatter)))
        {
            export_vertical_datum(datum, formatter);
        }
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
    let previous_geog = formatter.compound_geog_name.clone();
    if let Some(first) = crs.components.first() {
        formatter.compound_geog_name =
            super::bound::extract_geographic(first).map(|geographic| geographic.name().to_owned());
    }
    for component in &crs.components {
        export_crs(component, formatter)?;
    }
    formatter.compound_geog_name = previous_geog;
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
    if let CoordinateOperation::Transformation(transformation) = &crs.transformation {
        if let Some(grid) = transformation.height_to_geographic_3d_filename() {
            let hub_name = crs.target_crs().name();
            let for_compound = formatter.compound_geog_name.as_deref() == Some(hub_name);
            if hub_is_wgs84(crs.target_crs()) || for_compound {
                formatter.vdatum_grid = Some(grid.to_owned());
                let exported = export_crs(crs.source_crs(), formatter);
                formatter.vdatum_grid = None;
                return exported;
            }
        }
        if hub_is_wgs84(crs.target_crs()) && transformation.method().code() == "9615" {
            if let Some(grid) = transformation.nadgrids_filename() {
                formatter.hdatum_grid = Some(grid.to_owned());
                let exported = export_crs(&crs.source, formatter);
                formatter.hdatum_grid = None;
                return exported;
            }
        }
        if hub_is_wgs84(crs.target_crs()) {
            if let Ok(Some(params)) = transformation.towgs84_strict() {
                formatter.towgs84 = Some(params);
                let exported = export_crs(&crs.source, formatter);
                formatter.towgs84 = None;
                return exported;
            }
        }
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
    if formatter.wkt2
        || (!crs.datum.name().is_empty() && crs.datum.name() != "Unknown engineering datum")
    {
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

impl super::model::CartesianCS {
    /// Write this coordinate system into `formatter` and return the WKT. WKT1 has no
    /// standalone coordinate system node, so the result is empty there.
    #[must_use]
    pub fn export_to_wkt(&self, formatter: &mut WktFormatter) -> String {
        let previous = formatter.output_axis;
        formatter.output_axis = OutputAxisRule::Yes;
        export_cs(&CoordinateSystem::Cartesian(self.clone()), formatter);
        formatter.output_axis = previous;
        std::mem::take(&mut formatter.result)
    }
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
    if !formatter.wkt2 {
        if let Some(params) = formatter.towgs84 {
            formatter.start_node("TOWGS84", NodeId::Absent);
            for value in params {
                formatter.add_number(value);
            }
            formatter.end_node();
        }
        if let Some(grid) = formatter.hdatum_grid.clone() {
            formatter.start_node("EXTENSION", NodeId::Absent);
            formatter.add_quoted("PROJ4_GRIDS");
            formatter.add_quoted(&grid);
            formatter.end_node();
        }
    }
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
        if let Some(grid) = formatter.vdatum_grid.clone() {
            formatter.start_node("EXTENSION", NodeId::Absent);
            formatter.add_quoted("PROJ4_GRIDS");
            formatter.add_quoted(&grid);
            formatter.end_node();
        }
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
            realization_method: None,
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
    if formatter.wkt2 {
        if let Some(anchor) = datum.anchor_definition() {
            formatter.start_node("ANCHOR", NodeId::Absent);
            formatter.add_quoted(anchor);
            formatter.end_node();
        }
    } else {
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

/// WKT2:2019 names a temporal CS `TemporalDateTime`, `TemporalCount`, or `TemporalMeasure`.
fn cs_wkt2_type(cs: &CoordinateSystem, formatter: &WktFormatter) -> &'static str {
    if let CoordinateSystem::Temporal(cs) = cs {
        if !formatter.keywords_2019 {
            return "temporal";
        }
        return match cs.kind() {
            super::niche::TemporalCsKind::DateTime => "TemporalDateTime",
            super::niche::TemporalCsKind::Count => "TemporalCount",
            super::niche::TemporalCsKind::Measure => "TemporalMeasure",
        };
    }
    cs.wkt2_type()
}

fn export_cs(cs: &CoordinateSystem, formatter: &mut WktFormatter) {
    if formatter.output_axis != OutputAxisRule::Yes {
        return;
    }
    let axes = cs.axes();
    if formatter.wkt2 {
        formatter.start_node("CS", node_from_ids(&cs.identified().identifiers));
        formatter.add_raw(cs_wkt2_type(cs, formatter));
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
            } else {
                if !designation.is_empty() {
                    designation.push(' ');
                }
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

/// The CRS a conversion is exported with. The WKT1 ESRI method choice reads its
/// name, and its base ellipsoid for a Mercator variant A conversion.
#[derive(Clone, Copy)]
struct OwnerCrs<'a> {
    name: &'a str,
    squared_eccentricity: Option<f64>,
}

fn export_conversion(
    conversion: &Conversion,
    formatter: &mut WktFormatter,
    base_shape: BaseShape,
    owner: Option<OwnerCrs<'_>>,
) -> Result<(), FactoryError> {
    if !formatter.wkt2
        && conversion
            .method()
            .name()
            .starts_with("PROJ-based operation method")
    {
        return Err(FactoryError::Format(
            "PROJBasedOperation can only be exported to WKT2".to_owned(),
        ));
    }
    if !formatter.wkt2 && formatter.esri {
        export_conversion_wkt1(conversion, formatter, base_shape, owner)?;
        return Ok(());
    }
    if formatter.wkt2 {
        formatter.start_node(
            if formatter.use_deriving_conversion {
                "DERIVINGCONVERSION"
            } else {
                "CONVERSION"
            },
            node_from_ids(&conversion.identified.identifiers),
        );
        formatter.add_quoted(conversion.name());
        let inverse_method;
        let method = if conversion.inverted {
            inverse_method = inverted_method(&conversion.method);
            &inverse_method
        } else {
            &conversion.method
        };
        export_method(method, formatter)?;
        for parameter in &conversion.parameters {
            if omitted_zero_parameter(conversion, parameter) {
                continue;
            }
            export_parameter(parameter, formatter)?;
        }
        export_interpolation_code(conversion, formatter)?;
        formatter.format_ids(&conversion.identified.identifiers);
        formatter.end_node();
        return Ok(());
    }
    export_conversion_wkt1(conversion, formatter, base_shape, owner)
}

/// An interpolation CRS with an EPSG code is written as the `EPSG code for ... CRS` parameter,
/// unless the conversion already has that parameter.
fn export_interpolation_code(
    conversion: &Conversion,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let Some(crs) = conversion.interpolation_crs() else {
        return Ok(());
    };
    let is_code_parameter = |parameter: &Parameter| {
        parameter.code_space == "EPSG" && matches!(parameter.code.as_str(), "1048" | "1037")
    };
    if conversion.parameters.iter().any(is_code_parameter) {
        return Ok(());
    }
    let Some(code) = crs
        .authority_code("EPSG")
        .and_then(|identifier| identifier.code().parse::<i64>().ok())
    else {
        return Ok(());
    };
    let (name, parameter_code) = if conversion.method.code == "1046" {
        ("EPSG code for Horizontal CRS", "1037")
    } else {
        ("EPSG code for Interpolation CRS", "1048")
    };
    export_parameter(
        &Parameter {
            name: name.to_owned(),
            code_space: "EPSG".to_owned(),
            code: parameter_code.to_owned(),
            value: ParameterValue::Integer(code),
        },
        formatter,
    )
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
    owner: Option<OwnerCrs<'_>>,
) -> Result<(), FactoryError> {
    if formatter.esri {
        let squared = owner.and_then(|owner| owner.squared_eccentricity);
        if let Some(variant_b) = mercator_variant_b(conversion, squared)? {
            return export_conversion(&variant_b, formatter, base_shape, owner);
        }
    }
    formatter.output_unit.push(false);
    formatter.output_id.push(false);
    if formatter.esri {
        let sphere = base_shape == BaseShape::Sphere && conversion.method.code == "1026";
        let code = conversion.method.code.parse::<i32>().unwrap_or(0);
        let (name, code) = if sphere {
            ("Mercator (variant A)", 9804)
        } else {
            (conversion.method.name(), code)
        };
        if let Some(choice) = esri_method(conversion, name, code, owner.map(|owner| owner.name)) {
            export_esri_method(conversion, &choice, formatter);
            formatter.output_id.pop();
            formatter.output_unit.pop();
            return Ok(());
        }
    }
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
        if conversion.method.name.starts_with("PROJ ") {
            formatter.output_id.pop();
            formatter.output_unit.pop();
            return Ok(());
        }
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
        // A WKT1 Hotine without a rectified grid angle gets the azimuth as its
        // angle, as PROJ writes it for GDAL.
        if !formatter.esri
            && matches!(conversion.method.code.as_str(), "9812" | "9815")
            && !conversion
                .parameters
                .iter()
                .any(|parameter| parameter.code == "8814")
        {
            if let Some(azimuth) = conversion.parameters.iter().find(|p| p.code == "8813") {
                formatter.start_node("PARAMETER", NodeId::Absent);
                formatter.add_quoted("rectified_grid_angle");
                add_parameter_value(azimuth, formatter)?;
                formatter.end_node();
            }
        }
    }
    formatter.output_id.pop();
    formatter.output_unit.pop();
    Ok(())
}

/// The `PROJECTION` and `PARAMETER` nodes of an ESRI method (PROJ `_exportToWKT`).
fn export_esri_method(conversion: &Conversion, choice: &EsriChoice, formatter: &mut WktFormatter) {
    formatter.start_node("PROJECTION", NodeId::Absent);
    formatter.add_quoted(choice.name);
    formatter.end_node();
    for param in choice.params {
        formatter.start_node("PARAMETER", NodeId::Absent);
        match *param {
            EsriParam::Value {
                esri_name,
                wkt2_name,
                epsg_code,
            }
            | EsriParam::Defaulted {
                esri_name,
                wkt2_name,
                epsg_code,
                ..
            } => {
                formatter.add_quoted(esri_name);
                write_esri_value(conversion, esri_name, wkt2_name, epsg_code, formatter);
            }
            EsriParam::Fixed { esri_name, text } => {
                formatter.add_quoted(esri_name);
                formatter.add_raw(text);
            }
        }
        formatter.end_node();
    }
}

/// An ESRI parameter value in the axis units. A missing parameter is 1 for a
/// scale name and 0 otherwise.
fn write_esri_value(
    conversion: &Conversion,
    esri_name: &str,
    wkt2_name: &str,
    epsg_code: i32,
    formatter: &mut WktFormatter,
) {
    let Some(ParameterValue::Measure(measure)) =
        find_parameter(conversion, wkt2_name, epsg_code).map(|parameter| &parameter.value)
    else {
        let scale = esri_name.to_ascii_lowercase().contains("scale");
        formatter.add_number(if scale { 1.0 } else { 0.0 });
        return;
    };
    match measure.unit.kind {
        UnitKind::Linear => {
            let target = formatter
                .axis_linear
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::metre);
            formatter.add_number(measure.convert_to(&target));
        }
        UnitKind::Angular => {
            let target = formatter
                .axis_angular
                .last()
                .cloned()
                .unwrap_or_else(UnitOfMeasure::degree);
            let mut angle = measure.convert_to(&target);
            if target.factor == UnitOfMeasure::degree().factor {
                if angle > 180.0 {
                    angle -= 360.0;
                } else if angle < -180.0 {
                    angle += 360.0;
                }
            }
            formatter.add_number(angle);
        }
        _ => formatter.add_number(measure.si_value()),
    }
}

/// The method of an inverted conversion, as PROJ `InverseConversion` writes it in
/// WKT2: an `Inverse of` name and an `INVERSE(...)` code space.
fn inverted_method(method: &OperationMethod) -> OperationMethod {
    OperationMethod {
        name: format!("Inverse of {}", method.name),
        code_space: format!("INVERSE({})", method.code_space),
        code: method.code.clone(),
        parameters: method.parameters.clone(),
    }
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
    if formatter.output_id_now() && has_code(&method.code) {
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

/// A method or parameter code that PROJ would write as `ID`. The table's `0` means no EPSG
/// code, and PROJ writes no `ID` for such an object.
fn has_code(code: &str) -> bool {
    !code.is_empty() && code != "0"
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
    if formatter.output_id_now() && has_code(&parameter.code) {
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
            // `ParameterValue::_exportToWKT`: a scale in parts per million is
            // written as the unity scale `1 + ppm * 1e-6`.
            UnitKind::Scale if measure.unit.name == "parts per million" => {
                formatter.add_number(1.0 + measure.value * 1e-6);
            }
            _ => formatter.add_number(measure.value),
        }
        return Ok(());
    }
    if formatter.wkt2 {
        formatter.add_number(measure.value);
        // A WKT `PARAMETER` read without a unit carries an unnamed unit: no `UNIT` on output.
        let unitless = measure.unit.kind == UnitKind::None
            || (measure.unit.kind == UnitKind::Unknown && measure.unit.name.is_empty());
        if !unitless {
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
            export_conversion(conversion, formatter, BaseShape::NotSphere, None)
        }
        super::model::CoordinateOperation::Transformation(transformation) => {
            export_transformation(transformation, formatter)
        }
        super::model::CoordinateOperation::Concatenated(concatenated) => {
            export_concatenated(concatenated, formatter)
        }
    }
}

/// `ConcatenatedOperation::_exportToWKT`: WKT2:2019 only. The CRS pair and each step are
/// written as they are on their own, identifiers included, as PROJ's `pushHasId(false)` makes
/// them.
fn export_concatenated(
    concatenated: &super::model::ConcatenatedOperation,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    if !formatter.wkt2 || !formatter.keywords_2019 {
        return Err(FactoryError::Format(
            "Transformation can only be exported to WKT2:2019".to_owned(),
        ));
    }
    let (source, target) = concatenated.crs_pair().ok_or_else(|| {
        FactoryError::Format(format!(
            "concatenated operation {} has no CRS pair",
            concatenated.name()
        ))
    })?;
    formatter.start_node(
        "CONCATENATEDOPERATION",
        node_from_ids(&concatenated.identified.identifiers),
    );
    formatter.add_quoted(concatenated.name());
    if let Some(version) = &concatenated.version {
        formatter.start_node("VERSION", NodeId::Absent);
        formatter.add_quoted(version);
        formatter.end_node();
    }
    let disabled = usize::from(!concatenated.identified.domains.is_empty());
    formatter.usage_disabled += disabled;
    let exported = export_concatenated_body(concatenated, source, target, formatter);
    formatter.usage_disabled -= disabled;
    exported?;
    if let Some(text) = concatenated.accuracy_text() {
        formatter.start_node("OPERATIONACCURACY", NodeId::Absent);
        formatter.add_raw(&text);
        formatter.end_node();
    }
    formatter.base_usage(&concatenated.identified);
    formatter.end_node();
    Ok(())
}

fn export_concatenated_body(
    concatenated: &super::model::ConcatenatedOperation,
    source: &Crs,
    target: &Crs,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let can_export_id = !(formatter.id_on_top_only && top_has_id(formatter));
    for (keyword, crs) in [("SOURCECRS", source), ("TARGETCRS", target)] {
        formatter.start_node(keyword, NodeId::Absent);
        if can_export_id && !crs.identifiers().is_empty() {
            formatter.simul_cur_node_has_id_cleared();
        }
        export_crs(crs, formatter)?;
        formatter.end_node();
    }
    for step in concatenated.steps() {
        formatter.start_node("STEP", NodeId::Absent);
        if can_export_id && !step.operation().identifiers().is_empty() {
            formatter.simul_cur_node_has_id_cleared();
        }
        export_operation(step.operation(), formatter)?;
        formatter.end_node();
    }
    Ok(())
}

/// `exportSourceCRSAndTargetCRSToWKT`: the CRS keep their identifier in WKT2:2019 unless
/// identifiers are written on the top level only.
fn export_source_and_target(
    source: Option<&Crs>,
    target: Option<&Crs>,
    usage_disabled: usize,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let can_export_id = formatter.wkt2
        && formatter.keywords_2019
        && !(formatter.id_on_top_only && top_has_id(formatter));
    formatter.usage_disabled += usage_disabled;
    let mut result = Ok(());
    for (keyword, crs) in [("SOURCECRS", source), ("TARGETCRS", target)] {
        let Some(crs) = crs else {
            continue;
        };
        formatter.start_node(keyword, NodeId::Absent);
        let fake_no_id = can_export_id && !crs.identifiers().is_empty();
        if fake_no_id {
            formatter.stack_has_id.push(false);
        }
        result = export_crs(crs, formatter);
        if fake_no_id {
            formatter.stack_has_id.pop();
        }
        formatter.end_node();
        if result.is_err() {
            break;
        }
    }
    formatter.usage_disabled -= usage_disabled;
    result
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
    if let Some(approximate) = transformation.approximate_inverse() {
        return export_transformation(&approximate, formatter);
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
        // A point motion operation stays in one CRS, so WKT writes the source only.
        let target = if transformation.point_motion {
            None
        } else {
            transformation.target.as_deref()
        };
        export_source_and_target(
            transformation.source.as_deref(),
            target,
            usize::from(!transformation.identified.domains.is_empty()),
            formatter,
        )?;
    }
    if transformation.inverted {
        export_method(&transformation.method.inverse(), formatter)?;
    } else {
        export_method(&transformation.method, formatter)?;
    }
    for parameter in &transformation.parameters {
        export_parameter(parameter, formatter)?;
    }
    if formatter.abridged {
        export_interpolation_parameter(transformation, formatter)?;
    } else if let Some(interpolation) = &transformation.interpolation {
        if !transformation.point_motion {
            formatter.start_node("INTERPOLATIONCRS", NodeId::Absent);
            export_crs(interpolation, formatter)?;
            formatter.end_node();
        }
    }
    if !formatter.abridged {
        if let Some(accuracy) = transformation.accuracy {
            formatter.start_node("OPERATIONACCURACY", NodeId::Absent);
            match &transformation.accuracy_text {
                Some(text) => formatter.add_raw(text),
                None => formatter.add_number(accuracy),
            }
            formatter.end_node();
        }
    }
    formatter.base_usage(&transformation.identified);
    formatter.end_node();
    Ok(())
}

/// An abridged transformation re-emits its interpolation CRS as parameter 1048
/// (or 1037 for method 1046) when that parameter is not already present.
fn export_interpolation_parameter(
    transformation: &Transformation,
    formatter: &mut WktFormatter,
) -> Result<(), FactoryError> {
    let present = transformation.parameters.iter().any(|parameter| {
        matches!(parameter.code(), "1048" | "1037")
            || parameter.name() == "EPSG code for Interpolation CRS"
            || parameter.name() == "EPSG code for Horizontal CRS"
    });
    if present {
        return Ok(());
    }
    let Some(crs) = transformation.interpolation_crs() else {
        return Ok(());
    };
    let Some(identifier) = crs.authority_code("EPSG") else {
        return Ok(());
    };
    let Ok(code) = identifier.code().parse::<i64>() else {
        return Ok(());
    };
    if code == 0 {
        return Ok(());
    }
    let horizontal = transformation.method.code() == "1046";
    let (name, param_code) = if horizontal {
        ("EPSG code for Horizontal CRS", "1037")
    } else {
        ("EPSG code for Interpolation CRS", "1048")
    };
    export_parameter(
        &Parameter {
            name: name.to_owned(),
            code_space: "EPSG".to_owned(),
            code: param_code.to_owned(),
            value: ParameterValue::integer(code),
        },
        formatter,
    )
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
    let nature = if axis_count == 3 {
        "geographic_3D"
    } else {
        "geographic_2D"
    };
    if let Some(name) = formatter
        .db
        .as_ref()
        .and_then(|db| esri_crs_epsg_old_name(identified, table, nature, db))
    {
        return name;
    }
    let mut name = morph_name_to_esri(&identified.name);
    if !name.starts_with("GCS_") {
        name.insert_str(0, "GCS_");
    }
    name
}

/// The `EPSG_OLD` fallback of a geodetic CRS's ESRI name (`GeodeticCRS::_exportToWKT`,
/// crs.cpp:2269-2300), for a CRS with no ESRI alias of its own. The single old EPSG name of the
/// CRS is looked up among the EPSG geodetic CRSs, and the first one of the same nature that has
/// a single ESRI alias gives the name. ETRS89-NOR [EUREF89] (EPSG:10873) has the old name
/// `ETRS89`, which is EPSG:4258, whose ESRI alias is `GCS_ETRS_1989`.
fn esri_crs_epsg_old_name(
    identified: &Identified,
    table: &str,
    nature: &str,
    db: &DatabaseContext,
) -> Option<String> {
    let identifier = identified.identifiers.first()?;
    let old_names = db
        .aliases(&identifier.code_space, &identifier.code, table, "EPSG_OLD")
        .ok()?;
    let [old_name] = old_names.as_slice() else {
        return None;
    };
    let candidates = AuthorityFactory::create(db, "EPSG")
        .create_objects_from_name(old_name, &[ObjectType::GeodeticCrs], NameMatch::Exact, 0)
        .ok()?;
    candidates.iter().find_map(|object| {
        let AuthorityObject::Crs(crs) = object else {
            return None;
        };
        let (candidate_nature, identifiers) = match crs {
            Crs::Geographic(geographic) => {
                let axes = geographic.coordinate_system.axes().len();
                let candidate_nature = if axes == 3 {
                    "geographic_3D"
                } else {
                    "geographic_2D"
                };
                (candidate_nature, geographic.identifiers())
            }
            Crs::Geodetic(geodetic) => {
                let candidate_nature = if matches!(geodetic, GeodeticCRS::Geocentric(_)) {
                    "geocentric"
                } else {
                    "geographic_2D"
                };
                (candidate_nature, geodetic.identifiers())
            }
            _ => return None,
        };
        if candidate_nature != nature {
            return None;
        }
        let first = identifiers.first()?;
        let aliases = db
            .aliases(&first.code_space, &first.code, table, "ESRI")
            .ok()?;
        let [alias] = aliases.as_slice() else {
            return None;
        };
        Some(alias.clone())
    })
}

/// The `EPSG_OLD` fallback of a geodetic datum's ESRI name (`GeodeticReferenceFrame::_exportToWKT`,
/// datum.cpp:1392-1433). ETRS89-NOR [EUREF89] has the old name `European Terrestrial Reference
/// System 1989`, which names the ensemble EPSG:6258 with the ESRI alias `D_ETRS_1989`.
fn esri_datum_epsg_old_name(identified: &Identified, db: &DatabaseContext) -> Option<String> {
    let identifier = identified.identifiers.first()?;
    let old_names = db
        .aliases(
            &identifier.code_space,
            &identifier.code,
            "geodetic_datum",
            "EPSG_OLD",
        )
        .ok()?;
    let [old_name] = old_names.as_slice() else {
        return None;
    };
    let mut name = old_name.clone();
    if name == "European Terrestrial Reference System 1989" {
        name.push_str(" ensemble");
    }
    let candidates = AuthorityFactory::create(db, "EPSG")
        .create_objects_from_name(
            &name,
            &[ObjectType::GeodeticReferenceFrame],
            NameMatch::Exact,
            0,
        )
        .ok()?;
    candidates.iter().find_map(|object| {
        let identifiers = match object {
            AuthorityObject::Datum(Datum::Geodetic(frame)) => frame.identifiers(),
            AuthorityObject::DatumEnsemble(ensemble) => ensemble.identifiers(),
            _ => return None,
        };
        let first = identifiers.first()?;
        let aliases = db
            .aliases(&first.code_space, &first.code, "geodetic_datum", "ESRI")
            .ok()?;
        let [alias] = aliases.as_slice() else {
            return None;
        };
        Some(alias.clone())
    })
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
        if let Some(name) = formatter
            .db
            .as_ref()
            .and_then(|db| esri_datum_epsg_old_name(identified, db))
        {
            return name;
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
    if method.name.starts_with("PROJ ") {
        return Ok("custom_proj4".to_owned());
    }
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
