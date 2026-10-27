//! Erreurs de construction d'un ellipsoïde.
//!
//! Les erreurs de conversion arrivent avec les projections (lots 2 et 3) :
//! un seul enum pour tout le crate mélangerait des échecs sur lesquels
//! l'appelant n'agit pas de la même façon.

use std::fmt;

/// Paramètres d'ellipsoïde refusés.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EllipsoidError {
    /// Un paramètre n'est pas fini.
    NonFiniteParameter,
    /// Le demi-grand axe doit être strictement positif.
    NonPositiveSemiMajorAxis,
    /// L'inverse de l'aplatissement doit être fini et strictement supérieur à 1.
    InvalidFlattening,
}

impl fmt::Display for EllipsoidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFiniteParameter => "ellipsoid parameter is not finite",
            Self::NonPositiveSemiMajorAxis => "semi-major axis must be positive",
            Self::InvalidFlattening => "inverse flattening must be greater than 1",
        })
    }
}

impl std::error::Error for EllipsoidError {}

/// Échec d'une conversion ou d'un paramètre de projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    /// Un paramètre ou une coordonnée n'est pas fini.
    NonFinite,
    /// Latitude hors de [−90°, 90°].
    InvalidLatitude,
    /// Facteur d'échelle nul ou non fini.
    InvalidScale,
    /// Zone UTM hors de 1..=60.
    InvalidZone,
    /// Point hors du domaine de la projection (PROJ renvoie `HUGE_VAL`).
    OutsideDomain,
    /// Parallèles du cône illégaux (`|lat| = 90°` ou `lat_1 + lat_2 = 0`).
    InvalidParallel,
    /// Le cône dégénère (excentricité trop proche de 1, ou `n = 0`).
    DegenerateCone,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "coordinate or parameter is not finite",
            Self::InvalidLatitude => "latitude is outside [-90, 90]",
            Self::InvalidScale => "scale factor must be finite and non-zero",
            Self::InvalidZone => "UTM zone must be in 1..=60",
            Self::OutsideDomain => "point is outside the projection domain",
            Self::InvalidParallel => "standard parallels are invalid",
            Self::DegenerateCone => "cone constant is degenerate",
        })
    }
}

impl std::error::Error for ProjectionError {}
