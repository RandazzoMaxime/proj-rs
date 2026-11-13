//! PROJ 9.9.0 `test/unit` cases of the `shape` class of `docs/UNIT-TRIAGE.md` that still
//! pin a behavior a Rust API user can observe. Each function keeps the C++ `TEST()` name.
//! Inputs and expected text are copied from the C++ source.

use std::collections::BTreeSet;

use proj_rs::{
    insert_statements, AlternativeGridNames, AuthorityFactory, AuthorityObject, Axis, CartesianCS,
    ComponentCodes, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, DeprecatedCodes, EllipsoidalCS, Equivalence, Measure, Meridian,
    ObjectType, SpatialCriterion, UnitKind, UnitOfMeasure, WktConvention, WktFormatter, WktObject,
    WktParser,
};

fn parse(wkt: &str) -> WktObject {
    WktParser::new().create_from_wkt(wkt).expect("wkt")
}

fn refused(wkt: &str) {
    assert!(WktParser::new().create_from_wkt(wkt).is_err());
}

fn export(crs: &Crs, convention: WktConvention, db: Option<&DatabaseContext>) -> String {
    let mut formatter = match db {
        Some(db) => WktFormatter::create(convention).with_database(db.clone()),
        None => WktFormatter::create(convention),
    };
    crs.export_to_wkt(&mut formatter).expect("export")
}

const AXIS_RANGE_HEAD: &str = concat!(
    "PROJCRS[\"dummy\",\n",
    "    BASEGEOGCRS[\"WGS 84\",\n",
    "        DATUM[\"World Geodetic System 1984\",\n",
    "            ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n",
    "                LENGTHUNIT[\"metre\",1,\n",
    "                    ID[\"EPSG\",9001]]]],\n",
    "        PRIMEM[\"Greenwich\",0,\n",
    "            ANGLEUNIT[\"degree\",0.0174532925199433],\n",
    "            ID[\"EPSG\",8901]]],\n",
    "    CONVERSION[\"dummy\",\n",
    "        METHOD[\"dummy\"],\n",
    "        PARAMETER[\"dummy\",1]],\n",
    "    CS[Cartesian,2],\n",
    "        AXIS[\"latitude\",north,\n",
    "            ORDER[1],\n",
    "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
    "        AXIS[\"longitude\",east,\n",
    "            ORDER[2],\n",
    "            ANGLEUNIT[\"degree\",0.0174532925199433]"
);

fn axis_range_wkt(tail: &str) -> String {
    format!("{AXIS_RANGE_HEAD}{tail}]]")
}

#[test]
fn wkt_parse_cs_with_meridian() {
    let wkt = "PROJCRS[\"dummy\",\n\
    BASEGEOGCRS[\"WGS 84\",\n\
        DATUM[\"World Geodetic System 1984\",\n\
            ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
        UNIT[\"degree\",0.0174532925199433]],\n\
    CONVERSION[\"dummy\",\n\
        METHOD[\"dummy\"],\n\
        PARAMETER[\"dummy\",1.0]],\n\
    CS[Cartesian,2],\n\
        AXIS[\"easting (X)\",south,MERIDIAN[90,ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
        AXIS[\"northing (Y)\",north],\n\
        UNIT[\"metre\",1],\n\
    ID[\"EPSG\",32631]]";
    let obj = parse(wkt);
    let crs = obj.projected().expect("projected");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    let meridian = axes[0].meridian().expect("meridian");
    assert_eq!(meridian.longitude().value(), 90.0);
    assert_eq!(
        meridian.longitude().unit().name(),
        UnitOfMeasure::degree().name()
    );
    assert!(axes[1].meridian().is_none());
}

#[test]
fn wkt_parse_cs_with_multiple_id() {
    let wkt = "GEODCRS[\"WGS 84\",\n\
    DATUM[\"World Geodetic System 1984\",\n\
        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n\
    CS[Cartesian,3],\n\
        AXIS[\"(X)\",geocentricX],\n\
        AXIS[\"(Y)\",geocentricY],\n\
        AXIS[\"(Z)\",geocentricZ],\n\
        UNIT[\"metre\",1],\n\
    ID[\"authorityA\",\"codeA\"],\n\
    ID[\"authorityB\",\"codeB\"]]";
    let obj = parse(wkt);
    let crs = obj.geodetic_crs().expect("geodetic");
    assert_eq!(crs.name(), "WGS 84");
    let ids = crs.identifiers();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0].code(), "codeA");
    assert_eq!(ids[0].code_space(), "authorityA");
    assert_eq!(ids[1].code(), "codeB");
    assert_eq!(ids[1].code_space(), "authorityB");
}

