//! What libproj reports for a [`Proj`]: its definition, its `id`, and the area of
//! use of a CRS.
//!
//! libproj's definition of an object built from a PROJ string is not the input
//! read back. The string is read as a CRS (`proj_create` goes through the ISO 19111
//! model), that CRS is written as a PROJ string, and that string is initialised as
//! `pj_init` does: the parameters lose their `+`, and `pj_datum_set` appends the
//! ellipsoid and the `towgs84=` or `nadgrids=` of a `+datum=` it names. So
//! `+proj=longlat` reads back as `proj=longlat datum=WGS84 no_defs ellps=WGS84
//! towgs84=0,0,0`, and `+proj=tmerc` on the UTM parameters as `proj=utm`. The
//! functions here follow that path with this crate's reader and writer.
//!
//! The area of use is usage 0 of the CRS, as `proj_get_area_of_use` reads it.

use std::fmt;

use crate::ellipsoid::PJ_ELLPS;
use crate::iso19111::{create_from_user_input, DatabaseContext};
use crate::proj::{Area, Proj, ProjError};

/// `ProjInfo::description` of an object built from a PROJ string.
pub(crate) const OPERATION_DESCRIPTION: &str = "PROJ-based coordinate operation";

/// PROJ's `pj_datums` table (`datums.cpp`): the datum name, the parameters
/// `pj_datum_set` appends after its ellipsoid, and that ellipsoid.
///
/// The table is PROJ's, not the crate's own datum table: `GRS80` is a datum the
/// crate reads, and PROJ does not name it, so it is absent here.
const PROJ_DATUMS: &[(&str, &str, &str)] = &[
    ("WGS84", "towgs84=0,0,0", "WGS84"),
    ("GGRS87", "towgs84=-199.87,74.79,246.62", "GRS80"),
    ("NAD83", "towgs84=0,0,0", "GRS80"),
    (
        "NAD27",
        "nadgrids=@conus,@alaska,@ntv2_0.gsb,@ntv1_can.dat",
        "clrk66",
    ),
    ("potsdam", "nadgrids=@BETA2007.gsb", "bessel"),
    ("carthage", "towgs84=-263.0,6.0,431.0", "clrk80ign"),
    (
        "hermannskogel",
        "towgs84=577.326,90.129,463.919,5.137,1.474,5.297,2.4232",
        "bessel",
    ),
    (
        "ire65",
        "towgs84=482.530,-130.596,564.557,-1.042,-0.214,-0.631,8.15",
        "mod_airy",
    ),
    (
        "nzgd49",
        "towgs84=59.47,-5.04,187.44,0.47,-0.1,1.024,-4.5993",
        "intl",
    ),
    (
        "OSGB36",
        "towgs84=446.448,-125.157,542.060,0.1502,0.2470,0.8421,-20.4894",
        "airy",
    ),
];

/// The definition libproj reports for a PROJ string, see the module documentation.
///
/// A string `proj_create` reads as a CRS follows that path. A pipeline, `+proj=cart`, or
/// `+init=` does not: `proj_pj_info` flattens a one-step pipeline, leaves `cart` as `cart`,
/// and this crate expands `+init=` the way [`crate::crs::init_definition`] does before
/// reporting the expanded parameter list. `+init=` is rejected by PROJ 9 outside PROJ4
/// emulation; the reported text is `proj_pj_info` of the expanded operation.
///
/// # Errors
///
/// [`ProjError::Definition`] when the string names a datum PROJ does not name, or the
/// expanded `+init=` section is not a CRS this crate builds.
pub(crate) fn operation_definition(text: &str) -> Result<String, ProjError> {
    if let Some(spec) = lone_init(text) {
        let expanded = crate::crs::init_definition(&spec).map_err(|_| ProjError::Definition)?;
        return raw_parameters(&expanded);
    }
    match proj_kind(text).as_deref() {
        Some("pipeline") => pipeline_definition(text),
        Some("cart") => raw_parameters(text),
        _ => {
            let exported = export_crs(text)?;
            init_definition(&exported)
        }
    }
}

