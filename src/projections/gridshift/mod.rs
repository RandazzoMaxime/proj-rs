//! Horizontal, vertical and generic grid shifts.
//!
//! Ports of PROJ 9.9 `src/transformations/hgridshift.cpp`,
//! `vgridshift.cpp`, `gridshift.cpp`, and the NTv2 / GeoTIFF readers in
//! `src/grids.cpp`. A named grid that is not on disk is an error at creation
//! when the network is off, never a coordinate. When the network is on,
//! `decide_download` may fetch it into the cache before the file is opened.

mod geotiff;
mod ntv2;

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

use crate::angle::adjlon;
use crate::engine::{
    cartesian_to_geodetic, Coord, CreateError, Frame, IoKind, Params, Step, TransformError,
};

const REL: f64 = 1e-5;
const ARCSEC: f64 = (PI / 180.0) / 3600.0;
const MAX_ITER: i32 = 10;
const TOL2: f64 = 1e-12 * 1e-12;
const PAGE: u32 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Domain {
    Geographic,
    Projected,
}

#[derive(Clone, Copy)]
enum Attach {
    ParentName,
    Extent,
}

#[derive(Clone, Copy)]
enum Way {
    Forward,
    Inverse,
}

#[derive(Clone, Copy)]
enum Pass {
    Full,
    Vertical,
}

#[derive(Clone, Copy)]
enum Height {
    Keep,
    Skip,
}

#[derive(Clone, Copy)]
pub(super) struct Extent {
    geographic: bool,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    res_x: f64,
    res_y: f64,
    pub(super) inv_x: f64,
    pub(super) inv_y: f64,
}

impl Extent {
    pub(super) fn try_new(
        domain: Domain,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        res_x: f64,
        res_y: f64,
    ) -> Result<Self, ()> {
        let geographic = domain == Domain::Geographic;
        let angles_ok = !geographic
            || (west.abs() <= 4.0 * PI
                && east.abs() <= 4.0 * PI
                && north.abs() <= PI + 1e-5
                && south.abs() <= PI + 1e-5);
        if !(angles_ok && west < east && south < north && res_x > 1e-10 && res_y > 1e-10) {
            return Err(());
        }
        Ok(Self {
            geographic,
            west,
            south,
            east,
            north,
            res_x,
            res_y,
            inv_x: 1.0 / res_x,
            inv_y: 1.0 / res_y,
        })
    }

    fn contains(self, other: Self) -> bool {
        other.west >= self.west
            && other.east <= self.east
            && other.south >= self.south
            && other.north <= self.north
    }

    fn full_world(self) -> bool {
        self.geographic && self.east - self.west + self.res_x >= TAU - 1e-10
    }

    fn epsilon(self) -> f64 {
        (self.res_x + self.res_y) * REL
    }
}

struct HorizSpec {
    lat: usize,
    lon: usize,
    to_radian: f64,
    positive_east: bool,
}

struct Node {
    width: i32,
    height: i32,
    extent: Extent,
    bottom_up: bool,
    spp: usize,
    samples: Vec<f32>,
    scale: Vec<f64>,
    offset: Vec<f64>,
    nodata: Option<f32>,
    meta: BTreeMap<(i32, String), String>,
    grid_name: String,
    parent_name: String,
    children: Vec<usize>,
    inherit: Option<usize>,
    horizontal: Option<HorizSpec>,
    vertical: Option<usize>,
}

impl Node {
    fn value(&self, sample: usize, x: i32, y: i32) -> Option<f32> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height || sample >= self.spp {
            return None;
        }
        let y_tiff = if self.bottom_up {
            y
        } else {
            self.height - 1 - y
        };
        let x_us = usize::try_from(x).ok()?;
        let y_us = usize::try_from(y_tiff).ok()?;
        let width = usize::try_from(self.width).ok()?;
        let index = (y_us * width + x_us) * self.spp + sample;
        let raw = *self.samples.get(index)?;
        if self.nodata.is_some_and(|nodata| raw == nodata) {
            return Some(raw);
        }
        if let Some(&scale) = self.scale.get(sample) {
            let offset = self.offset.get(sample).copied().unwrap_or(0.0);
            return Some(f32_from_f64(f64::from(raw) * scale + offset));
        }
        Some(raw)
    }

    fn nodata_value(&self, value: f32) -> bool {
        self.nodata.is_some_and(|nodata| value == nodata) || value.is_nan()
    }

    fn own_meta(&self, sample: i32, key: &str) -> String {
        self.meta
            .get(&(sample, key.to_owned()))
            .cloned()
            .unwrap_or_default()
    }
}

struct GridSet {
    nodes: Vec<Node>,
    roots: Vec<usize>,
    first_root: Option<usize>,
    null: bool,
}

enum Hit {
    Null,
    Grid(usize),
}

enum Fail {
    Missing,
    Invalid,
}

enum Interp {
    FromGrid,
    Bilinear,
    Biquadratic,
}

struct GridMix {
    geog3d: bool,
    horizontal: bool,
}

struct Model {
    main: String,
    aux: String,
    mix: GridMix,
    offset_x: f64,
    offset_y: f64,
    interpolation: Interp,
    height: Height,
    space: Domain,
}

enum Located<'a> {
    Null,
    At(&'a GridSet, usize),
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "hgridshift" => Some(&["grids", "t_epoch", "t_final"]),
        "vgridshift" => Some(&["grids", "multiplier", "t_epoch", "t_final"]),
        "gridshift" => Some(&["grids", "interpolation", "no_z_transform", "coord_type"]),
        "xyzgridshift" => Some(&["grids", "grid_ref", "multiplier"]),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    match name {
        "hgridshift" => Ok(Some(Box::new(horizontal(params, frame)?))),
        "vgridshift" => Ok(Some(Box::new(vertical(params, frame)?))),
        "gridshift" => Ok(Some(Box::new(generic(params, frame)?))),
        "xyzgridshift" => Ok(Some(Box::new(xyz_shift(params, frame)?))),
        _ => Ok(None),
    }
}

struct Horizontal {
    sets: Vec<GridSet>,
    t_epoch: f64,
    t_final: f64,
    a: f64,
    es: f64,
}

struct Vertical {
    sets: Vec<GridSet>,
    multiplier: f64,
    t_epoch: f64,
    t_final: f64,
    a: f64,
    es: f64,
}

struct Generic {
    sets: Vec<GridSet>,
    model: Model,
    a: f64,
    es: f64,
}

fn horizontal(params: &Params, frame: &Frame) -> Result<Horizontal, CreateError> {
    let sets = load_all(required_grids(params)?, open_horizontal)?;
    Ok(Horizontal {
        sets,
        t_epoch: params.number("t_epoch")?.unwrap_or(0.0),
        t_final: parse_t_final(params)?,
        a: frame.earth.a(),
        es: frame.earth.es(),
    })
}

fn vertical(params: &Params, frame: &Frame) -> Result<Vertical, CreateError> {
    let sets = load_all(required_grids(params)?, open_vertical)?;
    Ok(Vertical {
        sets,
        multiplier: params.number("multiplier")?.unwrap_or(-1.0),
        t_epoch: params.number("t_epoch")?.unwrap_or(0.0),
        t_final: parse_t_final(params)?,
        a: frame.earth.a(),
        es: frame.earth.es(),
    })
}

