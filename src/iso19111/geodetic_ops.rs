//! `createOperationsGeodToGeod` of PROJ's `coordinateoperationfactory.cpp`: the operations
//! between two geodetic CRS that need no registry, such as the null or ballpark geographic
//! offset, the longitude rotation, the axis order change, and the geographic/geocentric
//! conversion.
//!
//! Not ported: the celestial body check, and the geodetic CRS that are neither geographic
//! nor geocentric.

use super::equiv::{geodetic_frame, same_geodetic_datum};
use super::factory::{
    crs_extent, pivot_concatenation, world_extent, CoordinateOperationContext,
    CoordinateOperationFactory,
};
use super::model::{
    Axis, Conversion, CoordinateOperation, Crs, EllipsoidalCS, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Measure, ObjectDomain, ParameterValue, Transformation, UnitOfMeasure,
};
use super::{FactoryError, OpProps};

const NULL_GEOGRAPHIC_OFFSET: &str = "Null geographic offset";
const BALLPARK_GEOGRAPHIC_OFFSET: &str = "Ballpark geographic offset";
const NULL_GEOCENTRIC_TRANSLATION: &str = "Null geocentric translation";
const BALLPARK_GEOCENTRIC_TRANSLATION: &str = "Ballpark geocentric translation";
const METHOD_LONGITUDE_ROTATION: i32 = 9601;
const METHOD_GEOGRAPHIC2D_OFFSETS: i32 = 9619;
const METHOD_GEOGRAPHIC3D_OFFSETS: i32 = 9660;
const METHOD_GEOCENTRIC_TRANSLATIONS: i32 = 1031;

/// Whether the datum of the two CRS is compared, or the operation is forced to be a ballpark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BallparkRule {
    /// The same datum gives a null operation, a different one a ballpark.
    ByDatum,
    /// The operation is a ballpark whatever the datums.
    Forced,
}

/// `getCRSQualifierStr`.
fn qualifier(crs: &Crs) -> &'static str {
    if matches!(crs, Crs::Geodetic(GeodeticCRS::Geocentric(_))) {
        return " (geocentric)";
    }
    match geographic_axes(crs) {
        Some(2) => " (geog2D)",
        Some(_) => " (geog3D)",
        None => "",
    }
}

fn geographic_axes(crs: &Crs) -> Option<usize> {
    super::factory::geographic_of(crs).map(|geographic| geographic.coordinate_system().axes().len())
}

/// Which object `createPropertiesForInverse` is naming.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NamedOperationKind {
    /// A transformation, including a null or ballpark offset.
    Transformation,
    /// A conversion. A name that joins steps with ` + ` is split.
    Conversion,
    /// A concatenated operation. Its computed name is rebuilt by the caller.
    Concatenated,
}

/// `createPropertiesForInverse`: the inverse name, swapping the CRS of a
/// `buildOpName` name and prefixing `Inverse of` otherwise.
pub(super) fn inverse_operation_name(
    name: &str,
    source: Option<&Crs>,
    target: Option<&Crs>,
    kind: NamedOperationKind,
) -> String {
    const INVERSE_OF: &str = "Inverse of ";
    // A geographic/geocentric conversion is stored as a transformation whose name
    // starts with `Conversion from`. PROJ types it as a conversion, so the inverse
    // name swaps that prefix rather than adding `Inverse of`.
    let op_type = if name.starts_with(BALLPARK_GEOCENTRIC_TRANSLATION) {
        BALLPARK_GEOCENTRIC_TRANSLATION
    } else if name.starts_with(BALLPARK_GEOGRAPHIC_OFFSET) {
        BALLPARK_GEOGRAPHIC_OFFSET
    } else if name.starts_with(NULL_GEOGRAPHIC_OFFSET) {
        NULL_GEOGRAPHIC_OFFSET
    } else if name.starts_with(NULL_GEOCENTRIC_TRANSLATION) {
        NULL_GEOCENTRIC_TRANSLATION
    } else if name.starts_with("Conversion from ") || kind == NamedOperationKind::Conversion {
        "Conversion"
    } else if kind == NamedOperationKind::Transformation || name.starts_with("Transformation from ")
    {
        "Transformation"
    } else {
        "Operation"
    };
    let mut result = String::new();
    if !name.is_empty() {
        let split = kind != NamedOperationKind::Transformation
            && kind != NamedOperationKind::Concatenated
            && (name.starts_with(INVERSE_OF) || name.contains(" + "));
        if split {
            result = inverse_joined_name(name);
        } else if let (Some(source), Some(target)) = (source, target) {
            if name != build_op_name(op_type, source, target) {
                result = prefixed_inverse_name(name);
            }
        } else {
            result = prefixed_inverse_name(name);
        }
    }
    if result.is_empty() {
        if let (Some(source), Some(target)) = (source, target) {
            result = build_op_name(op_type, target, source);
        }
    }
    result
}