#[test]
fn wkt_parse_cs_with_axisminval_axismaxval_rangemeaning() {
    let wkt = axis_range_wkt(
        ",\n            AXISMINVALUE[0],\n            AXISMAXVALUE[360],\n            RANGEMEANING[wrapAround]",
    );
    let obj = parse(&wkt);
    let crs = obj.projected().expect("projected");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].minimum_value(), None);
    assert_eq!(axes[0].maximum_value(), None);
    assert_eq!(axes[0].range_meaning(), None);
    assert_eq!(axes[1].minimum_value(), Some(0.0));
    assert_eq!(axes[1].maximum_value(), Some(360.0));
    assert_eq!(axes[1].range_meaning(), Some("wraparound"));

    let out = export(&Crs::Projected(crs.clone()), WktConvention::Wkt2_2019, None);
    assert_eq!(out, wkt.replace("wrapAround", "wraparound"));
}

#[test]
fn wkt_parse_cs_with_invalid_axisminval_string() {
    refused(&axis_range_wkt(",\n            AXISMINVALUE[invalid]"));
}

#[test]
fn wkt_parse_cs_with_invalid_axisminval_too_many_children() {
    refused(&axis_range_wkt(",\n            AXISMINVALUE[1,2]"));
}

#[test]
fn wkt_parse_cs_with_invalid_axismaxval_string() {
    refused(&axis_range_wkt(",\n            AXISMAXVALUE[invalid]"));
}

#[test]
fn wkt_parse_cs_with_invalid_axismaxval_too_many_children() {
    refused(&axis_range_wkt(",\n            AXISMAXVALUE[1,2]"));
}

#[test]
fn wkt_parse_cs_with_invalid_rangemeaning() {
    refused(&axis_range_wkt(
        ",\n            AXISMINVALUE[0],\n            AXISMAXVALUE[360],\n            RANGEMEANING[invalid]",
    ));
}

#[test]
fn wkt_parse_cs_with_invalid_rangemeaning_too_many_children() {
    refused(&axis_range_wkt(
        ",\n            AXISMINVALUE[0],\n            AXISMAXVALUE[360],\n            RANGEMEANING[exact,unexpected_value]",
    ));
}

#[test]
fn wkt_parse_compd_cs() {
    let wkt = "COMPD_CS[\"horizontal + vertical\",\n\
    PROJCS[\"WGS 84 / UTM zone 31N\",\n\
        GEOGCS[\"WGS 84\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                SPHEROID[\"WGS 84\",6378137,298.257223563,\n\
                    AUTHORITY[\"EPSG\",\"7030\"]],\n\
                AUTHORITY[\"EPSG\",\"6326\"]],\n\
            PRIMEM[\"Greenwich\",0,\n\
                AUTHORITY[\"EPSG\",\"8901\"]],\n\
            UNIT[\"degree\",0.0174532925199433,\n\
                AUTHORITY[\"EPSG\",\"9122\"]],\n\
            AXIS[\"Latitude\",NORTH],\n\
            AXIS[\"Longitude\",EAST],\n\
            AUTHORITY[\"EPSG\",\"4326\"]],\n\
        PROJECTION[\"Transverse_Mercator\"],\n\
        PARAMETER[\"latitude_of_origin\",0],\n\
        PARAMETER[\"central_meridian\",3],\n\
        PARAMETER[\"scale_factor\",0.9996],\n\
        PARAMETER[\"false_easting\",500000],\n\
        PARAMETER[\"false_northing\",0],\n\
        UNIT[\"metre\",1,\n\
            AUTHORITY[\"EPSG\",\"9001\"]],\n\
        AXIS[\"Easting\",EAST],\n\
        AXIS[\"Northing\",NORTH],\n\
        AUTHORITY[\"EPSG\",\"32631\"]],\n\
    VERT_CS[\"ODN height\",\n\
        VERT_DATUM[\"Ordnance Datum Newlyn\",2005,\n\
            AUTHORITY[\"EPSG\",\"5101\"]],\n\
        UNIT[\"metre\",1,\n\
            AUTHORITY[\"EPSG\",\"9001\"]],\n\
        AXIS[\"Gravity-related height\",UP],\n\
        AUTHORITY[\"EPSG\",\"5701\"]],\n\
    AUTHORITY[\"codespace\",\"code\"]]";
    let obj = parse(wkt);
    let crs = obj.compound().expect("compound");
    assert_eq!(crs.name(), "horizontal + vertical");
    assert_eq!(crs.components().len(), 2);
    let ids = crs.identifiers();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].code(), "code");
    assert_eq!(ids[0].code_space(), "codespace");
}

