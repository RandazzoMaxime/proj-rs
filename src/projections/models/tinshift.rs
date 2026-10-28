//! Triangulation shift (`+proj=tinshift`).
//!
//! JSON follows `TINShiftJSONFile::parse` and `FindTriangle` in
//! `tinshift_json_impl.hpp`. A GeoPackage is read into that same vertex
//! layout: source coordinates come from the GeoPackage point, and a vertical
//! component is stored as `target_z - source_z`, which is what the JSON parser
//! does before `TINShiftJSONEvaluator`.

use std::path::Path;

use rusqlite::Connection;

use super::json::{self, Value};
use crate::engine::{Coord, CreateError, Frame, Params, Step, TransformError};

const EPS: f64 = 1e-10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fallback {
    None,
    NearestSide,
    NearestCentroid,
}

struct Tin {
    horizontal: bool,
    vertical: bool,
    columns: usize,
    vertices: Vec<f64>,
    triangles: Vec<[usize; 3]>,
    fallback: Fallback,
}

struct TinShift {
    tin: Tin,
    a: f64,
    es: f64,
}

pub(super) fn keys() -> &'static [&'static str] {
    &["file"]
}

pub(super) fn build(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let Some(file) = params.text("file") else {
        return Err(CreateError::MissingArgument("file"));
    };
    if file.is_empty() {
        return Err(CreateError::MissingArgument("file"));
    }
    let tin = if has_suffix(file, ".gpkg") {
        let path = super::resolve(file)?;
        load_gpkg(&path)?
    } else {
        let bytes = super::read_resource(file)?;
        if bytes.len() > 100 * 1024 * 1024 {
            return Err(CreateError::IllegalValue("file"));
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| CreateError::IllegalValue("file"))?;
        parse_json(text).ok_or(CreateError::IllegalValue("file"))?
    };
    Ok(Box::new(TinShift {
        tin,
        a: frame.earth.a(),
        es: frame.earth.es(),
    }))
}

fn has_suffix(name: &str, suffix: &str) -> bool {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
        .ends_with(suffix)
}

