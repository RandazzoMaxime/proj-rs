//! Import of `+towgs84`, `+nadgrids`, `+geoidgrids`, `+vunits` and `+vto_meter`
//! from a PROJ string, as `PROJStringParser::createFromPROJString` does:
//! `buildBoundOrCompoundCRSIfNeeded`, the vertical unit of `buildUnit`, and the
//! vertical CRS a string made of `+vunits` or `+vto_meter` alone describes.

use super::factory::DatabaseContext;
use super::model::{
    Axis, BoundCRS, CompoundCRS, CoordinateOperation, Crs, Ellipsoid, FactoryError, GeodeticCRS,
    GeodeticReferenceFrame, GeographicCRS, Identified, OperationMethod, Parameter, ParameterValue,
    PrimeMeridian, Transformation, UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS,
    VerticalReferenceFrame,
};
use super::niche::DerivedKind;
use super::projstring::{geographic_axis, meridian_of, token};
use super::wkt_import::helmert_parameters;

type Tokens<'a> = &'a [(String, String)];

/// One row of PROJ's `linearUnitDescs`: the `+units` name, the factor to the
/// metre, the unit name and its EPSG code (0 when there is none).
const LINEAR_UNITS: &[(&str, f64, &str, u32)] = &[
    ("mm", 0.001, "millimetre", 1025),
    ("cm", 0.01, "centimetre", 1033),
    ("m", 1.0, "metre", 9001),
    ("meter", 1.0, "metre", 9001),
    ("metre", 1.0, "metre", 9001),
    ("ft", 0.3048, "foot", 9002),
    ("us-ft", 0.304_800_609_601_219_2, "US survey foot", 9003),
    ("fath", 1.8288, "fathom", 9014),
    ("kmi", 1852.0, "nautical mile", 9030),
    ("us-ch", 20.116_840_233_680_47, "US survey chain", 9033),
    ("us-mi", 1_609.347_218_694_437, "US survey mile", 9035),
    ("km", 1000.0, "kilometre", 9036),
    ("ind-ft", 0.304_798_41, "Indian foot (1937)", 9081),
    ("ind-yd", 0.914_395_23, "Indian yard (1937)", 9085),
    ("mi", 1609.344, "Statute mile", 9093),
    ("yd", 0.9144, "yard", 9096),
    ("ch", 20.1168, "chain", 9097),
    ("link", 0.201_168, "link", 9098),
    ("dm", 0.1, "decimetre", 0),
    ("in", 0.0254, "inch", 0),
    ("us-in", 0.025_400_050_800_101, "US survey inch", 0),
    ("us-yd", 0.914_401_828_803_658, "US survey yard", 0),
    ("ind-ch", 20.116_695_06, "Indian chain", 0),
];

fn linear_unit(row: &(&str, f64, &str, u32)) -> UnitOfMeasure {
    let (_, factor, name, code) = *row;
    let mut unit = UnitOfMeasure::new(name, factor, UnitKind::Linear);
    if code != 0 {
        unit.code_space = "EPSG".to_owned();
        unit.code = code.to_string();
    }
    unit
}

/// `PROJStringParser::Private::buildUnit`: the linear unit named by `units_key`,
/// or given as a factor by `to_meter_key`, which wins. The metre by default.
///
/// # Errors
///
/// An unknown unit name, a factor that is not a number, or a zero factor.
pub(super) fn build_unit(
    tokens: Tokens<'_>,
    units_key: &str,
    to_meter_key: &str,
) -> Result<UnitOfMeasure, FactoryError> {
    let mut unit = UnitOfMeasure::metre();
    let name = token(tokens, units_key);
    if !name.is_empty() {
        let row = LINEAR_UNITS
            .iter()
            .find(|row| row.0 == name)
            .ok_or_else(|| FactoryError::Invalid(format!("unhandled {units_key}={name}")))?;
        unit = linear_unit(row);
    }
    let text = token(tokens, to_meter_key);
    if !text.is_empty() {
        let factor: f64 = text
            .parse()
            .map_err(|_| FactoryError::Invalid(format!("invalid value for {to_meter_key}")))?;
        match LINEAR_UNITS
            .iter()
            .find(|row| (row.1 - factor).abs() < 1e-10 * factor)
        {
            Some(row) => unit = linear_unit(row),
            None if factor == 0.0 => {
                return Err(FactoryError::Invalid("invalid unit value".to_owned()))
            }
            None => unit = UnitOfMeasure::new("unknown", factor, UnitKind::Linear),
        }
    }
    Ok(unit)
}