#[test]
fn wkt_parse_vertcs_with_ellipsoidal_height_esri() {
    let wkt = "VERTCS[\"WGS_1984\",DATUM[\"D_WGS_1984\",\
SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],\
PARAMETER[\"Vertical_Shift\",0.0],\
PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]";
    let db = DatabaseContext::create().expect("proj.db");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let obj = parser.create_from_wkt(wkt).expect("wkt");
    let crs = Crs::Vertical(obj.vertical().expect("vertical").clone());
    assert_eq!(export(&crs, WktConvention::Wkt1Esri, Some(&db)), wkt);
    let expected_wkt1 = concat!(
        "VERT_CS[\"WGS_1984\",\n",
        "    VERT_DATUM[\"World Geodetic System 1984\",2002],\n",
        "    UNIT[\"metre\",1,\n",
        "        AUTHORITY[\"EPSG\",\"9001\"]],\n",
        "    AXIS[\"ellipsoidal height\",UP]]"
    );
    assert_eq!(
        export(&crs, WktConvention::Wkt1Gdal, Some(&db)),
        expected_wkt1
    );
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
    let obj = parse(wkt);
    let crs = obj.bound().expect("bound");
    assert_eq!(crs.name(), "Monte Mario / Italy zone 2");
    let out = export(&Crs::Bound(crs.clone()), WktConvention::Wkt2_2019, None);
    assert_eq!(out, wkt);
}

#[test]
fn datum_cs_with_meridian() {
    let axes = vec![
        Axis::create("Easting", "X", "south", UnitOfMeasure::metre())
            .with_meridian(Meridian::create(Measure::angle(90.0))),
        Axis::create("Northing", "Y", "south", UnitOfMeasure::metre())
            .with_meridian(Meridian::create(Measure::angle(180.0))),
    ];
    let cs = CartesianCS::create(axes).expect("cs");
    let expected = concat!(
        "CS[Cartesian,2],\n",
        "    AXIS[\"easting (X)\",south,\n",
        "        MERIDIAN[90,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "        ORDER[1],\n",
        "        LENGTHUNIT[\"metre\",1]],\n",
        "    AXIS[\"northing (Y)\",south,\n",
        "        MERIDIAN[180,\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "        ORDER[2],\n",
        "        LENGTHUNIT[\"metre\",1]]"
    );
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.hide_identifiers().expect("fresh formatter");
    assert_eq!(cs.export_to_wkt(&mut formatter), expected);
}

fn epsg_operation(code: &str) -> proj_rs::CoordinateOperation {
    let db = DatabaseContext::create().expect("proj.db");
    AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation(code, AlternativeGridNames::Official)
        .expect("operation")
}

#[test]
fn c_api_proj_get_remarks() {
    let transformation = epsg_operation("8048");
    assert!(transformation
        .remarks()
        .starts_with("Scale difference in ppb where 1/billion = 1E-9."));
    let conversion = epsg_operation("3811");
    assert_eq!(conversion.remarks(), "Replaces Lambert 2005.");
}