/// `Inverse of`, or the forward name when it already is one. A name that joins
/// steps is quoted, as `createPropertiesForInverse` quotes it.
fn prefixed_inverse_name(name: &str) -> String {
    if name.contains(" + ") {
        format!("Inverse of '{name}'")
    } else if let Some(rest) = name.strip_prefix("Inverse of ") {
        let quoted = rest
            .strip_prefix('\'')
            .and_then(|inner| inner.strip_suffix('\''));
        quoted.unwrap_or(rest).to_owned()
    } else {
        format!("Inverse of {name}")
    }
}

/// The inverse of a conversion name that is itself a join of steps.
fn inverse_joined_name(name: &str) -> String {
    const INVERSE_OF: &str = "Inverse of ";
    const AXIS_2D: &str = "axis order change (2D)";
    const AXIS_3D: &str = "axis order change (geographic3D horizontal)";
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let chars: Vec<char> = name.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if in_string {
            current.push(chars[index]);
            if chars[index] == '\'' {
                in_string = false;
            }
        } else if index + 3 < chars.len()
            && chars[index] == ' '
            && chars[index + 1] == '+'
            && chars[index + 2] == ' '
        {
            tokens.push(std::mem::take(&mut current));
            index += 2;
        } else if chars[index] == '\'' {
            in_string = true;
            current.push(chars[index]);
        } else {
            current.push(chars[index]);
        }
        index += 1;
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    let mut result = String::new();
    for token in tokens.iter().rev() {
        if !result.is_empty() {
            result.push_str(" + ");
        }
        if let Some(rest) = token.strip_prefix(INVERSE_OF) {
            result.push_str(rest);
        } else if token == AXIS_2D || token == AXIS_3D {
            result.push_str(token);
        } else {
            result.push_str(INVERSE_OF);
            result.push_str(token);
        }
    }
    result
}

/// `buildOpName`: the CRS names, qualified by their kind when they are equal.
pub(super) fn build_op_name(op_type: &str, source: &Crs, target: &Crs) -> String {
    let (mut source_qualifier, mut target_qualifier) = ("", "");
    if source.name() == target.name() {
        source_qualifier = qualifier(source);
        target_qualifier = qualifier(target);
        if source_qualifier == target_qualifier {
            source_qualifier = "";
            target_qualifier = "";
        }
    }
    format!(
        "{op_type} from {}{source_qualifier} to {}{target_qualifier}",
        source.name(),
        target.name()
    )
}

fn transformation_name(source: &str, target: &str) -> String {
    format!("Transformation from {source} to {target}")
}

fn is_geocentric(crs: &Crs) -> bool {
    matches!(crs, Crs::Geodetic(GeodeticCRS::Geocentric(_)))
}

