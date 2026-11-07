//! Constructors and PROJ-string export for conversions and transformations.
//!
//! The method and parameter names are the PROJ 9.9 maps in [`super::parammap`].

use super::equiv::Equivalence;
use super::factory::geographic_of;
use super::model::{
    ConcatenatedOperation, ConcatenatedStep, Conversion, CoordinateOperation, Crs, FactoryError,
    Identified, Measure, ObjectDomain, OperationMethod, OperationParameter,
    OperationParameterValue, Parameter, ParameterValue, Transformation, UnitKind,
};
use super::parammap::{self, MethodMap, ParamMap};
use super::pipeline::proj_number;

mod projections;
mod transformations;

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

/// Horizontal axis order of [`Conversion::create_axis_order_reversal`].
///
/// PROJ takes a `bool`. The two EPSG conversions are distinct objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisOrderReversal {
    /// EPSG:15498, method 9843, two horizontal axes.
    TwoD,
    /// EPSG:15499, method 9844, the horizontal axes of a geographic 3D CRS.
    Geographic3d,
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
    /// `createPropertiesForInverse(method)`: the `Inverse of ` prefix added or removed, and the
    /// identifier moved to the `INVERSE(...)` code space or back.
    #[must_use]
    pub(crate) fn inverse(&self) -> Self {
        let mut inverse = self.clone();
        inverse.name = match self.name.strip_prefix("Inverse of ") {
            Some(rest) => rest.to_owned(),
            None => format!("Inverse of {}", self.name),
        };
        if !inverse.code_space.is_empty() {
            inverse.code_space = inverse_code_space(&self.code_space);
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
    /// Parameters that are missing or not part of the method, as
    /// `SingleOperation::validateParameters` reports them.
    #[must_use]
    pub fn validate_parameters(&self) -> Vec<String> {
        validate_method_parameters(&self.method, &self.parameters)
    }

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
        // The parameter's identifier is its EPSG code, which the export reads (`getEPSGCode`).
        let stored = parameters
            .iter()
            .zip(&values)
            .map(|(parameter, value)| {
                let (code_space, code) = parameter
                    .identified
                    .identifiers
                    .first()
                    .map_or((String::new(), String::new()), |identifier| {
                        (identifier.code_space.clone(), identifier.code.clone())
                    });
                Parameter {
                    name: parameter.name().to_owned(),
                    code_space,
                    code,
                    value: value.clone(),
                }
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
            source_epoch: None,
            target_epoch: None,
            inverted: false,
        })
    }

    /// As [`Self::create`], with a method that carries an identifier.
    ///
    /// # Errors
    ///
    /// The method's parameter list and the value list have different lengths.
    pub fn create_with_operation_method(
        props: &OpProps,
        source: Crs,
        target: Crs,
        interpolation: Option<Crs>,
        method: OperationMethod,
        values: Vec<ParameterValue>,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let mut transformation = Self::create(
            props,
            source,
            target,
            interpolation,
            &method.name,
            method.parameters.clone(),
            values,
            accuracy,
        )?;
        transformation.method = method;
        Ok(transformation)
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
            source_epoch: None,
            target_epoch: None,
            inverted: false,
        })
    }

    /// Inverse transformation: swapped CRS, negated translations for the methods
    /// PROJ inverts that way, and the name from `createPropertiesForInverse`.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let mut inverse = self.clone();
        if self.has_translation_inverse() {
            // PROJ writes the inverse of these methods with negated values, not `+inv`.
            inverse.negate_values();
        } else if !self.point_motion {
            inverse.inverted = !self.inverted;
        }
        std::mem::swap(&mut inverse.source, &mut inverse.target);
        std::mem::swap(&mut inverse.source_epoch, &mut inverse.target_epoch);
        inverse.identified.name = if self.point_motion {
            point_motion_name(self.name(), inverse.source_epoch, inverse.target_epoch)
        } else {
            super::geodetic_ops::inverse_operation_name(
                self.name(),
                self.source_crs(),
                self.target_crs(),
                super::geodetic_ops::NamedOperationKind::Transformation,
            )
        };
        for identifier in &mut inverse.identified.identifiers {
            identifier.code_space = inverse_code_space(&identifier.code_space);
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
            // A geocentric translation, position vector and coordinate frame rotation
            // are the same operation when their `+towgs84` values agree.
            if criterion == Equivalence::Equivalent && helmert_parameters_match(self, other) {
                return crs_option(self.source_crs(), other.source_crs())
                    && crs_option(self.target_crs(), other.target_crs())
                    && crs_option(self.interpolation_crs(), other.interpolation_crs())
                    && self.accuracy == other.accuracy
                    && self.inverted == other.inverted;
            }
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

/// `getTOWGS84Parameters` values within `1e-10` relative tolerance.
fn helmert_parameters_match(left: &Transformation, right: &Transformation) -> bool {
    let (Ok(Some(left)), Ok(Some(right))) = (left.towgs84_strict(), right.towgs84_strict()) else {
        return false;
    };
    left.iter().zip(right).all(|(left, right)| {
        let difference = (left - right).abs();
        difference <= 1e-10 * left.abs()
    })
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

/// `createPropertiesForInverse` for a stored concatenated operation: the `Inverse of`
/// prefix is removed, or added with the name quoted when it joins steps with ` + `.
fn concatenated_inverse_name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("Inverse of ") {
        return rest
            .strip_prefix('\'')
            .and_then(|quoted| quoted.strip_suffix('\''))
            .unwrap_or(rest)
            .to_owned();
    }
    if name.contains(" + ") {
        format!("Inverse of '{name}'")
    } else {
        format!("Inverse of {name}")
    }
}

impl Conversion {
    /// Build a conversion from explicit parameters, as `proj_create_conversion`.
    ///
    /// # Errors
    ///
    /// The parameter list and the value list have different lengths.
    pub fn create(
        props: &OpProps,
        method_name: &str,
        parameters: Vec<OperationParameter>,
        values: Vec<ParameterValue>,
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
            source: None,
            target: None,
            version: None,
            inverted: false,
            interpolation: None,
        })
    }

    /// `PROJBasedOperation::create`: the PROJ string is the method name after
    /// `PROJ-based operation method: `.
    ///
    /// # Errors
    ///
    /// `proj_string` is a pipeline that contains another pipeline.
    pub fn create_proj_based(props: &OpProps, proj_string: &str) -> Result<Self, FactoryError> {
        reject_nested_pipeline(proj_string)?;
        let method_name = format!("PROJ-based operation method: {proj_string}");
        Ok(Self {
            identified: identified_from(props, "PROJ-based coordinate operation"),
            method: OperationMethod::create(method_name, Vec::new()),
            parameters: Vec::new(),
            source: None,
            target: None,
            version: None,
            inverted: false,
            interpolation: None,
        })
    }

    /// `Conversion::createAxisOrderReversal`: EPSG:15498 or EPSG:15499.
    ///
    /// # Errors
    ///
    /// The method map has no entry for the axis-order method.
    pub fn create_axis_order_reversal(kind: AxisOrderReversal) -> Result<Self, FactoryError> {
        let (code, name, id) = match kind {
            AxisOrderReversal::TwoD => (9843, "axis order change (2D)", "15498"),
            AxisOrderReversal::Geographic3d => {
                (9844, "axis order change (geographic3D horizontal)", "15499")
            }
        };
        Self::from_code(
            &OpProps::new().name(name).identifier("EPSG", id),
            code,
            Vec::new(),
        )
    }

    /// The inverse conversion, as `InverseConversion`: the same method and parameters, an
    /// `Inverse of` name, and `INVERSE(...)` identifiers. The CRS pair, when set, is swapped
    /// with the direction, as [`Transformation::inverse`] swaps its own.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let mut inverse = self.clone();
        inverse.inverted = !self.inverted;
        inverse.identified.name = super::geodetic_ops::inverse_operation_name(
            self.name(),
            self.source_crs(),
            self.target_crs(),
            super::geodetic_ops::NamedOperationKind::Conversion,
        );
        std::mem::swap(&mut inverse.source, &mut inverse.target);
        for identifier in &mut inverse.identified.identifiers {
            identifier.code_space = inverse_code_space(&identifier.code_space);
        }
        inverse
    }
}

