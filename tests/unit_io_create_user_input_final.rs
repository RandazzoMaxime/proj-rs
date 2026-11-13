//! The blocks of PROJ 9.9.0 `test/unit/test_io.cpp`, `TEST(io, createFromUserInput)` that
//! `tests/unit_io_create_from_user_input.rs` and `tests/unit_io_create_object_blocks.rs` left
//! open: the non-conformant compounds `EPSG:4326+4326`, `EPSG:32631+4326` and `EPSG:4326+4258`
//! (lines 13867-13908 and 13910-13960), `NN2000:2018 height` (line 13767), the name searches
//! (lines 14120-14274), and `ID74`. Expected values are copied from the C++ test, or measured
//! with the PROJ 9.9.0 `projinfo` of the reference build where the C++ gives no value.
//!
//! Errors: PROJ's `ParsingException` (unknown name, unknown format) is any `Err` here, and PROJ's
//! `InvalidCompoundCRSException` is `FactoryError::Invalid`, the variant of a constructor that
//! rejects its arguments.

use proj_rs::iso19111::{
    create_from_user_input, create_object_from_user_input, Crs, DatabaseContext, FactoryError,
    Identifier, JsonFormat, UserObject,
};
use proj_rs::{WktConvention, WktFormatter};

/// The EPSG code among `identifiers`, as `getEPSGCode()` gives it.
fn epsg(identifiers: &[Identifier]) -> Option<&str> {
    identifiers
        .iter()
        .find(|identifier| identifier.code_space() == "EPSG")
        .map(Identifier::code)
}

/// `createFromUserInput(text, dbContext)`, which must succeed.
fn read(text: &str, db: &DatabaseContext) -> Crs {
    create_from_user_input(text, db).unwrap_or_else(|error| panic!("{text}: {error}"))
}

/// `WKTFormatter::create(WKT1_GDAL, dbContext)` export of `crs`.
fn wkt1_gdal(crs: &Crs, db: &DatabaseContext) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db.clone());
    crs.export_to_wkt(&mut formatter).expect("WKT1_GDAL")
}

/// The WKT lines of the C++ test, joined with the newlines of its string literals.
fn lines(parts: &[&str]) -> String {
    parts.join("\n")
}

/// Lines 13867-13904: `EPSG:4326+4326` is accepted, as a geographic 3D CRS named `WGS 84`.
#[test]
fn io_create_from_user_input_non_conformant_geographic_compound() {
    let db = DatabaseContext::create().expect("proj.db");
    match read("EPSG:4326+4326", &db) {
        Crs::Geographic(crs) => {
            assert_eq!(crs.name(), "WGS 84");
            assert_eq!(epsg(crs.identifiers()), Some("4979"));
            assert_eq!(crs.coordinate_system().axes().len(), 3);
            let wkt = lines(&[
                "COMPD_CS[\"WGS 84 + WGS 84\",",
                "    GEOGCS[\"WGS 84\",",
                "        DATUM[\"WGS_1984\",",
                "            SPHEROID[\"WGS 84\",6378137,298.257223563,",
                "                AUTHORITY[\"EPSG\",\"7030\"]],",
                "            AUTHORITY[\"EPSG\",\"6326\"]],",
                "        PRIMEM[\"Greenwich\",0,",
                "            AUTHORITY[\"EPSG\",\"8901\"]],",
                "        UNIT[\"degree\",0.0174532925199433,",
                "            AUTHORITY[\"EPSG\",\"9122\"]],",
                "        AUTHORITY[\"EPSG\",\"4326\"]],",
                "    GEOGCS[\"WGS 84\",",
                "        DATUM[\"WGS_1984\",",
                "            SPHEROID[\"WGS 84\",6378137,298.257223563,",
                "                AUTHORITY[\"EPSG\",\"7030\"]],",
                "            AUTHORITY[\"EPSG\",\"6326\"]],",
                "        PRIMEM[\"Greenwich\",0,",
                "            AUTHORITY[\"EPSG\",\"8901\"]],",
                "        UNIT[\"degree\",0.0174532925199433,",
                "            AUTHORITY[\"EPSG\",\"9122\"]],",
                "        AUTHORITY[\"EPSG\",\"4326\"]]]",
            ]);
            assert_eq!(wkt1_gdal(&Crs::Geographic(crs), &db), wkt);
        }
        other => panic!("not a geographic CRS: {}", other.name()),
    }
}

