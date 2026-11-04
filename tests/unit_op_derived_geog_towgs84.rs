//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, fixture
//! `derivedGeographicCRS_with_to_wgs84_to_geographicCRS`.

use proj_rs::{
    create_crs_from_proj_string, create_from_user_input, CoordinateOperationFactory, Crs,
    DatabaseContext, GeographicCRS, WktConvention, WktFormatter,
};

const SRC: &str = "+proj=ob_tran +o_proj=latlon +lat_0=0 +lon_0=180 +o_lat_p=18.0 \
    +o_lon_p=-200.0 +ellps=WGS84 +towgs84=1,2,3 +over +type=crs";

fn forward_steps() -> &'static str {
    "+step +proj=unitconvert +xy_in=deg +xy_out=rad \
     +step +inv +proj=ob_tran +o_proj=latlon +over +lat_0=0 +lon_0=180 \
     +o_lat_p=18 +o_lon_p=-200 +ellps=WGS84 \
     +step +proj=push +v_3 +step +proj=cart +ellps=WGS84 \
     +step +proj=helmert +x=1 +y=2 +z=3 \
     +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 \
     +step +proj=unitconvert +xy_in=rad +xy_out=deg"
}

fn backward_steps() -> &'static str {
    "+step +proj=unitconvert +xy_in=deg +xy_out=rad \
     +step +proj=push +v_3 +step +proj=cart +ellps=WGS84 \
     +step +proj=helmert +x=-1 +y=-2 +z=-3 \
     +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 \
     +step +proj=ob_tran +o_proj=latlon +over +lat_0=0 +lon_0=180 \
     +o_lat_p=18 +o_lon_p=-200 +ellps=WGS84 \
     +step +proj=unitconvert +xy_in=rad +xy_out=deg"
}

fn pj(src: &Crs, dst: &Crs, db: &DatabaseContext) -> String {
    CoordinateOperationFactory::create()
        .create_operation(src, dst)
        .expect("operation")
        .expect("some operation")
        .export_to_proj_string_with_database(db, proj_rs::ProjLines::Single)
        .expect("PROJ string")
}

fn run(src: Crs) {
    let db = DatabaseContext::create().expect("proj.db");
    let dst = create_crs_from_proj_string("+proj=longlat +datum=WGS84 +type=crs").expect("dst");
    let epsg_4326 = Crs::Geographic(GeographicCRS::epsg_4326());
    let forward = format!("+proj=pipeline {}", forward_steps());
    assert_eq!(pj(&src, &dst, &db), forward);
    assert_eq!(
        pj(&src, &epsg_4326, &db),
        format!("{forward} +step +proj=axisswap +order=2,1")
    );
    let backward = format!("+proj=pipeline {}", backward_steps());
    assert_eq!(pj(&dst, &src, &db), backward);
    assert_eq!(
        pj(&epsg_4326, &src, &db),
        format!(
            "+proj=pipeline +step +proj=axisswap +order=2,1 {}",
            backward_steps()
        )
    );
}

/// `TEST_F(derivedGeographicCRS_with_to_wgs84_to_geographicCRS, src_from_proj)`
#[test]
fn derived_geographic_with_to_wgs84_src_from_proj() {
    run(create_crs_from_proj_string(SRC).expect("src"));
}

/// `TEST_F(derivedGeographicCRS_with_to_wgs84_to_geographicCRS, src_from_wkt2)`
#[test]
fn derived_geographic_with_to_wgs84_src_from_wkt2() {
    let db = DatabaseContext::create().expect("proj.db");
    let from_proj = create_crs_from_proj_string(SRC).expect("src");
    let wkt = from_proj
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt");
    run(create_from_user_input(&wkt, &db).expect("from wkt"));
}
