//! PROJ `test/unit/test_crs.cpp` checks that build or export a CRS.
//!
//! Each function is one `TEST()` name. WKT text is the expected string from that test.

use proj_rs::{
    create_crs_from_proj_string, create_from_user_input, AlternativeGridNames, AuthorityFactory,
    Axis, BoundCRS,
    CartesianCS, CoordinateOperation, CoordinateOperationFactory, IntermediateCrsUse, Measure,
    Meridian, Parameter, ParameterValue,
    CompoundCRS, Conversion, CoordinateSystem, Crs, DatabaseContext, Datum, DatumEnsemble,
    DateTimeTemporalCS, DerivedGeodeticCRS, DerivedGeographicCRS, DerivedProjectedCRS,
    TemporalCountCS, TemporalMeasureCS, ParametricCRS, ParametricCS, ParametricDatum,
    DerivedVerticalCRS, DerivedEngineeringCRS, DerivedParametricCRS, DerivedTemporalCRS,
    TemporalCRS, TemporalDatum,
    EllipsoidalCS, EngineeringCRS, EngineeringDatum,
    EllipsoidalHeightPolicy, Equivalence, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS,
    LinearParameterScale, LinunitPolicy, OpProps, OutputAxisRule, PrimeMeridian, ProjectedCRS,
    RealizationMethod,
    Transformation, UnitKind, UnitOfMeasure,
    VerticalCRS, VerticalCS, VerticalReferenceFrame, WktConvention, WktDialect, WktFormatter,
    WktLines, WktParser,
    WktStrictness,
};

fn export(crs: &GeographicCRS, convention: WktConvention) -> String {
    Crs::Geographic(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(convention))
        .expect("wkt")
}

fn export_db(crs: &Crs, convention: WktConvention, db: DatabaseContext) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention).with_database(db))
        .expect("wkt")
}

#[test]
fn crs_epsg_4901_as_wkt1_esri_with_primem_unit_name_morphing() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4901")
        .expect("EPSG:4901");
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("WKT1 ESRI");
    assert_eq!(
        wkt,
        "GEOGCS[\"GCS_ATF_Paris\",DATUM[\"D_ATF\",\
SPHEROID[\"Plessis_1817\",6376523.0,308.64]],\
PRIMEM[\"Paris_RGS\",2.33720833333333],\
UNIT[\"Grad\",0.0157079632679489]]"
    );
}

#[test]
fn cs_cartesian_cs() {
    let east = Axis::create("Easting", "X", "east", UnitOfMeasure::metre());
    let north = Axis::create("Northing", "Y", "north", UnitOfMeasure::metre());
    let up = Axis::create("Height", "H", "up", UnitOfMeasure::metre());
    assert!(CartesianCS::create(vec![east.clone(), north.clone()]).is_ok());
    assert!(CartesianCS::create(vec![east.clone(), north.clone(), up]).is_ok());
    let foot = Axis::create("Height", "H", "up", UnitOfMeasure::foot());
    assert!(CartesianCS::create(vec![east.clone(), foot.clone()]).is_err());
    assert!(CartesianCS::create_allowing_mixed_units(vec![east.clone(), foot.clone()]).is_ok());
    assert!(CartesianCS::create(vec![east.clone(), north.clone(), foot.clone()]).is_err());
    assert!(CartesianCS::create_allowing_mixed_units(vec![east, north, foot]).is_ok());
}

#[test]
fn crs_epsg_4326_get_components() {
    let crs = GeographicCRS::epsg_4326();
    assert_eq!(crs.identifiers().len(), 1);
    assert_eq!(crs.identifiers()[0].code(), "4326");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(crs.name(), "WGS 84");
    let datum = crs.datum().expect("datum");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "6326");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    assert_eq!(datum.name(), "World Geodetic System 1984");
    let ellipsoid = datum.ellipsoid();
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_137.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
    assert_eq!(ellipsoid.inverse_flattening(), 298.257_223_563);
    assert_eq!(ellipsoid.identifiers().len(), 1);
    assert_eq!(ellipsoid.identifiers()[0].code(), "7030");
    assert_eq!(ellipsoid.identifiers()[0].code_space(), "EPSG");
    assert_eq!(ellipsoid.name(), "WGS 84");
}

