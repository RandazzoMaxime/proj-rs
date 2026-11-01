//! Projection conversion constructors that `buildop` does not list above.
//!
//! Same method names, parameter order and EPSG codes as PROJ
//! `Conversion::create*` (`src/iso19111/conversions.cpp`). The parameter maps
//! are in [`super::super::parammap`]. Each constructor builds the conversion
//! from the method name, as the older constructors in `buildop` do.

use super::{angles_lengths, angles_scale_lengths, long_lengths, OpProps};
use crate::iso19111::model::{Conversion, FactoryError, ParameterValue};

impl Conversion {
    /// New Zealand Map Grid, EPSG:9811.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_new_zealand_mapping_grid(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "New Zealand Map Grid",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Oblique Stereographic, EPSG:9809.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_oblique_stereographic(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Oblique Stereographic",
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Stereographic, EPSG:9810.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_stereographic(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        scale: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Stereographic",
            angles_scale_lengths(latitude, longitude, scale, false_easting, false_northing),
        )
    }

    /// Orthographic, EPSG:9840.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_orthographic(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Orthographic",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Local Orthographic, EPSG:1130. The centre and azimuth are angles, the
    /// scale is a factor, and the easting and northing are lengths.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_local_orthographic(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        azimuth: f64,
        scale: f64,
        easting: f64,
        northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Local Orthographic",
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::angle(azimuth),
                ParameterValue::scale(scale),
                ParameterValue::length(easting),
                ParameterValue::length(northing),
            ],
        )
    }

    /// American Polyconic, EPSG:9818.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_american_polyconic(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "American Polyconic",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Robinson.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_robinson(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Robinson",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Sinusoidal.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_sinusoidal(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Sinusoidal",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Van Der Grinten.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_van_der_grinten(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Van Der Grinten",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Wagner I, which has a longitude of natural origin and false origin.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_wagner_i(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Wagner I",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Wagner II.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_wagner_ii(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Wagner II",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Wagner IV.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_wagner_iv(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Wagner IV",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Wagner V.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_wagner_v(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Wagner V",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Wagner VI.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_wagner_vi(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Wagner VI",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Wagner VII.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_wagner_vii(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Wagner VII",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Wagner III has a latitude of true scale, then the longitude of origin.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_wagner_iii(
        props: &OpProps,
        latitude_true_scale: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Wagner III",
            vec![
                ParameterValue::angle(latitude_true_scale),
                ParameterValue::angle(longitude),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }

    /// Quadrilateralized Spherical Cube.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_quadrilateralized_spherical_cube(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Quadrilateralized Spherical Cube",
            angles_lengths(latitude, longitude, false_easting, false_northing),
        )
    }

    /// Spherical Cross-Track Height: a peg point (latitude, longitude,
    /// heading) and its height.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_spherical_cross_track_height(
        props: &OpProps,
        peg_latitude: f64,
        peg_longitude: f64,
        peg_heading: f64,
        peg_height: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Spherical Cross-Track Height",
            vec![
                ParameterValue::angle(peg_latitude),
                ParameterValue::angle(peg_longitude),
                ParameterValue::angle(peg_heading),
                ParameterValue::length(peg_height),
            ],
        )
    }

    /// Equal Earth.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_equal_earth(
        props: &OpProps,
        longitude: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Equal Earth",
            long_lengths(longitude, false_easting, false_northing),
        )
    }

    /// Vertical Perspective, EPSG:9838: topocentric origin (latitude,
    /// longitude, ellipsoidal height), viewpoint height, then false origin.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_vertical_perspective(
        props: &OpProps,
        latitude: f64,
        longitude: f64,
        topocentric_height: f64,
        viewpoint_height: f64,
        false_easting: f64,
        false_northing: f64,
    ) -> Result<Self, FactoryError> {
        Self::from_name(
            props,
            "Vertical Perspective",
            vec![
                ParameterValue::angle(latitude),
                ParameterValue::angle(longitude),
                ParameterValue::length(topocentric_height),
                ParameterValue::length(viewpoint_height),
                ParameterValue::length(false_easting),
                ParameterValue::length(false_northing),
            ],
        )
    }
}
