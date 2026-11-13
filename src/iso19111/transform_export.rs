//! PROJ strings of transformations and concatenated operations: the
//! `_exportToPROJString` of `Transformation` and `ConcatenatedOperation`, and
//! the methods of `SingleOperation::exportToPROJStringGeneric` they use.
//!
//! Written here: affine (2D, 3D, similarity), Cartesian grid offsets, longitude
//! rotation, axis-order reversal, height/depth reversal, geographic/geocentric
//! conversion, change of vertical unit, the Helmert family (geocentric
//! translations, position vector, coordinate frame, and their time-dependent
//! forms), and the grid methods: `hgridshift` (NTv1, NTv2), `vgridshift` (geoid
//! and vertical grids), `defmodel` (New Zealand), and the `tinshift` JSON methods
//! on projected CRS (1138) and geographic 2D (1145). Grid steps are written by
//! [`Transformation::export_to_proj_string`] and
//! [`ConcatenatedOperation::export_to_proj_string`], matching PROJ
//! `exportToPROJString`, and by
//! [`CoordinateOperation::export_to_proj_string_with_database`], which the `cs2cs`
//! choice uses too, as PROJ's `isPROJExportable` and `proj_create_crs_to_crs` do.
//! Any other method is `Unimplemented`, the error PROJ raises for a method its
//! exporter lacks.
//!
//! A transformation keeps the source and target coordinate epochs it was given
//! (`CoordinateOperationFactory::create_operations_from_metadata` and its siblings).
//! The helmert and `defmodel` exports write PROJ's `set` steps for them. The other
//! methods write none, as PROJ does not for them either.

use std::f64::consts::PI;

use super::buildop::names_match;
use super::equiv::{geodetic_frame, Equivalence};
use super::factory::DatabaseContext;
use super::json::rounded_epoch_in_decimal_year;
use super::model::{
    Axis, ConcatenatedOperation, CoordinateOperation, CoordinateSystem, Crs, Ellipsoid,
    FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Measure,
    ParameterValue, ProjBased, ProjBasedKind, ProjectedCRS, SphericalGeodeticCRS, Transformation,
    UnitKind, UnitOfMeasure, VerticalCRS,
};
use super::pipeline::{proj_number, PipelineFormatter, ProjLines};
use super::projstring::{
    ellipsoid_of, ellipsoid_params, geodetic_member, geographic_of, meridian_of,
    prime_meridian_params, prime_meridian_radians_params, proj_unit_name, projected_axis_order,
    spherical_omits_geoc, well_known_pm_name,
};

const YEAR: f64 = 31_556_925.445;
const ARC_SECOND: f64 = PI / 180.0 / 3600.0;
const ARC_SECOND_PER_YEAR: f64 = PI / 180.0 / 3600.0 / YEAR;
const PARTS_PER_MILLION: f64 = 1e-6;
const PARTS_PER_MILLION_PER_YEAR: f64 = 1e-6 / YEAR;
const METRE_PER_YEAR: f64 = 1.0 / YEAR;
const FOOT: f64 = 0.3048;
const US_FOOT: f64 = 0.304_800_609_601_219_241_184;

const METHOD_AFFINE: i32 = 9624;
const METHOD_SIMILARITY: i32 = 9621;
const METHOD_AXIS_ORDER_REVERSAL_2D: i32 = 9843;
const METHOD_AXIS_ORDER_REVERSAL_3D: i32 = 9844;
const METHOD_GEOGRAPHIC_GEOCENTRIC: i32 = 9602;
const METHOD_CHANGE_VERTICAL_UNIT: i32 = 1069;
const METHOD_HEIGHT_DEPTH_REVERSAL: i32 = 1068;
const METHOD_VERTICAL_OFFSET: i32 = 9616;
const METHOD_VERTICAL_OFFSET_AND_SLOPE: i32 = 1046;
const METHOD_CARTESIAN_GRID_OFFSETS: i32 = 9656;
const METHOD_SEISMIC_BIN_GRID_I_EQ_J_PLUS_90: i32 = 9666;
const METHOD_SEISMIC_BIN_GRID_I_EQ_J_MINUS_90: i32 = 1049;
const METHOD_LONGITUDE_ROTATION: i32 = 9601;
const METHOD_MOLODENSKY: i32 = 9604;
const METHOD_ABRIDGED_MOLODENSKY: i32 = 9605;
const METHOD_GEOGRAPHIC2D_OFFSETS: i32 = 9619;
const METHOD_GEOGRAPHIC3D_OFFSETS: i32 = 9660;
const METHOD_GEOGRAPHIC2D_WITH_HEIGHT_OFFSETS: i32 = 9618;
const METHOD_GEOCENTRIC_GRID_IGN: i32 = 1087;
const METHOD_GEOGRAPHIC3D_TO_GRAVITY_HEIGHT: i32 = 1136;
const METHOD_GEOGRAPHIC3D_TO_GEOG2D_GRAVITY_HEIGHT: i32 = 1131;
const PARAMETER_GEOID_HEIGHT: i32 = 8604;
const METHOD_NTV1: i32 = 9614;
const METHOD_NTV2: i32 = 9615;
const METHOD_NADCON5_2D: i32 = 1074;
const METHOD_NADCON5_3D: i32 = 1075;
const METHOD_POINT_MOTION_CANADA_NTV2_VEL: i32 = 1070;
const METHOD_POINT_MOTION_GEOG3D_NEU_NTV2_VEL: i32 = 1141;
const METHOD_POINT_MOTION_GEOCEN_NEU_GRAVSOFT: i32 = 1139;
const METHOD_VERTCON: i32 = 9658;
const METHOD_NRCAN_BYN: i32 = 1112;
const METHOD_NRCAN_GEOID_DIFFERENCE: i32 = 1126;
const METHOD_NEW_ZEALAND_DEFORMATION_MODEL: i32 = 1079;
const METHOD_TIN_CARTESIAN: i32 = 1138;
const METHOD_TIN_GEOGRAPHIC2D: i32 = 1145;
const METHOD_TIN_VERTICAL: i32 = 1137;
const PARAMETER_LAT_LON_DIFFERENCE_FILE: i32 = 8656;
/// Name of the method of an operation PROJ builds from the two CRS exports (`PROJBasedOperation`).
pub(super) const PROJ_BASED_METHOD: &str = "PROJ-based operation method";
const METHOD_NADCON: i32 = 9613;
const PARAMETER_LATITUDE_DIFFERENCE_FILE: i32 = 8657;
const PARAMETER_LONGITUDE_DIFFERENCE_FILE: i32 = 8658;
const PARAMETER_GEOID_CORRECTION_FILENAME: i32 = 8666;
const PARAMETER_VERTICAL_OFFSET_FILE: i32 = 8732;
const PARAMETER_GEOID_MODEL_DIFFERENCE_FILE: i32 = 1063;
const PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE: i32 = 1050;
const PARAMETER_POINT_MOTION_VELOCITY_NORTH_GRID_FILE: i32 = 1072;
const PARAMETER_TIN_OFFSET_FILE: i32 = 1064;
const PARAMETER_SOURCE_EPOCH: i32 = 1068;
const PARAMETER_TARGET_EPOCH: i32 = 1069;
const METHOD_GEOGRAPHIC3D_OFFSET_BY_VELOCITY_GRID: i32 = 1114;
const METHOD_VERTICAL_OFFSET_NEU_VELOCITY_GRID: i32 = 1113;
const METHOD_POSITION_VECTOR_NEU_VELOCITIES_GTG: i32 = 1143;
const METHOD_GEOCENTRIC_TRANSLATIONS_NEU_VELOCITY_GRID_GTG: i32 = 1144;
const METHOD_GEOCENTRIC_TRANSLATIONS_BY_GRID_AND_NEU_VELOCITIES_GTG: i32 = 1142;

/// `Geographic3D to GravityRelatedHeight` methods, the codes `isGeographic3DToGravityRelatedHeight` lists.
const GEOG3D_TO_HEIGHT_METHODS: [i32; 42] = [
    1025, 1030, 1045, 1047, 1048, 1050, 1059, 1088, 1060, 1072, 1073, 1081, 1083, 1089, 1090, 1091,
    1092, 1093, 1094, 1095, 1096, 1097, 1098, 1100, 1103, 1105, 1109, 1110, 1115, 1118, 1122, 1124,
    1126, 1127, 1128, 1135, 9661, 9662, 9663, 9664, 9665, 9635,
];

/// Vertical grid methods that write `vgridshift` with the multiplier 1, or -1 for NRCan (`isRegularVerticalGridMethod`).
const REGULAR_VERTICAL_GRID_METHODS: [i32; 6] = [1071, 1080, 1084, 1085, 1129, 1101];

impl Transformation {
    /// `exportToPROJString` of a transformation: the pipeline PROJ writes for its method.
    ///
    /// # Errors
    ///
    /// The method is not one this crate writes, the operation is a point motion,
    /// or an endpoint is not a geodetic CRS the method needs.
    pub fn export_to_proj_string(&self, lines: ProjLines) -> Result<String, FactoryError> {
        let mut formatter = PipelineFormatter::with_grid_names(Vec::new()).with_grid_steps();
        self.export_steps(&mut formatter)?;
        Ok(formatter.finish(lines))
    }

    /// `_exportToPROJString`: an inverted operation is its forward operation
    /// written between `startInversion` and `stopInversion`, with the CRS the
    /// forward operation runs from and to.
    pub(crate) fn export_steps(
        &self,
        formatter: &mut PipelineFormatter,
    ) -> Result<(), FactoryError> {
        if self.point_motion {
            return point_motion_velocity_grid(self, formatter);
        }
        if self.inverted {
            formatter.start_inversion();
            let forward = Transformation {
                source_epoch: self.target_epoch,
                target_epoch: self.source_epoch,
                ..self.clone()
            };
            export_forward(&forward, self.target_crs(), self.source_crs(), formatter)?;
            formatter.stop_inversion();
            Ok(())
        } else {
            export_forward(self, self.source_crs(), self.target_crs(), formatter)
        }
    }
}

impl CoordinateOperation {
    /// `exportToPROJString` of a transformation or concatenated operation, with the database
    /// of the formatter: each grid the database renames is written under its PROJ name, as
    /// `substitutePROJAlternativeGridNames` does before the grid steps.
    ///
    /// # Errors
    ///
    /// A conversion has no CRS pair, or a step cannot be written.
    pub fn export_to_proj_string_with_database(
        &self,
        db: &DatabaseContext,
        lines: ProjLines,
    ) -> Result<String, FactoryError> {
        let names = super::select::proj_grid_names(self, db)?;
        let mut formatter = PipelineFormatter::with_grid_names(names).with_grid_steps();
        match self {
            Self::Transformation(transformation) => transformation.export_steps(&mut formatter)?,
            Self::Concatenated(concatenated) => concatenated.export_steps(&mut formatter)?,
            Self::Conversion(conversion) => conversion.export_steps(&mut formatter)?,
        }
        Ok(formatter.finish(lines))
    }
}

impl ConcatenatedOperation {
    /// `exportToPROJString` of a concatenated operation: its steps in order, each
    /// reversed when the step direction is `reverse`.
    ///
    /// # Errors
    ///
    /// A step has no PROJ string in this crate: a conversion step without its CRS
    /// pair, or a transformation method this crate does not write.
    pub fn export_to_proj_string(&self, lines: ProjLines) -> Result<String, FactoryError> {
        let mut formatter = PipelineFormatter::with_grid_names(Vec::new()).with_grid_steps();
        self.export_steps(&mut formatter)?;
        Ok(formatter.finish(lines))
    }

    pub(crate) fn export_steps(
        &self,
        formatter: &mut PipelineFormatter,
    ) -> Result<(), FactoryError> {
        if let Some(proj_based) = &self.proj_based {
            if proj_based.inverted {
                formatter.start_inversion();
            }
            export_proj_based(proj_based, formatter)?;
            if proj_based.inverted {
                formatter.stop_inversion();
            }
            return Ok(());
        }
        let (source_year, target_year) = concatenated_epoch_years(self);
        if let Some(year) = source_year {
            add_epoch_step(year, formatter);
        }
        for step in &self.steps {
            if step.reverse {
                formatter.start_inversion();
            }
            match &step.operation {
                CoordinateOperation::Transformation(transformation) => {
                    transformation.export_steps(formatter)?;
                }
                CoordinateOperation::Concatenated(concatenated) => {
                    concatenated.export_steps(formatter)?;
                }
                CoordinateOperation::Conversion(conversion) => {
                    conversion.export_steps(formatter)?;
                }
            }
            if step.reverse {
                formatter.stop_inversion();
            }
        }
        if let Some(year) = target_year {
            add_epoch_step(year, formatter);
        }
        Ok(())
    }
}

/// `ConcatenatedOperation::_exportToPROJString`: the source and target years of the epochs,
/// rounded, a missing side taking the other one. A year of zero or less writes no step.
fn concatenated_epoch_years(operation: &ConcatenatedOperation) -> (Option<f64>, Option<f64>) {
    let round = |epoch: Option<f64>| {
        epoch
            .map(rounded_epoch_in_decimal_year)
            .filter(|year| *year > 0.0)
    };
    match (round(operation.source_epoch), round(operation.target_epoch)) {
        (Some(year), None) | (None, Some(year)) => (Some(year), Some(year)),
        other => other,
    }
}