/// Line 13907-13908: `EPSG:4326+4258` is refused, the vertical part is not the horizontal one.
#[test]
fn io_create_from_user_input_non_consistent_compound() {
    let db = DatabaseContext::create().expect("proj.db");
    match create_from_user_input("EPSG:4326+4258", &db) {
        Err(FactoryError::Invalid(message)) => assert_eq!(
            message,
            "The 'vertical' geographic CRS is not equivalent to the geographic CRS of the horizontal part"
        ),
        other => panic!("expected InvalidCompoundCRSException, got {other:?}"),
    }
}

/// Lines 13910-13960: `EPSG:32631+4326` is accepted, as the projected CRS with a 3D base.
#[test]
fn io_create_from_user_input_non_conformant_projected_compound() {
    let db = DatabaseContext::create().expect("proj.db");
    match read("EPSG:32631+4326", &db) {
        Crs::Projected(crs) => {
            assert_eq!(crs.name(), "WGS 84 / UTM zone 31N");
            assert_eq!(epsg(crs.base_crs().identifiers()), Some("4979"));
            assert_eq!(crs.base_crs().coordinate_system().axes().len(), 3);
            assert_eq!(crs.coordinate_system().axes().len(), 3);
            let wkt = lines(&[
                "COMPD_CS[\"WGS 84 / UTM zone 31N + WGS 84\",",
                "    PROJCS[\"WGS 84 / UTM zone 31N\",",
                "        GEOGCS[\"WGS 84\",",
                "            DATUM[\"WGS_1984\",",
                "                SPHEROID[\"WGS 84\",6378137,298.257223563,",
                "                    AUTHORITY[\"EPSG\",\"7030\"]],",
                "                AUTHORITY[\"EPSG\",\"6326\"]],",
                "            PRIMEM[\"Greenwich\",0,",
                "                AUTHORITY[\"EPSG\",\"8901\"]],",
                "            UNIT[\"degree\",0.0174532925199433,",
                "                AUTHORITY[\"EPSG\",\"9122\"]],",
                "            AUTHORITY[\"EPSG\",\"4326\"]],",
                "        PROJECTION[\"Transverse_Mercator\"],",
                "        PARAMETER[\"latitude_of_origin\",0],",
                "        PARAMETER[\"central_meridian\",3],",
                "        PARAMETER[\"scale_factor\",0.9996],",
                "        PARAMETER[\"false_easting\",500000],",
                "        PARAMETER[\"false_northing\",0],",
                "        UNIT[\"metre\",1,",
                "            AUTHORITY[\"EPSG\",\"9001\"]],",
                "        AXIS[\"Easting\",EAST],",
                "        AXIS[\"Northing\",NORTH],",
                "        AUTHORITY[\"EPSG\",\"32631\"]],",
                "    GEOGCS[\"WGS 84\",",
                "        DATUM[\"WGS_1984\",",
                "            SPHEROID[\"WGS 84\",6378137,298.257223563,",
                "                AUTHORITY[\"EPSG\",\"7030\"]],",
                "            AUTHORITY[\"EPSG\",\"6326\"]],",
                "        PRIMEM[\"Greenwich\",0,",
                "            AUTHORITY[\"EPSG\",\"8901\"]],",
                "        UNIT[\"degree\",0.0174532925199433,",
                "            AUTHORITY[\"EPSG\",\"9122\"]],",
                "        AUTHORITY[\"EPSG\",\"4326\"]]]",
            ]);
            assert_eq!(wkt1_gdal(&Crs::Projected(crs), &db), wkt);
        }
        other => panic!("not a projected CRS: {}", other.name()),
    }
}