/// `+init=section` when the string is that section and, optionally, `no_defs`.
fn lone_init(text: &str) -> Option<String> {
    let mut spec = None;
    for token in text.split_whitespace() {
        let bare = token.strip_prefix('+').unwrap_or(token);
        if let Some(rest) = bare.strip_prefix("init=") {
            if spec.is_some() {
                return None;
            }
            spec = Some(rest.to_owned());
        } else if bare != "no_defs" {
            return None;
        }
    }
    spec
}

/// The first `proj=` value.
fn proj_kind(text: &str) -> Option<String> {
    text.split_whitespace().find_map(|token| {
        let bare = token.strip_prefix('+').unwrap_or(token);
        bare.strip_prefix("proj=").map(str::to_owned)
    })
}

/// `proj_pj_info` of an operation string that is already a PROJ parameter list.
///
/// A pipeline is flattened when it has one step. Anything else gets the datum
/// append and the default ellipsoid. A multi-step pipeline is left as written:
/// `pj_datum_set` does not run inside one.
pub(crate) fn parameters_of_operation(text: &str) -> Result<String, ProjError> {
    if proj_kind(text).as_deref() == Some("pipeline") {
        pipeline_definition(text)
    } else {
        raw_parameters(text)
    }
}

/// `proj_pj_info` of a pipeline: one step is the step itself, several steps stay a pipeline.
///
/// `pj_datum_set` runs only outside a pipeline, so a flattened step gets the datum
/// append and the default ellipsoid, and a multi-step pipeline does not.
fn pipeline_definition(text: &str) -> Result<String, ProjError> {
    let tokens = bare_tokens(text);
    if tokens.first().map(String::as_str) != Some("proj=pipeline") {
        return Err(ProjError::Definition);
    }
    let steps = split_steps(&tokens);
    match steps.as_slice() {
        [only] => raw_tokens(only.clone()),
        [] => Err(ProjError::Definition),
        _ => Ok(tokens.join(" ")),
    }
}

/// Tokens of each `step`, in order. The leading `proj=pipeline` is not a step.
fn split_steps(tokens: &[String]) -> Vec<Vec<String>> {
    let mut steps = Vec::new();
    let mut current = Vec::new();
    let mut seen_step = false;
    for token in tokens.iter().skip(1) {
        if token == "step" {
            if seen_step && !current.is_empty() {
                steps.push(std::mem::take(&mut current));
            }
            seen_step = true;
            continue;
        }
        if seen_step {
            current.push(token.clone());
        }
    }
    if !current.is_empty() {
        steps.push(current);
    }
    steps
}

/// Parameter list of a non-CRS operation: no `+`, no `type=crs`, then `pj_datum_set`
/// and the default ellipsoid.
fn raw_parameters(text: &str) -> Result<String, ProjError> {
    raw_tokens(bare_tokens(text))
}

fn bare_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|token| token.strip_prefix('+').unwrap_or(token).to_owned())
        .filter(|item| item != "type=crs")
        .collect()
}

fn raw_tokens(mut items: Vec<String>) -> Result<String, ProjError> {
    apply_datum(&mut items)?;
    if !has_no_defs(&items) && !has_shape(&items) {
        items.push("ellps=GRS80".to_owned());
    }
    Ok(items.join(" "))
}

fn has_no_defs(items: &[String]) -> bool {
    items.iter().any(|item| item == "no_defs")
}

/// A datum, an ellipsoid, or an axis that already names the shape.
fn has_shape(items: &[String]) -> bool {
    items.iter().any(|item| {
        let key = item.split('=').next().unwrap_or(item);
        matches!(key, "ellps" | "datum" | "a" | "b" | "R" | "rf")
    })
}