fn has_vertical_unit(tokens: Tokens<'_>) -> bool {
    !token(tokens, "vunits").is_empty() || !token(tokens, "vto_meter").is_empty()
}

fn gravity_related_height(unit: UnitOfMeasure) -> VerticalCS {
    VerticalCS {
        identified: Identified::default(),
        axis: Axis::new("Gravity-related height", "H", "up", unit),
    }
}

fn unknown_vertical(datum_name: &str, unit: UnitOfMeasure) -> VerticalCRS {
    VerticalCRS {
        identified: Identified::named("unknown"),
        datum: Some(VerticalReferenceFrame::named(datum_name)),
        ensemble: None,
        coordinate_system: gravity_related_height(unit),
        geoid_models: Vec::new(),
    }
}

/// The vertical CRS of a string with no `+proj` and a `+vunits` or `+vto_meter`.
/// `None` when the string has neither.
///
/// # Errors
///
/// The unit is unknown.
pub(super) fn vertical_from_units(tokens: Tokens<'_>) -> Result<Option<Crs>, FactoryError> {
    if !token(tokens, "proj").is_empty() || !has_vertical_unit(tokens) {
        return Ok(None);
    }
    let unit = build_unit(tokens, "vunits", "vto_meter")?;
    Ok(Some(Crs::Vertical(unknown_vertical("unknown", unit))))
}

/// The third axis of a geographic CRS with a vertical unit and no `+geoidgrids`:
/// `buildEllipsoidalCS` adds an ellipsoidal height in that unit.
///
/// # Errors
///
/// The unit is unknown.
pub(super) fn ellipsoidal_height_axis(tokens: Tokens<'_>) -> Result<Option<Axis>, FactoryError> {
    if !token(tokens, "geoidgrids").is_empty() || !has_vertical_unit(tokens) {
        return Ok(None);
    }
    let unit = build_unit(tokens, "vunits", "vto_meter")?;
    Ok(Some(Axis::new("Ellipsoidal height", "h", "up", unit)))
}

/// `buildBoundOrCompoundCRSIfNeeded`: `+nadgrids` (which has priority) or
/// `+towgs84` bind `crs` to WGS 84, then `+geoidgrids` makes a compound CRS
/// with a vertical CRS bound to a geographic 3D CRS.
///
/// `db` enables the sign repair of seven `+towgs84` terms.
///
/// # Errors
///
/// A `+towgs84` value that is not a number or has neither 3 nor 7 terms, an
/// unknown vertical unit or `+geoid_crs`, or a CRS that has no geodetic part.
pub(super) fn bind_if_needed(
    tokens: Tokens<'_>,
    crs: Crs,
    db: Option<&DatabaseContext>,
) -> Result<Crs, FactoryError> {
    let nadgrids = token(tokens, "nadgrids");
    let towgs84 = token(tokens, "towgs84");
    let mut crs = crs;
    if !nadgrids.is_empty() && !ignores_nadgrids(tokens) {
        crs = bind_nadgrids(crs, nadgrids);
    } else if !towgs84.is_empty() {
        crs = bind_towgs84(crs, &parse_towgs84(towgs84, db)?)?;
    }
    let geoidgrids = token(tokens, "geoidgrids");
    if geoidgrids.is_empty() {
        return Ok(crs);
    }
    let unit = build_unit(tokens, "vunits", "vto_meter")?;
    let bound = bind_geoid(&crs, tokens, geoidgrids, unit)?;
    Ok(Crs::Compound(CompoundCRS::create(
        "unknown",
        vec![crs, bound],
    )?))
}

