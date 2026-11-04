//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, bound CRS to bound CRS: the TOWGS84 pair
//! gives the NTF to NAD83 pipeline, and an unrelated hub is ignored.

use proj_rs::iso19111::{
    BoundCRS, CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    GeographicCRS, ProjLines, Transformation,
};

fn props() -> proj_rs::iso19111::OpProps {
    proj_rs::iso19111::OpProps::new()
}

/// `TEST(operation, boundCRS_to_boundCRS_noop_for_TOWGS84)`
#[test]
fn operation_bound_crs_to_bound_crs_noop_for_towgs84() {
    let db = DatabaseContext::create().expect("proj.db");
    let b1 = BoundCRS::from_towgs84(
        Crs::Geographic(GeographicCRS::epsg_4807()),
        &[1., 2., 3., 4., 5., 6., 7.],
    )
    .expect("b1");
    let b2 = BoundCRS::from_towgs84(
        Crs::Geographic(GeographicCRS::epsg_4269()),
        &[1., 2., 3., 4., 5., 6., 7.],
    )
    .expect("b2");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&Crs::Bound(b1), &Crs::Bound(b2), &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0].export_to_proj_string_with_database(&db, ProjLines::Single).expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=push +v_3 +step +proj=cart +ellps=clrk80ign +step +inv +proj=cart +ellps=GRS80 +step +proj=pop +v_3 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, boundCRS_to_boundCRS_unralated_hub)`
#[test]
fn operation_bound_crs_to_bound_crs_unrelated_hub() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let b1 = BoundCRS::from_towgs84(
        Crs::Geographic(GeographicCRS::epsg_4807()),
        &[1., 2., 3., 4., 5., 6., 7.],
    )
    .expect("b1");
    let transf = Transformation::create_geocentric_translations(
        &props(),
        Crs::Geographic(GeographicCRS::epsg_4269()),
        Crs::Geographic(GeographicCRS::epsg_4979()),
        1.0,
        2.0,
        3.0,
        None,
    )
    .expect("geocentric translations");
    let b2 = BoundCRS::create(
        Crs::Geographic(GeographicCRS::epsg_4269()),
        Crs::Geographic(GeographicCRS::epsg_4979()),
        proj_rs::iso19111::CoordinateOperation::Transformation(transf),
    );
    let bound = CoordinateOperationFactory::create()
        .create_operations(&Crs::Bound(b1), &Crs::Bound(b2), &ctxt)
        .expect("bound operations");
    let base = CoordinateOperationFactory::create()
        .create_operations(
            &Crs::Geographic(GeographicCRS::epsg_4807()),
            &Crs::Geographic(GeographicCRS::epsg_4269()),
            &ctxt,
        )
        .expect("base operations");
    assert!(!bound.is_empty() && !base.is_empty());
    assert_eq!(
        bound[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("bound PROJ"),
        base[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("base PROJ")
    );
}