fn parse_json(text: &str) -> Option<Tin> {
    let root = json::parse(text)?.as_object()?.clone();
    if json::req_str(&root, "file_type")? != "triangulation_file" {
        return None;
    }
    let version = json::req_str(&root, "format_version")?;
    if !version.starts_with("1.") {
        return None;
    }
    let mut fallback = Fallback::None;
    if root.contains_key("fallback_strategy") {
        if version != "1.1" {
            return None;
        }
        fallback = match json::opt_str(&root, "fallback_strategy")? {
            "nearest_side" => Fallback::NearestSide,
            "nearest_centroid" => Fallback::NearestCentroid,
            "none" => Fallback::None,
            _ => return None,
        };
    }
    let mut horizontal = false;
    let mut vertical = false;
    for item in json::arr(&root, "transformed_components")? {
        match item.as_str()? {
            "horizontal" => horizontal = true,
            "vertical" => vertical = true,
            _ => return None,
        }
    }
    let columns = json::arr(&root, "vertices_columns")?;
    let mut source_x = None;
    let mut source_y = None;
    let mut source_z = None;
    let mut target_x = None;
    let mut target_y = None;
    let mut target_z = None;
    let mut offset_z = None;
    for (index, column) in columns.iter().enumerate() {
        match column.as_str()? {
            "source_x" => source_x = Some(index),
            "source_y" => source_y = Some(index),
            "source_z" => source_z = Some(index),
            "target_x" => target_x = Some(index),
            "target_y" => target_y = Some(index),
            "target_z" => target_z = Some(index),
            "offset_z" => offset_z = Some(index),
            _ => {}
        }
    }
    let source_x = source_x?;
    let source_y = source_y?;
    if horizontal && (target_x.is_none() || target_y.is_none()) {
        return None;
    }
    if vertical && offset_z.is_none() && (source_z.is_none() || target_z.is_none()) {
        return None;
    }
    let tri_cols = json::arr(&root, "triangles_columns")?;
    let mut i1 = None;
    let mut i2 = None;
    let mut i3 = None;
    for (index, column) in tri_cols.iter().enumerate() {
        match column.as_str()? {
            "idx_vertex1" => i1 = Some(index),
            "idx_vertex2" => i2 = Some(index),
            "idx_vertex3" => i3 = Some(index),
            _ => {}
        }
    }
    let (i1, i2, i3) = (i1?, i2?, i3?);
    let mut stored = 2;
    if horizontal {
        stored += 2;
    }
    if vertical {
        stored += 1;
    }
    let mut vertices = Vec::new();
    let raw = json::arr(&root, "vertices")?;
    for vertex in raw {
        let row = vertex.as_array()?;
        if row.len() != columns.len() {
            return None;
        }
        vertices.push(row.get(source_x)?.as_f64()?);
        vertices.push(row.get(source_y)?.as_f64()?);
        if horizontal {
            vertices.push(row.get(target_x?)?.as_f64()?);
            vertices.push(row.get(target_y?)?.as_f64()?);
        }
        if vertical {
            if let Some(offset) = offset_z {
                vertices.push(row.get(offset)?.as_f64()?);
            } else {
                let source = row.get(source_z?)?.as_f64()?;
                let target = row.get(target_z?)?.as_f64()?;
                vertices.push(target - source);
            }
        }
    }
    let mut triangles = Vec::new();
    for triangle in json::arr(&root, "triangles")? {
        let row = triangle.as_array()?;
        if row.len() != tri_cols.len() {
            return None;
        }
        let a = index_of(row.get(i1)?)?;
        let b = index_of(row.get(i2)?)?;
        let c = index_of(row.get(i3)?)?;
        if a >= raw.len() || b >= raw.len() || c >= raw.len() {
            return None;
        }
        triangles.push([a, b, c]);
    }
    Some(Tin {
        horizontal,
        vertical,
        columns: stored,
        vertices,
        triangles,
        fallback,
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions,
    reason = "a vertex index in these files is a non-negative integer below 2^24"
)]
fn index_of(value: &Value) -> Option<usize> {
    let number = value.as_f64()?;
    if !(0.0..16_777_216.0).contains(&number) || number.fract() != 0.0 {
        return None;
    }
    Some(number as usize)
}

fn load_gpkg(path: &Path) -> Result<Tin, CreateError> {
    let connection = Connection::open(path).map_err(|_| CreateError::IllegalValue("file"))?;
    let metadata: String = connection
        .query_row(
            "SELECT metadata FROM gpkg_metadata WHERE md_standard_uri = 'https://proj.org'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| CreateError::IllegalValue("file"))?;
    let root = json::parse(&metadata)
        .and_then(|value| value.as_object().cloned())
        .ok_or(CreateError::IllegalValue("file"))?;
    if json::req_str(&root, "file_type") != Some("triangulation_file") {
        return Err(CreateError::IllegalValue("file"));
    }
    let version = json::req_str(&root, "format_version").unwrap_or("");
    if !version.starts_with("1.") {
        return Err(CreateError::IllegalValue("file"));
    }
    let mut horizontal = false;
    let mut vertical = false;
    let components =
        json::arr(&root, "transformed_components").ok_or(CreateError::IllegalValue("file"))?;
    for item in components {
        match item.as_str() {
            Some("horizontal") => horizontal = true,
            Some("vertical") => vertical = true,
            _ => return Err(CreateError::IllegalValue("file")),
        }
    }
    let mut fallback = Fallback::None;
    if root.contains_key("fallback_strategy") {
        fallback = match json::req_str(&root, "fallback_strategy") {
            Some("nearest_side") => Fallback::NearestSide,
            Some("nearest_centroid") => Fallback::NearestCentroid,
            Some("none") => Fallback::None,
            _ => return Err(CreateError::IllegalValue("file")),
        };
    }
    let mut names = Vec::new();
    {
        let mut info = connection
            .prepare("PRAGMA table_info(vertices)")
            .map_err(|_| CreateError::IllegalValue("file"))?;
        let rows = info
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|_| CreateError::IllegalValue("file"))?;
        for name in rows {
            let name = name.map_err(|_| CreateError::IllegalValue("file"))?;
            if name != "fid" && name != "geom" {
                names.push(name);
            }
        }
    }
    let has = |needle: &str| names.iter().any(|name| name == needle);
    if horizontal && (!has("target_x") || !has("target_y")) {
        return Err(CreateError::IllegalValue("file"));
    }
    if vertical && !((has("source_z") && has("target_z")) || has("offset_z")) {
        return Err(CreateError::IllegalValue("file"));
    }
    let offset_z = has("offset_z");
    let source_z = has("source_z");
    let select = vertex_sql(horizontal, vertical, source_z, offset_z);
    let extra_count = if horizontal { 2 } else { 0 }
        + if vertical {
            if offset_z {
                1
            } else {
                2
            }
        } else {
            0
        };
    let stored = 2 + if horizontal { 2 } else { 0 } + if vertical { 1 } else { 0 };
    let mut statement = connection
        .prepare(&select)
        .map_err(|_| CreateError::IllegalValue("file"))?;
    let mut fid_to_index = Vec::new();
    let mut vertices = Vec::new();
    let rows = statement
        .query_map([], |row| {
            let fid: i64 = row.get(0)?;
            let geom: Vec<u8> = row.get(1)?;
            let mut extras = Vec::new();
            for column in 0..extra_count {
                extras.push(row.get::<_, f64>(2 + column)?);
            }
            Ok((fid, geom, extras))
        })
        .map_err(|_| CreateError::IllegalValue("file"))?;
    for row in rows {
        let (fid, geom, extras) = row.map_err(|_| CreateError::IllegalValue("file"))?;
        let (x, y) = gpkg_xy(&geom).ok_or(CreateError::IllegalValue("file"))?;
        let index = vertices.len() / stored;
        fid_to_index.push((fid, index));
        vertices.push(x);
        vertices.push(y);
        if horizontal {
            vertices.push(*extras.first().ok_or(CreateError::IllegalValue("file"))?);
            vertices.push(*extras.get(1).ok_or(CreateError::IllegalValue("file"))?);
        }
        if vertical {
            if has("offset_z") {
                let offset = *extras.last().ok_or(CreateError::IllegalValue("file"))?;
                vertices.push(offset);
            } else {
                let base = if horizontal { 2 } else { 0 };
                let source = *extras.get(base).ok_or(CreateError::IllegalValue("file"))?;
                let target = *extras
                    .get(base + 1)
                    .ok_or(CreateError::IllegalValue("file"))?;
                vertices.push(target - source);
            }
        }
    }
    let mut triangles = Vec::new();
    let mut tri = connection
        .prepare("SELECT idx_vertex1, idx_vertex2, idx_vertex3 FROM triangles_def")
        .map_err(|_| CreateError::IllegalValue("file"))?;
    let mapped = tri
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|_| CreateError::IllegalValue("file"))?;
    for item in mapped {
        let (a, b, c): (i64, i64, i64) = item.map_err(|_| CreateError::IllegalValue("file"))?;
        triangles.push([
            lookup_fid(&fid_to_index, a)?,
            lookup_fid(&fid_to_index, b)?,
            lookup_fid(&fid_to_index, c)?,
        ]);
    }
    if vertices.is_empty() || triangles.is_empty() {
        return Err(CreateError::IllegalValue("file"));
    }
    Ok(Tin {
        horizontal,
        vertical,
        columns: stored,
        vertices,
        triangles,
        fallback,
    })
}

