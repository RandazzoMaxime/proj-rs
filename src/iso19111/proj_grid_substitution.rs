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
//! `GENERAL_SHIFT_GTIFF` (NTv1, NTv2, NADCON and NADCON5 file pairs), in both directions, the
//! geoid file of a `Geographic3D to GravityRelatedHeight` method, the VERTCON and regular vertical
//! grid files, the grid of the geocentric translation method 1087, the velocity grid methods 1113
//! and 1114, the `gridTransformations` table (New Zealand deformation model 1079, TIN methods
//! 1137, 1138 and 1145), the velocity grids next to other parameters (methods 1142, 1143 and
//! 1144), and the velocity grid of a point motion operation
//! (`PointMotionOperation::substitutePROJAlternativeGridNames`).
//!
//! Not ported: the `NTv1`, `NTv2` and `CTable2` grid formats (no `proj.db` row has them). For
//! those the caller keeps the file rename it already made.

use super::bound::{is_gravity_related_height, HORIZONTAL_SHIFT_GTIFF};
use super::factory::DatabaseContext;
use super::model::{
    Crs, FactoryError, Identified, Identifier, OperationMethod, Parameter, ParameterValue,
    Transformation,
};
use std::sync::Arc;

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

/// `EPSG_CODE_PARAMETER_VERTICAL_OFFSET_FILE` and `EPSG_CODE_PARAMETER_GEOID_MODEL_DIFFERENCE_FILE`.
const PARAMETER_VERTICAL_OFFSET_FILE: i32 = 8732;
const NAME_VERTICAL_OFFSET_FILE: &str = "Vertical offset file";
const PARAMETER_GEOID_MODEL_DIFFERENCE_FILE: i32 = 1063;
const NAME_GEOID_MODEL_DIFFERENCE_FILE: &str = "Geoid model difference file";
/// `EPSG_CODE_METHOD_VERTCON` and the `isRegularVerticalGridMethod` methods (`singleoperation.cpp`).
const METHOD_VERTCON: i32 = 9658;
const REGULAR_VERTICAL_GRID_METHODS: [i32; 8] = [1071, 1080, 1084, 1085, 1101, 1112, 1126, 1129];

/// `PROJ_WKT2_NAME_METHOD_GENERAL_SHIFT_GTIFF`.
const GENERAL_SHIFT_GTIFF: &str = "GENERAL_SHIFT_GTIFF";

/// `EPSG_CODE_METHOD_GEOCENTRIC_TRANSLATIONS_GEOG2D_DOMAIN_BY_GRID_IGN` and its parameter.
const METHOD_GEOCENTRIC_TRANSLATIONS_BY_GRID_IGN: i32 = 1087;
const PARAMETER_GEOCENTRIC_TRANSLATION_FILE: i32 = 8727;
const NAME_GEOCENTRIC_TRANSLATION_FILE: &str = "Geocentric translation file";

/// `EPSG_CODE_METHOD_POINT_MOTION_BY_GRID_CANADA_NTV2_VEL`, `..._GEOG3D_DOMAIN_..._NTV2_VEL` (the
/// method of the Canada velocity grids, 1141) and `..._GEOCEN_DOMAIN_..._GRAVSOFT`.
const METHOD_POINT_MOTION_BY_GRID_CANADA_NTV2_VEL: i32 = 1070;
const METHOD_POINT_MOTION_GEOG3D_DOMAIN_NEU_VELOCITY_GRID_NTV2_VEL: i32 = 1141;
const METHOD_POINT_MOTION_GEOCEN_DOMAIN_NEU_VELOCITY_GRID_GRAVSOFT: i32 = 1139;
/// `EPSG_CODE_PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE` and its name.
const PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE: i32 = 1050;
const NAME_POINT_MOTION_VELOCITY_GRID_FILE: &str = "Point motion velocity grid file";
/// `EPSG_CODE_METHOD_GEOGRAPHIC3D_OFFSET_BY_VELOCITY_GRID_NTV2_VEL`,
/// `EPSG_CODE_METHOD_VERTICAL_OFFSET_USING_NEU_VELOCITY_GRID_NTV2_VEL` and
/// `EPSG_CODE_METHOD_NEW_ZEALAND_DEFORMATION_MODEL`.
const METHOD_GEOGRAPHIC3D_OFFSET_BY_VELOCITY_GRID_NTV2_VEL: i32 = 1114;
const METHOD_VERTICAL_OFFSET_USING_NEU_VELOCITY_GRID_NTV2_VEL: i32 = 1113;
const METHOD_NEW_ZEALAND_DEFORMATION_MODEL: i32 = 1079;
/// The three `..._BY_TIN_INTERPOLATION_JSON` methods and the `TIN offset file` parameter.
const METHOD_VERTICAL_OFFSET_BY_TIN_INTERPOLATION_JSON: i32 = 1137;
const METHOD_CARTESIAN_GRID_OFFSETS_BY_TIN_INTERPOLATION_JSON: i32 = 1138;
const METHOD_GEOGRAPHIC2D_OFFSETS_BY_TIN_INTERPOLATION_JSON: i32 = 1145;
const PARAMETER_TIN_OFFSET_FILE: i32 = 1064;
const NAME_TIN_OFFSET_FILE: &str = "TIN offset file";
/// The methods with a geocentric translation or a velocity grid file next to other parameters.
const METHOD_GEOCENTRIC_TRANSLATIONS_BY_GRID_GTG_AND_GEOCENTRIC_TRANSLATIONS_NEU_VELOCITIES_GTG:
    i32 = 1142;