fn generic(params: &Params, frame: &Frame) -> Result<Generic, CreateError> {
    let sets = load_all(required_grids(params)?, open_generic)?;
    let interpolation = match params.text("interpolation") {
        None => Interp::FromGrid,
        Some("bilinear") => Interp::Bilinear,
        Some("biquadratic") => Interp::Biquadratic,
        Some(_) => return Err(CreateError::IllegalValue("interpolation")),
    };
    let mut model = classify(&sets)?;
    model.interpolation = interpolation;
    if params.flag("no_z_transform") {
        model.height = Height::Skip;
    }
    if let Some(kind) = params.text("coord_type") {
        match kind {
            "projected" => {
                if model.space != Domain::Projected && !sets.is_empty() {
                    return Err(CreateError::IllegalValue("coord_type"));
                }
                model.space = Domain::Projected;
            }
            "geographic" => {
                if model.space == Domain::Projected {
                    return Err(CreateError::IllegalValue("coord_type"));
                }
            }
            _ => return Err(CreateError::IllegalValue("coord_type")),
        }
    }
    Ok(Generic {
        sets,
        model,
        a: frame.earth.a(),
        es: frame.earth.es(),
    })
}

fn required_grids(params: &Params) -> Result<&str, CreateError> {
    params
        .text("grids")
        .ok_or(CreateError::MissingArgument("grids"))
}

fn parse_t_final(params: &Params) -> Result<f64, CreateError> {
    match params.text("t_final") {
        None => Ok(0.0),
        Some("now") => Ok(now_year()),
        Some(_) => params
            .number("t_final")?
            .ok_or(CreateError::IllegalValue("t_final")),
    }
}

fn now_year() -> f64 {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    1970.0 + f64_u64(seconds) / (365.25 * 24.0 * 3600.0)
}

fn f64_u64(value: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a unix timestamp is far below the 53-bit mantissa"
    )]
    {
        value as f64
    }
}

fn load_all(
    text: &str,
    open: fn(&str) -> Result<GridSet, Fail>,
) -> Result<Vec<GridSet>, CreateError> {
    let mut sets = Vec::new();
    for raw in text.split(',') {
        if raw.is_empty() {
            return Err(CreateError::IllegalValue("grids"));
        }
        let (optional, name) = raw
            .strip_prefix('@')
            .map_or((false, raw), |rest| (true, rest));
        if name.is_empty() {
            return Err(CreateError::IllegalValue("grids"));
        }
        match open(name) {
            Ok(set) => sets.push(set),
            Err(Fail::Missing) if optional => {}
            Err(fail) if !optional => return Err(as_create(fail)),
            Err(_) => {}
        }
    }
    Ok(sets)
}

fn as_create(fail: Fail) -> CreateError {
    match fail {
        Fail::Missing => CreateError::IllegalValue("grids"),
        Fail::Invalid => CreateError::IllegalValue("grids"),
    }
}

fn open_horizontal(name: &str) -> Result<GridSet, Fail> {
    if name == "null" {
        return Ok(GridSet::null_grid());
    }
    let bytes = read_grid(name)?;
    if ntv2::is_ntv2(&bytes) {
        let grids = ntv2::load(&bytes).map_err(|()| Fail::Invalid)?;
        return Ok(GridSet::from_ntv2(grids));
    }
    if geotiff::is_tiff(&bytes) {
        return GridSet::from_tiff_horizontal(&bytes);
    }
    if is_ctable2(&bytes) {
        return GridSet::from_ctable2(&bytes);
    }
    Err(Fail::Invalid)
}

fn is_ctable2(bytes: &[u8]) -> bool {
    bytes.len() >= 10 && &bytes[..10] == b"CTABLE V2."
}

fn open_vertical(name: &str) -> Result<GridSet, Fail> {
    if name == "null" {
        return Ok(GridSet::null_grid());
    }
    let bytes = read_grid(name)?;
    if geotiff::is_tiff(&bytes) {
        return GridSet::from_tiff_vertical(&bytes);
    }
    if gtx_name(name) {
        return GridSet::from_gtx(&bytes);
    }
    Err(Fail::Invalid)
}

fn gtx_name(name: &str) -> bool {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
        .ends_with(".gtx")
}

fn open_generic(name: &str) -> Result<GridSet, Fail> {
    if name == "null" {
        return Ok(GridSet::null_grid());
    }
    let bytes = read_grid(name)?;
    if geotiff::is_tiff(&bytes) {
        return GridSet::from_tiff_generic(&bytes);
    }
    Err(Fail::Invalid)
}

fn read_grid(name: &str) -> Result<Vec<u8>, Fail> {
    crate::gridpath::read(name).ok_or(Fail::Missing)
}

