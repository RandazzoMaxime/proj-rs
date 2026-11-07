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
    best_operation_exact, same_datum_noop, Axis, CoordinateOperation, Crs, FactoryError,
    GeodeticCRS, UnitOfMeasure,
};
use crate::proj::{Direction, ProjError};

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
        let lead = if must_switch_for_visualization(crs) {
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
    /// Why the forward direction is refused, when the pair cannot take a height.
    refusal: Option<String>,
    /// The ISO operation the pipeline was written from.
    chosen: CoordinateOperation,
    /// How many candidate operations libproj could build for the pair.
    instantiable: usize,
    /// The operation's PROJ string before an epoch is inserted, as `export` wrote it.
    base_definition: String,
    /// Coordinate epoch inserted as `proj=set` steps, when one side of the pair carries one.
    epoch: Option<f64>,
}

/// The ISO pipeline from `from` to `to`, or `None` when the selection fails.
///
/// A compound CRS (a horizontal part and a vertical one) is read by its horizontal part:
/// the axis order and the units come from there. Its vertical part is in the pipeline.
pub(crate) fn iso_pipeline(from: &str, to: &str) -> Option<IsoPipeline> {
    iso_pipeline_at(from, to, None)
}

/// [`iso_pipeline`] with a coordinate epoch.
///
/// PROJ inserts `+proj=set +v_4=<epoch>` at both ends of the pipeline
/// (`proj_coordoperation_create_proj_crs_to_crs` when `coordinateEpoch` is set).
/// The helmert then reads that time. No epoch leaves the coordinate time at 0,
/// which is what an undated `proj_trans` passes.
pub(crate) fn iso_pipeline_at(from: &str, to: &str, epoch: Option<f64>) -> Option<IsoPipeline> {
    // A same-datum pair has no registered operation. PROJ still builds the null
    // geographic offset, and that is the pipeline `def()` reports.
    let choice = match best_operation_exact(from, to) {
        Ok(choice) => choice,
        // Only an empty candidate list. An approximate or grid-less choice stays
        // on the caller's previous path instead of becoming a null offset.
        Err(FactoryError::Format(message)) if message.starts_with("no operation from ") => {
            same_datum_noop(from, to).ok()?
        }
        Err(_) => return None,
    };
    let base_definition = choice.definition.clone();
    let definition = match epoch {
        Some(year) => with_epoch_steps(&choice.definition, year),
        None => choice.definition,
    };
    let (source, target) = (choice.source, choice.target);
    let source_horizontal = horizontal_part(&source).filter(|crs| is_horizontal(crs))?;
    let target_horizontal = horizontal_part(&target).filter(|crs| is_horizontal(crs))?;
    let operation = Operation::create(&definition).ok()?;
    Some(IsoPipeline {
        operation,
        source: Side::of(source_horizontal),
        target: Side::of(target_horizontal),
        refusal: refusal(&source, &target),
        chosen: choice.operation,
        instantiable: choice.instantiable,
        base_definition,
        epoch,
    })
}

/// `proj=set v_4=<year>` as the first and last step. A bare operation is wrapped in a pipeline.
fn with_epoch_steps(definition: &str, year: f64) -> String {
    let year = format_year(year);
    let plus = definition
        .split_whitespace()
        .any(|token| token.starts_with('+'));
    let mark = |token: &str| {
        if plus {
            format!("+{token}")
        } else {
            token.to_owned()
        }
    };
    let tokens: Vec<String> = definition
        .split_whitespace()
        .map(|token| token.strip_prefix('+').unwrap_or(token).to_owned())
        .collect();
    let set = [
        "step".to_owned(),
        "proj=set".to_owned(),
        format!("v_4={year}"),
    ];
    let mut out = Vec::new();
    if tokens.first().map(String::as_str) == Some("proj=pipeline") {
        out.push(mark("proj=pipeline"));
        for token in &set {
            out.push(mark(token));
        }
        for token in tokens.iter().skip(1) {
            out.push(mark(token));
        }
    } else {
        out.push(mark("proj=pipeline"));
        for token in &set {
            out.push(mark(token));
        }
        out.push(mark("step"));
        for token in &tokens {
            out.push(mark(token));
        }
    }
    for token in &set {
        out.push(mark(token));
    }
    out.join(" ")
}

fn format_year(year: f64) -> String {
    if year.is_finite() && year.fract() == 0.0 {
        format!("{year:.0}")
    } else {
        format!("{year}")
    }
}

/// The horizontal CRS of a pair side: the CRS itself, or the first component of a
/// compound CRS of two components.
fn horizontal_part(crs: &Crs) -> Option<&Crs> {
    match crs {
        Crs::Compound(compound) => match compound.components() {
            [horizontal, _vertical] => Some(horizontal),
            _ => None,
        },
        other => Some(other),
    }
}

/// A geographic or projected CRS: the kinds whose two horizontal axes the facade
/// boundary covers.
fn is_horizontal(crs: &Crs) -> bool {
    is_geographic(crs)
        || matches!(
            crs,
            Crs::Projected(_) | Crs::Geodetic(GeodeticCRS::Geocentric(_))
        )
}

/// Whether a CRS carries a height: a compound, or a geographic CRS of three axes.
fn has_height(crs: &Crs) -> bool {
    match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            geographic.coordinate_system().axes().len() > 2
        }
        Crs::Projected(_) => false,
        _ => true,
    }
}