#[test]
fn c_api_proj_get_scope() {
    let transformation = epsg_operation("8048");
    assert_eq!(
        transformation.domains().first().and_then(|d| d.scope()),
        Some("Transformation of GDA94 coordinates that have been derived through GNSS CORS.")
    );
    let conversion = epsg_operation("3811");
    assert_eq!(
        conversion.domains().first().and_then(|d| d.scope()),
        Some("Engineering survey, topographic mapping.")
    );
}

fn check_axis(axis: &Axis, name: &str, abbreviation: &str, direction: &str, unit: &UnitOfMeasure) {
    assert_eq!(axis.name(), name);
    assert_eq!(axis.abbreviation(), abbreviation);
    assert_eq!(axis.direction(), direction);
    assert_eq!(axis.unit().name(), unit.name());
    assert_eq!(axis.unit().conversion_to_si(), unit.conversion_to_si());
}

#[test]
fn c_api_proj_create_ellipsoidal_3d_cs() {
    let degree = UnitOfMeasure::degree();
    let metre = UnitOfMeasure::metre();
    let cs = EllipsoidalCS::create_latitude_longitude_height(degree.clone(), metre.clone());
    assert_eq!(cs.axes().len(), 3);
    // Synthetic lat/lon axes use the PROJ abbreviations `lat`/`lon` (createLAT_NORTH and
    // createLONG_EAST, coordinatesystem.cpp:890-900; test_c_api.cpp:4947).
    check_axis(&cs.axes()[0], "Latitude", "lat", "north", &degree);
    check_axis(&cs.axes()[1], "Longitude", "lon", "east", &degree);
    check_axis(&cs.axes()[2], "Ellipsoidal height", "h", "up", &metre);

    let angular = UnitOfMeasure::new("foo", 0.5, UnitKind::Angular);
    let linear = UnitOfMeasure::new("bar", 0.6, UnitKind::Linear);
    let cs = EllipsoidalCS::create_longitude_latitude_height(angular.clone(), linear.clone());
    assert_eq!(cs.axes().len(), 3);
    check_axis(&cs.axes()[0], "Longitude", "lon", "east", &angular);
    check_axis(&cs.axes()[1], "Latitude", "lat", "north", &angular);
    check_axis(&cs.axes()[2], "Ellipsoidal height", "h", "up", &linear);
}

#[test]
fn c_api_proj_concatoperation_get_step() {
    let non_concatenated = epsg_operation("8048");
    assert!(!matches!(
        non_concatenated,
        CoordinateOperation::Concatenated(_)
    ));

    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let source = factory
        .create_coordinate_reference_system("28356")
        .expect("GDA94 / MGA zone 56");
    let target = factory
        .create_coordinate_reference_system("7856")
        .expect("GDA2020 / MGA zone 56");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    let CoordinateOperation::Concatenated(concatenated) = operations.first().expect("an operation")
    else {
        panic!("first operation is not concatenated");
    };
    let steps = concatenated.steps();
    assert_eq!(steps.len(), 3);
    assert!(steps.get(3).is_none());
    let step = steps[1].operation();
    assert!(step
        .domains()
        .first()
        .and_then(|d| d.scope())
        .is_some_and(|s| !s.is_empty()));
    assert!(!step.remarks().is_empty());
}

fn first(codes: &BTreeSet<String>) -> &str {
    codes.iter().next().expect("a code")
}

