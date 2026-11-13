//! georust `proj` 0.31 facade over the ported operations.
//!
//! [`Proj::new`] builds a PROJ string, an `EPSG:` code, or a minimal WKT or
//! PROJJSON geographic or projected CRS. [`Proj::new_known_crs`] builds a
//! transform between EPSG CRS looked up in the embedded `proj.db`. Network
//! grid download follows `proj.ini`: off unless `PROJ_NETWORK` or
//! [`ProjBuilder::enable_network`] turns it on. An unknown operation returns
//! an error at creation.

use std::error::Error;
use std::ffi::NulError;
use std::fmt;
use std::path::Path;

mod bounds;
#[path = "proj_export.rs"]
mod proj_export;

use bounds::Dims;

use crate::engine::Operation;
use crate::engine::TransformError as StepError;
use crate::iso19111::{Pipeline, ProjStringLayout};
use crate::proj_iso::{iso_pipeline, iso_pipeline_at, IsoPipeline};
use crate::{CreateError, Xyzt};

/// Numeric type accepted by [`Coord`].
///
/// `f32` and `f64` are the supported types, matching the floats georust `proj`
/// accepts through `num_traits::Float`.
pub trait CoordinateType: Copy + PartialOrd + fmt::Debug {
    /// Widen to `f64` for the transform.
    ///
    /// # Errors
    ///
    /// [`ProjError::FloatConversion`] when the value cannot be represented.
    fn into_f64(self) -> Result<f64, ProjError>;

    /// Narrow an `f64` result back to this type.
    ///
    /// # Errors
    ///
    /// [`ProjError::FloatConversion`] when the value cannot be represented.
    fn from_f64(value: f64) -> Result<Self, ProjError>;
}

impl CoordinateType for f64 {
    fn into_f64(self) -> Result<f64, ProjError> {
        Ok(self)
    }

    fn from_f64(value: f64) -> Result<Self, ProjError> {
        Ok(value)
    }
}

impl CoordinateType for f32 {
    fn into_f64(self) -> Result<f64, ProjError> {
        Ok(f64::from(self))
    }

    fn from_f64(value: f64) -> Result<Self, ProjError> {
        Ok(narrow_f32(value))
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "georust converts an f64 image to f32 with a saturating narrowing cast"
)]
fn narrow_f32(value: f64) -> f32 {
    value as f32
}

/// A point in two dimensions.
///
/// `(x, y)` tuples implement this. Angular values are radians for [`Proj::new`]
/// of a classic projection, and degrees for [`Proj::new_known_crs`].
pub trait Coord<T>
where
    T: CoordinateType,
{
    /// First axis.
    fn x(&self) -> T;
    /// Second axis.
    fn y(&self) -> T;
    /// Build a point from the two axes.
    fn from_xy(x: T, y: T) -> Self;
}

impl<T: CoordinateType> Coord<T> for (T, T) {
    fn x(&self) -> T {
        self.0
    }

    fn y(&self) -> T {
        self.1
    }

    fn from_xy(x: T, y: T) -> Self {
        (x, y)
    }
}

/// A coordinate transform failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProjError {
    /// Forward or inverse projection failed.
    Projection(String),
    /// A conversion between CRS failed.
    Conversion(String),
    /// A string passed to PROJ contained an interior nul.
    Creation(NulError),
    /// This CRS has no area of use in the built-in table.
    UnknownAreaOfUse,
    /// A filesystem path could not be used.
    Path,
    /// Bytes were not UTF-8.
    Utf8Error(std::str::Utf8Error),
    /// A coordinate could not be converted to the requested float.
    FloatConversion,
    /// Network download is not built.
    Network,
    /// Remote grid callbacks are not built.
    RemoteCallbacks,
    /// A request could not be cloned. Network access is not built.
    RequestCloneError,
    /// A download had no content length. Network access is not built.
    ContentLength,
    /// A response header was missing. Network access is not built.
    HeaderError(String),
    /// A download failed. Network access is not built.
    DownloadError(String, String, u8),
    /// The operation has no recoverable definition.
    Definition,
    /// PROJJSON export is not built.
    ExportToJson,
    /// WKT export is not built.
    ExportToWkt,
}

impl fmt::Display for ProjError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Projection(message) => write!(
                formatter,
                "The projection failed with the following error: {message}"
            ),
            Self::Conversion(message) => write!(
                formatter,
                "The conversion failed with the following error: {message}"
            ),
            Self::Creation(_) => {
                formatter.write_str("Couldn't create a raw pointer from the string")
            }
            Self::UnknownAreaOfUse => formatter.write_str("The projection area of use is unknown"),
            Self::Path => formatter.write_str("Couldn't convert path to slice"),
            Self::Utf8Error(_) => formatter.write_str("Couldn't convert bytes from PROJ to UTF-8"),
            Self::FloatConversion => formatter.write_str("Couldn't convert number to f64"),
            Self::Network => {
                formatter.write_str("Network download functionality could not be enabled")
            }
            Self::RemoteCallbacks => {
                formatter.write_str("Could not set remote grid download callbacks")
            }
            Self::RequestCloneError => formatter.write_str("Couldn't clone request"),
            Self::ContentLength => formatter.write_str("Could not retrieve content length"),
            Self::HeaderError(key) => write!(formatter, "Couldn't retrieve header for key {key}"),
            Self::DownloadError(error, url, retries) => {
                write!(
                    formatter,
                    "A {error} error occurred for url {url} after {retries} retries"
                )
            }
            Self::Definition => {
                formatter.write_str("The current definition could not be retrieved")
            }
            Self::ExportToJson => formatter
                .write_str("The definition could not be represented in the requested JSON format"),
            Self::ExportToWkt => formatter
                .write_str("The definition could not be represented in the requested WKT format"),
        }
    }
}

