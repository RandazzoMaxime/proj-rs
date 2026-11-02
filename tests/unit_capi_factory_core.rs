//! PROJ `test/unit/test_factory.cpp` and `test_c_api.cpp` cases of the lot `capi-factory-core`.
//!
//! Each function is one `TEST()`. Expected values are the ones the C++ test asserts.

use proj_rs::{
    AlternativeGridNames, AuthorityFactory, AuthorityObject, CoordinateOperation, DatabaseContext,
    NameMatch, ObjectType, ProjLines, SupersededOperations, WktConvention, WktFormatter,
};

fn epsg() -> AuthorityFactory {
    let db = DatabaseContext::create().expect("proj.db");
    AuthorityFactory::create(&db, "EPSG")
}

fn operation_with(code: &str, grids: AlternativeGridNames) -> CoordinateOperation {
    epsg().create_coordinate_operation(code, grids).expect(code)
}

fn operation(code: &str) -> CoordinateOperation {
    operation_with(code, AlternativeGridNames::Official)
}

fn wkt2_2019(operation: &CoordinateOperation) -> String {
    operation
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2:2019")
}

fn proj_string(operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Conversion(_) => panic!("a conversion has no PROJ string here"),
    }
    .expect("PROJ string")
}

const HELMERT_8_CF_EXPECTED: &str = "    PARAMETER[\"Transformation reference epoch\",2002,\n        TIMEUNIT[\"year\",31556925.445],\n        ID[\"EPSG\",1049]],\n";

