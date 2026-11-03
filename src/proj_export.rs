//! Export of a [`Proj`] to WKT and PROJJSON: `proj_as_wkt` and `proj_as_projjson` of libproj.
//!
//! A [`Proj`] built from a CRS (an `EPSG:` code, WKT, PROJJSON, or a `+type=crs` PROJ
//! string) is read again with [`create_from_user_input`] and written by the ISO 19111
//! exporters, in the dialect the caller asks for. libproj writes the object from its
//! model every time, so the text the caller passed in is not returned unchanged.
//!
//! This module is a child of `proj`, so it reads the private fields of
//! [`WktOptions`] directly: the options are honoured here, not stored and ignored.

use super::{Body, Proj, ProjError, WktOptions, WktOutputAxis, WktVersion};
use crate::iso19111::{
    create_from_user_input, normalize_for_visualization, CoordinateOperation, Crs, DatabaseContext,
    EllipsoidalHeightPolicy, JsonFormat, JsonLines, LinunitPolicy, OutputAxisRule, WktConvention,
    WktFormatter, WktLines, WktStrictness,
};

/// Indentation of WKT output, PROJ's default.
const WKT_INDENT: usize = 4;

/// The WKT text of `proj`, in `version` (WKT2:2019 when `None`).
///
/// # Errors
///
/// [`ProjError::ExportToWkt`] when `proj` is not a CRS this crate can read and write, or
/// the dialect cannot hold the CRS (WKT1 with a construct it does not define).
pub(super) fn as_wkt(
    proj: &Proj,
    version: Option<WktVersion>,
    options: Option<WktOptions>,
) -> Result<String, ProjError> {
    let (object, db) = exportable_of(proj).ok_or(ProjError::ExportToWkt)?;
    let version = version.unwrap_or(WktVersion::Wkt2_2019);
    let options = options.unwrap_or_default();
    let mut formatter = WktFormatter::create(convention(version)).with_database(db);
    formatter.set_indentation_width(options.indentation_width.unwrap_or(WKT_INDENT));
    if let Some(multiline) = options.multiline {
        formatter.set_lines(if multiline {
            WktLines::Multi
        } else {
            WktLines::Single
        });
    }
    match options.output_axis {
        None | Some(WktOutputAxis::Auto) => {}
        Some(WktOutputAxis::Yes) => {
            formatter.set_output_axis(OutputAxisRule::Yes);
        }
        Some(WktOutputAxis::No) => {
            formatter.set_output_axis(OutputAxisRule::No);
        }
    }
    if let Some(strict) = options.strict {
        formatter.set_strictness(if strict {
            WktStrictness::Strict
        } else {
            WktStrictness::Lenient
        });
    }
    if let Some(allow) = options.allow_ellipsoidal_height_as_vertical_crs {
        formatter.set_ellipsoidal_height(if allow {
            EllipsoidalHeightPolicy::AllowAsVertical
        } else {
            EllipsoidalHeightPolicy::Reject
        });
    }
    if let Some(allow) = options.allow_linunit_node {
        formatter.set_linunit(if allow {
            LinunitPolicy::Allow
        } else {
            LinunitPolicy::Reject
        });
    }
    match object {
        Exportable::Crs(crs) => crs.export_to_wkt(&mut formatter),
        Exportable::Operation(operation) => operation.export_to_wkt(&mut formatter),
    }
    .map_err(|_| ProjError::ExportToWkt)
}

/// The dialect of a georust [`WktVersion`].
fn convention(version: WktVersion) -> WktConvention {
    match version {
        WktVersion::Wkt2_2015 => WktConvention::Wkt2_2015,
        WktVersion::Wkt2_2015_Simplified => WktConvention::Wkt2_2015Simplified,
        WktVersion::Wkt2_2019 => WktConvention::Wkt2_2019,
        WktVersion::Wkt2_2019_Simplified => WktConvention::Wkt2_2019Simplified,
        WktVersion::Wkt1_Gdal => WktConvention::Wkt1Gdal,
        WktVersion::Wkt1_Esri => WktConvention::Wkt1Esri,
    }
}

/// What a [`Proj`] exports as: the CRS it was built from, or the ISO operation behind a pair.
enum Exportable {
    Crs(Box<Crs>),
    Operation(Box<CoordinateOperation>),
}