/// The operations from `source` to `target`, both geographic or geocentric.
///
/// # Errors
///
/// An operation cannot be built from the CRS.
pub(super) fn geodetic_operations(
    source: &Crs,
    target: &Crs,
    rule: BallparkRule,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let source_geographic = super::factory::geographic_of(source);
    let target_geographic = super::factory::geographic_of(target);
    if let (Some(left), Some(right)) = (source_geographic, target_geographic) {
        return geographic_to_geographic(source, target, left, right, rule);
    }
    let same_datum = rule == BallparkRule::ByDatum && same_geodetic_datum(source, target);
    match (
        is_geocentric(source),
        is_geocentric(target),
        source_geographic.is_some(),
        target_geographic.is_some(),
    ) {
        (true, false, _, true) | (false, true, true, _) => {
            if same_datum || rule == BallparkRule::Forced && same_geodetic_datum(source, target) {
                let operation = geographic_geocentric_conversion(source, target)?;
                return Ok(vec![CoordinateOperation::Transformation(operation)]);
            }
            geographic_geocentric_different_datum(source, target, rule)
        }
        (true, true, _, _) => Ok(vec![CoordinateOperation::Transformation(
            geocentric_translation(source, target, rule)?,
        )]),
        _ => Ok(Vec::new()),
    }
}

/// `Conversion::createGeographicGeocentric(source, target)`, as a transformation that keeps both
/// CRS so that the PROJ string reads the ellipsoid of the geographic side.
pub(super) fn geographic_geocentric_conversion(
    source: &Crs,
    target: &Crs,
) -> Result<Transformation, FactoryError> {
    let mut operation = Transformation::create_from_code(
        &OpProps::new().name(build_op_name("Conversion", source, target)),
        source.clone(),
        target.clone(),
        9602,
        Vec::new(),
        world_extent(),
    )?;
    operation.identified.domains.clear();
    operation.accuracy = Some(0.0);
    Ok(operation)
}

/// `isSphericalPlanetocentric`: a spherical CRS whose two axes are the planetocentric latitude
/// and longitude, in either order.
pub(super) fn is_spherical_planetocentric(crs: &Crs) -> bool {
    let Crs::Geodetic(GeodeticCRS::Spherical(spherical)) = crs else {
        return false;
    };
    matches!(
        spherical.coordinate_system.axes(),
        [first, second]
            if (first.name().eq_ignore_ascii_case("planetocentric latitude")
                && second.name().eq_ignore_ascii_case("planetocentric longitude"))
                || (first.name().eq_ignore_ascii_case("planetocentric longitude")
                    && second.name().eq_ignore_ascii_case("planetocentric latitude"))
    )
}

/// `createOperationsFromSphericalPlanetocentric`: from a spherical planetocentric source, the
/// geographic/geocentric latitude conversion to a geographic CRS of the same datum, then the
/// operations from that geographic CRS to the target. A geographic target of the same datum
/// takes the conversion alone.
///
/// Not ported: the domain of the intermediate CRS is not copied from the source.
///
/// # Errors
///
/// An operation cannot be built from the CRS.
pub(super) fn spherical_planetocentric_operations(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Crs::Geodetic(GeodeticCRS::Spherical(spherical)) = source else {
        return Ok(Vec::new());
    };
    if super::factory::geographic_of(target).is_some() && same_geodetic_datum(source, target) {
        return Ok(vec![CoordinateOperation::Transformation(
            geographic_geocentric_conversion(source, target)?,
        )]);
    }
    let intermediate = Crs::Geographic(GeographicCRS::create(
        format!("{} (geographic)", spherical.identified.name),
        spherical.datum.clone(),
        spherical.ensemble.clone(),
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )?);
    let first = CoordinateOperation::Transformation(geographic_geocentric_conversion(
        source,
        &intermediate,
    )?);
    let seconds = match target {
        Crs::Projected(projected)
            if source.is_equivalent_to(&Crs::Geodetic(projected.base_crs().clone())) =>
        {
            vec![CoordinateOperation::Conversion(
                projected
                    .deriving_conversion()
                    .clone()
                    .with_endpoints(intermediate, target.clone()),
            )]
        }
        _ => factory.create_candidates(&intermediate, target, context)?,
    };
    Ok(seconds
        .into_iter()
        .map(|second| {
            CoordinateOperation::Concatenated(pivot_concatenation(vec![first.clone(), second]))
        })
        .collect())
}