#[test]
fn factory_authority_factory_get_authority_codes() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let codes = |kind: ObjectType| {
        let set = factory
            .get_authority_codes(kind, DeprecatedCodes::Included)
            .expect("codes");
        assert!(!set.is_empty(), "{kind:?}");
        set
    };
    let merged = |sets: &[&BTreeSet<String>]| -> BTreeSet<String> {
        sets.iter().flat_map(|set| set.iter().cloned()).collect()
    };

    factory
        .create_prime_meridian(first(&codes(ObjectType::PrimeMeridian)))
        .expect("prime meridian");
    factory
        .create_ellipsoid(first(&codes(ObjectType::Ellipsoid)))
        .expect("ellipsoid");

    let datums = codes(ObjectType::Datum);
    factory.create_datum(first(&datums)).expect("datum");
    let geodetic = codes(ObjectType::GeodeticReferenceFrame);
    factory
        .create_geodetic_datum(first(&geodetic))
        .expect("geodetic datum");
    let dynamic_geodetic = codes(ObjectType::DynamicGeodeticReferenceFrame);
    let dgrf = factory
        .create_geodetic_datum(first(&dynamic_geodetic))
        .expect("dynamic geodetic datum");
    assert!(dgrf.frame_reference_epoch().is_some());
    assert!(dynamic_geodetic.len() < geodetic.len());
    let vertical = codes(ObjectType::VerticalReferenceFrame);
    factory
        .create_vertical_datum(first(&vertical))
        .expect("vertical datum");
    let dynamic_vertical = codes(ObjectType::DynamicVerticalReferenceFrame);
    let dvrf = factory
        .create_vertical_datum(first(&dynamic_vertical))
        .expect("dynamic vertical datum");
    assert!(dvrf.frame_reference_epoch().is_some());
    assert!(dynamic_vertical.len() < vertical.len());
    let engineering = codes(ObjectType::EngineeringDatum);
    factory
        .create_engineering_datum(first(&engineering))
        .expect("engineering datum");
    assert_eq!(datums, merged(&[&geodetic, &vertical, &engineering]));

    let all_crs = codes(ObjectType::Crs);
    factory
        .create_coordinate_reference_system(first(&all_crs))
        .expect("crs");
    let geodetic_crs = codes(ObjectType::GeodeticCrs);
    factory
        .create_geodetic_crs(first(&geodetic_crs))
        .expect("geodetic crs");
    let geocentric = codes(ObjectType::GeocentricCrs);
    factory
        .create_geodetic_crs(first(&geocentric))
        .expect("geocentric crs");
    assert!(geocentric.len() < geodetic_crs.len());
    let geographic = codes(ObjectType::GeographicCrs);
    factory
        .create_geographic_crs(first(&geographic))
        .expect("geographic crs");
    assert!(geographic.len() < geodetic_crs.len());
    assert!(geographic.iter().all(|code| geodetic_crs.contains(code)));
    let geographic_2d = codes(ObjectType::Geographic2dCrs);
    factory
        .create_geographic_crs(first(&geographic_2d))
        .expect("geographic 2D crs");
    let geographic_3d = codes(ObjectType::Geographic3dCrs);
    factory
        .create_geographic_crs(first(&geographic_3d))
        .expect("geographic 3D crs");
    assert_eq!(geographic_2d.len() + geographic_3d.len(), geographic.len());
    assert_eq!(geocentric.len() + geographic.len(), geodetic_crs.len());
    let vertical_crs = codes(ObjectType::VerticalCrs);
    factory
        .create_vertical_crs(first(&vertical_crs))
        .expect("vertical crs");
    let projected = codes(ObjectType::ProjectedCrs);
    factory
        .create_projected_crs(first(&projected))
        .expect("projected crs");
    let compound = codes(ObjectType::CompoundCrs);
    factory
        .create_compound_crs(first(&compound))
        .expect("compound crs");
    let engineering_crs = codes(ObjectType::EngineeringCrs);
    factory
        .create_engineering_crs(first(&engineering_crs))
        .expect("engineering crs");
    assert_eq!(
        all_crs,
        merged(&[
            &geodetic_crs,
            &vertical_crs,
            &projected,
            &compound,
            &engineering_crs
        ])
    );

    let operations = codes(ObjectType::CoordinateOperation);
    factory
        .create_coordinate_operation(first(&operations), AlternativeGridNames::Official)
        .expect("operation");
    let conversions = codes(ObjectType::Conversion);
    factory
        .create_conversion(first(&conversions))
        .expect("conversion");
    let transformations = codes(ObjectType::Transformation);
    let transformation = factory
        .create_coordinate_operation(first(&transformations), AlternativeGridNames::Official)
        .expect("transformation");
    assert!(matches!(
        transformation,
        CoordinateOperation::Transformation(_)
    ));
    let concatenated = codes(ObjectType::ConcatenatedOperation);
    let concatenated_op = factory
        .create_coordinate_operation(first(&concatenated), AlternativeGridNames::Official)
        .expect("concatenated operation");
    assert!(matches!(
        concatenated_op,
        CoordinateOperation::Concatenated(_)
    ));
    assert_eq!(
        operations,
        merged(&[&conversions, &transformations, &concatenated])
    );
}