/// `CoordinateOperation::interpolationCRS`: the interpolation CRS of a single operation, or of a
/// concatenated one as `ConcatenatedOperation::create` set it.
pub(crate) fn interpolation_crs_of(operation: &CoordinateOperation) -> Option<&Crs> {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.interpolation_crs(),
        CoordinateOperation::Transformation(transformation) => transformation.interpolation_crs(),
        CoordinateOperation::Concatenated(concatenated) => concatenated.interpolation_crs(),
    }
}

/// The interpolation CRS of a concatenation, as `ConcatenatedOperation::create` computes it
/// (concatenatedoperation.cpp:177-249). The steps' interpolation CRS is kept while it is the
/// same one in each step; a step without one resets the value, so a later step's CRS is taken.
/// Two vertical steps through a geographic CRS take that geographic CRS.
pub(crate) fn concatenation_interpolation(steps: &[CoordinateOperation]) -> Option<Crs> {
    let mut interpolation: Option<Crs> = None;
    let mut valid = true;
    for step in steps {
        if valid {
            let sub = interpolation_crs_of(step).cloned();
            match (&interpolation, sub) {
                (None, sub) => interpolation = sub,
                (Some(current), Some(sub))
                    if sub.is_equivalent_to_with(current, Equivalence::Equivalent) => {}
                _ => {
                    interpolation = None;
                    valid = false;
                }
            }
        }
    }
    if let ([first, second], None) = (steps, &interpolation) {
        if let (Some(Crs::Vertical(_)), Some(Crs::Vertical(_))) =
            (first.source_crs(), second.target_crs())
        {
            if let (Some(geog1), Some(geog2)) = (first.target_crs(), second.source_crs()) {
                if geographic_of(geog1).is_some()
                    && geographic_of(geog2).is_some()
                    && geog1.is_equivalent_to_with(geog2, Equivalence::Equivalent)
                {
                    interpolation = Some(geog1.clone());
                }
            }
        }
    }
    interpolation
}

