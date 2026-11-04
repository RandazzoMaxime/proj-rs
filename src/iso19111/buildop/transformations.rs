//! Transformation constructors of PROJ `Transformation::create*`: the Helmert
//! family (geocentric translations, position vector, coordinate frame rotation,
//! and their time-dependent forms), the Molodensky pair, longitude rotation,
//! the geographic and vertical offsets, NTv2, and VERTCON.
//!
//! The method, parameter order and EPSG codes come from [`super::super::parammap`].
//! Values are in the units of the EPSG parameters, as PROJ takes them: metres,
//! degrees for the offset angles, arc-seconds and parts per million for Helmert.

use std::f64::consts::PI;

use super::OpProps;
use crate::iso19111::model::{
    Crs, FactoryError, GeodeticCRS, GeographicCRS, Identified, Measure, ObjectDomain,
    OperationMethod, Parameter, ParameterValue, Transformation, UnitKind, UnitOfMeasure,
};

/// Seconds in a year, the factor of the `year` unit.
const YEAR: f64 = 31_556_925.445;

/// Seven Helmert values in EPSG order: three translations (metres), three
/// rotations (arc-seconds), and the scale difference (parts per million).
fn helmert_values(translation: [f64; 3], rotation: [f64; 3], scale: f64) -> Vec<ParameterValue> {
    let mut values: Vec<ParameterValue> = translation
        .iter()
        .map(|value| ParameterValue::length(*value))
        .collect();
    values.extend(rotation.iter().map(|value| arc_second(*value)));
    values.push(parts_per_million(scale));
    values
}

fn arc_second(value: f64) -> ParameterValue {
    ParameterValue::Measure(Measure {
        value,
        unit: UnitOfMeasure::new("arc-second", PI / 648_000.0, UnitKind::Angular),
    })
}

fn parts_per_million(value: f64) -> ParameterValue {
    ParameterValue::Measure(Measure {
        value,
        unit: UnitOfMeasure::new("parts per million", 1e-6, UnitKind::Scale),
    })
}

/// Rates of change, per year: metres, arc-seconds, and parts per million.
fn rate_values(rates: [f64; 7]) -> Vec<ParameterValue> {
    let mut values = Vec::with_capacity(7);
    for (index, value) in rates.into_iter().enumerate() {
        values.push(match index {
            0..=2 => ParameterValue::Measure(Measure {
                value,
                unit: UnitOfMeasure::new("metre per year", 1.0 / YEAR, UnitKind::Linear),
            }),
            3..=5 => ParameterValue::Measure(Measure {
                value,
                unit: UnitOfMeasure::new(
                    "arc-second per year",
                    PI / 648_000.0 / YEAR,
                    UnitKind::Angular,
                ),
            }),
            _ => ParameterValue::Measure(Measure {
                value,
                unit: UnitOfMeasure::new(
                    "parts per million per year",
                    1e-6 / YEAR,
                    UnitKind::Scale,
                ),
            }),
        });
    }
    values
}

/// Geocentric CRS get the geocentric method, geographic CRS the 2D one.
fn is_geocentric(crs: &Crs) -> bool {
    matches!(crs, Crs::Geodetic(_))
}

/// The geodetic CRS `createTOWGS84` transforms, walking projected, bound,
/// compound, and derived CRS the way PROJ `extractGeodeticCRS` does.
fn towgs84_geodetic(crs: &Crs) -> Option<Crs> {
    match crs {
        Crs::Geographic(_) | Crs::Geodetic(_) => Some(crs.clone()),
        Crs::Projected(projected) => Some(Crs::Geodetic(projected.base_crs().clone())),
        Crs::DerivedProjected(derived) => {
            Some(Crs::Geodetic(derived.base_crs().base_crs().clone()))
        }
        Crs::Bound(bound) => towgs84_geodetic(bound.source_crs()),
        Crs::Compound(compound) => compound.components().iter().find_map(towgs84_geodetic),
        Crs::Derived(derived) => towgs84_geodetic(derived.base_crs()),
        Crs::Vertical(_) | Crs::Engineering(_) | Crs::Temporal(_) | Crs::Parametric(_) => None,
    }
}

