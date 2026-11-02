//! PROJ `test/unit` checks of the `niche` class (rare CRS kinds and operations) from
//! `test_operation.cpp`, `test_crs.cpp`, `test_factory.cpp` and `test_c_api.cpp`.
//!
//! Each function is one `TEST()` name. Expected text and numbers are copied from PROJ
//! 9.9.0. The cases that stay open are listed in `docs/PORTING-niche-other.md`.

use proj_rs::{
    AuthorityFactory, Conversion, CoordinateOperationFactory, Crs, DatabaseContext, OpProps,
    ProjLines, WktObject, WktParser,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

fn parse(wkt: &str) -> WktObject {
    WktParser::new().create_from_wkt(wkt).expect("wkt")
}

fn parse_db(wkt: &str) -> WktObject {
    let mut parser = WktParser::new();
    parser.attach_database(database());
    parser.create_from_wkt(wkt).expect("wkt")
}

#[test]
fn operation_vertical_perspective_export_no_topocentric_height_and_false_easting_northing() {
    let conv =
        Conversion::create_vertical_perspective(&OpProps::new(), 1.0, 2.0, 0.0, 4.0, 0.0, 0.0)
            .expect("nsper");
    assert_eq!(
        conv.export_to_proj_string().expect("PROJ"),
        "+proj=nsper +lat_0=1 +lon_0=2 +h=4 +x_0=0 +y_0=0"
    );
    // False easting and false northing are not exported when they are 0.
    let wkt = conv
        .export_to_wkt(&mut proj_rs::WktFormatter::create(
            proj_rs::WktConvention::Wkt2_2019,
        ))
        .expect("wkt");
    assert_eq!(
        wkt,
        concat!(
            "CONVERSION[\"Vertical Perspective\",\n",
            "    METHOD[\"Vertical Perspective\",\n",
            "        ID[\"EPSG\",9838]],\n",
            "    PARAMETER[\"Latitude of topocentric origin\",1,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8834]],\n",
            "    PARAMETER[\"Longitude of topocentric origin\",2,\n",
            "        ANGLEUNIT[\"degree\",0.0174532925199433],\n",
            "        ID[\"EPSG\",8835]],\n",
            "    PARAMETER[\"Ellipsoidal height of topocentric origin\",0,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8836]],\n",
            "    PARAMETER[\"Viewpoint height\",4,\n",
            "        LENGTHUNIT[\"metre\",1],\n",
            "        ID[\"EPSG\",8840]]]"
        )
    );
}

#[test]
fn operation_spilhaus_esri_export() {
    let db = database();
    // ESRI:54099 WGS_1984_Spilhaus_Ocean_Map_in_Square
    let crs = AuthorityFactory::create(&db, "ESRI")
        .create_projected_crs("54099")
        .expect("ESRI:54099");
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ"),
        "+proj=spilhaus +lat_0=-49.56371678 +lon_0=66.94970198 \
         +azi=40.17823482 +k_0=1.4142135623731 +rot=45 \
         +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn operation_geographic_topocentric() {
    let wkt = r#"PROJCRS["EPSG topocentric example A",
    BASEGEOGCRS["WGS 84",
        ENSEMBLE["World Geodetic System 1984 ensemble",
            MEMBER["World Geodetic System 1984 (Transit)"],
            MEMBER["World Geodetic System 1984 (G730)"],
            MEMBER["World Geodetic System 1984 (G873)"],
            MEMBER["World Geodetic System 1984 (G1150)"],
            MEMBER["World Geodetic System 1984 (G1674)"],
            MEMBER["World Geodetic System 1984 (G1762)"],
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]],
            ENSEMBLEACCURACY[2.0]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4979]],
    CONVERSION["EPSG topocentric example A",
        METHOD["Geographic/topocentric conversions",
            ID["EPSG",9837]],
        PARAMETER["Latitude of topocentric origin",55,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8834]],
        PARAMETER["Longitude of topocentric origin",5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8835]],
        PARAMETER["Ellipsoidal height of topocentric origin",0,
            LENGTHUNIT["metre",1],
            ID["EPSG",8836]]],
    CS[Cartesian,3],
        AXIS["topocentric East (U)",east,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["topocentric North (V)",north,
            ORDER[2],
            LENGTHUNIT["metre",1]],
        AXIS["topocentric height (W)",up,
            ORDER[3],
            LENGTHUNIT["metre",1]],
    USAGE[
        SCOPE["Example only (fictitious)."],
        AREA["Description of the extent of the CRS."],
        BBOX[-90,-180,90,180]],
    ID["EPSG",5819]]"#;
    let db = database();
    let WktObject::Crs(Crs::Projected(dst)) = parse(wkt) else {
        panic!("not a projected CRS");
    };
    let source = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4979")
        .expect("EPSG:4979");
    let op = CoordinateOperationFactory::create()
        .create_operation_proj_string(&source, &Crs::Projected(dst))
        .expect("operation");
    assert_eq!(
        op,
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=topocentric +lat_0=55 +lon_0=5 +h_0=0 +ellps=WGS84"
    );
}

