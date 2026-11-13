//! Public read access to shift grids: the `grids.hpp` classes of PROJ.
//!
//! A set opens one file (`HorizontalShiftGridSet::open`, and the vertical and
//! generic ones), exposes its top-level grids, and answers `grid_at` with the
//! finest subgrid that contains a point, as `gridAt` does. A grid reads pixels
//! (`value_at`, `values_at`), horizontal shifts (`horizontal_shift`), metadata
//! and extent. Longitudes and latitudes are radians, as in the engine.
//!
//! Two C++ members have no counterpart. `reassign_context` is absent because
//! a set owns its bytes and holds no context. `reopen` re-reads the file.

use std::fmt;

use super::{
    f32_from_f64, geotiff, gtx_name, h_corner, is_ctable2, ntv2, open_generic, open_horizontal,
    open_vertical, opened_name, read_grid, Eps, Extent, Fail, GridSet, Hit, Node,
};

/// Why a shift grid could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GridError {
    /// The name does not resolve to a file on the search path.
    NotFound {
        /// The name that was asked for.
        name: String,
    },
    /// The file exists but is not a grid of the requested kind.
    Invalid {
        /// The name that was asked for.
        name: String,
    },
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "grid not found: {name}"),
            Self::Invalid { name } => write!(f, "invalid grid: {name}"),
        }
    }
}

impl std::error::Error for GridError {}

/// Whether a `values_at` block contains a nodata value.
///
/// The C++ call reports this through an out-parameter; here it is the return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nodata {
    /// Every value in the block is a real value.
    Absent,
    /// At least one value in the block is nodata.
    Found,
}

/// Extent and resolution of a grid, as `ExtentAndRes` in PROJ.
///
/// For a geographic grid the bounds are in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtentAndRes {
    /// `true` when the bounds are longitudes and latitudes.
    pub is_geographic: bool,
    /// Western bound.
    pub west: f64,
    /// Southern bound.
    pub south: f64,
    /// Eastern bound.
    pub east: f64,
    /// Northern bound.
    pub north: f64,
    /// Pixel size along x.
    pub res_x: f64,
    /// Pixel size along y.
    pub res_y: f64,
}

impl From<Extent> for ExtentAndRes {
    fn from(extent: Extent) -> Self {
        Self {
            is_geographic: extent.geographic,
            west: extent.west,
            south: extent.south,
            east: extent.east,
            north: extent.north,
            res_x: extent.res_x,
            res_y: extent.res_y,
        }
    }
}

/// The part of a set that every shift grid class shares.
struct Opened {
    request: String,
    name: String,
    format: &'static str,
    grids: GridSet,
}

impl fmt::Debug for Opened {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GridSet")
            .field("name", &self.name)
            .field("format", &self.format)
            .finish()
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Horizontal,
    Vertical,
    Generic,
}

impl Opened {
    fn open(kind: Kind, request: &str) -> Result<Self, GridError> {
        let grids = match kind {
            Kind::Horizontal => open_horizontal(request),
            Kind::Vertical => open_vertical(request),
            Kind::Generic => open_generic(request),
        }
        .map_err(|fail| match fail {
            Fail::Missing => GridError::NotFound {
                name: request.to_owned(),
            },
            Fail::Invalid => GridError::Invalid {
                name: request.to_owned(),
            },
        })?;
        Ok(Self {
            request: request.to_owned(),
            name: if request == "null" {
                "null".to_owned()
            } else {
                opened_name(request)
            },
            format: format_of(kind, request),
            grids,
        })
    }

    fn reopen(&mut self, kind: Kind) -> Result<(), GridError> {
        *self = Self::open(kind, &self.request)?;
        Ok(())
    }

    fn grids(&self) -> Vec<Grid<'_>> {
        if self.grids.null {
            return vec![Grid {
                set: self,
                node: None,
            }];
        }
        self.grids
            .roots
            .iter()
            .map(|&id| Grid {
                set: self,
                node: Some(id),
            })
            .collect()
    }

    fn grid_at(&self, lon: f64, lat: f64, eps: Eps) -> Option<Grid<'_>> {
        match self.grids.locate(lon, lat, eps)? {
            Hit::Null => Some(Grid {
                set: self,
                node: None,
            }),
            Hit::Grid(id) => Some(Grid {
                set: self,
                node: Some(id),
            }),
        }
    }
}

/// The format a file is read as, `"null"` for the null grid.
fn format_of(kind: Kind, request: &str) -> &'static str {
    if request == "null" {
        return "null";
    }
    // The bytes are read a second time here: the open functions own the
    // parsing, and a sniff of the magic numbers would duplicate it.
    let Ok(bytes) = read_grid(request) else {
        return "";
    };
    match kind {
        Kind::Horizontal if ntv2::is_ntv2(&bytes) => "ntv2",
        Kind::Horizontal if geotiff::is_tiff(&bytes) => "gtiff",
        Kind::Horizontal if is_ctable2(&bytes) => "ctable2",
        Kind::Vertical if geotiff::is_tiff(&bytes) => "gtiff",
        Kind::Vertical if gtx_name(&opened_name(request)) => "gtx",
        Kind::Generic if geotiff::is_tiff(&bytes) => "gtiff",
        _ => "",
    }
}

