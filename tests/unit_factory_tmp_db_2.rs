//! `FactoryWithTmpDatabase` tests of PROJ 9.9.0 `test/unit/test_factory.cpp`, on a
//! temporary `proj.db` (see `tests/common/tmp_db.rs`): `other_transformation` rows whose
//! method is `PROJ` (a PROJ string or a WKT `COORDINATEOPERATION`), the grid lookups of
//! `lookForGridInfo`, and the ordering of `createOperations` with a partial spatial criterion.
//!
//! Each function is one `TEST_F`. The rows are the C++ statements; the values are the
//! ones the C++ test asserts.

mod common;

use common::tmp_db::TmpDb;
use proj_rs::iso19111::{coordinate_metadata_from_urn, Equivalence};
use proj_rs::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, GeodeticCRS, ProjLines, SpatialCriterion, SupersededOperations,
};

/// An `other_transformation` row `code` of the authority `auth`, from the CRS `source` to the
/// CRS `target` (both `(authority, code)` pairs), with the method `method_code` `method_name`
/// and the given accuracy. The row has 74 columns: 12 values, 61 `NULL`s, then `deprecated`.
fn other_transformation_row(
    auth: &str,
    code: &str,
    name: &str,
    source: (&str, &str),
    target: (&str, &str),
    accuracy: &str,
    method: (&str, &str),
) -> String {
    format!(
        "INSERT INTO other_transformation VALUES('{auth}','{code}','{name}',NULL,'PROJ',\
         '{}','{}','{}','{}','{}','{}',{accuracy},{}0);",
        method.0,
        method.1,
        source.0,
        source.1,
        target.0,
        target.1,
        "NULL,".repeat(61)
    )
}

/// An `other_transformation` row of the `OTHER` authority with the method `PROJ`, from
/// `EPSG:4326` to `EPSG:4326`.
fn other_proj_row(
    code: &str,
    name: &str,
    accuracy: &str,
    method_code: &str,
    method_name: &str,
) -> String {
    other_transformation_row(
        "OTHER",
        code,
        name,
        ("EPSG", "4326"),
        ("EPSG", "4326"),
        accuracy,
        (method_code, method_name),
    )
}

/// The `usage` row of the `other_transformation` `code` of `OTHER`, in the extent `extent`.
fn other_usage(usage: &str, code: &str, extent: &str) -> String {
    format!(
        "INSERT INTO usage VALUES('OTHER','{usage}','other_transformation','OTHER','{code}',\
         'EPSG','{extent}','EPSG','1024');"
    )
}

