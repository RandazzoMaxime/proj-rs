//! PROJJSON export and the `{` branch of `createFromUserInput`.
//!
//! Identifier code spaces such as `IAU_2015` are split into an authority and a
//! version on the way out, and joined again on the way in, using
//! `versioned_auth_name_mapping`.

use super::equiv;
use super::factory::{AuthorityFactory, AuthorityObject, DatabaseContext, NameMatch, ObjectType};
use super::model::{
    Axis, BoundCRS, CartesianCS, CompoundCRS, ConcatenatedOperation, Conversion,
    CoordinateOperation, Crs, Datum, DatumEnsemble, Ellipsoid, EllipsoidalCS, FactoryError,
    GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, GeoidModel, Identified,
    Identifier, Measure, ObjectDomain, OperationMethod, Parameter, ParameterValue, PrimeMeridian,
    ProjectedCRS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS, VerticalReferenceFrame,
};
use super::parammap::{method_by_code, method_by_name};
use super::projstring::create_crs_from_proj_string_with_database;
use super::wkt_import::WktParser;
use crate::crs_text::{self, Object, Value};

const SCHEMA: &str = "https://proj.org/schemas/v0.7/projjson.schema.json";

/// Line breaks in PROJJSON text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonLines {
    /// One member per line, indented.
    Multi,
    /// A single line: no newline, no space after a colon.
    Single,
}

/// Layout of PROJJSON text. The default is PROJ's: multi-line, two spaces per level, and
/// the PROJJSON v0.7 `$schema`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonFormat {
    lines: JsonLines,
    indent_width: usize,
    /// Value of the top-level `$schema` member. Empty writes no member.
    schema: String,
}

impl Default for JsonFormat {
    fn default() -> Self {
        Self {
            lines: JsonLines::Multi,
            indent_width: 2,
            schema: SCHEMA.to_owned(),
        }
    }
}

impl JsonFormat {
    /// The default format with `lines` as its line breaks.
    #[must_use]
    pub fn with_lines(lines: JsonLines) -> Self {
        Self {
            lines,
            ..Self::default()
        }
    }

    /// Spaces per indentation level.
    #[must_use]
    pub fn with_indent_width(mut self, width: usize) -> Self {
        self.indent_width = width;
        self
    }

    /// Value of the top-level `$schema` member. An empty string writes none.
    #[must_use]
    pub fn with_schema(mut self, schema: &str) -> Self {
        schema.clone_into(&mut self.schema);
        self
    }
}

struct Frame {
    has_child: bool,
    expect_value: bool,
}

struct Writer {
    out: String,
    indent: usize,
    indent_width: usize,
    pretty: bool,
    stack: Vec<Frame>,
}

impl Writer {
    fn new(format: &JsonFormat) -> Self {
        Self {
            out: String::new(),
            indent: 0,
            indent_width: format.indent_width,
            pretty: format.lines == JsonLines::Multi,
            stack: Vec::new(),
        }
    }

    fn start_object(&mut self) {
        self.before_value();
        self.out.push('{');
        self.indent += 1;
        self.stack.push(Frame {
            has_child: false,
            expect_value: false,
        });
    }

    fn end_object(&mut self) {
        self.end_container();
        self.out.push('}');
    }

    fn start_array(&mut self) {
        self.before_value();
        self.out.push('[');
        self.indent += 1;
        self.stack.push(Frame {
            has_child: false,
            expect_value: false,
        });
    }

    fn end_array(&mut self) {
        self.end_container();
        self.out.push(']');
    }

    fn key(&mut self, name: &str) {
        self.separator();
        self.string_raw(name);
        self.out.push_str(if self.pretty { ": " } else { ":" });
        if let Some(frame) = self.stack.last_mut() {
            frame.expect_value = true;
        }
    }

    fn string(&mut self, text: &str) {
        self.before_value();
        self.string_raw(text);
    }

    fn number(&mut self, value: f64) {
        self.before_value();
        self.out.push_str(&format_json_number(value));
    }

    fn raw_token(&mut self, text: &str) {
        self.before_value();
        self.out.push_str(text);
    }

    fn finish(self) -> String {
        self.out
    }

    fn before_value(&mut self) {
        if self.stack.last().is_some_and(|frame| frame.expect_value) {
            if let Some(frame) = self.stack.last_mut() {
                frame.expect_value = false;
            }
            return;
        }
        if !self.stack.is_empty() {
            self.separator();
        }
    }

    fn separator(&mut self) {
        let needs_comma = self.stack.last().is_some_and(|frame| frame.has_child);
        if let Some(frame) = self.stack.last_mut() {
            frame.has_child = true;
            frame.expect_value = false;
        }
        if needs_comma {
            self.out.push(',');
        }
        self.newline();
    }

    fn end_container(&mut self) {
        let has_child = self.stack.pop().is_some_and(|frame| frame.has_child);
        self.indent = self.indent.saturating_sub(1);
        if has_child {
            self.newline();
        }
    }

    fn newline(&mut self) {
        if !self.pretty {
            return;
        }
        self.out.push('\n');
        self.out
            .extend(std::iter::repeat_n(' ', self.indent * self.indent_width));
    }

    fn string_raw(&mut self, text: &str) {
        self.out.push('"');
        for ch in text.chars() {
            match ch {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                other => self.out.push(other),
            }
        }
        self.out.push('"');
    }
}

fn format_json_number(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        super::wkt::format_g(value, 15)
    }
}

impl Crs {
    /// PROJJSON for this CRS, in PROJ's default layout.
    ///
    /// `proj.db` splits a versioned authority such as `IAU_2015` into
    /// `"authority": "IAU"` and `"version": 2015`.
    ///
    /// # Errors
    ///
    /// The CRS kind, or a part of it, has no PROJJSON export in this build.
    pub fn export_to_json(&self, db: &DatabaseContext) -> Result<String, FactoryError> {
        self.export_to_json_with(db, &JsonFormat::default())
    }

    /// PROJJSON for this CRS in `format`.
    ///
    /// # Errors
    ///
    /// As [`Self::export_to_json`].
    pub fn export_to_json_with(
        &self,
        db: &DatabaseContext,
        format: &JsonFormat,
    ) -> Result<String, FactoryError> {
        let mut writer = Writer::new(format);
        writer.start_object();
        if !format.schema.is_empty() {
            writer.key("$schema");
            writer.string(&format.schema);
        }
        write_crs(&mut writer, self, db, Place::top(crs_ident(self)))?;
        writer.end_object();
        Ok(writer.finish())
    }
}

impl CoordinateOperation {
    /// PROJJSON for this operation in `format`: a `Conversion`, a `Transformation`, or a
    /// `ConcatenatedOperation`, as `proj_as_projjson` writes it.
    ///
    /// A concatenated operation or a transformation names its CRS pair. A conversion does not,
    /// as PROJ writes none. A concatenated operation that stores no pair takes the source of
    /// its first step and the target of its last.
    ///
    /// # Errors
    ///
    /// A CRS or a step has no PROJJSON export here, or a concatenated operation has no CRS pair.
    pub fn export_to_json_with(
        &self,
        db: &DatabaseContext,
        format: &JsonFormat,
    ) -> Result<String, FactoryError> {
        let mut writer = Writer::new(format);
        writer.start_object();
        if !format.schema.is_empty() {
            writer.key("$schema");
            writer.string(&format.schema);
        }
        write_operation(&mut writer, self, db, Place::top(operation_ident(self)))?;
        writer.end_object();
        Ok(writer.finish())
    }
}

/// Whether an object has identifiers of its own.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ident {
    Has,
    Lacks,
}

/// Whether a parent asks for its child's `id`, as PROJ's `setAllowIDInImmediateChild` does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Allow {
    Asked,
    Plain,
}

/// Whether an object, or one of its ancestors, has an `id`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Chain {
    Clear,
    Has,
}

