//! PROJ `test/unit` checks of the `niche` class (rare CRS kinds and operations) from
//! `test_operation.cpp`, `test_crs.cpp`, `test_factory.cpp` and `test_c_api.cpp`.
//!
//! Each function is one `TEST()` name. Expected text and numbers are copied from PROJ
//! 9.9.0. The cases that stay open are listed in `docs/PORTING-niche-other.md`.

use proj_rs::{
    create_from_user_input, AlternativeGridNames, AuthorityFactory, Axis, CartesianCS, Conversion,
    CoordinateOperation, CoordinateOperationFactory, CoordinateSystem, Crs, DatabaseContext,
    DerivedCRS, DerivedProjectedCRS, EllipsoidalCS, EngineeringCRS, EngineeringDatum, Equivalence,
    GeodeticCRS, GeographicCRS, OpProps, ParameterValue, ParametricCRS, ParametricCS, ProjLines,
    ProjectedCRS, TemporalCRS, TemporalCS, TemporalDatum, TemporalKind, UnitKind, UnitOfMeasure,
    VerticalCRS, VerticalCS, VerticalReferenceFrame, WktConvention, WktFormatter, WktObject,
    WktParser, CALENDAR_PROLEPTIC_GREGORIAN,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

/// `UnitOfMeasure::operator==`: name, kind and factor.
fn same_unit(left: &UnitOfMeasure, right: &UnitOfMeasure) -> bool {
    left.name() == right.name()
        && left.kind() == right.kind()
        && left.conversion_to_si() == right.conversion_to_si()
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

#[test]
fn factory_authority_factory_create_coordinate_operation_point_motion_operation() {
    let db = database();
    let factory = AuthorityFactory::create(&db, "EPSG");
    let op = factory
        .create_coordinate_operation("9483", AlternativeGridNames::Official)
        .expect("EPSG:9483");
    let CoordinateOperation::Transformation(pmo) = &op else {
        panic!("not a transformation");
    };
    assert!(pmo.is_point_motion());
    let expected = r#"POINTMOTIONOPERATION["Canada velocity grid v7",
    VERSION["NRC-Can cvg7.0"],
    SOURCECRS[
        GEOGCRS["NAD83(CSRS)v7",
            DATUM["North American Datum of 1983 (CSRS) version 7",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]],
                ANCHOREPOCH[2010]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            CS[ellipsoidal,3],
                AXIS["geodetic latitude (Lat)",north,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["geodetic longitude (Lon)",east,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["ellipsoidal height (h)",up,
                    ORDER[3],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",8254]]],
    METHOD["Point motion (geog3D domain) using NEU velocity grid (NTv2_Vel)",
        ID["EPSG",1141]],
    PARAMETERFILE["Point motion velocity grid file","NAD83v70VG.gvb"],
    OPERATIONACCURACY[0.01],
    USAGE[
        SCOPE["Change of coordinate epoch for points referenced to NAD83(CSRS)v7."],
        AREA["Canada - onshore - Alberta; British Columbia (BC); Manitoba; New Brunswick (NB); Newfoundland and Labrador; Northwest Territories (NWT); Nova Scotia (NS); Nunavut; Ontario; Prince Edward Island (PEI); Quebec; Saskatchewan; Yukon."],
        BBOX[41.67,-141.01,83.17,-52.54]],
    ID["EPSG",9483],
    REMARK["File initially published with name cvg70.cvb, later renamed to NAD83v70VG.gvb with no change of content. Replaces Canada velocity grid v6 (code 8676). Replaced by Canada velocity grid v8 (code 10707). Although the interpolation CRS is given as NAD83(CSRS)v7 (also known as NAD83(CSRS) 2010), any version of NAD83(CSRS) may be used for grid interpolation without significant error."]]"#;
    assert_eq!(
        op.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
            .expect("wkt"),
        expected
    );
}

#[test]
fn factory_authority_factory_iau_2015() {
    let db = database();
    let factory = AuthorityFactory::create(&db, "IAU_2015");
    let degree = UnitOfMeasure::degree();
    let close = |left: f64, right: f64| (left - right).abs() < 1e-6;

    {
        let crs = factory.create_geographic_crs("19900").expect("19900");
        assert_eq!(crs.name(), "Mercury (2015) - Sphere / Ocentric");
        let ellipsoid = crs.ellipsoid().expect("ellipsoid");
        assert!(ellipsoid.is_sphere());
        assert!(close(ellipsoid.semi_major_axis().value(), 2_440_530.0));
        let axes = crs.coordinate_system().axes();
        assert_eq!(axes.len(), 2);
        assert_eq!(axes[0].name(), "Geodetic latitude");
        assert_eq!(axes[0].abbreviation(), "Lat");
        assert_eq!(axes[0].direction(), "north");
        assert!(same_unit(axes[0].unit(), &degree));
        assert_eq!(axes[1].name(), "Geodetic longitude");
        assert_eq!(axes[1].abbreviation(), "Lon");
        assert_eq!(axes[1].direction(), "east");
        assert!(same_unit(axes[1].unit(), &degree));
    }

    {
        let crs = factory.create_geographic_crs("19901").expect("19901");
        assert_eq!(crs.name(), "Mercury (2015) / Ographic");
        let ellipsoid = crs.ellipsoid().expect("ellipsoid");
        assert!(!ellipsoid.is_sphere());
        assert!(close(ellipsoid.semi_major_axis().value(), 2_440_530.0));
        assert!(close(
            ellipsoid.computed_semi_minor_axis().value(),
            2_438_260.0
        ));
        let axes = crs.coordinate_system().axes();
        assert_eq!(axes.len(), 2);
        assert_eq!(axes[0].name(), "Geodetic latitude");
        assert_eq!(axes[0].abbreviation(), "Lat");
        assert_eq!(axes[0].direction(), "north");
        assert!(same_unit(axes[0].unit(), &degree));
        assert_eq!(axes[1].name(), "Geodetic longitude");
        assert_eq!(axes[1].abbreviation(), "Lon");
        assert_eq!(axes[1].direction(), "west"); // WEST !
        assert!(same_unit(axes[1].unit(), &degree));
    }

    {
        let crs = factory.create_geodetic_crs("19902").expect("19902");
        assert_eq!(crs.name(), "Mercury (2015) / Ocentric");
        assert!(!matches!(crs, GeodeticCRS::Geographic(_)));
        let ellipsoid = crs.ellipsoid().expect("ellipsoid");
        assert!(!ellipsoid.is_sphere());
        assert!(close(ellipsoid.semi_major_axis().value(), 2_440_530.0));
        assert!(close(
            ellipsoid.computed_semi_minor_axis().value(),
            2_438_260.0
        ));
        let cs = crs.coordinate_system();
        assert!(matches!(cs, CoordinateSystem::Spherical(_)));
        let axes = cs.axes();
        assert_eq!(axes.len(), 2);
        assert_eq!(axes[0].name(), "Planetocentric latitude");
        assert_eq!(axes[0].abbreviation(), "U");
        assert_eq!(axes[0].direction(), "north");
        assert!(same_unit(axes[0].unit(), &degree));
        assert_eq!(axes[1].name(), "Planetocentric longitude");
        assert_eq!(axes[1].abbreviation(), "V");
        assert_eq!(axes[1].direction(), "east");
        assert!(same_unit(axes[1].unit(), &degree));
    }
}

fn exported(crs: &Crs, convention: WktConvention) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention))
        .expect("wkt")
}

fn refused(crs: &Crs, convention: WktConvention) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention))
        .expect_err("refused")
        .to_string()
}