fn vertex_sql(horizontal: bool, vertical: bool, source_z: bool, offset_z: bool) -> String {
    let mut sql = String::from("SELECT fid, geom");
    if horizontal {
        sql.push_str(", target_x, target_y");
    }
    if vertical {
        if offset_z {
            sql.push_str(", offset_z");
        } else if source_z {
            sql.push_str(", source_z, target_z");
        }
    }
    sql.push_str(" FROM vertices ORDER BY fid");
    sql
}

fn lookup_fid(map: &[(i64, usize)], fid: i64) -> Result<usize, CreateError> {
    map.iter()
        .find(|(key, _)| *key == fid)
        .map(|(_, index)| *index)
        .ok_or(CreateError::IllegalValue("file"))
}

fn gpkg_xy(blob: &[u8]) -> Option<(f64, f64)> {
    if blob.len() < 8 || blob[0] != b'G' || blob[1] != b'P' {
        return None;
    }
    let envelope = (blob[3] >> 1) & 0b111;
    let extra: usize = match envelope {
        0 => 0,
        1 => 32,
        2 | 3 => 48,
        4 => 64,
        _ => return None,
    };
    let wkb = 8 + extra;
    let order = *blob.get(wkb)?;
    let little = order == 1;
    let x = f64_bytes(blob.get(wkb + 5..wkb + 13)?, little)?;
    let y = f64_bytes(blob.get(wkb + 13..wkb + 21)?, little)?;
    Some((x, y))
}

fn f64_bytes(bytes: &[u8], little: bool) -> Option<f64> {
    let mut raw = [0u8; 8];
    raw.copy_from_slice(bytes);
    if !little {
        raw.reverse();
    }
    Some(f64::from_le_bytes(raw))
}