#[test]
fn operation_geocentric_topocentric() {
    let wkt = r#"PROJCRS["EPSG topocentric example B",
    BASEGEODCRS["WGS 84",
        ENSEMBLE["World Geodetic System 1984 ensemble",
            MEMBER["World Geodetic System 1984 (Transit)"],
            MEMBER["World Geodetic System 1984 (G730)"],
            MEMBER["World Geodetic System 1984 (G873)"],
            MEMBER["World Geodetic System 1984 (G1150)"],
            MEMBER["World Geodetic System 1984 (G1674)"],
            MEMBER["World Geodetic System 1984 (G1762)"],
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]],
            ENSEMBLEACCURACY[2.0]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4978]],
    CONVERSION["EPSG topocentric example B",
        METHOD["Geocentric/topocentric conversions",
            ID["EPSG",9836]],
        PARAMETER["Geocentric X of topocentric origin",3771793.97,
            LENGTHUNIT["metre",1],
            ID["EPSG",8837]],
        PARAMETER["Geocentric Y of topocentric origin",140253.34,
            LENGTHUNIT["metre",1],
            ID["EPSG",8838]],
        PARAMETER["Geocentric Z of topocentric origin",5124304.35,
            LENGTHUNIT["metre",1],
            ID["EPSG",8839]]],
    CS[Cartesian,3],
        AXIS["topocentric East (U)",east,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["topocentric North (V)",north,
            ORDER[2],
            LENGTHUNIT["metre",1]],
        AXIS["topocentric height (W)",up,
            ORDER[3],
            LENGTHUNIT["metre",1]],
    USAGE[
        SCOPE["Example only (fictitious)."],
        AREA["Description of the extent of the CRS."],
        BBOX[-90,-180,90,180]],
    ID["EPSG",5820]]"#;
    let db = database();
    // Need a database so that EPSG:4978 is resolved
    let WktObject::Crs(Crs::Projected(dst)) = parse_db(wkt) else {
        panic!("not a projected CRS");
    };
    let source = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4978")
        .expect("EPSG:4978");
    let op = CoordinateOperationFactory::create()
        .create_operation_proj_string(&source, &Crs::Projected(dst))
        .expect("operation");
    assert_eq!(
        op,
        "+proj=topocentric +X_0=3771793.97 +Y_0=140253.34 \
         +Z_0=5124304.35 +ellps=WGS84"
    );
}