/// A geocentric source and a geographic target, or the reverse, on different datums: the
/// conversion to the geographic CRS of the geocentric one, then the geographic operation.
fn geographic_geocentric_different_datum(
    source: &Crs,
    target: &Crs,
    rule: BallparkRule,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    if !is_geocentric(source) {
        let reverse = geographic_geocentric_different_datum(target, source, rule)?;
        return Ok(reverse.iter().map(CoordinateOperation::inverse).collect());
    }
    let Crs::Geodetic(geodetic) = source else {
        return Ok(Vec::new());
    };
    let mut coordinate_system = EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree());
    coordinate_system.axes.push(Axis::new(
        "Ellipsoidal height",
        "h",
        "up",
        UnitOfMeasure::metre(),
    ));
    let mut intermediate = GeographicCRS::create(
        format!("{} (geographic)", source.name()),
        geodetic.datum().cloned(),
        geodetic.datum_ensemble().cloned(),
        coordinate_system,
    )?;
    intermediate.identified.domains = source.domains().to_vec();
    let intermediate = Crs::Geographic(intermediate);
    let first = CoordinateOperation::Transformation(geographic_geocentric_conversion(
        source,
        &intermediate,
    )?);
    let mut found = Vec::new();
    for second in geodetic_operations(&intermediate, target, rule)? {
        found.push(CoordinateOperation::Concatenated(pivot_concatenation(
            vec![first.clone(), second],
        )));
    }
    Ok(found)
}

/// `Transformation::createGeocentricTranslations` with zero values: a null translation on the
/// same datum, a ballpark otherwise.
fn geocentric_translation(
    source: &Crs,
    target: &Crs,
    rule: BallparkRule,
) -> Result<Transformation, FactoryError> {
    let null = rule == BallparkRule::ByDatum
        && (source.is_equivalent_to(target) || same_geodetic_datum(source, target));
    let prefix = if null {
        NULL_GEOCENTRIC_TRANSLATION
    } else {
        BALLPARK_GEOCENTRIC_TRANSLATION
    };
    let mut operation = Transformation::create_from_code(
        &OpProps::new().name(format!(
            "{prefix} from {} to {}",
            source.name(),
            target.name()
        )),
        source.clone(),
        target.clone(),
        METHOD_GEOCENTRIC_TRANSLATIONS,
        vec![ParameterValue::length(0.0); 3],
        world_extent(),
    )?;
    if null {
        operation.accuracy = Some(0.0);
    }
    Ok(operation)
}

/// Offset of the prime meridians, in the unit of the source when both share one, in degrees
/// otherwise.
fn prime_meridian_offset(source: &Crs, target: &Crs) -> Option<Measure> {
    let left = geodetic_frame(source)?.prime_meridian().longitude().clone();
    let right = geodetic_frame(target)?.prime_meridian().longitude().clone();
    if left.unit().name() == right.unit().name()
        && left.unit().conversion_to_si() == right.unit().conversion_to_si()
    {
        return Some(Measure {
            value: left.value() - right.value(),
            unit: left.unit().clone(),
        });
    }
    let degree = UnitOfMeasure::degree();
    let to_degree = |measure: &Measure| {
        measure.value() * measure.unit().conversion_to_si() / degree.conversion_to_si()
    };
    Some(Measure {
        value: to_degree(&left) - to_degree(&right),
        unit: degree,
    })
}

fn prime_meridian_si(crs: &Crs) -> f64 {
    geodetic_frame(crs).map_or(0.0, |frame| frame.prime_meridian().longitude().si_value())
}

fn same_ellipsoid(source: &Crs, target: &Crs) -> bool {
    let (Some(left), Some(right)) = (geodetic_frame(source), geodetic_frame(target)) else {
        return false;
    };
    left.ellipsoid().is_equivalent_to(right.ellipsoid())
}