/// `SingleOperation::exportToPROJStringGeneric` for the forward direction, with
/// the CRS the forward operation runs from and to.
fn export_forward(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let code = epsg_method_code(operation);
    let method_name = operation.method().name();
    if let Some(text) = method_name.strip_prefix("PROJ-based operation method: ") {
        formatter.ingest_proj_string(text);
        return Ok(());
    }
    if operation.method().code_space() == "PROJ" && operation.method().code() == "PROJString" {
        // A `PROJString` row of `other_transformation` names its PROJ string as the method.
        formatter.ingest_proj_string(method_name);
        return Ok(());
    }
    if method_name.eq_ignore_ascii_case("Inverse of Geographic3D to GravityRelatedHeight")
        || method_name.eq_ignore_ascii_case("Inverse of Geog3D to Geog2D+GravityRelatedHeight")
    {
        return inverse_constant_geoid_height(operation, source, formatter);
    }
    if method_name == PROJ_BASED_METHOD {
        return geodetic_proj_based(source, target, formatter);
    }
    if code == METHOD_AFFINE {
        return affine(operation, formatter);
    }
    if operation.method().name() == "3D Affine parametric transformation" {
        return affine_3d(operation, formatter);
    }
    if code == METHOD_SIMILARITY {
        return similarity(operation, formatter);
    }
    if code == METHOD_SEISMIC_BIN_GRID_I_EQ_J_PLUS_90
        || code == METHOD_SEISMIC_BIN_GRID_I_EQ_J_MINUS_90
    {
        return seismic_bin_grid(operation, code, source, formatter);
    }
    if code == METHOD_AXIS_ORDER_REVERSAL_2D || code == METHOD_AXIS_ORDER_REVERSAL_3D {
        return axis_order_reversal(source, target, formatter);
    }
    if code == METHOD_GEOGRAPHIC_GEOCENTRIC {
        return geographic_geocentric(source, target, formatter);
    }
    if code == METHOD_CHANGE_VERTICAL_UNIT {
        return change_vertical_unit(operation, formatter);
    }
    if code == METHOD_CARTESIAN_GRID_OFFSETS {
        return cartesian_grid_offsets(operation, source, target, formatter);
    }
    if code == METHOD_LONGITUDE_ROTATION {
        return longitude_rotation(operation, source, target, formatter);
    }
    if code == METHOD_MOLODENSKY || code == METHOD_ABRIDGED_MOLODENSKY {
        return molodensky(operation, source, target, formatter);
    }
    if code == METHOD_GEOGRAPHIC2D_OFFSETS
        || code == METHOD_GEOGRAPHIC3D_OFFSETS
        || code == METHOD_GEOGRAPHIC2D_WITH_HEIGHT_OFFSETS
    {
        return geographic_offset(operation, code, source, target, formatter);
    }
    if code == METHOD_HEIGHT_DEPTH_REVERSAL {
        formatter.add_step("axisswap");
        formatter.add_param("order", "1,2,-3");
        return Ok(());
    }
    if code == METHOD_VERTICAL_OFFSET {
        return vertical_offset(operation, source, target, formatter);
    }
    if code == METHOD_VERTICAL_OFFSET_AND_SLOPE {
        return vertical_offset_and_slope(operation, source, target, formatter);
    }
    if code == METHOD_GEOCENTRIC_GRID_IGN {
        return geocentric_grid(operation, source, target, formatter);
    }
    if code == METHOD_GEOGRAPHIC3D_TO_GRAVITY_HEIGHT
        || code == METHOD_GEOGRAPHIC3D_TO_GEOG2D_GRAVITY_HEIGHT
    {
        return constant_geoid_height(operation, target, formatter);
    }
    if code == METHOD_GEOGRAPHIC3D_OFFSET_BY_VELOCITY_GRID {
        return geographic3d_velocity_grid(operation, source, target, formatter);
    }
    if code == METHOD_VERTICAL_OFFSET_NEU_VELOCITY_GRID {
        return vertical_velocity_grid(operation, source, target, formatter);
    }
    if code == METHOD_GEOCENTRIC_TRANSLATIONS_NEU_VELOCITY_GRID_GTG {
        return geocentric_velocity_grid(operation, source, target, formatter);
    }
    if code == METHOD_GEOCENTRIC_TRANSLATIONS_BY_GRID_AND_NEU_VELOCITIES_GTG {
        return geocentric_translation_grid_and_velocity(operation, source, target, formatter);
    }
    if code == METHOD_POSITION_VECTOR_NEU_VELOCITIES_GTG {
        return position_vector_velocity_grid(operation, source, target, formatter);
    }
    if height_to_geog3d_grid_shift(operation, target, formatter)? {
        return Ok(());
    }
    if grid_shift(operation, source, target, formatter)? {
        return Ok(());
    }
    if let Some(kind) = molodensky_badekas_kind(code) {
        return molodensky_badekas(operation, kind, source, target, formatter);
    }
    formatter.set_optimize_units();
    if let Some(kind) = helmert_kind(operation) {
        return helmert(operation, kind, source, target, formatter);
    }
    Err(unimplemented(operation))
}

/// `MyPROJStringExportableGeodToGeod`: the inverse of the source CRS, then the target CRS.
fn geodetic_proj_based(
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let (Some(source), Some(target)) = (
        source.and_then(extract_geographic),
        target.and_then(extract_geographic),
    ) else {
        return Err(not_geodetic_or_geographic("PROJ-based operation"));
    };
    formatter.start_inversion();
    geographic_export(source, formatter)?;
    formatter.stop_inversion();
    geographic_export(target, formatter)
}

/// Molodensky and abridged Molodensky: the source geographic CRS reversed, the `molodensky`
/// step on the source ellipsoid, then the target geographic CRS.
fn molodensky(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let not_geographic =
        || FactoryError::Format("Can apply Molodensky only to GeographicCRS".to_owned());
    let source = source.and_then(geographic_of).ok_or_else(not_geographic)?;
    let target = target.and_then(geographic_of).ok_or_else(not_geographic)?;
    formatter.start_inversion();
    geographic_export(source, formatter)?;
    formatter.stop_inversion();
    formatter.add_step("molodensky");
    ellipsoid_steps(ellipsoid_of(source), formatter);
    formatter.add_number("dx", si_value(operation, 8605));
    formatter.add_number("dy", si_value(operation, 8606));
    formatter.add_number("dz", si_value(operation, 8607));
    formatter.add_number("da", si_value(operation, 8654));
    formatter.add_number("df", si_value(operation, 8655));
    if epsg_method_code(operation) == METHOD_ABRIDGED_MOLODENSKY
        || operation
            .method()
            .name()
            .to_ascii_lowercase()
            .contains("abridged")
    {
        formatter.add_flag("abridged");
    }
    geographic_export(target, formatter)
}

/// The first grid file of a grid method: the file PROJ looks up.
fn primary_grid(operation: &Transformation) -> Result<String, FactoryError> {
    operation
        .parameters()
        .iter()
        .find_map(|parameter| match parameter.value() {
            ParameterValue::Filename(name) => Some(name.clone()),
            _ => None,
        })
        .ok_or_else(|| FactoryError::Format(format!("{} has no grid file", operation.name())))
}

fn geographic_required(method: &str) -> FactoryError {
    FactoryError::Format(format!("Can apply {method} only to GeographicCRS"))
}

/// `xyzgridshift` for a geocentric translation grid
/// (`exportToPROJStringGeneric`). The shift is applied in geocentric
/// coordinates on the ellipsoid of the interpolation CRS, which is the source
/// or the target.
fn geocentric_grid(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let grid = formatter.grid_name(&primary_grid(operation)?);
    let source = source
        .and_then(geographic_of)
        .ok_or_else(|| geographic_required("xyzgridshift"))?;
    let target = target
        .and_then(geographic_of)
        .ok_or_else(|| geographic_required("xyzgridshift"))?;
    let interpolation = operation.interpolation_crs().and_then(geographic_of);
    let on_target = interpolation
        .is_some_and(|crs| crs.is_equivalent_to_with(target, Equivalence::ExceptAxisOrder));
    let on_source = interpolation
        .is_some_and(|crs| crs.is_equivalent_to_with(source, Equivalence::ExceptAxisOrder));
    if !on_source && !on_target {
        return Err(FactoryError::Format(
            "interpolation CRS should be the source or target CRS".to_owned(),
        ));
    }
    formatter.start_inversion();
    angular_unit_and_axis_swap(source.coordinate_system().axes(), formatter)?;
    formatter.stop_inversion();
    formatter.add_step("push");
    formatter.add_flag("v_3");
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(source), formatter);
    formatter.add_step("xyzgridshift");
    formatter.add_param("grids", &grid);
    formatter.add_param(
        "grid_ref",
        if on_target { "output_crs" } else { "input_crs" },
    );
    ellipsoid_steps(
        ellipsoid_of(if on_target { target } else { source }),
        formatter,
    );
    formatter.start_inversion();
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(target), formatter);
    formatter.stop_inversion();
    formatter.add_step("pop");
    formatter.add_flag("v_3");
    angular_unit_and_axis_swap(target.coordinate_system().axes(), formatter)
}

/// The grid and deformation methods of `exportToPROJStringGeneric`. `Ok(false)` when
/// the method is none of them, or its file parameter is absent, so the caller goes on.
fn grid_shift(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    if !formatter.grid_steps() {
        return Ok(false);
    }
    Ok(horizontal_grid_shift(operation, source, target, formatter)?
        || geoid_grid_shift(operation, source, target, formatter)?
        || vertical_grid_shift(operation, formatter)?
        || deformation_grid_shift(operation, source, target, formatter)?
        || tin_shift(operation, source, target, formatter)?)
}

/// The file of the parameter with EPSG code `code`, when it is a file name.
/// `HORIZONTAL_SHIFT_GTIFF` or `GENERAL_SHIFT_GTIFF`, or the inverse of one, as the names
/// `_getHorizontalShiftGTIFFFilename` compares (`singleoperation.cpp:2064-2071`).
fn is_gtiff_method(name: &str) -> bool {
    let forward = match name.get(..INVERSE_OF_PREFIX.len()) {
        Some(prefix) if prefix.eq_ignore_ascii_case(INVERSE_OF_PREFIX) => {
            &name[INVERSE_OF_PREFIX.len()..]
        }
        _ => name,
    };
    forward.eq_ignore_ascii_case("HORIZONTAL_SHIFT_GTIFF")
        || forward.eq_ignore_ascii_case("GENERAL_SHIFT_GTIFF")
}

const INVERSE_OF_PREFIX: &str = "Inverse of ";

fn file_by_code(operation: &Transformation, code: i32) -> Option<String> {
    operation.parameters().iter().find_map(|parameter| {
        if parameter.code() != code.to_string() {
            return None;
        }
        match parameter.value() {
            ParameterValue::Filename(name) => Some(name.clone()),
            _ => None,
        }
    })
}

/// The filename of the parameter named `name`, for a parameter without an EPSG code.
fn file_by_name(operation: &Transformation, name: &str) -> Option<String> {
    operation.parameters().iter().find_map(|parameter| {
        if parameter.name() != name {
            return None;
        }
        match parameter.value() {
            ParameterValue::Filename(file) => Some(file.clone()),
            _ => None,
        }
    })
}

/// The vertical CRS of a target: itself, or the vertical component of a compound CRS.
fn vertical_of(crs: &Crs) -> Option<&VerticalCRS> {
    match crs {
        Crs::Vertical(vertical) => Some(vertical),
        Crs::Compound(compound) => compound.components().iter().find_map(|component| {
            if let Crs::Vertical(vertical) = component {
                Some(vertical)
            } else {
                None
            }
        }),
        _ => None,
    }
}

