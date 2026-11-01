//! PROJ strings of a bound CRS and of a compound CRS:
//! `BoundCRS::_exportToPROJString` and `CompoundCRS::_exportToPROJString`.

use super::bound::{extract_geographic, hub_is_wgs84};
use super::model::{
    BoundCRS, CompoundCRS, CoordinateOperation, Crs, FactoryError, GeographicCRS, Transformation,
};
use super::projstring::ProjExtras;

/// What every CRS string ends with. A compound CRS writes it once.
const TAIL: &str = " +no_defs +type=crs";

/// `BoundCRS::_exportToPROJString`: the base CRS, carrying the geoid grid of a
/// vertical base bound to a geographic 3D hub, else the `+nadgrids` of a grid
/// transformation to a WGS 84 hub, else the `+towgs84` of a Helmert one.
///
/// `compound_geographic` is the geographic CRS of the compound CRS the bound
/// vertical CRS belongs to: a hub of the same name is written
/// `+geoid_crs=horizontal_crs`.
///
/// # Errors
///
/// The base CRS has no PROJ string, or a Helmert transformation lacks a parameter.
pub(super) fn export_bound(
    bound: &BoundCRS,
    compound_geographic: Option<&GeographicCRS>,
) -> Result<String, FactoryError> {
    let hub = bound.target_crs();
    let transformation = match bound.transformation() {
        CoordinateOperation::Transformation(transformation) => Some(transformation),
        _ => None,
    };
    let mut extras = ProjExtras::default();
    if let (Crs::Vertical(_), Some(transformation)) = (bound.source_crs(), transformation) {
        if let Some(grid) = transformation.height_to_geographic_3d_filename() {
            if hub.name().eq_ignore_ascii_case("WGS 84") {
                extras.geoid_grid = Some(grid);
                extras.geoid_crs = "WGS84";
            } else if compound_geographic
                .is_some_and(|geographic| hub.name().eq_ignore_ascii_case(geographic.name()))
            {
                extras.geoid_grid = Some(grid);
                extras.geoid_crs = "horizontal_crs";
            }
        }
    }
    if extras.geoid_grid.is_none() {
        extras.nadgrids = horizontal_grids(hub, transformation);
    }
    if extras.geoid_grid.is_none() && extras.nadgrids.is_none() && hub_is_wgs84(hub) {
        extras.towgs84 = match transformation {
            Some(transformation) => transformation.towgs84_strict()?,
            None => None,
        };
    }
    bound.source_crs().export_with(&extras)
}

/// `getHDatumPROJ4GRIDS`: the grid of a transformation to a WGS 84 hub.
fn horizontal_grids<'a>(hub: &Crs, transformation: Option<&'a Transformation>) -> Option<&'a str> {
    if !hub.name().eq_ignore_ascii_case("WGS 84") {
        return None;
    }
    transformation?.nadgrids_filename()
}

/// `CompoundCRS::_exportToPROJString`: each component writes its parameters into
/// one string, which ends once with `+no_defs +type=crs`.
///
/// # Errors
///
/// A component has no PROJ string.
pub(super) fn export_compound(compound: &CompoundCRS) -> Result<String, FactoryError> {
    let geographic = compound.components().first().and_then(extract_geographic);
    let mut parts = Vec::new();
    for component in compound.components() {
        let text = match component {
            Crs::Bound(bound) => export_bound(bound, geographic)?,
            other => other.export_to_proj_string()?,
        };
        parts.push(text.strip_suffix(TAIL).unwrap_or(&text).to_owned());
    }
    Ok(format!("{}{TAIL}", parts.join(" ")))
}
