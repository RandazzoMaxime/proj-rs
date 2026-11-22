//! PROJ 9.9.0's `createFromUserInput` reads any PROJJSON object `JSONParser::create` knows, not
//! only a CRS: `proj_create` of an ellipsoid, a prime meridian or a coordinate system gives that
//! object. The C test `capi/tests/projjson_objects.c` compares twelve such objects to PROJ.

use proj_rs::iso19111::{
    coordinate_system_from_json, create_from_user_input, create_user_object, DatabaseContext,
    InitRules, UserObject,
};

const ELLIPSOID: &str = r#"{"type": "Ellipsoid", "name": "WGS 84", "semi_major_axis": 6378137, "inverse_flattening": 298.257223563}"#;
const PRIME_MERIDIAN: &str = r#"{"type": "PrimeMeridian", "name": "Paris", "longitude": {"value": 2.33722917, "unit": "degree"}}"#;
const ELLIPSOIDAL_CS: &str = r#"{"type": "CoordinateSystem", "subtype": "ellipsoidal", "axis": [{"name": "Geodetic latitude", "abbreviation": "Lat", "direction": "north", "unit": "degree"}, {"name": "Geodetic longitude", "abbreviation": "Lon", "direction": "east", "unit": "degree"}]}"#;

fn user_object(text: &str) -> UserObject {
    let db = DatabaseContext::create().expect("proj.db");
    create_user_object(text, Some(&db), InitRules::Iso19111).expect(text)
}

#[test]
fn an_ellipsoid_and_a_prime_meridian_are_read() {
    assert!(matches!(user_object(ELLIPSOID), UserObject::Ellipsoid(e) if e.name() == "WGS 84"));
    assert!(
        matches!(user_object(PRIME_MERIDIAN), UserObject::PrimeMeridian(m) if m.name() == "Paris")
    );
}

#[test]
fn a_non_crs_object_is_still_not_a_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    assert!(create_from_user_input(ELLIPSOID, &db).is_err());
}

#[test]
fn a_coordinate_system_is_read_on_its_own() {
    let system = coordinate_system_from_json(ELLIPSOIDAL_CS).expect("coordinate system");
    assert_eq!(system.axes().len(), 2);
    assert!(coordinate_system_from_json(ELLIPSOID).is_err());
}
