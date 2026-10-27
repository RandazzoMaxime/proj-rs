//! Réduction des longitudes, identique à `adjlon` de PROJ.

use std::f64::consts::{PI, TAU};

/// Ramène `lon` (radians) dans ]−π, π].
pub(crate) fn adjlon(mut lon: f64) -> f64 {
    if lon.abs() <= PI {
        return lon;
    }
    lon += PI;
    lon -= TAU * (lon / TAU).floor();
    lon -= PI;
    lon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_the_principal_range_untouched() {
        assert!(adjlon(0.0).abs() < 1e-15);
        assert!((adjlon(PI) - PI).abs() < 1e-15);
        assert!((adjlon(-PI + 1e-12) - (-PI + 1e-12)).abs() < 1e-15);
    }

    #[test]
    fn wraps_past_pi() {
        let wrapped = adjlon(PI + 0.5);
        assert!((wrapped + PI - 0.5).abs() < 1e-12, "{wrapped}");
    }
}
