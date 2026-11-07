//! PROJJSON export and the `{` branch of `createFromUserInput`.
//!
//! Identifier code spaces such as `IAU_2015` are split into an authority and a
//! version on the way out, and joined again on the way in, using
//! `versioned_auth_name_mapping`.

use super::buildop::OpProps;
use super::equiv;
use super::factory::{AuthorityFactory, AuthorityObject, DatabaseContext, NameMatch, ObjectType};
use super::json_read::{create_object_from_json, JsonObject};
use super::model::{
    Axis, BoundCRS, CartesianCS, CompoundCRS, ConcatenatedOperation, Conversion,
    CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble, DerivedProjectedCRS,
    Ellipsoid, EllipsoidalCS, EngineeringCRS, EngineeringDatum, FactoryError, GeocentricCRS,
    GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, GeoidModel, Identified, Identifier,
    Measure, ObjectDomain, OperationMethod, Parameter, ParameterValue, PrimeMeridian, ProjectedCRS,
    SphericalGeodeticCRS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS,
    VerticalExtent, VerticalReferenceFrame,
};
use super::niche::{
    DerivedCRS, ParametricCRS, ParametricCS, ParametricDatum, TemporalCRS, TemporalCS,
    TemporalCsKind, TemporalDatum, PROLEPTIC_GREGORIAN,
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

impl JsonObject {
    /// PROJJSON for this object, in PROJ's default layout.
    ///
    /// # Errors
    ///
    /// The object, or a part of it, has no PROJJSON export in this build.
    pub fn export_to_json(&self, db: &DatabaseContext) -> Result<String, FactoryError> {
        self.export_to_json_with(db, &JsonFormat::default())
    }

    /// PROJJSON for this object in `format`.
    ///
    /// # Errors
    ///
    /// As [`Self::export_to_json`].
    pub fn export_to_json_with(
        &self,
        db: &DatabaseContext,
        format: &JsonFormat,
    ) -> Result<String, FactoryError> {
        if let Self::Crs(crs) = self {
            return crs.export_to_json_with(db, format);
        }
        let mut writer = Writer::new(format);
        writer.start_object();
        if !format.schema.is_empty() {
            writer.key("$schema");
            writer.string(&format.schema);
        }
        match self {
            Self::Ellipsoid(ellipsoid) => {
                writer.key("type");
                writer.string("Ellipsoid");
                write_ellipsoid_members(
                    &mut writer,
                    ellipsoid,
                    db,
                    Place::top(ident(ellipsoid.identifiers())),
                );
            }
            Self::PrimeMeridian(meridian) => {
                writer.key("type");
                writer.string("PrimeMeridian");
                write_meridian_members(
                    &mut writer,
                    meridian,
                    db,
                    Place::top(ident(meridian.identifiers())),
                );
            }
            Self::Axis(axis) => {
                writer.key("type");
                writer.string("Axis");
                write_axis_members(&mut writer, axis, Chain::Clear);
            }
            Self::GeodeticReferenceFrame(datum) => {
                write_datum_fields(
                    &mut writer,
                    datum,
                    db,
                    Place::top(ident(datum.identifiers())),
                )?;
            }
            Self::VerticalReferenceFrame(datum) => {
                write_vertical_datum_members(
                    &mut writer,
                    datum,
                    db,
                    Place::top(ident(datum.identifiers())),
                )?;
            }
            Self::DatumEnsemble(ensemble) => {
                writer.key("type");
                writer.string("DatumEnsemble");
                write_ensemble_members(&mut writer, ensemble, db)?;
            }
            Self::Transformation(transformation) => {
                write_transformation(
                    &mut writer,
                    transformation,
                    db,
                    Place::top(ident(transformation.identifiers())),
                )?;
            }
            Self::Concatenated(operation) => {
                write_concatenated(
                    &mut writer,
                    operation,
                    db,
                    Place::top(ident(&operation.identified.identifiers)),
                )?;
            }
            Self::Conversion(_) | Self::Crs(_) => {
                return Err(FactoryError::Format(
                    "PROJJSON export of this object is not built".to_owned(),
                ));
            }
        }
        writer.end_object();
        Ok(writer.finish())
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

impl CoordinateSystem {
    /// PROJJSON for this coordinate system in `format`, as `CoordinateSystem::exportToJSON`
    /// writes it: its subtype, its axes, and its `id` when it has one.
    ///
    /// # Errors
    ///
    /// The export fails when a part of the system has no PROJJSON export in this build.
    pub fn export_to_json_with(
        &self,
        db: &DatabaseContext,
        format: &JsonFormat,
    ) -> Result<String, FactoryError> {
        export_wrapped(format, |writer| {
            writer.key("type");
            writer.string("CoordinateSystem");
            writer.key("subtype");
            writer.string(self.wkt2_type());
            writer.key("axis");
            writer.start_array();
            for axis in self.axes() {
                write_axis(writer, axis, Chain::Clear);
            }
            writer.end_array();
            write_id(writer, self.identifiers(), db);
            Ok(())
        })
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

/// Whether the object that carries this value has an `id`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Chain {
    Clear,
    Has,
}

/// Where an object sits, as PROJ's `JSONFormatter` tracks it.
///
/// PROJ writes an object's `id` when it is the top object, when its parent asks for it, or when
/// its parent has no `id`. The usage block (scope, area, extent) is written for the top object
/// only.
#[derive(Clone, Copy)]
struct Place {
    /// The top object of the export.
    top: bool,
    /// Whether this object writes its own `id`.
    id_out: bool,
    /// Whether this object has an `id`. Its children read it as their parent state.
    chain: Chain,
}

impl Place {
    /// The top object, which always writes its `id`.
    fn top(ident: Ident) -> Self {
        Self {
            top: true,
            id_out: true,
            chain: chain_of(ident),
        }
    }

    /// An object with `ident`, inside a parent whose `id` state is `parent`.
    fn inside(parent: Chain, ident: Ident, allow: Allow) -> Self {
        Self {
            top: false,
            id_out: allow == Allow::Asked || parent == Chain::Clear,
            chain: chain_of(ident),
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

fn chain_of(ident: Ident) -> Chain {
    if ident == Ident::Has {
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
        Crs::Geodetic(GeodeticCRS::Spherical(crs)) => ident(&crs.identified.identifiers),
        Crs::Projected(crs) => ident(crs.identifiers()),
        Crs::Vertical(crs) => ident(crs.identifiers()),
        Crs::Compound(crs) => ident(crs.identifiers()),
        Crs::Bound(crs) => ident(&crs.identified.identifiers),
        Crs::Engineering(crs) => ident(&crs.identified.identifiers),
        Crs::DerivedProjected(crs) => ident(&crs.identified.identifiers),
        Crs::Temporal(crs) => ident(crs.identifiers()),
        Crs::Parametric(crs) => ident(crs.identifiers()),
        Crs::Derived(crs) => ident(crs.identifiers()),
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
        Crs::Geodetic(GeodeticCRS::Spherical(crs)) => write_spherical(writer, crs, db, place),
        Crs::Projected(crs) => write_projected(writer, crs, db, place),
        Crs::Vertical(crs) => write_vertical(writer, crs, db, place),
        Crs::Compound(crs) => write_compound(writer, crs, db, place),
        Crs::Bound(crs) => write_bound(writer, crs, db, place),
        Crs::Engineering(crs) => write_engineering(writer, crs, db, place),
        Crs::Temporal(crs) => write_temporal(writer, crs, db, place),
        Crs::Parametric(crs) => write_parametric(writer, crs, db, place),
        Crs::Derived(crs) => write_derived(writer, crs, db, place),
        Crs::DerivedProjected(crs) => write_derived_projected(writer, crs, db, place),
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
    write_deformation_models(writer, dynamic_model(crs.datum()));
    write_base(writer, &crs.identified, place, db);
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
    write_deformation_models(writer, dynamic_model(crs.datum()));
    write_base(writer, &crs.identified, place, db);
    Ok(())
}

fn write_spherical(
    writer: &mut Writer,
    crs: &SphericalGeodeticCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("GeodeticCRS");
    write_name(writer, &crs.identified.name);
    write_datum_or_ensemble(
        writer,
        crs.datum.as_ref(),
        crs.ensemble.as_ref(),
        db,
        place.chain,
    )?;
    writer.key("coordinate_system");
    write_cs(
        writer,
        "spherical",
        &crs.coordinate_system.axes,
        place.chain,
    )?;
    write_deformation_models(writer, dynamic_model(crs.datum.as_ref()));
    write_base(writer, &crs.identified, place, db);
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
    write_base(writer, &crs.identified, place, db);
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
        write_ensemble(writer, ensemble, db)?;
    }
    writer.key("coordinate_system");
    write_cs(
        writer,
        "vertical",
        std::slice::from_ref(crs.coordinate_system().axis()),
        place.chain,
    )?;
    write_deformation_models(
        writer,
        crs.datum()
            .filter(|datum| datum.frame_reference_epoch().is_some())
            .and_then(VerticalReferenceFrame::deformation_model),
    );
    write_geoid_models(writer, crs.geoid_models(), db, place.chain)?;
    write_base(writer, &crs.identified, place, db);
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
    write_base(writer, &crs.identified, place, db);
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
    write_component(writer, source, db, place.chain, Allow::Asked)?;
    writer.key("target_crs");
    write_component(writer, crs.target_crs(), db, place.chain, Allow::Asked)?;
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
    write_base(writer, &crs.identified, place, db);
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
fn dynamic_model(datum: Option<&GeodeticReferenceFrame>) -> Option<&str> {
    datum
        .filter(|datum| datum.frame_reference_epoch().is_some())
        .and_then(GeodeticReferenceFrame::deformation_model)
}

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
        write_ensemble(writer, ensemble, db)?;
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
    write_datum_fields(writer, datum, db, place)?;
    writer.end_object();
    Ok(())
}

fn write_datum_fields(
    writer: &mut Writer,
    datum: &GeodeticReferenceFrame,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    let dynamic = datum.frame_reference_epoch().is_some();
    writer.key("type");
    writer.string(if dynamic {
        "DynamicGeodeticReferenceFrame"
    } else {
        "GeodeticReferenceFrame"
    });
    write_name(writer, datum.name());
    write_anchor(writer, datum.anchor(), datum.anchor_epoch())?;
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
    write_temporal_extent(
        writer,
        datum.identified.temporal_start.as_deref(),
        datum.identified.temporal_end.as_deref(),
    );
    write_base(writer, &datum.identified, place, db);
    Ok(())
}

fn write_temporal_extent(writer: &mut Writer, start: Option<&str>, end: Option<&str>) {
    if start.is_none() && end.is_none() {
        return;
    }
    writer.key("temporal_extent");
    writer.start_object();
    if let Some(start) = start {
        writer.key("start");
        writer.string(start);
    }
    if let Some(end) = end {
        writer.key("end");
        writer.string(end);
    }
    writer.end_object();
}

fn write_vertical_datum(
    writer: &mut Writer,
    datum: &VerticalReferenceFrame,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.start_object();
    write_vertical_datum_members(writer, datum, db, place)?;
    writer.end_object();
    Ok(())
}

fn write_vertical_datum_members(
    writer: &mut Writer,
    datum: &VerticalReferenceFrame,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    let dynamic = datum.frame_reference_epoch().is_some();
    writer.key("type");
    writer.string(if dynamic {
        "DynamicVerticalReferenceFrame"
    } else {
        "VerticalReferenceFrame"
    });
    write_name(writer, datum.name());
    write_anchor(writer, datum.anchor(), datum.anchor_epoch())?;
    if let Some(epoch) = datum.frame_reference_epoch() {
        writer.key("frame_reference_epoch");
        writer.number(epoch);
    }
    write_base(writer, &datum.identified, place, db);
    Ok(())
}

/// `anchor` and `anchor_epoch` as PROJ writes them. The deformation model of a dynamic datum is
/// written by the CRS, as `deformation_models`.
fn write_anchor(
    writer: &mut Writer,
    anchor: Option<&str>,
    anchor_epoch: Option<f64>,
) -> Result<(), FactoryError> {
    if let Some(anchor) = anchor {
        writer.key("anchor");
        writer.string(anchor);
    }
    if let Some(epoch) = anchor_epoch {
        writer.key("anchor_epoch");
        writer.number(rounded_epoch_in_decimal_year(epoch));
    }
    Ok(())
}

/// `getRoundedEpochInDecimalYear`: snap to 0.001 year when the residual is at most 1e-3.
pub(super) fn rounded_epoch_in_decimal_year(year: f64) -> f64 {
    let scaled = year * 1000.0;
    let nearest = scaled.round();
    if (scaled - nearest).abs() <= 1e-3 {
        nearest / 1000.0
    } else {
        year
    }
}

/// `deformation_models` of a CRS whose datum is dynamic and names a model.
fn write_deformation_models(writer: &mut Writer, model: Option<&str>) {
    if let Some(model) = model {
        writer.key("deformation_models");
        writer.start_array();
        writer.start_object();
        writer.key("name");
        writer.string(model);
        writer.end_object();
        writer.end_array();
    }
}

/// A datum ensemble. PROJ writes its `id` whatever the parent. The ellipsoid of its first member
/// follows the usual `id` rule, with the ensemble above it.
fn write_ensemble(
    writer: &mut Writer,
    ensemble: &DatumEnsemble,
    db: &DatabaseContext,
) -> Result<(), FactoryError> {
    writer.start_object();
    write_ensemble_members(writer, ensemble, db)?;
    writer.end_object();
    Ok(())
}

fn write_ensemble_members(
    writer: &mut Writer,
    ensemble: &DatumEnsemble,
    db: &DatabaseContext,
) -> Result<(), FactoryError> {
    let chain = chain_of(ident(ensemble.identifiers()));
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
    Ok(())
}

/// `geoid_model`, or `geoid_models` for several. A model's `id` is always written. The
/// interpolation CRS is written only for a model with no `id`.
fn write_geoid_models(
    writer: &mut Writer,
    models: &[GeoidModel],
    db: &DatabaseContext,
    parent: Chain,
) -> Result<(), FactoryError> {
    match models {
        [] => {}
        [model] => {
            writer.key("geoid_model");
            write_geoid_model(writer, model, db, parent)?;
        }
        many => {
            writer.key("geoid_models");
            writer.start_array();
            for model in many {
                write_geoid_model(writer, model, db, parent)?;
            }
            writer.end_array();
        }
    }
    Ok(())
}

fn write_geoid_model(
    writer: &mut Writer,
    model: &GeoidModel,
    db: &DatabaseContext,
    parent: Chain,
) -> Result<(), FactoryError> {
    writer.start_object();
    writer.key("name");
    writer.string(model.name());
    if let (true, Some(crs)) = (model.identifiers().is_empty(), &model.interpolation_crs) {
        writer.key("interpolation_crs");
        write_component(writer, crs, db, parent, Allow::Plain)?;
    }
    write_id(writer, model.identifiers(), db);
    writer.end_object();
    Ok(())
}

/// An ellipsoid as PROJ writes it: `radius` for a sphere, else `semi_major_axis` and either the
/// stored `inverse_flattening` or `semi_minor_axis`. An axis that is not in metres is an object
/// with its `value` and `unit`.
fn write_ellipsoid(writer: &mut Writer, ellipsoid: &Ellipsoid, db: &DatabaseContext, place: Place) {
    writer.start_object();
    write_ellipsoid_members(writer, ellipsoid, db, place);
    writer.end_object();
}

impl Ellipsoid {
    /// PROJJSON for this ellipsoid, including `"type": "Ellipsoid"`.
    #[must_use]
    pub fn export_to_json(&self, db: &DatabaseContext, format: &JsonFormat) -> String {
        let mut writer = Writer::new(format);
        writer.start_object();
        if !format.schema.is_empty() {
            writer.key("$schema");
            writer.string(&format.schema);
        }
        writer.key("type");
        writer.string("Ellipsoid");
        write_ellipsoid_members(&mut writer, self, db, Place::top(ident(self.identifiers())));
        writer.end_object();
        writer.finish()
    }
}

fn write_ellipsoid_members(
    writer: &mut Writer,
    ellipsoid: &Ellipsoid,
    db: &DatabaseContext,
    place: Place,
) {
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
    write_meridian_members(writer, meridian, db, place);
    writer.end_object();
}

fn write_meridian_members(
    writer: &mut Writer,
    meridian: &PrimeMeridian,
    db: &DatabaseContext,
    place: Place,
) {
    writer.key("name");
    if meridian.identified.name.is_empty() {
        writer.string("Greenwich");
    } else {
        writer.string(meridian.name());
    }
    writer.key("longitude");
    write_angle(writer, meridian.longitude(), place);
    write_id_if(writer, place, meridian.identifiers(), db);
}

/// A longitude: a bare number in degrees, else `{value, unit}`.
fn write_angle(writer: &mut Writer, angle: &Measure, place: Place) {
    if angle.unit().name() == "degree" {
        writer.number(angle.value());
    } else {
        writer.start_object();
        writer.key("value");
        writer.number(angle.value());
        writer.key("unit");
        write_unit_object(
            writer,
            angle.unit(),
            Place::inside(place.chain, Ident::Lacks, Allow::Plain),
        );
        writer.end_object();
    }
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
        write_axis(writer, axis, parent);
    }
    writer.end_array();
    writer.end_object();
    Ok(())
}

fn write_axis(writer: &mut Writer, axis: &Axis, parent: Chain) {
    writer.start_object();
    write_axis_members(writer, axis, parent);
    writer.end_object();
}

fn write_axis_members(writer: &mut Writer, axis: &Axis, parent: Chain) {
    writer.key("name");
    writer.string(axis.name());
    writer.key("abbreviation");
    writer.string(axis.abbreviation());
    writer.key("direction");
    writer.string(axis.direction());
    if let Some(meridian) = &axis.meridian {
        writer.key("meridian");
        writer.start_object();
        writer.key("longitude");
        write_angle(
            writer,
            meridian.longitude(),
            Place::inside(parent, Ident::Lacks, Allow::Plain),
        );
        writer.end_object();
    }
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
        UnitKind::Parametric => "ParametricUnit",
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

/// A conversion inside a projected CRS: no `type`. A step or a top-level conversion writes
/// `type` through [`write_operation`].
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

/// Name, method, parameters, and `id` of a conversion. An inverse conversion writes the inverse
/// method, as `InverseConversion` builds it. The caller writes `type` when the conversion is an
/// object of its own.
fn write_conversion_members(
    writer: &mut Writer,
    conversion: &Conversion,
    db: &DatabaseContext,
    place: Place,
) {
    write_name(writer, conversion.name());
    writer.key("method");
    let method = if conversion.inverted {
        conversion.method().inverse_of()
    } else {
        conversion.method().clone()
    };
    write_method(writer, &method);
    let parameters = conversion.parameters();
    let interpolation = interpolation_parameter(conversion);
    if !parameters.is_empty() || interpolation.is_some() {
        writer.key("parameters");
        writer.start_array();
        for parameter in parameters {
            write_parameter(writer, parameter, place.chain);
        }
        if let Some(parameter) = &interpolation {
            write_parameter(writer, parameter, place.chain);
        }
        writer.end_array();
    }
    write_id_if(writer, place, conversion.identifiers(), db);
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

/// A transformation, or a point-motion operation when `point_motion` is set.
///
/// The source and target CRS write their `id`. A point-motion operation has no target.
/// `accuracy` is the text PROJ stored, written as a JSON string.
fn write_transformation(
    writer: &mut Writer,
    transformation: &Transformation,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string(if transformation.point_motion {
        "PointMotionOperation"
    } else {
        "Transformation"
    });
    write_name(writer, transformation.name());
    let source = transformation
        .source_crs()
        .ok_or_else(|| FactoryError::Format("transformation has no source CRS".to_owned()))?;
    writer.key("source_crs");
    write_component(writer, source, db, place.chain, Allow::Asked)?;
    if !transformation.point_motion {
        let target = transformation
            .target_crs()
            .ok_or_else(|| FactoryError::Format("transformation has no target CRS".to_owned()))?;
        writer.key("target_crs");
        write_component(writer, target, db, place.chain, Allow::Asked)?;
        if let Some(interpolation) = transformation.interpolation_crs() {
            writer.key("interpolation_crs");
            write_component(writer, interpolation, db, place.chain, Allow::Asked)?;
        }
    }
    writer.key("method");
    let method = if transformation.inverted {
        transformation.method().inverse_of()
    } else {
        transformation.method().clone()
    };
    write_method(writer, &method);
    writer.key("parameters");
    writer.start_array();
    for parameter in transformation.parameters() {
        write_parameter(writer, parameter, place.chain);
    }
    writer.end_array();
    if let Some(text) = transformation.accuracy_text.clone().or_else(|| {
        transformation
            .accuracy()
            .map(|value| super::wkt::format_g(value, 15))
    }) {
        writer.key("accuracy");
        writer.string(&text);
    }
    write_base(writer, &transformation.identified, place, db);
    Ok(())
}

/// A concatenated operation: source and target CRS, steps, accuracy, then usage and `id`.
fn write_concatenated(
    writer: &mut Writer,
    operation: &ConcatenatedOperation,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("ConcatenatedOperation");
    write_name(writer, operation.name());
    let (source, target) = operation
        .crs_pair()
        .ok_or_else(|| FactoryError::Format("concatenated operation has no CRS pair".to_owned()))?;
    writer.key("source_crs");
    write_component(writer, source, db, place.chain, Allow::Asked)?;
    writer.key("target_crs");
    write_component(writer, target, db, place.chain, Allow::Asked)?;
    writer.key("steps");
    writer.start_array();
    for step in operation.steps() {
        write_operation_component(writer, step.operation(), db, place.chain)?;
    }
    writer.end_array();
    if let Some(text) = operation.accuracy_text() {
        writer.key("accuracy");
        writer.string(&text);
    }
    write_base(writer, &operation.identified, place, db);
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
/// Usage of a top object, then its `id` and `remarks`: PROJ's `ObjectUsage::baseExportToJSON`.
fn write_base(writer: &mut Writer, identified: &Identified, place: Place, db: &DatabaseContext) {
    write_usage(
        writer,
        &identified.domains,
        identified.vertical_extent.as_ref(),
        place,
    );
    write_id_if(writer, place, &identified.identifiers, db);
    if !identified.remarks.is_empty() {
        writer.key("remarks");
        writer.string(&identified.remarks);
    }
}

fn write_usage(
    writer: &mut Writer,
    domains: &[ObjectDomain],
    vertical: Option<&VerticalExtent>,
    place: Place,
) {
    if !place.top {
        return;
    }
    match domains {
        [] => {
            if let Some(vertical) = vertical {
                write_vertical_extent(writer, vertical);
            }
        }
        [domain] => {
            write_domain_members(writer, domain);
            if let Some(vertical) = vertical {
                write_vertical_extent(writer, vertical);
            }
        }
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

fn write_vertical_extent(writer: &mut Writer, vertical: &VerticalExtent) {
    writer.key("vertical_extent");
    writer.start_object();
    writer.key("minimum");
    writer.number(vertical.minimum);
    writer.key("maximum");
    writer.number(vertical.maximum);
    if vertical.unit.name() != "metre" {
        writer.key("unit");
        write_unit_object(
            writer,
            &vertical.unit,
            Place::inside(Chain::Clear, Ident::Lacks, Allow::Plain),
        );
    }
    writer.end_object();
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

/// The `id` of an object, or its `ids` when it has several identifiers. Each identifier is written
/// with its authority and, when `proj.db` has one, its version.
fn write_id(writer: &mut Writer, identifiers: &[Identifier], db: &DatabaseContext) {
    let written: Vec<&Identifier> = identifiers
        .iter()
        .filter(|identifier| !identifier.code_space().is_empty() && !identifier.code().is_empty())
        .collect();
    match written.as_slice() {
        [] => {}
        [identifier] => {
            writer.key("id");
            write_identifier(writer, identifier, db);
        }
        several => {
            writer.key("ids");
            writer.start_array();
            for identifier in several {
                write_identifier(writer, identifier, db);
            }
            writer.end_array();
        }
    }
}

fn write_identifier(writer: &mut Writer, identifier: &Identifier, db: &DatabaseContext) {
    let (authority, version) = db
        .authority_and_version(identifier.code_space())
        .map(|(authority, version)| (authority, Some(version)))
        .unwrap_or_else(|| {
            (
                identifier.code_space().to_owned(),
                identifier.version.clone(),
            )
        });
    writer.start_object();
    writer.key("authority");
    writer.string(&authority);
    writer.key("code");
    write_code(writer, identifier.code());
    if let Some(version) = version {
        writer.key("version");
        if version.parse::<f64>().is_ok() {
            writer.raw_token(&version);
        } else {
            writer.string(&version);
        }
    }
    if let Some(citation) = &identifier.authority_citation {
        writer.key("authority_citation");
        writer.string(citation);
    }
    if let Some(uri) = &identifier.uri {
        writer.key("uri");
        writer.string(uri);
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
    create_from_user_input_with_init_rules(text, db, InitRules::Iso19111)
}

/// Whether PROJ's `createFromUserInput` sends `text` to the PROJ string parser
/// (`io.cpp`, the `proj=`, `init=`, `title=` tests).
fn is_proj_string_text(text: &str) -> bool {
    let bare = text.strip_prefix('+').unwrap_or(text);
    bare.starts_with("proj=")
        || bare.starts_with("init=")
        || bare.starts_with("title=")
        || text.contains(" +proj=")
        || text.contains(" proj=")
        || text.contains(" +init=")
        || text.contains(" init=")
}

/// The `usePROJ4InitRules` of PROJ's `createFromUserInput` (`io.cpp`): whether the `init=` of a
/// PROJ string is read as an EPSG or IGNF code in the PROJ 4 way, with longitude and latitude in
/// degrees, and easting and northing for a projected CRS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InitRules {
    /// The ISO 19111 rules: `init=epsg:` and `init=IGNF:` are refused.
    #[default]
    Iso19111,
    /// The PROJ 4 rules: `init=epsg:` and `init=IGNF:` build the CRS of the code.
    Proj4,
}

/// [`create_from_user_input`] with the `init=` rules of `rules`, as PROJ's
/// `createFromUserInput(text, dbContext, usePROJ4InitRules)` (`io.cpp`).
///
/// # Errors
///
/// As [`create_from_user_input`]. An `init=` that is not an EPSG or IGNF code, or that has
/// parameters beside the flags PROJ accepts (`over`, `no_defs`, `wktext`, `type=crs`), is not
/// read: it is an error, not a silent drop.
pub fn create_from_user_input_with_init_rules(
    text: &str,
    db: &DatabaseContext,
    rules: InitRules,
) -> Result<Crs, FactoryError> {
    let text = text.trim();
    if text.starts_with('{') {
        return import_json(text, db);
    }
    if let Some(crs) = crs_from_ogc_auto(text)? {
        return Ok(crs);
    }
    if is_proj_string_text(text) && text.contains("init=") && !has_proj_step(text) {
        return crs_from_init_text(text, db, rules);
    }
    let bare = text.strip_prefix('+').unwrap_or(text);
    if bare.starts_with("proj=") || text.contains(" +proj=") || text.contains(" proj=") {
        return create_crs_from_proj_string_with_database(text, db);
    }
    // An OGC CRS URL contains colons, so it must be tried before `AUTH:CODE`.
    if is_ogc_crs_url(text) {
        return import_ogc_crs_url(text, db);
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

/// Whether a PROJ string has a `proj=` step, not only `init=` and flags.
fn has_proj_step(text: &str) -> bool {
    text.split_whitespace()
        .any(|word| word.strip_prefix('+').unwrap_or(word).starts_with("proj="))
}

/// A PROJ string whose only step is `init=AUTH:CODE`, read as PROJ's `createFromPROJString` does
/// for the `+init=xxxx:yyyy` case (`io.cpp`, lines 12514-12640). Under the PROJ 4 rules the CRS of
/// the code is rebuilt with longitude and latitude in degrees (easting and northing for a
/// projected CRS), named by `title=` and given the `over` flag. Under the ISO 19111 rules the
/// `epsg:` and `IGNF:` forms are refused, as PROJ does.
fn crs_from_init_text(
    text: &str,
    db: &DatabaseContext,
    rules: InitRules,
) -> Result<Crs, FactoryError> {
    let mut title: Option<String> = None;
    let mut init: Option<&str> = None;
    let mut over = false;
    let mut significant = false;
    for word in text.split_whitespace() {
        let word = word.strip_prefix('+').unwrap_or(word);
        if let Some(value) = word.strip_prefix("title=") {
            title = Some(value.trim_matches('"').to_owned());
        } else if let Some(name) = word.strip_prefix("init=") {
            init = Some(name);
        } else if word == "over" {
            over = true;
        } else if !matches!(word, "no_defs" | "wktext" | "type=crs") {
            significant = true;
        }
    }
    let name = init.ok_or_else(|| FactoryError::Format(format!("{text}: no init= step")))?;
    let (authority, code) = name
        .split_once(':')
        .ok_or_else(|| FactoryError::Format(format!("init={name}: no authority")))?;
    let authority = authority.to_ascii_uppercase();
    if !matches!(authority.as_str(), "EPSG" | "IGNF") {
        return Err(FactoryError::Unsupported(format!(
            "init={name}: only the EPSG and IGNF codes are read (no init file)"
        )));
    }
    if rules == InitRules::Iso19111 {
        return Err(FactoryError::Format(
            "init=epsg:/init=IGNF: syntax not supported in non-PROJ4 emulation mode".to_owned(),
        ));
    }
    if significant {
        return Err(FactoryError::Unsupported(format!(
            "init={name} with parameters beside over, no_defs, wktext and type=crs is not expanded"
        )));
    }
    let crs = create_from_user_input(&format!("{authority}:{code}"), db)?;
    match crs {
        Crs::Geographic(geographic) => {
            let cs = match geographic.coordinate_system().axes() {
                [_, _, height] => EllipsoidalCS::create_longitude_latitude_height(
                    UnitOfMeasure::degree(),
                    height.unit().clone(),
                ),
                _ => EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
            };
            let name = title.unwrap_or_else(|| geographic.identified.name.clone());
            let mut rebuilt = GeographicCRS::create(
                name,
                geographic.datum().cloned(),
                geographic.datum_ensemble().cloned(),
                cs,
            )?;
            rebuilt.identified.domains = geographic.identified.domains;
            rebuilt.identified.over = over;
            Ok(Crs::Geographic(rebuilt))
        }
        Crs::Projected(projected) => {
            // PROJ keeps the CRS of a Transverse Mercator South Orientated projection as it is.
            let south_oriented = projected.deriving_conversion().method().name()
                == "Transverse Mercator (South Orientated)";
            let Some(first) = projected.coordinate_system().axes().first() else {
                return Ok(Crs::Projected(projected));
            };
            if south_oriented {
                return Ok(Crs::Projected(projected));
            }
            let name = title.unwrap_or_else(|| projected.identified.name.clone());
            let mut rebuilt = ProjectedCRS::create(
                name,
                projected.base_crs().clone(),
                projected.deriving_conversion().clone(),
                CartesianCS::create_easting_northing(first.unit().clone()),
            );
            rebuilt.identified.domains = projected.identified.domains;
            rebuilt.identified.over = over;
            Ok(Crs::Projected(rebuilt))
        }
        other => Ok(other),
    }
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
        // PROJ retries EPSG:102100 as ESRI:102100. The identifiers stay ESRI.
        if tokens[0].eq_ignore_ascii_case("EPSG") && tokens[1] == "102100" {
            if let Ok(crs) =
                AuthorityFactory::create(db, "ESRI").create_coordinate_reference_system("102100")
            {
                return Ok(Some(crs));
            }
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

const OGC_CRS_URL_PREFIXES: &[&str] = &[
    "https://www.opengis.net/def/crs",
    "http://www.opengis.net/def/crs",
    "https://opengis.net/def/crs",
    "http://opengis.net/def/crs",
    "www.opengis.net/def/crs",
];

fn is_ogc_crs_url(text: &str) -> bool {
    OGC_CRS_URL_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

/// OGC Best Practice 11-135: `http://www.opengis.net/def/crs/AUTH/version/code`
/// and a `crs-compound` query of those URLs.
fn import_ogc_crs_url(text: &str, db: &DatabaseContext) -> Result<Crs, FactoryError> {
    let Some(prefix) = OGC_CRS_URL_PREFIXES
        .iter()
        .find(|prefix| text.starts_with(*prefix))
    else {
        return Err(FactoryError::Invalid("invalid OGC CRS URL".to_owned()));
    };
    let rest = &text[prefix.len()..];
    let parts: Vec<&str> = rest.split('/').collect();
    if parts
        .first()
        .is_some_and(|part| part.starts_with("-compound?"))
    {
        let query = text.split_once('?').map(|(_, query)| query).unwrap_or("");
        let mut keyed = std::collections::BTreeMap::new();
        for part in query.split('&') {
            let Some((index, url)) = part.split_once('=') else {
                return Err(FactoryError::Invalid("invalid OGC CRS URL".to_owned()));
            };
            let index = index
                .parse::<usize>()
                .map_err(|_| FactoryError::Invalid("invalid OGC CRS URL".to_owned()))?;
            keyed.insert(index, url);
        }
        let mut components = Vec::new();
        let mut name = String::new();
        for index in 1..=keyed.len() {
            let Some(url) = keyed.get(&index) else {
                return Err(FactoryError::Invalid("invalid OGC CRS URL".to_owned()));
            };
            let component = import_ogc_crs_url(url, db)?;
            if !name.is_empty() {
                name.push_str(" + ");
            }
            name.push_str(component.name());
            components.push(component);
        }
        return Ok(Crs::Compound(CompoundCRS::create(name, components)?));
    }
    if parts.len() < 4 {
        return Err(FactoryError::Invalid("invalid OGC CRS URL".to_owned()));
    }
    authority_crs(db, parts[1], parts[2], parts[3])
}

fn authority_crs(
    db: &DatabaseContext,
    authority: &str,
    version: &str,
    code: &str,
) -> Result<Crs, FactoryError> {
    match AuthorityFactory::create(db, authority).create_coordinate_reference_system(code) {
        Ok(crs) => Ok(crs),
        Err(error) => {
            if version.is_empty() || version == "0" {
                for versioned in db.versioned_authorities_from_name(authority) {
                    if let Ok(crs) = AuthorityFactory::create(db, &versioned)
                        .create_coordinate_reference_system(code)
                    {
                        return Ok(crs);
                    }
                }
                return Err(error);
            }
            let Some(versioned) = db.versioned_authority(authority, version) else {
                return Err(error);
            };
            AuthorityFactory::create(db, &versioned).create_coordinate_reference_system(code)
        }
    }
}

fn import_json(text: &str, db: &DatabaseContext) -> Result<Crs, FactoryError> {
    // Strict PROJJSON first. A datum at the root is not a CRS. A projected CRS that
    // omits `type` on its datum and `coordinate_system` fails both importers and is
    // still a CRS through the lenient reader.
    if let Ok(JsonObject::Crs(crs)) = create_object_from_json(text, Some(db)) {
        return Ok(crs);
    }
    match import_user_object(text, db) {
        Ok(UserObject::Crs(crs)) => Ok(crs),
        Ok(_) => Err(FactoryError::Invalid(
            "PROJJSON object is not a CRS".to_owned(),
        )),
        Err(error) => lenient_projected_json(text, db).map_err(|_| error),
    }
}

/// Projected PROJJSON with omitted names, units, and coordinate systems.
///
/// `create_object_from_json` stays strict. This runs only after that import fails.
fn lenient_projected_json(text: &str, db: &DatabaseContext) -> Result<Crs, FactoryError> {
    let value = crs_text::parse_json(text)
        .map_err(|error| FactoryError::Invalid(format!("PROJJSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| FactoryError::Invalid("PROJJSON root is not an object".to_owned()))?;
    if required_str(object, "type")? != "ProjectedCRS" {
        return Err(FactoryError::Invalid(
            "PROJJSON type is not a projected CRS".to_owned(),
        ));
    }
    read_projected(object, db)
}

/// A projected CRS. The base is geographic. A missing `type` on the base is still geographic.
/// Names may be absent; they are stored as `unnamed`.
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
        interpolation: None,
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
        UnitKind::Time | UnitKind::None | UnitKind::Unknown | UnitKind::Parametric => {
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
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => Err(
            FactoryError::Invalid("PROJJSON unit is not a string or an object".to_owned()),
        ),
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
    Ok(UnitOfMeasure::new(
        name,
        factor.unwrap_or(1.0),
        UnitKind::Unknown,
    ))
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
    if let Some(extent) = object.get("temporal_extent").and_then(Value::as_object) {
        identified.temporal_start = extent
            .get("start")
            .and_then(Value::as_str)
            .map(str::to_owned);
        identified.temporal_end = extent.get("end").and_then(Value::as_str).map(str::to_owned);
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
        frames.push(Datum::Geodetic(read_datum(member, db)?));
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
    if name.eq_ignore_ascii_case("degree") {
        return Ok(UnitOfMeasure::degree());
    }
    if name.eq_ignore_ascii_case("metre") || name.eq_ignore_ascii_case("meter") {
        return Ok(UnitOfMeasure::metre());
    }
    if name.eq_ignore_ascii_case("unity") {
        return Ok(UnitOfMeasure::unity());
    }
    let factor = object
        .get("conversion_factor")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .map(unit_kind_name)
        .unwrap_or(UnitKind::Unknown);
    Ok(UnitOfMeasure {
        name: name.to_owned(),
        factor,
        kind,
        code_space: String::new(),
        code: String::new(),
    })
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

/// The members of the datum ensemble that `name` approximately matches in `proj.db`.
pub(super) fn ensemble_members_by_name(
    db: &DatabaseContext,
    name: &str,
) -> Result<Vec<Datum>, FactoryError> {
    let found = AuthorityFactory::create(db, "").create_objects_from_name(
        name,
        &[ObjectType::DatumEnsemble],
        NameMatch::Approximate,
        1,
    )?;
    Ok(match found.into_iter().next() {
        Some(AuthorityObject::DatumEnsemble(ensemble)) => ensemble.members().to_vec(),
        _ => Vec::new(),
    })
}

/// The datum named exactly `name` in `proj.db`, when there is one.
pub(super) fn datum_by_name(
    db: &DatabaseContext,
    name: &str,
) -> Result<Option<Datum>, FactoryError> {
    let found = AuthorityFactory::create(db, "").create_objects_from_name(
        name,
        &[ObjectType::Datum],
        NameMatch::Exact,
        1,
    )?;
    Ok(found.into_iter().find_map(|object| match object {
        AuthorityObject::Datum(datum) => Some(datum),
        _ => None,
    }))
}

fn invalid(text: impl Into<String>) -> FactoryError {
    FactoryError::Invalid(text.into())
}

fn unit_kind_name(type_name: &str) -> UnitKind {
    match type_name {
        "LinearUnit" => UnitKind::Linear,
        "AngularUnit" => UnitKind::Angular,
        "ScaleUnit" => UnitKind::Scale,
        "TimeUnit" => UnitKind::Time,
        "ParametricUnit" => UnitKind::Parametric,
        _ => UnitKind::Unknown,
    }
}

/// A CRS, or a datum PROJJSON can name at the root. `createFromUserInput` in PROJ returns either.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "a geodetic datum carries an ellipsoid; the other objects do not"
)]
pub enum UserObject {
    /// A coordinate reference system.
    Crs(Crs),
    /// A geodetic reference frame, including a dynamic one.
    GeodeticDatum(GeodeticReferenceFrame),
    /// A parametric datum.
    ParametricDatum(ParametricDatum),
    /// An engineering datum.
    EngineeringDatum(EngineeringDatum),
    /// A temporal datum.
    TemporalDatum(TemporalDatum),
}

impl UserObject {
    /// PROJJSON for this object in `format`.
    ///
    /// # Errors
    ///
    /// A part of the object has no PROJJSON export in this build.
    pub fn export_to_json(
        &self,
        db: &DatabaseContext,
        format: &JsonFormat,
    ) -> Result<String, FactoryError> {
        match self {
            Self::Crs(crs) => crs.export_to_json_with(db, format),
            Self::GeodeticDatum(datum) => export_wrapped(format, |writer| {
                write_datum_fields(writer, datum, db, Place::top(ident(datum.identifiers())))
            }),
            Self::ParametricDatum(datum) => export_plain(
                format,
                &PlainDatum {
                    type_name: Some("ParametricDatum"),
                    name: datum.name(),
                    calendar: None,
                    origin: None,
                    anchor: datum.anchor_definition(),
                    identifiers: datum.identifiers(),
                },
                db,
            ),
            Self::EngineeringDatum(datum) => export_plain(
                format,
                &PlainDatum {
                    type_name: Some("EngineeringDatum"),
                    name: datum.name(),
                    calendar: None,
                    origin: None,
                    anchor: datum.anchor_definition(),
                    identifiers: datum.identifiers(),
                },
                db,
            ),
            Self::TemporalDatum(datum) => export_plain(
                format,
                &PlainDatum {
                    type_name: Some("TemporalDatum"),
                    name: datum.name(),
                    calendar: Some(datum.calendar()),
                    origin: Some(datum.temporal_origin()),
                    anchor: None,
                    identifiers: datum.identifiers(),
                },
                db,
            ),
        }
    }
}

/// `createFromUserInput` for a CRS or a root datum.
///
/// JSON whose `type` is a datum returns that datum. Every other accepted text is a CRS, the same
/// objects [`create_from_user_input`] builds.
///
/// # Errors
///
/// The text is not a CRS or a datum this importer accepts.
pub fn create_object_from_user_input(
    text: &str,
    db: &DatabaseContext,
) -> Result<UserObject, FactoryError> {
    let text = text.trim();
    if text.starts_with('{') {
        return import_user_object(text, db);
    }
    Ok(UserObject::Crs(create_from_user_input(text, db)?))
}

fn export_wrapped(
    format: &JsonFormat,
    write: impl FnOnce(&mut Writer) -> Result<(), FactoryError>,
) -> Result<String, FactoryError> {
    let mut writer = Writer::new(format);
    writer.start_object();
    if !format.schema.is_empty() {
        writer.key("$schema");
        writer.string(&format.schema);
    }
    write(&mut writer)?;
    writer.end_object();
    Ok(writer.finish())
}

/// Fields of a parametric, engineering, or temporal datum. Nested datums omit `type`.
struct PlainDatum<'a> {
    type_name: Option<&'a str>,
    name: &'a str,
    calendar: Option<&'a str>,
    origin: Option<&'a str>,
    anchor: Option<&'a str>,
    identifiers: &'a [Identifier],
}

fn export_plain(
    format: &JsonFormat,
    datum: &PlainDatum<'_>,
    db: &DatabaseContext,
) -> Result<String, FactoryError> {
    export_wrapped(format, |writer| {
        write_plain_datum_fields(writer, datum, db, Place::top(ident(datum.identifiers)))
    })
}

fn write_plain_datum_fields(
    writer: &mut Writer,
    datum: &PlainDatum<'_>,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    if let Some(type_name) = datum.type_name {
        writer.key("type");
        writer.string(type_name);
    }
    write_name(writer, datum.name);
    if let Some(calendar) = datum.calendar.filter(|text| !text.is_empty()) {
        writer.key("calendar");
        writer.string(calendar);
    }
    if let Some(origin) = datum.origin.filter(|text| !text.is_empty()) {
        writer.key("time_origin");
        writer.string(origin);
    }
    write_anchor(writer, datum.anchor, None)?;
    write_id_if(writer, place, datum.identifiers, db);
    Ok(())
}

fn write_plain_datum_object(
    writer: &mut Writer,
    datum: &PlainDatum<'_>,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.start_object();
    write_plain_datum_fields(writer, datum, db, place)?;
    writer.end_object();
    Ok(())
}

fn write_engineering(
    writer: &mut Writer,
    crs: &EngineeringCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("EngineeringCRS");
    write_name(writer, crs.name());
    writer.key("datum");
    let datum = crs.datum();
    write_plain_datum_object(
        writer,
        &PlainDatum {
            type_name: None,
            name: datum.name(),
            calendar: None,
            origin: None,
            anchor: datum.anchor_definition(),
            identifiers: datum.identifiers(),
        },
        db,
        Place::inside(place.chain, ident(datum.identifiers()), Allow::Plain),
    )?;
    writer.key("coordinate_system");
    write_system(writer, crs.coordinate_system(), place.chain)?;
    write_usage(writer, crs.domains(), None, place);
    write_id_if(writer, place, crs.identifiers(), db);
    Ok(())
}

fn write_temporal(
    writer: &mut Writer,
    crs: &TemporalCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("TemporalCRS");
    write_name(writer, crs.name());
    writer.key("datum");
    let datum = crs.datum();
    write_plain_datum_object(
        writer,
        &PlainDatum {
            type_name: None,
            name: datum.name(),
            calendar: Some(datum.calendar()),
            origin: Some(datum.temporal_origin()),
            anchor: None,
            identifiers: datum.identifiers(),
        },
        db,
        Place::inside(place.chain, ident(datum.identifiers()), Allow::Plain),
    )?;
    writer.key("coordinate_system");
    let subtype = match crs.coordinate_system().kind() {
        TemporalCsKind::DateTime => "TemporalDateTime",
        TemporalCsKind::Count => "TemporalCount",
        TemporalCsKind::Measure => "TemporalMeasure",
    };
    write_cs(
        writer,
        subtype,
        std::slice::from_ref(crs.coordinate_system().axis()),
        place.chain,
    )?;
    write_usage(writer, crs.domains(), None, place);
    write_id_if(writer, place, crs.identifiers(), db);
    Ok(())
}

fn write_parametric(
    writer: &mut Writer,
    crs: &ParametricCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("ParametricCRS");
    write_name(writer, crs.name());
    writer.key("datum");
    let datum = crs.datum();
    write_plain_datum_object(
        writer,
        &PlainDatum {
            type_name: None,
            name: datum.name(),
            calendar: None,
            origin: None,
            anchor: datum.anchor_definition(),
            identifiers: datum.identifiers(),
        },
        db,
        Place::inside(place.chain, ident(datum.identifiers()), Allow::Plain),
    )?;
    writer.key("coordinate_system");
    write_cs(
        writer,
        "parametric",
        std::slice::from_ref(crs.coordinate_system().axis()),
        place.chain,
    )?;
    write_usage(writer, crs.domains(), None, place);
    write_id_if(writer, place, crs.identifiers(), db);
    Ok(())
}

fn write_derived(
    writer: &mut Writer,
    crs: &DerivedCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string(derived_json_type(crs));
    write_name(writer, crs.name());
    writer.key("base_crs");
    write_component(writer, crs.base_crs(), db, place.chain, Allow::Asked)?;
    write_deriving_conversion(writer, crs.deriving_conversion(), db, place)?;
    writer.key("coordinate_system");
    write_system(writer, crs.coordinate_system(), place.chain)?;
    write_usage(writer, &crs.identified.domains, None, place);
    write_id_if(writer, place, crs.identifiers(), db);
    Ok(())
}

fn write_derived_projected(
    writer: &mut Writer,
    crs: &DerivedProjectedCRS,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("DerivedProjectedCRS");
    write_name(writer, crs.name());
    writer.key("base_crs");
    writer.start_object();
    let base = crs.base_crs();
    let base_place = Place::inside(place.chain, ident(base.identifiers()), Allow::Asked);
    write_projected(writer, base, db, base_place)?;
    writer.end_object();
    write_deriving_conversion(writer, crs.deriving_conversion(), db, place)?;
    writer.key("coordinate_system");
    write_system(writer, crs.coordinate_system(), place.chain)?;
    write_usage(writer, &crs.identified.domains, None, place);
    write_id_if(writer, place, &crs.identified.identifiers, db);
    Ok(())
}

fn write_deriving_conversion(
    writer: &mut Writer,
    conversion: &Conversion,
    db: &DatabaseContext,
    place: Place,
) -> Result<(), FactoryError> {
    writer.key("conversion");
    let conversion_place =
        Place::inside(place.chain, ident(conversion.identifiers()), Allow::Plain);
    write_conversion(writer, conversion, db, conversion_place)
}

fn write_system(
    writer: &mut Writer,
    system: &CoordinateSystem,
    parent: Chain,
) -> Result<(), FactoryError> {
    let owned: Vec<Axis> = system.axes().into_iter().cloned().collect();
    write_cs(writer, system.wkt2_type(), &owned, parent)
}

fn derived_json_type(crs: &DerivedCRS) -> &'static str {
    match crs.base_crs() {
        Crs::Vertical(_) => "DerivedVerticalCRS",
        Crs::Engineering(_) => "DerivedEngineeringCRS",
        Crs::Parametric(_) => "DerivedParametricCRS",
        Crs::Temporal(_) => "DerivedTemporalCRS",
        Crs::Geographic(_) | Crs::Geodetic(_)
            if matches!(crs.coordinate_system(), CoordinateSystem::Ellipsoidal(_)) =>
        {
            "DerivedGeographicCRS"
        }
        _ => "DerivedGeodeticCRS",
    }
}

/// The EPSG interpolation-code parameter PROJ re-emits from [`Conversion::interpolation_crs`].
fn interpolation_parameter(conversion: &Conversion) -> Option<Parameter> {
    let crs = conversion.interpolation_crs()?;
    let already = conversion.parameters().iter().any(|parameter| {
        parameter.code_space() == "EPSG" && matches!(parameter.code(), "1048" | "1037")
    });
    if already {
        return None;
    }
    let code = crs
        .authority_code("EPSG")
        .and_then(|identifier| identifier.code().parse::<i64>().ok())?;
    let (name, parameter_code) = if conversion.method().code() == "1046" {
        ("EPSG code for Horizontal CRS", "1037")
    } else {
        ("EPSG code for Interpolation CRS", "1048")
    };
    Some(Parameter {
        name: name.to_owned(),
        code_space: "EPSG".to_owned(),
        code: parameter_code.to_owned(),
        value: ParameterValue::Integer(code),
    })
}

fn import_user_object(text: &str, db: &DatabaseContext) -> Result<UserObject, FactoryError> {
    let value = crs_text::parse_json(text).map_err(FactoryError::Invalid)?;
    let Some(object) = value.as_object() else {
        return Err(invalid("PROJJSON value is not an object"));
    };
    let type_name = required_str(object, "type")?;
    if let Some(crs) = read_crs_of_type(type_name, object, db)? {
        return Ok(UserObject::Crs(crs));
    }
    match type_name {
        "GeodeticReferenceFrame" | "DynamicGeodeticReferenceFrame" => {
            Ok(UserObject::GeodeticDatum(read_datum(object, db)?))
        }
        "ParametricDatum" => Ok(UserObject::ParametricDatum(read_parametric_datum(
            object, db,
        )?)),
        "EngineeringDatum" => Ok(UserObject::EngineeringDatum(read_engineering_datum(
            object, db,
        )?)),
        "TemporalDatum" => Ok(UserObject::TemporalDatum(read_temporal_datum(object, db)?)),
        other => Err(invalid(format!("PROJJSON type {other} is not imported"))),
    }
}

fn read_crs_object(object: &Object, db: &DatabaseContext) -> Result<Crs, FactoryError> {
    let type_name = required_str(object, "type")?;
    read_crs_of_type(type_name, object, db)?
        .ok_or_else(|| invalid(format!("PROJJSON type {type_name} is not a CRS")))
}

fn read_crs_of_type(
    type_name: &str,
    object: &Object,
    db: &DatabaseContext,
) -> Result<Option<Crs>, FactoryError> {
    let crs = match type_name {
        "GeographicCRS" => Crs::Geographic(read_geographic(object, db)?),
        "ProjectedCRS" => Crs::Projected(read_projected_crs(object, db)?),
        "VerticalCRS" => Crs::Vertical(read_vertical_crs(object, db)?),
        "EngineeringCRS" => Crs::Engineering(read_engineering_crs(object, db)?),
        "ParametricCRS" => Crs::Parametric(read_parametric_crs(object, db)?),
        "TemporalCRS" => Crs::Temporal(read_temporal_crs(object, db)?),
        "DerivedProjectedCRS" => Crs::DerivedProjected(read_derived_projected(object, db)?),
        "DerivedGeodeticCRS"
        | "DerivedGeographicCRS"
        | "DerivedVerticalCRS"
        | "DerivedEngineeringCRS"
        | "DerivedParametricCRS"
        | "DerivedTemporalCRS" => Crs::Derived(read_derived(object, db)?),
        _ => return Ok(None),
    };
    Ok(Some(crs))
}

fn read_name_id(object: &Object, db: &DatabaseContext) -> Result<Identified, FactoryError> {
    let mut identified = Identified::named(required_str(object, "name")?);
    if let Some(identifier) = object
        .get("id")
        .and_then(Value::as_object)
        .map(|id| read_id(id, db))
        .transpose()?
        .flatten()
    {
        identified.identifiers.push(identifier);
    }
    Ok(identified)
}

fn read_datum_identified(
    object: &Object,
    db: &DatabaseContext,
) -> Result<Identified, FactoryError> {
    let mut identified = read_name_id(object, db)?;
    if let Some(anchor) = object.get("anchor").and_then(Value::as_str) {
        identified.anchor = Some(anchor.to_owned());
    }
    Ok(identified)
}

fn read_engineering_datum(
    object: &Object,
    db: &DatabaseContext,
) -> Result<EngineeringDatum, FactoryError> {
    Ok(EngineeringDatum {
        identified: read_datum_identified(object, db)?,
    })
}

fn read_parametric_datum(
    object: &Object,
    db: &DatabaseContext,
) -> Result<ParametricDatum, FactoryError> {
    Ok(ParametricDatum {
        identified: read_datum_identified(object, db)?,
    })
}

fn read_temporal_datum(
    object: &Object,
    db: &DatabaseContext,
) -> Result<TemporalDatum, FactoryError> {
    Ok(TemporalDatum {
        identified: read_datum_identified(object, db)?,
        calendar: object
            .get("calendar")
            .and_then(Value::as_str)
            .unwrap_or(PROLEPTIC_GREGORIAN)
            .to_owned(),
        origin: object
            .get("time_origin")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
    })
}

fn child_object<'a>(object: &'a Object, key: &str) -> Result<&'a Object, FactoryError> {
    object
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| invalid(format!("PROJJSON object is missing {key}")))
}

fn read_engineering_crs(
    object: &Object,
    db: &DatabaseContext,
) -> Result<EngineeringCRS, FactoryError> {
    Ok(EngineeringCRS {
        identified: read_name_id(object, db)?,
        datum: read_engineering_datum(child_object(object, "datum")?, db)?,
        coordinate_system: read_cs_object(object)?,
    })
}

fn read_parametric_crs(
    object: &Object,
    db: &DatabaseContext,
) -> Result<ParametricCRS, FactoryError> {
    let system = match read_cs_object(object)? {
        CoordinateSystem::Parametric(system) => system,
        _ => return Err(invalid("PROJJSON parametric CRS axis is not parametric")),
    };
    Ok(ParametricCRS {
        identified: read_name_id(object, db)?,
        datum: read_parametric_datum(child_object(object, "datum")?, db)?,
        coordinate_system: system,
    })
}

fn read_temporal_crs(object: &Object, db: &DatabaseContext) -> Result<TemporalCRS, FactoryError> {
    let system = match read_cs_object(object)? {
        CoordinateSystem::Temporal(system) => system,
        _ => return Err(invalid("PROJJSON temporal CRS axis is not temporal")),
    };
    Ok(TemporalCRS {
        identified: read_name_id(object, db)?,
        datum: read_temporal_datum(child_object(object, "datum")?, db)?,
        coordinate_system: system,
    })
}

fn read_vertical_crs(object: &Object, db: &DatabaseContext) -> Result<VerticalCRS, FactoryError> {
    let system = match read_cs_object(object)? {
        CoordinateSystem::Vertical(system) => system,
        _ => return Err(invalid("PROJJSON vertical CRS axis is not vertical")),
    };
    let datum = object
        .get("datum")
        .and_then(Value::as_object)
        .map(|datum| read_vertical_frame(datum, db))
        .transpose()?;
    Ok(VerticalCRS {
        identified: read_name_id(object, db)?,
        datum,
        ensemble: None,
        coordinate_system: system,
        geoid_models: Vec::new(),
    })
}

fn read_vertical_frame(
    object: &Object,
    db: &DatabaseContext,
) -> Result<VerticalReferenceFrame, FactoryError> {
    Ok(VerticalReferenceFrame {
        realization_method: None,
        identified: read_datum_identified(object, db)?,
        wkt1_type: "2005".to_owned(),
        frame_epoch: None,
        deformation_model: None,
    })
}

fn read_projected_crs(object: &Object, db: &DatabaseContext) -> Result<ProjectedCRS, FactoryError> {
    let base = match read_crs_object(child_object(object, "base_crs")?, db)? {
        Crs::Geographic(crs) => GeodeticCRS::Geographic(crs),
        Crs::Geodetic(crs) => crs,
        _ => return Err(invalid("PROJJSON projected CRS base is not geodetic")),
    };
    let coordinate_system = match read_cs_object(object)? {
        CoordinateSystem::Cartesian(system) => system,
        _ => {
            return Err(invalid(
                "PROJJSON projected CRS coordinate system is not cartesian",
            ))
        }
    };
    Ok(ProjectedCRS {
        identified: read_name_id(object, db)?,
        base,
        conversion: read_user_conversion(child_object(object, "conversion")?, db)?,
        coordinate_system,
        implicit_cs: false,
    })
}

fn read_derived(object: &Object, db: &DatabaseContext) -> Result<DerivedCRS, FactoryError> {
    Ok(DerivedCRS {
        identified: read_name_id(object, db)?,
        base: Box::new(read_crs_object(child_object(object, "base_crs")?, db)?),
        conversion: read_user_conversion(child_object(object, "conversion")?, db)?,
        coordinate_system: read_cs_object(object)?,
    })
}

fn read_derived_projected(
    object: &Object,
    db: &DatabaseContext,
) -> Result<DerivedProjectedCRS, FactoryError> {
    let base = match read_crs_object(child_object(object, "base_crs")?, db)? {
        Crs::Projected(crs) => crs,
        _ => {
            return Err(invalid(
                "PROJJSON derived projected CRS base is not projected",
            ))
        }
    };
    Ok(DerivedProjectedCRS {
        identified: read_name_id(object, db)?,
        base,
        conversion: read_user_conversion(child_object(object, "conversion")?, db)?,
        coordinate_system: read_cs_object(object)?,
    })
}

fn read_user_conversion(object: &Object, db: &DatabaseContext) -> Result<Conversion, FactoryError> {
    let method = child_object(object, "method")?;
    let (code_space, code) = id_parts(method, db)?;
    let mut parameters = Vec::new();
    if let Some(items) = object.get("parameters").and_then(Value::as_array) {
        for item in items {
            let Some(item) = item.as_object() else {
                return Err(invalid("PROJJSON parameter is not an object"));
            };
            parameters.push(read_parameter(item, db)?);
        }
    }
    let interpolation = take_json_interpolation(&mut parameters, db);
    Ok(Conversion {
        identified: read_name_id(object, db)?,
        method: OperationMethod::named_code(
            required_str(method, "name")?.to_owned(),
            code_space,
            code,
        ),
        parameters,
        source: None,
        target: None,
        version: None,
        inverted: false,
        interpolation,
    })
}

fn read_parameter(object: &Object, db: &DatabaseContext) -> Result<Parameter, FactoryError> {
    let value = object.get("value").and_then(Value::as_f64).ok_or_else(|| {
        invalid(format!(
            "PROJJSON parameter {} has no numeric value",
            object
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ))
    })?;
    let (code_space, code) = id_parts(object, db)?;
    let stored = if let Some(unit) = object.get("unit") {
        ParameterValue::Measure(Measure {
            value,
            unit: read_unit_value(unit)?,
        })
    } else if let Some(integer) = finite_integer(value) {
        ParameterValue::Integer(integer)
    } else {
        ParameterValue::Measure(Measure {
            value,
            unit: UnitOfMeasure::unity(),
        })
    };
    Ok(Parameter {
        name: required_str(object, "name")?.to_owned(),
        code_space,
        code,
        value: stored,
    })
}

/// An integer-valued finite float, parsed from its shortest decimal form so the conversion is checked.
fn finite_integer<T: std::str::FromStr>(value: f64) -> Option<T> {
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    format!("{value:.0}").parse().ok()
}

fn id_parts(object: &Object, db: &DatabaseContext) -> Result<(String, String), FactoryError> {
    let Some(id) = object.get("id").and_then(Value::as_object) else {
        return Ok((String::new(), String::new()));
    };
    match read_id(id, db)? {
        Some(identifier) => Ok((
            identifier.code_space().to_owned(),
            identifier.code().to_owned(),
        )),
        None => Ok((String::new(), String::new())),
    }
}

/// Drop an EPSG interpolation-code parameter and keep the geographic CRS it names.
fn take_json_interpolation(
    parameters: &mut Vec<Parameter>,
    db: &DatabaseContext,
) -> Option<Box<Crs>> {
    let index = parameters.iter().position(|parameter| {
        let named = matches!(
            parameter.name(),
            "EPSG code for Interpolation CRS" | "EPSG code for Horizontal CRS"
        );
        let coded = parameter.code_space() == "EPSG" && matches!(parameter.code(), "1048" | "1037");
        named || coded
    })?;
    let ParameterValue::Integer(code) = parameters[index].value() else {
        return None;
    };
    let crs = AuthorityFactory::create(db, "EPSG")
        .create_geographic_crs(&code.to_string())
        .ok()?;
    parameters.remove(index);
    Some(Box::new(Crs::Geographic(crs)))
}

fn read_cs_object(object: &Object) -> Result<CoordinateSystem, FactoryError> {
    read_coordinate_system(child_object(object, "coordinate_system")?)
}

fn read_coordinate_system(object: &Object) -> Result<CoordinateSystem, FactoryError> {
    let subtype = required_str(object, "subtype")?;
    let axes = read_axes(object)?;
    match subtype.to_ascii_lowercase().as_str() {
        "ellipsoidal" => {
            if axes.len() != 2 && axes.len() != 3 {
                return Err(invalid("an ellipsoidal coordinate system has 2 or 3 axes"));
            }
            Ok(CoordinateSystem::Ellipsoidal(EllipsoidalCS {
                identified: Identified::default(),
                axes,
            }))
        }
        "cartesian" => Ok(CoordinateSystem::Cartesian(CartesianCS::create(axes)?)),
        "affine" => Ok(CoordinateSystem::Affine(
            CartesianCS::create_allowing_mixed_units(axes)?,
        )),
        "vertical" => Ok(CoordinateSystem::Vertical(VerticalCS {
            identified: Identified::default(),
            axis: one_axis(axes)?,
        })),
        "parametric" => Ok(CoordinateSystem::Parametric(ParametricCS {
            identified: Identified::default(),
            axis: one_axis(axes)?,
        })),
        "temporaldatetime" => Ok(CoordinateSystem::Temporal(temporal_cs(
            axes,
            TemporalCsKind::DateTime,
        )?)),
        "temporalcount" => Ok(CoordinateSystem::Temporal(temporal_cs(
            axes,
            TemporalCsKind::Count,
        )?)),
        "temporalmeasure" => Ok(CoordinateSystem::Temporal(temporal_cs(
            axes,
            TemporalCsKind::Measure,
        )?)),
        _ => Err(invalid(format!(
            "PROJJSON coordinate system subtype {subtype} is not imported"
        ))),
    }
}

fn temporal_cs(axes: Vec<Axis>, kind: TemporalCsKind) -> Result<TemporalCS, FactoryError> {
    Ok(TemporalCS {
        identified: Identified::default(),
        axis: one_axis(axes)?,
        kind,
    })
}

fn one_axis(mut axes: Vec<Axis>) -> Result<Axis, FactoryError> {
    if axes.len() == 1 {
        return Ok(axes.remove(0));
    }
    Err(invalid("this coordinate system has one axis"))
}

fn read_axes(object: &Object) -> Result<Vec<Axis>, FactoryError> {
    let Some(items) = object.get("axis").and_then(Value::as_array) else {
        return Err(invalid("PROJJSON coordinate system has no axis"));
    };
    let mut axes = Vec::new();
    for item in items {
        let Some(axis) = item.as_object() else {
            return Err(invalid("PROJJSON axis is not an object"));
        };
        axes.push(read_json_axis(axis)?);
    }
    Ok(axes)
}

fn read_json_axis(object: &Object) -> Result<Axis, FactoryError> {
    let name = object.get("name").and_then(Value::as_str).unwrap_or("axis");
    let abbreviation = object
        .get("abbreviation")
        .and_then(Value::as_str)
        .unwrap_or("");
    let direction = object
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("unspecified");
    let unit = match object.get("unit") {
        Some(value) => read_unit_value(value)?,
        None => UnitOfMeasure {
            name: String::new(),
            factor: 1.0,
            kind: UnitKind::None,
            code_space: String::new(),
            code: String::new(),
        },
    };
    Ok(Axis::new(name, abbreviation, direction, unit))
}

/// `AUTO:` and `urn:ogc:def:crs:OGC::AUTO...`. `Ok(None)` when `text` is neither.
fn crs_from_ogc_auto(text: &str) -> Result<Option<Crs>, FactoryError> {
    if text
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("AUTO:"))
    {
        return import_wms_auto(text).map(Some);
    }
    let Some(normalized) = auto_from_urn(text) else {
        return Ok(None);
    };
    import_wms_auto(&normalized).map(Some)
}

/// `urn:ogc:def:crs:OGC::AUTO42001:-117:33` becomes `AUTO:42001,-117,33`.
fn auto_from_urn(text: &str) -> Option<String> {
    let tokens: Vec<&str> = text.split(':').collect();
    let code = tokens.get(6)?;
    if tokens.len() <= 7
        || !tokens
            .first()
            .is_some_and(|token| token.eq_ignore_ascii_case("urn"))
        || tokens.get(4).is_none_or(|token| *token != "OGC")
        || !code
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("AUTO"))
    {
        return None;
    }
    let marker = text.find(":AUTO")?;
    let body = text[marker + 5..].replace(':', ",");
    Some(format!("AUTO:{body}"))
}

fn import_wms_auto(text: &str) -> Result<Crs, FactoryError> {
    let body = text.get(5..).ok_or_else(wms_invalid)?;
    let parts: Vec<&str> = body.split(',').collect();
    let method = parts
        .first()
        .copied()
        .ok_or_else(wms_invalid)?
        .parse::<i32>()
        .map_err(|_| wms_invalid())?;
    let (unit_code, longitude, latitude) = auto_arguments(method, &parts)?;
    let conversion = auto_conversion(method, longitude, latitude)?;
    let crs = ProjectedCRS::create(
        "unknown",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        conversion,
        CartesianCS::create_easting_northing(auto_unit(unit_code)?),
    );
    Ok(Crs::Projected(crs))
}

fn wms_invalid() -> FactoryError {
    invalid("invalid WMS AUTO CRS definition")
}

fn auto_arguments(method: i32, parts: &[&str]) -> Result<(i32, f64, f64), FactoryError> {
    let number = |index: usize| -> Result<f64, FactoryError> {
        parts
            .get(index)
            .ok_or_else(wms_invalid)?
            .parse()
            .map_err(|_| wms_invalid())
    };
    let integer = |index: usize| -> Result<i32, FactoryError> {
        parts
            .get(index)
            .ok_or_else(wms_invalid)?
            .parse()
            .map_err(|_| wms_invalid())
    };
    if parts.len() == 4 {
        Ok((integer(1)?, number(2)?, number(3)?))
    } else if parts.len() == 3 && method == 42005 {
        Ok((integer(1)?, number(2)?, 0.0))
    } else if parts.len() == 3 {
        Ok((9001, number(1)?, number(2)?))
    } else if parts.len() == 2 && method == 42005 {
        Ok((9001, number(1)?, 0.0))
    } else {
        Err(wms_invalid())
    }
}

fn auto_conversion(method: i32, longitude: f64, latitude: f64) -> Result<Conversion, FactoryError> {
    let props = OpProps::new();
    match method {
        42001 => {
            if !(longitude >= -180.0 && longitude < 180.0) {
                return Err(wms_invalid());
            }
            let zone_index = finite_integer::<i32>(((longitude + 180.0) / 6.0).floor())
                .ok_or_else(wms_invalid)?;
            let zone = zone_index + 1;
            if latitude >= 0.0 {
                Conversion::create_utm_north(&props, zone)
            } else {
                Conversion::create_utm_south(&props, zone)
            }
        }
        42002 => {
            let northing = if latitude >= 0.0 { 0.0 } else { 10_000_000.0 };
            Conversion::create_transverse_mercator(
                &props, 0.0, longitude, 0.9996, 500_000.0, northing,
            )
        }
        42003 => Conversion::create_orthographic(&props, latitude, longitude, 0.0, 0.0),
        42004 => Conversion::create_equidistant_cylindrical(&props, latitude, longitude, 0.0, 0.0),
        42005 => Conversion::create_mollweide(&props, longitude, 0.0, 0.0),
        _ => return Err(wms_invalid()),
    }
    .map_err(|_| wms_invalid())
}

fn auto_unit(code: i32) -> Result<UnitOfMeasure, FactoryError> {
    match code {
        9001 => Ok(UnitOfMeasure::metre()),
        9002 => Ok(UnitOfMeasure::foot()),
        9003 => Ok(UnitOfMeasure {
            name: "US survey foot".to_owned(),
            factor: 1200.0 / 3937.0,
            kind: UnitKind::Linear,
            code_space: "EPSG".to_owned(),
            code: "9003".to_owned(),
        }),
        _ => Err(wms_invalid()),
    }
}