fn vertical_unit(crs: &GeographicCRS) -> f64 {
    crs.coordinate_system()
        .axes()
        .get(2)
        .map_or(1.0, |axis| axis.unit().conversion_to_si())
}

/// The axis directions as `lat/lon` order: `Some(true)` for latitude first.
fn latitude_first(crs: &GeographicCRS) -> Option<bool> {
    match crs.coordinate_system().axes().first()?.direction() {
        "north" | "south" => Some(true),
        "east" | "west" => Some(false),
        _ => None,
    }
}

fn coordinate_systems_equivalent(left: &GeographicCRS, right: &GeographicCRS) -> bool {
    let (left, right) = (
        left.coordinate_system().axes(),
        right.coordinate_system().axes(),
    );
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.direction() == right.direction()
                && (left.unit().conversion_to_si() - right.unit().conversion_to_si()).abs() < 1e-12
                && left.unit().kind() == right.unit().kind()
        })
}

/// `createOperationsGeogToGeog`.
fn geographic_to_geographic(
    source: &Crs,
    target: &Crs,
    left: &GeographicCRS,
    right: &GeographicCRS,
    rule: BallparkRule,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Some(offset) = prime_meridian_offset(source, target) else {
        return Ok(Vec::new());
    };
    let name = transformation_name(source.name(), target.name());
    let same_datum = rule == BallparkRule::ByDatum && same_geodetic_datum(source, target);
    let (source_height, target_height) = (vertical_unit(left), vertical_unit(right));

    let mut axis_reversal = false;
    if !coordinate_systems_equivalent(left, right) {
        axis_reversal = matches!(
            (latitude_first(left), latitude_first(right)),
            (Some(true), Some(false)) | (Some(false), Some(true))
        );
    }
    let three_d =
        left.coordinate_system().axes().len() == 3 || right.coordinate_system().axes().len() == 3;

    if (source_height - target_height).abs() > f64::EPSILON && same_ellipsoid(source, target) {
        if offset.value() == 0.0 && !axis_reversal {
            if target_height == 0.0 {
                return Err(FactoryError::Invalid(
                    "Conversion factor of target unit is 0".to_owned(),
                ));
            }
            let conversion = Conversion::create_change_vertical_unit(
                &OpProps::new().name(name),
                source_height / target_height,
            )?
            .with_endpoints(source.clone(), target.clone());
            return Ok(vec![CoordinateOperation::Conversion(conversion)]);
        }
        return Ok(vec![CoordinateOperation::Transformation(
            geodetic_proj_based(source, target)?,
        )]);
    }

    if same_datum && axis_reversal {
        let (code, name) = if three_d {
            (9844, "axis order change (geographic3D horizontal)")
        } else {
            (9843, "axis order change (2D)")
        };
        let conversion = Conversion::from_code(&OpProps::new().name(name), code, Vec::new())?
            .with_endpoints(source.clone(), target.clone());
        return Ok(vec![CoordinateOperation::Conversion(conversion)]);
    }

    let (source_pm, target_pm) = (prime_meridian_si(source), prime_meridian_si(target));
    let steps: Vec<CoordinateOperation> = if same_ellipsoid(source, target)
        && source_pm != target_pm
    {
        vec![CoordinateOperation::Transformation(longitude_rotation(
            &name, source, target, &offset,
        )?)]
    } else if source_pm == 0.0 && target_pm != 0.0 {
        let mut altered = altered_prime_meridian(target, source, right)?;
        altered.identified.domains = vec![world_extent()];
        let intermediate = Crs::Geographic(altered);
        let intermediate_name = intermediate.name().to_owned();
        vec![
            CoordinateOperation::Transformation(geographic_offset(source, &intermediate, rule)?),
            CoordinateOperation::Transformation(longitude_rotation(
                &transformation_name(source.name(), &intermediate_name),
                &intermediate,
                target,
                &offset,
            )?),
        ]
    } else if source_pm != target_pm {
        let intermediate = Crs::Geographic(altered_prime_meridian(source, target, left)?);
        let intermediate_name = intermediate.name().to_owned();
        vec![
            CoordinateOperation::Transformation(longitude_rotation(
                &transformation_name(source.name(), &intermediate_name),
                source,
                &intermediate,
                &offset,
            )?),
            CoordinateOperation::Transformation(geographic_offset(&intermediate, target, rule)?),
        ]
    } else {
        vec![CoordinateOperation::Transformation(geographic_offset(
            source, target, rule,
        )?)]
    };
    if steps.len() == 1 {
        return Ok(steps);
    }
    Ok(vec![CoordinateOperation::Concatenated(
        pivot_concatenation(steps),
    )])
}

