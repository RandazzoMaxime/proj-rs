//! A small JSON tree for the defmodel tests. `test_defmodel.cpp` edits a
//! `nlohmann::json` value in place (`j["a"]["b"] = ...`, `erase`, `push_back`);
//! this module does the same edits by path, then writes the text that
//! `MasterFile::parse` reads.

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Json {
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

pub(crate) fn s(text: &str) -> Json {
    Json::Str(text.to_owned())
}

pub(crate) fn n(value: f64) -> Json {
    Json::Num(value)
}

pub(crate) fn obj(pairs: Vec<(&str, Json)>) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

pub(crate) fn arr(items: Vec<Json>) -> Json {
    Json::Arr(items)
}

/// `j[path...] = value`. Missing object members are created, as with
/// `operator[]`. Array indices must exist.
pub(crate) fn set(root: &mut Json, path: &[&str], value: Json) {
    let (last, parents) = path.split_last().expect("non-empty path");
    let mut node = root;
    for key in parents {
        node = member_mut(node, key);
    }
    match node {
        Json::Obj(members) => {
            if let Some(slot) = members.iter_mut().find(|(k, _)| k == last) {
                slot.1 = value;
            } else {
                members.push(((*last).to_owned(), value));
            }
        }
        Json::Arr(items) => {
            let index: usize = last.parse().expect("array index");
            items[index] = value;
        }
        _ => panic!("cannot set a member of a scalar"),
    }
}

/// `j[path...].erase(key)`: removes the last key of `path`.
pub(crate) fn remove(root: &mut Json, path: &[&str]) {
    let (last, parents) = path.split_last().expect("non-empty path");
    let mut node = root;
    for key in parents {
        node = member_mut(node, key);
    }
    if let Json::Obj(members) = node {
        members.retain(|(k, _)| k != last);
    } else {
        panic!("erase on a non-object");
    }
}

/// `j[path...].clear()`: empties the array or object at `path`.
pub(crate) fn clear(root: &mut Json, path: &[&str]) {
    let mut node = root;
    for key in path {
        node = member_mut(node, key);
    }
    match node {
        Json::Arr(items) => items.clear(),
        Json::Obj(members) => members.clear(),
        _ => panic!("clear on a scalar"),
    }
}

/// The member `key` of an object, or the element at index `key` of an array.
fn member_mut<'a>(node: &'a mut Json, key: &str) -> &'a mut Json {
    match node {
        Json::Obj(members) => {
            if let Some(index) = members.iter().position(|(k, _)| k == key) {
                &mut members[index].1
            } else {
                members.push((key.to_owned(), Json::Obj(Vec::new())));
                let last = members.len() - 1;
                &mut members[last].1
            }
        }
        Json::Arr(items) => {
            let index: usize = key.parse().expect("array index");
            &mut items[index]
        }
        _ => panic!("member of a scalar"),
    }
}

