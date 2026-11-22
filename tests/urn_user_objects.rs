//! The OGC URN of an object that is not a CRS, read by `create_user_object` (the C API's
//! `proj_create`), as PROJ 9.9.0's `createFromURNPart` does: `urn:ogc:def:TYPE:AUTH:VERSION:CODE`
//! with the types `coordinateOperation`, `datum`, `ensemble`, `ellipsoid`, `meridian` and
//! `coordinateMetadata`. Expected names: `proj_get_name(proj_create(urn))` of PROJ 9.9.0.

use proj_rs::iso19111::{create_user_object, DatabaseContext, InitRules, UserObject};

fn read(urn: &str) -> UserObject {
    let db = DatabaseContext::create().expect("proj.db");
    create_user_object(urn, Some(&db), InitRules::Iso19111).unwrap_or_else(|e| panic!("{urn}: {e}"))
}

#[test]
fn each_urn_type_gives_its_object() {
    let cases = [
        (
            "urn:ogc:def:ensemble:EPSG::6326",
            "World Geodetic System 1984 ensemble",
        ),
        (
            "urn:ogc:def:coordinateOperation:EPSG::1671",
            "ETRS89-FRA [RGF93 v1] to WGS 84 (1)",
        ),
        (
            "urn:ogc:def:coordinateOperation:EPSG::16031",
            "UTM zone 31N",
        ),
        (
            "urn:ogc:def:datum:EPSG::6258",
            "European Terrestrial Reference System 1989",
        ),
        ("urn:ogc:def:datum:EPSG::5101", "Ordnance Datum Newlyn"),
        ("urn:ogc:def:ellipsoid:EPSG::7030", "WGS 84"),
        ("urn:ogc:def:meridian:EPSG::8903", "Paris"),
    ];
    for (urn, name) in cases {
        let object = read(urn);
        let got = match &object {
            UserObject::Ensemble(ensemble) => ensemble.name(),
            UserObject::Operation(operation) => operation.name(),
            UserObject::GeodeticDatum(datum) => datum.name(),
            UserObject::VerticalDatum(datum) => datum.name(),
            UserObject::Ellipsoid(ellipsoid) => ellipsoid.name(),
            UserObject::PrimeMeridian(meridian) => meridian.name(),
            _ => panic!("{urn}: unexpected kind"),
        };
        assert_eq!(got, name, "{urn}");
    }
}

#[test]
fn a_coordinate_metadata_urn_and_an_unknown_type() {
    assert!(matches!(
        read("urn:ogc:def:coordinateMetadata:NRCAN::NAD83_CSRS_1997_MTM10_HT2_1997"),
        UserObject::CoordinateMetadata(_)
    ));
    let db = DatabaseContext::create().expect("proj.db");
    assert!(create_user_object("urn:ogc:def:foo:EPSG::1", Some(&db), InitRules::Iso19111).is_err());
}
