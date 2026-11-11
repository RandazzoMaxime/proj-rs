//! PROJ 9.9.0 `test/unit/test_io.cpp`, `TEST(io, createFromUserInput)` (lines 13718-14275).
//!
//! One block per C++ block that returns a CRS, in the C++ order. The blocks left out are listed,
//! with their missing piece, in `docs/PORTING-io-create-from-user-input.md`: those returning an
//! operation, a datum, an ensemble, an ellipsoid, a prime meridian or a coordinate metadata, the
//! `@` epoch forms, the non-conformant `EPSG:4326+4326`, `EPSG:4326+4258` and `EPSG:32631+4326`,
//! the names not listed here, and `NN2000:2018 height` and `ID74`.

use proj_rs::iso19111::{
    create_from_user_input, create_from_user_input_with_optional_database, Crs, DatabaseContext,
    FactoryError, Identifier, InitRules,
};

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

/// `createFromUserInput(text, nullptr)`: no database.
fn read_without_database(text: &str) -> Result<Crs, FactoryError> {
    create_from_user_input_with_optional_database(text, None, InitRules::Iso19111)
}

/// `TEST(io, createFromUserInput)`
#[test]
fn io_create_from_user_input() {
    let db = DatabaseContext::create().expect("proj.db");

    // Without a database, only the texts that need none are read (lines 13720-13725).
    assert!(read_without_database("foo").is_err());
    assert!(read_without_database("GEOGCRS").is_err());
    assert!(read_without_database("+proj=unhandled +type=crs").is_err());
    assert!(read_without_database("EPSG:4326").is_err());
    assert!(read_without_database("ESRI:103668+EPSG:5703").is_err());

    // Unknown object type, and unknown authorities or codes (lines 13727-13745).
    assert!(create_from_user_input("urn:ogc:def:unhandled:EPSG::4326", &db).is_err());
    assert!(matches!(
        create_from_user_input("urn:ogc:def:crs:non_existing_auth::4326", &db),
        Err(FactoryError::NotFound { .. })
    ));
    assert!(create_from_user_input(
        "urn:ogc:def:crs,crs:EPSG::2393,unhandled_type:EPSG::5717",
        &db
    )
    .is_err());
    assert!(matches!(
        create_from_user_input(
            "urn:ogc:def:crs,crs:EPSG::2393,crs:EPSG::unexisting_code",
            &db
        ),
        Err(FactoryError::NotFound { .. })
    ));
    assert!(create_from_user_input(
        "urn:ogc:def:crs,crs:EPSG::2393::extra_element,crs:EPSG::EPSG",
        &db
    )
    .is_err());

    // Accepted without a database and with one (lines 13764-13767).
    assert!(read_without_database("+proj=longlat").is_ok());
    assert!(create_from_user_input("EPSG:4326", &db).is_ok());
    assert!(create_from_user_input("epsg:4326", &db).is_ok());

    // URN CRS forms (lines 13768-13773).
    assert!(create_from_user_input("urn:ogc:def:crs:EPSG::4326", &db).is_ok());
    assert!(create_from_user_input("urn:ogc:def:crs:EPSG:10:4326", &db).is_ok());
    assert!(read_without_database("urn:ogc:def:crs:EPSG::4326").is_err());

    // IAU authority, with and without its version (lines 13785-13797).
    assert!(create_from_user_input("IAU:1000", &db).is_ok());
    assert!(create_from_user_input("IAU_2015:1000", &db).is_ok());
    assert!(create_from_user_input("urn:ogc:def:crs:IAU::1000", &db).is_ok());
    assert!(create_from_user_input("urn:ogc:def:crs:IAU_2015::1000", &db).is_ok());
    assert!(create_from_user_input("urn:ogc:def:crs:IAU:2015:1000", &db).is_ok());
    assert!(read_without_database("urn:ogc:def:crs:IAU_2015::xxxx").is_err());
    assert!(read_without_database("urn:ogc:def:crs:IAU:xxxx:1000").is_err());

    // Found as srsName in some GMLs (lines 13800-13801).
    assert!(create_from_user_input("URN:OGC:DEF:CRS:OGC:1.3:CRS84", &db).is_ok());

    // Legacy formulations (lines 13804-13816).
    assert!(create_from_user_input("urn:x-ogc:def:crs:EPSG::4326", &db).is_ok());
    assert!(create_from_user_input("urn:opengis:def:crs:EPSG::4326", &db).is_ok());
    assert!(create_from_user_input("urn:opengis:crs:EPSG::4326", &db).is_ok());
    assert!(create_from_user_input("urn:x-ogc:def:crs:EPSG:4326", &db).is_ok());
    assert!(read_without_database("urn:opengis:crs:EPSG::4326").is_err());
    assert!(create_from_user_input("urn:opengis:unhandled:EPSG::4326", &db).is_err());

    // AUTH:CODE+CODE and AUTH:CODE+AUTH:CODE (lines 13818-13843).
    for text in ["EPSG:2393+5717", "EPSG:2393+EPSG:5717"] {
        match read(text, &db) {
            Crs::Compound(compound) => {
                assert_eq!(
                    compound.name(),
                    "KKJ / Finland Uniform Coordinate System + N60 height",
                    "{text}"
                );
            }
            other => panic!("{text}: not a compound CRS: {}", other.name()),
        }
    }
    match read("ESRI:103668+EPSG:5703", &db) {
        Crs::Compound(compound) => assert_eq!(
            compound.name(),
            "NAD_1983_HARN_Adj_MN_Ramsey_Meters + NAVD88 height"
        ),
        other => panic!("not a compound CRS: {}", other.name()),
    }
    assert!(matches!(
        create_from_user_input("ESRI:42+EPSG:5703", &db),
        Err(FactoryError::NotFound { .. })
    ));
    assert!(matches!(
        create_from_user_input("ESRI:103668+EPSG:999999", &db),
        Err(FactoryError::NotFound { .. })
    ));

    // URN compound CRS (lines 13845-13852).
    match read("urn:ogc:def:crs,crs:EPSG::2393,crs:EPSG::5717", &db) {
        Crs::Compound(compound) => assert_eq!(
            compound.name(),
            "KKJ / Finland Uniform Coordinate System + N60 height"
        ),
        other => panic!("not a compound CRS: {}", other.name()),
    }

    // URN projected CRS with a 3D base, through `cs:PROJ::ENh` (lines 13854-13865).
    match read(
        "urn:ogc:def:crs,crs:EPSG::4979,cs:PROJ::ENh,coordinateOperation:EPSG::16031",
        &db,
    ) {
        Crs::Projected(projected) => {
            assert_eq!(projected.name(), "WGS 84 (3D) / UTM zone 31N");
            assert_eq!(epsg(projected.base_crs().identifiers()), Some("4979"));
            assert_eq!(projected.coordinate_system().axes().len(), 3);
            assert_eq!(
                epsg(projected.deriving_conversion().identifiers()),
                Some("16031")
            );
        }
        other => panic!("not a projected CRS: {}", other.name()),
    }

    // Malformed URN combinations of CRS, CS and operation (lines 13968-13982).
    for text in [
        "urn:ogc:def:crs,crs:EPSG::4979,cs:PROJ::ENh,coordinateOperation:EPSG::1024",
        "urn:ogc:def:crs,crs:,cs:PROJ::ENh,coordinateOperation:EPSG::16031",
        "urn:ogc:def:crs,crs:EPSG::4979,cs:,coordinateOperation:EPSG::16031",
        "urn:ogc:def:crs,crs:EPSG::4979,cs:PROJ::ENh,coordinateOperation:",
    ] {
        assert!(create_from_user_input(text, &db).is_err(), "{text}");
    }

    // Derived CRS of a geocentric base (lines 13986-13995).
    match read(
        "urn:ogc:def:crs,crs:EPSG::4978,cs:EPSG::6500,coordinateOperation:EPSG::16031",
        &db,
    ) {
        Crs::Derived(derived) => {
            assert_eq!(epsg(derived.base_crs().identifiers()), Some("4978"));
            assert_eq!(
                epsg(derived.coordinate_system().identifiers()),
                Some("6500")
            );
            assert_eq!(
                epsg(derived.deriving_conversion().identifiers()),
                Some("16031")
            );
        }
        other => panic!("not a derived CRS: {}", other.name()),
    }

    // Derived geographic CRS (lines 13997-14006).
    match read(
        "urn:ogc:def:crs,crs:EPSG::4979,cs:EPSG::6423,coordinateOperation:EPSG::16031",
        &db,
    ) {
        Crs::Derived(derived) => {
            assert_eq!(epsg(derived.base_crs().identifiers()), Some("4979"));
            assert_eq!(
                epsg(derived.coordinate_system().identifiers()),
                Some("6423")
            );
            assert_eq!(
                epsg(derived.deriving_conversion().identifiers()),
                Some("16031")
            );
        }
        other => panic!("not a derived CRS: {}", other.name()),
    }

    // Derived projected CRS (lines 14009-14018).
    match read(
        "urn:ogc:def:crs,crs:EPSG::32631,cs:EPSG::4400,coordinateOperation:EPSG::16031",
        &db,
    ) {
        Crs::DerivedProjected(derived) => {
            assert_eq!(epsg(derived.base_crs().identifiers()), Some("32631"));
            assert_eq!(
                epsg(derived.coordinate_system().identifiers()),
                Some("4400")
            );
            assert_eq!(
                epsg(derived.deriving_conversion().identifiers()),
                Some("16031")
            );
        }
        other => panic!("not a derived projected CRS: {}", other.name()),
    }

    // Derived vertical CRS: the unit and the height/depth reversal rename the base
    // (lines 14021-14093).
    for (text, expected) in [
        (
            "urn:ogc:def:crs,crs:EPSG::5703,cs:EPSG::1030,coordinateOperation:EPSG::7813",
            "NAVD88 height (ft)",
        ),
        (
            "urn:ogc:def:crs,crs:EPSG::5703,cs:EPSG::6497,coordinateOperation:EPSG::7813",
            "NAVD88 height (ftUS)",
        ),
        (
            "urn:ogc:def:crs,crs:EPSG::6360,cs:EPSG::6499,coordinateOperation:EPSG::7813",
            "NAVD88 height",
        ),
        (
            "urn:ogc:def:crs,crs:EPSG::5703,cs:EPSG::6498,coordinateOperation:EPSG::7812",
            "NAVD88 depth",
        ),
        (
            "urn:ogc:def:crs,crs:EPSG::6360,cs:EPSG::1043,coordinateOperation:EPSG::7812",
            "NAVD88 depth (ftUS)",
        ),
        (
            "urn:ogc:def:crs,crs:EPSG::6358,cs:EPSG::6497,coordinateOperation:EPSG::7812",
            "NAVD88 height (ftUS)",
        ),
    ] {
        match read(text, &db) {
            Crs::Derived(derived) => assert_eq!(derived.name(), expected, "{text}"),
            other => panic!("{text}: not a derived CRS: {}", other.name()),
        }
    }

    // The WKT of a geographic CRS needs no database (lines 14105-14117).
    assert!(read_without_database(
        " \n\t\rGEOGCRS[\"WGS 84\",\n\
         \x20   DATUM[\"World Geodetic System 1984\",\n\
         \x20       ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
         \x20   CS[ellipsoidal,3],\n\
         \x20       AXIS[\"latitude\",north,\n\
         \x20           UNIT[\"degree\",0.0174532925199433]],\n\
         \x20       AXIS[\"longitude\",east,\n\
         \x20           UNIT[\"degree\",0.0174532925199433]],\n\
         \x20       AXIS[\"ellipsoidal height\",up,\n\
         \x20           UNIT[\"metre\",1]],\n\
         \x20   ID[\"EPSG\",4979]]"
    )
    .is_ok());

    // Names: an official name, and texts that match no CRS (lines 14120-14214).
    assert!(create_from_user_input("foobar", &db).is_err());
    assert!(matches!(read("WGS 84", &db), Crs::Geographic(_)));
    assert!(create_from_user_input("foo", &db).is_err());
    assert!(create_from_user_input("omerc", &db).is_err());
    assert!(create_from_user_input("WGS 84 + foobar", &db).is_err());
    assert!(create_from_user_input("foobar + EGM96 height", &db).is_err());
}
