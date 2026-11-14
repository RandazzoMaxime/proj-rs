//! The data index of PROJ (`files.geojson`) and the selection and download of its files: the
//! library of the `projsync` command (`src/bin/projsync.rs`).
//!
//! A port of PROJ 9.9.0 `src/apps/projsync.cpp`. Nothing here prints: the caller decides what
//! to show, from the [`Selection`] it gets back. The index is read with a small JSON reader
//! with the same limits as nlohmann in `projsync`: nesting of 128 at most, file sizes as
//! unsigned integers.
//!
//! Where this differs from the C++ program, the reason is in `docs/PORTING-network-cache.md`:
//! a download is not conditional, so a file whose record has expired is downloaded again.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;

use crate::network_cache::{self, DiskChunkCache, FileProperties};

/// Nesting limit of the index, `depth >= 128` in `projsync.cpp`.
const MAX_DEPTH: usize = 128;

/// The index file, served next to the data files.
pub const INDEX_FILE_NAME: &str = "files.geojson";

/// The error of a download: the URL that could not be fetched or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadError {
    url: String,
}

impl DownloadError {
    /// The URL that failed.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot download {}", self.url)
    }
}

impl std::error::Error for DownloadError {}

/// The error of an index that cannot be read. The message is the one `projsync` prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexError(String);

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for IndexError {}

/// A window of longitudes and latitudes, in degrees, as `--bbox` gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bbox {
    /// Western longitude.
    pub west: f64,
    /// Southern latitude.
    pub south: f64,
    /// Eastern longitude.
    pub east: f64,
    /// Northern latitude.
    pub north: f64,
}

impl Bbox {
    /// A window from `--bbox west,south,east,north`. A window that crosses the antimeridian is
    /// brought into the range of the grids, as `projsync.cpp` does: `west > 180` or `west < -180`
    /// with `east > west` is shifted by 360, and a window of less than 180 degrees whose east
    /// is less than its west is taken eastwards.
    #[must_use]
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        let (mut west, mut east) = (west, east);
        if west > 180.0 && east > west {
            west -= 360.0;
            east -= 360.0;
        } else if west < -180.0 && east > west {
            west += 360.0;
            east += 360.0;
        } else if west.abs() < 180.0 && east.abs() < 180.0 && east < west {
            east += 360.0;
        }
        Self {
            west,
            south,
            east,
            north,
        }
    }
}

/// How a `--bbox` window is compared with a grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpatialTest {
    /// The grid and the window overlap (`--spatial-test intersects`, the default).
    #[default]
    Intersects,
    /// The grid lies inside the window (`--spatial-test contains`).
    Contains,
}

/// Whether grids that cover the whole world take part in a `--bbox` query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldCoverage {
    /// They do (the default).
    #[default]
    Include,
    /// They are left out (`--exclude-world-coverage`).
    Exclude,
}

/// Whether files that another PROJ data version does not use are left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VersionFiltering {
    /// They are (the default): `version_added` and `version_removed` are compared with the
    /// data version of the database.
    #[default]
    Apply,
    /// They are not (`--no-version-filtering`).
    Ignore,
}

/// What to select. Each criterion is empty, or `None`, when it is not asked for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    /// A part of the source id (`--source-id`).
    pub source_id: String,
    /// A part of the area of use (`--area-of-use`).
    pub area_of_use: String,
    /// A part of the file name (`--file`).
    pub file: String,
    /// The window of `--bbox`.
    pub bbox: Option<Bbox>,
    /// Every file, as `--all`.
    pub all: bool,
    /// The comparison of `--bbox`.
    pub spatial_test: SpatialTest,
    /// Whether the world-wide grids are kept for `--bbox`.
    pub world_coverage: WorldCoverage,
    /// Whether the data version filters the files.
    pub version_filtering: VersionFiltering,
}