/// `hgridshift` for NTv1, NTv2, and NADCON, and `gridshift` for NADCON5 once its grid is the
/// GTiff the database names for PROJ. Between the angular unit and axis steps of each geographic CRS.
fn horizontal_grid_shift(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let method_name = operation.method().name();
    // `_getHorizontalShiftGTIFFFilename` (`singleoperation.cpp:2061`): a GTiff method is named,
    // without an EPSG code, once `substitutePROJAlternativeGridNames` wrote it.
    let gtiff = is_gtiff_method(method_name);
    let code = match epsg_method_code(operation) {
        0 if operation.method().name().eq_ignore_ascii_case("CTABLE2") => METHOD_NTV2,
        code => code,
    };
    let file = if gtiff {
        // `_getHorizontalShiftGTIFFFilename` (singleoperation.cpp:2072-2089): the lat-lon file, then
        // the ellipsoidal height file, which is the file of a NADCON5 3D method once substituted.
        file_by_code(operation, PARAMETER_LAT_LON_DIFFERENCE_FILE)
            .or_else(|| file_by_name(operation, "Latitude and longitude difference file"))
            .or_else(|| {
                file_by_name(
                    operation,
                    "Latitude, longitude and ellipsoidal height difference file",
                )
            })
    } else {
        match code {
            METHOD_NTV1 | METHOD_NTV2 => file_by_code(operation, PARAMETER_LAT_LON_DIFFERENCE_FILE)
                .or_else(|| file_by_name(operation, "Latitude and longitude difference file")),
            // The latitude file names the pair: `lookForGridAlternative` replaces it by one GTiff.
            METHOD_NADCON | METHOD_NADCON5_2D | METHOD_NADCON5_3D => {
                file_by_code(operation, PARAMETER_LONGITUDE_DIFFERENCE_FILE)
                    .and_then(|_| file_by_code(operation, PARAMETER_LATITUDE_DIFFERENCE_FILE))
            }
            _ => return Ok(false),
        }
    };
    let Some(file) = file else {
        return Ok(false);
    };
    let method = operation.method().name();
    let source = source
        .and_then(extract_geographic)
        .ok_or_else(|| geographic_required(method))?;
    let target = target
        .and_then(extract_geographic)
        .ok_or_else(|| geographic_required(method))?;
    let grid = formatter.grid_name(&file);
    // `methodName.find(GENERAL_SHIFT_GTIFF)`, and the NADCON5 EPSG methods.
    let nadcon5 = if gtiff {
        method_name.contains("GENERAL_SHIFT_GTIFF")
    } else {
        matches!(code, METHOD_NADCON5_2D | METHOD_NADCON5_3D)
    };
    if !gtiff && nadcon5 && grid == file {
        return Ok(false);
    }
    let inverse = formatter.grid_is_inverse(&file);
    formatter.start_inversion();
    angular_unit_and_axis_swap(source.coordinate_system().axes(), formatter)?;
    formatter.stop_inversion();
    if inverse {
        formatter.start_inversion();
    }
    if nadcon5 {
        formatter.add_step("gridshift");
        // `parameterValue(PROJ_WKT2_PARAMETER_LATITUDE_LONGITUDE_ELLIPOISDAL_HEIGHT_DIFFERENCE_FILE)`
        // for the GTiff method, the EPSG NADCON5 3D method otherwise.
        let three_d = if gtiff {
            operation.parameters().iter().any(|parameter| {
                parameter.name() == "Latitude, longitude and ellipsoidal height difference file"
            })
        } else {
            code == METHOD_NADCON5_3D
        };
        if three_d && source.coordinate_system().axes().len() == 2 {
            formatter.add_flag("no_z_transform");
        }
    } else {
        formatter.add_step("hgridshift");
    }
    formatter.add_param("grids", &grid);
    if inverse {
        formatter.stop_inversion();
    }
    angular_unit_and_axis_swap(target.coordinate_system().axes(), formatter)?;
    Ok(true)
}

/// `vgridshift` for a geographic 3D to gravity-related height method (`Geog3D to GravityRelatedHeight`),
/// with the horizontal steps of the geographic CRS and the axis reversal of a depth.
fn geoid_grid_shift(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let code = epsg_method_code(operation);
    let method = operation.method().name();
    let by_name = method
        .to_ascii_lowercase()
        .starts_with("geographic3d to gravityrelatedheight");
    if !by_name && !GEOG3D_TO_HEIGHT_METHODS.contains(&code) {
        return Ok(false);
    }
    let Some(file) = file_by_code(operation, PARAMETER_GEOID_CORRECTION_FILENAME) else {
        return Ok(false);
    };
    let source = source
        .and_then(extract_geographic)
        .ok_or_else(|| geographic_required(method))?;
    let vertical = target.and_then(vertical_of).ok_or_else(|| {
        FactoryError::Format(format!(
            "Can apply {method} only to a target CRS that has a VerticalCRS"
        ))
    })?;
    let grid = formatter.grid_name(&file);
    let axes = source.coordinate_system().axes();
    let with_horizontal = !formatter.omits_horizontal_in_vertical();
    if with_horizontal {
        formatter.start_inversion();
        horizontal_unit_and_axis_swap(axes, formatter)?;
        formatter.stop_inversion();
    }
    formatter.start_inversion();
    if vertical
        .coordinate_system()
        .axis()
        .direction()
        .eq_ignore_ascii_case("down")
    {
        formatter.add_step("axisswap");
        formatter.add_param("order", "1,2,-3");
    }
    formatter.add_step("vgridshift");
    formatter.add_param("grids", &grid);
    formatter.add_number("multiplier", 1.0);
    formatter.stop_inversion();
    if with_horizontal {
        horizontal_unit_and_axis_swap(axes, formatter)?;
    }
    Ok(true)
}

/// `vgridshift` for `GravityRelatedHeight to Geographic3D`, the method of a vertical CRS bound
/// to a 3D geographic CRS (`_getHeightToGeographic3DFilename`).
fn height_to_geog3d_grid_shift(
    operation: &Transformation,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    const METHOD: &str = "GravityRelatedHeight to Geographic3D";
    let name = operation.method().name();
    let inverse_of = name
        .to_ascii_lowercase()
        .strip_prefix("inverse of ")
        .is_some_and(|rest| rest.eq_ignore_ascii_case(METHOD));
    if !name.eq_ignore_ascii_case(METHOD) && !inverse_of {
        return Ok(false);
    }
    let file = operation.parameters().iter().find_map(|parameter| {
        let is_file = parameter.code() == PARAMETER_GEOID_CORRECTION_FILENAME.to_string()
            || parameter.name() == "Geoid (height correction) model file";
        match parameter.value() {
            ParameterValue::Filename(name) if is_file => Some(name.clone()),
            _ => None,
        }
    });
    let Some(file) = file else {
        return Ok(false);
    };
    let target = target
        .and_then(extract_geographic)
        .ok_or_else(|| geographic_required(name))?;
    let grid = formatter.grid_name(&file);
    let axes = target.coordinate_system().axes();
    let with_horizontal = !formatter.omits_horizontal_in_vertical();
    if with_horizontal {
        formatter.start_inversion();
        horizontal_unit_and_axis_swap(axes, formatter)?;
        formatter.stop_inversion();
    }
    if inverse_of {
        formatter.start_inversion();
    }
    formatter.add_step("vgridshift");
    formatter.add_param("grids", &grid);
    formatter.add_number("multiplier", 1.0);
    if inverse_of {
        formatter.stop_inversion();
    }
    if with_horizontal {
        horizontal_unit_and_axis_swap(axes, formatter)?;
    }
    Ok(true)
}

/// `vgridshift` for a vertical grid: VERTCON, the regular methods, and NRCan (`multiplier` -1).
/// A grid file named `.tif` takes the multiplier 1, a VERTCON `.gtx` grid the millimetre 0.001.
fn vertical_grid_shift(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let code = epsg_method_code(operation);
    if code == METHOD_VERTCON {
        let Some(file) = file_by_code(operation, PARAMETER_VERTICAL_OFFSET_FILE) else {
            return Ok(false);
        };
        let grid = formatter.grid_name(&file);
        let multiplier = if grid.contains(".tif") { 1.0 } else { 0.001 };
        formatter.add_step("vgridshift");
        formatter.add_param("grids", &grid);
        formatter.add_number("multiplier", multiplier);
        return Ok(true);
    }
    let reverse = code == METHOD_NRCAN_BYN || code == METHOD_NRCAN_GEOID_DIFFERENCE;
    if !reverse && !REGULAR_VERTICAL_GRID_METHODS.contains(&code) {
        return Ok(false);
    }
    let file = file_by_code(operation, PARAMETER_VERTICAL_OFFSET_FILE)
        .or_else(|| file_by_code(operation, PARAMETER_GEOID_MODEL_DIFFERENCE_FILE));
    let Some(file) = file else {
        return Ok(false);
    };
    let grid = formatter.grid_name(&file);
    formatter.add_step("vgridshift");
    formatter.add_param("grids", &grid);
    formatter.add_number("multiplier", if reverse { -1.0 } else { 1.0 });
    Ok(true)
}

/// `defmodel` for the New Zealand deformation model, between the geographic CRS of each side.
fn deformation_grid_shift(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    if epsg_method_code(operation) != METHOD_NEW_ZEALAND_DEFORMATION_MODEL {
        return Ok(false);
    }
    let Some(file) = file_by_code(operation, PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE) else {
        return Ok(false);
    };
    let method = operation.method().name();
    let source = source
        .and_then(geographic_of)
        .ok_or_else(|| geographic_required(method))?;
    let target = target
        .and_then(geographic_of)
        .ok_or_else(|| geographic_required(method))?;
    formatter.start_inversion();
    angular_unit_and_axis_swap(source.coordinate_system().axes(), formatter)?;
    formatter.stop_inversion();
    let (source_year, target_year) = operation_epoch_years(operation);
    if let Some(year) = source_year {
        add_epoch_step(year, formatter);
    }
    let model = formatter.grid_name(&file);
    formatter.start_inversion();
    formatter.add_step("defmodel");
    formatter.add_param("model", &model);
    formatter.stop_inversion();
    if let Some(year) = target_year {
        add_epoch_step(year, formatter);
    }
    angular_unit_and_axis_swap(target.coordinate_system().axes(), formatter)?;
    Ok(true)
}

/// `tinshift` on projected CRS (method 1138), on geographic 2D CRS in degrees (method 1145),
/// or on vertical CRS (method 1137).
fn tin_shift(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let code = epsg_method_code(operation);
    if code == METHOD_TIN_VERTICAL {
        // `EPSG_CODE_METHOD_VERTICAL_OFFSET_BY_TIN_INTERPOLATION_JSON` (singleoperation.cpp:5057):
        // both CRS must be vertical, checked before the file; no unit conversion is written.
        let method = operation.method().name();
        if !matches!(source, Some(Crs::Vertical(_))) || !matches!(target, Some(Crs::Vertical(_))) {
            return Err(FactoryError::Format(format!(
                "Can apply {method} only to VerticalCRS"
            )));
        }
        let Some(file) = file_by_code(operation, PARAMETER_TIN_OFFSET_FILE) else {
            return Ok(false);
        };
        // `isMethodInverseOf`: a method named `Inverse of ...` writes the step inverted.
        let inverse_method = method.starts_with("Inverse of ");
        if inverse_method {
            formatter.start_inversion();
        }
        formatter.add_step("tinshift");
        formatter.add_param("file", &formatter.grid_name(&file));
        if inverse_method {
            formatter.stop_inversion();
        }
        return Ok(true);
    }
    if code == METHOD_TIN_CARTESIAN {
        let Some(file) = file_by_code(operation, PARAMETER_TIN_OFFSET_FILE) else {
            return Ok(false);
        };
        let method = operation.method().name();
        let source = match source {
            Some(Crs::Projected(projected)) => projected,
            _ => {
                return Err(FactoryError::Format(format!(
                    "Can apply {method} only to ProjectedCRS"
                )))
            }
        };
        let target = match target {
            Some(Crs::Projected(projected)) => projected,
            _ => {
                return Err(FactoryError::Format(format!(
                    "Can apply {method} only to ProjectedCRS"
                )))
            }
        };
        formatter.start_inversion();
        projected_unit_and_axis_swap(source, formatter);
        formatter.stop_inversion();
        formatter.add_step("tinshift");
        formatter.add_param("file", &formatter.grid_name(&file));
        projected_unit_and_axis_swap(target, formatter);
        return Ok(true);
    }
    if code == METHOD_TIN_GEOGRAPHIC2D {
        let Some(file) = file_by_code(operation, PARAMETER_TIN_OFFSET_FILE) else {
            return Ok(false);
        };
        let method = operation.method().name();
        let degrees = |crs: &GeographicCRS| {
            crs.coordinate_system()
                .axes()
                .first()
                .is_some_and(|axis| units_equivalent(axis.unit(), &UnitOfMeasure::degree()))
        };
        let source = source
            .and_then(geographic_of)
            .filter(|crs| degrees(crs))
            .ok_or_else(|| {
                FactoryError::Format(format!(
                    "Can apply {method} only to source GeographicCRS with degree axis unit"
                ))
            })?;
        let target = target
            .and_then(geographic_of)
            .filter(|crs| degrees(crs))
            .ok_or_else(|| {
                FactoryError::Format(format!(
                    "Can apply {method} only to target GeographicCRS with degree axis unit"
                ))
            })?;
        formatter.start_inversion();
        axis_swap(source.coordinate_system().axes(), formatter);
        formatter.stop_inversion();
        formatter.add_step("tinshift");
        formatter.add_param("file", &formatter.grid_name(&file));
        axis_swap(target.coordinate_system().axes(), formatter);
        return Ok(true);
    }
    Ok(false)
}