macro_rules! shift_grid_set {
    ($(#[$doc:meta])* $name:ident, $kind:expr, $eps:expr) => {
        $(#[$doc])*
        #[derive(Debug)]
        pub struct $name {
            opened: Opened,
        }

        impl $name {
            /// Open the grid file or the `null` grid.
            ///
            /// # Errors
            ///
            /// [`GridError::NotFound`] when `name` is not on the search path,
            /// [`GridError::Invalid`] when the file is not a grid of this kind.
            pub fn open(name: &str) -> Result<Self, GridError> {
                Ok(Self {
                    opened: Opened::open($kind, name)?,
                })
            }

            /// The name the set was opened with.
            ///
            /// For a file this is the resolved path; for `null` it is `"null"`.
            #[must_use]
            pub fn name(&self) -> &str {
                &self.opened.name
            }

            /// The file format: `"gtiff"`, `"gtx"`, `"ntv2"`, `"ctable2"` or `"null"`.
            ///
            /// Empty when the file cannot be read again to identify it.
            #[must_use]
            pub fn format(&self) -> &str {
                self.opened.format
            }

            /// The top-level grids of the set. The null set has one null grid.
            #[must_use]
            pub fn grids(&self) -> Vec<Grid<'_>> {
                self.opened.grids()
            }

            /// The finest grid that contains the point, in radians.
            ///
            /// `None` when no grid of the set covers the point.
            #[must_use]
            pub fn grid_at(&self, lon: f64, lat: f64) -> Option<Grid<'_>> {
                self.opened.grid_at(lon, lat, $eps)
            }

            /// Read the file again, as PROJ's `reopen` does after a context change.
            ///
            /// # Errors
            ///
            /// The errors of [`Self::open`].
            pub fn reopen(&mut self) -> Result<(), GridError> {
                self.opened.reopen($kind)
            }
        }
    };
}

shift_grid_set!(
    /// A set of vertical shift grids (`VerticalShiftGridSet` in PROJ).
    VerticalShiftGridSet,
    Kind::Vertical,
    Eps::Strict
);

shift_grid_set!(
    /// A set of horizontal shift grids (`HorizontalShiftGridSet` in PROJ).
    HorizontalShiftGridSet,
    Kind::Horizontal,
    Eps::Horizontal
);

shift_grid_set!(
    /// A set of grids with any number of samples per pixel
    /// (`GenericShiftGridSet` in PROJ).
    GenericShiftGridSet,
    Kind::Generic,
    Eps::Horizontal
);

/// One grid of a set, or a subgrid of one.
///
/// Pixel coordinates count columns from the west edge and rows from the south
/// edge, as PROJ's `valueAt` does. The null grid is 3 by 3 pixels over the
/// whole world and has no samples; its values are zero.
#[derive(Clone, Copy)]
pub struct Grid<'a> {
    set: &'a Opened,
    node: Option<usize>,
}

impl fmt::Debug for Grid<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Grid")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("null", &self.is_null_grid())
            .finish()
    }
}

/// The 3 by 3 null grid spans the world; its resolution follows.
const NULL_EXTENT: ExtentAndRes = ExtentAndRes {
    is_geographic: true,
    west: -std::f64::consts::PI,
    south: -std::f64::consts::FRAC_PI_2,
    east: std::f64::consts::PI,
    north: std::f64::consts::FRAC_PI_2,
    res_x: std::f64::consts::PI,
    res_y: std::f64::consts::FRAC_PI_2,
};