impl Error for ProjError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Creation(error) => Some(error),
            Self::Utf8Error(error) => Some(error),
            _ => None,
        }
    }
}

impl From<NulError> for ProjError {
    fn from(error: NulError) -> Self {
        Self::Creation(error)
    }
}

impl From<std::str::Utf8Error> for ProjError {
    fn from(error: std::str::Utf8Error) -> Self {
        Self::Utf8Error(error)
    }
}

impl From<StepError> for ProjError {
    fn from(error: StepError) -> Self {
        Self::Conversion(error.to_string())
    }
}

/// Creating a [`Proj`] failed.
#[derive(Debug)]
pub enum ProjCreateError {
    /// The definition string contained an interior nul.
    ArgumentNulError(NulError),
    /// The operation or CRS is unknown, or a parameter was rejected.
    ProjError(String),
    /// A coordinate metadata object needs a single CRS, not a transform.
    MetadataObjectCreation,
}

impl fmt::Display for ProjCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArgumentNulError(error) => write!(
                formatter,
                "A nul byte was found in the PROJ string definition or CRS argument: {error}"
            ),
            Self::ProjError(message) => {
                write!(formatter, "The underlying PROJ call failed: {message}")
            }
            Self::MetadataObjectCreation => formatter.write_str(
                "Pipeline objects cannot be used to produce a MetadataObject. Try assigning the epoch to one of the input projections",
            ),
        }
    }
}

impl Error for ProjCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ArgumentNulError(error) => Some(error),
            _ => None,
        }
    }
}

/// Bounding box of an area of use, in degrees.
///
/// When the box crosses the antimeridian, `west` is greater than `east`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Area {
    /// Northern latitude, degrees.
    pub north: f64,
    /// Southern latitude, degrees.
    pub south: f64,
    /// Eastern longitude, degrees.
    pub east: f64,
    /// Western longitude, degrees.
    pub west: f64,
}

impl Area {
    /// Build a box from its four edges, in degrees.
    #[must_use]
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }
}

/// Version information for this port.
///
/// The numbers are the PROJ release this crate follows, not a libproj install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    /// Major version.
    pub major: i32,
    /// Minor version.
    pub minor: i32,
    /// Patch version.
    pub patch: i32,
    /// Release name.
    pub release: String,
    /// Version string.
    pub version: String,
    /// Extra grid search path set on the builder. Empty when none was added.
    pub searchpath: String,
}

/// Description of one built transform.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjInfo {
    /// Authority identifier, when the transform came from a named CRS pair.
    pub id: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// PROJ string or CRS pair that built this object.
    pub definition: Option<String>,
    /// Whether an inverse direction was found.
    pub has_inverse: bool,
    /// Accuracy in metres. `-1` means the table does not state one.
    pub accuracy: f64,
}

/// Options for [`Proj::as_wkt`]. An unset field keeps PROJ's default: multi-line output
/// except for WKT1 Esri, four spaces of indentation, and strict WKT1.
#[derive(Clone, Debug, Default)]
pub struct WktOptions {
    multiline: Option<bool>,
    indentation_width: Option<usize>,
    output_axis: Option<WktOutputAxis>,
    strict: Option<bool>,
    allow_ellipsoidal_height_as_vertical_crs: Option<bool>,
    allow_linunit_node: Option<bool>,
}

/// Whether a WKT writer emits axis order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WktOutputAxis {
    /// Follow the WKT version's default.
    Auto,
    /// Always emit axes.
    Yes,
    /// Never emit axes.
    No,
}

/// WKT dialect for [`Proj::as_wkt`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    non_camel_case_types,
    reason = "names match georust proj 0.31 WktVersion"
)]
pub enum WktVersion {
    /// WKT2:2015.
    Wkt2_2015,
    /// Simplified WKT2:2015.
    Wkt2_2015_Simplified,
    /// WKT2:2019.
    Wkt2_2019,
    /// Simplified WKT2:2019.
    Wkt2_2019_Simplified,
    /// WKT1 as GDAL writes it.
    Wkt1_Gdal,
    /// WKT1 as Esri writes it.
    Wkt1_Esri,
}

#[derive(Clone, Debug)]
enum Body {
    /// [`Proj::new`] of a PROJ operation. Angular axes are radians.
    Native(Operation),
    /// [`Proj::new_known_crs`] of a pair the ISO 19111 selection writes as a pipeline. The
    /// facade boundary is applied around it, see [`crate::proj_iso`].
    Iso(Box<IsoPipeline>),
    /// [`Proj::new`] of one named CRS. Not itself a transform.
    Single,
}

/// A coordinate transformation.
///
/// Build one with [`Proj::new`], [`Proj::new_known_crs`], or [`ProjBuilder`].
#[derive(Clone)]
pub struct Proj {
    body: Body,
    definition: String,
    area: Option<Area>,
    /// `new_known_crs` owns an area slot. [`Proj::area_set_bbox`] records into it.
    known: bool,
    net: crate::network::GridContext,
    /// The epoch of a coordinate metadata object, see [`crate::proj_meta`]. `None` otherwise.
    pub(crate) epoch: Option<f64>,
    /// The steps of a PROJ pipeline made by [`Proj::new`], kept for the inverse and the layout
    /// of [`Proj::as_proj_string`]. `None` for the other bodies.
    pipeline: Option<Pipeline>,
}

