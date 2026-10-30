//! Constructors and PROJ-string export for conversions and transformations.
//!
//! The method and parameter names are the PROJ 9.9 maps in [`super::parammap`].

use super::equiv::Equivalence;
use super::model::{
    Crs, FactoryError, Identified, Measure, ObjectDomain, OperationMethod, OperationParameter,
    OperationParameterValue, Parameter, ParameterValue, Transformation, UnitKind,
};
use super::parammap::{self, MethodMap, ParamMap};
use super::pipeline::proj_number;

/// Whether transverse Mercator export inserts `+approx`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TmercApprox {
    Yes,
    No,
}

/// Northern or southern UTM hemisphere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UtmHemisphere {
    North,
    South,
}

/// Name, authority code, and remarks passed to an operation constructor.
#[derive(Clone, Debug, Default)]
pub struct OpProps {
    name: Option<String>,
    code_space: Option<String>,
    code: Option<String>,
    remarks: String,
}

impl OpProps {
    /// Empty properties. The conversion name then defaults to the method name.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the object name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set one authority identifier.
    #[must_use]
    pub fn identifier(mut self, code_space: impl Into<String>, code: impl Into<String>) -> Self {
        self.code_space = Some(code_space.into());
        self.code = Some(code.into());
        self
    }

    /// Set the remark.
    #[must_use]
    pub fn remarks(mut self, remarks: impl Into<String>) -> Self {
        self.remarks = remarks.into();
        self
    }
}

impl OperationParameter {
    /// `isEquivalentTo` with the strict criterion.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        identified_names_match(self.name(), other.name(), criterion)
    }
}

impl OperationMethod {
    /// `isEquivalentTo` with the strict criterion.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    ///
    /// `Equivalent` ignores parameter order. `Strict` does not.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        if !identified_names_match(&self.name, &other.name, criterion) {
            return false;
        }
        if self.parameters.len() != other.parameters.len() {
            return false;
        }
        if criterion == Equivalence::Strict {
            return self
                .parameters
                .iter()
                .zip(&other.parameters)
                .all(|(left, right)| left.is_equivalent_to_with(right, criterion));
        }
        let mut used = vec![false; other.parameters.len()];
        for parameter in &self.parameters {
            let Some(index) = other
                .parameters
                .iter()
                .enumerate()
                .position(|(index, other)| {
                    !used[index] && parameter.is_equivalent_to_with(other, criterion)
                })
            else {
                return false;
            };
            used[index] = true;
        }
        true
    }
}

impl ParameterValue {
    /// `isEquivalentTo` with the strict criterion.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    ///
    /// Measures use a relative tolerance of `2e-10` under `Equivalent`.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        match (self, other) {
            (Self::Measure(left), Self::Measure(right)) => {
                measures_equivalent(left, right, criterion)
            }
            (Self::String(left), Self::String(right))
            | (Self::Filename(left), Self::Filename(right)) => left == right,
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            _ => false,
        }
    }
}

impl OperationParameterValue {
    /// `isEquivalentTo` with the strict criterion.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        self.parameter
            .is_equivalent_to_with(&other.parameter, criterion)
            && self.value.is_equivalent_to_with(&other.value, criterion)
    }
}

impl Transformation {
    /// Build a transformation from explicit parameters.
    ///
    /// # Errors
    ///
    /// The parameter list and the value list have different lengths.
    pub fn create(
        props: &OpProps,
        source: Crs,
        target: Crs,
        interpolation: Option<Crs>,
        method_name: &str,
        parameters: Vec<OperationParameter>,
        values: Vec<ParameterValue>,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        if parameters.len() != values.len() {
            return Err(FactoryError::Invalid(
                "Inconsistent number of parameters and parameter values".to_owned(),
            ));
        }
        let stored = parameters
            .iter()
            .zip(&values)
            .map(|(parameter, value)| Parameter {
                name: parameter.name().to_owned(),
                code_space: String::new(),
                code: String::new(),
                value: value.clone(),
            })
            .collect();
        Ok(Self {
            identified: identified_from(props, ""),
            method: OperationMethod::create(method_name, parameters),
            parameters: stored,
            source: Some(Box::new(source)),
            target: Some(Box::new(target)),
            interpolation: interpolation.map(Box::new),
            accuracy,
            accuracy_text: None,
            version: None,
            point_motion: false,
            inverted: false,
        })
    }

    /// Transformation with the method of EPSG `code`, `values` in its parameter
    /// order, and `domain` as its usage. Used for the ballpark operations.
    ///
    /// # Errors
    ///
    /// The method is not in the PROJ method maps, or `values` does not match its parameters.
    pub(crate) fn create_from_code(
        props: &OpProps,
        source: Crs,
        target: Crs,
        code: i32,
        values: Vec<ParameterValue>,
        domain: ObjectDomain,
    ) -> Result<Self, FactoryError> {
        let mapped = parammap::method_by_code(code).ok_or_else(|| {
            FactoryError::Invalid(format!("unknown operation method EPSG:{code}"))
        })?;
        let (method, parameters) = mapped_method(mapped, values)?;
        let mut identified = identified_from(props, mapped.wkt2_name);
        identified.domains = vec![domain];
        Ok(Self {
            identified,
            method,
            parameters,
            source: Some(Box::new(source)),
            target: Some(Box::new(target)),
            interpolation: None,
            accuracy: None,
            accuracy_text: None,
            version: None,
            point_motion: false,
            inverted: false,
        })
    }