/// Where an object sits, as PROJ's `JSONFormatter` tracks it.
///
/// PROJ writes an object's `id` when it is the top object, or when no object above it has an
/// `id` (`outputId`). A parent that asks for its child's `id` gets one anyway. The usage block
/// (scope, area, extent) is written for the top object only.
#[derive(Clone, Copy)]
struct Place {
    /// The top object of the export.
    top: bool,
    /// Whether this object writes its own `id`.
    id_out: bool,
    /// Whether this object or one of its ancestors has an `id`.
    chain: Chain,
}

impl Place {
    /// The top object, which always writes its `id`.
    fn top(ident: Ident) -> Self {
        Self {
            top: true,
            id_out: true,
            chain: chain_of(Chain::Clear, ident),
        }
    }

    /// An object with `ident`, inside a parent whose chain is `parent`.
    fn inside(parent: Chain, ident: Ident, allow: Allow) -> Self {
        Self {
            top: false,
            id_out: allow == Allow::Asked || parent == Chain::Clear,
            chain: chain_of(parent, ident),
        }
    }
}

fn ident(identifiers: &[Identifier]) -> Ident {
    if identifiers.is_empty() {
        Ident::Lacks
    } else {
        Ident::Has
    }
}

/// `Ident` of a parameter or a unit, from its code space.
fn code_ident(code_space: &str) -> Ident {
    if code_space.is_empty() {
        Ident::Lacks
    } else {
        Ident::Has
    }
}

fn chain_of(parent: Chain, ident: Ident) -> Chain {
    if parent == Chain::Has || ident == Ident::Has {
        Chain::Has
    } else {
        Chain::Clear
    }
}

fn crs_ident(crs: &Crs) -> Ident {
    match crs {
        Crs::Geographic(crs) | Crs::Geodetic(GeodeticCRS::Geographic(crs)) => {
            ident(crs.identifiers())
        }
        Crs::Geodetic(GeodeticCRS::Geocentric(crs)) => ident(crs.identifiers()),
        Crs::Projected(crs) => ident(crs.identifiers()),
        Crs::Vertical(crs) => ident(crs.identifiers()),
        Crs::Compound(crs) => ident(crs.identifiers()),
        Crs::Bound(crs) => ident(&crs.identified.identifiers),
        _ => Ident::Lacks,
    }
}

fn write_crs(
    writer: &mut Writer,
    crs: &Crs,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    match crs {
        Crs::Geographic(crs) | Crs::Geodetic(GeodeticCRS::Geographic(crs)) => {
            write_geographic(writer, crs, db, place)
        }
        Crs::Geodetic(GeodeticCRS::Geocentric(crs)) => write_geocentric(writer, crs, db, place),
        Crs::Projected(crs) => write_projected(writer, crs, db, place),
        Crs::Vertical(crs) => write_vertical(writer, crs, db, place),
        Crs::Compound(crs) => write_compound(writer, crs, db, place),
        Crs::Bound(crs) => write_bound(writer, crs, db, place),
        other => Err(FactoryError::Format(format!(
            "PROJJSON export is not built for {}",
            other.name()
        ))),
    }
}

/// A CRS object inside the parent chain `parent`, with its own `type`. `allow` is `Allow::Asked`
/// for a base CRS, whose `id` PROJ always writes.
fn write_component(
    writer: &mut Writer,
    crs: &Crs,
    db: &DatabaseContext,
    parent: Chain,
    allow: Allow,
) -> Result<(), FactoryError> {
    let place = Place::inside(parent, crs_ident(crs), allow);
    writer.start_object();
    write_crs(writer, crs, db, place)?;
    writer.end_object();
    Ok(())
}

fn write_name(writer: &mut Writer, name: &str) {
    writer.key("name");
    if name.is_empty() {
        writer.string("unnamed");
    } else {
        writer.string(name);
    }
}

fn write_geographic(
    writer: &mut Writer,
    crs: &GeographicCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("GeographicCRS");
    write_name(writer, crs.name());
    write_datum_or_ensemble(writer, crs.datum(), crs.datum_ensemble(), db, place.chain)?;
    writer.key("coordinate_system");
    write_cs(
        writer,
        "ellipsoidal",
        crs.coordinate_system().axes(),
        place.chain,
    )?;
    write_usage(writer, crs.domains(), place);
    write_id_if(writer, place, crs.identifiers(), db);
    write_remarks(writer, &crs.identified.remarks);
    Ok(())
}

fn write_geocentric(
    writer: &mut Writer,
    crs: &GeocentricCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("GeodeticCRS");
    write_name(writer, crs.name());
    write_datum_or_ensemble(writer, crs.datum(), crs.ensemble.as_ref(), db, place.chain)?;
    writer.key("coordinate_system");
    write_cs(
        writer,
        "Cartesian",
        crs.coordinate_system().axes(),
        place.chain,
    )?;
    write_usage(writer, crs.domains(), place);
    write_id_if(writer, place, crs.identifiers(), db);
    write_remarks(writer, &crs.identified.remarks);
    Ok(())
}

fn write_projected(
    writer: &mut Writer,
    crs: &ProjectedCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("ProjectedCRS");
    write_name(writer, crs.name());
    writer.key("base_crs");
    writer.start_object();
    // The base CRS always writes its `id`, as PROJ's `setAllowIDInImmediateChild` asks.
    match crs.base_crs() {
        GeodeticCRS::Geographic(base) => {
            let base_place = Place::inside(place.chain, ident(base.identifiers()), Allow::Asked);
            write_geographic(writer, base, db, base_place)?;
        }
        GeodeticCRS::Geocentric(base) => {
            let base_place = Place::inside(place.chain, ident(base.identifiers()), Allow::Asked);
            write_geocentric(writer, base, db, base_place)?;
        }
        GeodeticCRS::Spherical(_) => {
            return Err(FactoryError::Format(
                "PROJJSON export of a spherical base CRS is not built".to_owned(),
            ));
        }
    }
    writer.end_object();
    writer.key("conversion");
    let conversion = crs.deriving_conversion();
    let conversion_place =
        Place::inside(place.chain, ident(conversion.identifiers()), Allow::Plain);
    write_conversion(writer, conversion, db, conversion_place)?;
    writer.key("coordinate_system");
    write_cs(
        writer,
        "Cartesian",
        crs.coordinate_system().axes(),
        place.chain,
    )?;
    write_usage(writer, crs.domains(), place);
    write_id_if(writer, place, crs.identifiers(), db);
    write_remarks(writer, &crs.identified.remarks);
    Ok(())
}

fn write_vertical(
    writer: &mut Writer,
    crs: &VerticalCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("VerticalCRS");
    write_name(writer, crs.name());
    if let Some(datum) = crs.datum() {
        writer.key("datum");
        let datum_place = Place::inside(place.chain, ident(datum.identifiers()), Allow::Plain);
        write_vertical_datum(writer, datum, db, datum_place)?;
    } else if let Some(ensemble) = crs.datum_ensemble() {
        writer.key("datum_ensemble");
        write_ensemble(writer, ensemble, db, place.chain)?;
    }
    writer.key("coordinate_system");
    write_cs(
        writer,
        "vertical",
        std::slice::from_ref(crs.coordinate_system().axis()),
        place.chain,
    )?;
    write_geoid_models(writer, crs.geoid_models(), db);
    write_usage(writer, crs.domains(), place);
    write_id_if(writer, place, crs.identifiers(), db);
    write_remarks(writer, &crs.identified.remarks);
    Ok(())
}

fn write_compound(
    writer: &mut Writer,
    crs: &CompoundCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("CompoundCRS");
    write_name(writer, crs.name());
    writer.key("components");
    writer.start_array();
    for component in crs.components() {
        write_component(writer, component, db, place.chain, Allow::Plain)?;
    }
    writer.end_array();
    write_usage(writer, crs.domains(), place);
    write_id_if(writer, place, crs.identifiers(), db);
    write_remarks(writer, &crs.identified.remarks);
    Ok(())
}