#[test]
fn c_api_proj_is_equivalent_to_with_ctx() {
    let db = DatabaseContext::create().expect("proj.db");
    let from_epsg = AuthorityFactory::create(&db, "EPSG")
        .create_geographic_crs("7844")
        .expect("EPSG:7844");
    let wkt = concat!(
        "GEOGCRS[\"GDA2020\",\n",
        "    DATUM[\"GDA2020\",\n",
        "        ELLIPSOID[\"GRS_1980\",6378137,298.257222101,\n",
        "            LENGTHUNIT[\"metre\",1]]],\n",
        "    PRIMEM[\"Greenwich\",0,\n",
        "        ANGLEUNIT[\"Degree\",0.0174532925199433]],\n",
        "    CS[ellipsoidal,2],\n",
        "        AXIS[\"geodetic latitude (Lat)\",north,\n",
        "            ORDER[1],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "        AXIS[\"geodetic longitude (Lon)\",east,\n",
        "            ORDER[2],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]"
    );
    let parsed = parse(wkt);
    let from_wkt = parsed.geographic().expect("geographic");
    assert!(from_epsg.is_equivalent_to_with_database(from_wkt, Equivalence::Equivalent, &db));
}

#[test]
fn c_api_proj_get_insert_statements() {
    let wkt = concat!(
        "GEOGCRS[\"myGDA2020\",\n",
        "    DATUM[\"GDA2020\",\n",
        "        ELLIPSOID[\"GRS_1980\",6378137,298.257222101,\n",
        "            LENGTHUNIT[\"metre\",1]]],\n",
        "    PRIMEM[\"Greenwich\",0,\n",
        "        ANGLEUNIT[\"Degree\",0.0174532925199433]],\n",
        "    CS[ellipsoidal,2],\n",
        "        AXIS[\"geodetic latitude (Lat)\",north,\n",
        "            ORDER[1],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "        AXIS[\"geodetic longitude (Lon)\",east,\n",
        "            ORDER[2],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]"
    );
    let db = DatabaseContext::create().expect("proj.db");
    let parsed = parse(wkt);
    let crs = AuthorityObject::Crs(parsed.crs().expect("crs").clone());
    let statements = |allowed: &[&str]| {
        let allowed: Vec<String> = allowed.iter().map(|name| (*name).to_owned()).collect();
        insert_statements(
            &db,
            &crs,
            "HOBU",
            "XXXX",
            ComponentCodes::FromParentCode,
            &allowed,
        )
        .expect("statements")
    };

    let list = statements(&["EPSG", "PROJ"]);
    assert_eq!(
        list[0],
        "INSERT INTO geodetic_datum VALUES('HOBU','GEODETIC_DATUM_XXXX','GDA2020','',\
         'EPSG','7019','EPSG','8901',NULL,NULL,NULL,NULL,NULL,0);"
    );
    assert_eq!(list.len(), 4);
    // Without EPSG the ellipsoid and the prime meridian cannot be reused.
    assert_eq!(statements(&[]).len(), 6);
    assert_eq!(statements(&["PROJ"]).len(), 6);
    assert_eq!(statements(&["EPSG"]).len(), 4);
}

#[test]
fn factory_attach_extra_databases_none() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_geodetic_crs("4979")
        .expect("EPSG:4979");
    assert!(matches!(crs, proj_rs::GeodeticCRS::Geographic(_)));
}
