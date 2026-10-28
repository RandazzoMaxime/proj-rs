//! PROJ strings for the geographic CRS the unit tests export.
//!
//! Datum codes 6326, 6267, and 6269 become `+datum=`. Anything else is
//! `+ellps` plus `+pm` when the prime meridian is not Greenwich.

use super::model::{
    Datum, DatumEnsemble, Ellipsoid, FactoryError, GeodeticCRS, GeographicCRS, Identifier,
    PrimeMeridian, UnitOfMeasure,
};

impl GeographicCRS {
    /// `exportToPROJString` for a geographic CRS.
    #[must_use]
    pub fn export_to_proj_string(&self) -> String {
        geographic_proj(self)
    }
}

impl GeodeticCRS {
    /// `exportToPROJString` for a geographic CRS.
    ///
    /// # Errors
    ///
    /// The CRS is geocentric or spherical. Those strings are not built here.
    pub fn export_to_proj_string(&self) -> Result<String, FactoryError> {
        match self {
            Self::Geographic(crs) => Ok(crs.export_to_proj_string()),
            _ => Err(FactoryError::Format(
                "this geodetic CRS has no PROJ string in this build".to_owned(),
            )),
        }
    }
}

fn geographic_proj(crs: &GeographicCRS) -> String {
    let mut parts = vec!["+proj=longlat".to_owned()];
    match datum_token(crs) {
        Some(token) => parts.push(format!("+datum={token}")),
        None => {
            if let Some(ellipsoid) = ellipsoid_of(crs) {
                parts.push(ellipsoid_proj(ellipsoid));
            }
            if let Some(meridian) = meridian_of(crs) {
                if let Some(clause) = prime_meridian_proj(meridian) {
                    parts.push(clause);
                }
            }
        }
    }
    parts.push("+no_defs".to_owned());
    parts.push("+type=crs".to_owned());
    parts.join(" ")
}

fn datum_token(crs: &GeographicCRS) -> Option<&'static str> {
    let code = crs
        .datum()
        .and_then(|datum| epsg_code(datum.identifiers()))
        .or_else(|| {
            crs.datum_ensemble()
                .and_then(|ensemble| epsg_code(ensemble.identifiers()))
        })?;
    match code {
        "6326" => Some("WGS84"),
        "6267" => Some("NAD27"),
        "6269" => Some("NAD83"),
        _ => None,
    }
}

fn epsg_code(identifiers: &[Identifier]) -> Option<&str> {
    identifiers
        .iter()
        .find(|identifier| identifier.code_space() == "EPSG")
        .map(Identifier::code)
}

fn ellipsoid_of(crs: &GeographicCRS) -> Option<&Ellipsoid> {
    if let Some(datum) = crs.datum() {
        return Some(datum.ellipsoid());
    }
    geodetic_member(crs.datum_ensemble()?).map(super::model::GeodeticReferenceFrame::ellipsoid)
}

fn meridian_of(crs: &GeographicCRS) -> Option<&PrimeMeridian> {
    if let Some(datum) = crs.datum() {
        return Some(datum.prime_meridian());
    }
    geodetic_member(crs.datum_ensemble()?).map(super::model::GeodeticReferenceFrame::prime_meridian)
}

fn geodetic_member(ensemble: &DatumEnsemble) -> Option<&super::model::GeodeticReferenceFrame> {
    ensemble.members().iter().find_map(|member| match member {
        Datum::Geodetic(frame) => Some(frame),
        _ => None,
    })
}

fn prime_meridian_proj(meridian: &PrimeMeridian) -> Option<String> {
    if meridian.name().eq_ignore_ascii_case("paris")
        || epsg_code(meridian.identifiers()) == Some("8903")
    {
        return Some("+pm=paris".to_owned());
    }
    let degrees = meridian.longitude().convert_to(&UnitOfMeasure::degree());
    if degrees == 0.0 {
        None
    } else {
        Some(format!("+pm={degrees}"))
    }
}

fn ellipsoid_proj(ellipsoid: &Ellipsoid) -> String {
    let metres = ellipsoid.semi_major_axis().si_value();
    let inverse = ellipsoid.inverse_flattening();
    let flattening = (inverse != 0.0).then_some(inverse);
    let minor = ellipsoid
        .semi_minor_axis()
        .map(super::model::Measure::si_value);
    if let Some(name) = well_known_ellps(metres, flattening, minor) {
        return format!("+ellps={name}");
    }
    if let Some(inverse) = flattening {
        format!("+a={metres} +rf={inverse}")
    } else if let Some(minor) = minor {
        format!("+a={metres} +b={minor}")
    } else {
        format!("+R={metres}")
    }
}

fn well_known_ellps(major: f64, inverse: Option<f64>, minor: Option<f64>) -> Option<&'static str> {
    const TABLE: &[(&str, f64, f64, bool)] = &[
        ("GRS80", 6_378_137.0, 298.257_222_101, true),
        ("clrk66", 6_378_206.4, 6_356_583.8, false),
        ("clrk80", 6_378_249.145, 293.4663, true),
        ("clrk80ign", 6_378_249.2, 293.466_021_293_626_9, true),
        ("WGS84", 6_378_137.0, 298.257_223_563, true),
        ("sphere", 6_370_997.0, 6_370_997.0, false),
    ];
    let minor = minor.or_else(|| {
        inverse
            .filter(|value| *value != 0.0)
            .map(|value| major * (1.0 - 1.0 / value))
    });
    TABLE
        .iter()
        .find(|(_, table_major, second, is_inverse)| {
            if (major - table_major).abs() >= 1e-10 * table_major.abs() {
                return false;
            }
            if *is_inverse {
                inverse.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            } else {
                minor.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            }
        })
        .map(|(name, _, _, _)| *name)
}
