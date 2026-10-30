//! `BoundCRS` to WGS 84, as `projinfo` builds it: a CRS bound to WGS 84 through
//! the single usable transformation the database holds for it, and the PROJ
//! parameters such a bound CRS carries (`+towgs84`, `+geoidgrids`).

use std::f64::consts::PI;

use super::equiv::Equivalence;
use super::factory::{CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext};
use super::model::{
    BoundCRS, CoordinateOperation, Crs, FactoryError, GeodeticCRS, GeographicCRS, Parameter,
    ParameterValue, Transformation,
};

/// EPSG method codes that `Transformation::isGeographic3DToGravityRelatedHeight` accepts.
const GRAVITY_RELATED_HEIGHT_METHODS: [i32; 42] = [
    1025, 1030, 1045, 1047, 1048, 1050, 1059, 1088, 1060, 1072, 1073, 1081, 1083, 1089, 1090, 1091,
    1092, 1093, 1094, 1095, 1096, 1097, 1098, 1100, 1103, 1105, 1109, 1110, 1115, 1118, 1122, 1124,
    1126, 1127, 1128, 1135, 9661, 9662, 9663, 9664, 9665, 9635,
];

/// The EPSG code of the geoid correction file parameter.
const GEOID_CORRECTION_FILENAME: i32 = 8666;
const GEOID_CORRECTION_FILENAME_NAME: &str = "Geoid (height correction) model file";

/// `Helmert` method family, as `Transformation::getTOWGS84Parameters` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Helmert {
    /// Seven parameters, with the rotation signs of a coordinate frame method.
    CoordinateFrame,
    /// Seven parameters, position vector convention.
    PositionVector,
    /// Three geocentric translations.
    Translation,
}

fn helmert_kind(transformation: &Transformation) -> Option<Helmert> {
    let method = transformation.method();
    let name = method.name();
    let count = transformation.parameters().len();
    let code = epsg_method_code(transformation);
    if (count == 7 && contains_ci(name, "Coordinate Frame"))
        || matches!(code, 1032 | 1132 | 9607 | 1133 | 1140 | 1038 | 1149)
    {
        Some(Helmert::CoordinateFrame)
    } else if (count == 7 && contains_ci(name, "Position Vector"))
        || matches!(code, 1033 | 9606 | 1037)
    {
        Some(Helmert::PositionVector)
    } else if (count == 3 && contains_ci(name, "Geocentric translations"))
        || matches!(code, 1031 | 9603 | 1035)
    {
        Some(Helmert::Translation)
    } else {
        None
    }
}