#[test]
fn crs_geographic_crs_is_equivalent_to() {
    let crs = GeographicCRS::epsg_4326();
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.is_equivalent_to_with(&crs, Equivalence::Equivalent));
    assert!(crs.is_equivalent_to_with(&crs, Equivalence::ExceptAxisOrder));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!crs.is_equivalent_to_geographic(None, Equivalence::Strict));
    assert!(!crs.is_equivalent_to(&GeographicCRS::epsg_4979()));
    assert!(!crs.is_equivalent_to_with(&GeographicCRS::epsg_4979(), Equivalence::Equivalent));
    assert!(!crs.is_equivalent_to_with(&GeographicCRS::ogc_crs84(), Equivalence::Equivalent));
    assert!(crs.is_equivalent_to_with(&GeographicCRS::ogc_crs84(), Equivalence::ExceptAxisOrder));
    assert!(GeographicCRS::ogc_crs84().is_equivalent_to_with(&crs, Equivalence::ExceptAxisOrder));

    let anonymous = GeographicCRS::create(
        "",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("anonymous");
    assert!(!anonymous.is_equivalent_to(&crs));

    let built = GeographicCRS::create(
        "",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("left");
    let different = GeographicCRS::create(
        "",
        Some(GeodeticReferenceFrame::anonymous_wgs84()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("right");
    assert!(!built.is_equivalent_to(&different));
}

#[test]
fn crs_geographic_crs_datum_ensemble() {
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
        ],
        100.0,
    )
    .expect("ensemble");
    let named = GeographicCRS::create(
        "unnamed",
        None,
        Some(ensemble.clone()),
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("named")
    .with_identifier("MY_CODESPACE", "MY_ID");
    assert_eq!(
        export(&named, WktConvention::Wkt2_2019),
        "\
GEOGCRS[\"unnamed\",
    ENSEMBLE[\"unnamed\",
        MEMBER[\"World Geodetic System 1984\"],
        MEMBER[\"World Geodetic System 1984\"],
        ELLIPSOID[\"WGS 84\",6378137,298.257223563,
            LENGTHUNIT[\"metre\",1]],
        ENSEMBLEACCURACY[100]],
    PRIMEM[\"Greenwich\",0,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north,
            ORDER[1],
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        AXIS[\"longitude\",east,
            ORDER[2],
            ANGLEUNIT[\"degree\",0.0174532925199433]],
    ID[\"MY_CODESPACE\",\"MY_ID\"]]"
    );

    let plain = GeographicCRS::create(
        "unnamed",
        None,
        Some(ensemble),
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("plain");
    assert_eq!(
        export(&plain, WktConvention::Wkt2_2019),
        "\
GEOGCRS[\"unnamed\",
    ENSEMBLE[\"unnamed\",
        MEMBER[\"World Geodetic System 1984\",
            ID[\"EPSG\",6326]],
        MEMBER[\"World Geodetic System 1984\",
            ID[\"EPSG\",6326]],
        ELLIPSOID[\"WGS 84\",6378137,298.257223563,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",7030]],
        ENSEMBLEACCURACY[100]],
    PRIMEM[\"Greenwich\",0,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8901]],
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
fn crs_geographic_crs_ensemble_exception_in_create() {
    assert!(GeographicCRS::create(
        "",
        None,
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .is_err());
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Vertical(VerticalReferenceFrame::named("vdatum1")),
            Datum::Vertical(VerticalReferenceFrame::named("vdatum2")),
        ],
        100.0,
    )
    .expect("vertical ensemble");
    assert!(GeographicCRS::create(
        "",
        None,
        Some(ensemble),
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .is_err());
}

#[test]
fn crs_epsg_4326_as_wkt2() {
    let wkt = export(&GeographicCRS::epsg_4326(), WktConvention::Wkt2_2015);
    assert_eq!(
        wkt,
        "\
GEODCRS[\"WGS 84\",
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
    ID[\"EPSG\",4326]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt2_2019() {
    assert_eq!(
        export(&GeographicCRS::epsg_4326(), WktConvention::Wkt2_2019),
        "\
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
    ID[\"EPSG\",4326]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt2_simplified() {
    assert_eq!(
        export(
            &GeographicCRS::epsg_4326(),
            WktConvention::Wkt2_2015Simplified
        ),
        "\
GEODCRS[\"WGS 84\",
    DATUM[\"World Geodetic System 1984\",
        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north],
        AXIS[\"longitude\",east],
        UNIT[\"degree\",0.0174532925199433],
    ID[\"EPSG\",4326]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt2_simplified_single_line() {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015Simplified);
    formatter.set_lines(WktLines::Single);
    let wkt = Crs::Geographic(GeographicCRS::epsg_4326())
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert_eq!(
        wkt,
        "GEODCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],CS[ellipsoidal,2],AXIS[\"latitude\",north],AXIS[\"longitude\",east],UNIT[\"degree\",0.0174532925199433],ID[\"EPSG\",4326]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt2_2019_simplified() {
    assert_eq!(
        export(
            &GeographicCRS::epsg_4326(),
            WktConvention::Wkt2_2019Simplified
        ),
        "\
GEOGCRS[\"WGS 84\",
    DATUM[\"World Geodetic System 1984\",
        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north],
        AXIS[\"longitude\",east],
        UNIT[\"degree\",0.0174532925199433],
    ID[\"EPSG\",4326]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt1_gdal() {
    assert_eq!(
        export(&GeographicCRS::epsg_4326(), WktConvention::Wkt1Gdal),
        "\
GEOGCS[\"WGS 84\",
    DATUM[\"WGS_1984\",
        SPHEROID[\"WGS 84\",6378137,298.257223563,
            AUTHORITY[\"EPSG\",\"7030\"]],
        AUTHORITY[\"EPSG\",\"6326\"]],
    PRIMEM[\"Greenwich\",0,
        AUTHORITY[\"EPSG\",\"8901\"]],
    UNIT[\"degree\",0.0174532925199433,
        AUTHORITY[\"EPSG\",\"9122\"]],
    AUTHORITY[\"EPSG\",\"4326\"]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt1_gdal_with_axis() {
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    formatter.set_output_axis(OutputAxisRule::Yes);
    let wkt = Crs::Geographic(GeographicCRS::epsg_4326())
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEOGCS[\"WGS 84\",
    DATUM[\"WGS_1984\",
        SPHEROID[\"WGS 84\",6378137,298.257223563,
            AUTHORITY[\"EPSG\",\"7030\"]],
        AUTHORITY[\"EPSG\",\"6326\"]],
    PRIMEM[\"Greenwich\",0,
        AUTHORITY[\"EPSG\",\"8901\"]],
    UNIT[\"degree\",0.0174532925199433,
        AUTHORITY[\"EPSG\",\"9122\"]],
    AXIS[\"Latitude\",NORTH],
    AXIS[\"Longitude\",EAST],
    AUTHORITY[\"EPSG\",\"4326\"]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt1_esri_without_database() {
    assert_eq!(
        export(&GeographicCRS::epsg_4326(), WktConvention::Wkt1Esri),
        "GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]"
    );
}

#[test]
fn crs_epsg_4326_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = export_db(
        &Crs::Geographic(GeographicCRS::epsg_4326()),
        WktConvention::Wkt1Esri,
        db,
    );
    assert_eq!(
        wkt,
        "GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]"
    );
}

#[test]
fn crs_epsg_4326_as_proj_string() {
    assert_eq!(
        GeographicCRS::epsg_4326().export_to_proj_string(),
        "+proj=longlat +datum=WGS84 +no_defs +type=crs"
    );
}

#[test]
fn crs_epsg_4979_as_wkt2_simplified() {
    assert_eq!(
        export(
            &GeographicCRS::epsg_4979(),
            WktConvention::Wkt2_2015Simplified
        ),
        "\
GEODCRS[\"WGS 84\",
    DATUM[\"World Geodetic System 1984\",
        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
    CS[ellipsoidal,3],
        AXIS[\"latitude\",north,
            UNIT[\"degree\",0.0174532925199433]],
        AXIS[\"longitude\",east,
            UNIT[\"degree\",0.0174532925199433]],
        AXIS[\"ellipsoidal height\",up,
            UNIT[\"metre\",1]],
    ID[\"EPSG\",4979]]"
    );
}

#[test]
fn crs_epsg_4979_as_wkt2_2019_simplified() {
    assert_eq!(
        export(
            &GeographicCRS::epsg_4979(),
            WktConvention::Wkt2_2019Simplified
        ),
        "\
GEOGCRS[\"WGS 84\",
    DATUM[\"World Geodetic System 1984\",
        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
    CS[ellipsoidal,3],
        AXIS[\"latitude\",north,
            UNIT[\"degree\",0.0174532925199433]],
        AXIS[\"longitude\",east,
            UNIT[\"degree\",0.0174532925199433]],
        AXIS[\"ellipsoidal height\",up,
            UNIT[\"metre\",1]],
    ID[\"EPSG\",4979]]"
    );
}

#[test]
fn crs_epsg_4979_as_wkt1_gdal() {
    let error = Crs::Geographic(GeographicCRS::epsg_4979())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert!(error.is_err());
}

#[test]
fn crs_epsg_4979_as_wkt1_gdal_with_axis_not_strict_mode() {
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    formatter
        .set_strictness(WktStrictness::Lenient)
        .set_output_axis(OutputAxisRule::Yes);
    let wkt = Crs::Geographic(GeographicCRS::epsg_4979())
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEOGCS[\"WGS 84\",
    DATUM[\"WGS_1984\",
        SPHEROID[\"WGS 84\",6378137,298.257223563,
            AUTHORITY[\"EPSG\",\"7030\"]],
        AUTHORITY[\"EPSG\",\"6326\"]],
    PRIMEM[\"Greenwich\",0,
        AUTHORITY[\"EPSG\",\"8901\"]],
    UNIT[\"degree\",0.0174532925199433,
        AUTHORITY[\"EPSG\",\"9122\"]],
    AXIS[\"Latitude\",NORTH],
    AXIS[\"Longitude\",EAST],
    AXIS[\"Ellipsoidal height\",UP],
    AUTHORITY[\"EPSG\",\"4979\"]]"
    );
}

#[test]
fn crs_epsg_4979_as_wkt1_gdal_with_ellipsoidal_height_as_vertical_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db);
    formatter.set_ellipsoidal_height(EllipsoidalHeightPolicy::AllowAsVertical);
    let wkt = Crs::Geographic(GeographicCRS::epsg_4979())
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
COMPD_CS[\"WGS 84 + Ellipsoid (metre)\",
    GEOGCS[\"WGS 84\",
        DATUM[\"WGS_1984\",
            SPHEROID[\"WGS 84\",6378137,298.257223563,
                AUTHORITY[\"EPSG\",\"7030\"]],
            AUTHORITY[\"EPSG\",\"6326\"]],
        PRIMEM[\"Greenwich\",0,
            AUTHORITY[\"EPSG\",\"8901\"]],
        UNIT[\"degree\",0.0174532925199433,
            AUTHORITY[\"EPSG\",\"9122\"]],
        AUTHORITY[\"EPSG\",\"4326\"]],
    VERT_CS[\"Ellipsoid (metre)\",
        VERT_DATUM[\"Ellipsoid\",2002],
        UNIT[\"metre\",1,
            AUTHORITY[\"EPSG\",\"9001\"]],
        AXIS[\"Ellipsoidal height\",UP]]]"
    );
}

#[test]
fn crs_geographic3d_crs_as_wkt1_esri_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("7087")
        .expect("EPSG:7087");
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "GEOGCS[\"RGTAAF07_(lon-lat)_3D\",\
DATUM[\"D_Reseau_Geodesique_des_Terres_Australes_et_Antarctiques_Francaises_2007\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],\
UNIT[\"Degree\",0.0174532925199433],\
LINUNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_iau_1000_as_wkt2() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "IAU_2015")
        .create_coordinate_reference_system("1000")
        .expect("IAU_2015:1000");
    let wkt = crs
        .export_to_wkt(
            &mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db.clone()),
        )
        .expect("wkt");
    assert!(
        wkt.contains("ID[\"IAU\",1000,2015]"),
        "missing IAU versioned id in {wkt}"
    );
    let again = WktParser::new()
        .create_from_wkt(&wkt)
        .expect("parse IAU WKT")
        .crs()
        .expect("crs")
        .clone();
    let wkt2 = again
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db))
        .expect("wkt");
    assert!(
        wkt2.contains("ID[\"IAU\",1000,2015]"),
        "missing IAU versioned id after import in {wkt2}"
    );
}

#[test]
fn crs_iau_1000_as_projjson() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "IAU_2015")
        .create_coordinate_reference_system("1000")
        .expect("IAU_2015:1000");
    let json = crs.export_to_json(&db).expect("PROJJSON");
    assert!(
        json.contains("\"authority\": \"IAU\","),
        "missing IAU authority in {json}"
    );
    assert!(
        json.contains("\"code\": 1000,"),
        "missing IAU code in {json}"
    );
    assert!(
        json.contains("\"version\": 2015"),
        "missing IAU version in {json}"
    );
    let again = proj_rs::create_from_user_input(&json, &db).expect("import PROJJSON");
    let wkt = again
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db))
        .expect("wkt");
    assert!(
        wkt.contains("ID[\"IAU\",1000,2015]"),
        "missing IAU versioned id after PROJJSON import in {wkt}"
    );
}

fn geocentric_4328() -> GeodeticCRS {
    GeodeticCRS::create(
        "WGS 84",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("geocentric")
    .with_identifier("EPSG", "4328")
}

#[test]
fn crs_geodetic_crs_both_datum_datum_ensemble_null() {
    let error = GeodeticCRS::create(
        "",
        None,
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    );
    assert!(error.is_err());
}

#[test]
fn crs_geodetic_crs_both_datum_datum_ensemble_non_null() {
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
        ],
        100.0,
    )
    .expect("ensemble");
    let error = GeodeticCRS::create(
        "",
        Some(GeodeticReferenceFrame::epsg_6326()),
        Some(ensemble),
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    );
    assert!(error.is_err());
}

#[test]
fn crs_geocentric_crs_as_wkt2() {
    let crs = geocentric_4328();
    let expected = "\
GEODCRS[\"WGS 84\",
    DATUM[\"World Geodetic System 1984\",
        ELLIPSOID[\"WGS 84\",6378137,298.257223563,
            LENGTHUNIT[\"metre\",1]]],
    PRIMEM[\"Greenwich\",0,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    CS[Cartesian,3],
        AXIS[\"(X)\",geocentricX,
            ORDER[1],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(Y)\",geocentricY,
            ORDER[2],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(Z)\",geocentricZ,
            ORDER[3],
            LENGTHUNIT[\"metre\",1]],
    ID[\"EPSG\",4328]]";
    let wkt2 = Crs::Geodetic(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt2");
    assert_eq!(wkt2, expected);
    let wkt2019 = Crs::Geodetic(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2 2019");
    assert_eq!(wkt2019, expected);
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!Crs::Geodetic(crs).is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
}

#[test]
fn crs_geocentric_crs_as_wkt2_simplified() {
    let wkt = Crs::Geodetic(geocentric_4328())
        .export_to_wkt(&mut WktFormatter::create(
            WktConvention::Wkt2_2015Simplified,
        ))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEODCRS[\"WGS 84\",
    DATUM[\"World Geodetic System 1984\",
        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
    CS[Cartesian,3],
        AXIS[\"(X)\",geocentricX],
        AXIS[\"(Y)\",geocentricY],
        AXIS[\"(Z)\",geocentricZ],
        UNIT[\"metre\",1],
    ID[\"EPSG\",4328]]"
    );
}

#[test]
fn crs_geocentric_crs_as_wkt1_gdal() {
    let wkt = Crs::Geodetic(geocentric_4328())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEOCCS[\"WGS 84\",
    DATUM[\"WGS_1984\",
        SPHEROID[\"WGS 84\",6378137,298.257223563,
            AUTHORITY[\"EPSG\",\"7030\"]],
        AUTHORITY[\"EPSG\",\"6326\"]],
    PRIMEM[\"Greenwich\",0,
        AUTHORITY[\"EPSG\",\"8901\"]],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Geocentric X\",OTHER],
    AXIS[\"Geocentric Y\",OTHER],
    AXIS[\"Geocentric Z\",NORTH],
    AUTHORITY[\"EPSG\",\"4328\"]]"
    );
}

#[test]
fn crs_epsg_4978_as_wkt1_gdal_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = Crs::Geodetic(GeodeticCRS::epsg_4978())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEOCCS[\"WGS 84\",
    DATUM[\"WGS_1984\",
        SPHEROID[\"WGS 84\",6378137,298.257223563,
            AUTHORITY[\"EPSG\",\"7030\"]],
        AUTHORITY[\"EPSG\",\"6326\"]],
    PRIMEM[\"Greenwich\",0,
        AUTHORITY[\"EPSG\",\"8901\"]],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Geocentric X\",OTHER],
    AXIS[\"Geocentric Y\",OTHER],
    AXIS[\"Geocentric Z\",NORTH],
    AUTHORITY[\"EPSG\",\"4978\"]]"
    );
}

#[test]
fn crs_geocentric_crs_as_proj_string() {
    assert_eq!(
        geocentric_4328().export_to_proj_string().expect("proj"),
        "+proj=geocent +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

fn geocentric_with_unit(name: &str, factor: f64) -> GeodeticCRS {
    GeodeticCRS::create(
        "",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::new(
            name,
            factor,
            UnitKind::Linear,
        ))),
    )
    .expect("geocentric")
}

#[test]
fn crs_geocentric_crs_non_meter_unit_as_proj_string() {
    let crs = geocentric_with_unit("kilometre", 1000.0);
    let operation = proj_rs::CoordinateOperationFactory::create()
        .create_operation_proj_string(
            &Crs::Geographic(GeographicCRS::epsg_4326()),
            &Crs::Geodetic(crs.clone()),
        )
        .expect("operation");
    assert_eq!(
        operation,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=cart +ellps=WGS84 +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=km +z_out=km"
    );
    assert!(crs.export_to_proj_string().is_err());
}

#[test]
fn crs_geodeticcrs_identify_no_db() {
    let geocentric = GeodeticCRS::create(
        "",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("geocentric");
    assert!(geocentric.identify().is_empty());

    let canonical = |found: Vec<(Crs, i32)>, confidence: i32| {
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, confidence);
        assert_eq!(found[0].0.name(), "WGS 84");
        assert_eq!(found[0].0.identifiers()[0].code(), "4326");
        assert_eq!(found[0].0.identifiers()[0].code_space(), "EPSG");
    };

    let wgs84 = Crs::Geographic(GeographicCRS::epsg_4326());
    canonical(wgs84.identify(), 100);
    canonical(
        Crs::Geodetic(GeodeticCRS::Geographic(GeographicCRS::epsg_4326())).identify(),
        100,
    );

    let named = GeographicCRS::create(
        "WGS 84",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("named");
    canonical(Crs::Geographic(named).identify(), 100);

    let compact = GeographicCRS::create(
        "WGS84",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("compact name");
    canonical(Crs::Geographic(compact).identify(), 90);

    let longlat = GeographicCRS::create(
        "WGS 84",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    )
    .expect("longlat");
    canonical(Crs::Geographic(longlat).identify(), 25);

    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db);
    let parsed = parser
        .create_from_wkt(
            "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",\
6378137,298.257223563]],PRIMEM[\"Greenwich\",0],\
UNIT[\"Degree\",0.0174532925199433]]",
        )
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    assert!(matches!(parsed, Crs::Geographic(_) | Crs::Geodetic(_)));
    canonical(parsed.identify(), 100);
}

#[test]
fn crs_geodeticcrs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let all = AuthorityFactory::create(&db, "");

    let no_match = GeographicCRS::create(
        "",
        Some(GeodeticReferenceFrame::create(
            "",
            proj_rs::iso19111::Ellipsoid::create_flattened_sphere("", 6_378_137.0, 10.0),
            PrimeMeridian::greenwich(),
        )),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("no match");
    assert!(Crs::Geographic(no_match).identify_with(None).is_empty());

    let geocentric = GeodeticCRS::create(
        "",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("geocentric");
    let found = geocentric.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4978"));
    assert_eq!(found[0].1, 70);

    let unnamed = geographic(
        "unnamed",
        GeodeticReferenceFrame::epsg_6326(),
        UnitOfMeasure::degree(),
    );
    let found = unnamed.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 70);

    let foobar = geographic(
        "foobar",
        GeodeticReferenceFrame::epsg_6326(),
        UnitOfMeasure::degree(),
    );
    let found = foobar.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 70);

    let found = Crs::Geographic(GeographicCRS::epsg_4326()).identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 100);
    assert_eq!(domain_count(&found[0].0), 0);

    let named = geographic(
        "WGS 84",
        GeodeticReferenceFrame::epsg_6326(),
        UnitOfMeasure::degree(),
    );
    let found = named.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 100);
    assert_eq!(domain_count(&found[0].0), 0);

    let ntf = geographic(
        "NTF (Paris)",
        GeographicCRS::epsg_4807().datum().expect("datum").clone(),
        UnitOfMeasure::grad(),
    );
    let found = ntf.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4807"));
    assert_eq!(found[0].1, 100);

    let found = Crs::Geographic(GeographicCRS::epsg_4807()).identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4807"));
    assert_eq!(found[0].1, 100);

    let missing = geographic(
        "NTF (Paris)",
        GeographicCRS::epsg_4807().datum().expect("datum").clone(),
        UnitOfMeasure::grad(),
    )
    .with_identifier("EPSG", "1");
    assert!(missing.identify_with(Some(&factory)).is_empty());

    let wrong = geographic(
        "NTF (Paris)",
        GeographicCRS::epsg_4807().datum().expect("datum").clone(),
        UnitOfMeasure::grad(),
    )
    .with_identifier("EPSG", "4326");
    let found = wrong.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 25);

    let compact = geographic(
        "WGS84",
        GeodeticReferenceFrame::epsg_6326(),
        UnitOfMeasure::degree(),
    );
    let found = compact.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 90);

    let longlat = GeographicCRS::create(
        "WGS 84",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    )
    .expect("longlat");
    let found = longlat.identify_with(Some(&factory));
    assert_eq!(found.len(), 3);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 25);

    let by_ellipsoid = geographic(
        "",
        GeodeticReferenceFrame::anonymous_wgs84(),
        UnitOfMeasure::degree(),
    );
    let found = by_ellipsoid.identify_with(Some(&factory));
    assert!(found.len() > 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 60);

    let unnamed_ellipsoid = geographic(
        "unnamed",
        GeodeticReferenceFrame::anonymous_wgs84(),
        UnitOfMeasure::degree(),
    );
    let found = unnamed_ellipsoid.identify_with(Some(&factory));
    assert!(found.len() > 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 60);

    let described = create_crs_from_proj_string(
        "+proj=longlat +a=6378521.049 +rf=298.257222100883 +axis=neu +type=crs",
    )
    .expect("proj");
    assert_eq!(described.identify_with(Some(&all)).len(), 5);

    let lisboa = wkt_crs(
        "GEODCRS[\"GCS_Datum_Lisboa_Bessel\",\n\
    DATUM[\"D_Datum_Lisboa_Bessel\",\n\
        ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,\n\
            LENGTHUNIT[\"metre\",1]]],\n\
    PRIMEM[\"Greenwich\",0,\n\
        ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
    CS[ellipsoidal,2],\n\
        AXIS[\"geodetic latitude (Lat)\",north,\n\
            ORDER[1],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
        AXIS[\"geodetic longitude (Lon)\",east,\n\
            ORDER[2],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]]]",
        None,
    );
    let found = lisboa.identify_with(Some(&all));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("ESRI", "104105"));
    assert_eq!(found[0].1, 100);

    let missing_name = geographic(
        "foobar",
        GeodeticReferenceFrame::epsg_6326(),
        UnitOfMeasure::degree(),
    )
    .with_identifier("EPSG", "i_dont_exist");
    assert!(missing_name.identify_with(Some(&factory)).is_empty());

    let from_datum =
        create_crs_from_proj_string("+proj=longlat +datum=WGS84 +type=crs").expect("wgs84");
    let found = from_datum.identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 70);

    let from_ellps =
        create_crs_from_proj_string("+proj=longlat +ellps=GRS80 +type=crs").expect("grs80");
    let found = from_ellps.identify_with(Some(&factory));
    assert!(found
        .iter()
        .any(|(crs, confidence)| { epsg_code(crs) == ("EPSG", "7844") && *confidence == 60 }));

    let gda_with_id = wkt_crs(GDA2020_WITH_ID, None);
    let found = gda_with_id.identify_with(Some(&all));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "7844"));
    assert_eq!(found[0].1, 100);
    assert!(gda_with_id.is_equivalent_to_db(&found[0].0, &db));
    assert!(found[0].0.is_equivalent_to_db(&gda_with_id, &db));

    let gda = wkt_crs(GDA2020, None);
    let found = gda.identify_with(Some(&all));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "7844"));
    assert_eq!(found[0].1, 100);
    assert!(gda.is_equivalent_to_db(&found[0].0, &db));
    assert!(found[0].0.is_equivalent_to_db(&gda, &db));

    let gda_esri = wkt_crs(
        "GEOGCS[\"GDA2020\",DATUM[\"D_GDA2020\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],\
UNIT[\"Degree\",0.017453292519943295]]",
        Some(db),
    );
    let found = gda_esri.identify_with(Some(&all));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "7844"));
    assert_eq!(found[0].1, 100);

    let ensemble = wkt_crs(WGS84_ENSEMBLE, None);
    let found = ensemble.identify_with(Some(&all));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 100);

    let ensemble_unknown = wkt_crs(WGS84_ENSEMBLE_UNKNOWN, None);
    let found = ensemble_unknown.identify_with(Some(&all));
    assert_eq!(found.len(), 1);
    assert_eq!(epsg_code(&found[0].0), ("EPSG", "4326"));
    assert_eq!(found[0].1, 70);
}

fn geographic(name: &str, datum: GeodeticReferenceFrame, unit: UnitOfMeasure) -> GeographicCRS {
    GeographicCRS::create(
        name,
        Some(datum),
        None,
        EllipsoidalCS::create_latitude_longitude(unit),
    )
    .expect("geographic")
}

fn epsg_code(crs: &Crs) -> (&str, &str) {
    let identifier = &crs.identifiers()[0];
    (identifier.code_space(), identifier.code())
}

fn domain_count(crs: &Crs) -> usize {
    match crs {
        Crs::Geographic(crs) => crs.domains().len(),
        Crs::Geodetic(GeodeticCRS::Geographic(crs)) => crs.domains().len(),
        Crs::Geodetic(GeodeticCRS::Geocentric(crs)) => crs.domains().len(),
        _ => usize::MAX,
    }
}

fn wkt_crs(wkt: &str, db: Option<DatabaseContext>) -> Crs {
    let mut parser = WktParser::new();
    if let Some(db) = db {
        parser.attach_database(db);
    }
    parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

const GDA2020_WITH_ID: &str = "\
GEOGCRS[\"GDA2020\",\n\
    DATUM[\"GDA2020\",\n\
        ELLIPSOID[\"GRS_1980\",6378137,298.257222101,\n\
            LENGTHUNIT[\"metre\",1]]],\n\
    PRIMEM[\"Greenwich\",0,\n\
        ANGLEUNIT[\"Degree\",0.0174532925199433]],\n\
    CS[ellipsoidal,2],\n\
        AXIS[\"geodetic latitude (Lat)\",north,\n\
            ORDER[1],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
        AXIS[\"geodetic longitude (Lon)\",east,\n\
            ORDER[2],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
    ID[\"EPSG\",7844]]";

const GDA2020: &str = "\
GEOGCRS[\"GDA2020\",\n\
    DATUM[\"GDA2020\",\n\
        ELLIPSOID[\"GRS_1980\",6378137,298.257222101,\n\
            LENGTHUNIT[\"metre\",1]]],\n\
    PRIMEM[\"Greenwich\",0,\n\
        ANGLEUNIT[\"Degree\",0.0174532925199433]],\n\
    CS[ellipsoidal,2],\n\
        AXIS[\"geodetic latitude (Lat)\",north,\n\
            ORDER[1],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
        AXIS[\"geodetic longitude (Lon)\",east,\n\
            ORDER[2],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]]]";

const WGS84_ENSEMBLE: &str = "\
GEOGCRS[\"WGS 84\",\
  ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
     MEMBER[\"World Geodetic System 1984 (Transit)\",\
       ID[\"EPSG\",1166]],\
     MEMBER[\"World Geodetic System 1984 (G730)\",\
       ID[\"EPSG\",1152]],\
     MEMBER[\"World Geodetic System 1984 (G873)\",\
       ID[\"EPSG\",1153]],\
     MEMBER[\"World Geodetic System 1984 (G1150)\",\
       ID[\"EPSG\",1154]],\
     MEMBER[\"World Geodetic System 1984 (G1674)\",\
       ID[\"EPSG\",1155]],\
     MEMBER[\"World Geodetic System 1984 (G1762)\",\
       ID[\"EPSG\",1156]],\
     ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
      LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]],\
      ID[\"EPSG\",7030]],\
     ENSEMBLEACCURACY[2]],\
  PRIMEM[\"Greenwich\",0,\
    ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9102]],\
    ID[\"EPSG\",8901]],\
  CS[ellipsoidal,2,\
    ID[\"EPSG\",6422]],\
  AXIS[\"Geodetic latitude (Lat)\",north,\
    ORDER[1]],\
  AXIS[\"Geodetic longitude (Lon)\",east,\
    ORDER[2]],\
  ANGLEUNIT[\"degree (supplier to define representation)\",\
0.0174532925199433,ID[\"EPSG\",9122]]]";

const WGS84_ENSEMBLE_UNKNOWN: &str = "\
GEOGCRS[\"unknown\",\
  ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
     MEMBER[\"World Geodetic System 1984 (Transit)\",\
       ID[\"EPSG\",1166]],\
     MEMBER[\"World Geodetic System 1984 (G730)\",\
       ID[\"EPSG\",1152]],\
     MEMBER[\"World Geodetic System 1984 (G873)\",\
       ID[\"EPSG\",1153]],\
     MEMBER[\"World Geodetic System 1984 (G1150)\",\
       ID[\"EPSG\",1154]],\
     MEMBER[\"World Geodetic System 1984 (G1674)\",\
       ID[\"EPSG\",1155]],\
     MEMBER[\"World Geodetic System 1984 (G1762)\",\
       ID[\"EPSG\",1156]],\
     ELLIPSOID[\"WGS 84\",6378137,298.257223563,\
      LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]],\
      ID[\"EPSG\",7030]],\
     ENSEMBLEACCURACY[2]],\
  PRIMEM[\"Greenwich\",0,\
    ANGLEUNIT[\"degree\",0.0174532925199433,ID[\"EPSG\",9102]],\
    ID[\"EPSG\",8901]],\
  CS[ellipsoidal,2,\
    ID[\"EPSG\",6422]],\
  AXIS[\"Geodetic latitude (Lat)\",north,\
    ORDER[1]],\
  AXIS[\"Geodetic longitude (Lon)\",east,\
    ORDER[2]],\
  ANGLEUNIT[\"degree (supplier to define representation)\",\
0.0174532925199433,ID[\"EPSG\",9122]]]";

#[test]
fn crs_geocentric_crs_unsupported_unit_as_proj_string() {
    let crs = geocentric_with_unit("my unit", 500.0);
    let operation = proj_rs::CoordinateOperationFactory::create()
        .create_operation_proj_string(
            &Crs::Geographic(GeographicCRS::epsg_4326()),
            &Crs::Geodetic(crs),
        )
        .expect("operation");
    assert_eq!(
        operation,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=cart +ellps=WGS84 +step +proj=unitconvert +xy_in=m +z_in=m +xy_out=500 +z_out=500"
    );
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

fn export_projected(crs: &ProjectedCRS, convention: WktConvention) -> String {
    Crs::Projected(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(convention))
        .expect("wkt")
}

#[test]
fn crs_projected_crs_as_wkt2() {
    assert_eq!(
        export_projected(&utm_31n(), WktConvention::Wkt2_2015),
        "\
PROJCRS[\"WGS 84 / UTM zone 31N\",
    BASEGEODCRS[\"WGS 84\",
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
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1]],
    ID[\"EPSG\",32631]]"
    );
}

#[test]
fn crs_projected_crs_as_wkt2_2019() {
    assert_eq!(
        export_projected(&utm_31n(), WktConvention::Wkt2_2019),
        "\
PROJCRS[\"WGS 84 / UTM zone 31N\",
    BASEGEOGCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        ID[\"EPSG\",4326]],
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
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1]],
    ID[\"EPSG\",32631]]"
    );
}

#[test]
fn crs_projected_crs_as_wkt2_simplified() {
    assert_eq!(
        export_projected(&utm_31n(), WktConvention::Wkt2_2015Simplified),
        "\
PROJCRS[\"WGS 84 / UTM zone 31N\",
    BASEGEODCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
        UNIT[\"degree\",0.0174532925199433]],
    CONVERSION[\"UTM zone 31N\",
        METHOD[\"Transverse Mercator\"],
        PARAMETER[\"Latitude of natural origin\",0],
        PARAMETER[\"Longitude of natural origin\",3],
        PARAMETER[\"Scale factor at natural origin\",0.9996],
        PARAMETER[\"False easting\",500000],
        PARAMETER[\"False northing\",0]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east],
        AXIS[\"(N)\",north],
        UNIT[\"metre\",1],
    ID[\"EPSG\",32631]]"
    );
}

#[test]
fn crs_projected_crs_as_wkt2_2019_simplified() {
    assert_eq!(
        export_projected(&utm_31n(), WktConvention::Wkt2_2019Simplified),
        "\
PROJCRS[\"WGS 84 / UTM zone 31N\",
    BASEGEOGCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
        UNIT[\"degree\",0.0174532925199433]],
    CONVERSION[\"UTM zone 31N\",
        METHOD[\"Transverse Mercator\"],
        PARAMETER[\"Latitude of natural origin\",0],
        PARAMETER[\"Longitude of natural origin\",3],
        PARAMETER[\"Scale factor at natural origin\",0.9996],
        PARAMETER[\"False easting\",500000],
        PARAMETER[\"False northing\",0]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east],
        AXIS[\"(N)\",north],
        UNIT[\"metre\",1],
    ID[\"EPSG\",32631]]"
    );
}

#[test]
fn crs_projected_crs_as_wkt1_gdal() {
    let expected = "\
PROJCS[\"WGS 84 / UTM zone 31N\",
    GEOGCS[\"WGS 84\",
        DATUM[\"WGS_1984\",
            SPHEROID[\"WGS 84\",6378137,298.257223563,
                AUTHORITY[\"EPSG\",\"7030\"]],
            AUTHORITY[\"EPSG\",\"6326\"]],
        PRIMEM[\"Greenwich\",0,
            AUTHORITY[\"EPSG\",\"8901\"]],
        UNIT[\"degree\",0.0174532925199433,
            AUTHORITY[\"EPSG\",\"9122\"]],
        AUTHORITY[\"EPSG\",\"4326\"]],
    PROJECTION[\"Transverse_Mercator\"],
    PARAMETER[\"latitude_of_origin\",0],
    PARAMETER[\"central_meridian\",3],
    PARAMETER[\"scale_factor\",0.9996],
    PARAMETER[\"false_easting\",500000],
    PARAMETER[\"false_northing\",0],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Easting\",EAST],
    AXIS[\"Northing\",NORTH],
    AUTHORITY[\"EPSG\",\"32631\"]]";
    assert_eq!(
        export_projected(&utm_31n(), WktConvention::Wkt1Gdal),
        expected
    );
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    formatter.set_output_axis(OutputAxisRule::Yes);
    let forced = Crs::Projected(utm_31n())
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert_eq!(forced, expected);
}

#[test]
fn crs_projected_crs_with_other_deprecated_crs_of_same_name_as_wkt1_esri() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_projected_crs("3800")
        .expect("EPSG:3800");
    let expected = "\
PROJCS[\"NAD_1927_3TM_120\",GEOGCS[\"GCS_North_American_1927\",\
DATUM[\"D_North_American_1927\",\
SPHEROID[\"Clarke_1866\",6378206.4,294.978698213898]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Transverse_Mercator\"],\
PARAMETER[\"False_Easting\",0.0],\
PARAMETER[\"False_Northing\",0.0],\
PARAMETER[\"Central_Meridian\",-120.0],\
PARAMETER[\"Scale_Factor\",0.9999],\
PARAMETER[\"Latitude_Of_Origin\",0.0],UNIT[\"Meter\",1.0]]";
    let wkt = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db.clone()))
        .expect("wkt");
    assert_eq!(wkt, expected);
    let parsed = WktParser::new()
        .attach_database(db.clone())
        .create_from_wkt(&wkt)
        .expect("parse")
        .crs()
        .expect("crs")
        .clone();
    assert_eq!(parsed.name(), "NAD27 / Alberta 3TM ref merid 120 W");
    let again = parsed
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(again, expected);
}

#[test]
fn crs_projected_crs_with_esri_code_as_wkt1_esri() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "ESRI")
        .create_projected_crs("102113")
        .expect("ESRI:102113");
    let wkt = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "PROJCS[\"WGS_1984_Web_Mercator\",\
GEOGCS[\"GCS_WGS_1984_Major_Auxiliary_Sphere\",\
DATUM[\"D_WGS_1984_Major_Auxiliary_Sphere\",\
SPHEROID[\"WGS_1984_Major_Auxiliary_Sphere\",6378137.0,0.0]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Mercator\"],PARAMETER[\"False_Easting\",0.0],\
PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],\
PARAMETER[\"Standard_Parallel_1\",0.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_projected_crs_from_wkt1_esri_as_wkt1_esri() {
    let db = DatabaseContext::create().expect("proj.db");
    let esri_wkt = "\
PROJCS[\"WGS_1984_Web_Mercator\",\
GEOGCS[\"GCS_WGS_1984_Major_Auxiliary_Sphere\",\
DATUM[\"D_WGS_1984_Major_Auxiliary_Sphere\",\
SPHEROID[\"WGS_1984_Major_Auxiliary_Sphere\",6378137.0,0.0]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Mercator\"],PARAMETER[\"False_Easting\",0.0],\
PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],\
PARAMETER[\"Standard_Parallel_1\",0.0],UNIT[\"Meter\",1.0]]";
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(esri_wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    assert!(matches!(crs, Crs::Projected(_)));
    let exported = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("export");
    assert_eq!(exported, esri_wkt);
}

#[test]
fn crs_projected_crs_from_wkt1_esri_as_wkt1_esri_s_jtsk03_krovak_east_north() {
    let db = DatabaseContext::create().expect("proj.db");
    let esri_wkt = "\
PROJCS[\"S-JTSK_[JTSK03]_Krovak_East_North\",\
GEOGCS[\"S-JTSK_[JTSK03]\",DATUM[\"S-JTSK_[JTSK03]\",\
SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Krovak\"],\
PARAMETER[\"False_Easting\",0.0],\
PARAMETER[\"False_Northing\",0.0],\
PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],\
PARAMETER[\"Scale_Factor\",0.9999],\
PARAMETER[\"Azimuth\",30.2881397527778],\
PARAMETER[\"Longitude_Of_Center\",24.8333333333333],\
PARAMETER[\"Latitude_Of_Center\",49.5],\
PARAMETER[\"X_Scale\",-1.0],\
PARAMETER[\"Y_Scale\",1.0],\
PARAMETER[\"XY_Plane_Rotation\",90.0],\
UNIT[\"Meter\",1.0]]";
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(esri_wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    assert!(matches!(crs, Crs::Projected(_)));
    let expected_wkt2 = "\
PROJCRS[\"S-JTSK [JTSK03] / Krovak East North\",
    BASEGEOGCRS[\"S-JTSK [JTSK03]\",
        DATUM[\"System of the Unified Trigonometrical Cadastral Network [JTSK03]\",
            ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,
                LENGTHUNIT[\"metre\",1]],
            ID[\"EPSG\",1201]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"Degree\",0.0174532925199433]]],
    CONVERSION[\"unknown\",
        METHOD[\"Krovak (North Orientated)\",
            ID[\"EPSG\",1041]],
        PARAMETER[\"Latitude of projection centre\",49.5,
            ANGLEUNIT[\"Degree\",0.0174532925199433],
            ID[\"EPSG\",8811]],
        PARAMETER[\"Longitude of origin\",24.8333333333333,
            ANGLEUNIT[\"Degree\",0.0174532925199433],
            ID[\"EPSG\",8833]],
        PARAMETER[\"Co-latitude of cone axis\",30.2881397527778,
            ANGLEUNIT[\"Degree\",0.0174532925199433],
            ID[\"EPSG\",1036]],
        PARAMETER[\"Latitude of pseudo standard parallel\",78.5,
            ANGLEUNIT[\"Degree\",0.0174532925199433],
            ID[\"EPSG\",8818]],
        PARAMETER[\"Scale factor on pseudo standard parallel\",0.9999,
            SCALEUNIT[\"unity\",1],
            ID[\"EPSG\",8819]],
        PARAMETER[\"False easting\",0,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8806]],
        PARAMETER[\"False northing\",0,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8807]]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]]";
    let wkt2 = crs
        .export_to_wkt(
            &mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db.clone()),
        )
        .expect("wkt2");
    assert_eq!(wkt2, expected_wkt2);
    let exported = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("esri");
    assert_eq!(exported, esri_wkt);
}

#[test]
fn crs_projected_crs_from_epsg_as_wkt1_esri_s_jtsk03_krovak_east_north() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_projected_crs("8353")
        .expect("EPSG:8353");
    let wkt = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "PROJCS[\"S-JTSK_[JTSK03]_Krovak_East_North\",\
GEOGCS[\"S-JTSK_[JTSK03]\",DATUM[\"S-JTSK_[JTSK03]\",\
SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Krovak\"],\
PARAMETER[\"False_Easting\",0.0],\
PARAMETER[\"False_Northing\",0.0],\
PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],\
PARAMETER[\"Scale_Factor\",0.9999],\
PARAMETER[\"Azimuth\",30.2881397527778],\
PARAMETER[\"Longitude_Of_Center\",24.8333333333333],\
PARAMETER[\"Latitude_Of_Center\",49.5],\
PARAMETER[\"X_Scale\",-1.0],\
PARAMETER[\"Y_Scale\",1.0],\
PARAMETER[\"XY_Plane_Rotation\",90.0],\
UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_projected_crs_from_epsg_with_deprecated_esri_name_as_wkt1_esri() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_projected_crs("5186")
        .expect("EPSG:5186");
    let wkt = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "PROJCS[\"KGD2002_Central_Belt_2010\",GEOGCS[\"KGD2002\",\
DATUM[\"D_Korea_Geodetic_Datum_2002\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Transverse_Mercator\"],\
PARAMETER[\"False_Easting\",200000.0],\
PARAMETER[\"False_Northing\",600000.0],\
PARAMETER[\"Central_Meridian\",127.0],\
PARAMETER[\"Scale_Factor\",1.0],\
PARAMETER[\"Latitude_Of_Origin\",38.0],\
UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_projected_crs_from_wkt1_etrs89_with_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = "\
PROJCS[\"ETRS89 / UTM zone 32N (N-E)\",\
GEOGCS[\"ETRS89\",\
    DATUM[\"European_Terrestrial_Reference_System_1989\",\
        SPHEROID[\"GRS 1980\",6378137,298.257222101,\
            AUTHORITY[\"EPSG\",\"7019\"]],\
        AUTHORITY[\"EPSG\",\"6258\"]],\
    PRIMEM[\"Greenwich\",0,\
        AUTHORITY[\"EPSG\",\"8901\"]],\
    UNIT[\"degree\",0.0174532925199433,\
        AUTHORITY[\"EPSG\",\"9122\"]],\
    AUTHORITY[\"EPSG\",\"4258\"]],\
PROJECTION[\"Transverse_Mercator\"],\
PARAMETER[\"latitude_of_origin\",0],\
PARAMETER[\"central_meridian\",9],\
PARAMETER[\"scale_factor\",0.9996],\
PARAMETER[\"false_easting\",500000],\
PARAMETER[\"false_northing\",0],\
UNIT[\"metre\",1,\
    AUTHORITY[\"EPSG\",\"9001\"]],\
AXIS[\"Northing\",NORTH],\
AXIS[\"Easting\",EAST]]";
    let mut parser = WktParser::new();
    parser.attach_database(db);
    let crs = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    let Crs::Projected(projected) = crs else {
        panic!("ETRS89 / UTM zone 32N (N-E) is a projected CRS");
    };
    let datum = projected.base_crs().datum().expect("datum");
    assert_eq!(datum.name(), "European Terrestrial Reference System 1989");
}

#[test]
fn crs_projected_crs_from_wkt1_esri_etrs89_with_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = "\
PROJCS[\"ETRS89 / ETRS-LAEA\",\
GEOGCS[\"ETRS89\",\
DATUM[\"European_Terrestrial_Reference_System_1989\",\
SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.01745329251994328]],\
PROJECTION[\"Lambert_Azimuthal_Equal_Area\"],\
PARAMETER[\"latitude_of_center\",52],\
PARAMETER[\"longitude_of_center\",10],\
PARAMETER[\"false_easting\",4321000],\
PARAMETER[\"false_northing\",3210000],\
UNIT[\"metre\",1]]";
    let mut parser = WktParser::new();
    parser.attach_database(db);
    let crs = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    let Crs::Projected(projected) = crs else {
        panic!("ETRS89 / ETRS-LAEA is a projected CRS");
    };
    let datum = projected.base_crs().datum().expect("datum");
    assert_eq!(datum.name(), "European Terrestrial Reference System 1989");
}

#[test]
fn crs_projected_crs_3d_is_wkt1_equivalent_to_wkt2() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt1 = "\
PROJCS[\"WGS 84 / UTM zone 16N [EGM08-1]\",\
GEOGCS[\"WGS 84 / UTM zone 16N [EGM08-1]\",\
DATUM[\"WGS84\",SPHEROID[\"WGS84\",6378137.000,298.257223563,\
AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY[\"EPSG\",\"6326\"]],\
PRIMEM[\"Greenwich\",0.0000000000000000,\
AUTHORITY[\"EPSG\",\"8901\"]],\
UNIT[\"Degree\",0.01745329251994329547,\
AUTHORITY[\"EPSG\",\"9102\"]],AUTHORITY[\"EPSG\",\"32616\"]],\
UNIT[\"Meter\",1.00000000000000000000,\
AUTHORITY[\"EPSG\",\"9001\"]],\
PROJECTION[\"Transverse_Mercator\"],\
PARAMETER[\"latitude_of_origin\",0.0000000000000000],\
PARAMETER[\"central_meridian\",-87.0000000002777938],\
PARAMETER[\"scale_factor\",0.9996000000000000],\
PARAMETER[\"false_easting\",500000.000],\
PARAMETER[\"false_northing\",0.000],\
AXIS[\"Easting\",EAST],\
AXIS[\"Northing\",NORTH],\
AXIS[\"Height\",UP],\
AUTHORITY[\"EPSG\",\"32616\"]]";
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(wkt1)
        .expect("wkt1")
        .crs()
        .expect("projected")
        .clone();
    let Crs::Projected(projected) = crs else {
        panic!("the WKT1 object is a projected CRS");
    };
    let wkt2 = Crs::Projected(projected.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2");
    let mut parser = WktParser::new();
    parser.attach_database(db);
    let again = parser
        .create_from_wkt(&wkt2)
        .expect("parse wkt2")
        .crs()
        .expect("projected")
        .clone();
    let Crs::Projected(again) = again else {
        panic!("the WKT2 object is a projected CRS: {wkt2}");
    };
    assert!(
        projected.is_equivalent_to_with(&again, Equivalence::ExceptAxisOrder),
        "WKT1 projected 3D is not equivalent to its WKT2 form:\n{wkt2}"
    );
}

#[test]
fn crs_projected_crs_3d_as_wkt1_gdal_with_ellipsoidal_height_as_vertical_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_projected_crs("32631")
        .expect("EPSG:32631")
        .promote_to_3d(Some(&db));
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db);
    formatter.set_ellipsoidal_height(EllipsoidalHeightPolicy::AllowAsVertical);
    let wkt = Crs::Projected(crs)
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
COMPD_CS[\"WGS 84 / UTM zone 31N + Ellipsoid (metre)\",
    PROJCS[\"WGS 84 / UTM zone 31N\",
        GEOGCS[\"WGS 84\",
            DATUM[\"WGS_1984\",
                SPHEROID[\"WGS 84\",6378137,298.257223563,
                    AUTHORITY[\"EPSG\",\"7030\"]],
                AUTHORITY[\"EPSG\",\"6326\"]],
            PRIMEM[\"Greenwich\",0,
                AUTHORITY[\"EPSG\",\"8901\"]],
            UNIT[\"degree\",0.0174532925199433,
                AUTHORITY[\"EPSG\",\"9122\"]],
            AUTHORITY[\"EPSG\",\"4326\"]],
        PROJECTION[\"Transverse_Mercator\"],
        PARAMETER[\"latitude_of_origin\",0],
        PARAMETER[\"central_meridian\",3],
        PARAMETER[\"scale_factor\",0.9996],
        PARAMETER[\"false_easting\",500000],
        PARAMETER[\"false_northing\",0],
        UNIT[\"metre\",1,
            AUTHORITY[\"EPSG\",\"9001\"]],
        AXIS[\"Easting\",EAST],
        AXIS[\"Northing\",NORTH],
        AUTHORITY[\"EPSG\",\"32631\"]],
    VERT_CS[\"Ellipsoid (metre)\",
        VERT_DATUM[\"Ellipsoid\",2002],
        UNIT[\"metre\",1,
            AUTHORITY[\"EPSG\",\"9001\"]],
        AXIS[\"Ellipsoidal height\",UP]]]"
    );
}

#[test]
fn crs_projected_crs_as_wkt1_esri() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = Crs::Projected(utm_31n())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "PROJCS[\"WGS_1984_UTM_Zone_31N\",GEOGCS[\"GCS_WGS_1984\",\
DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Transverse_Mercator\"],\
PARAMETER[\"False_Easting\",500000.0],\
PARAMETER[\"False_Northing\",0.0],\
PARAMETER[\"Central_Meridian\",3.0],\
PARAMETER[\"Scale_Factor\",0.9996],\
PARAMETER[\"Latitude_Of_Origin\",0.0],\
UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_epsg_27561_projected_with_geodetic_in_grad_as_proj_string_and_wkt1() {
    let wkt = "\
PROJCRS[\"NTF (Paris) / Lambert Nord France\",
  BASEGEODCRS[\"NTF (Paris)\",
    DATUM[\"Nouvelle Triangulation Francaise (Paris)\",
      ELLIPSOID[\"Clarke 1880 (IGN)\",6378249.2,293.4660213,LENGTHUNIT[\"metre\",1.0]]],
    PRIMEM[\"Paris\",2.5969213,ANGLEUNIT[\"grad\",0.015707963268]]],
  CONVERSION[\"Lambert Nord France\",
    METHOD[\"Lambert Conic Conformal (1SP)\",ID[\"EPSG\",9801]],
    PARAMETER[\"Latitude of natural origin\",55,ANGLEUNIT[\"grad\",0.015707963268]],
    PARAMETER[\"Longitude of natural origin\",0,ANGLEUNIT[\"grad\",0.015707963268]],
    PARAMETER[\"Scale factor at natural origin\",0.999877341,SCALEUNIT[\"unity\",1.0]],
    PARAMETER[\"False easting\",600000,LENGTHUNIT[\"metre\",1.0]],
    PARAMETER[\"False northing\",200000,LENGTHUNIT[\"metre\",1.0]]],
  CS[cartesian,2],
    AXIS[\"easting (X)\",east,ORDER[1]],
    AXIS[\"northing (Y)\",north,ORDER[2]],
    LENGTHUNIT[\"metre\",1.0],
  ID[\"EPSG\",27561]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("projected")
        .clone();
    let Crs::Projected(projected) = &crs else {
        panic!("EPSG:27561 is a projected CRS");
    };
    assert_eq!(
        projected.export_to_proj_string().expect("proj"),
        "+proj=lcc +lat_1=49.5 +lat_0=49.5 +lon_0=0 +k_0=0.999877341 +x_0=600000 +y_0=200000 +ellps=clrk80ign +pm=paris +units=m +no_defs +type=crs"
    );
    let db = DatabaseContext::create().expect("proj.db");
    assert_eq!(
        crs.export_to_wkt(
            &mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db.clone())
        )
        .expect("wkt1"),
        "PROJCS[\"NTF (Paris) / Lambert Nord France\",\n    \
GEOGCS[\"NTF (Paris)\",\n        \
DATUM[\"Nouvelle_Triangulation_Francaise_Paris\",\n            \
SPHEROID[\"Clarke 1880 (IGN)\",6378249.2,293.4660213]],\n        \
PRIMEM[\"Paris\",2.33722917000759],\n        \
UNIT[\"grad\",0.015707963268]],\n    \
PROJECTION[\"Lambert_Conformal_Conic_1SP\"],\n    \
PARAMETER[\"latitude_of_origin\",55],\n    \
PARAMETER[\"central_meridian\",0],\n    \
PARAMETER[\"scale_factor\",0.999877341],\n    \
PARAMETER[\"false_easting\",600000],\n    \
PARAMETER[\"false_northing\",200000],\n    \
UNIT[\"metre\",1],\n    \
AXIS[\"Easting\",EAST],\n    \
AXIS[\"Northing\",NORTH],\n    \
AUTHORITY[\"EPSG\",\"27561\"]]"
    );
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
            .expect("esri"),
        "PROJCS[\"NTF_Paris_Lambert_Nord_France\",\
GEOGCS[\"GCS_NTF_Paris\",\
DATUM[\"Nouvelle_Triangulation_Francaise_(Paris)\",\
SPHEROID[\"Clarke_1880_IGN\",6378249.2,293.4660213]],\
PRIMEM[\"Paris\",2.33722917000759],\
UNIT[\"Grad\",0.015707963268]],\
PROJECTION[\"Lambert_Conformal_Conic\"],\
PARAMETER[\"False_Easting\",600000.0],\
PARAMETER[\"False_Northing\",200000.0],\
PARAMETER[\"Central_Meridian\",0.0],\
PARAMETER[\"Standard_Parallel_1\",55.0],\
PARAMETER[\"Scale_Factor\",0.999877341],\
PARAMETER[\"Latitude_Of_Origin\",55.0],\
UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_epsg_3040_projected_northing_easting_as_proj_string() {
    let wkt = "\
PROJCRS[\"ETRS89 / UTM zone 28N (N-E)\",
  BASEGEODCRS[\"ETRS89\",
    DATUM[\"European Terrestrial Reference System 1989\",
      ELLIPSOID[\"GRS 1980\",6378137,298.257222101,LENGTHUNIT[\"metre\",1.0]]]],
  CONVERSION[\"UTM zone 28N\",
    METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],
    PARAMETER[\"Latitude of natural origin\",0,ANGLEUNIT[\"degree\",0.01745329252]],
    PARAMETER[\"Longitude of natural origin\",-15,ANGLEUNIT[\"degree\",0.01745329252]],
    PARAMETER[\"Scale factor at natural origin\",0.9996,SCALEUNIT[\"unity\",1.0]],
    PARAMETER[\"False easting\",500000,LENGTHUNIT[\"metre\",1.0]],
    PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1.0]]],
  CS[cartesian,2],
    AXIS[\"northing (N)\",north,ORDER[1]],
    AXIS[\"easting (E)\",east,ORDER[2]],
    LENGTHUNIT[\"metre\",1.0],
  ID[\"EPSG\",3040]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("projected")
        .clone();
    let Crs::Projected(projected) = crs else {
        panic!("EPSG:3040 is a projected CRS");
    };
    assert_eq!(
        projected.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=28 +ellps=GRS80 +units=m +no_defs +type=crs"
    );
}

#[test]
fn crs_epsg_2222_projected_unit_foot_as_proj_string_and_wkt1() {
    let wkt = "\
PROJCRS[\"NAD83 / Arizona East (ft)\",
  BASEGEODCRS[\"NAD83\",
    DATUM[\"North American Datum 1983\",
      ELLIPSOID[\"GRS 1980\",6378137,298.257222101,LENGTHUNIT[\"metre\",1.0]]]],
  CONVERSION[\"SPCS83 Arizona East zone (International feet)\",
    METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],
    PARAMETER[\"Latitude of natural origin\",31,ANGLEUNIT[\"degree\",0.01745329252]],
    PARAMETER[\"Longitude of natural origin\",-110.166666666667,ANGLEUNIT[\"degree\",0.01745329252]],
    PARAMETER[\"Scale factor at natural origin\",0.9999,SCALEUNIT[\"unity\",1.0]],
    PARAMETER[\"False easting\",700000,LENGTHUNIT[\"foot\",0.3048]],
    PARAMETER[\"False northing\",0,LENGTHUNIT[\"foot\",0.3048]]],
  CS[cartesian,2],
    AXIS[\"easting (X)\",east,ORDER[1]],
    AXIS[\"northing (Y)\",north,ORDER[2]],
    LENGTHUNIT[\"foot\",0.3048],
  ID[\"EPSG\",2222]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("projected")
        .clone();
    let Crs::Projected(projected) = &crs else {
        panic!("EPSG:2222 is a projected CRS");
    };
    assert_eq!(
        projected.export_to_proj_string().expect("proj"),
        "+proj=tmerc +lat_0=31 +lon_0=-110.166666666667 +k=0.9999 +x_0=213360 +y_0=0 +datum=NAD83 +units=ft +no_defs +type=crs"
    );
    let db = DatabaseContext::create().expect("proj.db");
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db))
            .expect("wkt1"),
        "PROJCS[\"NAD83 / Arizona East (ft)\",\n    GEOGCS[\"NAD83\",\n        DATUM[\"North_American_Datum_1983\",\n            SPHEROID[\"GRS 1980\",6378137,298.257222101]],\n        PRIMEM[\"Greenwich\",0,\n            AUTHORITY[\"EPSG\",\"8901\"]],\n        UNIT[\"degree\",0.0174532925199433,\n            AUTHORITY[\"EPSG\",\"9122\"]]],\n    PROJECTION[\"Transverse_Mercator\"],\n    PARAMETER[\"latitude_of_origin\",31],\n    PARAMETER[\"central_meridian\",-110.166666666667],\n    PARAMETER[\"scale_factor\",0.9999],\n    PARAMETER[\"false_easting\",700000],\n    PARAMETER[\"false_northing\",0],\n    UNIT[\"foot\",0.3048],\n    AXIS[\"Easting\",EAST],\n    AXIS[\"Northing\",NORTH],\n    AUTHORITY[\"EPSG\",\"2222\"]]"
    );
}

#[test]
fn crs_projected_with_parameter_unit_different_than_cs_unit_as_wkt1() {
    let wkt = "\
PROJCRS[\"unknown\",\
    BASEGEODCRS[\"unknown\",\
        DATUM[\"Unknown based on GRS80 ellipsoid\",\
            ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\
                LENGTHUNIT[\"metre\",1]]],\
        PRIMEM[\"Greenwich\",0]],\
    CONVERSION[\"UTM zone 32N\",\
        METHOD[\"Transverse Mercator\"],\
        PARAMETER[\"Latitude of natural origin\",0],\
        PARAMETER[\"Longitude of natural origin\",9],\
        PARAMETER[\"Scale factor at natural origin\",0.9996],\
        PARAMETER[\"False easting\",500000,LENGTHUNIT[\"metre\",1]],\
        PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1]]],\
    CS[Cartesian,2],\
        AXIS[\"(E)\",east],\
        AXIS[\"(N)\",north],\
    LENGTHUNIT[\"US survey foot\",0.304800609601219]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("projected")
        .clone();
    let db = DatabaseContext::create().expect("proj.db");
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db))
            .expect("wkt1"),
        "PROJCS[\"unknown\",\n    GEOGCS[\"unknown\",\n        DATUM[\"Unknown based on GRS80 ellipsoid\",\n            SPHEROID[\"GRS 1980\",6378137,298.257222101]],\n        PRIMEM[\"Greenwich\",0],\n        UNIT[\"degree\",0.0174532925199433,\n            AUTHORITY[\"EPSG\",\"9122\"]]],\n    PROJECTION[\"Transverse_Mercator\"],\n    PARAMETER[\"latitude_of_origin\",0],\n    PARAMETER[\"central_meridian\",9],\n    PARAMETER[\"scale_factor\",0.9996],\n    PARAMETER[\"false_easting\",1640416.66666667],\n    PARAMETER[\"false_northing\",0],\n    UNIT[\"US survey foot\",0.304800609601219],\n    AXIS[\"Easting\",EAST],\n    AXIS[\"Northing\",NORTH]]"
    );
}

fn operation_to_projected(source: &Crs, code: &str) -> (String, String) {
    let db = DatabaseContext::create().expect("proj.db");
    let target = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system(code)
        .unwrap_or_else(|error| panic!("{code}: {error}"));
    let factory = proj_rs::CoordinateOperationFactory::create();
    let operation = factory
        .create_operation_proj_string(source, &target)
        .unwrap_or_else(|error| panic!("{code} operation: {error}"));
    let normalized = factory
        .create_operation_proj_string(
            &source.normalize_for_visualization(),
            &target.normalize_for_visualization(),
        )
        .unwrap_or_else(|error| panic!("{code} normalized: {error}"));
    (operation, normalized)
}

#[test]
fn crs_epsg_32661_projected_north_pole_north_east() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326");
    let (operation, normalized) = operation_to_projected(&source, "32661");
    assert_eq!(
        operation,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84 +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        normalized,
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84"
    );
    let normalized_source = source.normalize_for_visualization();
    let Crs::Geographic(geographic) = &normalized_source else {
        panic!("normalized source stays geographic");
    };
    assert_eq!(geographic.domains().len(), 1);
    assert_eq!(
        normalized_source.remarks(),
        "Axis order reversed compared to EPSG:4326"
    );
}

#[test]
fn crs_epsg_5041_projected_north_pole_east_north() {
    let source = Crs::Geographic(GeographicCRS::epsg_4326());
    let (operation, normalized) = operation_to_projected(&source, "5041");
    let expected = "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84";
    assert_eq!(operation, expected);
    assert_eq!(
        normalized,
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84"
    );
}

#[test]
fn crs_epsg_32761_projected_south_pole_north_east() {
    let source = Crs::Geographic(GeographicCRS::epsg_4326());
    let (operation, normalized) = operation_to_projected(&source, "32761");
    assert_eq!(
        operation,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=-90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84 +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        normalized,
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=-90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84"
    );
}

#[test]
fn crs_epsg_5042_projected_south_pole_east_north() {
    let source = Crs::Geographic(GeographicCRS::epsg_4326());
    let (operation, normalized) = operation_to_projected(&source, "5042");
    let expected = "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=-90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84";
    assert_eq!(operation, expected);
    assert_eq!(
        normalized,
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=-90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +ellps=WGS84"
    );
}

#[test]
fn crs_epsg_5482_projected_south_pole_south_west() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4764")
        .expect("EPSG:4764");
    let (operation, normalized) = operation_to_projected(&source, "5482");
    assert_eq!(
        operation,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=-90 +lon_0=180 +k=0.994 +x_0=5000000 +y_0=1000000 +ellps=GRS80 +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        normalized,
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=-90 +lon_0=180 +k=0.994 +x_0=5000000 +y_0=1000000 +ellps=GRS80"
    );
}

#[test]
fn crs_spherical_mercator_on_sphere_to_wkt1() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(
            "\
PROJCRS[\"Moon (2015) - Sphere / Ocentric / Mercator\",
    BASEGEOGCRS[\"Moon (2015) - Sphere / Ocentric\",
        DATUM[\"Moon (2015) - Sphere\",
            ELLIPSOID[\"Moon (2015) - Sphere\",1737400,0,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Reference Meridian\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        ID[\"IAU\",30100,2015]],
    CONVERSION[\"Mercator\",
        METHOD[\"Mercator (Spherical)\",
            ID[\"EPSG\",1026]],
        PARAMETER[\"Latitude of natural origin\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433],
            ID[\"EPSG\",8801]],
        PARAMETER[\"Longitude of natural origin\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433],
            ID[\"EPSG\",8802]],
        PARAMETER[\"False easting\",0,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8806]],
        PARAMETER[\"False northing\",0,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8807]]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1]],
    ID[\"IAU\",30190,2015]]",
        )
        .expect("wkt")
        .crs()
        .expect("projected")
        .clone();
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db))
            .expect("wkt1"),
        "PROJCS[\"Moon (2015) - Sphere / Ocentric / Mercator\",\n    GEOGCS[\"Moon (2015) - Sphere / Ocentric\",\n        DATUM[\"Moon (2015) - Sphere\",\n            SPHEROID[\"Moon (2015) - Sphere\",1737400,0]],\n        PRIMEM[\"Reference Meridian\",0],\n        UNIT[\"degree\",0.0174532925199433,\n            AUTHORITY[\"EPSG\",\"9122\"]],\n        AUTHORITY[\"IAU\",\"30100\"]],\n    PROJECTION[\"Mercator_1SP\"],\n    PARAMETER[\"central_meridian\",0],\n    PARAMETER[\"false_easting\",0],\n    PARAMETER[\"false_northing\",0],\n    UNIT[\"metre\",1],\n    AXIS[\"Easting\",EAST],\n    AXIS[\"Northing\",NORTH],\n    AUTHORITY[\"IAU\",\"30190\"]]"
    );
}

#[test]
fn crs_mercator_variant_a_lat0_non_zero_to_wkt1() {
    let wkt = "\
PROJCRS[\"test\",
    BASEGEOGCRS[\"Moon (2015) - Sphere / Ocentric\",
        DATUM[\"Moon (2015) - Sphere\",
            ELLIPSOID[\"Moon (2015) - Sphere\",1737400,0,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Reference Meridian\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        ID[\"IAU\",30100,2015]],
    CONVERSION[\"Mercator\",
        METHOD[\"Mercator (Spherical)\",
            ID[\"EPSG\",1026]],
        PARAMETER[\"Latitude of natural origin\",10,
            ANGLEUNIT[\"degree\",0.0174532925199433],
            ID[\"EPSG\",8801]],
        PARAMETER[\"Longitude of natural origin\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433],
            ID[\"EPSG\",8802]],
        PARAMETER[\"False easting\",0,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8806]],
        PARAMETER[\"False northing\",0,
            LENGTHUNIT[\"metre\",1],
            ID[\"EPSG\",8807]]],
    CS[Cartesian,2],
        AXIS[\"(E)\",east,
            ORDER[1],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(N)\",north,
            ORDER[2],
            LENGTHUNIT[\"metre\",1]],
    ID[\"IAU\",30190,2015]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("projected")
        .clone();
    let db = DatabaseContext::create().expect("proj.db");
    let exported =
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db));
    assert!(
        exported.is_err(),
        "a non-zero latitude of origin is not WKT1 Mercator"
    );
}

#[test]
fn crs_projected_crs_deriving_conversion() {
    let crs = utm_31n();
    let operation = crs.deriving_operation();
    assert_eq!(operation.source_crs().name(), "WGS 84");
    assert!(std::ptr::eq(operation.target_crs(), &crs));
    let again = operation.target_crs().deriving_operation();
    assert!(!std::ptr::eq(again.conversion(), operation.conversion()));
}

#[test]
fn crs_projected_crs_shallow_clone() {
    let crs = utm_31n();
    assert!(crs.is_equivalent_to(&crs));
    assert!(
        !Crs::Projected(crs.clone()).is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326()))
    );
    let clone = crs.clone();
    assert!(clone.is_equivalent_to(&crs));
    assert!(std::ptr::eq(
        clone.deriving_operation().target_crs(),
        &clone
    ));

    let surviving = {
        let original = ProjectedCRS::create(
            "",
            GeodeticCRS::epsg_4978(),
            Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
            CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
        );
        let clone = original.clone();
        drop(original);
        clone
    };
    assert_eq!(
        surviving.base_crs().export_to_proj_string().expect("proj"),
        "+proj=geocent +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn crs_projected_crs_as_proj_string() {
    let crs = utm_31n();
    let source = Crs::Geographic(GeographicCRS::epsg_4326());
    let target = Crs::Projected(crs.clone());
    assert_eq!(
        proj_rs::CoordinateOperationFactory::create()
            .create_operation_proj_string(&source, &target)
            .expect("operation"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=utm +zone=31 +ellps=WGS84"
    );
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=31 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn crs_projected_crs_krovak_epsg_5221_as_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let projected = AuthorityFactory::create(&db, "EPSG")
        .create_projected_crs("5221")
        .expect("EPSG:5221");
    let source = Crs::Geodetic(projected.base_crs().clone());
    let target = Crs::Projected(projected);
    assert_eq!(
        proj_rs::CoordinateOperationFactory::create()
            .create_operation_proj_string(&source, &target)
            .expect("operation"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=bessel +pm=ferro +step +proj=krovak +lat_0=49.5 +lon_0=42.5 +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +pm=ferro"
    );
}

#[test]
fn crs_projected_crs_krovak_with_approximate_alpha_as_proj_string() {
    let crs = create_crs_from_proj_string(
        "+proj=krovak +lat_0=49.5 +lon_0=42.5 +alpha=30.28813972222222 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +pm=ferro +units=m +no_defs +type=crs",
    )
    .expect("crs");
    let Crs::Projected(projected) = &crs else {
        panic!("+proj=krovak is a projected CRS");
    };
    let source = Crs::Geodetic(projected.base_crs().clone());
    assert_eq!(
        proj_rs::CoordinateOperationFactory::create()
            .create_operation_proj_string(&source, &crs)
            .expect("operation"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=bessel +pm=ferro +step +proj=krovak +lat_0=49.5 +lon_0=42.5 +alpha=30.2881397222222 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +pm=ferro"
    );
}

#[test]
fn crs_projected_crs_identify_no_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let unnamed = utm_projected("", GeographicCRS::epsg_4326(), 1, UtmSide::North);
    let found = Crs::Projected(unnamed).identify();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].0.authority_code("EPSG").expect("code").code(),
        "32601"
    );
    assert_eq!(found[0].1, 70);
    let official = factory.create_projected_crs("32601").expect("32601");
    let Crs::Projected(found_crs) = &found[0].0 else {
        panic!("identified UTM is projected");
    };
    assert!(found_crs.is_equivalent_to_with(&official, Equivalence::Equivalent));

    let named = Crs::Projected(utm_projected(
        "WGS 84 / UTM zone 60S",
        GeographicCRS::epsg_4326(),
        60,
        UtmSide::South,
    ));
    let found = named.identify();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].0.authority_code("EPSG").expect("code").code(),
        "32760"
    );
    assert_eq!(found[0].1, 100);
    let official = factory.create_projected_crs("32760").expect("32760");
    let Crs::Projected(found_crs) = &found[0].0 else {
        panic!("identified UTM is projected");
    };
    assert!(found_crs.is_equivalent_to_with(&official, Equivalence::Equivalent));

    let nad27 = Crs::Projected(utm_projected(
        "NAD27_UTM_zone_11N",
        GeographicCRS::epsg_4267(),
        11,
        UtmSide::North,
    ));
    let found = nad27.identify();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].0.authority_code("EPSG").expect("code").code(),
        "26711"
    );
    assert_eq!(found[0].1, 90);
    let official = factory.create_projected_crs("26711").expect("26711");
    let Crs::Projected(found_crs) = &found[0].0 else {
        panic!("identified UTM is projected");
    };
    assert!(found_crs.is_equivalent_to_with(&official, Equivalence::Equivalent));

    let nad83 = Crs::Projected(utm_projected(
        "",
        GeographicCRS::epsg_4269(),
        11,
        UtmSide::North,
    ));
    let found = nad83.identify();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].0.authority_code("EPSG").expect("code").code(),
        "26911"
    );
    assert_eq!(found[0].1, 70);
    let official = factory.create_projected_crs("26911").expect("26911");
    let Crs::Projected(found_crs) = &found[0].0 else {
        panic!("identified UTM is projected");
    };
    assert!(found_crs.is_equivalent_to_with(&official, Equivalence::Equivalent));

    let parsed =
        create_crs_from_proj_string("+proj=utm +zone=31 +datum=WGS84 +type=crs").expect("utm");
    let Crs::Projected(parsed_projected) = &parsed else {
        panic!("+proj=utm is a projected CRS");
    };
    let found = parsed.identify();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].0.authority_code("EPSG").expect("code").code(),
        "32631"
    );
    assert_eq!(found[0].1, 70);
    let Crs::Projected(found_crs) = &found[0].0 else {
        panic!("identified UTM is projected");
    };
    assert!(found_crs.is_equivalent_to_with(parsed_projected, Equivalence::ExceptAxisOrder));
}

enum UtmSide {
    North,
    South,
}

fn utm_projected(name: &str, base: GeographicCRS, zone: i32, side: UtmSide) -> ProjectedCRS {
    let conversion = match side {
        UtmSide::North => Conversion::create_utm_north(&OpProps::new(), zone),
        UtmSide::South => Conversion::create_utm_south(&OpProps::new(), zone),
    }
    .expect("utm");
    ProjectedCRS::create(
        name,
        GeodeticCRS::Geographic(base),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
}

#[test]
fn crs_geographic3d_nad83_as_wkt1_esri_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(
            "GEOGCS[\"GCS_North_American_1983\",DATUM[\"D_North_American_1983\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],\
UNIT[\"Degree\",0.0174532925199433]],\
VERTCS[\"NAD_1983\",DATUM[\"D_North_American_1983\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PARAMETER[\"Vertical_Shift\",0.0],\
PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]",
        )
        .expect("wkt")
        .crs()
        .expect("geographic CRS")
        .clone();
    assert!(matches!(crs, Crs::Geographic(_)));
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("export");
    assert_eq!(
        wkt,
        "GEOGCS[\"GCS_NAD83\",DATUM[\"D_North_American_1983\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],\
UNIT[\"Degree\",0.0174532925199433],\
LINUNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_implicit_compound_esri_104024_plus_115844_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input("ESRI:104024+115844", &db).expect("compound");
    let Crs::Geographic(geographic) = &crs else {
        panic!("ESRI:104024+115844 is a geographic 3D CRS");
    };
    assert_eq!(geographic.coordinate_system().axes().len(), 3);
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Esri).with_database(db);
    formatter.set_linunit(LinunitPolicy::Reject);
    let wkt = crs.export_to_wkt(&mut formatter).expect("wkt");
    assert_eq!(
        wkt,
        "GEOGCS[\"California_SRS_Epoch_2017.50_(NAD83)\",\
DATUM[\"California_SRS_Epoch_2017.50_(NAD83)\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
VERTCS[\"California_SRS_Epoch_2017.50_(NAD83)\",\
DATUM[\"California_SRS_Epoch_2017.50_(NAD83)\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PARAMETER[\"Vertical_Shift\",0.0],\
PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_implicit_compound_esri_104971_to_3d_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "ESRI")
        .create_geographic_crs("104971")
        .expect("ESRI:104971")
        .promote_to_3d();
    assert_eq!(crs.coordinate_system().axes().len(), 3);
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Esri).with_database(db.clone());
    formatter.set_linunit(LinunitPolicy::Reject);
    let wkt = Crs::Geographic(crs)
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    let expected = "\
GEOGCS[\"Mars_2000_(Sphere)\",\
DATUM[\"Mars_2000_(Sphere)\",\
SPHEROID[\"Mars_2000_(Sphere)\",3396190.0,0.0]],\
PRIMEM[\"Reference_Meridian\",0.0],\
UNIT[\"Degree\",0.0174532925199433]],\
VERTCS[\"Mars_2000_(Sphere)\",\
DATUM[\"Mars_2000_(Sphere)\",\
SPHEROID[\"Mars_2000_(Sphere)\",3396190.0,0.0]],\
PARAMETER[\"Vertical_Shift\",0.0],\
PARAMETER[\"Direction\",1.0],\
UNIT[\"Meter\",1.0]]";
    assert_eq!(wkt, expected);
    let mut parser = WktParser::new();
    parser.attach_database(db);
    let again = parser
        .create_from_wkt(&wkt)
        .expect("parse")
        .crs()
        .expect("crs")
        .clone();
    let Crs::Geographic(again) = again else {
        panic!("parsed object is not a geographic CRS: {again:?}");
    };
    assert_eq!(again.coordinate_system().axes().len(), 3);
}

#[test]
fn crs_epsg_4979_as_wkt1_esri() {
    assert_eq!(
        export(&GeographicCRS::epsg_4979(), WktConvention::Wkt1Esri),
        "GEOGCS[\"WGS_1984_3D\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433],LINUNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_geographic_crs_is2d_part_of3d() {
    assert!(GeographicCRS::epsg_4326().is_2d_part_of_3d(&GeographicCRS::epsg_4979()));
    assert!(!GeographicCRS::epsg_4326().is_2d_part_of_3d(&GeographicCRS::epsg_4326()));
    assert!(!GeographicCRS::epsg_4979().is_2d_part_of_3d(&GeographicCRS::epsg_4326()));
    assert!(!GeographicCRS::epsg_4979().is_2d_part_of_3d(&GeographicCRS::epsg_4979()));
}

#[test]
fn crs_epsg_4807_as_proj_string() {
    assert_eq!(
        GeographicCRS::epsg_4807().export_to_proj_string(),
        "+proj=longlat +ellps=clrk80ign +pm=paris +no_defs +type=crs"
    );
}

#[test]
fn crs_epsg_4267_as_proj_string() {
    assert_eq!(
        GeographicCRS::epsg_4267().export_to_proj_string(),
        "+proj=longlat +datum=NAD27 +no_defs +type=crs"
    );
}

#[test]
fn crs_epsg_4269_as_proj_string() {
    assert_eq!(
        GeographicCRS::epsg_4269().export_to_proj_string(),
        "+proj=longlat +datum=NAD83 +no_defs +type=crs"
    );
}

#[test]
fn crs_epsg_4807_as_wkt2() {
    assert_eq!(
        export(&GeographicCRS::epsg_4807(), WktConvention::Wkt2_2015),
        "\
GEODCRS[\"NTF (Paris)\",
    DATUM[\"Nouvelle Triangulation Francaise (Paris)\",
        ELLIPSOID[\"Clarke 1880 (IGN)\",6378249.2,293.466021293627,
            LENGTHUNIT[\"metre\",1]]],
    PRIMEM[\"Paris\",2.5969213,
        ANGLEUNIT[\"grad\",0.015707963267949]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north,
            ORDER[1],
            ANGLEUNIT[\"grad\",0.015707963267949]],
        AXIS[\"longitude\",east,
            ORDER[2],
            ANGLEUNIT[\"grad\",0.015707963267949]],
    ID[\"EPSG\",4807]]"
    );
}

#[test]
fn crs_epsg_4807_as_wkt2_simplified() {
    assert_eq!(
        export(
            &GeographicCRS::epsg_4807(),
            WktConvention::Wkt2_2015Simplified
        ),
        "\
GEODCRS[\"NTF (Paris)\",
    DATUM[\"Nouvelle Triangulation Francaise (Paris)\",
        ELLIPSOID[\"Clarke 1880 (IGN)\",6378249.2,293.466021293627]],
    PRIMEM[\"Paris\",2.5969213],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north],
        AXIS[\"longitude\",east],
        UNIT[\"grad\",0.015707963267949],
    ID[\"EPSG\",4807]]"
    );
}

#[test]
fn crs_epsg_4807_as_wkt1_gdal() {
    assert_eq!(
        export(&GeographicCRS::epsg_4807(), WktConvention::Wkt1Gdal),
        "\
GEOGCS[\"NTF (Paris)\",
    DATUM[\"Nouvelle_Triangulation_Francaise_Paris\",
        SPHEROID[\"Clarke 1880 (IGN)\",6378249.2,293.466021293627,
            AUTHORITY[\"EPSG\",\"7011\"]],
        AUTHORITY[\"EPSG\",\"6807\"]],
    PRIMEM[\"Paris\",2.33722917,
        AUTHORITY[\"EPSG\",\"8903\"]],
    UNIT[\"grad\",0.015707963267949,
        AUTHORITY[\"EPSG\",\"9105\"]],
    AUTHORITY[\"EPSG\",\"4807\"]]"
    );
}

#[test]
fn crs_epsg_4807_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = export_db(
        &Crs::Geographic(GeographicCRS::epsg_4807()),
        WktConvention::Wkt1Esri,
        db,
    );
    assert_eq!(
        wkt,
        "GEOGCS[\"GCS_NTF_Paris\",DATUM[\"Nouvelle_Triangulation_Francaise_(Paris)\",SPHEROID[\"Clarke_1880_IGN\",6378249.2,293.466021293627]],PRIMEM[\"Paris\",2.33722917],UNIT[\"Grad\",0.015707963267949]]"
    );
}

#[test]
fn crs_epsg_4807_as_wkt1_esri_without_database() {
    assert_eq!(
        export(&GeographicCRS::epsg_4807(), WktConvention::Wkt1Esri),
        "GEOGCS[\"GCS_NTF_Paris\",DATUM[\"D_Nouvelle_Triangulation_Francaise_Paris\",SPHEROID[\"Clarke_1880_IGN\",6378249.2,293.466021293627]],PRIMEM[\"Paris\",2.33722917],UNIT[\"Grad\",0.015707963267949]]"
    );
}

#[test]
fn crs_epsg_4267() {
    assert_eq!(
        export(&GeographicCRS::epsg_4267(), WktConvention::Wkt2_2015),
        "\
GEODCRS[\"NAD27\",
    DATUM[\"North American Datum 1927\",
        ELLIPSOID[\"Clarke 1866\",6378206.4,294.978698213898,
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
    ID[\"EPSG\",4267]]"
    );
    assert_eq!(
        GeographicCRS::epsg_4267().export_to_proj_string(),
        "+proj=longlat +datum=NAD27 +no_defs +type=crs"
    );
}

#[test]
fn crs_epsg_4267_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = export_db(
        &Crs::Geographic(GeographicCRS::epsg_4267()),
        WktConvention::Wkt1Esri,
        db,
    );
    assert_eq!(
        wkt,
        "GEOGCS[\"GCS_North_American_1927\",DATUM[\"D_North_American_1927\",SPHEROID[\"Clarke_1866\",6378206.4,294.978698213898]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]"
    );
}

#[test]
fn crs_epsg_4269() {
    assert_eq!(
        export(&GeographicCRS::epsg_4269(), WktConvention::Wkt2_2015),
        "\
GEODCRS[\"NAD83\",
    DATUM[\"North American Datum 1983\",
        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,
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
    ID[\"EPSG\",4269]]"
    );
    assert_eq!(
        GeographicCRS::epsg_4269().export_to_proj_string(),
        "+proj=longlat +datum=NAD83 +no_defs +type=crs"
    );
}

#[test]
fn crs_epsg_4326_from_db_as_wkt1_gdal_with_axis() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = proj_rs::AuthorityFactory::create(&db, "EPSG");
    let crs = factory
        .create_coordinate_reference_system("4326")
        .expect("4326");
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    formatter.set_output_axis(OutputAxisRule::Yes);
    let wkt = crs.export_to_wkt(&mut formatter).expect("wkt");
    assert_eq!(
        wkt,
        "\
GEOGCS[\"WGS 84\",
    DATUM[\"WGS_1984\",
        SPHEROID[\"WGS 84\",6378137,298.257223563,
            AUTHORITY[\"EPSG\",\"7030\"]],
        AUTHORITY[\"EPSG\",\"6326\"]],
    PRIMEM[\"Greenwich\",0,
        AUTHORITY[\"EPSG\",\"8901\"]],
    UNIT[\"degree\",0.0174532925199433,
        AUTHORITY[\"EPSG\",\"9122\"]],
    AXIS[\"Latitude\",NORTH],
    AXIS[\"Longitude\",EAST],
    AUTHORITY[\"EPSG\",\"4326\"]]"
    );
}

#[test]
fn crs_epsg_4268_geogcrs_deprecated_as_wkt1_gdal() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = proj_rs::AuthorityFactory::create(&db, "EPSG");
    let crs = factory
        .create_coordinate_reference_system("4268")
        .expect("4268");
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt");
    assert!(
        wkt.starts_with("GEOGCS[\"NAD27 Michigan (deprecated)\""),
        "{wkt}"
    );
}

#[test]
fn crs_esri_104971_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = proj_rs::AuthorityFactory::create(&db, "ESRI");
    let crs = factory
        .create_coordinate_reference_system("104971")
        .expect("104971");
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = export_db(&crs, WktConvention::Wkt1Esri, db);
    assert_eq!(
        wkt,
        "GEOGCS[\"Mars_2000_(Sphere)\",DATUM[\"Mars_2000_(Sphere)\",SPHEROID[\"Mars_2000_(Sphere)\",3396190.0,0.0]],PRIMEM[\"Reference_Meridian\",0.0],UNIT[\"Degree\",0.0174532925199433]]"
    );
}

#[test]
fn crs_itrf2020_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = proj_rs::AuthorityFactory::create(&db, "EPSG");
    let crs = factory
        .create_coordinate_reference_system("9990")
        .expect("9990");
    let wkt = export_db(&crs, WktConvention::Wkt1Esri, db);
    assert_eq!(
        wkt,
        "GEOGCS[\"ITRF2020\",DATUM[\"International_Terrestrial_Reference_Frame_2020\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]"
    );
}

#[test]
fn crs_epsg_2008_projcrs_deprecated_as_wkt1_gdal() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = proj_rs::AuthorityFactory::create(&db, "EPSG");
    let crs = factory
        .create_coordinate_reference_system("2008")
        .expect("2008");
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt");
    assert!(
        wkt.starts_with("PROJCS[\"NAD27(CGQ77) / SCoPQ zone 2 (deprecated)\""),
        "{wkt}"
    );
}

fn identified_codes(res: &[(Crs, i32)]) -> String {
    res.iter()
        .map(|(crs, confidence)| {
            let id = crs.identifiers().first().map_or_else(
                || crs.name().to_owned(),
                |identifier| format!("{}:{}", identifier.code_space(), identifier.code()),
            );
            format!("{id}/{confidence}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn epsg_of(crs: &Crs) -> i32 {
    crs.identifiers()
        .iter()
        .find(|identifier| identifier.code_space() == "EPSG")
        .and_then(|identifier| identifier.code().parse().ok())
        .unwrap_or(0)
}

fn expect_one(label: &str, res: &[(Crs, i32)], code: i32, confidence: i32) {
    assert_eq!(res.len(), 1, "{label}: {}", identified_codes(res));
    assert_eq!(epsg_of(&res[0].0), code, "{label}");
    assert_eq!(res[0].1, confidence, "{label}");
}

fn expect_contains(label: &str, res: &[(Crs, i32)], code: &str, confidence: i32) {
    let found = res.iter().any(|(crs, found_confidence)| {
        crs.identifiers()
            .first()
            .is_some_and(|identifier| identifier.code() == code && *found_confidence == confidence)
    });
    assert!(found, "{label}: {}", identified_codes(res));
}

fn projected_wkt(db: &DatabaseContext, wkt: &str) -> ProjectedCRS {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let crs = parser
        .create_from_wkt(wkt)
        .unwrap_or_else(|error| panic!("{error}"))
        .crs()
        .cloned()
        .unwrap_or_else(|| panic!("wkt is not a CRS"));
    let Crs::Projected(crs) = crs else {
        panic!("wkt is not projected: {}", crs.name());
    };
    crs
}

fn projected_proj(definition: &str) -> ProjectedCRS {
    let crs = create_crs_from_proj_string(definition)
        .unwrap_or_else(|error| panic!("{definition}: {error}"));
    let Crs::Projected(crs) = crs else {
        panic!("{definition} is not projected");
    };
    crs
}

fn retitled(source: &ProjectedCRS, name: &str, code: Option<&str>) -> ProjectedCRS {
    let mut crs = ProjectedCRS::create(
        name,
        source.base_crs().clone(),
        source.deriving_conversion().clone(),
        source.coordinate_system().clone(),
    );
    if let Some(code) = code {
        crs = crs.with_identifier("EPSG", code);
    }
    crs
}

#[test]
fn crs_projected_crs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let ignf = AuthorityFactory::create(&db, "IGNF");
    let all = AuthorityFactory::create(&db, "");

    let crs = epsg.create_projected_crs("2172").expect("2172");
    expect_one("code 2172", &crs.identify_with(Some(&epsg)), 2172, 100);
    assert_eq!(crs.identify_with(Some(&all)).len(), 1, "anonymous");
    assert_eq!(crs.identify_with(Some(&ignf)).len(), 0, "IGNF");

    let ignf_crs = ignf
        .create_projected_crs("ETRS89UTM28")
        .expect("ETRS89UTM28");
    expect_one(
        "IGNF ETRS89UTM28",
        &ignf_crs.identify_with(Some(&epsg)),
        25828,
        70,
    );

    let source = epsg.create_projected_crs("2172").expect("2172");
    let renamed = retitled(&source, "Pulkovo 1942(58) / Poland zone II", Some("1"));
    expect_one(
        "missing code",
        &renamed.identify_with(Some(&epsg)),
        2172,
        70,
    );
    let wrong_code = retitled(&source, "Pulkovo 1942(58) / Poland zone II", Some("32631"));
    expect_one(
        "wrong code",
        &wrong_code.identify_with(Some(&epsg)),
        2172,
        70,
    );
    let exact = retitled(&source, "Pulkovo 1942(58) / Poland zone II", None);
    expect_one("exact name", &exact.identify_with(Some(&epsg)), 2172, 100);
    let equivalent = retitled(&source, "Pulkovo_1942_58_Poland_zone_II", None);
    expect_one(
        "equivalent name",
        &equivalent.identify_with(Some(&epsg)),
        2172,
        90,
    );
    let fake = retitled(&source, "i am a faked name", None);
    expect_one("properties", &fake.identify_with(Some(&epsg)), 2172, 70);
    let other = epsg.create_projected_crs("3375").expect("3375");
    let mismatched = retitled(&other, "Pulkovo 1942(58) / Poland zone II", None);
    expect_one(
        "name only",
        &mismatched.identify_with(Some(&epsg)),
        2172,
        25,
    );

    let pipeline = projected_proj(
        "+type=crs +proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=omerc \
         +no_uoff +lat_0=4 +lonc=102.25 +alpha=323.025796466667 \
         +gamma=323.130102361111 +k=0.99984 +x_0=804671 +y_0=0 +ellps=GRS80",
    );
    expect_one(
        "omerc pipeline",
        &pipeline.identify_with(Some(&epsg)),
        3375,
        70,
    );
    let omerc = projected_proj(
        "+proj=omerc +no_uoff +lat_0=4 +lonc=102.25 \
         +alpha=323.025796466667 +gamma=323.130102361111 +k=0.99984 \
         +x_0=804671 +y_0=0 +ellps=GRS80 +type=crs",
    );
    expect_one("omerc", &omerc.identify_with(Some(&epsg)), 3375, 70);

    let northing = projected_wkt(
        &db,
        "PROJCS[\"ETRS89 / UTM zone 32N (N-E)\",GEOGCS[\"ETRS89\",\
         DATUM[\"European_Terrestrial_Reference_System_1989\",\
         SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
         PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
         PROJECTION[\"Transverse_Mercator\"],\
         PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",9],\
         PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],\
         PARAMETER[\"false_northing\",0],UNIT[\"metre\",1],\
         AXIS[\"Northing\",NORTH],AXIS[\"Easting\",EAST]]",
    );
    expect_one(
        "northing easting",
        &northing.identify_with(Some(&epsg)),
        3044,
        100,
    );
    let easting = projected_wkt(
        &db,
        "PROJCS[\"ETRS89 / UTM zone 32N (N-E)\",GEOGCS[\"ETRS89\",\
         DATUM[\"European_Terrestrial_Reference_System_1989\",\
         SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
         PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
         PROJECTION[\"Transverse_Mercator\"],\
         PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",9],\
         PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],\
         PARAMETER[\"false_northing\",0],UNIT[\"metre\",1],\
         AXIS[\"Easting\",EAST], AXIS[\"Northing\",NORTH]]",
    );
    expect_one("wrong axis", &easting.identify_with(Some(&epsg)), 3044, 50);
    let no_axis = projected_wkt(
        &db,
        "PROJCS[\"ETRS89 / UTM zone 32N (N-E)\",GEOGCS[\"ETRS89\",\
         DATUM[\"European_Terrestrial_Reference_System_1989\",\
         SPHEROID[\"GRS 1980\",6378137,298.257222101]],\
         PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
         PROJECTION[\"Transverse_Mercator\"],\
         PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",9],\
         PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],\
         PARAMETER[\"false_northing\",0],UNIT[\"metre\",1]]",
    );
    expect_one(
        "implicit axis",
        &no_axis.identify_with(Some(&epsg)),
        3044,
        100,
    );

    let lcc = projected_wkt(
        &db,
        "PROJCS[\"Lambert Conformal Conic\",GEOGCS[\"grs80\",\
         DATUM[\"D_North_American_1983\",\
         SPHEROID[\"Geodetic_Reference_System_1980\",6378137,298.257222101]],\
         PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],\
         PROJECTION[\"Lambert_Conformal_Conic\"],\
         PARAMETER[\"standard_parallel_1\",34.33333333333334],\
         PARAMETER[\"standard_parallel_2\",36.16666666666666],\
         PARAMETER[\"latitude_of_origin\",33.75],\
         PARAMETER[\"central_meridian\",-79],\
         PARAMETER[\"false_easting\",609601.22],\
         PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]",
    );
    expect_one("ESRI lcc", &lcc.identify_with(Some(&epsg)), 32119, 70);

    let generic = projected_proj("+proj=tmerc +datum=WGS84 +type=crs");
    assert_eq!(generic.identify_with(Some(&epsg)).len(), 0, "generic tmerc");

    let feet = projected_proj(
        "+proj=lcc +lat_0=43.5 +lon_0=-93.95 +lat_1=43.5666666666667 \
         +lat_2=43.8 +x_0=152400.30480061 +y_0=30480.0609601219 \
         +a=6378521.049 +rf=298.257222100883 +units=us-ft +type=crs",
    );
    expect_contains("ESRI 103729", &feet.identify_with(Some(&all)), "103729", 70);

    let xian = projected_proj(
        "+proj=tmerc +lat_0=0 +lon_0=75 +k=1 +x_0=13500000 +y_0=0 \
         +a=6378140 +b=6356755.288157528 +units=m +type=crs",
    );
    expect_one("EPSG 2327", &xian.identify_with(Some(&epsg)), 2327, 70);
    let clarke = projected_proj(
        "+proj=tmerc +lat_0=29.02626833333333 +lon_0=46.5 \
         +k=0.9994 +x_0=800000 +y_0=0 +ellps=clrk80 +units=m +type=crs",
    );
    expect_one("EPSG 6646", &clarke.identify_with(Some(&epsg)), 6646, 70);

    let albers = projected_wkt(
        &db,
        "PROJCS[\"USA_Contiguous_Albers_Equal_Area_Conic_USGS_version\",\
         GEOGCS[\"GCS_North_American_1983\",DATUM[\"D_North_American_1983\",\
         SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],\
         UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Albers\"],\
         PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],\
         PARAMETER[\"Central_Meridian\",-96.0],PARAMETER[\"Standard_Parallel_1\",29.5],\
         PARAMETER[\"Standard_Parallel_2\",45.5],PARAMETER[\"Latitude_Of_Origin\",23.0],\
         UNIT[\"Foot_US\",0.3048006096012192]]",
    );
    expect_contains(
        "ESRI 102039",
        &albers.identify_with(Some(&all)),
        "102039",
        50,
    );

    let nz = projected_wkt(
        &db,
        "PROJCS[\"NZGD2000_New_Zealand_Transverse_Mercator_2000\",\
         GEOGCS[\"GCS_NZGD2000\",DATUM[\"New_Zealand_Geodetic_Datum_2000\",\
         SPHEROID[\"GRS 1980\",6378137,298.2572221010042,AUTHORITY[\"EPSG\",\"7019\"]],\
         AUTHORITY[\"EPSG\",\"6167\"]],PRIMEM[\"Greenwich\",0],\
         UNIT[\"degree\",0.0174532925199433]],PROJECTION[\"Transverse_Mercator\"],\
         PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",173],\
         PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",1600000],\
         PARAMETER[\"false_northing\",10000000],UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]]]",
    );
    expect_one("NZGD2000", &nz.identify_with(Some(&all)), 2193, 90);

    let colorado = projected_wkt(
        &db,
        "PROJCS[\"NAD_1983_HARN_StatePlane_Colorado_North_FIPS_0501\",\
         GEOGCS[\"GCS_North_American_1983_HARN\",DATUM[\"D_North_American_1983_HARN\",\
         SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],\
         UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Lambert_Conformal_Conic\"],\
         PARAMETER[\"False_Easting\",3000000.000316083],\
         PARAMETER[\"False_Northing\",999999.999996],\
         PARAMETER[\"Central_Meridian\",-105.5],\
         PARAMETER[\"Standard_Parallel_1\",39.71666666666667],\
         PARAMETER[\"Standard_Parallel_2\",40.78333333333333],\
         PARAMETER[\"Latitude_Of_Origin\",39.33333333333334],\
         UNIT[\"Foot_US\",0.3048006096012192]]",
    );
    expect_one(
        "colorado feet",
        &colorado.identify_with(Some(&all)),
        2876,
        100,
    );

    let japan = projected_wkt(
        &db,
        "PROJCS[\"JGD2011_Japan_Zone_2\",GEOGCS[\"GCS_JGD_2011\",\
         DATUM[\"D_JGD_2011\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"False_Easting\",0.0],\
         PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",131],\
         PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Latitude_Of_Origin\",33],\
         UNIT[\"Meter\",1.0]]",
    );
    expect_one("JGD2011", &japan.identify_with(Some(&epsg)), 6670, 90);

    let pei = projected_wkt(
        &db,
        "PROJCS[\"NAD_1983_CSRS_Prince_Edward_Island\",\
         GEOGCS[\"GCS_North_American_1983_CSRS\",\
         DATUM[\"D_North_American_1983_CSRS\",\
         SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Double_Stereographic\"],PARAMETER[\"False_Easting\",400000.0],\
         PARAMETER[\"False_Northing\",800000.0],PARAMETER[\"Central_Meridian\",-63.0],\
         PARAMETER[\"Scale_Factor\",0.999912],PARAMETER[\"Latitude_Of_Origin\",47.25],\
         UNIT[\"Meter\",1.0]]",
    );
    expect_one("PEI", &pei.identify_with(Some(&all)), 2954, 100);

    let swapped = projected_wkt(
        &db,
        "PROJCS[\"foo\",GEOGCS[\"RGF93\",DATUM[\"Reseau_Geodesique_Francais_1993\",\
         SPHEROID[\"GRS 1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],\
         UNIT[\"degree\",0.0174532925199433]],\
         PROJECTION[\"Lambert_Conformal_Conic_2SP\"],\
         PARAMETER[\"latitude_of_origin\",46.5],PARAMETER[\"central_meridian\",3],\
         PARAMETER[\"standard_parallel_1\",44],PARAMETER[\"standard_parallel_2\",49],\
         PARAMETER[\"false_easting\",700000],PARAMETER[\"false_northing\",6600000],\
         UNIT[\"metre\",1]]",
    );
    expect_one(
        "swapped parallels",
        &swapped.identify_with(Some(&epsg)),
        2154,
        70,
    );

    let latvia = projected_wkt(
        &db,
        "PROJCS[\"LKS92_Latvia_TM\",GEOGCS[\"GCS_LKS92\",DATUM[\"D_Latvia_1992\",\
         SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],\
         UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],\
         PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",24],\
         PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],\
         PARAMETER[\"false_northing\",-6000000],UNIT[\"Meter\",1]]",
    );
    expect_one("Latvia", &latvia.identify_with(Some(&epsg)), 3059, 90);

    let wrong_datum = projected_wkt(
        &db,
        "PROJCS[\"WGS_1984_UTM_Zone_31N\",GEOGCS[\"GCS_WGS_1984\",\
         DATUM[\"wrong_datum_name\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"False_Easting\",500000.0],\
         PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",3.0],\
         PARAMETER[\"Scale_Factor\",0.9996],PARAMETER[\"Latitude_Of_Origin\",0.0],\
         UNIT[\"Meter\",1.0]]",
    );
    expect_one(
        "wrong datum",
        &wrong_datum.identify_with(Some(&epsg)),
        32631,
        60,
    );

    let laea = projected_wkt(
        &db,
        "PROJCS[\"ETRS89_LAEA_Europe\",GEOGCS[\"GCS_ETRS_1989\",\
         DATUM[\"D_ETRS_1989\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Lambert_Azimuthal_Equal_Area\"],\
         PARAMETER[\"false_easting\",4321000.0],PARAMETER[\"false_northing\",3210000.0],\
         PARAMETER[\"central_meridian\",10.0],PARAMETER[\"latitude_of_origin\",52.0],\
         UNIT[\"Meter\",1.0]]",
    );
    expect_one("LAEA", &laea.identify_with(Some(&all)), 3035, 90);

    let lambert93 = projected_wkt(
        &db,
        "PROJCS[\"RGF93_Lambert_93\",GEOGCS[\"GCS_RGF_1993\",\
         DATUM[\"D_RGF_1993\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",700000.0],\
         PARAMETER[\"False_Northing\",6600000.0],PARAMETER[\"Central_Meridian\",3.0],\
         PARAMETER[\"Standard_Parallel_1\",44.0],PARAMETER[\"Standard_Parallel_2\",49.0],\
         PARAMETER[\"Latitude_Of_Origin\",46.5],UNIT[\"Meter\",1.0]]",
    );
    expect_contains(
        "Lambert 93",
        &lambert93.identify_with(Some(&all)),
        "2154",
        90,
    );

    let ensemble = "PROJCRS[\"WGS 84 / UTM zone 31N\",BASEGEOGCRS[\"WGS 84\",\
         ENSEMBLE[\"World Geodetic System 1984 ensemble\",\
         MEMBER[\"World Geodetic System 1984 (Transit)\",ID[\"EPSG\",1166]],\
         MEMBER[\"World Geodetic System 1984 (G730)\",ID[\"EPSG\",1152]],\
         MEMBER[\"World Geodetic System 1984 (G873)\",ID[\"EPSG\",1153]],\
         MEMBER[\"World Geodetic System 1984 (G1150)\",ID[\"EPSG\",1154]],\
         MEMBER[\"World Geodetic System 1984 (G1674)\",ID[\"EPSG\",1155]],\
         MEMBER[\"World Geodetic System 1984 (G1762)\",ID[\"EPSG\",1156]],\
         ELLIPSOID[\"WGS 84\",6378137,298.257223563,LENGTHUNIT[\"metre\",1]],\
         ENSEMBLEACCURACY[2]]],CONVERSION[\"UTM zone 31N\",\
         METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],\
         PARAMETER[\"Latitude of natural origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],\
         PARAMETER[\"Longitude of natural origin\",3,ANGLEUNIT[\"degree\",0.0174532925199433]],\
         PARAMETER[\"Scale factor at natural origin\",0.9996,SCALEUNIT[\"unity\",1]],\
         PARAMETER[\"False easting\",500000,LENGTHUNIT[\"metre\",1]],\
         PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1]]],\
         CS[Cartesian,2],AXIS[\"Easting (E)\",east,ORDER[1]],\
         AXIS[\"Northing (N)\",north,ORDER[2]],LENGTHUNIT[\"metre\",1]]";
    let ensemble_crs = WktParser::new()
        .create_from_wkt(ensemble)
        .expect("ensemble")
        .crs()
        .cloned()
        .expect("crs");
    let Crs::Projected(ensemble_crs) = ensemble_crs else {
        panic!("ensemble WKT");
    };
    expect_one(
        "ensemble",
        &ensemble_crs.identify_with(Some(&all)),
        32631,
        100,
    );
    let unknown_ensemble = ensemble
        .replace("PROJCRS[\"WGS 84 / UTM zone 31N\"", "PROJCRS[\"unknown\"")
        .replace("BASEGEOGCRS[\"WGS 84\"", "BASEGEOGCRS[\"unknown\"");
    let unknown_crs = WktParser::new()
        .create_from_wkt(&unknown_ensemble)
        .expect("unknown ensemble")
        .crs()
        .cloned()
        .expect("crs");
    let Crs::Projected(unknown_crs) = unknown_crs else {
        panic!("unknown ensemble WKT");
    };
    let unknown_res = unknown_crs.identify_with(Some(&epsg));
    assert_eq!(
        unknown_res.len(),
        1,
        "unknown ensemble: {}",
        identified_codes(&unknown_res)
    );
    assert_eq!(epsg_of(&unknown_res[0].0), 32631);
    assert!(unknown_res[0].1 >= 70, "{}", unknown_res[0].1);

    let krovak = "PROJCS[\"S-JTSK_[JTSK03]_Krovak_East_North\",GEOGCS[\"S-JTSK_[JTSK03]\",\
         DATUM[\"S-JTSK_[JTSK03]\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],\
         PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],\
         PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.2881397527778],\
         PARAMETER[\"Longitude_Of_Center\",24.8333333333333],\
         PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",-1.0],\
         PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",90.0],UNIT[\"Meter\",1.0]]";
    assert_eq!(WktParser::guess_dialect(krovak), WktDialect::Wkt1Esri);
    let krovak = projected_wkt(&db, krovak);
    expect_one(
        "Krovak East North",
        &krovak.identify_with(Some(&all)),
        8353,
        100,
    );

    let pseudo = projected_wkt(
        &db,
        "PROJCS[\"ETRS_1989_UTM_Zone_32N_6Stellen\",GEOGCS[\"GCS_ETRS_1989\",\
         DATUM[\"D_ETRS_1989\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"False_Easting\",500000.0],\
         PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",9.0],\
         PARAMETER[\"Scale_Factor\",0.9996],PARAMETER[\"Latitude_Of_Origin\",0.0],\
         UNIT[\"Meter\",1.0],AUTHORITY[\"ESRI\",\"102328\"]]",
    );
    let pseudo_res = pseudo.identify_with(Some(&all));
    assert!(!pseudo_res.is_empty(), "pseudo ESRI");
    assert_eq!(
        epsg_of(&pseudo_res[0].0),
        25832,
        "{}",
        identified_codes(&pseudo_res)
    );
    assert_eq!(pseudo_res[0].1, 70);

    let mercury = projected_wkt(
        &db,
        "PROJCRS[\"unknown\",BASEGEODCRS[\"Mercury (2015) / Ocentric\",\
         DATUM[\"Mercury (2015)\",ELLIPSOID[\"Mercury (2015)\",2440530,1075.12334801762,\
         LENGTHUNIT[\"metre\",1]],ANCHOR[\"Hun Kal: 20 W.0\"]],\
         PRIMEM[\"Reference Meridian\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],\
         ID[\"IAU\",19902,2015]],CONVERSION[\"Equirectangular, clon = 0\",\
         METHOD[\"Equidistant Cylindrical\",ID[\"EPSG\",1028]],\
         PARAMETER[\"Latitude of 1st standard parallel\",0,\
         ANGLEUNIT[\"degree\",0.0174532925199433],ID[\"EPSG\",8823]],\
         PARAMETER[\"Longitude of natural origin\",0,\
         ANGLEUNIT[\"degree\",0.0174532925199433],ID[\"EPSG\",8802]],\
         PARAMETER[\"False easting\",0,LENGTHUNIT[\"metre\",1],ID[\"EPSG\",8806]],\
         PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1],ID[\"EPSG\",8807]]],\
         CS[Cartesian,2],AXIS[\"(E)\",east,ORDER[1],LENGTHUNIT[\"metre\",1]],\
         AXIS[\"(N)\",north,ORDER[2],LENGTHUNIT[\"metre\",1]]]",
    );
    let mercury_res = mercury.identify_with(Some(&all));
    assert!(!mercury_res.is_empty(), "mercury");
    let mercury_id = mercury_res[0].0.identifiers().first().expect("id");
    assert_eq!(mercury_id.code(), "19912");
    assert_eq!(mercury_id.code_space(), "IAU_2015");
    assert_eq!(mercury_res[0].1, 90);

    let korea = projected_wkt(
        &db,
        "PROJCS[\"Korea_2000_Korea_Central_Belt_2010\",GEOGCS[\"GCS_Korea_2000\",\
         DATUM[\"D_Korea_2000\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
         PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
         PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"False_Easting\",200000.0],\
         PARAMETER[\"False_Northing\",600000.0],PARAMETER[\"Central_Meridian\",127.0],\
         PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Latitude_Of_Origin\",38.0],\
         UNIT[\"Meter\",1.0]]",
    );
    let korea_res = korea.identify_with(Some(&all));
    assert!(!korea_res.is_empty(), "korea");
    let korea_id = korea_res[0].0.identifiers().first().expect("id");
    assert_eq!(korea_id.code(), "5186", "{}", identified_codes(&korea_res));
    assert_eq!(korea_id.code_space(), "EPSG");
    assert_eq!(korea_res[0].1, 100);

    let arizona = projected_wkt(
        &db,
        "PROJCS[\"NAD_1983_StatePlane_Arizona_Central_FIPS_0202_IntlFeet\",\
         GEOGCS[\"GCS_North_American_1983\",DATUM[\"D_North_American_1983\",\
         SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],\
         UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Transverse_Mercator\"],\
         PARAMETER[\"False_Easting\",700000.0],PARAMETER[\"False_Northing\",0.0],\
         PARAMETER[\"Central_Meridian\",-111.9166666666667],\
         PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Latitude_Of_Origin\",31.0],\
         UNIT[\"Foot\",0.3048]]",
    );
    let arizona_res = arizona.identify_with(Some(&all));
    assert!(!arizona_res.is_empty(), "arizona");
    let arizona_id = arizona_res[0].0.identifiers().first().expect("id");
    assert_eq!(
        arizona_id.code(),
        "2223",
        "{}",
        identified_codes(&arizona_res)
    );
    assert_eq!(arizona_id.code_space(), "EPSG");
    assert_eq!(arizona_res[0].1, 70);

    let portugal = projected_wkt(
        &db,
        "PROJCS[\"ETRS89 / Portugal TM06\",GEOGCS[\"ETRS89\",\
         DATUM[\"European Terrestrial Reference System 1989\",\
         SPHEROID[\"GRS 1980\",6378137.0,298.257222101,AUTHORITY[\"EPSG\",\"7019\"]],\
         AUTHORITY[\"EPSG\",\"6258\"]],PRIMEM[\"Greenwich\",0.0,AUTHORITY[\"EPSG\",\"8901\"]],\
         UNIT[\"degree\",0.017453292519943295],AXIS[\"Geodetic latitude\",NORTH],\
         AXIS[\"Geodetic longitude\",EAST],AUTHORITY[\"EPSG\",\"4258\"]],\
         PROJECTION[\"Transverse_Mercator\",AUTHORITY[\"EPSG\",\"9807\"]],\
         PARAMETER[\"central_meridian\",-8.133108333333334],\
         PARAMETER[\"latitude_of_origin\",39.66825833333334],\
         PARAMETER[\"scale_factor\",1.0],PARAMETER[\"false_easting\",0.0],\
         PARAMETER[\"false_northing\",0.0],UNIT[\"m\",1.0],\
         AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH],AUTHORITY[\"EPSG\",\"3763\"]]",
    );
    let portugal_res = portugal.identify_with(Some(&epsg));
    assert!(!portugal_res.is_empty(), "portugal");
    let portugal_id = portugal_res[0].0.identifiers().first().expect("id");
    assert_eq!(
        portugal_id.code(),
        "3763",
        "{}",
        identified_codes(&portugal_res)
    );
    assert_eq!(portugal_id.code_space(), "EPSG");
    assert_eq!(portugal_res[0].1, 100);

    let bessel = projected_wkt(
        &db,
        "PROJCRS[\"Transverse Mercator\",BASEGEOGCRS[\"DE_DHDN (whole country, 2001) to ETRS89\",\
         DATUM[\"DE_DHDN (whole country, 2001) to ETRS89\",\
         ELLIPSOID[\"Bessel\",6377397.155,299.15281310608,LENGTHUNIT[\"metre\",1]]],\
         PRIMEM[\"Greenwich\",0,ANGLEUNIT[\"degree\",0.0174532925199433]]],\
         CONVERSION[\"Transverse Mercator\",METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],\
         PARAMETER[\"Latitude of natural origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433],ID[\"EPSG\",8801]],\
         PARAMETER[\"Longitude of natural origin\",12,ANGLEUNIT[\"degree\",0.0174532925199433],ID[\"EPSG\",8802]],\
         PARAMETER[\"Scale factor at natural origin\",1,SCALEUNIT[\"unity\",1],ID[\"EPSG\",8805]],\
         PARAMETER[\"False easting\",4500000,LENGTHUNIT[\"metre\",1],ID[\"EPSG\",8806]],\
         PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1],ID[\"EPSG\",8807]]],\
         CS[Cartesian,2],AXIS[\"easting\",east,ORDER[1],LENGTHUNIT[\"meters\",1]],\
         AXIS[\"northing\",north,ORDER[2],LENGTHUNIT[\"meters\",1]]]",
    );
    let bessel_res = bessel.identify_with(Some(&all));
    assert_eq!(bessel_res.len(), 13, "{}", identified_codes(&bessel_res));
}

#[test]
fn crs_projected_crs_identify_wrong_auth_name_case() {
    let db = DatabaseContext::create().expect("proj.db");
    let all = AuthorityFactory::create(&db, "");
    let mut parser = WktParser::new();
    parser.attach_database(db);
    parser.set_strictness(WktStrictness::Lenient);
    let crs = parser
        .create_from_wkt(
            "PROJCS[\"World_Cylindrical_Equal_Area\",\
             GEOGCS[\"GCS_WGS_1984\",\
             DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
             PRIMEM[\"Greenwich\",0.0],\
             UNIT[\"Degree\",0.0174532925199433]],\
             PROJECTION[\"Cylindrical_Equal_Area\"],\
             PARAMETER[\"False_Easting\",0.0],\
             PARAMETER[\"False_Northing\",0.0],\
             PARAMETER[\"Central_Meridian\",0.0],\
             PARAMETER[\"Standard_Parallel_1\",0.0],UNIT[\"Meter\",1.0],\
             AUTHORITY[\"Esri\",54034]]",
        )
        .expect("wkt")
        .crs()
        .cloned()
        .expect("crs");
    let Crs::Projected(crs) = crs else {
        panic!("the WKT is a projected CRS");
    };
    let res = crs.identify_with(Some(&all));
    assert_eq!(res.len(), 1, "{}", identified_codes(&res));
    let identifiers = res[0].0.identifiers();
    assert_eq!(identifiers.len(), 1);
    assert_eq!(identifiers[0].code_space(), "ESRI");
    assert_eq!(identifiers[0].code(), "54034");
}

#[test]
fn crs_mercator_1sp_as_wkt1_esri() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_crs_from_proj_string(
        "+proj=merc +lon_0=110 +k=0.997 +x_0=3900000 +y_0=900000 +ellps=bessel +type=crs",
    )
    .expect("proj");
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",\
DATUM[\"D_Unknown_based_on_Bessel_1841_ellipsoid\",\
SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],\
PRIMEM[\"Greenwich\",0.0],\
UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Mercator\"],\
PARAMETER[\"False_Easting\",3900000.0],\
PARAMETER[\"False_Northing\",900000.0],\
PARAMETER[\"Central_Meridian\",110.0],\
PARAMETER[\"Standard_Parallel_1\",4.45405154589748],\
UNIT[\"Meter\",1.0]]"
    );
}

fn esri_from_proj(definition: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    create_crs_from_proj_string(definition)
        .unwrap_or_else(|error| panic!("{definition}: {error}"))
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt")
}

const WGS84_GEOGCS: &str = "GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]";

const BESSEL_GEOGCS: &str = "GEOGCS[\"GCS_unknown\",DATUM[\"D_Unknown_based_on_Bessel_1841_ellipsoid\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]";

#[test]
fn crs_plate_carree_as_wkt1_esri() {
    let wkt = esri_from_proj("+title=my Plate carree +proj=eqc +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"my_Plate_carree\",{WGS84_GEOGCS},PROJECTION[\"Plate_Carree\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_equidistant_cylindrical_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=eqc +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Equidistant_Cylindrical\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",0.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_hotine_oblique_mercator_azimuth_natural_origin_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=omerc +no_uoff +gamma=295 +alpha=295 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Hotine_Oblique_Mercator_Azimuth_Natural_Origin\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",-65.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_rectified_skew_orthomorphic_natural_origin_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=omerc +no_uoff +gamma=3 +alpha=2 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Rectified_Skew_Orthomorphic_Natural_Origin\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",2.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],PARAMETER[\"XY_Plane_Rotation\",3.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_hotine_oblique_mercator_azimuth_center_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=omerc +gamma=2 +alpha=2 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Hotine_Oblique_Mercator_Azimuth_Center\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",2.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_rectified_skew_orthomorphic_center_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=omerc +gamma=3 +alpha=2 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Rectified_Skew_Orthomorphic_Center\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",2.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],PARAMETER[\"XY_Plane_Rotation\",3.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_gauss_kruger_as_wkt1_esri() {
    let wkt = esri_from_proj("+title=my Gauss Kruger +proj=tmerc +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"my_Gauss_Kruger\",{WGS84_GEOGCS},PROJECTION[\"Gauss_Kruger\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Latitude_Of_Origin\",0.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_stereographic_north_pole_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=stere +lat_0=90 +lat_ts=70 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Stereographic_North_Pole\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",70.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_stereographic_south_pole_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=stere +lat_0=-90 +lat_ts=-70 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Stereographic_South_Pole\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",-70.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_krovak_north_orientated_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=krovak +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{BESSEL_GEOGCS},PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.2881397527778],PARAMETER[\"Longitude_Of_Center\",24.8333333333333],PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",-1.0],PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",90.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_krovak_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=krovak +axis=swu +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{BESSEL_GEOGCS},PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.2881397527778],PARAMETER[\"Longitude_Of_Center\",24.8333333333333],PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",1.0],PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",0.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_lcc_1sp_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=lcc +lat_1=1 +lat_0=1 +k=0.9 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",1.0],PARAMETER[\"Scale_Factor\",0.9],PARAMETER[\"Latitude_Of_Origin\",1.0],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_lcc_2sp_as_wkt1_esri() {
    let wkt = esri_from_proj("+proj=lcc +lat_0=1.5 +lat_1=1 +lat_2=2 +type=crs");
    assert_eq!(
        wkt,
        format!(
            "PROJCS[\"unknown\",{WGS84_GEOGCS},PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",1.0],PARAMETER[\"Standard_Parallel_2\",2.0],PARAMETER[\"Latitude_Of_Origin\",1.5],UNIT[\"Meter\",1.0]]"
        )
    );
}

#[test]
fn crs_esri_wkt1_to_esri_wkt1() {
    let input = "PROJCS[\"NAD_1983_CORS96_StatePlane_North_Carolina_FIPS_3200_Ft_US\",\
GEOGCS[\"GCS_NAD_1983_CORS96\",DATUM[\"D_NAD_1983_CORS96\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Lambert_Conformal_Conic\"],\
PARAMETER[\"False_Easting\",2000000.0],\
PARAMETER[\"False_Northing\",0.0],\
PARAMETER[\"Central_Meridian\",-79.0],\
PARAMETER[\"Standard_Parallel_1\",34.33333333333334],\
PARAMETER[\"Standard_Parallel_2\",36.16666666666666],\
PARAMETER[\"Latitude_Of_Origin\",33.75],\
UNIT[\"Foot_US\",0.3048006096012192]]";
    let mut parser = WktParser::new();
    let object = parser.create_from_wkt(input).expect("wkt");
    let Crs::Projected(crs) = object.crs().cloned().expect("crs") else {
        panic!("the WKT is a projected CRS");
    };
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = Crs::Projected(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "PROJCS[\"NAD_1983_CORS96_StatePlane_North_Carolina_FIPS_3200_Ft_US\",\
GEOGCS[\"GCS_NAD_1983_CORS96\",DATUM[\"D_NAD_1983_CORS96\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],\
PROJECTION[\"Lambert_Conformal_Conic\"],\
PARAMETER[\"False_Easting\",2000000.0],\
PARAMETER[\"False_Northing\",0.0],\
PARAMETER[\"Central_Meridian\",-79.0],\
PARAMETER[\"Standard_Parallel_1\",34.3333333333333],\
PARAMETER[\"Standard_Parallel_2\",36.1666666666667],\
PARAMETER[\"Latitude_Of_Origin\",33.75],\
UNIT[\"Foot_US\",0.304800609601219]]"
    );
}

#[test]
fn datum_cs_with_meridian() {
    let east = Axis::create("Easting", "X", "south", UnitOfMeasure::metre())
        .with_meridian(Meridian::create(90.0));
    let north = Axis::create("Northing", "Y", "south", UnitOfMeasure::metre())
        .with_meridian(Meridian::create(180.0));
    let cs = CartesianCS::create(vec![east, north]).expect("cs");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.hide_identifiers().expect("ids");
    let wkt = cs.export_to_wkt(&mut formatter).expect("wkt");
    assert_eq!(
        wkt,
        "CS[Cartesian,2],\n    \
AXIS[\"easting (X)\",south,\n        \
MERIDIAN[90,\n            \
ANGLEUNIT[\"degree\",0.0174532925199433]],\n        \
ORDER[1],\n        \
LENGTHUNIT[\"metre\",1]],\n    \
AXIS[\"northing (Y)\",south,\n        \
MERIDIAN[180,\n            \
ANGLEUNIT[\"degree\",0.0174532925199433]],\n        \
ORDER[2],\n        \
LENGTHUNIT[\"metre\",1]]"
    );
}

#[test]
fn crs_scope_area_bbox_remark() {
    let input = "GEODETICCRS[\"JGD2000\",\
DATUM[\"Japanese Geodetic Datum 2000\",\
  ELLIPSOID[\"GRS 1980\",6378137,298.257222101]],\
CS[Cartesian,3],\
  AXIS[\"(X)\",geocentricX],\
  AXIS[\"(Y)\",geocentricY],\
  AXIS[\"(Z)\",geocentricZ],\
  LENGTHUNIT[\"metre\",1.0],\
SCOPE[\"Geodesy, topographic mapping and cadastre\"],\
AREA[\"Japan\"],\
BBOX[17.09,122.38,46.05,157.64],\
VERTICALEXTENT[-10000,10000],\
TIMEEXTENT[2002-04-01,2011-10-21],\
ID[\"EPSG\",4946],\n\
REMARK[\"some_remark\"]]";
    let mut parser = WktParser::new();
    let object = parser.create_from_wkt(input).expect("wkt");
    let Crs::Geodetic(GeodeticCRS::Geocentric(crs)) = object.crs().cloned().expect("crs") else {
        panic!("the WKT is a geocentric CRS");
    };
    assert_eq!(crs.domains().len(), 1);
    let domain = &crs.domains()[0];
    assert_eq!(
        domain.scope(),
        Some("Geodesy, topographic mapping and cadastre")
    );
    assert_eq!(domain.description(), Some("Japan"));
    assert_eq!(domain.south_bound_latitude(), Some(17.09));
    assert_eq!(domain.west_bound_longitude(), Some(122.38));
    assert_eq!(domain.north_bound_latitude(), Some(46.05));
    assert_eq!(domain.east_bound_longitude(), Some(157.64));
    assert_eq!(domain.vertical_minimum(), Some(-10_000.0));
    assert_eq!(domain.vertical_maximum(), Some(10_000.0));
    assert_eq!(
        domain.vertical_unit().map(UnitOfMeasure::name),
        Some("metre")
    );
    assert_eq!(domain.temporal_start(), Some("2002-04-01"));
    assert_eq!(domain.temporal_stop(), Some("2011-10-21"));
    let wkt = Crs::Geodetic(GeodeticCRS::Geocentric(crs))
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEODCRS[\"JGD2000\",
    DATUM[\"Japanese Geodetic Datum 2000\",
        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,
            LENGTHUNIT[\"metre\",1]]],
    PRIMEM[\"Greenwich\",0,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    CS[Cartesian,3],
        AXIS[\"(X)\",geocentricX,
            ORDER[1],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(Y)\",geocentricY,
            ORDER[2],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(Z)\",geocentricZ,
            ORDER[3],
            LENGTHUNIT[\"metre\",1]],
    SCOPE[\"Geodesy, topographic mapping and cadastre\"],
    AREA[\"Japan\"],
    BBOX[17.09,122.38,46.05,157.64],
    VERTICALEXTENT[-10000,10000,
        LENGTHUNIT[\"metre\",1]],
    TIMEEXTENT[2002-04-01,2011-10-21],
    ID[\"EPSG\",4946],
    REMARK[\"some_remark\"]]"
    );
}

#[test]
fn crs_usage() {
    let input = "GEODETICCRS[\"JGD2000\",\
DATUM[\"Japanese Geodetic Datum 2000\",\
  ELLIPSOID[\"GRS 1980\",6378137,298.257222101]],\
CS[Cartesian,3],\
  AXIS[\"(X)\",geocentricX],\
  AXIS[\"(Y)\",geocentricY],\
  AXIS[\"(Z)\",geocentricZ],\
  LENGTHUNIT[\"metre\",1.0],\
USAGE[SCOPE[\"scope\"],AREA[\"area.\"]]]";
    let mut parser = WktParser::new();
    let object = parser.create_from_wkt(input).expect("wkt");
    let crs = object.crs().cloned().expect("crs");
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEODCRS[\"JGD2000\",
    DATUM[\"Japanese Geodetic Datum 2000\",
        ELLIPSOID[\"GRS 1980\",6378137,298.257222101,
            LENGTHUNIT[\"metre\",1,
                ID[\"EPSG\",9001]]]],
    PRIMEM[\"Greenwich\",0,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8901]],
    CS[Cartesian,3],
        AXIS[\"(X)\",geocentricX,
            ORDER[1],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(Y)\",geocentricY,
            ORDER[2],
            LENGTHUNIT[\"metre\",1]],
        AXIS[\"(Z)\",geocentricZ,
            ORDER[3],
            LENGTHUNIT[\"metre\",1]],
    USAGE[
        SCOPE[\"scope\"],
        AREA[\"area.\"]]]"
    );
}

#[test]
fn crs_multiple_id() {
    let crs = GeodeticCRS::create(
        "WGS 84",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("crs")
    .with_identifier("authorityA", "codeA")
    .with_identifier("authorityB", "codeB");
    let wkt = Crs::Geodetic(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015Simplified))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
GEODCRS[\"WGS 84\",
    DATUM[\"World Geodetic System 1984\",
        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],
    CS[Cartesian,3],
        AXIS[\"(X)\",geocentricX],
        AXIS[\"(Y)\",geocentricY],
        AXIS[\"(Z)\",geocentricZ],
        UNIT[\"metre\",1],
    ID[\"authorityA\",\"codeA\"],
    ID[\"authorityB\",\"codeB\"]]"
    );
}

fn odn_height() -> VerticalCRS {
    let datum = VerticalReferenceFrame::named("Ordnance Datum Newlyn").with_identifier("EPSG", "5101");
    VerticalCRS::create(
        "ODN height",
        datum,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "5701")
}

#[test]
fn crs_vertical_crs_as_wkt2() {
    let crs = odn_height();
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!Crs::Vertical(crs.clone()).is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
    let wkt = Crs::Vertical(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
VERTCRS[\"ODN height\",
    VDATUM[\"Ordnance Datum Newlyn\"],
    CS[vertical,1],
        AXIS[\"gravity-related height (H)\",up,
            LENGTHUNIT[\"metre\",1]],
    ID[\"EPSG\",5701]]"
    );
}

#[test]
fn crs_vertical_crs_as_wkt1_gdal() {
    let wkt = Crs::Vertical(odn_height())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
VERT_CS[\"ODN height\",
    VERT_DATUM[\"Ordnance Datum Newlyn\",2005,
        AUTHORITY[\"EPSG\",\"5101\"]],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Gravity-related height\",UP],
    AUTHORITY[\"EPSG\",\"5701\"]]"
    );
}

#[test]
fn crs_vertical_crs_as_wkt1_esri() {
    let wkt = Crs::Vertical(odn_height())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri))
        .expect("wkt");
    assert_eq!(
        wkt,
        "VERTCS[\"ODN_height\",VDATUM[\"Ordnance_Datum_Newlyn\"],PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_vertical_crs_as_wkt1_esri_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let wkt = Crs::Vertical(odn_height())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("wkt");
    assert_eq!(
        wkt,
        "VERTCS[\"Newlyn\",VDATUM[\"Ordnance_Datum_Newlyn\"],PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_vertical_crs_down_as_wkt1_esri() {
    let input = "VERTCS[\"Caspian\",VDATUM[\"Caspian_Sea\"],\
PARAMETER[\"Vertical_Shift\",0.0],\
PARAMETER[\"Direction\",-1.0],UNIT[\"Meter\",1.0]]";
    let mut parser = WktParser::new();
    let object = parser.create_from_wkt(input).expect("wkt");
    let Crs::Vertical(crs) = object.crs().cloned().expect("crs") else {
        panic!("the WKT is a vertical CRS");
    };
    let wkt = Crs::Vertical(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri))
        .expect("wkt");
    assert_eq!(wkt, input);
}

#[test]
fn crs_vertical_crs_esri_115834_as_wkt1_esri_with_database() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "ESRI")
        .create_coordinate_reference_system("115834")
        .expect("ESRI:115834");
    let wkt = crs
        .export_to_wkt(
            &mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db),
        )
        .expect("wkt");
    assert_eq!(
        wkt,
        "VERTCS[\"NAD83(CSRS)v5\",\
DATUM[\"North_American_Datum_of_1983_(CSRS)_version_5\",\
SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],\
PARAMETER[\"Vertical_Shift\",0.0],\
PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

fn vertical_like(source: &VerticalCRS, name: &str, code: Option<&str>) -> VerticalCRS {
    let mut crs = VerticalCRS::create(
        name,
        source.datum().expect("datum").clone(),
        source.coordinate_system().clone(),
    );
    if let Some(code) = code {
        crs = crs.with_identifier("EPSG", code);
    }
    crs
}

fn vertical_hit(hits: &[(Crs, i32)]) -> Vec<(String, i32)> {
    hits.iter()
        .map(|(crs, confidence)| {
            let Crs::Vertical(crs) = crs else {
                panic!("identify returned a vertical CRS");
            };
            let code = crs.identifiers().first().expect("id").code().to_owned();
            (code, *confidence)
        })
        .collect()
}

#[test]
fn crs_vertical_crs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let source = factory.create_vertical_crs("7651").expect("EPSG:7651");
    assert_eq!(
        vertical_hit(&source.identify_with(Some(&factory))),
        vec![("7651".to_owned(), 100)]
    );
    assert!(source.identify_with(None).is_empty());

    let missing = vertical_like(&source, source.name(), Some("1"));
    assert!(missing.identify_with(Some(&factory)).is_empty());

    let wrong = vertical_like(&source, source.name(), Some("7652"));
    assert_eq!(
        vertical_hit(&wrong.identify_with(Some(&factory))),
        vec![("7652".to_owned(), 25)]
    );

    let named = vertical_like(&source, source.name(), None);
    assert_eq!(
        vertical_hit(&named.identify_with(Some(&factory))),
        vec![("7651".to_owned(), 100)]
    );

    let equivalent_name = vertical_like(&source, "Kumul_34_height", None);
    assert_eq!(
        vertical_hit(&equivalent_name.identify_with(Some(&factory))),
        vec![("7651".to_owned(), 90)]
    );

    let other = factory.create_vertical_crs("7652").expect("EPSG:7652");
    let mismatched = vertical_like(&other, "Kumul_34_height", None);
    assert_eq!(
        vertical_hit(&mismatched.identify_with(Some(&factory))),
        vec![("7651".to_owned(), 25)]
    );

    let unmatched = vertical_like(&source, "no match", None);
    assert!(unmatched.identify_with(Some(&factory)).is_empty());
}

#[test]
fn crs_vertical_crs_datum_ensemble() {
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Vertical(VerticalReferenceFrame::named("vdatum1")),
            Datum::Vertical(VerticalReferenceFrame::named("vdatum2")),
        ],
        100.0,
    )
    .expect("ensemble");
    let crs = VerticalCRS::from_ensemble(
        "unnamed",
        ensemble,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .expect("crs");
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.simul_cur_node_has_id();
    let wkt = Crs::Vertical(crs).export_to_wkt(&mut formatter).expect("wkt");
    assert_eq!(
        wkt,
        "\
VERTCRS[\"unnamed\",
    ENSEMBLE[\"unnamed\",
        MEMBER[\"vdatum1\"],
        MEMBER[\"vdatum2\"],
        ENSEMBLEACCURACY[100]],
    CS[vertical,1],
        AXIS[\"gravity-related height (H)\",up,
            LENGTHUNIT[\"metre\",1]]]"
    );
}

#[test]
fn crs_vertical_crs_ensemble_exception_in_create() {
    let height = VerticalCS::create_gravity_related_height(UnitOfMeasure::metre());
    let missing = VerticalCRS::create_with("", None, None, height.clone()).expect_err("anchors");
    assert_eq!(
        missing.to_string(),
        "One of Datum or DatumEnsemble should be defined"
    );
    let ensemble = DatumEnsemble::create(
        "",
        vec![
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
            Datum::Geodetic(GeodeticReferenceFrame::epsg_6326()),
        ],
        100.0,
    )
    .expect("geodetic ensemble");
    let wrong = VerticalCRS::create_with("", None, Some(ensemble), height).expect_err("kind");
    assert_eq!(
        wrong.to_string(),
        "Ensemble should contain VerticalReferenceFrame"
    );
}

#[test]
fn datum_vdatum_with_anchor() {
    let datum = VerticalReferenceFrame::named("Ordnance Datum Newlyn")
        .with_identifier("EPSG", "5101")
        .with_anchor("my anchor")
        .with_realization(RealizationMethod::Levelling);
    assert_eq!(
        datum.realization_method(),
        Some(RealizationMethod::Levelling)
    );
    let wkt = datum
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
VDATUM[\"Ordnance Datum Newlyn\",
    ANCHOR[\"my anchor\"],
    ID[\"EPSG\",5101]]"
    );
    assert!(datum.is_equivalent_to(&datum));
    assert!(!datum.is_equivalent_to_datum(&Datum::Geodetic(GeodeticReferenceFrame::epsg_6326())));
}

fn unimplemented_conversion() -> Conversion {
    Conversion::create_with_method("PROJ unimplemented", "", "", Vec::new())
}

fn derived_projected_wkt() -> DerivedProjectedCRS {
    DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        Conversion::create("unnamed", "PROJ unimplemented", Vec::new()),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
}

fn derived_projected() -> DerivedProjectedCRS {
    DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        unimplemented_conversion(),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
}

fn derived_vertical() -> DerivedVerticalCRS {
    DerivedVerticalCRS::create(
        "Derived vertCRS",
        odn_height(),
        Conversion::create("unnamed", "PROJ unimplemented", Vec::new()),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
}

#[test]
fn crs_derived_projected_crs_wkt2_2019() {
    let derived = derived_projected_wkt();
    let wkt = Crs::DerivedProjected(derived.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    assert!(derived.is_equivalent_to(&derived));
    assert!(derived.clone().is_equivalent_to(&derived));
    assert!(!Crs::DerivedProjected(derived.clone()).is_equivalent_to(&Crs::Vertical(odn_height())));
    assert!(Crs::DerivedProjected(derived).extract_geodetic_crs().is_some());
}

#[test]
fn crs_derived_projected_crs_wkt2_2015() {
    let exported = Crs::DerivedProjected(derived_projected_wkt())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015));
    assert!(exported.is_err());
}

#[test]
fn crs_derived_vertical_crs_basic() {
    let derived = derived_vertical();
    assert!(derived.is_equivalent_to(&derived));
    assert!(derived.clone().is_equivalent_to(&derived));
    assert!(Crs::DerivedVertical(derived.clone())
        .is_equivalent_to(&Crs::DerivedVertical(derived.clone())));
    assert!(!Crs::DerivedVertical(derived.clone())
        .is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
    assert!(!Crs::DerivedVertical(derived.clone())
        .is_equivalent_to(&Crs::Vertical(derived.base_crs().clone())));
    assert!(!Crs::Vertical(derived.base_crs().clone())
        .is_equivalent_to(&Crs::DerivedVertical(derived)));
}

#[test]
fn crs_derived_vertical_crs_wkt2() {
    let wkt = Crs::DerivedVertical(derived_vertical())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt2");
    assert_eq!(
        wkt,
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
    let exported = Crs::DerivedVertical(derived_vertical())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert_eq!(
        exported.expect_err("wkt1").to_string(),
        "DerivedVerticalCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_derived_vertical_crs_wkt1_when_simple_derivation() {
    let conversion = Conversion::create_change_vertical_unit_no_conv_factor(
        &OpProps::new().name("Vertical Axis Unit Conversion"),
    )
    .expect("conversion");
    let derived = DerivedVerticalCRS::create(
        "Derived vertCRS",
        odn_height(),
        conversion,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::foot()),
    );
    let wkt = Crs::DerivedVertical(derived)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt1");
    assert_eq!(
        wkt,
        "\
VERT_CS[\"Derived vertCRS\",
    VERT_DATUM[\"Ordnance Datum Newlyn\",2005,
        AUTHORITY[\"EPSG\",\"5101\"]],
    UNIT[\"foot\",0.3048,
        AUTHORITY[\"EPSG\",\"9002\"]],
    AXIS[\"Gravity-related height\",UP]]"
    );
}

fn derived_engineering() -> DerivedEngineeringCRS {
    DerivedEngineeringCRS::create(
        "Derived EngineeringCRS",
        engineering_crs(),
        Conversion::create("unnamed", "PROJ unimplemented", Vec::new()),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
}

#[test]
fn crs_derived_engineering_crs_wkt2() {
    let derived = derived_engineering();
    assert!(derived.is_equivalent_to(&derived));
    assert!(derived.clone().is_equivalent_to(&derived));
    assert!(!Crs::DerivedEngineering(derived.clone())
        .is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
    assert!(derived.coordinate_system().is_equivalent_to(&CoordinateSystem::Cartesian(
        CartesianCS::create_easting_northing(UnitOfMeasure::metre())
    )));
    assert!(derived.datum().is_equivalent_to(engineering_crs().datum()));
    let wkt = Crs::DerivedEngineering(derived)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let rejected = Crs::DerivedEngineering(derived_engineering())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015));
    assert_eq!(
        rejected.expect_err("wkt2-2015").to_string(),
        "DerivedEngineeringCRS can only be exported to WKT2:2019"
    );
}

#[test]
fn crs_derived_engineering_crs_wkt1() {
    let exported = Crs::DerivedEngineering(derived_engineering())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert_eq!(
        exported.expect_err("wkt1").to_string(),
        "DerivedEngineeringCRS can only be exported to WKT2:2019"
    );
}

fn date_time_temporal() -> TemporalCRS {
    TemporalCRS::create(
        "Temporal CRS",
        TemporalDatum::create(
            "Gregorian calendar",
            "0000-01-01",
            "proleptic Gregorian",
        ),
        DateTimeTemporalCS::create(Axis::create(
            "Time",
            "T",
            "future",
            UnitOfMeasure::none(),
        )),
    )
}

fn derived_temporal() -> DerivedTemporalCRS {
    let base = date_time_temporal();
    let coordinate_system = base.coordinate_system().clone();
    DerivedTemporalCRS::create(
        "Derived TemporalCRS",
        base,
        Conversion::create("unnamed", "PROJ unimplemented", Vec::new()),
        coordinate_system,
    )
}

#[test]
fn crs_derive_temporal_crs_wkt2() {
    let derived = derived_temporal();
    assert!(derived.is_equivalent_to(&derived));
    assert!(derived.clone().is_equivalent_to(&derived));
    assert!(!Crs::DerivedTemporal(derived.clone())
        .is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
    assert!(derived
        .coordinate_system()
        .is_equivalent_to(date_time_temporal().coordinate_system()));
    assert!(derived.datum().is_equivalent_to(date_time_temporal().datum()));
    let wkt = Crs::DerivedTemporal(derived)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let exported = Crs::DerivedTemporal(derived_temporal())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert_eq!(
        exported.expect_err("wkt1").to_string(),
        "DerivedTemporalCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_create_bound_crs_to_wgs84_if_possible() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let unchanged = [
        "4326", "32631", "5670", "3844", "2171", "4269", "5340", "6316", "2936",
    ];
    for code in unchanged {
        let crs = epsg
            .create_coordinate_reference_system(code)
            .unwrap_or_else(|_| panic!("EPSG:{code}"));
        let bound = crs.create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never);
        assert!(bound.is_equivalent_to(&crs), "EPSG:{code} stays itself");
    }
    let ntf = epsg
        .create_coordinate_reference_system("4807")
        .expect("4807");
    let ntf_bound = ntf.create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never);
    assert!(!ntf_bound.is_equivalent_to(&ntf));
    assert!(ntf_bound
        .create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never)
        .is_equivalent_to(&ntf_bound));
    let Crs::Bound(ntf_bound) = ntf_bound else {
        panic!("NTF (Paris) is a bound CRS");
    };
    assert_eq!(
        ntf_bound.export_to_proj_string().expect("proj"),
        "+proj=longlat +ellps=clrk80ign +pm=paris +towgs84=-168,-60,320,0,0,0,0 +no_defs +type=crs"
    );
    let compound = create_from_user_input("EPSG:4326+3855", &db).expect("compound");
    let compound_bound =
        compound.create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never);
    assert!(!compound_bound.is_equivalent_to(&compound));
    assert!(compound_bound
        .create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never)
        .is_equivalent_to(&compound_bound));
    let Crs::Compound(compound_bound) = compound_bound else {
        panic!("EGM2008 compound stays a compound CRS");
    };
    assert_eq!(
        compound_bound.export_to_proj_string().expect("proj"),
        "+proj=longlat +datum=WGS84 +geoidgrids=us_nga_egm08_25.tif +geoid_crs=WGS84 +vunits=m +no_defs +type=crs"
    );
    let vertical = Crs::Vertical(odn_height());
    assert!(vertical
        .create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never)
        .is_equivalent_to(&vertical));
    let handmade = Crs::Compound(
        CompoundCRS::create(
            "horizontal + vertical",
            vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
        )
        .expect("compound")
        .with_identifier("codespace", "code"),
    );
    assert!(handmade
        .create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never)
        .is_equivalent_to(&handmade));
    let ignf = AuthorityFactory::create(&db, "IGNF");
    let stereo = ignf
        .create_coordinate_reference_system("TERA50STEREO")
        .expect("TERA50STEREO");
    let stereo_bound =
        stereo.create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never);
    let Crs::Bound(stereo_bound) = stereo_bound else {
        panic!("TERA50STEREO is a bound CRS");
    };
    assert_eq!(
        stereo_bound.export_to_proj_string().expect("proj"),
        "+proj=stere +lat_0=-90 +lon_0=140 +k=0.960272946 +x_0=300000 +y_0=-2299363.482 +ellps=intl +towgs84=324.8,153.6,172.1,0,0,0,0 +units=m +no_defs +type=crs"
    );
    let geocentric = ignf
        .create_coordinate_reference_system("PGP50")
        .expect("PGP50");
    let geocentric_bound =
        geocentric.create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never);
    let Crs::Bound(geocentric_bound) = geocentric_bound else {
        panic!("PGP50 is a bound CRS");
    };
    assert_eq!(
        geocentric_bound.export_to_proj_string().expect("proj"),
        "+proj=geocent +ellps=intl +towgs84=324.8,153.6,172.1,0,0,0,0 +units=m +no_defs +type=crs"
    );
    let gda = epsg
        .create_coordinate_reference_system("7842")
        .expect("7842");
    let started = std::time::Instant::now();
    let _ = gda.create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never);
    assert!(started.elapsed().as_millis() <= 500);
    let pulkovo = epsg
        .create_coordinate_reference_system("2936")
        .expect("2936");
    let wkt = pulkovo
        .export_to_wkt(
            &mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db.clone()),
        )
        .expect("wkt1");
    let parsed = WktParser::new()
        .attach_database(db.clone())
        .create_from_wkt(&wkt)
        .expect("parse")
        .crs()
        .expect("crs")
        .clone();
    assert!(parsed
        .create_bound_crs_to_wgs84_if_possible(Some(&db), IntermediateCrsUse::Never)
        .is_equivalent_to(&parsed));
}

#[test]
fn crs_strip_vertical_component() {
    let stripped = Crs::Geographic(GeographicCRS::epsg_4979()).strip_vertical_component();
    let Crs::Geographic(geographic) = stripped else {
        panic!("EPSG:4979 stays a geographic CRS");
    };
    assert_eq!(geographic.coordinate_system().axes().len(), 2);
    let wgs84 = Crs::Geographic(GeographicCRS::epsg_4326());
    assert!(wgs84.strip_vertical_component().is_equivalent_to(&wgs84));
    let projected = ProjectedCRS::create(
        "",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create(vec![
            Axis::create("Easting", "E", "east", UnitOfMeasure::metre()),
            Axis::create("Northing", "N", "north", UnitOfMeasure::metre()),
            Axis::create("Height", "z", "up", UnitOfMeasure::metre()),
        ])
        .expect("cartesian"),
    );
    let stripped = Crs::Projected(projected).strip_vertical_component();
    let Crs::Projected(projected) = stripped else {
        panic!("the 3D projected CRS stays projected");
    };
    assert_eq!(projected.coordinate_system().axes().len(), 2);
    let promoted = derived_projected().promote_to_3d();
    let CoordinateSystem::Cartesian(cs) = promoted.coordinate_system() else {
        panic!("the promoted derived projected CRS stays cartesian");
    };
    assert_eq!(cs.axes().len(), 3);
    let stripped = Crs::DerivedProjected(promoted).strip_vertical_component();
    let Crs::DerivedProjected(derived) = stripped else {
        panic!("the derived projected CRS stays derived projected");
    };
    let CoordinateSystem::Cartesian(cs) = derived.coordinate_system() else {
        panic!("the stripped derived projected CRS stays cartesian");
    };
    assert_eq!(cs.axes().len(), 2);
}

#[test]
fn crs_alter_geodetic_crs() {
    let target = GeodeticCRS::Geographic(GeographicCRS::epsg_4979());
    let geographic = Crs::Geographic(GeographicCRS::epsg_4326()).alter_geodetic_crs(&target);
    assert!(geographic.is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4979())));
    let projected = Crs::Projected(utm_31n()).alter_geodetic_crs(&target);
    let Crs::Projected(projected) = projected else {
        panic!("the projected CRS stays projected");
    };
    assert!(Crs::Geodetic(projected.base_crs().clone())
        .is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4979())));
    let compound = CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
    )
    .expect("compound");
    let altered = Crs::Compound(compound).alter_geodetic_crs(&target);
    let Crs::Compound(altered) = altered else {
        panic!("the compound CRS stays compound");
    };
    let geographic = altered.components()[0]
        .extract_geographic_crs()
        .expect("geographic");
    assert!(geographic.is_equivalent_to(&GeographicCRS::epsg_4979()));
    let vertical = Crs::Vertical(odn_height());
    assert!(vertical
        .alter_geodetic_crs(&target)
        .is_equivalent_to(&vertical));
    let derived = Crs::DerivedProjected(derived_projected()).alter_geodetic_crs(&target);
    let Crs::DerivedProjected(derived) = derived else {
        panic!("the derived projected CRS stays derived projected");
    };
    assert!(Crs::Geodetic(derived.base_crs().base_crs().clone())
        .is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4979())));
}

#[test]
fn crs_alter_cs_linear_unit() {
    let unit = UnitOfMeasure::new("my unit", 2.0, UnitKind::Linear);
    let projected = Crs::Projected(utm_31n()).alter_cs_linear_unit(&unit);
    let Crs::Projected(projected) = projected else {
        panic!("the projected CRS stays projected");
    };
    assert_eq!(projected.coordinate_system().axes().len(), 2);
    for axis in projected.coordinate_system().axes() {
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }
    let derived = Crs::DerivedProjected(derived_projected()).alter_cs_linear_unit(&unit);
    let Crs::DerivedProjected(derived) = derived else {
        panic!("the derived projected CRS stays derived projected");
    };
    let CoordinateSystem::Cartesian(cs) = derived.coordinate_system() else {
        panic!("the derived projected CRS stays cartesian");
    };
    assert_eq!(cs.axes().len(), 2);
    for axis in cs.axes() {
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }
    let northing_easting = DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        Conversion::create("unnamed", "PROJ unimplemented", Vec::new()),
        CoordinateSystem::Cartesian(
            CartesianCS::create(vec![
                Axis::create("Northing", "N", "north", UnitOfMeasure::foot()),
                Axis::create("Easting", "E", "east", UnitOfMeasure::foot()),
            ])
            .expect("northing easting"),
        ),
    );
    let altered = Crs::DerivedProjected(northing_easting).alter_cs_linear_unit(&unit);
    let Crs::DerivedProjected(altered) = altered else {
        panic!("northing/easting stays a derived projected CRS");
    };
    let CoordinateSystem::Cartesian(cs) = altered.coordinate_system() else {
        panic!("northing/easting stays cartesian");
    };
    assert_eq!(cs.axes().len(), 2);
    assert_eq!(cs.axes()[0].unit().name(), "my unit");
    assert_eq!(cs.axes()[0].direction(), "north");
    assert_eq!(cs.axes()[0].unit().conversion_to_si(), 2.0);
    assert_eq!(cs.axes()[1].unit().name(), "my unit");
    assert_eq!(cs.axes()[1].direction(), "east");
    assert_eq!(cs.axes()[1].unit().conversion_to_si(), 2.0);
    let geocentric = Crs::Geodetic(GeodeticCRS::epsg_4978()).alter_cs_linear_unit(&unit);
    let Crs::Geodetic(GeodeticCRS::Geocentric(geocentric)) = geocentric else {
        panic!("EPSG:4978 stays geocentric");
    };
    assert_eq!(geocentric.coordinate_system().axes().len(), 3);
    for axis in geocentric.coordinate_system().axes() {
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }
    let geographic = Crs::Geographic(GeographicCRS::epsg_4979()).alter_cs_linear_unit(&unit);
    let Crs::Geographic(geographic) = geographic else {
        panic!("EPSG:4979 stays geographic");
    };
    let axes = geographic.coordinate_system().axes();
    assert_eq!(axes.len(), 3);
    assert_ne!(axes[0].unit().name(), "my unit");
    assert_ne!(axes[0].unit().conversion_to_si(), 2.0);
    assert_ne!(axes[1].unit().name(), "my unit");
    assert_ne!(axes[1].unit().conversion_to_si(), 2.0);
    assert_eq!(axes[2].unit().name(), "my unit");
    assert_eq!(axes[2].unit().conversion_to_si(), 2.0);
    let vertical = Crs::Vertical(odn_height()).alter_cs_linear_unit(&unit);
    let Crs::Vertical(vertical) = vertical else {
        panic!("the vertical CRS stays vertical");
    };
    assert_eq!(vertical.coordinate_system().axis().unit().name(), "my unit");
    assert_eq!(vertical.coordinate_system().axis().unit().conversion_to_si(), 2.0);
    let engineering = WktParser::new()
        .create_from_wkt("LOCAL_CS[\"foo\"]")
        .expect("local")
        .crs()
        .expect("crs")
        .clone();
    let altered = engineering.alter_cs_linear_unit(&unit);
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    formatter.set_lines(WktLines::Single);
    assert_eq!(
        altered.export_to_wkt(&mut formatter).expect("wkt1"),
        "LOCAL_CS[\"foo\",UNIT[\"my unit\",2],AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH]]"
    );
    let compound = CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
    )
    .expect("compound");
    let altered = Crs::Compound(compound).alter_cs_linear_unit(&unit);
    let Crs::Compound(altered) = altered else {
        panic!("the compound CRS stays compound");
    };
    assert_eq!(altered.components().len(), 2);
    for component in altered.components() {
        let axis = match component {
            Crs::Projected(crs) => crs.coordinate_system().axes()[0].unit().clone(),
            Crs::Vertical(crs) => crs.coordinate_system().axis().unit().clone(),
            _ => panic!("a compound component is projected or vertical"),
        };
        assert_eq!(axis.name(), "my unit");
        assert_eq!(axis.conversion_to_si(), 2.0);
    }
    let bound = Crs::Bound(
        BoundCRS::create_from_towgs84(
            Crs::Projected(utm_31n()),
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        )
        .expect("bound"),
    )
    .alter_cs_linear_unit(&unit);
    let Crs::Bound(bound) = bound else {
        panic!("the bound CRS stays bound");
    };
    let Crs::Projected(projected) = bound.source_crs() else {
        panic!("the bound source stays projected");
    };
    for axis in projected.coordinate_system().axes() {
        assert_eq!(axis.unit().name(), "my unit");
        assert_eq!(axis.unit().conversion_to_si(), 2.0);
    }
    let parametric = Crs::Parametric(parametric_crs());
    assert!(parametric
        .alter_cs_linear_unit(&unit)
        .is_equivalent_to(&parametric));
}

#[test]
fn crs_alter_parameters_linear_unit() {
    let unit = UnitOfMeasure::new("my unit", 2.0, UnitKind::Unknown);
    let kept = utm_31n().alter_parameters_linear_unit(&unit, LinearParameterScale::KeepNumber);
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.set_lines(WktLines::Single);
    let wkt = Crs::Projected(kept)
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert!(
        wkt.contains("PARAMETER[\"Longitude of natural origin\",3"),
        "{wkt}"
    );
    assert!(
        wkt.contains("PARAMETER[\"False easting\",500000,UNIT[\"my unit\",2]"),
        "{wkt}"
    );
    let converted =
        utm_31n().alter_parameters_linear_unit(&unit, LinearParameterScale::ConvertValue);
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2015);
    formatter.set_lines(WktLines::Single);
    let wkt = Crs::Projected(converted)
        .export_to_wkt(&mut formatter)
        .expect("wkt");
    assert!(
        wkt.contains("PARAMETER[\"False easting\",250000,UNIT[\"my unit\",2]"),
        "{wkt}"
    );
}

#[test]
fn crs_date_time_temporal_crs_wkt2() {
    let crs = date_time_temporal();
    let wkt = Crs::Temporal(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt2");
    assert_eq!(
        wkt,
        "\
TIMECRS[\"Temporal CRS\",
    TDATUM[\"Gregorian calendar\",
        TIMEORIGIN[0000-01-01]],
    CS[temporal,1],
        AXIS[\"time (T)\",future]]"
    );
    let wkt1 = Crs::Temporal(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert!(wkt1.is_err());
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!Crs::Temporal(crs).is_equivalent_to(&Crs::Vertical(odn_height())));
}

#[test]
fn crs_date_time_temporal_crs_wkt2_2019() {
    let wkt = Crs::Temporal(date_time_temporal())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let crs = TemporalCRS::create_count(
        "GPS milliseconds",
        TemporalDatum::create(
            "GPS time origin",
            "1980-01-01T00:00:00.0Z",
            "proleptic Gregorian",
        ),
        TemporalCountCS::create(Axis::create(
            "",
            "T",
            "future",
            UnitOfMeasure::new("milliseconds (ms)", 0.001, UnitKind::Time),
        )),
    );
    let wkt = Crs::Temporal(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let crs = TemporalCRS::create_count(
        "Calendar hours from 1979-12-29",
        TemporalDatum::create("29 December 1979", "1979-12-29T00", "proleptic Gregorian"),
        TemporalCountCS::create(Axis::create(
            "Time",
            "",
            "future",
            UnitOfMeasure::new("hour", 0.0, UnitKind::Time),
        )),
    );
    let wkt = Crs::Temporal(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let crs = TemporalCRS::create_measure(
        "Decimal Years CE",
        TemporalDatum::create("Common Era", "0000", "proleptic Gregorian"),
        TemporalMeasureCS::create(Axis::create(
            "Decimal years",
            "a",
            "future",
            UnitOfMeasure::new("year", 0.0, UnitKind::Time),
        )),
    );
    let wkt = Crs::Temporal(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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

fn engineering_crs() -> EngineeringCRS {
    EngineeringCRS::create(
        "Engineering CRS",
        EngineeringDatum::create("Engineering datum"),
        CoordinateSystem::Cartesian(CartesianCS::create_easting_northing(UnitOfMeasure::metre())),
    )
}

#[test]
fn crs_engineering_crs_wkt2() {
    let crs = engineering_crs();
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!Crs::Engineering(crs.clone()).is_equivalent_to(&Crs::Vertical(odn_height())));
    let wkt = Crs::Engineering(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt2");
    assert_eq!(
        wkt,
        "\
ENGCRS[\"Engineering CRS\",
    EDATUM[\"Engineering datum\"],
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
}

#[test]
fn crs_engineering_crs_wkt1() {
    let wkt = Crs::Engineering(engineering_crs())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt1");
    assert_eq!(
        wkt,
        "\
LOCAL_CS[\"Engineering CRS\",
    LOCAL_DATUM[\"Engineering datum\",32767],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Easting\",EAST],
    AXIS[\"Northing\",NORTH]]"
    );
}

fn parametric_crs() -> ParametricCRS {
    ParametricCRS::create(
        "Parametric CRS",
        ParametricDatum::create("Parametric datum"),
        ParametricCS::create(Axis::create(
            "pressure",
            "hPa",
            "up",
            UnitOfMeasure::new("HectoPascal", 100.0, UnitKind::Parametric),
        )),
    )
}

#[test]
fn crs_default_identify_method() {
    assert!(Crs::Parametric(parametric_crs()).identify().is_empty());
}

#[test]
fn crs_parametric_crs_wkt2() {
    let crs = parametric_crs();
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!Crs::Parametric(crs.clone()).is_equivalent_to(&Crs::Vertical(odn_height())));
    let wkt = Crs::Parametric(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt2");
    assert_eq!(
        wkt,
        "\
PARAMETRICCRS[\"Parametric CRS\",
    PDATUM[\"Parametric datum\"],
    CS[parametric,1],
        AXIS[\"pressure (hPa)\",up,
            PARAMETRICUNIT[\"HectoPascal\",100]]]"
    );
}

#[test]
fn crs_parametric_crs_wkt1() {
    let exported = Crs::Parametric(parametric_crs())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert!(exported.is_err());
}

fn derived_parametric() -> DerivedParametricCRS {
    DerivedParametricCRS::create(
        "Derived ParametricCRS",
        parametric_crs(),
        Conversion::create("unnamed", "PROJ unimplemented", Vec::new()),
        parametric_crs().coordinate_system().clone(),
    )
}

#[test]
fn crs_derived_parametric_crs_wkt2() {
    let derived = derived_parametric();
    assert!(derived.is_equivalent_to(&derived));
    assert!(derived.clone().is_equivalent_to(&derived));
    assert!(!Crs::DerivedParametric(derived.clone())
        .is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
    assert!(CoordinateSystem::Parametric(derived.coordinate_system().clone()).is_equivalent_to(
        &CoordinateSystem::Parametric(parametric_crs().coordinate_system().clone())
    ));
    assert!(derived.datum().is_equivalent_to(parametric_crs().datum()));
    let wkt = Crs::DerivedParametric(derived)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let exported = Crs::DerivedParametric(derived_parametric())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert_eq!(
        exported.expect_err("wkt1").to_string(),
        "DerivedParametricCRS can only be exported to WKT2"
    );
}

#[test]
fn crs_engineering_crs_unknown_equivalence() {
    let gpkg = "\
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
    let geotiff = "\
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
    let Crs::Engineering(from_gpkg) = WktParser::new()
        .create_from_wkt(gpkg)
        .expect("gpkg")
        .crs()
        .expect("crs")
        .clone()
    else {
        panic!("GPKG WKT is not an engineering CRS");
    };
    let Crs::Engineering(from_geotiff) = WktParser::new()
        .create_from_wkt(geotiff)
        .expect("geotiff")
        .crs()
        .expect("crs")
        .clone()
    else {
        panic!("GeoTIFF WKT is not an engineering CRS");
    };
    assert!(from_gpkg.is_equivalent_to_with(&from_geotiff, Equivalence::Equivalent));
    assert!(from_geotiff.is_equivalent_to_with(&from_gpkg, Equivalence::Equivalent));
}

#[test]
fn crs_compound_crs_valid() {
    let vertical = Crs::Vertical(odn_height());
    assert!(CompoundCRS::create(
        "",
        vec![Crs::Geographic(GeographicCRS::epsg_4326()), vertical.clone()],
    )
    .is_ok());
    assert!(CompoundCRS::create(
        "",
        vec![Crs::Projected(utm_31n()), vertical.clone()],
    )
    .is_ok());
    assert!(CompoundCRS::create(
        "",
        vec![Crs::DerivedProjected(derived_projected()), vertical],
    )
    .is_ok());
    assert!(CompoundCRS::create(
        "",
        vec![
            Crs::DerivedProjected(derived_projected()),
            Crs::DerivedVertical(derived_vertical()),
        ],
    )
    .is_ok());
}

#[test]
fn crs_compound_crs_invalid() {
    let too_few = CompoundCRS::create("", Vec::new()).expect_err("empty");
    assert_eq!(
        too_few.to_string(),
        "compound CRS should have at least 2 components"
    );
    let single = CompoundCRS::create("", vec![Crs::Projected(utm_31n())]).expect_err("one");
    assert_eq!(
        single.to_string(),
        "compound CRS should have at least 2 components"
    );
    let two_geographic = CompoundCRS::create(
        "",
        vec![
            Crs::Geographic(GeographicCRS::epsg_4326()),
            Crs::Geographic(GeographicCRS::epsg_4326()),
        ],
    )
    .expect_err("two geographic");
    let rejected = "components of the compound CRS do not belong to one of the allowed combinations of http://docs.opengeospatial.org/as/18-005r5/18-005r5.html#34";
    assert_eq!(two_geographic.to_string(), rejected);
    let geographic_3d = CompoundCRS::create(
        "",
        vec![
            Crs::Geographic(GeographicCRS::epsg_4979()),
            Crs::Vertical(odn_height()),
        ],
    )
    .expect_err("geographic 3D");
    assert_eq!(geographic_3d.to_string(), rejected);
}

#[test]
fn crs_compound_crs_as_wkt2() {
    let crs = CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
    )
    .expect("compound")
    .with_identifier("codespace", "code");
    let wkt = Crs::Compound(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
COMPOUNDCRS[\"horizontal + vertical\",
    PROJCRS[\"WGS 84 / UTM zone 31N\",
        BASEGEODCRS[\"WGS 84\",
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
        CS[Cartesian,2],
            AXIS[\"(E)\",east,
                ORDER[1],
                LENGTHUNIT[\"metre\",1]],
            AXIS[\"(N)\",north,
                ORDER[2],
                LENGTHUNIT[\"metre\",1]]],
    VERTCRS[\"ODN height\",
        VDATUM[\"Ordnance Datum Newlyn\"],
        CS[vertical,1],
            AXIS[\"gravity-related height (H)\",up,
                LENGTHUNIT[\"metre\",1]]],
    ID[\"codespace\",\"code\"]]"
    );
}

#[test]
fn crs_compound_crs_is_equivalent_to() {
    let crs = CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
    )
    .expect("compound");
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!Crs::Compound(crs.clone()).is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
    let other = CompoundCRS::create(
        "",
        vec![
            Crs::Geographic(GeographicCRS::epsg_4326()),
            Crs::Vertical(odn_height()),
        ],
    )
    .expect("other");
    assert!(!crs.is_equivalent_to(&other));
}

#[test]
fn crs_compound_crs_as_wkt1_gdal() {
    let crs = CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
    )
    .expect("compound")
    .with_identifier("codespace", "code");
    let wkt = Crs::Compound(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt");
    assert_eq!(
        wkt,
        "\
COMPD_CS[\"horizontal + vertical\",
    PROJCS[\"WGS 84 / UTM zone 31N\",
        GEOGCS[\"WGS 84\",
            DATUM[\"WGS_1984\",
                SPHEROID[\"WGS 84\",6378137,298.257223563,
                    AUTHORITY[\"EPSG\",\"7030\"]],
                AUTHORITY[\"EPSG\",\"6326\"]],
            PRIMEM[\"Greenwich\",0,
                AUTHORITY[\"EPSG\",\"8901\"]],
            UNIT[\"degree\",0.0174532925199433,
                AUTHORITY[\"EPSG\",\"9122\"]],
            AUTHORITY[\"EPSG\",\"4326\"]],
        PROJECTION[\"Transverse_Mercator\"],
        PARAMETER[\"latitude_of_origin\",0],
        PARAMETER[\"central_meridian\",3],
        PARAMETER[\"scale_factor\",0.9996],
        PARAMETER[\"false_easting\",500000],
        PARAMETER[\"false_northing\",0],
        UNIT[\"metre\",1,
            AUTHORITY[\"EPSG\",\"9001\"]],
        AXIS[\"Easting\",EAST],
        AXIS[\"Northing\",NORTH],
        AUTHORITY[\"EPSG\",\"32631\"]],
    VERT_CS[\"ODN height\",
        VERT_DATUM[\"Ordnance Datum Newlyn\",2005,
            AUTHORITY[\"EPSG\",\"5101\"]],
        UNIT[\"metre\",1,
            AUTHORITY[\"EPSG\",\"9001\"]],
        AXIS[\"Gravity-related height\",UP],
        AUTHORITY[\"EPSG\",\"5701\"]],
    AUTHORITY[\"codespace\",\"code\"]]"
    );
}

#[test]
fn crs_compound_crs_as_proj_string() {
    let crs = CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
    )
    .expect("compound");
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=31 +datum=WGS84 +units=m +vunits=m +no_defs +type=crs"
    );
}

#[test]
fn crs_compound_crs_no_name_provided() {
    let crs = CompoundCRS::create(
        "",
        vec![Crs::Projected(utm_31n()), Crs::Vertical(odn_height())],
    )
    .expect("compound");
    assert_eq!(crs.name(), "WGS 84 / UTM zone 31N + ODN height");
}

fn compound_like(source: &CompoundCRS, name: &str, code: Option<&str>) -> CompoundCRS {
    let crs = CompoundCRS::create(name, source.components().to_vec()).expect("compound");
    match code {
        Some(code) => crs.with_identifier("EPSG", code),
        None => crs,
    }
}

fn epsg_number(crs: &Crs) -> i32 {
    crs.authority_code("EPSG")
        .and_then(|identifier| identifier.code().parse().ok())
        .unwrap_or(0)
}

fn compound_codes(hits: &[(Crs, i32)]) -> Vec<(i32, i32)> {
    hits.iter()
        .map(|(crs, confidence)| (epsg_number(crs), *confidence))
        .collect()
}

#[test]
fn crs_compound_crs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let source = factory.create_compound_crs("8769").expect("EPSG:8769");
    assert_eq!(
        compound_codes(&source.identify_with(Some(&factory))),
        vec![(8769, 100)]
    );
    assert!(source.identify_with(None).is_empty());

    let missing = compound_like(&source, source.name(), Some("1"));
    assert!(missing.identify_with(Some(&factory)).is_empty());

    let wrong = compound_like(&source, source.name(), Some("8770"));
    assert_eq!(
        compound_codes(&wrong.identify_with(Some(&factory))),
        vec![(8770, 25)]
    );

    let named = compound_like(&source, source.name(), None);
    assert_eq!(
        compound_codes(&named.identify_with(Some(&factory))),
        vec![(8769, 100)]
    );

    let equivalent = compound_like(&source, "NAD83_Ohio_North_ftUS_NAVD88_height_ftUS", None);
    assert_eq!(
        compound_codes(&equivalent.identify_with(Some(&factory))),
        vec![(8769, 90)]
    );

    let other = factory.create_compound_crs("8770").expect("EPSG:8770");
    let mismatched = compound_like(&other, "NAD83_Ohio_North_ftUS_NAVD88_height_ftUS", None);
    assert_eq!(
        compound_codes(&mismatched.identify_with(Some(&factory))),
        vec![(8769, 25)]
    );

    let unrelated = compound_like(&source, "unrelated name", None);
    assert_eq!(
        compound_codes(&unrelated.identify_with(Some(&factory))),
        vec![(8769, 70)]
    );

    let geoid = create_crs_from_proj_string(
        "+proj=tmerc +lat_0=0 +lon_0=72.05 +k=1 +x_0=3500000 \
         +y_0=-5811057.63 +ellps=krass \
         +towgs84=23.57,-140.95,-79.8,0,-0.35,-0.79,-0.22 \
         +geoidgrids=egm08_25.gtx +units=m +no_defs +type=crs",
    )
    .expect("geoidgrids");
    let Crs::Compound(geoid) = geoid else {
        panic!("geoidgrids PROJ string is a compound CRS");
    };
    let _ = geoid.identify_with(Some(&factory));

    let osgb = factory.create_compound_crs("7405").expect("EPSG:7405");
    let wkt = Crs::Compound(osgb)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db.clone()))
        .expect("esri");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let imported = parser
        .create_from_wkt(&wkt)
        .expect("import")
        .crs()
        .expect("crs")
        .clone();
    assert_eq!(
        compound_codes(&imported.identify_with(Some(&factory))),
        vec![(7405, 100)]
    );

    let navd = create_from_user_input("EPSG:4326+5703", &db).expect("4326+5703");
    let wkt = navd
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("esri");
    let parsed = parser
        .create_from_wkt(&wkt)
        .expect("import")
        .crs()
        .expect("crs")
        .clone();
    let hits = parsed.identify_with(Some(&factory));
    assert_eq!(hits.len(), 1);
    assert_eq!(epsg_number(&hits[0].0), 0);
    assert_eq!(hits[0].1, 100);
    let Crs::Compound(found) = &hits[0].0 else {
        panic!("synthesized compound");
    };
    assert_eq!(found.components().len(), 2);
    assert_eq!(epsg_number(&found.components()[0]), 4326);
    assert_eq!(epsg_number(&found.components()[1]), 5703);
}

fn compact_wkt(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn measure_of(value: &ParameterValue) -> &Measure {
    let ParameterValue::Measure(measure) = value else {
        panic!("parameter is a measure");
    };
    measure
}

#[test]
fn crs_bound_crs_to_wkt2() {
    let geographic = GeographicCRS::create(
        "my GEOGCRS",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("geographic");
    let projected = ProjectedCRS::create(
        "my PROJCRS",
        GeodeticCRS::Geographic(geographic),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let crs = BoundCRS::create_from_towgs84(Crs::Projected(projected.clone()), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0])
        .expect("bound");
    assert_eq!(crs.source_crs().name(), projected.name());
    assert_eq!(crs.target_crs().name(), GeographicCRS::epsg_4326().name());
    let transformation = crs.transformation();
    let CoordinateOperation::Transformation(transformation) = transformation else {
        panic!("transformation");
    };
    assert_eq!(
        transformation.source_crs().expect("source").name(),
        projected.base_crs().name()
    );
    assert_eq!(
        transformation.target_crs().expect("target").name(),
        GeographicCRS::epsg_4326().name()
    );
    let parameters = transformation.parameters();
    assert_eq!(parameters.len(), 7);
    let expected = [
        ("8605", "X-axis translation", UnitOfMeasure::metre(), 1.0),
        ("8606", "Y-axis translation", UnitOfMeasure::metre(), 2.0),
        ("8607", "Z-axis translation", UnitOfMeasure::metre(), 3.0),
        ("8608", "X-axis rotation", UnitOfMeasure::arc_second(), 4.0),
        ("8609", "Y-axis rotation", UnitOfMeasure::arc_second(), 5.0),
        ("8610", "Z-axis rotation", UnitOfMeasure::arc_second(), 6.0),
        (
            "8611",
            "Scale difference",
            UnitOfMeasure::parts_per_million(),
            7.0,
        ),
    ];
    for (parameter, (code, name, unit, value)) in parameters.iter().zip(expected) {
        assert_eq!(parameter.code(), code);
        assert_eq!(parameter.name(), name);
        let measure = measure_of(parameter.value());
        assert_eq!(measure.unit(), &unit);
        assert_eq!(measure.value(), value);
    }
    let source_wkt = Crs::Projected(projected)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("source");
    let target_wkt = Crs::Geographic(GeographicCRS::epsg_4326())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("target");
    let expected_wkt = format!(
        "BOUNDCRS[SOURCECRS[{source_wkt}],\nTARGETCRS[{target_wkt}],\n\
         ABRIDGEDTRANSFORMATION[\"Transformation from my GEOGCRS to WGS84\",\n\
         METHOD[\"Position Vector transformation (geog2D domain)\",\n\
         ID[\"EPSG\",9606]],\n\
         PARAMETER[\"X-axis translation\",1,\nID[\"EPSG\",8605]],\n\
         PARAMETER[\"Y-axis translation\",2,\nID[\"EPSG\",8606]],\n\
         PARAMETER[\"Z-axis translation\",3,\nID[\"EPSG\",8607]],\n\
         PARAMETER[\"X-axis rotation\",4,\nID[\"EPSG\",8608]],\n\
         PARAMETER[\"Y-axis rotation\",5,\nID[\"EPSG\",8609]],\n\
         PARAMETER[\"Z-axis rotation\",6,\nID[\"EPSG\",8610]],\n\
         PARAMETER[\"Scale difference\",1.000007,\nID[\"EPSG\",8611]]]]"
    );
    let got = Crs::Bound(crs.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt");
    assert_eq!(compact_wkt(&got), compact_wkt(&expected_wkt));
    assert!(crs.is_equivalent_to(&crs));
    assert!(crs.clone().is_equivalent_to(&crs));
    assert!(!Crs::Bound(crs).is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
}

#[test]
fn crs_bound_crs_with_usage() {
    let wkt = r#"BOUNDCRS[
    SOURCECRS[
        PROJCRS["Monte Mario / Italy zone 2",
            BASEGEOGCRS["Monte Mario",
                DATUM["Monte Mario",
                    ELLIPSOID["International 1924",6378388,297,
                        LENGTHUNIT["metre",1]]],
                PRIMEM["Greenwich",0,
                    ANGLEUNIT["degree",0.0174532925199433]],
                ID["EPSG",4265]],
            CONVERSION["unnamed",
                METHOD["Transverse Mercator",
                    ID["EPSG",9807]],
                PARAMETER["Latitude of natural origin",0,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8801]],
                PARAMETER["Longitude of natural origin",15,
                    ANGLEUNIT["degree",0.0174532925199433],
                    ID["EPSG",8802]],
                PARAMETER["Scale factor at natural origin",0.9996,
                    SCALEUNIT["unity",1],
                    ID["EPSG",8805]],
                PARAMETER["False easting",2520000,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8806]],
                PARAMETER["False northing",0,
                    LENGTHUNIT["metre",1],
                    ID["EPSG",8807]]],
            CS[Cartesian,2],
                AXIS["x",east,
                    ORDER[1],
                    LENGTHUNIT["metre",1]],
                AXIS["y",north,
                    ORDER[2],
                    LENGTHUNIT["metre",1]],
            ID["EPSG",3004]]],
    TARGETCRS[
        GEOGCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]],
            CS[ellipsoidal,2],
                AXIS["geodetic latitude (Lat)",north,
                    ORDER[1],
                    ANGLEUNIT["degree",0.0174532925199433]],
                AXIS["geodetic longitude (Lon)",east,
                    ORDER[2],
                    ANGLEUNIT["degree",0.0174532925199433]],
            ID["EPSG",4326]]],
    ABRIDGEDTRANSFORMATION["Transformation from Monte Mario to WGS84",
        METHOD["Position Vector transformation (geog2D domain)",
            ID["EPSG",9606]],
        PARAMETER["X-axis translation",-50.2,
            ID["EPSG",8605]],
        PARAMETER["Y-axis translation",-50.4,
            ID["EPSG",8606]],
        PARAMETER["Z-axis translation",84.8,
            ID["EPSG",8607]],
        PARAMETER["X-axis rotation",-0.69,
            ID["EPSG",8608]],
        PARAMETER["Y-axis rotation",-2.012,
            ID["EPSG",8609]],
        PARAMETER["Z-axis rotation",0.459,
            ID["EPSG",8610]],
        PARAMETER["Scale difference",0.99997192,
            ID["EPSG",8611]]],
    USAGE[
        SCOPE["unknown"],
        AREA["Italy - Sicily onshore"],
        BBOX[36.59,12.36,38.35,15.71]]]"#;
    let crs = wkt_crs(wkt, None);
    let Crs::Bound(bound) = &crs else {
        panic!("the WKT is a bound CRS");
    };
    assert_eq!(bound.name(), "Monte Mario / Italy zone 2");
    let got = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt");
    assert_eq!(got, wkt);
}

fn towgs84_bound(base: Crs) -> BoundCRS {
    BoundCRS::create_from_towgs84(base, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]).expect("towgs84")
}

#[test]
fn crs_bound_crs_crs_link() {
    let nad27 = Crs::Geographic(GeographicCRS::epsg_4267());
    let linked = {
        let bound = towgs84_bound(nad27.clone());
        assert!(bound
            .source_crs()
            .is_equivalent_to(&nad27));
        assert!(bound.source_crs().canonical_bound_crs().is_none());
        bound.base_crs_with_canonical_bound()
    };
    assert!(linked.is_equivalent_to(&nad27));
    let canonical = linked.canonical_bound_crs().expect("canonical bound");
    assert!(canonical.source_crs().is_equivalent_to(&nad27));
    assert!(linked
        .create_bound_crs_to_wgs84_if_possible(None, IntermediateCrsUse::Never)
        .is_equivalent_to(&Crs::Bound(canonical.clone())));

    let projected = Crs::Projected(utm_31n());
    let linked = {
        let bound = towgs84_bound(projected.clone());
        bound.base_crs_with_canonical_bound()
    };
    let canonical = linked.canonical_bound_crs().expect("projected canonical bound");
    assert!(canonical.source_crs().is_equivalent_to(&projected));
}

#[test]
fn crs_bound_crs_to_wkt1() {
    let geographic = GeographicCRS::create(
        "my GEOGCRS",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("geographic");
    let projected = ProjectedCRS::create(
        "my PROJCRS",
        GeodeticCRS::Geographic(geographic),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let crs = BoundCRS::create_from_towgs84(
        Crs::Projected(projected),
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
    )
    .expect("bound");
    let expected = "\
PROJCS[\"my PROJCRS\",
    GEOGCS[\"my GEOGCRS\",
        DATUM[\"WGS_1984\",
            SPHEROID[\"WGS 84\",6378137,298.257223563,
                AUTHORITY[\"EPSG\",\"7030\"]],
            TOWGS84[1,2,3,4,5,6,7],
            AUTHORITY[\"EPSG\",\"6326\"]],
        PRIMEM[\"Greenwich\",0,
            AUTHORITY[\"EPSG\",\"8901\"]],
        UNIT[\"degree\",0.0174532925199433,
            AUTHORITY[\"EPSG\",\"9122\"]]],
    PROJECTION[\"Transverse_Mercator\"],
    PARAMETER[\"latitude_of_origin\",0],
    PARAMETER[\"central_meridian\",3],
    PARAMETER[\"scale_factor\",0.9996],
    PARAMETER[\"false_easting\",500000],
    PARAMETER[\"false_northing\",0],
    UNIT[\"metre\",1,
        AUTHORITY[\"EPSG\",\"9001\"]],
    AXIS[\"Easting\",EAST],
    AXIS[\"Northing\",NORTH]]";
    assert_eq!(
        Crs::Bound(crs)
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("wkt"),
        expected
    );
}

#[test]
fn crs_bound_crs_geographic_crs_to_proj_string() {
    let geographic = GeographicCRS::create(
        "my GEOGCRS",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("geographic");
    let crs = BoundCRS::create_from_towgs84(
        Crs::Geographic(geographic),
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
    )
    .expect("bound");
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=longlat +ellps=WGS84 +towgs84=1,2,3,4,5,6,7 +no_defs +type=crs"
    );
}

#[test]
fn crs_bound_crs_projected_crs_to_proj_string() {
    let geographic = GeographicCRS::create(
        "my GEOGCRS",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("geographic");
    let projected = ProjectedCRS::create(
        "my PROJCRS",
        GeodeticCRS::Geographic(geographic),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let crs = BoundCRS::create_from_towgs84(
        Crs::Projected(projected),
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
    )
    .expect("bound");
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=31 +ellps=WGS84 +towgs84=1,2,3,4,5,6,7 +units=m +no_defs +type=crs"
    );
}

#[test]
fn crs_bound_crs_nadcon_to_proj_string_use_proj_alternative_grid_names_false() {
    let db = DatabaseContext::create().expect("proj.db");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("15851", AlternativeGridNames::Official)
        .expect("EPSG:15851");
    let CoordinateOperation::Transformation(transformation) = operation else {
        panic!("EPSG:15851 is a transformation");
    };
    let crs = BoundCRS::create(
        Crs::Geographic(GeographicCRS::epsg_4267()),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        CoordinateOperation::Transformation(transformation),
    );
    assert_eq!(
        crs.export_to_proj_string_with_database(&db).expect("proj"),
        "+proj=longlat +ellps=clrk66 +nadgrids=us_noaa_conus.tif +no_defs +type=crs"
    );
}

#[test]
fn crs_bound_crs_nadcon_to_proj_string_use_proj_alternative_grid_names_true() {
    let db = DatabaseContext::create().expect("proj.db");
    let operation = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("15851", AlternativeGridNames::Proj)
        .expect("EPSG:15851");
    let CoordinateOperation::Transformation(transformation) = operation else {
        panic!("EPSG:15851 is a transformation");
    };
    let crs = BoundCRS::create(
        Crs::Geographic(GeographicCRS::epsg_4267()),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        CoordinateOperation::Transformation(transformation),
    );
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=longlat +ellps=clrk66 +nadgrids=us_noaa_conus.tif +no_defs +type=crs"
    );
}

fn one_bound(found: Vec<(Crs, i32)>) -> (BoundCRS, i32) {
    assert_eq!(found.len(), 1);
    let (crs, confidence) = found.into_iter().next().expect("one hit");
    let Crs::Bound(bound) = crs else {
        panic!("identified object is a bound CRS");
    };
    (bound, confidence)
}

fn bound_from_wkt(db: &DatabaseContext, wkt: &str) -> Crs {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone()
}

fn transformation_method_code(bound: &BoundCRS) -> String {
    let CoordinateOperation::Transformation(transformation) = bound.transformation() else {
        panic!("binding operation is a transformation");
    };
    transformation.method().code().to_owned()
}

#[test]
fn crs_bound_crs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");

    let tmerc = create_crs_from_proj_string(
        "+proj=tmerc +lat_0=-37.76111111111111 +lon_0=176.4661111111111 +k=1 \
         +x_0=400000 +y_0=800000 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +type=crs",
    )
    .expect("tmerc");
    assert!(matches!(tmerc, Crs::Bound(_)));
    let (bound, confidence) = one_bound(tmerc.identify_with(Some(&factory)));
    assert_eq!(epsg_number(bound.source_crs()), 2106);
    assert_eq!(bound.transformation().name(), "NZGD2000 to WGS 84 (1)");
    assert_eq!(confidence, 70);

    let ohio = bound_from_wkt(
        &db,
        "PROJCS[\"NAD83 / Ohio North\",GEOGCS[\"NAD83\",\
         DATUM[\"North_American_Datum_1983\",SPHEROID[\"GRS 1980\",\
         6378137,298.257222101,AUTHORITY[\"EPSG\",\"7019\"]],\
         TOWGS84[0,0,0,0,0,0,0],AUTHORITY[\"EPSG\",\"6269\"]],\
         PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],\
         UNIT[\"degree\",0.0174532925199433, AUTHORITY[\"EPSG\",\"9122\"]],\
         AUTHORITY[\"EPSG\",\"4269\"]],\
         PROJECTION[\"Lambert_Conformal_Conic_2SP\"],\
         PARAMETER[\"standard_parallel_1\",41.7],\
         PARAMETER[\"standard_parallel_2\",40.43333333333333],\
         PARAMETER[\"latitude_of_origin\",39.66666666666666],\
         PARAMETER[\"central_meridian\",-82.5],\
         PARAMETER[\"false_easting\",1968503.937007874],\
         PARAMETER[\"false_northing\",0],\
         UNIT[\"International Foot\",0.3048,AUTHORITY[\"EPSG\",\"9002\"]],\
         AXIS[\"X\",EAST],AXIS[\"Y\",NORTH],AUTHORITY[\"EPSG\",\"32122\"]]",
    );
    assert!(matches!(ohio, Crs::Bound(_)));
    let (bound, confidence) = one_bound(ohio.identify_with(Some(&factory)));
    assert_eq!(confidence, 25);
    assert_eq!(epsg_number(bound.source_crs()), 32122);
    assert_eq!(transformation_method_code(&bound), "9603");

    let indian = create_crs_from_proj_string(
        "+proj=utm +zone=48 +a=6377276.345 +b=6356075.41314024 \
         +towgs84=198,881,317,0,0,0,0 +units=m +no_defs +type=crs",
    )
    .expect("utm");
    assert!(matches!(indian, Crs::Bound(_)));
    let (bound, confidence) = one_bound(indian.identify_with(Some(&factory)));
    assert_eq!(epsg_number(bound.source_crs()), 3148);
    assert_eq!(confidence, 70);

    let wgs84 = bound_from_wkt(
        &db,
        "GEOGCS[\"WGS84 Coordinate System\",DATUM[\"WGS 1984\",\
         SPHEROID[\"WGS 1984\",6378137,298.257223563],\
         TOWGS84[0,0,0,0,0,0,0],AUTHORITY[\"EPSG\",\"6326\"]],\
         PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433],\
         AUTHORITY[\"EPSG\",\"4326\"]]",
    );
    assert!(matches!(wgs84, Crs::Bound(_)));
    let (bound, confidence) = one_bound(wgs84.identify_with(Some(&factory)));
    assert_eq!(confidence, 100);
    assert_eq!(epsg_number(bound.source_crs()), 4326);
    assert_eq!(transformation_method_code(&bound), "9606");

    let ntf = bound_from_wkt(
        &db,
        "PROJCS[\"NTF (Paris) / Lambert zone II\",\
         GEOGCS[\"NTF (Paris)\",\
         DATUM[\"Nouvelle_Triangulation_Francaise_Paris\",\
         SPHEROID[\"Clarke 1880 (IGN)\",6378249.2,293.4660212936269,\
         AUTHORITY[\"EPSG\",\"7011\"]],TOWGS84[-168,-60,320,0,0,0,0],\
         AUTHORITY[\"EPSG\",\"6807\"]],PRIMEM[\"Paris\",2.33722917,\
         AUTHORITY[\"EPSG\",\"8903\"]],UNIT[\"grad\",0.01570796326794897,\
         AUTHORITY[\"EPSG\",\"9105\"]],AUTHORITY[\"EPSG\",\"4807\"]],\
         PROJECTION[\"Lambert_Conformal_Conic_1SP\"],\
         PARAMETER[\"latitude_of_origin\",52],\
         PARAMETER[\"central_meridian\",0],\
         PARAMETER[\"scale_factor\",0.99987742],\
         PARAMETER[\"false_easting\",600000],\
         PARAMETER[\"false_northing\",2200000],\
         UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]],\
         AXIS[\"X\",EAST],AXIS[\"Y\",NORTH],AUTHORITY[\"EPSG\",\"27572\"]]",
    );
    assert!(matches!(ntf, Crs::Bound(_)));
    let (bound, confidence) = one_bound(ntf.identify_with(Some(&factory)));
    assert_eq!(epsg_number(bound.source_crs()), 27572);
    assert_eq!(bound.transformation().name(), "NTF to WGS 84 (1)");
    assert_eq!(confidence, 100);
}

fn geocentric_translation(source: Crs, target: Crs) -> CoordinateOperation {
    let transformation = Transformation::create(
        &OpProps::new(),
        source,
        target,
        None,
        "Geocentric translations (geog2D domain)",
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("transformation");
    CoordinateOperation::Transformation(transformation)
}

#[test]
fn crs_incompatible_bound_crs_hub_crs_to_wkt1() {
    let source = Crs::Geographic(GeographicCRS::epsg_4326());
    let hub = Crs::Geographic(GeographicCRS::epsg_4807());
    let crs = BoundCRS::create(
        source.clone(),
        hub.clone(),
        geocentric_translation(source, hub),
    );
    assert!(Crs::Bound(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

#[test]
fn crs_incompatible_bound_crs_transformation_to_wkt1() {
    let source = Crs::Geographic(GeographicCRS::epsg_4807());
    let hub = Crs::Geographic(GeographicCRS::epsg_4326());
    let transformation = Transformation::create(
        &OpProps::new(),
        source.clone(),
        hub.clone(),
        None,
        "",
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("transformation");
    let crs = BoundCRS::create(
        source,
        hub,
        CoordinateOperation::Transformation(transformation),
    );
    assert!(Crs::Bound(crs)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

#[test]
fn crs_wkt1_datum_extension_to_wkt1_and_proj_string() {
    let wkt = "PROJCS[\"unnamed\",\n    GEOGCS[\"International 1909 (Hayford)\",\n        DATUM[\"unknown\",\n            SPHEROID[\"intl\",6378388,297],\n            EXTENSION[\"PROJ4_GRIDS\",\"nzgd2kgrid0005.gsb\"]],\n        PRIMEM[\"Greenwich\",0],\n        UNIT[\"degree\",0.0174532925199433]],\n    PROJECTION[\"New_Zealand_Map_Grid\"],\n    PARAMETER[\"latitude_of_origin\",-41],\n    PARAMETER[\"central_meridian\",173],\n    PARAMETER[\"false_easting\",2510000],\n    PARAMETER[\"false_northing\",6023150],\n    UNIT[\"Meter\",1],\n    AXIS[\"Easting\",EAST],\n    AXIS[\"Northing\",NORTH]]";
    let mut parser = WktParser::new();
    let parsed = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    let Crs::Bound(crs) = parsed else {
        panic!("PROJ4_GRIDS builds a bound CRS");
    };
    assert_eq!(
        Crs::Bound(crs.clone())
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("wkt1"),
        wkt
    );
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=nzmg +lat_0=-41 +lon_0=173 +x_0=2510000 +y_0=6023150 \
         +ellps=intl +nadgrids=nzgd2kgrid0005.gsb +units=m +no_defs +type=crs"
    );
}

#[test]
fn crs_wkt1_vert_datum_extension_to_wkt1() {
    let wkt = "VERT_CS[\"EGM2008 geoid height\",\n    VERT_DATUM[\"EGM2008 geoid\",2005,\n        EXTENSION[\"PROJ4_GRIDS\",\"egm08_25.gtx\"],\n        AUTHORITY[\"EPSG\",\"1027\"]],\n    UNIT[\"metre\",1,\n        AUTHORITY[\"EPSG\",\"9001\"]],\n    AXIS[\"Gravity-related height\",UP],\n    AUTHORITY[\"EPSG\",\"3855\"]]";
    let mut parser = WktParser::new();
    let parsed = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    let Crs::Bound(crs) = parsed else {
        panic!("a vertical PROJ4_GRIDS extension builds a bound CRS");
    };
    assert_eq!(
        Crs::Bound(crs)
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("wkt1"),
        wkt
    );
}

#[test]
fn crs_wkt1_vert_datum_extension_to_wkt2() {
    let wkt = "VERT_CS[\"EGM2008 geoid height\",\n    VERT_DATUM[\"EGM2008 geoid\",2005,\n        EXTENSION[\"PROJ4_GRIDS\",\"egm08_25.gtx\"],\n        AUTHORITY[\"EPSG\",\"1027\"]],\n    UNIT[\"metre\",1,\n        AUTHORITY[\"EPSG\",\"9001\"]],\n    AXIS[\"Up\",UP],\n    AUTHORITY[\"EPSG\",\"3855\"]]";
    let expected = "BOUNDCRS[\n    SOURCECRS[\n        VERTCRS[\"EGM2008 geoid height\",\n            VDATUM[\"EGM2008 geoid\"],\n            CS[vertical,1],\n                AXIS[\"up\",up,\n                    LENGTHUNIT[\"metre\",1]],\n            ID[\"EPSG\",3855]]],\n    TARGETCRS[\n        GEODCRS[\"WGS 84\",\n            DATUM[\"World Geodetic System 1984\",\n                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]],\n            CS[ellipsoidal,3],\n                AXIS[\"latitude\",north,\n                    ORDER[1],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n                AXIS[\"longitude\",east,\n                    ORDER[2],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n                AXIS[\"ellipsoidal height\",up,\n                    ORDER[3],\n                    LENGTHUNIT[\"metre\",1]],\n            ID[\"EPSG\",4979]]],\n    ABRIDGEDTRANSFORMATION[\"EGM2008 geoid height to WGS 84 ellipsoidal height\",\n        METHOD[\"GravityRelatedHeight to Geographic3D\"],\n        PARAMETERFILE[\"Geoid (height correction) model file\",\"egm08_25.gtx\",\n            ID[\"EPSG\",8666]]]]";
    let mut parser = WktParser::new();
    let parsed = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    let Crs::Bound(crs) = parsed else {
        panic!("a vertical PROJ4_GRIDS extension builds a bound CRS");
    };
    assert_eq!(
        Crs::Bound(crs)
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("wkt2"),
        expected
    );
}

#[test]
fn crs_wkt1_vert_datum_extension_to_proj_string() {
    let wkt = "VERT_CS[\"EGM2008 geoid height\",\n    VERT_DATUM[\"EGM2008 geoid\",2005,\n        EXTENSION[\"PROJ4_GRIDS\",\"egm08_25.gtx\"],\n        AUTHORITY[\"EPSG\",\"1027\"]],\n    UNIT[\"metre\",1,\n        AUTHORITY[\"EPSG\",\"9001\"]],\n    AXIS[\"Up\",UP],\n    AUTHORITY[\"EPSG\",\"3855\"]]";
    let mut parser = WktParser::new();
    let parsed = parser
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("crs")
        .clone();
    let Crs::Bound(crs) = parsed else {
        panic!("a vertical PROJ4_GRIDS extension builds a bound CRS");
    };
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+geoidgrids=egm08_25.gtx +geoid_crs=WGS84 +vunits=m +no_defs +type=crs"
    );
}

#[test]
fn crs_extract_geographic_crs() {
    let wgs84 = GeographicCRS::epsg_4326();
    let from_geographic = Crs::Geographic(wgs84.clone())
        .extract_geographic_crs()
        .expect("EPSG:4326");
    assert!(from_geographic.is_equivalent_to(&wgs84));
    assert_eq!(from_geographic.identifiers()[0].code(), "4326");

    let projected = ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631");
    let from_projected = Crs::Projected(projected)
        .extract_geographic_crs()
        .expect("UTM base");
    assert!(from_projected.is_equivalent_to(&wgs84));
    assert_eq!(from_projected.identifiers()[0].code(), "4326");

    let compound = CompoundCRS::create(
        "",
        vec![
            Crs::Geographic(GeographicCRS::epsg_4326()),
            Crs::Vertical(odn_height()),
        ],
    )
    .expect("compound");
    let from_compound = Crs::Compound(compound)
        .extract_geographic_crs()
        .expect("horizontal");
    assert!(from_compound.is_equivalent_to(&wgs84));
    assert_eq!(from_compound.identifiers()[0].code(), "4326");
}

#[test]
fn crs_extract_vertical_crs() {
    assert!(Crs::Geographic(GeographicCRS::epsg_4326())
        .extract_vertical_crs()
        .is_none());
    let vertical = odn_height();
    let projected = ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631");
    let compound = CompoundCRS::create(
        "horizontal + vertical",
        vec![Crs::Projected(projected), Crs::Vertical(vertical.clone())],
    )
    .expect("compound")
    .with_identifier("codespace", "code");
    let found = Crs::Compound(compound)
        .extract_vertical_crs()
        .expect("vertical");
    assert!(found.is_equivalent_to(&vertical));
}

fn atlantic_pole() -> DerivedGeographicCRS {
    DerivedGeographicCRS::create(
        "WMO Atlantic Pole",
        GeographicCRS::epsg_4326(),
        Conversion::create(
            "Atlantic pole",
            "Pole rotation",
            vec![
                Parameter::new(
                    "Latitude of rotated pole",
                    ParameterValue::Measure(Measure::angle(52.0)),
                ),
                Parameter::new(
                    "Longitude of rotated pole",
                    ParameterValue::Measure(Measure::angle(-30.0)),
                ),
                Parameter::new(
                    "Axis rotation",
                    ParameterValue::Measure(Measure::angle(-25.0)),
                ),
            ],
        ),
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
}

#[test]
fn crs_derived_geographic_crs_basic() {
    let derived = atlantic_pole();
    assert!(derived.is_equivalent_to(&derived));
    assert!(!Crs::DerivedGeographic(derived.clone()).is_equivalent_to(&Crs::Geographic(
        derived.base_crs().clone()
    )));
    assert!(!Crs::Geographic(derived.base_crs().clone())
        .is_equivalent_to(&Crs::DerivedGeographic(derived)));
}

#[test]
fn crs_derived_geographic_crs_wkt2() {
    let derived = atlantic_pole();
    let wkt = Crs::DerivedGeographic(derived.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt2");
    assert_eq!(
        wkt,
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
    assert!(derived.is_equivalent_to(&derived));
    assert!(derived.clone().is_equivalent_to(&derived));
    assert!(!Crs::DerivedGeographic(derived).is_equivalent_to(&Crs::Vertical(odn_height())));
}

#[test]
fn crs_derived_geographic_crs_wkt2_2019() {
    let wkt = Crs::DerivedGeographic(atlantic_pole())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let exported = Crs::DerivedGeographic(atlantic_pole())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert!(exported.is_err());
}

#[test]
fn crs_derived_geographic_crs_to_proj() {
    let wkt = "\
GEODCRS[\"WMO Atlantic Pole\",
    BASEGEODCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0,
            ANGLEUNIT[\"degree\",0.0174532925199433]]],
    DERIVINGCONVERSION[\"unnamed\",
        METHOD[\"PROJ ob_tran o_proj=longlat\"],
        PARAMETER[\"o_lat_p\",52,
            UNIT[\"degree\",0.0174532925199433]],
        PARAMETER[\"o_lon_p\",-30,
            UNIT[\"degree\",0.0174532925199433]],
        PARAMETER[\"lon_0\",-25,
            UNIT[\"degree\",0.0174532925199433]]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north,
            ORDER[1],
            ANGLEUNIT[\"degree\",0.0174532925199433]],
        AXIS[\"longitude\",east,
            ORDER[2],
            ANGLEUNIT[\"degree\",0.0174532925199433]]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("parse")
        .crs()
        .expect("crs")
        .clone();
    let Crs::DerivedGeographic(derived) = &crs else {
        panic!("parsed CRS is not a derived geographic CRS");
    };
    assert_eq!(
        derived.export_to_proj_string().expect("proj"),
        "+proj=ob_tran +o_proj=longlat +o_lat_p=52 +o_lon_p=-30 +lon_0=-25 +datum=WGS84 +no_defs +type=crs"
    );
    let operation = CoordinateOperationFactory::create()
        .create_operation_proj_string(&Crs::Geographic(derived.base_crs().clone()), &crs)
        .expect("operation");
    assert_eq!(
        operation,
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=ob_tran +o_proj=longlat +o_lat_p=52 +o_lon_p=-30 +lon_0=-25 +ellps=WGS84 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn crs_derived_geographic_crs_with_affine_transform_to_proj() {
    let wkt = "\
GEODCRS[\"WGS 84 Translated\",
    BASEGEODCRS[\"WGS 84\",
        DATUM[\"World Geodetic System 1984\",
            ELLIPSOID[\"WGS 84\",6378137,298.257223563,
                LENGTHUNIT[\"metre\",1]]],
        PRIMEM[\"Greenwich\",0]],
    DERIVINGCONVERSION[\"Translation\",
        METHOD[\"Affine parametric transformation\",
            ID[\"EPSG\",9624]],
        PARAMETER[\"A0\",0.5,
            ANGLEUNIT[\"degree\",0.0174532925199433],
            ID[\"EPSG\",8623]],
        PARAMETER[\"A1\",1,
            SCALEUNIT[\"coefficient\",1],
            ID[\"EPSG\",8624]],
        PARAMETER[\"A2\",0,
            SCALEUNIT[\"coefficient\",1],
            ID[\"EPSG\",8625]],
        PARAMETER[\"B0\",2.5,
            ANGLEUNIT[\"degree\",0.0174532925199433],
            ID[\"EPSG\",8639]],
        PARAMETER[\"B1\",0,
            SCALEUNIT[\"coefficient\",1],
            ID[\"EPSG\",8640]],
        PARAMETER[\"B2\",1,
            SCALEUNIT[\"coefficient\",1],
            ID[\"EPSG\",8641]]],
    CS[ellipsoidal,2],
        AXIS[\"latitude\",north],
        AXIS[\"longitude\",east],
        ANGLEUNIT[\"degree\",0.0174532925199433]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("parse")
        .crs()
        .expect("crs")
        .clone();
    let Crs::DerivedGeographic(derived) = &crs else {
        panic!("parsed CRS is not a derived geographic CRS");
    };
    assert!(derived.deriving_conversion().validate_parameters().is_empty());
    let operation = CoordinateOperationFactory::create()
        .create_operation_proj_string(&Crs::Geographic(derived.base_crs().clone()), &crs)
        .expect("operation");
    assert_eq!(
        operation,
        "+proj=affine +xoff=0.5 +s11=1 +s12=0 +yoff=2.5 +s21=0 +s22=1"
    );
}

fn derived_geodetic() -> DerivedGeodeticCRS {
    DerivedGeodeticCRS::create(
        "Derived geodetic CRS",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create("Some conversion", "Some method", Vec::new()),
        CartesianCS::create_geocentric(UnitOfMeasure::metre()),
    )
}

#[test]
fn crs_derived_geodetic_crs_basic() {
    let derived = derived_geodetic();
    assert!(derived.is_equivalent_to(&derived));
    assert!(!Crs::DerivedGeodetic(derived.clone()).is_equivalent_to(&Crs::Geodetic(
        derived.base_crs().clone()
    )));
    assert!(!Crs::Geodetic(derived.base_crs().clone())
        .is_equivalent_to(&Crs::DerivedGeodetic(derived)));
}

#[test]
fn crs_derived_geodetic_crs_wkt2() {
    let derived = derived_geodetic();
    let wkt = Crs::DerivedGeodetic(derived.clone())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("wkt2");
    assert_eq!(
        wkt,
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
    assert!(derived.is_equivalent_to(&derived));
    assert!(derived.clone().is_equivalent_to(&derived));
    assert!(!Crs::DerivedGeodetic(derived).is_equivalent_to(&Crs::Vertical(odn_height())));
}

#[test]
fn crs_derived_geodetic_crs_wkt2_2019() {
    let wkt = Crs::DerivedGeodetic(derived_geodetic())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2-2019");
    assert_eq!(
        wkt,
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
    let exported = Crs::DerivedGeodetic(derived_geodetic())
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal));
    assert!(exported.is_err());
}