/// A bound CRS: its name when it differs from the source's, the source and target CRS, the
/// transformation in its abridged form, and its usage and `id`.
fn write_bound(
    writer: &mut Writer,
    crs: &BoundCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    let source = crs.source_crs();
    let CoordinateOperation::Transformation(transformation) = crs.transformation() else {
        return Err(FactoryError::Format(
            "PROJJSON export of a bound CRS with a conversion or a concatenated operation is not built"
                .to_owned(),
        ));
    };
    writer.key("type");
    writer.string("BoundCRS");
    if !crs.name().is_empty() && crs.name() != source.name() {
        writer.key("name");
        writer.string(crs.name());
    }
    writer.key("source_crs");
    write_component(writer, source, db, place.chain, Allow::Plain)?;
    writer.key("target_crs");
    write_component(writer, crs.target_crs(), db, place.chain, Allow::Plain)?;
    writer.key("transformation");
    let transformation_place = Place::inside(
        place.chain,
        ident(&transformation.identified.identifiers),
        Allow::Plain,
    );
    let source_to_write = bound_source_to_write(crs, transformation);
    write_abridged_transformation(
        writer,
        transformation,
        db,
        transformation_place,
        source_to_write,
    )?;
    write_usage(writer, &crs.identified.domains, place);
    write_id_if(writer, place, &crs.identified.identifiers, db);
    write_remarks(writer, &crs.identified.remarks);
    Ok(())
}

/// The source CRS of a bound CRS's transformation, when it is written. PROJ writes it when that
/// source is not equivalent to the bound CRS's source and, for a projected source, when the
/// target is geographic and the projected CRS's base is not equivalent either. A transformation
/// read from WKT has no source of its own: PROJ takes the bound CRS's source.
fn bound_source_to_write<'a>(
    crs: &'a BoundCRS,
    transformation: &'a Transformation,
) -> Option<&'a Crs> {
    let source = crs.source_crs();
    let operation_source = transformation.source_crs().unwrap_or(source);
    if source.is_equivalent_to(operation_source) {
        return None;
    }
    let write = match source {
        Crs::Projected(projected) => {
            let target_is_geographic = matches!(
                crs.target_crs(),
                Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_))
            );
            let base = Crs::Geodetic(projected.base_crs().clone());
            target_is_geographic && !base.is_equivalent_to(operation_source)
        }
        _ => true,
    };
    write.then_some(operation_source)
}

/// The transformation of a bound CRS: name, the source CRS when it is to be written, method,
/// parameter values, and `id`. PROJ writes it without `type`, and with no accuracy or usage.
fn write_abridged_transformation(
    writer: &mut Writer,
    transformation: &Transformation,
    db: &DatabaseContext,
    place: Place,
    source_to_write: Option<&Crs>,
) -> Result<(), FactoryError> {
    writer.start_object();
    write_name(writer, &transformation.identified.name);
    if let Some(source) = source_to_write {
        writer.key("source_crs");
        write_component(writer, source, db, place.chain, Allow::Asked)?;
    }
    writer.key("method");
    write_method(writer, transformation.method());
    writer.key("parameters");
    writer.start_array();
    for parameter in transformation.parameters() {
        write_parameter(writer, parameter, place.chain);
    }
    writer.end_array();
    write_id_if(writer, place, &transformation.identified.identifiers, db);
    writer.end_object();
    Ok(())
}

/// The `datum` or `datum_ensemble` of a geodetic CRS. The ensemble has no `type` in the CRS.
fn write_datum_or_ensemble(
    writer: &mut Writer,
    datum: Option<&GeodeticReferenceFrame>,
    ensemble: Option<&DatumEnsemble>,
    db: &DatabaseContext,
    parent: Chain,
) -> Result<(), FactoryError> {
    if let Some(datum) = datum {
        writer.key("datum");
        let datum_place = Place::inside(parent, ident(datum.identifiers()), Allow::Plain);
        write_datum(writer, datum, db, datum_place)?;
    } else if let Some(ensemble) = ensemble {
        writer.key("datum_ensemble");
        write_ensemble(writer, ensemble, db, parent)?;
    }
    Ok(())
}

fn write_datum(
    writer: &mut Writer,
    datum: &GeodeticReferenceFrame,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.start_object();
    let dynamic = datum.frame_reference_epoch().is_some();
    writer.key("type");
    writer.string(if dynamic {
        "DynamicGeodeticReferenceFrame"
    } else {
        "GeodeticReferenceFrame"
    });
    write_name(writer, datum.name());
    write_anchor(
        writer,
        datum.anchor(),
        datum.anchor_epoch(),
        datum.deformation_model(),
    )?;
    if let Some(epoch) = datum.frame_reference_epoch() {
        writer.key("frame_reference_epoch");
        writer.number(epoch);
    }
    writer.key("ellipsoid");
    let ellipsoid = datum.ellipsoid();
    let ellipsoid_place = Place::inside(place.chain, ident(ellipsoid.identifiers()), Allow::Plain);
    write_ellipsoid(writer, ellipsoid, db, ellipsoid_place);
    let meridian = datum.prime_meridian();
    if meridian.name() != "Greenwich" {
        writer.key("prime_meridian");
        let meridian_place =
            Place::inside(place.chain, ident(meridian.identifiers()), Allow::Plain);
        write_meridian(writer, meridian, db, meridian_place);
    }
    write_id_if(writer, place, datum.identifiers(), db);
    writer.end_object();
    Ok(())
}

fn write_vertical_datum(
    writer: &mut Writer,
    datum: &VerticalReferenceFrame,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.start_object();
    let dynamic = datum.frame_reference_epoch().is_some();
    writer.key("type");
    writer.string(if dynamic {
        "DynamicVerticalReferenceFrame"
    } else {
        "VerticalReferenceFrame"
    });
    write_name(writer, datum.name());
    write_anchor(
        writer,
        datum.anchor(),
        datum.anchor_epoch(),
        datum.deformation_model(),
    )?;
    if let Some(epoch) = datum.frame_reference_epoch() {
        writer.key("frame_reference_epoch");
        writer.number(epoch);
    }
    write_id_if(writer, place, datum.identifiers(), db);
    writer.end_object();
    Ok(())
}

/// `anchor` then `anchor_epoch`, as `GeodeticReferenceFrame::_exportToJSON` writes them.
///
/// A deformation model is WKT `MODEL[]` only. The PROJJSON writer does not emit it, so it is
/// omitted here as well. The epoch is rounded the way `getRoundedEpochInDecimalYear` does: when
/// the year is within 1e-3 of a milliyear, it is that milliyear.
fn write_anchor(
    writer: &mut Writer,
    anchor: Option<&str>,
    anchor_epoch: Option<f64>,
    _deformation_model: Option<&str>,
) -> Result<(), FactoryError> {
    if let Some(anchor) = anchor {
        writer.key("anchor");
        writer.string(anchor);
    }
    if let Some(epoch) = anchor_epoch {
        writer.key("anchor_epoch");
        writer.number(rounded_anchor_epoch(epoch));
    }
    Ok(())
}

/// `getRoundedEpochInDecimalYear`: snap to 0.001 year when the residual is at most 1e-3.
fn rounded_anchor_epoch(year: f64) -> f64 {
    let scaled = year * 1000.0;
    let nearest = scaled.round();
    if (scaled - nearest).abs() <= 1e-3 {
        nearest / 1000.0
    } else {
        year
    }
}

