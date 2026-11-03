//! PROJ `test/unit/test_factory.cpp` and `test_c_api.cpp` cases of the lot `capi-factory-core`.
//!
//! Each function is one `TEST()`. Expected values are the ones the C++ test asserts.

use proj_rs::iso19111::{
    create_crs_from_proj_string, Ellipsoid as IsoEllipsoid, JsonFormat, JsonLines,
};
use proj_rs::{
    AlternativeGridNames, AuthorityFactory, AuthorityObject, BoundCRS, BoundingBox, CartesianCS,
    CompoundCRS, ConcatenatedStep, Conversion, CoordinateOperation, CoordinateSystem, Crs, CrsInfo,
    DatabaseContext, EllipsoidalCS, Equivalence, GeodeticCRS, GeodeticComponents, GeographicCRS,
    GridInfo, NameMatch, ObjectType, OpProps, OperationParameter, ParameterValue, ProjLines,
    ProjectedCRS, SupersededOperations, Transformation, UnitKind, UnitOfMeasure, VerticalCRS,
    VerticalCS, VerticalReferenceFrame, WktConvention, WktFormatter, WktParser,
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

// ---- test_c_api.cpp ----

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

fn epsg_crs(code: &str) -> Crs {
    epsg().create_coordinate_reference_system(code).expect(code)
}

#[test]
fn c_api_proj_crs_promote_to_3d() {
    let crs3d = GeographicCRS::epsg_4326().promote_to_3d_with_db(Some(&database()));
    assert_eq!(crs3d.coordinate_system().axes().len(), 3);
    assert_eq!(crs3d.identifiers()[0].code(), "4979");
}

#[test]
fn c_api_proj_crs_demote_to_2d() {
    let crs2d = GeographicCRS::epsg_4979().demote_to_2d(Some(&database()));
    assert_eq!(crs2d.coordinate_system().axes().len(), 2);
    assert_eq!(crs2d.identifiers()[0].code(), "4326");
}

#[test]
fn c_api_proj_crs_create_projected_3d_crs_from_2d() {
    let Crs::Projected(proj_crs) = epsg_crs("32631") else {
        panic!("EPSG:32631 is projected");
    };
    let geog3d = GeographicCRS::epsg_4979();
    for crs3d in [
        proj_crs.promote_to_3d_on(&geog3d),
        proj_crs.promote_to_3d(None),
    ] {
        assert_eq!(crs3d.name(), proj_crs.name());
        assert_eq!(crs3d.coordinate_system().axes().len(), 3);
    }
}

#[test]
fn c_api_proj_normalize_for_visualization_on_crs() {
    let normalized = epsg_crs("4326").normalize_for_visualization();
    assert!(normalized.identifiers().is_empty());
    let Crs::Geographic(geographic) = normalized else {
        panic!("EPSG:4326 is geographic");
    };
    assert_eq!(
        geographic.coordinate_system().axes()[0].name(),
        "Geodetic longitude"
    );
}

#[test]
fn c_api_proj_query_geodetic_crs_from_datum() {
    let db = database();
    let any = AuthorityFactory::create(&db, "");
    assert!(
        any.create_geodetic_crs_from_datum("EPSG", "6326", "")
            .expect("query")
            .len()
            >= 3
    );
    let epsg = AuthorityFactory::create(&db, "EPSG");
    assert_eq!(
        epsg.create_geodetic_crs_from_datum("EPSG", "6326", "geographic 2D")
            .expect("query")
            .len(),
        1
    );
}

#[test]
fn c_api_proj_get_geoid_models_from_database() {
    let list = epsg().get_geoid_models("5703").expect("geoid models");
    let has = |name: &str| list.iter().any(|model| model == name);
    assert!(has("GEOID12B"));
    assert!(has("GEOID18"));
    assert!(has("GGM10"));
    assert!(!has("OSGM15"));
}

#[test]
fn c_api_proj_uom_get_info_from_database() {
    let factory = AuthorityFactory::create(&database(), "auth");
    assert!(factory.create_unit_of_measure("code").is_err());
    let factory = epsg();
    let metre = factory.create_unit_of_measure("9001").expect("metre");
    assert_eq!(metre.name(), "metre");
    assert_eq!(metre.conversion_to_si(), 1.0);
    assert_eq!(metre.kind(), UnitKind::Linear);
    let degree = factory.create_unit_of_measure("9102").expect("degree");
    assert_eq!(degree.name(), "degree");
    assert!((degree.conversion_to_si() - UnitOfMeasure::degree().conversion_to_si()).abs() < 1e-10);
    assert_eq!(degree.kind(), UnitKind::Angular);
}

#[test]
fn c_api_proj_get_units_from_database() {
    let all = AuthorityFactory::create(&database(), "")
        .get_unit_list()
        .expect("units");
    assert!(!all.is_empty());
    assert!(!all[0].authority.is_empty() && !all[0].code.is_empty() && !all[0].name.is_empty());

    let linear: Vec<_> = epsg()
        .get_unit_list()
        .expect("units")
        .into_iter()
        .filter(|unit| unit.category == "linear" && !unit.deprecated)
        .collect();
    assert!(linear.len() > 1);
    let mut found_9001 = false;
    for unit in &linear {
        assert_eq!(unit.authority, "EPSG");
        if unit.code == "9001" {
            assert_eq!(unit.name, "metre");
            assert_eq!(unit.category, "linear");
            assert_eq!(unit.conv_factor, 1.0);
            assert_eq!(unit.proj_short_name, "m");
            assert!(!unit.deprecated);
            found_9001 = true;
        }
    }
    assert!(found_9001);
}

#[test]
fn c_api_proj_create_from_name() {
    let db = database();
    let any = AuthorityFactory::create(&db, "");
    let res = any
        .create_objects_from_name("WGS 84", &[], NameMatch::Exact, 0)
        .expect("search");
    assert_eq!(res.len(), 5);
    let unknown = AuthorityFactory::create(&db, "xx");
    let res = unknown
        .create_objects_from_name("WGS 84", &[], NameMatch::Exact, 0)
        .expect("search");
    assert_eq!(res.len(), 0);
    let res = any
        .create_objects_from_name(
            "WGS 84",
            &[ObjectType::GeodeticCrs, ObjectType::ProjectedCrs],
            NameMatch::Approximate,
            10,
        )
        .expect("search");
    assert_eq!(res.len(), 10);
}

#[test]
fn c_api_proj_crs_create_bound_crs_to_wgs84() {
    let db = database();
    let bound = epsg_crs("4807").create_bound_crs_to_wgs84_if_possible(&db);
    let Crs::Bound(bound_crs) = &bound else {
        panic!("EPSG:4807 binds to WGS 84");
    };
    let proj =
        "+proj=longlat +ellps=clrk80ign +pm=paris +towgs84=-168,-60,320,0,0,0,0 +no_defs +type=crs";
    assert_eq!(bound.export_to_proj_string().expect("PROJ"), proj);
    assert!(bound_crs.target_crs().name().eq_ignore_ascii_case("WGS 84"));
    let rebuilt = Crs::Bound(BoundCRS::create(
        bound_crs.source_crs().clone(),
        bound_crs.target_crs().clone(),
        bound_crs.transformation().clone(),
    ));
    assert_eq!(rebuilt.export_to_proj_string().expect("PROJ"), proj);
}

#[test]
fn c_api_proj_get_non_deprecated() {
    let db = database();
    let replacements = epsg_crs("4226").non_deprecated(&db).expect("replacements");
    assert_eq!(replacements.len(), 2);
}

#[test]
fn c_api_proj_grid_get_info_from_database() {
    let db = database();
    assert!(db.grid_info("xxx").expect("query").is_none());
    let info = db
        .grid_info("au_icsm_GDA94_GDA2020_conformal.tif")
        .expect("query")
        .expect("known grid");
    assert_grid_au(&info);
    let old = db
        .grid_info("GDA94_GDA2020_conformal.gsb")
        .expect("query")
        .expect("old name");
    assert_grid_au(&old);
}

fn assert_grid_au(info: &GridInfo) {
    assert!(info.full_name.is_empty());
    assert!(info.package_name.is_empty());
    assert_eq!(
        info.url,
        "https://cdn.proj.org/au_icsm_GDA94_GDA2020_conformal.tif"
    );
    assert!(info.direct_download);
    assert!(info.open_license);
}

#[test]
fn c_api_proj_as_projjson() {
    let db = database();
    let ellipsoid = IsoEllipsoid::wgs84();
    let multiline = "{\n  \"$schema\": \"https://proj.org/schemas/v0.7/projjson.schema.json\",\n  \"type\": \"Ellipsoid\",\n  \"name\": \"WGS 84\",\n  \"semi_major_axis\": 6378137,\n  \"inverse_flattening\": 298.257223563,\n  \"id\": {\n    \"authority\": \"EPSG\",\n    \"code\": 7030\n  }\n}";
    assert_eq!(
        ellipsoid.export_to_json(&db, &JsonFormat::default()),
        multiline
    );
    let indent_4 = "{\n    \"type\": \"Ellipsoid\",\n    \"name\": \"WGS 84\",\n    \"semi_major_axis\": 6378137,\n    \"inverse_flattening\": 298.257223563,\n    \"id\": {\n        \"authority\": \"EPSG\",\n        \"code\": 7030\n    }\n}";
    assert_eq!(
        ellipsoid.export_to_json(
            &db,
            &JsonFormat::with_lines(JsonLines::Multi)
                .with_indent_width(4)
                .with_schema("")
        ),
        indent_4
    );
    let single = "{\"type\":\"Ellipsoid\",\"name\":\"WGS 84\",\"semi_major_axis\":6378137,\"inverse_flattening\":298.257223563,\"id\":{\"authority\":\"EPSG\",\"code\":7030}}";
    assert_eq!(
        ellipsoid.export_to_json(
            &db,
            &JsonFormat::with_lines(JsonLines::Single).with_schema("")
        ),
        single
    );
}

#[test]
fn c_api_datum_ensemble() {
    let wkt = "GEOGCRS[\"ETRS89\",\
        ENSEMBLE[\"European Terrestrial Reference System 1989 ensemble\",\
            MEMBER[\"European Terrestrial Reference Frame 1989\"],\
            MEMBER[\"European Terrestrial Reference Frame 1990\"],\
            MEMBER[\"European Terrestrial Reference Frame 1991\"],\
            MEMBER[\"European Terrestrial Reference Frame 1992\"],\
            MEMBER[\"European Terrestrial Reference Frame 1993\"],\
            MEMBER[\"European Terrestrial Reference Frame 1994\"],\
            MEMBER[\"European Terrestrial Reference Frame 1996\"],\
            MEMBER[\"European Terrestrial Reference Frame 1997\"],\
            MEMBER[\"European Terrestrial Reference Frame 2000\"],\
            MEMBER[\"European Terrestrial Reference Frame 2005\"],\
            MEMBER[\"European Terrestrial Reference Frame 2014\"],\
            ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\
                LENGTHUNIT[\"metre\",1]],\
            ENSEMBLEACCURACY[0.1]],\
        PRIMEM[\"Greenwich\",0,\
            ANGLEUNIT[\"degree\",0.0174532925199433]],\
        CS[ellipsoidal,2],\
            AXIS[\"geodetic latitude (Lat)\",north,\
                ORDER[1],\
                ANGLEUNIT[\"degree\",0.0174532925199433]],\
            AXIS[\"geodetic longitude (Lon)\",east,\
                ORDER[2],\
                ANGLEUNIT[\"degree\",0.0174532925199433]]]";
    let parsed = WktParser::new()
        .create_from_wkt(wkt)
        .expect("ETRS89 ensemble");
    let geographic = parsed.geographic().expect("geographic");
    assert!(geographic.datum().is_none());
    let ensemble = geographic.datum_ensemble().expect("ensemble");
    assert_eq!(ensemble.members().len(), 11);
    assert!(ensemble.members().get(11).is_none());
    assert_eq!(
        ensemble.members()[0].name(),
        "European Terrestrial Reference Frame 1989"
    );
    assert_eq!(
        ensemble.members()[10].name(),
        "European Terrestrial Reference Frame 2014"
    );
    assert_eq!(ensemble.positional_accuracy(), 0.1);
    let forced = ensemble.as_datum().expect("forced datum");
    assert_eq!(forced.name(), "European Terrestrial Reference System 1989");
    let rebuilt = GeographicCRS::create(
        geographic.name(),
        None,
        Some(ensemble.clone()),
        geographic.coordinate_system().clone(),
    )
    .expect("rebuild");
    assert!(rebuilt.is_equivalent_to_with(geographic, Equivalence::Equivalent));
    let geocentric = GeodeticCRS::create(
        geographic.name(),
        None,
        Some(ensemble.clone()),
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("geocentric");
    assert!(matches!(geocentric, GeodeticCRS::Geocentric(_)));
}

fn concatenated(code: &str) -> proj_rs::ConcatenatedOperation {
    let CoordinateOperation::Concatenated(operation) = operation(code) else {
        panic!("EPSG:{code} is concatenated");
    };
    operation
}

fn same_operation(left: &CoordinateOperation, right: &CoordinateOperation) -> bool {
    match (left, right) {
        (CoordinateOperation::Transformation(left), CoordinateOperation::Transformation(right)) => {
            left.is_equivalent_to(right)
        }
        (CoordinateOperation::Conversion(left), CoordinateOperation::Conversion(right)) => {
            left.is_equivalent_to(right)
        }
        (CoordinateOperation::Concatenated(left), CoordinateOperation::Concatenated(right)) => {
            left.name() == right.name() && left.steps().len() == right.steps().len()
        }
        _ => false,
    }
}

fn endpoint_names(operation: &CoordinateOperation) -> (Option<&str>, Option<&str>) {
    let pair = match operation {
        CoordinateOperation::Transformation(operation) => {
            (operation.source_crs(), operation.target_crs())
        }
        CoordinateOperation::Conversion(operation) => {
            (operation.source_crs(), operation.target_crs())
        }
        CoordinateOperation::Concatenated(operation) => {
            (operation.source_crs(), operation.target_crs())
        }
    };
    (pair.0.map(Crs::name), pair.1.map(Crs::name))
}

fn effective_step(step: &ConcatenatedStep) -> CoordinateOperation {
    if step.is_reverse() {
        step.operation().inverse()
    } else {
        step.operation().clone()
    }
}

#[test]
fn factory_authority_factory_create_coordinate_operation_concatenated_operation_inverse_step1() {
    let steps = concatenated("8443").steps().to_vec();
    assert_eq!(steps.len(), 2);
    assert!(same_operation(
        &effective_step(&steps[0]),
        &operation("8364").inverse()
    ));
    assert!(same_operation(
        &effective_step(&steps[1]),
        &operation("8367")
    ));
}

#[test]
fn factory_authority_factory_create_coordinate_operation_concatenated_operation_inverse_step2() {
    let steps = concatenated("7811").steps().to_vec();
    assert_eq!(steps.len(), 2);
    assert!(same_operation(
        &effective_step(&steps[0]),
        &operation("1763")
    ));
    assert!(same_operation(
        &effective_step(&steps[1]),
        &operation("15958").inverse()
    ));
}

#[test]
fn factory_authority_factory_create_coordinate_operation_concatenated_operation_step_2_and_3_are_conversion(
) {
    let concatenated_operation = concatenated("7987");
    let steps = concatenated_operation.steps();
    assert_eq!(steps.len(), 3);
    assert!(same_operation(
        &effective_step(&steps[0]),
        &operation("7980")
    ));
    assert!(same_operation(
        &effective_step(&steps[1]),
        &operation("7812")
    ));
    assert!(same_operation(
        &effective_step(&steps[2]),
        &operation("7813")
    ));
    let (_source, target) = endpoint_names(steps[1].operation());
    assert_eq!(target, Some("KOC WD depth"));
    let (next_source, _) = endpoint_names(steps[2].operation());
    assert_eq!(next_source, target);
    assert_eq!(
        concatenated_operation
            .export_to_proj_string(ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +proj=geogoffset +dh=-4.74 +step +proj=axisswap +order=1,2,-3 +step +proj=unitconvert +z_in=m +z_out=ft"
    );
    assert_eq!(
        concatenated_operation
            .inverse()
            .export_to_proj_string(ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +proj=unitconvert +z_in=ft +z_out=m +step +proj=axisswap +order=1,2,-3 +step +proj=geogoffset +dh=4.74"
    );
}

#[test]
fn factory_authority_factory_create_coordinate_operation_concatenated_operation_epsg_9103() {
    let steps = concatenated("9103").steps().to_vec();
    assert_eq!(steps.len(), 5);
    assert_eq!(steps[0].operation().name(), "NAD27 to NAD83 (1)");
    assert_eq!(steps[1].operation().name(), "NAD83 to NAD83(2011) (1)");
    assert_eq!(
        steps[2].operation().name(),
        "Conversion from NAD83(2011) (geog2D) to NAD83(2011) (geocentric)"
    );
    assert_eq!(
        steps[3].operation().name(),
        "Inverse of ITRF2008 to NAD83(2011) (1)"
    );
    assert_eq!(steps[4].operation().name(), "ITRF2008 to ITRF2014 (1)");
}

fn lat_lon() -> EllipsoidalCS {
    EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree())
}

fn geodetic_parts<'a>(
    crs_name: Option<&'a str>,
    datum_name: Option<&'a str>,
    ellipsoid_name: Option<&'a str>,
    semi_major_metre: f64,
    inverse_flattening: f64,
    prime_meridian_name: Option<&'a str>,
    prime_meridian_offset: f64,
) -> GeodeticComponents<'a> {
    GeodeticComponents {
        crs_name,
        datum_name,
        ellipsoid_name,
        semi_major_metre,
        inverse_flattening,
        prime_meridian_name,
        prime_meridian_offset,
        angular_unit_name: Some("Degree"),
        angular_unit_to_radian: 0.0174532925199433,
    }
}

#[test]
fn c_api_proj_create_geographic_crs() {
    // CApi.proj_create_geographic_crs
    let db = DatabaseContext::create().expect("proj.db");
    let cs = lat_lon();
    let wgs84 = db.create_geographic_crs(
        &geodetic_parts(
            Some("WGS 84"),
            Some("World Geodetic System 1984"),
            Some("WGS 84"),
            6_378_137.0,
            298.257_223_563,
            Some("Greenwich"),
            0.0,
        ),
        cs.clone(),
    );
    assert!(wgs84.is_equivalent_to_with(&GeographicCRS::epsg_4326(), Equivalence::Equivalent));
    let from_datum =
        GeographicCRS::create("WGS 84", wgs84.datum().cloned(), None, cs.clone()).expect("datum");
    assert!(wgs84.is_equivalent_to(&from_datum));

    let unnamed = db.create_geographic_crs(
        &geodetic_parts(None, None, None, 1.0, 0.0, None, 0.0),
        cs.clone(),
    );
    assert_eq!(unnamed.name(), "unnamed");

    let wkt1 = db.create_geographic_crs(
        &geodetic_parts(
            Some("WGS 84"),
            Some("WGS_1984"),
            Some("WGS 84"),
            6_378_137.0,
            298.257_223_563,
            Some("Greenwich"),
            0.0,
        ),
        cs.clone(),
    );
    assert!(wkt1.is_equivalent_to_with(&GeographicCRS::epsg_4326(), Equivalence::Equivalent));

    let nad83 = db.create_geographic_crs(
        &geodetic_parts(
            Some("NAD83"),
            Some("North_American_Datum_1983"),
            Some("GRS 1980"),
            6_378_137.0,
            298.257_222_101,
            Some("Greenwich"),
            0.0,
        ),
        cs.clone(),
    );
    assert!(nad83.is_equivalent_to_with(&GeographicCRS::epsg_4269(), Equivalence::Equivalent));

    let ferro = db.create_geographic_crs(
        &geodetic_parts(
            Some("S-JTSK (Ferro)"),
            Some("System_Jednotne_Trigonometricke_Site_Katastralni_Ferro"),
            Some("Bessel 1841"),
            6_377_397.155,
            299.152_812_8,
            Some("Ferro"),
            -17.666_666_666_666_67,
        ),
        cs.clone(),
    );
    assert_eq!(
        ferro.datum().expect("datum").name(),
        "System of the Unified Trigonometrical Cadastral Network (Ferro)"
    );

    let deprecated = db.create_geographic_crs(
        &geodetic_parts(
            Some("SAD69 (deprecated)"),
            Some("South_American_Datum_1969"),
            Some("GRS 1967"),
            6_378_160.0,
            298.247_167_427,
            Some("Greenwich"),
            0.0,
        ),
        cs,
    );
    assert_eq!(deprecated.name(), "SAD69");
    assert!(Crs::Geographic(deprecated).is_deprecated());
}

#[test]
fn c_api_proj_create_geocentric_crs() {
    // CApi.proj_create_geocentric_crs
    let db = DatabaseContext::create().expect("proj.db");
    let parts = geodetic_parts(
        Some("WGS 84"),
        Some("World Geodetic System 1984"),
        Some("WGS 84"),
        6_378_137.0,
        298.257_223_563,
        Some("Greenwich"),
        0.0,
    );
    let crs = db.create_geocentric_crs(&parts, Some("Metre"), 1.0);
    assert!(crs.is_equivalent_to_with(&GeodeticCRS::epsg_4978(), Equivalence::Equivalent));
    let datum = crs.datum().expect("datum").clone();
    let from_datum = GeodeticCRS::create(
        "WGS 84",
        Some(datum),
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("datum");
    assert!(crs.is_equivalent_to(&from_datum));

    let unnamed = db.create_geocentric_crs(
        &geodetic_parts(None, None, None, 1.0, 0.0, None, 0.0),
        None,
        0.0,
    );
    assert_eq!(unnamed.name(), "unnamed");
}

fn wgs84_geographic(db: &DatabaseContext, name: &str, cs: EllipsoidalCS) -> GeographicCRS {
    db.create_geographic_crs(
        &geodetic_parts(
            Some(name),
            Some("World Geodetic System 1984"),
            Some("WGS 84"),
            6_378_137.0,
            298.257_223_563,
            Some("Greenwich"),
            0.0,
        ),
        cs,
    )
}

fn scale_parameter() -> (OperationParameter, ParameterValue) {
    (
        OperationParameter::create("param name"),
        ParameterValue::scale(0.99),
    )
}

#[test]
fn c_api_proj_create_projected_crs() {
    // CApi.proj_create_projected_crs
    let db = DatabaseContext::create().expect("proj.db");
    let (parameter, value) = scale_parameter();
    let conversion = Conversion::create(
        &OpProps::new()
            .name("conv")
            .identifier("conv auth", "conv code"),
        "method",
        vec![parameter],
        vec![value],
    )
    .expect("conversion");
    let geographic = wgs84_geographic(
        &db,
        "WGS 84",
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    );
    let projected = ProjectedCRS::create(
        "my CRS",
        GeodeticCRS::Geographic(geographic),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    assert_eq!(projected.name(), "my CRS");
}

#[test]
fn c_api_proj_create_transformation() {
    // CApi.proj_create_transformation
    let db = DatabaseContext::create().expect("proj.db");
    let cs = EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree());
    let source = Crs::Geographic(wgs84_geographic(&db, "Source CRS", cs.clone()));
    let target = Crs::Geographic(wgs84_geographic(&db, "WGS 84", cs.clone()));
    let interpolation = Crs::Geographic(wgs84_geographic(&db, "Interpolation CRS", cs));
    let (parameter, value) = scale_parameter();
    let transformation = Transformation::create(
        &OpProps::new()
            .name("transf")
            .identifier("transf auth", "transf code"),
        source.clone(),
        target.clone(),
        Some(interpolation),
        "method",
        vec![parameter],
        vec![value],
        Some(0.0),
    )
    .expect("transformation");
    assert_eq!(transformation.parameters().len(), 1);
    assert!(transformation
        .source_crs()
        .expect("source")
        .is_equivalent_to(&source));
    assert!(transformation
        .target_crs()
        .expect("target")
        .is_equivalent_to(&target));

    let (parameter, value) = scale_parameter();
    let without_interpolation = Transformation::create(
        &OpProps::new().name("transf"),
        source,
        target,
        None,
        "method",
        vec![parameter],
        vec![value],
        None,
    )
    .expect("accuracy below zero is absent");
    assert!(without_interpolation.interpolation_crs().is_none());
}

#[test]
fn c_api_proj_create_compound_crs() {
    // CApi.proj_create_compound_crs
    let db = DatabaseContext::create().expect("proj.db");
    let horizontal = Crs::Geographic(wgs84_geographic(
        &db,
        "WGS 84",
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    ));
    let vertical = Crs::Vertical(VerticalCRS::create(
        "myVertCRS",
        VerticalReferenceFrame::named("myVertDatum"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    ));
    assert_eq!(vertical.name(), "myVertCRS");
    let compound = CompoundCRS::create("myCompoundCRS", vec![horizontal.clone(), vertical.clone()])
        .expect("compound");
    assert_eq!(compound.name(), "myCompoundCRS");
    assert!(compound.components()[0].is_equivalent_to(&horizontal));
    assert!(compound.components()[1].is_equivalent_to(&vertical));
}

#[test]
fn c_api_proj_convert_conversion_to_other_method() {
    // CApi.proj_convert_conversion_to_other_method
    let db = DatabaseContext::create().expect("proj.db");
    let geographic = wgs84_geographic(
        &db,
        "WGS 84",
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    );
    let conversion =
        Conversion::create_mercator_variant_a(&OpProps::new(), 0.0, 1.0, 0.99, 2.0, 3.0)
            .expect("mercator A");
    let projected = ProjectedCRS::create(
        "my CRS",
        GeodeticCRS::Geographic(geographic),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    );
    let derived = projected.deriving_operation();
    assert!(derived.convert_to_other_method(0).is_none());
    let variant_b = derived
        .convert_to_other_method(9805)
        .expect("Mercator (variant B)");
    assert!(!variant_b.is_equivalent_to(derived.conversion()));
    assert!(variant_b.is_equivalent_to_with_source(
        derived.conversion(),
        derived.source_crs(),
        Equivalence::Equivalent
    ));
    let by_name = derived
        .convert_to_other_method(9805)
        .expect("method name Mercator (variant B)");
    assert!(variant_b.is_equivalent_to(&by_name));
    let back = derived
        .convert_to_other_method(9804)
        .expect("Mercator (variant A)");
    assert!(derived.conversion().is_equivalent_to(&back));
}

fn crs_rows(authority: &str) -> Vec<CrsInfo> {
    let db = DatabaseContext::create().expect("proj.db");
    AuthorityFactory::create(&db, authority)
        .get_crs_info_list()
        .expect("crs list")
}

fn type_selected(info: &CrsInfo, wanted: &[ObjectType]) -> bool {
    if wanted.is_empty() {
        return true;
    }
    let kind = info.object_type();
    wanted.iter().any(|want| {
        *want == kind
            || (*want == ObjectType::GeographicCrs
                && matches!(
                    kind,
                    ObjectType::Geographic2dCrs | ObjectType::Geographic3dCrs
                ))
            || (*want == ObjectType::GeodeticCrs
                && matches!(
                    kind,
                    ObjectType::GeocentricCrs
                        | ObjectType::Geographic2dCrs
                        | ObjectType::Geographic3dCrs
                ))
    })
}

fn bbox_of(info: &CrsInfo) -> Option<BoundingBox> {
    Some(BoundingBox::new(
        info.west_lon?,
        info.south_lat?,
        info.east_lon?,
        info.north_lat?,
    ))
}

#[test]
fn c_api_proj_get_crs_info_list_from_database() {
    // CApi.proj_get_crs_info_list_from_database
    let all = crs_rows("");
    assert!(all.first().is_some_and(|info| {
        !info.authority.is_empty() && !info.code.is_empty() && !info.name.is_empty()
    }));

    let epsg: Vec<_> = crs_rows("EPSG")
        .into_iter()
        .filter(|info| !info.deprecated)
        .collect();
    assert!(epsg.len() > 1);
    let mut found = [false; 6];
    for info in &epsg {
        assert_eq!(info.celestial_body_name, "Earth");
        match info.code.as_str() {
            "4326" => {
                found[0] = true;
                assert_eq!(info.authority, "EPSG");
                assert_eq!(info.name, "WGS 84");
                assert_eq!(info.object_type(), ObjectType::Geographic2dCrs);
                assert!(!info.deprecated);
                assert_eq!(info.west_lon, Some(-180.0));
                assert_eq!(info.south_lat, Some(-90.0));
                assert_eq!(info.east_lon, Some(180.0));
                assert_eq!(info.north_lat, Some(90.0));
                assert!(info.area_name.starts_with("World"));
                assert!(info.projection_method_name.is_empty());
            }
            "4978" => {
                found[1] = true;
                assert_eq!(info.object_type(), ObjectType::GeocentricCrs);
            }
            "4979" => {
                found[2] = true;
                assert_eq!(info.object_type(), ObjectType::Geographic3dCrs);
            }
            "32631" => {
                found[3] = true;
                assert_eq!(info.object_type(), ObjectType::ProjectedCrs);
                assert_eq!(info.projection_method_name, "Transverse Mercator");
            }
            "3855" => {
                found[4] = true;
                assert_eq!(info.object_type(), ObjectType::VerticalCrs);
            }
            "3901" => {
                found[5] = true;
                assert_eq!(info.object_type(), ObjectType::CompoundCrs);
            }
            _ => {}
        }
    }
    assert!(found.iter().all(|present| *present));

    let geodetic: Vec<_> = crs_rows("")
        .into_iter()
        .filter(|info| !info.deprecated && type_selected(info, &[ObjectType::GeodeticCrs]))
        .collect();
    assert!(geodetic
        .iter()
        .any(|info| info.object_type() == ObjectType::Geographic2dCrs));
    assert!(geodetic
        .iter()
        .any(|info| info.object_type() == ObjectType::Geographic3dCrs));
    assert!(geodetic
        .iter()
        .any(|info| info.object_type() == ObjectType::GeocentricCrs));
    assert!(geodetic
        .iter()
        .any(|info| info.object_type() == ObjectType::GeodeticCrs));

    let geographic: Vec<_> = epsg
        .iter()
        .filter(|info| type_selected(info, &[ObjectType::GeographicCrs]))
        .collect();
    assert!(geographic
        .iter()
        .any(|info| info.object_type() == ObjectType::Geographic2dCrs));
    assert!(geographic
        .iter()
        .any(|info| info.object_type() == ObjectType::Geographic3dCrs));
    assert!(geographic.iter().all(|info| {
        matches!(
            info.object_type(),
            ObjectType::Geographic2dCrs | ObjectType::Geographic3dCrs
        )
    }));

    let horizontal: Vec<_> = epsg
        .iter()
        .filter(|info| {
            type_selected(
                info,
                &[ObjectType::Geographic2dCrs, ObjectType::ProjectedCrs],
            )
        })
        .collect();
    assert!(horizontal
        .iter()
        .any(|info| info.object_type() == ObjectType::Geographic2dCrs));
    assert!(horizontal
        .iter()
        .any(|info| info.object_type() == ObjectType::ProjectedCrs));

    let query = BoundingBox::new(2.0, 49.0, 2.1, 49.1);
    let contained: Vec<_> = epsg
        .iter()
        .filter(|info| info.object_type() == ObjectType::ProjectedCrs)
        .filter(|info| bbox_of(info).is_some_and(|extent| extent.contains(&query)))
        .collect();
    assert!(contained.len() > 1);
    for info in &contained {
        if info.west_lon < info.east_lon {
            assert!(info.west_lon.unwrap_or(0.0) <= 2.0);
            assert!(info.east_lon.unwrap_or(0.0) >= 2.1);
        }
        assert!(info.south_lat.unwrap_or(0.0) <= 49.0);
        assert!(info.north_lat.unwrap_or(0.0) >= 49.1);
    }
    let intersecting: Vec<_> = epsg
        .iter()
        .filter(|info| info.object_type() == ObjectType::ProjectedCrs)
        .filter(|info| bbox_of(info).is_some_and(|extent| extent.intersects(&query)))
        .collect();
    assert!(intersecting.len() > 1);
    for info in &intersecting {
        if info.west_lon < info.east_lon {
            assert!(info.west_lon.unwrap_or(0.0) <= 2.0);
            assert!(info.east_lon.unwrap_or(0.0) >= 2.1);
        }
        assert!(info.south_lat.unwrap_or(0.0) <= 49.1);
        assert!(info.north_lat.unwrap_or(0.0) >= 49.0);
    }

    assert!(crs_rows("")
        .iter()
        .filter(|info| !info.deprecated)
        .all(|info| info.celestial_body_name != "non existing"));
    assert!(crs_rows("")
        .iter()
        .any(|info| info.celestial_body_name == "Earth"));
}

fn crs_authority_code(object: &Crs) -> Option<(&str, &str)> {
    object
        .identifiers()
        .first()
        .map(|identifier| (identifier.code_space(), identifier.code()))
}

#[test]
fn c_api_proj_identify() {
    // CApi.proj_identify
    // An ellipsoid is not a CRS, so it has no identification.
    let db = DatabaseContext::create().expect("proj.db");
    let any = AuthorityFactory::create(&db, "");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let ntf = Crs::Geographic(GeographicCRS::epsg_4807());
    assert_eq!(ntf.identify_with(Some(&any)).len(), 1);
    let found = ntf.identify_with(Some(&epsg));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, 100);

    let wgs84 = create_crs_from_proj_string("+proj=longlat +datum=WGS84 +no_defs +type=crs")
        .expect("WGS84");
    assert_eq!(wgs84.identify_with(Some(&any)).len(), 4);

    let ignf = AuthorityFactory::create(&db, "IGNF");
    let utm = ignf
        .create_coordinate_reference_system("ETRS89UTM28")
        .expect("ETRS89UTM28");
    let found = utm.identify_with(Some(&epsg));
    assert_eq!(found.len(), 1);
    assert_eq!(crs_authority_code(&found[0].0), Some(("EPSG", "25828")));
    assert_eq!(found[0].1, 70);
}

#[test]
fn c_api_proj_crs_create_bound_vertical_crs() {
    // CApi.proj_crs_create_bound_vertical_crs
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let vertical = Crs::Vertical(VerticalCRS::create(
        "myVertCRS",
        VerticalReferenceFrame::named("myVertDatum"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    ));
    let hub = epsg
        .create_coordinate_reference_system("4979")
        .expect("EPSG:4979");
    let transformation = Transformation::create(
        &OpProps::new().name("unknown to WGS 84 ellipsoidal height"),
        vertical.clone(),
        hub.clone(),
        None,
        "GravityRelatedHeight to Geographic3D",
        vec![OperationParameter::create(
            "Geoid (height correction) model file",
        )],
        vec![ParameterValue::filename("foo.gtx")],
        None,
    )
    .expect("geoid transformation");
    let bound = Crs::Bound(BoundCRS::create(
        vertical,
        hub,
        CoordinateOperation::Transformation(transformation),
    ));
    let horizontal = epsg
        .create_coordinate_reference_system("32631")
        .expect("EPSG:32631");
    let compound = CompoundCRS::create("myCompoundCRS", vec![horizontal, bound]).expect("compound");
    assert_eq!(
        Crs::Compound(compound)
            .export_to_proj_string()
            .expect("PROJ string"),
        "+proj=utm +zone=31 +datum=WGS84 +units=m +geoidgrids=foo.gtx +geoid_crs=WGS84 \
+vunits=m +no_defs +type=crs"
    );
}

#[test]
fn c_api_proj_crs_is_dynamic() {
    let dynamic = |code: &str| epsg_crs(code).is_dynamic();
    assert!(dynamic("9990"));
    assert!(!dynamic("4258"));
    assert!(dynamic("5613"));
    assert!(!dynamic("3855"));
    assert!(!dynamic("6871"));
    assert!(dynamic("5628"));
}