/// `Geographic3D offset using NEU velocity grid (NTv2_Vel)` (1114): the source CRS in radians,
/// a `deformation` step on the velocity grid over the time from the source datum epoch to the
/// target datum epoch, then the target CRS. The interpolation CRS, the source or the target,
/// gives the ellipsoid of the deformation.
fn geographic3d_velocity_grid(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let method = operation.method().name();
    let not_geographic =
        || FactoryError::Format(format!("Can apply {method} only to GeographicCRS"));
    let source_year = anchor_years(source, method, "source")?;
    let target_year = anchor_years(target, method, "target")?;
    let source = source.and_then(geographic_of).ok_or_else(not_geographic)?;
    let target = target.and_then(geographic_of).ok_or_else(not_geographic)?;
    let interpolation = operation.interpolation_crs().and_then(geographic_of);
    let on_target = interpolation.is_some_and(|crs| {
        crs.is_equivalent_to_with(&target.demote_to_2d(None), Equivalence::Equivalent)
    });
    let on_source = interpolation.is_some_and(|crs| {
        crs.is_equivalent_to_with(&source.demote_to_2d(None), Equivalence::Equivalent)
    });
    if !on_source && !on_target {
        return Err(FactoryError::Format(format!(
            "For {method}, interpolation CRS should be the source or target CRS"
        )));
    }
    let grid = formatter.grid_name(&velocity_grid_file(operation)?);
    let source_axes = source.coordinate_system().axes();
    let target_axes = target.coordinate_system().axes();
    let push_pop = source_axes.len() == 2 || target_axes.len() == 2;

    formatter.start_inversion();
    angular_unit_and_axis_swap(source_axes, formatter)?;
    formatter.stop_inversion();
    if push_pop {
        formatter.add_step("push");
        formatter.add_flag("v_3");
    }
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(source), formatter);
    formatter.add_step("deformation");
    formatter.add_number("dt", target_year - source_year);
    formatter.add_param("grids", &grid);
    ellipsoid_steps(
        ellipsoid_of(if on_target { target } else { source }),
        formatter,
    );
    formatter.start_inversion();
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(target), formatter);
    formatter.stop_inversion();
    if push_pop {
        formatter.add_step("pop");
        formatter.add_flag("v_3");
    }
    angular_unit_and_axis_swap(target_axes, formatter)
}

/// `PointMotionOperation::_exportToPROJString` for `Point motion by grid (Canada NTv2_Vel)`
/// (1070), a geographic source: the source CRS in radians, a `set` to the source epoch that
/// omits the forward direction, a `deformation` step over the time between the two epochs, a
/// `set` to the target epoch that omits the inverse direction, then the source CRS back.
fn point_motion_velocity_grid(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    if !matches!(
        epsg_method_code(operation),
        METHOD_POINT_MOTION_CANADA_NTV2_VEL
            | METHOD_POINT_MOTION_GEOG3D_NEU_NTV2_VEL
            | METHOD_POINT_MOTION_GEOCEN_NEU_GRAVSOFT
    ) {
        return Err(FactoryError::Format(format!(
            "Unimplemented {}",
            operation.name()
        )));
    }
    let (Some(source_epoch), Some(target_epoch)) = (
        operation.source_epoch.map(rounded_epoch_in_decimal_year),
        operation.target_epoch.map(rounded_epoch_in_decimal_year),
    ) else {
        return Err(FactoryError::Format(
            "CoordinateOperationNNPtr::_exportToPROJString() unimplemented when source coordinate epoch is missing"
                .to_owned(),
        ));
    };
    let grid = formatter.grid_name(&velocity_grid_file(operation)?);
    if let Some(Crs::Geodetic(GeodeticCRS::Geocentric(geocentric))) = operation.source_crs() {
        formatter.add_step("set");
        formatter.add_number("v_4", source_epoch);
        formatter.add_step("deformation");
        formatter.add_number("dt", target_epoch - source_epoch);
        formatter.add_param("grids", &grid);
        ellipsoid_steps(
            geocentric.datum().map(GeodeticReferenceFrame::ellipsoid),
            formatter,
        );
        formatter.add_step("set");
        formatter.add_number("v_4", target_epoch);
        return Ok(());
    }
    let source = operation
        .source_crs()
        .and_then(geographic_of)
        .ok_or_else(|| {
            FactoryError::Format(
                "Can apply PointMotionOperation VelocityGrid only to GeodeticCRS".to_owned(),
            )
        })?;
    let axes = source.coordinate_system().axes();

    formatter.start_inversion();
    angular_unit_and_axis_swap(axes, formatter)?;
    formatter.stop_inversion();
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(source), formatter);
    formatter.add_step("set");
    formatter.add_number("v_4", source_epoch);
    formatter.add_flag("omit_fwd");
    formatter.add_step("deformation");
    formatter.add_number("dt", target_epoch - source_epoch);
    formatter.add_param("grids", &grid);
    ellipsoid_steps(ellipsoid_of(source), formatter);
    formatter.add_step("set");
    formatter.add_number("v_4", target_epoch);
    formatter.add_flag("omit_inv");
    formatter.start_inversion();
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(source), formatter);
    formatter.stop_inversion();
    angular_unit_and_axis_swap(axes, formatter)
}

/// `Geocen translations by grid (gtg) & Geocen translations NEU velocities (gtg)` (1142): the
/// geocentric source, a `xyzgridshift` on the translation grid, a `deformation` over the time
/// between the two epochs on the velocity grid, then the geocentric target.
fn geocentric_translation_grid_and_velocity(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let method = operation.method().name();
    let (source, target) = geocentric_endpoints(source, target, method)?;
    let translation = file_by_name(operation, "Geocentric translation file").ok_or_else(|| {
        FactoryError::Format("Missing parameter Geocentric translation file".to_owned())
    })?;
    let translation = formatter.grid_name(&translation);
    let velocity = formatter.grid_name(&velocity_grid_file(operation)?);
    let source_year = epoch_years(operation, PARAMETER_SOURCE_EPOCH)
        .ok_or_else(|| FactoryError::Format("Missing parameter source epoch".to_owned()))?;
    let target_year = target_epoch_years(operation)?;
    setup_source(source, PushV3::No, formatter)?;
    formatter.add_step("xyzgridshift");
    formatter.add_param("grids", &translation);
    formatter.add_step("deformation");
    formatter.add_number(
        "dt",
        rounded_epoch_in_decimal_year(target_year - source_year),
    );
    formatter.add_param("grids", &velocity);
    geocentric_ellipsoid_steps(source, formatter);
    setup_target(target, PushV3::No, formatter)
}

/// `Vertical offset using NEU velocity grid (NTv2_Vel)` (1113): the horizontal position is
/// pushed aside, a `deformation` step on the velocity grid moves the height over the time from
/// the source to the target vertical datum epoch, then the position is popped back.
fn vertical_velocity_grid(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let method = operation.method().name();
    let interpolation = operation
        .interpolation_crs()
        .and_then(geographic_of)
        .ok_or_else(|| {
            FactoryError::Format(format!(
                "Can apply {method} only to a GeographicCRS interpolation CRS"
            ))
        })?;
    let vertical_epoch = |crs: Option<&Crs>, role: &str| {
        let vertical = crs.and_then(vertical_of).ok_or_else(|| {
            FactoryError::Format(format!("Can apply {method} only to a {role} VerticalCRS"))
        })?;
        vertical
            .datum()
            .and_then(|datum| datum.anchor_epoch())
            .ok_or_else(|| {
                FactoryError::Format(format!("For {method}, missing epoch for {role} CRS"))
            })
    };
    let source_year = vertical_epoch(source, "source")?;
    let target_year = vertical_epoch(target, "target")?;
    let grid = formatter.grid_name(&velocity_grid_file(operation)?);
    formatter.add_step("push");
    formatter.add_flag("v_1");
    formatter.add_flag("v_2");
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(interpolation), formatter);
    formatter.add_step("deformation");
    formatter.add_number("dt", target_year - source_year);
    formatter.add_param("grids", &grid);
    ellipsoid_steps(ellipsoid_of(interpolation), formatter);
    formatter.start_inversion();
    formatter.add_step("cart");
    ellipsoid_steps(ellipsoid_of(interpolation), formatter);
    formatter.stop_inversion();
    formatter.add_step("pop");
    formatter.add_flag("v_1");
    formatter.add_flag("v_2");
    Ok(())
}

/// `Geocentric translations using NEU velocity grid (gtg)` (1144): the source CRS, a
/// `deformation` step run backwards to the target epoch, then the target CRS.
fn geocentric_velocity_grid(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let method = operation.method().name();
    let (source, target) = geocentric_endpoints(source, target, method)?;
    let grid = formatter.grid_name(&velocity_grid_file(operation)?);
    if measure_by_code(operation, PARAMETER_SOURCE_EPOCH).is_some() {
        return Err(FactoryError::Format(
            "Unsupported parameter source epoch".to_owned(),
        ));
    }
    let target_year = target_epoch_years(operation)?;
    setup_source(source, PushV3::No, formatter)?;
    formatter.start_inversion();
    formatter.add_step("deformation");
    formatter.add_number("t_epoch", target_year);
    formatter.add_param("grids", &grid);
    geocentric_ellipsoid_steps(source, formatter);
    formatter.stop_inversion();
    setup_target(target, PushV3::No, formatter)
}

/// `Position Vector (geocen) & Geocen translations NEU velocities (gtg)` (1143): a Helmert
/// step between geocentric CRS, then a `deformation` step on the velocity grid over the time
/// from the source to the target epoch, or to the target epoch alone.
fn position_vector_velocity_grid(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let method = operation.method().name();
    let (source, target) = geocentric_endpoints(source, target, method)?;
    let grid = formatter.grid_name(&velocity_grid_file(operation)?);
    setup_source(source, PushV3::No, formatter)?;
    formatter.add_step("helmert");
    formatter.add_number("x", si_value(operation, 8605));
    formatter.add_number("y", si_value(operation, 8606));
    formatter.add_number("z", si_value(operation, 8607));
    formatter.add_number("rx", numeric(operation, 8608, ARC_SECOND));
    formatter.add_number("ry", numeric(operation, 8609, ARC_SECOND));
    formatter.add_number("rz", numeric(operation, 8610, ARC_SECOND));
    formatter.add_number("s", numeric(operation, 8611, PARTS_PER_MILLION));
    formatter.add_param("convention", "position_vector");
    let target_year = target_epoch_years(operation)?;
    formatter.add_step("deformation");
    match epoch_years(operation, PARAMETER_SOURCE_EPOCH) {
        Some(source_year) => formatter.add_number(
            "dt",
            rounded_epoch_in_decimal_year(target_year - source_year),
        ),
        None => formatter.add_number("t_epoch", target_year),
    }
    formatter.add_param("grids", &grid);
    geocentric_ellipsoid_steps(source, formatter);
    setup_target(target, PushV3::No, formatter)
}

/// The two CRS of a velocity grid method on geocentric CRS.
fn geocentric_endpoints<'a>(
    source: Option<&'a Crs>,
    target: Option<&'a Crs>,
    method: &str,
) -> Result<(&'a Crs, &'a Crs), FactoryError> {
    let refused = || FactoryError::Format(format!("Can apply {method} only to a geocentric CRS"));
    let geocentric = |crs: &Crs| matches!(geodetic_direct(crs), Some(Geodetic::Geocentric(_)));
    match (source, target) {
        (Some(source), Some(target)) if geocentric(source) && geocentric(target) => {
            Ok((source, target))
        }
        _ => Err(refused()),
    }
}

fn geocentric_ellipsoid_steps(crs: &Crs, formatter: &mut PipelineFormatter) {
    if let Some(Geodetic::Geocentric(geocentric)) = geodetic_direct(crs) {
        ellipsoid_steps(
            geocentric_frame(geocentric).map(GeodeticReferenceFrame::ellipsoid),
            formatter,
        );
    }
}

fn velocity_grid_file(operation: &Transformation) -> Result<String, FactoryError> {
    file_by_code(operation, PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE)
        .or_else(|| file_by_code(operation, PARAMETER_POINT_MOTION_VELOCITY_NORTH_GRID_FILE))
        .ok_or_else(|| {
            FactoryError::Format("Missing parameter Point motion velocity grid file".to_owned())
        })
}

fn target_epoch_years(operation: &Transformation) -> Result<f64, FactoryError> {
    epoch_years(operation, PARAMETER_TARGET_EPOCH)
        .ok_or_else(|| FactoryError::Format("Missing parameter target epoch".to_owned()))
}

/// An epoch parameter in years, as `parameterValueNumeric(code, YEAR)`.
fn epoch_years(operation: &Transformation, code: i32) -> Option<f64> {
    measure_by_code(operation, code).map(|measure| {
        if measure.unit().conversion_to_si() == YEAR {
            measure.value()
        } else {
            measure.si_value() / YEAR
        }
    })
}

/// The anchor epoch in years of the datum of `crs`, which a velocity grid method needs.
fn anchor_years(crs: Option<&Crs>, method: &str, role: &str) -> Result<f64, FactoryError> {
    crs.and_then(geodetic_frame)
        .and_then(|frame| frame.anchor_epoch())
        .ok_or_else(|| FactoryError::Format(format!("For {method}, missing epoch for {role} CRS")))
}

/// `addAngularUnitConvertAndAxisSwap` with the height unit left out: the `pushOmitZUnitConversion` form.
fn horizontal_unit_and_axis_swap(
    axes: &[Axis],
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let Some(first) = axes.first() else {
        return Err(FactoryError::Format(
            "a geographic CRS has no axis".to_owned(),
        ));
    };
    formatter.add_step("unitconvert");
    formatter.add_param("xy_in", "rad");
    formatter.add_param("xy_out", &unit_text(first.unit()));
    axis_swap(axes, formatter);
    Ok(())
}

fn unimplemented(operation: &Transformation) -> FactoryError {
    FactoryError::Format(format!("Unimplemented {}", operation.name()))
}

