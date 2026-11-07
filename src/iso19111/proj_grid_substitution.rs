//! `Transformation::substitutePROJAlternativeGridNames` (`singleoperation.cpp:2402`): the
//! operation a registered grid transformation becomes when PROJ reads another file for its
//! grid (`grid_alternatives` in `proj.db`).
//!
//! The replacement is a new operation, built as `createSimilarPropertiesOperation`
//! (`singleoperation.cpp:2305`) builds it: the same name, remarks, domains and accuracy, the
//! identifiers as `DERIVED_FROM(<authority>)` (`addModifiedIdentifier` with `derivedFrom`,
//! `oputils.cpp:177`), and no version. Its method follows the PROJ grid format.
//!
//! Ported: the horizontal grid methods that PROJ writes as `HORIZONTAL_SHIFT_GTIFF` or
//! `GENERAL_SHIFT_GTIFF` (NTv1, NTv2, NADCON and NADCON5 file pairs), in both directions, and
//! the geoid file of a `Geographic3D to GravityRelatedHeight` method.
//!
//! Not ported: the `NTv1`, `NTv2` and `CTable2` grid formats (no `proj.db` row has them), the
//! geocentric translation, velocity and regular vertical grid branches. For those the caller
//! keeps the file rename it already made.

use super::bound::{is_gravity_related_height, HORIZONTAL_SHIFT_GTIFF};
use super::factory::DatabaseContext;
use super::model::{
    Crs, FactoryError, Identified, Identifier, OperationMethod, Parameter, ParameterValue,
    Transformation,
};

/// `EPSG_CODE_METHOD_NTV1`, `EPSG_CODE_METHOD_NTV2` (`proj_constants.h`).
const METHOD_NTV1: i32 = 9614;
const METHOD_NTV2: i32 = 9615;
/// `EPSG_CODE_METHOD_NADCON`, `EPSG_CODE_METHOD_NADCON5_2D`, `EPSG_CODE_METHOD_NADCON5_3D`.
const METHOD_NADCON: i32 = 9613;
const METHOD_NADCON5_2D: i32 = 1074;
const METHOD_NADCON5_3D: i32 = 1075;

/// `EPSG_CODE_PARAMETER_LATITUDE_LONGITUDE_DIFFERENCE_FILE` and its name.
const PARAMETER_LATITUDE_LONGITUDE_DIFFERENCE_FILE: i32 = 8656;
const NAME_LATITUDE_LONGITUDE_DIFFERENCE_FILE: &str = "Latitude and longitude difference file";
/// `EPSG_CODE_PARAMETER_LATITUDE_DIFFERENCE_FILE`, `EPSG_CODE_PARAMETER_LONGITUDE_DIFFERENCE_FILE`.
const PARAMETER_LATITUDE_DIFFERENCE_FILE: i32 = 8657;
const NAME_LATITUDE_DIFFERENCE_FILE: &str = "Latitude difference file";
const PARAMETER_LONGITUDE_DIFFERENCE_FILE: i32 = 8658;
const NAME_LONGITUDE_DIFFERENCE_FILE: &str = "Longitude difference file";
/// `PROJ_WKT2_PARAMETER_LATITUDE_LONGITUDE_ELLIPOISDAL_HEIGHT_DIFFERENCE_FILE`, a name only.
const NAME_LATITUDE_LONGITUDE_ELLIPOISDAL_HEIGHT_DIFFERENCE_FILE: &str =
    "Latitude, longitude and ellipsoidal height difference file";
/// `EPSG_CODE_PARAMETER_GEOID_CORRECTION_FILENAME` and its name.
const PARAMETER_GEOID_CORRECTION_FILENAME: i32 = 8666;
const NAME_GEOID_CORRECTION_FILENAME: &str = "Geoid (height correction) model file";

/// `PROJ_WKT2_NAME_METHOD_GENERAL_SHIFT_GTIFF`.
const GENERAL_SHIFT_GTIFF: &str = "GENERAL_SHIFT_GTIFF";