impl Step for TinShift {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (x, y, z) = self
            .tin
            .apply(coord.x, coord.y, coord.z, true)
            .ok_or(TransformError::OutsideDomain)?;
        Ok(Coord {
            x,
            y,
            z,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (x, y, z) = self
            .tin
            .apply(coord.x, coord.y, coord.z, false)
            .ok_or(TransformError::OutsideDomain)?;
        Ok(Coord {
            x,
            y,
            z,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> crate::engine::IoKind {
        crate::engine::IoKind::Any
    }

    fn output_kind(&self) -> crate::engine::IoKind {
        crate::engine::IoKind::Any
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

impl Tin {
    fn vertex(&self, index: usize, column: usize) -> f64 {
        self.vertices[index * self.columns + column]
    }

    fn apply(&self, x: f64, y: f64, z: f64, forward: bool) -> Option<(f64, f64, f64)> {
        let (l1, l2, l3, triangle) = find_triangle(self, x, y, forward)?;
        let [i1, i2, i3] = triangle;
        let (x_out, y_out) = if self.horizontal {
            let col_x = if forward { 2 } else { 0 };
            let col_y = if forward { 3 } else { 1 };
            (
                self.vertex(i1, col_x) * l1
                    + self.vertex(i2, col_x) * l2
                    + self.vertex(i3, col_x) * l3,
                self.vertex(i1, col_y) * l1
                    + self.vertex(i2, col_y) * l2
                    + self.vertex(i3, col_y) * l3,
            )
        } else {
            (x, y)
        };
        let z_out = if self.vertical {
            let column = if self.horizontal { 4 } else { 2 };
            let delta = self.vertex(i1, column) * l1
                + self.vertex(i2, column) * l2
                + self.vertex(i3, column) * l3;
            if forward {
                z + delta
            } else {
                z - delta
            }
        } else {
            z
        };
        Some((x_out, y_out, z_out))
    }
}

fn find_triangle(tin: &Tin, x: f64, y: f64, forward: bool) -> Option<(f64, f64, f64, [usize; 3])> {
    let col_x = if tin.horizontal && !forward { 2 } else { 0 };
    let col_y = if tin.horizontal && !forward { 3 } else { 1 };
    for triangle in &tin.triangles {
        if let Some((l1, l2, l3)) = barycentric(tin, *triangle, col_x, col_y, x, y, true) {
            return Some((l1, l2, l3, *triangle));
        }
    }
    if tin.fallback == Fallback::None {
        return None;
    }
    let mut best_d2 = f64::INFINITY;
    let mut best: Option<[usize; 3]> = None;
    for triangle in &tin.triangles {
        let [i1, i2, i3] = *triangle;
        let x1 = tin.vertex(i1, col_x);
        let y1 = tin.vertex(i1, col_y);
        let x2 = tin.vertex(i2, col_x);
        let y2 = tin.vertex(i2, col_y);
        let x3 = tin.vertex(i3, col_x);
        let y3 = tin.vertex(i3, col_y);
        let dist12 = squared(x1, y1, x2, y2);
        let dist23 = squared(x2, y2, x3, y3);
        let dist13 = squared(x1, y1, x3, y3);
        if dist12 < EPS || dist23 < EPS || dist13 < EPS {
            continue;
        }
        let d2 = match tin.fallback {
            Fallback::NearestSide => dist_segment(x, y, x1, y1, x2, y2, dist12)
                .min(dist_segment(x, y, x2, y2, x3, y3, dist23))
                .min(dist_segment(x, y, x1, y1, x3, y3, dist13)),
            Fallback::NearestCentroid => {
                let cx = (x1 + x2 + x3) / 3.0;
                let cy = (y1 + y2 + y3) / 3.0;
                squared(x, y, cx, cy)
            }
            Fallback::None => continue,
        };
        if d2 < best_d2 {
            best_d2 = d2;
            best = Some(*triangle);
        }
    }
    let triangle = best?;
    let (l1, l2, l3) = barycentric(tin, triangle, col_x, col_y, x, y, false)?;
    Some((l1, l2, l3, triangle))
}

fn barycentric(
    tin: &Tin,
    triangle: [usize; 3],
    col_x: usize,
    col_y: usize,
    x: f64,
    y: f64,
    inside: bool,
) -> Option<(f64, f64, f64)> {
    let [i1, i2, i3] = triangle;
    let x1 = tin.vertex(i1, col_x);
    let y1 = tin.vertex(i1, col_y);
    let x2 = tin.vertex(i2, col_x);
    let y2 = tin.vertex(i2, col_y);
    let x3 = tin.vertex(i3, col_x);
    let y3 = tin.vertex(i3, col_y);
    let det = (y2 - y3) * (x1 - x3) + (x3 - x2) * (y1 - y3);
    if det.abs() < EPS {
        return None;
    }
    let l1 = ((y2 - y3) * (x - x3) + (x3 - x2) * (y - y3)) / det;
    let l2 = ((y3 - y1) * (x - x3) + (x1 - x3) * (y - y3)) / det;
    let l3 = 1.0 - l1 - l2;
    if inside && !(l1 >= -EPS && l1 <= 1.0 + EPS && l2 >= -EPS && l2 <= 1.0 + EPS && l3 >= 0.0) {
        return None;
    }
    Some((l1, l2, l3))
}

fn squared(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x1 - x2;
    let dy = y1 - y2;
    dx * dx + dy * dy
}

fn dist_segment(x: f64, y: f64, x1: f64, y1: f64, x2: f64, y2: f64, dist12: f64) -> f64 {
    let t = ((x - x1) * (x2 - x1) + (y - y1) * (y2 - y1)) / dist12;
    if t <= 0.0 {
        squared(x, y, x1, y1)
    } else if t >= 1.0 {
        squared(x, y, x2, y2)
    } else {
        squared(x, y, x1 + t * (x2 - x1), y1 + t * (y2 - y1))
    }
}