    /// Inverse transformation: swapped CRS and an `Inverse of` name.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let mut inverse = self.clone();
        inverse.inverted = !self.inverted;
        std::mem::swap(&mut inverse.source, &mut inverse.target);
        for identifier in &mut inverse.identified.identifiers {
            identifier.code_space = inverse_code_space(&identifier.code_space);
        }
        if self.inverted {
            inverse.identified.name = self
                .name()
                .strip_prefix("Inverse of ")
                .unwrap_or(self.name())
                .to_owned();
        } else {
            inverse.identified.name = format!("Inverse of {}", self.name());
        }
        inverse
    }

    /// `isEquivalentTo` with the strict criterion.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.is_equivalent_to_with(other, Equivalence::Strict)
    }

    /// `isEquivalentTo` with `criterion`.
    ///
    /// The operation name is compared only for `Strict`, matching PROJ.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        if criterion == Equivalence::Strict
            && !identified_names_match(self.name(), other.name(), criterion)
        {
            return false;
        }
        if !self.method.is_equivalent_to_with(&other.method, criterion) {
            return false;
        }
        values_equivalent(&self.parameters, &other.parameters, criterion)
            && crs_option(self.source_crs(), other.source_crs())
            && crs_option(self.target_crs(), other.target_crs())
            && crs_option(self.interpolation_crs(), other.interpolation_crs())
            && self.accuracy == other.accuracy
            && self.inverted == other.inverted
    }
}

/// The code space of an inverse operation's identifier: `EPSG` becomes `INVERSE(EPSG)`,
/// and an `INVERSE(...)` one goes back to its authority, as `addModifiedIdentifier` does.
fn inverse_code_space(code_space: &str) -> String {
    match code_space
        .strip_prefix("INVERSE(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        Some(authority) => authority.to_owned(),
        None => format!("INVERSE({code_space})"),
    }
}

fn crs_option(left: Option<&Crs>, right: Option<&Crs>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.is_equivalent_to(right),
        (None, None) => true,
        _ => false,
    }
}

fn values_equivalent(left: &[Parameter], right: &[Parameter], criterion: Equivalence) -> bool {
    if criterion == Equivalence::Strict {
        return left.len() == right.len()
            && left
                .iter()
                .zip(right)
                .all(|(left, right)| parameter_equivalent(left, right, criterion));
    }
    if left.len() != right.len() {
        return false;
    }
    let mut used = vec![false; right.len()];
    for parameter in left {
        let mut same_name = false;
        let mut found = false;
        for (index, other) in right.iter().enumerate() {
            if used[index] {
                continue;
            }
            if parameter_equivalent(parameter, other, criterion) {
                used[index] = true;
                found = true;
                break;
            } else if identified_names_match(&parameter.name, &other.name, criterion) {
                same_name = true;
                used[index] = true;
                break;
            }
        }
        if !found || same_name {
            return false;
        }
    }
    true
}

fn parameter_equivalent(left: &Parameter, right: &Parameter, criterion: Equivalence) -> bool {
    identified_names_match(&left.name, &right.name, criterion)
        && left.value.is_equivalent_to_with(&right.value, criterion)
}

fn measures_equivalent(left: &Measure, right: &Measure, criterion: Equivalence) -> bool {
    if criterion == Equivalence::Strict {
        return left.value() == right.value()
            && left.unit().name() == right.unit().name()
            && left.unit().conversion_to_si() == right.unit().conversion_to_si();
    }
    let left_si = left.si_value();
    let right_si = right.si_value();
    if left_si.is_infinite() && right_si.is_infinite() {
        return left_si.is_sign_positive() == right_si.is_sign_positive();
    }
    (left_si - right_si).abs() <= 2e-10 * left_si.abs()
}

fn identified_names_match(left: &str, right: &str, criterion: Equivalence) -> bool {
    if criterion == Equivalence::Strict {
        return left.eq_ignore_ascii_case(right);
    }
    names_match(left, right)
        || (left == "unknown" && right == "unnamed")
        || (left == "unnamed" && right == "unknown")
}

pub(super) fn names_match(left: &str, right: &str) -> bool {
    canonical(left) == canonical(right)
}

fn canonical(name: &str) -> String {
    name.chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '/' | '{' | '}'))
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

impl super::model::Conversion {
    /// `isEquivalentTo` with `criterion`.
    ///
    /// Lambert 2SP standard parallels may be exchanged under `Equivalent`.
    #[must_use]
    pub fn is_equivalent_to_with(&self, other: &Self, criterion: Equivalence) -> bool {
        if criterion == Equivalence::Strict {
            return self.is_equivalent_to(other);
        }
        if !self.method.is_equivalent_to_with(&other.method, criterion) {
            return false;
        }
        if values_equivalent(&self.parameters, &other.parameters, criterion) {
            return true;
        }
        self.method.code == "9802"
            && other.method.code == "9802"
            && lcc2sp_parallels_swapped(self, other)
    }

