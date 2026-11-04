//! PROJ 9.9.0 `test/unit/test_io.cpp`, derived geographic CRS read from an ESRI PROJCS
//! that carries the PROJ4 extension.

use proj_rs::{
    create_crs_from_proj_string, CoordinateOperationFactory, Crs, DatabaseContext, Equivalence,
    ProjLines, WktParser, WktStrictness,
};

/// `TEST(wkt_parse, DerivedGeographicCRS_GDAL_PROJ4_EXSTENSION_hack)`
#[test]
fn wkt_parse_derived_geographic_crs_gdal_proj4_extension_hack() {
    let wkt = "PROJCS[\"unnamed\",\
        GEOGCS[\"unknown\",\
        DATUM[\"unnamed\",\
        SPHEROID[\"Spheroid\",6367470,594.313048347956]],\
        PRIMEM[\"Greenwich\",0],\
        UNIT[\"degree\",0.0174532925199433,\
        AUTHORITY[\"EPSG\",\"9122\"]]],\
        PROJECTION[\"Rotated_pole\"],\
        EXTENSION[\"PROJ4\",\"+proj=ob_tran +o_proj=longlat +lon_0=18 \
        +o_lon_p=0 +o_lat_p=39.25 +a=6367470 +b=6367470 \
        +to_meter=0.0174532925199 +wktext\"]]";

    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let Some(Crs::Derived(crs)) = obj.crs().cloned() else {
        panic!("not a derived geographic CRS");
    };

    let crs2 = create_crs_from_proj_string(
        "+proj=ob_tran +o_proj=longlat +lon_0=18 \
         +o_lon_p=0 +o_lat_p=39.25 +a=6367470 +b=6367470 \
         +to_meter=0.0174532925199 +wktext +type=crs",
    )
    .expect("proj string");
    assert!(matches!(crs2, Crs::Derived(_)));

    let db = DatabaseContext::create().expect("proj.db");
    let derived = Crs::Derived(crs.clone());
    assert!(derived.is_equivalent_to_with(&crs2, Equivalence::Equivalent));

    let factory = CoordinateOperationFactory::create();
    {
        let op = factory
            .create_operation(crs.base_crs(), &derived)
            .expect("operation")
            .expect("some operation");
        assert_eq!(
            op.export_to_proj_string_with_database(&db, ProjLines::Single)
                .expect("proj string"),
            "+proj=pipeline \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=ob_tran +o_proj=longlat +lon_0=18 +o_lon_p=0 \
             +o_lat_p=39.25 +R=6367470 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg"
        );
    }
    {
        let op = factory
            .create_operation(&derived, crs.base_crs())
            .expect("operation")
            .expect("some operation");
        assert_eq!(
            op.export_to_proj_string_with_database(&db, ProjLines::Single)
                .expect("proj string"),
            "+proj=pipeline \
             +step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +inv +proj=ob_tran +o_proj=longlat +lon_0=18 +o_lon_p=0 \
             +o_lat_p=39.25 +R=6367470 \
             +step +proj=unitconvert +xy_in=rad +xy_out=deg"
        );
    }
}
