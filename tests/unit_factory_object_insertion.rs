//! PROJ 9.9.0 `test/unit/test_factory.cpp`, `TEST(factory, objectInsertion)` (l. 4411-5252).
//!
//! The C++ test is one function of about 840 lines, with one block per case. Each block
//! becomes one Rust test here, in the same order, with the block's C++ line range in its
//! comment. The expected SQL is the C++ text. The identities and counts are the C++ ones.
//! Where the Rust model has no public constructor for a part of the C++ object, the object
//! is built another way, and the test says so; see `docs/PORTING-object-insertion.md`.

use proj_rs::iso19111::{
    create_crs_from_proj_string, suggests_code_for, AuthorityFactory, AuthorityObject, BoundCRS,
    BoundingBox, CodeForm, ComponentCodes, Crs, DatabaseContext, Ellipsoid, Equivalence,
    GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, InsertSession, Measure, ObjectDomain,
    PrimeMeridian, ProjectedCRS, UnitKind, UnitOfMeasure, VerticalCRS, VerticalCS,
    VerticalReferenceFrame, WktParser,
};

/// The default `allowedAuthorities` of `getInsertStatementsFor`.
fn default_allowed() -> Vec<String> {
    vec!["EPSG".to_owned(), "PROJ".to_owned()]
}

fn only_hobu() -> Vec<String> {
    vec!["HOBU".to_owned()]
}

fn db() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

/// `getInsertStatementsFor(object, auth, code, numericCode, allowed)` in `session`.
fn statements(
    session: &InsertSession,
    object: &AuthorityObject,
    auth: &str,
    code: &str,
    components: ComponentCodes,
    allowed: &[String],
) -> Vec<String> {
    session
        .statements_for(object, auth, code, components, allowed)
        .expect("getInsertStatementsFor")
}

/// `crs->identify(AuthorityFactory::create(ctxt, ""))` has one result, in `HOBU`, that is
/// equivalent to `crs` with confidence 100.
fn assert_identified_in_hobu(db: &DatabaseContext, crs: &Crs) {
    let factory = AuthorityFactory::create(db, "");
    let identified = crs.identify_with(Some(&factory));
    assert_eq!(identified.len(), 1, "identify: {identified:?}");
    let (found, confidence) = &identified[0];
    assert_eq!(found.identifiers()[0].code_space(), "HOBU");
    assert!(found.is_equivalent_to_with(crs, Equivalence::Equivalent));
    assert_eq!(*confidence, 100);
}

fn epsg(db: &DatabaseContext) -> AuthorityFactory {
    AuthorityFactory::create(db, "EPSG")
}

fn hobu(db: &DatabaseContext) -> AuthorityFactory {
    AuthorityFactory::create(db, "HOBU")
}

/// `GeographicCRS::create(props("my EPSG:4326"), EPSG_4326->datum(), ..datumEnsemble(),
/// ..coordinateSystem())`.
fn my_epsg_4326(db: &DatabaseContext) -> GeographicCRS {
    let base = epsg(db).create_geographic_crs("4326").expect("EPSG:4326");
    GeographicCRS::create(
        "my EPSG:4326",
        base.datum().cloned(),
        base.datum_ensemble().cloned(),
        base.coordinate_system().clone(),
    )
    .expect("GeographicCRS")
}

/// A geographic CRS over an unidentified datum `datum`, with the EPSG 4326 axes.
fn geographic_over(db: &DatabaseContext, name: &str, datum: GeodeticReferenceFrame) -> Crs {
    let axes = epsg(db)
        .create_geographic_crs("4326")
        .expect("EPSG:4326")
        .coordinate_system()
        .clone();
    Crs::Geographic(GeographicCRS::create(name, Some(datum), None, axes).expect("GeographicCRS"))
}

fn wgs84_datum(name: &str) -> GeodeticReferenceFrame {
    GeodeticReferenceFrame::create(name, Ellipsoid::wgs84(), PrimeMeridian::greenwich())
}