impl GridSet {
    fn null_grid() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            first_root: None,
            null: true,
        }
    }

    fn fresh() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            first_root: None,
            null: false,
        }
    }

    fn from_ntv2(grids: Vec<ntv2::NtvGrid>) -> Self {
        let mut set = Self::fresh();
        for grid in grids {
            let spp = 2;
            let node = Node {
                width: grid.width,
                height: grid.height,
                extent: grid.extent,
                bottom_up: true,
                spp,
                samples: grid.samples,
                scale: Vec::new(),
                offset: Vec::new(),
                nodata: None,
                meta: BTreeMap::new(),
                grid_name: grid.grid_name,
                parent_name: grid.parent_name,
                children: Vec::new(),
                inherit: None,
                horizontal: Some(HorizSpec {
                    lat: 0,
                    lon: 1,
                    to_radian: 1.0,
                    positive_east: true,
                }),
                vertical: None,
            };
            set.insert(node, Attach::ParentName);
        }
        set
    }

    fn from_tiff_horizontal(bytes: &[u8]) -> Result<Self, Fail> {
        let ifds = geotiff::load(bytes).map_err(|()| Fail::Invalid)?;
        let mut set = Self::fresh();
        let mut lat_i = 0_usize;
        let mut lon_i = 1_usize;
        let mut factor = ARCSEC;
        let mut positive_east = true;
        for (index, ifd) in ifds.into_iter().enumerate() {
            if skipped_page(ifd.subfile, index)? {
                continue;
            }
            if ifd.spp < 2 {
                if index == 0 {
                    return Err(Fail::Invalid);
                }
                continue;
            }
            let mut saw = false;
            let mut saw_lat = false;
            let mut saw_lon = false;
            for band in 0..ifd.spp {
                let desc = lookup(&ifd.meta, i32::try_from(band).unwrap_or(0), "DESCRIPTION");
                if !desc.is_empty() {
                    saw = true;
                }
                if desc == "latitude_offset" {
                    lat_i = band;
                    saw_lat = true;
                } else if desc == "longitude_offset" {
                    lon_i = band;
                    saw_lon = true;
                }
            }
            if saw && !saw_lat && !saw_lon {
                if index == 0 {
                    return Err(Fail::Invalid);
                }
                continue;
            }
            if saw_lat != saw_lon || lat_i >= ifd.spp || lon_i >= ifd.spp {
                return Err(Fail::Invalid);
            }
            if saw_lon {
                let positive = lookup(
                    &ifd.meta,
                    i32::try_from(lon_i).unwrap_or(0),
                    "positive_value",
                );
                if !positive.is_empty() {
                    positive_east = match positive.as_str() {
                        "east" => true,
                        "west" => false,
                        _ => return Err(Fail::Invalid),
                    };
                }
            }
            let unit_lat = lookup(&ifd.meta, i32::try_from(lat_i).unwrap_or(0), "UNITTYPE");
            let unit_lon = lookup(&ifd.meta, i32::try_from(lon_i).unwrap_or(0), "UNITTYPE");
            if unit_lat != unit_lon {
                return Err(Fail::Invalid);
            }
            if !unit_lat.is_empty() {
                factor = match unit_lat.as_str() {
                    "arc-second" | "arc-seconds per year" => ARCSEC,
                    "radian" => 1.0,
                    "degree" => PI / 180.0,
                    _ => return Err(Fail::Invalid),
                };
            }
            let spec = HorizSpec {
                lat: lat_i,
                lon: lon_i,
                to_radian: factor,
                positive_east,
            };
            set.insert(node_from_ifd(ifd, Some(spec), None), Attach::Extent);
        }
        if set.roots.is_empty() {
            return Err(Fail::Invalid);
        }
        Ok(set)
    }

    fn from_tiff_vertical(bytes: &[u8]) -> Result<Self, Fail> {
        let ifds = geotiff::load(bytes).map_err(|()| Fail::Invalid)?;
        let mut set = Self::fresh();
        let mut sample = 0_usize;
        for (index, ifd) in ifds.into_iter().enumerate() {
            if skipped_page(ifd.subfile, index)? {
                continue;
            }
            let mut saw = false;
            let mut saw_shift = false;
            for band in 0..ifd.spp {
                let desc = lookup(&ifd.meta, i32::try_from(band).unwrap_or(0), "DESCRIPTION");
                if !desc.is_empty() {
                    saw = true;
                }
                if matches!(
                    desc.as_str(),
                    "geoid_undulation"
                        | "vertical_offset"
                        | "hydroid_height"
                        | "ellipsoidal_height_offset"
                ) {
                    sample = band;
                    saw_shift = true;
                }
            }
            if saw && !saw_shift {
                if index == 0 {
                    return Err(Fail::Invalid);
                }
                continue;
            }
            if sample >= ifd.spp {
                return Err(Fail::Invalid);
            }
            set.insert(node_from_ifd(ifd, None, Some(sample)), Attach::Extent);
        }
        if set.roots.is_empty() {
            return Err(Fail::Invalid);
        }
        Ok(set)
    }

    fn from_tiff_generic(bytes: &[u8]) -> Result<Self, Fail> {
        let ifds = geotiff::load(bytes).map_err(|()| Fail::Invalid)?;
        let mut set = Self::fresh();
        for (index, ifd) in ifds.into_iter().enumerate() {
            if skipped_page(ifd.subfile, index)? {
                continue;
            }
            set.insert(node_from_ifd(ifd, None, None), Attach::Extent);
        }
        if set.roots.is_empty() {
            return Err(Fail::Invalid);
        }
        Ok(set)
    }

    fn from_gtx(bytes: &[u8]) -> Result<Self, Fail> {
        if bytes.len() < 40 {
            return Err(Fail::Invalid);
        }
        let yorigin = be_f64(bytes, 0)?;
        let mut xorigin = be_f64(bytes, 8)?;
        let ystep = be_f64(bytes, 16)?;
        let xstep = be_f64(bytes, 24)?;
        let rows = be_i32(bytes, 32)?;
        let columns = be_i32(bytes, 36)?;
        if columns <= 0
            || rows <= 0
            || !(-360.0..=360.0).contains(&xorigin)
            || !(-90.0..=90.0).contains(&yorigin)
            || xstep <= 0.0
            || ystep <= 0.0
        {
            return Err(Fail::Invalid);
        }
        if xorigin >= 180.0 {
            xorigin -= 360.0;
        }
        let width_us = usize::try_from(columns).map_err(|_| Fail::Invalid)?;
        let height_us = usize::try_from(rows).map_err(|_| Fail::Invalid)?;
        let count = width_us.checked_mul(height_us).ok_or(Fail::Invalid)?;
        let data = bytes.get(40..40 + count * 4).ok_or(Fail::Invalid)?;
        let mut samples = Vec::with_capacity(count);
        for index in 0..count {
            let mut value = be_f32(data, index * 4)?;
            // Official GTX nodata. Stored as NaN so bilinear skips the corner.
            if (f64::from(value) + 88.8888).abs() < 1e-3 {
                value = f32::NAN;
            }
            samples.push(value);
        }
        let deg = PI / 180.0;
        let west = xorigin * deg;
        let south = yorigin * deg;
        let res_x = xstep * deg;
        let res_y = ystep * deg;
        let east = west + res_x * (f64::from(columns) - 1.0);
        let north = south + res_y * (f64::from(rows) - 1.0);
        let extent = Extent::try_new(Domain::Geographic, west, south, east, north, res_x, res_y)
            .map_err(|()| Fail::Invalid)?;
        let mut set = Self::fresh();
        set.insert(
            Node {
                width: columns,
                height: rows,
                extent,
                bottom_up: true,
                spp: 1,
                samples,
                scale: Vec::new(),
                offset: Vec::new(),
                nodata: Some(f32::NAN),
                meta: BTreeMap::new(),
                grid_name: String::new(),
                parent_name: String::new(),
                children: Vec::new(),
                inherit: None,
                horizontal: None,
                vertical: Some(0),
            },
            Attach::Extent,
        );
        if set.roots.is_empty() {
            return Err(Fail::Invalid);
        }
        Ok(set)
    }

    fn from_ctable2(bytes: &[u8]) -> Result<Self, Fail> {
        if bytes.len() < 160 || &bytes[..10] != b"CTABLE V2." {
            return Err(Fail::Invalid);
        }
        let west = le_f64(bytes, 96)?;
        let south = le_f64(bytes, 104)?;
        let res_x = le_f64(bytes, 112)?;
        let res_y = le_f64(bytes, 120)?;
        let width = le_i32(bytes, 128)?;
        let height = le_i32(bytes, 132)?;
        if width <= 0
            || height <= 0
            || west.abs() > 4.0 * PI
            || south.abs() > PI + 1e-5
            || res_x <= 1e-10
            || res_y <= 1e-10
        {
            return Err(Fail::Invalid);
        }
        let count = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .ok_or(Fail::Invalid)?;
        let data = bytes.get(160..).ok_or(Fail::Invalid)?;
        if data.len() < count * 8 {
            return Err(Fail::Invalid);
        }
        // First float is longitude, second is latitude. Already radians.
        let mut samples = Vec::with_capacity(count * 2);
        for index in 0..count {
            samples.push(le_f32(data, index * 8)?);
            samples.push(le_f32(data, index * 8 + 4)?);
        }
        // PROJ sets north from `(height - 1) * resX`, not `resY`.
        let east = west + res_x * (f64::from(width) - 1.0);
        let north = south + res_x * (f64::from(height) - 1.0);
        let extent = Extent::try_new(Domain::Geographic, west, south, east, north, res_x, res_y)
            .map_err(|()| Fail::Invalid)?;
        let mut set = Self::fresh();
        set.insert(
            Node {
                width,
                height,
                extent,
                bottom_up: true,
                spp: 2,
                samples,
                scale: Vec::new(),
                offset: Vec::new(),
                nodata: None,
                meta: BTreeMap::new(),
                grid_name: String::new(),
                parent_name: String::new(),
                children: Vec::new(),
                inherit: None,
                horizontal: Some(HorizSpec {
                    lat: 1,
                    lon: 0,
                    to_radian: 1.0,
                    positive_east: false,
                }),
                vertical: None,
            },
            Attach::Extent,
        );
        if set.roots.is_empty() {
            return Err(Fail::Invalid);
        }
        Ok(set)
    }

    fn insert(&mut self, mut node: Node, attach: Attach) {
        let own_type = node.own_meta(-1, "TYPE");
        node.inherit = if own_type.is_empty() {
            self.first_root
                .filter(|&root| !self.local_type(root).is_empty())
        } else {
            None
        };
        let parent = node.parent_name.clone();
        let name = node.grid_name.clone();
        let id = self.nodes.len();
        self.nodes.push(node);
        if !parent.is_empty() {
            if let Some(pid) = self.find_name(&parent, id) {
                let fits = self.nodes[pid].extent.contains(self.nodes[id].extent);
                if matches!(attach, Attach::ParentName) || fits {
                    self.nodes[pid].children.push(id);
                    return;
                }
            } else if matches!(attach, Attach::ParentName) {
                self.roots.push(id);
                self.note_root(id);
                return;
            }
        } else if !name.is_empty() {
            self.roots.push(id);
            self.note_root(id);
            return;
        }
        let kind = self.nodes[id].own_meta(-1, "TYPE");
        let host = self.roots.iter().copied().find(|&root| {
            root != id
                && (kind.is_empty() || self.kind_of(root) == kind)
                && self.nodes[root].extent.contains(self.nodes[id].extent)
        });
        if let Some(root) = host {
            self.nest(root, id);
        } else {
            self.roots.push(id);
            self.note_root(id);
        }
    }

    fn note_root(&mut self, id: usize) {
        if self.first_root.is_none() {
            self.first_root = Some(id);
        }
    }

    fn nest(&mut self, parent: usize, id: usize) {
        let extent = self.nodes[id].extent;
        let host = self.nodes[parent]
            .children
            .iter()
            .copied()
            .find(|&child| self.nodes[child].extent.contains(extent));
        if let Some(child) = host {
            self.nest(child, id);
        } else {
            self.nodes[parent].children.push(id);
        }
    }

    fn find_name(&self, name: &str, skip: usize) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .find_map(|(index, node)| (index != skip && node.grid_name == name).then_some(index))
    }

    fn local_type(&self, id: usize) -> String {
        self.nodes[id].own_meta(-1, "TYPE")
    }

    fn kind_of(&self, id: usize) -> String {
        self.meta_of(id, -1, "TYPE")
    }

    fn meta_of(&self, id: usize, sample: i32, key: &str) -> String {
        let own = self.nodes[id].own_meta(sample, key);
        if !own.is_empty() {
            return own;
        }
        if let Some(source) = self.nodes[id].inherit {
            return self.nodes[source].own_meta(sample, key);
        }
        String::new()
    }

    fn locate(&self, x: f64, y: f64, eps_mode: Eps) -> Option<Hit> {
        if self.null {
            return Some(Hit::Null);
        }
        for &root in &self.roots {
            let eps = match eps_mode {
                Eps::Horizontal => self.nodes[root].extent.epsilon(),
                Eps::Strict => 0.0,
            };
            if point_in(self.nodes[root].extent, x, y, eps) {
                return Some(Hit::Grid(self.deepest(root, x, y, eps_mode)));
            }
        }
        None
    }

    fn locate_type(&self, kind: &str, x: f64, y: f64) -> Option<Hit> {
        if self.null {
            return Some(Hit::Null);
        }
        for &root in &self.roots {
            if self.kind_of(root) != kind {
                continue;
            }
            if point_in(self.nodes[root].extent, x, y, 0.0) {
                return Some(Hit::Grid(self.deepest(root, x, y, Eps::Strict)));
            }
        }
        None
    }

    fn deepest(&self, id: usize, x: f64, y: f64, eps_mode: Eps) -> usize {
        for &child in &self.nodes[id].children {
            let eps = match eps_mode {
                Eps::Horizontal => self.nodes[child].extent.epsilon(),
                Eps::Strict => 0.0,
            };
            if point_in(self.nodes[child].extent, x, y, eps) {
                return self.deepest(child, x, y, eps_mode);
            }
        }
        id
    }
}