impl Proj {
    /// Build a transform from a PROJ string, one CRS from an `EPSG:` code, or
    /// a minimal WKT or PROJJSON geographic or projected CRS.
    ///
    /// For a classic projection, [`Self::project`] and [`Self::convert`] take
    /// that operation's native units. Angular axes are radians. An `EPSG:`
    /// code, WKT, or PROJJSON builds one CRS, not a transform.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when the string is empty, contains a nul, or names
    /// an operation, CRS, or conversion method this port does not build.
    pub fn new(definition: &str) -> Result<Self, ProjCreateError> {
        reject_nul(definition)?;
        if definition.trim().is_empty() {
            return Err(ProjCreateError::ProjError("empty definition".to_owned()));
        }
        if is_epsg(definition) {
            crate::crs::check_crs(definition)?;
            return Ok(built(Body::Single, definition.to_owned(), None));
        }
        if crate::crs_text::is_crs_text(definition) {
            crate::crs::check_well_known(definition)?;
            return Ok(built(Body::Single, definition.to_owned(), None));
        }
        let op = Operation::create(definition).map_err(from_create)?;
        let mut proj = built(Body::Native(op), definition.to_owned(), None);
        proj.pipeline = Some(Pipeline::parse(definition));
        Ok(proj)
    }

    /// Transform from `from` to `to`.
    ///
    /// Each side is an `EPSG:` code in `proj.db`, or a PROJ string for one
    /// geographic, projected, or geocentric CRS. Longitude, latitude order is
    /// degrees. Easting, northing and geocentric axes are metres.
    ///
    /// The operation is the one `cs2cs` picks, from the ISO 19111 selection
    /// ([`crate::iso19111::best_pipeline`]), when that selection writes a pipeline.
    /// Otherwise the pair goes through the CRS path, which emits a datum step only
    /// when every step is an operation this crate builds. `area` is stored and does
    /// not select an operation or a grid.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when either CRS is unknown, or the operation needs a
    /// grid, a deformation model, or a method this crate does not build.
    pub fn new_known_crs(
        from: &str,
        to: &str,
        area: Option<Area>,
    ) -> Result<Self, ProjCreateError> {
        reject_nul(from)?;
        reject_nul(to)?;
        // The ISO 19111 selection, as `cs2cs` takes it. A pair the selection does not write is
        // refused: libproj's creation has no other path for it.
        if let Some(pipeline) = iso_pipeline(from, to) {
            return Ok(built(
                Body::Iso(Box::new(pipeline)),
                format!("{from} -> {to}"),
                area,
            ));
        }
        Err(ProjCreateError::ProjError(format!(
            "no transformation from {from} to {to}: the selection writes no pipeline for it"
        )))
    }

    /// Build the transform from `self` to `target_crs`.
    ///
    /// Both objects must be single CRS values, such as `Proj::new("EPSG:4326")`.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when either object is already a transform, or `options`
    /// asks for a database search this port does not do.
    pub fn create_crs_to_crs_from_pj(
        &self,
        target_crs: &Self,
        area: Option<Area>,
        options: Option<Vec<&str>>,
    ) -> Result<Self, ProjCreateError> {
        if options
            .as_ref()
            .is_some_and(|items| items.iter().any(|item| !item.is_empty()))
        {
            return Err(ProjCreateError::ProjError(
                "CRS-to-CRS options are not built".to_owned(),
            ));
        }
        if !matches!(self.body, Body::Single) {
            return Err(ProjCreateError::ProjError(
                "source is not a single CRS".to_owned(),
            ));
        }
        if !matches!(target_crs.body, Body::Single) {
            return Err(ProjCreateError::ProjError(
                "target is not a single CRS".to_owned(),
            ));
        }
        // Two epochs are not a single `coordinateEpoch`. One epoch is applied on the
        // ISO pipeline. If that pipeline cannot be built, the transform is refused: no other
        // path carries the time.
        if let Some(error) = crate::proj_meta::time_dependent_refusal(self, target_crs) {
            return Err(error);
        }
        let epoch = crate::proj_meta::epoch_to_apply(self, target_crs);
        if let Some(pipeline) = iso_pipeline_at(&self.definition, &target_crs.definition, epoch) {
            return Ok(built(
                Body::Iso(Box::new(pipeline)),
                format!("{} -> {}", self.definition, target_crs.definition),
                area,
            ));
        }
        Err(ProjCreateError::ProjError(format!(
            "no transformation from {} to {}: the selection writes no pipeline for it",
            self.definition, target_crs.definition
        )))
    }

    /// Build a coordinate metadata object: this CRS at `epoch`.
    ///
    /// The object keeps the CRS and the epoch. [`Self::create_crs_to_crs_from_pj`]
    /// applies that epoch when exactly one side carries one. A static CRS such as
    /// `EPSG:4326` still accepts an epoch here, as `proj_coordinate_metadata_create`
    /// does in the georust test, even though PROJ 9 rejects an epoch on a static datum.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError::MetadataObjectCreation`] when `self` is a transform rather
    /// than a single CRS.
    pub fn coordinate_metadata_create(&self, epoch: f64) -> Result<Self, ProjCreateError> {
        if !matches!(self.body, Body::Single) {
            return Err(ProjCreateError::MetadataObjectCreation);
        }
        Ok(crate::proj_meta::with_epoch(self, epoch))
    }

    /// Epoch of a coordinate metadata object.
    ///
    /// An object without an epoch gives [`f64::NAN`].
    #[must_use]
    pub fn coordinate_metadata_get_epoch(&self) -> f64 {
        crate::proj_meta::epoch_of(self)
    }