fn pole_rotation() -> Conversion {
    Conversion::create_with_method(
        "Pole rotation",
        "",
        "",
        vec![
            Conversion::parameter(
                "Latitude of rotated pole",
                "",
                "",
                ParameterValue::angle(52.0),
            ),
            Conversion::parameter(
                "Longitude of rotated pole",
                "",
                "",
                ParameterValue::angle(-30.0),
            ),
            Conversion::parameter("Axis rotation", "", "", ParameterValue::angle(-25.0)),
        ],
    )
    .with_name("Atlantic pole")
}

fn unimplemented_conversion(name: &str) -> Conversion {
    Conversion::create_with_method("PROJ unimplemented", "", "", Vec::new()).with_name(name)
}

fn derived_geographic() -> DerivedCRS {
    DerivedCRS::create(
        "WMO Atlantic Pole",
        Crs::Geographic(GeographicCRS::epsg_4326()),
        pole_rotation(),
        CoordinateSystem::Ellipsoidal(EllipsoidalCS::create_latitude_longitude(
            UnitOfMeasure::degree(),
        )),
    )
    .expect("derived geographic")
}

fn derived_geodetic() -> DerivedCRS {
    DerivedCRS::create(
        "Derived geodetic CRS",
        Crs::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_with_method("Some method", "", "", Vec::new())
            .with_name("Some conversion"),
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("derived geodetic")
}

fn utm_31n() -> ProjectedCRS {
    let conversion = Conversion::create_utm_north(&OpProps::new(), 31).expect("utm");
    ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631")
}

fn derived_projected() -> DerivedProjectedCRS {
    DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
}

fn derived_projected_northing_easting() -> DerivedProjectedCRS {
    DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_northing_easting(UnitOfMeasure::foot())),
    )
}

fn none_unit() -> UnitOfMeasure {
    UnitOfMeasure::new("", 0.0, UnitKind::None)
}

fn date_time_temporal() -> TemporalCRS {
    TemporalCRS::create(
        "Temporal CRS",
        TemporalDatum::create(
            "Gregorian calendar",
            "0000-01-01",
            CALENDAR_PROLEPTIC_GREGORIAN,
        ),
        TemporalCS::create(
            TemporalKind::DateTime,
            Axis::create("Time", "T", "future", none_unit()),
        ),
    )
}

fn vertical_odn() -> VerticalCRS {
    VerticalCRS::create(
        "ODN height",
        VerticalReferenceFrame::named("Ordnance Datum Newlyn").with_identifier("EPSG", "5101"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "5701")
}

fn derived_vertical() -> DerivedCRS {
    DerivedCRS::create(
        "Derived vertCRS",
        Crs::Vertical(vertical_odn()),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Vertical(VerticalCS::create_gravity_related_height(
            UnitOfMeasure::metre(),
        )),
    )
    .expect("derived vertical")
}

fn engineering() -> EngineeringCRS {
    EngineeringCRS::create(
        "Engineering CRS",
        EngineeringDatum::create("Engineering datum"),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
}

fn parametric() -> ParametricCRS {
    ParametricCRS::create(
        "Parametric CRS",
        proj_rs::ParametricDatum::create("Parametric datum"),
        ParametricCS::create(Axis::create(
            "pressure",
            "hPa",
            "up",
            proj_rs::parametric_unit("HectoPascal", 100.0),
        )),
    )
}

#[test]
fn crs_derived_geographic_crs_basic() {
    let derived = derived_geographic();
    let crs = Crs::Derived(derived.clone());
    let base = derived.base_crs().clone();
    assert!(crs.is_equivalent_to(&crs));
    assert!(!crs.is_equivalent_to_with(&base, Equivalence::Equivalent));
    assert!(!base.is_equivalent_to_with(&crs, Equivalence::Equivalent));
}

#[test]
fn crs_derived_geographic_crs_wkt2() {
    let crs = Crs::Derived(derived_geographic());
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2015),
        "\
GEODCRS[\"WMO Atlantic Pole\",
    BASEGEODCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]]],
    DERIVINGCONVERSION[\"Atlantic pole\",
        METHOD[\"Pole rotation\"],
        PARAMETER[\"Latitude of rotated pole\",52,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"Longitude of rotated pole\",-30,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"Axis rotation\",-25,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north,
            ORDER[1],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        AXIS[\"longitude\",east,
            ORDER[2],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]]"
    );
    assert!(crs.is_equivalent_to(&crs.clone()));
}