/// A conversion, as `Conversion::createUTM(PropertyMap(), 31, true)` builds it: the
/// Transverse Mercator method of EPSG and the `degree (supplier to define representation)`
/// unit of EPSG 9122. The Rust model has no UTM constructor, so the conversion is read from
/// a PROJCRS of WKT.
fn utm31n_conversion() -> proj_rs::iso19111::Conversion {
    const WKT: &str = concat!(
        "PROJCRS[\"unknown\",",
        "BASEGEOGCRS[\"unknown\",DATUM[\"World Geodetic System 1984\",",
        "ELLIPSOID[\"WGS 84\",6378137,298.257223563,LENGTHUNIT[\"metre\",1]]],",
        "PRIMEM[\"Greenwich\",0,ANGLEUNIT[\"degree\",0.0174532925199433]]],",
        "CONVERSION[\"UTM zone 31N\",",
        "METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],",
        "PARAMETER[\"Latitude of natural origin\",0,",
        "ANGLEUNIT[\"degree (supplier to define representation)\",0.0174532925199433,ID[\"EPSG\",9122]],",
        "ID[\"EPSG\",8801]],",
        "PARAMETER[\"Longitude of natural origin\",3,",
        "ANGLEUNIT[\"degree (supplier to define representation)\",0.0174532925199433,ID[\"EPSG\",9122]],",
        "ID[\"EPSG\",8802]],",
        "PARAMETER[\"Scale factor at natural origin\",0.9996,",
        "SCALEUNIT[\"unity\",1,ID[\"EPSG\",9201]],ID[\"EPSG\",8805]],",
        "PARAMETER[\"False easting\",500000,",
        "LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]],ID[\"EPSG\",8806]],",
        "PARAMETER[\"False northing\",0,",
        "LENGTHUNIT[\"metre\",1,ID[\"EPSG\",9001]],ID[\"EPSG\",8807]]],",
        "CS[Cartesian,2],AXIS[\"(E)\",east,ORDER[1],LENGTHUNIT[\"metre\",1]],",
        "AXIS[\"(N)\",north,ORDER[2],LENGTHUNIT[\"metre\",1]]]"
    );
    let parsed = WktParser::new().create_from_wkt(WKT).expect("UTM WKT");
    match parsed.crs().expect("PROJCRS") {
        Crs::Projected(projected) => projected.deriving_conversion().clone(),
        other => panic!("not a projected CRS: {other:?}"),
    }
}

/// `TEST(factory, objectInsertion)`, the lines 4411-4424 block: a session cannot nest.
#[test]
fn object_insertion_session_cannot_nest() {
    let db = db();
    let _session = InsertSession::start(&db).expect("start");
    assert!(InsertSession::start(&db).is_err());
}

/// Lines 4426-4436: a session that is not stopped is stopped when it is dropped.
#[test]
fn object_insertion_session_tolerated_without_stop() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    drop(session);
    assert!(InsertSession::start(&db).is_ok());
}

/// Lines 4438-4442: stopping when no session is open is a no-op.
#[test]
fn object_insertion_stop_without_session_is_tolerated() {
    let db = db();
    assert!(InsertSession::start(&db).is_ok());
}

/// Lines 4444-4451: `getInsertStatementsFor()` must be preceded with
/// `startInsertStatementsSession()`. In Rust the statements are produced by an
/// `InsertSession`, so there is no call without one to refuse; the check is the type.
#[test]
fn object_insertion_nothing_to_do_for_a_known_object() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let known = AuthorityObject::Crs(Crs::Geographic(
        epsg(&db).create_geographic_crs("4326").expect("EPSG:4326"),
    ));
    assert!(statements(
        &session,
        &known,
        "EPSG",
        "4326",
        ComponentCodes::Numeric,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4453-4520: a geographic 2D CRS with the numeric code of the new object.
#[test]
fn object_insertion_geographic_2d_crs() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let crs = Crs::Geographic(my_epsg_4326(&db));
    let object = AuthorityObject::Crs(crs.clone());

    assert_eq!(
        suggests_code_for(&db, &object, "HOBU", CodeForm::Numeric).expect("code"),
        "1"
    );
    assert_eq!(
        suggests_code_for(&db, &object, "HOBU", CodeForm::FromName).expect("code"),
        "MY_EPSG_4326"
    );

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "1234",
        ComponentCodes::Numeric,
        &default_allowed(),
    );
    assert_eq!(
        suggests_code_for(&db, &object, "HOBU", CodeForm::Numeric).expect("code"),
        "1235"
    );
    assert_eq!(sql.len(), 2);
    assert_eq!(
        sql[0],
        "INSERT INTO geodetic_crs VALUES('HOBU','1234','my EPSG:4326','','geographic \
         2D','EPSG','6422','EPSG','6326',NULL,0);"
    );
    assert_eq!(
        sql[1],
        "INSERT INTO usage VALUES('HOBU','USAGE_GEODETIC_CRS_1234','geodetic_crs','HOBU','\
         1234','PROJ','EXTENT_UNKNOWN','PROJ','SCOPE_UNKNOWN');"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "1234",
        ComponentCodes::Numeric,
        &default_allowed()
    )
    .is_empty());
    drop(session);

    epsg(&db).create_geographic_crs("4326").expect("EPSG:4326");
    assert!(hobu(&db).create_geographic_crs("1234").is_err());
}