const METHOD_POSITION_VECTOR_GEOCENTRIC_AND_GEOCENTRIC_TRANSLATIONS_NEU_VELOCITIES_GTG: i32 = 1143;
const METHOD_GEOCENTRIC_TRANSLATIONS_USING_NEU_VELOCITY_GRID_GTG: i32 = 1144;
/// `EPSG_CODE_PARAMETER_POINT_MOTION_VELOCITY_NORTH_GRID_FILE` and its name.
const PARAMETER_POINT_MOTION_VELOCITY_NORTH_GRID_FILE: i32 = 1072;
const NAME_POINT_MOTION_VELOCITY_NORTH_GRID_FILE: &str = "Point motion velocity north grid file";

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
    if let Some(replacement) = point_motion_grid(transformation, db)? {
        return Ok(Some(replacement));
    }
    if let Some(replacement) = horizontal_grid(transformation, db)? {
        return Ok(Some(replacement));
    }
    if let Some(replacement) = geoid_model(transformation, db)? {
        return Ok(Some(replacement));
    }
    if let Some(replacement) = vertical_grid(transformation, db)? {
        return Ok(Some(replacement));
    }
    if let Some(replacement) = geocentric_grid(transformation, db)? {
        return Ok(Some(replacement));
    }
    if let Some(replacement) = single_file_grid(transformation, db)? {
        return Ok(Some(replacement));
    }
    velocity_grids(transformation, db)
}

/// The branches of `substitutePROJAlternativeGridNames` that replace the whole parameter list by
/// the one file parameter: the velocity grid of the `Geographic3D Offset by velocity grid` and
/// `Vertical Offset using NEU velocity grid` methods (`singleoperation.cpp:2639-2711`), and the
/// `gridTransformations` table (`singleoperation.cpp:2772-2851`: New Zealand deformation model and
/// the TIN interpolation methods).
fn single_file_grid(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    let (code, name) = match epsg_method_code(transformation) {
        METHOD_GEOGRAPHIC3D_OFFSET_BY_VELOCITY_GRID_NTV2_VEL
        | METHOD_VERTICAL_OFFSET_USING_NEU_VELOCITY_GRID_NTV2_VEL
        | METHOD_NEW_ZEALAND_DEFORMATION_MODEL => (
            PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE,
            NAME_POINT_MOTION_VELOCITY_GRID_FILE,
        ),
        METHOD_VERTICAL_OFFSET_BY_TIN_INTERPOLATION_JSON
        | METHOD_CARTESIAN_GRID_OFFSETS_BY_TIN_INTERPOLATION_JSON
        | METHOD_GEOGRAPHIC2D_OFFSETS_BY_TIN_INTERPOLATION_JSON => {
            (PARAMETER_TIN_OFFSET_FILE, NAME_TIN_OFFSET_FILE)
        }
        _ => return Ok(None),
    };
    let Some(filename) = file_value(transformation, code, name) else {
        return Ok(None);
    };
    let Some((proj_filename, _format, inverse)) = db.proj_grid_alternative_format(&filename)?
    else {
        return Ok(None);
    };
    if inverse || filename == proj_filename {
        return Ok(None);
    }
    Ok(Some(similar_operation(
        transformation,
        similar_method(transformation),
        vec![epsg_parameter(code, name, &proj_filename)],
        transformation.source.clone(),
        transformation.target.clone(),
    )))
}