/// The forward refusal of a pair. A two-dimensional source has no height to carry into a
/// compound target, so the operation the selection writes for it is a null offset, and its
/// result would be an ellipsoidal height of zero read as the target's vertical CRS. The
/// facade refuses that conversion instead of writing the made-up height. The inverse, a
/// compound source to a two-dimensional target, drops the height and is kept.
fn refusal(source: &Crs, target: &Crs) -> Option<String> {
    let compound_target = matches!(target, Crs::Compound(_));
    if compound_target && !has_height(source) {
        return Some(format!(
            "{} has a vertical part that needs a height from {}, which has none: the conversion is not built",
            target.name(),
            source.name()
        ));
    }
    None
}

/// Whether `proj_normalize_for_visualization` reverses the two horizontal axes of a CRS, so that
/// the facade's order (longitude or easting first) differs from the CRS's own.
///
/// This is `CRS::mustAxisOrderBeSwitchedForVisualization` (`crs.cpp:869-947`): a latitude or
/// northing first CRS is switched, and so is a polar CRS whose axes point north or south along
/// the meridians PROJ names. A polar CRS with the same directions along other meridians is not:
/// `EPSG:3413` writes easting first, and `cs2cs`, which reads the CRS's own order, keeps it.
fn must_switch_for_visualization(crs: &Crs) -> bool {
    let axes = match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            geographic.coordinate_system().axes()
        }
        Crs::Projected(projected) => projected.coordinate_system().axes(),
        _ => return false,
    };
    let [first, second, ..] = axes else {
        return false;
    };
    let meridian = |axis: &Axis| {
        axis.meridian()
            .map(|meridian| meridian.longitude().convert_to(&UnitOfMeasure::degree()))
    };
    let along = |axis: &Axis, degrees: f64| {
        meridian(axis).is_some_and(|value| (value - degrees).abs() < 1e-10)
    };
    match (first.direction(), second.direction()) {
        ("north", "east") => true,
        // EPSG:32661 "WGS 84 / UPS North (N,E)".
        ("south", "south") => along(first, 180.0) && along(second, 90.0),
        // EPSG:32761 "WGS 84 / UPS South (N,E)", and EPSG:5482 "RSRGD2000 / RSPS2000".
        ("north", "north") => {
            (along(first, 0.0) && along(second, 90.0))
                || (along(first, 180.0) && along(second, -90.0))
        }
        _ => false,
    }
}

fn is_geographic(crs: &Crs) -> bool {
    matches!(
        crs,
        Crs::Geographic(_) | Crs::Geodetic(GeodeticCRS::Geographic(_))
    )
}

impl IsoPipeline {
    /// The ISO operation this pipeline was written from.
    pub(crate) fn chosen(&self) -> &CoordinateOperation {
        &self.chosen
    }

    /// How many candidate operations libproj could build for the pair.
    pub(crate) fn instantiable(&self) -> usize {
        self.instantiable
    }

    /// `proj_pj_info().definition` of this pair, after `proj_normalize_for_visualization`.
    ///
    /// One instantiable operation is that pipeline. The axis swap a lat-first or
    /// northing-first side contributes is not in the string: normalization folds
    /// it into the CRS, which is what georust stores (`new_known_crs` normalizes).
    /// Several instantiable operations are the text PROJ stores until `proj_trans`
    /// picks one: `unavailable until proj_trans is called`. An epoch adds
    /// `proj=set v_4=<year>` at both ends.
    pub(crate) fn reported_definition(&self) -> Result<String, ProjError> {
        if self.instantiable() != 1 {
            return Ok("unavailable until proj_trans is called".to_owned());
        }
        let visualized = visualization_definition(&self.base_definition, self.source, self.target);
        let finished = crate::proj_info::parameters_of_operation(&visualized)
            .map_err(|_| ProjError::Definition)?;
        Ok(match self.epoch {
            Some(year) => with_epoch_steps(&finished, year),
            None => finished,
        })
    }

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

    /// The message that fails a conversion in `way`, when the pair refuses it.
    pub(crate) fn refusal(&self, way: Direction) -> Option<&str> {
        match way {
            Direction::Forward => self.refusal.as_deref(),
            Direction::Inverse => None,
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

/// The PROJ string `proj_normalize_for_visualization` leaves in `proj_pj_info`.
///
/// A side whose first axis is latitude or northing contributes
/// `step proj=axisswap order=2,1` at that end of the authority pipeline. The
/// normalized object drops those steps. Measured on PROJ 9.8.1: `EPSG:4326` to
/// `EPSG:3857` loses the leading swap and keeps the unit conversion and `webmerc`.
fn visualization_definition(definition: &str, source: Side, target: Side) -> String {
    let mut tokens = bare_tokens(definition);
    if source.lead == Lead::LatOrNorthing {
        tokens = drop_leading_swap(tokens);
    }
    if target.lead == Lead::LatOrNorthing {
        tokens = drop_trailing_swap(tokens);
    }
    tokens.join(" ")
}

fn bare_tokens(definition: &str) -> Vec<String> {
    definition
        .split_whitespace()
        .map(|token| token.strip_prefix('+').unwrap_or(token).to_owned())
        .collect()
}

fn drop_leading_swap(mut tokens: Vec<String>) -> Vec<String> {
    if tokens.len() >= 4
        && tokens[0] == "proj=pipeline"
        && tokens[1] == "step"
        && tokens[2] == "proj=axisswap"
        && tokens[3] == "order=2,1"
    {
        let rest = tokens.split_off(4);
        if rest.first().map(String::as_str) == Some("step") {
            let mut out = vec!["proj=pipeline".to_owned()];
            out.extend(rest);
            return out;
        }
        return rest;
    }
    tokens
}

fn drop_trailing_swap(mut tokens: Vec<String>) -> Vec<String> {
    let len = tokens.len();
    if len >= 3
        && tokens[len - 3] == "step"
        && tokens[len - 2] == "proj=axisswap"
        && tokens[len - 1] == "order=2,1"
    {
        tokens.truncate(len - 3);
    }
    tokens
}