fn contains_ci(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn starts_with_ci(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// The EPSG method code, or 0 when the method has no EPSG identifier.
fn epsg_method_code(transformation: &Transformation) -> i32 {
    let method = transformation.method();
    if method.code_space() == "EPSG" {
        method.code().parse().unwrap_or(0)
    } else {
        0
    }
}

/// `Transformation::isGeographic3DToGravityRelatedHeight` with `allowInverse` false.
fn is_gravity_related_height(transformation: &Transformation) -> bool {
    let method = transformation.method();
    if starts_with_ci(method.name(), "Geographic3D to GravityRelatedHeight") {
        return true;
    }
    method.code_space() == "EPSG"
        && method
            .code()
            .parse::<i32>()
            .is_ok_and(|code| GRAVITY_RELATED_HEIGHT_METHODS.contains(&code))
}

fn geoid_file_parameter(transformation: &Transformation) -> Option<&str> {
    transformation.parameters().iter().find_map(|parameter| {
        let is_file = parameter.code() == GEOID_CORRECTION_FILENAME.to_string()
            || parameter.name() == GEOID_CORRECTION_FILENAME_NAME;
        match parameter.value() {
            ParameterValue::Filename(name) if is_file => Some(name.as_str()),
            _ => None,
        }
    })
}

/// The index, in the seven `+towgs84` values, of a Helmert parameter.
fn towgs84_slot(parameter: &Parameter) -> Option<usize> {
    match parameter.code().parse::<i32>().ok()? {
        8605 => Some(0),
        8606 => Some(1),
        8607 => Some(2),
        8608 => Some(3),
        8609 => Some(4),
        8610 => Some(5),
        8611 => Some(6),
        _ => None,
    }
}

impl Transformation {
    /// `getTOWGS84Parameters(false)`: the seven `+towgs84` values, or `None`.
    pub(crate) fn towgs84_lenient(&self) -> Option<[f64; 7]> {
        self.towgs84_strict().ok().flatten()
    }

    /// `getTOWGS84Parameters(true)`: the seven `+towgs84` values of a Helmert
    /// transformation. Other methods give `None`.
    ///
    /// # Errors
    ///
    /// A Helmert method with a missing parameter.
    pub(crate) fn towgs84_strict(&self) -> Result<Option<[f64; 7]>, FactoryError> {
        let Some(kind) = helmert_kind(self) else {
            return Ok(None);
        };
        let seven = kind != Helmert::Translation;
        let rotation_sign = if kind == Helmert::CoordinateFrame {
            -1.0
        } else {
            1.0
        };
        let mut params = [0.0_f64; 7];
        let mut found = [false; 7];
        for parameter in self.parameters() {
            let ParameterValue::Measure(measure) = parameter.value() else {
                continue;
            };
            let Some(slot) = towgs84_slot(parameter) else {
                continue;
            };
            if slot >= 3 && !seven {
                continue;
            }
            params[slot] = match slot {
                0..=2 => measure.si_value(),
                3..=5 => fix_negative_zero(rotation_sign * (measure.si_value() / (PI / 648_000.0))),
                _ => measure.si_value() / 1e-6,
            };
            found[slot] = true;
        }
        let translations = found[..3].iter().all(|value| *value);
        let complete = translations && (!seven || found[3..].iter().all(|value| *value));
        if complete {
            Ok(Some(params))
        } else {
            Err(FactoryError::Format(
                "Missing required parameter values in transformation".to_owned(),
            ))
        }
    }

    /// `getHeightToGeographic3DFilename`: the geoid grid of a height-to-geographic
    /// 3D transformation, when it names one.
    pub(crate) fn height_to_geographic_3d_filename(&self) -> Option<&str> {
        if !is_gravity_related_height(self) {
            return None;
        }
        geoid_file_parameter(self)
    }

    /// `substitutePROJAlternativeGridNames`: each grid file name is replaced by the
    /// name PROJ reads it under, when the database renames it.
    fn with_proj_grid_names(mut self, db: &DatabaseContext) -> Result<Self, FactoryError> {
        for parameter in &mut self.parameters {
            if let ParameterValue::Filename(name) = &parameter.value {
                if let Some(proj_name) = db.proj_grid_name(name)? {
                    parameter.value = ParameterValue::Filename(proj_name);
                }
            }
        }
        Ok(self)
    }

    /// Whether every grid the transformation needs is known to the database
    /// (`gridsNeeded(db, true)`): a package, or a download URL under an open
    /// licence, or a local file. A transformation that needs no grid is not known.
    fn grids_known(&self, db: &DatabaseContext) -> Result<bool, FactoryError> {
        let mut names = Vec::new();
        for parameter in self.parameters() {
            if let ParameterValue::Filename(text) = parameter.value() {
                names.extend(text.split(','));
            }
        }
        if names.is_empty() {
            return Ok(false);
        }
        for name in names {
            if !db.grid_is_known(name)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn fix_negative_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

/// `isTOWGS84Compatible`: a geodetic hub named `WGS 84`.
pub(crate) fn hub_is_wgs84(hub: &Crs) -> bool {
    matches!(hub, Crs::Geographic(_) | Crs::Geodetic(_))
        && hub.name().eq_ignore_ascii_case("WGS 84")
}

impl Crs {
    /// `CRS::createBoundCRSToWGS84IfPossible`: this CRS bound to WGS 84 through the
    /// one transformation the database holds for it, or a copy of `self` when there
    /// is none, or when there are several candidates.
    ///
    /// A vertical CRS is bound to EPSG:4979 through a transformation whose grids are
    /// known. A geographic or projected CRS is bound to EPSG:4326 through a Helmert
    /// transformation, when exactly one of them has `+towgs84` parameters. A bound
    /// CRS and a compound CRS are returned as they are.
    #[must_use]
    pub fn create_bound_crs_to_wgs84_if_possible(&self, db: &DatabaseContext) -> Crs {
        self.bound_to_wgs84(db)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.clone())
    }

    fn bound_to_wgs84(&self, db: &DatabaseContext) -> Result<Option<Crs>, FactoryError> {
        if matches!(self, Self::Bound(_) | Self::Compound(_)) {
            return Ok(None);
        }
        let authority = self
            .identifiers()
            .first()
            .map_or_else(String::new, |identifier| identifier.code_space().to_owned());
        if let Self::Vertical(_) = self {
            return vertical_bound(self, db, &authority);
        }
        horizontal_bound(self, db, &authority)
    }
}

fn vertical_bound(
    vertical: &Crs,
    db: &DatabaseContext,
    authority: &str,
) -> Result<Option<Crs>, FactoryError> {
    let hub = Crs::Geographic(GeographicCRS::epsg_4979());
    let operations = candidate_operations(db, &hub, vertical, authority)?;
    let mut candidate = None;
    for operation in operations {
        let CoordinateOperation::Transformation(transformation) = operation else {
            continue;
        };
        let transformation = transformation.with_proj_grid_names(db)?;
        if !transformation.grids_known(db)? {
            continue;
        }
        if candidate.is_some() {
            return Ok(None);
        }
        candidate = Some(transformation);
    }
    Ok(candidate.map(|transformation| bind(vertical, hub, transformation)))
}

fn horizontal_bound(
    crs: &Crs,
    db: &DatabaseContext,
    authority: &str,
) -> Result<Option<Crs>, FactoryError> {
    let Some(geographic) = extract_geographic(crs) else {
        return Ok(None);
    };
    let known_datum = geographic
        .datum()
        .is_some_and(|datum| datum.name() != "unknown");
    if known_datum
        && geographic.is_equivalent_to_with(&GeographicCRS::epsg_4326(), Equivalence::Equivalent)
    {
        return Ok(None);
    }
    let hub = Crs::Geographic(GeographicCRS::epsg_4326());
    let source = Crs::Geographic(geographic.clone());
    let operations = candidate_operations(db, &source, &hub, authority)?;
    let mut count = 0_usize;
    let mut candidate = None;
    for operation in operations {
        let CoordinateOperation::Transformation(transformation) = operation else {
            continue;
        };
        if transformation.name().starts_with("Ballpark geo") {
            continue;
        }
        if transformation.towgs84_lenient().is_none() {
            continue;
        }
        if candidate.is_none() {
            count = 0;
            candidate = Some(transformation);
        }
        count += 1;
    }
    Ok(match candidate {
        Some(transformation) if count == 1 => Some(bind(crs, hub, transformation)),
        _ => None,
    })
}

fn candidate_operations(
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
    authority: &str,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let context = CoordinateOperationContext::create(db, authority);
    CoordinateOperationFactory::create().create_operations(source, target, &context)
}

fn bind(source: &Crs, hub: Crs, transformation: Transformation) -> Crs {
    Crs::Bound(BoundCRS::create(
        source.clone(),
        hub,
        CoordinateOperation::Transformation(transformation),
    ))
}

/// `CRS::extractGeographicCRS` for the kinds this port binds.
fn extract_geographic(crs: &Crs) -> Option<&GeographicCRS> {
    match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            Some(geographic)
        }
        Crs::Projected(projected) => match projected.base_crs() {
            GeodeticCRS::Geographic(geographic) => Some(geographic),
            _ => None,
        },
        Crs::Bound(bound) => extract_geographic(bound.source_crs()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso19111::factory::AuthorityFactory;

    fn epsg_crs(db: &DatabaseContext, code: &str) -> Crs {
        AuthorityFactory::create(db, "EPSG")
            .create_coordinate_reference_system(code)
            .expect("EPSG CRS")
    }

    #[test]
    fn vertical_crs_binds_to_its_known_geoid_grid() {
        let db = DatabaseContext::create().expect("database");
        let bound = epsg_crs(&db, "3855").create_bound_crs_to_wgs84_if_possible(&db);
        assert!(matches!(bound, Crs::Bound(_)));
        assert_eq!(
            bound.export_to_proj_string().expect("PROJ string"),
            "+geoidgrids=us_nga_egm08_25.tif +geoid_crs=WGS84 +vunits=m +no_defs +type=crs"
        );
    }

    #[test]
    fn projected_crs_on_etrs89_carries_the_helmert_towgs84() {
        let db = DatabaseContext::create().expect("database");
        let bound = epsg_crs(&db, "25832").create_bound_crs_to_wgs84_if_possible(&db);
        let text = bound.export_to_proj_string().expect("PROJ string");
        assert!(
            text.contains("+ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m"),
            "{text}"
        );
    }

    #[test]
    fn wgs84_itself_is_not_bound() {
        let db = DatabaseContext::create().expect("database");
        let crs = epsg_crs(&db, "4326");
        assert!(matches!(
            crs.create_bound_crs_to_wgs84_if_possible(&db),
            Crs::Geographic(_) | Crs::Geodetic(_)
        ));
    }
}