/// The geometry of a file, reduced to what `--bbox` needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geometry {
    /// No geometry: the file matches only `--all`.
    Absent,
    /// A geometry that cannot be used: the file never matches a `--bbox`.
    Unusable,
    /// A grid that covers the whole world in longitude and most of it in latitude.
    World {
        /// Southern latitude.
        south: f64,
        /// Northern latitude.
        north: f64,
    },
    /// The box of the grid.
    Bounds {
        /// Western longitude.
        west: f64,
        /// Southern latitude.
        south: f64,
        /// Eastern longitude.
        east: f64,
        /// Northern latitude.
        north: f64,
    },
}

/// One file of the index.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The file name, `properties.name`.
    pub name: String,
    /// `properties.source_id`, when it is a string.
    pub source_id: Option<String>,
    /// `properties.area_of_use`, when it is a string.
    pub area_of_use: Option<String>,
    /// `properties.file_size` in bytes, 0 when it is missing or not an unsigned integer.
    pub file_size: u64,
    /// `properties.version_added`, when it is a string.
    pub version_added: Option<String>,
    /// `properties.version_removed`, when it is a string.
    pub version_removed: Option<String>,
    /// The geometry of the file.
    pub geometry: Geometry,
}

/// The files of an index, in the order of the index.
#[derive(Clone, Debug, PartialEq)]
pub struct Index {
    entries: Vec<Entry>,
}

impl Index {
    /// Read an index: a GeoJSON feature collection. A feature is used when it has an object
    /// `properties` with a string `name`, as `projsync.cpp` requires.
    ///
    /// # Errors
    ///
    /// [`IndexError`] when the text is not JSON, is nested too deeply, or has no `features`.
    pub fn parse(text: &str) -> Result<Self, IndexError> {
        let root = Json::parse(text).map_err(IndexError)?;
        let Json::Object(members) = &root else {
            return Err(IndexError("no features member".to_owned()));
        };
        let Some(features) = member(members, "features") else {
            return Err(IndexError("no features member".to_owned()));
        };
        let features: Vec<&Json> = match features {
            Json::Array(items) => items.iter().collect(),
            Json::Object(members) => members.iter().map(|(_, value)| value).collect(),
            _ => Vec::new(),
        };
        let entries = features.into_iter().filter_map(entry_of).collect();
        Ok(Self { entries })
    }

    /// The files, in the order of the index.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

/// What happened to one file of the index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Decision<'a> {
    /// The file is for a later data version than the database: `version_added` is after it.
    TooRecent {
        /// The file.
        entry: &'a Entry,
        /// The `version_added` of the file.
        version: &'a str,
    },
    /// The file is for an earlier data version than the database: `version_removed` is not
    /// after it.
    Removed {
        /// The file.
        entry: &'a Entry,
        /// The `version_removed` of the file.
        version: &'a str,
    },
    /// The file matches the query.
    Matched(&'a Entry),
}

/// The result of [`select`]: what happened to each file, and the values a query could have
/// used, for the "unknown value" messages.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection<'a> {
    /// One decision per file that is not left out before matching, in the order of the index.
    pub decisions: Vec<Decision<'a>>,
    /// Source ids of the files, sorted.
    pub source_ids: BTreeSet<String>,
    /// Areas of use of the files, sorted.
    pub areas_of_use: BTreeSet<String>,
    /// File names of the files, sorted.
    pub files: BTreeSet<String>,
    /// Whether some file has the source id asked for.
    pub found_source_id: bool,
    /// Whether some file has the area of use asked for.
    pub found_area_of_use: bool,
    /// Whether some file has the name asked for.
    pub found_file: bool,
}

