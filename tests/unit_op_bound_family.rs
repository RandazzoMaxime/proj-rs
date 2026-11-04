//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, bound CRS cases built from a projected CRS,
//! a PROJ string, or a database grid: the pipeline each case exports.

use proj_rs::iso19111::{
    create_crs_from_proj_string, BoundCRS, CartesianCS, Conversion, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GeodeticCRS, GeographicCRS, OpProps,
    ProjLines, ProjectedCRS, UnitOfMeasure, WktConvention, WktFormatter, WktObject, WktParser,
};

fn utm31_bound() -> BoundCRS {
    let utm31 = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4807()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm 31"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    BoundCRS::from_towgs84(Crs::Projected(utm31), &[1., 2., 3., 4., 5., 6., 7.]).expect("bound")
}

fn bound_from_proj(text: &str) -> Crs {
    create_crs_from_proj_string(text).expect(text)
}

/// `TEST(operation, boundCRS_of_projCRS_to_geogCRS)`
#[test]
fn operation_bound_crs_of_proj_crs_to_geog_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &Crs::Bound(utm31_bound()),
            &Crs::Geographic(GeographicCRS::epsg_4326()),
            &ctxt,
        )
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=clrk80ign +pm=paris +step +proj=push +v_3 +step +proj=cart +ellps=clrk80ign +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, boundCRS_to_boundCRS)`
#[test]
fn operation_bound_crs_to_bound_crs_utm() {
    let db = DatabaseContext::create().expect("proj.db");
    let utm32 = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4269()),
        Conversion::create_utm_north(&OpProps::new(), 32).expect("utm 32"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let bound2 = BoundCRS::from_towgs84(Crs::Projected(utm32), &[8., 9., 10., 11., 12., 13., 14.])
        .expect("bound 2");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&Crs::Bound(utm31_bound()), &Crs::Bound(bound2), &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=clrk80ign +pm=paris +step +proj=push +v_3 +step +proj=cart +ellps=clrk80ign +step +proj=helmert +x=1 +y=2 +z=3 +rx=4 +ry=5 +rz=6 +s=7 +convention=position_vector +step +inv +proj=helmert +x=8 +y=9 +z=10 +rx=11 +ry=12 +rz=13 +s=14 +convention=position_vector +step +inv +proj=cart +ellps=GRS80 +step +proj=pop +v_3 +step +proj=utm +zone=32 +ellps=GRS80"
    );
}

/// `TEST(operation, boundCRS_of_projCRS_towgs84_non_metre_unit_to_geocentric)`
#[test]
fn operation_bound_crs_of_proj_crs_towgs84_non_metre_unit_to_geocentric() {
    let db = DatabaseContext::create().expect("proj.db");
    let src =
        bound_from_proj("+proj=merc +ellps=GRS80 +towgs84=0,0,0 +units=ft +vunits=ft +type=crs");
    let dst = bound_from_proj("+proj=geocent +datum=WGS84 +type=crs");
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=ft +z_in=ft +xy_out=m +z_out=m +step +inv +proj=merc +lon_0=0 +k=1 +x_0=0 +y_0=0 +ellps=GRS80 +step +proj=cart +ellps=WGS84"
    );
}

/// `TEST(operation, createOperation_boundCRS_identified_by_datum)`
#[test]
fn operation_create_operation_bound_crs_identified_by_datum() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = bound_from_proj("+proj=longlat +datum=WGS84 +type=crs");
    let dst = bound_from_proj(
        "+proj=utm +zone=32 +a=6378249.2 +b=6356515 +towgs84=-263.0,6.0,431.0 +no_defs +type=crs",
    );
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 +step +proj=cart +ellps=WGS84 +step +proj=helmert +x=263 +y=-6 +z=-431 +step +inv +proj=cart +ellps=clrk80ign +step +proj=pop +v_3 +step +proj=utm +zone=32 +ellps=clrk80ign"
    );
}

/// `TEST(operation, boundCRS_of_projCRS_towgs84_to_boundCRS_of_projCRS_nadgrids)`
#[test]
fn operation_bound_crs_of_proj_crs_towgs84_to_bound_crs_of_proj_crs_nadgrids() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = bound_from_proj(
        "+proj=utm +zone=15 +datum=NAD83 +units=m +no_defs +ellps=GRS80 +towgs84=0,0,0 +type=crs",
    );
    let dst = bound_from_proj(
        "+proj=utm +zone=15 +datum=NAD27 +units=m +no_defs +ellps=clrk66 +nadgrids=@conus,@alaska,@ntv2_0.gsb,@ntv1_can.dat +type=crs",
    );
    let ctxt = CoordinateOperationContext::create(&db, "");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=utm +zone=15 +ellps=GRS80 +step +inv +proj=hgridshift +grids=@conus,@alaska,@ntv2_0.gsb,@ntv1_can.dat +step +proj=utm +zone=15 +ellps=clrk66"
    );
}

/// `TEST(operation, projCRS_to_projCRS_different_baseCRS)`
#[test]
fn operation_proj_crs_to_proj_crs_different_base_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let utm31 = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm 31"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let utm32 = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4807()),
        Conversion::create_utm_north(&OpProps::new(), 32).expect("utm 32"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let operation = CoordinateOperationFactory::create()
        .create_operation(&Crs::Projected(utm31), &Crs::Projected(utm32))
        .expect("operations")
        .expect("operation");
    assert_eq!(
        operation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=WGS84 +step +proj=utm +zone=32 +ellps=clrk80ign +pm=paris"
    );
}

/// `TEST(operation, boundCRS_with_basecrs_with_extent_to_geogCRS)`
#[test]
fn operation_bound_crs_with_base_crs_with_extent_to_geog_crs() {
    let wkt = r#"BOUNDCRS[
    SOURCECRS[
        PROJCRS["NAD83 / California zone 3 (ftUS)",
            BASEGEODCRS["NAD83",
                DATUM["North American Datum 1983",
                    ELLIPSOID["GRS 1980",6378137,298.257222101,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]]],
            CONVERSION["SPCS83 California zone 3 (US Survey feet)",
                METHOD["Lambert Conic Conformal (2SP)",
                    ID["EPSG",9802]],
                PARAMETER["Latitude of false origin",36.5,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8821]],
                PARAMETER["Longitude of false origin",-120.5,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8822]],
                PARAMETER["Latitude of 1st standard parallel",38.4333333333333,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8823]],
                PARAMETER["Latitude of 2nd standard parallel",37.0666666666667,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8824]],
                PARAMETER["Easting at false origin",6561666.667,
                    LENGTHUNIT["US survey foot",0.304800609601219],
                    ID["EPSG",8826]],
                PARAMETER["Northing at false origin",1640416.667,
                    LENGTHUNIT["US survey foot",0.304800609601219],
                    ID["EPSG",8827]]],
            CS[Cartesian,2],
                AXIS["easting (X)",east,
                    ORDER[1],
                    LENGTHUNIT["US survey foot",0.304800609601219]],
                AXIS["northing (Y)",north,
                    ORDER[2],
                    LENGTHUNIT["US survey foot",0.304800609601219]],
            SCOPE["unknown"],
            AREA["USA - California - SPCS - 3"],
            BBOX[36.73,-123.02,38.71,-117.83],
            ID["EPSG",2227]]],
    TARGETCRS[
        GEODCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            CS[ellipsoidal,2],
                AXIS["latitude",north,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["longitude",east,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433]],
            ID["EPSG",4326]]],
    ABRIDGEDTRANSFORMATION["NAD83 to WGS 84 (1)",
        METHOD["Geocentric translations (geog2D domain)",
            ID["EPSG",9603]],
        PARAMETER["X-axis translation",0,
            ID["EPSG",8605]],
        PARAMETER["Y-axis translation",0,
            ID["EPSG",8606]],
        PARAMETER["Z-axis translation",0,
            ID["EPSG",8607]],
        SCOPE["unknown"],
        AREA["North America - Canada and USA (CONUS, Alaska mainland)"],
        BBOX[23.81,-172.54,86.46,-47.74],
        ID["EPSG",1188]]]"#;
    let WktObject::Crs(source) = WktParser::new().create_from_wkt(wkt).expect("wkt") else {
        panic!("expected a CRS");
    };
    let operation = CoordinateOperationFactory::create()
        .create_operation(&source, &Crs::Geographic(GeographicCRS::epsg_4326()))
        .expect("operations")
        .expect("operation");
    assert_eq!(
        operation.name(),
        "Inverse of SPCS83 California zone 3 (US Survey feet) + NAD83 to WGS 84 (1)"
    );
}

/// `buildCRSFromProjStrThroughWKT`: the PROJ string as a CRS, read back from its WKT2.
fn crs_through_wkt(text: &str) -> Crs {
    let crs = create_crs_from_proj_string(text).expect(text);
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2");
    let WktObject::Crs(read) = WktParser::new().create_from_wkt(&wkt).expect("WKT2 read") else {
        panic!("expected a CRS");
    };
    read
}

/// `TEST(operation, boundCRS_to_boundCRS_with_base_geog_crs_different_from_source_of_transf)`
#[test]
fn operation_bound_crs_to_bound_crs_with_base_geog_crs_different_from_source_of_transf() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = crs_through_wkt(
        "+proj=lcc +lat_1=49 +lat_0=49 +lon_0=0 +k_0=0.999877499 +x_0=600000 +y_0=200000 +ellps=clrk80ign +pm=paris +towgs84=-168,-60,320,0,0,0,0 +units=m +no_defs +type=crs",
    );
    let dst = crs_through_wkt(
        "+proj=longlat +ellps=clrk80ign +pm=paris +towgs84=-168,-60,320,0,0,0,0 +no_defs +type=crs",
    );
    let operation = CoordinateOperationFactory::create()
        .create_operation(&src, &dst)
        .expect("operations")
        .expect("operation");
    assert_eq!(
        operation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +inv +proj=lcc +lat_1=49 +lat_0=49 +lon_0=0 +k_0=0.999877499 +x_0=600000 +y_0=200000 +ellps=clrk80ign +pm=paris +step +proj=longlat +ellps=clrk80ign +pm=paris +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}
