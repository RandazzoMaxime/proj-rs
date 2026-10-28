//! ESRI WKT1 method mappings from PROJ `esriparammappings.cpp`.
//!
//! Several ESRI method names map to more than one WKT2 method. The parser
//! keeps the candidate whose fixed parameter values match the WKT.

/// One ESRI parameter, or a fixed value that is not an EPSG parameter.
pub(crate) struct EsriParam {
    pub(crate) esri_name: &'static str,
    /// WKT2 name. Absent when the parameter is an ESRI-only fixed value.
    pub(crate) wkt2_name: Option<&'static str>,
    pub(crate) epsg_code: i32,
    pub(crate) fixed_value: &'static str,
    /// When the WKT omits this parameter, use `fixed_value` instead of scoring a miss.
    pub(crate) is_fixed_value: bool,
}

/// One ESRI projection method and its parameter list.
pub(crate) struct EsriMethod {
    pub(crate) esri_name: &'static str,
    pub(crate) wkt2_name: &'static str,
    pub(crate) epsg_code: i32,
    pub(crate) params: &'static [EsriParam],
}

const KROVAK_ALT1: &[EsriParam] = &[
    EsriParam { esri_name: "False_Easting", wkt2_name: Some("False easting"), epsg_code: 8806, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "False_Northing", wkt2_name: Some("False northing"), epsg_code: 8807, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Pseudo_Standard_Parallel_1", wkt2_name: Some("Latitude of pseudo standard parallel"), epsg_code: 8818, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Scale_Factor", wkt2_name: Some("Scale factor on pseudo standard parallel"), epsg_code: 8819, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Azimuth", wkt2_name: Some("Co-latitude of cone axis"), epsg_code: 1036, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Longitude_Of_Center", wkt2_name: Some("Longitude of origin"), epsg_code: 8833, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Latitude_Of_Center", wkt2_name: Some("Latitude of projection centre"), epsg_code: 8811, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "X_Scale", wkt2_name: None, epsg_code: 0, fixed_value: "1.0", is_fixed_value: false },
    EsriParam { esri_name: "Y_Scale", wkt2_name: None, epsg_code: 0, fixed_value: "1.0", is_fixed_value: false },
    EsriParam { esri_name: "XY_Plane_Rotation", wkt2_name: None, epsg_code: 0, fixed_value: "0.0", is_fixed_value: false },
];

const KROVAK_ALT2: &[EsriParam] = &[
    EsriParam { esri_name: "False_Easting", wkt2_name: Some("False easting"), epsg_code: 8806, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "False_Northing", wkt2_name: Some("False northing"), epsg_code: 8807, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Pseudo_Standard_Parallel_1", wkt2_name: Some("Latitude of pseudo standard parallel"), epsg_code: 8818, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Scale_Factor", wkt2_name: Some("Scale factor on pseudo standard parallel"), epsg_code: 8819, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Azimuth", wkt2_name: Some("Co-latitude of cone axis"), epsg_code: 1036, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Longitude_Of_Center", wkt2_name: Some("Longitude of origin"), epsg_code: 8833, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "Latitude_Of_Center", wkt2_name: Some("Latitude of projection centre"), epsg_code: 8811, fixed_value: "0.0", is_fixed_value: false },
    EsriParam { esri_name: "X_Scale", wkt2_name: None, epsg_code: 0, fixed_value: "-1.0", is_fixed_value: false },
    EsriParam { esri_name: "Y_Scale", wkt2_name: None, epsg_code: 0, fixed_value: "1.0", is_fixed_value: false },
    EsriParam { esri_name: "XY_Plane_Rotation", wkt2_name: None, epsg_code: 0, fixed_value: "90.0", is_fixed_value: false },
];

static ESRI_METHODS: &[EsriMethod] = &[
    EsriMethod {
        esri_name: "Krovak",
        wkt2_name: "Krovak",
        epsg_code: 9819,
        params: KROVAK_ALT1,
    },
    EsriMethod {
        esri_name: "Krovak",
        wkt2_name: "Krovak (North Orientated)",
        epsg_code: 1041,
        params: KROVAK_ALT2,
    },
];

/// Whether the WKT is already known to be ESRI, or only guessed.
pub(crate) enum EsriCertainty {
    /// `esriStyle_` is set.
    Known,
    /// Only `maybeEsriStyle_` is set.
    Maybe,
}

/// First mapping whose EPSG code or WKT2 name matches, in table order.
pub(crate) fn esri_mapping_for_method(wkt2_name: &str, epsg_code: i32) -> Option<&'static EsriMethod> {
    ESRI_METHODS.iter().find(|mapping| {
        (epsg_code != 0 && mapping.epsg_code == epsg_code) || ci_eq(wkt2_name, mapping.wkt2_name)
    })
}

/// Mapping for `projection` whose fixed values agree with `values`.
pub(crate) fn select_esri_mapping(
    projection: &str,
    values: &[(String, f64)],
    certainty: EsriCertainty,
) -> Option<&'static EsriMethod> {
    let mut chosen: Option<&EsriMethod> = None;
    let mut best_match = -1;
    for mapping in ESRI_METHODS.iter().filter(|mapping| ci_eq(mapping.esri_name, projection)) {
        let mut match_count = 0;
        let mut unmatch_count = 0;
        let mut rejected = false;
        for param in mapping.params {
            if let Some(value) = find_value(values, param.esri_name) {
                if param.wkt2_name.is_none() {
                    match param.fixed_value.parse::<f64>() {
                        Ok(fixed) if fixed == value => match_count += 1,
                        _ => {
                            rejected = true;
                            break;
                        }
                    }
                } else {
                    match_count += 1;
                }
            } else if !param.is_fixed_value {
                unmatch_count += 1;
            }
        }
        if rejected {
            continue;
        }
        let maybe_too_sparse = matches!(certainty, EsriCertainty::Maybe) && unmatch_count >= match_count;
        if match_count > best_match && !maybe_too_sparse {
            chosen = Some(mapping);
            best_match = match_count;
        }
    }
    chosen
}

fn find_value(values: &[(String, f64)], name: &str) -> Option<f64> {
    values
        .iter()
        .find(|(key, _)| ci_eq(key, name))
        .map(|(_, value)| *value)
}

fn ci_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}
