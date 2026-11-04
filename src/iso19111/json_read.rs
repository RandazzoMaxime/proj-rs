//! PROJJSON import: the `JSONParser` of PROJ 9.9.0 (`io.cpp`).
//!
//! The object kinds that the model holds are built. Every other `type` is an
//! error, as is a missing or mistyped member. `createFromUserInput` takes the
//! CRS branch of [`JsonObject`].

use super::factory::{AuthorityFactory, DatabaseContext};
use super::model::{
    Axis, BoundCRS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep, Conversion,
    CoordinateOperation, CoordinateSystem, Crs, Datum, DatumEnsemble, Ellipsoid, EllipsoidalCS,
    FactoryError, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, GeoidModel, Identified,
    Identifier, Measure, Meridian, ObjectDomain, Parameter, ParameterValue, PrimeMeridian,
    ProjectedCRS, SphericalCS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS,
    VerticalExtent, VerticalReferenceFrame,
};
use super::niche::DerivedCRS;
use crate::crs_text::{self, Object, Value};

type Read<T> = Result<T, FactoryError>;

/// `Transformation` has a target CRS. `PointMotionOperation` does not.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Motion {
    Stationary,
    Point,
}

/// An object that PROJJSON import produces.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "a CRS is stored inline; the other objects are smaller PROJJSON nodes"
)]
pub enum JsonObject {
    /// `Ellipsoid`.
    Ellipsoid(Ellipsoid),
    /// `PrimeMeridian`.
    PrimeMeridian(PrimeMeridian),
    /// `Axis`.
    Axis(Axis),
    /// `GeodeticReferenceFrame` or `DynamicGeodeticReferenceFrame`.
    GeodeticReferenceFrame(GeodeticReferenceFrame),
    /// `VerticalReferenceFrame` or `DynamicVerticalReferenceFrame`.
    VerticalReferenceFrame(VerticalReferenceFrame),
    /// `DatumEnsemble`.
    DatumEnsemble(DatumEnsemble),
    /// A map projection or other conversion, including a repaired UTM id.
    Conversion(Conversion),
    /// A transformation, or a point-motion operation (`point_motion`).
    Transformation(Transformation),
    /// A concatenated operation.
    Concatenated(ConcatenatedOperation),
    /// Any CRS.
    Crs(Crs),
}

fn invalid<T>(message: impl Into<String>) -> Read<T> {
    Err(FactoryError::Invalid(message.into()))
}

/// Build the object that `text` describes, as `createFromUserInput` does for `{`.
///
/// `db` is optional: it resolves `IAU` plus a version to the versioned code
/// space, and the members of a datum ensemble.
///
/// # Errors
///
/// The text is not JSON, a member is missing or has the wrong type, or the
/// `type` is not one this build imports.
pub fn create_object_from_json(text: &str, db: Option<&DatabaseContext>) -> Read<JsonObject> {
    let value = crs_text::parse_json(text)
        .map_err(|error| FactoryError::Invalid(format!("PROJJSON: {error}")))?;
    let mut reader = Reader {
        db,
        deformation_model: None,
    };
    reader.create(&value)
}

struct Reader<'a> {
    db: Option<&'a DatabaseContext>,
    deformation_model: Option<String>,
}

fn contains(object: &Object, key: &str) -> bool {
    object.get(key).is_some()
}

fn get_value<'v>(object: &'v Object, key: &str) -> Read<&'v Value> {
    object
        .get(key)
        .map_or_else(|| invalid(format!("Missing \"{key}\" key")), Ok)
}

fn get_string<'v>(object: &'v Object, key: &str) -> Read<&'v str> {
    match get_value(object, key)? {
        Value::String(text) => Ok(text),
        _ => invalid(format!("The value of \"{key}\" should be a string")),
    }
}

fn get_object<'v>(object: &'v Object, key: &str) -> Read<&'v Object> {
    match get_value(object, key)? {
        Value::Object(inner) => Ok(inner),
        _ => invalid(format!("The value of \"{key}\" should be a object")),
    }
}

fn get_array<'v>(object: &'v Object, key: &str) -> Read<&'v [Value]> {
    match get_value(object, key)? {
        Value::Array(items) => Ok(items),
        _ => invalid(format!("The value of \"{key}\" should be a array")),
    }
}

fn get_number(object: &Object, key: &str) -> Read<f64> {
    match get_value(object, key)? {
        Value::Number(number) => Ok(*number),
        _ => invalid(format!("The value of \"{key}\" should be a number")),
    }
}

fn is_integral(number: f64) -> bool {
    number.fract() == 0.0 && number >= f64::from(i32::MIN) && number <= f64::from(i32::MAX)
}

fn get_integer(object: &Object, key: &str) -> Read<i64> {
    match get_value(object, key)? {
        Value::Number(number) if is_integral(*number) => {
            format!("{number:.0}").parse().map_err(|_| {
                FactoryError::Invalid(format!("The value of \"{key}\" should be an integer"))
            })
        }
        _ => invalid(format!("The value of \"{key}\" should be an integer")),
    }
}