    /// Record an area of use on a CRS-to-CRS transform.
    ///
    /// The area does not change the operation. A [`Proj`] that is not a
    /// CRS-to-CRS transform ignores the call, as libproj does when it has no
    /// area object.
    pub fn area_set_bbox(&mut self, new_bbox: Area) {
        if self.known {
            self.area = Some(new_bbox);
        }
    }

    /// Area of use of a CRS, and its name, as `proj_get_area_of_use` reports them.
    ///
    /// The box is `None` when the CRS has no bounding box in `proj.db`. The
    /// name is the description of its first usage.
    ///
    /// # Errors
    ///
    /// [`ProjError::UnknownAreaOfUse`] when this transform is not a CRS built from
    /// `EPSG:`, WKT, or PROJJSON, or when the CRS cannot be read.
    pub fn area_of_use(&self) -> Result<(Option<Area>, Option<String>), ProjError> {
        if matches!(self.body, Body::Single) {
            return crate::proj_info::crs_area_of_use(&self.definition);
        }
        Err(ProjError::UnknownAreaOfUse)
    }

    /// Description of this transform.
    ///
    /// For a PROJ string, `id` is its `proj=` name, `description` is
    /// `PROJ-based coordinate operation`, and `definition` is [`Self::def`].
    #[must_use]
    pub fn proj_info(&self) -> ProjInfo {
        if matches!(self.body, Body::Native(_)) {
            let definition = self.def().ok();
            let id = definition.as_deref().and_then(crate::proj_info::proj_name);
            return ProjInfo {
                id,
                description: Some(crate::proj_info::OPERATION_DESCRIPTION.to_owned()),
                definition,
                has_inverse: self.has_inverse(),
                accuracy: -1.0,
            };
        }
        let definition = self.def().ok();
        let deferred = definition.as_deref() == Some("unavailable until proj_trans is called");
        let id = if deferred {
            Some("unknown".to_owned())
        } else {
            definition.as_deref().and_then(crate::proj_info::proj_name)
        };
        let description = match &self.body {
            Body::Iso(_) if deferred => Some("unavailable until proj_trans is called".to_owned()),
            Body::Iso(pipeline) => Some(pipeline.description().to_owned()),
            _ => Some(self.definition.clone()),
        };
        ProjInfo {
            id,
            description,
            definition,
            has_inverse: self.has_inverse(),
            accuracy: -1.0,
        }
    }

    /// Definition of this object, as libproj reports it.
    ///
    /// For a PROJ string this is the CRS the string reads as, written back as
    /// `proj=… key=value …` with the datum's ellipsoid and `towgs84=` or
    /// `nadgrids=` appended. `+proj=longlat +datum=WGS84 +no_defs` gives
    /// `proj=longlat datum=WGS84 no_defs ellps=WGS84 towgs84=0,0,0`.
    /// A one-step pipeline is that step. `+proj=cart` stays `cart`. `+init=` is
    /// the expanded operation. A single CRS has no operation definition: PROJ's
    /// `proj_pj_info` leaves it null. A pair is the normalized pipeline when one
    /// operation can be built, and `unavailable until proj_trans is called` when
    /// several can.
    ///
    /// # Errors
    ///
    /// [`ProjError::Definition`] when no definition was stored, when a PROJ
    /// string names a datum PROJ does not know, or when a single CRS is asked
    /// for an operation definition.
    pub fn def(&self) -> Result<String, ProjError> {
        if self.definition.is_empty() {
            return Err(ProjError::Definition);
        }
        match &self.body {
            Body::Native(_) => crate::proj_info::operation_definition(&self.definition),
            Body::Single => Err(ProjError::Definition),
            Body::Iso(pipeline) => pipeline.reported_definition(),
        }
    }

    /// Version of the PROJ release this port follows.
    ///
    /// # Errors
    ///
    /// Never. The result is the in-crate version, not a libproj query.
    pub fn lib_info(&self) -> Result<Info, ProjError> {
        Ok(port_info(&self.net.searchpath()))
    }

    /// Whether this object was built with network grid download enabled.
    #[must_use]
    pub fn network_enabled(&self) -> bool {
        self.net.enabled
    }

    /// Grid download endpoint in effect when this object was built.
    ///
    /// # Errors
    ///
    /// Never. The default is `https://cdn.proj.org` when `proj.ini` and
    /// `PROJ_NETWORK_ENDPOINT` do not set one.
    pub fn get_url_endpoint(&self) -> Result<String, ProjError> {
        Ok(self.net.endpoint.clone())
    }