const HELMERT_15_CF_EXPECTED: &str = "COORDINATEOPERATION[\"ITRF2008 to GDA94 (1)\",\n    VERSION[\"GA-Aus 2010\"],\n    SOURCECRS[\n        GEODCRS[\"ITRF2008\",\n            DYNAMIC[\n                FRAMEEPOCH[2005]],\n            DATUM[\"International Terrestrial Reference Frame 2008\",\n                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]],\n            CS[Cartesian,3],\n                AXIS[\"(X)\",geocentricX,\n                    ORDER[1],\n                    LENGTHUNIT[\"metre\",1]],\n                AXIS[\"(Y)\",geocentricY,\n                    ORDER[2],\n                    LENGTHUNIT[\"metre\",1]],\n                AXIS[\"(Z)\",geocentricZ,\n                    ORDER[3],\n                    LENGTHUNIT[\"metre\",1]],\n            ID[\"EPSG\",5332]]],\n    TARGETCRS[\n        GEODCRS[\"GDA94\",\n            DATUM[\"Geocentric Datum of Australia 1994\",\n                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]],\n            CS[Cartesian,3],\n                AXIS[\"(X)\",geocentricX,\n                    ORDER[1],\n                    LENGTHUNIT[\"metre\",1]],\n                AXIS[\"(Y)\",geocentricY,\n                    ORDER[2],\n                    LENGTHUNIT[\"metre\",1]],\n                AXIS[\"(Z)\",geocentricZ,\n                    ORDER[3],\n                    LENGTHUNIT[\"metre\",1]],\n            ID[\"EPSG\",4938]]],\n    METHOD[\"Time-dependent Coordinate Frame rotation (geocen)\",\n        ID[\"EPSG\",1056]],\n    PARAMETER[\"X-axis translation\",-84.68,\n        LENGTHUNIT[\"millimetre\",0.001],\n        ID[\"EPSG\",8605]],\n    PARAMETER[\"Y-axis translation\",-19.42,\n        LENGTHUNIT[\"millimetre\",0.001],\n        ID[\"EPSG\",8606]],\n    PARAMETER[\"Z-axis translation\",32.01,\n        LENGTHUNIT[\"millimetre\",0.001],\n        ID[\"EPSG\",8607]],\n    PARAMETER[\"X-axis rotation\",-0.4254,\n        ANGLEUNIT[\"milliarc-second\",4.84813681109536E-09],\n        ID[\"EPSG\",8608]],\n    PARAMETER[\"Y-axis rotation\",2.2578,\n        ANGLEUNIT[\"milliarc-second\",4.84813681109536E-09],\n        ID[\"EPSG\",8609]],\n    PARAMETER[\"Z-axis rotation\",2.4015,\n        ANGLEUNIT[\"milliarc-second\",4.84813681109536E-09],\n        ID[\"EPSG\",8610]],\n    PARAMETER[\"Scale difference\",9.71,\n        SCALEUNIT[\"parts per billion\",1E-09],\n        ID[\"EPSG\",8611]],\n    PARAMETER[\"Rate of change of X-axis translation\",1.42,\n        LENGTHUNIT[\"millimetres per year\",3.16887651727315E-11],\n        ID[\"EPSG\",1040]],\n    PARAMETER[\"Rate of change of Y-axis translation\",1.34,\n        LENGTHUNIT[\"millimetres per year\",3.16887651727315E-11],\n        ID[\"EPSG\",1041]],\n    PARAMETER[\"Rate of change of Z-axis translation\",0.9,\n        LENGTHUNIT[\"millimetres per year\",3.16887651727315E-11],\n        ID[\"EPSG\",1042]],\n    PARAMETER[\"Rate of change of X-axis rotation\",1.5461,\n        ANGLEUNIT[\"milliarc-seconds per year\",1.53631468932076E-16],\n        ID[\"EPSG\",1043]],\n    PARAMETER[\"Rate of change of Y-axis rotation\",1.182,\n        ANGLEUNIT[\"milliarc-seconds per year\",1.53631468932076E-16],\n        ID[\"EPSG\",1044]],\n    PARAMETER[\"Rate of change of Z-axis rotation\",1.1551,\n        ANGLEUNIT[\"milliarc-seconds per year\",1.53631468932076E-16],\n        ID[\"EPSG\",1045]],\n    PARAMETER[\"Rate of change of Scale difference\",0.109,\n        SCALEUNIT[\"parts per billion per year\",3.16887651727315E-17],\n        ID[\"EPSG\",1046]],\n    PARAMETER[\"Parameter reference epoch\",1994,\n        TIMEUNIT[\"year\",31556925.445],\n        ID[\"EPSG\",1047]],\n    OPERATIONACCURACY[0.03],\n    USAGE[\n        SCOPE[\"Geodesy.\"],\n        AREA[\"Australia - onshore and offshore to 200 nautical mile EEZ boundary. Includes Lord Howe Island, Ashmore and Cartier Islands.\"],\n        BBOX[-47.2,109.23,-8.88,163.2]],\n    ID[\"EPSG\",6276],\n    REMARK[\"RMS residuals 5mm north, 8mm east and 28mm vertical, maximum residuals 10mm north, 13mm east and 51mm vertical. Scale difference in ppb and scale difference rate in ppb/yr where 1/billion = 1E-9 or nm/m.\"]]";

const GRID_ONE_PARAMETER_EXPECTED: &str = "COORDINATEOPERATION[\"RGNC91-93 to NEA74 Noumea (4)\",\n    VERSION[\"ESRI-Ncl 0.05m\"],\n    SOURCECRS[\n        GEOGCRS[\"RGNC91-93\",\n            DATUM[\"Reseau Geodesique de Nouvelle Caledonie 91-93\",\n                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]],\n            CS[ellipsoidal,2],\n                AXIS[\"geodetic latitude (Lat)\",north,\n                    ORDER[1],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n                AXIS[\"geodetic longitude (Lon)\",east,\n                    ORDER[2],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n            ID[\"EPSG\",4749]]],\n    TARGETCRS[\n        GEOGCRS[\"NEA74 Noumea\",\n            DATUM[\"NEA74 Noumea\",\n                ELLIPSOID[\"International 1924\",6378388,297,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]],\n            CS[ellipsoidal,2],\n                AXIS[\"geodetic latitude (Lat)\",north,\n                    ORDER[1],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n                AXIS[\"geodetic longitude (Lon)\",east,\n                    ORDER[2],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n            ID[\"EPSG\",4644]]],\n    METHOD[\"NTv2\",\n        ID[\"EPSG\",9615]],\n    PARAMETERFILE[\"Latitude and longitude difference file\",\"RGNC1991_NEA74Noumea.gsb\"],\n    OPERATIONACCURACY[0.05],\n    USAGE[\n        SCOPE[\"Geodesy.\"],\n        AREA[\"New Caledonia - Grande Terre - Noumea district.\"],\n        BBOX[-22.37,166.35,-22.19,166.54]],\n    ID[\"EPSG\",1295],\n    REMARK[\"Emulation using NTv2 method of tfm NEA74 Noumea to RGNC91-93 (3) (code 9328). Note reversal of sign of parameter values in grid file.\"]]";