/// A datum ensemble. PROJ writes its `id` whatever the parent. The ellipsoid of its first member
/// follows the usual `id` rule, with the ensemble above it.
fn write_ensemble(
    writer: &mut Writer,
    ensemble: &DatumEnsemble,
    db: &DatabaseContext,
    parent: Chain,
) -> Result<(), FactoryError> {
    let chain = chain_of(parent, ident(ensemble.identifiers()));
    writer.start_object();
    write_name(writer, ensemble.name());
    writer.key("members");
    writer.start_array();
    for member in ensemble.members() {
        writer.start_object();
        let (name, identifiers) = match member {
            Datum::Geodetic(frame) => (frame.name(), frame.identifiers()),
            Datum::Vertical(frame) => (frame.name(), frame.identifiers()),
            _ => {
                return Err(FactoryError::Format(
                    "PROJJSON export of this datum ensemble member is not built".to_owned(),
                ));
            }
        };
        write_name(writer, name);
        write_id(writer, identifiers, db);
        writer.end_object();
    }
    writer.end_array();
    if let Some(Datum::Geodetic(first)) = ensemble.members().first() {
        writer.key("ellipsoid");
        let ellipsoid = first.ellipsoid();
        let ellipsoid_place = Place::inside(chain, ident(ellipsoid.identifiers()), Allow::Plain);
        write_ellipsoid(writer, ellipsoid, db, ellipsoid_place);
    }
    writer.key("accuracy");
    // The accuracy is the text stored in proj.db, so a whole value reads "2.0".
    match ensemble.accuracy_text.as_deref() {
        Some(text) => writer.string(text),
        None => writer.string(&super::wkt::format_g(ensemble.positional_accuracy(), 15)),
    }
    write_id(writer, ensemble.identifiers(), db);
    writer.end_object();
    Ok(())
}

/// `geoid_model`, or `geoid_models` for several. A model's `id` is always written.
fn write_geoid_models(writer: &mut Writer, models: &[GeoidModel], db: &DatabaseContext) {
    match models {
        [] => {}
        [model] => {
            writer.key("geoid_model");
            write_geoid_model(writer, model, db);
        }
        many => {
            writer.key("geoid_models");
            writer.start_array();
            for model in many {
                write_geoid_model(writer, model, db);
            }
            writer.end_array();
        }
    }
}

fn write_geoid_model(writer: &mut Writer, model: &GeoidModel, db: &DatabaseContext) {
    writer.start_object();
    writer.key("name");
    writer.string(model.name());
    write_id(writer, model.identifiers(), db);
    writer.end_object();
}

/// An ellipsoid as PROJ writes it: `radius` for a sphere, else `semi_major_axis` and either the
/// stored `inverse_flattening` or `semi_minor_axis`. An axis that is not in metres is an object
/// with its `value` and `unit`.
fn write_ellipsoid(writer: &mut Writer, ellipsoid: &Ellipsoid, db: &DatabaseContext, place: Place) {
    writer.start_object();
    write_name(writer, ellipsoid.name());
    let sphere = ellipsoid.is_sphere();
    let major_key = if sphere { "radius" } else { "semi_major_axis" };
    write_length(writer, major_key, ellipsoid.semi_major_axis(), place);
    if !sphere {
        if let Some(inverse_flattening) = ellipsoid.declared_inverse_flattening() {
            writer.key("inverse_flattening");
            writer.number(inverse_flattening);
        } else if let Some(minor) = ellipsoid.semi_minor_axis() {
            write_length(writer, "semi_minor_axis", minor, place);
        }
    }
    write_id_if(writer, place, ellipsoid.identifiers(), db);
    writer.end_object();
}

/// A length under `key`: a bare number in metres, else `{value, unit}`.
fn write_length(writer: &mut Writer, key: &str, length: &Measure, place: Place) {
    writer.key(key);
    let unit = length.unit();
    if unit.name() == "metre" {
        writer.number(length.value());
    } else {
        writer.start_object();
        writer.key("value");
        writer.number(length.value());
        writer.key("unit");
        let unit_place = Place::inside(place.chain, Ident::Lacks, Allow::Plain);
        write_unit_object(writer, unit, unit_place);
        writer.end_object();
    }
}

fn write_meridian(
    writer: &mut Writer,
    meridian: &PrimeMeridian,
    db: &DatabaseContext,
    place: Place,
) {
    writer.start_object();
    writer.key("name");
    writer.string(meridian.name());
    writer.key("longitude");
    writer.number(meridian.longitude().convert_to(&UnitOfMeasure::degree()));
    write_id_if(writer, place, meridian.identifiers(), db);
    writer.end_object();
}

/// `coordinate_system`: its subtype and its axes. The coordinate system's own `id` is not
/// written: none of the CRS built here has one in its CS.
fn write_cs(
    writer: &mut Writer,
    subtype: &str,
    axes: &[Axis],
    parent: Chain,
) -> Result<(), FactoryError> {
    writer.start_object();
    writer.key("subtype");
    writer.string(subtype);
    writer.key("axis");
    writer.start_array();
    for axis in axes {
        if axis.meridian.is_some() {
            return Err(FactoryError::Format(
                "PROJJSON axis meridian is not written in this build".to_owned(),
            ));
        }
        write_axis(writer, axis, parent);
    }
    writer.end_array();
    writer.end_object();
    Ok(())
}

fn write_axis(writer: &mut Writer, axis: &Axis, parent: Chain) {
    writer.start_object();
    writer.key("name");
    writer.string(axis.name());
    writer.key("abbreviation");
    writer.string(axis.abbreviation());
    writer.key("direction");
    writer.string(axis.direction());
    let unit = &axis.unit;
    if matches!(unit.name(), "metre" | "degree") {
        writer.key("unit");
        writer.string(unit.name());
    } else if unit.kind != UnitKind::None {
        writer.key("unit");
        let unit_place = Place::inside(parent, Ident::Lacks, Allow::Plain);
        write_unit_object(writer, unit, unit_place);
    }
    if let Some(minimum) = axis.minimum {
        writer.key("minimum_value");
        writer.number(minimum);
    }
    if let Some(maximum) = axis.maximum {
        writer.key("maximum_value");
        writer.number(maximum);
    }
    if let (Some(_), Some(_), Some(range)) = (axis.minimum, axis.maximum, &axis.range_meaning) {
        writer.key("range_meaning");
        writer.string(range);
    }
    writer.end_object();
}

/// A unit that is not metre, degree or unity, as an object. Its `id` is written when `place`
/// writes one.
fn write_unit_object(writer: &mut Writer, unit: &UnitOfMeasure, place: Place) {
    writer.start_object();
    writer.key("type");
    writer.string(match unit.kind {
        UnitKind::Linear => "LinearUnit",
        UnitKind::Angular => "AngularUnit",
        UnitKind::Scale => "ScaleUnit",
        UnitKind::Time => "TimeUnit",
        UnitKind::None | UnitKind::Unknown => "Unit",
    });
    writer.key("name");
    writer.string(unit.name());
    writer.key("conversion_factor");
    writer.number(unit.conversion_to_si());
    if place.id_out && !unit.code_space.is_empty() && !unit.code.is_empty() {
        write_id_code(writer, &unit.code_space, &unit.code);
    }
    writer.end_object();
}

/// A conversion inside a projected CRS: name, method, parameter values, and `id` by the usual
/// rule. The conversion has no `type` there.
fn write_conversion(
    writer: &mut Writer,
    conversion: &Conversion,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.start_object();
    write_conversion_members(writer, conversion, db, place);
    writer.end_object();
    Ok(())
}

/// The members of a conversion: name, method, parameter values, `id`. An inverse conversion
/// writes the inverse method, as `InverseConversion` builds it.
fn write_conversion_members(
    writer: &mut Writer,
    conversion: &Conversion,
    db: &DatabaseContext,
    place: Place,
) {
    write_name(writer, conversion.name());
    writer.key("method");
    let way = if conversion.inverted {
        MethodWay::Inverse
    } else {
        MethodWay::Forward
    };
    write_method(writer, &method_for_direction(&conversion.method, way));
    let parameters = conversion.parameters();
    if !parameters.is_empty() {
        writer.key("parameters");
        writer.start_array();
        for parameter in parameters {
            write_parameter(writer, parameter, place.chain);
        }
        writer.end_array();
    }
    write_id_if(writer, place, conversion.identifiers(), db);
}

/// Forward or inverse method, as written in PROJJSON.
enum MethodWay {
    Forward,
    Inverse,
}

