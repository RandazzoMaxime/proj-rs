//! PROJ `test/unit/test_crs.cpp` checks that build or export a CRS.
//!
//! Each function is one `TEST()` name. WKT text is the expected string from that test.

use proj_rs::{
    create_crs_from_proj_string, create_from_user_input, AuthorityFactory, Axis, CartesianCS,
    Conversion, CoordinateSystem, Crs, DatabaseContext, Datum, DatumEnsemble, EllipsoidalCS,
    EllipsoidalHeightPolicy, Equivalence, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS,
    LinunitPolicy, OpProps, OutputAxisRule, PrimeMeridian, ProjectedCRS, UnitKind, UnitOfMeasure,
    VerticalReferenceFrame, WktConvention, WktFormatter, WktLines, WktParser, WktStrictness,
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

fn esri_wkt1_from_proj_string(definition: &str) -> String {
    let crs = proj_rs::create_crs_from_proj_string(definition).expect("PROJ string");
    assert!(matches!(crs, Crs::Projected(_)), "{definition}");
    let db = DatabaseContext::create().expect("proj.db");
    crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
        .expect("WKT1 ESRI")
}

#[test]
fn crs_mercator_1sp_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string(
            "+proj=merc +lon_0=110 +k=0.997 +x_0=3900000 +y_0=900000 +ellps=bessel +type=crs"
        ),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_Unknown_based_on_Bessel_1841_ellipsoid\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Mercator\"],PARAMETER[\"False_Easting\",3900000.0],PARAMETER[\"False_Northing\",900000.0],PARAMETER[\"Central_Meridian\",110.0],PARAMETER[\"Standard_Parallel_1\",4.45405154589748],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_plate_carree_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+title=my Plate carree +proj=eqc +type=crs"),
        "PROJCS[\"my_Plate_carree\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Plate_Carree\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_equidistant_cylindrical_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=eqc +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Equidistant_Cylindrical\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",0.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_hotine_oblique_mercator_azimuth_natural_origin_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=omerc +no_uoff +gamma=295 +alpha=295 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Hotine_Oblique_Mercator_Azimuth_Natural_Origin\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",-65.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_rectified_skew_orthomorphic_natural_origin_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=omerc +no_uoff +gamma=3 +alpha=2 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Rectified_Skew_Orthomorphic_Natural_Origin\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",2.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],PARAMETER[\"XY_Plane_Rotation\",3.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_hotine_oblique_mercator_azimuth_center_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=omerc +gamma=2 +alpha=2 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Hotine_Oblique_Mercator_Azimuth_Center\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",2.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_rectified_skew_orthomorphic_center_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=omerc +gamma=3 +alpha=2 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Rectified_Skew_Orthomorphic_Center\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Azimuth\",2.0],PARAMETER[\"Longitude_Of_Center\",0.0],PARAMETER[\"Latitude_Of_Center\",0.0],PARAMETER[\"XY_Plane_Rotation\",3.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_gauss_kruger_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+title=my Gauss Kruger +proj=tmerc +type=crs"),
        "PROJCS[\"my_Gauss_Kruger\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Gauss_Kruger\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Latitude_Of_Origin\",0.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_stereographic_north_pole_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=stere +lat_0=90 +lat_ts=70 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Stereographic_North_Pole\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",70.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_stereographic_south_pole_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=stere +lat_0=-90 +lat_ts=-70 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Stereographic_South_Pole\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",-70.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_krovak_north_orientated_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=krovak +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_Unknown_based_on_Bessel_1841_ellipsoid\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.2881397527778],PARAMETER[\"Longitude_Of_Center\",24.8333333333333],PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",-1.0],PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",90.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_krovak_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=krovak +axis=swu +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_Unknown_based_on_Bessel_1841_ellipsoid\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.2881397527778],PARAMETER[\"Longitude_Of_Center\",24.8333333333333],PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",1.0],PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",0.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_lcc_1sp_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=lcc +lat_1=1 +lat_0=1 +k=0.9 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",1.0],PARAMETER[\"Scale_Factor\",0.9],PARAMETER[\"Latitude_Of_Origin\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_lcc_2sp_as_wkt1_esri() {
    assert_eq!(
        esri_wkt1_from_proj_string("+proj=lcc +lat_0=1.5 +lat_1=1 +lat_2=2 +type=crs"),
        "PROJCS[\"unknown\",GEOGCS[\"GCS_unknown\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",0.0],PARAMETER[\"Standard_Parallel_1\",1.0],PARAMETER[\"Standard_Parallel_2\",2.0],PARAMETER[\"Latitude_Of_Origin\",1.5],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_esri_wkt1_to_esri_wkt1() {
    let in_wkt = "PROJCS[\"NAD_1983_CORS96_StatePlane_North_Carolina_FIPS_3200_Ft_US\",GEOGCS[\"GCS_NAD_1983_CORS96\",DATUM[\"D_NAD_1983_CORS96\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",2000000.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",-79.0],PARAMETER[\"Standard_Parallel_1\",34.33333333333334],PARAMETER[\"Standard_Parallel_2\",36.16666666666666],PARAMETER[\"Latitude_Of_Origin\",33.75],UNIT[\"Foot_US\",0.3048006096012192]]";
    let crs = WktParser::new()
        .create_from_wkt(in_wkt)
        .expect("wkt")
        .crs()
        .expect("projected")
        .clone();
    let db = DatabaseContext::create().expect("proj.db");
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
            .expect("WKT1 ESRI"),
        "PROJCS[\"NAD_1983_CORS96_StatePlane_North_Carolina_FIPS_3200_Ft_US\",GEOGCS[\"GCS_NAD_1983_CORS96\",DATUM[\"D_NAD_1983_CORS96\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",2000000.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",-79.0],PARAMETER[\"Standard_Parallel_1\",34.3333333333333],PARAMETER[\"Standard_Parallel_2\",36.1666666666667],PARAMETER[\"Latitude_Of_Origin\",33.75],UNIT[\"Foot_US\",0.304800609601219]]"
    );
}