const GRID_TWO_PARAMETER_EXPECTED: &str = "    PARAMETERFILE[\"Latitude difference file\",\"alaska.las\"],\n    PARAMETERFILE[\"Longitude difference file\",\"alaska.los\"],\n";

const OTHER_TRANSFORMATION_EXPECTED: &str = "COORDINATEOPERATION[\"S-JTSK (Ferro) to S-JTSK (1)\",\n    VERSION[\"EPSG-Cze\"],\n    SOURCECRS[\n        GEOGCRS[\"S-JTSK (Ferro)\",\n            DATUM[\"System of the Unified Trigonometrical Cadastral Network (Ferro)\",\n                ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Ferro\",-17.6666666666667,\n                ANGLEUNIT[\"degree\",0.0174532925199433]],\n            CS[ellipsoidal,2],\n                AXIS[\"geodetic latitude (Lat)\",north,\n                    ORDER[1],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n                AXIS[\"geodetic longitude (Lon)\",east,\n                    ORDER[2],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n            ID[\"EPSG\",4818]]],\n    TARGETCRS[\n        GEOGCRS[\"S-JTSK\",\n            DATUM[\"System of the Unified Trigonometrical Cadastral Network\",\n                ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,\n                    LENGTHUNIT[\"metre\",1]]],\n            PRIMEM[\"Greenwich\",0,\n                ANGLEUNIT[\"degree\",0.0174532925199433]],\n            CS[ellipsoidal,2],\n                AXIS[\"geodetic latitude (Lat)\",north,\n                    ORDER[1],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n                AXIS[\"geodetic longitude (Lon)\",east,\n                    ORDER[2],\n                    ANGLEUNIT[\"degree\",0.0174532925199433]],\n            ID[\"EPSG\",4156]]],\n    METHOD[\"Longitude rotation\",\n        ID[\"EPSG\",9601]],\n    PARAMETER[\"Longitude offset\",-17.6666666666667,\n        ANGLEUNIT[\"degree\",0.0174532925199433],\n        ID[\"EPSG\",8602]],\n    OPERATIONACCURACY[0.0],\n    USAGE[\n        SCOPE[\"Change of prime meridian.\"],\n        AREA[\"Czechia; Slovakia.\"],\n        BBOX[47.73,12.09,51.06,22.56]],\n    ID[\"EPSG\",1884]]";

#[test]
fn factory_authority_factory_create_coordinate_operation_helmert_8_cf() {
    let wkt = wkt2_2019(&operation("7702"));
    assert!(wkt.contains(HELMERT_8_CF_EXPECTED), "{wkt}");
}

#[test]
fn factory_authority_factory_create_coordinate_operation_helmert_15_cf() {
    assert_eq!(wkt2_2019(&operation("6276")), HELMERT_15_CF_EXPECTED);
}

