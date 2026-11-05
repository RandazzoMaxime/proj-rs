//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `towgs84_pm_3d`: a PROJ-string CRS with
//! `+towgs84` and `+pm`, promoted to 3D, directly and after a WKT2 round trip.

use proj_rs::{
    create_crs_from_proj_string, AuthorityFactory, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, ProjLines, WktConvention, WktFormatter,
    WktObject, WktParser,
};

/// `TEST(operation, towgs84_pm_3d)`
#[test]
fn operation_towgs84_pm_3d() {
    let db = DatabaseContext::create().expect("proj.db");
    let _ = AuthorityFactory::create(&db, "");
    let src = create_crs_from_proj_string(
        "+proj=tmerc +lat_0=0 +lon_0=34 +k=1 +x_0=0 +y_0=-5000000 \
         +ellps=bessel +pm=ferro +towgs84=1,2,3,4,5,6,7 \
         +units=m +no_defs +type=crs",
    )
    .expect("source");
    let src3d = src.promote_to_3d(Some(&db));
    let dst = create_crs_from_proj_string(
        "+proj=longlat +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +no_defs +type=crs",
    )
    .expect("target");
    let dst3d = dst.promote_to_3d(Some(&db));
    let expected = "+proj=pipeline \
        +step +inv +proj=tmerc +lat_0=0 +lon_0=34 +k=1 +x_0=0 +y_0=-5000000 \
        +ellps=bessel +pm=ferro \
        +step +proj=cart +ellps=bessel \
        +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 \
        +ry=5 +rz=6 +s=7 +convention=position_vector \
        +step +inv +proj=cart +ellps=GRS80 \
        +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m";
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src3d, &dst3d, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        expected
    );

    let from_wkt = |crs: &Crs| {
        let text = crs
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
            .expect("WKT");
        match WktParser::new().create_from_wkt(&text).expect("parse") {
            WktObject::Crs(crs) => crs,
            other => panic!("expected a CRS: {other:?}"),
        }
    };
    let list = CoordinateOperationFactory::create()
        .create_operations(&from_wkt(&src3d), &from_wkt(&dst3d), &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        expected
    );
}
