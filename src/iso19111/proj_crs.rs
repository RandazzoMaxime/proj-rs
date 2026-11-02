//! CRS-level keys of a PROJ string that the geographic and geocentric importers
//! share: `+title=`, and `+proj=geocent`, the geocentric CRS of
//! `buildGeocentricCRS`. Units and vertical axes are in `bound_import`, and the
//! projected CRS is in `projstring`.

use super::bound_import::build_unit;
use super::model::{
    CartesianCS, Crs, FactoryError, GeocentricCRS, GeodeticCRS, GeodeticReferenceFrame, Identified,
};
use super::projstring::{datum_from_proj, token};

type Tokens<'a> = &'a [(String, String)];

/// The `+title=` of the string, when it is not empty.
fn title(tokens: Tokens<'_>) -> Option<&str> {
    let text = token(tokens, "title");
    (!text.is_empty()).then_some(text)
}

/// The name of a geographic or geocentric CRS: `+title=`, or `unknown`.
pub(super) fn title_or_unknown(tokens: Tokens<'_>) -> &str {
    title(tokens).unwrap_or("unknown")
}

/// The datum of a geographic or geocentric step. `+title=` names it, and so
/// does `+nadgrids`, as `buildDatum` does.
pub(super) fn titled_datum(
    frame: GeodeticReferenceFrame,
    tokens: Tokens<'_>,
) -> GeodeticReferenceFrame {
    named_datum(frame, tokens, title(tokens))
}

/// The datum of a projected CRS. Its base geographic step has no title, so
/// `+nadgrids` or `+towgs84` changes the name.
pub(super) fn grid_datum(
    frame: GeodeticReferenceFrame,
    tokens: Tokens<'_>,
) -> GeodeticReferenceFrame {
    named_datum(frame, tokens, None)
}

/// `buildDatum` and `createGRF`: a datum built from an ellipsoid (`+R`,
/// `+ellps`, `+a` with a shape) takes the title when there is one. Otherwise
/// `+nadgrids` appends ` using nadgrids=<file>`, else `+towgs84` appends
/// ` using towgs84=<values>`. A named `+datum=` keeps its own name.
fn named_datum(
    frame: GeodeticReferenceFrame,
    tokens: Tokens<'_>,
    title: Option<&str>,
) -> GeodeticReferenceFrame {
    let from_ellipsoid = ["R", "ellps", "a", "b", "rf", "f", "e", "es"]
        .iter()
        .any(|key| !token(tokens, key).is_empty());
    if !from_ellipsoid || !token(tokens, "datum").is_empty() {
        return frame;
    }
    let grids = token(tokens, "nadgrids");
    let towgs84 = token(tokens, "towgs84");
    let name = match title {
        Some(title) => title.to_owned(),
        None if !grids.is_empty() => format!("{} using nadgrids={grids}", frame.name()),
        None if !towgs84.is_empty() => format!("{} using towgs84={towgs84}", frame.name()),
        None => return frame,
    };
    let mut frame = frame;
    frame.identified = Identified::named(name);
    frame
}

/// How `buildProjectedCRS` treats a `+proj=merc` string that is the EPSG
/// pseudo-Mercator method (1024): the web Mercator CRS of `createPseudoMercator`,
/// or a pseudo-Mercator on another origin or ellipsoid.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PseudoMercator {
    Web,
    Other,
}

/// The pseudo-Mercator case of `+proj=merc`: `+a` and `+b` are equal, `+lat_ts`
/// is zero or absent, `+k` is 1, and `+nadgrids=@null`. It is the web Mercator
/// when `+a` is 6378137 and `+lon_0`, `+lat_0`, `+x_0` and `+y_0` are zero.
pub(super) fn pseudo_mercator(tokens: Tokens<'_>) -> Option<PseudoMercator> {
    let number = |key: &str| token(tokens, key).parse::<f64>().ok();
    let a = token(tokens, "a");
    let same_axes = !a.is_empty() && a == token(tokens, "b");
    let unit_scale = number("k").or_else(|| number("k_0")) == Some(1.0);
    let zero_lat_ts = number("lat_ts").is_none_or(|lat_ts| lat_ts == 0.0);
    let grids_null = token(tokens, "nadgrids") == "@null";
    if token(tokens, "proj") != "merc" || !same_axes || !unit_scale || !zero_lat_ts || !grids_null {
        return None;
    }
    let zero = |key: &str| number(key).is_none_or(|value| value == 0.0);
    let web = number("a") == Some(6_378_137.0)
        && zero("lon_0")
        && zero("lat_0")
        && zero("x_0")
        && zero("y_0");
    Some(if web {
        PseudoMercator::Web
    } else {
        PseudoMercator::Other
    })
}

/// `buildGeocentricCRS`: `+proj=geocent` is a geocentric CRS on the datum of the
/// string, with X, Y, Z in `+units` or `+to_meter` (the metre by default). A
/// `+towgs84` or `+nadgrids` makes a bound CRS in `bound_import`.
///
/// # Errors
///
/// The datum or the linear unit is not known.
pub(super) fn geocentric_from_proj_string(tokens: Tokens<'_>) -> Result<Crs, FactoryError> {
    let datum = titled_datum(datum_from_proj(tokens)?, tokens);
    let unit = build_unit(tokens, "units", "to_meter")?;
    Ok(Crs::Geodetic(GeodeticCRS::Geocentric(GeocentricCRS {
        identified: Identified::named(title_or_unknown(tokens)),
        datum: Some(datum),
        ensemble: None,
        coordinate_system: CartesianCS::create_geocentric(unit),
    })))
}
