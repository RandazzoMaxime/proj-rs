use crate::api::{Context, Error};
use crate::iso19111::{self, GeodeticCRS, JsonFormat, JsonLines, WktConvention, WktFormatter};

/// A bounding box in degrees, in the order of ISO 19115: west, south, east, north.
///
/// A box that crosses the antimeridian has `west > east`.
///
/// ```
/// use proj_rs::BBox;
///
/// let bbox = BBox::new(-180.0, -90.0, 180.0, 90.0);
/// assert_eq!(bbox.north, 90.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    /// Western longitude.
    pub west: f64,
    /// Southern latitude.
    pub south: f64,
    /// Eastern longitude.
    pub east: f64,
    /// Northern latitude.
    pub north: f64,
}

impl BBox {
    /// A box from its four sides, in degrees.
    ///
    /// ```
    /// use proj_rs::BBox;
    ///
    /// let bbox = BBox::new(-5.0, 41.0, 10.0, 52.0);
    /// assert_eq!((bbox.west, bbox.south, bbox.east, bbox.north), (-5.0, 41.0, 10.0, 52.0));
    /// ```
    #[must_use]
    pub const fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }
}

/// The kind of a [`Crs`].
///
/// The enum is `#[non_exhaustive]`.
///
/// ```
/// use proj_rs::{Context, Crs, CrsKind};
///
/// let crs = Crs::from_user_input(&Context::default(), "EPSG:2154").unwrap();
/// assert_eq!(crs.kind(), CrsKind::Projected);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CrsKind {
    /// Latitude, longitude (and height) on an ellipsoid.
    Geographic,
    /// Cartesian axes from the centre of the Earth.
    Geocentric,
    /// A map projection of a geographic CRS.
    Projected,
    /// A height or a depth.
    Vertical,
    /// A horizontal CRS and a vertical one.
    Compound,
    /// A CRS bound to another by a transformation.
    Bound,
    /// A local system with no geodetic datum.
    Engineering,
    /// Any other kind: temporal, parametric, spherical, or derived.
    Other,
}

/// The WKT dialect of [`Crs::to_wkt`].
///
/// The enum is `#[non_exhaustive]`.
///
/// ```
/// use proj_rs::{Context, Crs, WktVersion};
///
/// let crs = Crs::from_user_input(&Context::default(), "EPSG:4326").unwrap();
/// assert!(crs.to_wkt(WktVersion::Wkt2019).unwrap().starts_with("GEOGCRS["));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WktVersion {
    /// WKT2:2015.
    Wkt2015,
    /// Simplified WKT2:2015.
    Wkt2015Simplified,
    /// WKT2:2019, the default.
    #[default]
    Wkt2019,
    /// Simplified WKT2:2019.
    Wkt2019Simplified,
    /// WKT1 as GDAL writes it.
    Wkt1Gdal,
    /// WKT1 as Esri writes it.
    Wkt1Esri,
}

impl WktVersion {
    fn convention(self) -> WktConvention {
        match self {
            Self::Wkt2015 => WktConvention::Wkt2_2015,
            Self::Wkt2015Simplified => WktConvention::Wkt2_2015Simplified,
            Self::Wkt2019 => WktConvention::Wkt2_2019,
            Self::Wkt2019Simplified => WktConvention::Wkt2_2019Simplified,
            Self::Wkt1Gdal => WktConvention::Wkt1Gdal,
            Self::Wkt1Esri => WktConvention::Wkt1Esri,
        }
    }
}

/// A coordinate reference system.
///
/// Built by [`Crs::from_user_input`] from an authority code, WKT, PROJJSON or a PROJ string.
/// A `Crs` is a value: clone it freely, it is `Send + Sync`.
///
/// ```
/// use proj_rs::{Context, Crs};
///
/// let crs = Crs::from_user_input(&Context::default(), "EPSG:4326").unwrap();
/// assert_eq!(crs.name(), "WGS 84");
/// ```
#[derive(Clone, Debug)]
pub struct Crs {
    pub(crate) object: iso19111::Crs,
    pub(crate) context: Context,
}

impl Crs {
    /// Read a CRS from a code (`EPSG:4326`, `IGNF:LAMB93`), a name, WKT1 or WKT2, PROJJSON, or
    /// a PROJ string (`+proj=utm +zone=31 +type=crs`), as PROJ's `proj_create` does.
    ///
    /// The `Crs` keeps `context`, and builds its [`Transformer`](crate::Transformer)s under it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] when `input` is empty or holds a nul byte, and [`Error::Crs`]
    /// when it names no CRS this crate can read.
    ///
    /// ```
    /// use proj_rs::{Context, Crs};
    ///
    /// let ctx = Context::default();
    /// assert!(Crs::from_user_input(&ctx, "EPSG:32631").is_ok());
    /// assert!(Crs::from_user_input(&ctx, "").is_err());
    /// ```
    pub fn from_user_input(context: &Context, input: &str) -> Result<Self, Error> {
        if input.trim().is_empty() {
            return Err(Error::InvalidInput("the CRS text is empty".to_owned()));
        }
        if input.contains('\0') {
            return Err(Error::InvalidInput(
                "the CRS text contains a nul byte".to_owned(),
            ));
        }
        let object = crate::network::with_context(&context.net, || {
            context.legacy.create_from_user_input(input)
        })
        .map_err(|error| Error::Crs(error.to_string()))?;
        Ok(Self {
            object,
            context: context.clone(),
        })
    }