fn code_text(value: &Value) -> Read<String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) if is_integral(*number) => Ok(format!("{number:.0}")),
        _ => invalid("Unexpected type for value of \"code\""),
    }
}

fn get_unit(object: &Object, key: &str) -> Read<UnitOfMeasure> {
    match get_value(object, key)? {
        Value::String(name) => match name.as_str() {
            "metre" => Ok(UnitOfMeasure::metre()),
            "degree" => Ok(UnitOfMeasure::degree()),
            "unity" => Ok(UnitOfMeasure::new("unity", 1.0, UnitKind::Scale)),
            other => invalid(format!("Unknown unit name: {other}")),
        },
        Value::Object(unit) => {
            let kind = match get_string(unit, "type")? {
                "LinearUnit" => UnitKind::Linear,
                "AngularUnit" => UnitKind::Angular,
                "ScaleUnit" => UnitKind::Scale,
                "TimeUnit" => UnitKind::Time,
                "ParametricUnit" | "Unit" => UnitKind::Unknown,
                _ => return invalid("Unsupported value of \"type\""),
            };
            let name = get_string(unit, "name")?;
            let factor = get_number(unit, "conversion_factor")?;
            let mut built = UnitOfMeasure::new(name, factor, kind);
            if contains(unit, "authority") && contains(unit, "code") {
                built.code_space = get_string(unit, "authority")?.to_owned();
                built.code = code_text(get_value(unit, "code")?)?;
            }
            Ok(built)
        }
        _ => invalid(format!(
            "The value of \"{key}\" should be a string or an object"
        )),
    }
}

fn get_measure(object: &Object) -> Read<Measure> {
    Ok(Measure {
        value: get_number(object, "value")?,
        unit: get_unit(object, "unit")?,
    })
}

fn get_length(object: &Object, key: &str) -> Read<Measure> {
    match get_value(object, key)? {
        Value::Number(number) => Ok(Measure {
            value: *number,
            unit: UnitOfMeasure::metre(),
        }),
        Value::Object(inner) => get_measure(inner),
        _ => invalid(format!(
            "The value of \"{key}\" should be a number or an object"
        )),
    }
}

/// `accuracy` is a JSON string. The number is kept when the text parses.
fn accuracy_of(object: &Object) -> Read<(Option<f64>, Option<String>)> {
    if !contains(object, "accuracy") {
        return Ok((None, None));
    }
    let text = get_string(object, "accuracy")?.to_owned();
    let parsed = text.parse::<f64>().ok();
    Ok((parsed, Some(text)))
}

fn get_optional_number(object: &Object, key: &str) -> Read<Option<f64>> {
    if contains(object, key) {
        get_number(object, key).map(Some)
    } else {
        Ok(None)
    }
}

