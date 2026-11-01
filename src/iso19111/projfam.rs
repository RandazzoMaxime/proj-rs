//! The projection families whose PROJ string import or export needs a rule that
//! a method name alone does not give: `+proj=utm` (a transverse Mercator named
//! by its zone), the spherical `laea` and `eqc` on a sphere, the polar axes of
//! `laea`, the default `+lat_0` of `peirce_q`, and the `peirce_q` export check.
//! Lot io-6. Each rule is the one that the PROJ 9.9.0 `io.projparse_*` tests
//! pin (`test/unit/test_io.cpp`); the rules were not read from the C++ source.

use super::buildop::OpProps;
use super::model::{Axis, CartesianCS, Conversion, Crs, FactoryError, ProjectedCRS, UnitOfMeasure};
use super::parammap::ParamMap;
use super::projstring::{projected_from_proj_string, token};

/// `+proj=utm +zone=N [+south]`: the transverse Mercator of the zone, with the
/// conversion `UTM zone NN<N|S>` that PROJ names. The datum comes from the
/// string, as for any projected CRS.
///
/// # Errors
///
/// `+zone` is missing or is not in 1..=60, or the datum or units are refused.
pub(super) fn utm_projected(tokens: &[(String, String)]) -> Result<Crs, FactoryError> {
    let zone = token(tokens, "zone")
        .parse::<i32>()
        .map_err(|_| FactoryError::Invalid("+proj=utm needs +zone=1..60".to_owned()))?;
    let conversion = if tokens.iter().any(|(key, _)| key == "south") {
        Conversion::create_utm_south(&OpProps::new(), zone)?
    } else {
        Conversion::create_utm_north(&OpProps::new(), zone)?
    };
    let transverse: Vec<(String, String)> = tokens
        .iter()
        .filter(|(key, _)| key != "zone" && key != "south")
        .map(|(key, value)| {
            if key == "proj" {
                (key.clone(), "tmerc".to_owned())
            } else {
                (key.clone(), value.clone())
            }
        })
        .collect();
    let Crs::Projected(base) = projected_from_proj_string(&transverse)? else {
        return Err(FactoryError::Invalid(
            "+proj=utm is not projected".to_owned(),
        ));
    };
    Ok(Crs::Projected(ProjectedCRS::create(
        base.name(),
        base.base_crs().clone(),
        conversion,
        base.coordinate_system().clone(),
    )))
}

/// The EPSG method that PROJ picks for `laea` or `eqc` on a sphere (`+R=`, or
/// `+f=0`): Lambert azimuthal equal area (spherical) and equidistant
/// cylindrical (spherical). `None` otherwise. `inverse_flattening` is 0 for a
/// sphere.
pub(super) fn spherical_method_code(proj: &str, inverse_flattening: f64) -> Option<i32> {
    let sphere = inverse_flattening == 0.0;
    match (proj, sphere) {
        ("laea", true) => Some(1027),
        ("eqc", true) => Some(1029),
        _ => None,
    }
}

/// The coordinate system of a projected CRS built from a PROJ string. `laea`
/// centred on a pole has its easting and northing pointing south at the north
/// pole and north at the south pole, as PROJ writes it. Other CRSs are
/// easting, northing in `unit`.
///
/// # Errors
///
/// The axes cannot be made into a cartesian system.
pub(super) fn projected_cs(
    proj: &str,
    tokens: &[(String, String)],
    unit: UnitOfMeasure,
) -> Result<CartesianCS, FactoryError> {
    let lat_0 = token(tokens, "lat_0").parse::<f64>().ok();
    let direction = match (proj, lat_0) {
        ("laea", Some(lat)) if (lat - 90.0).abs() < 1e-10 => "south",
        ("laea", Some(lat)) if (lat + 90.0).abs() < 1e-10 => "north",
        _ => "",
    };
    if direction.is_empty() {
        return Ok(CartesianCS::create_easting_northing(unit));
    }
    CartesianCS::create(vec![
        Axis::new("Easting", "E", direction, unit.clone()),
        Axis::new("Northing", "N", direction, unit),
    ])
}

/// A parameter value PROJ supplies when the string does not give it. `peirce_q`
/// centres on the north pole, so its `+lat_0` is 90.
pub(super) fn default_value(proj: &str, param: &ParamMap) -> Option<f64> {
    (proj == "peirce_q" && param.proj_name == Some("lat_0")).then_some(90.0)
}

/// PROJ writes `peirce_q` only with a latitude of projection centre of ±90 and
/// a scale factor of 1. `latitude` and `scale` are the conversion's values in
/// radians and unity.
///
/// # Errors
///
/// The latitude is not a pole, or the scale is not 1.
pub(super) fn check_peirce_q_export(latitude: f64, scale: f64) -> Result<(), FactoryError> {
    if (latitude.abs() - std::f64::consts::FRAC_PI_2).abs() > 1e-10 {
        return Err(FactoryError::Format(
            "peirce_q needs a latitude of projection centre of +-90".to_owned(),
        ));
    }
    if (scale - 1.0).abs() > 1e-10 {
        return Err(FactoryError::Format(
            "peirce_q needs a scale factor of 1".to_owned(),
        ));
    }
    Ok(())
}
