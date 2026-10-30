//! PROJJSON export and the `{` branch of `createFromUserInput`.
//!
//! Identifier code spaces such as `IAU_2015` are split into an authority and a
//! version on the way out, and joined again on the way in, using
//! `versioned_auth_name_mapping`.

use super::factory::{AuthorityFactory, DatabaseContext};
use super::model::{
    Axis, CompoundCRS, Crs, DatumEnsemble, Ellipsoid, EllipsoidalCS, FactoryError,
    GeodeticReferenceFrame, GeographicCRS, Identified, Identifier, Measure, PrimeMeridian,
    UnitOfMeasure,
};
use super::projstring::create_crs_from_proj_string;
use super::wkt_import::WktParser;
use crate::crs_text::{self, Object, Value};

const SCHEMA: &str = "https://proj.org/schemas/v0.7/projjson.schema.json";

struct Frame {
    has_child: bool,
    expect_value: bool,
}

struct Writer {
    out: String,
    indent: usize,
    stack: Vec<Frame>,
}

impl Writer {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
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
        self.out.push_str(": ");
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
        self.out.push('\n');
        self.out.extend(std::iter::repeat_n(' ', self.indent * 2));
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
    /// PROJJSON for this CRS.
    ///
    /// `proj.db` splits a versioned authority such as `IAU_2015` into
    /// `"authority": "IAU"` and `"version": 2015`.
    ///
    /// # Errors
    ///
    /// The CRS kind has no PROJJSON export in this build.
    pub fn export_to_json(&self, db: &DatabaseContext) -> Result<String, FactoryError> {
        let mut writer = Writer::new();
        writer.start_object();
        writer.key("$schema");
        writer.string(SCHEMA);
        match self {
            Self::Geographic(crs) => write_geographic(&mut writer, crs, db)?,
            Self::Geodetic(_)
            | Self::Projected(_)
            | Self::Vertical(_)
            | Self::Compound(_)
            | Self::Bound(_)
            | Self::Engineering(_)
            | Self::DerivedProjected(_) => {
                return Err(FactoryError::Format(format!(
                    "PROJJSON export is not built for {}",
                    self.name()
                )));
            }
        }
        writer.end_object();
        Ok(writer.finish())
    }
}

fn write_geographic(
    writer: &mut Writer,
    crs: &GeographicCRS,
    db: &DatabaseContext,
) -> Result<(), FactoryError> {
    writer.key("type");
    writer.string("GeographicCRS");
    writer.key("name");
    writer.string(crs.name());
    if let Some(datum) = crs.datum() {
        writer.key("datum");
        write_datum(writer, datum, db);
    } else if let Some(ensemble) = crs.datum_ensemble() {
        writer.key("datum_ensemble");
        write_ensemble(writer, ensemble, db);
    }
    writer.key("coordinate_system");
    write_ellipsoidal_cs(writer, crs.coordinate_system());
    write_domain(writer, crs.domains());
    write_id(writer, crs.identifiers(), db);
    Ok(())
}

