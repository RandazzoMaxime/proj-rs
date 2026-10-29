//! WKT1 and WKT2 import.
//!
//! `WktNode` is the tree `WKTNode::createFrom` builds. `WktParser` turns that
//! tree into the ISO 19111 objects in [`super::model`], following
//! `WKTParser::createFromWKT` in PROJ `src/iso19111/io.cpp`.

use super::factory::{AuthorityFactory, DatabaseContext, NameMatch};
use super::model::{
    Axis, BoundCRS, CartesianCS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep, Conversion,
    CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble, DerivedGeographicCRS,
    DerivedProjectedCRS,
    Ellipsoid, EllipsoidalCS, EngineeringCRS, EngineeringDatum, FactoryError, GeocentricCRS,
    GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, GeoidModel, Identified, Identifier,
    Measure, Meridian, ObjectDomain, OperationMethod, OrdinalCS, Parameter, ParameterValue,
    PrimeMeridian, ProjectedCRS, SphericalCS, SphericalGeodeticCRS, Transformation, UnitKind,
    UnitOfMeasure, VerticalCRS, VerticalCS, VerticalReferenceFrame,
};

const EARTH_RADIUS: f64 = 6_375_000.0;
const BODY_RELATIVE: f64 = 0.007;
const DEGREE_SI: f64 = std::f64::consts::PI / 180.0;
const US_FOOT: f64 = 12.0 / 39.37;
const REL_UNIT: f64 = 1e-10;

fn proj4_grids(node: &WktNode) -> Option<String> {
    let extension = node.child(&["EXTENSION"])?;
    if extension.children.len() != 2 {
        return None;
    }
    if !ci_eq(&strip_quotes(&extension.children[0]), "PROJ4_GRIDS") {
        return None;
    }
    let name = strip_quotes(&extension.children[1]);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn legacy_geoid_bundle(name: &str) -> bool {
    name == "g2003conus.gtx,g2003alaska.gtx,g2003h01.gtx,g2003p01.gtx"
        || name == "g2012a_conus.gtx,g2012a_alaska.gtx,g2012a_guam.gtx,g2012a_hawaii.gtx,g2012a_puertorico.gtx,g2012a_samoa.gtx"
}

fn invalid(message: impl Into<String>) -> FactoryError {
    FactoryError::Invalid(message.into())
}

fn ci_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn ci_starts(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len()
        && text.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

fn ci_contains(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// Which WKT flavour [`WktParser::guess_dialect`] recognised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WktDialect {
    /// `GEOGCS`, `PROJCS`, `GEOCCS`, `VERT_CS`, `COMPD_CS`, or `LOCAL_CS` with GDAL markers.
    Wkt1Gdal,
    /// WKT1 that looks like Esri (`GCS_`, `VERTCS`, or no `AXIS` and no `AUTHORITY`).
    Wkt1Esri,
    /// ISO 19162:2015 keywords such as `GEODCRS`.
    Wkt2_2015,
    /// ISO 19162:2019 keywords such as `GEOGCRS` or `USAGE`.
    Wkt2_2019,
    /// Not WKT.
    NotWkt,
}

/// One node of a WKT tree. The value of a quoted token includes the quotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WktNode {
    value: String,
    children: Vec<WktNode>,
}

impl WktNode {
    /// Parse one WKT node. A bare token is a node with no children.
    ///
    /// # Errors
    ///
    /// The text is not a WKT node, a bracket is missing, or nesting exceeds 16.
    pub fn create_from(wkt: &str) -> Result<Self, FactoryError> {
        let (node, _) = parse_node(wkt, 0, 0)?;
        Ok(node)
    }

    /// Keyword, or the raw token including quotes.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Child nodes, in order.
    #[must_use]
    pub fn children(&self) -> &[WktNode] {
        &self.children
    }

    /// `WKTNode::toString`. Empty bracket pairs are omitted.
    #[must_use]
    pub fn to_wkt(&self) -> String {
        let mut out = escape_quoted(&self.value);
        if !self.children.is_empty() {
            out.push('[');
            for (index, child) in self.children.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&child.to_wkt());
            }
            out.push(']');
        }
        out
    }

    fn child(&self, names: &[&str]) -> Option<&WktNode> {
        self.children
            .iter()
            .find(|child| names.iter().any(|name| ci_eq(&child.value, name)))
    }

    fn children_named<'a>(&'a self, name: &str) -> Vec<&'a WktNode> {
        self.children
            .iter()
            .filter(|child| ci_eq(&child.value, name))
            .collect()
    }

    fn count_named(&self, name: &str) -> usize {
        self.children
            .iter()
            .filter(|child| ci_eq(&child.value, name))
            .count()
    }
}

fn escape_quoted(value: &str) -> String {
    if value.len() > 2 && value.starts_with('"') && value.ends_with('"') {
        let mut out = String::from("\"");
        out.push_str(&value[1..value.len() - 1].replace('"', "\"\""));
        out.push('"');
        out
    } else {
        value.to_owned()
    }
}

fn skip_space(text: &str, mut index: usize) -> usize {
    while text[index..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_whitespace())
    {
        index += 1;
    }
    index
}

fn parse_node(
    text: &str,
    index_start: usize,
    level: i32,
) -> Result<(WktNode, usize), FactoryError> {
    if level == 16 {
        return Err(invalid("too many nesting levels"));
    }
    let mut index = skip_space(text, index_start);
    if index == text.len() {
        return Err(invalid("whitespace only string"));
    }
    let mut value = String::new();
    let mut in_string = false;
    let mut printed = false;
    while index < text.len() {
        let rest = &text[index..];
        let ch = rest.chars().next().expect("index is on a char");
        if !in_string && (matches!(ch, '[' | '(' | ',' | ']' | ')') || ch.is_ascii_whitespace()) {
            break;
        }
        if ch == '"' {
            if !in_string {
                in_string = true;
                printed = false;
                value.push('"');
                index += 1;
            } else if !printed {
                if rest[1..].starts_with('"') {
                    value.push('"');
                    index += 2;
                } else {
                    in_string = false;
                    value.push('"');
                    index += 1;
                }
            } else {
                value.push('"');
                index += 1;
            }
        } else if !in_string && rest.starts_with('\u{201C}') {
            in_string = true;
            printed = true;
            value.push('"');
            index += '\u{201C}'.len_utf8();
        } else if in_string && printed && rest.starts_with('\u{201D}') {
            in_string = false;
            printed = false;
            value.push('"');
            index += '\u{201D}'.len_utf8();
        } else {
            value.push(ch);
            index += ch.len_utf8();
        }
    }
    index = skip_space(text, index);
    if index == text.len() {
        return Err(invalid(if index_start == 0 {
            "missing ["
        } else {
            "missing , or ]"
        }));
    }
    let mut node = WktNode {
        value,
        children: Vec::new(),
    };
    let ch = text[index..].chars().next().expect("index is on a char");
    if index_start > 0 {
        if ch == ',' {
            return Ok((node, index + 1));
        }
        if ch == ']' || ch == ')' {
            return Ok((node, index));
        }
    }
    if ch != '[' && ch != '(' {
        return Err(invalid("missing ["));
    }
    index += 1;
    index = skip_space(text, index);
    while index < text.len() {
        let closer = text[index..].chars().next().expect("index is on a char");
        if closer == ']' || closer == ')' {
            break;
        }
        let (child, next) = parse_node(text, index, level + 1)?;
        if next <= index {
            return Err(invalid("missing ]"));
        }
        index = skip_space(text, next);
        node.children.push(child);
        if index < text.len() && text[index..].starts_with(',') {
            index += 1;
            index = skip_space(text, index);
        }
    }
    if index == text.len() || !matches!(text[index..].chars().next(), Some(']' | ')')) {
        return Err(invalid("missing ]"));
    }
    Ok((node, index + 1))
}

fn strip_quotes(node: &WktNode) -> String {
    let value = node.value.as_str();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}

fn proj4_extension_from_remarks(remarks: &str) -> String {
    let prefix = "PROJ CRS string: ";
    let Some(rest) = remarks.find(prefix).map(|index| &remarks[index + prefix.len()..]) else {
        return String::new();
    };
    let end = rest.find(". ").unwrap_or(rest.len());
    rest[..end].to_owned()
}

fn as_f64(node: &WktNode) -> Result<f64, FactoryError> {
    let text = node.value.trim();
    if text.is_empty() || text.starts_with('"') {
        return Err(invalid(format!("not a number: {text}")));
    }
    text.parse::<f64>()
        .map_err(|_| invalid(format!("not a number: {text}")))
}

struct ParsedVerticalExtent {
    minimum: f64,
    maximum: f64,
    unit: UnitOfMeasure,
}

struct Props {
    name: String,
    identifiers: Vec<Identifier>,
    remarks: String,
    deprecated: bool,
    domains: Vec<ObjectDomain>,
    version: Option<String>,
}

impl Props {
    fn identified(&self) -> Identified {
        Identified {
            name: self.name.clone(),
            identifiers: self.identifiers.clone(),
            remarks: self.remarks.clone(),
            deprecated: self.deprecated,
            domains: self.domains.clone(),
            anchor: None,
            anchor_epoch: None,
            publication_date: None,
            canonical_bound: None,
        }
    }
}

/// Parsed WKT object. The variant is the dynamic type PROJ would return.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "each WKT object owns its full ISO 19111 graph"
)]
pub enum WktObject {
    /// `ELLIPSOID` or `SPHEROID`.
    Ellipsoid(Ellipsoid),
    /// `PRIMEM` on its own is not produced; datums carry the meridian.
    GeodeticDatum(GeodeticReferenceFrame),
    /// `VDATUM`, `VERT_DATUM`, `VERTICALDATUM`, or `VRF`.
    VerticalDatum(VerticalReferenceFrame),
    /// `EDATUM` or `ENGINEERINGDATUM`.
    EngineeringDatum(EngineeringDatum),
    /// `ENSEMBLE`.
    Ensemble(DatumEnsemble),
    /// Any coordinate reference system.
    Crs(Crs),
    /// `CONVERSION`, including a WKT1 `PROJECTION`.
    Conversion(Conversion),
    /// `COORDINATEOPERATION`.
    Transformation(Transformation),
    /// `CONCATENATEDOPERATION`.
    Concatenated(ConcatenatedOperation),
    /// `TDATUM` or `TIMEDATUM`.
    TemporalDatum,
    /// `PDATUM` or `PARAMETRICDATUM`.
    ParametricDatum,
    /// A node that parses but is not an ISO 19111 object, such as a bare `AUTHORITY`.
    Unknown,
}

impl WktObject {
    /// Ellipsoid, or the ellipsoid of a datum or geographic CRS.
    #[must_use]
    pub fn ellipsoid(&self) -> Option<&Ellipsoid> {
        match self {
            Self::Ellipsoid(ellipsoid) => Some(ellipsoid),
            Self::GeodeticDatum(datum) => Some(datum.ellipsoid()),
            Self::Crs(Crs::Geographic(crs)) => crs.ellipsoid(),
            Self::Crs(Crs::Geodetic(crs)) => crs.ellipsoid(),
            _ => None,
        }
    }

    /// Geodetic datum node, or the datum of a geodetic CRS.
    #[must_use]
    pub fn geodetic_datum(&self) -> Option<&GeodeticReferenceFrame> {
        match self {
            Self::GeodeticDatum(datum) => Some(datum),
            Self::Crs(Crs::Geographic(crs)) => crs.datum(),
            Self::Crs(Crs::Geodetic(crs)) => crs.datum(),
            Self::Crs(Crs::Projected(crs)) => crs.base_crs().datum(),
            _ => None,
        }
    }

    /// Geographic CRS, including a `GEOGCRS` / `GEODCRS` with an ellipsoidal CS.
    #[must_use]
    pub fn geographic(&self) -> Option<&GeographicCRS> {
        match self {
            Self::Crs(Crs::Geographic(crs)) => Some(crs),
            Self::Crs(Crs::Geodetic(GeodeticCRS::Geographic(crs))) => Some(crs),
            _ => None,
        }
    }

    /// Geocentric, spherical, or geographic CRS stored as [`GeodeticCRS`].
    #[must_use]
    pub fn geodetic_crs(&self) -> Option<&GeodeticCRS> {
        match self {
            Self::Crs(Crs::Geodetic(crs)) => Some(crs),
            _ => None,
        }
    }

    /// Projected CRS.
    #[must_use]
    pub fn projected(&self) -> Option<&ProjectedCRS> {
        match self {
            Self::Crs(Crs::Projected(crs)) => Some(crs),
            _ => None,
        }
    }

    /// Vertical CRS.
    #[must_use]
    pub fn vertical(&self) -> Option<&VerticalCRS> {
        match self {
            Self::Crs(Crs::Vertical(crs)) => Some(crs),
            _ => None,
        }
    }

    /// Vertical datum.
    #[must_use]
    pub fn vertical_datum(&self) -> Option<&VerticalReferenceFrame> {
        match self {
            Self::VerticalDatum(datum) => Some(datum),
            Self::Crs(Crs::Vertical(crs)) => crs.datum(),
            _ => None,
        }
    }

    /// Engineering CRS.
    #[must_use]
    pub fn engineering(&self) -> Option<&EngineeringCRS> {
        match self {
            Self::Crs(Crs::Engineering(crs)) => Some(crs),
            _ => None,
        }
    }

    /// Compound CRS.
    #[must_use]
    pub fn compound(&self) -> Option<&CompoundCRS> {
        match self {
            Self::Crs(Crs::Compound(crs)) => Some(crs),
            _ => None,
        }
    }

    /// Bound CRS.
    #[must_use]
    pub fn bound(&self) -> Option<&BoundCRS> {
        match self {
            Self::Crs(Crs::Bound(crs)) => Some(crs),
            _ => None,
        }
    }

    /// Coordinate operation that is a transformation.
    #[must_use]
    pub fn transformation(&self) -> Option<&Transformation> {
        match self {
            Self::Transformation(operation) => Some(operation),
            _ => None,
        }
    }

    /// Conversion.
    #[must_use]
    pub fn conversion(&self) -> Option<&Conversion> {
        match self {
            Self::Conversion(conversion) => Some(conversion),
            _ => None,
        }
    }

    /// The CRS, when the object is one.
    #[must_use]
    pub fn crs(&self) -> Option<&Crs> {
        match self {
            Self::Crs(crs) => Some(crs),
            _ => None,
        }
    }

