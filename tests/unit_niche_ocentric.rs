//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the five `niche` cases on CRS of other bodies
//! (IAU: planetocentric latitude, west-positive planetographic longitude) that were open.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected PROJ strings and WKT
//! are copied from that file, never from the port.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, AuthorityFactory, CoordinateOperation,
    CoordinateOperationFactory, Crs, DatabaseContext, ProjLines,
};

/// `exportToPROJString(PROJStringFormatter::create().get())` of an operation.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, createOperation_bound_of_spherical_ocentric_to_same_type)`
#[test]
fn operation_bound_of_spherical_ocentric_to_same_type() {
    let db = DatabaseContext::create().expect("proj.db");
    let src =
        create_crs_from_proj_string("+proj=longlat +geoc +ellps=GRS80 +towgs84=1,2,3 +type=crs")
            .expect("source");
    assert!(matches!(src, Crs::Bound(_)));
    let dest =
        create_crs_from_proj_string("+proj=longlat +geoc +ellps=clrk66 +towgs84=4,5,6 +type=crs")
            .expect("target");
    assert!(matches!(dest, Crs::Bound(_)));

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=geoc +ellps=GRS80 \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=-3 +y=-3 +z=-3 \
         +step +inv +proj=cart +ellps=clrk66 \
         +step +proj=pop +v_3 \
         +step +proj=geoc +ellps=clrk66 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation, createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west)`
#[test]
fn operation_spherical_ocentric_spherical_to_ellipsoidal_north_west() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = create_from_user_input(
        r#"GEODCRS["Mercury (2015) - Sphere / Ocentric",
    DATUM["Mercury (2015) - Sphere",
        ELLIPSOID["Mercury (2015) - Sphere",2440530,0,
            LENGTHUNIT["metre",1]]],
    PRIMEM["Reference Meridian",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[spherical,2],
        AXIS["planetocentric latitude (U)",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433]],
        AXIS["planetocentric longitude (V)",east,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433]]]"#,
        &db,
    )
    .expect("source");
    let dest = create_from_user_input(
        r#"GEOGCRS["Mercury (2015) / Ographic",
    DATUM["Mercury (2015)",
        ELLIPSOID["Mercury (2015)",2440530,1075.12334801762,
            LENGTHUNIT["metre",1]]],
    PRIMEM["Reference Meridian",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[ellipsoidal,2],
        AXIS["latitude",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433]],
        AXIS["longitude",west,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433]]]"#,
        &db,
    )
    .expect("target");

    let forward = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(proj(&db, &forward), "+proj=axisswap +order=1,-2");

    let backward = CoordinateOperationFactory::create()
        .create_operation(&dest, &src)
        .expect("operation")
        .expect("some operation");
    assert_eq!(proj(&db, &backward), "+proj=axisswap +order=1,-2");
}

/// `TEST(operation, createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west)`
#[test]
fn operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west() {
    let db = DatabaseContext::create().expect("proj.db");
    let authority = AuthorityFactory::create(&db, "IAU_2015");
    let src = authority
        .create_coordinate_reference_system("19901")
        .expect("19901");
    let dest = authority
        .create_coordinate_reference_system("19911")
        .expect("19911");

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=axisswap +order=-2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260 \
         +step +proj=axisswap +order=-1,2"
    );

    let inverse = CoordinateOperationFactory::create()
        .create_operation(&dest, &src)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &inverse),
        "+proj=pipeline \
         +step +inv +proj=axisswap +order=-1,2 \
         +step +inv +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,-1"
    );
}

/// `TEST(operation, createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric)`
#[test]
fn operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric() {
    let db = DatabaseContext::create().expect("proj.db");
    let authority = AuthorityFactory::create(&db, "IAU_2015");
    let src = authority
        .create_coordinate_reference_system("19901")
        .expect("19901");
    let dest = authority
        .create_coordinate_reference_system("19912")
        .expect("19912");

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=axisswap +order=-2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260"
    );

    let inverse = CoordinateOperationFactory::create()
        .create_operation(&dest, &src)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &inverse),
        "+proj=pipeline \
         +step +inv +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,-1"
    );
}

/// `TEST(operation, createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west)`
#[test]
fn operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west() {
    let db = DatabaseContext::create().expect("proj.db");
    let authority = AuthorityFactory::create(&db, "IAU_2015");
    let src = authority
        .create_coordinate_reference_system("19902")
        .expect("19902");
    let dest = authority
        .create_coordinate_reference_system("19911")
        .expect("19911");

    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &op),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +inv +proj=geoc +a=2440530 +b=2438260 \
         +step +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260 \
         +step +proj=axisswap +order=-1,2"
    );

    let inverse = CoordinateOperationFactory::create()
        .create_operation(&dest, &src)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        proj(&db, &inverse),
        "+proj=pipeline \
         +step +inv +proj=axisswap +order=-1,2 \
         +step +inv +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 \
         +a=2440530 +b=2438260 \
         +step +proj=geoc +a=2440530 +b=2438260 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