/// Lines 4522-4560: a geographic 3D CRS whose usages are those of EPSG:4979.
#[test]
fn object_insertion_geographic_3d_crs_with_known_usage() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let epsg4979 = epsg(&db).create_geographic_crs("4979").expect("EPSG:4979");
    let base = epsg4979.datum().cloned();
    let crs = Crs::Geographic(
        GeographicCRS::create(
            "my EPSG:4979",
            base,
            epsg4979.datum_ensemble().cloned(),
            epsg4979.coordinate_system().clone(),
        )
        .expect("GeographicCRS")
        .with_domains(epsg4979.domains().to_vec()),
    );
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "4979",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 2);
    assert_eq!(
        sql[0],
        "INSERT INTO geodetic_crs VALUES('HOBU','4979','my EPSG:4979','','geographic \
         3D','EPSG','6423','EPSG','6326',NULL,0);"
    );
    assert_eq!(
        sql[1],
        "INSERT INTO usage VALUES('HOBU','USAGE_GEODETIC_CRS_4979','geodetic_crs','HOBU','\
         4979','EPSG','1262','EPSG','1176');"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "4979",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4562-4606: a bound geocentric CRS with a new usage. The statements are those of
/// its base CRS.
#[test]
fn object_insertion_bound_geocentric_crs_with_new_usage() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let base = epsg(&db).create_geodetic_crs("4978").expect("EPSG:4978");
    let domain = ObjectDomain::from_bounding_box(BoundingBox::new(1.0, 2.0, 3.0, 4.0))
        .with_scope("my scope");
    let crs = GeodeticCRS::create(
        "my EPSG:4978",
        base.datum().cloned(),
        base.datum_ensemble().cloned(),
        base.coordinate_system(),
    )
    .expect("GeodeticCRS")
    .with_domains(vec![domain]);
    let bound = BoundCRS::from_towgs84(
        Crs::Geodetic(crs.clone()),
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
    )
    .expect("bound");
    let bound = AuthorityObject::Crs(Crs::Bound(bound));

    let sql = statements(
        &session,
        &bound,
        "HOBU",
        "4978",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 4);
    assert_eq!(
        sql[0],
        "INSERT INTO geodetic_crs VALUES('HOBU','4978','my \
         EPSG:4978','','geocentric','EPSG','6500','EPSG','6326',NULL,0);"
    );
    assert_eq!(
        sql[1],
        "INSERT INTO scope VALUES('HOBU','SCOPE_geodetic_crs_4978','my scope',0);"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO extent VALUES('HOBU','EXTENT_geodetic_crs_4978','unknown','unknown',2,4,1,3,0);"
    );
    assert_eq!(
        sql[3],
        "INSERT INTO usage VALUES('HOBU','USAGE_GEODETIC_CRS_4978','geodetic_crs','HOBU','4978','HOBU',\
         'EXTENT_geodetic_crs_4978','HOBU','SCOPE_geodetic_crs_4978');"
    );
    assert_identified_in_hobu(&db, &Crs::Geodetic(crs));
    assert!(statements(
        &session,
        &bound,
        "HOBU",
        "4978",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4608-4640: an unknown datum with an anchor and an anchor epoch, numeric code.
#[test]
fn object_insertion_geographic_2d_unknown_datum_numeric_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let datum = wgs84_datum("my datum")
        .with_anchor("my anchor")
        .with_anchor_epoch(2023.0);
    let crs = geographic_over(&db, "my EPSG:4326", datum);
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 4);
    assert_eq!(
        sql[0],
        "INSERT INTO geodetic_datum VALUES('HOBU','1','my datum','','EPSG','7030','EPSG','8901',NULL,NULL,NULL,\
         'my anchor',2023.000,0);"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4642-4669: an unknown datum without anchor, alpha code.
#[test]
fn object_insertion_geographic_2d_unknown_datum_alpha_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let crs = geographic_over(&db, "my EPSG:4326", wgs84_datum("my datum"));
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "MY_EPSG_4326",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(
        suggests_code_for(&db, &object, "HOBU", CodeForm::FromName).expect("code"),
        "MY_EPSG_4326_2"
    );
    assert_eq!(sql.len(), 4);
    assert_eq!(
        sql[0],
        "INSERT INTO geodetic_datum VALUES('HOBU','GEODETIC_DATUM_MY_EPSG_4326','my \
         datum','','EPSG','7030','EPSG','8901',NULL,NULL,NULL,NULL,NULL,0);"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "MY_EPSG_4326",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4671-4698: an unknown flattened ellipsoid, numeric code.
#[test]
fn object_insertion_geographic_2d_unknown_ellipsoid_numeric_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let ellipsoid = Ellipsoid::create_flattened_sphere("my ellipsoid", 6378137.0, 295.0);
    let datum = GeodeticReferenceFrame::create("my datum", ellipsoid, PrimeMeridian::greenwich());
    let crs = geographic_over(&db, "my EPSG:4326", datum);
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 5);
    assert_eq!(
        sql[0],
        "INSERT INTO ellipsoid VALUES('HOBU','1','my ellipsoid','','IAU_2015','399',6378137,'EPSG','9001',295,NULL,0);"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4700-4727: an unknown two-axis ellipsoid, alpha code.
#[test]
fn object_insertion_geographic_2d_unknown_ellipsoid_alpha_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let ellipsoid = Ellipsoid::create_two_axis("my ellipsoid", 6378137.0, 6378136.0);
    let datum = GeodeticReferenceFrame::create("my datum", ellipsoid, PrimeMeridian::greenwich());
    let crs = geographic_over(&db, "my EPSG:4326", datum);
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 5);
    assert_eq!(
        sql[0],
        "INSERT INTO ellipsoid VALUES('HOBU','ELLPS_GEODETIC_DATUM_XXXX','my ellipsoid','','IAU_2015','399',\
         6378137,'EPSG','9001',NULL,6378136,0);"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4729-4755: an unknown prime meridian, numeric code.
#[test]
fn object_insertion_geographic_2d_unknown_prime_meridian_numeric_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let pm = PrimeMeridian::create("My meridian", Measure::angle(10.0));
    let datum = GeodeticReferenceFrame::create("my datum", Ellipsoid::wgs84(), pm);
    let crs = geographic_over(&db, "my EPSG:4326", datum);
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 5);
    assert_eq!(
        sql[0],
        "INSERT INTO prime_meridian VALUES('HOBU','1','My meridian',10,'EPSG','9122',0);"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4757-4783: an unknown prime meridian, alpha code.
#[test]
fn object_insertion_geographic_2d_unknown_prime_meridian_alpha_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let pm = PrimeMeridian::create("My meridian", Measure::angle(10.0));
    let datum = GeodeticReferenceFrame::create("my datum", Ellipsoid::wgs84(), pm);
    let crs = geographic_over(&db, "my EPSG:4326", datum);
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 5);
    assert_eq!(
        sql[0],
        "INSERT INTO prime_meridian VALUES('HOBU','PM_GEODETIC_DATUM_XXXX','My meridian',10,'EPSG','9122',0);"
    );
    assert_identified_in_hobu(&db, &crs);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4785-4829: a projected CRS over EPSG:4807 with a UTM conversion, numeric code.
#[test]
fn object_insertion_projected_crs_numeric_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let base = epsg(&db).create_geodetic_crs("4807").expect("EPSG:4807");
    let crs = ProjectedCRS::create(
        "my projected CRS",
        base,
        utm31n_conversion(),
        proj_rs::iso19111::CartesianCS::create_easting_northing(UnitOfMeasure::new(
            "metre",
            1.0,
            UnitKind::Linear,
        )),
    );
    let object = AuthorityObject::Crs(Crs::Projected(crs.clone()));

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 4);
    assert_eq!(
        sql[0],
        "INSERT INTO conversion VALUES('HOBU','1','UTM zone 31N','','EPSG','9807','Transverse Mercator',\
         'EPSG','8801','Latitude of natural origin',0,'EPSG','9122',\
         'EPSG','8802','Longitude of natural origin',3,'EPSG','9122',\
         'EPSG','8805','Scale factor at natural origin',0.9996,'EPSG','9201',\
         'EPSG','8806','False easting',500000,'EPSG','9001',\
         'EPSG','8807','False northing',0,'EPSG','9001',\
         NULL,NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,NULL,0);"
    );
    assert_eq!(
        sql[1],
        "INSERT INTO usage VALUES('HOBU','USAGE_CONVERSION_1','conversion','HOBU','1','\
         PROJ','EXTENT_UNKNOWN','PROJ','SCOPE_UNKNOWN');"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO projected_crs VALUES('HOBU','XXXX','my projected CRS','','EPSG','4400','EPSG','4807',\
         'HOBU','1',NULL,0);"
    );
    assert_eq!(
        sql[3],
        "INSERT INTO usage VALUES('HOBU','USAGE_PROJECTED_CRS_XXXX','projected_crs','HOBU','XXXX','PROJ',\
         'EXTENT_UNKNOWN','PROJ','SCOPE_UNKNOWN');"
    );
    assert_identified_in_hobu(&db, &Crs::Projected(crs));
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4831-4864: a gravity-related height CRS over the known vertical datum EPSG:5101.
#[test]
fn object_insertion_vertical_crs_known_datum_numeric_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let vdatum =
        VerticalReferenceFrame::named("Ordnance Datum Newlyn").with_identifier("EPSG", "5101");
    let uom = UnitOfMeasure::new("my unit", 3.0, UnitKind::Linear);
    let crs = VerticalCRS::create(
        "my height",
        vdatum,
        VerticalCS::create_gravity_related_height(uom),
    );
    let object = AuthorityObject::Crs(Crs::Vertical(crs.clone()));

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 5);
    assert_eq!(
        sql[0],
        "INSERT INTO coordinate_system VALUES('HOBU','CS_VERTICAL_CRS_XXXX','vertical',1);"
    );
    assert_eq!(
        sql[1],
        "INSERT INTO unit_of_measure VALUES('HOBU','MY_UNIT','my unit','length',3,NULL,0);"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO axis VALUES('HOBU','CS_VERTICAL_CRS_XXXX_AXIS_1','Gravity-related height','H',\
         'up','HOBU','CS_VERTICAL_CRS_XXXX',1,'HOBU','MY_UNIT');"
    );
    assert_eq!(
        sql[3],
        "INSERT INTO vertical_crs VALUES('HOBU','XXXX','my height','','HOBU','CS_VERTICAL_CRS_XXXX',\
         'EPSG','5101',0);"
    );
    assert_eq!(
        sql[4],
        "INSERT INTO usage VALUES('HOBU','USAGE_VERTICAL_CRS_XXXX','vertical_crs','HOBU','XXXX',\
         'PROJ','EXTENT_UNKNOWN','PROJ','SCOPE_UNKNOWN');"
    );
    assert_identified_in_hobu(&db, &Crs::Vertical(crs));
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::Numeric,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4866-4898: a vertical CRS over an unidentified vertical datum with an anchor.
#[test]
fn object_insertion_vertical_crs_unknown_datum_alpha_code() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let vdatum = VerticalReferenceFrame::named("my datum").with_anchor("my anchor");
    let crs = VerticalCRS::create(
        "my height",
        vdatum,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    );
    let object = AuthorityObject::Crs(Crs::Vertical(crs.clone()));

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 4);
    assert_eq!(
        sql[0],
        "INSERT INTO vertical_datum VALUES('HOBU','VERTICAL_DATUM_XXXX','my datum','',NULL,NULL,NULL,\
         'my anchor',NULL,0);"
    );
    assert_identified_in_hobu(&db, &Crs::Vertical(crs));
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4900-4932: the same as above, with `ANCHOR_EPOCH` 2023.
#[test]
fn object_insertion_vertical_crs_unknown_datum_anchor_epoch() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let vdatum = VerticalReferenceFrame::named("my datum")
        .with_anchor("my anchor")
        .with_anchor_epoch(2023.0);
    let crs = VerticalCRS::create(
        "my height",
        vdatum,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    );
    let object = AuthorityObject::Crs(Crs::Vertical(crs.clone()));

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "YYYY",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 4);
    assert_eq!(
        sql[0],
        "INSERT INTO vertical_datum VALUES('HOBU','VERTICAL_DATUM_YYYY','my datum','',NULL,NULL,NULL,\
         'my anchor',2023.000,0);"
    );
    assert_identified_in_hobu(&db, &Crs::Vertical(crs));
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "YYYY",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4934-4987: a compound CRS read from WKT1, whose vertical component is EPSG:6360.
#[test]
fn object_insertion_compound_crs() {
    const WKT: &str = concat!(
        "COMPD_CS[\"unknown\",",
        "PROJCS[\"NAD_1983_2011_StatePlane_South_Carolina_FIPS_3900_USFT\",",
        "GEOGCS[\"NAD83(2011)\",",
        "DATUM[\"NAD83_National_Spatial_Reference_System_2011\",",
        "SPHEROID[\"GRS 1980\",6378137,298.257222101004,AUTHORITY[\"EPSG\",\"7019\"]],",
        "AUTHORITY[\"EPSG\",\"1116\"]],PRIMEM[\"Greenwich\",0],",
        "UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]]],",
        "PROJECTION[\"Lambert_Conformal_Conic_2SP\"],",
        "PARAMETER[\"latitude_of_origin\",31.8333333333333],",
        "PARAMETER[\"central_meridian\",-81],",
        "PARAMETER[\"standard_parallel_1\",32.5],",
        "PARAMETER[\"standard_parallel_2\",34.8333333333333],",
        "PARAMETER[\"false_easting\",1999996],",
        "PARAMETER[\"false_northing\",0],",
        "UNIT[\"US survey foot\",0.304800609601219,AUTHORITY[\"EPSG\",\"9003\"]],",
        "AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH]],",
        "VERT_CS[\"NAVD88 height (ftUS)\",",
        "VERT_DATUM[\"North American Vertical Datum 1988\",2005,AUTHORITY[\"EPSG\",\"5103\"]],",
        "UNIT[\"US survey foot\",0.304800609601219,AUTHORITY[\"EPSG\",\"9003\"]],",
        "AXIS[\"Up\",UP],AUTHORITY[\"EPSG\",\"6360\"]]]"
    );
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let crs = WktParser::new()
        .create_from_wkt(WKT)
        .expect("compound WKT")
        .crs()
        .expect("CRS")
        .clone();
    let object = AuthorityObject::Crs(crs.clone());

    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert_eq!(sql.len(), 6);
    assert_eq!(
        sql[4],
        "INSERT INTO compound_crs VALUES('HOBU','XXXX','unknown','','HOBU','COMPONENT_XXXX_1','EPSG','6360',0);"
    );
    assert_eq!(
        sql[5],
        "INSERT INTO usage VALUES('HOBU','USAGE_COMPOUND_CRS_XXXX','compound_crs','HOBU','XXXX','PROJ',\
         'EXTENT_UNKNOWN','PROJ','SCOPE_UNKNOWN');"
    );
    let factory = AuthorityFactory::create(&db, "");
    let identified = crs.identify_with(Some(&factory));
    assert_eq!(identified.len(), 1, "identify: {identified:?}");
    let (found, confidence) = &identified[0];
    assert_eq!(found.identifiers()[0].code_space(), "HOBU");
    assert!(found.is_equivalent_to_with(&crs, Equivalence::ExceptAxisOrder));
    assert_eq!(*confidence, 100);
    assert!(statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed()
    )
    .is_empty());
}