#[test]
fn operation_export_of_cartesian_grid_offsets_with_engineering_crs() {
    let wkt = r#"COORDINATEOPERATION["CIG85 to GDA94 / MGA zone 48",
    VERSION["GA-Cxr"],
    SOURCECRS[
        ENGCRS["Christmas Island Grid 1985",
            EDATUM["Christmas Island Datum 1985"],
            CS[Cartesian,2],
                AXIS["(E)",east,
                    ORDER[1],
                    LENGTHUNIT["metre",1]],
                AXIS["(N)",north,
                    ORDER[2],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",6715]]],
    TARGETCRS[
        PROJCRS["GDA94 / MGA zone 48",
            BASEGEOGCRS["GDA94",
                DATUM["Geocentric Datum of Australia 1994",
                    ELLIPSOID["GRS 1980",6378137,298.257222101,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                ID["EPSG",4283]],
            CONVERSION["Map Grid of Australia zone 48",
                METHOD["Transverse Mercator",
                    ID["EPSG",9807]],
                PARAMETER["Latitude of natural origin",0,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8801]],
                PARAMETER["Longitude of natural origin",105,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8802]],
                PARAMETER["Scale factor at natural origin",0.9996,
                    SCALEUNIT["unity",1],
                    ID["EPSG",8805]],
                PARAMETER["False easting",500000,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8806]],
                PARAMETER["False northing",10000000,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8807]]],
            CS[Cartesian,2],
                AXIS["(E)",east,
                    ORDER[1],
                    LENGTHUNIT["metre",1]],
                AXIS["(N)",north,
                    ORDER[2],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",28348]]],
    METHOD["Cartesian Grid Offsets",
        ID["EPSG",9656]],
    PARAMETER["Easting offset",550015,
        LENGTHUNIT["metre",1],
        ID["EPSG",8728]],
    PARAMETER["Northing offset",8780001,
        LENGTHUNIT["metre",1],
        ID["EPSG",8729]],
    OPERATIONACCURACY[5],
    ID["EPSG",6724]]"#;
    let WktObject::Transformation(transf) = parse(wkt) else {
        panic!("not a transformation");
    };
    assert_eq!(
        transf
            .export_to_proj_string(ProjLines::Single)
            .expect("PROJ"),
        "+proj=affine +xoff=550015 +yoff=8780001"
    );
    assert_eq!(
        transf
            .inverse()
            .export_to_proj_string(ProjLines::Single)
            .expect("PROJ"),
        "+proj=affine +xoff=-550015 +yoff=-8780001"
    );
}

#[test]
fn operation_export_of_seismic_bin_grid_i_eq_j_plus_90() {
    let wkt = r#"COORDINATEOPERATION["EPSG example of georeferencing I=J+90 local bin grid",
    VERSION["P6/11 I = J+90"],
    SOURCECRS[
        PROJCRS["WGS 84 / UTM zone 31N",
            BASEGEOGCRS["WGS 84",
                DATUM["World Geodetic System 1984",
                    ELLIPSOID["WGS 84",6378137,298.257223563,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                ID["EPSG",4326]],
            CONVERSION["UTM zone 31N",
                METHOD["Transverse Mercator",
                    ID["EPSG",9807]],
                PARAMETER["Latitude of natural origin",0,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8801]],
                PARAMETER["Longitude of natural origin",3,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8802]],
                PARAMETER["Scale factor at natural origin",0.9996,
                    SCALEUNIT["unity",1],
                    ID["EPSG",8805]],
                PARAMETER["False easting",500000,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8806]],
                PARAMETER["False northing",0,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8807]]],
            CS[Cartesian,2],
                AXIS["(E)",east,
                    ORDER[1],
                    LENGTHUNIT["metre",1]],
                AXIS["(N)",north,
                    ORDER[2],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",32631]]],
    TARGETCRS[
        ENGCRS["EPSG example I=J+90 local bin grid (integer values)",
            EDATUM["Bin grid origin"],
            CS[ordinal,2],
                AXIS["bin grid I (I)",columnPositive,
                    ORDER[1]],
                AXIS["bin grid J (J)",rowPositive,
                    ORDER[2]],
            ID["EPSG",32764]]],
    METHOD["P6 I=J+90 seismic bin grid coordinate operation",
        ID["EPSG",9666]],
    PARAMETER["Bin grid origin I",1,
        SCALEUNIT["bin",1],
        ID["EPSG",8733]],
    PARAMETER["Bin grid origin J",1,
        SCALEUNIT["bin",1],
        ID["EPSG",8734]],
    PARAMETER["Bin grid origin Easting",456781,
        LENGTHUNIT["metre",1],
        ID["EPSG",8735]],
    PARAMETER["Bin grid origin Northing",5836723,
        LENGTHUNIT["metre",1],
        ID["EPSG",8736]],
    PARAMETER["Scale factor of bin grid",0.99984,
        SCALEUNIT["unity",1],
        ID["EPSG",8737]],
    PARAMETER["Bin width on I-axis",25,
        LENGTHUNIT["metre",1],
        ID["EPSG",8738]],
    PARAMETER["Bin width on J-axis",12.5,
        LENGTHUNIT["metre",1],
        ID["EPSG",8739]],
    PARAMETER["Map grid bearing of bin grid J-axis",20,
        ANGLEUNIT["degree",0.0174532925199433],
        ID["EPSG",8740]],
    PARAMETER["Bin node increment on I-axis",1,
        SCALEUNIT["bin",1],
        ID["EPSG",8741]],
    PARAMETER["Bin node increment on J-axis",1,
        SCALEUNIT["bin",1],
        ID["EPSG",8742]],
    ID["EPSG",6918]]"#;
    let WktObject::Transformation(transf) = parse(wkt) else {
        panic!("not a transformation");
    };
    // Map grid (456781, 5836723) maps to bin (1, 1), as in EPSG guidance note 7-2.
    assert_eq!(
        transf
            .export_to_proj_string(ProjLines::Single)
            .expect("PROJ"),
        "+proj=affine +xoff=62692.7547606425 +s11=0.0375937198266086 \
         +s12=-0.0136829950122287 +yoff=-451347.522624406 \
         +s21=0.0273659900244574 +s22=0.0751874396532172"
    );
}

#[test]
fn operation_export_of_seismic_bin_grid_i_eq_j_minus_90() {
    let wkt = r#"COORDINATEOPERATION["EPSG example of georeferencing I=J-90 local bin grid",
    VERSION["P6/11 I = J-90"],
    SOURCECRS[
        PROJCRS["NAD27 / BLM 16N (ftUS)",
            BASEGEOGCRS["NAD27",
                DATUM["North American Datum 1927",
                    ELLIPSOID["Clarke 1866",6378206.4,294.978698213,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                ID["EPSG",4267]],
            CONVERSION["BLM zone 16N (US survey foot)",
                METHOD["Transverse Mercator",
                    ID["EPSG",9807]],
                PARAMETER["Latitude of natural origin",0,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8801]],
                PARAMETER["Longitude of natural origin",-87,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8802]],
                PARAMETER["Scale factor at natural origin",0.9996,
                    SCALEUNIT["unity",1],
                    ID["EPSG",8805]],
                PARAMETER["False easting",1640416.67,
                    LENGTHUNIT["US survey foot",0.304800609601219],
                    ID["EPSG",8806]],
                PARAMETER["False northing",0,
                    LENGTHUNIT["US survey foot",0.304800609601219],
                    ID["EPSG",8807]]],
            CS[Cartesian,2],
                AXIS["(E)",east,
                    ORDER[1],
                    LENGTHUNIT["US survey foot",0.304800609601219]],
                AXIS["(N)",north,
                    ORDER[2],
                    LENGTHUNIT["US survey foot",0.304800609601219]],
            ID["EPSG",32066]]],
    TARGETCRS[
        ENGCRS["EPSG example I=J-90 local bin grid (integer values)",
            EDATUM["Bin grid origin"],
            CS[ordinal,2],
                AXIS["bin grid I (I)",columnNegative,
                    ORDER[1]],
                AXIS["bin grid J (J)",rowPositive,
                    ORDER[2]],
            ID["EPSG",32765]]],
    METHOD["P6 I=J-90 seismic bin grid coordinate operation",
        ID["EPSG",1049]],
    PARAMETER["Bin grid origin I",5000,
        SCALEUNIT["bin",1],
        ID["EPSG",8733]],
    PARAMETER["Bin grid origin J",0,
        SCALEUNIT["bin",1],
        ID["EPSG",8734]],
    PARAMETER["Bin grid origin Easting",871200,
        LENGTHUNIT["US survey foot",0.304800609601219],
        ID["EPSG",8735]],
    PARAMETER["Bin grid origin Northing",10280160,
        LENGTHUNIT["US survey foot",0.304800609601219],
        ID["EPSG",8736]],
    PARAMETER["Scale factor of bin grid",1,
        SCALEUNIT["unity",1],
        ID["EPSG",8737]],
    PARAMETER["Bin width on I-axis",82.5,
        LENGTHUNIT["US survey foot",0.304800609601219],
        ID["EPSG",8738]],
    PARAMETER["Bin width on J-axis",41.25,
        LENGTHUNIT["US survey foot",0.304800609601219],
        ID["EPSG",8739]],
    PARAMETER["Map grid bearing of bin grid J-axis",340,
        ANGLEUNIT["degree",0.0174532925199433],
        ID["EPSG",8740]],
    PARAMETER["Bin node increment on I-axis",1,
        SCALEUNIT["bin",1],
        ID["EPSG",8741]],
    PARAMETER["Bin node increment on J-axis",1,
        SCALEUNIT["bin",1],
        ID["EPSG",8742]],
    ID["EPSG",6919]]"#;
    let WktObject::Transformation(transf) = parse(wkt) else {
        panic!("not a transformation");
    };
    // The source CRS is in US survey foot, which exercises the unit conversion.
    assert_eq!(
        transf
            .export_to_proj_string(ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline \
         +step +proj=unitconvert +xy_in=us-ft +xy_out=m \
         +step +proj=affine +xoff=57541.6000950241 \
         +s11=-0.0373693924043851 +s12=-0.0136013465078097 \
         +yoff=-226962.970754743 +s21=-0.0272026930156193 \
         +s22=0.0747387848087701"
    );
}