/// The method of an operation. An inverse is `Inverse of ...` under the `INVERSE(...)`
/// code space, as `createPropertiesForInverse` names it.
fn method_for_direction(method: &OperationMethod, way: MethodWay) -> OperationMethod {
    match way {
        MethodWay::Forward => method.clone(),
        MethodWay::Inverse => method.inverse_of(),
    }
}

fn operation_ident(operation: &CoordinateOperation) -> Ident {
    ident(operation.identifiers())
}

/// An operation object: its `type`, then the members of its kind.
fn write_operation(
    writer: &mut Writer,
    operation: &CoordinateOperation,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    match operation {
        CoordinateOperation::Conversion(conversion) => {
            writer.key("type");
            writer.string("Conversion");
            write_conversion_members(writer, conversion, db, place);
            Ok(())
        }
        CoordinateOperation::Transformation(transformation) => {
            write_transformation(writer, transformation, db, place)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            write_concatenated(writer, concatenated, db, place)
        }
    }
}

/// A step or another operation inside a parent chain, which asks for its `id`.
fn write_operation_component(
    writer: &mut Writer,
    operation: &CoordinateOperation,
    db: &DatabaseContext,
    parent: Chain,
) -> Result<(), FactoryError> {
    let place = Place::inside(parent, operation_ident(operation), Allow::Asked);
    writer.start_object();
    write_operation(writer, operation, db, place)?;
    writer.end_object();
    Ok(())
}

fn write_transformation(
    writer: &mut Writer,
    transformation: &Transformation,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    let (Some(source), Some(target)) = (transformation.source_crs(), transformation.target_crs())
    else {
        return Err(FactoryError::Format(format!(
            "transformation {} has no CRS pair",
            transformation.name()
        )));
    };
    writer.key("type");
    writer.string("Transformation");
    write_name(writer, transformation.name());
    writer.key("source_crs");
    write_component(writer, source, db, place.chain, Allow::Asked)?;
    writer.key("target_crs");
    write_component(writer, target, db, place.chain, Allow::Asked)?;
    if let Some(interpolation) = transformation.interpolation_crs() {
        writer.key("interpolation_crs");
        write_component(writer, interpolation, db, place.chain, Allow::Asked)?;
    }
    writer.key("method");
    let way = if transformation.inverted {
        MethodWay::Inverse
    } else {
        MethodWay::Forward
    };
    write_method(writer, &method_for_direction(transformation.method(), way));
    writer.key("parameters");
    writer.start_array();
    for parameter in transformation.parameters() {
        write_parameter(writer, parameter, place.chain);
    }
    writer.end_array();
    if let Some(text) = transformation_accuracy_text(transformation) {
        writer.key("accuracy");
        writer.string(&text);
    }
    write_usage(writer, &transformation.identified.domains, place);
    write_id_if(writer, place, &transformation.identified.identifiers, db);
    write_remarks(writer, &transformation.identified.remarks);
    Ok(())
}

/// The accuracy of a transformation as PROJ's `PositionalAccuracy::value()` holds it: the text
/// the database gave, or the number written with `%.15g`.
fn transformation_accuracy_text(transformation: &Transformation) -> Option<String> {
    transformation.accuracy_text.clone().or_else(|| {
        transformation
            .accuracy()
            .map(|value| super::wkt::format_g(value, 15))
    })
}

fn write_concatenated(
    writer: &mut Writer,
    concatenated: &ConcatenatedOperation,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    let (source, target) = concatenated.crs_pair().ok_or_else(|| {
        FactoryError::Format(format!(
            "concatenated operation {} has no CRS pair",
            concatenated.name()
        ))
    })?;
    writer.key("type");
    writer.string("ConcatenatedOperation");
    write_name(writer, concatenated.name());
    writer.key("source_crs");
    write_component(writer, source, db, place.chain, Allow::Asked)?;
    writer.key("target_crs");
    write_component(writer, target, db, place.chain, Allow::Asked)?;
    writer.key("steps");
    writer.start_array();
    for step in concatenated.steps() {
        write_operation_component(writer, step.operation(), db, place.chain)?;
    }
    writer.end_array();
    if let Some(text) = concatenated.accuracy_text() {
        writer.key("accuracy");
        writer.string(&text);
    }
    write_usage(writer, &concatenated.identified.domains, place);
    write_id_if(writer, place, &concatenated.identified.identifiers, db);
    write_remarks(writer, &concatenated.identified.remarks);
    Ok(())
}

/// An operation method: its name and `id`. The method's `id` is always written, PROJ's
/// `setAllowIDInImmediateChild` comes before it.
fn write_method(writer: &mut Writer, method: &OperationMethod) {
    writer.start_object();
    writer.key("name");
    writer.string(method.name());
    if !method.code_space().is_empty() && !method.code().is_empty() {
        write_id_code(writer, method.code_space(), method.code());
    }
    writer.end_object();
}

/// One parameter value: name, then `value` and `unit`, then `id`. The `id` is always written. A
/// string or boolean value is not written, as PROJ writes neither.
fn write_parameter(writer: &mut Writer, parameter: &Parameter, parent: Chain) {
    writer.start_object();
    writer.key("name");
    writer.string(parameter.name());
    let has_id = code_ident(parameter.code_space());
    match parameter.value() {
        ParameterValue::Measure(measure) => {
            writer.key("value");
            writer.number(measure.value());
            writer.key("unit");
            let unit = measure.unit();
            if matches!(unit.name(), "metre" | "degree" | "unity") {
                writer.string(unit.name());
            } else {
                let unit_place = Place::inside(parent, has_id, Allow::Plain);
                write_unit_object(writer, unit, unit_place);
            }
        }
        ParameterValue::Filename(text) => {
            writer.key("value");
            writer.string(text);
        }
        ParameterValue::Integer(value) => {
            writer.key("value");
            writer.raw_token(&value.to_string());
        }
        ParameterValue::String(_) | ParameterValue::Boolean(_) => {}
    }
    if has_id == Ident::Has && !parameter.code().is_empty() {
        write_id_code(writer, parameter.code_space(), parameter.code());
    }
    writer.end_object();
}

/// Usage of a top object: `scope`, `area` and `bbox` for one domain, or a `usages` array for
/// several. Nothing is written below the top object.
fn write_usage(writer: &mut Writer, domains: &[ObjectDomain], place: Place) {
    if !place.top {
        return;
    }
    match domains {
        [] => {}
        [domain] => write_domain_members(writer, domain),
        many => {
            writer.key("usages");
            writer.start_array();
            for domain in many {
                writer.start_object();
                write_domain_members(writer, domain);
                writer.end_object();
            }
            writer.end_array();
        }
    }
}

fn write_domain_members(writer: &mut Writer, domain: &ObjectDomain) {
    if let Some(scope) = domain.scope() {
        writer.key("scope");
        writer.string(scope);
    }
    if let Some(area) = domain.area() {
        writer.key("area");
        writer.string(area);
    }
    if let (Some(south), Some(west), Some(north), Some(east)) = (
        domain.south_bound_latitude(),
        domain.west_bound_longitude(),
        domain.north_bound_latitude(),
        domain.east_bound_longitude(),
    ) {
        writer.key("bbox");
        writer.start_object();
        writer.key("south_latitude");
        writer.number(south);
        writer.key("west_longitude");
        writer.number(west);
        writer.key("north_latitude");
        writer.number(north);
        writer.key("east_longitude");
        writer.number(east);
        writer.end_object();
    }
}

/// The `remarks` of an object, `formatRemarks`: nothing when there are none.
fn write_remarks(writer: &mut Writer, remarks: &str) {
    if !remarks.is_empty() {
        writer.key("remarks");
        writer.string(remarks);
    }
}

/// The `id` of an object that writes one, when `place` says so.
fn write_id_if(
    writer: &mut Writer,
    place: Place,
    identifiers: &[Identifier],
    db: &DatabaseContext,
) {
    if place.id_out {
        write_id(writer, identifiers, db);
    }
}

