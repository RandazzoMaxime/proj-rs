//! The ISO 19111 pipeline behind [`crate::Proj::new_known_crs`].
//!
//! `new_known_crs` takes the operation `cs2cs` takes: the best operation of
//! [`crate::iso19111::best_pipeline`], written as a PROJ pipeline. That pipeline reads the source
//! CRS in its own axis order and units, and writes the target the same way. The
//! facade boundary is the one georust `proj` has: longitude then latitude for a
//! geographic CRS, easting then northing for a projected one, in the CRS units. This
//! module swaps the two axes on each side where the CRS writes latitude or northing
//! first.
//!
//! A pair that the selection cannot write (no operation, a missing grid, a CRS that
//! is neither geographic nor projected) returns `None`, and the caller keeps its
//! previous path.

use crate::engine::{Coord, Operation, TransformError};
use crate::iso19111::{
    best_pipeline_exact, create_from_user_input, lat_or_northing_first, Crs, DatabaseContext,
    GeodeticCRS,
};
use crate::proj::Direction;

/// Which axis a CRS writes first in the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lead {
    /// Latitude or northing.
    LatOrNorthing,
    /// Longitude or easting.
    LonOrEasting,
}

/// One side of a pipeline: the axis it writes first, and whether it is geographic.
#[derive(Clone, Copy, Debug)]
struct Side {
    lead: Lead,
    geographic: bool,
}

impl Side {
    fn of(crs: &Crs) -> Self {
        let lead = if lat_or_northing_first(crs) {
            Lead::LatOrNorthing
        } else {
            Lead::LonOrEasting
        };
        Self {
            lead,
            geographic: is_geographic(crs),
        }
    }

    /// The coordinate with its first two axes in pipeline order: latitude or northing
    /// first when this side writes that axis first. The swap is its own inverse, so one
    /// function serves both directions.
    fn in_pipeline_order(self, coord: Coord) -> Coord {
        match self.lead {
            Lead::LatOrNorthing => Coord {
                x: coord.y,
                y: coord.x,
                z: coord.z,
                t: coord.t,
            },
            Lead::LonOrEasting => coord,
        }
    }
}

/// The ISO pipeline of one pair, with the axis order of each side.
#[derive(Clone, Debug)]
pub(crate) struct IsoPipeline {
    operation: Operation,
    source: Side,
    target: Side,
}

/// The ISO pipeline from `from` to `to`, or `None` when the selection fails.
pub(crate) fn iso_pipeline(from: &str, to: &str) -> Option<IsoPipeline> {
    let (definition, _) = best_pipeline_exact(from, to).ok()?;
    let db = DatabaseContext::create().ok()?;
    let source = create_from_user_input(from, &db).ok()?;
    let target = create_from_user_input(to, &db).ok()?;
    if !is_horizontal(&source) || !is_horizontal(&target) {
        return None;
    }
    let operation = Operation::create(&definition).ok()?;
    Some(IsoPipeline {
        operation,
        source: Side::of(&source),
        target: Side::of(&target),
    })
}

/// A geographic or projected CRS: the kinds whose two horizontal axes the facade
/// boundary covers.
fn is_horizontal(crs: &Crs) -> bool {
    is_geographic(crs) || matches!(crs, Crs::Projected(_))
}

fn is_geographic(crs: &Crs) -> bool {
    matches!(
        crs,
        Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_))
    )
}

impl IsoPipeline {
    /// One point in `way` direction, facade order on both sides. The third axis passes
    /// through the pipeline, as `z` does in its geocentric steps.
    pub(crate) fn map(&self, coord: Coord, way: Direction) -> Result<Coord, TransformError> {
        match way {
            Direction::Forward => {
                let out = self
                    .operation
                    .forward(self.source.in_pipeline_order(coord))?;
                Ok(self.target.in_pipeline_order(out))
            }
            Direction::Inverse => {
                let out = self
                    .operation
                    .inverse(self.target.in_pipeline_order(coord))?;
                Ok(self.source.in_pipeline_order(out))
            }
        }
    }

    /// Whether the output of `way` writes degrees: the target of a forward transform, the
    /// source of an inverse one.
    pub(crate) fn degree_output(&self, way: Direction) -> bool {
        match way {
            Direction::Forward => self.target.geographic,
            Direction::Inverse => self.source.geographic,
        }
    }

    /// Whether the inverse direction exists, as for a native operation.
    pub(crate) fn has_inverse(&self) -> bool {
        !matches!(
            self.operation.inverse(Coord::xy(0.0, 0.0)),
            Err(TransformError::NoInverse)
        )
    }
}