/// [`concatenation_interpolation`] of the operations of concatenated steps.
pub(crate) fn concatenation_interpolation_of_steps(steps: &[ConcatenatedStep]) -> Option<Box<Crs>> {
    let operations: Vec<CoordinateOperation> =
        steps.iter().map(|step| step.operation.clone()).collect();
    concatenation_interpolation(&operations).map(Box::new)
}

impl ConcatenatedOperation {
    /// `ConcatenatedOperation::create`: the source is the first step's source and the target
    /// the last step's target. An empty name becomes the step names joined by ` + `.
    ///
    /// # Errors
    ///
    /// Fewer than two steps, a step without CRS, or a step whose source is not the
    /// previous step's target.
    pub fn create(
        props: &OpProps,
        steps: Vec<CoordinateOperation>,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        if steps.len() < 2 {
            return Err(FactoryError::Invalid(
                "ConcatenatedOperation must have at least 2 operations".to_owned(),
            ));
        }
        for pair in steps.windows(2) {
            let (Some(previous), Some(next)) = (pair[0].target_crs(), pair[1].source_crs()) else {
                return Err(FactoryError::Invalid(
                    "Operations are missing their source or target CRS".to_owned(),
                ));
            };
            if !previous.is_equivalent_to(next) {
                return Err(FactoryError::Invalid(format!(
                    "Inconsistent chaining of CRS in operations: {} != {}",
                    previous.name(),
                    next.name()
                )));
            }
        }
        let source = steps.first().and_then(CoordinateOperation::source_crs);
        let target = steps.last().and_then(CoordinateOperation::target_crs);
        let (source, target) = (source.cloned().map(Box::new), target.cloned().map(Box::new));
        let interpolation = concatenation_interpolation(&steps).map(Box::new);
        let computed_name = props.name.as_deref().is_none_or(str::is_empty);
        let joined = steps
            .iter()
            .map(CoordinateOperation::name)
            .collect::<Vec<_>>()
            .join(" + ");
        let mut identified = identified_from(props, &joined);
        if computed_name {
            identified.name = joined;
        }
        Ok(Self {
            identified,
            steps: steps
                .into_iter()
                .map(|operation| ConcatenatedStep {
                    operation,
                    reverse: false,
                })
                .collect(),
            source,
            target,
            accuracy,
            version: None,
            computed_name,
            proj_based: None,
            source_epoch: None,
            target_epoch: None,
            interpolation,
        })
    }

    /// `isEquivalentTo` with the strict criterion: the same steps, in order.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        self.steps.len() == other.steps.len()
            && self.steps.iter().zip(&other.steps).all(|(left, right)| {
                left.reverse == right.reverse && left.operation.is_equivalent_to(&right.operation)
            })
    }

    /// The inverse concatenated operation: the steps in reverse order, each inverted, and
    /// the source and target swapped. A computed name is rebuilt from the inverted steps.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let steps: Vec<ConcatenatedStep> = self
            .steps
            .iter()
            .rev()
            .map(|step| ConcatenatedStep {
                operation: step.operation.inverse(),
                reverse: step.reverse,
            })
            .collect();
        let mut inverse = self.clone();
        inverse.identified.name = if self.computed_name {
            steps
                .iter()
                .map(|step| step.operation.name())
                .collect::<Vec<_>>()
                .join(" + ")
        } else {
            concatenated_inverse_name(self.name())
        };
        inverse.steps = steps;
        if let Some(proj_based) = inverse.proj_based.as_mut() {
            proj_based.inverted = !proj_based.inverted;
        }
        std::mem::swap(&mut inverse.source, &mut inverse.target);
        std::mem::swap(&mut inverse.source_epoch, &mut inverse.target_epoch);
        for identifier in &mut inverse.identified.identifiers {
            identifier.code_space = inverse_code_space(&identifier.code_space);
        }
        inverse
    }
}

impl CoordinateOperation {
    /// Source CRS, when the operation has one.
    #[must_use]
    pub fn source_crs(&self) -> Option<&Crs> {
        match self {
            Self::Conversion(operation) => operation.source_crs(),
            Self::Transformation(operation) => operation.source_crs(),
            Self::Concatenated(operation) => operation.source_crs(),
        }
    }

    /// Target CRS, when the operation has one.
    #[must_use]
    pub fn target_crs(&self) -> Option<&Crs> {
        match self {
            Self::Conversion(operation) => operation.target_crs(),
            Self::Transformation(operation) => operation.target_crs(),
            Self::Concatenated(operation) => operation.target_crs(),
        }
    }

