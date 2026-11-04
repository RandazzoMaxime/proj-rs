//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, NAD83 (EPSG:4269) to NAD83(2011) / California
//! Albers (EPSG:6414). The NADCON5 chain is the first operation, and the ballpark offset the second.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    ProjLines, SpatialCriterion,
};

/// `TEST(operation, NAD83_to_projected_CRS_based_on_NAD83_2011)`
#[test]
fn operation_nad83_to_projected_crs_based_on_nad83_2011() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let source = epsg
        .create_coordinate_reference_system("4269")
        .expect("NAD83");
    let target = epsg
        .create_coordinate_reference_system("6414")
        .expect("NAD83(2011) / California Albers");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 2);

    assert_eq!(
        list[0].name(),
        "NAD83 to NAD83(HARN) (47) + NAD83(HARN) to NAD83(FBN) (1) + \
         NAD83(FBN) to NAD83(NSRS2007) (1) + NAD83(NSRS2007) to NAD83(2011) (1) + \
         California Albers"
    );

    assert_eq!(
        list[1].name(),
        "Ballpark geographic offset from NAD83 to NAD83(2011) + California Albers"
    );
    assert_eq!(
        list[1]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=aea +lat_0=0 +lon_0=-120 +lat_1=34 \
         +lat_2=40.5 +x_0=0 +y_0=-4000000 +ellps=GRS80"
    );
}