/// `+proj=merc +a=R +b=R +k=1 +nadgrids=@null` is the pseudo-Mercator of the
/// web, where PROJ drops the null grid (`ignoreNadgrids_`).
fn ignores_nadgrids(tokens: Tokens<'_>) -> bool {
    let number = |key: &str| token(tokens, key).parse::<f64>().ok();
    let k = tokens
        .iter()
        .find(|(key, _)| key == "k" || key == "k_0")
        .and_then(|(_, value)| value.parse::<f64>().ok());
    token(tokens, "proj") == "merc"
        && token(tokens, "nadgrids") == "@null"
        && matches!((number("a"), number("b")), (Some(a), Some(b)) if a == b)
        && number("lat_ts").is_none_or(|lat_ts| lat_ts == 0.0)
        && k == Some(1.0)
}

/// The `+towgs84` terms. Seven terms are repaired against the EPSG Helmert
/// rows when a database is attached (a coordinate frame rotation written with
/// the position vector sign).
fn parse_towgs84(text: &str, db: Option<&DatabaseContext>) -> Result<Vec<f64>, FactoryError> {
    let mut values = Vec::new();
    for part in text.split(',') {
        values.push(part.trim().parse::<f64>().map_err(|_| {
            FactoryError::Invalid("Non numerical value in towgs84 clause".to_owned())
        })?);
    }
    if let (Some(db), Ok(mut seven)) = (db, <[f64; 7]>::try_from(values.as_slice())) {
        if db.to_wgs84_autocorrect_wrong_values(&mut seven)? {
            values = seven.to_vec();
        }
    }
    Ok(values)
}

/// `CRS::extractGeodeticCRS` for the kinds a PROJ string imports.
fn extract_geodetic(crs: &Crs) -> Option<GeodeticCRS> {
    match crs {
        Crs::Geographic(geographic) => Some(GeodeticCRS::Geographic(geographic.clone())),
        Crs::Geodetic(geodetic) => Some(geodetic.clone()),
        Crs::Projected(projected) => Some(projected.base_crs().clone()),
        Crs::Bound(bound) => extract_geodetic(bound.source_crs()),
        Crs::Compound(compound) => compound.components().iter().find_map(extract_geodetic),
        _ => None,
    }
}

fn geodetic_as_crs(geodetic: GeodeticCRS) -> Crs {
    match geodetic {
        GeodeticCRS::Geographic(geographic) => Crs::Geographic(geographic),
        other => Crs::Geodetic(other),
    }
}

/// `createFromTOWGS84` for a `DerivedGeographicCRS`, which is itself a geodetic CRS: the
/// transformation starts from the derived CRS.
fn bind_towgs84_derived_geographic(
    crs: &Crs,
    axes: usize,
    values: &[f64],
) -> Result<Crs, FactoryError> {
    let target = Crs::Geographic(GeographicCRS::epsg_4326());
    let three_d = axes != 2;
    let (name, code) = match (values.len() == 3, three_d) {
        (true, false) => ("Geocentric translations (geog2D domain)", "9603"),
        (true, true) => ("Geocentric translations (geog3D domain)", "1035"),
        (false, false) => ("Position Vector transformation (geog2D domain)", "9606"),
        (false, true) => ("Position Vector transformation (geog3D domain)", "1037"),
    };
    let transformation = Transformation {
        identified: Identified::named(format!("Transformation from {} to WGS84", crs.name())),
        method: epsg_method(name, code),
        parameters: helmert_parameters(values),
        source: Some(Box::new(crs.clone())),
        target: Some(Box::new(target.clone())),
        ..plain_transformation()
    };
    Ok(Crs::Bound(BoundCRS::create(
        crs.clone(),
        target,
        CoordinateOperation::Transformation(transformation),
    )))
}