impl Reader<'_> {
    fn create(&mut self, value: &Value) -> Read<JsonObject> {
        let Value::Object(object) = value else {
            return invalid("JSON object expected");
        };
        let kind = get_string(object, "type")?;
        Ok(match kind {
            "GeographicCRS" => JsonObject::Crs(Crs::Geographic(self.build_geographic_crs(object)?)),
            "GeodeticCRS" => JsonObject::Crs(Crs::Geodetic(self.build_geodetic_crs(object)?)),
            "ProjectedCRS" => JsonObject::Crs(Crs::Projected(self.build_projected_crs(object)?)),
            "DerivedGeographicCRS" => {
                JsonObject::Crs(Crs::Derived(self.build_derived_crs(object)?))
            }
            "VerticalCRS" => JsonObject::Crs(Crs::Vertical(self.build_vertical_crs(object)?)),
            "CompoundCRS" => JsonObject::Crs(Crs::Compound(self.build_compound_crs(object)?)),
            "BoundCRS" => JsonObject::Crs(Crs::Bound(self.build_bound_crs(object)?)),
            "DatumEnsemble" => JsonObject::DatumEnsemble(self.build_ensemble(object)?),
            "GeodeticReferenceFrame" => {
                JsonObject::GeodeticReferenceFrame(self.build_geodetic_frame(object, false)?)
            }
            "DynamicGeodeticReferenceFrame" => {
                JsonObject::GeodeticReferenceFrame(self.build_geodetic_frame(object, true)?)
            }
            "VerticalReferenceFrame" => {
                JsonObject::VerticalReferenceFrame(self.build_vertical_frame(object, false)?)
            }
            "DynamicVerticalReferenceFrame" => {
                JsonObject::VerticalReferenceFrame(self.build_vertical_frame(object, true)?)
            }
            "Ellipsoid" => JsonObject::Ellipsoid(self.build_ellipsoid(object)?),
            "PrimeMeridian" => JsonObject::PrimeMeridian(self.build_prime_meridian(object)?),
            "Axis" => JsonObject::Axis(self.build_axis(object)?),
            "Conversion" => JsonObject::Conversion(self.build_conversion(object)?),
            "Transformation" => {
                JsonObject::Transformation(self.build_transformation(object, Motion::Stationary)?)
            }
            "PointMotionOperation" => {
                JsonObject::Transformation(self.build_transformation(object, Motion::Point)?)
            }
            "ConcatenatedOperation" => JsonObject::Concatenated(self.build_concatenated(object)?),
            _ => return invalid("Unsupported value of \"type\""),
        })
    }

    fn build_crs(&mut self, value: &Value) -> Read<Crs> {
        match self.create(value)? {
            JsonObject::Crs(crs) => Ok(crs),
            _ => invalid("Object is not a CRS"),
        }
    }

    fn build_id(&self, parent: &Object, object: &Object) -> Read<Identifier> {
        let mut code_space = get_string(object, "authority")?.to_owned();
        let mut version = String::new();
        if let Some(value) = object.get("version") {
            version = match value {
                Value::String(text) => text.clone(),
                Value::Number(number) if is_integral(*number) => format!("{number:.0}"),
                Value::Number(number) => super::wkt::format_g(*number, 15),
                _ => return invalid("Unexpected type for value of \"version\""),
            };
        }
        if let (Some(db), false) = (self.db, version.is_empty()) {
            if let Some(versioned) = db.versioned_authority(&code_space, &version) {
                code_space = versioned;
                version.clear();
            }
        }
        let mut code = code_text(
            object
                .get("code")
                .map_or_else(|| invalid("Missing \"code\" key"), Ok)?,
        )?;
        // PROJ before 9.5 wrote a wrong code for a southern UTM conversion.
        if matches!(parent.get("type"), Some(Value::String(kind)) if kind == "Conversion")
            && code_space == "EPSG"
        {
            if let Some(Value::String(name)) = parent.get("name") {
                if name.to_ascii_lowercase().starts_with("utm zone ") && name.contains('S') {
                    let digits: String = name["UTM zone ".len()..]
                        .chars()
                        .take_while(char::is_ascii_digit)
                        .collect();
                    if let Ok(zone @ 1..=60) = digits.parse::<i32>() {
                        code = (16100 + zone).to_string();
                    }
                }
            }
        }
        let mut identifier = Identifier::new(code_space, code);
        if !version.is_empty() {
            identifier.version = Some(version);
        }
        if contains(object, "authority_citation") {
            let citation = get_string(object, "authority_citation")?;
            if citation != identifier.code_space {
                identifier.authority_citation = Some(citation.to_owned());
            }
        }
        if contains(object, "uri") {
            identifier.uri = Some(get_string(object, "uri")?.to_owned());
        }
        Ok(identifier)
    }

    fn build_domain(object: &Object) -> Read<Option<ObjectDomain>> {
        let scope = if contains(object, "scope") {
            Some(get_string(object, "scope")?.to_owned())
        } else {
            None
        };
        let area = if contains(object, "area") {
            Some(get_string(object, "area")?.to_owned())
        } else {
            None
        };
        let mut bounds = None;
        if contains(object, "bbox") {
            let bbox = get_object(object, "bbox")?;
            let south = get_number(bbox, "south_latitude")?;
            let west = get_number(bbox, "west_longitude")?;
            let north = get_number(bbox, "north_latitude")?;
            let east = get_number(bbox, "east_longitude")?;
            if !(-90.0..=90.0).contains(&south)
                || !(-90.0..=90.0).contains(&north)
                || south > north
                || !(-180.0..=180.0).contains(&west)
                || !(-180.0..=180.0).contains(&east)
            {
                return invalid("Invalid bbox node: invalid latitudes or longitudes");
            }
            bounds = Some((south, west, north, east));
        }
        if scope.is_none() && area.is_none() && bounds.is_none() {
            return Ok(None);
        }
        Ok(Some(ObjectDomain {
            scope,
            area,
            south: bounds.map(|b| b.0),
            west: bounds.map(|b| b.1),
            north: bounds.map(|b| b.2),
            east: bounds.map(|b| b.3),
        }))
    }

    fn build_vertical_extent(object: &Object) -> Read<Option<VerticalExtent>> {
        if !contains(object, "vertical_extent") {
            return Ok(None);
        }
        let extent = get_object(object, "vertical_extent")?;
        let minimum = get_number(extent, "minimum")?;
        let maximum = get_number(extent, "maximum")?;
        let unit = if contains(extent, "unit") {
            get_unit(extent, "unit")?
        } else {
            UnitOfMeasure::metre()
        };
        Ok(Some(VerticalExtent {
            minimum,
            maximum,
            unit,
        }))
    }

    /// `buildProperties`: name, ids, remarks and usages.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "name_required is PROJ's buildProperties flag for a node that must be named"
    )]
    fn properties(&self, object: &Object, name_required: bool) -> Read<Identified> {
        let mut identified = Identified::named(String::new());
        if contains(object, "name") || name_required {
            identified.name = get_string(object, "name")?.to_owned();
        }
        if contains(object, "ids") {
            for id in get_array(object, "ids")? {
                let Value::Object(id) = id else {
                    return invalid("Unexpected type for value of \"ids\" child");
                };
                identified.identifiers.push(self.build_id(object, id)?);
            }
        } else if contains(object, "id") {
            let id = get_object(object, "id")?;
            identified.identifiers.push(self.build_id(object, id)?);
        }
        if contains(object, "remarks") {
            identified.remarks = get_string(object, "remarks")?.to_owned();
        }
        if contains(object, "usages") {
            let Value::Array(usages) = get_value(object, "usages")? else {
                return invalid("Unexpected type for value of \"usages\"");
            };
            for usage in usages {
                let Value::Object(usage) = usage else {
                    return invalid("Unexpected type for value of \"usages\" child");
                };
                match Self::build_domain(usage)? {
                    Some(domain) => identified.domains.push(domain),
                    None => return invalid("missing children in \"usages\" child"),
                }
            }
        } else if let Some(domain) = Self::build_domain(object)? {
            identified.domains.push(domain);
        }
        identified.vertical_extent = Self::build_vertical_extent(object)?;
        Ok(identified)
    }

    fn anchor(object: &Object) -> Read<Option<String>> {
        if contains(object, "anchor") {
            Ok(Some(get_string(object, "anchor")?.to_owned()))
        } else {
            Ok(None)
        }
    }

    fn anchor_epoch(object: &Object) -> Read<Option<f64>> {
        get_optional_number(object, "anchor_epoch")
    }

    fn build_ellipsoid(&self, object: &Object) -> Read<Ellipsoid> {
        let mut identified = self.properties(object, false)?;
        if contains(object, "semi_major_axis") {
            let semi_major = get_length(object, "semi_major_axis")?;
            let body = super::projstring::guess_body(semi_major.si_value());
            if contains(object, "semi_minor_axis") {
                return Ok(Ellipsoid {
                    identified,
                    semi_major,
                    inverse_flattening: None,
                    semi_minor: Some(get_length(object, "semi_minor_axis")?),
                    body,
                });
            }
            if contains(object, "inverse_flattening") {
                return Ok(Ellipsoid {
                    identified,
                    semi_major,
                    inverse_flattening: Some(get_number(object, "inverse_flattening")?),
                    semi_minor: None,
                    body,
                });
            }
            return invalid("Missing semi_minor_axis or inverse_flattening");
        }
        if contains(object, "radius") {
            let radius = get_length(object, "radius")?;
            let body = super::projstring::guess_body(radius.si_value());
            identified.name = get_string(object, "name")?.to_owned();
            return Ok(Ellipsoid {
                identified,
                semi_major: radius,
                inverse_flattening: Some(0.0),
                semi_minor: None,
                body,
            });
        }
        invalid("Missing semi_major_axis or radius")
    }

    fn build_prime_meridian(&self, object: &Object) -> Read<PrimeMeridian> {
        let identified = self.properties(object, false)?;
        let longitude = match get_value(object, "longitude")? {
            Value::Number(number) => Measure {
                value: *number,
                unit: UnitOfMeasure::degree(),
            },
            Value::Object(inner) => get_measure(inner)?,
            _ => return invalid("Unexpected type for value of \"longitude\""),
        };
        Ok(PrimeMeridian {
            identified,
            longitude,
        })
    }

    fn build_meridian(object: &Object) -> Read<Meridian> {
        let longitude = match get_value(object, "longitude")? {
            Value::Number(number) => Measure {
                value: *number,
                unit: UnitOfMeasure::degree(),
            },
            Value::Object(inner) => get_measure(inner)?,
            _ => return invalid("Unexpected type for value of \"longitude\""),
        };
        Ok(Meridian { longitude })
    }

    fn build_axis(&self, object: &Object) -> Read<Axis> {
        let direction = get_string(object, "direction")?;
        let abbreviation = get_string(object, "abbreviation")?;
        let unit = if contains(object, "unit") {
            get_unit(object, "unit")?
        } else {
            UnitOfMeasure::new(String::new(), 1.0, UnitKind::None)
        };
        if !AXIS_DIRECTIONS.contains(&direction) {
            return invalid(format!("unhandled axis direction: {direction}"));
        }
        let meridian = if contains(object, "meridian") {
            Some(Self::build_meridian(get_object(object, "meridian")?)?)
        } else {
            None
        };
        let minimum = get_optional_number(object, "minimum_value")?;
        let maximum = get_optional_number(object, "maximum_value")?;
        let range_meaning = if contains(object, "range_meaning") {
            let meaning = get_string(object, "range_meaning")?;
            if meaning != "exact" && meaning != "wraparound" {
                return invalid(format!("buildAxis: invalid range_meaning value: {meaning}"));
            }
            Some(meaning.to_owned())
        } else {
            None
        };
        let name = self.properties(object, false)?.name;
        Ok(Axis {
            name,
            abbreviation: abbreviation.to_owned(),
            direction: direction.to_owned(),
            unit,
            meridian,
            minimum,
            maximum,
            range_meaning,
        })
    }

    fn build_cs(&self, object: &Object) -> Read<CoordinateSystem> {
        let subtype = get_string(object, "subtype")?;
        let Value::Array(items) = get_value(object, "axis")? else {
            return invalid("Unexpected type for value of \"axis\"");
        };
        let mut axes = Vec::new();
        for item in items {
            let Value::Object(item) = item else {
                return invalid("Unexpected type for value of a \"axis\" member");
            };
            axes.push(self.build_axis(item)?);
        }
        let count = axes.len();
        match subtype {
            "ellipsoidal" if count == 2 || count == 3 => {
                Ok(CoordinateSystem::Ellipsoidal(EllipsoidalCS {
                    identified: Identified::default(),
                    axes,
                }))
            }
            "Cartesian" if count == 2 || count == 3 => Ok(CoordinateSystem::Cartesian(
                super::model::CartesianCS::create_allowing_mixed_units(axes)?,
            )),
            "vertical" if count == 1 => Ok(CoordinateSystem::Vertical(VerticalCS {
                identified: Identified::default(),
                axis: axes.remove(0),
            })),
            "spherical" if count == 2 || count == 3 => {
                Ok(CoordinateSystem::Spherical(SphericalCS {
                    identified: Identified::default(),
                    axes,
                }))
            }
            "ellipsoidal" | "Cartesian" | "spherical" => invalid("Expected 2 or 3 axis"),
            "vertical" => invalid("Expected 1 axis"),
            _ => invalid("Unhandled value for subtype"),
        }
    }

    fn build_geodetic_datum_or_ensemble(
        &mut self,
        object: &Object,
    ) -> Read<(Option<GeodeticReferenceFrame>, Option<DatumEnsemble>)> {
        if contains(object, "datum") {
            let datum = get_object(object, "datum")?;
            if contains(object, "deformation_models") {
                if let Some(Value::Object(first)) = get_array(object, "deformation_models")?.first()
                {
                    self.deformation_model = Some(get_string(first, "name")?.to_owned());
                }
            }
            let built = self.create(&Value::Object(datum.clone()))?;
            self.deformation_model = None;
            match built {
                JsonObject::GeodeticReferenceFrame(frame) => Ok((Some(frame), None)),
                _ => invalid("datum of wrong type"),
            }
        } else {
            Ok((
                None,
                Some(self.build_ensemble(get_object(object, "datum_ensemble")?)?),
            ))
        }
    }

    fn build_geographic_crs(&mut self, object: &Object) -> Read<GeographicCRS> {
        let (datum, ensemble) = self.build_geodetic_datum_or_ensemble(object)?;
        let CoordinateSystem::Ellipsoidal(coordinate_system) =
            self.build_cs(get_object(object, "coordinate_system")?)?
        else {
            return invalid("expected an ellipsoidal CS");
        };
        let identified = self.properties(object, false)?;
        let mut crs =
            GeographicCRS::create(identified.name.clone(), datum, ensemble, coordinate_system)?;
        crs.identified = identified;
        Ok(crs)
    }

    fn build_geodetic_crs(&mut self, object: &Object) -> Read<GeodeticCRS> {
        let (datum, ensemble) = self.build_geodetic_datum_or_ensemble(object)?;
        let cs = self.build_cs(get_object(object, "coordinate_system")?)?;
        let identified = self.properties(object, false)?;
        match &cs {
            CoordinateSystem::Cartesian(cartesian) => {
                if cartesian.axes().len() != 3 {
                    return invalid("Cartesian CS for a GeodeticCRS should have 3 axis");
                }
            }
            CoordinateSystem::Spherical(_) => {}
            _ => return invalid("expected a Cartesian or spherical CS"),
        }
        let mut crs = GeodeticCRS::create(identified.name.clone(), datum, ensemble, cs)
            .map_err(|error| FactoryError::Invalid(format!("buildGeodeticCRS: {error}")))?;
        set_geodetic_identified(&mut crs, identified);
        Ok(crs)
    }

    fn build_derived_crs(&mut self, object: &Object) -> Read<DerivedCRS> {
        let base = self.build_crs(&Value::Object(get_object(object, "base_crs")?.clone()))?;
        let coordinate_system = self.build_cs(get_object(object, "coordinate_system")?)?;
        let conversion = self.build_conversion(get_object(object, "conversion")?)?;
        let identified = self.properties(object, false)?;
        let mut crs =
            DerivedCRS::create(identified.name.clone(), base, conversion, coordinate_system)?;
        crs.identified = identified;
        Ok(crs)
    }

    fn build_projected_crs(&mut self, object: &Object) -> Read<ProjectedCRS> {
        let base = get_object(object, "base_crs")?;
        let base_cs = self.build_cs(get_object(base, "coordinate_system")?)?;
        let base_crs = if matches!(base_cs, CoordinateSystem::Ellipsoidal(_)) {
            GeodeticCRS::Geographic(self.build_geographic_crs(base)?)
        } else {
            self.build_geodetic_crs(base)?
        };
        let CoordinateSystem::Cartesian(cartesian) =
            self.build_cs(get_object(object, "coordinate_system")?)?
        else {
            return invalid("expected a Cartesian CS");
        };
        let conversion = self.build_conversion(get_object(object, "conversion")?)?;
        let identified = self.properties(object, false)?;
        let mut crs =
            ProjectedCRS::create(identified.name.clone(), base_crs, conversion, cartesian);
        crs.identified = identified;
        Ok(crs)
    }

    fn build_vertical_crs(&mut self, object: &Object) -> Read<VerticalCRS> {
        let (datum, ensemble) = if contains(object, "datum") {
            let datum = get_object(object, "datum")?;
            if contains(object, "deformation_models") {
                if let Some(Value::Object(first)) = get_array(object, "deformation_models")?.first()
                {
                    self.deformation_model = Some(get_string(first, "name")?.to_owned());
                }
            }
            let built = self.create(&Value::Object(datum.clone()))?;
            match built {
                JsonObject::VerticalReferenceFrame(frame) => (Some(frame), None),
                _ => return invalid("datum of wrong type"),
            }
        } else {
            (
                None,
                Some(self.build_ensemble(get_object(object, "datum_ensemble")?)?),
            )
        };
        let CoordinateSystem::Vertical(coordinate_system) =
            self.build_cs(get_object(object, "coordinate_system")?)?
        else {
            return invalid("expected a vertical CS");
        };
        let mut geoid_models = Vec::new();
        if contains(object, "geoid_model") {
            geoid_models.push(self.build_geoid_model(get_object(object, "geoid_model")?)?);
        } else if contains(object, "geoid_models") {
            for model in get_array(object, "geoid_models")? {
                let Value::Object(model) = model else {
                    return invalid("Unexpected type for value of \"geoid_models\" child");
                };
                geoid_models.push(self.build_geoid_model(model)?);
            }
        }
        let identified = self.properties(object, false)?;
        Ok(VerticalCRS {
            identified,
            datum,
            ensemble,
            coordinate_system,
            geoid_models,
        })
    }

    fn build_geoid_model(&mut self, object: &Object) -> Read<GeoidModel> {
        let identified = self.properties(object, false)?;
        let interpolation_crs = if contains(object, "interpolation_crs") {
            let crs = get_object(object, "interpolation_crs")?;
            Some(Box::new(self.build_crs(&Value::Object(crs.clone()))?))
        } else {
            None
        };
        Ok(GeoidModel {
            name: identified.name,
            identifiers: identified.identifiers,
            interpolation_crs,
        })
    }

    fn build_compound_crs(&mut self, object: &Object) -> Read<CompoundCRS> {
        let mut components = Vec::new();
        for component in get_array(object, "components")? {
            if !matches!(component, Value::Object(_)) {
                return invalid("Unexpected type for a \"components\" child");
            }
            components.push(self.build_crs(component)?);
        }
        let identified = self.properties(object, false)?;
        let mut crs = CompoundCRS::create(identified.name.clone(), components)?;
        crs.identified = identified;
        Ok(crs)
    }

    fn build_conversion(&self, object: &Object) -> Read<Conversion> {
        let method = get_object(object, "method")?;
        let identified = self.properties(object, false)?;
        let method_properties = self.properties(method, false)?;
        let (code_space, code) = method_properties
            .identifiers
            .first()
            .map(|id| (id.code_space().to_owned(), id.code().to_owned()))
            .unwrap_or_default();
        let mut parameters = Vec::new();
        if contains(object, "parameters") {
            for parameter in get_array(object, "parameters")? {
                let Value::Object(parameter) = parameter else {
                    return invalid("Unexpected type for a \"parameters\" child");
                };
                parameters.push(self.build_parameter(parameter)?);
            }
        }
        let mut conversion =
            Conversion::create_with_method(&method_properties.name, &code_space, &code, parameters);
        conversion.identified = identified;
        Ok(conversion)
    }

    /// `buildTransformation` and `buildPointMotionOperation`. Accuracy stays the JSON string.
    fn build_transformation(&mut self, object: &Object, motion: Motion) -> Read<Transformation> {
        let point_motion = motion == Motion::Point;
        let source =
            Box::new(self.build_crs(&Value::Object(get_object(object, "source_crs")?.clone()))?);
        let target = if point_motion {
            None
        } else {
            Some(Box::new(self.build_crs(&Value::Object(
                get_object(object, "target_crs")?.clone(),
            ))?))
        };
        let interpolation = if !point_motion && contains(object, "interpolation_crs") {
            Some(Box::new(self.build_crs(&Value::Object(
                get_object(object, "interpolation_crs")?.clone(),
            ))?))
        } else {
            None
        };
        let method = get_object(object, "method")?;
        let mut parameters = Vec::new();
        for parameter in get_array(object, "parameters")? {
            let Value::Object(parameter) = parameter else {
                return invalid("Unexpected type for a \"parameters\" child");
            };
            parameters.push(self.build_parameter(parameter)?);
        }
        let identified = self.properties(object, false)?;
        let method_properties = self.properties(method, false)?;
        let (code_space, code) = method_properties
            .identifiers
            .first()
            .map(|id| (id.code_space().to_owned(), id.code().to_owned()))
            .unwrap_or_default();
        let mut conversion =
            Conversion::create_with_method(&method_properties.name, &code_space, &code, parameters);
        conversion.identified = identified;
        let (accuracy, accuracy_text) = accuracy_of(object)?;
        Ok(Transformation {
            identified: conversion.identified,
            method: conversion.method,
            parameters: conversion.parameters,
            source: Some(source),
            target,
            interpolation,
            accuracy,
            accuracy_text,
            version: None,
            point_motion,
            inverted: false,
            source_epoch: None,
            target_epoch: None,
        })
    }

    /// `buildConcatenatedOperation`. Steps are stored as written; PROJ's `fixSteps` is a no-op
    /// on the JSON this importer is asked to round-trip.
    fn build_concatenated(&mut self, object: &Object) -> Read<ConcatenatedOperation> {
        let source =
            Box::new(self.build_crs(&Value::Object(get_object(object, "source_crs")?.clone()))?);
        let target =
            Box::new(self.build_crs(&Value::Object(get_object(object, "target_crs")?.clone()))?);
        let mut steps = Vec::new();
        for step in get_array(object, "steps")? {
            let Value::Object(step) = step else {
                return invalid("Unexpected type for a \"steps\" child");
            };
            let operation = match self.create(&Value::Object(step.clone()))? {
                JsonObject::Conversion(conversion) => CoordinateOperation::Conversion(conversion),
                JsonObject::Transformation(transformation) => {
                    CoordinateOperation::Transformation(transformation)
                }
                JsonObject::Concatenated(operation) => CoordinateOperation::Concatenated(operation),
                _ => return invalid("Invalid content in a \"steps\" child"),
            };
            steps.push(ConcatenatedStep {
                operation,
                reverse: false,
            });
        }
        if steps.len() < 2 {
            return invalid("a concatenated operation needs at least two steps");
        }
        let identified = self.properties(object, false)?;
        let (accuracy, _) = accuracy_of(object)?;
        Ok(ConcatenatedOperation {
            identified,
            steps,
            source: Some(source),
            target: Some(target),
            accuracy,
            version: None,
            computed_name: false,
            source_epoch: None,
            target_epoch: None,
            proj_based: None,
        })
    }

    fn build_parameter(&self, object: &Object) -> Read<Parameter> {
        let identified = self.properties(object, true)?;
        let (code_space, code) = identified
            .identifiers
            .first()
            .map(|id| (id.code_space().to_owned(), id.code().to_owned()))
            .unwrap_or_default();
        let integer = code_space == "EPSG" && code == "1049";
        let value = if integer {
            ParameterValue::Integer(get_integer(object, "value")?)
        } else if let Some(Value::String(text)) = object.get("value") {
            ParameterValue::Filename(text.clone())
        } else {
            ParameterValue::Measure(get_measure(object)?)
        };
        Ok(Parameter {
            name: identified.name,
            code_space,
            code,
            value,
        })
    }

    fn build_bound_crs(&mut self, object: &Object) -> Read<BoundCRS> {
        let source = self.build_crs(&Value::Object(get_object(object, "source_crs")?.clone()))?;
        let target = self.build_crs(&Value::Object(get_object(object, "target_crs")?.clone()))?;
        let transformation = get_object(object, "transformation")?;
        let method = get_object(transformation, "method")?;
        let mut parameters = Vec::new();
        for parameter in get_array(transformation, "parameters")? {
            let Value::Object(parameter) = parameter else {
                return invalid("Unexpected type for a \"parameters\" child");
            };
            parameters.push(self.build_parameter(parameter)?);
        }
        let operation_source = if contains(transformation, "source_crs") {
            let object = get_object(transformation, "source_crs")?;
            Some(Box::new(self.build_crs(&Value::Object(object.clone()))?))
        } else {
            None
        };
        let identified = self.properties(transformation, false)?;
        let method_properties = self.properties(method, false)?;
        let (code_space, code) = method_properties
            .identifiers
            .first()
            .map(|id| (id.code_space().to_owned(), id.code().to_owned()))
            .unwrap_or_default();
        let mut conversion =
            Conversion::create_with_method(&method_properties.name, &code_space, &code, parameters);
        conversion.identified = identified;
        // A bound CRS binds with a transformation. PROJJSON writes it abridged, with no `type`.
        let operation = CoordinateOperation::Transformation(Transformation {
            identified: conversion.identified,
            method: conversion.method,
            parameters: conversion.parameters,
            source: operation_source,
            target: None,
            interpolation: None,
            accuracy: None,
            accuracy_text: None,
            version: None,
            point_motion: false,
            source_epoch: None,
            target_epoch: None,
            inverted: false,
        });
        let mut bound = BoundCRS::create(source, target, operation);
        bound.identified = self.properties(object, false)?;
        Ok(bound)
    }

    fn build_ensemble(&self, object: &Object) -> Read<DatumEnsemble> {
        let mut members = Vec::new();
        if contains(object, "members") {
            let has_ellipsoid = contains(object, "ellipsoid");
            for member in get_array(object, "members")? {
                let Value::Object(member) = member else {
                    return invalid("Unexpected type for value of a \"members\" member");
                };
                members.push(self.build_ensemble_member(object, member, has_ellipsoid)?);
            }
        } else {
            let name = get_string(object, "name")?;
            match self.db {
                Some(db) => {
                    let found = super::json::ensemble_members_by_name(db, name)?;
                    if found.is_empty() {
                        return invalid(format!(
                            "No entry for datum ensemble '{name}' in database, and no explicit member specified"
                        ));
                    }
                    members = found;
                }
                None => {
                    return invalid(format!(
                        "Datum ensemble '{name}' has no explicit member specified and no connection to database"
                    ));
                }
            }
        }
        let accuracy_text = get_string(object, "accuracy")?;
        let accuracy = accuracy_text.parse::<f64>().unwrap_or(0.0);
        let identified = self.properties(object, false)?;
        let mut ensemble = DatumEnsemble::create(identified.name.clone(), members, accuracy)?;
        ensemble.identified = identified;
        ensemble.accuracy_text = Some(accuracy_text.to_owned());
        Ok(ensemble)
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "has_ellipsoid selects a geodetic member against a vertical one"
    )]
    fn build_ensemble_member(
        &self,
        parent: &Object,
        member: &Object,
        has_ellipsoid: bool,
    ) -> Read<Datum> {
        let name = get_string(member, "name")?;
        if let Some(db) = self.db {
            if contains(member, "id") {
                let id = get_object(member, "id")?;
                let authority = get_string(id, "authority")?;
                let code = code_text(
                    id.get("code")
                        .map_or_else(|| invalid("Missing \"code\" key"), Ok)?,
                )?;
                if let Ok(datum) = AuthorityFactory::create(db, authority).create_datum(&code) {
                    return Ok(datum);
                }
            }
            if let Some(datum) = super::json::datum_by_name(db, name)? {
                return Ok(datum);
            }
        }
        let identified = self.properties(member, false)?;
        if has_ellipsoid {
            let ellipsoid = self.build_ellipsoid(get_object(parent, "ellipsoid")?)?;
            Ok(Datum::Geodetic(GeodeticReferenceFrame {
                identified,
                ellipsoid,
                prime_meridian: PrimeMeridian::greenwich(),
                frame_epoch: None,
                deformation_model: None,
            }))
        } else {
            Ok(Datum::Vertical(VerticalReferenceFrame {
                identified,
                wkt1_type: "2005".to_owned(),
                frame_epoch: None,
                deformation_model: None,
            }))
        }
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "dynamic selects DynamicGeodeticReferenceFrame against GeodeticReferenceFrame"
    )]
    fn build_geodetic_frame(&self, object: &Object, dynamic: bool) -> Read<GeodeticReferenceFrame> {
        let ellipsoid = self.build_ellipsoid(get_object(object, "ellipsoid")?)?;
        let prime_meridian = if contains(object, "prime_meridian") {
            self.build_prime_meridian(get_object(object, "prime_meridian")?)?
        } else {
            PrimeMeridian::greenwich()
        };
        let mut identified = self.properties(object, false)?;
        identified.anchor = Self::anchor(object)?;
        let (frame_epoch, deformation_model) = if dynamic {
            (
                Some(get_number(object, "frame_reference_epoch")?),
                self.dynamic_model(object)?,
            )
        } else {
            identified.anchor_epoch = Self::anchor_epoch(object)?;
            (None, None)
        };
        Ok(GeodeticReferenceFrame {
            identified,
            ellipsoid,
            prime_meridian,
            frame_epoch,
            deformation_model,
        })
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "dynamic selects DynamicVerticalReferenceFrame against VerticalReferenceFrame"
    )]
    fn build_vertical_frame(&self, object: &Object, dynamic: bool) -> Read<VerticalReferenceFrame> {
        let mut identified = self.properties(object, false)?;
        identified.anchor = Self::anchor(object)?;
        let (frame_epoch, deformation_model) = if dynamic {
            (
                Some(get_number(object, "frame_reference_epoch")?),
                self.dynamic_model(object)?,
            )
        } else {
            identified.anchor_epoch = Self::anchor_epoch(object)?;
            (None, None)
        };
        Ok(VerticalReferenceFrame {
            identified,
            wkt1_type: "2005".to_owned(),
            frame_epoch,
            deformation_model,
        })
    }

    /// The `deformation_model` of PROJJSON before 0.5, else the CRS-level model.
    fn dynamic_model(&self, object: &Object) -> Read<Option<String>> {
        if contains(object, "deformation_model") {
            Ok(Some(get_string(object, "deformation_model")?.to_owned()))
        } else {
            Ok(self.deformation_model.clone())
        }
    }
}

fn set_geodetic_identified(crs: &mut GeodeticCRS, identified: Identified) {
    match crs {
        GeodeticCRS::Geographic(crs) => crs.identified = identified,
        GeodeticCRS::Geocentric(crs) => crs.identified = identified,
        GeodeticCRS::Spherical(crs) => crs.identified = identified,
    }
}

/// `AxisDirection` names that PROJ accepts.
const AXIS_DIRECTIONS: &[&str] = &[
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