/// The EPSG method code, or 0 when the method has no EPSG identifier.
fn epsg_method_code(operation: &Transformation) -> i32 {
    let method = operation.method();
    let identified = if method.code_space() == "EPSG" {
        method.code().parse().unwrap_or(0)
    } else {
        0
    };
    if identified != 0 {
        return identified;
    }
    // `OperationMethod::getEPSGCode`: a method without an EPSG identifier takes the code
    // of its name.
    super::parammap::method_by_name(method.name()).map_or(0, |known| known.epsg_code)
}

/// The measure of the parameter with EPSG code `code`, as PROJ's
/// `parameterValueMeasure(code)`. Absent or not a measure gives `None`.
fn measure_by_code(operation: &Transformation, code: i32) -> Option<&Measure> {
    let by_code = operation
        .parameters()
        .iter()
        .find(|parameter| parameter.code() == code.to_string());
    // `OperationParameter::getEPSGCode`: a parameter without an EPSG identifier takes the
    // code of its name, within the method the name gives.
    let parameter = by_code.or_else(|| {
        let method = super::parammap::method_by_name(operation.method().name())?;
        let known = method.params.iter().find(|known| known.epsg_code == code)?;
        operation.parameters().iter().find(|parameter| {
            parameter.code().is_empty() && names_match(parameter.name(), known.wkt2_name)
        })
    })?;
    match parameter.value() {
        ParameterValue::Measure(measure) => Some(measure),
        _ => None,
    }
}

fn measure_by_name<'a>(operation: &'a Transformation, name: &str) -> Option<&'a Measure> {
    operation.parameters().iter().find_map(|parameter| {
        if names_match(parameter.name(), name) {
            match parameter.value() {
                ParameterValue::Measure(measure) => Some(measure),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// `parameterValueMeasure(code).value()`: the value in its own unit, zero when absent.
fn raw_value(operation: &Transformation, code: i32) -> f64 {
    measure_by_code(operation, code).map_or(0.0, Measure::value)
}

/// `parameterValueNumericAsSI(code)`: the SI value, zero when absent.
fn si_value(operation: &Transformation, code: i32) -> f64 {
    measure_by_code(operation, code).map_or(0.0, Measure::si_value)
}

/// `parameterValueNumeric(code, unit)`: the value in a unit with factor `to_si`, zero when absent.
fn numeric(operation: &Transformation, code: i32, to_si: f64) -> f64 {
    si_value(operation, code) / to_si
}

fn affine(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    formatter.add_step("affine");
    formatter.add_number("xoff", raw_value(operation, 8623));
    formatter.add_number("s11", raw_value(operation, 8624));
    formatter.add_number("s12", raw_value(operation, 8625));
    formatter.add_number("yoff", raw_value(operation, 8639));
    formatter.add_number("s21", raw_value(operation, 8640));
    formatter.add_number("s22", raw_value(operation, 8641));
    Ok(())
}

fn affine_3d(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let named = |name: &str| measure_by_name(operation, name).map_or(0.0, Measure::value);
    formatter.add_step("affine");
    formatter.add_number("xoff", raw_value(operation, 8623));
    formatter.add_number("s11", raw_value(operation, 8624));
    formatter.add_number("s12", raw_value(operation, 8625));
    formatter.add_number("s13", named("A3"));
    formatter.add_number("yoff", raw_value(operation, 8639));
    formatter.add_number("s21", raw_value(operation, 8640));
    formatter.add_number("s22", raw_value(operation, 8641));
    formatter.add_number("s23", named("B3"));
    formatter.add_number("zoff", named("C0"));
    formatter.add_number("s31", named("C1"));
    formatter.add_number("s32", named("C2"));
    formatter.add_number("s33", named("C3"));
    Ok(())
}

fn similarity(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let xt0 = raw_value(operation, 8621);
    let yt0 = raw_value(operation, 8622);
    let scale = raw_value(operation, 1061);
    let angle = numeric(operation, 8614, 1.0);
    formatter.add_step("affine");
    formatter.add_number("xoff", xt0);
    formatter.add_number("s11", scale * angle.cos());
    formatter.add_number("s12", scale * angle.sin());
    formatter.add_number("yoff", yt0);
    formatter.add_number("s21", -scale * angle.sin());
    formatter.add_number("s22", scale * angle.cos());
    Ok(())
}

/// EPSG guidance note 7-2, section 2.4.3.3. EPSG orders these operations from the map grid
/// to the bin grid, so the coefficients are the reverse of the guidance note's formulas.
fn seismic_bin_grid(
    operation: &Transformation,
    code: i32,
    source: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let origin_i = si_value(operation, 8733);
    let origin_j = si_value(operation, 8734);
    let origin_easting = numeric(operation, 8735, 1.0);
    let origin_northing = numeric(operation, 8736, 1.0);
    let scale_factor = si_value(operation, 8737);
    let width_i = numeric(operation, 8738, 1.0);
    let width_j = numeric(operation, 8739, 1.0);
    let theta = numeric(operation, 8740, 1.0);
    let increment_i = si_value(operation, 8741);
    let increment_j = si_value(operation, 8742);
    if scale_factor == 0.0 || width_i == 0.0 || width_j == 0.0 {
        return Err(FactoryError::Format(
            "Invalid seismic bin grid operation: the scale factor of the bin grid and the bin \
             widths must be non-zero"
                .to_owned(),
        ));
    }
    let scale_i = increment_i / (scale_factor * width_i);
    let scale_j = increment_j / (scale_factor * width_j);
    // I=J-90 (left-handed) differs from I=J+90 only by the sign of the I axis.
    let sign_i = if code == METHOD_SEISMIC_BIN_GRID_I_EQ_J_PLUS_90 {
        1.0
    } else {
        -1.0
    };
    let s11 = sign_i * scale_i * theta.cos();
    let s12 = -sign_i * scale_i * theta.sin();
    let s21 = scale_j * theta.sin();
    let s22 = scale_j * theta.cos();
    let xoff = origin_i - (s11 * origin_easting + s12 * origin_northing);
    let yoff = origin_j - (s21 * origin_easting + s22 * origin_northing);
    if let Some(Crs::Projected(projected)) = source {
        formatter.start_inversion();
        projected_unit_and_axis_swap(projected, formatter);
        formatter.stop_inversion();
    }
    formatter.add_step("affine");
    formatter.add_number("xoff", xoff);
    formatter.add_number("s11", s11);
    formatter.add_number("s12", s12);
    formatter.add_number("yoff", yoff);
    formatter.add_number("s21", s21);
    formatter.add_number("s22", s22);
    Ok(())
}

pub(super) fn axis_order_reversal(
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    formatter.add_step("axisswap");
    formatter.add_param("order", "2,1");
    let (Some(source), Some(target)) = (
        source.and_then(geographic_of),
        target.and_then(geographic_of),
    ) else {
        return Ok(());
    };
    let source_unit = source.coordinate_system().axes()[0].unit();
    let target_unit = target.coordinate_system().axes()[0].unit();
    if !units_equivalent(source_unit, target_unit) {
        formatter.add_step("unitconvert");
        formatter.add_param("xy_in", &unit_text(source_unit));
        formatter.add_param("xy_out", &unit_text(target_unit));
    }
    Ok(())
}

/// `UnitOfMeasure::_isEquivalentTo` with the equivalent criterion.
fn units_equivalent(left: &UnitOfMeasure, right: &UnitOfMeasure) -> bool {
    let left_si = left.conversion_to_si();
    (left_si - right.conversion_to_si()).abs() <= 1e-10 * left_si.abs()
}

/// The PROJ name of a unit, or its factor when PROJ has no name for it.
fn unit_text(unit: &UnitOfMeasure) -> String {
    proj_unit_name(unit).map_or_else(|| proj_number(unit.conversion_to_si()), str::to_owned)
}

fn geographic_geocentric(
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let (Some(source), Some(target)) = (
        source.and_then(geodetic_front),
        target.and_then(geodetic_front),
    ) else {
        return Err(invalid_geographic_geocentric());
    };
    let source_geocentric = matches!(source, Geodetic::Geocentric(_) | Geodetic::Spherical(_));
    let source_geographic = matches!(source, Geodetic::Geographic(_));
    let target_geocentric = matches!(target, Geodetic::Geocentric(_) | Geodetic::Spherical(_));
    let target_geographic = matches!(target, Geodetic::Geographic(_));
    if !((source_geocentric && target_geographic) || (source_geographic && target_geocentric)) {
        return Err(invalid_geographic_geocentric());
    }
    formatter.start_inversion();
    geographic_geocentric_end(source, formatter)?;
    formatter.stop_inversion();
    geographic_geocentric_end(target, formatter)
}

/// One end of a geographic/geocentric conversion. A geographic end with a zero prime
/// meridian writes only its angular unit and axis order, not `longlat`. A spherical
/// planetocentric end omits `geoc` when `spherical_omits_geoc` holds, as the latitude
/// conversion's export does (conversion.cpp:3868, crs.cpp:2504-2511).
fn geographic_geocentric_end(
    geodetic: Geodetic<'_>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    match geodetic {
        Geodetic::Geographic(geographic) => {
            let zero_meridian = meridian_of(geographic)
                .is_none_or(|meridian| prime_meridian_params(meridian).is_none());
            if zero_meridian {
                angular_unit_and_axis_swap(geographic.coordinate_system().axes(), formatter)
            } else {
                geodetic_export(geodetic, formatter)
            }
        }
        Geodetic::Spherical(spherical) if spherical_omits_geoc(spherical) => {
            angular_unit_and_axis_swap(spherical.coordinate_system.axes(), formatter)
        }
        other => geodetic_export(other, formatter),
    }
}

fn invalid_geographic_geocentric() -> FactoryError {
    FactoryError::Format(
        "Invalid nature of source and/or targetCRS for Geographic/Geocentric conversion".to_owned(),
    )
}

/// EPSG method code of a conversion, or 0 when the method has no EPSG identifier.
pub(super) fn conversion_method_code(conversion: &super::model::Conversion) -> i32 {
    let method = conversion.method();
    if method.code_space() == "EPSG" {
        method.code().parse().unwrap_or(0)
    } else {
        0
    }
}

/// `Geographic2D offsets` (9619) or `Geographic3D offsets` (9660).
pub(super) fn is_geographic_offset(conversion: &super::model::Conversion) -> bool {
    let code = conversion_method_code(conversion);
    code == METHOD_GEOGRAPHIC2D_OFFSETS || code == METHOD_GEOGRAPHIC3D_OFFSETS
}

/// `Geographic2D offsets` and `Geographic3D offsets` (`exportToPROJStringGeneric`).
///
/// Angular offsets are arc-seconds. A zero offset writes no `geogoffset` step.
/// The source and target are geographic CRS, a derived CRS with an ellipsoidal
/// system, or a bound CRS of one of those.
///
/// # Errors
///
/// A CRS is not geographic.
pub(super) fn geographic_offsets(
    conversion: &super::model::Conversion,
    source: &Crs,
    target: &Crs,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let three_d = conversion_method_code(conversion) == METHOD_GEOGRAPHIC3D_OFFSETS;
    let refused = if three_d {
        "Can apply Geographic 3D offsets only to GeographicCRS"
    } else {
        "Can apply Geographic 2D offsets only to GeographicCRS"
    };
    let source_axes = offset_axes(source, refused)?;
    let target_axes = offset_axes(target, refused)?;
    let latitude = parameter_numeric(conversion, 8601, ARC_SECOND);
    let longitude = parameter_numeric(conversion, 8602, ARC_SECOND);
    let height = if three_d {
        parameter_si(conversion, 8603)
    } else {
        0.0
    };
    formatter.start_inversion();
    angular_unit_and_axis_swap(&source_axes, formatter)?;
    formatter.stop_inversion();
    if latitude != 0.0 || longitude != 0.0 || height != 0.0 {
        formatter.add_step("geogoffset");
        formatter.add_number("dlat", latitude);
        formatter.add_number("dlon", longitude);
        if three_d {
            formatter.add_number("dh", height);
        }
    }
    angular_unit_and_axis_swap(&target_axes, formatter)
}

/// Axes of the geographic CRS a geographic-offset step accepts.
fn offset_axes(crs: &Crs, message: &'static str) -> Result<Vec<Axis>, FactoryError> {
    let refused = || FactoryError::Format(message.to_owned());
    match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            Ok(geographic.coordinate_system().axes().to_vec())
        }
        Crs::Derived(derived) => match derived.coordinate_system() {
            CoordinateSystem::Ellipsoidal(system) => Ok(system.axes().to_vec()),
            _ => Err(refused()),
        },
        Crs::Bound(bound) => offset_axes(bound.source_crs(), message),
        _ => Err(refused()),
    }
}

/// Parameter value in the unit whose SI factor is `to_si`, zero when absent.
fn parameter_numeric(conversion: &super::model::Conversion, code: i32, to_si: f64) -> f64 {
    parameter_si(conversion, code) / to_si
}

/// SI value of the parameter with EPSG code `code`, zero when absent.
fn parameter_si(conversion: &super::model::Conversion, code: i32) -> f64 {
    conversion
        .parameters()
        .iter()
        .find_map(|parameter| {
            if parameter.code() != code.to_string() {
                return None;
            }
            match parameter.value() {
                ParameterValue::Measure(measure) => Some(measure.si_value()),
                _ => None,
            }
        })
        .unwrap_or(0.0)
}

/// `Geographic2D offsets`, `Geographic3D offsets` and `Geographic2D with Height Offsets`:
/// the angular unit and axis steps of each endpoint, and a `geogoffset` step when an
/// offset is not zero. A zero offset still converts the units, and cancels to `noop`
/// only when the endpoints already share them.
fn geographic_offset(
    operation: &Transformation,
    code: i32,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let height_code = match code {
        METHOD_GEOGRAPHIC3D_OFFSETS => Some(8603),
        METHOD_GEOGRAPHIC2D_WITH_HEIGHT_OFFSETS => Some(8604),
        _ => None,
    };
    let latitude = numeric(operation, 8601, ARC_SECOND);
    let longitude = numeric(operation, 8602, ARC_SECOND);
    let height = height_code.map(|code| si_value(operation, code));
    let method = operation.method().name();
    let not_geographic =
        || FactoryError::Format(format!("Can apply {method} only to GeographicCRS"));
    let source = source
        .and_then(extract_geographic)
        .ok_or_else(not_geographic)?;
    let target = target
        .and_then(extract_geographic)
        .ok_or_else(not_geographic)?;
    formatter.start_inversion();
    angular_unit_and_axis_swap(source.coordinate_system().axes(), formatter)?;
    formatter.stop_inversion();
    if latitude != 0.0 || longitude != 0.0 || height.is_some_and(|height| height != 0.0) {
        formatter.add_step("geogoffset");
        formatter.add_number("dlat", latitude);
        formatter.add_number("dlon", longitude);
        if let Some(height) = height {
            formatter.add_number("dh", height);
        }
    }
    angular_unit_and_axis_swap(target.coordinate_system().axes(), formatter)
}

/// `Vertical Offset`: the linear unit of each vertical CRS, and `geogoffset +dh`
/// when the offset is not zero. A compound CRS whose horizontal parts match
/// contributes its vertical component.
fn vertical_offset(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let (source, target) = vertical_pair(source, target)?;
    formatter.start_inversion();
    linear_unit_convert(source, formatter);
    formatter.stop_inversion();
    let offset = si_value(operation, 8603);
    if offset != 0.0 {
        formatter.add_step("geogoffset");
        formatter.add_number("dh", offset);
    }
    linear_unit_convert(target, formatter);
    Ok(())
}

/// `Vertical Offset and Slope` (singleoperation.cpp:4255-4311): the linear unit of each
/// vertical CRS around `vertoffset`, with the evaluation point in degrees, the offset in metres
/// and the inclinations in arc-seconds. A compound CRS whose horizontal parts match contributes
/// its vertical component.
fn vertical_offset_and_slope(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let (source, target) = vertical_pair(source, target).map_err(|_| {
        FactoryError::Format("Can apply Vertical offset and slope only to VerticalCRS".to_owned())
    })?;
    let degree = PI / 180.0;
    formatter.start_inversion();
    linear_unit_convert(source, formatter);
    formatter.stop_inversion();
    formatter.add_step("vertoffset");
    formatter.add_number("lat_0", numeric(operation, 8617, degree));
    formatter.add_number("lon_0", numeric(operation, 8618, degree));
    formatter.add_number("dh", si_value(operation, 8603));
    formatter.add_number("slope_lat", numeric(operation, 8730, ARC_SECOND));
    formatter.add_number("slope_lon", numeric(operation, 8731, ARC_SECOND));
    linear_unit_convert(target, formatter);
    Ok(())
}

/// Which vertical CRS carries the constant geoid shift, and the sign of `zoff`.
enum GeoidShift {
    /// Method 1136: subtract the geoid height from the target vertical CRS.
    Forward,
    /// An inverse method name: add the geoid height on the source vertical CRS.
    InverseMethod,
}

/// `Geographic3D to GravityRelatedHeight` with a constant geoid height: an
/// `affine` height shift, then the linear unit of the target vertical CRS.
fn constant_geoid_height(
    operation: &Transformation,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    write_geoid_shift(operation, target, GeoidShift::Forward, formatter)
}

/// Inverse method names of 1136 and 1131. `INVERSE(EPSG)` is not an EPSG code,
/// so the forward branch never sees them. The height is read from the source.
fn inverse_constant_geoid_height(
    operation: &Transformation,
    source: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    write_geoid_shift(operation, source, GeoidShift::InverseMethod, formatter)
}

fn write_geoid_shift(
    operation: &Transformation,
    crs: Option<&Crs>,
    shift: GeoidShift,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let vertical = match crs {
        Some(Crs::Compound(compound)) => compound.components().get(1),
        other => other,
    };
    let Some(Crs::Vertical(vertical)) = vertical else {
        let message = match shift {
            GeoidShift::Forward => {
                "Can apply Geographic3D to GravityRelatedHeight only to VerticalCRS"
            }
            GeoidShift::InverseMethod => {
                "Inverse of Geographic3D to GravityRelatedHeight only to VerticalCRS"
            }
        };
        return Err(FactoryError::Format(message.to_owned()));
    };
    let geoid_height = si_value(operation, PARAMETER_GEOID_HEIGHT);
    let zoff = match shift {
        GeoidShift::Forward => -geoid_height,
        GeoidShift::InverseMethod => geoid_height,
    };
    if zoff != 0.0 {
        formatter.add_step("affine");
        formatter.add_number("zoff", zoff);
    }
    let unit = vertical.coordinate_system().axis().unit();
    if unit.conversion_to_si() != 1.0 {
        formatter.add_step("unitconvert");
        formatter.add_param("z_in", "m");
        match proj_unit_name(unit) {
            Some(name) => formatter.add_param("z_out", name),
            None => formatter.add_number("z_out", unit.conversion_to_si()),
        }
    }
    Ok(())
}

fn change_vertical_unit(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    export_vertical_unit_factor(si_value(operation, 1051), formatter);
    Ok(())
}

/// `exportToPROJStringChangeVerticalUnit`: `z_in`/`z_out` from the SI factor of the
/// source unit over the target unit.
pub(crate) fn export_vertical_unit_factor(factor: f64, formatter: &mut PipelineFormatter) {
    let forward = proj_linear_name(factor);
    let reverse = (factor != 0.0)
        .then(|| proj_linear_name(1.0 / factor))
        .flatten();
    if forward == Some("m") {
        return;
    }
    if let Some(from) = forward {
        formatter.add_step("unitconvert");
        formatter.add_param("z_in", from);
        formatter.add_param("z_out", "m");
    } else if let Some(to) = reverse {
        formatter.add_step("unitconvert");
        formatter.add_param("z_in", "m");
        formatter.add_param("z_out", to);
    } else if (factor - FOOT / US_FOOT).abs() < 1e-10 {
        formatter.add_step("unitconvert");
        formatter.add_param("z_in", "ft");
        formatter.add_param("z_out", "us-ft");
    } else if (factor - US_FOOT / FOOT).abs() < 1e-10 {
        formatter.add_step("unitconvert");
        formatter.add_param("z_in", "us-ft");
        formatter.add_param("z_out", "ft");
    } else {
        formatter.add_step("affine");
        formatter.add_number("s33", factor);
    }
}

/// `Change of Vertical Unit` for a unit conversion factor.
pub(super) fn change_vertical_unit_by_factor(factor: f64, formatter: &mut PipelineFormatter) {
    export_vertical_unit_factor(factor, formatter);
}

fn vertical_pair<'a>(
    source: Option<&'a Crs>,
    target: Option<&'a Crs>,
) -> Result<(&'a VerticalCRS, &'a VerticalCRS), FactoryError> {
    let (source, target) = match (source, target) {
        (Some(Crs::Compound(source_crs)), Some(Crs::Compound(target_crs)))
            if source_crs.components().first().is_some_and(|left| {
                target_crs
                    .components()
                    .first()
                    .is_some_and(|right| left.is_equivalent_to(right))
            }) =>
        {
            (
                source_crs.components().get(1),
                target_crs.components().get(1),
            )
        }
        other => other,
    };
    match (vertical_crs(source), vertical_crs(target)) {
        (Some(source), Some(target)) => Ok((source, target)),
        _ => Err(FactoryError::Format(
            "Can apply Vertical offset only to VerticalCRS".to_owned(),
        )),
    }
}