/// Lines 13767 and 14137: `NN2000:2018 height` (a name with a colon, and no authority token)
/// and `ID74` are read by name. Their codes are from the reference `projinfo`.
#[test]
fn io_create_from_user_input_name_with_colon_and_id74() {
    let db = DatabaseContext::create().expect("proj.db");
    match read("NN2000:2018 height", &db) {
        Crs::Vertical(crs) => assert_eq!(epsg(crs.identifiers()), Some("5941")),
        other => panic!("not a vertical CRS: {}", other.name()),
    }
    match read("ID74", &db) {
        Crs::Geographic(crs) => {
            assert_eq!(crs.name(), "ID74");
            assert_eq!(epsg(crs.identifiers()), Some("4238"));
        }
        other => panic!("not a geographic CRS: {}", other.name()),
    }
}

/// Lines 14120-14274: the name searches of `createFromUserInput`.
#[test]
fn io_create_from_user_input_names() {
    let db = DatabaseContext::create().expect("proj.db");

    // Official name, then PROJ alias (lines 14123-14136).
    assert!(matches!(read("WGS 84", &db), Crs::Geographic(_)));
    assert!(matches!(read("WGS84", &db), Crs::Geographic(_)));
    // "UTM zone 31N" is the name of the conversion EPSG:16031, found by the exact pass on the
    // other object kinds (`io.cpp` 8260-8300): an object, not a CRS. `projinfo` reads it as a
    // conversion.
    assert!(create_from_user_input("UTM zone 31N", &db).is_err());
    assert!(matches!(
        create_object_from_user_input("UTM zone 31N", &db),
        Ok(UserObject::Operation(_))
    ));
    assert!(create_from_user_input("UTM zone 31", &db).is_err());
    assert!(create_from_user_input("WGS84 UTM zone 31N", &db).is_ok());

    // Approximate matches of vertical CRS (lines 14141-14160).
    match read("NGF IGN69 height", &db) {
        Crs::Vertical(crs) => assert_eq!(crs.name(), "NGF-IGN69 height"),
        other => panic!("not a vertical CRS: {}", other.name()),
    }
    match read("NGF IGN1969", &db) {
        Crs::Vertical(crs) => assert_eq!(crs.name(), "NGF-IGN 1969"),
        other => panic!("not a vertical CRS: {}", other.name()),
    }
    match read("NGF IGN69", &db) {
        Crs::Vertical(crs) => assert_eq!(crs.name(), "NGF-IGN 1969"),
        other => panic!("not a vertical CRS: {}", other.name()),
    }

    // Compound CRS from two names (lines 14166-14190).
    match read("WGS 84 + EGM96 height", &db) {
        Crs::Compound(crs) => assert_eq!(crs.name(), "WGS 84 + EGM96 height"),
        other => panic!("not a compound CRS: {}", other.name()),
    }
    match read("WGS 84 + EGM96", &db) {
        Crs::Compound(crs) => assert_eq!(crs.name(), "WGS 84 + EGM96 height"),
        other => panic!("not a compound CRS: {}", other.name()),
    }
    match read("WGS84 + NAVD88 height", &db) {
        Crs::Compound(crs) => assert_eq!(crs.name(), "WGS 84 + NAVD88 height"),
        other => panic!("not a compound CRS: {}", other.name()),
    }
    match read("WGS 84 / World Mercator + EGM2008 height", &db) {
        Crs::Compound(crs) => assert_eq!(crs.identifiers().len(), 1),
        other => panic!("not a compound CRS: {}", other.name()),
    }
    assert!(create_from_user_input("WGS 84 + foobar", &db).is_err());
    assert!(create_from_user_input("foobar + EGM96 height", &db).is_err());

    // A datum ensemble is an object, not a CRS (lines 14203-14205).
    assert!(create_from_user_input("World Geodetic System 1984 ensemble", &db).is_err());
    match create_object_from_user_input("World Geodetic System 1984 ensemble", &db) {
        Ok(UserObject::Ensemble(ensemble)) => assert_eq!(ensemble.identifiers().len(), 1),
        other => panic!("not a datum ensemble: {other:?}"),
    }

    // Short texts and near misses match nothing (lines 14211-14217).
    assert!(create_from_user_input("foo", &db).is_err());
    assert!(create_from_user_input("omerc", &db).is_err());
    assert!(create_from_user_input("WGS84 PseudoMercator", &db).is_ok());

    // Approximate match with "north" for "N" and no "zone" (line 14267).
    match read("WGS 84 UTM 31 north", &db) {
        Crs::Projected(crs) => assert_eq!(crs.name(), "WGS 84 / UTM zone 31N"),
        other => panic!("not a projected CRS: {}", other.name()),
    }

    // Should not match WGS84 or IGM85 (line 14274).
    assert!(create_from_user_input("WGS 85", &db).is_err());
}

