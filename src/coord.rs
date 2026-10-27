//! Coordonnées géographiques (degrés) et planes (mètres).

/// Longitude / latitude en degrés décimaux.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geographic {
    /// Longitude, degrés. Positive vers l'est.
    pub lon_deg: f64,
    /// Latitude, degrés. Positive vers le nord.
    pub lat_deg: f64,
}

impl Geographic {
    /// Les deux composantes sont finies.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.lon_deg.is_finite() && self.lat_deg.is_finite()
    }

    /// Longitude ramenée à ]−180°, 180°], comme `adjlon` de PROJ.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            lon_deg: crate::angle::adjlon(self.lon_deg.to_radians()).to_degrees(),
            lat_deg: self.lat_deg,
        }
    }
}

/// Easting / northing en mètres, faux origine inclus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projected {
    /// Easting, mètres.
    pub east: f64,
    /// Northing, mètres.
    pub north: f64,
}

impl Projected {
    /// Les deux composantes sont finies.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.east.is_finite() && self.north.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_rejects_nan() {
        assert!(Geographic {
            lon_deg: 2.0,
            lat_deg: 48.0
        }
        .is_finite());
        assert!(!Projected {
            east: f64::NAN,
            north: 0.0
        }
        .is_finite());
    }

    #[test]
    fn normalized_wraps_longitude() {
        let g = Geographic {
            lon_deg: 180.0 + 0.5_f64.to_degrees(),
            lat_deg: 10.0,
        };
        let n = g.normalized();
        assert!((n.lon_deg + 180.0 - 0.5_f64.to_degrees()).abs() < 1e-9);
        assert!((n.lat_deg - 10.0).abs() < 1e-12);
    }
}