fn vertical_crs(crs: Option<&Crs>) -> Option<&VerticalCRS> {
    match crs {
        Some(Crs::Vertical(vertical)) => Some(vertical),
        _ => None,
    }
}

fn linear_unit_convert(vertical: &VerticalCRS, formatter: &mut PipelineFormatter) {
    let unit = vertical.coordinate_system().axis().unit();
    if unit.conversion_to_si() == 1.0 {
        return;
    }
    formatter.add_step("unitconvert");
    formatter.add_param("z_in", "m");
    match proj_unit_name(unit) {
        Some(name) => formatter.add_param("z_out", name),
        None => formatter.add_number("z_out", unit.conversion_to_si()),
    }
}

/// `Cartesian grid offsets`: a shift of the projected or engineering CRS
/// coordinates, between the unit and axis handling of each endpoint.
fn cartesian_grid_offsets(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let easting = numeric(operation, 8728, 1.0);
    let northing = numeric(operation, 8729, 1.0);
    let source_projected = grid_offset_endpoint(source)?;
    let target_projected = grid_offset_endpoint(target)?;
    if let Some(projected) = source_projected {
        formatter.start_inversion();
        projected_unit_and_axis_swap(projected, formatter);
        formatter.stop_inversion();
    }
    if easting != 0.0 || northing != 0.0 {
        formatter.add_step("affine");
        formatter.add_number("xoff", easting);
        formatter.add_number("yoff", northing);
    }
    if let Some(projected) = target_projected {
        projected_unit_and_axis_swap(projected, formatter);
    }
    Ok(())
}

/// The projected CRS of a Cartesian grid offset endpoint, or `None` for an
/// engineering CRS, which must be Cartesian with metre axes.
fn grid_offset_endpoint(crs: Option<&Crs>) -> Result<Option<&ProjectedCRS>, FactoryError> {
    match crs {
        Some(Crs::Projected(projected)) => Ok(Some(projected)),
        Some(Crs::Engineering(engineering)) => {
            let CoordinateSystem::Cartesian(cartesian) = engineering.coordinate_system() else {
                return Err(FactoryError::Format(
                    "Can apply Cartesian grid offsets only to EngineeringCRS with CartesianCS"
                        .to_owned(),
                ));
            };
            let metric = cartesian
                .axes()
                .first()
                .is_some_and(|axis| units_equivalent(axis.unit(), &UnitOfMeasure::metre()));
            if !metric {
                return Err(FactoryError::Format(
                    "Can apply Cartesian grid offsets only to EngineeringCRS with CartesianCS with metre unit"
                        .to_owned(),
                ));
            }
            Ok(None)
        }
        _ => Err(FactoryError::Format(
            "Can apply Cartesian grid offsets only to ProjectedCRS or EngineeringCRS".to_owned(),
        )),
    }
}

/// `ProjectedCRS::addUnitConvertAndAxisSwap` outside a CRS: the metre
/// conversion when an axis is not metre, then the axis order.
fn projected_unit_and_axis_swap(projected: &ProjectedCRS, formatter: &mut PipelineFormatter) {
    let axes = projected.coordinate_system().axes();
    let Some(first) = axes.first() else {
        return;
    };
    let unit = first.unit();
    let height = if axes.len() == 3 {
        axes.get(2).map(Axis::unit)
    } else {
        None
    };
    let metre = UnitOfMeasure::metre();
    let metric = units_equivalent(unit, &metre)
        && height.is_none_or(|height| units_equivalent(height, &metre));
    if !metric {
        formatter.add_step("unitconvert");
        formatter.add_param("xy_in", "m");
        if height.is_some() {
            formatter.add_param("z_in", "m");
        }
        formatter.add_param("xy_out", &unit_text(unit));
        if let Some(height) = height {
            formatter.add_param("z_out", &unit_text(height));
        }
    }
    if let Some(order) = projected_axis_order(projected) {
        formatter.add_step("axisswap");
        formatter.add_param("order", &order);
    }
}

/// PROJ name of the linear unit with factor `to_si`.
fn proj_linear_name(to_si: f64) -> Option<&'static str> {
    proj_unit_name(&UnitOfMeasure::new("", to_si, UnitKind::Linear))
}

/// Which parameters a Helmert method has, as `SingleOperation::exportToPROJStringGeneric` counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HelmertParams {
    /// Three translations only.
    Three,
    /// Translations, rotations, and scale.
    Seven,
    /// The seven, with a rate of change of each and a reference epoch.
    Fifteen,
}

/// Which Helmert form a method writes, as `SingleOperation::exportToPROJStringGeneric` decides it.
#[derive(Clone, Copy, Debug)]
struct HelmertKind {
    /// `position_vector` rather than `coordinate_frame` in the `convention` parameter.
    position_vector: bool,
    params: HelmertParams,
    full_matrix: bool,
}

