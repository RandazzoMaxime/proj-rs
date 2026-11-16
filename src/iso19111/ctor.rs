//! Constructors that the C API needs and the model did not expose: a geocentric CRS from a
//! datum, coordinate systems from their axes, and a conversion or a transformation whose
//! method and parameters carry authority codes (`proj_create_conversion`,
//! `proj_create_transformation`).

use super::buildop::OpProps;
use super::model::{
    Axis, CartesianCS, Conversion, Crs, DatumEnsemble, EllipsoidalCS, GeocentricCRS, GeodeticCRS,
    GeodeticReferenceFrame, Identified, OperationMethod, OperationParameter, OrdinalCS, Parameter,
    ParameterValue, Transformation, VerticalCS,
};
use super::parammap;
use super::FactoryError;

impl GeodeticCRS {
    /// A geocentric CRS named `name` on a datum or a datum ensemble.
    #[must_use]
    pub fn create_geocentric(
        name: impl Into<String>,
        datum: Option<GeodeticReferenceFrame>,
        ensemble: Option<DatumEnsemble>,
        coordinate_system: CartesianCS,
    ) -> Self {
        Self::Geocentric(GeocentricCRS {
            identified: Identified::named(name),
            datum,
            ensemble,
            coordinate_system,
        })
    }
}

impl EllipsoidalCS {
    /// An ellipsoidal coordinate system of two or three axes.
    ///
    /// # Errors
    ///
    /// The number of axes is not two or three.
    pub fn create(axes: Vec<Axis>) -> Result<Self, FactoryError> {
        if !(2..=3).contains(&axes.len()) {
            return Err(FactoryError::Invalid(
                "an ellipsoidal coordinate system has two or three axes".to_owned(),
            ));
        }
        Ok(Self {
            identified: Identified::default(),
            axes,
        })
    }
}

impl VerticalCS {
    /// A vertical coordinate system of one axis.
    #[must_use]
    pub fn create(axis: Axis) -> Self {
        Self {
            identified: Identified::default(),
            axis,
        }
    }
}

impl OrdinalCS {
    /// An ordinal coordinate system.
    #[must_use]
    pub fn create(axes: Vec<Axis>) -> Self {
        Self {
            identified: Identified::default(),
            axes,
        }
    }
}

/// A parameter given to `proj_create_conversion`: its name, its authority code, and its value.
#[derive(Clone, Debug)]
pub struct ParameterSpec {
    /// Name.
    pub name: String,
    /// Code space of the identifier, empty when it has none.
    pub code_space: String,
    /// Code of the identifier, empty when it has none.
    pub code: String,
    /// Value.
    pub value: ParameterValue,
}

fn parameters_of(specs: &[ParameterSpec]) -> (Vec<OperationParameter>, Vec<ParameterValue>) {
    let parameters = specs
        .iter()
        .map(|spec| OperationParameter::create(spec.name.clone()))
        .collect();
    let values = specs.iter().map(|spec| spec.value.clone()).collect();
    (parameters, values)
}

fn identify(
    stored: &mut [Parameter],
    method: &mut OperationMethod,
    specs: &[ParameterSpec],
    method_space: &str,
    method_code: &str,
) {
    method_space.clone_into(&mut method.code_space);
    method_code.clone_into(&mut method.code);
    for (parameter, spec) in stored.iter_mut().zip(specs) {
        spec.code_space.clone_into(&mut parameter.code_space);
        spec.code.clone_into(&mut parameter.code);
    }
}

impl Conversion {
    /// `proj_create_conversion`: a conversion of a method and parameters given with their
    /// authority codes.
    ///
    /// # Errors
    ///
    /// As [`Conversion::create`].
    pub fn create_identified(
        props: &OpProps,
        method_name: &str,
        method_code_space: &str,
        method_code: &str,
        specs: &[ParameterSpec],
    ) -> Result<Self, FactoryError> {
        let (parameters, values) = parameters_of(specs);
        let mut conversion = Self::create(props, method_name, parameters, values)?;
        identify(
            &mut conversion.parameters,
            &mut conversion.method,
            specs,
            method_code_space,
            method_code,
        );
        Ok(conversion)
    }
}

impl Transformation {
    /// `proj_create_transformation`: a transformation of a method and parameters given with
    /// their authority codes.
    ///
    /// # Errors
    ///
    /// As [`Transformation::create`].
    #[expect(
        clippy::too_many_arguments,
        reason = "the arguments of proj_create_transformation"
    )]
    pub fn create_identified(
        props: &OpProps,
        source: Crs,
        target: Crs,
        interpolation: Option<Crs>,
        method_name: &str,
        method_code_space: &str,
        method_code: &str,
        specs: &[ParameterSpec],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let (parameters, values) = parameters_of(specs);
        let mut transformation = Self::create(
            props,
            source,
            target,
            interpolation,
            method_name,
            parameters,
            values,
            accuracy,
        )?;
        identify(
            &mut transformation.parameters,
            &mut transformation.method,
            specs,
            method_code_space,
            method_code,
        );
        Ok(transformation)
    }
}

/// The EPSG code of an operation method given by name, `None` when it has none.
#[must_use]
pub fn method_epsg_code(name: &str) -> Option<i32> {
    parammap::method_by_name(name)
        .map(|method| method.epsg_code)
        .filter(|code| *code != 0)
}

impl Conversion {
    /// This conversion with `source` as its source CRS, as PROJ's `proj_crs_get_coordoperation`
    /// hands a deriving conversion back with the base CRS it was made from.
    #[must_use]
    pub fn with_source_crs(mut self, source: Crs) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}