/// Lines 4989-5006: a dynamic geodetic reference frame (EPSG:1165, ITRF2014), with the
/// allowed authorities of `{"HOBU"}`.
#[test]
fn object_insertion_dynamic_geodetic_datum() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let datum = epsg(&db).create_datum("1165").expect("EPSG:1165");
    let sql = statements(
        &session,
        &AuthorityObject::Datum(datum.clone()),
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &only_hobu(),
    );
    assert!(!sql.is_empty());
    let datum_new = hobu(&db).create_datum("XXXX").expect("HOBU:XXXX");
    assert!(datum_new.is_equivalent_to(&datum));
}

/// Lines 5008-5025: a dynamic vertical reference frame (EPSG:1096), `{"HOBU"}` allowed.
#[test]
fn object_insertion_dynamic_vertical_datum() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let datum = epsg(&db).create_datum("1096").expect("EPSG:1096");
    let sql = statements(
        &session,
        &AuthorityObject::Datum(datum.clone()),
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &only_hobu(),
    );
    assert!(!sql.is_empty());
    let datum_new = hobu(&db).create_datum("XXXX").expect("HOBU:XXXX");
    assert!(datum_new.is_equivalent_to(&datum));
}

/// Lines 5027-5043: a geodetic datum ensemble (EPSG:6326, WGS 84), members inline.
#[test]
fn object_insertion_geodetic_datum_ensemble_inline_members() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let ensemble = epsg(&db).create_datum_ensemble("6326").expect("EPSG:6326");
    let sql = statements(
        &session,
        &AuthorityObject::DatumEnsemble(ensemble.clone()),
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &only_hobu(),
    );
    assert!(!sql.is_empty());
    let ensemble_new = hobu(&db).create_datum_ensemble("XXXX").expect("HOBU:XXXX");
    assert!(ensemble_new.is_equivalent_to(&ensemble));
}