impl Transformation {
    /// `createApproximateInverseIfPossible`: an inverted seven or fifteen parameter Helmert
    /// transformation written as the forward method with every value but the reference epoch
    /// negated. The name says `approx. inversion` unless every rotation, scale and rate is zero.
    pub(super) fn approximate_inverse(&self) -> Option<Self> {
        if !self.inverted {
            return None;
        }
        let kind = helmert_kind(self)?;
        if kind.full_matrix || kind.params == HelmertParams::Three {
            return None;
        }
        let mut approximate = self.clone();
        approximate.inverted = false;
        let mut exact = true;
        for parameter in &mut approximate.parameters {
            if parameter.name == "Parameter reference epoch" {
                continue;
            }
            if let ParameterValue::Measure(measure) = &mut parameter.value {
                let is_translation = measure.unit.kind == UnitKind::Linear;
                measure.value = 0.0 - measure.value;
                exact &= is_translation || measure.value == 0.0;
            }
        }
        if !exact {
            approximate.identified.name.push_str(" (approx. inversion)");
        }
        Some(approximate)
    }
}

/// The convention and domain of a Molodensky-Badekas method.
#[derive(Clone, Copy)]
struct MolodenskyBadekasKind {
    position_vector: bool,
    push_pop_v3: PushV3,
}

/// The Molodensky-Badekas variants: coordinate frame 1034, 9636, 1039 and
/// position vector 1061, 1063, 1062, for the geocentric, geographic 2D and geographic 3D domains.
fn molodensky_badekas_kind(code: i32) -> Option<MolodenskyBadekasKind> {
    let (position_vector, geographic_2d) = match code {
        1034 | 1039 => (false, false),
        9636 => (false, true),
        1061 | 1062 => (true, false),
        1063 => (true, true),
        _ => return None,
    };
    Some(MolodenskyBadekasKind {
        position_vector,
        push_pop_v3: if geographic_2d {
            PushV3::Yes
        } else {
            PushV3::No
        },
    })
}

fn molodensky_badekas(
    operation: &Transformation,
    kind: MolodenskyBadekasKind,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    if let Some(source) = source {
        setup_source(source, kind.push_pop_v3, formatter)?;
    }
    formatter.add_step("molobadekas");
    formatter.add_number("x", si_value(operation, 8605));
    formatter.add_number("y", si_value(operation, 8606));
    formatter.add_number("z", si_value(operation, 8607));
    formatter.add_number("rx", numeric(operation, 8608, ARC_SECOND));
    formatter.add_number("ry", numeric(operation, 8609, ARC_SECOND));
    formatter.add_number("rz", numeric(operation, 8610, ARC_SECOND));
    formatter.add_number("s", numeric(operation, 8611, PARTS_PER_MILLION));
    formatter.add_number("px", si_value(operation, 8617));
    formatter.add_number("py", si_value(operation, 8618));
    formatter.add_number("pz", si_value(operation, 8667));
    formatter.add_param(
        "convention",
        if kind.position_vector {
            "position_vector"
        } else {
            "coordinate_frame"
        },
    );
    if let Some(target) = target {
        setup_target(target, kind.push_pop_v3, formatter)?;
    }
    Ok(())
}

/// The Helmert form of a method, or `None` when the method is not a Helmert
/// family method.
fn helmert_kind(operation: &Transformation) -> Option<HelmertKind> {
    let code = epsg_method_code(operation);
    let name = operation.method().name();
    let count = operation.parameters().len();
    let time_dependent = is_time_dependent(name);
    let is_position_vector = ci_contains(name, "Position Vector") || ci_contains(name, "PV");
    let is_coordinate_frame = ci_contains(name, "Coordinate Frame") || ci_contains(name, "CF");
    let form = |position_vector, params, full_matrix| HelmertKind {
        position_vector,
        params,
        full_matrix,
    };
    if matches!(code, 1132 | 1133 | 1140) {
        Some(form(false, HelmertParams::Seven, true))
    } else if (count == 7 && is_coordinate_frame && !time_dependent)
        || matches!(code, 1032 | 9607 | 1038 | 1149)
    {
        Some(form(false, HelmertParams::Seven, false))
    } else if (count == 15 && is_coordinate_frame && time_dependent) || matches!(code, 1056..=1058)
    {
        Some(form(false, HelmertParams::Fifteen, false))
    } else if (count == 7 && is_position_vector && !time_dependent)
        || matches!(code, 1033 | 9606 | 1037)
    {
        Some(form(true, HelmertParams::Seven, false))
    } else if (count == 15 && is_position_vector && time_dependent) || matches!(code, 1053..=1055) {
        Some(form(true, HelmertParams::Fifteen, false))
    } else if (count == 3 && ci_contains(name, "Geocentric translations"))
        || matches!(code, 1031 | 9603 | 1035)
    {
        Some(form(true, HelmertParams::Three, false))
    } else {
        None
    }
}

/// Whether a Helmert step is wrapped in `push v_3` and `pop v_3`, which keeps the
/// height aside while the horizontal transformation runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PushV3 {
    Yes,
    No,
}

/// `isTimeDependent`: the method name says so.
fn is_time_dependent(name: &str) -> bool {
    ci_contains(name, "Time dependent") || ci_contains(name, "Time-dependent")
}

/// Case-insensitive substring test, as PROJ's `ci_find`.
fn ci_contains(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn helmert(
    operation: &Transformation,
    kind: HelmertKind,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let source_geographic = source.and_then(geographic_of);
    let target_geographic = target.and_then(geographic_of);
    let source_compound = source_geographic.is_none() && matches!(source, Some(Crs::Compound(_)));
    let target_compound = target_geographic.is_none() && matches!(target, Some(Crs::Compound(_)));
    let source_axes = source_geographic.map(|crs| crs.coordinate_system().axes().len());
    let target_axes = target_geographic.map(|crs| crs.coordinate_system().axes().len());
    let push_pop =
        ((source_axes == Some(2) || target_axes == Some(2)) || source_compound || target_compound)
            && !(source_axes == Some(3) && target_compound)
            && !(target_axes == Some(3) && source_compound);

    let push_v3 = if push_pop { PushV3::Yes } else { PushV3::No };
    if let Some(source) = source {
        setup_source(source, push_v3, formatter)?;
    }
    let (source_year, target_year) = operation_epoch_years(operation);
    if let Some(year) = source_year {
        add_epoch_step(year, formatter);
    }
    formatter.add_step("helmert");
    if kind.full_matrix {
        formatter.add_flag("exact");
    }
    formatter.add_number("x", si_value(operation, 8605));
    formatter.add_number("y", si_value(operation, 8606));
    formatter.add_number("z", si_value(operation, 8607));
    if kind.params != HelmertParams::Three {
        formatter.add_number("rx", numeric(operation, 8608, ARC_SECOND));
        formatter.add_number("ry", numeric(operation, 8609, ARC_SECOND));
        formatter.add_number("rz", numeric(operation, 8610, ARC_SECOND));
        formatter.add_number("s", numeric(operation, 8611, PARTS_PER_MILLION));
        if kind.params == HelmertParams::Fifteen {
            formatter.add_number("dx", numeric(operation, 1040, METRE_PER_YEAR));
            formatter.add_number("dy", numeric(operation, 1041, METRE_PER_YEAR));
            formatter.add_number("dz", numeric(operation, 1042, METRE_PER_YEAR));
            formatter.add_number("drx", numeric(operation, 1043, ARC_SECOND_PER_YEAR));
            formatter.add_number("dry", numeric(operation, 1044, ARC_SECOND_PER_YEAR));
            formatter.add_number("drz", numeric(operation, 1045, ARC_SECOND_PER_YEAR));
            formatter.add_number("ds", numeric(operation, 1046, PARTS_PER_MILLION_PER_YEAR));
            formatter.add_number("t_epoch", numeric(operation, 1047, YEAR));
        }
        let convention = if kind.position_vector {
            "position_vector"
        } else {
            "coordinate_frame"
        };
        formatter.add_param("convention", convention);
    }
    if let Some(year) = target_year {
        add_epoch_step(year, formatter);
    }
    if let Some(target) = target {
        setup_target(target, push_v3, formatter)?;
    }
    Ok(())
}

/// `sourceYear` and `targetYear` of PROJ's helmert and defmodel exports: the rounded epoch of
/// each side, a year of zero or less counting as none. A side without an epoch takes the other's.
fn operation_epoch_years(operation: &Transformation) -> (Option<f64>, Option<f64>) {
    let round = |epoch: Option<f64>| {
        epoch
            .map(rounded_epoch_in_decimal_year)
            .filter(|year| *year > 0.0)
    };
    match (round(operation.source_epoch), round(operation.target_epoch)) {
        (Some(year), None) | (None, Some(year)) => (Some(year), Some(year)),
        other => other,
    }
}

/// `set` with `v_4`, the coordinate time the following step reads.
fn add_epoch_step(year: f64, formatter: &mut PipelineFormatter) {
    formatter.add_step("set");
    formatter.add_number("v_4", year);
}

/// `setupPROJGeodeticSourceCRS`: the source CRS as the pipeline reaches it
/// from geodetic coordinates, written in reverse.
fn setup_source(
    crs: &Crs,
    push_v3: PushV3,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    if let Some(geographic) = extract_geographic(crs) {
        formatter.start_inversion();
        geographic_export(geographic, formatter)?;
        formatter.stop_inversion();
        if push_v3 == PushV3::Yes {
            formatter.add_step("push");
            formatter.add_flag("v_3");
        }
        formatter.add_step("cart");
        ellipsoid_steps(ellipsoid_of(geographic), formatter);
        return Ok(());
    }
    if let Some((derived, base)) = derived_geographic(crs) {
        if let Some(pole) = super::projstring::derived_pole_step(derived) {
            formatter.start_inversion();
            formatter.ingest_proj_string(&pole);
            angular_unit_and_axis_swap(&derived_axes(derived), formatter)?;
            formatter.stop_inversion();
            if push_v3 == PushV3::Yes {
                formatter.add_step("push");
                formatter.add_flag("v_3");
            }
            formatter.add_step("cart");
            ellipsoid_steps(ellipsoid_of(base), formatter);
            return Ok(());
        }
    }
    let Some(geodetic) = geodetic_direct(crs) else {
        return Err(not_geodetic_or_geographic("Helmert"));
    };
    formatter.start_inversion();
    geocentric_unit(geodetic, formatter)?;
    formatter.stop_inversion();
    Ok(())
}

/// `setupPROJGeodeticTargetCRS`: the target CRS as the pipeline leaves it for
/// geodetic coordinates.
fn setup_target(
    crs: &Crs,
    push_v3: PushV3,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    if let Some(geographic) = extract_geographic(crs) {
        formatter.add_step("cart");
        formatter.invert_last_step();
        ellipsoid_steps(ellipsoid_of(geographic), formatter);
        if push_v3 == PushV3::Yes {
            formatter.add_step("pop");
            formatter.add_flag("v_3");
        }
        return geographic_export(geographic, formatter);
    }
    if let Some((derived, base)) = derived_geographic(crs) {
        if let Some(pole) = super::projstring::derived_pole_step(derived) {
            formatter.add_step("cart");
            formatter.invert_last_step();
            ellipsoid_steps(ellipsoid_of(base), formatter);
            if push_v3 == PushV3::Yes {
                formatter.add_step("pop");
                formatter.add_flag("v_3");
            }
            formatter.ingest_proj_string(&pole);
            return angular_unit_and_axis_swap(&derived_axes(derived), formatter);
        }
    }
    let Some(geodetic) = geodetic_direct(crs) else {
        return Err(not_geodetic_or_geographic("Helmert"));
    };
    geocentric_unit(geodetic, formatter)
}

/// A derived geographic CRS and its geographic base.
fn derived_geographic(crs: &Crs) -> Option<(&super::niche::DerivedCRS, &GeographicCRS)> {
    let Crs::Derived(derived) = crs else {
        return None;
    };
    if derived.kind() != super::niche::DerivedKind::Geographic {
        return None;
    }
    geographic_of(derived.base_crs()).map(|base| (derived, base))
}

fn derived_axes(derived: &super::niche::DerivedCRS) -> Vec<Axis> {
    match derived.coordinate_system() {
        CoordinateSystem::Ellipsoidal(system) => system.axes().to_vec(),
        _ => Vec::new(),
    }
}

fn not_geodetic_or_geographic(method: &str) -> FactoryError {
    FactoryError::Format(format!(
        "Can apply {method} only to GeodeticCRS / GeographicCRS"
    ))
}

/// A geodetic CRS as PROJ's `dynamic_cast<GeodeticCRS>` sees it.
#[derive(Clone, Copy)]
enum Geodetic<'a> {
    Geographic(&'a GeographicCRS),
    Geocentric(&'a GeocentricCRS),
    Spherical(&'a SphericalGeodeticCRS),
}

/// The geodetic CRS of `crs`, or of its first component when `crs` is compound.
fn geodetic_front(crs: &Crs) -> Option<Geodetic<'_>> {
    match crs {
        Crs::Compound(compound) => compound.components().first().and_then(geodetic_direct),
        other => geodetic_direct(other),
    }
}

fn geodetic_direct(crs: &Crs) -> Option<Geodetic<'_>> {
    match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            Some(Geodetic::Geographic(geographic))
        }
        Crs::Geodetic(GeodeticCRS::Geocentric(geocentric)) => {
            Some(Geodetic::Geocentric(geocentric))
        }
        Crs::Geodetic(GeodeticCRS::Spherical(spherical)) => Some(Geodetic::Spherical(spherical)),
        _ => None,
    }
}

