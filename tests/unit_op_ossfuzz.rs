//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, a fuzzer input: the operation factory must
//! return, without a panic or an infinite recursion.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationFactory, DatabaseContext, WktObject, WktParser,
};

/// `TEST(operation, createOperation_ossfuzz_47873_simplified_if_i_might_say)`
#[test]
fn operation_ossfuzz_47873_simplified_if_i_might_say() {
    let wkt = r#"BOUNDCRS[
    SOURCECRS[
        VERTCRS["unknown",
            VDATUM["unknown using geoidgrids=@x"],
            CS[vertical,1],
                AXIS["gravity-related height (H)",up,
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]]]],
    TARGETCRS[
        GEOGCRS["unnamed",
            BASEGEOGCRS["unknown",
                DATUM["World Geodetic System 1984",
                    ELLIPSOID["WGS 84",6378137,298.257223563,
                        LENGTHUNIT["metre",1]],
                    ID["EPSG",6326]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8901]]],
            DERIVINGCONVERSION["unknown",
                METHOD["PROJ ob_tran o_proj=longlat"],
                PARAMETER["o_lat_1",1,
                    ANGLEUNIT["degree",0.0174532925199433,
                        ID["EPSG",9122]]],
                PARAMETER["o_lat_2",2,
                    ANGLEUNIT["degree",0.0174532925199433,
                        ID["EPSG",9122]]]],
            CS[ellipsoidal,3],
                AXIS["longitude",east,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433,
                        ID["EPSG",9122]]],
                AXIS["latitude",north,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433,
                        ID["EPSG",9122]]],
                AXIS["ellipsoidal height (h)",up,
                    ORDER[3],
                    LENGTHUNIT["metre",1,
                        ID["EPSG",9001]]]]],
    ABRIDGEDTRANSFORMATION["unknown to unnamed ellipsoidal height",
        METHOD["GravityRelatedHeight to Geographic3D"],
        PARAMETERFILE["Geoid (height correction) model file","@x",
            ID["EPSG",8666]]]]"#;
    let WktObject::Crs(src) = WktParser::new().create_from_wkt(wkt).expect("wkt") else {
        panic!("expected a CRS");
    };
    let db = DatabaseContext::create().expect("proj.db");
    let dst = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4979")
        .expect("4979");
    let _ = CoordinateOperationFactory::create().create_operation(&src, &dst);
}
