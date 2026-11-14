//! Deformation model master files (`+proj=defmodel`).
//!
//! [`MasterFile::parse`] follows `MasterFile::parse` in `defmodel_impl.hpp`.
//! [`TimeFunction::evaluate_at`] follows the `evaluateAt` methods of the same
//! file, and [`Evaluator`] follows `Evaluator::forward` and
//! `Evaluator::inverse`. The grids come from a [`GridProvider`]: the engine
//! supplies the GeoTIFF, NTv2 and CTable2 readers, a test supplies synthetic
//! grids, as `test_defmodel.cpp` does. Geographic input and output are radians.

use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::fmt;

use super::grid::Grid as EngineGrid;
use super::json;
use crate::engine::{Coord, CreateError, Frame, IoKind, Params, Step, TransformError};

const DEG_TO_RAD: f64 = PI / 180.0;

/// Error of a deformation model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefmodelError {
    /// The master file, or an ISO 8601 date, is not valid
    /// (`ParsingException` in PROJ).
    Parsing(String),
    /// The model cannot be evaluated: the CRS and the offset settings are
    /// incompatible, or a grid cannot be opened (`EvaluatorException` in PROJ).
    Evaluator(String),
}

impl fmt::Display for DefmodelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parsing(message) | Self::Evaluator(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DefmodelError {}

fn parsing<T>(message: impl Into<String>) -> Result<T, DefmodelError> {
    Err(DefmodelError::Parsing(message.into()))
}

fn eval_error<T>(message: impl Into<String>) -> Result<T, DefmodelError> {
    Err(DefmodelError::Evaluator(message.into()))
}

type Object = BTreeMap<String, json::Value>;

fn member<'a>(object: &'a Object, key: &str) -> Result<&'a json::Value, DefmodelError> {
    object
        .get(key)
        .ok_or_else(|| DefmodelError::Parsing(format!("Missing \"{key}\" key")))
}

fn req_string(object: &Object, key: &str) -> Result<String, DefmodelError> {
    member(object, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| DefmodelError::Parsing(format!("The value of \"{key}\" should be a string")))
}

/// An absent optional string is empty, as in PROJ.
fn opt_string(object: &Object, key: &str) -> Result<String, DefmodelError> {
    if object.contains_key(key) {
        req_string(object, key)
    } else {
        Ok(String::new())
    }
}

fn req_object<'a>(object: &'a Object, key: &str) -> Result<&'a Object, DefmodelError> {
    member(object, key)?
        .as_object()
        .ok_or_else(|| DefmodelError::Parsing(format!("The value of \"{key}\" should be a object")))
}

fn req_array<'a>(object: &'a Object, key: &str) -> Result<&'a [json::Value], DefmodelError> {
    member(object, key)?
        .as_array()
        .ok_or_else(|| DefmodelError::Parsing(format!("The value of \"{key}\" should be a array")))
}

fn req_number(object: &Object, key: &str) -> Result<f64, DefmodelError> {
    member(object, key)?
        .as_f64()
        .ok_or_else(|| DefmodelError::Parsing(format!("The value of \"{key}\" should be a number")))
}