/// The object `proj_as_wkt` and `proj_as_projjson` write for `proj`.
///
/// A CRS is read again from its definition. A pair from [`Proj::new_known_crs`] exports the
/// ISO 19111 operation its pipeline was written from, with the axis order of the CRS as the
/// database gives it. libproj normalises that operation for visualization, so it is the normalised one. A side
/// that cannot be reversed (a compound CRS with a latitude-first horizontal part) is not
/// exported.
fn exportable_of(proj: &Proj) -> Option<(Exportable, DatabaseContext)> {
    let db = DatabaseContext::create().ok()?;
    match &proj.body {
        Body::Single(_) => {}
        Body::Native(_) if has_crs_token(&proj.definition) => {}
        Body::Iso(pipeline) => {
            let normalized = normalize_for_visualization(pipeline.chosen())?;
            return Some((Exportable::Operation(Box::new(normalized)), db));
        }
        Body::Native(_) | Body::Between { .. } => return None,
    }
    let crs = create_from_user_input(&proj.definition, &db).ok()?;
    Some((Exportable::Crs(Box::new(crs)), db))
}

/// Whether a PROJ string carries the `+type=crs` token.
fn has_crs_token(definition: &str) -> bool {
    definition.split_whitespace().any(|token| {
        let bare = token.strip_prefix('+').unwrap_or(token);
        bare == "type=crs"
    })
}

/// The PROJJSON text of `proj`, with the layout and schema the caller asks for. An unset
/// option keeps PROJ's default: multi-line, two spaces, the v0.7 schema.
///
/// # Errors
///
/// [`ProjError::ExportToJson`] when `proj` is not a CRS this crate can write as PROJJSON.
pub(super) fn to_projjson(
    proj: &Proj,
    multiline: Option<bool>,
    indentation_width: Option<usize>,
    schema: Option<&str>,
) -> Result<String, ProjError> {
    let (object, db) = exportable_of(proj).ok_or(ProjError::ExportToJson)?;
    let mut format = JsonFormat::with_lines(match multiline {
        Some(false) => JsonLines::Single,
        None | Some(true) => JsonLines::Multi,
    });
    if let Some(width) = indentation_width {
        format = format.with_indent_width(width);
    }
    if let Some(schema) = schema {
        format = format.with_schema(schema);
    }
    match object {
        Exportable::Crs(crs) => crs.export_to_json_with(&db, &format),
        Exportable::Operation(operation) => operation.export_to_json_with(&db, &format),
    }
    .map_err(|_| ProjError::ExportToJson)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proj(definition: &str) -> Proj {
        Proj::new(definition).expect("crs")
    }

    #[test]
    fn single_line_is_the_multi_line_text_joined() {
        let proj = proj("EPSG:4326");
        let multi = as_wkt(&proj, Some(WktVersion::Wkt2_2019), None).expect("multi");
        let options = WktOptions {
            multiline: Some(false),
            ..WktOptions::default()
        };
        let single = as_wkt(&proj, Some(WktVersion::Wkt2_2019), Some(options)).expect("single");
        let joined: String = multi.lines().map(str::trim).collect();
        assert_eq!(single, joined);
    }

    #[test]
    fn indentation_width_is_honoured() {
        let proj = proj("EPSG:4326");
        let options = WktOptions {
            indentation_width: Some(2),
            ..WktOptions::default()
        };
        let text = as_wkt(&proj, Some(WktVersion::Wkt2_2019), Some(options)).expect("wkt");
        let second = text.lines().nth(1).unwrap_or_default();
        assert!(second.starts_with("  ENSEMBLE["), "{text}");
        assert!(!second.starts_with("   "), "{text}");
    }

    #[test]
    fn wkt1_esri_is_single_line_unless_asked() {
        let proj = proj("EPSG:4326");
        let esri = as_wkt(&proj, Some(WktVersion::Wkt1_Esri), None).expect("esri");
        assert!(!esri.contains('\n'), "{esri}");
        let gdal = as_wkt(&proj, Some(WktVersion::Wkt1_Gdal), None).expect("gdal");
        assert!(gdal.contains('\n'), "{gdal}");
    }

    #[test]
    fn a_pair_exports_its_operation_and_a_proj_operation_exports_nothing() {
        let pair = Proj::new_known_crs("EPSG:2230", "EPSG:26946", None).expect("pair");
        assert!(matches!(
            exportable_of(&pair),
            Some((Exportable::Operation(_), _))
        ));
        let operation = Proj::new("+proj=utm +zone=32 +ellps=WGS84").expect("utm");
        assert!(exportable_of(&operation).is_none());
        let crs = Proj::new("+proj=longlat +datum=WGS84 +no_defs +type=crs").expect("crs");
        assert!(matches!(exportable_of(&crs), Some((Exportable::Crs(_), _))));
    }
}