/// `pj_datum_set` for the first `datum=`: append `ellps=` and the datum parameters.
fn apply_datum(items: &mut Vec<String>) -> Result<(), ProjError> {
    let datum = items
        .iter()
        .find_map(|item| item.strip_prefix("datum="))
        .map(str::to_owned);
    let Some(name) = datum else {
        return Ok(());
    };
    let (_, parameters, ellipsoid) = PROJ_DATUMS
        .iter()
        .find(|(id, ..)| *id == name)
        .ok_or(ProjError::Definition)?;
    if !PJ_ELLPS.iter().any(|(id, ..)| id == ellipsoid) {
        return Err(ProjError::Definition);
    }
    items.push(format!("ellps={ellipsoid}"));
    items.push((*parameters).to_owned());
    Ok(())
}

/// The `proj=` name of a definition, which is libproj's `ProjInfo::id`.
pub(crate) fn proj_name(definition: &str) -> Option<String> {
    definition
        .split_whitespace()
        .find_map(|token| token.strip_prefix("proj="))
        .map(str::to_owned)
}

/// The area of use of the CRS that `text` names, as `proj_get_area_of_use` reports it.
///
/// The box is `None` when usage 0 has no bounding box. The name is the usage's
/// description. A CRS without any usage gives `(None, None)`.
///
/// # Errors
///
/// [`ProjError::UnknownAreaOfUse`] when the crate cannot read `text` as a CRS.
pub(crate) fn crs_area_of_use(text: &str) -> Result<(Option<Area>, Option<String>), ProjError> {
    let db = DatabaseContext::create().map_err(|_| ProjError::UnknownAreaOfUse)?;
    let crs = create_from_user_input(text, &db).map_err(|_| ProjError::UnknownAreaOfUse)?;
    let Some(usage) = crs.domains().first() else {
        return Ok((None, None));
    };
    let area = match (
        usage.west_bound_longitude(),
        usage.south_bound_latitude(),
        usage.east_bound_longitude(),
        usage.north_bound_latitude(),
    ) {
        (Some(west), Some(south), Some(east), Some(north)) => {
            Some(Area::new(west, south, east, north))
        }
        _ => None,
    };
    Ok((area, usage.description().map(str::to_owned)))
}

/// The PROJ string of the CRS that `text` reads as.
fn export_crs(text: &str) -> Result<String, ProjError> {
    let db = DatabaseContext::create().map_err(|_| ProjError::Definition)?;
    let crs = create_from_user_input(text, &db).map_err(|_| ProjError::Definition)?;
    crs.export_to_proj_string()
        .map_err(|_| ProjError::Definition)
}

/// `pj_init` on an exported string: the parameters without their `+`, then the
/// additions `pj_datum_set` makes for the first `datum=`.
///
/// The export ends a CRS with a `type=crs` marker that is not a PROJ parameter;
/// libproj does not report it, so it is dropped. The default ellipsoid of
/// `pj_init` (`GRS80` when a string names none) is not added: the export always
/// writes an ellipsoid, a datum, or a sphere.
fn init_definition(exported: &str) -> Result<String, ProjError> {
    let mut items: Vec<String> = exported
        .split_whitespace()
        .map(|token| token.strip_prefix('+').unwrap_or(token).to_owned())
        .filter(|item| item != "type=crs")
        .collect();
    let datum = items
        .iter()
        .find_map(|item| item.strip_prefix("datum="))
        .map(str::to_owned);
    if let Some(name) = datum {
        let (_, parameters, ellipsoid) = PROJ_DATUMS
            .iter()
            .find(|(id, ..)| *id == name)
            .ok_or(ProjError::Definition)?;
        if !PJ_ELLPS.iter().any(|(id, ..)| id == ellipsoid) {
            return Err(ProjError::Definition);
        }
        items.push(format!("ellps={ellipsoid}"));
        items.push((*parameters).to_owned());
    }
    Ok(items.join(" "))
}

impl fmt::Debug for Proj {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let info = self.proj_info();
        formatter
            .debug_struct("Proj")
            .field("id", &info.id)
            .field("description", &info.description)
            .field("definition", &info.definition)
            .field("has_inverse", &info.has_inverse)
            .field("accuracy", &info.accuracy)
            .finish()
    }
}
