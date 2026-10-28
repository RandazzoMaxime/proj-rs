//! PROJ `test/unit/test_crs.cpp` checks that build or export a CRS.
//!
//! Each function is one `TEST()` name. WKT text is the expected string from that test.

use proj_rs::{
    Axis, CartesianCS, Crs, DatabaseContext, Datum, DatumEnsemble, EllipsoidalCS, Equivalence,
    GeodeticReferenceFrame, GeographicCRS, OutputAxisRule, UnitOfMeasure, VerticalReferenceFrame,
    WktConvention, WktFormatter, WktLines, WktStrictness,
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
