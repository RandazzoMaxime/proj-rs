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
//! on projected CRS (1138) and geographic 2D (1145). The grid methods are written
//! only by [`CoordinateOperation::export_to_proj_string_with_database`], the export
//! `projinfo` uses; the `cs2cs` selection still sees them as unimplemented. Any
//! other method is `Unimplemented`, the error PROJ raises for a method its
//! exporter lacks.
//!
//! Coordinate epochs are not stored by this crate, so PROJ's `set` steps for a
//! source or target epoch are never written.

use std::f64::consts::PI;

use super::buildop::names_match;
use super::equiv::Equivalence;
use super::factory::DatabaseContext;
use super::model::{
    Axis, ConcatenatedOperation, CoordinateOperation, CoordinateSystem, Crs, Ellipsoid,
    FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Measure,
    ParameterValue, ProjectedCRS, Transformation, UnitKind, UnitOfMeasure, VerticalCRS,
};
use super::pipeline::{proj_number, PipelineFormatter, ProjLines};
use super::projstring::{
    ellipsoid_of, ellipsoid_params, geodetic_member, geographic_of, meridian_of,
    prime_meridian_params, prime_meridian_radians_params, proj_unit_name, projected_axis_order,
    well_known_pm_name,
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
const METHOD_CARTESIAN_GRID_OFFSETS: i32 = 9656;
const METHOD_LONGITUDE_ROTATION: i32 = 9601;
const METHOD_GEOGRAPHIC2D_OFFSETS: i32 = 9619;
const METHOD_GEOGRAPHIC3D_OFFSETS: i32 = 9660;
const METHOD_GEOCENTRIC_GRID_IGN: i32 = 1087;
const METHOD_NTV1: i32 = 9614;
const METHOD_NTV2: i32 = 9615;
const METHOD_VERTCON: i32 = 9658;
const METHOD_NRCAN_BYN: i32 = 1112;
const METHOD_NRCAN_GEOID_DIFFERENCE: i32 = 1126;
const METHOD_NEW_ZEALAND_DEFORMATION_MODEL: i32 = 1079;
const METHOD_TIN_CARTESIAN: i32 = 1138;
const METHOD_TIN_GEOGRAPHIC2D: i32 = 1145;
const PARAMETER_LAT_LON_DIFFERENCE_FILE: i32 = 8656;
const PARAMETER_GEOID_CORRECTION_FILENAME: i32 = 8666;
const PARAMETER_VERTICAL_OFFSET_FILE: i32 = 8732;
const PARAMETER_GEOID_MODEL_DIFFERENCE_FILE: i32 = 1063;
const PARAMETER_POINT_MOTION_VELOCITY_GRID_FILE: i32 = 1050;
const PARAMETER_TIN_OFFSET_FILE: i32 = 1064;

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
        self.export_to_proj_string_with(lines, Vec::new())
    }

    /// As [`Self::export_to_proj_string`], each grid written under the PROJ name
    /// `grid_names` gives it.
    pub(crate) fn export_to_proj_string_with(
        &self,
        lines: ProjLines,
        grid_names: Vec<(String, String)>,
    ) -> Result<String, FactoryError> {
        let mut formatter = PipelineFormatter::with_grid_names(grid_names);
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
            return Err(FactoryError::Format(format!(
                "Unimplemented {}",
                self.name()
            )));
        }
        if self.inverted {
            formatter.start_inversion();
            export_forward(self, self.target_crs(), self.source_crs(), formatter)?;
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
    /// The operation is a conversion, which needs its CRS pair, or a step cannot be written.
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
            Self::Conversion(_) => {
                return Err(FactoryError::Format(
                    "a conversion has no PROJ string without its CRS pair".to_owned(),
                ))
            }
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
    /// A step has no PROJ string in this crate: a conversion step, or a
    /// transformation method this crate does not write.
    pub fn export_to_proj_string(&self, lines: ProjLines) -> Result<String, FactoryError> {
        self.export_to_proj_string_with(lines, Vec::new())
    }

    /// As [`Self::export_to_proj_string`], each grid written under the PROJ name
    /// `grid_names` gives it.
    pub(crate) fn export_to_proj_string_with(
        &self,
        lines: ProjLines,
        grid_names: Vec<(String, String)>,
    ) -> Result<String, FactoryError> {
        let mut formatter = PipelineFormatter::with_grid_names(grid_names);
        self.export_steps(&mut formatter)?;
        Ok(formatter.finish(lines))
    }

    pub(crate) fn export_steps(
        &self,
        formatter: &mut PipelineFormatter,
    ) -> Result<(), FactoryError> {
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
                CoordinateOperation::Conversion(_) => {
                    return Err(FactoryError::Format(
                        "a conversion step of a concatenated operation has no PROJ string in this crate"
                            .to_owned(),
                    ));
                }
            }
            if step.reverse {
                formatter.stop_inversion();
            }
        }
        Ok(())
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
    if code == METHOD_AFFINE {
        return affine(operation, formatter);
    }
    if operation.method().name() == "3D Affine parametric transformation" {
        return affine_3d(operation, formatter);
    }
    if code == METHOD_SIMILARITY {
        return similarity(operation, formatter);
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
    if code == METHOD_GEOGRAPHIC2D_OFFSETS || code == METHOD_GEOGRAPHIC3D_OFFSETS {
        return null_geographic_offset(operation, formatter);
    }
    if code == METHOD_HEIGHT_DEPTH_REVERSAL {
        formatter.add_step("axisswap");
        formatter.add_param("order", "1,2,-3");
        return Ok(());
    }
    if code == METHOD_GEOCENTRIC_GRID_IGN {
        return geocentric_grid(operation, source, target, formatter);
    }
    if grid_shift(operation, source, target, formatter)? {
        return Ok(());
    }
    formatter.set_optimize_units();
    if let Some(kind) = helmert_kind(operation) {
        return helmert(operation, kind, source, target, formatter);
    }
    Err(unimplemented(operation))
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

/// `hgridshift` for NTv1 and NTv2, between the angular unit and axis steps of each geographic CRS.
fn horizontal_grid_shift(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let code = epsg_method_code(operation);
    if code != METHOD_NTV1 && code != METHOD_NTV2 {
        return Ok(false);
    }
    let Some(file) = file_by_code(operation, PARAMETER_LAT_LON_DIFFERENCE_FILE) else {
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
    formatter.start_inversion();
    angular_unit_and_axis_swap(source.coordinate_system().axes(), formatter)?;
    formatter.stop_inversion();
    formatter.add_step("hgridshift");
    formatter.add_param("grids", &grid);
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
    formatter.start_inversion();
    horizontal_unit_and_axis_swap(axes, formatter)?;
    formatter.stop_inversion();
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
    horizontal_unit_and_axis_swap(axes, formatter)?;
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
    let model = formatter.grid_name(&file);
    formatter.start_inversion();
    formatter.add_step("defmodel");
    formatter.add_param("model", &model);
    formatter.stop_inversion();
    angular_unit_and_axis_swap(target.coordinate_system().axes(), formatter)?;
    Ok(true)
}

/// `tinshift` on projected CRS (method 1138), or on geographic 2D CRS in degrees (method 1145).
fn tin_shift(
    operation: &Transformation,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let code = epsg_method_code(operation);
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
    if method.code_space() == "EPSG" {
        method.code().parse().unwrap_or(0)
    } else {
        0
    }
}

/// The measure of the parameter with EPSG code `code`, as PROJ's
/// `parameterValueMeasure(code)`. Absent or not a measure gives `None`.
fn measure_by_code(operation: &Transformation, code: i32) -> Option<&Measure> {
    operation.parameters().iter().find_map(|parameter| {
        if parameter.code() == code.to_string() {
            match parameter.value() {
                ParameterValue::Measure(measure) => Some(measure),
                _ => None,
            }
        } else {
            None
        }
    })
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

fn axis_order_reversal(
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
    let source_geocentric = matches!(source, Geodetic::Geocentric(_));
    let source_geographic = matches!(source, Geodetic::Geographic(_));
    let target_geocentric = matches!(target, Geodetic::Geocentric(_));
    let target_geographic = matches!(target, Geodetic::Geographic(_));
    if !((source_geocentric && target_geographic) || (source_geographic && target_geocentric)) {
        return Err(invalid_geographic_geocentric());
    }
    formatter.start_inversion();
    geodetic_export(source, formatter)?;
    formatter.stop_inversion();
    geodetic_export(target, formatter)
}

fn invalid_geographic_geocentric() -> FactoryError {
    FactoryError::Format(
        "Invalid nature of source and/or targetCRS for Geographic/Geocentric conversion".to_owned(),
    )
}

/// `Geographic2D offsets` and `Geographic3D offsets` with zero offsets, the null offset
/// PROJ writes as `noop`. Non-zero offsets are not ported.
fn null_geographic_offset(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    if [8601, 8602, 8603]
        .iter()
        .any(|&code| si_value(operation, code) != 0.0)
    {
        return Err(unimplemented(operation));
    }
    formatter.add_step("noop");
    Ok(())
}

fn change_vertical_unit(
    operation: &Transformation,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let factor = si_value(operation, 1051);
    let forward = proj_linear_name(factor);
    let reverse = (factor != 0.0)
        .then(|| proj_linear_name(1.0 / factor))
        .flatten();
    if forward == Some("m") {
        return Ok(());
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
    Ok(())
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
    if let Some(target) = target {
        setup_target(target, push_v3, formatter)?;
    }
    Ok(())
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
    let Some(geodetic) = geodetic_direct(crs) else {
        return Err(not_geodetic_or_geographic("Helmert"));
    };
    geocentric_unit(geodetic, formatter)
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
    Spherical,
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
        Crs::Geodetic(GeodeticCRS::Spherical(_)) => Some(Geodetic::Spherical),
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
fn angular_unit_and_axis_swap(
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
    if axes.len() == 3 {
        formatter.add_param("z_in", "m");
    }
    formatter.add_param("xy_out", &unit_text(first.unit()));
    if let (3, Some(height)) = (axes.len(), axes.get(2)) {
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
        Geodetic::Spherical => Err(FactoryError::Format(
            "a spherical CRS has no PROJ string in this crate".to_owned(),
        )),
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
        Geodetic::Geographic(_) | Geodetic::Spherical => Err(not_geodetic_or_geographic("Helmert")),
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

fn geocentric_frame(crs: &GeocentricCRS) -> Option<&GeodeticReferenceFrame> {
    crs.datum()
        .or_else(|| crs.ensemble.as_ref().and_then(geodetic_member))
}