#[derive(Clone, Copy)]
enum Eps {
    Horizontal,
    Strict,
}

fn skipped_page(subfile: u32, index: usize) -> Result<bool, Fail> {
    if subfile != 0 && subfile != PAGE {
        if index == 0 {
            return Err(Fail::Invalid);
        }
        return Ok(true);
    }
    Ok(false)
}

fn lookup(meta: &BTreeMap<(i32, String), String>, sample: i32, key: &str) -> String {
    meta.get(&(sample, key.to_owned()))
        .cloned()
        .unwrap_or_default()
}

fn node_from_ifd(
    ifd: geotiff::RawIfd,
    horizontal: Option<HorizSpec>,
    vertical: Option<usize>,
) -> Node {
    let grid_name = lookup(&ifd.meta, -1, "grid_name");
    let parent_name = lookup(&ifd.meta, -1, "parent_grid_name");
    Node {
        width: ifd.width,
        height: ifd.height,
        extent: ifd.extent,
        bottom_up: ifd.bottom_up,
        spp: ifd.spp,
        samples: ifd.samples,
        scale: ifd.scale,
        offset: ifd.offset,
        nodata: ifd.nodata,
        meta: ifd.meta,
        grid_name,
        parent_name,
        children: Vec::new(),
        inherit: None,
        horizontal,
        vertical,
    }
}

fn point_in(extent: Extent, x: f64, y: f64, eps: f64) -> bool {
    if !(y + eps >= extent.south && y - eps <= extent.north) {
        return false;
    }
    if extent.full_world() {
        return true;
    }
    let mut x = x;
    if extent.geographic {
        if x + eps < extent.west {
            x += TAU;
        } else if x - eps > extent.east {
            x -= TAU;
        }
    }
    x + eps >= extent.west && x - eps <= extent.east
}

fn classify(sets: &[GridSet]) -> Result<Model, CreateError> {
    let mut has_h = false;
    let mut has_3d = false;
    let mut has_ellip = false;
    let mut has_v2v = false;
    let mut has_g2v = false;
    let mut offset_x = String::new();
    let mut offset_y = String::new();
    let mut count = 0_i32;
    let mut projected = false;
    for set in sets {
        if set.null {
            continue;
        }
        for &root in &set.roots {
            count += 1;
            match set.kind_of(root).as_str() {
                "HORIZONTAL_OFFSET" => has_h = true,
                "GEOGRAPHIC_3D_OFFSET" => has_3d = true,
                "ELLIPSOIDAL_HEIGHT_OFFSET" => has_ellip = true,
                "VERTICAL_OFFSET_VERTICAL_TO_VERTICAL" => has_v2v = true,
                "VERTICAL_OFFSET_GEOGRAPHIC_TO_VERTICAL" => has_g2v = true,
                "" => return Err(CreateError::IllegalValue("grids")),
                _ => return Err(CreateError::IllegalValue("grids")),
            }
            if offset_x.is_empty() {
                offset_x = set.meta_of(root, 0, "constant_offset");
            }
            if offset_y.is_empty() {
                offset_y = set.meta_of(root, 1, "constant_offset");
            }
            projected = !set.nodes[root].extent.geographic;
        }
    }
    if (!offset_x.is_empty() || !offset_y.is_empty()) && count > 1 {
        return Err(CreateError::IllegalValue("grids"));
    }
    let verticals = i32::from(has_ellip) + i32::from(has_v2v) + i32::from(has_g2v);
    if verticals > 1 {
        return Err(CreateError::IllegalValue("grids"));
    }
    let (main, _main_is_geog3d) = if has_3d {
        ("GEOGRAPHIC_3D_OFFSET", true)
    } else if !has_h {
        if has_ellip {
            ("ELLIPSOIDAL_HEIGHT_OFFSET", false)
        } else if has_g2v {
            ("VERTICAL_OFFSET_GEOGRAPHIC_TO_VERTICAL", false)
        } else if has_v2v {
            ("VERTICAL_OFFSET_VERTICAL_TO_VERTICAL", false)
        } else {
            ("", false)
        }
    } else {
        ("HORIZONTAL_OFFSET", false)
    };
    let aux = if has_h {
        if has_ellip {
            "ELLIPSOIDAL_HEIGHT_OFFSET"
        } else if has_g2v {
            "VERTICAL_OFFSET_GEOGRAPHIC_TO_VERTICAL"
        } else if has_v2v {
            "VERTICAL_OFFSET_VERTICAL_TO_VERTICAL"
        } else {
            ""
        }
    } else {
        ""
    };
    let offset_x = parse_offset(&offset_x)?;
    let offset_y = parse_offset(&offset_y)?;
    Ok(Model {
        main: main.to_owned(),
        aux: aux.to_owned(),
        mix: GridMix {
            geog3d: has_3d,
            horizontal: has_h,
        },
        offset_x,
        offset_y,
        interpolation: Interp::FromGrid,
        height: Height::Keep,
        space: if projected {
            Domain::Projected
        } else {
            Domain::Geographic
        },
    })
}