/// Lines 5045-5066: a geodetic datum ensemble whose members are referenced, not inserted.
#[test]
fn object_insertion_geodetic_datum_ensemble_reference_members() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let ensemble = epsg(&db).create_datum_ensemble("6326").expect("EPSG:6326");
    let sql = statements(
        &session,
        &AuthorityObject::DatumEnsemble(ensemble.clone()),
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    assert!(!sql.is_empty());
    let ensemble_new = hobu(&db).create_datum_ensemble("XXXX").expect("HOBU:XXXX");
    assert!(ensemble_new.is_equivalent_to(&ensemble));
}

/// Lines 5068-5088: a vertical datum ensemble (EPSG:1288, British Isles height).
#[test]
fn object_insertion_vertical_datum_ensemble() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let ensemble = epsg(&db).create_datum_ensemble("1288").expect("EPSG:1288");
    let sql = statements(
        &session,
        &AuthorityObject::DatumEnsemble(ensemble.clone()),
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &only_hobu(),
    );
    assert!(!sql.is_empty());
    let ensemble_new = hobu(&db).create_datum_ensemble("XXXX").expect("HOBU:XXXX");
    assert!(ensemble_new.is_equivalent_to(&ensemble));
}

/// Lines 5090-5120: a sinusoidal projection whose method is a PROJ name, not an EPSG code.
#[test]
fn object_insertion_non_epsg_projection_method() {
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let crs = create_crs_from_proj_string(
        "+proj=sinu +lon_0=195 +x_0=0 +y_0=0 +R=3396000 +units=m +no_defs +type=crs",
    )
    .expect("PROJ string");
    let object = AuthorityObject::Crs(crs.clone());
    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    let conversions: Vec<&String> = sql
        .iter()
        .filter(|sql| sql.contains("INSERT INTO conversion"))
        .collect();
    assert_eq!(conversions.len(), 1);
    assert!(conversions[0]
        .contains("VALUES('HOBU','CONVERSION_XXXX','unknown','','PROJ','sinu','Sinusoidal',"));
    let crs_new = hobu(&db).create_projected_crs("XXXX").expect("HOBU:XXXX");
    assert!(crs_new.is_equivalent_to_with(
        &match crs {
            Crs::Projected(projected) => projected,
            other => panic!("not projected: {other:?}"),
        },
        Equivalence::Equivalent
    ));
}