/// Select the files of `index` for `query`, against the data version of the database
/// (`data_version`, as `major.minor`, `None` when the database has none).
#[must_use]
pub fn select<'a>(
    index: &'a Index,
    query: &Query,
    data_version: Option<(i64, i64)>,
) -> Selection<'a> {
    let mut selection = Selection {
        decisions: Vec::new(),
        source_ids: BTreeSet::new(),
        areas_of_use: BTreeSet::new(),
        files: BTreeSet::new(),
        found_source_id: false,
        found_area_of_use: false,
        found_file: false,
    };
    let filtering = query.version_filtering == VersionFiltering::Apply;
    for entry in &index.entries {
        if let Some((major, minor)) = data_version.filter(|(major, _)| *major > 0 && filtering) {
            if let Some(version) = entry.version_added.as_deref() {
                if let Some((added_major, added_minor)) = version_pair(version) {
                    if major < added_major || (major == added_major && minor < added_minor) {
                        selection
                            .decisions
                            .push(Decision::TooRecent { entry, version });
                        continue;
                    }
                }
            }
            if let Some(version) = entry.version_removed.as_deref() {
                if let Some((removed_major, removed_minor)) = version_pair(version) {
                    if major > removed_major || (major == removed_major && minor >= removed_minor) {
                        selection
                            .decisions
                            .push(Decision::Removed { entry, version });
                        continue;
                    }
                }
            }
        }
        let _names = selection.files.insert(entry.name.clone());
        let Some(source_id) = entry.source_id.as_deref() else {
            continue;
        };
        let _sources = selection.source_ids.insert(source_id.to_owned());
        let area_of_use = entry.area_of_use.as_deref().unwrap_or("");
        if let Some(area) = entry.area_of_use.as_deref() {
            let _areas = selection.areas_of_use.insert(area.to_owned());
        }

        let matches_source =
            query.all || query.source_id.is_empty() || source_id.contains(query.source_id.as_str());
        if !query.source_id.is_empty() && source_id.contains(query.source_id.as_str()) {
            selection.found_source_id = true;
        }
        let matches_area = query.all
            || query.area_of_use.is_empty()
            || area_of_use.contains(query.area_of_use.as_str());
        if !query.area_of_use.is_empty() && area_of_use.contains(query.area_of_use.as_str()) {
            selection.found_area_of_use = true;
        }
        let matches_file = query.all || query.file.is_empty() || entry.name.contains(&query.file);
        if !query.file.is_empty() && entry.name.contains(&query.file) {
            selection.found_file = true;
        }
        let matches_bbox = bbox_matches(entry.geometry, query);
        if matches_file && matches_source && matches_area && matches_bbox {
            selection.decisions.push(Decision::Matched(entry));
        }
    }
    selection
}

/// Whether the geometry of a file meets the `--bbox` of `query` (`matchBbox` in `projsync.cpp`).
fn bbox_matches(geometry: Geometry, query: &Query) -> bool {
    if !query.all && query.bbox.is_none() {
        return true;
    }
    // The box of the grid, for the test of `--bbox`. A world-wide grid spans all longitudes,
    // and a grid east of 180 is moved west by 360 when the window starts before -180.
    let (west, east, south, north) = match geometry {
        Geometry::Absent => return query.all,
        Geometry::Unusable => return false,
        Geometry::World { south, north } => {
            if query.world_coverage == WorldCoverage::Exclude {
                return false;
            }
            if query.all {
                return true;
            }
            (-f64::MAX, f64::MAX, south, north)
        }
        Geometry::Bounds {
            west,
            south,
            east,
            north,
        } => {
            let shift = query.bbox.is_some_and(|bbox| bbox.west < -180.0) && east > 180.0;
            if query.all {
                return true;
            }
            if shift {
                (west - 360.0, east - 360.0, south, north)
            } else {
                (west, east, south, north)
            }
        }
    };
    let Some(bbox) = query.bbox else {
        return false;
    };
    match query.spatial_test {
        SpatialTest::Intersects => {
            bbox.west < east && west < bbox.east && bbox.south < north && south < bbox.north
        }
        SpatialTest::Contains => {
            west >= bbox.west && south >= bbox.south && east <= bbox.east && north <= bbox.north
        }
    }
}