/// PROJJSON of the object kinds `createFromUserInput` returns besides a CRS. The members are
/// those of the reference `projinfo -o PROJJSON` output of EPSG:4326 (its `datum_ensemble`,
/// `ellipsoid` and `prime_meridian`), and of EPSG:5703 (its `datum`). Each object has the
/// `type` of PROJ's `_exportToJSON`, and the `id` of its EPSG code.
#[test]
fn io_create_from_user_input_projjson_of_objects() {
    let db = DatabaseContext::create().expect("proj.db");
    let json = |text: &str| {
        create_object_from_user_input(text, &db)
            .and_then(|object| object.export_to_json(&db, &JsonFormat::default()))
            .unwrap_or_else(|error| panic!("{text}: {error}"))
    };

    let ensemble = json("urn:ogc:def:ensemble:EPSG::6326");
    assert!(
        ensemble.contains("\"type\": \"DatumEnsemble\""),
        "{ensemble}"
    );
    assert!(
        ensemble.contains("\"name\": \"World Geodetic System 1984 ensemble\""),
        "{ensemble}"
    );
    assert!(ensemble.contains("\"name\": \"World Geodetic System 1984 (G2296)\""));
    assert!(
        ensemble.contains("\"ellipsoid\": {\n    \"name\": \"WGS 84\","),
        "{ensemble}"
    );
    assert!(ensemble.contains("\"code\": 6326"), "{ensemble}");

    let meridian = json("urn:ogc:def:meridian:EPSG::8901");
    assert!(
        meridian.contains("\"type\": \"PrimeMeridian\""),
        "{meridian}"
    );
    assert!(
        meridian.contains("\"name\": \"Greenwich\",\n  \"longitude\": 0,"),
        "{meridian}"
    );
    assert!(meridian.contains("\"code\": 8901"), "{meridian}");

    let ellipsoid = json("urn:ogc:def:ellipsoid:EPSG::7030");
    assert!(ellipsoid.contains("\"type\": \"Ellipsoid\""), "{ellipsoid}");
    assert!(
        ellipsoid.contains(
            "\"name\": \"WGS 84\",\n  \"semi_major_axis\": 6378137,\n  \"inverse_flattening\": 298.257223563,"
        ),
        "{ellipsoid}"
    );

    let vertical = json("urn:ogc:def:datum:EPSG::5103");
    assert!(
        vertical.contains("\"type\": \"VerticalReferenceFrame\""),
        "{vertical}"
    );
    assert!(
        vertical.contains("\"name\": \"North American Vertical Datum 1988\""),
        "{vertical}"
    );
    assert!(vertical.contains("\"code\": 5103"), "{vertical}");

    let operation = json("urn:ogc:def:coordinateOperation:EPSG::1671");
    assert!(
        operation.contains("\"type\": \"Transformation\""),
        "{operation}"
    );
    assert!(
        operation.contains("\"name\": \"ETRS89-FRA [RGF93 v1] to WGS 84 (1)\""),
        "{operation}"
    );

    let concatenated = json(
        "urn:ogc:def:coordinateOperation,coordinateOperation:EPSG::3895,coordinateOperation:EPSG::1618",
    );
    assert!(
        concatenated.contains("\"type\": \"ConcatenatedOperation\""),
        "{concatenated}"
    );
    assert!(
        concatenated.contains("\"name\": \"MGI (Ferro) to MGI (1) + MGI to WGS 84 (3)\""),
        "{concatenated}"
    );

    let metadata = json("ITRF2014@2025.1");
    assert!(
        metadata.contains("\"type\": \"CoordinateMetadata\""),
        "{metadata}"
    );
    assert!(
        metadata.contains("\"crs\": {\n    \"type\": \"GeographicCRS\""),
        "{metadata}"
    );
    assert!(metadata.contains("\"name\": \"ITRF2014\""), "{metadata}");
    assert!(
        metadata.contains("\"coordinateEpoch\": 2025.1\n"),
        "{metadata}"
    );
}