impl Json {
    /// The JSON text of the value.
    pub(crate) fn dump(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Num(value) => {
                let _ = write!(out, "{value}");
            }
            Self::Str(text) => {
                out.push('"');
                for c in text.chars() {
                    if c == '"' || c == '\\' {
                        out.push('\\');
                    }
                    out.push(c);
                }
                out.push('"');
            }
            Self::Arr(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Obj(members) => {
                out.push('{');
                for (index, (key, value)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    Self::Str(key.clone()).write(out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }

    /// The member names of an object, in order.
    pub(crate) fn keys(&self) -> Vec<String> {
        match self {
            Self::Obj(members) => members.iter().map(|(k, _)| k.clone()).collect(),
            _ => Vec::new(),
        }
    }
}

pub(crate) const MODEL_MIN_X: f64 = 158.0;
pub(crate) const MODEL_MIN_Y: f64 = -58.0;
pub(crate) const MODEL_MAX_X: f64 = 194.0;
pub(crate) const MODEL_MAX_Y: f64 = -25.0;

pub(crate) const IDX_CONSTANT: usize = 0;
pub(crate) const IDX_VELOCITY: usize = 1;
pub(crate) const IDX_STEP: usize = 2;
pub(crate) const IDX_REVERSE_STEP: usize = 3;
pub(crate) const IDX_PIECEWISE: usize = 4;
pub(crate) const IDX_EXPONENTIAL: usize = 5;

fn model_bbox() -> Json {
    arr(vec![
        n(MODEL_MIN_X),
        n(MODEL_MIN_Y),
        n(MODEL_MAX_X),
        n(MODEL_MAX_Y),
    ])
}

/// `getMinValidContent()` of `test_defmodel.cpp`.
pub(crate) fn min_valid() -> Json {
    obj(vec![
        ("file_type", s("GeoTIFF")),
        ("format_version", s("1.0")),
        ("source_crs", s("EPSG:4959")),
        ("target_crs", s("EPSG:7907")),
        ("definition_crs", s("EPSG:4959")),
        (
            "extent",
            obj(vec![
                ("type", s("bbox")),
                ("parameters", obj(vec![("bbox", model_bbox())])),
            ]),
        ),
        (
            "time_extent",
            obj(vec![
                ("first", s("1900-01-01T00:00:00Z")),
                ("last", s("2050-01-01T00:00:00Z")),
            ]),
        ),
        ("components", arr(Vec::new())),
    ])
}

/// One component of `getFullValidContent()`, before its time function is set.
pub(crate) fn full_component() -> Json {
    obj(vec![
        ("description", s("description")),
        ("displacement_type", s("horizontal")),
        ("uncertainty_type", s("none")),
        ("horizontal_uncertainty", n(0.01)),
        ("vertical_uncertainty", n(0.02)),
        (
            "extent",
            obj(vec![
                ("type", s("bbox")),
                ("parameters", obj(vec![("bbox", model_bbox())])),
            ]),
        ),
        (
            "spatial_model",
            obj(vec![
                ("type", s("GeoTIFF")),
                ("interpolation_method", s("bilinear")),
                ("filename", s("nzgd2000-ndm-grid02.tif")),
                ("md5_checksum", s("49fce8ab267be2c8d00d43683060a032")),
            ]),
        ),
        (
            "time_function",
            obj(vec![("type", s("constant")), ("parameters", obj(vec![]))]),
        ),
    ])
}

/// `getFullValidContent()` of `test_defmodel.cpp`: the six time functions,
/// one component each, in the order of the `IDX_*` constants.
pub(crate) fn full_valid() -> Json {
    let mut j = min_valid();
    set(&mut j, &["name"], s("name"));
    set(&mut j, &["version"], s("version"));
    set(&mut j, &["publication_date"], s("2018-07-01T00:00:00Z"));
    set(&mut j, &["license"], s("license"));
    set(&mut j, &["description"], s("description"));
    set(
        &mut j,
        &["authority"],
        obj(vec![
            ("name", s("authority_name")),
            ("url", s("authority_url")),
            ("address", s("authority_address")),
            ("email", s("authority_email")),
        ]),
    );
    set(
        &mut j,
        &["links"],
        arr(vec![obj(vec![
            ("href", s("href")),
            ("rel", s("rel")),
            ("type", s("type")),
            ("title", s("title")),
        ])]),
    );
    set(&mut j, &["reference_epoch"], s("2000-01-01T00:00:00Z"));
    set(
        &mut j,
        &["uncertainty_reference_epoch"],
        s("2018-12-15T00:00:00Z"),
    );
    set(&mut j, &["horizontal_offset_method"], s("addition"));
    set(&mut j, &["horizontal_offset_unit"], s("metre"));
    set(&mut j, &["vertical_offset_unit"], s("metre"));
    set(
        &mut j,
        &["horizontal_uncertainty_type"],
        s("circular 95% confidence limit"),
    );
    set(&mut j, &["horizontal_uncertainty_unit"], s("metre"));
    set(
        &mut j,
        &["vertical_uncertainty_type"],
        s("95% confidence limit"),
    );
    set(&mut j, &["vertical_uncertainty_unit"], s("metre"));

    let time_functions = [
        obj(vec![("type", s("constant")), ("parameters", obj(vec![]))]),
        obj(vec![
            ("type", s("velocity")),
            (
                "parameters",
                obj(vec![("reference_epoch", s("2000-01-01T00:00:00Z"))]),
            ),
        ]),
        obj(vec![
            ("type", s("step")),
            (
                "parameters",
                obj(vec![("step_epoch", s("2000-01-01T00:00:00Z"))]),
            ),
        ]),
        obj(vec![
            ("type", s("reverse_step")),
            (
                "parameters",
                obj(vec![("step_epoch", s("2000-01-01T00:00:00Z"))]),
            ),
        ]),
        obj(vec![
            ("type", s("piecewise")),
            (
                "parameters",
                obj(vec![
                    ("before_first", s("zero")),
                    ("after_last", s("constant")),
                    (
                        "model",
                        arr(vec![
                            obj(vec![
                                ("epoch", s("2016-01-01T00:00:00Z")),
                                ("scale_factor", n(0.5)),
                            ]),
                            obj(vec![
                                ("epoch", s("2017-01-01T00:00:00Z")),
                                ("scale_factor", n(1.0)),
                            ]),
                            obj(vec![
                                ("epoch", s("2017-01-01T00:00:00Z")),
                                ("scale_factor", n(2.0)),
                            ]),
                            obj(vec![
                                ("epoch", s("2018-01-01T00:00:00Z")),
                                ("scale_factor", n(1.0)),
                            ]),
                        ]),
                    ),
                ]),
            ),
        ]),
        obj(vec![
            ("type", s("exponential")),
            (
                "parameters",
                obj(vec![
                    ("reference_epoch", s("2000-01-01T00:00:00Z")),
                    ("end_epoch", s("2001-01-01T00:00:00Z")),
                    ("relaxation_constant", n(2.0)),
                    ("before_scale_factor", n(0.0)),
                    ("initial_scale_factor", n(1.0)),
                    ("final_scale_factor", n(3.0)),
                ]),
            ),
        ]),
    ];
    let components: Vec<Json> = time_functions
        .into_iter()
        .map(|time_function| {
            let mut component = full_component();
            set(&mut component, &["time_function"], time_function);
            component
        })
        .collect();
    set(&mut j, &["components"], arr(components));
    j
}