/// The intermediate CRS of the longitude rotation: the ellipsoid of `crs`, the prime meridian of
/// `other`, and the coordinate system of `shape`.
fn altered_prime_meridian(
    crs: &Crs,
    other: &Crs,
    shape: &GeographicCRS,
) -> Result<GeographicCRS, FactoryError> {
    let (Some(frame), Some(other_frame)) = (geodetic_frame(crs), geodetic_frame(other)) else {
        return Err(FactoryError::Invalid("no geodetic datum".to_owned()));
    };
    GeographicCRS::create(
        format!(
            "{} altered to use prime meridian of {}",
            crs.name(),
            other.name()
        ),
        Some(GeodeticReferenceFrame::create(
            "",
            frame.ellipsoid().clone(),
            other_frame.prime_meridian().clone(),
        )),
        None,
        shape.coordinate_system().clone(),
    )
}

fn longitude_rotation(
    name: &str,
    source: &Crs,
    target: &Crs,
    offset: &Measure,
) -> Result<Transformation, FactoryError> {
    Transformation::create_from_code(
        &OpProps::new().name(name),
        source.clone(),
        target.clone(),
        METHOD_LONGITUDE_ROTATION,
        vec![ParameterValue::Measure(offset.clone())],
        world_extent(),
    )
}

/// `createBallparkGeographicOffset`.
fn geographic_offset(
    source: &Crs,
    target: &Crs,
    rule: BallparkRule,
) -> Result<Transformation, FactoryError> {
    let null = rule == BallparkRule::ByDatum && same_geodetic_datum(source, target);
    let prefix = if null {
        NULL_GEOGRAPHIC_OFFSET
    } else {
        BALLPARK_GEOGRAPHIC_OFFSET
    };
    let domain: ObjectDomain = match (crs_extent(source), crs_extent(target)) {
        (Some(left), Some(right)) if left == right => left,
        _ => world_extent(),
    };
    let three_d = geographic_axes(source) == Some(3) || geographic_axes(target) == Some(3);
    let (code, values) = if three_d {
        (
            METHOD_GEOGRAPHIC3D_OFFSETS,
            vec![
                ParameterValue::angle(0.0),
                ParameterValue::angle(0.0),
                ParameterValue::length(0.0),
            ],
        )
    } else {
        (
            METHOD_GEOGRAPHIC2D_OFFSETS,
            vec![ParameterValue::angle(0.0), ParameterValue::angle(0.0)],
        )
    };
    let mut operation = Transformation::create_from_code(
        &OpProps::new().name(build_op_name(prefix, source, target)),
        source.clone(),
        target.clone(),
        code,
        values,
        domain,
    )?;
    if null {
        operation.accuracy = Some(0.0);
    }
    Ok(operation)
}

/// `createGeodToGeodPROJBased`: the target CRS exported after the inverse of the source CRS.
fn geodetic_proj_based(source: &Crs, target: &Crs) -> Result<Transformation, FactoryError> {
    let mut operation = Transformation::create(
        &OpProps::new().name(transformation_name(source.name(), target.name())),
        source.clone(),
        target.clone(),
        None,
        super::transform_export::PROJ_BASED_METHOD,
        Vec::new(),
        Vec::new(),
        None,
    )?;
    operation.identified.domains = vec![world_extent()];
    Ok(operation)
}
