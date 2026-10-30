//! Réduction des longitudes, identique à `adjlon` de PROJ.

use std::f64::consts::{PI, TAU};

/// Ramène `lon` (radians) dans ]−π, π]. Comme PROJ, une longitude qui dépasse
/// π de moins de 1e-12 est laissée telle quelle, pour éviter un changement de
/// signe parasite à l'antiméridien.
pub(crate) fn adjlon(mut lon: f64) -> f64 {
    if lon.abs() < PI + 1e-12 {
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
        assert_eq!(adjlon(-PI - 1e-13), -PI - 1e-13);
    }

    #[test]
    fn wraps_past_pi() {
        let wrapped = adjlon(PI + 0.5);
        assert!((wrapped + PI - 0.5).abs() < 1e-12, "{wrapped}");
    }
}