    /// `isEquivalentTo` with the strict criterion. Two operations of different kinds differ.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Conversion(left), Self::Conversion(right)) => left.is_equivalent_to(right),
            (Self::Transformation(left), Self::Transformation(right)) => {
                left.is_equivalent_to(right)
            }
            (Self::Concatenated(left), Self::Concatenated(right)) => left.is_equivalent_to(right),
            _ => false,
        }
    }

    /// The operation from the target to the source.
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            Self::Conversion(operation) => Self::Conversion(operation.inverse()),
            Self::Transformation(operation) => Self::Transformation(operation.inverse()),
            Self::Concatenated(operation) => Self::Concatenated(operation.inverse()),
        }
    }

    /// `requiresPerCoordinateInputTime`: a time-dependent Helmert, or a step that has one.
    #[must_use]
    pub fn requires_per_coordinate_input_time(&self) -> bool {
        match self {
            Self::Conversion(_) => false,
            Self::Transformation(operation) => operation.requires_per_coordinate_input_time(),
            Self::Concatenated(operation) => operation
                .steps
                .iter()
                .any(|step| step.operation.requires_per_coordinate_input_time()),
        }
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
    match one_way_parameters(left, right, criterion) {
        None => false,
        Some(false) => true,
        Some(true) => one_way_parameters(right, left, criterion).is_some(),
    }
}

/// `SingleOperation::_isEquivalentTo` for parameter lists, one direction.
///
/// Returns `None` on a mismatch, and `Some(found_missing)` otherwise. Under
/// `Equivalent`, a parameter missing on the other side is accepted when it is the
/// neutral value (1 for a scale, 0 otherwise). A missing parameter makes the caller
/// run the reverse pass.
fn one_way_parameters(
    left: &[Parameter],
    right: &[Parameter],
    criterion: Equivalence,
) -> Option<bool> {
    let mut used = vec![false; right.len()];
    let mut found_missing = left.len() != right.len();
    for parameter in left {
        let mut equivalent = false;
        let mut same_name_different_value = false;
        for (index, other) in right.iter().enumerate() {
            if used[index] {
                continue;
            }
            if parameter_equivalent(parameter, other, criterion) {
                used[index] = true;
                equivalent = true;
                break;
            } else if parameter_names_match(parameter, other, criterion) {
                same_name_different_value = true;
                used[index] = true;
                break;
            }
        }
        if equivalent {
            continue;
        }
        if same_name_different_value || !neutral_parameter(parameter) {
            return None;
        }
        found_missing = true;
    }
    Some(found_missing)
}

fn parameter_equivalent(left: &Parameter, right: &Parameter, criterion: Equivalence) -> bool {
    parameter_names_match(left, right, criterion)
        && left.value.is_equivalent_to_with(&right.value, criterion)
}

/// Same parameter: the name, or, under `Equivalent`, the same non-zero EPSG code.
fn parameter_names_match(left: &Parameter, right: &Parameter, criterion: Equivalence) -> bool {
    if identified_names_match(&left.name, &right.name, criterion) {
        return true;
    }
    if criterion != Equivalence::Equivalent {
        return false;
    }
    let left_code = left.code.parse::<i32>().unwrap_or(0);
    let right_code = right.code.parse::<i32>().unwrap_or(0);
    left_code != 0 && left_code == right_code
}