#[test]
fn crs_derived_geographic_crs_wkt2_2019() {
    assert_eq!(
        exported(
            &Crs::Derived(derived_geographic()),
            WktConvention::Wkt2_2019
        ),
        "\
GEOGCRS[\"WMO Atlantic Pole\",
    BASEGEOGCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        ID[\"EPSG\",4326]],
    DERIVINGCONVERSION[\"Atlantic pole\",
        METHOD[\"Pole rotation\"],
        PARAMETER[\"Latitude of rotated pole\",52,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"Longitude of rotated pole\",-30,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"Axis rotation\",-25,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north,
            ORDER[1],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        AXIS[\"longitude\",east,
            ORDER[2],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]]"
    );
}

#[test]
fn crs_derived_geographic_crs_wkt1() {
    assert_eq!(
        refused(&Crs::Derived(derived_geographic()), WktConvention::Wkt1Gdal),
        "DerivedGeographicCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_derived_geodetic_crs_basic() {
    let derived = derived_geodetic();
    let crs = Crs::Derived(derived.clone());
    let base = derived.base_crs().clone();
    assert!(crs.is_equivalent_to(&crs));
    assert!(!crs.is_equivalent_to_with(&base, Equivalence::Equivalent));
    assert!(!base.is_equivalent_to_with(&crs, Equivalence::Equivalent));
}

#[test]
fn crs_derived_geodetic_crs_wkt2() {
    let crs = Crs::Derived(derived_geodetic());
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2015),
        "\
GEODCRS[\"Derived geodetic CRS\",
    BASEGEODCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]]],
    DERIVINGCONVERSION[\"Some conversion\",
        METHOD[\"Some method\"]],
    CS[Cartesian,3],
        AXIS[\"(X)\",geocentricX,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(Y)\",geocentricY,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(Z)\",geocentricZ,
            ORDER[3],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]"
    );
    assert!(crs.is_equivalent_to(&crs.clone()));
}

#[test]
fn crs_derived_geodetic_crs_wkt2_2019() {
    assert_eq!(
        exported(&Crs::Derived(derived_geodetic()), WktConvention::Wkt2_2019),
        "\
GEODCRS[\"Derived geodetic CRS\",
    BASEGEOGCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        ID[\"EPSG\",4326]],
    DERIVINGCONVERSION[\"Some conversion\",
        METHOD[\"Some method\"]],
    CS[Cartesian,3],
        AXIS[\"(X)\",geocentricX,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(Y)\",geocentricY,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(Z)\",geocentricZ,
            ORDER[3],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]"
    );
}

#[test]
fn crs_derived_geodetic_crs_wkt1() {
    assert_eq!(
        refused(&Crs::Derived(derived_geodetic()), WktConvention::Wkt1Gdal),
        "DerivedGeodeticCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_derived_projected_crs_wkt2_2019() {
    let crs = derived_projected();
    assert_eq!(
        exported(
            &Crs::DerivedProjected(crs.clone()),
            WktConvention::Wkt2_2019
        ),
        "\
DERIVEDPROJCRS[\"derived projectedCRS\",
    BASEPROJCRS[\"WGS 84 / UTM zone 31N\",
        BASEGEOGCRS[\"WGS 84\",
            DATUM[\"World Geodetic System 1984\",
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                    LENGTHUNIT[\"metre\",1]]],
            PRIMEM[\"Greenwich\",0,
                ANGLEUNIT[\"degree\",0.0174532925199433]]],
        CONVERSION[\"UTM zone 31N\",
            METHOD[\"Transverse Mercator\",
                ID[\"EPSG\",9807]],
            PARAMETER[\"Latitude of natural origin\",0,
                ANGLEUNIT[\"degree\",0.0174532925199433],
                ID[\"EPSG\",8801]],
            PARAMETER[\"Longitude of natural origin\",3,
                ANGLEUNIT[\"degree\",0.0174532925199433],
                ID[\"EPSG\",8802]],
            PARAMETER[\"Scale factor at natural origin\",0.9996,
                SCALEUNIT[\"unity\",1],
                ID[\"EPSG\",8805]],
            PARAMETER[\"False easting\",500000,
                LENGTHUNIT[\"metre\",1],
                ID[\"EPSG\",8806]],
            PARAMETER[\"False northing\",0,
                LENGTHUNIT[\"metre\",1],
                ID[\"EPSG\",8807]]],
        ID[\"EPSG\",32631]],
    DERIVINGCONVERSION[\"unnamed\",
        METHOD[\"PROJ unimplemented\"]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]"
    );
    let wrapped = Crs::DerivedProjected(crs);
    assert!(wrapped.is_equivalent_to(&wrapped.clone()));
    assert!(matches!(
        wrapped,
        Crs::DerivedProjected(ref derived) if matches!(derived.base_crs().base_crs(), GeodeticCRS::Geographic(_))
    ));
}

#[test]
fn crs_derived_projected_crs_wkt2_2015() {
    assert_eq!(
        refused(
            &Crs::DerivedProjected(derived_projected()),
            WktConvention::Wkt2_2015
        ),
        "DerivedProjectedCRS can only be exported to WKT2:2019"
    );
}

#[test]
fn crs_date_time_temporal_crs_wkt2() {
    let crs = Crs::Temporal(date_time_temporal());
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2015),
        "\
TIMECRS[\"Temporal CRS\",
    TDATUM[\"Gregorian calendar\",
        TIMEORIGIN[0000-01-01]],
    CS[temporal,1],
        AXIS[\"time (T)\",future]]"
    );
    assert_eq!(
        refused(&crs, WktConvention::Wkt1Gdal),
        "TemporalCRS can only be exported to WKT2"
    );
    assert!(crs.is_equivalent_to(&crs.clone()));
}

#[test]
fn crs_date_time_temporal_crs_wkt2_2019() {
    assert_eq!(
        exported(
            &Crs::Temporal(date_time_temporal()),
            WktConvention::Wkt2_2019
        ),
        "\
TIMECRS[\"Temporal CRS\",
    TDATUM[\"Gregorian calendar\",
        CALENDAR[\"proleptic Gregorian\"],
        TIMEORIGIN[0000-01-01]],
    CS[TemporalDateTime,1],
        AXIS[\"time (T)\",future]]"
    );
}

#[test]
fn crs_temporal_count_crs_with_conv_factor_wkt2_2019() {
    let crs = TemporalCRS::create(
        "GPS milliseconds",
        TemporalDatum::create(
            "GPS time origin",
            "1980-01-01T00:00:00.0Z",
            CALENDAR_PROLEPTIC_GREGORIAN,
        ),
        TemporalCS::create(
            TemporalKind::Count,
            Axis::create(
                "",
                "T",
                "future",
                UnitOfMeasure::new("milliseconds (ms)", 0.001, UnitKind::Time),
            ),
        ),
    );
    assert_eq!(
        exported(&Crs::Temporal(crs), WktConvention::Wkt2_2019),
        "\
TIMECRS[\"GPS milliseconds\",
    TDATUM[\"GPS time origin\",
        CALENDAR[\"proleptic Gregorian\"],
        TIMEORIGIN[1980-01-01T00:00:00.0Z]],
    CS[TemporalCount,1],
        AXIS[\"(T)\",future,
            TIMEUNIT[\"milliseconds (ms)\",0.001]]]"
    );
}

#[test]
fn crs_temporal_count_crs_without_conv_factor_wkt2_2019() {
    let crs = TemporalCRS::create(
        "Calendar hours from 1979-12-29",
        TemporalDatum::create(
            "29 December 1979",
            "1979-12-29T00",
            CALENDAR_PROLEPTIC_GREGORIAN,
        ),
        TemporalCS::create(
            TemporalKind::Count,
            Axis::create(
                "Time",
                "",
                "future",
                UnitOfMeasure::new("hour", 0.0, UnitKind::Time),
            ),
        ),
    );
    assert_eq!(
        exported(&Crs::Temporal(crs), WktConvention::Wkt2_2019),
        "\
TIMECRS[\"Calendar hours from 1979-12-29\",
    TDATUM[\"29 December 1979\",
        CALENDAR[\"proleptic Gregorian\"],
        TIMEORIGIN[1979-12-29T00]],
    CS[TemporalCount,1],
        AXIS[\"time\",future,
            TIMEUNIT[\"hour\"]]]"
    );
}

#[test]
fn crs_temporal_measure_crs_without_conv_factor_wkt2_2019() {
    let crs = TemporalCRS::create(
        "Decimal Years CE",
        TemporalDatum::create("Common Era", "0000", CALENDAR_PROLEPTIC_GREGORIAN),
        TemporalCS::create(
            TemporalKind::Measure,
            Axis::create(
                "Decimal years",
                "a",
                "future",
                UnitOfMeasure::new("year", 0.0, UnitKind::Time),
            ),
        ),
    );
    assert_eq!(
        exported(&Crs::Temporal(crs), WktConvention::Wkt2_2019),
        "\
TIMECRS[\"Decimal Years CE\",
    TDATUM[\"Common Era\",
        CALENDAR[\"proleptic Gregorian\"],
        TIMEORIGIN[0000]],
    CS[TemporalMeasure,1],
        AXIS[\"decimal years (a)\",future,
            TIMEUNIT[\"year\"]]]"
    );
}

#[test]
fn crs_engineering_crs_unknown_equivalence() {
    let wkt1 = "\
ENGCRS[\"Undefined Cartesian SRS with unknown unit\",
    EDATUM[\"\"],
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]";
    let wkt2 = "\
ENGCRS[\"Undefined Cartesian SRS with unknown unit\",
    EDATUM[\"Unknown engineering datum\"],
    CS[Cartesian,2],
        AXIS[\"x\",unspecified,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"y\",unspecified,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]";
    let Crs::Engineering(_) = parse(wkt1).crs().expect("wkt1").clone() else {
        panic!("wkt1 engineering");
    };
    let left = parse(wkt1).crs().expect("wkt1").clone();
    let right = parse(wkt2).crs().expect("wkt2").clone();
    assert!(left.is_equivalent_to_with(&right, Equivalence::Equivalent));
    assert!(right.is_equivalent_to_with(&left, Equivalence::Equivalent));
}

#[test]
fn crs_parametric_crs_wkt2() {
    let crs = Crs::Parametric(parametric());
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2015),
        "\
PARAMETRICCRS[\"Parametric CRS\",
    PDATUM[\"Parametric datum\"],
    CS[parametric,1],
        AXIS[\"pressure (hPa)\",up,
            PARAMETRICUNIT[\"HectoPascal\",100]]]"
    );
    assert!(crs.is_equivalent_to(&crs.clone()));
}

#[test]
fn crs_parametric_crs_wkt1() {
    assert_eq!(
        refused(&Crs::Parametric(parametric()), WktConvention::Wkt1Gdal),
        "ParametricCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_derived_vertical_crs_basic() {
    let derived = derived_vertical();
    let crs = Crs::Derived(derived.clone());
    let base = derived.base_crs().clone();
    assert!(crs.is_equivalent_to(&crs.clone()));
    assert!(!crs.is_equivalent_to(&base));
    assert!(!base.is_equivalent_to(&crs));
}

#[test]
fn crs_derived_vertical_crs_wkt2() {
    assert_eq!(
        exported(&Crs::Derived(derived_vertical()), WktConvention::Wkt2_2015),
        "\
VERTCRS[\"Derived vertCRS\",
    BASEVERTCRS[\"ODN height\",
        VDATUM[\"Ordnance Datum Newlyn\",
            ID[\"EPSG\",5101]]],
    DERIVINGCONVERSION[\"unnamed\",
        METHOD[\"PROJ unimplemented\"]],
    CS[vertical,1],
        AXIS[\"gravity-related height (H)\",up,
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]"
    );
}

#[test]
fn crs_derived_vertical_crs_wkt1() {
    assert_eq!(
        refused(&Crs::Derived(derived_vertical()), WktConvention::Wkt1Gdal),
        "DerivedVerticalCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_derived_vertical_crs_wkt1_when_simple_derivation() {
    let conversion = Conversion::create_change_vertical_unit_no_conv_factor(
        &OpProps::new().name("Vertical Axis Unit Conversion"),
    )
    .expect("1104");
    let crs = DerivedCRS::create(
        "Derived vertCRS",
        Crs::Vertical(vertical_odn()),
        conversion,
        CoordinateSystem::Vertical(VerticalCS::create_gravity_related_height(
            UnitOfMeasure::foot(),
        )),
    )
    .expect("simple vertical");
    assert_eq!(
        exported(&Crs::Derived(crs), WktConvention::Wkt1Gdal),
        "\
VERT_CS[\"Derived vertCRS\",
    VERT_DATUM[\"Ordnance Datum Newlyn\",2005,
        AUTHORITY[\"EPSG\",\"5101\"]],
    UNIT[\"foot\",0.3048,
        AUTHORITY[\"EPSG\",\"9002\"]],
    AXIS[\"Gravity-related height\",UP]]"
    );
}

#[test]
fn crs_derived_engineering_crs_wkt2() {
    let derived = DerivedCRS::create(
        "Derived EngineeringCRS",
        Crs::Engineering(engineering()),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
    .expect("derived engineering");
    let crs = Crs::Derived(derived);
    assert!(crs.is_equivalent_to(&crs.clone()));
    let Crs::Derived(derived) = &crs else {
        unreachable!("derived");
    };
    assert!(derived
        .coordinate_system()
        .is_equivalent_to(&CoordinateSystem::Cartesian(
            CartesianCS::create_easting_northing(UnitOfMeasure::metre())
        )));
    let Crs::Engineering(base) = derived.base_crs() else {
        panic!("engineering base");
    };
    assert!(base.datum().is_equivalent_to(engineering().datum()));
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2019),
        "\
ENGCRS[\"Derived EngineeringCRS\",
    BASEENGCRS[\"Engineering CRS\",
        EDATUM[\"Engineering datum\"]],
    DERIVINGCONVERSION[\"unnamed\",
        METHOD[\"PROJ unimplemented\"]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]"
    );
    assert_eq!(
        refused(&crs, WktConvention::Wkt2_2015),
        "DerivedEngineeringCRS can only be exported to WKT2:2019"
    );
}

#[test]
fn crs_derived_engineering_crs_wkt1() {
    let crs = DerivedCRS::create(
        "Derived EngineeringCRS",
        Crs::Engineering(engineering()),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
    .expect("derived engineering");
    assert_eq!(
        refused(&Crs::Derived(crs), WktConvention::Wkt1Gdal),
        "DerivedEngineeringCRS can only be exported to WKT2:2019"
    );
}

#[test]
fn crs_derived_parametric_crs_wkt2() {
    let cs = ParametricCS::create(Axis::create(
        "pressure",
        "hPa",
        "up",
        proj_rs::parametric_unit("HectoPascal", 100.0),
    ));
    let derived = DerivedCRS::create(
        "Derived ParametricCRS",
        Crs::Parametric(parametric()),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Parametric(cs.clone()),
    )
    .expect("derived parametric");
    let crs = Crs::Derived(derived);
    assert!(crs.is_equivalent_to(&crs.clone()));
    let Crs::Derived(derived) = &crs else {
        unreachable!("derived");
    };
    assert!(derived
        .coordinate_system()
        .is_equivalent_to(&CoordinateSystem::Parametric(cs)));
    let Crs::Parametric(base) = derived.base_crs() else {
        panic!("parametric base");
    };
    assert_eq!(base.datum().name(), parametric().datum().name());
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2019),
        "\
PARAMETRICCRS[\"Derived ParametricCRS\",
    BASEPARAMCRS[\"Parametric CRS\",
        PDATUM[\"Parametric datum\"]],
    DERIVINGCONVERSION[\"unnamed\",
        METHOD[\"PROJ unimplemented\"]],
    CS[parametric,1],
        AXIS[\"pressure (hPa)\",up,
            PARAMETRICUNIT[\"HectoPascal\",100]]]"
    );
}

#[test]
fn crs_derived_parametric_crs_wkt1() {
    let crs = DerivedCRS::create(
        "Derived ParametricCRS",
        Crs::Parametric(parametric()),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Parametric(parametric().coordinate_system().clone()),
    )
    .expect("derived parametric");
    assert_eq!(
        refused(&Crs::Derived(crs), WktConvention::Wkt1Gdal),
        "DerivedParametricCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_derive_temporal_crs_wkt2() {
    let cs = date_time_temporal().coordinate_system().clone();
    let derived = DerivedCRS::create(
        "Derived TemporalCRS",
        Crs::Temporal(date_time_temporal()),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Temporal(cs.clone()),
    )
    .expect("derived temporal");
    let crs = Crs::Derived(derived);
    assert!(crs.is_equivalent_to(&crs.clone()));
    let Crs::Derived(derived) = &crs else {
        unreachable!("derived");
    };
    assert!(derived
        .coordinate_system()
        .is_equivalent_to(&CoordinateSystem::Temporal(cs)));
    let Crs::Temporal(base) = derived.base_crs() else {
        panic!("temporal base");
    };
    let datum = date_time_temporal().datum().clone();
    assert_eq!(base.datum().name(), datum.name());
    assert_eq!(base.datum().temporal_origin(), datum.temporal_origin());
    assert_eq!(base.datum().calendar(), datum.calendar());
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2019),
        "\
TIMECRS[\"Derived TemporalCRS\",
    BASETIMECRS[\"Temporal CRS\",
        TDATUM[\"Gregorian calendar\",
            CALENDAR[\"proleptic Gregorian\"],
            TIMEORIGIN[0000-01-01]]],
    DERIVINGCONVERSION[\"unnamed\",
        METHOD[\"PROJ unimplemented\"]],
    CS[TemporalDateTime,1],
        AXIS[\"time (T)\",future]]"
    );
}

#[test]
fn crs_derive_temporal_crs_wkt1() {
    let crs = DerivedCRS::create(
        "Derived TemporalCRS",
        Crs::Temporal(date_time_temporal()),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Temporal(date_time_temporal().coordinate_system().clone()),
    )
    .expect("derived temporal");
    assert_eq!(
        refused(&Crs::Derived(crs), WktConvention::Wkt1Gdal),
        "DerivedTemporalCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_normalize_for_visualization_derivedprojected() {
    let crs = derived_projected_northing_easting();
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].direction(), "north");
    assert_eq!(axes[1].direction(), "east");

    let normalized = crs.normalize_for_visualization();
    let axes = normalized.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].direction(), "east");
    assert_eq!(axes[1].direction(), "north");

    let normalized_3d = crs.promote_to_3d(None).normalize_for_visualization();
    let axes = normalized_3d.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert_eq!(axes[0].direction(), "east");
    assert_eq!(axes[1].direction(), "north");
    assert_eq!(axes[2].direction(), "up");
}

#[test]
fn c_api_proj_create_engineering_crs() {
    let crs = EngineeringCRS::create(
        "name",
        EngineeringDatum::create(""),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    );
    assert_eq!(
        exported(&Crs::Engineering(crs), WktConvention::Wkt1Gdal),
        "\
LOCAL_CS[\"name\",
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Easting\",EAST],
    AXIS[\"Northing\",NORTH]]"
    );
}

#[test]
fn c_api_proj_crs_is_derived() {
    let projected = Crs::Projected(utm_31n());
    assert!(projected.is_derived());
    let base = Crs::Geodetic(utm_31n().base_crs().clone());
    assert!(!base.is_derived());
}

fn conversion_wkt(conversion: &Conversion) -> String {
    conversion
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("conversion wkt")
}

fn affine_2d(name: &str, units: [UnitOfMeasure; 6]) -> Conversion {
    let values = [1.1, 2.1, 3.1, 4.1, 5.1, 6.1]
        .into_iter()
        .zip(units)
        .map(|(value, unit)| ParameterValue::measure(value, unit))
        .collect();
    Conversion::create_by_name(
        &OpProps::new().name(name),
        "Affine parametric transformation",
        values,
    )
    .expect("affine 2d")
}

fn default_affine_2d(name: &str) -> Conversion {
    let metre = UnitOfMeasure::metre();
    let unity = UnitOfMeasure::unity();
    affine_2d(
        name,
        [
            metre.clone(),
            unity.clone(),
            unity.clone(),
            metre,
            unity.clone(),
            unity,
        ],
    )
}

fn unity_9201() -> UnitOfMeasure {
    UnitOfMeasure::unity().with_identifier("EPSG", "9201")
}

fn affine_3d_default() -> Conversion {
    let metre = UnitOfMeasure::metre();
    let unity = unity_9201();
    let units = [
        metre.clone(),
        unity.clone(),
        unity.clone(),
        unity.clone(),
        metre.clone(),
        unity.clone(),
        unity.clone(),
        unity.clone(),
        metre,
        unity.clone(),
        unity.clone(),
        unity,
    ];
    let values = [
        1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 7.1, 8.1, 9.1, 10.1, 11.1, 12.1,
    ]
    .into_iter()
    .zip(units)
    .map(|(value, unit)| ParameterValue::measure(value, unit))
    .collect();
    Conversion::create_by_name(
        &OpProps::new().name("unnamed"),
        "3D Affine parametric transformation",
        values,
    )
    .expect("affine 3d")
}

fn easting_northing_metre() -> CoordinateSystem {
    CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre()))
}

fn database_crs(text: &str) -> Crs {
    create_from_user_input(text, &database()).expect(text)
}

const DERIVED_PROJ_WKT: &str = "\
DERIVEDPROJCRS[\"my derived crs\",
    BASEPROJCRS[\"WGS 84 / UTM zone 31N\",
        BASEGEOGCRS[\"WGS 84\",
            ENSEMBLE[\"World Geodetic System 1984 ensemble\",
                MEMBER[\"World Geodetic System 1984 (Transit)\"],
                MEMBER[\"World Geodetic System 1984 (G730)\"],
                MEMBER[\"World Geodetic System 1984 (G873)\"],
                MEMBER[\"World Geodetic System 1984 (G1150)\"],
                MEMBER[\"World Geodetic System 1984 (G1674)\"],
                MEMBER[\"World Geodetic System 1984 (G1762)\"],
                MEMBER[\"World Geodetic System 1984 (G2139)\"],
                MEMBER[\"World Geodetic System 1984 (G2296)\"],
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                    LENGTHUNIT[\"metre\",1]],
                ENSEMBLEACCURACY[2.0]],
            PRIMEM[\"Greenwich\",0,
                ANGLEUNIT[\"degree\",0.0174532925199433]]],
        CONVERSION[\"UTM zone 31N\",
            METHOD[\"Transverse Mercator\",
                ID[\"EPSG\",9807]],
            PARAMETER[\"Latitude of natural origin\",0,
                ANGLEUNIT[\"degree\",0.0174532925199433],
                ID[\"EPSG\",8801]],
            PARAMETER[\"Longitude of natural origin\",3,
                ANGLEUNIT[\"degree\",0.0174532925199433],
                ID[\"EPSG\",8802]],
            PARAMETER[\"Scale factor at natural origin\",0.9996,
                SCALEUNIT[\"unity\",1],
                ID[\"EPSG\",8805]],
            PARAMETER[\"False easting\",500000,
                LENGTHUNIT[\"metre\",1],
                ID[\"EPSG\",8806]],
            PARAMETER[\"False northing\",0,
                LENGTHUNIT[\"metre\",1],
                ID[\"EPSG\",8807]]],
        ID[\"EPSG\",32631]],
    DERIVINGCONVERSION[\"unnamed\",
        METHOD[\"Affine parametric transformation\",
            ID[\"EPSG\",9624]],
        PARAMETER[\"A0\",1.1,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8623]],
        PARAMETER[\"A1\",2.1,
            SCALEUNIT[\"unity\",1],
            ID[\"EPSG\",8624]],
        PARAMETER[\"A2\",3.1,
            SCALEUNIT[\"unity\",1],
            ID[\"EPSG\",8625]],
        PARAMETER[\"B0\",4.1,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8639]],
        PARAMETER[\"B1\",5.1,
            SCALEUNIT[\"unity\",1],
            ID[\"EPSG\",8640]],
        PARAMETER[\"B2\",6.1,
            SCALEUNIT[\"unity\",1],
            ID[\"EPSG\",8641]]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]";

#[test]
fn c_api_proj_get_celestial_body_list_from_database() {
    let all = AuthorityFactory::create(&database(), "")
        .get_celestial_body_list()
        .expect("bodies");
    assert!(!all.is_empty());
    assert!(!all[0].authority.is_empty());
    assert!(!all[0].name.is_empty());
    let esri = AuthorityFactory::create(&database(), "ESRI")
        .get_celestial_body_list()
        .expect("esri");
    assert!(esri.len() > 1);
    assert!(esri.iter().all(|body| body.authority == "ESRI"));
    assert!(esri.iter().any(|body| body.name == "Ganymede"));
}

#[test]
fn c_api_proj_create_linear_affine_parametric_conversion() {
    assert_eq!(
        conversion_wkt(&default_affine_2d("unnamed")),
        "\
CONVERSION[\"unnamed\",
    METHOD[\"Affine parametric transformation\",
        ID[\"EPSG\",9624]],
    PARAMETER[\"A0\",1.1,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8623]],
    PARAMETER[\"A1\",2.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8624]],
    PARAMETER[\"A2\",3.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8625]],
    PARAMETER[\"B0\",4.1,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8639]],
    PARAMETER[\"B1\",5.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8640]],
    PARAMETER[\"B2\",6.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8641]]]"
    );
    let custom = affine_2d(
        "conversion name",
        [
            UnitOfMeasure::new("my unit1", 0.11, UnitKind::Linear),
            UnitOfMeasure::new("my unit2", 0.12, UnitKind::Scale),
            UnitOfMeasure::new("my unit3", 0.13, UnitKind::Scale),
            UnitOfMeasure::new("my unit4", 0.14, UnitKind::Linear),
            UnitOfMeasure::new("my unit5", 0.15, UnitKind::Scale),
            UnitOfMeasure::new("my unit6", 0.16, UnitKind::Scale),
        ],
    );
    assert_eq!(
        conversion_wkt(&custom),
        "\
CONVERSION[\"conversion name\",
    METHOD[\"Affine parametric transformation\",
        ID[\"EPSG\",9624]],
    PARAMETER[\"A0\",1.1,
        LENGTHUNIT[\"my unit1\",0.11],
        ID[\"EPSG\",8623]],
    PARAMETER[\"A1\",2.1,
        SCALEUNIT[\"my unit2\",0.12],
        ID[\"EPSG\",8624]],
    PARAMETER[\"A2\",3.1,
        SCALEUNIT[\"my unit3\",0.13],
        ID[\"EPSG\",8625]],
    PARAMETER[\"B0\",4.1,
        LENGTHUNIT[\"my unit4\",0.14],
        ID[\"EPSG\",8639]],
    PARAMETER[\"B1\",5.1,
        SCALEUNIT[\"my unit5\",0.15],
        ID[\"EPSG\",8640]],
    PARAMETER[\"B2\",6.1,
        SCALEUNIT[\"my unit6\",0.16],
        ID[\"EPSG\",8641]]]"
    );
}

#[test]
fn c_api_proj_create_linear_3d_affine_parametric_conversion() {
    let conversion = affine_3d_default();
    assert_eq!(
        conversion_wkt(&conversion),
        "\
CONVERSION[\"unnamed\",
    METHOD[\"3D Affine parametric transformation\"],
    PARAMETER[\"A0\",1.1,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8623]],
    PARAMETER[\"A1\",2.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8624]],
    PARAMETER[\"A2\",3.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8625]],
    PARAMETER[\"A3\",4.1,
        SCALEUNIT[\"unity\",1,
            ID[\"EPSG\",9201]]],
    PARAMETER[\"B0\",5.1,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8639]],
    PARAMETER[\"B1\",6.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8640]],
    PARAMETER[\"B2\",7.1,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8641]],
    PARAMETER[\"B3\",8.1,
        SCALEUNIT[\"unity\",1,
            ID[\"EPSG\",9201]]],
    PARAMETER[\"C0\",9.1,
        LENGTHUNIT[\"metre\",1,
            ID[\"EPSG\",9001]]],
    PARAMETER[\"C1\",10.1,
        SCALEUNIT[\"unity\",1,
            ID[\"EPSG\",9201]]],
    PARAMETER[\"C2\",11.1,
        SCALEUNIT[\"unity\",1,
            ID[\"EPSG\",9201]]],
    PARAMETER[\"C3\",12.1,
        SCALEUNIT[\"unity\",1,
            ID[\"EPSG\",9201]]]]"
    );
    assert_eq!(
        conversion.export_to_proj_string().expect("proj"),
        "+proj=affine +xoff=1.1 +s11=2.1 +s12=3.1 +s13=4.1 +yoff=5.1 +s21=6.1 +s22=7.1 +s23=8.1 +zoff=9.1 +s31=10.1 +s32=11.1 +s33=12.1"
    );
}

#[test]
fn c_api_proj_create_derived_projected_crs() {
    let Crs::Projected(base) = database_crs("EPSG:32631") else {
        panic!("EPSG:32631 is projected");
    };
    let derived = DerivedProjectedCRS::create(
        "my derived crs",
        base,
        default_affine_2d("unnamed"),
        easting_northing_metre(),
    );
    assert_eq!(
        exported(&Crs::DerivedProjected(derived), WktConvention::Wkt2_2019),
        DERIVED_PROJ_WKT
    );
}

#[test]
fn c_api_proj_crs_add_horizontal_derived_conversion() {
    let projected = database_crs("EPSG:32631")
        .add_horizontal_derived_conversion(
            "my derived crs",
            default_affine_2d("unnamed"),
            easting_northing_metre(),
        )
        .expect("projected");
    assert_eq!(
        exported(&projected, WktConvention::Wkt2_2019),
        DERIVED_PROJ_WKT
    );

    let compound = database_crs("EPSG:32631+3855")
        .add_horizontal_derived_conversion(
            "my derived crs",
            default_affine_2d("unnamed"),
            easting_northing_metre(),
        )
        .expect("compound");
    assert_eq!(
        exported(&compound, WktConvention::Wkt2_2019),
        "\
COMPOUNDCRS[\"my derived crs + EGM2008 height\",
    DERIVEDPROJCRS[\"my derived crs\",
        BASEPROJCRS[\"WGS 84 / UTM zone 31N\",
            BASEGEOGCRS[\"WGS 84\",
                ENSEMBLE[\"World Geodetic System 1984 ensemble\",
                    MEMBER[\"World Geodetic System 1984 (Transit)\"],
                    MEMBER[\"World Geodetic System 1984 (G730)\"],
                    MEMBER[\"World Geodetic System 1984 (G873)\"],
                    MEMBER[\"World Geodetic System 1984 (G1150)\"],
                    MEMBER[\"World Geodetic System 1984 (G1674)\"],
                    MEMBER[\"World Geodetic System 1984 (G1762)\"],
                    MEMBER[\"World Geodetic System 1984 (G2139)\"],
                    MEMBER[\"World Geodetic System 1984 (G2296)\"],
                    ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                        LENGTHUNIT[\"metre\",1]],
                    ENSEMBLEACCURACY[2.0]],
                PRIMEM[\"Greenwich\",0,
                    ANGLEUNIT[\"degree\",0.0174532925199433]]],
            CONVERSION[\"UTM zone 31N\",
                METHOD[\"Transverse Mercator\",
                    ID[\"EPSG\",9807]],
                PARAMETER[\"Latitude of natural origin\",0,
                    ANGLEUNIT[\"degree\",0.0174532925199433],
                    ID[\"EPSG\",8801]],
                PARAMETER[\"Longitude of natural origin\",3,
                    ANGLEUNIT[\"degree\",0.0174532925199433],
                    ID[\"EPSG\",8802]],
                PARAMETER[\"Scale factor at natural origin\",0.9996,
                    SCALEUNIT[\"unity\",1],
                    ID[\"EPSG\",8805]],
                PARAMETER[\"False easting\",500000,
                    LENGTHUNIT[\"metre\",1],
                    ID[\"EPSG\",8806]],
                PARAMETER[\"False northing\",0,
                    LENGTHUNIT[\"metre\",1],
                    ID[\"EPSG\",8807]]],
            ID[\"EPSG\",32631]],
        DERIVINGCONVERSION[\"unnamed\",
            METHOD[\"Affine parametric transformation\",
                ID[\"EPSG\",9624]],
            PARAMETER[\"A0\",1.1,
                LENGTHUNIT[\"metre\",1],
                ID[\"EPSG\",8623]],
            PARAMETER[\"A1\",2.1,
                SCALEUNIT[\"unity\",1],
                ID[\"EPSG\",8624]],
            PARAMETER[\"A2\",3.1,
                SCALEUNIT[\"unity\",1],
                ID[\"EPSG\",8625]],
            PARAMETER[\"B0\",4.1,
                LENGTHUNIT[\"metre\",1],
                ID[\"EPSG\",8639]],
            PARAMETER[\"B1\",5.1,
                SCALEUNIT[\"unity\",1],
                ID[\"EPSG\",8640]],
            PARAMETER[\"B2\",6.1,
                SCALEUNIT[\"unity\",1],
                ID[\"EPSG\",8641]]],
        CS[Cartesian,2],
            AXIS[\"(E)\",east,
                ORDER[1],
                LENGTHUNIT[\"metre\",1,
                    ID[\"EPSG\",9001]]],
            AXIS[\"(N)\",north,
                ORDER[2],
                LENGTHUNIT[\"metre\",1,
                    ID[\"EPSG\",9001]]]],
    VERTCRS[\"EGM2008 height\",
        VDATUM[\"EGM2008 geoid\"],
        CS[vertical,1],
            AXIS[\"gravity-related height (H)\",up,
                LENGTHUNIT[\"metre\",1]],
        USAGE[
            SCOPE[\"Geodesy.\"],
            AREA[\"World.\"],
            BBOX[-90,-180,90,180]],
        ID[\"EPSG\",3855]]]"
    );

    let bound = database_crs("+proj=utm +zone=31 +ellps=GRS80 +towgs84=0,0,0 +type=crs")
        .add_horizontal_derived_conversion(
            "my derived crs",
            default_affine_2d("unnamed"),
            easting_northing_metre(),
        )
        .expect("bound");
    assert_eq!(
        exported(&bound, WktConvention::Wkt2_2019),
        "\
BOUNDCRS[
    SOURCECRS[
        DERIVEDPROJCRS[\"my derived crs\",
            BASEPROJCRS[\"unknown\",
                BASEGEOGCRS[\"unknown\",
                    DATUM[\"Unknown based on GRS 1980 ellipsoid using towgs84=0,0,0\",
                        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,
                            LENGTHUNIT[\"metre\",1],
                            ID[\"EPSG\",7019]]],
                    PRIMEM[\"Greenwich\",0,
                        ANGLEUNIT[\"degree\",0.0174532925199433],
                        ID[\"EPSG\",8901]]],
                CONVERSION[\"UTM zone 31N\",
                    METHOD[\"Transverse Mercator\",
                        ID[\"EPSG\",9807]],
                    PARAMETER[\"Latitude of natural origin\",0,
                        ANGLEUNIT[\"degree\",0.0174532925199433],
                        ID[\"EPSG\",8801]],
                    PARAMETER[\"Longitude of natural origin\",3,
                        ANGLEUNIT[\"degree\",0.0174532925199433],
                        ID[\"EPSG\",8802]],
                    PARAMETER[\"Scale factor at natural origin\",0.9996,
                        SCALEUNIT[\"unity\",1],
                        ID[\"EPSG\",8805]],
                    PARAMETER[\"False easting\",500000,
                        LENGTHUNIT[\"metre\",1],
                        ID[\"EPSG\",8806]],
                    PARAMETER[\"False northing\",0,
                        LENGTHUNIT[\"metre\",1],
                        ID[\"EPSG\",8807]],
                    ID[\"EPSG\",16031]]],
            DERIVINGCONVERSION[\"unnamed\",
                METHOD[\"Affine parametric transformation\",
                    ID[\"EPSG\",9624]],
                PARAMETER[\"A0\",1.1,
                    LENGTHUNIT[\"metre\",1],
                    ID[\"EPSG\",8623]],
                PARAMETER[\"A1\",2.1,
                    SCALEUNIT[\"unity\",1],
                    ID[\"EPSG\",8624]],
                PARAMETER[\"A2\",3.1,
                    SCALEUNIT[\"unity\",1],
                    ID[\"EPSG\",8625]],
                PARAMETER[\"B0\",4.1,
                    LENGTHUNIT[\"metre\",1],
                    ID[\"EPSG\",8639]],
                PARAMETER[\"B1\",5.1,
                    SCALEUNIT[\"unity\",1],
                    ID[\"EPSG\",8640]],
                PARAMETER[\"B2\",6.1,
                    SCALEUNIT[\"unity\",1],
                    ID[\"EPSG\",8641]]],
            CS[Cartesian,2],
                AXIS[\"(E)\",east,
                    ORDER[1],
                    LENGTHUNIT[\"metre\",1,
                        ID[\"EPSG\",9001]]],
                AXIS[\"(N)\",north,
                    ORDER[2],
                    LENGTHUNIT[\"metre\",1,
                        ID[\"EPSG\",9001]]]]],
    TARGETCRS[
        GEOGCRS[\"WGS 84\",
            DATUM[\"World Geodetic System 1984\",
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                    LENGTHUNIT[\"metre\",1]]],
            PRIMEM[\"Greenwich\",0,
                ANGLEUNIT[\"degree\",0.0174532925199433]],
            CS[ellipsoidal,2],
                AXIS[\"latitude\",north,
                    ORDER[1],
                    ANGLEUNIT[\"degree\",0.0174532925199433]],
                AXIS[\"longitude\",east,
                    ORDER[2],
                    ANGLEUNIT[\"degree\",0.0174532925199433]],
            ID[\"EPSG\",4326]]],
    ABRIDGEDTRANSFORMATION[\"Transformation from unknown to WGS84\",
        METHOD[\"Geocentric translations (geog2D domain)\",
            ID[\"EPSG\",9603]],
        PARAMETER[\"X-axis translation\",0,
            ID[\"EPSG\",8605]],
        PARAMETER[\"Y-axis translation\",0,
            ID[\"EPSG\",8606]],
        PARAMETER[\"Z-axis translation\",0,
            ID[\"EPSG\",8607]]]]"
    );

    let refused = database_crs("EPSG:4326").add_horizontal_derived_conversion(
        "my derived crs",
        default_affine_2d("unnamed"),
        easting_northing_metre(),
    );
    assert!(refused.is_err());
}

fn rotated_geographic(code: &str, method: &str) -> Crs {
    let base = database_crs(code);
    let Crs::Geographic(geographic) = &base else {
        panic!("{code} is not geographic");
    };
    let cs = CoordinateSystem::Ellipsoidal(geographic.coordinate_system().clone());
    let conversion = Conversion::create_by_name(
        &OpProps::new(),
        method,
        vec![
            ParameterValue::angle(2.0),
            ParameterValue::angle(3.0),
            ParameterValue::angle(4.0),
        ],
    )
    .expect(method);
    Crs::Derived(DerivedCRS::create("my rotated CRS", base, conversion, cs).expect("derived"))
}

#[test]
fn c_api_proj_create_derived_geographic_crs() {
    let base = database_crs("EPSG:4326");
    assert!(!base.is_derived());
    let crs = rotated_geographic("EPSG:4326", "Pole rotation (GRIB convention)");
    assert!(crs.is_derived());
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2019),
        "\
GEOGCRS[\"my rotated CRS\",
    BASEGEOGCRS[\"WGS 84\",
        ENSEMBLE[\"World Geodetic System 1984 ensemble\",
            MEMBER[\"World Geodetic System 1984 (Transit)\"],
            MEMBER[\"World Geodetic System 1984 (G730)\"],
            MEMBER[\"World Geodetic System 1984 (G873)\"],
            MEMBER[\"World Geodetic System 1984 (G1150)\"],
            MEMBER[\"World Geodetic System 1984 (G1674)\"],
            MEMBER[\"World Geodetic System 1984 (G1762)\"],
            MEMBER[\"World Geodetic System 1984 (G2139)\"],
            MEMBER[\"World Geodetic System 1984 (G2296)\"],
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]],
            ENSEMBLEACCURACY[2.0]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        ID[\"EPSG\",4326]],
    DERIVINGCONVERSION[\"Pole rotation (GRIB convention)\",
        METHOD[\"Pole rotation (GRIB convention)\"],
        PARAMETER[\"Latitude of the southern pole (GRIB convention)\",2,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"Longitude of the southern pole (GRIB convention)\",3,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"Axis rotation (GRIB convention)\",4,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]],
    CS[ellipsoidal,2],
        AXIS[\"geodetic latitude (Lat)\",north,
            ORDER[1],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        AXIS[\"geodetic longitude (Lon)\",east,
            ORDER[2],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]]"
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=ob_tran +o_proj=longlat +o_lon_p=-4 +o_lat_p=-2 +lon_0=3 +datum=WGS84 +no_defs +type=crs"
    );
}

#[test]
fn c_api_proj_create_derived_geographic_crs_netcdf_cf() {
    let crs = rotated_geographic("EPSG:4019", "Pole rotation (netCDF CF convention)");
    assert_eq!(
        exported(&crs, WktConvention::Wkt2_2019),
        "\
GEOGCRS[\"my rotated CRS\",
    BASEGEOGCRS[\"Unknown datum based upon the GRS 1980 ellipsoid\",
        DATUM[\"Not specified (based on GRS 1980 ellipsoid)\",
            ELLIPSOID[\"GRS 1980\",6378137,298.257222101,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        ID[\"EPSG\",4019]],
    DERIVINGCONVERSION[\"Pole rotation (netCDF CF convention)\",
        METHOD[\"Pole rotation (netCDF CF convention)\"],
        PARAMETER[\"Grid north pole latitude (netCDF CF convention)\",2,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"Grid north pole longitude (netCDF CF convention)\",3,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        PARAMETER[\"North pole grid longitude (netCDF CF convention)\",4,
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]],
    CS[ellipsoidal,2],
        AXIS[\"geodetic latitude (Lat)\",north,
            ORDER[1],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]],
        AXIS[\"geodetic longitude (Lon)\",east,
            ORDER[2],
            ANGLEUNIT[\"degree\",0.0174532925199433,
                ID[\"EPSG\",9122]]]]"
    );
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=ob_tran +o_proj=longlat +o_lon_p=4 +o_lat_p=2 +lon_0=183 +ellps=GRS80 +no_defs +type=crs"
    );
}