/// The WKT of `AuthorityFactory_wkt_based_transformation`, with its line breaks.
const WKT_OPERATION: &str = "COORDINATEOPERATION[\"My WKT string based op\",\n\
    SOURCECRS[\n\
        GEODCRS[\"unknown\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
                    LENGTHUNIT[\"metre\",1]],\n\
                ID[\"EPSG\",6326]],\n\
            PRIMEM[\"Greenwich\",0,\n\
                ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                ID[\"EPSG\",8901]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"geodetic latitude (Lat)\",north],\n\
                AXIS[\"geodetic longitude (Lon)\",east],\n\
                ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
    TARGETCRS[\n\
        GEODCRS[\"unknown\",\n\
            DATUM[\"World Geodetic System 1984\",\n\
                ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
                    LENGTHUNIT[\"metre\",1]],\n\
                ID[\"EPSG\",6326]],\n\
            PRIMEM[\"Greenwich\",0,\n\
                ANGLEUNIT[\"degree\",0.0174532925199433],\n\
                ID[\"EPSG\",8901]],\n\
            CS[ellipsoidal,2],\n\
                AXIS[\"geodetic latitude (Lat)\",north],\n\
                AXIS[\"geodetic longitude (Lon)\",east],\n\
                ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
    METHOD[\"Geocentric translations (geog2D domain)\"],\n\
    PARAMETER[\"X-axis translation\",1,UNIT[\"metre\",1]],\n\
    PARAMETER[\"Y-axis translation\",2,UNIT[\"metre\",1]],\n\
    PARAMETER[\"Z-axis translation\",3,UNIT[\"metre\",1]]]";

/// `createFromCoordinateReferenceSystemCodes` of the `OTHER` authority for `EPSG:4326` to
/// `EPSG:4326`, after the rows of the test are written.
fn operations_of_foo(tmp: &TmpDb) -> Result<Vec<proj_rs::CoordinateOperation>, String> {
    let db = tmp.database();
    let factory = AuthorityFactory::create(&db, "OTHER");
    factory
        .create_from_coordinate_reference_system_codes_in(
            ("EPSG", "4326"),
            ("EPSG", "4326"),
            SupersededOperations::Keep,
        )
        .map_err(|error| error.to_string())
}

#[test]
fn factory_with_tmp_database_authority_factory_proj_based_transformation() {
    let tmp = TmpDb::new("proj-based");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute(&other_proj_row(
        "FOO",
        "My PROJ string based op",
        "0.0",
        "PROJString",
        "+proj=pipeline +ellps=WGS84 +step +proj=longlat",
    ));
    tmp.execute(&other_usage(
        "other_transformation_FOO_usage",
        "FOO",
        "1262",
    ));

    let res = operations_of_foo(&tmp).expect("operations");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name(), "My PROJ string based op");
    let db = tmp.database();
    assert_eq!(
        res[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +ellps=WGS84 +step +proj=longlat"
    );
}

#[test]
fn factory_with_tmp_database_authority_factory_wkt_based_transformation() {
    let tmp = TmpDb::new("wkt-based");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute(&other_proj_row(
        "FOO",
        "My WKT string based op",
        "0.0",
        "WKT",
        WKT_OPERATION,
    ));
    tmp.execute(&other_usage(
        "other_transformation_FOO_usage",
        "FOO",
        "1262",
    ));

    let res = operations_of_foo(&tmp).expect("operations");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name(), "My WKT string based op");
    let db = tmp.database();
    assert_eq!(
        res[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step \
         +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 \
         +step +proj=cart +ellps=WGS84 +step +proj=helmert +x=1 +y=2 \
         +z=3 +step +inv +proj=cart +ellps=WGS84 +step +proj=pop +v_3 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg +step \
         +proj=axisswap +order=2,1"
    );
}

#[test]
fn factory_with_tmp_database_authority_factory_wkt_based_transformation_not_wkt() {
    let tmp = TmpDb::new("wkt-not-wkt");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute(&other_proj_row(
        "FOO",
        "My WKT string based op",
        "0.0",
        "WKT",
        "invalid_wkt",
    ));
    tmp.execute(&other_usage(
        "other_transformation_FOO_usage",
        "FOO",
        "1262",
    ));

    assert!(operations_of_foo(&tmp).is_err());
}

#[test]
fn factory_with_tmp_database_authority_factory_wkt_based_transformation_not_co_wkt() {
    let tmp = TmpDb::new("wkt-not-co-wkt");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute(&other_proj_row(
        "FOO",
        "My WKT string based op",
        "0.0",
        "WKT",
        "LOCAL_CS[\"foo\"]",
    ));
    tmp.execute(&other_usage(
        "other_transformation_FOO_usage",
        "FOO",
        "1262",
    ));

    assert!(operations_of_foo(&tmp).is_err());
}

/// `grid_alternatives` row of `lookForGridInfo`, the grid `PROJ_fake_grid` not yet in a grid
/// transformation.
const GRID_FAKE: &str = "INSERT INTO grid_alternatives(original_grid_name, \
     proj_grid_name, old_proj_grid_name, proj_grid_format, proj_method, inverse_direction, \
     package_name, url, direct_download, open_license, directory) VALUES (\
     'NOT-YET-IN-GRID-TRANSFORMATION-PROJ_fake_grid', 'PROJ_fake_grid', 'old_PROJ_fake_grid', \
     'NTv2', 'hgridshift', 0, NULL, 'url', 1, 1, NULL);";

/// `grid_alternatives` row of `lookForGridInfo_from_old_name_with_new_grid_available`: the
/// file `tests/egm96_15_uncompressed_truncated.tif` replaces the old name `old_name.gtx`.
const GRID_OLD_NAME: &str = "INSERT INTO grid_alternatives(original_grid_name, \
     proj_grid_name, old_proj_grid_name, proj_grid_format, proj_method, inverse_direction, \
     package_name, url, direct_download, open_license, directory) VALUES (\
     'NOT-YET-IN-GRID-TRANSFORMATION-original_grid_name', \
     'tests/egm96_15_uncompressed_truncated.tif', 'old_name.gtx', 'NTv2', 'hgridshift', 0, \
     NULL, 'url', 1, 1, NULL);";

#[test]
fn factory_with_tmp_database_look_for_grid_info() {
    common::use_fixture_search_path();
    let tmp = TmpDb::new("grid-info");
    tmp.create_structure();
    tmp.execute(GRID_FAKE);

    let info = tmp
        .database()
        .grid_info("PROJ_fake_grid")
        .expect("grid info")
        .expect("a row for PROJ_fake_grid");
    assert!(info.full_name.is_empty());
    assert!(info.package_name.is_empty());
    assert_eq!(info.url, "url");
    assert!(info.direct_download);
    assert!(info.open_license);
    assert!(!info.available);
}

#[test]
fn factory_with_tmp_database_look_for_grid_info_from_old_name_with_new_grid_available() {
    common::use_fixture_search_path();
    let tmp = TmpDb::new("grid-info-old-name");
    tmp.create_structure();
    tmp.execute(GRID_OLD_NAME);

    let info = tmp
        .database()
        .grid_info("old_name.gtx")
        .expect("grid info")
        .expect("a row for old_name.gtx");
    assert!(
        info.full_name
            .contains("tests/egm96_15_uncompressed_truncated.tif"),
        "{}",
        info.full_name
    );
    assert!(info.available);
}

#[test]
fn factory_with_tmp_database_create_operations_exact_transform_not_whole_area() {
    let tmp = TmpDb::new("partial-area");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute(&other_proj_row(
        "PARTIAL_AREA_PERFECT_ACCURACY",
        "PARTIAL_AREA_PERFECT_ACCURACY",
        "0.0",
        "PROJString",
        "+proj=helmert +x=1",
    ));
    tmp.execute(&other_usage("1", "PARTIAL_AREA_PERFECT_ACCURACY", "1933"));
    tmp.execute(&other_proj_row(
        "WHOLE_AREA_APPROX_ACCURACY",
        "WHOLE_AREA_APPROX_ACCURACY",
        "1.0",
        "PROJString",
        "+proj=helmert +x=2",
    ));
    tmp.execute(&other_usage("2", "WHOLE_AREA_APPROX_ACCURACY", "1262"));

    let db = tmp.database();
    let mut context = CoordinateOperationContext::create(&db, "OTHER");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let source = epsg
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326");
    let target = epsg
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name(), "WHOLE_AREA_APPROX_ACCURACY");
    assert_eq!(list[1].name(), "PARTIAL_AREA_PERFECT_ACCURACY");
}

#[test]
fn factory_with_tmp_database_check_fixup_direction_concatenated_inverse_map_projection() {
    // Issue 2817 of the PROJ tracker.
    let tmp = TmpDb::new("fixup-direction");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute(&other_transformation_row(
        "EPSG",
        "NOOP_TRANSFORMATION_32631",
        "NOOP_TRANSFORMATION_32631",
        ("EPSG", "32631"),
        ("EPSG", "32631"),
        "0.0",
        ("PROJString", "+proj=noop"),
    ));
    tmp.execute(
        "INSERT INTO usage VALUES('EPSG','other_transformation_NOOP_TRANSFORMATION_32631_usage',\
         'other_transformation','EPSG','NOOP_TRANSFORMATION_32631','EPSG','1262','EPSG','1024');",
    );
    tmp.execute(&other_transformation_row(
        "EPSG",
        "NOOP_TRANSFORMATION_4326",
        "NOOP_TRANSFORMATION_4326",
        ("EPSG", "4326"),
        ("EPSG", "4326"),
        "0.0",
        ("PROJString", "+proj=noop"),
    ));
    tmp.execute(
        "INSERT INTO usage VALUES('EPSG','other_transformation_NOOP_TRANSFORMATION_4326_usage',\
         'other_transformation','EPSG','NOOP_TRANSFORMATION_4326','EPSG','1262','EPSG','1024');",
    );
    tmp.execute(
        "INSERT INTO concatenated_operation VALUES('EPSG','TEST_CONCATENATED','name',NULL,\
         'EPSG','4326','EPSG','4326',NULL,NULL,0);",
    );
    tmp.execute(
        "INSERT INTO usage VALUES('EPSG','concatenated_operation_TEST_CONCATENATED_usage',\
         'concatenated_operation','EPSG','TEST_CONCATENATED','EPSG','1262','EPSG','1024');",
    );
    // Forward map projection, noop projected, inverse map projection, noop geographic,
    // forward map projection, noop projected, inverse map projection.
    let steps = [
        "16031",
        "NOOP_TRANSFORMATION_32631",
        "16031",
        "NOOP_TRANSFORMATION_4326",
        "16031",
        "NOOP_TRANSFORMATION_32631",
        "16031",
    ];
    for (index, code) in steps.iter().enumerate() {
        tmp.execute(&format!(
            "INSERT INTO concatenated_operation_step VALUES('EPSG','TEST_CONCATENATED',{},\
             'EPSG','{code}',NULL);",
            index + 1
        ));
    }

    let db = tmp.database();
    let factory = AuthorityFactory::create(&db, "EPSG");
    let op = factory
        .create_coordinate_operation("TEST_CONCATENATED", AlternativeGridNames::Official)
        .expect("TEST_CONCATENATED");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=noop"
    );
}

/// The `geodetic_crs` rows of `custom_geodetic_crs`, in the authority `TEST_NS`.
const CUSTOM_GEODETIC_INSERTS: &[&str] = &[
    "INSERT INTO geodetic_crs VALUES('TEST_NS','TEST','my name TEST',NULL,'geographic 2D',NULL,NULL,NULL,NULL,'+proj=longlat +a=2 +rf=300',0);",
    "INSERT INTO geodetic_crs VALUES('TEST_NS','TEST_BOUND','my name TEST',NULL,'geographic 2D',NULL,NULL,NULL,NULL,'+proj=longlat +a=2 +rf=300 +towgs84=1,2,3',0);",
    "INSERT INTO geodetic_crs VALUES('TEST_NS','TEST_GC','my name',NULL,'geocentric',NULL,NULL,NULL,NULL,'+proj=geocent +a=2 +rf=300',0);",
    "INSERT INTO geodetic_crs VALUES('TEST_NS','TEST_REF_ANOTHER','my name TEST_REF_ANOTHER',NULL,'geographic 2D',NULL,NULL,NULL,NULL,'TEST_NS:TEST',0);",
    "INSERT INTO geodetic_crs VALUES('TEST_NS','TEST_WRONG','my name',NULL,'geographic 2D',NULL,NULL,NULL,NULL,'+proj=merc',0);",
    "INSERT INTO geodetic_crs VALUES('TEST_NS','TEST_RECURSIVE','my name',NULL,'geographic 2D',NULL,NULL,NULL,NULL,'TEST_NS:TEST_RECURSIVE',0);",
];

#[test]
fn factory_with_tmp_database_custom_geodetic_crs() {
    let tmp = TmpDb::new("custom-geodetic");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute_all(CUSTOM_GEODETIC_INSERTS);

    let db = tmp.database();
    let factory = AuthorityFactory::create(&db, "TEST_NS");
    {
        let crs = factory.create_geodetic_crs("TEST").expect("TEST");
        assert!(matches!(crs, GeodeticCRS::Geographic(_)));
        assert_eq!(crs.name(), "my name TEST");
        assert_eq!(crs.identifiers().len(), 1);
        let ellipsoid = crs.ellipsoid().expect("ellipsoid");
        assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
        assert_eq!(ellipsoid.inverse_flattening(), 300.0);
        assert!(Crs::Geodetic(crs).canonical_bound_crs().is_none());
    }
    {
        let crs = factory
            .create_geodetic_crs("TEST_BOUND")
            .expect("TEST_BOUND");
        assert!(matches!(crs, GeodeticCRS::Geographic(_)));
        assert_eq!(crs.name(), "my name TEST");
        assert_eq!(crs.identifiers().len(), 1);
        let ellipsoid = crs.ellipsoid().expect("ellipsoid");
        assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
        assert_eq!(ellipsoid.inverse_flattening(), 300.0);
        assert!(Crs::Geodetic(crs).canonical_bound_crs().is_some());
    }
    {
        let crs = factory.create_geodetic_crs("TEST_GC").expect("TEST_GC");
        assert!(matches!(crs, GeodeticCRS::Geocentric(_)));
        assert_eq!(crs.name(), "my name");
        assert_eq!(crs.identifiers().len(), 1);
        let ellipsoid = crs.ellipsoid().expect("ellipsoid");
        assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
        assert_eq!(ellipsoid.inverse_flattening(), 300.0);
    }
    {
        let crs = factory
            .create_geodetic_crs("TEST_REF_ANOTHER")
            .expect("TEST_REF_ANOTHER");
        assert!(matches!(crs, GeodeticCRS::Geographic(_)));
        assert_eq!(crs.name(), "my name TEST_REF_ANOTHER");
        assert_eq!(crs.identifiers().len(), 1);
        let ellipsoid = crs.ellipsoid().expect("ellipsoid");
        assert_eq!(ellipsoid.semi_major_axis().si_value(), 2.0);
        assert_eq!(ellipsoid.inverse_flattening(), 300.0);
    }
    assert!(factory.create_geodetic_crs("TEST_WRONG").is_err());
    assert!(factory.create_geodetic_crs("TEST_RECURSIVE").is_err());
}

/// The `TEST2` row of `CoordinateMetadata`: a CRS given as a WKT string.
const COORDINATE_METADATA_WKT: &str = "GEOGCRS[\"WGS 84\",\n\
    ENSEMBLE[\"World Geodetic System 1984 ensemble\",\n\
        MEMBER[\"World Geodetic System 1984 (Transit)\"],\n\
        MEMBER[\"World Geodetic System 1984 (G730)\"],\n\
        MEMBER[\"World Geodetic System 1984 (G873)\"],\n\
        MEMBER[\"World Geodetic System 1984 (G1150)\"],\n\
        MEMBER[\"World Geodetic System 1984 (G1674)\"],\n\
        MEMBER[\"World Geodetic System 1984 (G1762)\"],\n\
        MEMBER[\"World Geodetic System 1984 (G2139)\"],\n\
        ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
            LENGTHUNIT[\"metre\",1]],\n\
        ENSEMBLEACCURACY[2.0]],\n\
    PRIMEM[\"Greenwich\",0,\n\
        ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
    CS[ellipsoidal,2],\n\
        AXIS[\"geodetic latitude (Lat)\",north,\n\
            ORDER[1],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
        AXIS[\"geodetic longitude (Lon)\",east,\n\
            ORDER[2],\n\
            ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
    USAGE[\n\
        SCOPE[\"Horizontal component of 3D system.\"],\n\
        AREA[\"World.\"],\n\
        BBOX[-90,-180,90,180]],\n\
    ID[\"EPSG\",4326]]";

fn assert_epoch(found: Option<f64>, expected: f64) {
    let found = found.expect("a coordinate epoch");
    assert!((found - expected).abs() < 1e-10, "{found} != {expected}");
}

#[test]
fn factory_with_tmp_database_coordinate_metadata() {
    let tmp = TmpDb::new("coordinate-metadata");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute("INSERT INTO coordinate_metadata VALUES('TEST_NS','TEST','my desc','EPSG',4326,NULL,2020.1,0);");
    tmp.execute(&format!(
        "INSERT INTO coordinate_metadata VALUES('TEST_NS','TEST2','my desc',NULL,NULL,'{COORDINATE_METADATA_WKT}',2021.1,0);"
    ));
    tmp.execute("INSERT INTO coordinate_metadata VALUES('TEST_NS','TEST_NO_EPOCH','my desc','EPSG',4326,NULL,NULL,0);");

    let db = tmp.database();
    let crs_4326 = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326");
    let factory = AuthorityFactory::create(&db, "TEST_NS");
    {
        let metadata = factory.create_coordinate_metadata("TEST").expect("TEST");
        assert!(metadata.crs().is_equivalent_to(&crs_4326));
        assert_epoch(metadata.coordinate_epoch(), 2020.1);
    }
    {
        let metadata = factory.create_coordinate_metadata("TEST2").expect("TEST2");
        assert!(metadata
            .crs()
            .is_equivalent_to_with(&crs_4326, Equivalence::Equivalent));
        assert_epoch(metadata.coordinate_epoch(), 2021.1);
    }
    {
        let metadata = factory
            .create_coordinate_metadata("TEST_NO_EPOCH")
            .expect("TEST_NO_EPOCH");
        assert!(metadata.crs().is_equivalent_to(&crs_4326));
        assert!(metadata.coordinate_epoch().is_none());
    }
    {
        let metadata =
            coordinate_metadata_from_urn("urn:ogc:def:coordinateMetadata:TEST_NS::TEST", &db)
                .expect("URN")
                .expect("a coordinate metadata");
        assert!(metadata.crs().is_equivalent_to(&crs_4326));
        assert_epoch(metadata.coordinate_epoch(), 2020.1);
    }
}