fn parse_offset(text: &str) -> Result<f64, CreateError> {
    if text.is_empty() {
        return Ok(0.0);
    }
    text.parse::<f64>()
        .map_err(|_| CreateError::IllegalValue("grids"))
}

fn find_horizontal(sets: &[GridSet], x: f64, y: f64) -> Option<Located<'_>> {
    for set in sets {
        if let Some(hit) = set.locate(x, y, Eps::Horizontal) {
            return Some(match hit {
                Hit::Null => Located::Null,
                Hit::Grid(id) => Located::At(set, id),
            });
        }
    }
    None
}

fn find_vertical(sets: &[GridSet], x: f64, y: f64) -> Option<Located<'_>> {
    for set in sets {
        if let Some(hit) = set.locate(x, y, Eps::Strict) {
            return Some(match hit {
                Hit::Null => Located::Null,
                Hit::Grid(id) => Located::At(set, id),
            });
        }
    }
    None
}

fn find_typed<'a>(sets: &'a [GridSet], kind: &str, x: f64, y: f64) -> Option<Located<'a>> {
    for set in sets {
        if let Some(hit) = set.locate_type(kind, x, y) {
            return Some(match hit {
                Hit::Null => Located::Null,
                Hit::Grid(id) => Located::At(set, id),
            });
        }
    }
    None
}

fn in_window(t_final: f64, t_epoch: f64, t: f64) -> bool {
    t_final == 0.0 || t_epoch == 0.0 || (t < t_epoch && t_final > t_epoch)
}

fn geographic_in(coord: Coord) -> Result<Coord, TransformError> {
    if !coord.x.is_finite() || !coord.y.is_finite() || !coord.z.is_finite() {
        return Err(TransformError::NonFinite);
    }
    if coord.y.abs() > PI / 2.0 + 1e-12 {
        return Err(TransformError::InvalidCoord);
    }
    let mut lon = coord.x;
    if lon.abs() > PI {
        if lon.abs() > 10.0 {
            return Err(TransformError::InvalidCoord);
        }
        lon = adjlon(lon);
    }
    Ok(Coord {
        x: lon,
        y: coord.y,
        z: coord.z,
        t: coord.t,
    })
}

fn floor_i32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let floored = value.floor();
    if !(0.0..2_147_483_647.0).contains(&floored) && floored >= 0.0 {
        return 0;
    }
    if floored < f64::from(i32::MIN) || floored > f64::from(i32::MAX) {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the floor was range-checked against i32"
    )]
    {
        floored as i32
    }
}

fn f32_from_f64(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "PROJ rounds a scaled grid sample back to float"
    )]
    {
        value as f32
    }
}

fn round_factor(raw: f32, factor: f64) -> f64 {
    f64::from(f32_from_f64(f64::from(raw) * factor))
}

fn snap_index(index: i32, fraction: f64, limit: i32) -> Option<(i32, f64)> {
    if index < 0 {
        if index == -1 && fraction > 1.0 - 10.0 * REL {
            Some((0, 0.0))
        } else {
            None
        }
    } else if index + 1 >= limit {
        if index + 1 == limit && fraction < 10.0 * REL {
            Some((index - 1, 1.0))
        } else {
            None
        }
    } else {
        Some((index, fraction))
    }
}

fn h_corner(node: &Node, x: i32, y: i32) -> Option<(f64, f64)> {
    let spec = node.horizontal.as_ref()?;
    let lat = node.value(spec.lat, x, y)?;
    let lon = node.value(spec.lon, x, y)?;
    let lat_r = round_factor(lat, spec.to_radian);
    let mut lon_r = round_factor(lon, spec.to_radian);
    if !spec.positive_east {
        lon_r = -lon_r;
    }
    Some((lon_r, lat_r))
}

fn h_interpolate(node: &Node, east: f64, north: f64) -> Option<(f64, f64)> {
    let x = east / node.extent.res_x;
    let y = north / node.extent.res_y;
    let mut ix = floor_i32(x);
    let mut iy = floor_i32(y);
    let mut fx = x - f64::from(ix);
    let mut fy = y - f64::from(iy);
    (ix, fx) = snap_index(ix, fx, node.width)?;
    (iy, fy) = snap_index(iy, fy, node.height)?;
    let (lon00, lat00) = h_corner(node, ix, iy)?;
    let (lon10, lat10) = h_corner(node, ix + 1, iy)?;
    let (lon01, lat01) = h_corner(node, ix, iy + 1)?;
    let (lon11, lat11) = h_corner(node, ix + 1, iy + 1)?;
    let mut west = 1.0 - fx;
    let mut east_w = fx;
    let mut north_w = west;
    let mut north_e = east_w;
    north_e *= fy;
    north_w *= fy;
    let south = 1.0 - fy;
    west *= south;
    east_w *= south;
    Some((
        west * lon00 + east_w * lon10 + north_w * lon01 + north_e * lon11,
        west * lat00 + east_w * lat10 + north_w * lat01 + north_e * lat11,
    ))
}

fn h_normalize(extent: Extent, lon: f64, lat: f64) -> (f64, f64) {
    let eps = extent.epsilon();
    let mut east = lon - extent.west;
    if east + eps < 0.0 {
        east += TAU;
    } else if east - eps > extent.east - extent.west {
        east -= TAU;
    }
    (east, lat - extent.south)
}

fn bilinear(fx: f64, fy: f64, v00: f64, v10: f64, v01: f64, v11: f64) -> f64 {
    let mut west = 1.0 - fx;
    let mut east = fx;
    let mut north_w = west;
    let mut north_e = east;
    north_e *= fy;
    north_w *= fy;
    let south = 1.0 - fy;
    west *= south;
    east *= south;
    west * v00 + east * v10 + north_w * v01 + north_e * v11
}

fn quadratic(t: f64, f0: f64, f1: f64, f2: f64) -> f64 {
    let df0 = f1 - f0;
    let df1 = f2 - f1;
    let d2 = df1 - df0;
    f0 + t * df0 + 0.5 * t * (t - 1.0) * d2
}

struct Shift {
    x: f64,
    y: f64,
    z: f64,
}

struct Plan {
    x: Option<usize>,
    y: Option<usize>,
    z: Option<usize>,
    easting: bool,
    bilinear: bool,
}

fn plan_for(set: &GridSet, id: usize, kind: &str, interp: &Interp, height: Height) -> Option<Plan> {
    let node = &set.nodes[id];
    let projected = !node.extent.geographic;
    let mut x = None;
    let mut y = None;
    let mut z = None;
    let mut easting = false;
    for band in 0..node.spp {
        let desc = set.meta_of(id, i32::try_from(band).unwrap_or(0), "DESCRIPTION");
        let unit = set.meta_of(id, i32::try_from(band).unwrap_or(0), "UNITTYPE");
        if !projected && desc == "latitude_offset" {
            if !unit.is_empty() && unit != "arc-second" {
                return None;
            }
            y = Some(band);
        } else if !projected && desc == "longitude_offset" {
            if !unit.is_empty() && unit != "arc-second" {
                return None;
            }
            x = Some(band);
        } else if projected && desc == "easting_offset" {
            if !unit.is_empty() && unit != "metre" {
                return None;
            }
            easting = true;
            x = Some(band);
        } else if projected && desc == "northing_offset" {
            if !unit.is_empty() && unit != "metre" {
                return None;
            }
            easting = true;
            y = Some(band);
        } else if matches!(
            desc.as_str(),
            "ellipsoidal_height_offset" | "geoid_undulation" | "hydroid_height" | "vertical_offset"
        ) {
            if !unit.is_empty() && unit != "metre" {
                return None;
            }
            z = Some(band);
        }
    }
    if node.spp >= 2 && y.is_none() && x.is_none() && kind == "HORIZONTAL_OFFSET" {
        if projected {
            easting = true;
            x = Some(0);
            y = Some(1);
        } else {
            x = Some(1);
            y = Some(0);
        }
    }
    if (kind == "HORIZONTAL_OFFSET" || kind == "GEOGRAPHIC_3D_OFFSET")
        && (x.is_none() || y.is_none())
    {
        return None;
    }
    if matches!(
        kind,
        "ELLIPSOIDAL_HEIGHT_OFFSET"
            | "VERTICAL_OFFSET_GEOGRAPHIC_TO_VERTICAL"
            | "VERTICAL_OFFSET_VERTICAL_TO_VERTICAL"
            | "GEOGRAPHIC_3D_OFFSET"
    ) && z.is_none()
    {
        return None;
    }
    let method = match interp {
        Interp::Bilinear => "bilinear",
        Interp::Biquadratic => "biquadratic",
        Interp::FromGrid => {
            let text = set.meta_of(id, -1, "interpolation_method");
            if text.is_empty() {
                "bilinear"
            } else if text == "bilinear" || text == "biquadratic" {
                if text == "bilinear" {
                    "bilinear"
                } else {
                    "biquadratic"
                }
            } else {
                return None;
            }
        }
    };
    let bilinear = method == "bilinear" || node.width < 3 || node.height < 3;
    if matches!(height, Height::Skip) {
        z = None;
    }
    Some(Plan {
        x,
        y,
        z,
        easting,
        bilinear,
    })
}