    /// Build a conversion whose method name and code are supplied explicitly.
    ///
    /// This is the form `validateParameters` uses for a method that is not the
    /// official spelling of its EPSG code.
    #[must_use]
    pub fn create_with_method(
        method_name: &str,
        method_code_space: &str,
        method_code: &str,
        parameters: Vec<Parameter>,
    ) -> Self {
        Self {
            identified: Identified::named(String::new()),
            method: OperationMethod::named_code(
                method_name.to_owned(),
                method_code_space.to_owned(),
                method_code.to_owned(),
            ),
            parameters,
            version: None,
        }
    }

    /// Parameter value used by [`Self::create_with_method`].
    #[must_use]
    pub fn parameter(name: &str, code_space: &str, code: &str, value: ParameterValue) -> Parameter {
        Parameter {
            name: name.to_owned(),
            code_space: code_space.to_owned(),
            code: code.to_owned(),
            value,
        }
    }

    /// Parameters that are missing or not part of the method.
    ///
    /// The messages are the ones PROJ `SingleOperation::validateParameters` emits.
    #[must_use]
    pub fn validate_parameters(&self) -> Vec<String> {
        let Some(mapped) = find_for_validate(&self.method) else {
            return vec![format!("Unknown method {}", self.method.name())];
        };
        let mut messages = Vec::new();
        if mapped.wkt2_name != self.method.name() {
            if names_match(mapped.wkt2_name, self.method.name()) {
                messages.push(format!(
                    "Method name {} is equivalent to official {} but not strictly equal",
                    self.method.name(),
                    mapped.wkt2_name
                ));
            } else {
                messages.push(format!(
                    "Method name {}, matched to {}, through its EPSG code has not an equivalent name",
                    self.method.name(),
                    mapped.wkt2_name
                ));
            }
        }
        let method_code = self.method.code.parse::<i32>().unwrap_or(0);
        if method_code != 0 && method_code != mapped.epsg_code {
            messages.push(format!(
                "Method of EPSG code {method_code} does not match official code ({})",
                mapped.epsg_code
            ));
        }
        let mut matched = vec![false; self.parameters.len()];
        for param in mapped.params {
            let found = self.parameters.iter().position(|have| {
                let have_code = have.code.parse::<i32>().unwrap_or(0);
                (param.epsg_code != 0 && have_code == param.epsg_code)
                    || have.name.eq_ignore_ascii_case(param.wkt2_name)
            });
            let Some(index) = found else {
                messages.push(format!(
                    "Cannot find expected parameter {}",
                    param.wkt2_name
                ));
                continue;
            };
            matched[index] = true;
            let have = &self.parameters[index];
            if param.wkt2_name != have.name {
                if have.name.eq_ignore_ascii_case(param.wkt2_name) {
                    messages.push(format!(
                        "Parameter name {} is equivalent to official {} but not strictly equal",
                        have.name, param.wkt2_name
                    ));
                } else {
                    messages.push(format!(
                        "Parameter name {}, matched to {}, through its EPSG code has not an equivalent name",
                        have.name, param.wkt2_name
                    ));
                }
            }
            let have_code = have.code.parse::<i32>().unwrap_or(0);
            if have_code != 0 && have_code != param.epsg_code {
                messages.push(format!(
                    "Parameter of EPSG code {have_code} does not match official code ({})",
                    param.epsg_code
                ));
            }
        }
        for (index, have) in self.parameters.iter().enumerate() {
            if matched[index] {
                continue;
            }
            let have_code = have.code.parse::<i32>().unwrap_or(0);
            let known = mapped.params.iter().any(|param| {
                (param.epsg_code != 0 && have_code == param.epsg_code)
                    || have.name.eq_ignore_ascii_case(param.wkt2_name)
            });
            if !known {
                messages.push(format!(
                    "Parameter {} found but not expected for this method",
                    have.name
                ));
            }
        }
        messages
    }