/// The first usage, as PROJ writes it: scope, area, and bounding box.
fn write_domain(writer: &mut Writer, domains: &[super::model::ObjectDomain]) {
    let Some(domain) = domains.first() else {
        return;
    };
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

fn write_datum(writer: &mut Writer, datum: &GeodeticReferenceFrame, db: &DatabaseContext) {
    writer.start_object();
    writer.key("type");
    writer.string("GeodeticReferenceFrame");
    writer.key("name");
    writer.string(datum.name());
    writer.key("ellipsoid");
    write_ellipsoid(writer, datum.ellipsoid(), db);
    writer.key("prime_meridian");
    write_meridian(writer, datum.prime_meridian());
    write_id(writer, datum.identifiers(), db);
    writer.end_object();
}

fn write_ensemble(writer: &mut Writer, ensemble: &DatumEnsemble, db: &DatabaseContext) {
    writer.start_object();
    writer.key("name");
    writer.string(ensemble.name());
    writer.key("members");
    writer.start_array();
    for member in ensemble.members() {
        if let super::model::Datum::Geodetic(frame) = member {
            // PROJ writes an ensemble member as its name and id only.
            writer.start_object();
            writer.key("name");
            writer.string(frame.name());
            write_id(writer, frame.identifiers(), db);
            writer.end_object();
        }
    }
    writer.end_array();
    if let Some(super::model::Datum::Geodetic(frame)) = ensemble.members().first() {
        // The ensemble's ellipsoid is written without an id.
        let ellipsoid = frame.ellipsoid();
        writer.key("ellipsoid");
        writer.start_object();
        writer.key("name");
        writer.string(ellipsoid.name());
        writer.key("semi_major_axis");
        writer.number(ellipsoid.semi_major_axis().si_value());
        writer.key("inverse_flattening");
        writer.number(ellipsoid.inverse_flattening());
        writer.end_object();
    }
    writer.key("accuracy");
    // The accuracy is the text stored in proj.db, so a whole value reads "2.0".
    match ensemble.accuracy_text.as_deref() {
        Some(text) => writer.string(text),
        None => writer.string(&super::wkt::format_g(ensemble.positional_accuracy(), 15)),
    }
    write_id(writer, ensemble.identifiers(), db);
    writer.end_object();
}

fn write_ellipsoid(writer: &mut Writer, ellipsoid: &Ellipsoid, db: &DatabaseContext) {
    writer.start_object();
    writer.key("name");
    writer.string(ellipsoid.name());
    writer.key("semi_major_axis");
    writer.number(ellipsoid.semi_major_axis().si_value());
    writer.key("inverse_flattening");
    writer.number(ellipsoid.inverse_flattening());
    write_id(writer, ellipsoid.identifiers(), db);
    writer.end_object();
}

fn write_meridian(writer: &mut Writer, meridian: &PrimeMeridian) {
    writer.start_object();
    writer.key("name");
    writer.string(meridian.name());
    writer.key("longitude");
    writer.number(meridian.longitude().convert_to(&UnitOfMeasure::degree()));
    writer.end_object();
}

fn write_ellipsoidal_cs(writer: &mut Writer, cs: &EllipsoidalCS) {
    writer.start_object();
    writer.key("subtype");
    writer.string("ellipsoidal");
    writer.key("axis");
    writer.start_array();
    for axis in cs.axes() {
        writer.start_object();
        writer.key("name");
        writer.string(axis.name());
        writer.key("abbreviation");
        writer.string(axis.abbreviation());
        writer.key("direction");
        writer.string(axis.direction());
        writer.key("unit");
        // PROJ writes the two everyday units by name; any other unit as an object.
        if matches!(axis.unit().name(), "degree" | "metre") {
            writer.string(axis.unit().name());
        } else {
            writer.start_object();
            writer.key("name");
            writer.string(axis.unit().name());
            writer.key("conversion_factor");
            writer.number(axis.unit().conversion_to_si());
            writer.end_object();
        }
        writer.end_object();
    }
    writer.end_array();
    writer.end_object();
}

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
    if is_integer_token(identifier.code()) {
        writer.raw_token(identifier.code());
    } else {
        writer.string(identifier.code());
    }
    if let Some(version) = version {
        writer.key("version");
        if is_integer_token(&version) {
            writer.raw_token(&version);
        } else {
            writer.string(&version);
        }
    }
    writer.end_object();
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
        return create_crs_from_proj_string(text);
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
/// `createFromUserInput` does for names. An exact name first; several matches
/// keep the 2D one, then the 3D one, and anything else is an error.
fn crs_from_name(text: &str, db: &DatabaseContext) -> Result<Option<Crs>, FactoryError> {
    if let Some((left, right)) = text.split_once(" + ") {
        let (Some(first), Some(second)) = (
            crs_named_one(left.trim(), db)?,
            crs_named_one(right.trim(), db)?,
        ) else {
            return Ok(None);
        };
        return Ok(Some(Crs::Compound(CompoundCRS::create(
            format!("{} + {}", first.name(), second.name()),
            vec![first, second],
        )?)));
    }
    crs_named_one(text, db)
}

fn crs_named_one(text: &str, db: &DatabaseContext) -> Result<Option<Crs>, FactoryError> {
    let codes = db.crs_codes_named(text)?;
    let mut found = Vec::new();
    for (authority, code) in &codes {
        found.push(
            AuthorityFactory::create(db, authority).create_coordinate_reference_system(code)?,
        );
    }
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.pop()),
        _ => {
            for dimension in [2, 3] {
                if let Some(crs) = found
                    .iter()
                    .find(|crs| geographic_dimension(crs) == dimension)
                {
                    return Ok(Some(crs.clone()));
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

fn geographic_dimension(crs: &Crs) -> usize {
    match crs {
        Crs::Geographic(geographic) => geographic.coordinate_system().axes().len(),
        _ => 2,
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
        other => Err(FactoryError::Invalid(format!(
            "PROJJSON type {other} is not imported in this build"
        ))),
    }
}

fn read_geographic(object: &Object, db: &DatabaseContext) -> Result<GeographicCRS, FactoryError> {
    let name = required_str(object, "name")?;
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
    let name = required_str(object, "name")?;
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
    let name = required_str(object, "name")?;
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
        let name = axis.get("name").and_then(Value::as_str).unwrap_or("axis");
        let abbreviation = axis
            .get("abbreviation")
            .and_then(Value::as_str)
            .unwrap_or("");
        let direction = axis
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("north");
        let unit = axis
            .get("unit")
            .and_then(Value::as_object)
            .map(read_unit)
            .transpose()?
            .unwrap_or_else(UnitOfMeasure::degree);
        built.push(Axis::new(name, abbreviation, direction, unit));
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
    let factor = object
        .get("conversion_factor")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    Ok(UnitOfMeasure {
        name: name.to_owned(),
        factor,
        kind: super::model::UnitKind::Unknown,
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