impl<'a> Grid<'a> {
    fn node(&self) -> Option<&'a Node> {
        self.node.and_then(|id| self.set.grids.nodes.get(id))
    }

    /// Columns of the grid.
    #[must_use]
    pub fn width(&self) -> usize {
        self.node()
            .map_or(3, |node| usize::try_from(node.width).unwrap_or(0))
    }

    /// Rows of the grid.
    #[must_use]
    pub fn height(&self) -> usize {
        self.node()
            .map_or(3, |node| usize::try_from(node.height).unwrap_or(0))
    }

    /// Bounds and pixel size of the grid.
    #[must_use]
    pub fn extent(&self) -> ExtentAndRes {
        self.node()
            .map_or(NULL_EXTENT, |node| ExtentAndRes::from(node.extent))
    }

    /// `true` for the null grid, which shifts nothing.
    #[must_use]
    pub fn is_null_grid(&self) -> bool {
        self.node.is_none()
    }

    /// `true` when the grid was modified after it was read. No operation here
    /// modifies a grid, so this is always `false`.
    #[must_use]
    pub const fn has_changed(&self) -> bool {
        false
    }

    /// Number of samples per pixel. The null grid has none.
    #[must_use]
    pub fn samples_per_pixel(&self) -> usize {
        self.node().map_or(0, |node| node.spp)
    }

    /// The value of one sample of one pixel. `None` past the last sample or
    /// the last pixel. The null grid returns zero for any index.
    #[must_use]
    pub fn value_at(&self, x: usize, y: usize, sample: usize) -> Option<f32> {
        let Some(node) = self.node() else {
            return Some(0.0);
        };
        let x = i32::try_from(x).ok()?;
        let y = i32::try_from(y).ok()?;
        node.value(sample, x, y)
    }

    /// Fill `out` with the samples of a block of pixels, in the order row by
    /// row, then column, then sample. `out` must hold at least
    /// `x_count * y_count * samples.len()` values.
    ///
    /// `None` when the block or a sample index is out of range, or when `out`
    /// is too short.
    #[must_use]
    pub fn values_at(
        &self,
        x_start: usize,
        y_start: usize,
        x_count: usize,
        y_count: usize,
        samples: &[usize],
        out: &mut [f32],
    ) -> Option<Nodata> {
        let needed = x_count.checked_mul(y_count)?.checked_mul(samples.len())?;
        if out.len() < needed {
            return None;
        }
        let Some(node) = self.node() else {
            out[..needed].fill(0.0);
            return Some(Nodata::Absent);
        };
        let mut nodata = Nodata::Absent;
        let mut index = 0;
        for dy in 0..y_count {
            for dx in 0..x_count {
                for &sample in samples {
                    let value = self.value_at(x_start + dx, y_start + dy, sample)?;
                    if node.nodata_value(value) {
                        nodata = Nodata::Found;
                    }
                    out[index] = value;
                    index += 1;
                }
            }
        }
        Some(nodata)
    }

    /// The longitude and latitude shift of a pixel, in radians. Defined for
    /// horizontal grids only; `None` for other grids and out-of-range pixels.
    /// The null grid returns zero shifts.
    #[must_use]
    pub fn horizontal_shift(&self, x: usize, y: usize) -> Option<(f32, f32)> {
        let Some(node) = self.node() else {
            return Some((0.0, 0.0));
        };
        let x = i32::try_from(x).ok()?;
        let y = i32::try_from(y).ok()?;
        let (lon, lat) = h_corner(node, x, y)?;
        Some((f32_from_f64(lon), f32_from_f64(lat)))
    }

    /// Whether `value` is the nodata marker of this grid.
    ///
    /// A GTX vertical grid also treats any value beyond 1000 in magnitude,
    /// after `value * multiplier`, as nodata, as PROJ does. The null grid has
    /// no nodata.
    #[must_use]
    pub fn is_nodata(&self, value: f32, multiplier: f64) -> bool {
        let Some(node) = self.node() else {
            return false;
        };
        if self.set.format == "gtx" {
            let scaled = f64::from(value) * multiplier;
            return scaled > 1000.0 || scaled < -1000.0 || value == -88.888_8_f32;
        }
        node.nodata_value(value)
    }

    /// The unit of a sample, from its `UNITTYPE` metadata. Empty when absent.
    #[must_use]
    pub fn unit(&self, sample: usize) -> String {
        self.node().map_or_else(String::new, |node| {
            node.own_meta(i32::try_from(sample).unwrap_or(-1), "UNITTYPE")
        })
    }

    /// The description of a sample, from its `DESCRIPTION` metadata. Empty when absent.
    #[must_use]
    pub fn description(&self, sample: usize) -> String {
        self.node().map_or_else(String::new, |node| {
            node.own_meta(i32::try_from(sample).unwrap_or(-1), "DESCRIPTION")
        })
    }

    /// A dataset-level metadata item, or `None` when the key is absent.
    #[must_use]
    pub fn metadata_item(&self, key: &str) -> Option<&'a str> {
        self.metadata_item_for_sample(key, -1)
    }

    /// A metadata item of one sample (`-1` for the dataset), or `None`.
    #[must_use]
    pub fn metadata_item_for_sample(&self, key: &str, sample: i32) -> Option<&'a str> {
        let node = self.node()?;
        node.meta.get(&(sample, key.to_owned())).map(String::as_str)
    }

    /// The subgrids of this grid, finest last. Empty for a grid with none,
    /// and for the null grid.
    #[must_use]
    pub fn sub_grids(&self) -> Vec<Grid<'a>> {
        let Some(node) = self.node() else {
            return Vec::new();
        };
        node.children
            .iter()
            .map(|&id| Grid {
                set: self.set,
                node: Some(id),
            })
            .collect()
    }
}