/// `Transformation::substitutePROJAlternativeGridNames` for the grid methods ported here.
///
/// `None` when PROJ returns the transformation itself (`return self`), or for a branch this
/// port does not cover. The caller then keeps the file rename it made before.
///
/// # Errors
///
/// The database rejects the `grid_alternatives` lookup.
pub(crate) fn substitute_proj_alternative_grid_names(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    if let Some(replacement) = horizontal_grid(transformation, db)? {
        return Ok(Some(replacement));
    }
    geoid_model(transformation, db)
}

/// The horizontal branch of `substitutePROJAlternativeGridNames` (`singleoperation.cpp:2411-2530`).
/// `None` when no horizontal grid of the transformation has a PROJ alternative in a ported
/// format, so that the geoid branch is tried next, as PROJ falls through.
fn horizontal_grid(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    let method = epsg_method_code(transformation);
    let ntv1 = if method == METHOD_NTV1 {
        file_value(
            transformation,
            PARAMETER_LATITUDE_LONGITUDE_DIFFERENCE_FILE,
            NAME_LATITUDE_LONGITUDE_DIFFERENCE_FILE,
        )
    } else {
        None
    };
    let ntv2 = if method == METHOD_NTV2 {
        file_value(
            transformation,
            PARAMETER_LATITUDE_LONGITUDE_DIFFERENCE_FILE,
            NAME_LATITUDE_LONGITUDE_DIFFERENCE_FILE,
        )
    } else {
        None
    };
    // `lasFilename`: the latitude file, when the longitude file is one too.
    let las = if matches!(
        method,
        METHOD_NADCON | METHOD_NADCON5_2D | METHOD_NADCON5_3D
    ) {
        let latitude = file_value(
            transformation,
            PARAMETER_LATITUDE_DIFFERENCE_FILE,
            NAME_LATITUDE_DIFFERENCE_FILE,
        );
        let longitude = file_value(
            transformation,
            PARAMETER_LONGITUDE_DIFFERENCE_FILE,
            NAME_LONGITUDE_DIFFERENCE_FILE,
        );
        match (latitude, longitude) {
            (Some(latitude), Some(_)) => Some(latitude),
            _ => None,
        }
    } else {
        None
    };
    let horizontal = [ntv1, ntv2, las]
        .into_iter()
        .flatten()
        .find(|name| !name.is_empty());
    let Some(horizontal) = horizontal else {
        return Ok(None);
    };
    let Some((proj_filename, proj_format, inverse)) =
        db.proj_grid_alternative_format(&horizontal)?
    else {
        return Ok(None);
    };
    if horizontal == proj_filename {
        // PROJ throws for `inverseDirection` here. No `proj.db` row has an inverse direction
        // for a grid already named as PROJ names it, so the transformation is kept.
        return Ok(None);
    }
    if proj_format != "GTiff" {
        return Ok(None);
    }
    let mut replacement = gtiff_horizontal(transformation, method, &proj_filename);
    // The inverse direction reads the grid as an inverted step.
    replacement.inverted = inverse;
    Ok(Some(replacement))
}

/// The `GTiff` case of the horizontal branch (`singleoperation.cpp:2446-2464`).
fn gtiff_horizontal(
    transformation: &Transformation,
    method: i32,
    proj_filename: &str,
) -> Transformation {
    let method_name = if matches!(method, METHOD_NADCON5_2D | METHOD_NADCON5_3D) {
        GENERAL_SHIFT_GTIFF
    } else {
        HORIZONTAL_SHIFT_GTIFF
    };
    let parameter = if method == METHOD_NADCON5_3D {
        Parameter {
            name: NAME_LATITUDE_LONGITUDE_ELLIPOISDAL_HEIGHT_DIFFERENCE_FILE.to_owned(),
            code_space: String::new(),
            code: String::new(),
            value: ParameterValue::filename(proj_filename),
        }
    } else {
        epsg_parameter(
            PARAMETER_LATITUDE_LONGITUDE_DIFFERENCE_FILE,
            NAME_LATITUDE_LONGITUDE_DIFFERENCE_FILE,
            proj_filename,
        )
    };
    let method = OperationMethod::named_code(method_name.to_owned(), String::new(), String::new());
    similar_operation(
        transformation,
        method,
        vec![parameter],
        transformation.source_crs().cloned(),
        transformation.target_crs().cloned(),
    )
}