/// The `id` of an object: its first identifier, with the authority and, when `proj.db` has one,
/// the version. Only the first identifier is written.
fn write_id(writer: &mut Writer, identifiers: &[Identifier], db: &DatabaseContext) {
    let Some(identifier) = identifiers.first() else {
        return;
    };
    if identifier.code_space().is_empty() || identifier.code().is_empty() {
        return;
    }
    let (authority, version) = db
        .authority_and_version(identifier.code_space())
        .map(|(authority, version)| (authority, Some(version)))
        .unwrap_or_else(|| (identifier.code_space().to_owned(), None));
    writer.key("id");
    writer.start_object();
    writer.key("authority");
    writer.string(&authority);
    writer.key("code");
    write_code(writer, identifier.code());
    if let Some(version) = version {
        writer.key("version");
        write_code(writer, &version);
    }
    writer.end_object();
}

/// An `id` object with an `authority` and a `code`, for a parameter, method or unit.
fn write_id_code(writer: &mut Writer, code_space: &str, code: &str) {
    writer.key("id");
    writer.start_object();
    writer.key("authority");
    writer.string(code_space);
    writer.key("code");
    write_code(writer, code);
    writer.end_object();
}

fn write_code(writer: &mut Writer, code: &str) {
    if is_integer_token(code) {
        writer.raw_token(code);
    } else {
        writer.string(code);
    }
}

fn is_integer_token(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| ch.is_ascii_digit())
}

/// `createFromUserInput` for an authority code, PROJJSON (`{...}`), a PROJ
/// string, and WKT.
///
/// `AUTH:code` and `AUTH:horizontal+vertical` (including
/// `AUTH:code+AUTH:code`) are resolved in `db`. A `+` between two codes of
/// the same authority is the implicit compound of PROJ's user-input parser.
///
/// # Errors
///
/// The text is not an authority code, PROJJSON this importer accepts, a PROJ
/// string this importer accepts, or WKT for a CRS.
pub fn create_from_user_input(text: &str, db: &DatabaseContext) -> Result<Crs, FactoryError> {
    let text = text.trim();
    if text.starts_with('{') {
        return import_json(text, db);
    }
    let bare = text.strip_prefix('+').unwrap_or(text);
    if bare.starts_with("proj=") || text.contains(" +proj=") || text.contains(" proj=") {
        return create_crs_from_proj_string_with_database(text, db);
    }
    if let Some(crs) = crs_from_authority_code(text, db)? {
        return Ok(crs);
    }
    if !text.contains(['[', '+', ':']) || text.contains(" + ") {
        if let Some(crs) = crs_from_name(text, db)? {
            return Ok(crs);
        }
    }
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    parser
        .create_from_wkt(text)?
        .crs()
        .cloned()
        .ok_or_else(|| FactoryError::Invalid("WKT is not a CRS".to_owned()))
}

/// The `EPSG:CODE` text of a CRS that `text` names, or `EPSG:A+B` for a
/// compound of two EPSG CRS. `None` when the CRS has no EPSG identity.
///
/// # Errors
///
/// `text` does not name a CRS this crate can build.
pub fn epsg_identity(text: &str, db: &DatabaseContext) -> Result<Option<String>, FactoryError> {
    let crs = create_from_user_input(text, db)?;
    if let Some(code) = epsg_code_of(&crs) {
        return Ok(Some(format!("EPSG:{code}")));
    }
    if let Crs::Compound(compound) = &crs {
        let parts: Vec<Option<String>> = compound.components().iter().map(epsg_code_of).collect();
        if let [Some(first), Some(second)] = parts.as_slice() {
            return Ok(Some(format!("EPSG:{first}+{second}")));
        }
    }
    Ok(None)
}

fn epsg_code_of(crs: &Crs) -> Option<String> {
    crs.authority_code("EPSG")
        .map(|identifier| identifier.code().to_owned())
}

/// A CRS named `text`, or `A + B` for two named CRS, as PROJ's
/// `createFromUserInput` does for names. An exact name first, then an
/// approximate one for a single name; several matches keep the 2D one, then
/// the 3D one, and anything else is an error.
fn crs_from_name(text: &str, db: &DatabaseContext) -> Result<Option<Crs>, FactoryError> {
    if let Some((left, right)) = text.split_once(" + ") {
        let (Some(first), Some(second)) = (
            crs_named_one(left.trim(), db, NameMatch::Approximate)?,
            crs_named_one(right.trim(), db, NameMatch::Approximate)?,
        ) else {
            return Ok(None);
        };
        return Ok(Some(Crs::Compound(CompoundCRS::create(
            format!("{} + {}", first.name(), second.name()),
            vec![first, second],
        )?)));
    }
    if let Some(crs) = crs_named_one(text, db, NameMatch::Exact)? {
        return Ok(Some(crs));
    }
    // PROJ's `createFromUserInput` tries an approximate match only for a text
    // longer than four bytes, and keeps it only when its name is shorter than
    // twice the text, so `EGM2008` does not pick `EGM2008 height`.
    if text.len() <= 4 {
        return Ok(None);
    }
    Ok(crs_named_one(text, db, NameMatch::Approximate)?
        .filter(|crs| crs.name().len() < 2 * text.len()))
}

fn crs_named_one(
    text: &str,
    db: &DatabaseContext,
    mode: NameMatch,
) -> Result<Option<Crs>, FactoryError> {
    let codes = db.crs_codes_named(text)?;
    let mut found = Vec::new();
    for (authority, code) in &codes {
        found.push(
            AuthorityFactory::create(db, authority).create_coordinate_reference_system(code)?,
        );
    }
    if found.is_empty() && mode == NameMatch::Approximate {
        for object in AuthorityFactory::create(db, "").create_objects_from_name(
            text,
            &[ObjectType::Crs],
            NameMatch::Approximate,
            10,
        )? {
            if let AuthorityObject::Crs(crs) = object {
                found.push(crs);
            }
        }
    }
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.pop()),
        _ => {
            for dimension in [2, 3] {
                if let Some(crs) = found
                    .iter()
                    .find(|crs| geographic_dimension(crs) == Some(dimension))
                {
                    return Ok(Some(crs.clone()));
                }
            }
            for bigger_differences in [false, true] {
                let equivalent: Vec<&Crs> = found
                    .iter()
                    .filter(|crs| {
                        if bigger_differences {
                            equiv::equivalent_name_bigger_differences(crs.name(), text)
                        } else {
                            equiv::equivalent_name(crs.name(), text)
                        }
                    })
                    .collect();
                if let [only] = equivalent.as_slice() {
                    return Ok(Some((*only).clone()));
                }
            }
            let names: Vec<&str> = found.iter().map(Crs::name).collect();
            Err(FactoryError::Invalid(format!(
                "several objects matching this name: {}",
                names.join(", ")
            )))
        }
    }
}

/// The axis count of a geographic CRS. A geocentric CRS is not one.
fn geographic_dimension(crs: &Crs) -> Option<usize> {
    match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            Some(geographic.coordinate_system().axes().len())
        }
        _ => None,
    }
}

/// `AUTH:CODE`, `AUTH:CODE+CODE`, or `AUTH:CODE+AUTH:CODE` with no spaces.
///
/// Anything else returns `Ok(None)` so the caller can try WKT.
fn crs_from_authority_code(text: &str, db: &DatabaseContext) -> Result<Option<Crs>, FactoryError> {
    if text.contains([' ', '\t']) {
        return Ok(None);
    }
    let tokens: Vec<&str> = text.split(':').collect();
    if tokens.len() == 2 {
        let factory = AuthorityFactory::create(db, tokens[0]);
        if let Ok(crs) = factory.create_coordinate_reference_system(tokens[1]) {
            return Ok(Some(crs));
        }
        let parts: Vec<&str> = tokens[1].split('+').collect();
        if parts.len() == 2 {
            return Ok(Some(implicit_compound(
                &factory, parts[0], &factory, parts[1],
            )?));
        }
        return Err(FactoryError::missing("CRS", tokens[0], tokens[1]));
    }
    if tokens.len() == 3 {
        let center: Vec<&str> = tokens[1].split('+').collect();
        if center.len() == 2 {
            let first = AuthorityFactory::create(db, tokens[0]);
            let second = AuthorityFactory::create(db, center[1]);
            return Ok(Some(implicit_compound(
                &first, center[0], &second, tokens[2],
            )?));
        }
    }
    Ok(None)
}

