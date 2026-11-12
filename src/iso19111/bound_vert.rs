//! `createOperationsBoundToVert` of PROJ's `coordinateoperationfactory.cpp` (5957-5978): a bound
//! vertical CRS to a vertical CRS. The binding transformation itself when the target is equivalent
//! to the hub; otherwise the base CRS of the source to the target.

use super::bound_ops::with_endpoints;
use super::equiv::Equivalence;
use super::factory::{CoordinateOperationContext, CoordinateOperationFactory};
use super::model::{BoundCRS, CoordinateOperation, Crs};
use super::FactoryError;

pub(super) fn bound_to_vert(
    factory: &CoordinateOperationFactory,
    bound: &BoundCRS,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let base_is_vertical = matches!(bound.source_crs(), Crs::Vertical(_));
    let hub_is_vertical = matches!(bound.target_crs(), Crs::Vertical(_));
    if base_is_vertical
        && hub_is_vertical
        && target.is_equivalent_to_with(bound.target_crs(), Equivalence::Equivalent)
    {
        return Ok(vec![with_endpoints(bound)]);
    }
    factory.create_candidates(bound.source_crs(), target, context)
}