/// The methods that carry a velocity grid next to other parameters
/// (`singleoperation.cpp:2853-2990`): the velocity grid file of 1143 and 1144, and the geocentric
/// translation file and the velocity grid file of 1142, are replaced where they stand and every
/// other parameter is kept. For 1142 both files need a PROJ alternative.
fn velocity_grids(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    let velocity_file = (
        PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE,
        NAME_POINT_MOTION_VELOCITY_GRID_FILE,
    );
    let files = match epsg_method_code(transformation) {
        METHOD_POSITION_VECTOR_GEOCENTRIC_AND_GEOCENTRIC_TRANSLATIONS_NEU_VELOCITIES_GTG
        | METHOD_GEOCENTRIC_TRANSLATIONS_USING_NEU_VELOCITY_GRID_GTG => vec![velocity_file],
        METHOD_GEOCENTRIC_TRANSLATIONS_BY_GRID_GTG_AND_GEOCENTRIC_TRANSLATIONS_NEU_VELOCITIES_GTG =>
        {
            vec![
                (
                    PARAMETER_GEOCENTRIC_TRANSLATION_FILE,
                    NAME_GEOCENTRIC_TRANSLATION_FILE,
                ),
                velocity_file,
            ]
        }
        _ => return Ok(None),
    };
    let mut replaced = Vec::new();
    for (code, name) in files {
        let Some(filename) = file_value(transformation, code, name) else {
            return Ok(None);
        };
        let Some((proj_filename, _format, inverse)) = db.proj_grid_alternative_format(&filename)?
        else {
            return Ok(None);
        };
        if inverse {
            return Ok(None);
        }
        replaced.push((code, filename, proj_filename));
    }
    if replaced
        .iter()
        .all(|(_, filename, proj_filename)| filename == proj_filename)
    {
        return Ok(None);
    }
    let parameters = transformation
        .parameters
        .iter()
        .map(|parameter| {
            let replacement = replaced.iter().find(|(code, _, _)| {
                parameter.code_space == "EPSG" && parameter.code == code.to_string()
            });
            match replacement {
                Some((_, _, proj_filename)) => Parameter {
                    value: ParameterValue::filename(proj_filename),
                    ..parameter.clone()
                },
                None => parameter.clone(),
            }
        })
        .collect();
    Ok(Some(similar_operation(
        transformation,
        similar_method(transformation),
        parameters,
        transformation.source.clone(),
        transformation.target.clone(),
    )))
}

/// `createSimilarPropertiesMethod`: the name and the identifier of the method, nothing else.
fn similar_method(transformation: &Transformation) -> OperationMethod {
    let method = transformation.method();
    OperationMethod::named_code(
        method.name().to_owned(),
        method.code_space().to_owned(),
        method.code().to_owned(),
    )
}

/// The VERTCON and regular vertical grid branch of `substitutePROJAlternativeGridNames`
/// (`singleoperation.cpp:2713-2770`): the vertical offset file, or the geoid model difference
/// file when there is none, is replaced by PROJ's name for it. The method keeps its name and
/// identifier; the parameter is the EPSG one, by code.
fn vertical_grid(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    let method = epsg_method_code(transformation);
    if method != METHOD_VERTCON && !REGULAR_VERTICAL_GRID_METHODS.contains(&method) {
        return Ok(None);
    }
    let (code, name, filename) = match file_value(
        transformation,
        PARAMETER_VERTICAL_OFFSET_FILE,
        NAME_VERTICAL_OFFSET_FILE,
    ) {
        Some(filename) => (
            PARAMETER_VERTICAL_OFFSET_FILE,
            NAME_VERTICAL_OFFSET_FILE,
            filename,
        ),
        None => match file_value(
            transformation,
            PARAMETER_GEOID_MODEL_DIFFERENCE_FILE,
            NAME_GEOID_MODEL_DIFFERENCE_FILE,
        ) {
            Some(filename) => (
                PARAMETER_GEOID_MODEL_DIFFERENCE_FILE,
                NAME_GEOID_MODEL_DIFFERENCE_FILE,
                filename,
            ),
            None => return Ok(None),
        },
    };
    let Some((proj_filename, _format, inverse)) = db.proj_grid_alternative_format(&filename)?
    else {
        return Ok(None);
    };
    if inverse || filename == proj_filename {
        return Ok(None);
    }
    let method = transformation.method();
    let method = OperationMethod::named_code(
        method.name().to_owned(),
        method.code_space().to_owned(),
        method.code().to_owned(),
    );
    Ok(Some(similar_operation(
        transformation,
        method,
        vec![epsg_parameter(code, name, &proj_filename)],
        transformation.source.clone(),
        transformation.target.clone(),
    )))
}