/// The geoid branch of `substitutePROJAlternativeGridNames` (`singleoperation.cpp:2540-2577`).
fn geoid_model(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    if !is_gravity_related_height(transformation) {
        return Ok(None);
    }
    let Some(filename) = file_value(
        transformation,
        PARAMETER_GEOID_CORRECTION_FILENAME,
        NAME_GEOID_CORRECTION_FILENAME,
    ) else {
        return Ok(None);
    };
    let Some((proj_filename, _format, inverse)) = db.proj_grid_alternative_format(&filename)?
    else {
        return Ok(None);
    };
    if inverse || filename == proj_filename {
        // PROJ throws for an inverse direction here; no `proj.db` row has one for a geoid file.
        return Ok(None);
    }
    // `createSimilarPropertiesMethod`: the name and the identifier of the method, nothing else.
    let method = transformation.method();
    let method = OperationMethod::named_code(
        method.name().to_owned(),
        method.code_space().to_owned(),
        method.code().to_owned(),
    );
    Ok(Some(similar_operation(
        transformation,
        method,
        vec![epsg_parameter(
            PARAMETER_GEOID_CORRECTION_FILENAME,
            NAME_GEOID_CORRECTION_FILENAME,
            &proj_filename,
        )],
        transformation.source_crs().cloned(),
        transformation.target_crs().cloned(),
    )))
}

/// `createSimilarPropertiesOperation` with the given method, parameters and CRS pair. The
/// name, remarks, domains, interpolation CRS and accuracy are the registered ones; the
/// identifiers are `DERIVED_FROM(<authority>)`; the version and the epochs are not kept.
fn similar_operation(
    transformation: &Transformation,
    method: OperationMethod,
    parameters: Vec<Parameter>,
    source: Option<Crs>,
    target: Option<Crs>,
) -> Transformation {
    let mut identified = Identified::named(transformation.name());
    identified.identifiers = transformation
        .identified
        .identifiers
        .iter()
        .map(|identifier| {
            Identifier::new(
                format!("DERIVED_FROM({})", identifier.code_space()),
                identifier.code(),
            )
        })
        .collect();
    identified.remarks = transformation.identified.remarks.clone();
    identified.domains = transformation.identified.domains.clone();
    Transformation {
        identified,
        method,
        parameters,
        source: source.map(Box::new),
        target: target.map(Box::new),
        interpolation: transformation.interpolation.clone(),
        accuracy: transformation.accuracy,
        accuracy_text: transformation.accuracy_text.clone(),
        version: None,
        point_motion: false,
        inverted: false,
        source_epoch: None,
        target_epoch: None,
    }
}

/// `createOpParamNameEPSGCode`: a file parameter with its EPSG code and name.
fn epsg_parameter(code: i32, name: &str, filename: &str) -> Parameter {
    Parameter {
        name: name.to_owned(),
        code_space: "EPSG".to_owned(),
        code: code.to_string(),
        value: ParameterValue::filename(filename),
    }
}

/// `SingleOperation::parameterValue(name, code)`: the parameter with the EPSG code, else the
/// parameter with the name. Its value is returned when it is a file name.
fn file_value(transformation: &Transformation, code: i32, name: &str) -> Option<String> {
    let by_code = transformation
        .parameters
        .iter()
        .find(|parameter| parameter.code_space == "EPSG" && parameter.code == code.to_string());
    let found = by_code.or_else(|| {
        transformation
            .parameters
            .iter()
            .find(|parameter| parameter.name == name)
    })?;
    match &found.value {
        ParameterValue::Filename(filename) => Some(filename.clone()),
        _ => None,
    }
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

impl DatabaseContext {
    /// `lookForGridAlternative`: the PROJ grid name, its PROJ format and whether PROJ reads the
    /// grid in the inverse direction, for an official grid name.
    pub(crate) fn proj_grid_alternative_format(
        &self,
        original: &str,
    ) -> Result<Option<(String, String, bool)>, FactoryError> {
        let rows = self.query(
            "SELECT proj_grid_name, proj_grid_format, inverse_direction FROM grid_alternatives \
             WHERE original_grid_name = ? AND proj_grid_name <> ''",
            &[original.to_owned()],
        )?;
        Ok(rows
            .first()
            .map(|row| (row[0].text(), row[1].text(), row[2].text() == "1")))
    }
}