fn sample_raw(node: &Node, sample: usize, x: i32, y: i32) -> Option<f64> {
    let value = node.value(sample, x, y)?;
    if node.nodata_value(value) {
        return None;
    }
    Some(f64::from(value))
}

fn g_interpolate(set: &GridSet, id: usize, xy: (f64, f64), plan: &Plan) -> Option<Shift> {
    let node = &set.nodes[id];
    let extent = node.extent;
    let x = (xy.0 - extent.west) * extent.inv_x;
    let y = (xy.1 - extent.south) * extent.inv_y;
    let mut ix = floor_i32(x);
    let mut iy = floor_i32(y);
    let mut fx = x - f64::from(ix);
    let mut fy = y - f64::from(iy);
    (ix, fx) = snap_index(ix, fx, node.width)?;
    (iy, fy) = snap_index(iy, fy, node.height)?;
    if plan.bilinear {
        return Some(g_bilinear(node, ix, iy, fx, fy, plan));
    }
    if (fx <= 0.5 && ix > 0) || ix + 2 == node.width {
        ix -= 1;
        fx += 1.0;
    }
    if (fy <= 0.5 && iy > 0) || iy + 2 == node.height {
        iy -= 1;
        fy += 1.0;
    }
    g_biquad(node, ix, iy, fx, fy, plan)
}

fn g_bilinear(node: &Node, ix: i32, iy: i32, fx: f64, fy: f64, plan: &Plan) -> Shift {
    let take = |sample: usize| -> Option<f64> {
        let v00 = sample_raw(node, sample, ix, iy)?;
        let v10 = sample_raw(node, sample, ix + 1, iy)?;
        let v01 = sample_raw(node, sample, ix, iy + 1)?;
        let v11 = sample_raw(node, sample, ix + 1, iy + 1)?;
        Some(bilinear(fx, fy, v00, v10, v01, v11))
    };
    let mut shift = Shift {
        x: plan.x.and_then(take).unwrap_or(f64::NAN),
        y: plan.y.and_then(take).unwrap_or(f64::NAN),
        z: plan.z.and_then(take).unwrap_or(0.0),
    };
    if plan.x.is_none() {
        shift.x = 0.0;
    }
    if plan.y.is_none() {
        shift.y = 0.0;
    }
    finish_shift(shift, plan)
}

fn g_biquad(node: &Node, ix: i32, iy: i32, fx: f64, fy: f64, plan: &Plan) -> Option<Shift> {
    let take = |sample: usize| -> Option<f64> {
        let mut rows = [0.0; 3];
        for row in 0..3 {
            let y = iy + row;
            let x0 = sample_raw(node, sample, ix, y)?;
            let x1 = sample_raw(node, sample, ix + 1, y)?;
            let x2 = sample_raw(node, sample, ix + 2, y)?;
            rows[usize::try_from(row).ok()?] = quadratic(fx, x0, x1, x2);
        }
        Some(quadratic(fy, rows[0], rows[1], rows[2]))
    };
    let shift = Shift {
        x: match plan.x {
            Some(sample) => take(sample)?,
            None => 0.0,
        },
        y: match plan.y {
            Some(sample) => take(sample)?,
            None => 0.0,
        },
        z: match plan.z {
            Some(sample) => take(sample)?,
            None => 0.0,
        },
    };
    Some(finish_shift(shift, plan))
}

fn finish_shift(mut shift: Shift, plan: &Plan) -> Shift {
    if plan.x.is_some() && plan.y.is_some() && !plan.easting {
        shift.x *= ARCSEC;
        shift.y *= ARCSEC;
    }
    shift
}

fn normalize_xy(extent: Extent, x: f64, y: f64) -> (f64, f64) {
    if !extent.geographic {
        return (x, y);
    }
    let eps = extent.epsilon();
    let mut x = x;
    if x < extent.west - eps {
        x += TAU;
    } else if x > extent.east + eps {
        x -= TAU;
    }
    (x, y)
}

fn wrap_mod(value: f64, width: f64) -> f64 {
    let once = (value + width) % width;
    (once + width) % width
}

fn v_sample(node: &Node, lon: f64, lat: f64) -> Result<f64, TransformError> {
    bilinear_sample(node, node.vertical.unwrap_or(0), lon, lat)
}

fn bilinear_sample(node: &Node, sample: usize, lon: f64, lat: f64) -> Result<f64, TransformError> {
    let extent = node.extent;
    if !extent.geographic {
        return Err(TransformError::OutsideGrid);
    }
    let width = f64::from(node.width);
    let mut grid_x = (lon - extent.west) * extent.inv_x;
    if lon < extent.west {
        grid_x = if extent.full_world() {
            wrap_mod(grid_x, width)
        } else {
            (lon + TAU - extent.west) * extent.inv_x
        };
    } else if lon > extent.east {
        grid_x = if extent.full_world() {
            wrap_mod(grid_x, width)
        } else {
            (lon - TAU - extent.west) * extent.inv_x
        };
    }
    let grid_y = (lat - extent.south) * extent.inv_y;
    let ix = floor_i32(grid_x);
    let iy = floor_i32(grid_y);
    if ix < 0 || ix >= node.width || iy < 0 || iy >= node.height {
        return Err(TransformError::OutsideGrid);
    }
    let fx = grid_x - f64::from(ix);
    let fy = grid_y - f64::from(iy);
    let ix2 = if ix + 1 >= node.width {
        if extent.full_world() {
            0
        } else {
            node.width - 1
        }
    } else {
        ix + 1
    };
    let iy2 = if iy + 1 >= node.height {
        node.height - 1
    } else {
        iy + 1
    };
    let read = |x, y| node.value(sample, x, y).ok_or(TransformError::OutsideGrid);
    let a = read(ix, iy)?;
    let b = read(ix2, iy)?;
    let c = read(ix, iy2)?;
    let d = read(ix2, iy2)?;
    let valid = |value: f32| !node.nodata_value(value);
    let corners = [(a, valid(a)), (b, valid(b)), (c, valid(c)), (d, valid(d))];
    let count = corners.iter().filter(|(_, ok)| *ok).count();
    if count == 0 {
        return Err(TransformError::OutsideGrid);
    }
    let xy = fx * fy;
    let weights = [1.0 - fx - fy + xy, fx - xy, fy - xy, xy];
    let values = [f64::from(a), f64::from(b), f64::from(c), f64::from(d)];
    if count == 4 {
        let mut sum = 0.0;
        for index in 0..4 {
            sum += values[index] * weights[index];
        }
        return Ok(sum);
    }
    let mut sum = 0.0;
    let mut weight = 0.0;
    for index in 0..4 {
        if corners[index].1 {
            sum += values[index] * weights[index];
            weight += weights[index];
        }
    }
    if weight == 0.0 {
        return Err(TransformError::OutsideGrid);
    }
    Ok(sum / weight)
}