/// `Transformation::createTOWGS84` and `BoundCRS::createFromTOWGS84`: a
/// geocentric translations transformation for three terms, a position vector
/// one for seven, from the geodetic CRS of `crs` to WGS 84.
pub(crate) fn bind_towgs84(crs: Crs, values: &[f64]) -> Result<Crs, FactoryError> {
    if values.len() != 3 && values.len() != 7 {
        return Err(FactoryError::Invalid(
            "Invalid number of elements in TOWGS84Parameters".to_owned(),
        ));
    }
    if let Crs::Derived(derived) = &crs {
        if derived.kind() == DerivedKind::Geographic {
            return bind_towgs84_derived_geographic(
                &crs,
                derived.coordinate_system().axes().len(),
                values,
            );
        }
    }
    let geodetic = extract_geodetic(&crs).ok_or_else(|| {
        FactoryError::Invalid(
            "Cannot find GeodeticCRS in sourceCRS of TOWGS84 transformation".to_owned(),
        )
    })?;
    let geocentric = matches!(geodetic, GeodeticCRS::Geocentric(_));
    let target = if geocentric {
        Crs::Geodetic(GeodeticCRS::epsg_4978())
    } else {
        Crs::Geographic(GeographicCRS::epsg_4326())
    };
    let three_d = !geocentric && geodetic.coordinate_system().axes().len() != 2;
    let (name, code) = match (values.len() == 3, geocentric, three_d) {
        (true, true, _) => ("Geocentric translations (geocentric domain)", "1031"),
        (true, false, false) => ("Geocentric translations (geog2D domain)", "9603"),
        (true, false, true) => ("Geocentric translations (geog3D domain)", "1035"),
        (false, true, _) => ("Position Vector transformation (geocentric domain)", "1033"),
        (false, false, false) => ("Position Vector transformation (geog2D domain)", "9606"),
        (false, false, true) => ("Position Vector transformation (geog3D domain)", "1037"),
    };
    let source = geodetic_as_crs(geodetic);
    let transformation = Transformation {
        identified: Identified::named(format!("Transformation from {} to WGS84", source.name())),
        method: epsg_method(name, code),
        parameters: helmert_parameters(values),
        source: Some(Box::new(source)),
        target: Some(Box::new(target.clone())),
        ..plain_transformation()
    };
    Ok(Crs::Bound(BoundCRS::create(
        crs,
        target,
        CoordinateOperation::Transformation(transformation),
    )))
}

/// `BoundCRS::createFromNadgrids`: an NTv2 transformation from the geographic
/// CRS of `crs` (on a Greenwich meridian) to WGS 84.
pub(super) fn bind_nadgrids(crs: Crs, filename: &str) -> Crs {
    let source = transformation_source_of(&crs);
    let target = Crs::Geographic(GeographicCRS::epsg_4326());
    let transformation = Transformation {
        identified: Identified::named(format!("{} to WGS84", source.name())),
        method: epsg_method("NTv2", "9615"),
        parameters: vec![Parameter {
            name: "Latitude and longitude difference file".to_owned(),
            code_space: "EPSG".to_owned(),
            code: "8656".to_owned(),
            value: ParameterValue::filename(filename),
        }],
        source: Some(Box::new(source)),
        target: Some(Box::new(target.clone())),
        ..plain_transformation()
    };
    Crs::Bound(BoundCRS::create(
        crs,
        target,
        CoordinateOperation::Transformation(transformation),
    ))
}

/// The geographic CRS of `crs`, or `None` for a geocentric or vertical one.
fn geographic_of(crs: &Crs) -> Option<GeographicCRS> {
    match extract_geodetic(crs)? {
        GeodeticCRS::Geographic(geographic) => Some(geographic),
        _ => None,
    }
}

/// `createBoundCRSSourceTransformationCRS` (crs.cpp:5989-6010): the source CRS of the binding
/// transformation of a bound CRS whose base is `base`. The geographic CRS of the base, on a
/// Greenwich meridian (a copy named `(with Greenwich prime meridian)` when its meridian is not
/// Greenwich's); a base with no geographic CRS is its own source.
pub(super) fn transformation_source_of(base: &Crs) -> Crs {
    match geographic_of(base) {
        Some(geographic) => Crs::Geographic(greenwich_based(&geographic)),
        None => base.clone(),
    }
}

/// `crs` itself on a Greenwich meridian, or else a latitude/longitude CRS named
/// `<name> (with Greenwich prime meridian)` on the same ellipsoid.
fn greenwich_based(geographic: &GeographicCRS) -> GeographicCRS {
    let meridian = meridian_of(geographic);
    if meridian.is_none_or(|meridian| meridian.longitude().si_value() == 0.0) {
        return geographic.clone();
    }
    greenwich_copy(
        geographic,
        &format!("{} (with Greenwich prime meridian)", geographic.name()),
        EllipsoidalCsKind::LatitudeLongitude,
    )
}

enum EllipsoidalCsKind {
    LatitudeLongitude,
    LongitudeLatitudeHeight,
}

