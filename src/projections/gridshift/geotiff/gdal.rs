//! The GDAL metadata of a GeoTIFF: band descriptions, scale and offset, and the nodata value.

use std::collections::BTreeMap;

use super::samples::f32_from_f64;

pub(super) const GDAL_METADATA: u16 = 42_112;
pub(super) const GDAL_NODATA: u16 = 42_113;

pub(super) struct GdalFields {
    pub(super) meta: BTreeMap<(i32, String), String>,
    pub(super) scale: Vec<f64>,
    pub(super) offset: Vec<f64>,
}

pub(super) fn parse_gdal(text: &str, spp: usize) -> GdalFields {
    let mut meta = BTreeMap::new();
    let mut scale = Vec::new();
    let mut offset = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<Item ") {
        rest = &rest[start..];
        let Some(end_tag) = rest.find('>') else { break };
        let Some(end_rel) = rest[end_tag + 1..].find('<') else {
            break;
        };
        let end_value = end_tag + 1 + end_rel;
        let tag = &rest[..end_tag];
        let value = &rest[end_tag + 1..end_value];
        let Some(name) = attr(tag, "name") else { break };
        let sample = attr(tag, "sample")
            .and_then(|text| text.parse::<i32>().ok())
            .unwrap_or(-1);
        if let Some(role) = attr(tag, "role") {
            if (role == "offset" || role == "scale")
                && sample >= 0
                && usize::try_from(sample).ok().is_some_and(|slot| slot < spp)
            {
                if scale.is_empty() {
                    scale.resize(spp, 1.0);
                    offset.resize(spp, 0.0);
                }
                if let (Ok(number), Ok(slot)) = (value.parse::<f64>(), usize::try_from(sample)) {
                    if role == "offset" {
                        offset[slot] = number;
                    } else {
                        scale[slot] = number;
                    }
                }
            }
        }
        meta.insert((sample, name.to_owned()), value.to_owned());
        rest = &rest[end_value + 1..];
    }
    GdalFields {
        meta,
        scale,
        offset,
    }
}

fn attr<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

pub(super) fn parse_nodata(text: &str) -> Option<f32> {
    let text = text.trim_matches(|ch: char| ch == '\0' || ch.is_whitespace());
    let value = text.parse::<f64>().ok()?;
    Some(f32_from_f64(value))
}