impl Step for Horizontal {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !in_window(self.t_final, self.t_epoch, coord.t) {
            return Ok(coord);
        }
        self.apply(coord, Way::Forward)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !in_window(self.t_final, self.t_epoch, coord.t) {
            return Ok(coord);
        }
        self.apply(coord, Way::Inverse)
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

impl Horizontal {
    fn apply(&self, coord: Coord, way: Way) -> Result<Coord, TransformError> {
        let inverse = matches!(way, Way::Inverse);
        if !self.sets.is_empty() && self.sets.iter().all(|set| set.null) {
            return Ok(coord);
        }
        let coord = geographic_in(coord)?;
        if self.sets.is_empty() {
            return Ok(coord);
        }
        let Some(located) = find_horizontal(&self.sets, coord.x, coord.y) else {
            return Err(TransformError::OutsideGrid);
        };
        let Located::At(set, mut id) = located else {
            return Ok(coord);
        };
        let (mut east, mut north) = h_normalize(set.nodes[id].extent, coord.x, coord.y);
        let Some((mut dlon, mut dlat)) = h_interpolate(&set.nodes[id], east, north) else {
            return Err(TransformError::OutsideGrid);
        };
        if !inverse {
            return Ok(Coord {
                x: coord.x + dlon,
                y: coord.y + dlat,
                z: coord.z,
                t: coord.t,
            });
        }
        let mut guess_e = east - dlon;
        let mut guess_n = north - dlat;
        let mut left = MAX_ITER;
        let mut failed = false;
        while left > 0 {
            match h_interpolate(&set.nodes[id], guess_e, guess_n) {
                None => {
                    let lon = guess_e + set.nodes[id].extent.west;
                    let lat = guess_n + set.nodes[id].extent.south;
                    let Some(Located::At(next, next_id)) = find_horizontal(&self.sets, lon, lat)
                    else {
                        failed = true;
                        break;
                    };
                    if next_id == id && std::ptr::eq(next, set) {
                        failed = true;
                        break;
                    }
                    if !std::ptr::eq(next, set) {
                        failed = true;
                        break;
                    }
                    id = next_id;
                    (east, north) = h_normalize(set.nodes[id].extent, coord.x, coord.y);
                }
                Some((shift_e, shift_n)) => {
                    dlon = shift_e;
                    dlat = shift_n;
                    let dif_e = guess_e + shift_e - east;
                    let dif_n = guess_n + shift_n - north;
                    guess_e -= dif_e;
                    guess_n -= dif_n;
                    if dif_e * dif_e + dif_n * dif_n <= TOL2 {
                        break;
                    }
                }
            }
            left -= 1;
        }
        if left == 0 {
            return Err(TransformError::OutsideGrid);
        }
        let _ = (failed, dlon, dlat);
        let extent = set.nodes[id].extent;
        Ok(Coord {
            x: adjlon(guess_e + extent.west),
            y: guess_n + extent.south,
            z: coord.z,
            t: coord.t,
        })
    }
}

impl Step for Vertical {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.shift(coord, Way::Forward)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.shift(coord, Way::Inverse)
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

impl Vertical {
    fn shift(&self, coord: Coord, way: Way) -> Result<Coord, TransformError> {
        let inverse = matches!(way, Way::Inverse);
        if !in_window(self.t_final, self.t_epoch, coord.t) {
            return Ok(coord);
        }
        let coord = geographic_in(coord)?;
        if self.sets.is_empty() {
            return Ok(coord);
        }
        let Some(located) = find_vertical(&self.sets, coord.x, coord.y) else {
            return Err(TransformError::OutsideGrid);
        };
        let Located::At(set, id) = located else {
            return Ok(coord);
        };
        let value = v_sample(&set.nodes[id], coord.x, coord.y)? * self.multiplier;
        let z = if inverse {
            coord.z - value
        } else {
            coord.z + value
        };
        Ok(Coord {
            x: coord.x,
            y: coord.y,
            z,
            t: coord.t,
        })
    }
}

impl Step for Generic {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let mut out = self.apply(coord, Way::Forward)?;
        out.x += self.model.offset_x;
        out.y += self.model.offset_y;
        Ok(out)
    }

    fn inverse(&self, mut coord: Coord) -> Result<Coord, TransformError> {
        coord.x -= self.model.offset_x;
        coord.y -= self.model.offset_y;
        self.apply(coord, Way::Inverse)
    }

    fn input_kind(&self) -> IoKind {
        if self.model.space == Domain::Projected {
            IoKind::Projected
        } else {
            IoKind::Angular
        }
    }