fn odn_height_vertical_crs() -> Crs {
    WktParser::new()
        .create_from_wkt(
            "VERTCRS[\"ODN height\",VDATUM[\"Ordnance Datum Newlyn\",ID[\"EPSG\",5101]],CS[vertical,1],AXIS[\"gravity-related height (H)\",up,LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],ID[\"EPSG\",5701]]",
        )
        .expect("wkt")
        .crs()
        .expect("vertical")
        .clone()
}

#[test]
fn crs_vertical_crs_as_wkt2() {
    let crs = odn_height_vertical_crs();
    assert!(crs.is_equivalent_to(&crs));
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2"),
        "VERTCRS[\"ODN height\",\n    VDATUM[\"Ordnance Datum Newlyn\"],\n    CS[vertical,1],\n        AXIS[\"gravity-related height (H)\",up,\n            LENGTHUNIT[\"metre\",1]],\n    ID[\"EPSG\",5701]]"
    );
}

#[test]
fn crs_vertical_crs_as_wkt1_gdal() {
    let crs = odn_height_vertical_crs();
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("WKT1 GDAL"),
        "VERT_CS[\"ODN height\",\n    VERT_DATUM[\"Ordnance Datum Newlyn\",2005,\n        AUTHORITY[\"EPSG\",\"5101\"]],\n    UNIT[\"metre\",1,\n        AUTHORITY[\"EPSG\",\"9001\"]],\n    AXIS[\"Gravity-related height\",UP],\n    AUTHORITY[\"EPSG\",\"5701\"]]"
    );
}

#[test]
fn crs_vertical_crs_as_wkt1_esri() {
    let crs = odn_height_vertical_crs();
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri))
            .expect("WKT1 ESRI"),
        "VERTCS[\"ODN_height\",VDATUM[\"Ordnance_Datum_Newlyn\"],PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_vertical_crs_as_wkt1_esri_context() {
    let crs = odn_height_vertical_crs();
    let db = DatabaseContext::create().expect("proj.db");
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
            .expect("WKT1 ESRI"),
        "VERTCS[\"Newlyn\",VDATUM[\"Ordnance_Datum_Newlyn\"],PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]"
    );
}

#[test]
fn crs_vertical_crs_down_as_wkt1_esri() {
    let wkt = "VERTCS[\"Caspian\",VDATUM[\"Caspian_Sea\"],PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",-1.0],UNIT[\"Meter\",1.0]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("vertical")
        .clone();
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri))
            .expect("WKT1 ESRI"),
        wkt
    );
}

fn engineering_crs() -> Crs {
    WktParser::new()
        .create_from_wkt(
            "ENGCRS[\"Engineering CRS\",EDATUM[\"Engineering datum\"],CS[Cartesian,2],AXIS[\"(E)\",east,ORDER[1],LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]],AXIS[\"(N)\",north,ORDER[2],LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]]]]",
        )
        .expect("wkt")
        .crs()
        .expect("engineering")
        .clone()
}