fn empty_domain() -> ObjectDomain {
    ObjectDomain {
        scope: None,
        area: None,
        south: None,
        west: None,
        north: None,
        east: None,
    }
}

/// Build the transformation with the EPSG method `code`, then remove the
/// domain that `create_from_code` records, as PROJ does not give one.
fn build(
    props: &OpProps,
    source: Crs,
    target: Crs,
    code: i32,
    values: Vec<ParameterValue>,
    accuracy: Option<f64>,
) -> Result<Transformation, FactoryError> {
    let mut transformation =
        Transformation::create_from_code(props, source, target, code, values, empty_domain())?;
    transformation.identified.domains.clear();
    transformation.accuracy = accuracy;
    if props.name.is_none() {
        transformation.identified.name = transformation_name(
            transformation
                .source_crs()
                .map(Crs::name)
                .unwrap_or_default(),
            transformation
                .target_crs()
                .map(Crs::name)
                .unwrap_or_default(),
        );
    }
    Ok(transformation)
}

/// `buildTransfName`.
fn transformation_name(source: &str, target: &str) -> String {
    format!("Transformation from {source} to {target}")
}

impl Transformation {
    /// `requiresPerCoordinateInputTime`: the method is a time-dependent Helmert one.
    #[must_use]
    pub fn requires_per_coordinate_input_time(&self) -> bool {
        let code = self.method.code.parse::<i32>().unwrap_or(0);
        let name = self.method.name.to_ascii_lowercase();
        name.contains("time dependent")
            || name.contains("time-dependent")
            || (self.method.code_space == "EPSG" && (1053..=1058).contains(&code))
    }

    /// Methods whose inverse is the same method with every value negated:
    /// geocentric translations, Molodensky, longitude rotation, the geographic
    /// offsets, the vertical offset, and Cartesian grid offsets.
    pub(super) fn has_translation_inverse(&self) -> bool {
        matches!(
            self.method.code.parse::<i32>(),
            Ok(1031 | 9603 | 9604 | 9605 | 9601 | 9619 | 9660 | 9618 | 9616 | 9656)
        )
    }

    /// Negate every measured value. `0.0 - v` keeps a zero positive.
    pub(super) fn negate_values(&mut self) {
        for parameter in &mut self.parameters {
            if let ParameterValue::Measure(measure) = &mut parameter.value {
                measure.value = 0.0 - measure.value;
            }
        }
    }

    /// PROJ `getTOWGS84Parameters(true)`: the seven Helmert values of the
    /// `+towgs84` convention, or `None` when the method is not a Helmert one. A
    /// translation has zero rotations and scale.
    ///
    /// # Errors
    ///
    /// A Helmert method with a missing parameter.
    pub fn towgs84_parameters(&self) -> Result<Option<[f64; 7]>, FactoryError> {
        self.towgs84_strict()
    }