    /// Project geodetic coordinates.
    ///
    /// For [`Proj::new`] of a classic projection, axes are radians.
    /// For [`Proj::new_known_crs`], axes are the CRS units (degrees or metres)
    /// in longitude, latitude or easting, northing order.
    /// `inverse` selects the reverse direction.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when the object is not a transform or the point has no image.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "georust proj 0.31 project takes inverse as a bool"
    )]
    pub fn project<C, F>(&self, point: C, inverse: bool) -> Result<C, ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        let way = if inverse {
            Direction::Inverse
        } else {
            Direction::Forward
        };
        let (x, y) = self.map_point(point.x(), point.y(), way, Report::Projection)?;
        Ok(C::from_xy(F::from_f64(x)?, F::from_f64(y)?))
    }

    /// Convert coordinates in the forward direction.
    ///
    /// Units match [`Self::project`]. A failure is a [`ProjError::Conversion`],
    /// as in georust `proj`.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when the object is not a transform or the point has no image.
    pub fn convert<C, F>(&self, point: C) -> Result<C, ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        let (x, y) =
            self.map_point(point.x(), point.y(), Direction::Forward, Report::Conversion)?;
        Ok(C::from_xy(F::from_f64(x)?, F::from_f64(y)?))
    }

    /// Convert a three-dimensional coordinate in the forward direction.
    ///
    /// `z` is the third axis: height for a geographic or projected CRS,
    /// geocentric Z for a geocentric CRS, topocentric Up for a topocentric
    /// CRS. Units match [`Self::convert`], metres for `z`.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when the object is not a transform or the point has no image.
    pub fn convert_xyz(&self, x: f64, y: f64, z: f64) -> Result<(f64, f64, f64), ProjError> {
        let coord = self.map_xyz(x, y, z, Direction::Forward, Report::Conversion)?;
        Ok((coord.x, coord.y, coord.z))
    }

    /// Convert a four-dimensional coordinate in the forward direction.
    ///
    /// `t` is the coordinate time in decimal years, the fourth axis of PROJ's `proj_trans`.
    /// It drives the time-dependent steps of the operation, such as a Helmert with
    /// `+t_epoch` or a deformation model. A step that takes no time ignores it.
    /// Units match [`Self::convert_xyz`]. [`Self::convert_xyz`] is this method at `t = 0`.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when the object is not a transform or the point has no image.
    pub fn convert_xyzt(
        &self,
        x: f64,
        y: f64,
        z: f64,
        t: f64,
    ) -> Result<(f64, f64, f64, f64), ProjError> {
        let coord = self.map_xyzt(x, y, z, t, Direction::Forward, Report::Conversion)?;
        Ok((coord.x, coord.y, coord.z, coord.t))
    }

    /// Whether this transform reads degrees, rather than radians or CRS units, in `direction`.
    ///
    /// The input of `direction` is the output of the opposite direction, as PROJ's
    /// `proj_degree_input` defines it. A CRS object is not a transform and reads no angular
    /// unit: `false`.
    #[must_use]
    pub fn degree_input(&self, direction: Direction) -> bool {
        self.degree_output(direction.opposite())
    }

    /// Whether this transform writes degrees, rather than radians or CRS units, in `direction`.
    ///
    /// A CRS pair is normalized (`proj_normalize_for_visualization`): its output is the target
    /// of a forward transform and the source of an inverse one.
    #[must_use]
    pub fn degree_output(&self, direction: Direction) -> bool {
        match &self.body {
            Body::Native(op) => op.degree_output(direction == Direction::Inverse),
            Body::Iso(pipeline) => pipeline.degree_output(direction),
            Body::Single => false,
        }
    }

    /// Convert a slice in place, forward only.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when any point fails. The slice is left unchanged.
    pub fn convert_array<'a, C, F>(&self, points: &'a mut [C]) -> Result<&'a mut [C], ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        self.fill_array(points, Direction::Forward)
    }

    /// Project a slice in place.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when any point fails. The slice is left unchanged.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "georust proj 0.31 project_array takes inverse as a bool"
    )]
    pub fn project_array<'a, C, F>(
        &self,
        points: &'a mut [C],
        inverse: bool,
    ) -> Result<&'a mut [C], ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        let way = if inverse {
            Direction::Inverse
        } else {
            Direction::Forward
        };
        self.fill_array(points, way)
    }

    /// Bounds of a rectangle after a forward transform, densifying each edge.
    ///
    /// Shorthand for [`Self::transform_bounds_in`] with [`Direction::Forward`].
    ///
    /// # Errors
    ///
    /// [`ProjError`] when `densify_pts` is out of range, or the transform
    /// cannot tell the output's axis order.
    pub fn transform_bounds<F>(
        &self,
        left: F,
        bottom: F,
        right: F,
        top: F,
        densify_pts: i32,
    ) -> Result<[F; 4], ProjError>
    where
        F: CoordinateType,
    {
        self.transform_bounds_in(Direction::Forward, left, bottom, right, top, densify_pts)
    }

    /// Bounds of a rectangle after a transform in `direction`, PROJ's
    /// `proj_trans_bounds`.
    ///
    /// The rectangle is in the input axis order: longitude, latitude for a
    /// geographic input, as [`Self::convert`]. The result is
    /// `[min_x, min_y, max_x, max_y]` in the output order. When the output is
    /// geographic and the box crosses the antimeridian, `min_x > max_x`. When
    /// it contains a pole, the range is the full `-180..180` on that side.
    /// Points with no image are skipped, as PROJ does.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when `densify_pts` is outside `0..=10000`, is below 2 for a
    /// geographic output, or the output axis order is unknown.
    pub fn transform_bounds_in<F>(
        &self,
        direction: Direction,
        left: F,
        bottom: F,
        right: F,
        top: F,
        densify_pts: i32,
    ) -> Result<[F; 4], ProjError>
    where
        F: CoordinateType,
    {
        let min = [left.into_f64()?, bottom.into_f64()?, 0.0];
        let max = [right.into_f64()?, top.into_f64()?, 0.0];
        let (lo, hi) = self.bounds(direction, min, max, densify_pts, Dims::Two)?;
        Ok([
            F::from_f64(lo[0])?,
            F::from_f64(lo[1])?,
            F::from_f64(hi[0])?,
            F::from_f64(hi[1])?,
        ])
    }

    /// Bounds of a box after a transform in `direction`, PROJ's
    /// `proj_trans_bounds_3D`.
    ///
    /// `min` and `max` are `[x, y, z]` in the input axis order. The result is
    /// `(min, max)` in the output order. Each height plane is walked on its own,
    /// and a geocentric input is walked as its full box boundary. Otherwise the
    /// same rules as [`Self::transform_bounds_in`] apply.
    ///
    /// # Errors
    ///
    /// [`ProjError`] as for [`Self::transform_bounds_in`].
    pub fn transform_bounds_3d(
        &self,
        direction: Direction,
        min: [f64; 3],
        max: [f64; 3],
        densify_pts: i32,
    ) -> Result<([f64; 3], [f64; 3]), ProjError> {
        self.bounds(direction, min, max, densify_pts, Dims::Three)
    }

    /// PROJJSON stored for an EPSG code, or the PROJJSON text passed to [`Self::new`].
    ///
    /// PROJJSON of the CRS this object was built from, written by the ISO 19111
    /// exporter as `proj_as_projjson` does.
    ///
    /// `multiline` (multi-line by default), `indentation_width`, and `schema` (the
    /// PROJJSON v0.7 schema by default) set the layout. A transform built from a pair
    /// or a PROJ operation has no CRS to write: the export of that operation is not built.
    ///
    /// # Errors
    ///
    /// [`ProjError::ExportToJson`] when this object is not a CRS, or the CRS kind has no
    /// PROJJSON export in this crate.
    pub fn to_projjson(
        &self,
        multiline: Option<bool>,
        indentation_width: Option<usize>,
        schema: Option<&str>,
    ) -> Result<String, ProjError> {
        proj_export::to_projjson(self, multiline, indentation_width, schema)
    }

    /// WKT of the CRS this object was built from, in `version` (WKT2:2019 by default),
    /// written by the ISO 19111 exporter as `proj_as_wkt` does.
    ///
    /// `options` sets the layout, the axis rule, and the WKT1 strictness, as in
    /// [`WktOptions`]. A transform built from a pair or a PROJ operation has no CRS to
    /// write: the export of that operation is not built.
    ///
    /// # Errors
    ///
    /// [`ProjError::ExportToWkt`] when this object is not a CRS, or the CRS cannot be
    /// written in `version`.
    pub fn as_wkt(
        &self,
        version: Option<WktVersion>,
        options: Option<WktOptions>,
    ) -> Result<String, ProjError> {
        proj_export::as_wkt(self, version, options)
    }

    fn has_inverse(&self) -> bool {
        match &self.body {
            Body::Single => false,
            Body::Iso(pipeline) => pipeline.has_inverse(),
            Body::Native(op) => {
                !matches!(op.inverse(Xyzt::xy(0.0, 0.0)), Err(StepError::NoInverse))
            }
        }
    }

    fn fill_array<'a, C, F>(
        &self,
        points: &'a mut [C],
        way: Direction,
    ) -> Result<&'a mut [C], ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        let mut converted = Vec::with_capacity(points.len());
        for point in &*points {
            let (x, y) = self.map_point(point.x(), point.y(), way, Report::Conversion)?;
            converted.push(C::from_xy(F::from_f64(x)?, F::from_f64(y)?));
        }
        for (point, value) in points.iter_mut().zip(converted) {
            *point = value;
        }
        Ok(points)
    }

    fn map_point<F: CoordinateType>(
        &self,
        x: F,
        y: F,
        way: Direction,
        report: Report,
    ) -> Result<(f64, f64), ProjError> {
        self.map_f64(x.into_f64()?, y.into_f64()?, way, report)
    }

    fn map_f64(
        &self,
        x: f64,
        y: f64,
        way: Direction,
        report: Report,
    ) -> Result<(f64, f64), ProjError> {
        let coord = self.map_xyz(x, y, 0.0, way, report)?;
        Ok((coord.x, coord.y))
    }

    fn map_xyz(
        &self,
        x: f64,
        y: f64,
        z: f64,
        way: Direction,
        report: Report,
    ) -> Result<Xyzt, ProjError> {
        self.map_xyzt(x, y, z, 0.0, way, report)
    }

    fn map_xyzt(
        &self,
        x: f64,
        y: f64,
        z: f64,
        t: f64,
        way: Direction,
        report: Report,
    ) -> Result<Xyzt, ProjError> {
        let mapped = match &self.body {
            Body::Single => {
                return Err(match report {
                    Report::Projection => {
                        ProjError::Projection("CRS is not a transform".to_owned())
                    }
                    Report::Conversion => {
                        ProjError::Conversion("CRS is not a transform".to_owned())
                    }
                });
            }
            Body::Native(op) => {
                let coord = Xyzt { x, y, z, t };
                match way {
                    Direction::Inverse => op.inverse(coord),
                    Direction::Forward => op.forward(coord),
                }
            }
            Body::Iso(pipeline) => pipeline.map(Xyzt { x, y, z, t }, way),
        };
        mapped.map_err(|error| {
            // libproj words an invalid coordinate as `Invalid coordinate`.
            let message = match error {
                crate::engine::TransformError::InvalidCoord => "Invalid coordinate".to_owned(),
                other => other.to_string(),
            };
            match report {
                Report::Projection => ProjError::Projection(message),
                Report::Conversion => ProjError::Conversion(message),
            }
        })
    }
}