/// A geographic CRS, or the horizontal one of a compound or bound CRS, as
/// `extractGeographicCRSIfGeographicCRSOrEquivalent` finds it.
fn extract_geographic(crs: &Crs) -> Option<&GeographicCRS> {
    match crs {
        Crs::Compound(compound) => {
            let first = compound.components().first()?;
            match first {
                Crs::Bound(bound) => geographic_of(bound.source_crs()),
                other => geographic_of(other),
            }
        }
        Crs::Bound(bound) => geographic_of(bound.source_crs()),
        other => geographic_of(other),
    }
}

fn ellipsoid_steps(ellipsoid: Option<&Ellipsoid>, formatter: &mut PipelineFormatter) {
    if let Some(ellipsoid) = ellipsoid {
        for (key, value) in ellipsoid_params(ellipsoid) {
            formatter.add_param(key, &value);
        }
    }
}

/// `GeographicCRS::_exportToPROJString` outside a CRS: `longlat` with the
/// datum, then the angular unit conversion and the axis order.
fn geographic_export(
    crs: &GeographicCRS,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    formatter.add_step("longlat");
    ellipsoid_steps(ellipsoid_of(crs), formatter);
    if let Some(meridian) = meridian_of(crs) {
        if let Some((key, value)) = prime_meridian_params(meridian) {
            formatter.add_param(key, &value);
        }
    }
    angular_unit_and_axis_swap(crs.coordinate_system().axes(), formatter)
}

/// `GeodeticCRS::addAngularUnitConvertAndAxisSwap`: radians to the CRS angular
/// unit, the height unit when there is one, then the axis order.
pub(super) fn angular_unit_and_axis_swap(
    axes: &[Axis],
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let Some(first) = axes.first() else {
        return Err(FactoryError::Format(
            "a geographic CRS has no axis".to_owned(),
        ));
    };
    formatter.add_step("unitconvert");
    formatter.add_param("xy_in", "rad");
    let with_z = axes.len() == 3 && !formatter.omits_z_unit_conversion();
    if with_z {
        formatter.add_param("z_in", "m");
    }
    formatter.add_param("xy_out", &unit_text(first.unit()));
    if let (true, Some(height)) = (with_z, axes.get(2)) {
        formatter.add_param("z_out", &unit_text(height.unit()));
    }
    axis_swap(axes, formatter);
    Ok(())
}

/// `Longitude rotation`: a `longlat` step on the source ellipsoid whose prime
/// meridian is the longitude offset. The offset is written inverted, as PROJ
/// does, unless its negation is a well-known prime meridian.
fn longitude_rotation(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let method = operation.method().name();
    let not_geographic =
        || FactoryError::Format(format!("Can apply {method} only to GeographicCRS"));
    let source = source
        .and_then(extract_geographic)
        .ok_or_else(not_geographic)?;
    let target = target
        .and_then(extract_geographic)
        .ok_or_else(not_geographic)?;
    let (Some(source_ellipsoid), Some(target_ellipsoid)) =
        (ellipsoid_of(source), ellipsoid_of(target))
    else {
        return Err(FactoryError::Format(format!(
            "Can apply {method} only to SRS with same ellipsoid"
        )));
    };
    if !ellipsoids_equivalent(source_ellipsoid, target_ellipsoid) {
        return Err(FactoryError::Format(format!(
            "Can apply {method} only to SRS with same ellipsoid"
        )));
    }
    let offset = numeric(operation, 8602, UnitOfMeasure::degree().conversion_to_si());

    formatter.start_inversion();
    angular_unit_and_axis_swap(source.coordinate_system().axes(), formatter)?;
    formatter.stop_inversion();

    let named = (offset != 0.0)
        .then(|| well_known_pm_name((-offset).to_radians()))
        .flatten();
    if let Some(name) = named {
        formatter.add_step("longlat");
        ellipsoid_steps(Some(source_ellipsoid), formatter);
        formatter.add_param("pm", name);
    } else {
        formatter.start_inversion();
        formatter.add_step("longlat");
        ellipsoid_steps(Some(source_ellipsoid), formatter);
        if let Some((key, value)) = prime_meridian_radians_params(offset.to_radians()) {
            formatter.add_param(key, &value);
        }
        formatter.stop_inversion();
    }

    angular_unit_and_axis_swap(target.coordinate_system().axes(), formatter)
}

/// `Ellipsoid::_isEquivalentTo` with the equivalent criterion: the same
/// semi-major axis and inverse flattening, names aside.
fn ellipsoids_equivalent(left: &Ellipsoid, right: &Ellipsoid) -> bool {
    let (a, b) = (
        left.semi_major_axis().si_value(),
        right.semi_major_axis().si_value(),
    );
    let (f, g) = (left.inverse_flattening(), right.inverse_flattening());
    (a - b).abs() <= 1e-10 * a.abs() && (f - g).abs() <= 1e-10 * f.abs()
}

/// `GeodeticCRS::addAxisSwap`: `order` when the first two axes are not east, north.
fn axis_swap(axes: &[Axis], formatter: &mut PipelineFormatter) {
    if axes.len() < 2 {
        return;
    }
    let (Some(first), Some(second)) = (axis_order(&axes[0]), axis_order(&axes[1])) else {
        return;
    };
    if first != "1" || second != "2" {
        formatter.add_step("axisswap");
        formatter.add_param("order", &format!("{first},{second}"));
    }
}

fn axis_order(axis: &Axis) -> Option<&'static str> {
    match axis.direction() {
        "west" => Some("-1"),
        "east" => Some("1"),
        "south" => Some("-2"),
        "north" => Some("2"),
        _ => None,
    }
}

/// `GeodeticCRS::_exportToPROJString` outside a CRS, for a geodetic endpoint of
/// a geographic/geocentric conversion.
fn geodetic_export(
    geodetic: Geodetic<'_>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    match geodetic {
        Geodetic::Geographic(geographic) => geographic_export(geographic, formatter),
        Geodetic::Geocentric(geocentric) => {
            formatter.add_step("cart");
            if let Some(frame) = geocentric_frame(geocentric) {
                ellipsoid_steps(Some(frame.ellipsoid()), formatter);
                if let Some((key, value)) = prime_meridian_params(frame.prime_meridian()) {
                    formatter.add_param(key, &value);
                }
            }
            geocentric_unit_steps(geocentric.coordinate_system().axes(), formatter)
        }
        Geodetic::Spherical(spherical) => {
            formatter.add_step("geoc");
            if let Some(frame) = spherical_frame(spherical) {
                ellipsoid_steps(Some(frame.ellipsoid()), formatter);
                if let Some((key, value)) = prime_meridian_params(frame.prime_meridian()) {
                    formatter.add_param(key, &value);
                }
            }
            angular_unit_and_axis_swap(spherical.coordinate_system.axes(), formatter)
        }
    }
}

/// `addGeocentricUnitConversionIntoPROJString` for a geodetic endpoint.
fn geocentric_unit(
    geodetic: Geodetic<'_>,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    match geodetic {
        Geodetic::Geocentric(geocentric) => {
            geocentric_unit_steps(geocentric.coordinate_system().axes(), formatter)
        }
        Geodetic::Geographic(_) | Geodetic::Spherical(_) => {
            Err(not_geodetic_or_geographic("Helmert"))
        }
    }
}

fn geocentric_unit_steps(
    axes: &[Axis],
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let Some(axis) = axes.first() else {
        return Err(FactoryError::Format(
            "a geocentric CRS has no axis".to_owned(),
        ));
    };
    let unit = axis.unit();
    if units_equivalent(unit, &UnitOfMeasure::metre()) {
        return Ok(());
    }
    formatter.add_step("unitconvert");
    formatter.add_param("xy_in", "m");
    formatter.add_param("z_in", "m");
    let text = unit_text(unit);
    formatter.add_param("xy_out", &text);
    formatter.add_param("z_out", &text);
    Ok(())
}

fn spherical_frame(crs: &SphericalGeodeticCRS) -> Option<&GeodeticReferenceFrame> {
    crs.datum
        .as_ref()
        .or_else(|| crs.ensemble.as_ref().and_then(geodetic_member))
}

fn geocentric_frame(crs: &GeocentricCRS) -> Option<&GeodeticReferenceFrame> {
    crs.datum()
        .or_else(|| crs.ensemble.as_ref().and_then(geodetic_member))
}

fn export_operation_steps(
    operation: &CoordinateOperation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_steps(formatter)
        }
        CoordinateOperation::Concatenated(concatenated) => concatenated.export_steps(formatter),
        CoordinateOperation::Conversion(conversion) => conversion.export_steps(formatter),
    }
}

/// The interpolation CRS of a proj-based operation, as PROJ's
/// `MyPROJStringExportableHorizVertical(HorizVerticalHoriz)PROJBased` writes it: a geographic
/// one with `addAngularUnitConvertAndAxisSwap`, else a projected one with
/// `addUnitConvertAndAxisSwap` (coordinateoperationfactory.cpp:2533-2536, 2547-2550).
fn interpolation_axes(
    interpolation: &Crs,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    if let Some(geographic) = extract_geographic(interpolation) {
        return angular_unit_and_axis_swap(geographic.coordinate_system().axes(), formatter);
    }
    if let Crs::Projected(projected) = interpolation {
        projected_unit_and_axis_swap(projected, formatter);
    }
    Ok(())
}

/// `MyPROJStringExportableHorizVertical` and `MyPROJStringExportableHorizVerticalHorizPROJBased`.
fn export_proj_based(
    proj_based: &ProjBased,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    match proj_based.kind {
        ProjBasedKind::HorizVertical => {
            export_operation_steps(&proj_based.horizontal, formatter)?;
            formatter.start_inversion();
            interpolation_axes(&proj_based.interpolation, formatter)?;
            formatter.stop_inversion();
            formatter.push_omit_horizontal_in_vertical();
            export_operation_steps(&proj_based.vertical, formatter)?;
            formatter.pop_omit_horizontal_in_vertical();
            formatter.push_omit_z_unit_conversion();
            interpolation_axes(&proj_based.interpolation, formatter)?;
            formatter.pop_omit_z_unit_conversion();
            Ok(())
        }
        ProjBasedKind::HorizNullVertical => {
            export_operation_steps(&proj_based.horizontal, formatter)
        }
        ProjBasedKind::HorizVerticalHoriz => {
            let save_horizontal = saves_horizontal_coordinates(proj_based);
            if save_horizontal {
                formatter.add_step("push");
                formatter.add_flag("v_1");
                formatter.add_flag("v_2");
            }
            formatter.push_omit_z_unit_conversion();
            export_operation_steps(&proj_based.horizontal, formatter)?;
            formatter.start_inversion();
            interpolation_axes(&proj_based.interpolation, formatter)?;
            formatter.stop_inversion();
            formatter.pop_omit_z_unit_conversion();
            formatter.push_omit_horizontal_in_vertical();
            export_operation_steps(&proj_based.vertical, formatter)?;
            formatter.pop_omit_horizontal_in_vertical();
            formatter.push_omit_z_unit_conversion();
            interpolation_axes(&proj_based.interpolation, formatter)?;
            if let Some(last) = &proj_based.last {
                export_operation_steps(last, formatter)?;
            }
            formatter.pop_omit_z_unit_conversion();
            if save_horizontal {
                formatter.add_step("pop");
                formatter.add_flag("v_1");
                formatter.add_flag("v_2");
            }
            Ok(())
        }
    }
}

/// The horizontal operation is a Helmert-type transformation that returns to the source datum:
/// the horizontal coordinates are saved around the vertical step.
fn saves_horizontal_coordinates(proj_based: &ProjBased) -> bool {
    let CoordinateOperation::Transformation(transformation) = &proj_based.horizontal else {
        return false;
    };
    let Some(last) = &proj_based.last else {
        return false;
    };
    let (Some(source), Some(target)) = (
        transformation.source_crs(),
        super::bound_ops::operation_target(last),
    ) else {
        return false;
    };
    let (Some(source), Some(target)) = (extract_geographic(source), extract_geographic(target))
    else {
        return false;
    };
    if !source.is_equivalent_to_with(&target.demote_to_2d(None), Equivalence::Equivalent) {
        return false;
    }
    let code = {
        let method = transformation.method();
        let code = if method.code_space() == "EPSG" {
            method.code().parse().unwrap_or(0)
        } else {
            0
        };
        if code == 0 {
            epsg_method_code(&transformation.inverse())
        } else {
            code
        }
    };
    let translation_is_null = [8605, 8606, 8607]
        .iter()
        .all(|id| si_value(transformation, *id) == 0.0);
    let geocentric_translation = matches!(code, 1031 | 9603 | 1035);
    (geocentric_translation && !translation_is_null)
        || matches!(
            code,
            1032 | 1132
                | 9607
                | 1133
                | 1140
                | 1038
                | 1149
                | 1033
                | 9606
                | 1037
                | 1034
                | 1056
                | 1057
                | 1053
                | 1054
                | 1055
                | 1065
                | 1066
                | 1067
        )
}