#[test]
fn factory_authority_factory_create_coordinate_operation_cf_full_matrix_geog3d() {
    assert_eq!(
        proj_string(&operation("10675")),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m +step +proj=cart +ellps=GRS80 +step +inv +proj=helmert +exact +x=1138.7432 +y=-2064.4761 +z=110.7016 +rx=-214.615206 +ry=479.360036 +rz=-164.703951 +s=-402.32073 +convention=coordinate_frame +step +inv +proj=cart +ellps=intl +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn factory_authority_factory_create_coordinate_operation_molodensky_badekas_pv() {
    let operation = operation("1066");
    let CoordinateOperation::Transformation(transformation) = &operation else {
        panic!("EPSG:1066 is not a transformation");
    };
    assert!(transformation.validate_parameters().is_empty());
    assert_eq!(
        proj_string(&operation),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 +step +proj=cart +ellps=bessel +step +proj=molobadekas +x=593.032 +y=26 +z=478.741 +rx=0.409394387439237 +ry=-0.359705195614311 +rz=1.86849100035057 +s=4.0772 +px=3903453.148 +py=368135.313 +pz=5012970.306 +convention=coordinate_frame +step +inv +proj=cart +ellps=GRS80 +step +proj=pop +v_3 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn factory_authority_factory_create_coordinate_operation_grid_transformation_one_parameter() {
    assert_eq!(wkt2_2019(&operation("1295")), GRID_ONE_PARAMETER_EXPECTED);
}

#[test]
fn factory_authority_factory_create_coordinate_operation_grid_transformation_two_parameter() {
    let wkt = wkt2_2019(&operation("15864"));
    assert!(wkt.contains(GRID_TWO_PARAMETER_EXPECTED), "{wkt}");
}

#[test]
fn factory_authority_factory_create_coordinate_operation_other_transformation() {
    assert_eq!(wkt2_2019(&operation("1884")), OTHER_TRANSFORMATION_EXPECTED);
}

#[test]
fn factory_authority_factory_10566_issue_4212() {
    let db = DatabaseContext::create().expect("proj.db");
    assert_eq!(
        operation_with("10566", AlternativeGridNames::Proj)
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=vgridshift +grids=dk_sdfi_gllmsl_2022.tif +multiplier=1"
    );
}

fn epsg_code(operation: &CoordinateOperation) -> Option<i32> {
    operation
        .identifiers()
        .iter()
        .find(|id| id.code_space() == "EPSG")
        .and_then(|id| id.code().parse().ok())
}

fn crs_code(crs: Option<&proj_rs::Crs>) -> Option<i32> {
    let identifiers = match crs? {
        proj_rs::Crs::Geographic(crs) => crs.identifiers(),
        proj_rs::Crs::Geodetic(crs) => crs.identifiers(),
        proj_rs::Crs::Projected(crs) => crs.identifiers(),
        _ => return None,
    };
    identifiers.first()?.code().parse().ok()
}

fn endpoints(operation: &CoordinateOperation) -> (Option<&proj_rs::Crs>, Option<&proj_rs::Crs>) {
    match operation {
        CoordinateOperation::Conversion(op) => (op.source_crs(), op.target_crs()),
        CoordinateOperation::Transformation(op) => (op.source_crs(), op.target_crs()),
        CoordinateOperation::Concatenated(op) => (op.source_crs(), op.target_crs()),
    }
}

fn is_deprecated(operation: &CoordinateOperation) -> bool {
    let code = epsg_code(operation).expect("EPSG code").to_string();
    matches!(
        epsg().create_object(&code),
        Ok(object) if object.is_deprecated()
    )
}

#[test]
fn factory_authority_factory_create_from_coordinate_reference_system_codes() {
    let factory = epsg();
    assert!(factory
        .create_from_coordinate_reference_system_codes("-1", "-1")
        .expect("search")
        .is_empty());
    {
        let res = factory
            .create_from_coordinate_reference_system_codes("4326", "32631")
            .expect("search");
        assert_eq!(res.len(), 1);
        let (source, target) = endpoints(&res[0]);
        assert!(source.is_some());
        assert!(target.is_some());
        let (CoordinateOperation::Conversion(found), Ok(expected)) =
            (&res[0], factory.create_conversion("16031"))
        else {
            panic!("expected the conversion EPSG:16031");
        };
        assert!(found.is_equivalent_to(&expected));
    }
    {
        let res = factory
            .create_from_coordinate_reference_system_codes("4209", "4326")
            .expect("search");
        assert!(!res.is_empty());
        for conv in &res {
            let (source, target) = endpoints(conv);
            assert_eq!(crs_code(source), Some(4209));
            assert_eq!(crs_code(target), Some(4326));
            assert!(!is_deprecated(conv));
        }
    }
    {
        let list = factory
            .create_from_coordinate_reference_system_codes("4179", "4258")
            .expect("search");
        assert_eq!(list.len(), 3);
        // Romania has a larger area than Poland (given the approximate formula)
        assert_eq!(epsg_code(&list[0]), Some(15994)); // Romania - 3m
        assert_eq!(epsg_code(&list[1]), Some(15993)); // Romania - 10m
        assert_eq!(epsg_code(&list[2]), Some(1644)); // Poland - 1m
    }
    {
        // Test removal of superseded transform
        let list = factory
            .create_from_coordinate_reference_system_codes_in(
                ("EPSG", "4179"),
                ("EPSG", "4258"),
                SupersededOperations::Discard,
            )
            .expect("search");
        assert_eq!(list.len(), 2);
        assert_eq!(epsg_code(&list[0]), Some(15994)); // Romania - 3m
        assert_eq!(epsg_code(&list[1]), Some(1644)); // Poland - 1m
    }
}

#[test]
fn factory_authority_factory_create_from_coordinate_reference_system_codes_anonymous_authority() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "");
    {
        let res = factory
            .create_from_coordinate_reference_system_codes_in(
                ("EPSG", "4326"),
                ("EPSG", "32631"),
                SupersededOperations::Keep,
            )
            .expect("search");
        assert_eq!(res.len(), 1);
    }
    {
        let res = factory
            .create_from_coordinate_reference_system_codes_in(
                ("EPSG", "4209"),
                ("EPSG", "4326"),
                SupersededOperations::Keep,
            )
            .expect("search");
        assert!(!res.is_empty());
        for conv in &res {
            let (source, target) = endpoints(conv);
            assert_eq!(crs_code(source), Some(4209));
            assert_eq!(crs_code(target), Some(4326));
            assert!(!is_deprecated(conv));
        }
    }
}

fn authority_code(object: &AuthorityObject) -> Option<i32> {
    let ids = match object {
        AuthorityObject::Crs(crs) => match crs {
            proj_rs::Crs::Geographic(crs) => crs.identifiers().to_vec(),
            proj_rs::Crs::Geodetic(crs) => crs.identifiers().to_vec(),
            proj_rs::Crs::Projected(crs) => crs.identifiers().to_vec(),
            _ => return None,
        },
        AuthorityObject::DatumEnsemble(ensemble) => ensemble.identifiers().to_vec(),
        _ => return None,
    };
    ids.first()?.code().parse().ok()
}

#[test]
fn factory_create_objects_from_name() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let find = |factory: &AuthorityFactory, name: &str, types: &[ObjectType], mode, limit| {
        factory
            .create_objects_from_name(name, types, mode, limit)
            .expect(name)
    };
    let exact = NameMatch::Exact;
    let approx = NameMatch::Approximate;

    assert_eq!(find(&factory, "", &[], exact, 0).len(), 0);
    // ellipsoid + datum + 3 geodeticCRS
    assert_eq!(find(&factory, "WGS 84", &[], exact, 0).len(), 5);
    assert_eq!(find(&factory, "WGS 84", &[], approx, 10).len(), 10);
    assert_eq!(
        find(&factory, "WGS 84", &[ObjectType::Crs], exact, 0).len(),
        3
    );
    assert_eq!(
        find(&factory, "WGS 84", &[ObjectType::GeocentricCrs], exact, 0).len(),
        1
    );
    {
        let res = find(&epsg, "WGS84", &[ObjectType::Geographic2dCrs], approx, 0);
        // EPSG:4326 and the 6 WGS84 realizations
        // and EPSG:7881 'Tritan St. Helena'' whose alias is 'WGS 84 Tritan St. Helena'
        assert_eq!(res.len(), 10);
        assert_eq!(authority_code(&res[0]), Some(4326));
    }
    // Exact name, but just not the official case ==> should match with exact match
    assert_eq!(
        find(&factory, "WGS 84 / utm zone 31n", &[], exact, 0).len(),
        1
    );
    // Exact name, but with other CRS that have an aliases to it ==> should
    // match only the CRS with the given name, not those other CRS.
    assert_eq!(
        find(&factory, "ETRS89 / UTM zone 32N", &[], exact, 0).len(),
        1
    );
    for name in [
        "Paris",                                                // Prime meridian
        "Clarke 1880 (IGN)",                                    // Ellipsoid
        "Hungarian Datum 1909",                                 // Geodetic datum
        "EGM2008 geoid",                                        // Vertical datum
        "Christmas Island Datum 1985",                          // Engineering datum
        "Unknown datum based upon the Airy 1830 ellipsoid",     // Geodetic CRS
        "Anguilla 1957 / British West Indies Grid",             // Projected CRS
        "EGM2008 height",                                       // Vertical CRS
        "KKJ / Finland Uniform Coordinate System + N60 height", // Compound CRS
        "Christmas Island Grid 1985",                           // Engineering CRS
        "Belgian Lambert 2008",                                 // Conversion
        "MGI to ETRS89 (4)",                                    // Helmert transform
        "Guam 1963 to NAD83(HARN) (1)",                         // Grid transform
        "Monte Mario (Rome) to Monte Mario (1)",                // Other transform
        "MGI (Ferro) to WGS 84 (2)",                            // Concatenated operation
        "NAD27(CGQ77) / SCoPQ zone 2 (deprecated)",             // Deprecated object
        "NAD27(CGQ77) / SCoPQ zone 2", // Deprecated object (without explicit deprecated)
    ] {
        assert_eq!(find(&epsg, name, &[], exact, 2).len(), 1, "{name}");
    }
    // Dynamic Geodetic datum
    assert_eq!(
        find(
            &epsg,
            "International Terrestrial Reference Frame 2008",
            &[ObjectType::DynamicGeodeticReferenceFrame],
            exact,
            2
        )
        .len(),
        1
    );
    for types in [vec![ObjectType::DatumEnsemble], Vec::new()] {
        let res = find(
            &factory,
            "World Geodetic System 1984 ensemble",
            &types,
            exact,
            0,
        );
        assert_eq!(res.len(), 1);
        assert_eq!(authority_code(&res[0]), Some(6326));
        assert!(matches!(res[0], AuthorityObject::DatumEnsemble(_)));
    }
    let types = [
        ObjectType::PrimeMeridian,
        ObjectType::Ellipsoid,
        ObjectType::Datum,
        ObjectType::GeodeticReferenceFrame,
        ObjectType::DynamicGeodeticReferenceFrame,
        ObjectType::VerticalReferenceFrame,
        ObjectType::DynamicVerticalReferenceFrame,
        ObjectType::EngineeringDatum,
        ObjectType::Crs,
        ObjectType::GeodeticCrs,
        ObjectType::GeocentricCrs,
        ObjectType::GeographicCrs,
        ObjectType::Geographic2dCrs,
        ObjectType::Geographic3dCrs,
        ObjectType::ProjectedCrs,
        ObjectType::VerticalCrs,
        ObjectType::CompoundCrs,
        ObjectType::EngineeringCrs,
        ObjectType::CoordinateOperation,
        ObjectType::Conversion,
        ObjectType::Transformation,
        ObjectType::ConcatenatedOperation,
        ObjectType::DatumEnsemble,
    ];
    for kind in types {
        find(&factory, "i_dont_exist", &[kind], exact, 1);
    }
    find(&factory, "i_dont_exist", &types, exact, 1);
    {
        let res = find(&epsg, "ETRS89", &[ObjectType::Geographic2dCrs], exact, 1);
        assert_eq!(res.len(), 1);
        assert_eq!(authority_code(&res[0]), Some(4258));
    }
}

#[test]
fn factory_get_point_motion_operations_for() {
    let factory = epsg();
    // "NAD83(CSRS)v7"
    let crs = factory.create_geodetic_crs("8255").expect("8255");
    let operations = factory
        .get_point_motion_operations_for(&crs, AlternativeGridNames::Official)
        .expect("search");
    assert!(!operations.is_empty());
    assert_eq!(operations[0].identifiers()[0].code(), "9483");
}