impl TryFrom<&str> for Proj {
    type Error = ProjCreateError;

    fn try_from(definition: &str) -> Result<Self, Self::Error> {
        Self::new(definition)
    }
}

impl TryFrom<(&str, &str)> for Proj {
    type Error = ProjCreateError;

    fn try_from((source_crs, target_crs): (&str, &str)) -> Result<Self, Self::Error> {
        Self::new_known_crs(source_crs, target_crs, None)
    }
}

/// Builder for a [`Proj`]. Network, endpoint, cache, and search paths apply
/// to the object it builds.
#[derive(Clone, Debug)]
pub struct ProjBuilder {
    net: crate::network::GridContext,
}

impl Default for ProjBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjBuilder {
    /// Create a builder.
    ///
    /// Network access starts from `proj.ini` (off) unless `PROJ_NETWORK` is
    /// `ON`, `YES`, or `TRUE`. The endpoint starts at `https://cdn.proj.org`
    /// unless `proj.ini` or `PROJ_NETWORK_ENDPOINT` sets one.
    #[must_use]
    pub fn new() -> Self {
        Self {
            net: crate::network::GridContext::load(),
        }
    }

    /// Build from a PROJ string. Same outcome as [`Proj::new`].
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] from [`Proj::new`].
    pub fn proj(self, definition: &str) -> Result<Proj, ProjCreateError> {
        crate::network::with_context(&self.net, || Proj::new(definition))
    }

    /// Build a CRS-to-CRS transform. Same outcome as [`Proj::new_known_crs`].
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] from [`Proj::new_known_crs`].
    pub fn proj_known_crs(
        self,
        from: &str,
        to: &str,
        area: Option<Area>,
    ) -> Result<Proj, ProjCreateError> {
        crate::network::with_context(&self.net, || Proj::new_known_crs(from, to, area))
    }

    /// Builder form of [`Proj::create_crs_to_crs_from_pj`].
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] from [`Proj::create_crs_to_crs_from_pj`].
    pub fn proj_create_crs_to_crs_from_pj(
        self,
        source_crs: &Proj,
        target_crs: &Proj,
        area: Option<Area>,
        options: Option<Vec<&str>>,
    ) -> Result<Proj, ProjCreateError> {
        crate::network::with_context(&self.net, || {
            source_crs.create_crs_to_crs_from_pj(target_crs, area, options)
        })
    }

    /// Enable or disable network grid download.
    ///
    /// This overrides `proj.ini` and `PROJ_NETWORK` for objects built from
    /// this builder. `1` means access is on, `0` means it is off.
    ///
    /// # Errors
    ///
    /// Never. The HTTP client is part of this build.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "georust proj 0.31 enable_network takes a bool"
    )]
    pub fn enable_network(&mut self, enable: bool) -> Result<u8, ProjError> {
        self.net.enabled = enable;
        Ok(u8::from(enable))
    }

    /// Whether network access is on for objects built from this builder.
    #[must_use]
    pub fn network_enabled(&self) -> bool {
        self.net.enabled
    }

    /// Grid download endpoint.
    ///
    /// # Errors
    ///
    /// Never. The default is `https://cdn.proj.org`.
    pub fn get_url_endpoint(&self) -> Result<String, ProjError> {
        Ok(self.net.endpoint.clone())
    }

    /// Set the grid download endpoint.
    ///
    /// This overrides `proj.ini` and `PROJ_NETWORK_ENDPOINT`. A missing grid
    /// is requested at `endpoint + "/" + filename`.
    ///
    /// # Errors
    ///
    /// [`ProjError::Creation`] when `endpoint` contains an interior nul.
    pub fn set_url_endpoint(&mut self, endpoint: &str) -> Result<(), ProjError> {
        let _checked = std::ffi::CString::new(endpoint).map_err(ProjError::Creation)?;
        self.net.endpoint = endpoint.to_owned();
        Ok(())
    }

    /// Add a grid search path, keeping paths already added.
    ///
    /// The user-writable directory is searched first, then the data
    /// directories (`PROJ_DATA`, or the default `data/grids`). The added path
    /// is searched after them.
    ///
    /// # Errors
    ///
    /// [`ProjError::Path`] when `newpath` is not valid UTF-8 or contains a nul.
    pub fn set_search_paths<P: AsRef<Path>>(&mut self, newpath: P) -> Result<(), ProjError> {
        let path = newpath.as_ref();
        let text = path.to_str().ok_or(ProjError::Path)?;
        if text.contains('\0') {
            return Err(ProjError::Path);
        }
        self.net.push_search_path(path.to_path_buf());
        Ok(())
    }

    /// Directory downloaded grids are written to.
    ///
    /// This is PROJ's user-writable directory. The directory is created when
    /// it does not exist. Tests point it at a temporary folder.
    ///
    /// # Errors
    ///
    /// [`ProjError::Path`] when `path` is not valid UTF-8, contains a nul, or
    /// the directory cannot be made.
    pub fn set_user_writable_directory<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), ProjError> {
        let path = path.as_ref();
        let text = path.to_str().ok_or(ProjError::Path)?;
        if text.contains('\0') {
            return Err(ProjError::Path);
        }
        std::fs::create_dir_all(path).map_err(|_| ProjError::Path)?;
        self.net.set_cache_dir(path.to_path_buf());
        Ok(())
    }

    /// Enable or disable the local cache of downloaded grids.
    ///
    /// The cache is on by default (`cache_enabled` in `proj.ini`). When it is
    /// off, a fetch is not written to disk and a previously cached file is
    /// not reused.
    ///
    /// # Errors
    ///
    /// Never.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "georust proj 0.31 grid_cache_enable takes a bool"
    )]
    pub fn grid_cache_enable(&mut self, enable: bool) -> Result<(), ProjError> {
        self.net.cache_enabled = enable;
        Ok(())
    }

    /// Version of the PROJ release this port follows.
    ///
    /// # Errors
    ///
    /// Never. The result is the in-crate version.
    pub fn lib_info(&self) -> Result<Info, ProjError> {
        Ok(port_info(&self.net.searchpath()))
    }
}