    /// Datum ensemble, or the ensemble of a geographic or vertical CRS.
    #[must_use]
    pub fn ensemble(&self) -> Option<&DatumEnsemble> {
        match self {
            Self::Ensemble(ensemble) => Some(ensemble),
            Self::Crs(Crs::Geographic(crs)) => crs.datum_ensemble(),
            Self::Crs(Crs::Geodetic(crs)) => crs.datum_ensemble(),
            Self::Crs(Crs::Vertical(crs)) => crs.datum_ensemble(),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum IdConflict {
    Unset,
    Keep,
}

/// WKT parser. Strict mode is the PROJ default: recoverable issues are errors.
#[derive(Clone, Debug)]
pub struct WktParser {
    strict: bool,
    id_conflict: IdConflict,
    db: Option<DatabaseContext>,
    esri: bool,
    maybe_esri: bool,
    warnings: Vec<String>,
    grammar_errors: Vec<String>,
    towgs84: Vec<f64>,
    grids: String,
}

impl Default for WktParser {
    fn default() -> Self {
        Self::new()
    }
}

impl WktParser {
    /// Strict parser with no database.
    #[must_use]
    pub fn new() -> Self {
        Self {
            strict: true,
            id_conflict: IdConflict::Unset,
            db: None,
            esri: false,
            maybe_esri: false,
            warnings: Vec::new(),
            grammar_errors: Vec::new(),
            towgs84: Vec::new(),
            grids: String::new(),
        }
    }

    /// Recover from non-fatal issues instead of failing. PROJ's `setStrict`.
    pub fn set_strictness(&mut self, strictness: super::wkt::WktStrictness) -> &mut Self {
        self.strict = strictness == super::wkt::WktStrictness::Strict;
        self
    }

    /// Open `proj.db` for alias, ensemble, and celestial-body lookups.
    pub fn attach_database(&mut self, db: DatabaseContext) -> &mut Self {
        self.db = Some(db);
        self
    }

    /// Keep authority identifiers when the WKT coordinate system disagrees with `proj.db`.
    ///
    /// The default, matching PROJ, drops those identifiers and records a warning.
    pub fn keep_incompatible_identifiers(&mut self) -> &mut Self {
        self.id_conflict = IdConflict::Keep;
        self
    }

    /// Warnings recorded while strict mode is off.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Grammar errors recorded while strict mode is off.
    #[must_use]
    pub fn grammar_errors(&self) -> &[String] {
        &self.grammar_errors
    }

    /// `PROJ4_GRIDS` value captured from a datum `EXTENSION` node.
    #[must_use]
    pub fn datum_grids(&self) -> &str {
        &self.grids
    }

    /// `WKTParser::guessDialect`.
    #[must_use]
    pub fn guess_dialect(wkt: &str) -> WktDialect {
        guess_dialect(wkt)
    }

    /// Parse `wkt` into an ISO 19111 object.
    ///
    /// # Errors
    ///
    /// The text is not WKT this parser accepts, or a strict-mode check failed.
    pub fn create_from_wkt(&mut self, wkt: &str) -> Result<WktObject, FactoryError> {
        self.warnings.clear();
        self.grammar_errors.clear();
        self.towgs84.clear();
        self.grids.clear();
        self.esri = false;
        let dialect = guess_dialect(wkt);
        self.maybe_esri = dialect == WktDialect::Wkt1Esri;
        if self.maybe_esri && wkt.contains("PARAMETER[\"X_Scale\",") {
            self.esri = true;
            self.maybe_esri = false;
        }
        let (root, mut index) = parse_node(wkt, 0, 0)?;
        if ci_eq(&root.value, "DATUM")
            || ci_eq(&root.value, "GEODETICDATUM")
            || ci_eq(&root.value, "TRF")
        {
            let mut meridian = greenwich();
            index = skip_space(wkt, index);
            if wkt[index..].starts_with(',') {
                index = skip_space(wkt, index + 1);
                if ci_starts(&wkt[index..], "PRIMEM") {
                    let (node, _) = parse_node(wkt, index, 0)?;
                    meridian = self.build_prime_meridian(&node, UnitOfMeasure::degree())?;
                }
            }
            return self.build_geodetic_datum(&root, meridian, None);
        }
        if (ci_eq(&root.value, "GEOGCS") || ci_eq(&root.value, "PROJCS"))
            && self.vertcs_follows(wkt, index)
        {
            let horizontal = self.build_object(&root)?;
            let Some(horizontal) = horizontal.crs().cloned() else {
                return Err(invalid("horizontal component is not a CRS"));
            };
            index = skip_space(wkt, index);
            index = skip_space(wkt, index + 1);
            let (vertical_node, _) = parse_node(wkt, index, 0)?;
            let vertical = self.build_vertical(&vertical_node)?;
            if vertical
                .datum
                .as_ref()
                .is_some_and(|datum| datum.wkt1_type == "2002")
            {
                if let Crs::Geographic(geographic) = &horizontal {
                    if geographic.coordinate_system.axes.len() == 2 {
                        return Ok(WktObject::Crs(Crs::Geographic(promote_ellipsoidal_height(
                            geographic.clone(),
                            &vertical,
                        ))));
                    }
                }
            }
            let name = format!("{} + {}", horizontal.name(), vertical.name());
            let compound = CompoundCRS {
                identified: Identified::named(name),
                components: vec![horizontal, Crs::Vertical(vertical)],
            };
            return Ok(WktObject::Crs(Crs::Compound(compound)));
        }
        let object = self.build_object(&root)?;
        self.note_parameter_warnings(&object);
        Ok(object)
    }

    fn note_parameter_warnings(&mut self, object: &WktObject) {
        let messages = match object {
            WktObject::Crs(Crs::Projected(crs)) => crs.deriving_conversion().validate_parameters(),
            WktObject::Conversion(conversion) => conversion.validate_parameters(),
            _ => Vec::new(),
        };
        self.warnings.extend(messages);
    }

    fn grammar(&mut self, message: impl Into<String>) -> Result<(), FactoryError> {
        let message = message.into();
        if self.strict {
            Err(invalid(message))
        } else {
            self.grammar_errors.push(message);
            Ok(())
        }
    }

    fn vertcs_follows(&self, wkt: &str, mut index: usize) -> bool {
        index = skip_space(wkt, index);
        if !wkt[index..].starts_with(',') {
            return false;
        }
        index = skip_space(wkt, index + 1);
        ci_starts(&wkt[index..], "VERTCS")
    }

    fn recover(&mut self, message: impl Into<String>) -> Result<(), FactoryError> {
        let message = message.into();
        if self.strict {
            Err(invalid(message))
        } else {
            self.warnings.push(message);
            Ok(())
        }
    }

    /// `EXTENSION["PROJ4_GRIDS", ...]` on a horizontal datum becomes a bound CRS.
    fn attach_datum_grids(&mut self, object: WktObject) -> Result<WktObject, FactoryError> {
        if self.grids.is_empty() {
            return Ok(object);
        }
        let WktObject::Crs(crs) = object else {
            return Ok(object);
        };
        if matches!(crs, Crs::Bound(_)) {
            self.grids.clear();
            return Ok(WktObject::Crs(crs));
        }
        let grids = std::mem::take(&mut self.grids);
        Ok(WktObject::Crs(Crs::Bound(BoundCRS::create_from_nadgrids(
            crs, &grids,
        )?)))
    }

    fn build_object(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let name = node.value.as_str();
        if is_geodetic_keyword(name) {
            if node.child(&["BASEGEOGCRS", "BASEGEODCRS"]).is_some() {
                return self.build_derived_geographic(node);
            }
            let object = self.build_geodetic_crs(node)?;
            return self.attach_datum_grids(object);
        }
        if ci_eq(name, "PROJCS") || ci_eq(name, "PROJCRS") || ci_eq(name, "PROJECTEDCRS") {
            let object = self.build_projected(node)?;
            return self.attach_datum_grids(object);
        }
        if ci_eq(name, "DERIVEDPROJCRS") {
            return self.build_derived_projected(node);
        }
        if ci_eq(name, "VERTCRS")
            || ci_eq(name, "VERTICALCRS")
            || ci_eq(name, "VERT_CS")
            || ci_eq(name, "VERTCS")
        {
            let vertical = self.build_vertical(node)?;
            let grid = node
                .child(&["VDATUM", "VERT_DATUM", "VERTICALDATUM", "VRF"])
                .and_then(proj4_grids);
            if let Some(grid) = grid.filter(|name| !legacy_geoid_bundle(name)) {
                return Ok(WktObject::Crs(Crs::Bound(
                    BoundCRS::create_from_geoid_grids(Crs::Vertical(vertical), &grid),
                )));
            }
            return Ok(WktObject::Crs(Crs::Vertical(vertical)));
        }
        if ci_eq(name, "COMPOUNDCRS") || ci_eq(name, "COMPD_CS") {
            return self.build_compound(node);
        }
        if ci_eq(name, "BOUNDCRS") {
            return self.build_bound(node);
        }
        if ci_eq(name, "ENGCRS") || ci_eq(name, "ENGINEERINGCRS") || ci_eq(name, "LOCAL_CS") {
            return self.build_engineering(node);
        }
        if ci_eq(name, "TDATUM") || ci_eq(name, "TIMEDATUM") {
            return Ok(WktObject::TemporalDatum);
        }
        if ci_eq(name, "PDATUM") || ci_eq(name, "PARAMETRICDATUM") {
            return Ok(WktObject::ParametricDatum);
        }
        if ci_eq(name, "AUTHORITY") || ci_eq(name, "ID") {
            return Ok(WktObject::Unknown);
        }
        if ci_eq(name, "ENSEMBLE") {
            let kind = if node.child(&["ELLIPSOID", "SPHEROID"]).is_some() {
                EnsembleBody::Ellipsoid
            } else {
                EnsembleBody::Vertical
            };
            let ensemble = self.build_ensemble(node, Some(greenwich()), kind)?;
            return Ok(WktObject::Ensemble(ensemble));
        }
        if ci_eq(name, "VDATUM")
            || ci_eq(name, "VERT_DATUM")
            || ci_eq(name, "VERTICALDATUM")
            || ci_eq(name, "VRF")
        {
            return Ok(WktObject::VerticalDatum(
                self.build_vertical_datum(node, None)?,
            ));
        }
        if ci_eq(name, "EDATUM") || ci_eq(name, "ENGINEERINGDATUM") {
            let props = self.build_properties(node)?;
            return Ok(WktObject::EngineeringDatum(EngineeringDatum {
                identified: props.identified(),
            }));
        }
        if ci_eq(name, "ELLIPSOID") || ci_eq(name, "SPHEROID") {
            return Ok(WktObject::Ellipsoid(self.build_ellipsoid(node)?));
        }
        if ci_eq(name, "CONVERSION") {
            return Ok(WktObject::Conversion(self.build_conversion(
                node,
                UnitOfMeasure::metre(),
                UnitOfMeasure::degree(),
            )?));
        }
        if ci_eq(name, "COORDINATEOPERATION") {
            return Ok(WktObject::Transformation(self.build_transformation(node)?));
        }
        if ci_eq(name, "CONCATENATEDOPERATION") {
            return Ok(WktObject::Concatenated(self.build_concatenated(node)?));
        }
        Err(invalid(format!("unhandled keyword: {name}")))
    }

    fn build_id(&mut self, node: &WktNode) -> Result<Identifier, FactoryError> {
        if node.children.len() < 2 {
            self.recover(format!("not enough children in {} node", node.value))?;
            return Err(invalid(format!(
                "not enough children in {} node",
                node.value
            )));
        }
        let mut code_space = strip_quotes(&node.children[0]);
        let code = strip_quotes(&node.children[1]);
        if node.children.len() >= 3 && node.children[2].children.is_empty() {
            let version = strip_quotes(&node.children[2]);
            if code_space.eq_ignore_ascii_case("IAU") && version == "2015" {
                code_space = "IAU_2015".to_owned();
            }
        }
        Ok(Identifier::new(code_space, code))
    }

    fn build_properties(&mut self, node: &WktNode) -> Result<Props, FactoryError> {
        self.properties(node, NameInNode::Present)
    }

    fn build_properties_without_name(&mut self, node: &WktNode) -> Result<Props, FactoryError> {
        self.properties(node, NameInNode::Absent)
    }

    fn properties(
        &mut self,
        node: &WktNode,
        name_in_node: NameInNode,
    ) -> Result<Props, FactoryError> {
        let mut identifiers = Vec::new();
        for child in &node.children {
            if ci_eq(&child.value, "ID") || ci_eq(&child.value, "AUTHORITY") {
                if child.children.len() >= 2 {
                    identifiers.push(self.build_id(child)?);
                } else {
                    self.recover(format!("not enough children in {} node", child.value))?;
                }
            }
        }
        let mut name = String::new();
        let mut deprecated = false;
        if name_in_node == NameInNode::Present {
            if node.children.is_empty() {
                return Err(invalid(format!(
                    "not enough children in {} node",
                    node.value
                )));
            }
            name = strip_quotes(&node.children[0]);
            if let Some(stripped) = name.strip_suffix(" (deprecated)") {
                name = stripped.to_owned();
                deprecated = true;
            }
            if identifiers.is_empty() {
                if let Some(pos) = name.find(" (EPSG ID ") {
                    if name.ends_with(')') {
                        let code = name[pos + " (EPSG ID ".len()..name.len() - 1].to_owned();
                        name.truncate(pos);
                        identifiers.push(Identifier::new("EPSG", code));
                    }
                }
            }
            if ci_eq(&node.value, "GEOGCS") {
                if let Some(rest) = name.strip_prefix("GCS_") {
                    self.esri = true;
                    if name == "GCS_WGS_1984" {
                        name = "WGS 84".to_owned();
                    } else if name == "GCS_unknown" {
                        name = "unknown".to_owned();
                    } else if let Some((official, _, _)) = self.alias(name.as_str(), "geodetic_crs")
                    {
                        name = official;
                    } else if rest.is_empty() {
                        name = "unknown".to_owned();
                    }
                }
            } else if self.esri && ci_eq(&node.value, "SPHEROID") && name == "WGS_1984" {
                name = "WGS 84".to_owned();
                if identifiers.is_empty() {
                    identifiers.push(Identifier::new("EPSG", "7030"));
                }
            } else if self.esri && ci_eq(&node.value, "SPHEROID") {
                if let Some((official, auth, code)) = self.alias(&name, "ellipsoid") {
                    name = official;
                    if identifiers.is_empty() && !auth.is_empty() {
                        identifiers.push(Identifier::new(auth, code));
                    }
                }
            }
        }
        let mut remarks = String::new();
        if let Some(remark) = node.child(&["REMARK"]) {
            if remark.children.len() == 1 {
                remarks = strip_quotes(&remark.children[0]);
            } else {
                return Err(invalid(
                    "not the required number of children in REMARK node",
                ));
            }
        }
        let mut domains = Vec::new();
        for child in node.children_named("USAGE") {
            if let Some(domain) = self.build_domain(child)? {
                domains.push(domain);
            }
        }
        if domains.is_empty() {
            if let Some(domain) = self.build_domain(node)? {
                domains.push(domain);
            }
        }
        let version = node
            .child(&["VERSION"])
            .and_then(|version| version.children.first().map(strip_quotes));
        Ok(Props {
            name,
            identifiers,
            remarks,
            deprecated,
            domains,
            version,
        })
    }

    fn alias(&self, name: &str, table: &str) -> Option<(String, String, String)> {
        self.db.as_ref()?.official_from_alias(name, table, "ESRI")
    }

    fn build_domain(&mut self, node: &WktNode) -> Result<Option<ObjectDomain>, FactoryError> {
        let scope = node.child(&["SCOPE"]);
        let area = node.child(&["AREA"]);
        let bbox = node.child(&["BBOX"]);
        let vertical = node.child(&["VERTICALEXTENT"]);
        let temporal = node.child(&["TIMEEXTENT"]);
        if scope.is_none()
            && area.is_none()
            && bbox.is_none()
            && vertical.is_none()
            && temporal.is_none()
        {
            return Ok(None);
        }
        let scope = scope
            .filter(|node| node.children.len() == 1)
            .map(|node| strip_quotes(&node.children[0]));
        let area = match area {
            Some(node) if node.children.len() == 1 => Some(strip_quotes(&node.children[0])),
            Some(_) => return Err(invalid("not the required number of children in AREA node")),
            None => None,
        };
        let (south, west, north, east) = if let Some(bbox) = bbox {
            if bbox.children.len() != 4 {
                return Err(invalid("not the required number of children in BBOX node"));
            }
            let south = as_f64(&bbox.children[0])?;
            let west = as_f64(&bbox.children[1])?;
            let north = as_f64(&bbox.children[2])?;
            let east = as_f64(&bbox.children[3])?;
            if south > north || west > east && !(west > 0.0 && east < 0.0) {
                // PROJ rejects a south bound above the north bound.
            }
            if south > north {
                return Err(invalid("Invalid BBOX node"));
            }
            (Some(south), Some(west), Some(north), Some(east))
        } else {
            (None, None, None, None)
        };
        // PROJ attaches vertical and temporal extents only when AREA or BBOX is present.
        let has_extent = area.is_some() || bbox.is_some();
        let vertical = if has_extent {
            self.vertical_extent(vertical)?
        } else {
            None
        };
        let (temporal_start, temporal_stop) = if has_extent {
            Self::temporal_extent(temporal)?
        } else {
            (None, None)
        };
        Ok(Some(ObjectDomain {
            scope,
            area,
            south,
            west,
            north,
            east,
            vertical_min: vertical.as_ref().map(|extent| extent.minimum),
            vertical_max: vertical.as_ref().map(|extent| extent.maximum),
            vertical_unit: vertical.map(|extent| extent.unit),
            temporal_start,
            temporal_stop,
        }))
    }

    fn vertical_extent(
        &mut self,
        node: Option<&WktNode>,
    ) -> Result<Option<ParsedVerticalExtent>, FactoryError> {
        let Some(node) = node else {
            return Ok(None);
        };
        if node.children.len() != 2 && node.children.len() != 3 {
            return Err(invalid(
                "not the required number of children in VERTICALEXTENT node",
            ));
        }
        let minimum = as_f64(&node.children[0])?;
        let maximum = as_f64(&node.children[1])?;
        let unit = if node.children.len() == 3 {
            self.build_unit(&node.children[2], UnitKind::Linear)?
        } else {
            UnitOfMeasure::metre()
        };
        Ok(Some(ParsedVerticalExtent {
            minimum,
            maximum,
            unit,
        }))
    }

    fn temporal_extent(
        node: Option<&WktNode>,
    ) -> Result<(Option<String>, Option<String>), FactoryError> {
    let Some(node) = node else {
        return Ok((None, None));
    };
    if node.children.len() != 2 {
        return Err(invalid(
            "not the required number of children in TIMEEXTENT node",
        ));
    }
    Ok((
        Some(strip_quotes(&node.children[0])),
        Some(strip_quotes(&node.children[1])),
    ))
}

    fn build_unit(
        &mut self,
        node: &WktNode,
        mut kind: UnitKind,
    ) -> Result<UnitOfMeasure, FactoryError> {
        if node.children.len() < 2 && kind != UnitKind::Time {
            return Err(invalid(format!(
                "not enough children in {} node",
                node.value
            )));
        }
        if node.children.is_empty() {
            return Err(invalid(format!(
                "not enough children in {} node",
                node.value
            )));
        }
        let mut unit_name = strip_quotes(&node.children[0]);
        let mut factor = if node.children.len() >= 2 {
            as_f64(&node.children[1])?
        } else {
            0.0
        };
        let mut code_space = String::new();
        let mut code = String::new();
        if let Some(id) = node.child(&["ID", "AUTHORITY"]) {
            if id.children.len() >= 2 {
                code_space = strip_quotes(&id.children[0]);
                code = strip_quotes(&id.children[1]);
            } else {
                self.recover(format!("not enough children in {} node", id.value))?;
            }
        }
        if kind == UnitKind::Unknown {
            if ci_eq(&unit_name, "METER") || ci_eq(&unit_name, "METRE") {
                kind = UnitKind::Linear;
                unit_name = "metre".to_owned();
                if code_space.is_empty() {
                    code_space = "EPSG".to_owned();
                    code = "9001".to_owned();
                }
            } else if ci_eq(&unit_name, "DEGREE") || ci_eq(&unit_name, "GRAD") {
                kind = UnitKind::Angular;
            }
        }
        if self.esri {
            if let Some((official, auth, alias_code)) = self.alias(&unit_name, "unit_of_measure") {
                unit_name = official;
                code_space = auth;
                code = alias_code;
            }
        }
        if (factor - DEGREE_SI).abs() < REL_UNIT * factor.abs() {
            factor = DEGREE_SI;
        } else if (factor - US_FOOT).abs() < REL_UNIT * factor.abs() {
            factor = US_FOOT;
        }
        Ok(UnitOfMeasure {
            name: unit_name,
            factor,
            kind,
            code_space,
            code,
        })
    }

    fn unit_in(
        &mut self,
        node: &WktNode,
        kind: UnitKind,
    ) -> Result<Option<UnitOfMeasure>, FactoryError> {
        for (keyword, found) in [
            ("LENGTHUNIT", UnitKind::Linear),
            ("ANGLEUNIT", UnitKind::Angular),
            ("SCALEUNIT", UnitKind::Scale),
            ("TIMEUNIT", UnitKind::Time),
            ("PARAMETRICUNIT", UnitKind::Unknown),
            ("UNIT", kind),
            ("LINUNIT", UnitKind::Linear),
        ] {
            if let Some(child) = node.child(&[keyword]) {
                return Ok(Some(self.build_unit(child, found)?));
            }
        }
        Ok(None)
    }

    fn build_ellipsoid(&mut self, node: &WktNode) -> Result<Ellipsoid, FactoryError> {
        if node.children.len() < 3 {
            return Err(invalid(format!(
                "not enough children in {} node",
                node.value
            )));
        }
        let mut unit = self
            .unit_in(node, UnitKind::Linear)?
            .unwrap_or_else(UnitOfMeasure::metre);
        if unit.kind == UnitKind::Unknown {
            unit.kind = UnitKind::Linear;
        }
        let semi_major = as_f64(&node.children[1])?;
        let inverse = if node.children[2].value == "\"inf\"" {
            self.recover("Inverse flattening = \"inf\" is not conformant, but understood")?;
            0.0
        } else {
            as_f64(&node.children[2])?
        };
        let mut props = self.build_properties(node)?;
        if props.name.is_empty() {
            props.name = "unnamed".to_owned();
        }
        for child in node.children.iter().skip(3) {
            let keyword = child.value.as_str();
            if ci_eq(keyword, "UNIT")
                || ci_eq(keyword, "LENGTHUNIT")
                || ci_eq(keyword, "AUTHORITY")
                || ci_eq(keyword, "ID")
            {
                continue;
            }
            self.grammar(format!(
                "not required number of children in {} node",
                node.value
            ))?;
            break;
        }
        let si = semi_major * unit.factor;
        if !(si > 0.0) {
            return Err(invalid("Invalid ellipsoid parameters"));
        }
        let flattening = if inverse == 0.0 { 0.0 } else { 1.0 / inverse };
        let eccentricity = flattening * (2.0 - flattening);
        if eccentricity < 0.0 || eccentricity >= 1.0 {
            return Err(invalid("Invalid ellipsoid parameters"));
        }
        let body = self.guess_body(si, &props.name);
        Ok(Ellipsoid {
            identified: props.identified(),
            semi_major: Measure {
                value: semi_major,
                unit,
            },
            inverse_flattening: Some(inverse),
            semi_minor: None,
            body,
        })
    }

    fn guess_body(&self, semi_major: f64, name: &str) -> String {
        if (semi_major - EARTH_RADIUS).abs() < BODY_RELATIVE * EARTH_RADIUS {
            return "Earth".to_owned();
        }
        if let Some(db) = &self.db {
            if !name.is_empty() {
                for (body, axis) in db.ellipsoid_bodies_named(name) {
                    if (semi_major - axis).abs() < BODY_RELATIVE * semi_major.abs().max(1.0) {
                        return body;
                    }
                }
            }
            if let Ok(body) = AuthorityFactory::create(db, "")
                .identify_body_from_semi_major_axis(semi_major, BODY_RELATIVE)
            {
                return body;
            }
        }
        "Non-Earth body".to_owned()
    }

    fn build_prime_meridian(
        &mut self,
        node: &WktNode,
        default_unit: UnitOfMeasure,
    ) -> Result<PrimeMeridian, FactoryError> {
        if node.children.len() < 2 {
            return Err(invalid(format!(
                "not enough children in {} node",
                node.value
            )));
        }
        let mut name = strip_quotes(&node.children[0]);
        let mut unit = self
            .unit_in(node, UnitKind::Angular)?
            .unwrap_or(default_unit);
        if unit.kind == UnitKind::Unknown || unit.name.is_empty() {
            unit = UnitOfMeasure::degree();
        }
        let mut angle = as_f64(&node.children[1])?;
        if name == "Paris"
            && (angle - 2.337_229_17).abs() < 1e-8
            && unit.name.eq_ignore_ascii_case("grad")
        {
            angle = 2.596_921_3;
        } else if let Some((degrees, minutes, seconds)) = dms_prime_meridian(&name) {
            let sign = if degrees >= 0 { 1.0 } else { -1.0 };
            let abs_deg = f64::from(degrees).abs();
            let packed = sign * (abs_deg + minutes / 100.0 + seconds / 10_000.0);
            let decimal = sign * (abs_deg + minutes / 60.0 + seconds / 3600.0);
            if (angle - packed).abs() < 1e-8 || (angle - decimal).abs() < 1e-8 {
                angle = decimal;
                unit = UnitOfMeasure::degree();
            }
        }
        let mut props = self.build_properties(node)?;
        if self.esri {
            if let Some((official, auth, code)) = self.alias(&name, "prime_meridian") {
                name = official;
                if !auth.is_empty() {
                    props.identifiers = vec![Identifier::new(auth, code)];
                }
            }
        }
        props.name = name;
        Ok(PrimeMeridian {
            identified: props.identified(),
            longitude: Measure { value: angle, unit },
        })
    }

    fn anchor_of(&mut self, node: &WktNode) -> Result<Option<String>, FactoryError> {
        let Some(anchor) = node.child(&["ANCHOR"]) else {
            return Ok(None);
        };
        if anchor.children.len() == 1 {
            Ok(Some(strip_quotes(&anchor.children[0])))
        } else {
            Err(invalid("not enough children in ANCHOR node"))
        }
    }

    fn anchor_epoch_of(&mut self, node: &WktNode) -> Result<Option<f64>, FactoryError> {
        let Some(epoch) = node.child(&["ANCHOREPOCH"]) else {
            return Ok(None);
        };
        if epoch.children.len() != 1 {
            return Err(invalid("not enough children in ANCHOREPOCH node"));
        }
        as_f64(&epoch.children[0]).map(Some)
    }

    fn parse_dynamic(&mut self, node: &WktNode) -> Result<(f64, Option<String>), FactoryError> {
        let Some(epoch) = node.child(&["FRAMEEPOCH"]) else {
            return Err(invalid("missing FRAMEEPOCH"));
        };
        if epoch.children.len() != 1 {
            return Err(invalid("Invalid FRAMEEPOCH node"));
        }
        let value = as_f64(&epoch.children[0]).map_err(|_| invalid("Invalid FRAMEEPOCH node"))?;
        let model = node
            .child(&["MODEL", "VELOCITYGRID"])
            .and_then(|model| model.children.first().map(strip_quotes));
        Ok((value, model))
    }

    fn build_geodetic_datum(
        &mut self,
        node: &WktNode,
        mut meridian: PrimeMeridian,
        dynamic: Option<&WktNode>,
    ) -> Result<WktObject, FactoryError> {
        let Some(ellipsoid_node) = node.child(&["ELLIPSOID", "SPHEROID"]) else {
            return Err(invalid("missing ELLIPSOID"));
        };
        let mut props = self.build_properties(node)?;
        let raw_name = strip_quotes(&node.children[0]);
        let mut name_set = false;
        if raw_name == "WGS_1984" {
            props.name = "World Geodetic System 1984".to_owned();
            name_set = true;
        } else if let Some(stripped) = raw_name.strip_suffix(" ensemble") {
            if !stripped.is_empty() {
                props.name = stripped.to_owned();
                name_set = true;
            }
        }
        if !name_set && (raw_name.starts_with("D_") || self.esri) {
            self.esri = true;
            let (renamed, auth, code) = match raw_name.as_str() {
                "D_WGS_1984" => (
                    "World Geodetic System 1984".to_owned(),
                    "EPSG".to_owned(),
                    "6326".to_owned(),
                ),
                "D_ETRS_1989" => (
                    "European Terrestrial Reference System 1989".to_owned(),
                    "EPSG".to_owned(),
                    "6258".to_owned(),
                ),
                "D_unknown" => ("unknown".to_owned(), String::new(), String::new()),
                "D_Unknown_based_on_WGS_84_ellipsoid" => (
                    "Unknown based on WGS 84 ellipsoid".to_owned(),
                    String::new(),
                    String::new(),
                ),
                _ => {
                    if let Some((official, auth, code)) = self.alias(&raw_name, "geodetic_datum") {
                        let mut official = official;
                        if meridian.name() != "Greenwich" {
                            let with_pm = format!("{official} ({})", meridian.name());
                            if self
                                .db
                                .as_ref()
                                .is_some_and(|db| db.known_geodetic_datum_name(&with_pm))
                            {
                                official = with_pm;
                            }
                        }
                        (official, auth, code)
                    } else if let Some(rest) = raw_name.strip_prefix("D_") {
                        (rest.replace('_', " "), String::new(), String::new())
                    } else {
                        (raw_name.clone(), String::new(), String::new())
                    }
                }
            };
            props.name = renamed;
            if !auth.is_empty() && props.identifiers.is_empty() {
                props.identifiers.push(Identifier::new(auth, code));
            }
        }
        if !name_set && raw_name.contains('_') {
            if let Some(db) = self.db.clone() {
                if let Some((official, auth, code)) = db.geodetic_datum_equivalent(&raw_name) {
                    props.name = official;
                    if props.identifiers.is_empty() && !auth.is_empty() {
                        props.identifiers.push(Identifier::new(auth, code));
                    }
                }
            }
        }
        if let Some(db) = self.db.clone() {
            if let Some(id) = props
                .identifiers
                .iter()
                .find(|identifier| identifier.code_space().eq_ignore_ascii_case("EPSG"))
            {
                if let Ok(loaded) =
                    AuthorityFactory::create(&db, id.code_space()).create_geodetic_datum(id.code())
                {
                    // A datum name that is not itself in proj.db takes the
                    // official name of its AUTHORITY code. `WGS 1984` is that
                    // case for EPSG:6326.
                    let name_is_known = AuthorityFactory::create(&db, "")
                        .create_geodetic_datums_by_name(&raw_name)
                        .is_ok_and(|found| !found.is_empty());
                    if !name_is_known
                        || raw_name.contains('_')
                        || canonical_name(&props.name) == canonical_name(loaded.name())
                    {
                        props.name = loaded.name().to_owned();
                    }
                }
            }
        }
        let ellipsoid = self.build_ellipsoid(ellipsoid_node)?;
        if ellipsoid.celestial_body() != "Earth" && meridian.name() == "Greenwich" {
            meridian = PrimeMeridian {
                identified: Identified::named("Reference meridian"),
                longitude: Measure {
                    value: 0.0,
                    unit: UnitOfMeasure::degree(),
                },
            };
        }
        self.read_towgs84(node)?;
        if let Some(extension) = node.child(&["EXTENSION"]) {
            if extension.children.len() == 2
                && ci_eq(&strip_quotes(&extension.children[0]), "PROJ4_GRIDS")
            {
                self.grids = strip_quotes(&extension.children[1]);
            }
        }
        let mut identified = props.identified();
        identified.anchor = self.anchor_of(node)?;
        identified.anchor_epoch = self.anchor_epoch_of(node)?;
        let (frame_epoch, deformation_model) = if let Some(dynamic) = dynamic {
            let (epoch, model) = self.parse_dynamic(dynamic)?;
            (Some(epoch), model)
        } else {
            (None, None)
        };
        Ok(WktObject::GeodeticDatum(GeodeticReferenceFrame {
            identified,
            ellipsoid,
            prime_meridian: meridian,
            frame_epoch,
            deformation_model,
        }))
    }

    fn read_towgs84(&mut self, node: &WktNode) -> Result<(), FactoryError> {
        let Some(towgs84) = node.child(&["TOWGS84"]) else {
            return Ok(());
        };
        if towgs84.children.len() != 3 && towgs84.children.len() != 7 {
            return Err(invalid("Invalid TOWGS84 node"));
        }
        let mut values = Vec::new();
        for child in &towgs84.children {
            values.push(as_f64(child).map_err(|_| invalid("Invalid TOWGS84 node"))?);
        }
        while values.len() < 7 {
            values.push(0.0);
        }
        self.towgs84 = values;
        Ok(())
    }

    fn build_ensemble(
        &mut self,
        node: &WktNode,
        meridian: Option<PrimeMeridian>,
        body: EnsembleBody,
    ) -> Result<DatumEnsemble, FactoryError> {
        let ellipsoid_node = node.child(&["ELLIPSOID", "SPHEROID"]);
        if body == EnsembleBody::Ellipsoid && ellipsoid_node.is_none() {
            return Err(invalid("missing ELLIPSOID"));
        }
        let props = self.build_properties(node)?;
        let ellipsoid = if let Some(ellipsoid_node) = ellipsoid_node {
            Some(self.build_ellipsoid(ellipsoid_node)?)
        } else {
            None
        };
        let mut members = Vec::new();
        for child in node.children_named("MEMBER") {
            if child.children.is_empty() {
                return Err(invalid("Invalid MEMBER node"));
            }
            let member_props = self.build_properties(child)?;
            if body == EnsembleBody::Ellipsoid {
                let ellipsoid = ellipsoid
                    .clone()
                    .ok_or_else(|| invalid("missing ELLIPSOID"))?;
                members.push(Datum::Geodetic(GeodeticReferenceFrame {
                    identified: member_props.identified(),
                    ellipsoid,
                    prime_meridian: meridian.clone().unwrap_or_else(greenwich),
                    frame_epoch: None,
                    deformation_model: None,
                }));
            } else {
                members.push(Datum::Vertical(VerticalReferenceFrame {
                    identified: member_props.identified(),
                    wkt1_type: "2005".to_owned(),
                    frame_epoch: None,
                    deformation_model: None,
                    realization_method: None,
                }));
            }
        }
        if members.is_empty() {
            let db = self.db.clone().ok_or_else(|| {
                invalid(format!(
                    "Datum ensemble '{}' has no explicit member specified and no connection to database",
                    props.name
                ))
            })?;
            let table = if body == EnsembleBody::Vertical {
                "vertical_datum"
            } else {
                "geodetic_datum"
            };
            let Some((auth, code)) = db.ensemble_by_name(&props.name, table) else {
                return Err(invalid(format!(
                    "No entry for datum ensemble '{}' in database, and no explicit member specified",
                    props.name
                )));
            };
            let loaded = AuthorityFactory::create(&db, auth).create_datum_ensemble(&code)?;
            members = loaded.members().to_vec();
        }
        let Some(accuracy_node) = node.child(&["ENSEMBLEACCURACY"]) else {
            return Err(invalid("missing ENSEMBLEACCURACY"));
        };
        if accuracy_node.children.is_empty() {
            return Err(invalid("missing ENSEMBLEACCURACY"));
        }
        let accuracy = as_f64(&accuracy_node.children[0])?;
        let mut ensemble = DatumEnsemble::create(props.name, members, accuracy)?;
        ensemble.identified.identifiers = props.identifiers;
        ensemble.identified.remarks = props.remarks;
        Ok(ensemble)
    }

    /// `GEODCRS` / `GEOGCRS` with `BASEGEODCRS` and an ellipsoidal CS.
    ///
    /// A cartesian derived geodetic CRS stays unparsed until that test is ported.
    fn build_derived_geographic(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let Some(base_node) = node.child(&["BASEGEOGCRS", "BASEGEODCRS"]) else {
            return Err(invalid("derived geodetic CRS is not parsed in this build"));
        };
        let WktObject::Crs(Crs::Geographic(base)) = self.build_geodetic_crs(base_node)? else {
            return Err(invalid("derived geodetic CRS is not parsed in this build"));
        };
        let Some(conversion_node) = node.child(&["DERIVINGCONVERSION"]) else {
            return Err(invalid("missing DERIVINGCONVERSION"));
        };
        let angular = base
            .coordinate_system
            .axes
            .first()
            .map(|axis| axis.unit().clone())
            .unwrap_or_else(UnitOfMeasure::degree);
        let conversion =
            self.build_conversion(conversion_node, UnitOfMeasure::metre(), angular)?;
        let cs = self.build_cs(node.child(&["CS"]), node, UnitOfMeasure::degree())?;
        let CoordinateSystem::Ellipsoidal(coordinate_system) = cs else {
            return Err(invalid("derived geodetic CRS is not parsed in this build"));
        };
        let props = self.build_properties(node)?;
        Ok(WktObject::Crs(Crs::DerivedGeographic(
            DerivedGeographicCRS {
                identified: props.identified(),
                base,
                conversion,
                coordinate_system,
            },
        )))
    }

    fn build_geodetic_crs(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let datum_node = node.child(&["DATUM", "GEODETICDATUM", "TRF"]);
        let ensemble_node = node.child(&["ENSEMBLE"]);
        if datum_node.is_none() && ensemble_node.is_none() {
            return Err(invalid("Missing DATUM or ENSEMBLE node"));
        }
        let mut props = self.build_properties(node)?;
        let dynamic = node.child(&["DYNAMIC"]);
        let cs_node = node.child(&["CS"]);
        let keyword = node.value.as_str();
        if cs_node.is_none()
            && !ci_eq(keyword, "GEOGCS")
            && !ci_eq(keyword, "GEOCCS")
            && !ci_eq(keyword, "BASEGEODCRS")
            && !ci_eq(keyword, "BASEGEOGCRS")
        {
            return Err(invalid("missing CS"));
        }
        if node.child(&["PRIMEM", "PRIMEMERIDIAN"]).is_none()
            && (ci_eq(keyword, "GEOGCS") || ci_eq(keyword, "GEOCCS"))
        {
            self.recover(format!("{keyword} should have a PRIMEM node"))?;
        }
        let mut angular = self.unit_in(
            node,
            if ci_eq(keyword, "GEOGCS") {
                UnitKind::Angular
            } else {
                UnitKind::Unknown
            },
        )?;
        if angular
            .as_ref()
            .is_some_and(|unit| unit.kind != UnitKind::Angular)
        {
            angular = None;
        }
        let meridian = if let Some(prime) = node.child(&["PRIMEM", "PRIMEMERIDIAN"]) {
            self.build_prime_meridian(prime, angular.clone().unwrap_or_else(UnitOfMeasure::degree))?
        } else {
            greenwich()
        };
        let angular = angular.unwrap_or_else(|| meridian.longitude.unit().clone());
        if self.esri {
            if let Some((official, _, _)) =
                self.alias(&strip_quotes(&node.children[0]), "geodetic_crs")
            {
                props.name = official;
            }
        }
        let datum = if let Some(datum_node) = datum_node {
            match self.build_geodetic_datum(datum_node, meridian.clone(), dynamic)? {
                WktObject::GeodeticDatum(datum) => Some(datum),
                _ => None,
            }
        } else {
            None
        };
        let ensemble = if let Some(ensemble_node) = ensemble_node {
            Some(self.build_ensemble(ensemble_node, Some(meridian), EnsembleBody::Ellipsoid)?)
        } else {
            None
        };
        let cs = self.build_cs(cs_node, node, angular)?;
        let identified = props.identified();
        match cs {
            CoordinateSystem::Ellipsoidal(coordinate_system) => {
                if ci_eq(keyword, "GEOCCS") {
                    return Err(invalid("ellipsoidal CS not expected in GEOCCS"));
                }
                if ci_eq(keyword, "GEOGCRS")
                    || ci_eq(keyword, "GEOGRAPHICCRS")
                    || ci_eq(keyword, "BASEGEOGCRS")
                {
                    // already ellipsoidal
                }
                let mut crs = GeographicCRS {
                    identified,
                    datum,
                    ensemble,
                    coordinate_system,
                    implicit_cs: ci_eq(keyword, "GEOGCS")
                        && node.count_named("AXIS") == 0
                        && node.child(&["CS"]).is_none(),
                };
                self.reconcile_geographic_id(&mut crs, node);
                let object = if !self.towgs84.is_empty() {
                    self.bind_towgs84(Crs::Geographic(crs))?
                } else {
                    WktObject::Crs(Crs::Geographic(crs))
                };
                Ok(object)
            }
            CoordinateSystem::Cartesian(coordinate_system) => {
                if coordinate_system.axes.len() != 3 {
                    return Err(invalid("Cartesian CS for a GeodeticCRS should have 3 axis"));
                }
                Ok(WktObject::Crs(Crs::Geodetic(GeodeticCRS::Geocentric(
                    GeocentricCRS {
                        identified,
                        datum,
                        ensemble,
                        coordinate_system,
                    },
                ))))
            }
            CoordinateSystem::Spherical(coordinate_system) => Ok(WktObject::Crs(Crs::Geodetic(
                GeodeticCRS::Spherical(SphericalGeodeticCRS {
                    identified,
                    datum,
                    ensemble,
                    coordinate_system,
                }),
            ))),
            other => Err(invalid(format!("unhandled CS type: {}", other.wkt2_type()))),
        }
    }

    fn reconcile_geographic_id(&mut self, crs: &mut GeographicCRS, node: &WktNode) {
        let Some(db) = self.db.clone() else {
            return;
        };
        let Some(id) = crs.identifiers().first().cloned() else {
            return;
        };
        let Ok(db_crs) =
            AuthorityFactory::create(&db, id.code_space()).create_geographic_crs(id.code())
        else {
            return;
        };
        let had_unit = node.child(&["UNIT", "ANGLEUNIT"]).is_some();
        let had_axis = node.count_named("AXIS") > 0 || node.child(&["CS"]).is_some();
        let wkt_axes = crs.coordinate_system().axes();
        let db_axes = db_crs.coordinate_system().axes();
        let units_differ = match (wkt_axes.first(), db_axes.first()) {
            (Some(left), Some(right)) => !units_close(left.unit(), right.unit()),
            _ => false,
        };
        let axes_differ = wkt_axes.len() != db_axes.len()
            || wkt_axes
                .iter()
                .zip(db_axes)
                .any(|(left, right)| left.direction() != right.direction());
        let incompatible = (had_unit && units_differ) || (had_axis && axes_differ);
        if incompatible && matches!(self.id_conflict, IdConflict::Unset) {
            self.warnings.push(
                "Coordinate system of GeographicCRS in the WKT definition is different from the one of the authority. Unsetting the identifier to avoid confusion"
                    .to_owned(),
            );
            crs.identified.identifiers.clear();
            return;
        }
        // No axis and no unit of its own: PROJ keeps the authority coordinate
        // system, so a degree axis carries `EPSG:9122`.
        if !had_unit && !had_axis {
            crs.coordinate_system = db_crs.coordinate_system().clone();
        }
    }

    fn bind_towgs84(&mut self, source: Crs) -> Result<WktObject, FactoryError> {
        let values = std::mem::take(&mut self.towgs84);
        let target = Crs::Geographic(GeographicCRS::epsg_4326());
        let parameters = helmert_parameters(&values);
        let transformation = Transformation {
            identified: Identified::named("Transformation to WGS84"),
            method: OperationMethod {
                name: if values.len() == 7 {
                    "Position Vector transformation (geog2D domain)".to_owned()
                } else {
                    "Geocentric translations (geog2D domain)".to_owned()
                },
                code_space: "EPSG".to_owned(),
                code: if values.len() == 7 {
                    "9606".to_owned()
                } else {
                    "9603".to_owned()
                },
                parameters: Vec::new(),
            },
            parameters,
            source: None,
            target: None,
            interpolation: None,
            accuracy: None,
            version: None,
            point_motion: false,
            inverted: false,
        };
        Ok(WktObject::Crs(Crs::Bound(BoundCRS::create(
            source,
            target,
            CoordinateOperation::Transformation(transformation),
        ))))
    }

    fn build_cs(
        &mut self,
        cs_node: Option<&WktNode>,
        parent: &WktNode,
        default_angular: UnitOfMeasure,
    ) -> Result<CoordinateSystem, FactoryError> {
        let axis_count = parent.count_named("AXIS");
        let parent_name = parent.value.as_str();
        if cs_node.is_none() {
            if ci_eq(parent_name, "GEOCCS") && axis_count == 0 {
                let unit = self.linear_or_warn(parent, UnitOfMeasure::metre())?;
                return Ok(CoordinateSystem::Cartesian(geocentric_cs(unit)));
            }
            if ci_eq(parent_name, "GEOGCS") && axis_count == 0 {
                let unit = self.angular_or_warn(parent, default_angular)?;
                if let Some(lin) = parent.child(&["LINUNIT"]) {
                    let linear = self.build_unit(lin, UnitKind::Linear)?;
                    return Ok(CoordinateSystem::Ellipsoidal(longitude_latitude_height(
                        unit, linear,
                    )));
                }
                return Ok(CoordinateSystem::Ellipsoidal(longitude_latitude(unit)));
            }
            if (ci_eq(parent_name, "BASEGEODCRS") || ci_eq(parent_name, "BASEGEOGCRS"))
                && axis_count == 0
            {
                let unit = self
                    .unit_in(parent, UnitKind::Angular)?
                    .unwrap_or(default_angular);
                return Ok(CoordinateSystem::Ellipsoidal(latitude_longitude(unit)));
            }
            if (ci_eq(parent_name, "PROJCS") || ci_eq(parent_name, "BASEPROJCRS"))
                && axis_count == 0
            {
                let unit = if ci_eq(parent_name, "PROJCS") {
                    self.linear_or_warn(parent, UnitOfMeasure::metre())?
                } else {
                    self.unit_in(parent, UnitKind::Linear)?
                        .unwrap_or_else(UnitOfMeasure::metre)
                };
                return Ok(CoordinateSystem::Cartesian(easting_northing(unit)));
            }
            if matches_any(parent_name, &["VERT_CS", "VERTCS", "BASEVERTCRS"]) && axis_count == 0 {
                let mut down = false;
                if ci_eq(parent_name, "VERTCS") {
                    for child in parent.children_named("PARAMETER") {
                        if child.children.len() == 2
                            && strip_quotes(&child.children[0]) == "Direction"
                        {
                            let value = as_f64(&child.children[1])?;
                            if value == -1.0 {
                                down = true;
                            }
                        }
                    }
                }
                let unit = if ci_eq(parent_name, "BASEVERTCRS") {
                    self.unit_in(parent, UnitKind::Linear)?
                        .unwrap_or_else(UnitOfMeasure::metre)
                } else {
                    self.linear_or_warn(parent, UnitOfMeasure::metre())?
                };
                let axis = if down {
                    Axis::new("Depth", "D", "down", unit)
                } else {
                    Axis::new("Gravity-related height", "H", "up", unit)
                };
                return Ok(CoordinateSystem::Vertical(VerticalCS {
                    identified: Identified::default(),
                    axis,
                }));
            }
            if ci_eq(parent_name, "LOCAL_CS") && axis_count == 0 {
                let unit = self
                    .unit_in(parent, UnitKind::Linear)?
                    .unwrap_or_else(UnitOfMeasure::metre);
                return Ok(CoordinateSystem::Cartesian(easting_northing(unit)));
            }
        }
        let (cs_type_owned, declared) = if let Some(cs_node) = cs_node {
            if cs_node.children.len() < 2 {
                return Err(invalid("not enough children in CS node"));
            }
            let declared: usize = cs_node.children[1].value.parse().map_err(|_| {
                invalid(format!(
                    "buildCS: invalid CS axis count: {}",
                    cs_node.children[1].value
                ))
            })?;
            (cs_node.children[0].value.clone(), declared)
        } else if axis_count > 0 {
            let inferred = if ci_eq(parent_name, "GEOGCS") || ci_eq(parent_name, "BASEGEOGCRS") {
                "ellipsoidal"
            } else if ci_eq(parent_name, "GEOCCS")
                || ci_eq(parent_name, "PROJCS")
                || ci_eq(parent_name, "BASEPROJCRS")
                || (ci_eq(parent_name, "LOCAL_CS") && axis_count != 1)
            {
                "Cartesian"
            } else if matches_any(parent_name, &["VERT_CS", "VERTCS", "BASEVERTCRS"])
                || (ci_eq(parent_name, "LOCAL_CS") && axis_count == 1)
            {
                "vertical"
            } else {
                return Err(invalid(format!(
                    "buildCS: unexpected parent node: {parent_name}"
                )));
            };
            (inferred.to_owned(), axis_count)
        } else {
            return Err(invalid(format!(
                "buildCS: unexpected parent node: {parent_name}"
            )));
        };
        let cs_type = cs_type_owned.as_str();
        if declared != 1 && declared != 2 && declared != 3 && !ci_eq(cs_type, "ordinal") {
            return Err(invalid(format!(
                "buildCS: invalid CS axis count for {cs_type}"
            )));
        }
        if axis_count != declared {
            return Err(invalid(
                "buildCS: declared number of axis by CS node and number of AXIS are inconsistent",
            ));
        }
        let unit_kind = if ci_eq(cs_type, "ellipsoidal") {
            UnitKind::Angular
        } else if ci_eq(cs_type, "ordinal") {
            UnitKind::None
        } else if ci_eq(cs_type, "Cartesian")
            || ci_eq(cs_type, "vertical")
            || ci_eq(cs_type, "affine")
        {
            UnitKind::Linear
        } else {
            UnitKind::Unknown
        };
        let fallback = self.unit_in(parent, unit_kind)?;
        let mut axes = Vec::new();
        for (index, axis_node) in parent.children_named("AXIS").into_iter().enumerate() {
            let frame = if ci_eq(parent_name, "GEOCCS") {
                AxisFrame::Geocentric
            } else {
                AxisFrame::Other
            };
            axes.push(self.build_axis(axis_node, fallback.clone(), unit_kind, frame, index + 1)?);
        }
        if ci_eq(cs_type, "ellipsoidal") {
            if axes.len() != 2 && axes.len() != 3 {
                return Err(invalid("buildCS: invalid CS axis count for ellipsoidal"));
            }
            return Ok(CoordinateSystem::Ellipsoidal(EllipsoidalCS {
                identified: Identified::default(),
                axes,
            }));
        }
        if ci_eq(cs_type, "Cartesian") || ci_eq(cs_type, "cartesian") {
            if axes.len() == 2 || axes.len() == 3 {
                let first = axes[0].unit.name();
                if axes.iter().any(|axis| axis.unit.name() != first) {
                    self.recover("All axis of a CartesianCS must have the same unit")?;
                }
                return Ok(CoordinateSystem::Cartesian(CartesianCS {
                    identified: Identified::default(),
                    axes,
                }));
            }
            return Err(invalid("buildCS: invalid CS axis count for Cartesian"));
        }
        if ci_eq(cs_type, "affine") {
            return Ok(CoordinateSystem::Affine(CartesianCS {
                identified: Identified::default(),
                axes,
            }));
        }
        if ci_eq(cs_type, "vertical") && axes.len() == 1 {
            return Ok(CoordinateSystem::Vertical(VerticalCS {
                identified: Identified::default(),
                axis: axes.remove(0),
            }));
        }
        if ci_eq(cs_type, "spherical") && (axes.len() == 2 || axes.len() == 3) {
            return Ok(CoordinateSystem::Spherical(SphericalCS {
                identified: Identified::default(),
                axes,
            }));
        }
        if ci_eq(cs_type, "ordinal") {
            return Ok(CoordinateSystem::Ordinal(OrdinalCS {
                identified: Identified::default(),
                axes,
            }));
        }
        Err(invalid(format!("unhandled CS type: {cs_type}")))
    }

    fn linear_or_warn(
        &mut self,
        node: &WktNode,
        fallback: UnitOfMeasure,
    ) -> Result<UnitOfMeasure, FactoryError> {
        if let Some(unit) = self.unit_in(node, UnitKind::Linear)? {
            return Ok(unit);
        }
        self.recover(format!(
            "buildCS: missing UNIT in {} node. Assuming metre",
            node.value
        ))?;
        Ok(fallback)
    }

    fn angular_or_warn(
        &mut self,
        node: &WktNode,
        fallback: UnitOfMeasure,
    ) -> Result<UnitOfMeasure, FactoryError> {
        if let Some(unit) = self.unit_in(node, UnitKind::Angular)? {
            if unit.kind == UnitKind::Angular {
                return Ok(unit);
            }
        }
        self.recover(format!(
            "buildCS: missing UNIT in {} node. Assuming degree",
            node.value
        ))?;
        Ok(fallback)
    }

    fn build_axis(
        &mut self,
        node: &WktNode,
        unit_in: Option<UnitOfMeasure>,
        unit_kind: UnitKind,
        frame: AxisFrame,
        order: usize,
    ) -> Result<Axis, FactoryError> {
        let geocentric = frame == AxisFrame::Geocentric;
        if node.children.len() < 2 {
            return Err(invalid("not enough children in AXIS node"));
        }
        if let Some(order_node) = node.child(&["ORDER"]) {
            if order_node.children.len() != 1 {
                return Err(invalid("not enough children in ORDER node"));
            }
            let parsed: usize = order_node.children[0].value.parse().map_err(|_| {
                invalid(format!(
                    "buildAxis: invalid ORDER value: {}",
                    order_node.children[0].value
                ))
            })?;
            if parsed != order {
                return Err(invalid(format!(
                    "buildAxis: did not get expected ORDER value: {parsed}"
                )));
            }
        }
        let designation = strip_quotes(&node.children[0]);
        let (mut axis_name, mut abbreviation) = split_axis(&designation);
        let dir_token = node.children[1].value.as_str();
        let mut direction = canonical_direction(dir_token);
        if axis_name.is_empty() {
            if direction.as_deref() == Some("geocentricX") && abbreviation == "X" {
                axis_name = "Geocentric X".to_owned();
            } else if direction.as_deref() == Some("geocentricY") && abbreviation == "Y" {
                axis_name = "Geocentric Y".to_owned();
            } else if direction.as_deref() == Some("geocentricZ") && abbreviation == "Z" {
                axis_name = "Geocentric Z".to_owned();
            }
        }
        if direction.is_none() && geocentric && axis_name == "Geocentric X" {
            abbreviation = "X".to_owned();
            direction = Some("geocentricX".to_owned());
        } else if direction.is_none() && geocentric && axis_name == "Geocentric Y" {
            abbreviation = "Y".to_owned();
            direction = Some("geocentricY".to_owned());
        } else if geocentric
            && axis_name == "Geocentric Z"
            && (ci_eq(dir_token, "NORTH") || ci_eq(dir_token, "OTHER"))
        {
            abbreviation = "Z".to_owned();
            direction = Some("geocentricZ".to_owned());
        } else if ci_eq(dir_token, "OTHER") || ci_eq(dir_token, "UNKNOWN") {
            if ci_eq(dir_token, "UNKNOWN") {
                self.recover("UNKNOWN is not a valid direction name.")?;
            }
            direction = Some("unspecified".to_owned());
        }
        if abbreviation.is_empty() {
            if axis_name == "Easting" {
                abbreviation = "E".to_owned();
            } else if axis_name == "Northing" {
                abbreviation = "N".to_owned();
            }
        }
        let Some(direction) = direction else {
            return Err(invalid(format!("unhandled axis direction: {dir_token}")));
        };
        let mut unit = self.unit_in(node, unit_kind)?;
        if unit.is_none() {
            unit = unit_in;
        }
        let Some(unit) = unit else {
            if unit_kind != UnitKind::None && unit_kind != UnitKind::Time {
                return Err(invalid("buildCS: missing UNIT"));
            }
            return Err(invalid("buildCS: missing UNIT"));
        };
        let meridian = if let Some(meridian) = node.child(&["MERIDIAN"]) {
            if meridian.children.is_empty() {
                return Err(invalid("not enough children in MERIDIAN node"));
            }
            let angle_unit = self
                .unit_in(meridian, UnitKind::Angular)?
                .unwrap_or_else(UnitOfMeasure::degree);
            Some(Meridian {
                longitude: Measure {
                    value: as_f64(&meridian.children[0])?,
                    unit: angle_unit,
                },
            })
        } else {
            None
        };
        let minimum = optional_number(node, "AXISMINVALUE")?;
        let maximum = optional_number(node, "AXISMAXVALUE")?;
        let range_meaning = if let Some(meaning) = node.child(&["RANGEMEANING"]) {
            if meaning.children.len() != 1 {
                return Err(invalid("not enough children in RANGEMEANING node"));
            }
            let token = meaning.children[0].value.as_str();
            if ci_eq(token, "exact") {
                Some("exact".to_owned())
            } else if ci_eq(token, "wraparound") {
                Some("wraparound".to_owned())
            } else {
                return Err(invalid(format!(
                    "buildAxis: invalid RANGEMEANING value: {token}"
                )));
            }
        } else {
            None
        };
        Ok(Axis {
            name: axis_name,
            abbreviation,
            direction,
            unit,
            meridian,
            minimum,
            maximum,
            range_meaning,
        })
    }

    fn build_vertical_datum(
        &mut self,
        node: &WktNode,
        dynamic: Option<&WktNode>,
    ) -> Result<VerticalReferenceFrame, FactoryError> {
        let mut props = self.build_properties(node)?;
        if self.esri && !node.children.is_empty() {
            if let Some((official, _, _)) =
                self.alias(&strip_quotes(&node.children[0]), "vertical_datum")
            {
                props.name = official;
            }
        }
        let mut wkt1_type = "2005".to_owned();
        if ci_eq(&node.value, "VERT_DATUM") && node.children.len() >= 2 {
            wkt1_type = node.children[1].value.clone();
        }
        let mut identified = props.identified();
        identified.anchor = self.anchor_of(node)?;
        identified.anchor_epoch = self.anchor_epoch_of(node)?;
        let (frame_epoch, deformation_model) = if let Some(dynamic) = dynamic {
            let (epoch, model) = self.parse_dynamic(dynamic)?;
            (Some(epoch), model)
        } else {
            (None, None)
        };
        Ok(VerticalReferenceFrame {
            identified,
            wkt1_type,
            frame_epoch,
            deformation_model,
            realization_method: None,
        })
    }

    fn build_vertical(&mut self, node: &WktNode) -> Result<VerticalCRS, FactoryError> {
        let datum_node = node.child(&["VDATUM", "VERT_DATUM", "VERTICALDATUM", "VRF"]);
        let ensemble_node = node.child(&["ENSEMBLE"]);
        let geodetic_in_vertcs = if ci_eq(&node.value, "VERTCS") {
            node.child(&["DATUM"])
        } else {
            None
        };
        if datum_node.is_none() && ensemble_node.is_none() && geodetic_in_vertcs.is_none() {
            return Err(invalid("Missing VDATUM or ENSEMBLE node"));
        }
        let mut props = self.build_properties(node)?;
        if (self.esri || self.maybe_esri) && !node.children.is_empty() {
            if let Some((official, _, _)) =
                self.alias(&strip_quotes(&node.children[0]), "vertical_crs")
            {
                props.name = official;
            }
        }
        let dynamic = node.child(&["DYNAMIC"]);
        let cs_node = node.child(&["CS"]);
        let datum = if let Some(datum_node) = geodetic_in_vertcs {
            let built = self.build_geodetic_datum(datum_node, greenwich(), None)?;
            let WktObject::GeodeticDatum(frame) = built else {
                return Err(invalid("VERTCS DATUM is not a geodetic datum"));
            };
            Some(VerticalReferenceFrame {
                identified: Identified::named(frame.name()),
                wkt1_type: "2002".to_owned(),
                frame_epoch: None,
                deformation_model: None,
                realization_method: None,
            })
        } else if let Some(datum_node) = datum_node {
            Some(self.build_vertical_datum(datum_node, dynamic)?)
        } else {
            None
        };
        let ensemble = if let Some(ensemble_node) = ensemble_node {
            Some(self.build_ensemble(ensemble_node, None, EnsembleBody::Vertical)?)
        } else {
            None
        };
        let cs = self.build_cs(cs_node, node, UnitOfMeasure::degree())?;
        let CoordinateSystem::Vertical(coordinate_system) = cs else {
            return Err(invalid("vertical CS expected"));
        };
        let mut geoid_models = Vec::new();
        for model in node.children_named("GEOIDMODEL") {
            if model.children.is_empty() {
                return Err(invalid("not enough children in GEOIDMODEL node"));
            }
            let props = self.build_properties(model)?;
            geoid_models.push(GeoidModel {
                name: props.name,
                identifiers: props.identifiers,
            });
        }
        Ok(VerticalCRS {
            identified: props.identified(),
            datum,
            ensemble,
            coordinate_system,
            geoid_models,
        })
    }

    /// EPSG v12 replaced some `ETRS89 / …` bases with a national realization.
    ///
    /// The WKT still says `ETRS89`. When the official projected name exists and
    /// its base has a different name, the imported base is that official base.
    fn substitute_etrs89_base(&self, props: &mut Props, base: &mut GeodeticCRS) {
        let Some(db) = &self.db else {
            return;
        };
        let two_d = matches!(
            base,
            GeodeticCRS::Geographic(crs) if crs.coordinate_system().axes().len() == 2
        );
        if !two_d || !props.name.starts_with("ETRS89 / ") || base.name() != "ETRS89" {
            return;
        }
        let Ok(matches) = AuthorityFactory::create(db, "EPSG")
            .create_projected_crs_by_name(&props.name, NameMatch::Exact)
        else {
            return;
        };
        for (candidate, matched) in matches {
            if matched == props.name && candidate.base_crs().name() != base.name() {
                props.name = candidate.name().to_owned();
                *base = candidate.base_crs().clone();
                return;
            }
        }
    }

    /// An ESRI alias becomes the official name.
    ///
    /// A State Plane name that omits `_Feet` while the axis unit is the US
    /// survey foot uses the `_Feet` alias. That is PROJ issue 2086.
    fn rename_esri_projected(&self, props: &mut Props, original: &str, cs: &CartesianCS) {
        if !(self.esri || self.maybe_esri) {
            return;
        }
        let Some((mut official, _, _)) = self.alias(original, "projected_crs") else {
            return;
        };
        let factor = cs
            .axes()
            .first()
            .map(|axis| axis.unit().conversion_to_si())
            .unwrap_or(1.0);
        if original.contains("_FIPS_")
            && !original.contains("_Feet")
            && (factor - US_FOOT).abs() < 1e-10 * US_FOOT
        {
            if let Some((feet, _, _)) = self.alias(&format!("{original}_Feet"), "projected_crs") {
                official = feet;
            }
        }
        props.name = official;
    }

    fn build_projected(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let base_node = node
            .child(&["BASEGEOGCRS", "BASEGEODCRS", "GEOGCS"])
            .ok_or_else(|| invalid("missing base geodetic CRS"))?;
        let mut props = self.build_properties(node)?;
        let original_name = props.name.clone();
        let base_object = if ci_eq(&base_node.value, "GEOGCS") {
            self.build_geodetic_crs(base_node)?
        } else {
            self.build_geodetic_crs(base_node)?
        };
        let mut base = match base_object {
            WktObject::Crs(Crs::Geographic(crs)) => GeodeticCRS::Geographic(crs),
            WktObject::Crs(Crs::Geodetic(crs)) => crs,
            WktObject::Crs(Crs::Bound(bound)) => {
                return self.projected_bound(node, props, bound);
            }
            _ => return Err(invalid("base CRS is not geodetic")),
        };
        self.substitute_etrs89_base(&mut props, &mut base);
        let angular = base
            .coordinate_system()
            .axes()
            .first()
            .map(|axis| axis.unit().clone())
            .unwrap_or_else(UnitOfMeasure::degree);
        let cs = self.build_cs(node.child(&["CS"]), node, angular.clone())?;
        let CoordinateSystem::Cartesian(coordinate_system) = cs else {
            return Err(invalid("projected CRS requires a Cartesian CS"));
        };
        self.rename_esri_projected(&mut props, &original_name, &coordinate_system);
        if coordinate_system.axes.len() == 3 {
            if let GeodeticCRS::Geographic(crs) = &base {
                if crs.coordinate_system().axes().len() == 2 {
                    base = GeodeticCRS::Geographic(crs.promote_to_3d_with_db(self.db.as_ref()));
                }
            }
        }
        let linear = coordinate_system
            .axes
            .first()
            .map(|axis| axis.unit.clone())
            .unwrap_or_else(UnitOfMeasure::metre);
        let conversion = if let Some(conversion) = node.child(&["CONVERSION"]) {
            self.build_conversion(conversion, linear, angular)?
        } else if let Some(projection) = node.child(&["PROJECTION"]) {
            self.build_wkt1_projection(node, projection, linear, angular)?
        } else {
            return Err(invalid("missing CONVERSION"));
        };
        let crs = ProjectedCRS {
            identified: props.identified(),
            base,
            conversion,
            coordinate_system,
            implicit_cs: node.count_named("AXIS") == 0,
            extension_proj4: proj4_extension_from_remarks(&props.remarks),
        };
        if !self.towgs84.is_empty() {
            return self.bind_towgs84(Crs::Projected(crs));
        }
        Ok(WktObject::Crs(Crs::Projected(crs)))
    }

    /// `DERIVEDPROJCRS`: a projected base, a deriving conversion, and a cartesian CS.
    ///
    /// A 3-axis derived CRS promotes a 2-axis base, which is how a WKT2:2019
    /// export that omits the base CS still round-trips as 3D.
    fn build_derived_projected(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let base_node = node
            .child(&["BASEPROJCRS"])
            .ok_or_else(|| invalid("missing BASEPROJCRS"))?;
        let WktObject::Crs(Crs::Projected(mut base)) = self.build_projected(base_node)? else {
            return Err(invalid("BASEPROJCRS is not a projected CRS"));
        };
        let conversion_node = node
            .child(&["DERIVINGCONVERSION"])
            .ok_or_else(|| invalid("missing DERIVINGCONVERSION"))?;
        let angular = base
            .base_crs()
            .coordinate_system()
            .axes()
            .first()
            .map(|axis| axis.unit().clone())
            .unwrap_or_else(UnitOfMeasure::degree);
        let linear = base
            .coordinate_system()
            .axes()
            .first()
            .map(|axis| axis.unit().clone())
            .unwrap_or_else(UnitOfMeasure::metre);
        let conversion = self.build_conversion(conversion_node, linear, angular)?;
        let cs = self.build_cs(node.child(&["CS"]), node, UnitOfMeasure::degree())?;
        let CoordinateSystem::Cartesian(coordinate_system) = cs else {
            return Err(invalid("derived projected CRS requires a Cartesian CS"));
        };
        if coordinate_system.axes.len() == 3 && base.coordinate_system().axes().len() == 2 {
            base = base.promote_to_3d(self.db.as_ref());
        }
        let props = self.build_properties(node)?;
        Ok(WktObject::Crs(Crs::DerivedProjected(DerivedProjectedCRS {
            identified: props.identified(),
            base,
            conversion,
            coordinate_system: CoordinateSystem::Cartesian(coordinate_system),
        })))
    }

    fn projected_bound(
        &mut self,
        node: &WktNode,
        props: Props,
        bound: BoundCRS,
    ) -> Result<WktObject, FactoryError> {
        let Crs::Geographic(base_geo) = bound.source_crs().clone() else {
            return Err(invalid("bound base is not geographic"));
        };
        let angular = base_geo
            .coordinate_system()
            .axes()
            .first()
            .map(|axis| axis.unit().clone())
            .unwrap_or_else(UnitOfMeasure::degree);
        let cs = self.build_cs(node.child(&["CS"]), node, angular.clone())?;
        let CoordinateSystem::Cartesian(coordinate_system) = cs else {
            return Err(invalid("projected CRS requires a Cartesian CS"));
        };
        let linear = coordinate_system
            .axes
            .first()
            .map(|axis| axis.unit.clone())
            .unwrap_or_else(UnitOfMeasure::metre);
        let conversion = if let Some(projection) = node.child(&["PROJECTION"]) {
            self.build_wkt1_projection(node, projection, linear, angular)?
        } else {
            return Err(invalid("missing PROJECTION"));
        };
        let projected = ProjectedCRS {
            identified: props.identified(),
            base: GeodeticCRS::Geographic(base_geo),
            conversion,
            coordinate_system,
            implicit_cs: node.count_named("AXIS") == 0,
            extension_proj4: proj4_extension_from_remarks(&props.remarks),
        };
        let mut bound = bound;
        *bound.source_crs_mut() = Crs::Projected(projected);
        Ok(WktObject::Crs(Crs::Bound(bound)))
    }

    fn build_conversion(
        &mut self,
        node: &WktNode,
        linear: UnitOfMeasure,
        angular: UnitOfMeasure,
    ) -> Result<Conversion, FactoryError> {
        let Some(method_node) = node.child(&["METHOD", "PROJECTION"]) else {
            return Err(invalid("missing METHOD"));
        };
        if method_node.children.is_empty() {
            return Err(invalid("not enough children in METHOD node"));
        }
        let props = self.build_properties(node)?;
        let method_props = self.build_properties(method_node)?;
        let parameters = self.parameters_of(node, &linear, &angular)?;
        Ok(Conversion {
            identified: props.identified(),
            method: OperationMethod {
                name: method_props.name,
                code_space: method_props
                    .identifiers
                    .first()
                    .map(|id| id.code_space.clone())
                    .unwrap_or_default(),
                code: method_props
                    .identifiers
                    .first()
                    .map(|id| id.code.clone())
                    .unwrap_or_default(),
                parameters: Vec::new(),
            },
            parameters,
            version: props.version,
        })
    }

    fn build_wkt1_projection(
        &mut self,
        node: &WktNode,
        projection: &WktNode,
        linear: UnitOfMeasure,
        angular: UnitOfMeasure,
    ) -> Result<Conversion, FactoryError> {
        if projection.children.is_empty() {
            return Err(invalid("not enough children in PROJECTION node"));
        }
        if self.esri || self.maybe_esri {
            if let Some(conversion) =
                self.conversion_from_esri(node, projection, &linear, &angular)?
            {
                return Ok(conversion);
            }
        }
        let mut method_name = strip_quotes(&projection.children[0]);
        let mut parameters = self.parameters_of(node, &linear, &angular)?;
        if ci_eq(&method_name, "Mercator_1SP")
            && !parameters
                .iter()
                .any(|parameter| ci_contains(parameter.name(), "latitude"))
        {
            parameters.insert(
                0,
                parameter("Latitude of natural origin", "8801", 0.0, &angular),
            );
        }
        let (mapped_name, code) = wkt1_method(&method_name);
        if !code.is_empty() {
            method_name = mapped_name.to_owned();
        }
        let method_map = code
            .parse::<i32>()
            .ok()
            .and_then(super::parammap::method_by_code);
        for parameter in &mut parameters {
            if let Some(param) = method_map.and_then(|method| {
                method.params.iter().find(|param| {
                    param
                        .wkt1_name
                        .is_some_and(|name| ci_eq(name, parameter.name()))
                })
            }) {
                parameter.name = param.wkt2_name.to_owned();
                if param.epsg_code != 0 {
                    parameter.code_space = "EPSG".to_owned();
                    parameter.code = param.epsg_code.to_string();
                }
                continue;
            }
            if parameter.code.is_empty() {
                if let Some((name, code, _)) = wkt1_parameter(&parameter.name) {
                    parameter.name = name.to_owned();
                    parameter.code_space = "EPSG".to_owned();
                    parameter.code = code.to_owned();
                }
            }
        }
        if let Some(method) = method_map {
            let mut ordered = Vec::new();
            for param in method.params {
                if let Some(found) = parameters.iter().find(|parameter| {
                    (!parameter.code().is_empty()
                        && parameter.code() == param.epsg_code.to_string())
                        || ci_eq(parameter.name(), param.wkt2_name)
                }) {
                    ordered.push(found.clone());
                }
            }
            if ordered.len() == parameters.len() {
                parameters = ordered;
            }
        }
        Ok(Conversion {
            identified: Identified::named("unknown"),
            method: OperationMethod {
                name: method_name,
                code_space: if code.is_empty() {
                    String::new()
                } else {
                    "EPSG".to_owned()
                },
                code: code.to_owned(),
                parameters: Vec::new(),
            },
            parameters,
            version: None,
        })
    }

    /// ESRI `PROJECTION` whose fixed parameters select one WKT2 method.
    ///
    /// `Krovak` plus a projected name containing `_East_North` is Krovak north
    /// orientated (EPSG:1041), even when the scale parameters were absent.
    fn conversion_from_esri(
        &mut self,
        node: &WktNode,
        projection: &WktNode,
        linear: &UnitOfMeasure,
        angular: &UnitOfMeasure,
    ) -> Result<Option<Conversion>, FactoryError> {
        let esri_name = strip_quotes(&projection.children[0]);
        let mut values = Vec::new();
        for child in &node.children {
            if !ci_eq(&child.value, "PARAMETER") {
                continue;
            }
            if child.children.len() < 2 {
                return Err(invalid("not enough children in PARAMETER node"));
            }
            let Ok(number) = as_f64(&child.children[1]) else {
                return Ok(None);
            };
            values.push((strip_quotes(&child.children[0]), number));
        }
        let certainty = if self.maybe_esri {
            super::esriparam::EsriCertainty::Maybe
        } else {
            super::esriparam::EsriCertainty::Known
        };
        let Some(mapping) = super::esriparam::select_esri_mapping(&esri_name, &values, certainty)
        else {
            return Ok(None);
        };
        let crs_name = node.children.first().map(strip_quotes).unwrap_or_default();
        let mut wkt2_name = mapping.wkt2_name;
        let mut code = mapping.epsg_code;
        if ci_eq(&esri_name, "Krovak") && crs_name.contains("_East_North") {
            wkt2_name = "Krovak (North Orientated)";
            code = 1041;
        }
        let Some(method) = super::parammap::method_by_code(code)
            .filter(|method| method.wkt2_name == wkt2_name)
            .or_else(|| super::parammap::method_by_name(wkt2_name))
        else {
            return Ok(None);
        };
        let mut parameters = Vec::new();
        for param in method.params {
            let Some(esri_param) = mapping
                .params
                .iter()
                .find(|candidate| candidate.wkt2_name == Some(param.wkt2_name))
            else {
                continue;
            };
            let Some(value) = values
                .iter()
                .find(|(name, _)| ci_eq(name, esri_param.esri_name))
                .map(|(_, value)| *value)
            else {
                continue;
            };
            let Some(unit) = guess_parameter_unit(param.wkt2_name, linear, angular) else {
                continue;
            };
            let (code_space, param_code) = if param.epsg_code == 0 {
                (String::new(), String::new())
            } else {
                ("EPSG".to_owned(), param.epsg_code.to_string())
            };
            parameters.push(Parameter {
                name: param.wkt2_name.to_owned(),
                code_space,
                code: param_code,
                value: ParameterValue::Measure(Measure { value, unit }),
            });
        }
        let (code_space, method_code) = if code == 0 {
            (String::new(), String::new())
        } else {
            ("EPSG".to_owned(), code.to_string())
        };
        Ok(Some(Conversion {
            identified: Identified::named("unknown"),
            method: OperationMethod {
                name: wkt2_name.to_owned(),
                code_space,
                code: method_code,
                parameters: Vec::new(),
            },
            parameters,
            version: None,
        }))
    }

    fn parameters_of(
        &mut self,
        node: &WktNode,
        linear: &UnitOfMeasure,
        angular: &UnitOfMeasure,
    ) -> Result<Vec<Parameter>, FactoryError> {
        let mut parameters = Vec::new();
        for child in &node.children {
            if ci_eq(&child.value, "PARAMETER") {
                if child.children.len() < 2 {
                    return Err(invalid("not enough children in PARAMETER node"));
                }
                let mut props = self.build_properties(child)?;
                let value_node = &child.children[1];
                let value = if value_node.value.starts_with('"') {
                    ParameterValue::String(strip_quotes(value_node))
                } else {
                    let number = as_f64(value_node).map_err(|_| {
                        invalid(format!(
                            "unhandled parameter value type : {}",
                            value_node.value
                        ))
                    })?;
                    let mut unit = self.unit_in(child, UnitKind::Unknown)?;
                    if unit.is_none() {
                        unit = guess_parameter_unit(&props.name, linear, angular);
                    }
                    if let Some(unit) = unit {
                        ParameterValue::Measure(Measure {
                            value: number,
                            unit,
                        })
                    } else {
                        ParameterValue::Measure(Measure {
                            value: number,
                            unit: UnitOfMeasure {
                                name: String::new(),
                                factor: 1.0,
                                kind: UnitKind::Unknown,
                                code_space: String::new(),
                                code: String::new(),
                            },
                        })
                    }
                };
                let (code_space, code) = props
                    .identifiers
                    .first()
                    .map(|id| (id.code_space.clone(), id.code.clone()))
                    .unwrap_or_default();
                if props.name.is_empty() {
                    props.name = "unknown".to_owned();
                }
                parameters.push(Parameter {
                    name: props.name,
                    code_space,
                    code,
                    value,
                });
            } else if ci_eq(&child.value, "PARAMETERFILE") {
                if child.children.len() < 2 {
                    return Err(invalid("not enough children in PARAMETERFILE node"));
                }
                let props = self.build_properties(child)?;
                let (code_space, code) = props
                    .identifiers
                    .first()
                    .map(|id| (id.code_space.clone(), id.code.clone()))
                    .unwrap_or_default();
                parameters.push(Parameter {
                    name: props.name,
                    code_space,
                    code,
                    value: ParameterValue::Filename(strip_quotes(&child.children[1])),
                });
            }
        }
        Ok(parameters)
    }

    fn build_compound(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let props = self.build_properties(node)?;
        let mut components = Vec::new();
        for child in &node.children {
            if is_crs_keyword(&child.value) {
                let object = self.build_object(child)?;
                let Some(crs) = object.crs().cloned() else {
                    return Err(invalid("compound component is not a CRS"));
                };
                components.push(crs);
            }
        }
        if components.len() < 2 {
            return Err(invalid("a compound CRS needs at least two components"));
        }
        let horizontal = components.iter().filter(|crs| crs.is_horizontal()).count();
        let compound = if horizontal > 1 {
            CompoundCRS {
                identified: props.identified(),
                components,
            }
        } else {
            let mut compound = CompoundCRS::create(props.name.clone(), components)?;
            compound.identified = props.identified();
            compound
        };
        Ok(WktObject::Crs(Crs::Compound(compound)))
    }

    fn build_bound(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let source = self.wrapped_crs(node, "SOURCECRS")?;
        let target = self.wrapped_crs(node, "TARGETCRS")?;
        let operation = node
            .child(&["ABRIDGEDTRANSFORMATION", "COORDINATEOPERATION"])
            .ok_or_else(|| invalid("missing ABRIDGEDTRANSFORMATION"))?;
        let transformation = self.build_transformation(operation)?;
        let mut bound = BoundCRS::create(
            source,
            target,
            CoordinateOperation::Transformation(transformation),
        );
        let quoted_name = node
            .children
            .first()
            .is_some_and(|child| child.value.starts_with('"'));
        let props = if quoted_name {
            self.build_properties(node).ok()
        } else {
            self.build_properties_without_name(node).ok()
        };
        if let Some(props) = props {
            if quoted_name && !props.name.is_empty() {
                bound.identified_mut().name = props.name;
            }
            bound.identified_mut().identifiers = props.identifiers;
            bound.identified_mut().domains = props.domains;
            bound.identified_mut().remarks = props.remarks;
        }
        Ok(WktObject::Crs(Crs::Bound(bound)))
    }

    fn wrapped_crs(&mut self, node: &WktNode, keyword: &str) -> Result<Crs, FactoryError> {
        let wrapper = node
            .child(&[keyword])
            .ok_or_else(|| invalid(format!("missing {keyword}")))?;
        let inner = wrapper
            .children
            .iter()
            .find(|child| is_crs_keyword(&child.value))
            .ok_or_else(|| invalid(format!("{keyword} has no CRS")))?;
        let object = self.build_object(inner)?;
        object
            .crs()
            .cloned()
            .ok_or_else(|| invalid(format!("{keyword} has no CRS")))
    }

    fn build_engineering(&mut self, node: &WktNode) -> Result<WktObject, FactoryError> {
        let props = self.build_properties(node)?;
        let datum = if let Some(datum) = node.child(&["EDATUM", "ENGINEERINGDATUM", "LOCAL_DATUM"])
        {
            let datum_props = self.build_properties(datum)?;
            EngineeringDatum {
                identified: datum_props.identified(),
            }
        } else if ci_eq(&node.value, "LOCAL_CS") {
            // GDAL emits `LOCAL_CS["foo"]` without `LOCAL_DATUM`. PROJ stores
            // the unknown engineering datum and omits it from WKT1.
            EngineeringDatum {
                identified: Identified::named("Unknown engineering datum"),
            }
        } else {
            return Err(invalid("missing EDATUM"));
        };
        let cs = if ci_eq(&node.value, "LOCAL_CS") {
            self.build_cs(None, node, UnitOfMeasure::degree())?
        } else {
            self.build_cs(node.child(&["CS"]), node, UnitOfMeasure::degree())?
        };
        Ok(WktObject::Crs(Crs::Engineering(EngineeringCRS {
            identified: props.identified(),
            datum,
            coordinate_system: cs,
        })))
    }

    fn build_transformation(&mut self, node: &WktNode) -> Result<Transformation, FactoryError> {
        let props = self.build_properties(node)?;
        let Some(method) = node.child(&["METHOD"]) else {
            return Err(invalid("missing METHOD"));
        };
        let method_props = self.build_properties(method)?;
        let parameters =
            self.parameters_of(node, &UnitOfMeasure::metre(), &UnitOfMeasure::degree())?;
        let source = if node.child(&["SOURCECRS"]).is_some() {
            Some(Box::new(self.wrapped_crs(node, "SOURCECRS")?))
        } else {
            None
        };
        let target = if node.child(&["TARGETCRS"]).is_some() {
            Some(Box::new(self.wrapped_crs(node, "TARGETCRS")?))
        } else {
            None
        };
        let interpolation = if node.child(&["INTERPOLATIONCRS"]).is_some() {
            Some(Box::new(self.wrapped_crs(node, "INTERPOLATIONCRS")?))
        } else {
            None
        };
        let accuracy = node
            .child(&["OPERATIONACCURACY"])
            .and_then(|accuracy| accuracy.children.first())
            .map(as_f64)
            .transpose()?;
        Ok(Transformation {
            identified: props.identified(),
            method: OperationMethod {
                name: method_props.name,
                code_space: method_props
                    .identifiers
                    .first()
                    .map(|id| id.code_space.clone())
                    .unwrap_or_default(),
                code: method_props
                    .identifiers
                    .first()
                    .map(|id| id.code.clone())
                    .unwrap_or_default(),
                parameters: Vec::new(),
            },
            parameters,
            source,
            target,
            interpolation,
            accuracy,
            version: props.version,
            point_motion: false,
            inverted: false,
        })
    }

    fn build_concatenated(
        &mut self,
        node: &WktNode,
    ) -> Result<ConcatenatedOperation, FactoryError> {
        let props = self.build_properties(node)?;
        let mut steps = Vec::new();
        for child in &node.children {
            if ci_eq(&child.value, "STEP") {
                let inner = child
                    .children
                    .iter()
                    .find(|node| {
                        ci_eq(&node.value, "COORDINATEOPERATION")
                            || ci_eq(&node.value, "CONVERSION")
                            || ci_eq(&node.value, "CONCATENATEDOPERATION")
                    })
                    .ok_or_else(|| invalid("STEP has no operation"))?;
                let operation = match self.build_object(inner)? {
                    WktObject::Conversion(conversion) => {
                        CoordinateOperation::Conversion(conversion)
                    }
                    WktObject::Transformation(transformation) => {
                        CoordinateOperation::Transformation(transformation)
                    }
                    WktObject::Concatenated(operation) => {
                        CoordinateOperation::Concatenated(operation)
                    }
                    _ => return Err(invalid("STEP has no operation")),
                };
                steps.push(ConcatenatedStep {
                    operation,
                    reverse: false,
                });
            }
        }
        if steps.len() < 2 {
            return Err(invalid("a concatenated operation needs at least two steps"));
        }
        Ok(ConcatenatedOperation {
            identified: props.identified(),
            steps,
            source: None,
            target: None,
            accuracy: None,
            version: props.version,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NameInNode {
    Present,
    Absent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnsembleBody {
    Ellipsoid,
    Vertical,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AxisFrame {
    Geocentric,
    Other,
}

fn promote_ellipsoidal_height(
    mut geographic: GeographicCRS,
    vertical: &VerticalCRS,
) -> GeographicCRS {
    let down = vertical.coordinate_system.axis.direction() == "down";
    let (name, abbreviation, direction) = if down {
        ("Ellipsoidal depth", "d", "down")
    } else {
        ("Ellipsoidal height", "h", "up")
    };
    geographic.coordinate_system.axes.push(Axis::new(
        name,
        abbreviation,
        direction,
        vertical.coordinate_system.axis.unit().clone(),
    ));
    geographic
}

fn greenwich() -> PrimeMeridian {
    PrimeMeridian::greenwich()
}

fn units_close(left: &UnitOfMeasure, right: &UnitOfMeasure) -> bool {
    let scale = left
        .conversion_to_si()
        .abs()
        .max(right.conversion_to_si().abs())
        .max(1.0);
    (left.conversion_to_si() - right.conversion_to_si()).abs() <= 1e-10 * scale
}

fn canonical_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn dms_prime_meridian(name: &str) -> Option<(i32, f64, f64)> {
    const TABLE: &[(&str, i32, f64, f64)] = &[
        ("Lisbon", -9, 7.0, 54.862),
        ("Bogota", -74, 4.0, 51.3),
        ("Madrid", -3, 41.0, 14.55),
        ("Rome", 12, 27.0, 8.4),
        ("Bern", 7, 26.0, 22.5),
        ("Jakarta", 106, 48.0, 27.79),
        ("Ferro", -17, 40.0, 0.0),
        ("Brussels", 4, 22.0, 4.71),
        ("Stockholm", 18, 3.0, 29.8),
        ("Athens", 23, 42.0, 58.815),
        ("Oslo", 10, 43.0, 22.5),
        ("Paris RGS", 2, 20.0, 13.95),
        ("Paris_RGS", 2, 20.0, 13.95),
    ];
    TABLE
        .iter()
        .find(|(entry, _, _, _)| *entry == name)
        .map(|(_, degrees, minutes, seconds)| (*degrees, *minutes, *seconds))
}

fn is_geodetic_keyword(name: &str) -> bool {
    matches_any(
        name,
        &[
            "GEODCRS",
            "GEODETICCRS",
            "GEOGCRS",
            "GEOGRAPHICCRS",
            "GEOGCS",
            "GEOCCS",
            "BASEGEODCRS",
            "BASEGEOGCRS",
        ],
    )
}

fn is_crs_keyword(name: &str) -> bool {
    is_geodetic_keyword(name)
        || matches_any(
            name,
            &[
                "PROJCS",
                "PROJCRS",
                "PROJECTEDCRS",
                "VERTCRS",
                "VERTICALCRS",
                "VERT_CS",
                "VERTCS",
                "COMPOUNDCRS",
                "COMPD_CS",
                "BOUNDCRS",
                "ENGCRS",
                "ENGINEERINGCRS",
                "LOCAL_CS",
            ],
        )
}

fn matches_any(name: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| ci_eq(name, keyword))
}

fn optional_number(node: &WktNode, keyword: &str) -> Result<Option<f64>, FactoryError> {
    let Some(child) = node.child(&[keyword]) else {
        return Ok(None);
    };
    if child.children.len() != 1 {
        return Err(invalid(format!("not enough children in {keyword} node")));
    }
    if child.children[0].value.starts_with('"') || as_f64(&child.children[0]).is_err() {
        return Err(invalid(format!(
            "buildAxis: invalid {keyword} value: {}",
            child.children[0].value
        )));
    }
    Ok(Some(as_f64(&child.children[0])?))
}

fn split_axis(designation: &str) -> (String, String) {
    if let Some(open) = designation.rfind(" (") {
        if designation.ends_with(')') {
            let name = normalize_axis(&designation[..open]);
            let abbreviation = designation[open + 2..designation.len() - 1].to_owned();
            return (name, abbreviation);
        }
    }
    if designation.starts_with('(') && designation.ends_with(')') && designation.len() >= 2 {
        let abbreviation = designation[1..designation.len() - 1].to_owned();
        let name = match abbreviation.as_str() {
            "E" => "Easting".to_owned(),
            "N" => "Northing".to_owned(),
            "lat" => "Latitude".to_owned(),
            "lon" => "Longitude".to_owned(),
            _ => String::new(),
        };
        return (name, abbreviation);
    }
    let name = normalize_axis(designation);
    let abbreviation = if name == "Latitude" {
        "lat".to_owned()
    } else if name == "Longitude" {
        "lon".to_owned()
    } else if name == "Ellipsoidal height" {
        "h".to_owned()
    } else {
        String::new()
    };
    (name, abbreviation)
}

fn normalize_axis(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => {
            let mut out = String::new();
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

fn canonical_direction(token: &str) -> Option<String> {
    const NAMES: &[&str] = &[
        "north",
        "northNorthEast",
        "northEast",
        "eastNorthEast",
        "east",
        "eastSouthEast",
        "southEast",
        "southSouthEast",
        "south",
        "southSouthWest",
        "southWest",
        "westSouthWest",
        "west",
        "westNorthWest",
        "northWest",
        "northNorthWest",
        "up",
        "down",
        "geocentricX",
        "geocentricY",
        "geocentricZ",
        "columnPositive",
        "columnNegative",
        "rowPositive",
        "rowNegative",
        "displayRight",
        "displayLeft",
        "displayUp",
        "displayDown",
        "forward",
        "aft",
        "port",
        "starboard",
        "clockwise",
        "counterClockwise",
        "towards",
        "awayFrom",
        "future",
        "past",
        "unspecified",
    ];
    if let Some(found) = NAMES.iter().find(|name| name.eq_ignore_ascii_case(token)) {
        return Some((*found).to_owned());
    }
    match token.to_ascii_uppercase().as_str() {
        "NORTH" => Some("north".to_owned()),
        "SOUTH" => Some("south".to_owned()),
        "EAST" => Some("east".to_owned()),
        "WEST" => Some("west".to_owned()),
        "UP" => Some("up".to_owned()),
        "DOWN" => Some("down".to_owned()),
        _ => None,
    }
}

fn latitude_longitude(unit: UnitOfMeasure) -> EllipsoidalCS {
    EllipsoidalCS {
        identified: Identified::default(),
        axes: vec![
            Axis::new("Latitude", "lat", "north", unit.clone()),
            Axis::new("Longitude", "lon", "east", unit),
        ],
    }
}

fn longitude_latitude(unit: UnitOfMeasure) -> EllipsoidalCS {
    EllipsoidalCS {
        identified: Identified::default(),
        axes: vec![
            Axis::new("Longitude", "lon", "east", unit.clone()),
            Axis::new("Latitude", "lat", "north", unit),
        ],
    }
}

fn longitude_latitude_height(angular: UnitOfMeasure, linear: UnitOfMeasure) -> EllipsoidalCS {
    EllipsoidalCS {
        identified: Identified::default(),
        axes: vec![
            Axis::new("Longitude", "lon", "east", angular.clone()),
            Axis::new("Latitude", "lat", "north", angular),
            Axis::new("Ellipsoidal height", "h", "up", linear),
        ],
    }
}

fn geocentric_cs(unit: UnitOfMeasure) -> CartesianCS {
    CartesianCS {
        identified: Identified::default(),
        axes: vec![
            Axis::new("Geocentric X", "X", "geocentricX", unit.clone()),
            Axis::new("Geocentric Y", "Y", "geocentricY", unit.clone()),
            Axis::new("Geocentric Z", "Z", "geocentricZ", unit),
        ],
    }
}

fn easting_northing(unit: UnitOfMeasure) -> CartesianCS {
    CartesianCS {
        identified: Identified::default(),
        axes: vec![
            Axis::new("Easting", "E", "east", unit.clone()),
            Axis::new("Northing", "N", "north", unit),
        ],
    }
}

fn guess_parameter_unit(
    name: &str,
    linear: &UnitOfMeasure,
    angular: &UnitOfMeasure,
) -> Option<UnitOfMeasure> {
    if ci_contains(name, "scale") || ci_contains(name, "scaling factor") {
        return Some(UnitOfMeasure {
            name: "unity".to_owned(),
            factor: 1.0,
            kind: UnitKind::Scale,
            code_space: String::new(),
            code: String::new(),
        });
    }
    if ci_contains(name, "rotation") {
        return Some(UnitOfMeasure::arc_second());
    }
    if ci_contains(name, "latitude")
        || ci_contains(name, "longitude")
        || ci_contains(name, "meridian")
        || ci_contains(name, "parallel")
        || ci_contains(name, "azimuth")
        || ci_contains(name, "angle")
        || ci_contains(name, "heading")
    {
        return Some(angular.clone());
    }
    if ci_contains(name, "easting") || ci_contains(name, "northing") || ci_contains(name, "height")
    {
        return Some(linear.clone());
    }
    None
}

fn parameter(name: &str, code: &str, value: f64, unit: &UnitOfMeasure) -> Parameter {
    Parameter {
        name: name.to_owned(),
        code_space: "EPSG".to_owned(),
        code: code.to_owned(),
        value: ParameterValue::Measure(Measure {
            value,
            unit: unit.clone(),
        }),
    }
}

fn helmert_parameters(values: &[f64]) -> Vec<Parameter> {
    let names = [
        ("X-axis translation", "8605", UnitKind::Linear),
        ("Y-axis translation", "8606", UnitKind::Linear),
        ("Z-axis translation", "8607", UnitKind::Linear),
        ("X-axis rotation", "8608", UnitKind::Angular),
        ("Y-axis rotation", "8609", UnitKind::Angular),
        ("Z-axis rotation", "8610", UnitKind::Angular),
        ("Scale difference", "8611", UnitKind::Scale),
    ];
    values
        .iter()
        .zip(names)
        .map(|(value, (name, code, kind))| {
            let unit = match kind {
                UnitKind::Linear => UnitOfMeasure::metre(),
                UnitKind::Angular => UnitOfMeasure {
                    name: "arc-second".to_owned(),
                    factor: DEGREE_SI / 3600.0,
                    kind: UnitKind::Angular,
                    code_space: "EPSG".to_owned(),
                    code: "9104".to_owned(),
                },
                _ => UnitOfMeasure {
                    name: "parts per million".to_owned(),
                    factor: 1e-6,
                    kind: UnitKind::Scale,
                    code_space: "EPSG".to_owned(),
                    code: "9202".to_owned(),
                },
            };
            parameter(name, code, *value, &unit)
        })
        .collect()
}

fn wkt1_method(name: &str) -> (&'static str, &'static str) {
    if ci_eq(name, "Transverse_Mercator") {
        ("Transverse Mercator", "9807")
    } else if ci_eq(name, "Transverse_Mercator_South_Orientated") {
        ("Transverse Mercator (South Orientated)", "9808")
    } else if ci_eq(name, "Lambert_Conformal_Conic_1SP") {
        ("Lambert Conic Conformal (1SP)", "9801")
    } else if ci_eq(name, "Lambert_Conformal_Conic_2SP") {
        ("Lambert Conic Conformal (2SP)", "9802")
    } else if ci_eq(name, "Albers_Conic_Equal_Area") {
        ("Albers Equal Area", "9822")
    } else if ci_eq(name, "Mercator_1SP") {
        ("Mercator (variant A)", "9804")
    } else if ci_eq(name, "Mercator_2SP") {
        ("Mercator (variant B)", "9805")
    } else if ci_eq(name, "Polar_Stereographic") {
        ("Polar Stereographic (variant A)", "9810")
    } else if ci_eq(name, "Oblique_Stereographic") {
        ("Oblique Stereographic", "9809")
    } else if ci_eq(name, "Hotine_Oblique_Mercator_Azimuth_Natural_Origin") {
        ("Hotine Oblique Mercator (variant A)", "9812")
    } else if ci_eq(name, "Hotine_Oblique_Mercator_Azimuth_Center") {
        ("Hotine Oblique Mercator (variant B)", "9815")
    } else if ci_eq(name, "Krovak") {
        ("Krovak", "9819")
    } else if ci_eq(name, "Cassini_Soldner") {
        ("Cassini-Soldner", "9806")
    } else if ci_eq(name, "Equidistant_Cylindrical") {
        ("Equidistant Cylindrical", "1028")
    } else {
        ("", "")
    }
}

fn wkt1_parameter(name: &str) -> Option<(&'static str, &'static str, UnitKind)> {
    let name = name.trim_matches('"');
    if ci_eq(name, "latitude_of_origin") || ci_eq(name, "latitude_of_center") {
        Some(("Latitude of natural origin", "8801", UnitKind::Angular))
    } else if ci_eq(name, "central_meridian") || ci_eq(name, "longitude_of_center") {
        Some(("Longitude of natural origin", "8802", UnitKind::Angular))
    } else if ci_eq(name, "scale_factor") {
        Some(("Scale factor at natural origin", "8805", UnitKind::Scale))
    } else if ci_eq(name, "false_easting") {
        Some(("False easting", "8806", UnitKind::Linear))
    } else if ci_eq(name, "false_northing") {
        Some(("False northing", "8807", UnitKind::Linear))
    } else if ci_eq(name, "standard_parallel_1") {
        Some((
            "Latitude of 1st standard parallel",
            "8823",
            UnitKind::Angular,
        ))
    } else if ci_eq(name, "standard_parallel_2") {
        Some((
            "Latitude of 2nd standard parallel",
            "8824",
            UnitKind::Angular,
        ))
    } else if ci_eq(name, "azimuth") {
        Some(("Azimuth of initial line", "8813", UnitKind::Angular))
    } else if ci_eq(name, "rectified_grid_angle") {
        Some((
            "Angle from Rectified to Skew Grid",
            "8814",
            UnitKind::Angular,
        ))
    } else {
        None
    }
}

fn guess_dialect(input: &str) -> WktDialect {
    let wkt = input.trim_start_matches([' ', '\t', '\r', '\n']);
    if ci_starts(wkt, "VERTCS") {
        return WktDialect::Wkt1Esri;
    }
    const WKT1: &[&str] = &[
        "GEOCCS", "GEOGCS", "COMPD_CS", "PROJCS", "VERT_CS", "LOCAL_CS",
    ];
    for keyword in WKT1 {
        if ci_starts(wkt, keyword) {
            let lower = wkt.to_ascii_lowercase();
            let gcs = lower.contains("geogcs[\"gcs_");
            let no_axis = !ci_starts(wkt, "LOCAL_CS")
                && !lower.contains("axis[")
                && !lower.contains("authority[");
            let rectified = lower.contains("parameter[\"rectified_grid_angle");
            if (gcs || no_axis) && !rectified {
                return WktDialect::Wkt1Esri;
            }
            return WktDialect::Wkt1Gdal;
        }
    }
    const WKT2019: &[&str] = &[
        "GEOGCRS",
        "CONCATENATEDOPERATION",
        "USAGE",
        "DYNAMIC",
        "FRAMEEPOCH",
        "MODEL",
        "VELOCITYGRID",
        "ENSEMBLE",
        "DERIVEDPROJCRS",
        "BASEPROJCRS",
        "GEOGRAPHICCRS",
        "TRF",
        "VRF",
        "POINTMOTIONOPERATION",
    ];
    let lower = wkt.to_ascii_lowercase();
    for keyword in WKT2019 {
        let needle = keyword.to_ascii_lowercase();
        if let Some(pos) = lower.find(&needle) {
            let after = pos + needle.len();
            if wkt.as_bytes().get(after) == Some(&b'[') {
                return WktDialect::Wkt2_2019;
            }
        }
    }
    for needle in [
        "cs[temporaldatetime,",
        "cs[temporalcount,",
        "cs[temporalmeasure,",
    ] {
        if lower.contains(needle) {
            return WktDialect::Wkt2_2019;
        }
    }
    const WKT2: &[&str] = &[
        "GEODCRS",
        "GEODETICCRS",
        "PROJCRS",
        "PROJECTEDCRS",
        "VERTCRS",
        "VERTICALCRS",
        "COMPOUNDCRS",
        "BOUNDCRS",
        "ENGCRS",
        "ENGINEERINGCRS",
        "TIMECRS",
        "PARAMETRICCRS",
        "COORDINATEOPERATION",
        "CONVERSION",
        "DATUM",
        "ELLIPSOID",
        "BASEGEODCRS",
        "BASEGEOGCRS",
    ];
    for keyword in WKT2 {
        if ci_starts(wkt, keyword) {
            let rest = &wkt[keyword.len()..];
            let rest = rest.trim_start_matches([' ', '\t', '\r', '\n']);
            if rest.starts_with('[') {
                return WktDialect::Wkt2_2015;
            }
        }
    }
    WktDialect::NotWkt
}

trait BoundEdit {
    fn source_crs_mut(&mut self) -> &mut Crs;
    fn identified_mut(&mut self) -> &mut Identified;
}

impl BoundEdit for BoundCRS {
    fn source_crs_mut(&mut self) -> &mut Crs {
        &mut self.source
    }

    fn identified_mut(&mut self) -> &mut Identified {
        &mut self.identified
    }
}

impl AuthorityFactory {
    /// Projected CRS, including an ESRI row stored only as `text_definition`.
    ///
    /// # Errors
    ///
    /// The code is missing, or the stored WKT is not a projected CRS.
    pub fn create_projected_crs(&self, code: &str) -> Result<ProjectedCRS, FactoryError> {
        match self.create_projected_crs_from_columns(code) {
            Ok(crs) => Ok(crs),
            Err(error) => self.projected_from_text(code).or(Err(error)),
        }
    }

    fn projected_from_text(&self, code: &str) -> Result<ProjectedCRS, FactoryError> {
        let Some(text) =
            self.database_context()
                .text_definition("projected_crs", self.authority(), code)
        else {
            return Err(FactoryError::missing(
                "projected CRS",
                self.authority(),
                code,
            ));
        };
        if !text.starts_with("PROJCS") {
            return Err(FactoryError::wrap(
                "projected CRS",
                self.authority(),
                code,
                "text_definition is not expanded",
            ));
        }
        let mut parser = WktParser::new();
        parser.attach_database(self.database_context().clone());
        let object = parser.create_from_wkt(&text)?;
        let Some(Crs::Projected(crs)) = object.crs().cloned() else {
            return Err(FactoryError::wrap(
                "projected CRS",
                self.authority(),
                code,
                "text_definition is not a projected CRS",
            ));
        };
        let already = crs.identifiers().iter().any(|identifier| {
            identifier.code_space() == self.authority() && identifier.code() == code
        });
        if already {
            Ok(crs)
        } else {
            Ok(crs.with_identifier(self.authority(), code))
        }
    }
}