#[test]
fn crs_engineering_crs_as_wkt2() {
    let crs = engineering_crs();
    assert!(crs.is_equivalent_to(&crs));
    assert!(!crs.is_equivalent_to(&Crs::Geographic(GeographicCRS::epsg_4326())));
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2"),
        "ENGCRS[\"Engineering CRS\",\n    EDATUM[\"Engineering datum\"],\n    CS[Cartesian,2],\n        AXIS[\"(E)\",east,\n            ORDER[1],\n            LENGTHUNIT[\"metre\",1,\n                ID[\"EPSG\",9001]]],\n        AXIS[\"(N)\",north,\n            ORDER[2],\n            LENGTHUNIT[\"metre\",1,\n                ID[\"EPSG\",9001]]]]"
    );
}

#[test]
fn crs_engineering_crs_as_wkt1_gdal() {
    assert_eq!(
        engineering_crs()
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
            .expect("WKT1 GDAL"),
        "LOCAL_CS[\"Engineering CRS\",\n    LOCAL_DATUM[\"Engineering datum\",32767],\n    UNIT[\"metre\",1,\n        AUTHORITY[\"EPSG\",\"9001\"]],\n    AXIS[\"Easting\",EAST],\n    AXIS[\"Northing\",NORTH]]"
    );
}

#[test]
fn crs_compound_crs_invalid() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let by_code = |code: &str| {
        factory
            .create_coordinate_reference_system(code)
            .expect(code)
    };
    assert!(proj_rs::CompoundCRS::create("", vec![]).is_err());
    assert!(proj_rs::CompoundCRS::create("", vec![by_code("32631")]).is_err());
    assert!(proj_rs::CompoundCRS::create("", vec![by_code("4326"), by_code("4326")]).is_err());
    assert!(
        proj_rs::CompoundCRS::create("", vec![by_code("4979"), odn_height_vertical_crs()]).is_err()
    );
}

#[test]
fn crs_compound_crs_no_name_provided() {
    let db = DatabaseContext::create().expect("proj.db");
    let utm31 = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("32631")
        .expect("32631");
    let crs =
        proj_rs::CompoundCRS::create("", vec![utm31, odn_height_vertical_crs()]).expect("compound");
    assert_eq!(crs.name(), "WGS 84 / UTM zone 31N + ODN height");
}

#[test]
fn crs_compound_crs_as_wkt2() {
    let wkt = "COMPOUNDCRS[\"horizontal + vertical\",\n    PROJCRS[\"WGS 84 / UTM zone 31N\",\n        BASEGEODCRS[\"WGS 84\",\n            DATUM[\"World Geodetic System 1984\",\n                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]]],\n        CONVERSION[\"UTM zone 31N\",\n            METHOD[\"Transverse Mercator\",\n                ID[\"EPSG\",9807]],\n            PARAMETER[\"Latitude of natural origin\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433],\n                ID[\"EPSG\",8801]],\n            PARAMETER[\"Longitude of natural origin\",3,\n                ANGLEUNIT[\"degree\",0.0174532925199433],\n                ID[\"EPSG\",8802]],\n            PARAMETER[\"Scale factor at natural origin\",0.9996,\n                SCALEUNIT[\"unity\",1],\n                ID[\"EPSG\",8805]],\n            PARAMETER[\"False easting\",500000,\n                LENGTHUNIT[\"metre\",1],\n                ID[\"EPSG\",8806]],\n            PARAMETER[\"False northing\",0,\n                LENGTHUNIT[\"metre\",1],\n                ID[\"EPSG\",8807]]],\n        CS[Cartesian,2],\n            AXIS[\"(E)\",east,\n                ORDER[1],\n                LENGTHUNIT[\"metre\",1]],\n            AXIS[\"(N)\",north,\n                ORDER[2],\n                LENGTHUNIT[\"metre\",1]]],\n    VERTCRS[\"ODN height\",\n        VDATUM[\"Ordnance Datum Newlyn\"],\n        CS[vertical,1],\n            AXIS[\"gravity-related height (H)\",up,\n                LENGTHUNIT[\"metre\",1]]],\n    ID[\"codespace\",\"code\"]]";
    let crs = WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .crs()
        .expect("compound")
        .clone();
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2"),
        wkt
    );
}