/// An absent optional number is NaN, as in PROJ.
fn opt_number(object: &Object, key: &str) -> Result<f64, DefmodelError> {
    if object.contains_key(key) {
        req_number(object, key)
    } else {
        Ok(f64::NAN)
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` to a decimal year. Leap years are counted, leap
/// seconds are not. The year must be 1582 or later.
pub fn decimal_year_from_iso8601(text: &str) -> Result<f64, DefmodelError> {
    iso8601_to_decimal_year(text).ok_or_else(|| {
        DefmodelError::Parsing(format!("Wrong formatting / invalid date-time for {text}"))
    })
}

fn iso8601_to_decimal_year(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let field = |range: std::ops::Range<usize>| -> Option<i32> {
        std::str::from_utf8(&bytes[range]).ok()?.parse::<i32>().ok()
    };
    let year = field(0..4)?;
    let month = field(5..7)?;
    let day = field(8..10)?;
    let hour = field(11..13)?;
    let minute = field(14..16)?;
    let second = field(17..19)?;
    if year < 1582
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..61).contains(&second)
    {
        return None;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let table: [i32; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let month_index = usize::try_from(month - 1).ok()?;
    if day > table[month_index] {
        return None;
    }
    let day_in_year = day - 1 + table.iter().take(month_index).sum::<i32>();
    let seconds = f64::from(day_in_year) * 86_400.0
        + f64::from(hour) * 3_600.0
        + f64::from(minute) * 60.0
        + f64::from(second);
    let length = if leap {
        86_400.0 * 366.0
    } else {
        86_400.0 * 365.0
    };
    Some(f64::from(year) + seconds / length)
}

/// The bounding box of a spatial extent, in the units of the file (degrees
/// for a geographic CRS).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialExtent {
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
}

impl SpatialExtent {
    /// Western bound, as written in the file.
    pub fn minx(&self) -> f64 {
        self.minx
    }

    /// Southern bound, as written in the file.
    pub fn miny(&self) -> f64 {
        self.miny
    }

    /// Eastern bound, as written in the file.
    pub fn maxx(&self) -> f64 {
        self.maxx
    }

    /// Northern bound, as written in the file.
    pub fn maxy(&self) -> f64 {
        self.maxy
    }

    fn parse(object: &Object) -> Result<Self, DefmodelError> {
        if req_string(object, "type")? != "bbox" {
            return parsing("unsupported type of extent");
        }
        let bbox = req_array(req_object(object, "parameters")?, "bbox")?;
        if bbox.len() != 4 {
            return parsing("bbox is not an array of 4 numeric elements");
        }
        let mut values = [0.0; 4];
        for (slot, item) in values.iter_mut().zip(bbox) {
            *slot = item.as_f64().ok_or_else(|| {
                DefmodelError::Parsing("bbox is not an array of 4 numeric elements".into())
            })?;
        }
        Ok(Self {
            minx: values[0],
            miny: values[1],
            maxx: values[2],
            maxy: values[3],
        })
    }

    /// The bounds in the units of the evaluator: radians for a geographic CRS.
    fn normalized(&self, geographic: bool) -> Bounds {
        let scale = if geographic { DEG_TO_RAD } else { 1.0 };
        Bounds {
            minx: self.minx * scale,
            miny: self.miny * scale,
            maxx: self.maxx * scale,
            maxy: self.maxy * scale,
        }
    }
}

/// An epoch written as `YYYY-MM-DDTHH:MM:SSZ`, with its decimal year. An empty
/// epoch has the decimal year 0, as in PROJ.
#[derive(Debug, Clone, PartialEq)]
pub struct Epoch {
    text: String,
    decimal_year: f64,
}

impl Epoch {
    fn parse(text: String) -> Result<Self, DefmodelError> {
        let decimal_year = if text.is_empty() {
            0.0
        } else {
            decimal_year_from_iso8601(&text)?
        };
        Ok(Self { text, decimal_year })
    }

    /// The epoch as written in the file.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The epoch as a decimal year.
    pub fn decimal_year(&self) -> f64 {
        self.decimal_year
    }
}

/// The first and the last epoch covered by a model.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeExtent {
    first: Epoch,
    last: Epoch,
}

impl TimeExtent {
    /// The first epoch covered.
    pub fn first(&self) -> &Epoch {
        &self.first
    }

    /// The last epoch covered.
    pub fn last(&self) -> &Epoch {
        &self.last
    }
}

/// Behaviour of a piecewise time function before its first or after its last
/// epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// The factor is 0.
    Zero,
    /// The factor is the first (or last) scale factor.
    Constant,
    /// The factor is extrapolated along the first (or last) segment.
    Linear,
}

impl Bound {
    /// The keyword of the master file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::Constant => "constant",
            Self::Linear => "linear",
        }
    }

    fn parse(text: &str, message: &str) -> Result<Self, DefmodelError> {
        match text {
            "zero" => Ok(Self::Zero),
            "constant" => Ok(Self::Constant),
            "linear" => Ok(Self::Linear),
            _ => parsing(message),
        }
    }
}

/// One `(epoch, scale_factor)` row of a piecewise time function.
#[derive(Debug, Clone, PartialEq)]
pub struct EpochScaleFactor {
    epoch: Epoch,
    scale_factor: f64,
}

impl EpochScaleFactor {
    /// The epoch of the row.
    pub fn epoch(&self) -> &Epoch {
        &self.epoch
    }

    /// The scale factor at the epoch.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }
}

/// The time dependency of a component. The variants mirror the `type` keyword
/// of the master file.
#[derive(Debug, Clone, PartialEq)]
pub enum TimeFunction {
    /// Factor 1 at all times.
    Constant,
    /// Factor `t - reference_epoch`, in years.
    Velocity {
        /// The epoch where the factor is 0.
        reference_epoch: Epoch,
    },
    /// Factor 0 before `step_epoch`, 1 after.
    Step {
        /// The epoch of the step.
        step_epoch: Epoch,
    },
    /// Factor -1 before `step_epoch`, 0 after.
    ReverseStep {
        /// The epoch of the step.
        step_epoch: Epoch,
    },
    /// Linear interpolation between the rows of `model`.
    Piecewise {
        /// Behaviour before the first row.
        before_first: Bound,
        /// Behaviour after the last row.
        after_last: Bound,
        /// The rows, in the order of the file.
        model: Vec<EpochScaleFactor>,
    },
    /// Relaxation towards `final_scale_factor`.
    Exponential {
        /// The epoch where the relaxation starts.
        reference_epoch: Epoch,
        /// The epoch where the relaxation stops, when the file gives one.
        end_epoch: Option<Epoch>,
        /// The relaxation constant, in years. Positive.
        relaxation_constant: f64,
        /// The factor before `reference_epoch`.
        before_scale_factor: f64,
        /// The factor at `reference_epoch`.
        initial_scale_factor: f64,
        /// The factor reached after the relaxation.
        final_scale_factor: f64,
    },
}

impl TimeFunction {
    /// The `type` keyword of the master file.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Constant => "constant",
            Self::Velocity { .. } => "velocity",
            Self::Step { .. } => "step",
            Self::ReverseStep { .. } => "reverse_step",
            Self::Piecewise { .. } => "piecewise",
            Self::Exponential { .. } => "exponential",
        }
    }

    /// The factor of the component at the decimal year `decimal_year`.
    pub fn evaluate_at(&self, decimal_year: f64) -> f64 {
        match self {
            Self::Constant => 1.0,
            Self::Velocity { reference_epoch } => decimal_year - reference_epoch.decimal_year,
            Self::Step { step_epoch } => {
                if decimal_year < step_epoch.decimal_year {
                    0.0
                } else {
                    1.0
                }
            }
            Self::ReverseStep { step_epoch } => {
                if decimal_year < step_epoch.decimal_year {
                    -1.0
                } else {
                    0.0
                }
            }
            Self::Piecewise {
                before_first,
                after_last,
                model,
            } => piecewise(*before_first, *after_last, model, decimal_year),
            Self::Exponential {
                reference_epoch,
                end_epoch,
                relaxation_constant,
                before_scale_factor,
                initial_scale_factor,
                final_scale_factor,
            } => {
                let t0 = reference_epoch.decimal_year;
                if decimal_year < t0 {
                    return *before_scale_factor;
                }
                let t = match end_epoch {
                    Some(end) => decimal_year.min(end.decimal_year),
                    None => decimal_year,
                };
                initial_scale_factor
                    + (final_scale_factor - initial_scale_factor)
                        * (1.0 - (-(t - t0) / relaxation_constant).exp())
            }
        }
    }

    fn parse(object: &Object) -> Result<Self, DefmodelError> {
        let kind = req_string(object, "type")?;
        if kind == "constant" {
            return Ok(Self::Constant);
        }
        let parameters = req_object(object, "parameters")?;
        match kind.as_str() {
            "velocity" => Ok(Self::Velocity {
                reference_epoch: Epoch::parse(req_string(parameters, "reference_epoch")?)?,
            }),
            "step" => Ok(Self::Step {
                step_epoch: Epoch::parse(req_string(parameters, "step_epoch")?)?,
            }),
            "reverse_step" => Ok(Self::ReverseStep {
                step_epoch: Epoch::parse(req_string(parameters, "step_epoch")?)?,
            }),
            "piecewise" => {
                let before_first = Bound::parse(
                    &req_string(parameters, "before_first")?,
                    "Unsupported value for before_first",
                )?;
                let after_last = Bound::parse(
                    &req_string(parameters, "after_last")?,
                    "Unsupported value for afterLast",
                )?;
                let mut model = Vec::new();
                for item in req_array(parameters, "model")? {
                    let row = item.as_object().ok_or_else(|| {
                        DefmodelError::Parsing("model[] element is not an object".into())
                    })?;
                    model.push(EpochScaleFactor {
                        epoch: Epoch::parse(req_string(row, "epoch")?)?,
                        scale_factor: req_number(row, "scale_factor")?,
                    });
                }
                Ok(Self::Piecewise {
                    before_first,
                    after_last,
                    model,
                })
            }
            "exponential" => {
                let reference_epoch = Epoch::parse(req_string(parameters, "reference_epoch")?)?;
                let end_text = opt_string(parameters, "end_epoch")?;
                let end_epoch = if end_text.is_empty() {
                    None
                } else {
                    Some(Epoch::parse(end_text)?)
                };
                let relaxation_constant = req_number(parameters, "relaxation_constant")?;
                if relaxation_constant <= 0.0 {
                    return parsing("Invalid value for relaxation_constant");
                }
                Ok(Self::Exponential {
                    reference_epoch,
                    end_epoch,
                    relaxation_constant,
                    before_scale_factor: req_number(parameters, "before_scale_factor")?,
                    initial_scale_factor: req_number(parameters, "initial_scale_factor")?,
                    final_scale_factor: req_number(parameters, "final_scale_factor")?,
                })
            }
            other => parsing(format!("Unsupported type of time function: {other}")),
        }
    }
}

/// Piecewise linear interpolation over the rows of a model.
fn piecewise(before: Bound, after: Bound, model: &[EpochScaleFactor], time: f64) -> f64 {
    let row = |index: usize| (model[index].epoch.decimal_year, model[index].scale_factor);
    if model.is_empty() {
        return 0.0;
    }
    let (t1, f1) = row(0);
    if time < t1 {
        return match before {
            Bound::Zero => 0.0,
            Bound::Constant => f1,
            Bound::Linear => {
                if model.len() == 1 {
                    return f1;
                }
                let (t2, f2) = row(1);
                if t1 == t2 {
                    f1
                } else {
                    (f1 * (t2 - time) + f2 * (time - t1)) / (t2 - t1)
                }
            }
        };
    }
    for index in 1..model.len() {
        let (t_next, f_next) = row(index);
        if time < t_next {
            let (t_prev, f_prev) = row(index - 1);
            return (f_prev * (t_next - time) + f_next * (time - t_prev)) / (t_next - t_prev);
        }
    }
    let (t_last, f_last) = row(model.len() - 1);
    match after {
        Bound::Zero => 0.0,
        Bound::Constant => f_last,
        Bound::Linear => {
            if model.len() == 1 {
                return f_last;
            }
            let (t_prev, f_prev) = row(model.len() - 2);
            if t_prev == t_last {
                f_last
            } else {
                (f_prev * (t_last - time) + f_last * (time - t_prev)) / (t_last - t_prev)
            }
        }
    }
}

/// The kind of displacement a component applies. `None` components are read
/// and then ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Displacement {
    /// The component is ignored.
    None,
    /// Horizontal displacement only.
    Horizontal,
    /// Vertical displacement only.
    Vertical,
    /// Horizontal and vertical displacement.
    ThreeD,
}

impl Displacement {
    /// The keyword of the master file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::ThreeD => "3d",
        }
    }

    fn parse(text: &str) -> Result<Self, DefmodelError> {
        match text {
            "none" => Ok(Self::None),
            "horizontal" => Ok(Self::Horizontal),
            "vertical" => Ok(Self::Vertical),
            "3d" => Ok(Self::ThreeD),
            _ => parsing("Unsupported value for displacement_type"),
        }
    }
}

/// The grid file of a component and how it is interpolated.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialModel {
    model_type: String,
    interpolation_method: String,
    filename: String,
    md5_checksum: String,
}

impl SpatialModel {
    /// The `type` keyword, for example `GeoTIFF`.
    pub fn model_type(&self) -> &str {
        &self.model_type
    }

    /// `bilinear` or `geocentric_bilinear`.
    pub fn interpolation_method(&self) -> &str {
        &self.interpolation_method
    }

    /// The grid file, as named in the master file.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// The MD5 checksum of the grid file, empty when absent.
    pub fn md5_checksum(&self) -> &str {
        &self.md5_checksum
    }

    fn parse(object: &Object) -> Result<Self, DefmodelError> {
        let model_type = req_string(object, "type")?;
        let interpolation_method = req_string(object, "interpolation_method")?;
        if interpolation_method != "bilinear" && interpolation_method != "geocentric_bilinear" {
            return parsing("Unsupported value for interpolation_method");
        }
        Ok(Self {
            model_type,
            interpolation_method,
            filename: req_string(object, "filename")?,
            md5_checksum: opt_string(object, "md5_checksum")?,
        })
    }
}

/// One component of a deformation model.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    description: String,
    displacement: Displacement,
    uncertainty_type: String,
    horizontal_uncertainty: f64,
    vertical_uncertainty: f64,
    extent: SpatialExtent,
    spatial_model: SpatialModel,
    time_function: TimeFunction,
}

impl Component {
    /// The description, empty when absent.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The displacement kind.
    pub fn displacement(&self) -> Displacement {
        self.displacement
    }

    /// The `displacement_type` keyword.
    pub fn displacement_type(&self) -> &'static str {
        self.displacement.as_str()
    }

    /// The uncertainty type, as written.
    pub fn uncertainty_type(&self) -> &str {
        &self.uncertainty_type
    }

    /// The horizontal uncertainty, NaN when absent.
    pub fn horizontal_uncertainty(&self) -> f64 {
        self.horizontal_uncertainty
    }

    /// The vertical uncertainty, NaN when absent.
    pub fn vertical_uncertainty(&self) -> f64 {
        self.vertical_uncertainty
    }

    /// The area where the component applies.
    pub fn extent(&self) -> &SpatialExtent {
        &self.extent
    }

    /// The grid file of the component.
    pub fn spatial_model(&self) -> &SpatialModel {
        &self.spatial_model
    }

    /// The time dependency of the component.
    pub fn time_function(&self) -> &TimeFunction {
        &self.time_function
    }

    fn parse(value: &json::Value) -> Result<Self, DefmodelError> {
        let object = value
            .as_object()
            .ok_or_else(|| DefmodelError::Parsing("component is not an object".into()))?;
        let description = opt_string(object, "description")?;
        let extent = SpatialExtent::parse(req_object(object, "extent")?)?;
        let displacement = Displacement::parse(&req_string(object, "displacement_type")?)?;
        let uncertainty_type = req_string(object, "uncertainty_type")?;
        let horizontal_uncertainty = opt_number(object, "horizontal_uncertainty")?;
        let vertical_uncertainty = opt_number(object, "vertical_uncertainty")?;
        let spatial_model = SpatialModel::parse(req_object(object, "spatial_model")?)?;
        let time_function = TimeFunction::parse(req_object(object, "time_function")?)?;
        Ok(Self {
            description,
            displacement,
            uncertainty_type,
            horizontal_uncertainty,
            vertical_uncertainty,
            extent,
            spatial_model,
            time_function,
        })
    }
}

/// An author or publisher of a master file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Authority {
    /// Name.
    pub name: String,
    /// URL.
    pub url: String,
    /// Postal address.
    pub address: String,
    /// E-mail address.
    pub email: String,
}

/// A link of a master file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Link {
    /// URL of the information.
    pub href: String,
    /// Relation of the link.
    pub rel: String,
    /// Media type.
    pub type_: String,
    /// Title.
    pub title: String,
}

/// A deformation model master file (`MasterFile` in PROJ).
#[derive(Debug, Clone, PartialEq)]
pub struct MasterFile {
    file_type: String,
    format_version: String,
    name: String,
    version: String,
    license: String,
    description: String,
    publication_date: String,
    authority: Authority,
    links: Vec<Link>,
    source_crs: String,
    target_crs: String,
    definition_crs: String,
    reference_epoch: String,
    uncertainty_reference_epoch: String,
    horizontal_offset_unit: String,
    vertical_offset_unit: String,
    horizontal_uncertainty_type: String,
    horizontal_uncertainty_unit: String,
    vertical_uncertainty_type: String,
    vertical_uncertainty_unit: String,
    horizontal_offset_method: String,
    extent: SpatialExtent,
    time_extent: TimeExtent,
    components: Vec<Component>,
}

macro_rules! text_accessors {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            pub fn $name(&self) -> &str {
                &self.$name
            }
        )*
    };
}

impl MasterFile {
    /// Parses a master file from its JSON text.
    ///
    /// The checks are those of PROJ: the required keys, the supported values
    /// of the units, the offset method and the interpolation, and the
    /// combinations that need a horizontal or a vertical unit.
    ///
    /// # Errors
    ///
    /// [`DefmodelError::Parsing`] on invalid JSON or on any rejected value.
    pub fn parse(text: &str) -> Result<Self, DefmodelError> {
        let root =
            json::parse(text).ok_or_else(|| DefmodelError::Parsing("Invalid JSON".into()))?;
        let object = root
            .as_object()
            .ok_or_else(|| DefmodelError::Parsing("Not an object".into()))?;
        let file_type = req_string(object, "file_type")?;
        let format_version = req_string(object, "format_version")?;
        let name = opt_string(object, "name")?;
        let version = opt_string(object, "version")?;
        let license = opt_string(object, "license")?;
        let description = opt_string(object, "description")?;
        let publication_date = opt_string(object, "publication_date")?;
        let authority = match object.get("authority") {
            None => Authority::default(),
            Some(value) => {
                let authority = value
                    .as_object()
                    .ok_or_else(|| DefmodelError::Parsing("authority is not a object".into()))?;
                Authority {
                    name: opt_string(authority, "name")?,
                    url: opt_string(authority, "url")?,
                    address: opt_string(authority, "address")?,
                    email: opt_string(authority, "email")?,
                }
            }
        };
        let mut links = Vec::new();
        if let Some(value) = object.get("links") {
            let items = value
                .as_array()
                .ok_or_else(|| DefmodelError::Parsing("links is not an array".into()))?;
            for item in items {
                let link = item.as_object().ok_or_else(|| {
                    DefmodelError::Parsing("links[] item is not an object".into())
                })?;
                links.push(Link {
                    href: opt_string(link, "href")?,
                    rel: opt_string(link, "rel")?,
                    type_: opt_string(link, "type")?,
                    title: opt_string(link, "title")?,
                });
            }
        }
        let source_crs = req_string(object, "source_crs")?;
        let target_crs = req_string(object, "target_crs")?;
        let definition_crs = req_string(object, "definition_crs")?;
        if source_crs != definition_crs {
            return parsing("source_crs != definition_crs not currently supported");
        }
        let reference_epoch = opt_string(object, "reference_epoch")?;
        let uncertainty_reference_epoch = opt_string(object, "uncertainty_reference_epoch")?;
        let horizontal_offset_unit = opt_string(object, "horizontal_offset_unit")?;
        if !horizontal_offset_unit.is_empty()
            && horizontal_offset_unit != "metre"
            && horizontal_offset_unit != "degree"
        {
            return parsing("Unsupported value for horizontal_offset_unit");
        }
        let vertical_offset_unit = opt_string(object, "vertical_offset_unit")?;
        if !vertical_offset_unit.is_empty() && vertical_offset_unit != "metre" {
            return parsing("Unsupported value for vertical_offset_unit");
        }
        let horizontal_uncertainty_type = opt_string(object, "horizontal_uncertainty_type")?;
        let horizontal_uncertainty_unit = opt_string(object, "horizontal_uncertainty_unit")?;
        let vertical_uncertainty_type = opt_string(object, "vertical_uncertainty_type")?;
        let vertical_uncertainty_unit = opt_string(object, "vertical_uncertainty_unit")?;
        let horizontal_offset_method = opt_string(object, "horizontal_offset_method")?;
        if !horizontal_offset_method.is_empty()
            && horizontal_offset_method != "addition"
            && horizontal_offset_method != "geocentric"
        {
            return parsing("Unsupported value for horizontal_offset_method");
        }
        let extent = SpatialExtent::parse(req_object(object, "extent")?)?;
        let time = req_object(object, "time_extent")?;
        let time_extent = TimeExtent {
            first: Epoch::parse(req_string(time, "first")?)?,
            last: Epoch::parse(req_string(time, "last")?)?,
        };
        let mut components = Vec::new();
        for item in req_array(object, "components")? {
            let component = Component::parse(item)?;
            let displacement = component.displacement;
            if matches!(
                displacement,
                Displacement::Horizontal | Displacement::ThreeD
            ) {
                if horizontal_offset_unit.is_empty() {
                    return parsing(
                        "horizontal_offset_unit should be defined as there is a component with displacement_type = horizontal/3d",
                    );
                }
                if horizontal_offset_method.is_empty() {
                    return parsing(
                        "horizontal_offset_method should be defined as there is a component with displacement_type = horizontal/3d",
                    );
                }
            }
            if matches!(displacement, Displacement::Vertical | Displacement::ThreeD)
                && vertical_offset_unit.is_empty()
            {
                return parsing(
                    "vertical_offset_unit should be defined as there is a component with displacement_type = vertical/3d",
                );
            }
            if horizontal_offset_unit == "degree"
                && component.spatial_model.interpolation_method != "bilinear"
            {
                return parsing(
                    "horizontal_offset_unit = degree can only be used with interpolation_method = bilinear",
                );
            }
            components.push(component);
        }
        if horizontal_offset_unit == "degree" && horizontal_offset_method != "addition" {
            return parsing(
                "horizontal_offset_unit = degree can only be used with horizontal_offset_method = addition",
            );
        }
        Ok(Self {
            file_type,
            format_version,
            name,
            version,
            license,
            description,
            publication_date,
            authority,
            links,
            source_crs,
            target_crs,
            definition_crs,
            reference_epoch,
            uncertainty_reference_epoch,
            horizontal_offset_unit,
            vertical_offset_unit,
            horizontal_uncertainty_type,
            horizontal_uncertainty_unit,
            vertical_uncertainty_type,
            vertical_uncertainty_unit,
            horizontal_offset_method,
            extent,
            time_extent,
            components,
        })
    }

    text_accessors!(
        /// The `file_type` keyword.
        file_type,
        /// The `format_version` keyword.
        format_version,
        /// The model name, empty when absent.
        name,
        /// The model version, empty when absent.
        version,
        /// The publication date, empty when absent.
        publication_date,
        /// The license, empty when absent.
        license,
        /// The description, empty when absent.
        description,
        /// The source CRS.
        source_crs,
        /// The target CRS.
        target_crs,
        /// The CRS in which the model is defined. It equals the source CRS.
        definition_crs,
        /// The reference epoch, empty when absent.
        reference_epoch,
        /// The reference epoch of the uncertainties, empty when absent.
        uncertainty_reference_epoch,
        /// `metre`, `degree`, or empty when absent.
        horizontal_offset_unit,
        /// `metre`, or empty when absent.
        vertical_offset_unit,
        /// The horizontal uncertainty type, empty when absent.
        horizontal_uncertainty_type,
        /// The horizontal uncertainty unit, empty when absent.
        horizontal_uncertainty_unit,
        /// The vertical uncertainty type, empty when absent.
        vertical_uncertainty_type,
        /// The vertical uncertainty unit, empty when absent.
        vertical_uncertainty_unit,
        /// `addition`, `geocentric`, or empty when absent.
        horizontal_offset_method,
    );

    /// The authority of the model.
    pub fn authority(&self) -> &Authority {
        &self.authority
    }

    /// The links of the model.
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// The area covered by the model.
    pub fn extent(&self) -> &SpatialExtent {
        &self.extent
    }

    /// The first and the last epoch covered by the model.
    pub fn time_extent(&self) -> &TimeExtent {
        &self.time_extent
    }

    /// The components, in the order of the file.
    pub fn components(&self) -> &[Component] {
        &self.components
    }
}

/// Node values of a grid, in the units of the file: radians for a geographic
/// CRS, metres for a projected one. Node indices count columns from the west
/// edge and rows from the south edge.
pub trait Grid {
    /// The west edge, in radians for a geographic grid.
    fn min_x(&self) -> f64;
    /// The south edge, in radians for a geographic grid.
    fn min_y(&self) -> f64;
    /// The cell width.
    fn res_x(&self) -> f64;
    /// The cell height.
    fn res_y(&self) -> f64;
    /// The number of nodes along x.
    fn width(&self) -> usize;
    /// The number of nodes along y.
    fn height(&self) -> usize;

    /// Longitude and latitude offsets in radians at a node. `None` when the
    /// grid has no such values.
    fn long_lat_offset(&self, _ix: usize, _iy: usize) -> Option<(f64, f64)> {
        None
    }

    /// Longitude and latitude offsets in radians, and the vertical offset, at
    /// a node.
    fn long_lat_z_offset(&self, _ix: usize, _iy: usize) -> Option<(f64, f64, f64)> {
        None
    }

    /// Easting and northing offsets in metres at a node.
    fn easting_northing_offset(&self, _ix: usize, _iy: usize) -> Option<(f64, f64)> {
        None
    }

    /// Easting and northing offsets in metres, and the vertical offset, at a
    /// node.
    fn easting_northing_z_offset(&self, _ix: usize, _iy: usize) -> Option<(f64, f64, f64)> {
        None
    }

    /// The vertical offset in metres at a node.
    fn z_offset(&self, _ix: usize, _iy: usize) -> Option<f64> {
        None
    }
}

/// The grids of one component, covering the points it applies to.
pub trait GridSet {
    /// The grid type.
    type Node: Grid;

    /// The grid that holds the point `(x, y)`, in the units of the evaluator.
    /// `None` when no grid holds the point.
    fn grid_at(&self, x: f64, y: f64) -> Option<&Self::Node>;
}

/// Opens the grid sets named by the components, and answers the CRS question.
pub trait GridProvider {
    /// The grid set type.
    type Set: GridSet;

    /// Opens the grid file `filename`. `None` when it cannot be opened.
    fn open(&self, filename: &str) -> Option<Self::Set>;

    /// Whether the CRS `definition` is geographic.
    fn is_geographic_crs(&self, definition: &str) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    Forward,
    Inverse,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
}

/// Bilinear weights of the four nodes of a cell.
#[derive(Debug, Clone, Copy)]
struct Weights {
    m00: f64,
    m10: f64,
    m01: f64,
    m11: f64,
}

/// The values at the four nodes of a cell, read in the order PROJ reads them:
/// `(ix0, iy0)`, `(ix1, iy0)`, `(ix0, iy1)`, `(ix1, iy1)`.
#[derive(Debug, Clone, Copy)]
struct Cell<T> {
    c00: T,
    c10: T,
    c01: T,
    c11: T,
}

impl<T> Cell<T> {
    fn read(
        (ix0, iy0, ix1, iy1): (usize, usize, usize, usize),
        mut value: impl FnMut(usize, usize) -> Option<T>,
    ) -> Option<Self> {
        Some(Self {
            c00: value(ix0, iy0)?,
            c10: value(ix1, iy0)?,
            c01: value(ix0, iy1)?,
            c11: value(ix1, iy1)?,
        })
    }

    fn map<U>(self, mut f: impl FnMut(T) -> U) -> Cell<U> {
        Cell {
            c00: f(self.c00),
            c10: f(self.c10),
            c01: f(self.c01),
            c11: f(self.c11),
        }
    }
}

impl Cell<f64> {
    fn interpolate(&self, w: Weights) -> f64 {
        self.c00 * w.m00 + self.c01 * w.m01 + self.c10 * w.m10 + self.c11 * w.m11
    }
}

/// One component as the evaluator holds it: its grid set, opened once, and
/// whether it is bilinear (`bilinear`) or geocentric bilinear.
struct Slot<G> {
    bilinear: bool,
    grid_set: Option<G>,
}

/// Evaluates a [`MasterFile`] (`Evaluator` in `defmodel_impl.hpp`).
///
/// [`Evaluator::new`] checks the combinations of CRS and offset settings, and
/// opens every grid of the non-`none` components. The offsets are then pure
/// functions of the inputs.
pub struct Evaluator<P: GridProvider> {
    model: MasterFile,
    slots: Vec<Slot<P::Set>>,
    geographic: bool,
    horizontal_degree: bool,
    addition: bool,
    a: f64,
    b: f64,
    es: f64,
}

impl<P: GridProvider> fmt::Debug for Evaluator<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Evaluator")
            .field("model", &self.model.definition_crs)
            .field("geographic", &self.geographic)
            .field("horizontal_degree", &self.horizontal_degree)
            .field("addition", &self.addition)
            .finish_non_exhaustive()
    }
}

impl<P: GridProvider> Evaluator<P> {
    /// Builds the evaluator of `model` on the ellipsoid of semi-major axis `a`
    /// and semi-minor axis `b`.
    ///
    /// # Errors
    ///
    /// [`DefmodelError::Evaluator`] when a projected CRS is combined with a
    /// degree unit, a geocentric method or a geocentric bilinear
    /// interpolation, or when a grid cannot be opened.
    pub fn new(model: MasterFile, provider: &P, a: f64, b: f64) -> Result<Self, DefmodelError> {
        let geographic = provider.is_geographic_crs(model.definition_crs());
        let horizontal_degree = model.horizontal_offset_unit == "degree";
        let addition = model.horizontal_offset_method == "addition";
        if !geographic && horizontal_degree {
            return eval_error(
                "definition_crs = projected CRS and horizontal_offset_unit = degree are incompatible",
            );
        }
        if !geographic && !addition {
            return eval_error(
                "definition_crs = projected CRS and horizontal_offset_method = geocentric are incompatible",
            );
        }
        let mut slots = Vec::with_capacity(model.components.len());
        for component in &model.components {
            let bilinear = component.spatial_model.interpolation_method == "bilinear";
            if !geographic && !bilinear {
                return eval_error(
                    "definition_crs = projected CRS and interpolation_method = geocentric_bilinear are incompatible",
                );
            }
            let grid_set = if component.displacement == Displacement::None {
                None
            } else {
                let filename = component.spatial_model.filename.as_str();
                let set = provider.open(filename).ok_or_else(|| {
                    DefmodelError::Evaluator(format!("cannot open grid \"{filename}\""))
                })?;
                Some(set)
            };
            slots.push(Slot { bilinear, grid_set });
        }
        Ok(Self {
            model,
            slots,
            geographic,
            horizontal_degree,
            addition,
            a,
            b,
            es: 1.0 - (b * b) / (a * a),
        })
    }

    /// The master file being evaluated.
    pub fn master_file(&self) -> &MasterFile {
        &self.model
    }

    /// Forward evaluation at `(x, y, z)` and decimal year `t`. For a
    /// geographic CRS, `x` and `y` are radians. Returns the transformed
    /// `[x, y, z]`, or `None` when the point is outside the model, or when a
    /// grid gives no value at it.
    pub fn evaluate(&self, x: f64, y: f64, z: f64, t: f64) -> Option<[f64; 3]> {
        self.forward(x, y, z, t, Pass::Forward)
    }

    /// Inverse evaluation by fixed-point iteration over [`Evaluator::evaluate`]
    /// (ten steps at most). Returns `None` when the iteration does not
    /// converge.
    pub fn evaluate_inverse(&self, x: f64, y: f64, z: f64, t: f64) -> Option<[f64; 3]> {
        let (mut x_out, mut y_out, mut z_out) = (x, y, z);
        for _ in 0..10 {
            let [x_new, y_new, z_new] = self.forward(x_out, y_out, z_out, t, Pass::Inverse)?;
            let dx = x_new - x;
            let dy = y_new - y;
            let dz = z_new - z;
            x_out -= dx;
            y_out -= dy;
            z_out -= dz;
            if dx.abs().max(dy.abs()) < 1e-12 && dz.abs() < 1e-3 {
                return Some([x_out, y_out, z_out]);
            }
        }
        None
    }

    fn forward(&self, x_in: f64, y_in: f64, z_in: f64, t: f64, pass: Pass) -> Option<[f64; 3]> {
        let eps = if self.geographic { 1e-10 } else { 1e-5 };
        let (mut x, mut y) = (x_in, y_in);
        let model_bounds = self.model.extent.normalized(self.geographic);
        if self.geographic {
            while x < model_bounds.minx - eps {
                x += 2.0 * PI;
            }
            while x > model_bounds.maxx + eps {
                x -= 2.0 * PI;
            }
        }
        let margin = if self.geographic {
            0.1 * DEG_TO_RAD
        } else {
            10_000.0
        };
        if !bbox_check(&mut x, &mut y, pass, model_bounds, eps, margin) {
            return None;
        }
        let time = &self.model.time_extent;
        if t < time.first.decimal_year || t > time.last.decimal_year {
            return None;
        }

        let (sinphi, cosphi) = (y.sin(), y.cos());
        let (mut dlam, mut dphi, mut de, mut dn, mut dz) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (component, slot) in self.model.components.iter().zip(&self.slots) {
            let Some(grid_set) = &slot.grid_set else {
                continue;
            };
            let bounds = component.extent.normalized(self.geographic);
            let (mut x_grid, mut y_grid) = (x, y);
            if !bbox_check(&mut x_grid, &mut y_grid, pass, bounds, eps, 0.0) {
                continue;
            }
            x_grid = x_grid.max(bounds.minx).min(bounds.maxx);
            y_grid = y_grid.max(bounds.miny).min(bounds.maxy);
            let factor = component.time_function.evaluate_at(t);
            if factor == 0.0 {
                continue;
            }
            let Some(grid) = grid_set.grid_at(x_grid, y_grid) else {
                continue;
            };
            if grid.width() < 2 || grid.height() < 2 {
                return None;
            }
            let ix_d = (x_grid - grid.min_x()) / grid.res_x();
            let iy_d = (y_grid - grid.min_y()) / grid.res_y();
            if ix_d < -eps
                || iy_d < -eps
                || ix_d + 1.0 >= dim(grid.width()) + eps
                || iy_d + 1.0 >= dim(grid.height()) + eps
            {
                continue;
            }
            let ix0 = trunc_nonneg(ix_d).min(grid.width() - 2);
            let iy0 = trunc_nonneg(iy_d).min(grid.height() - 2);
            let (ix1, iy1) = (ix0 + 1, iy0 + 1);
            let frct_x = ix_d - dim(ix0);
            let frct_y = iy_d - dim(iy0);
            let (one_minus_x, one_minus_y) = (1.0 - frct_x, 1.0 - frct_y);
            let w = Weights {
                m00: one_minus_x * one_minus_y,
                m10: frct_x * one_minus_y,
                m01: one_minus_x * frct_y,
                m11: frct_x * frct_y,
            };
            let nodes = (ix0, iy0, ix1, iy1);
            let displacement = component.displacement;
            if displacement == Displacement::Vertical {
                let z = Cell::read(nodes, |ix, iy| grid.z_offset(ix, iy))?;
                dz += factor * z.interpolate(w);
            } else if self.horizontal_degree {
                let cell = Cell::read(nodes, |ix, iy| {
                    if displacement == Displacement::ThreeD {
                        grid.long_lat_z_offset(ix, iy)
                    } else {
                        grid.long_lat_offset(ix, iy)
                            .map(|(lam, phi)| (lam, phi, 0.0))
                    }
                })?;
                if displacement == Displacement::ThreeD {
                    dz += factor * cell.map(|v| v.2).interpolate(w);
                }
                dlam += factor * cell.map(|v| v.0).interpolate(w);
                dphi += factor * cell.map(|v| v.1).interpolate(w);
            } else {
                let cell = Cell::read(nodes, |ix, iy| {
                    if displacement == Displacement::ThreeD {
                        grid.easting_northing_z_offset(ix, iy)
                    } else {
                        grid.easting_northing_offset(ix, iy)
                            .map(|(e, n)| (e, n, 0.0))
                    }
                })?;
                if displacement == Displacement::ThreeD {
                    dz += factor * cell.map(|v| v.2).interpolate(w);
                }
                if slot.bilinear {
                    de += factor * cell.map(|v| v.0).interpolate(w);
                    dn += factor * cell.map(|v| v.1).interpolate(w);
                } else {
                    let (d_x, d_y, d_z) = geocentric_cell(grid, iy0, &cell, w);
                    let lam = (frct_x - 0.5) * grid.res_x();
                    let small = grid.res_x() < DEG_TO_RAD;
                    let (sinlam, coslam) = if small {
                        (
                            lam * (1.0 - (1.0 / 6.0) * (lam * lam)),
                            1.0 - 0.5 * (lam * lam),
                        )
                    } else {
                        (lam.sin(), lam.cos())
                    };
                    de += factor * (-d_x * sinlam + d_y * coslam);
                    dn += factor * ((-d_x * coslam - d_y * sinlam) * sinphi + d_z * cosphi);
                }
            }
        }

        let mut x_out = x_in;
        let mut y_out = y_in;
        if self.horizontal_degree {
            x_out += dlam;
            y_out += dphi;
        } else if self.addition && !self.geographic {
            x_out += de;
            y_out += dn;
        } else if self.addition {
            let (dlam, dphi) = east_north_to_lonlat(cosphi, de, dn, self.a, self.b, self.es);
            x_out += dlam;
            y_out += dphi;
        } else {
            let (sinlam, coslam) = (x.sin(), x.cos());
            let dnsinphi = dn * sinphi;
            let d_x = -de * sinlam - dnsinphi * coslam;
            let d_y = de * coslam - dnsinphi * sinlam;
            let d_z = dn * cosphi;
            let (gx, gy, gz) = geodetic_to_geocentric(x, y, self.a, self.es);
            let (lam, phi) = geocentric_to_geodetic(gx + d_x, gy + d_y, gz + d_z, self.a, self.es);
            x_out = lam;
            y_out = phi;
        }
        Some([x_out, y_out, z_in + dz])
    }
}

/// Keeps a point inside `bounds` (with the margin `margin` in the inverse
/// pass, where the point is clamped to the edge). Returns `false` when the
/// point is outside.
fn bbox_check(x: &mut f64, y: &mut f64, pass: Pass, bounds: Bounds, eps: f64, margin: f64) -> bool {
    let Bounds {
        minx,
        miny,
        maxx,
        maxy,
    } = bounds;
    if *x >= minx - eps && *x <= maxx + eps && *y >= miny - eps && *y <= maxy + eps {
        return true;
    }
    if pass == Pass::Forward {
        return false;
    }
    let x_ok = if *x >= minx - eps && *x <= maxx + eps {
        true
    } else if *x > minx - margin && *x < minx {
        *x = minx;
        true
    } else if *x < maxx + margin && *x > maxx {
        *x = maxx;
        true
    } else {
        false
    };
    let y_ok = if *y >= miny - eps && *y <= maxy + eps {
        true
    } else if *y > miny - margin && *y < miny {
        *y = miny;
        true
    } else if *y < maxy + margin && *y > maxy {
        *y = maxy;
        true
    } else {
        false
    };
    x_ok && y_ok
}

/// Easting and northing offsets to longitude and latitude offsets, in radians.
fn east_north_to_lonlat(cosphi: f64, de: f64, dn: f64, a: f64, b: f64, es: f64) -> (f64, f64) {
    let one_minus_x = es * (1.0 - cosphi * cosphi);
    let x = 1.0 - one_minus_x;
    let sqrt_x = x.sqrt();
    let dlam = de * sqrt_x / (a * cosphi);
    let dphi = dn * a * sqrt_x * x / (b * b);
    (dlam, dphi)
}

/// Geographic (height 0) to geocentric coordinates.
fn geodetic_to_geocentric(lam: f64, phi: f64, a: f64, es: f64) -> (f64, f64, f64) {
    let sinphi = phi.sin();
    let n = a / (1.0 - es * sinphi * sinphi).sqrt();
    (
        n * phi.cos() * lam.cos(),
        n * phi.cos() * lam.sin(),
        n * (1.0 - es) * sinphi,
    )
}

/// Geocentric to geographic longitude and latitude, in radians. The height is
/// not returned, as PROJ ignores it here. The latitude is a fixed-point
/// iteration that gains about three digits per step.
fn geocentric_to_geodetic(x: f64, y: f64, z: f64, a: f64, es: f64) -> (f64, f64) {
    let lam = y.atan2(x);
    let p = x.hypot(y);
    let mut phi = z.atan2(p * (1.0 - es));
    for _ in 0..30 {
        let sinphi = phi.sin();
        let n = a / (1.0 - es * sinphi * sinphi).sqrt();
        let next = (z + es * n * sinphi).atan2(p);
        let converged = (next - phi).abs() < 1e-15;
        phi = next;
        if converged {
            break;
        }
    }
    (lam, phi)
}

/// Geocentric offsets of a cell, relative to the point where `x = -res_x / 2`
/// on the row of `iy0`, as `getBilinearGeocentric` computes them.
fn geocentric_cell(
    grid: &impl Grid,
    iy0: usize,
    cell: &Cell<(f64, f64, f64)>,
    w: Weights,
) -> (f64, f64, f64) {
    let y0 = grid.min_y() + dim(iy0) * grid.res_y();
    let sinphi0 = y0.sin();
    let cosphi0 = y0.cos();
    let sin_res = grid.res_y().sin();
    let cos_res = grid.res_y().cos();
    let sinphi1 = sinphi0 * cos_res + cosphi0 * sin_res;
    let cosphi1 = cosphi0 * cos_res - sinphi0 * sin_res;
    let sin_half = (grid.res_x() / 2.0).sin();
    let cos_half = (grid.res_x() / 2.0).cos();
    let corner = |sinlam: f64, coslam: f64, sinphi: f64, cosphi: f64, value: (f64, f64, f64)| {
        enu(sinlam, coslam, sinphi, cosphi, value.0, value.1)
    };
    let c00 = corner(-sin_half, cos_half, sinphi0, cosphi0, cell.c00);
    let c01 = corner(-sin_half, cos_half, sinphi1, cosphi1, cell.c01);
    let c10 = corner(sin_half, cos_half, sinphi0, cosphi0, cell.c10);
    let c11 = corner(sin_half, cos_half, sinphi1, cosphi1, cell.c11);
    (
        w.m00 * c00.0 + w.m01 * c01.0 + w.m10 * c10.0 + w.m11 * c11.0,
        w.m00 * c00.1 + w.m01 * c01.1 + w.m10 * c10.1 + w.m11 * c11.1,
        w.m00 * c00.2 + w.m01 * c01.2 + w.m10 * c10.2 + w.m11 * c11.2,
    )
}

/// The geocentric offset of one east, north offset at a corner.
fn enu(sinlam: f64, coslam: f64, sinphi: f64, cosphi: f64, de: f64, dn: f64) -> (f64, f64, f64) {
    let north_sin = dn * sinphi;
    (
        -de * sinlam - north_sin * coslam,
        de * coslam - north_sin * sinlam,
        dn * cosphi,
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions,
    reason = "ix_d was rejected when negative or past the grid, so the truncation matches PROJ's int cast"
)]
fn trunc_nonneg(value: f64) -> usize {
    let truncated = value as i32;
    if truncated <= 0 {
        0
    } else {
        truncated as usize
    }
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "grid dimensions used here are far below the f64 mantissa"
)]
fn dim(value: usize) -> f64 {
    value as f64
}

/// The keyword that selects this model in a PROJ string.
pub(super) fn keys() -> &'static [&'static str] {
    &["model"]
}

/// The `+proj=defmodel` operation.
pub(super) fn build(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let Some(file) = params.text("model") else {
        return Err(CreateError::MissingArgument("model"));
    };
    if file.is_empty() {
        return Err(CreateError::MissingArgument("model"));
    }
    let bytes = super::read_resource(file)?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(CreateError::IllegalValue("model"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| CreateError::IllegalValue("model"))?;
    let master = MasterFile::parse(text).map_err(|_| CreateError::IllegalValue("model"))?;
    if master.file_type != "deformation_model_master_file" || master.format_version != "1.0" {
        return Err(CreateError::IllegalValue("model"));
    }
    if master.components.iter().any(|component| {
        component.displacement != Displacement::None
            && component.spatial_model.model_type != "GeoTIFF"
    }) {
        return Err(CreateError::IllegalValue("model"));
    }
    let evaluator = Evaluator::new(master, &EngineProvider, frame.earth.a(), frame.earth.b())
        .map_err(|_| CreateError::IllegalValue("model"))?;
    Ok(Box::new(Model {
        geographic: evaluator.geographic,
        a: frame.earth.a(),
        es: frame.earth.es(),
        evaluator,
    }))
}

/// Opens the grids with the crate's readers.
struct EngineProvider;

/// The one grid of a grid file, as the readers of the crate give it.
struct EngineGridSet(EngineGrid);

impl GridProvider for EngineProvider {
    type Set = EngineGridSet;

    fn open(&self, filename: &str) -> Option<EngineGridSet> {
        let bytes = super::read_resource(filename).ok()?;
        super::grid::open_named(filename, &bytes).map(EngineGridSet)
    }

    fn is_geographic_crs(&self, definition: &str) -> bool {
        is_geographic(definition)
    }
}

impl GridSet for EngineGridSet {
    type Node = EngineGrid;

    fn grid_at(&self, _x: f64, _y: f64) -> Option<&EngineGrid> {
        Some(&self.0)
    }
}

impl Grid for EngineGrid {
    fn min_x(&self) -> f64 {
        self.west
    }

    fn min_y(&self) -> f64 {
        self.south
    }

    fn res_x(&self) -> f64 {
        self.res_x
    }

    fn res_y(&self) -> f64 {
        self.res_y
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn long_lat_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64)> {
        let (sample_x, sample_y) = band_pair(self)?;
        let east = f64::from(self.value(ix, iy, sample_x)?) * DEG_TO_RAD;
        let north = f64::from(self.value(ix, iy, sample_y)?) * DEG_TO_RAD;
        Some((east, north))
    }

    fn long_lat_z_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64, f64)> {
        let (lam, phi) = self.long_lat_offset(ix, iy)?;
        Some((lam, phi, self.z_offset(ix, iy)?))
    }

    fn easting_northing_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64)> {
        let (sample_x, sample_y) = band_pair(self)?;
        Some((
            f64::from(self.value(ix, iy, sample_x)?),
            f64::from(self.value(ix, iy, sample_y)?),
        ))
    }

    fn easting_northing_z_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64, f64)> {
        let (east, north) = self.easting_northing_offset(ix, iy)?;
        Some((east, north, self.z_offset(ix, iy)?))
    }

    fn z_offset(&self, ix: usize, iy: usize) -> Option<f64> {
        let sample = band_z(self)?;
        Some(f64::from(self.value(ix, iy, sample)?))
    }
}

/// The east and north bands of a grid file, found by their descriptions. A
/// file without descriptions uses bands 0 and 1.
fn band_pair(grid: &EngineGrid) -> Option<(usize, usize)> {
    if grid.samples < 2 {
        return None;
    }
    let mut x = 0usize;
    let mut y = 1usize;
    let mut found = false;
    let mut any = false;
    for sample in 0..grid.samples {
        let desc = grid.description(sample);
        if !desc.is_empty() {
            any = true;
        }
        if desc == "east_offset" {
            x = sample;
            found = true;
        } else if desc == "north_offset" {
            y = sample;
        }
    }
    let have_y = (0..grid.samples).any(|sample| grid.description(sample) == "north_offset");
    if any && (!found || !have_y) {
        return None;
    }
    let unit = grid.unit(x);
    if !unit.is_empty() && unit != "degree" && unit != "metre" {
        return None;
    }
    Some((x, y))
}

/// The vertical band of a grid file.
fn band_z(grid: &EngineGrid) -> Option<usize> {
    if grid.samples == 1 {
        return Some(0);
    }
    if grid.samples < 3 && grid.description(0) != "vertical_offset" {
        return None;
    }
    let mut sample_z = 2;
    let mut any = false;
    let mut found = false;
    for sample in 0..grid.samples {
        let desc = grid.description(sample);
        if !desc.is_empty() {
            any = true;
        }
        if desc == "vertical_offset" {
            sample_z = sample;
            found = true;
        }
    }
    if any && !found {
        return None;
    }
    let unit = grid.unit(sample_z);
    if !unit.is_empty() && unit != "metre" {
        return None;
    }
    Some(sample_z)
}

/// Whether the CRS `definition` is geographic: it is not a projected CRS of
/// the database.
fn is_geographic(definition: &str) -> bool {
    let Some((auth, code)) = definition.split_once(':') else {
        return true;
    };
    match crate::projdb::with_db(|connection| {
        connection.query_row(
            "SELECT 1 FROM projected_crs WHERE auth_name = ?1 AND code = ?2",
            rusqlite::params![auth, code],
            |_| Ok(()),
        )
    }) {
        Ok(()) => false,
        Err(_) => true,
    }
}

struct Model {
    geographic: bool,
    a: f64,
    es: f64,
    evaluator: Evaluator<EngineProvider>,
}

impl Step for Model {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !coord.t.is_finite() {
            return Err(TransformError::MissingTime);
        }
        let [x, y, z] = self
            .evaluator
            .evaluate(coord.x, coord.y, coord.z, coord.t)
            .ok_or(TransformError::OutsideDomain)?;
        Ok(Coord {
            x,
            y,
            z,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !coord.t.is_finite() {
            return Err(TransformError::MissingTime);
        }
        let [x, y, z] = self
            .evaluator
            .evaluate_inverse(coord.x, coord.y, coord.z, coord.t)
            .ok_or(TransformError::OutsideDomain)?;
        Ok(Coord {
            x,
            y,
            z,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        if self.geographic {
            IoKind::Angular
        } else {
            IoKind::Projected
        }
    }

    fn output_kind(&self) -> IoKind {
        self.input_kind()
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}