/// The path of a file of `url`: its last component, without a query string.
#[must_use]
pub fn file_name_of(url: &str) -> Option<&str> {
    let without_query = url.split('?').next()?;
    let name = without_query.rsplit('/').next()?;
    (!name.is_empty() && name != "." && name != "..").then_some(name)
}

/// Whether `url` must be downloaded into `target_dir`: its file is missing, or its record in
/// the database of `target_dir` is missing or has expired (`proj_is_download_needed`).
#[must_use]
pub fn is_download_needed(target_dir: &Path, url: &str) -> bool {
    let Some(name) = file_name_of(url) else {
        return true;
    };
    if !target_dir.join(name).is_file() {
        return true;
    }
    let Some(cache) = DiskChunkCache::open(&target_dir.join(network_cache::DEFAULT_FILE_NAME))
    else {
        return true;
    };
    match cache.download_properties(url) {
        None => true,
        Some(props) => network_cache::is_expired(&props, network_cache::DEFAULT_TTL_SECONDS),
    }
}

/// Download `url` into `target_dir` under its file name, and record it in the database of
/// `target_dir` (`proj_download_file`). The file is written under a temporary name first.
///
/// # Errors
///
/// [`DownloadError`] when the server does not answer with the file or it cannot be written.
pub fn download(target_dir: &Path, url: &str) -> Result<PathBuf, DownloadError> {
    let failed = || DownloadError {
        url: url.to_owned(),
    };
    let name = file_name_of(url).ok_or_else(failed)?;
    let fetched = crate::network::http_get_validated(url).ok_or_else(failed)?;
    fs::create_dir_all(target_dir).map_err(|_| failed())?;
    let destination = target_dir.join(name);
    let temporary = target_dir.join(format!(".{name}.download"));
    fs::write(&temporary, &fetched.body).map_err(|_| failed())?;
    if destination.is_file() {
        let _removed = fs::remove_file(&destination);
    }
    if fs::rename(&temporary, &destination).is_err() {
        let _removed = fs::remove_file(&temporary);
        return Err(failed());
    }
    let props = FileProperties {
        size: u64::try_from(fetched.body.len()).unwrap_or(u64::MAX),
        last_checked: network_cache::unix_now(),
        last_modified: fetched.last_modified,
        etag: fetched.etag,
    };
    if let Some(cache) = DiskChunkCache::open(&target_dir.join(network_cache::DEFAULT_FILE_NAME)) {
        let _recorded = cache.set_download_properties(url, &props);
    }
    Ok(destination)
}

/// The `PROJ_DATA.VERSION` of the embedded database, as `major.minor` and its text.
#[must_use]
pub fn database_data_version() -> Option<(i64, i64, String)> {
    let text: String = crate::projdb::with_db(|connection| {
        connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'PROJ_DATA.VERSION'",
                [],
                |row| row.get(0),
            )
            .optional()
    })
    .ok()
    .flatten()?;
    let (major, minor) = version_pair(&text).unwrap_or((0, 0));
    Some((major, minor, text))
}

/// `major` and `minor` of a `major.minor...` version, as `atoi` reads each part. `None` when
/// there are fewer than two parts.
fn version_pair(text: &str) -> Option<(i64, i64)> {
    let mut parts = text.split('.');
    let major = atoi(parts.next()?);
    let minor = atoi(parts.next()?);
    Some((major, minor))
}

