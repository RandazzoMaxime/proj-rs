//! Geodesics on an ellipsoid, as PROJ 9.9.0's `geod` computes them.
//!
//! The algorithm is Karney's, in the C library PROJ vendors and this crate
//! compiles (`src/geodesic/geodesic.c`). Angles are in degrees, as in that
//! library, and lengths are in metres.

use std::fmt;

use crate::geodesic::{Geodesic, GeodesicLine};

/// An ellipsoid for geodesic computations.
pub struct Geod {
    earth: Geodesic,
}

/// Distance and the two azimuths of the geodesic between two points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inverse {
    /// Length of the geodesic, in metres.
    pub distance: f64,
    /// Azimuth at the first point, in degrees clockwise from north.
    pub azimuth1: f64,
    /// Azimuth at the second point, in degrees, as the geodesic arrives there.
    pub azimuth2: f64,
}

/// A geodesic line leaving a point with a given azimuth.
pub struct Line {
    line: GeodesicLine,
}

impl Geod {
    /// Ellipsoid of semi-major axis `a`, in metres, and flattening `f`.
    #[must_use]
    pub fn new(a: f64, f: f64) -> Self {
        Self {
            earth: Geodesic::new(a, f),
        }
    }

    /// Shortest geodesic from the first point to the second.
    #[must_use]
    pub fn inverse(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Inverse {
        let (distance, azimuth1, azimuth2) = self.earth.inverse_full(lat1, lon1, lat2, lon2);
        Inverse {
            distance,
            azimuth1,
            azimuth2,
        }
    }

    /// The geodesic line leaving `(lat1, lon1)` with azimuth `azimuth1`.
    #[must_use]
    pub fn line(&self, lat1: f64, lon1: f64, azimuth1: f64) -> Line {
        Line {
            line: GeodesicLine::plain(&self.earth, lat1, lon1, azimuth1),
        }
    }
}

impl Line {
    /// Latitude, longitude and azimuth at `distance` metres along the line.
    #[must_use]
    pub fn position(&self, distance: f64) -> (f64, f64, f64) {
        self.line.position_azimuth(distance)
    }
}

impl fmt::Debug for Geod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Geod").finish_non_exhaustive()
    }
}

impl fmt::Debug for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Line").finish_non_exhaustive()
    }
}