    /// Geocentric translations (EPSG:9603 for geographic CRS, 1031 for geocentric).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geocentric_translations(
        props: &OpProps,
        source: Crs,
        target: Crs,
        x: f64,
        y: f64,
        z: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let code = if is_geocentric(&source) { 1031 } else { 9603 };
        let values = vec![
            ParameterValue::length(x),
            ParameterValue::length(y),
            ParameterValue::length(z),
        ];
        build(props, source, target, code, values, accuracy)
    }

    /// Position vector transformation (EPSG:9606, or 1033 for geocentric CRS).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_position_vector(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let code = if is_geocentric(&source) { 1033 } else { 9606 };
        let values = helmert_values(
            [helmert[0], helmert[1], helmert[2]],
            [helmert[3], helmert[4], helmert[5]],
            helmert[6],
        );
        build(props, source, target, code, values, accuracy)
    }

    /// `Transformation::createTOWGS84`: three geocentric translations or a
    /// seven-parameter position vector, from the geodetic CRS inside `source`
    /// to WGS 84.
    ///
    /// # Errors
    ///
    /// `values` has a length other than 3 or 7, or `source` has no geodetic CRS.
    pub fn create_towgs84(source: Crs, values: &[f64]) -> Result<Self, FactoryError> {
        if values.len() != 3 && values.len() != 7 {
            return Err(FactoryError::Invalid(
                "Invalid number of elements in TOWGS84Parameters".to_owned(),
            ));
        }
        let Some(geodetic) = towgs84_geodetic(&source) else {
            return Err(FactoryError::Invalid(
                "Cannot find GeodeticCRS in sourceCRS of TOWGS84 transformation".to_owned(),
            ));
        };
        let target = match &geodetic {
            Crs::Geodetic(GeodeticCRS::Geocentric(_)) => Crs::Geodetic(GeodeticCRS::epsg_4978()),
            _ => Crs::Geographic(GeographicCRS::epsg_4326()),
        };
        let props = OpProps::new().name(format!("Transformation from {} to WGS84", source.name()));
        if values.len() == 3 {
            Self::create_geocentric_translations(
                &props, geodetic, target, values[0], values[1], values[2], None,
            )
        } else {
            let mut helmert = [0.0; 7];
            helmert.copy_from_slice(values);
            Self::create_position_vector(&props, geodetic, target, helmert, None)
        }
    }

    /// Coordinate frame rotation (EPSG:9607, or 1034 for geocentric CRS).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_coordinate_frame_rotation(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let code = if is_geocentric(&source) { 1034 } else { 9607 };
        let values = helmert_values(
            [helmert[0], helmert[1], helmert[2]],
            [helmert[3], helmert[4], helmert[5]],
            helmert[6],
        );
        build(props, source, target, code, values, accuracy)
    }

    /// Time-dependent position vector (EPSG:1054): seven Helmert values, their
    /// seven rates of change, then the parameter reference epoch.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_time_dependent_position_vector(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        rates: [f64; 7],
        epoch: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let values = time_dependent_values(helmert, rates, epoch);
        build(props, source, target, 1054, values, accuracy)
    }

    /// Time-dependent coordinate frame rotation (EPSG:1057).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_time_dependent_coordinate_frame_rotation(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        rates: [f64; 7],
        epoch: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let values = time_dependent_values(helmert, rates, epoch);
        build(props, source, target, 1057, values, accuracy)
    }

    /// Molodensky (EPSG:9604): three translations, the semi-major axis
    /// difference, and the flattening difference.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_molodensky(
        props: &OpProps,
        source: Crs,
        target: Crs,
        shifts: [f64; 5],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        molodensky(props, source, target, 9604, shifts, accuracy)
    }

    /// Abridged Molodensky (EPSG:9605), with the same five values.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_abridged_molodensky(
        props: &OpProps,
        source: Crs,
        target: Crs,
        shifts: [f64; 5],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        molodensky(props, source, target, 9605, shifts, accuracy)
    }

    /// Longitude rotation (EPSG:9601). `offset` is the longitude offset in degrees.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_longitude_rotation(
        props: &OpProps,
        source: Crs,
        target: Crs,
        offset: f64,
    ) -> Result<Self, FactoryError> {
        build(
            props,
            source,
            target,
            9601,
            vec![ParameterValue::angle(offset)],
            None,
        )
    }

    /// Geographic 2D offsets (EPSG:9619). Offsets are angles in degrees.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geographic_2d_offsets(
        props: &OpProps,
        source: Crs,
        target: Crs,
        latitude: f64,
        longitude: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        build(
            props,
            source,
            target,
            9619,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
            ],
            accuracy,
        )
    }

    /// Geographic 3D offsets (EPSG:9660). Angles are degrees, `height` is metres.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geographic_3d_offsets(
        props: &OpProps,
        source: Crs,
        target: Crs,
        latitude: f64,
        longitude: f64,
        height: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        build(
            props,
            source,
            target,
            9660,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::length(height),
            ],
            accuracy,
        )
    }

    /// Geographic 2D with height offsets (EPSG:9618). Angles are degrees, `height` is metres.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geographic_2d_with_height_offsets(
        props: &OpProps,
        source: Crs,
        target: Crs,
        latitude: f64,
        longitude: f64,
        height: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        build(
            props,
            source,
            target,
            9618,
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::length(height),
            ],
            accuracy,
        )
    }

    /// Vertical offset (EPSG:9616). `offset` is metres.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_vertical_offset(
        props: &OpProps,
        source: Crs,
        target: Crs,
        offset: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        build(
            props,
            source,
            target,
            9616,
            vec![ParameterValue::length(offset)],
            accuracy,
        )
    }

    /// NTv2 (EPSG:9615).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_ntv2(
        props: &OpProps,
        source: Crs,
        target: Crs,
        filename: &str,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        build(
            props,
            source,
            target,
            9615,
            vec![ParameterValue::filename(filename)],
            accuracy,
        )
    }

    /// `GravityRelatedHeight to Geographic3D`: a vertical CRS to a geographic 3D CRS through
    /// a geoid model file. The name is the one of PROJ when `props` has none.
    #[must_use]
    pub fn create_gravity_related_height_to_geographic_3d(
        props: &OpProps,
        source: Crs,
        target: Crs,
        filename: &str,
        accuracy: Option<f64>,
    ) -> Self {
        let name = props
            .name
            .clone()
            .unwrap_or_else(|| transformation_name(source.name(), target.name()));
        Self {
            identified: Identified::named(name),
            method: OperationMethod::create("GravityRelatedHeight to Geographic3D", Vec::new()),
            parameters: vec![Parameter {
                name: "Geoid (height correction) model file".to_owned(),
                code_space: "EPSG".to_owned(),
                code: "8666".to_owned(),
                value: ParameterValue::filename(filename),
            }],
            source: Some(Box::new(source)),
            target: Some(Box::new(target)),
            interpolation: None,
            accuracy,
            accuracy_text: None,
            version: None,
            point_motion: false,
            source_epoch: None,
            target_epoch: None,
            inverted: false,
        }
    }

    /// VERTCON (EPSG:9658).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_vertcon(
        props: &OpProps,
        source: Crs,
        target: Crs,
        filename: &str,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        build(
            props,
            source,
            target,
            9658,
            vec![ParameterValue::filename(filename)],
            accuracy,
        )
    }
}

/// The Helmert values, their rates, then the parameter reference epoch, in the
/// order of the time-dependent methods.
fn time_dependent_values(helmert: [f64; 7], rates: [f64; 7], epoch: f64) -> Vec<ParameterValue> {
    let mut values = helmert_values(
        [helmert[0], helmert[1], helmert[2]],
        [helmert[3], helmert[4], helmert[5]],
        helmert[6],
    );
    values.extend(rate_values(rates));
    values.push(ParameterValue::Measure(Measure {
        value: epoch,
        unit: UnitOfMeasure::new("year", YEAR, UnitKind::Time),
    }));
    values
}

fn molodensky(
    props: &OpProps,
    source: Crs,
    target: Crs,
    code: i32,
    shifts: [f64; 5],
    accuracy: Option<f64>,
) -> Result<Transformation, FactoryError> {
    let values = vec![
        ParameterValue::length(shifts[0]),
        ParameterValue::length(shifts[1]),
        ParameterValue::length(shifts[2]),
        ParameterValue::length(shifts[3]),
        ParameterValue::scale(shifts[4]),
    ];
    build(props, source, target, code, values, accuracy)
}