    fn output_kind(&self) -> IoKind {
        self.input_kind()
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

impl Generic {
    fn apply(&self, coord: Coord, way: Way) -> Result<Coord, TransformError> {
        let coord = if self.model.space == Domain::Projected {
            if !coord.x.is_finite() || !coord.y.is_finite() || !coord.z.is_finite() {
                return Err(TransformError::NonFinite);
            }
            coord
        } else {
            geographic_in(coord)?
        };
        if self.sets.is_empty() || self.model.main.is_empty() {
            return Err(TransformError::OutsideGrid);
        }
        let vertical_only = !(self.model.mix.geog3d || self.model.mix.horizontal);
        let mut kind = self.model.main.clone();
        let mut found_3d = false;
        let located = match find_typed(&self.sets, &kind, coord.x, coord.y) {
            Some(hit) => {
                if self.model.mix.geog3d {
                    found_3d = true;
                }
                hit
            }
            None if self.model.mix.geog3d && self.model.mix.horizontal => {
                kind = "HORIZONTAL_OFFSET".to_owned();
                find_typed(&self.sets, &kind, coord.x, coord.y)
                    .ok_or(TransformError::OutsideGrid)?
            }
            None => return Err(TransformError::OutsideGrid),
        };
        let pass = if vertical_only {
            Pass::Vertical
        } else {
            Pass::Full
        };
        let mut out = self.one(&kind, pass, coord, way, &located)?;
        if !found_3d && !self.model.aux.is_empty() {
            let aux = find_typed(&self.sets, &self.model.aux, out.x, out.y)
                .ok_or(TransformError::OutsideGrid)?;
            out = self.one(&self.model.aux, Pass::Vertical, out, way, &aux)?;
        }
        Ok(out)
    }

    fn one(
        &self,
        kind: &str,
        pass: Pass,
        coord: Coord,
        way: Way,
        located: &Located<'_>,
    ) -> Result<Coord, TransformError> {
        let inverse = matches!(way, Way::Inverse);
        let vertical_only = matches!(pass, Pass::Vertical);
        let Located::At(set, mut id) = located else {
            return Ok(coord);
        };
        let plan = plan_for(set, id, kind, &self.model.interpolation, self.model.height)
            .ok_or(TransformError::OutsideGrid)?;
        let mut extent = set.nodes[id].extent;
        let (mut nx, mut ny) = normalize_xy(extent, coord.x, coord.y);
        let Some(mut shift) = g_interpolate(set, id, (nx, ny), &plan) else {
            return Err(TransformError::OutsideGrid);
        };
        if !shift.x.is_finite() || !shift.y.is_finite() {
            return Err(TransformError::OutsideGrid);
        }
        if !inverse {
            return Ok(Coord {
                x: coord.x + shift.x,
                y: coord.y + shift.y,
                z: coord.z + shift.z,
                t: coord.t,
            });
        }
        if vertical_only {
            return Ok(Coord {
                x: coord.x,
                y: coord.y,
                z: coord.z - shift.z,
                t: coord.t,
            });
        }
        let mut guess_x = nx - shift.x;
        let mut guess_y = ny - shift.y;
        if plan.bilinear {
            let mut left = MAX_ITER;
            while left > 0 {
                match g_interpolate(set, id, (guess_x, guess_y), &plan) {
                    None => {
                        let Some(Located::At(next, next_id)) =
                            find_typed(&self.sets, kind, guess_x, guess_y)
                        else {
                            break;
                        };
                        if !std::ptr::eq(*set, next) || next_id == id {
                            break;
                        }
                        id = next_id;
                        extent = set.nodes[id].extent;
                        (nx, ny) = normalize_xy(extent, coord.x, coord.y);
                    }
                    Some(next) => {
                        if !next.x.is_finite() {
                            break;
                        }
                        let dif_x = guess_x + next.x - nx;
                        let dif_y = guess_y + next.y - ny;
                        shift = next;
                        guess_x -= dif_x;
                        guess_y -= dif_y;
                        if dif_x * dif_x + dif_y * dif_y <= TOL2 {
                            break;
                        }
                    }
                }
                left -= 1;
            }
            if left == 0 {
                return Err(TransformError::OutsideGrid);
            }
        }
        let x = if extent.geographic {
            adjlon(guess_x)
        } else {
            guess_x
        };
        Ok(Coord {
            x,
            y: guess_y,
            z: coord.z - shift.z,
            t: coord.t,
        })
    }
}

fn le_f64(bytes: &[u8], offset: usize) -> Result<f64, Fail> {
    let slice = bytes.get(offset..offset + 8).ok_or(Fail::Invalid)?;
    let mut raw = [0_u8; 8];
    raw.copy_from_slice(slice);
    Ok(f64::from_le_bytes(raw))
}

fn le_f32(bytes: &[u8], offset: usize) -> Result<f32, Fail> {
    let slice = bytes.get(offset..offset + 4).ok_or(Fail::Invalid)?;
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(slice);
    Ok(f32::from_le_bytes(raw))
}

fn le_i32(bytes: &[u8], offset: usize) -> Result<i32, Fail> {
    let slice = bytes.get(offset..offset + 4).ok_or(Fail::Invalid)?;
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(slice);
    Ok(i32::from_le_bytes(raw))
}

fn be_f64(bytes: &[u8], offset: usize) -> Result<f64, Fail> {
    let slice = bytes.get(offset..offset + 8).ok_or(Fail::Invalid)?;
    let mut raw = [0_u8; 8];
    raw.copy_from_slice(slice);
    Ok(f64::from_be_bytes(raw))
}

fn be_f32(bytes: &[u8], offset: usize) -> Result<f32, Fail> {
    let slice = bytes.get(offset..offset + 4).ok_or(Fail::Invalid)?;
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(slice);
    Ok(f32::from_be_bytes(raw))
}

fn be_i32(bytes: &[u8], offset: usize) -> Result<i32, Fail> {
    let slice = bytes.get(offset..offset + 4).ok_or(Fail::Invalid)?;
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(slice);
    Ok(i32::from_be_bytes(raw))
}

struct XyzShift {
    sets: Vec<GridSet>,
    /// `grid_ref=input_crs`. `output_crs` iterates on the forward path.
    grid_ref_is_input: bool,
    multiplier: f64,
    earth: crate::Ellipsoid,
    a: f64,
    es: f64,
}

fn xyz_shift(params: &Params, frame: &Frame) -> Result<XyzShift, CreateError> {
    let sets = load_all(required_grids(params)?, open_generic)?;
    if sets.is_empty() {
        return Err(CreateError::IllegalValue("grids"));
    }
    let grid_ref_is_input = match params.text("grid_ref") {
        None => true,
        Some("input_crs") => true,
        Some("output_crs") => false,
        Some(_) => return Err(CreateError::IllegalValue("grid_ref")),
    };
    let multiplier = params.number("multiplier")?.unwrap_or(1.0);
    if !multiplier.is_finite() {
        return Err(CreateError::IllegalValue("multiplier"));
    }
    for set in &sets {
        if set.null {
            continue;
        }
        for node in &set.nodes {
            if node.spp < 3 {
                return Err(CreateError::IllegalValue("grids"));
            }
            let (x_band, y_band, z_band) = translation_bands(node);
            for band in [x_band, y_band, z_band] {
                let unit = node.own_meta(i32_band(band), "UNITTYPE");
                if !unit.is_empty() && unit != "metre" {
                    return Err(CreateError::IllegalValue("grids"));
                }
            }
        }
    }
    Ok(XyzShift {
        sets,
        grid_ref_is_input,
        multiplier,
        earth: frame.earth,
        a: frame.earth.a(),
        es: frame.earth.es(),
    })
}

fn i32_band(band: usize) -> i32 {
    i32::try_from(band).unwrap_or(0)
}

fn translation_bands(node: &Node) -> (usize, usize, usize) {
    let mut x_band = 0usize;
    let mut y_band = 1usize;
    let mut z_band = 2usize;
    for band in 0..node.spp {
        match node.own_meta(i32_band(band), "DESCRIPTION").as_str() {
            "x_translation" => x_band = band,
            "y_translation" => y_band = band,
            "z_translation" => z_band = band,
            _ => {}
        }
    }
    (x_band, y_band, z_band)
}

fn grid_delta(shift: &XyzShift, lam: f64, phi: f64) -> Result<(f64, f64, f64), TransformError> {
    let Some(located) = find_horizontal(&shift.sets, lam, phi) else {
        return Err(TransformError::OutsideGrid);
    };
    let Located::At(set, id) = located else {
        return Ok((0.0, 0.0, 0.0));
    };
    let node = &set.nodes[id];
    let (x_band, y_band, z_band) = translation_bands(node);
    let scale = shift.multiplier;
    Ok((
        bilinear_sample(node, x_band, lam, phi)? * scale,
        bilinear_sample(node, y_band, lam, phi)? * scale,
        bilinear_sample(node, z_band, lam, phi)? * scale,
    ))
}

fn direct_xyz(shift: &XyzShift, mut coord: Coord, factor: f64) -> Result<Coord, TransformError> {
    let (lam, phi, _) = cartesian_to_geodetic(shift.earth, coord.x, coord.y, coord.z);
    let (dx, dy, dz) = grid_delta(shift, lam, phi)?;
    coord.x += factor * dx;
    coord.y += factor * dy;
    coord.z += factor * dz;
    Ok(coord)
}

fn iterative_xyz(shift: &XyzShift, input: Coord, factor: f64) -> Result<Coord, TransformError> {
    let mut point = input;
    for _ in 0..10 {
        let (lam, phi, _) = cartesian_to_geodetic(shift.earth, point.x, point.y, point.z);
        let (dx, dy, dz) = grid_delta(shift, lam, phi)?;
        let dx = factor * dx;
        let dy = factor * dy;
        let dz = factor * dz;
        let err = (point.x - input.x - dx).powi(2)
            + (point.y - input.y - dy).powi(2)
            + (point.z - input.z - dz).powi(2);
        point.x = input.x + dx;
        point.y = input.y + dy;
        point.z = input.z + dz;
        if err < 1e-10 {
            break;
        }
    }
    point.t = input.t;
    Ok(point)
}

impl Step for XyzShift {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if self.grid_ref_is_input {
            direct_xyz(self, coord, 1.0)
        } else {
            iterative_xyz(self, coord, 1.0)
        }
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if self.grid_ref_is_input {
            iterative_xyz(self, coord, -1.0)
        } else {
            direct_xyz(self, coord, -1.0)
        }
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}
