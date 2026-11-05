//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `vertCRS_to_vertCRS_pivot_context`.

use proj_rs::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    ProjLines, SpatialCriterion,
};

/// `TEST(operation, vertCRS_to_vertCRS_pivot_context)`
#[test]
fn operation_vert_crs_to_vert_crs_pivot_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let f = AuthorityFactory::create(&db, "EPSG");
    let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let check = |a: &str, b: &str, expected: &str| {
        let list = CoordinateOperationFactory::create()
            .create_operations(
                &f.create_coordinate_reference_system(a).expect("a"),
                &f.create_coordinate_reference_system(b).expect("b"),
                &ctx,
            )
            .expect("ops");
        assert!(!list.is_empty(), "{a} {b}");
        assert!(!list[0].has_ballpark_transformation(), "{a} {b}");
        assert_eq!(
            list[0]
                .export_to_proj_string_with_database(&db, ProjLines::Single)
                .expect("p"),
            expected,
            "{a} {b}"
        );
    };
    check(
        "5705",
        "5706",
        "+proj=pipeline +step +proj=geogoffset +dh=28 +step +proj=axisswap +order=1,2,-3",
    );
    check(
        "5706",
        "5705",
        "+proj=pipeline +step +proj=geogoffset +dh=28 +step +proj=axisswap +order=1,2,-3",
    );
    check("5790", "5614", "+proj=pipeline +step +proj=geogoffset +dh=-4.74 +step +proj=axisswap +order=1,2,-3 +step +proj=unitconvert +z_in=m +z_out=ft");
    check("5614", "5790", "+proj=pipeline +step +proj=unitconvert +z_in=ft +z_out=m +step +proj=axisswap +order=1,2,-3 +step +proj=geogoffset +dh=4.74");
    check(
        "5705",
        "5336",
        "+proj=pipeline +step +proj=geogoffset +dh=0.4 +step +proj=axisswap +order=1,2,-3",
    );
    check(
        "5336",
        "5705",
        "+proj=pipeline +step +proj=axisswap +order=1,2,-3 +step +proj=geogoffset +dh=-0.4",
    );
}
