//! The grids of `+proj=defmodel`, as PROJ opens them: a `GenericShiftGridSet` per file, read by
//! tile on demand, and one grid wrapper per grid of the set (`GridSet::mapGrids` of
//! `defmodel.cpp`).

use std::sync::{Arc, OnceLock};

use super::defmodel::{is_geographic, Grid, GridProvider, GridSet, DEG_TO_RAD};
use crate::projections::{GenericShiftGridSet, Grid as ShiftGrid};

/// Opens the grid files of a model (`EvaluatorIface::open`).
pub(super) struct ShiftGridProvider;

/// The grids of one file, with their wrappers built on first use.
pub(super) struct ShiftGridFile {
    set: Arc<GenericShiftGridSet>,
    null: OnceLock<ShiftGridNode>,
    grids: Vec<OnceLock<ShiftGridNode>>,
}

/// One grid of a file, or its null grid.
pub(super) struct ShiftGridNode {
    set: Arc<GenericShiftGridSet>,
    id: Option<usize>,
}

impl GridProvider for ShiftGridProvider {
    type Set = ShiftGridFile;

    fn open(&self, filename: &str) -> Option<ShiftGridFile> {
        let set = Arc::new(GenericShiftGridSet::open(filename).ok()?);
        let grids = (0..set.grid_count()).map(|_| OnceLock::new()).collect();
        Some(ShiftGridFile {
            set,
            null: OnceLock::new(),
            grids,
        })
    }

    fn is_geographic_crs(&self, definition: &str) -> bool {
        is_geographic(definition)
    }
}

impl GridSet for ShiftGridFile {
    type Node = ShiftGridNode;

    fn grid_at(&self, x: f64, y: f64) -> Option<&ShiftGridNode> {
        let id = self.set.grid_id_at(x, y)?;
        let cell = match id {
            None => &self.null,
            Some(index) => self.grids.get(index)?,
        };
        Some(cell.get_or_init(|| ShiftGridNode {
            set: Arc::clone(&self.set),
            id,
        }))
    }
}

impl ShiftGridNode {
    fn grid(&self) -> ShiftGrid<'_> {
        self.set.grid_by_id(self.id)
    }

    fn value(&self, ix: usize, iy: usize, sample: usize) -> Option<f64> {
        self.grid().value_at(ix, iy, sample).map(f64::from)
    }
}

impl Grid for ShiftGridNode {
    fn min_x(&self) -> f64 {
        self.grid().extent().west
    }

    fn min_y(&self) -> f64 {
        self.grid().extent().south
    }

    fn res_x(&self) -> f64 {
        self.grid().extent().res_x
    }

    fn res_y(&self) -> f64 {
        self.grid().extent().res_y
    }

    fn width(&self) -> usize {
        self.grid().width()
    }

    fn height(&self) -> usize {
        self.grid().height()
    }

    fn long_lat_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64)> {
        let (sample_x, sample_y) = band_pair(&self.grid())?;
        let east = self.value(ix, iy, sample_x)? * DEG_TO_RAD;
        let north = self.value(ix, iy, sample_y)? * DEG_TO_RAD;
        Some((east, north))
    }

    fn long_lat_z_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64, f64)> {
        let (lam, phi) = self.long_lat_offset(ix, iy)?;
        Some((lam, phi, self.z_offset(ix, iy)?))
    }

    fn easting_northing_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64)> {
        let (sample_x, sample_y) = band_pair(&self.grid())?;
        Some((self.value(ix, iy, sample_x)?, self.value(ix, iy, sample_y)?))
    }

    fn easting_northing_z_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64, f64)> {
        let (east, north) = self.easting_northing_offset(ix, iy)?;
        Some((east, north, self.z_offset(ix, iy)?))
    }

    fn z_offset(&self, ix: usize, iy: usize) -> Option<f64> {
        let sample = band_z(&self.grid())?;
        self.value(ix, iy, sample)
    }
}

/// The east and north bands of a grid file, found by their descriptions. A
/// file without descriptions uses bands 0 and 1.
fn band_pair(grid: &ShiftGrid<'_>) -> Option<(usize, usize)> {
    let samples = grid.samples_per_pixel();
    if samples < 2 {
        return None;
    }
    let mut x = 0usize;
    let mut y = 1usize;
    let mut found = false;
    let mut any = false;
    for sample in 0..samples {
        let desc = grid.description(sample);
        if !desc.is_empty() {
            any = true;
        }
        if desc == "east_offset" {
            x = sample;
            found = true;
        } else if desc == "north_offset" {
            y = sample;
        }
    }
    let have_y = (0..samples).any(|sample| grid.description(sample) == "north_offset");
    if any && (!found || !have_y) {
        return None;
    }
    let unit = grid.unit(x);
    if !unit.is_empty() && unit != "degree" && unit != "metre" {
        return None;
    }
    Some((x, y))
}

/// The vertical band of a grid file.
fn band_z(grid: &ShiftGrid<'_>) -> Option<usize> {
    let samples = grid.samples_per_pixel();
    if samples == 1 {
        return Some(0);
    }
    if samples < 3 && grid.description(0) != "vertical_offset" {
        return None;
    }
    let mut sample_z = 2;
    let mut any = false;
    let mut found = false;
    for sample in 0..samples {
        let desc = grid.description(sample);
        if !desc.is_empty() {
            any = true;
        }
        if desc == "vertical_offset" {
            sample_z = sample;
            found = true;
        }
    }
    if any && !found {
        return None;
    }
    let unit = grid.unit(sample_z);
    if !unit.is_empty() && unit != "metre" {
        return None;
    }
    Some(sample_z)
}