/// A missing parameter is neutral when a scale is 1 and every other measure is 0.
fn neutral_parameter(parameter: &Parameter) -> bool {
    let ParameterValue::Measure(measure) = &parameter.value else {
        return false;
    };
    if measure.unit().kind() == UnitKind::Scale {
        measure.si_value() == 1.0
    } else {
        measure.si_value() == 0.0
    }
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
    let difference = left_si - right_si;
    if difference.abs() <= 2e-10 * left_si.abs() {
        return true;
    }
    // Under `Equivalent`, an angle equals the same angle a whole turn away:
    // `-3` and `357` degrees describe one Hotine azimuth (PROJ test
    // `hotine_oblique_mercator_variant_A_export_equivalent_modulo_360`).
    if left.unit().kind() == UnitKind::Angular {
        let turns = (difference / std::f64::consts::TAU).round();
        let residual = difference - turns * std::f64::consts::TAU;
        return residual.abs() <= 2e-10 * left_si.abs().max(right_si.abs());
    }
    false
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

/// Groups of parameter names that PROJ's `areEquivalentParameters` treats as one.
const EQUIVALENT_PARAMETERS: &[&[&str]] = &[
    &["latitude_of_point_1", "Latitude_Of_1st_Point"],
    &["longitude_of_point_1", "Longitude_Of_1st_Point"],
    &["latitude_of_point_2", "Latitude_Of_2nd_Point"],
    &["longitude_of_point_2", "Longitude_Of_2nd_Point"],
    &["satellite_height", "height"],
    &[
        "False easting",
        "Easting at false origin",
        "Easting at projection centre",
    ],
    &[
        "False northing",
        "Northing at false origin",
        "Northing at projection centre",
    ],
    &[
        "Scale factor at natural origin",
        "scale_factor",
        "Scale factor on initial line",
        "Scale factor at projection centre",
        "Scale factor on pseudo standard parallel",
    ],
    &[
        "latitude_of_origin",
        "latitude_of_center",
        "Latitude of natural origin",
        "Latitude of false origin",
        "Latitude of projection centre",
        "Central_Parallel",
    ],
    &[
        "central_meridian",
        "longitude_of_center",
        "Longitude of natural origin",
        "Longitude of false origin",
        "Longitude of projection centre",
        "Longitude of origin",
    ],
    &["Azimuth at projection centre", "Azimuth of initial line"],
    &["pseudo_standard_parallel_1", "standard_parallel_1"],
];

/// `_isEquivalentTo(UnitOfMeasure::METRE, EQUIVALENT)`: a length unit of one metre, whatever its name.
fn is_metre(unit: &super::model::UnitOfMeasure) -> bool {
    unit.kind() == UnitKind::Linear && (unit.conversion_to_si() - 1.0).abs() < 1e-10
}

fn are_equivalent_parameters(left: &str, right: &str) -> bool {
    let (left, right) = (canonical(left), canonical(right));
    left != right
        && EQUIVALENT_PARAMETERS.iter().any(|group| {
            let has = |name: &str| group.iter().any(|entry| canonical(entry) == name);
            has(&left) && has(&right)
        })
}

fn canonical(name: &str) -> String {
    name.chars()
        .filter(|ch| !matches!(ch, ' ' | '_' | '-' | '/' | '{' | '}'))
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

/// `SingleOperation::validateParameters`.
fn validate_method_parameters(method: &OperationMethod, parameters: &[Parameter]) -> Vec<String> {
    let Some(mapped) = find_for_validate(method) else {
        return vec![format!("Unknown method {}", method.name())];
    };
    let mut messages = Vec::new();
    if mapped.wkt2_name != method.name() {
        if names_match(mapped.wkt2_name, method.name()) {
            messages.push(format!(
                "Method name {} is equivalent to official {} but not strictly equal",
                method.name(),
                mapped.wkt2_name
            ));
        } else {
            messages.push(format!(
                "Method name {}, matched to {}, through its EPSG code has not an equivalent name",
                method.name(),
                mapped.wkt2_name
            ));
        }
    }
    let method_code = method.code.parse::<i32>().unwrap_or(0);
    if method_code != 0 && method_code != mapped.epsg_code {
        messages.push(format!(
            "Method of EPSG code {method_code} does not match official code ({})",
            mapped.epsg_code
        ));
    }
    let mut matched = vec![false; parameters.len()];
    for param in mapped.params {
        let found = parameters.iter().position(|have| {
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
        let have = &parameters[index];
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
    for (index, have) in parameters.iter().enumerate() {
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

    /// This conversion with its CRS pair set: `source` to `target` in the forward direction.
    #[must_use]
    pub(crate) fn with_endpoints(mut self, source: Crs, target: Crs) -> Self {
        self.source = Some(Box::new(source));
        self.target = Some(Box::new(target));
        self
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
            source: None,
            target: None,
            version: None,
            inverted: false,
            interpolation: None,
        }
    }

    /// This conversion with `name` as its identified name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.identified.name = name.into();
        self
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
        validate_method_parameters(&self.method, &self.parameters)
    }

    /// `exportToPROJString` for a conversion that has no source or target CRS.
    ///
    /// # Errors
    ///
    /// The method cannot be written as a PROJ string, or a parameter value is outside
    /// the range PROJ accepts. An inverse is the forward string inside `+inv`.
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
        if let Some(text) = self
            .method
            .name()
            .strip_prefix("PROJ-based operation method: ")
        {
            let direction = if self.inverted {
                ProjBasedDirection::Inverse
            } else {
                ProjBasedDirection::Forward
            };
            return export_proj_based_definition(text, direction);
        }
        if self.inverted {
            // A factory inverse still carries its CRS pair. That string is the
            // projection pipeline, not this bare `+inv` form.
            if self.source_crs().is_some() || self.target_crs().is_some() {
                return Err(FactoryError::Format(format!(
                    "inverse conversion {} has no PROJ string in this crate",
                    self.name()
                )));
            }
            let mut forward = self.clone();
            forward.inverted = false;
            let forward_string = forward.export_to_proj_inner(approx_tmerc)?;
            let mut formatter = super::pipeline::PipelineFormatter::with_grid_names(Vec::new());
            formatter.start_inversion();
            formatter.ingest_proj_string(&forward_string);
            formatter.stop_inversion();
            return Ok(formatter.finish(super::pipeline::ProjLines::Single));
        }
        if let Some(text) = self.proj_ob_tran_string() {
            return Ok(text);
        }
        if self.method.name.eq_ignore_ascii_case("Adams_Square_II") {
            return Ok(adams_square_ii_proj(self));
        }
        if self.method.name().eq_ignore_ascii_case("Adams_Square_II") {
            return Ok(self.adams_square_ii_proj_string());
        }
        let mapped = lookup_method(&self.method).ok_or_else(|| {
            FactoryError::Format(format!(
                "Unsupported conversion method: {}",
                self.method.name()
            ))
        })?;
        let code = mapped.epsg_code;
        // A registered axis-order reversal has no CRS pair. The swap of the two
        // horizontal axes is still `+proj=axisswap +order=2,1`.
        if code == 9843 || code == 9844 {
            return Ok("+proj=axisswap +order=2,1".to_owned());
        }
        if mapped.wkt2_name.starts_with("Peirce Quincuncial") {
            super::projfam::check_peirce_q_export(self.numeric_si(8801), self.numeric_si(8805))?;
        }
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
        if code == 9807 || (code == 1111 && self.method.code.is_empty()) {
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
        if let Some(text) = self.affine_parametric_proj_string() {
            return Ok(text);
        }
        if let Some(text) = self.pole_rotation_proj_string() {
            return Ok(text);
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

    /// `isUTM`: the UTM zone number and hemisphere of a transverse Mercator conversion.
    #[must_use]
    pub fn is_utm(&self) -> Option<(i32, crate::Hemisphere)> {
        self.utm_zone().map(|(zone, hemisphere)| {
            let hemisphere = match hemisphere {
                UtmHemisphere::North => crate::Hemisphere::North,
                UtmHemisphere::South => crate::Hemisphere::South,
            };
            (zone, hemisphere)
        })
    }

    /// `+proj=affine` for the 2D and 3D affine parametric methods. Values stay in the
    /// unit stored on the parameter, as `PROJStringFormatter` writes them.
    fn affine_parametric_proj_string(&self) -> Option<String> {
        let three_d = self
            .method
            .name()
            .eq_ignore_ascii_case("3D Affine parametric transformation");
        let two_d = self.method.code == "9624"
            || self
                .method
                .name()
                .eq_ignore_ascii_case("Affine parametric transformation");
        if !two_d && !three_d {
            return None;
        }
        let raw = |code: i32| {
            self.measure_of_code(code)
                .map(Measure::value)
                .unwrap_or(0.0)
        };
        let named = |name: &str| {
            self.parameters
                .iter()
                .find(|parameter| parameter.name.eq_ignore_ascii_case(name))
                .and_then(|parameter| measure_ref(&parameter.value))
                .map(Measure::value)
                .unwrap_or(0.0)
        };
        let mut parts = vec!["+proj=affine".to_owned()];
        push_number(&mut parts, "xoff", raw(8623));
        push_number(&mut parts, "s11", raw(8624));
        push_number(&mut parts, "s12", raw(8625));
        if three_d {
            push_number(&mut parts, "s13", named("A3"));
        }
        push_number(&mut parts, "yoff", raw(8639));
        push_number(&mut parts, "s21", raw(8640));
        push_number(&mut parts, "s22", raw(8641));
        if three_d {
            push_number(&mut parts, "s23", named("B3"));
            push_number(&mut parts, "zoff", named("C0"));
            push_number(&mut parts, "s31", named("C1"));
            push_number(&mut parts, "s32", named("C2"));
            push_number(&mut parts, "s33", named("C3"));
        }
        Some(parts.join(" "))
    }

    /// `Conversion::_exportToPROJString` of a `PROJ ob_tran o_proj=...` method: the method
    /// name is the PROJ operation, and each parameter is `+name=value` in degrees.
    fn proj_ob_tran_string(&self) -> Option<String> {
        let rest = self.method.name().strip_prefix("PROJ ob_tran o_proj=")?;
        let mut parts = vec![format!("+proj=ob_tran +o_proj={rest}")];
        for parameter in &self.parameters {
            let measure = measure_ref(&parameter.value)?;
            let degrees = measure.convert_to(&super::model::UnitOfMeasure::degree());
            push_number(&mut parts, &parameter.name, degrees);
        }
        Some(parts.join(" "))
    }

    /// `+proj=ob_tran` for the GRIB and netCDF CF pole-rotation methods. Angles are
    /// degrees. GRIB negates the southern-pole latitude and the axis rotation.
    /// netCDF CF writes `lon_0` as 180 plus the grid north-pole longitude.
    pub(crate) fn pole_rotation_proj_string(&self) -> Option<String> {
        let grib = self
            .method
            .name()
            .eq_ignore_ascii_case("Pole rotation (GRIB convention)");
        let netcdf = self
            .method
            .name()
            .eq_ignore_ascii_case("Pole rotation (netCDF CF convention)");
        if !grib && !netcdf {
            return None;
        }
        let degrees = |name: &str| {
            self.parameters
                .iter()
                .find(|parameter| parameter.name.eq_ignore_ascii_case(name))
                .and_then(|parameter| measure_ref(&parameter.value))
                .map(|measure| measure.convert_to(&super::model::UnitOfMeasure::degree()))
                .unwrap_or(0.0)
        };
        let (lon_p, lat_p, lon_0) = if grib {
            (
                -degrees("Axis rotation (GRIB convention)"),
                -degrees("Latitude of the southern pole (GRIB convention)"),
                degrees("Longitude of the southern pole (GRIB convention)"),
            )
        } else {
            let grid_longitude = degrees("Grid north pole longitude (netCDF CF convention)");
            (
                degrees("North pole grid longitude (netCDF CF convention)"),
                degrees("Grid north pole latitude (netCDF CF convention)"),
                180.0 + grid_longitude,
            )
        };
        let mut parts = vec!["+proj=ob_tran".to_owned(), "+o_proj=longlat".to_owned()];
        push_number(&mut parts, "o_lon_p", lon_p);
        push_number(&mut parts, "o_lat_p", lat_p);
        push_number(&mut parts, "lon_0", lon_0);
        Some(parts.join(" "))
    }

    /// ESRI `Adams_Square_II` as `+proj=spilhaus`, from the ESRI parameter names, which have
    /// no OGC mapping. PROJ's `spilhaus` matches it up to a `sqrt(2)` factor on the scale.
    fn adams_square_ii_proj_string(&self) -> String {
        let value = |name: &str, unit: &super::model::UnitOfMeasure| {
            self.parameters
                .iter()
                .find(|parameter| parameter.name.eq_ignore_ascii_case(name))
                .and_then(|parameter| measure_ref(&parameter.value))
                .map_or(0.0, |measure| measure.convert_to(unit))
        };
        let metre = super::model::UnitOfMeasure::metre();
        let degree = super::model::UnitOfMeasure::degree();
        let has_scale = self
            .parameters
            .iter()
            .any(|parameter| parameter.name.eq_ignore_ascii_case("Scale_Factor"));
        let scale = if has_scale {
            value(
                "Scale_Factor",
                &super::model::UnitOfMeasure::new("unity", 1.0, UnitKind::Scale),
            )
        } else {
            1.0
        };
        let mut parts = vec!["+proj=spilhaus".to_owned()];
        push_number(&mut parts, "lat_0", value("Latitude_Of_Center", &degree));
        push_number(&mut parts, "lon_0", value("Longitude_Of_Center", &degree));
        push_number(&mut parts, "azi", value("Azimuth", &degree));
        push_number(&mut parts, "k_0", std::f64::consts::SQRT_2 * scale);
        push_number(&mut parts, "rot", value("XY_Plane_Rotation", &degree));
        push_number(&mut parts, "x_0", value("False_Easting", &metre));
        push_number(&mut parts, "y_0", value("False_Northing", &metre));
        parts.join(" ")
    }

    pub(crate) fn utm_zone(&self) -> Option<(i32, UtmHemisphere)> {
        // `OperationMethod::getEPSGCode`: a method without an ID is found by its name, less " (3D)".
        let is_tmerc = self.method.code == "9807"
            || (self.method.code.is_empty()
                && names_match(
                    self.method
                        .name
                        .strip_suffix(" (3D)")
                        .unwrap_or(&self.method.name),
                    "Transverse Mercator",
                ));
        if !is_tmerc {
            return None;
        }
        let latitude = self.numeric_degrees(8801);
        if (latitude - 0.0).abs() >= 1e-10 {
            return None;
        }
        let longitude = self
            .measure_of_code(8802)
            .or_else(|| self.measure_of_code(8833))
            .map_or(0.0, |measure| {
                measure.convert_to(&super::model::UnitOfMeasure::degree())
            });
        let zone = (longitude + 183.0) / 6.0;
        if !(0.9..60.1).contains(&zone) || (zone - zone.round()).abs() >= 1e-10 {
            return None;
        }
        let scale = self.measure_of_code(8805)?;
        if scale.unit().kind() != UnitKind::Scale || (scale.value() - 0.9996).abs() >= 1e-10 {
            return None;
        }
        let easting = self.measure_of_code(8806)?;
        if easting.value() != 500_000.0 || !is_metre(easting.unit()) {
            return None;
        }
        let northing = self.measure_of_code(8807)?;
        if !is_metre(northing.unit()) {
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

    pub(crate) fn measure_of_code(&self, code: i32) -> Option<&Measure> {
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
        let by_name = self.parameters.iter().find_map(|parameter| {
            if names_match(&parameter.name, param.wkt2_name) {
                match &parameter.value {
                    ParameterValue::Measure(measure) => Some(measure),
                    _ => None,
                }
            } else {
                None
            }
        });
        by_name.or_else(|| {
            self.parameters.iter().find_map(|parameter| {
                if are_equivalent_parameters(param.wkt2_name, &parameter.name) {
                    measure_ref(&parameter.value)
                } else {
                    None
                }
            })
        })
    }

    pub(crate) fn from_code(
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
            source: None,
            target: None,
            version: None,
            inverted: false,
            interpolation: None,
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

/// A pipeline step whose `+proj` is itself `pipeline` is rejected by PROJ.
fn reject_nested_pipeline(text: &str) -> Result<(), FactoryError> {
    let mut pipelines = 0;
    let mut saw_step = false;
    for token in text.split_whitespace() {
        let token = token.trim_start_matches('+');
        if token == "step" {
            saw_step = true;
        }
        if token == "proj=pipeline" {
            pipelines += 1;
            if pipelines > 1 && saw_step {
                return Err(FactoryError::Unsupported(
                    "PROJBasedOperation::create() failed: nested pipeline".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

/// Direction of a PROJ-based conversion's stored string.
enum ProjBasedDirection {
    Forward,
    Inverse,
}

/// Forward text is the stored PROJ string. An empty definition is `+proj=noop`
/// in both directions. An inverse re-ingests the text, as PROJ does, so a
/// global parameter before the first step is kept on the forward string.
fn export_proj_based_definition(
    text: &str,
    direction: ProjBasedDirection,
) -> Result<String, FactoryError> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() || normalized == "+proj=noop" {
        return Ok("+proj=noop".to_owned());
    }
    if let ProjBasedDirection::Forward = direction {
        return Ok(normalized);
    }
    let mut formatter = super::pipeline::PipelineFormatter::with_grid_names(Vec::new());
    formatter.start_inversion();
    formatter.ingest_proj_string(&normalized);
    formatter.stop_inversion();
    Ok(formatter.finish(super::pipeline::ProjLines::Single))
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
    // `OperationParameter::getEPSGCode` falls back to these two WKT1-like names.
    if names_match(name, "Latitude of origin") {
        return Some(8801);
    }
    if names_match(name, "Scale factor") {
        return Some(8805);
    }
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

/// `+proj=spilhaus` for ESRI `Adams_Square_II`. The scale is `sqrt(2)` times
/// `Scale_Factor`, which defaults to 1 when the parameter is absent.
fn adams_square_ii_proj(conversion: &Conversion) -> String {
    let mut parts = vec!["+proj=spilhaus".to_owned()];
    push_number(
        &mut parts,
        "lat_0",
        adams_degrees(conversion, "Latitude_Of_Center"),
    );
    push_number(
        &mut parts,
        "lon_0",
        adams_degrees(conversion, "Longitude_Of_Center"),
    );
    push_number(&mut parts, "azi", adams_degrees(conversion, "Azimuth"));
    let scale = adams_measure(conversion, "Scale_Factor")
        .map(Measure::si_value)
        .unwrap_or(1.0);
    push_number(&mut parts, "k_0", std::f64::consts::SQRT_2 * scale);
    push_number(
        &mut parts,
        "rot",
        adams_degrees(conversion, "XY_Plane_Rotation"),
    );
    push_number(&mut parts, "x_0", adams_metres(conversion, "False_Easting"));
    push_number(
        &mut parts,
        "y_0",
        adams_metres(conversion, "False_Northing"),
    );
    parts.join(" ")
}

fn adams_degrees(conversion: &Conversion, name: &str) -> f64 {
    adams_measure(conversion, name)
        .map(|measure| measure.convert_to(&super::model::UnitOfMeasure::degree()))
        .unwrap_or(0.0)
}

fn adams_metres(conversion: &Conversion, name: &str) -> f64 {
    adams_measure(conversion, name)
        .map(Measure::si_value)
        .unwrap_or(0.0)
}

/// ESRI Adams names, or the EPSG names WKT1 import substitutes for them.
fn adams_measure<'a>(conversion: &'a Conversion, name: &str) -> Option<&'a Measure> {
    conversion.parameters.iter().find_map(|parameter| {
        let same = super::equiv::equivalent_name(&parameter.name, name)
            || super::equiv::are_equivalent_parameters(&parameter.name, name);
        if !same {
            return None;
        }
        match &parameter.value {
            ParameterValue::Measure(measure) => Some(measure),
            _ => None,
        }
    })
}

/// `PointMotionOperation::inverse`: the name with the epochs of the inverse, in years.
fn point_motion_name(name: &str, source: Option<f64>, target: Option<f64>) -> String {
    let (Some(source), Some(target)) = (source, target) else {
        return name.to_owned();
    };
    let base = name
        .split_once(" from epoch ")
        .map_or(name, |(base, _)| base);
    format!(
        "{base} from epoch {} to epoch {}",
        super::json::rounded_epoch_in_decimal_year(source),
        super::json::rounded_epoch_in_decimal_year(target)
    )
}