fn implicit_compound(
    first_factory: &AuthorityFactory,
    first_code: &str,
    second_factory: &AuthorityFactory,
    second_code: &str,
) -> Result<Crs, FactoryError> {
    let horizontal = first_factory.create_coordinate_reference_system(first_code)?;
    let vertical = second_factory.create_coordinate_reference_system(second_code)?;
    let name = format!("{} + {}", horizontal.name(), vertical.name());
    CompoundCRS::create_lax(name, vec![horizontal, vertical])
}

fn import_json(text: &str, db: &DatabaseContext) -> Result<Crs, FactoryError> {
    let value = crs_text::parse_json(text)
        .map_err(|error| FactoryError::Invalid(format!("PROJJSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| FactoryError::Invalid("PROJJSON root is not an object".to_owned()))?;
    let kind = required_str(object, "type")?;
    match kind {
        "GeographicCRS" => Ok(Crs::Geographic(read_geographic(object, db)?)),
        "ProjectedCRS" => read_projected(object, db),
        other => Err(FactoryError::Invalid(format!(
            "PROJJSON type {other} is not imported in this build"
        ))),
    }
}

/// A projected CRS. The base is geographic. A missing `type` on the base is still geographic:
/// PROJ's own projected-CRS unit test compares a document that omits it. Names may be absent;
/// they are stored as `unnamed`, which is what the writer emits for an empty name.
fn read_projected(object: &Object, db: &DatabaseContext) -> Result<Crs, FactoryError> {
    let name = optional_name(object);
    let base = object
        .get("base_crs")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            FactoryError::Invalid("PROJJSON projected CRS has no base_crs".to_owned())
        })?;
    let base_kind = base
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("GeographicCRS");
    if base_kind != "GeographicCRS" {
        return Err(FactoryError::Invalid(format!(
            "PROJJSON projected base CRS type {base_kind} is not imported"
        )));
    }
    let geographic = read_geographic(base, db)?;
    let conversion_object = object
        .get("conversion")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            FactoryError::Invalid("PROJJSON projected CRS has no conversion".to_owned())
        })?;
    let conversion = read_conversion(conversion_object, db)?;
    let coordinate_system = object
        .get("coordinate_system")
        .and_then(Value::as_object)
        .map(read_cartesian_cs)
        .transpose()?
        .unwrap_or_else(|| CartesianCS::create_easting_northing(UnitOfMeasure::metre()));
    let mut crs = ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(geographic),
        conversion,
        coordinate_system,
    );
    if let Some(identifier) = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten()
    {
        crs = crs.with_identifier(identifier.code_space(), identifier.code());
    }
    Ok(Crs::Projected(crs))
}

fn read_conversion(object: &Object, db: &DatabaseContext) -> Result<Conversion, FactoryError> {
    let name = optional_name(object);
    let method_object = object
        .get("method")
        .and_then(Value::as_object)
        .ok_or_else(|| FactoryError::Invalid("PROJJSON conversion has no method".to_owned()))?;
    let (method, mapped) = read_method(method_object, db)?;
    let parameters = object
        .get("parameters")
        .and_then(Value::as_array)
        .map(|items| read_parameters(items, mapped, db))
        .transpose()?
        .unwrap_or_default();
    let mut identified = Identified::named(name);
    if let Some(identifier) = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten()
    {
        identified = identified.with_id(identifier.code_space(), identifier.code());
    }
    Ok(Conversion {
        identified,
        method,
        parameters,
        source: None,
        target: None,
        version: None,
        inverted: false,
    })
}

fn read_method(
    object: &Object,
    db: &DatabaseContext,
) -> Result<(OperationMethod, &'static super::parammap::MethodMap), FactoryError> {
    let name = object.get("name").and_then(Value::as_str);
    let identifier = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten();
    let code_number = identifier
        .as_ref()
        .and_then(|identifier| identifier.code().parse::<i32>().ok());
    let mapped = code_number
        .and_then(method_by_code)
        .or_else(|| name.and_then(method_by_name))
        .ok_or_else(|| {
            FactoryError::Invalid(format!(
                "PROJJSON conversion method {} is not imported",
                name.unwrap_or("without a name")
            ))
        })?;
    let (code_space, code) = match &identifier {
        Some(identifier) => (
            identifier.code_space().to_owned(),
            identifier.code().to_owned(),
        ),
        None => (String::new(), String::new()),
    };
    let method_name = name.unwrap_or(mapped.wkt2_name).to_owned();
    Ok((
        OperationMethod::named_code(method_name, code_space, code),
        mapped,
    ))
}

fn read_parameters(
    items: &[Value],
    method: &super::parammap::MethodMap,
    db: &DatabaseContext,
) -> Result<Vec<Parameter>, FactoryError> {
    let mut parameters = Vec::with_capacity(items.len());
    for item in items {
        let object = item.as_object().ok_or_else(|| {
            FactoryError::Invalid("PROJJSON parameter is not an object".to_owned())
        })?;
        let name = optional_name(object);
        let identifier = object
            .get("id")
            .and_then(Value::as_object)
            .map(|id| read_id(id, db))
            .transpose()?
            .flatten();
        let value = object.get("value").and_then(Value::as_f64).ok_or_else(|| {
            FactoryError::Invalid(format!("PROJJSON parameter {name} has no value"))
        })?;
        let unit = match object.get("unit") {
            Some(unit) => read_unit_value(unit)?,
            None => parameter_unit(method, name, identifier.as_ref())?,
        };
        let (code_space, code) = match &identifier {
            Some(identifier) => (
                identifier.code_space().to_owned(),
                identifier.code().to_owned(),
            ),
            None => (String::new(), String::new()),
        };
        parameters.push(Parameter {
            name: name.to_owned(),
            code_space,
            code,
            value: ParameterValue::Measure(Measure { value, unit }),
        });
    }
    Ok(parameters)
}

/// Unit of a parameter that names none, from the method's parameter map.
fn parameter_unit(
    method: &super::parammap::MethodMap,
    name: &str,
    identifier: Option<&Identifier>,
) -> Result<UnitOfMeasure, FactoryError> {
    let code = identifier.and_then(|identifier| identifier.code().parse::<i32>().ok());
    let found = method.params.iter().find(|param| {
        code.is_some_and(|code| code != 0 && param.epsg_code == code)
            || same_token(param.wkt2_name, name)
    });
    let Some(param) = found else {
        return Err(FactoryError::Invalid(format!(
            "PROJJSON parameter {name} has no unit"
        )));
    };
    Ok(match param.unit {
        UnitKind::Linear => UnitOfMeasure::metre(),
        UnitKind::Angular => UnitOfMeasure::degree(),
        UnitKind::Scale => UnitOfMeasure::unity(),
        UnitKind::Time | UnitKind::None | UnitKind::Unknown => {
            return Err(FactoryError::Invalid(format!(
                "PROJJSON parameter {name} has no unit"
            )));
        }
    })
}

fn same_token(left: &str, right: &str) -> bool {
    let fold = |text: &str| {
        text.chars()
            .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '/'))
            .flat_map(|ch| ch.to_lowercase())
            .collect::<String>()
    };
    fold(left) == fold(right)
}

fn read_cartesian_cs(object: &Object) -> Result<CartesianCS, FactoryError> {
    let Some(axes) = object.get("axis").and_then(Value::as_array) else {
        return Ok(CartesianCS::create_easting_northing(UnitOfMeasure::metre()));
    };
    let mut built = Vec::new();
    for axis in axes {
        let Some(axis) = axis.as_object() else {
            continue;
        };
        built.push(read_axis(axis, UnitOfMeasure::metre())?);
    }
    CartesianCS::create(built)
}