/// `PointMotionOperation::substitutePROJAlternativeGridNames` (`singleoperation.cpp:5591-5629`):
/// a velocity grid file of a point motion operation, read from its `Point motion velocity grid
/// file` parameter (the north grid file for the Gravsoft method), is replaced by PROJ's name for
/// it. The replacement is `createSimilarPropertiesOperation` with the same CRS pair, so its
/// identifiers are `DERIVED_FROM(<authority>)`. The format and the inverse flag of the lookup are
/// not read, as PROJ does not read them here.
fn point_motion_grid(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    if !transformation.is_point_motion() {
        return Ok(None);
    }
    let (code, name) = match epsg_method_code(transformation) {
        METHOD_POINT_MOTION_BY_GRID_CANADA_NTV2_VEL
        | METHOD_POINT_MOTION_GEOG3D_DOMAIN_NEU_VELOCITY_GRID_NTV2_VEL => (
            PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE,
            NAME_POINT_MOTION_VELOCITY_GRID_FILE,
        ),
        METHOD_POINT_MOTION_GEOCEN_DOMAIN_NEU_VELOCITY_GRID_GRAVSOFT => (
            PARAMETER_POINT_MOTION_VELOCITY_NORTH_GRID_FILE,
            NAME_POINT_MOTION_VELOCITY_NORTH_GRID_FILE,
        ),
        _ => return Ok(None),
    };
    let Some(filename) = file_value(transformation, code, name) else {
        return Ok(None);
    };
    let Some((proj_filename, _format, _inverse)) = db.proj_grid_alternative_format(&filename)?
    else {
        return Ok(None);
    };
    if filename == proj_filename {
        return Ok(None);
    }
    // `createSimilarPropertiesMethod`: the name and the identifier of the method.
    let method = transformation.method();
    let method = OperationMethod::named_code(
        method.name().to_owned(),
        method.code_space().to_owned(),
        method.code().to_owned(),
    );
    let mut replacement = similar_operation(
        transformation,
        method,
        vec![epsg_parameter(code, name, &proj_filename)],
        transformation.source.clone(),
        transformation.target.clone(),
    );
    replacement.point_motion = true;
    Ok(Some(replacement))
}

/// The geocentric translation branch of `substitutePROJAlternativeGridNames`
/// (`singleoperation.cpp:2599-2636`): method 1087 reads its grid from the `Geocentric translation
/// file` parameter, and PROJ's name for the grid replaces it.
fn geocentric_grid(
    transformation: &Transformation,
    db: &DatabaseContext,
) -> Result<Option<Transformation>, FactoryError> {
    if epsg_method_code(transformation) != METHOD_GEOCENTRIC_TRANSLATIONS_BY_GRID_IGN {
        return Ok(None);
    }
    let Some(filename) = file_value(
        transformation,
        PARAMETER_GEOCENTRIC_TRANSLATION_FILE,
        NAME_GEOCENTRIC_TRANSLATION_FILE,
    ) else {
        return Ok(None);
    };
    let Some((proj_filename, _format, inverse)) = db.proj_grid_alternative_format(&filename)?
    else {
        return Ok(None);
    };
    if inverse || filename == proj_filename {
        // PROJ throws for an inverse direction here; no `proj.db` row has one for such a grid.
        return Ok(None);
    }
    // `createSimilarPropertiesMethod`: the name and the identifier of the method.
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
            PARAMETER_GEOCENTRIC_TRANSLATION_FILE,
            NAME_GEOCENTRIC_TRANSLATION_FILE,
            &proj_filename,
        )],
        transformation.source.clone(),
        transformation.target.clone(),
    )))
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
        transformation.source.clone(),
        transformation.target.clone(),
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
        transformation.source.clone(),
        transformation.target.clone(),
    )))
}

/// `createSimilarPropertiesOperation` with the given method, parameters and CRS pair. The
/// name, remarks, domains, interpolation CRS and accuracy are the registered ones; the
/// identifiers are `DERIVED_FROM(<authority>)`; the version and the epochs are not kept.
fn similar_operation(
    transformation: &Transformation,
    method: OperationMethod,
    parameters: Vec<Parameter>,
    source: Option<Arc<Crs>>,
    target: Option<Arc<Crs>>,
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
        source,
        target,
        interpolation: transformation.interpolation.clone(),
        accuracy: transformation.accuracy,
        accuracy_text: transformation.accuracy_text.clone(),
        version: None,
        point_motion: false,
        ballpark: false,
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