/// Lines 5122-5180: a UTM conversion without a method identifier, with parameters out of
/// their nominal order.
#[test]
fn object_insertion_missing_method_id_and_parameter_order() {
    const WKT: &str = concat!(
        "PROJCRS[\"unknown\",\n",
        "    BASEGEOGCRS[\"unknown\",\n",
        "        DATUM[\"World Geodetic System 1984\",\n",
        "            ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
        "                LENGTHUNIT[\"metre\",1]]],\n",
        "        PRIMEM[\"Greenwich\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]],\n",
        "    CONVERSION[\"UTM zone 31N\",\n",
        "        METHOD[\"Transverse Mercator\"],\n",
        "        PARAMETER[\"Longitude of natural origin\",3,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "        PARAMETER[\"Latitude of natural origin\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "        PARAMETER[\"Scale factor at natural origin\",0.9996,\n",
        "            SCALEUNIT[\"unity\",1]],\n",
        "        PARAMETER[\"False easting\",500000,\n",
        "            LENGTHUNIT[\"metre\",1]],\n",
        "        PARAMETER[\"False northing\",0,\n",
        "            LENGTHUNIT[\"metre\",1]]],\n",
        "    CS[Cartesian,2],\n",
        "        AXIS[\"(E)\",east,\n",
        "            ORDER[1],\n",
        "            LENGTHUNIT[\"metre\",1]],\n",
        "        AXIS[\"(N)\",north,\n",
        "            ORDER[2],\n",
        "            LENGTHUNIT[\"metre\",1]]]"
    );
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let crs = WktParser::new()
        .create_from_wkt(WKT)
        .expect("WKT")
        .crs()
        .expect("CRS")
        .clone();
    let object = AuthorityObject::Crs(crs.clone());
    let sql = statements(
        &session,
        &object,
        "HOBU",
        "XXXX",
        ComponentCodes::FromParentCode,
        &default_allowed(),
    );
    let expected = "INSERT INTO conversion VALUES('HOBU','CONVERSION_XXXX',\
         'UTM zone 31N','','EPSG','9807','Transverse Mercator',\
         'EPSG','8801','Latitude of natural origin',0,'EPSG','9102',\
         'EPSG','8802','Longitude of natural origin',3,'EPSG','9102',\
         'EPSG','8805','Scale factor at natural origin',0.9996,'EPSG','9201',\
         'EPSG','8806','False easting',500000,'EPSG','9001',\
         'EPSG','8807','False northing',0,'EPSG','9001',\
         NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0)";
    let conversions: Vec<&String> = sql
        .iter()
        .filter(|sql| sql.contains("INSERT INTO conversion"))
        .collect();
    assert_eq!(conversions.len(), 1);
    assert!(conversions[0].contains(expected), "{}", conversions[0]);
    let crs_new = hobu(&db).create_projected_crs("XXXX").expect("HOBU:XXXX");
    assert!(crs_new.is_equivalent_to_with(
        &match crs {
            Crs::Projected(projected) => projected,
            other => panic!("not projected: {other:?}"),
        },
        Equivalence::Equivalent
    ));
}

/// Lines 5182-5207: an unknown projection method is an error.
#[test]
fn object_insertion_error_unknown_projection_method() {
    const WKT: &str = concat!(
        "PROJCRS[\"unknown\",\n",
        "    BASEGEOGCRS[\"unknown\",\n",
        "        DATUM[\"World Geodetic System 1984\",\n",
        "            ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
        "                LENGTHUNIT[\"metre\",1]]],\n",
        "        PRIMEM[\"Greenwich\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]],\n",
        "    CONVERSION[\"unknown\",\n",
        "        METHOD[\"unknown\"]],\n",
        "    CS[Cartesian,2],\n",
        "        AXIS[\"(E)\",east,\n",
        "            ORDER[1],\n",
        "            LENGTHUNIT[\"metre\",1]],\n",
        "        AXIS[\"(N)\",north,\n",
        "            ORDER[2],\n",
        "            LENGTHUNIT[\"metre\",1]]]"
    );
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let crs = WktParser::new()
        .create_from_wkt(WKT)
        .expect("WKT")
        .crs()
        .expect("CRS")
        .clone();
    assert!(session
        .statements_for(
            &AuthorityObject::Crs(crs),
            "HOBU",
            "XXXX",
            ComponentCodes::FromParentCode,
            &default_allowed()
        )
        .is_err());
}

/// Lines 5209-5250: an unknown projection parameter is an error.
#[test]
fn object_insertion_error_unknown_projection_parameter() {
    const WKT: &str = concat!(
        "PROJCRS[\"unknown\",\n",
        "    BASEGEOGCRS[\"unknown\",\n",
        "        DATUM[\"World Geodetic System 1984\",\n",
        "            ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
        "                LENGTHUNIT[\"metre\",1]]],\n",
        "        PRIMEM[\"Greenwich\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]],\n",
        "    CONVERSION[\"unknown\",\n",
        "        METHOD[\"Transverse Mercator\"],\n",
        "        PARAMETER[\"unknown\",0,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]],\n",
        "    CS[Cartesian,2],\n",
        "        AXIS[\"(E)\",east,\n",
        "            ORDER[1],\n",
        "            LENGTHUNIT[\"metre\",1]],\n",
        "        AXIS[\"(N)\",north,\n",
        "            ORDER[2],\n",
        "            LENGTHUNIT[\"metre\",1]]]"
    );
    let db = db();
    let session = InsertSession::start(&db).expect("start");
    let crs = WktParser::new()
        .create_from_wkt(WKT)
        .expect("WKT")
        .crs()
        .expect("CRS")
        .clone();
    assert!(session
        .statements_for(
            &AuthorityObject::Crs(crs),
            "HOBU",
            "XXXX",
            ComponentCodes::FromParentCode,
            &default_allowed()
        )
        .is_err());
}