fn read_axis(axis: &Object, default_unit: UnitOfMeasure) -> Result<Axis, FactoryError> {
    let name = axis.get("name").and_then(Value::as_str).unwrap_or("axis");
    let abbreviation = axis
        .get("abbreviation")
        .and_then(Value::as_str)
        .unwrap_or("");
    let direction = axis
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("north");
    let unit = match axis.get("unit") {
        Some(unit) => read_unit_value(unit)?,
        None => default_unit,
    };
    Ok(Axis::new(name, abbreviation, direction, unit))
}

fn read_unit_value(value: &Value) -> Result<UnitOfMeasure, FactoryError> {
    match value {
        Value::String(name) => named_unit(name, None),
        Value::Object(object) => read_unit(object),
        _ => Err(FactoryError::Invalid(
            "PROJJSON unit is not a string or an object".to_owned(),
        )),
    }
}

fn named_unit(name: &str, factor: Option<f64>) -> Result<UnitOfMeasure, FactoryError> {
    if name.eq_ignore_ascii_case("degree") {
        return Ok(UnitOfMeasure::degree());
    }
    if name.eq_ignore_ascii_case("metre") || name.eq_ignore_ascii_case("meter") {
        return Ok(UnitOfMeasure::metre());
    }
    if name.eq_ignore_ascii_case("unity") {
        return Ok(UnitOfMeasure::unity());
    }
    Ok(UnitOfMeasure {
        name: name.to_owned(),
        factor: factor.unwrap_or(1.0),
        kind: UnitKind::Unknown,
        code_space: String::new(),
        code: String::new(),
    })
}

fn optional_name(object: &Object) -> &str {
    object
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("unnamed")
}

fn read_geographic(object: &Object, db: &DatabaseContext) -> Result<GeographicCRS, FactoryError> {
    let name = optional_name(object);
    let datum = object
        .get("datum")
        .and_then(Value::as_object)
        .map(|datum| read_datum(datum, db))
        .transpose()?;
    let ensemble = object
        .get("datum_ensemble")
        .and_then(Value::as_object)
        .map(|ensemble| read_ensemble(ensemble, db))
        .transpose()?;
    if datum.is_none() && ensemble.is_none() {
        return Err(FactoryError::Invalid(
            "PROJJSON geographic CRS has no datum".to_owned(),
        ));
    }
    let coordinate_system = object
        .get("coordinate_system")
        .and_then(Value::as_object)
        .map(read_ellipsoidal_cs)
        .transpose()?
        .unwrap_or_else(|| EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()));
    let mut identified = Identified::named(name);
    if let Some(identifier) = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten()
    {
        identified.identifiers.push(identifier);
    }
    Ok(GeographicCRS {
        identified,
        datum,
        ensemble,
        coordinate_system,
        implicit_cs: false,
    })
}

fn read_datum(
    object: &Object,
    db: &DatabaseContext,
) -> Result<GeodeticReferenceFrame, FactoryError> {
    let name = optional_name(object);
    let ellipsoid = object
        .get("ellipsoid")
        .and_then(Value::as_object)
        .map(|ellipsoid| read_ellipsoid(ellipsoid, db))
        .transpose()?
        .unwrap_or_else(Ellipsoid::wgs84);
    let prime_meridian = object
        .get("prime_meridian")
        .and_then(Value::as_object)
        .map(read_meridian)
        .transpose()?
        .unwrap_or_else(PrimeMeridian::greenwich);
    let mut identified = Identified::named(name);
    if let Some(identifier) = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten()
    {
        identified.identifiers.push(identifier);
    }
    Ok(GeodeticReferenceFrame {
        identified,
        ellipsoid,
        prime_meridian,
        frame_epoch: None,
        deformation_model: None,
    })
}

fn read_ensemble(object: &Object, db: &DatabaseContext) -> Result<DatumEnsemble, FactoryError> {
    let name = required_str(object, "name")?;
    let members = object
        .get("members")
        .and_then(Value::as_array)
        .ok_or_else(|| FactoryError::Invalid("PROJJSON ensemble has no members".to_owned()))?;
    let mut frames = Vec::new();
    for member in members {
        let Some(member) = member.as_object() else {
            continue;
        };
        frames.push(super::model::Datum::Geodetic(read_datum(member, db)?));
    }
    let accuracy = object
        .get("accuracy")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let mut ensemble = DatumEnsemble::create(name, frames, accuracy)?;
    if let Some(identifier) = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten()
    {
        ensemble.identified.identifiers.push(identifier);
    }
    Ok(ensemble)
}

fn read_ellipsoid(object: &Object, db: &DatabaseContext) -> Result<Ellipsoid, FactoryError> {
    let name = optional_name(object);
    let semi_major = object
        .get("semi_major_axis")
        .and_then(Value::as_f64)
        .unwrap_or(6_378_137.0);
    let inverse = object
        .get("inverse_flattening")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let mut identified = Identified::named(name);
    if let Some(identifier) = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten()
    {
        identified.identifiers.push(identifier);
    }
    Ok(Ellipsoid {
        identified,
        semi_major: Measure::length(semi_major),
        inverse_flattening: Some(inverse),
        semi_minor: None,
        body: String::new(),
    })
}

fn read_meridian(object: &Object) -> Result<PrimeMeridian, FactoryError> {
    let name = required_str(object, "name")?;
    let longitude = object
        .get("longitude")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    Ok(PrimeMeridian {
        identified: Identified::named(name),
        longitude: Measure::angle(longitude),
    })
}

fn read_ellipsoidal_cs(object: &Object) -> Result<EllipsoidalCS, FactoryError> {
    let Some(axes) = object.get("axis").and_then(Value::as_array) else {
        return Ok(EllipsoidalCS::create_latitude_longitude(
            UnitOfMeasure::degree(),
        ));
    };
    let mut built = Vec::new();
    for axis in axes {
        let Some(axis) = axis.as_object() else {
            continue;
        };
        built.push(read_axis(axis, UnitOfMeasure::degree())?);
    }
    if built.len() == 2 || built.len() == 3 {
        Ok(EllipsoidalCS {
            identified: Identified::default(),
            axes: built,
        })
    } else {
        Ok(EllipsoidalCS::create_latitude_longitude(
            UnitOfMeasure::degree(),
        ))
    }
}

fn read_unit(object: &Object) -> Result<UnitOfMeasure, FactoryError> {
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("degree");
    let factor = object.get("conversion_factor").and_then(Value::as_f64);
    named_unit(name, factor)
}

fn read_id(object: &Object, db: &DatabaseContext) -> Result<Option<Identifier>, FactoryError> {
    let Some(authority) = object.get("authority").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(code_value) = object.get("code") else {
        return Err(FactoryError::Invalid(
            "PROJJSON id is missing \"code\"".to_owned(),
        ));
    };
    let code = json_code(code_value)?;
    let version = object.get("version").and_then(json_version);
    let code_space = version
        .as_deref()
        .and_then(|version| db.versioned_authority(authority, version))
        .unwrap_or_else(|| authority.to_owned());
    Ok(Some(Identifier::new(code_space, code)))
}

fn json_code(value: &Value) -> Result<String, FactoryError> {
    if let Some(text) = value.as_str() {
        return Ok(text.to_owned());
    }
    let number = value.as_f64().ok_or_else(|| {
        FactoryError::Invalid("PROJJSON id code is not a string or a number".to_owned())
    })?;
    if number.fract() == 0.0 {
        Ok(format!("{number:.0}"))
    } else {
        Ok(number.to_string())
    }
}

fn json_version(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_owned());
    }
    let number = value.as_f64()?;
    if number.fract() == 0.0 {
        Some(format!("{number:.0}"))
    } else {
        Some(super::wkt::format_g(number, 15))
    }
}

fn required_str<'a>(object: &'a Object, key: &str) -> Result<&'a str, FactoryError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| FactoryError::Invalid(format!("PROJJSON object is missing {key}")))
}