/// Direction of a transform: [`Self::Forward`] is the source-to-target way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// From the source to the target, as [`Proj::convert`].
    Forward,
    /// From the target back to the source, as [`Proj::project`] with `inverse`.
    Inverse,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Forward => Self::Inverse,
            Self::Inverse => Self::Forward,
        }
    }
}

#[derive(Clone, Copy)]
enum Report {
    Projection,
    Conversion,
}

fn port_info(searchpath: &str) -> Info {
    Info {
        major: 9,
        minor: 9,
        patch: 0,
        release: "9.9.0".to_owned(),
        version: "9.9.0".to_owned(),
        searchpath: searchpath.to_owned(),
    }
}

fn built(body: Body, definition: String, area: Option<Area>) -> Proj {
    let known = matches!(body, Body::Iso(_));
    Proj {
        body,
        definition,
        area,
        known,
        net: crate::network::context(),
        epoch: None,
        pipeline: None,
    }
}

fn is_epsg(definition: &str) -> bool {
    definition
        .trim()
        .split_once(':')
        .is_some_and(|(authority, code)| {
            authority.eq_ignore_ascii_case("EPSG") && code.chars().all(|ch| ch.is_ascii_digit())
        })
}

fn reject_nul(text: &str) -> Result<(), ProjCreateError> {
    if text.as_bytes().contains(&0) {
        let error = std::ffi::CString::new(text).expect_err("the text contains a nul byte");
        return Err(ProjCreateError::ArgumentNulError(error));
    }
    Ok(())
}

