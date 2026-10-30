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

use crate::crs::Crs;
use crate::engine::Operation;
use crate::engine::TransformError as StepError;
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
            Self::ExportToWkt => formatter.write_str("WKT export is not built"),
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
    /// Coordinate metadata objects are not built.
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

/// Options for [`Proj::as_wkt`]. WKT export is not built.
#[derive(Clone, Debug, Default)]
#[expect(
    dead_code,
    reason = "fields match georust WktOptions; this port does not write WKT"
)]
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

/// WKT dialect. Export of every dialect returns an error.
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
    /// [`Proj::new_known_crs`]. Angular axes are degrees.
    ///
    /// `bridge` is a PROJ pipeline from source geodetic radians to target
    /// geodetic radians. It is absent when no datum step is required.
    Between {
        source: Crs,
        target: Crs,
        bridge: Option<Operation>,
    },
    /// [`Proj::new`] of one named CRS. Not itself a transform.
    Single(Crs),
}

/// A coordinate transformation.
///
/// Build one with [`Proj::new`], [`Proj::new_known_crs`], or [`ProjBuilder`].
#[derive(Clone, Debug)]
pub struct Proj {
    body: Body,
    definition: String,
    area: Option<Area>,
    /// `new_known_crs` owns an area slot. [`Proj::area_set_bbox`] records into it.
    known: bool,
    net: crate::network::GridContext,
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
            let crs = Crs::parse(definition)?;
            return Ok(built(Body::Single(crs), definition.to_owned(), None));
        }
        if crate::crs_text::is_crs_text(definition) {
            let crs = Crs::from_well_known(definition)?;
            return Ok(built(Body::Single(crs), definition.to_owned(), None));
        }
        let op = Operation::create(definition).map_err(from_create)?;
        Ok(built(Body::Native(op), definition.to_owned(), None))
    }

    /// Transform from `from` to `to`.
    ///
    /// Each side is an `EPSG:` code in `proj.db`, or a PROJ string for one
    /// geographic, projected, or geocentric CRS. Longitude, latitude order is
    /// degrees. Easting, northing and geocentric axes are metres.
    ///
    /// `area` is stored and does not select a grid. A datum step is emitted as
    /// a PROJ pipeline only when every step is an operation this crate builds.
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
        let source = Crs::parse(from)?;
        let target = Crs::parse(to)?;
        let bridge = source.datum_bridge(&target)?;
        Ok(built(
            Body::Between {
                source,
                target,
                bridge,
            },
            format!("{from} -> {to}"),
            area,
        ))
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
        let Body::Single(source) = &self.body else {
            return Err(ProjCreateError::ProjError(
                "source is not a single CRS".to_owned(),
            ));
        };
        let Body::Single(target) = &target_crs.body else {
            return Err(ProjCreateError::ProjError(
                "target is not a single CRS".to_owned(),
            ));
        };
        let bridge = source.datum_bridge(target)?;
        Ok(built(
            Body::Between {
                source: source.clone(),
                target: target.clone(),
                bridge,
            },
            format!("{} -> {}", self.definition, target_crs.definition),
            area,
        ))
    }

    /// Coordinate metadata objects are not built.
    ///
    /// # Errors
    ///
    /// Always [`ProjCreateError::MetadataObjectCreation`].
    pub fn coordinate_metadata_create(&self, _epoch: f64) -> Result<Self, ProjCreateError> {
        Err(ProjCreateError::MetadataObjectCreation)
    }

    /// Epoch of a coordinate metadata object.
    ///
    /// This port does not build metadata objects, so the result is [`f64::NAN`].
    #[must_use]
    pub fn coordinate_metadata_get_epoch(&self) -> f64 {
        f64::NAN
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

    /// Area of use.
    ///
    /// # Errors
    ///
    /// [`ProjError::UnknownAreaOfUse`]. Extents in `proj.db` are not reported,
    /// and this method does not invent one.
    pub fn area_of_use(&self) -> Result<(Option<Area>, Option<String>), ProjError> {
        Err(ProjError::UnknownAreaOfUse)
    }

    /// Description of this transform.
    #[must_use]
    pub fn proj_info(&self) -> ProjInfo {
        ProjInfo {
            id: None,
            description: Some(self.definition.clone()),
            definition: Some(self.definition.clone()),
            has_inverse: self.has_inverse(),
            accuracy: -1.0,
        }
    }

    /// PROJ string or CRS pair that built this object.
    ///
    /// # Errors
    ///
    /// [`ProjError::Definition`] when no definition was stored.
    pub fn def(&self) -> Result<String, ProjError> {
        if self.definition.is_empty() {
            Err(ProjError::Definition)
        } else {
            Ok(self.definition.clone())
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
        let way = if inverse { Way::Inverse } else { Way::Forward };
        let (x, y) = self.map_point(point.x(), point.y(), way, Report::Projection)?;
        Ok(C::from_xy(F::from_f64(x)?, F::from_f64(y)?))
    }

    /// Convert coordinates in the forward direction.
    ///
    /// Units match [`Self::project`].
    ///
    /// # Errors
    ///
    /// [`ProjError`] when the object is not a transform or the point has no image.
    pub fn convert<C, F>(&self, point: C) -> Result<C, ProjError>
    where
        C: Coord<F>,
        F: CoordinateType,
    {
        self.project(point, false)
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
        let coord = self.map_xyz(x, y, z, Way::Forward, Report::Projection)?;
        Ok((coord.x, coord.y, coord.z))
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
        self.fill_array(points, Way::Forward)
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
        let way = if inverse { Way::Inverse } else { Way::Forward };
        self.fill_array(points, way)
    }

    /// Forward bounds of a rectangle, densifying each edge.
    ///
    /// `densify_pts` is the number of extra samples on each edge.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when `densify_pts` is negative or a sample has no image.
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
        if densify_pts < 0 {
            return Err(ProjError::Conversion(
                "densify_pts must be zero or positive".to_owned(),
            ));
        }
        let left = left.into_f64()?;
        let bottom = bottom.into_f64()?;
        let right = right.into_f64()?;
        let top = top.into_f64()?;
        let steps = usize::try_from(densify_pts)
            .map_err(|_| ProjError::Conversion("densify_pts does not fit usize".to_owned()))?
            + 1;
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let edges = [
            (left, bottom, right, bottom),
            (right, bottom, right, top),
            (right, top, left, top),
            (left, top, left, bottom),
        ];
        for (x0, y0, x1, y1) in edges {
            for step in 0..steps {
                #[expect(
                    clippy::cast_precision_loss,
                    clippy::as_conversions,
                    reason = "densify_pts is a small edge count and fits exactly in f64"
                )]
                let t = step as f64 / steps as f64;
                let (x, y) = self.map_f64(
                    x0 + (x1 - x0) * t,
                    y0 + (y1 - y0) * t,
                    Way::Forward,
                    Report::Conversion,
                )?;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        Ok([
            F::from_f64(min_x)?,
            F::from_f64(min_y)?,
            F::from_f64(max_x)?,
            F::from_f64(max_y)?,
        ])
    }

    /// PROJJSON stored for an EPSG code, or the PROJJSON text passed to [`Self::new`].
    ///
    /// `multiline`, `indentation_width`, and `schema` are accepted and ignored:
    /// the text is returned as stored. A CRS-to-CRS transform has none.
    ///
    /// # Errors
    ///
    /// [`ProjError::ExportToJson`] when this object was not loaded from a
    /// PROJJSON string and `proj.db` has no PROJJSON column or value for it.
    pub fn to_projjson(
        &self,
        _multiline: Option<bool>,
        _indentation_width: Option<usize>,
        _schema: Option<&str>,
    ) -> Result<String, ProjError> {
        crate::crs::stored_projjson(&self.definition).ok_or(ProjError::ExportToJson)
    }

    /// WKT stored for an EPSG code, or the WKT text passed to [`Self::new`].
    ///
    /// `version` and `options` select a dialect this port does not write. When
    /// a stored string exists it is returned unchanged.
    ///
    /// # Errors
    ///
    /// [`ProjError::ExportToWkt`] when this object was not loaded from WKT and
    /// `proj.db` has no WKT column or value for it.
    pub fn as_wkt(
        &self,
        _version: Option<WktVersion>,
        _options: Option<WktOptions>,
    ) -> Result<String, ProjError> {
        crate::crs::stored_wkt(&self.definition).ok_or(ProjError::ExportToWkt)
    }

    fn has_inverse(&self) -> bool {
        match &self.body {
            Body::Single(_) => false,
            Body::Native(op) => {
                !matches!(op.inverse(Xyzt::xy(0.0, 0.0)), Err(StepError::NoInverse))
            }
            Body::Between {
                source,
                target,
                bridge,
            } => {
                direction_exists(source, target, bridge.as_ref(), Way::Forward)
                    && direction_exists(target, source, bridge.as_ref(), Way::Inverse)
            }
        }
    }

    fn fill_array<'a, C, F>(&self, points: &'a mut [C], way: Way) -> Result<&'a mut [C], ProjError>
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
        way: Way,
        report: Report,
    ) -> Result<(f64, f64), ProjError> {
        self.map_f64(x.into_f64()?, y.into_f64()?, way, report)
    }

    fn map_f64(&self, x: f64, y: f64, way: Way, report: Report) -> Result<(f64, f64), ProjError> {
        let coord = self.map_xyz(x, y, 0.0, way, report)?;
        Ok((coord.x, coord.y))
    }

    fn map_xyz(&self, x: f64, y: f64, z: f64, way: Way, report: Report) -> Result<Xyzt, ProjError> {
        let mapped = match &self.body {
            Body::Single(_) => {
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
                let coord = Xyzt { x, y, z, t: 0.0 };
                match way {
                    Way::Inverse => op.inverse(coord),
                    Way::Forward => op.forward(coord),
                }
            }
            Body::Between {
                source,
                target,
                bridge,
            } => {
                let (source, target, bridge_way) = match way {
                    Way::Inverse => (target, source, Way::Inverse),
                    Way::Forward => (source, target, Way::Forward),
                };
                source.to_geodetic(x, y, z).and_then(|coord| {
                    apply_bridge(bridge.as_ref(), coord, bridge_way)
                        .and_then(|coord| target.geodetic_out(coord))
                })
            }
        };
        mapped.map_err(|error| match report {
            Report::Projection => ProjError::Projection(error.to_string()),
            Report::Conversion => ProjError::Conversion(error.to_string()),
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
    /// Built-in data directories stay in front. The added path is searched
    /// after them, then the download cache.
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

#[derive(Clone, Copy)]
enum Way {
    Forward,
    Inverse,
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
    let known = matches!(body, Body::Between { .. });
    Proj {
        body,
        definition,
        area,
        known,
        net: crate::network::context(),
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

/// A sample point outside the domain still means the inverse exists.
fn direction_exists(source: &Crs, target: &Crs, bridge: Option<&Operation>, way: Way) -> bool {
    !matches!(
        source.to_geodetic(0.0, 0.0, 0.0).and_then(|coord| {
            apply_bridge(bridge, coord, way).and_then(|coord| target.geodetic_out(coord))
        }),
        Err(StepError::NoInverse)
    )
}

fn apply_bridge(bridge: Option<&Operation>, coord: Xyzt, way: Way) -> Result<Xyzt, StepError> {
    let Some(bridge) = bridge else {
        return Ok(coord);
    };
    match way {
        Way::Inverse => bridge.inverse(coord),
        Way::Forward => bridge.forward(coord),
    }
}
