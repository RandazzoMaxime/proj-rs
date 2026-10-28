//! Named CRS used by [`crate::Proj::new_known_crs`].
//!
//! The table is the ported subset: EPSG:4326, 4258, 4978, 2154, 3857, and the
//! WGS84 UTM zones 32601–32660 and 32701–32760. Axis order is longitude,
//! latitude and easting, northing. There is no `proj.db` and no grid.

use crate::engine::{CreateError, Operation};
use crate::proj::ProjCreateError;
use crate::Xyzt;

/// How a CRS presents coordinates to the georust facade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// Longitude and latitude, degrees on the public boundary.
    Geographic,
    /// Easting and northing, metres.
    Projected,
    /// Geocentric X, Y, Z, metres.
    Geocentric,
}

/// One coordinate reference system the facade can build.
#[derive(Clone, Debug)]
pub(crate) struct Crs {
    kind: Kind,
    op: Operation,
}

impl Crs {
    /// Parse an `AUTHORITY:CODE` or a PROJ string that names one CRS.
    ///
    /// # Errors
    ///
    /// [`ProjCreateError`] when the code is outside the table or the PROJ
    /// string does not build.
    pub(crate) fn parse(text: &str) -> Result<Self, ProjCreateError> {
        let text = text.trim();
        if let Some(code) = authority_code(text) {
            return from_epsg(code);
        }
        if text.contains("proj=") {
            return from_proj(text);
        }
        Err(ProjCreateError::ProjError(format!("unknown CRS `{text}`")))
    }

    /// Geodetic radians on this CRS's ellipsoid, height in `z`.
    pub(crate) fn to_geodetic(
        &self,
        x: f64,
        y: f64,
    ) -> Result<Xyzt, crate::engine::TransformError> {
        match self.kind {
            Kind::Geographic => Ok(Xyzt::xy(x.to_radians(), y.to_radians())),
            Kind::Projected | Kind::Geocentric => self.op.inverse(Xyzt::xy(x, y)),
        }
    }

    /// Public coordinates of this CRS from geodetic radians.
    pub(crate) fn geodetic_out(&self, coord: Xyzt) -> Result<Xyzt, crate::engine::TransformError> {
        match self.kind {
            Kind::Geographic => Ok(Xyzt::xy(coord.x.to_degrees(), coord.y.to_degrees())),
            Kind::Projected | Kind::Geocentric => self.op.forward(coord),
        }
    }
}

fn authority_code(text: &str) -> Option<u32> {
    let (authority, code) = text.split_once(':')?;
    if !authority.eq_ignore_ascii_case("EPSG") {
        return None;
    }
    code.parse().ok()
}

fn from_epsg(code: u32) -> Result<Crs, ProjCreateError> {
    let definition: String = match code {
        4326 => "+proj=longlat +ellps=WGS84 +no_defs".to_owned(),
        4258 => "+proj=longlat +ellps=GRS80 +no_defs".to_owned(),
        4978 => "+proj=cart +ellps=WGS84 +no_defs".to_owned(),
        2154 => "+proj=lcc +lat_0=46.5 +lat_1=49 +lat_2=44 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80 +units=m +no_defs".to_owned(),
        3857 => "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1 +units=m +no_defs".to_owned(),
        zone if (32_601..=32_660).contains(&zone) => {
            format!("+proj=utm +zone={} +ellps=WGS84 +units=m +no_defs", zone - 32_600)
        }
        zone if (32_701..=32_760).contains(&zone) => {
            format!(
                "+proj=utm +zone={} +south +ellps=WGS84 +units=m +no_defs",
                zone - 32_700
            )
        }
        _ => {
            return Err(ProjCreateError::ProjError(format!(
                "EPSG:{code} is not a built-in CRS"
            )));
        }
    };
    from_proj(&definition)
}

fn from_proj(definition: &str) -> Result<Crs, ProjCreateError> {
    if definition.contains("proj=pipeline") {
        return Err(ProjCreateError::ProjError(
            "a pipeline is not a CRS".to_owned(),
        ));
    }
    let op = Operation::create(definition).map_err(|error| create_error(&error))?;
    let kind = if definition.contains("proj=cart") || definition.contains("proj=geocent") {
        Kind::Geocentric
    } else if definition.contains("proj=longlat")
        || definition.contains("proj=latlong")
        || definition.contains("proj=latlon")
        || definition.contains("proj=lonlat")
    {
        Kind::Geographic
    } else if op.angular_input(false) && !op.angular_output(false) {
        Kind::Projected
    } else {
        return Err(ProjCreateError::ProjError(format!(
            "`{definition}` is not a geographic, projected, or geocentric CRS"
        )));
    };
    Ok(Crs { kind, op })
}

fn create_error(error: &CreateError) -> ProjCreateError {
    ProjCreateError::ProjError(error.to_string())
}