fn from_create(error: CreateError) -> ProjCreateError {
    ProjCreateError::ProjError(error.to_string())
}

// `proj_coordoperation_create_inverse` and `proj_as_proj_string` of a pipeline.
impl Proj {
    /// The inverse of this coordinate operation, as `proj_coordoperation_create_inverse`.
    ///
    /// Only a PROJ pipeline made by [`Proj::new`] has steps to invert. The inverse keeps its own
    /// steps, so it can be written again with [`Proj::as_proj_string`].
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when this object is a single CRS, when it is a pair selected from
    /// CRS definitions (its operation is not kept as steps), or when the inverse text does not
    /// build.
    pub fn coordoperation_create_inverse(&self) -> Result<Self, ProjCreateError> {
        let (Body::Native(_), Some(pipeline)) = (&self.body, &self.pipeline) else {
            return Err(ProjCreateError::ProjError(
                "the object is not a PROJ pipeline".to_owned(),
            ));
        };
        let inverse = pipeline.inverse();
        let text = inverse.render(ProjStringLayout::Single);
        let op = Operation::create(&text).map_err(from_create)?;
        let mut proj = built(Body::Native(op), text, None);
        proj.pipeline = Some(inverse);
        Ok(proj)
    }

    /// The PROJ string of this pipeline, laid out as `proj_as_proj_string` with `layout`.
    ///
    /// The steps are the ones [`Proj::new`] read. Global parameters and `title=` are not kept,
    /// so the text is the pipeline's steps, not the input string.
    ///
    /// # Errors
    ///
    /// [`ProjError::Definition`] when this object is not a PROJ pipeline made by [`Proj::new`].
    pub fn as_proj_string(&self, layout: ProjStringLayout) -> Result<String, ProjError> {
        match (&self.body, &self.pipeline) {
            (Body::Native(_), Some(pipeline)) => Ok(pipeline.render(layout)),
            _ => Err(ProjError::Definition),
        }
    }
}

// Alternatives of a pair, `proj_trans_get_last_used_operation` and `proj_is_equivalent_to`.
impl Proj {
    /// The operation the last point of [`Self::convert`] used, as
    /// `proj_trans_get_last_used_operation`.
    ///
    /// For a pair with alternative operations, this is the candidate the last point chose, as a
    /// transform of its own. A pair with one operation gives a copy of itself, which libproj also
    /// gives once a point was transformed; this port does not record that point, so it gives the
    /// copy before any point too. Other objects give `None`.
    #[must_use]
    pub fn last_used_operation(&self) -> Option<Self> {
        match &self.body {
            Body::Iso(pipeline) => match pipeline.current_item() {
                Some(item) => {
                    let definition = item.definition.clone()?;
                    let op = item.operation.clone()?;
                    let mut proj = built(Body::Native(op), definition.clone(), None);
                    proj.pipeline = Some(Pipeline::parse(&definition));
                    Some(proj)
                }
                None if pipeline.is_alternatives() => None,
                None => Some(self.clone()),
            },
            _ => None,
        }
    }

    /// Whether two transforms are equivalent, as `proj_is_equivalent_to` with `PJ_COMP_STRICT`.
    ///
    /// Two pairs with alternatives are equivalent when their candidate lists are (see
    /// `Alternatives::is_equivalent_to`). A single CRS or a PROJ string is compared by its
    /// text, not by the CRS it names: two codes for one CRS are not equivalent here.
    #[must_use]
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        match (&self.body, &other.body) {
            (Body::Iso(a), Body::Iso(b)) => a.is_equivalent_to(b),
            (Body::Native(_), Body::Native(_)) | (Body::Single, Body::Single) => {
                self.definition == other.definition
            }
            _ => false,
        }
    }
}