/// The leading integer of `text`, as C `atoi`: optional sign, then digits, else 0.
fn atoi(text: &str) -> i64 {
    let trimmed = text.trim_start();
    let (sign, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let number: String = digits.chars().take_while(char::is_ascii_digit).collect();
    number.parse::<i64>().map_or(0, |value| sign * value)
}

fn member<'a>(members: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    members
        .iter()
        .rev()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

fn entry_of(feature: &Json) -> Option<Entry> {
    let Json::Object(members) = feature else {
        return None;
    };
    let Json::Object(properties) = member(members, "properties")? else {
        return None;
    };
    let Json::Str(name) = member(properties, "name")? else {
        return None;
    };
    let string_member = |key: &str| match member(properties, key) {
        Some(Json::Str(text)) => Some(text.clone()),
        _ => None,
    };
    let file_size = match member(properties, "file_size") {
        Some(Json::Number {
            unsigned: Some(size),
            ..
        }) => *size,
        _ => 0,
    };
    Some(Entry {
        name: name.clone(),
        source_id: string_member("source_id"),
        area_of_use: string_member("area_of_use"),
        file_size,
        version_added: string_member("version_added"),
        version_removed: string_member("version_removed"),
        geometry: geometry_of(member(members, "geometry")),
    })
}

fn geometry_of(geometry: Option<&Json>) -> Geometry {
    let Some(Json::Object(members)) = geometry else {
        return Geometry::Absent;
    };
    let Some(coordinates) = member(members, "coordinates") else {
        return Geometry::Unusable;
    };
    let Json::Array(items) = coordinates else {
        return Geometry::Unusable;
    };
    let Some(Json::Str(kind)) = member(members, "type") else {
        return Geometry::Unusable;
    };
    let bounds = if kind == "MultiPolygon" {
        multipolygon_bounds(items)
    } else {
        get_bbox(coordinates)
    };
    let &[west, south, east, north] = bounds.as_slice() else {
        return Geometry::Unusable;
    };
    if east - west > 359.0 && north - south > 179.0 {
        return Geometry::World { south, north };
    }
    Geometry::Bounds {
        west,
        south,
        east,
        north,
    }
}

/// The box of a MultiPolygon: the polygon boxes merged, with a polygon that starts at -180
/// moved to 180 when the set has polygons on both sides of the antimeridian.
fn multipolygon_bounds(polygons: &[Json]) -> Vec<f64> {
    let mut found_minus_180 = false;
    let mut found_plus_180 = false;
    let mut boxes: Vec<Vec<f64>> = Vec::new();
    for polygon in polygons {
        if !matches!(polygon, Json::Array(_)) {
            continue;
        }
        let tmp = get_bbox(polygon);
        if tmp.len() == 4 {
            if tmp[0] == -180.0 {
                found_minus_180 = true;
            } else if tmp[2] == 180.0 {
                found_plus_180 = true;
            }
            boxes.push(tmp);
        }
    }
    let mut merged: Vec<f64> = Vec::new();
    for mut bbox in boxes {
        if found_minus_180 && found_plus_180 && bbox[0] == -180.0 {
            bbox[0] = 180.0;
            bbox[2] += 360.0;
        }
        if merged.is_empty() {
            merged = bbox;
        } else {
            merged[0] = merged[0].min(bbox[0]);
            merged[1] = merged[1].min(bbox[1]);
            merged[2] = merged[2].max(bbox[2]);
            merged[3] = merged[3].max(bbox[3]);
        }
    }
    merged
}

/// The box of the coordinates of `json`, `get_bbox` in `projsync.cpp`: a point gives a degenerate
/// box, a list of arrays gives the union of the boxes of its arrays that have one.
fn get_bbox(json: &Json) -> Vec<f64> {
    let Json::Array(items) = json else {
        return Vec::new();
    };
    if let [Json::Number { value: x, .. }, Json::Number { value: y, .. }] = items.as_slice() {
        return vec![*x, *y, *x, *y];
    }
    let mut result: Vec<f64> = Vec::new();
    for item in items {
        if !matches!(item, Json::Array(_)) {
            continue;
        }
        let sub = get_bbox(item);
        if sub.len() != 4 {
            continue;
        }
        if result.is_empty() {
            result = sub;
        } else {
            result[0] = result[0].min(sub[0]);
            result[1] = result[1].min(sub[1]);
            result[2] = result[2].max(sub[2]);
            result[3] = result[3].max(sub[3]);
        }
    }
    result
}

/// A JSON value, as far as the index needs it.
#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool,
    Number { value: f64, unsigned: Option<u64> },
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Parse a whole document: one value, surrounded by white space.
    fn parse(text: &str) -> Result<Json, String> {
        let mut parser = Reader {
            bytes: text.as_bytes(),
            text,
            position: 0,
        };
        let value = parser.value(0)?;
        parser.skip_space();
        if parser.position != parser.bytes.len() {
            return Err(parser.error("unexpected trailing characters"));
        }
        Ok(value)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    text: &'a str,
    position: usize,
}

impl Reader<'_> {
    fn error(&self, what: &str) -> String {
        format!(
            "[json.exception.parse_error] at byte {}: {what}",
            self.position
        )
    }

    fn skip_space(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        self.skip_space();
        match self.peek() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.literal("true", Json::Bool),
            Some(b'f') => self.literal("false", Json::Bool),
            Some(b'n') => self.literal("null", Json::Null),
            Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.number(),
            _ => Err(self.error("unexpected character")),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.position..].starts_with(word.as_bytes()) {
            self.position += word.len();
            Ok(value)
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn enter(&self, depth: usize) -> Result<(), String> {
        if depth >= MAX_DEPTH {
            Err("Too deep nesting in JSON content".to_owned())
        } else {
            Ok(())
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, String> {
        self.enter(depth)?;
        self.position += 1;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, String> {
        self.enter(depth)?;
        self.position += 1;
        let mut members = Vec::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_space();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a key"));
            }
            let key = self.string()?;
            self.skip_space();
            if self.peek() != Some(b':') {
                return Err(self.error("expected ':'"));
            }
            self.position += 1;
            let value = self.value(depth)?;
            members.push((key, value));
            self.skip_space();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    return Ok(Json::Object(members));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        let digits_start = self.position;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
        if self.position == digits_start {
            return Err(self.error("invalid number"));
        }
        let mut integer = true;
        if self.peek() == Some(b'.') {
            integer = false;
            self.position += 1;
            while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                self.position += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integer = false;
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                self.position += 1;
            }
        }
        let literal = &self.text[start..self.position];
        let value: f64 = literal.parse().map_err(|_| self.error("invalid number"))?;
        let unsigned = if integer {
            literal.parse::<u64>().ok()
        } else {
            None
        };
        Ok(Json::Number { value, unsigned })
    }

    fn string(&mut self) -> Result<String, String> {
        self.position += 1;
        let mut out = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.error("unterminated string"));
            };
            match byte {
                b'"' => {
                    self.position += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.position += 1;
                    let escape = self.peek().ok_or_else(|| self.error("bad escape"))?;
                    self.position += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => return Err(self.error("bad escape")),
                    }
                }
                0..=0x1f => return Err(self.error("control character in a string")),
                _ => {
                    let rest = &self.text[self.position..];
                    let ch = rest.chars().next().ok_or_else(|| self.error("bad text"))?;
                    out.push(ch);
                    self.position += ch.len_utf8();
                }
            }
        }
    }

    fn unicode_escape(&mut self) -> Result<char, String> {
        let first = self.hex4()?;
        let code = if (0xd800..0xdc00).contains(&first) {
            if self.bytes.get(self.position..self.position + 2) != Some(b"\\u") {
                return Err(self.error("lone surrogate"));
            }
            self.position += 2;
            let second = self.hex4()?;
            if !(0xdc00..0xe000).contains(&second) {
                return Err(self.error("lone surrogate"));
            }
            0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00)
        } else {
            first
        };
        char::from_u32(code).ok_or_else(|| self.error("invalid code point"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self
            .bytes
            .get(self.position..self.position + 4)
            .ok_or_else(|| self.error("short unicode escape"))?;
        let text = std::str::from_utf8(digits).map_err(|_| self.error("bad unicode escape"))?;
        let value = u32::from_str_radix(text, 16).map_err(|_| self.error("bad unicode escape"))?;
        self.position += 4;
        Ok(value)
    }
}