    /// The name of the CRS, `WGS 84` for `EPSG:4326`.
    ///
    /// ```
    /// use proj_rs::{Context, Crs};
    ///
    /// let crs = Crs::from_user_input(&Context::default(), "EPSG:2154").unwrap();
    /// assert_eq!(crs.name(), "ETRS89-FRA [RGF93 v1] / Lambert-93");
    /// ```
    #[must_use]
    pub fn name(&self) -> &str {
        self.object.name()
    }

    /// The kind of the CRS.
    ///
    /// ```
    /// use proj_rs::{Context, Crs, CrsKind};
    ///
    /// let crs = Crs::from_user_input(&Context::default(), "EPSG:4326").unwrap();
    /// assert_eq!(crs.kind(), CrsKind::Geographic);
    /// ```
    #[must_use]
    pub fn kind(&self) -> CrsKind {
        match &self.object {
            iso19111::Crs::Geographic(_) | iso19111::Crs::Geodetic(GeodeticCRS::Geographic(_)) => {
                CrsKind::Geographic
            }
            iso19111::Crs::Geodetic(GeodeticCRS::Geocentric(_)) => CrsKind::Geocentric,
            iso19111::Crs::Projected(_) => CrsKind::Projected,
            iso19111::Crs::Vertical(_) => CrsKind::Vertical,
            iso19111::Crs::Compound(_) => CrsKind::Compound,
            iso19111::Crs::Bound(_) => CrsKind::Bound,
            iso19111::Crs::Engineering(_) => CrsKind::Engineering,
            _ => CrsKind::Other,
        }
    }

    /// The area where the CRS is valid, from its first usage in `proj.db`.
    ///
    /// `None` when the CRS has no bounding box, as a CRS read from a PROJ string has none.
    ///
    /// ```
    /// use proj_rs::{Context, Crs};
    ///
    /// let crs = Crs::from_user_input(&Context::default(), "EPSG:2154").unwrap();
    /// let bbox = crs.area_of_use().unwrap();
    /// assert!(bbox.west < bbox.east && bbox.south < bbox.north);
    /// ```
    #[must_use]
    pub fn area_of_use(&self) -> Option<BBox> {
        let bbox = self.object.domains().first()?.bounding_box()?;
        Some(BBox::new(
            bbox.west_bound_longitude(),
            bbox.south_bound_latitude(),
            bbox.east_bound_longitude(),
            bbox.north_bound_latitude(),
        ))
    }

    /// The description of the area where the CRS is valid, `France - onshore and offshore,
    /// mainland and Corsica (France métropolitaine including Corsica).` for `EPSG:2154`.
    ///
    /// ```
    /// use proj_rs::{Context, Crs};
    ///
    /// let crs = Crs::from_user_input(&Context::default(), "EPSG:2154").unwrap();
    /// assert!(crs.area_name().unwrap().starts_with("France"));
    /// ```
    #[must_use]
    pub fn area_name(&self) -> Option<&str> {
        self.object.domains().first()?.area()
    }

    /// The WKT of the CRS in `version`.
    ///
    /// # Errors
    ///
    /// [`Error::Export`] when the CRS cannot be written in that dialect (WKT1 has no
    /// construct for some CRS kinds).
    ///
    /// ```
    /// use proj_rs::{Context, Crs, WktVersion};
    ///
    /// let crs = Crs::from_user_input(&Context::default(), "EPSG:4326").unwrap();
    /// let wkt = crs.to_wkt(WktVersion::Wkt1Gdal).unwrap();
    /// assert!(wkt.starts_with("GEOGCS[\"WGS 84\""));
    /// ```
    pub fn to_wkt(&self, version: WktVersion) -> Result<String, Error> {
        let db = self.database()?;
        let mut formatter = WktFormatter::create(version.convention()).with_database(db);
        self.object
            .export_to_wkt(&mut formatter)
            .map_err(|error| Error::Export(error.to_string()))
    }

    /// The PROJJSON of the CRS, in PROJ's default layout.
    ///
    /// # Errors
    ///
    /// [`Error::Export`] when the CRS kind has no PROJJSON export.
    ///
    /// ```
    /// use proj_rs::{Context, Crs};
    ///
    /// let crs = Crs::from_user_input(&Context::default(), "EPSG:4326").unwrap();
    /// assert!(crs.to_projjson().unwrap().contains("\"type\": \"GeographicCRS\""));
    /// ```
    pub fn to_projjson(&self) -> Result<String, Error> {
        let db = self.database()?;
        self.object
            .export_to_json_with(&db, &JsonFormat::with_lines(JsonLines::Multi))
            .map_err(|error| Error::Export(error.to_string()))
    }

    /// The PROJ string of the CRS, with `+type=crs`.
    ///
    /// # Errors
    ///
    /// [`Error::Export`] when the CRS kind has no PROJ string, an engineering CRS for
    /// example.
    ///
    /// ```
    /// use proj_rs::{Context, Crs};
    ///
    /// let crs = Crs::from_user_input(&Context::default(), "EPSG:32631").unwrap();
    /// let text = crs.to_proj_string().unwrap();
    /// assert!(text.contains("+proj=utm") && text.contains("+zone=31"));
    /// ```
    pub fn to_proj_string(&self) -> Result<String, Error> {
        let db = self.database()?;
        self.object
            .export_to_proj_string_with_database(&db)
            .map_err(|error| Error::Export(error.to_string()))
    }

    fn database(&self) -> Result<iso19111::DatabaseContext, Error> {
        self.context
            .legacy
            .database()
            .map_err(|error| Error::Export(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crs_is_send_and_sync() {
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<Crs>();
    }
}