    /// `exportToPROJString` for a conversion that has no source or target CRS.
    ///
    /// # Errors
    ///
    /// The method cannot be written as a PROJ string, or a parameter value is
    /// outside the range PROJ accepts.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        self.export_to_proj_inner(TmercApprox::No)
    }

    /// Same as [`Self::export_to_proj_string`], with `+approx` on transverse Mercator.
    ///
    /// # Errors
    ///
    /// The method cannot be written as a PROJ string.
    pub fn export_to_proj_string_approx_tmerc(&self) -> Result<String, FactoryError> {
        self.export_to_proj_inner(TmercApprox::Yes)
    }

    fn export_to_proj_inner(&self, approx_tmerc: TmercApprox) -> Result<String, FactoryError> {
        let mapped = lookup_method(&self.method).ok_or_else(|| {
            FactoryError::Format(format!(
                "Unsupported conversion method: {}",
                self.method.name()
            ))
        })?;
        let code = mapped.epsg_code;
        if code == 9808 {
            let easting = self.numeric_si(8806);
            let northing = self.numeric_si(8807);
            if easting != 0.0 {
                return Err(FactoryError::Format(
                    "Unsupported value for False easting".to_owned(),
                ));
            }
            if northing != 0.0 {
                return Err(FactoryError::Format(
                    "Unsupported value for False northing".to_owned(),
                ));
            }
        }
        if code == 9805 {
            if let Some(scale) = self.measure_of_code(8805) {
                if (scale.si_value() - 1.0).abs() > 1e-10 {
                    return Err(FactoryError::Format(
                        "Unexpected presence of scale factor in Mercator (variant B)".to_owned(),
                    ));
                }
            }
        }
        if code == 9804 || code == 9805 {
            let latitude = self.numeric_degrees(8801);
            if latitude != 0.0 {
                return Err(FactoryError::Format(
                    "Unsupported value for Latitude of natural origin".to_owned(),
                ));
            }
        }
        if code == 1041 || code == 1043 {
            let colatitude = self.numeric_degrees(1036);
            let parallel = self.numeric_degrees(8818);
            if (colatitude - 30.288_139_7).abs() > 1e-7 {
                return Err(FactoryError::Format(
                    "Unsupported value for Co-latitude of cone axis".to_owned(),
                ));
            }
            if (parallel - 78.5).abs() > 1e-8 {
                return Err(FactoryError::Format(
                    "Unsupported value for Latitude of pseudo standard parallel".to_owned(),
                ));
            }
        }
        if matches!(code, 9812 | 9815) {
            let azimuth = self.numeric_degrees(8813);
            let gamma = self.numeric_degrees(8814);
            if (azimuth - 90.0).abs() < 1e-4 && (gamma - 90.0).abs() < 1e-4 {
                let mut parts = vec!["+proj=somerc".to_owned()];
                push_number(&mut parts, "lat_0", self.numeric_degrees(8811));
                push_number(&mut parts, "lon_0", self.numeric_degrees(8812));
                push_number(&mut parts, "k_0", self.numeric_si(8815));
                let (x_code, y_code) = if code == 9812 {
                    (8806, 8807)
                } else {
                    (8816, 8817)
                };
                push_number(&mut parts, "x_0", self.numeric_si(x_code));
                push_number(&mut parts, "y_0", self.numeric_si(y_code));
                return Ok(parts.join(" "));
            }
        }
        if code == 9807 {
            if let Some((zone, hemisphere)) = self.utm_zone() {
                let mut parts = vec!["+proj=utm".to_owned()];
                if approx_tmerc == TmercApprox::Yes {
                    parts.push("+approx".to_owned());
                }
                parts.push(format!("+zone={zone}"));
                if hemisphere == UtmHemisphere::South {
                    parts.push("+south".to_owned());
                }
                return Ok(parts.join(" "));
            }
        }
        let Some(proj_name) = mapped.proj_name else {
            return Err(FactoryError::Format(format!(
                "Unsupported conversion method: {}",
                self.method.name()
            )));
        };
        let mut parts = vec![format!("+proj={proj_name}")];
        if approx_tmerc == TmercApprox::Yes && (code == 9807 || code == 1111) {
            parts.push("+approx".to_owned());
        }
        if let Some(aux) = mapped.proj_aux {
            push_aux(&mut parts, aux);
        }
        if code == 9829 {
            let latitude = self.numeric_degrees(8832);
            push_number(
                &mut parts,
                "lat_0",
                if latitude >= 0.0 { 90.0 } else { -90.0 },
            );
        }
        for param in mapped.params {
            let Some(proj_name) = param.proj_name else {
                continue;
            };
            if param.epsg_code == 8814
                && self.measure_of(param).is_none()
                && matches!(code, 9812 | 9815)
            {
                continue;
            }
            let value = self.proj_value(param);
            push_number(&mut parts, proj_name, value);
            if code == 9801 && proj_name == "lat_1" {
                push_number(&mut parts, "lat_0", value);
            }
        }
        Ok(parts.join(" "))
    }

    pub(crate) fn utm_zone(&self) -> Option<(i32, UtmHemisphere)> {
        if self.method.code != "9807" {
            return None;
        }
        let latitude = self.numeric_degrees(8801);
        if (latitude - 0.0).abs() >= 1e-10 {
            return None;
        }
        let longitude = self.numeric_degrees(8802);
        let zone = (longitude + 183.0) / 6.0;
        if !(0.9..60.1).contains(&zone) || (zone - zone.round()).abs() >= 1e-10 {
            return None;
        }
        let scale = self.measure_of_code(8805)?;
        if scale.unit().kind() != UnitKind::Scale || (scale.value() - 0.9996).abs() >= 1e-10 {
            return None;
        }
        let easting = self.measure_of_code(8806)?;
        if easting.value() != 500_000.0 || easting.unit().name() != "metre" {
            return None;
        }
        let northing = self.measure_of_code(8807)?;
        if northing.unit().name() != "metre" {
            return None;
        }
        let hemisphere = if (northing.value() - 0.0).abs() < 1e-10 {
            UtmHemisphere::North
        } else if (northing.value() - 10_000_000.0).abs() < 1e-10 {
            UtmHemisphere::South
        } else {
            return None;
        };
        let zone_number =
            (1..=60).find(|candidate| (f64::from(*candidate) - zone).abs() < 1e-10)?;
        Some((zone_number, hemisphere))
    }

    pub(crate) fn numeric_degrees(&self, code: i32) -> f64 {
        self.measure_of_code(code)
            .map(|measure| measure.convert_to(&super::model::UnitOfMeasure::degree()))
            .unwrap_or(0.0)
    }

    pub(crate) fn numeric_si(&self, code: i32) -> f64 {
        self.measure_of_code(code)
            .map(Measure::si_value)
            .unwrap_or(0.0)
    }

    fn proj_value(&self, param: &ParamMap) -> f64 {
        let Some(measure) = self.measure_of(param) else {
            if param.epsg_code == 8805 {
                return 1.0;
            }
            return 0.0;
        };
        if param.unit == UnitKind::Angular {
            measure.convert_to(&super::model::UnitOfMeasure::degree())
        } else {
            measure.si_value()
        }
    }

    fn measure_of_code(&self, code: i32) -> Option<&Measure> {
        let by_code = self.parameters.iter().find_map(|parameter| {
            if parameter.code.parse::<i32>().ok() == Some(code) {
                measure_ref(&parameter.value)
            } else {
                None
            }
        });
        if by_code.is_some() {
            return by_code;
        }
        self.parameters.iter().find_map(|parameter| {
            if epsg_code_of_parameter_name(&parameter.name) == Some(code) {
                measure_ref(&parameter.value)
            } else {
                None
            }
        })
    }

    fn measure_of(&self, param: &ParamMap) -> Option<&Measure> {
        if param.epsg_code != 0 {
            if let Some(measure) = self.measure_of_code(param.epsg_code) {
                return Some(measure);
            }
        }
        self.parameters.iter().find_map(|parameter| {
            if names_match(&parameter.name, param.wkt2_name) {
                match &parameter.value {
                    ParameterValue::Measure(measure) => Some(measure),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    fn from_code(
        props: &OpProps,
        code: i32,
        values: Vec<ParameterValue>,
    ) -> Result<Self, FactoryError> {
        let mapped = parammap::method_by_code(code).ok_or_else(|| {
            FactoryError::Invalid(format!("unknown operation method EPSG:{code}"))
        })?;
        Self::from_mapped(props, mapped, values)
    }

    fn from_name(
        props: &OpProps,
        name: &str,
        values: Vec<ParameterValue>,
    ) -> Result<Self, FactoryError> {
        let mapped = parammap::method_by_name(name)
            .ok_or_else(|| FactoryError::Invalid(format!("unknown operation method {name}")))?;
        Self::from_mapped(props, mapped, values)
    }

    fn from_mapped(
        props: &OpProps,
        mapped: &MethodMap,
        values: Vec<ParameterValue>,
    ) -> Result<Self, FactoryError> {
        let (method, parameters) = mapped_method(mapped, values)?;
        Ok(Self {
            identified: identified_from(props, mapped.wkt2_name),
            method,
            parameters,
            version: None,
        })
    }

    /// UTM northern zone `zone` (`1..=60`).
    ///
    /// # Errors
    ///
    /// `zone` is outside 1..=60.
    pub fn create_utm_north(props: &OpProps, zone: i32) -> Result<Self, FactoryError> {
        Self::create_utm_hemisphere(props, zone, UtmHemisphere::North)
    }

    /// UTM southern zone `zone` (`1..=60`).
    ///
    /// # Errors
    ///
    /// `zone` is outside 1..=60.
    pub fn create_utm_south(props: &OpProps, zone: i32) -> Result<Self, FactoryError> {
        Self::create_utm_hemisphere(props, zone, UtmHemisphere::South)
    }

    fn create_utm_hemisphere(
        props: &OpProps,
        zone: i32,
        hemisphere: UtmHemisphere,
    ) -> Result<Self, FactoryError> {
        if !(1..=60).contains(&zone) {
            return Err(FactoryError::Invalid("Invalid zone number".to_owned()));
        }
        let mut props = props.clone();
        if props.name.is_none() {
            let letter = match hemisphere {
                UtmHemisphere::North => 'N',
                UtmHemisphere::South => 'S',
            };
            let base = match hemisphere {
                UtmHemisphere::North => 16_000,
                UtmHemisphere::South => 16_100,
            };
            props.name = Some(format!("UTM zone {zone}{letter}"));
            props.code_space = Some("EPSG".to_owned());
            props.code = Some((base + zone).to_string());
        }
        let longitude = f64::from(zone) * 6.0 - 183.0;
        let northing = match hemisphere {
            UtmHemisphere::North => 0.0,
            UtmHemisphere::South => 10_000_000.0,
        };
        Self::from_code(
            &props,
            9807,
            vec![
                ParameterValue::angle(0.0),
                ParameterValue::angle(longitude),
                ParameterValue::scale(0.9996),
                ParameterValue::length(500_000.0),
                ParameterValue::length(northing),
            ],
        )
    }

    /// Transverse Mercator, EPSG:9807.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_transverse_mercator(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9807,
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Gauss-Schreiber Transverse Mercator.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_gauss_schreiber_transverse_mercator(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Gauss Schreiber Transverse Mercator",
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Transverse Mercator south oriented, EPSG:9808.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_transverse_mercator_south_oriented(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9808,
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Two Point Equidistant.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_two_point_equidistant(
        props: &OpProps,
        latitude_1: f64,
        longitude_1: f64,
        latitude_2: f64,
        longitude_2: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Two Point Equidistant",
            vec![
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(longitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::angle(longitude_2),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Tunisia Mining Grid, EPSG:9816.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_tunisia_mining_grid(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9816,
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Albers Equal Area, EPSG:9822.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_albers_equal_area(
        props: &OpProps,
        latitude_origin: f64,
        longitude_origin: f64,
        latitude_1: f64,
        latitude_2: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9822,
            vec![
                ParameterValue::angle(latitude_origin),
                ParameterValue::angle(longitude_origin),
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// Lambert Conic Conformal (1SP), EPSG:9801.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_conic_conformal_1sp(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9801,
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Lambert Conic Conformal (1SP variant B), EPSG:1102.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_conic_conformal_1sp_variant_b(
        props: &OpProps,
        latitude: f64,
        scale: f64,
        latitude_false: f64,
        longitude_false: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            1102,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::scale(scale),
                ParameterValue::angle(latitude_false),
                ParameterValue::angle(longitude_false),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// Lambert Conic Conformal (2SP), EPSG:9802.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_conic_conformal_2sp(
        props: &OpProps,
        latitude_origin: f64,
        longitude_origin: f64,
        latitude_1: f64,
        latitude_2: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9802,
            vec![
                ParameterValue::angle(latitude_origin),
                ParameterValue::angle(longitude_origin),
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// Lambert Conic Conformal (2SP Michigan), EPSG:1051.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_conic_conformal_2sp_michigan(
        props: &OpProps,
        latitude_origin: f64,
        longitude_origin: f64,
        latitude_1: f64,
        latitude_2: f64,
        easting: f64,
        northing: f64,
        ellipsoid_scale: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            1051,
            vec![
                ParameterValue::angle(latitude_origin),
                ParameterValue::angle(longitude_origin),
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
                ParameterValue::scale(ellipsoid_scale),
            ],
        )
    }

    /// Lambert Conic Conformal (2SP Belgium), EPSG:9803.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_conic_conformal_2sp_belgium(
        props: &OpProps,
        latitude_origin: f64,
        longitude_origin: f64,
        latitude_1: f64,
        latitude_2: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9803,
            vec![
                ParameterValue::angle(latitude_origin),
                ParameterValue::angle(longitude_origin),
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// Azimuthal Equidistant.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_azimuthal_equidistant(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Azimuthal Equidistant",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Guam Projection.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_guam_projection(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Guam Projection",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Bonne.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_bonne(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Bonne",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Lambert Cylindrical Equal Area (Spherical).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_cylindrical_equal_area_spherical(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Lambert Cylindrical Equal Area (Spherical)",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Lambert Cylindrical Equal Area.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_cylindrical_equal_area(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Lambert Cylindrical Equal Area",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Cassini-Soldner.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_cassini_soldner(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Cassini-Soldner",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Equidistant Conic, EPSG:1119.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_equidistant_conic(
        props: &OpProps,
        latitude_origin: f64,
        longitude_origin: f64,
        latitude_1: f64,
        latitude_2: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Equidistant Conic",
            vec![
                ParameterValue::angle(latitude_origin),
                ParameterValue::angle(longitude_origin),
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// Eckert I through VI. `which` is 1..=6.
    ///
    /// # Errors
    ///
    /// `which` is not 1..=6.
    pub fn create_eckert(
        props: &OpProps,
        which: u8,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        let name = match which {
            1 => "Eckert I",
            2 => "Eckert II",
            3 => "Eckert III",
            4 => "Eckert IV",
            5 => "Eckert V",
            6 => "Eckert VI",
            _ => {
                return Err(FactoryError::Invalid(
                    "Eckert projection number must be 1 to 6".to_owned(),
                ))
            }
        };
        Self::from_name(
            props,
            name,
            vec![
                ParameterValue::angle(longitude),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Equidistant Cylindrical.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_equidistant_cylindrical(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Equidistant Cylindrical",
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(0.0),
                ParameterValue::angle(longitude),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Equidistant Cylindrical (Spherical).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_equidistant_cylindrical_spherical(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Equidistant Cylindrical (Spherical)",
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(0.0),
                ParameterValue::angle(longitude),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Gall Stereographic.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_gall(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Gall Stereographic",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Goode Homolosine.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_goode_homolosine(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Goode Homolosine",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Interrupted Goode Homolosine.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_interrupted_goode_homolosine(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Interrupted Goode Homolosine",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Geostationary Satellite (Sweep X).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geostationary_satellite_sweep_x(
        props: &OpProps,
        longitude: f64,
        height: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Geostationary Satellite (Sweep X)",
            vec![
                ParameterValue::angle(longitude),
                ParameterValue::length(height),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Geostationary Satellite (Sweep Y).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geostationary_satellite_sweep_y(
        props: &OpProps,
        longitude: f64,
        height: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Geostationary Satellite (Sweep Y)",
            vec![
                ParameterValue::angle(longitude),
                ParameterValue::length(height),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Gnomonic.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_gnomonic(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Gnomonic",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Hotine Oblique Mercator variant A, EPSG:9812.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_hotine_oblique_mercator_variant_a(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        azimuth: f64,
        gamma: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9812,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::angle(azimuth),
                ParameterValue::angle(gamma),
                ParameterValue::scale(scale),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Hotine Oblique Mercator variant B, EPSG:9815.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_hotine_oblique_mercator_variant_b(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        azimuth: f64,
        gamma: f64,
        scale: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9815,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::angle(azimuth),
                ParameterValue::angle(gamma),
                ParameterValue::scale(scale),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// Krovak north oriented, EPSG:1041.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_krovak_north_oriented(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        colatitude: f64,
        pseudo_parallel: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            1041,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::angle(colatitude),
                ParameterValue::angle(pseudo_parallel),
                ParameterValue::scale(scale),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Krovak, EPSG:9819.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_krovak(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        colatitude: f64,
        pseudo_parallel: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9819,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::angle(colatitude),
                ParameterValue::angle(pseudo_parallel),
                ParameterValue::scale(scale),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Mercator variant A, EPSG:9804.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_mercator_variant_a(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9804,
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Mercator variant B, EPSG:9805.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_mercator_variant_b(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9805,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Hotine Oblique Mercator Two Point Natural Origin. It has no EPSG method code.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_hotine_oblique_mercator_two_point_natural_origin(
        props: &OpProps,
        latitude_centre: f64,
        latitude_1: f64,
        longitude_1: f64,
        latitude_2: f64,
        longitude_2: f64,
        scale: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Hotine Oblique Mercator Two Point Natural Origin",
            vec![
                ParameterValue::angle(latitude_centre),
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(longitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::angle(longitude_2),
                ParameterValue::scale(scale),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// International Map of the World Polyconic. It has no EPSG method code.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_international_map_world_polyconic(
        props: &OpProps,
        longitude: f64,
        latitude_1: f64,
        latitude_2: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "International Map of the World Polyconic",
            vec![
                ParameterValue::angle(longitude),
                ParameterValue::angle(latitude_1),
                ParameterValue::angle(latitude_2),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Laborde Oblique Mercator, EPSG:9813.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_laborde_oblique_mercator(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        azimuth: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9813,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::angle(azimuth),
                ParameterValue::scale(scale),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Popular Visualisation Pseudo Mercator, EPSG:1024.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_popular_visualisation_pseudo_mercator(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            1024,
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Mercator Spherical, EPSG:1026.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_mercator_spherical(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            1026,
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Lambert Azimuthal Equal Area.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_lambert_azimuthal_equal_area(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Lambert Azimuthal Equal Area",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Miller Cylindrical.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_miller_cylindrical(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Miller Cylindrical",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Mollweide.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_mollweide(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Mollweide",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Polar Stereographic variant A, EPSG:9810.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_polar_stereographic_variant_a(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9810,
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Polar Stereographic variant B, EPSG:9829.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_polar_stereographic_variant_b(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_code(
            props,
            9829,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Change of vertical unit, EPSG:1069, with a scale factor.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_change_vertical_unit(props: &OpProps, factor: f64) -> Result<Self, FactoryError> {
        Self::from_code(props, 1069, vec![ParameterValue::scale(factor)])
    }

    /// Change of vertical unit without a conversion factor, EPSG:1104.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_change_vertical_unit_no_conv_factor(
        props: &OpProps,
    ) -> Result<Self, FactoryError> {
        Self::from_code(props, 1104, Vec::new())
    }

    /// Geographic/geocentric conversion, EPSG:9602.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geographic_geocentric(props: &OpProps) -> Result<Self, FactoryError> {
        Self::from_code(props, 9602, Vec::new())
    }

    /// Build a conversion whose WKT2 method name is `name`.
    ///
    /// # Errors
    ///
    /// `name` is not a known method, or `values` does not match its parameters.
    pub fn create_by_name(
        props: &OpProps,
        name: &str,
        values: Vec<ParameterValue>,
    ) -> Result<Self, FactoryError> {
        Self::from_name(props, name, values)
    }
}

/// The method and parameters of `mapped`, with `values` in parameter order.
fn mapped_method(
    mapped: &MethodMap,
    values: Vec<ParameterValue>,
) -> Result<(OperationMethod, Vec<Parameter>), FactoryError> {
    if values.len() != mapped.params.len() {
        return Err(FactoryError::Invalid(
            "Inconsistent number of parameters and parameter values".to_owned(),
        ));
    }
    let parameters = mapped
        .params
        .iter()
        .zip(values)
        .map(|(param, value)| Parameter {
            name: param.wkt2_name.to_owned(),
            code_space: if param.epsg_code == 0 {
                String::new()
            } else {
                "EPSG".to_owned()
            },
            code: if param.epsg_code == 0 {
                String::new()
            } else {
                param.epsg_code.to_string()
            },
            value,
        })
        .collect();
    let method_code = if mapped.epsg_code == 0 {
        String::new()
    } else {
        mapped.epsg_code.to_string()
    };
    let method_space = if method_code.is_empty() {
        String::new()
    } else {
        "EPSG".to_owned()
    };
    let method =
        OperationMethod::named_code(mapped.wkt2_name.to_owned(), method_space, method_code);
    Ok((method, parameters))
}

fn angles_scale_lengths(
    latitude: f64,
    longitude: f64,
    scale: f64,
    false_easting: f64,
    false_northing: f64,
) -> Vec<ParameterValue> {
    vec![
        ParameterValue::angle(latitude),
        ParameterValue::angle(longitude),
        ParameterValue::scale(scale),
        ParameterValue::length(false_easting),
        ParameterValue::length(false_northing),
    ]
}

fn angles_lengths(
    latitude: f64,
    longitude: f64,
    false_easting: f64,
    false_northing: f64,
) -> Vec<ParameterValue> {
    vec![
        ParameterValue::angle(latitude),
        ParameterValue::angle(longitude),
        ParameterValue::length(false_easting),
        ParameterValue::length(false_northing),
    ]
}

fn long_lengths(longitude: f64, false_easting: f64, false_northing: f64) -> Vec<ParameterValue> {
    vec![
        ParameterValue::angle(longitude),
        ParameterValue::length(false_easting),
        ParameterValue::length(false_northing),
    ]
}

fn identified_from(props: &OpProps, default_name: &str) -> Identified {
    let mut identified = Identified::named(
        props
            .name
            .clone()
            .unwrap_or_else(|| default_name.to_owned()),
    );
    if let (Some(space), Some(code)) = (&props.code_space, &props.code) {
        identified = identified.with_id(space, code);
    }
    identified.remarks = props.remarks.clone();
    identified
}

fn lcc2sp_parallels_swapped(
    left: &super::model::Conversion,
    right: &super::model::Conversion,
) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() <= 1e-10 * a.abs().max(1.0);
    near(left.numeric_degrees(8823), right.numeric_degrees(8824))
        && near(left.numeric_degrees(8824), right.numeric_degrees(8823))
        && [8821, 8822, 8826, 8827]
            .into_iter()
            .all(|code| near(left.numeric_si(code), right.numeric_si(code)))
}

fn find_for_validate(method: &OperationMethod) -> Option<&'static MethodMap> {
    let code = method.code.parse::<i32>().unwrap_or(0);
    if code != 0 {
        if let Some(mapped) = parammap::PROJECTION_METHODS
            .iter()
            .chain(parammap::OTHER_METHODS)
            .find(|mapped| mapped.epsg_code == code)
        {
            return Some(mapped);
        }
    }
    parammap::method_by_name(&method.name)
}

fn lookup_method(method: &OperationMethod) -> Option<&'static MethodMap> {
    let code = method.code.parse::<i32>().unwrap_or(0);
    if code != 0 {
        let named = parammap::PROJECTION_METHODS
            .iter()
            .chain(parammap::OTHER_METHODS)
            .find(|mapped| mapped.epsg_code == code && names_match(mapped.wkt2_name, &method.name));
        if named.is_some() {
            return named;
        }
        if let Some(mapped) = parammap::method_by_code(code) {
            return Some(mapped);
        }
    }
    parammap::method_by_name(&method.name)
}

fn measure_ref(value: &ParameterValue) -> Option<&Measure> {
    match value {
        ParameterValue::Measure(measure) => Some(measure),
        _ => None,
    }
}

fn epsg_code_of_parameter_name(name: &str) -> Option<i32> {
    parammap::PROJECTION_METHODS
        .iter()
        .chain(parammap::OTHER_METHODS)
        .find_map(|method| {
            method.params.iter().find_map(|param| {
                (param.epsg_code != 0 && names_match(param.wkt2_name, name))
                    .then_some(param.epsg_code)
            })
        })
}

fn push_aux(parts: &mut Vec<String>, aux: &str) {
    if let Some((key, value)) = aux.split_once('=') {
        parts.push(format!("+{key}={value}"));
    } else {
        parts.push(format!("+{aux}"));
    }
}

fn push_number(parts: &mut Vec<String>, name: &str, value: f64) {
    parts.push(format!("+{name}={}", proj_number(value)));
}
