//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, NTF (Paris) to RGF93 v1 with intermediate
//! CRS allowed. The grid of a reversed step is written as the inverse of its PROJ grid
//! (`inverse_direction` in `grid_alternatives`), which is the second operation's pipeline.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, GridAvailabilityUse, IntermediateCrsUse, ProjLines,
};

/// `TEST(operation, geogCRS_to_geogCRS_context_concatenated_operation)`
#[test]
fn operation_geog_crs_to_geog_crs_context_concatenated_operation() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt.set_intermediate_crs_use(IntermediateCrsUse::Allow);
    let source = epsg
        .create_coordinate_reference_system("4807")
        .expect("NTF(Paris)");
    let target = epsg
        .create_coordinate_reference_system("4171")
        .expect("RGF93");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 2);

    assert_eq!(list[0].name(), "NTF (Paris) to ETRS89-FRA [RGF93 v1] (1)");
    assert_eq!(
        plain(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=grad +xy_out=rad \
         +step +inv +proj=longlat +ellps=clrk80ign +pm=paris \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=clrk80ign \
         +step +proj=xyzgridshift +grids=fr_ign_gr3df97a.tif \
         +grid_ref=output_crs +ellps=GRS80 \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );

    assert_eq!(list[1].name(), "NTF (Paris) to ETRS89-FRA [RGF93 v1] (2)");
    assert_eq!(
        plain(&db, &list[1]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=grad +xy_out=rad \
         +step +inv +proj=longlat +ellps=clrk80ign +pm=paris \
         +step +proj=hgridshift +grids=fr_ign_ntf_r93.tif \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );

    assert!(matches!(list[0], CoordinateOperation::Concatenated(_)));
    let grids = list[0]
        .grids_needed(&db, GridAvailabilityUse::UseForSorting)
        .expect("grids");
    assert_eq!(grids.len(), 1);
}

fn plain(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}