fn greenwich_copy(
    geographic: &GeographicCRS,
    name: &str,
    kind: EllipsoidalCsKind,
) -> GeographicCRS {
    let datum = geographic.datum();
    let datum_name = datum.map_or("unknown", GeodeticReferenceFrame::name);
    let ellipsoid = geographic
        .ellipsoid()
        .cloned()
        .unwrap_or_else(Ellipsoid::wgs84);
    let frame = GeodeticReferenceFrame::create(
        format!("{datum_name} (with Greenwich prime meridian)"),
        ellipsoid,
        PrimeMeridian::greenwich(),
    );
    let axes = match kind {
        EllipsoidalCsKind::LatitudeLongitude => {
            super::model::EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree())
                .axes()
                .to_vec()
        }
        EllipsoidalCsKind::LongitudeLatitudeHeight => vec![
            axis_or_default(geographic_axis('e', UnitOfMeasure::degree())),
            axis_or_default(geographic_axis('n', UnitOfMeasure::degree())),
            Axis::new("Ellipsoidal height", "h", "up", UnitOfMeasure::metre()),
        ],
    };
    GeographicCRS {
        identified: Identified::named(name),
        datum: Some(frame),
        ensemble: None,
        coordinate_system: super::model::EllipsoidalCS {
            identified: Identified::default(),
            axes,
        },
        implicit_cs: false,
    }
}

fn axis_or_default(axis: Result<Axis, FactoryError>) -> Axis {
    axis.unwrap_or_else(|_| Axis::new("Longitude", "lon", "east", UnitOfMeasure::degree()))
}

/// The vertical CRS of `+geoidgrids`, bound to the geographic 3D CRS `+geoid_crs`
/// names (WGS 84 by default) through a gravity-related height transformation.
fn bind_geoid(
    horizontal: &Crs,
    tokens: Tokens<'_>,
    filename: &str,
    unit: UnitOfMeasure,
) -> Result<Crs, FactoryError> {
    let datum_name = format!("unknown using geoidgrids={filename}");
    let vertical = Crs::Vertical(unknown_vertical(&datum_name, unit));
    let hub = Crs::Geographic(geoid_hub(horizontal, token(tokens, "geoid_crs"))?);
    let metre = Crs::Vertical(unknown_vertical(&datum_name, UnitOfMeasure::metre()));
    let transformation = Transformation {
        identified: Identified::named(format!("unknown to {} ellipsoidal height", hub.name())),
        method: OperationMethod::create("GravityRelatedHeight to Geographic3D", Vec::new()),
        parameters: vec![Parameter {
            name: "Geoid (height correction) model file".to_owned(),
            code_space: "EPSG".to_owned(),
            code: "8666".to_owned(),
            value: ParameterValue::filename(filename),
        }],
        source: Some(Box::new(metre)),
        target: Some(Box::new(hub.clone())),
        ..plain_transformation()
    };
    Ok(Crs::Bound(BoundCRS::create(
        vertical,
        hub,
        CoordinateOperation::Transformation(transformation),
    )))
}

/// `+geoid_crs`: nothing or `WGS84` is EPSG:4979. `horizontal_crs` is the
/// geographic CRS of the horizontal part in 3D, on a Greenwich meridian when it
/// is not already one in degrees.
fn geoid_hub(horizontal: &Crs, geoid_crs: &str) -> Result<GeographicCRS, FactoryError> {
    match geoid_crs {
        "" | "WGS84" => Ok(GeographicCRS::epsg_4979()),
        "horizontal_crs" => {
            let Some(geographic) = geographic_of(horizontal) else {
                return Ok(GeographicCRS::epsg_4979());
            };
            let greenwich = meridian_of(&geographic)
                .is_none_or(|meridian| meridian.longitude().si_value() == 0.0);
            let degrees = geographic
                .coordinate_system()
                .axes()
                .first()
                .is_some_and(|axis| axis.unit().same_name(&UnitOfMeasure::degree()));
            if greenwich && degrees {
                Ok(geographic.promote_to_3d())
            } else {
                Ok(greenwich_copy(
                    &geographic,
                    "unknown",
                    EllipsoidalCsKind::LongitudeLatitudeHeight,
                ))
            }
        }
        _ => Err(FactoryError::Invalid(
            "Unsupported value for geoid_crs: should be 'WGS84' or 'horizontal_crs'".to_owned(),
        )),
    }
}

