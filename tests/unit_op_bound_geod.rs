//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, bound CRS to a geodetic CRS that is not
//! geographic: the geographic 3D intermediate on the target datum, and the `push v_3` that a
//! 3D pair does not write.

use proj_rs::iso19111::{
    AuthorityFactory, BoundCRS, CoordinateOperationContext, CoordinateOperationFactory, Crs,
    DatabaseContext, GeodeticCRS, GeographicCRS, ProjLines,
};

/// `TEST(operation, boundCRS_of_geogCRS_to_geodCRS)`
#[test]
fn operation_bound_crs_of_geog_crs_to_geod_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let bound = BoundCRS::from_towgs84(
        Crs::Geographic(GeographicCRS::epsg_4807()),
        &[1., 2., 3., 4., 5., 6., 7.],
    )
    .expect("bound");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &Crs::Bound(bound),
            &Crs::Geodetic(GeodeticCRS::epsg_4978()),
            &ctxt,
        )
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=cart +ellps=clrk80ign +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector"
    );
}

/// `TEST(operation, boundCRS_of_geogCRS_to_geodCRS_not_related_to_hub)`
#[test]
fn operation_bound_crs_of_geog_crs_to_geod_crs_not_related_to_hub() {
    let db = DatabaseContext::create().expect("proj.db");
    let bound = BoundCRS::from_towgs84(
        Crs::Geographic(GeographicCRS::epsg_4807()),
        &[1., 2., 3., 4., 5., 6., 7.],
    )
    .expect("bound");
    let target = AuthorityFactory::create(&db, "EPSG")
        .create_geodetic_crs("4936")
        .expect("ETRS89 geocentric");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&Crs::Bound(bound), &Crs::Geodetic(target), &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=grad +xy_out=rad +step +inv +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=push +v_3 +step +proj=cart +ellps=clrk80ign +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector +step +inv +proj=cart +ellps=GRS80 +step +proj=pop +v_3 +step +proj=cart +ellps=GRS80"
    );
}