fn epsg_method(name: &str, code: &str) -> OperationMethod {
    OperationMethod {
        name: name.to_owned(),
        code_space: "EPSG".to_owned(),
        code: code.to_owned(),
        parameters: Vec::new(),
    }
}

/// A transformation with every optional part empty; callers set the rest.
fn plain_transformation() -> Transformation {
    Transformation {
        identified: Identified::default(),
        method: OperationMethod::create("", Vec::new()),
        parameters: Vec::new(),
        source: None,
        target: None,
        interpolation: None,
        accuracy: None,
        accuracy_text: None,
        version: None,
        point_motion: false,
        ballpark: false,
        source_epoch: None,
        target_epoch: None,
        inverted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso19111::projstring::create_crs_from_proj_string;

    fn text(definition: &str) -> String {
        create_crs_from_proj_string(definition)
            .expect("import")
            .export_to_proj_string()
            .expect("export")
    }

    #[test]
    fn units_come_from_the_table_or_the_factor() {
        let tokens = [("vunits".to_owned(), "us-ft".to_owned())];
        let unit = build_unit(&tokens, "vunits", "vto_meter").expect("unit");
        assert_eq!(unit.name(), "US survey foot");
        let tokens = [("vto_meter".to_owned(), "0.3048".to_owned())];
        let unit = build_unit(&tokens, "vunits", "vto_meter").expect("unit");
        assert_eq!(unit.name(), "foot");
        let tokens = [("vunits".to_owned(), "furlong".to_owned())];
        assert!(build_unit(&tokens, "vunits", "vto_meter").is_err());
        let tokens = [("vto_meter".to_owned(), "0".to_owned())];
        assert!(build_unit(&tokens, "vunits", "vto_meter").is_err());
    }

    #[test]
    fn towgs84_needs_three_or_seven_numbers() {
        assert!(
            create_crs_from_proj_string("+proj=longlat +ellps=GRS80 +towgs84=1,2 +type=crs")
                .is_err()
        );
        assert!(
            create_crs_from_proj_string("+proj=longlat +ellps=GRS80 +towgs84=1,x,3 +type=crs")
                .is_err()
        );
    }

    #[test]
    fn nadgrids_wins_over_towgs84_and_projected_crs_keep_their_shift() {
        assert_eq!(
            text("+proj=longlat +ellps=GRS80 +towgs84=1,2,3 +nadgrids=foo.gsb +type=crs"),
            "+proj=longlat +ellps=GRS80 +nadgrids=foo.gsb +no_defs +type=crs"
        );
        let projected = text(
            "+proj=tmerc +lat_0=0 +lon_0=9 +k=0.9996 +x_0=500000 +y_0=0 +ellps=intl \
             +towgs84=-87,-98,-121 +units=m +type=crs",
        );
        assert!(
            projected.contains("+ellps=intl +towgs84=-87,-98,-121,0,0,0,0 +units=m"),
            "{projected}"
        );
    }

    #[test]
    fn geoid_crs_horizontal_crs_round_trips() {
        let definition =
            "+proj=longlat +ellps=GRS80 +geoidgrids=foo.gtx +geoid_crs=horizontal_crs \
                          +vunits=m +no_defs +type=crs";
        assert_eq!(text(definition), definition);
        assert!(create_crs_from_proj_string(
            "+proj=longlat +ellps=GRS80 +geoidgrids=foo.gtx +geoid_crs=bar +type=crs"
        )
        .is_err());
    }

    #[test]
    fn pseudo_mercator_null_grid_is_dropped() {
        let crs = create_crs_from_proj_string(
            "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1 \
             +units=m +nadgrids=@null +no_defs +type=crs",
        )
        .expect("import");
        assert!(matches!(crs, Crs::Projected(_)));
    }

    #[test]
    fn coordinate_system_of_a_vertical_unit_geographic_crs_has_three_axes() {
        let crs = create_crs_from_proj_string("+proj=longlat +ellps=GRS80 +vunits=ft +type=crs")
            .expect("import");
        let Crs::Geographic(geographic) = crs else {
            panic!("not geographic");
        };
        assert_eq!(geographic.coordinate_system().axes().len(), 3);
    }
}
