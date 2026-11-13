//! `FactoryWithTmpDatabase` tests of PROJ 9.9.0 `test/unit/test_factory.cpp`:
//! the ones that run on a temporary `proj.db`, after `createStructure` and
//! `populateWithFakeEPSG` (see `tests/common/tmp_db.rs`).
//!
//! Each function is one `TEST_F`. The inserts are the C++ statements, in their
//! order. Values are the ones the C++ test asserts.

mod common;

use common::tmp_db::TmpDb;
use proj_rs::{
    AuthorityFactory, AuthorityObject, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, Datum, IntermediateCrsUse, SupersededOperations,
};

/// `AuthorityFactory_test_inversion_first_and_last_steps_of_concat_op`, the rows after `populateWithFakeEPSG`.
const INVERSION_INSERTS: &[&str] = &[
    r#"INSERT INTO geodetic_datum VALUES('EPSG','OTHER_DATUM','Other datum','','EPSG','7030','EPSG','8901',NULL,NULL,NULL,'my anchor',NULL,0);"#,
    r#"INSERT INTO geodetic_crs VALUES('EPSG','OTHER_GEOG_CRS','OTHER_GEOG_CRS',NULL,'geographic 2D','EPSG','6422','EPSG','OTHER_DATUM',NULL,0);"#,
    r#"INSERT INTO other_transformation VALUES('EPSG','4326_TO_OTHER_GEOG_CRS','4326_to_other_geog_crs',NULL,'EPSG','9601','Longitude rotation','EPSG','4326','EPSG','OTHER_GEOG_CRS',0.0,'EPSG','8602','Longitude offset',-17.4,'EPSG','9110',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO other_transformation VALUES('EPSG','OTHER_GEOG_CRS_TO_4326','other_geog_crs_to_4326',NULL,'EPSG','9601','Longitude rotation','EPSG','OTHER_GEOG_CRS','EPSG','4326',0.0,'EPSG','8602','Longitude offset',17.4,'EPSG','9110',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO concatenated_operation VALUES('EPSG','DUMMY_CONCATENATED_2','dummy_concatenated_2',NULL,'EPSG','4326','EPSG','4326',NULL,NULL,0);"#,
    r#"INSERT INTO concatenated_operation_step VALUES('EPSG','DUMMY_CONCATENATED_2',1,'EPSG','OTHER_GEOG_CRS_TO_4326',NULL);"#,
    r#"INSERT INTO concatenated_operation_step VALUES('EPSG','DUMMY_CONCATENATED_2',2,'EPSG','4326_TO_OTHER_GEOG_CRS',NULL);"#,
];

/// `AuthorityFactory_test_with_fake_EPSG_and_OTHER_database`: the first CRS rows of the `OTHER` authority.
const OTHER_CRS_INSERTS: &[&str] = &[
    r#"INSERT INTO geodetic_crs VALUES('OTHER','OTHER_4326','WGS 84',NULL,'geographic 2D','EPSG','6422','EPSG','6326',NULL,0);"#,
    r#"INSERT INTO usage VALUES('OTHER','geodetic_crs_OTHER_4326_usage','geodetic_crs','OTHER','OTHER_4326','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO projected_crs VALUES('OTHER','OTHER_32631','my WGS 84 / UTM zone 31N',NULL,'EPSG','4400','OTHER','OTHER_4326','EPSG','16031',NULL,0);"#,
    r#"INSERT INTO usage VALUES('OTHER','projected_crs_OTHER_32631_usage','projected_crs','OTHER','OTHER_32631','EPSG','2060','EPSG','1024');"#,
];

/// `AuthorityFactory_test_with_fake_EPSG_and_OTHER_database`: the grid transformation added after the first checks.
const OTHER_GRID_INSERTS: &[&str] = &[
    r#"INSERT INTO grid_transformation VALUES('OTHER','OTHER_GRID_TRANSFORMATION','other_grid_transformation_2',NULL,'EPSG','9615','NTv2','EPSG','4326','OTHER','OTHER_4326',1.0,'EPSG','8656','Latitude and longitude difference file','nzgd2kgrid0005.gsb',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('OTHER','grid_transformation_OTHER_GRID_TRANSFORMATION_usage','grid_transformation','OTHER','OTHER_GRID_TRANSFORMATION','EPSG','1262','EPSG','1024');"#,
];

/// `AuthorityFactory_test_sorting_of_coordinate_operations`: the grid transformations of `OTHER`.
const SORTING_INSERTS: &[&str] = &[
    r#"INSERT INTO grid_transformation VALUES('OTHER','TRANSFORMATION_10M','TRANSFORMATION_10M',NULL,'EPSG','9615','NTv2','EPSG','4326','EPSG','4326',10.0,'EPSG','8656','Latitude and longitude difference file','nzgd2kgrid0005.gsb',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('OTHER','grid_transformation_TTRANSFORMATION_10M_usage','grid_transformation','OTHER','TRANSFORMATION_10M','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO grid_transformation VALUES('OTHER','TRANSFORMATION_1M_SMALL_EXTENT','TRANSFORMATION_1M_SMALL_EXTENT',NULL,'EPSG','9615','NTv2','EPSG','4326','EPSG','4326',1.0,'EPSG','8656','Latitude and longitude difference file','nzgd2kgrid0005.gsb',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('OTHER','grid_transformation_TRANSFORMATION_1M_SMALL_EXTENT_usage','grid_transformation','OTHER','TRANSFORMATION_1M_SMALL_EXTENT','EPSG','2060','EPSG','1024');"#,
    r#"INSERT INTO grid_transformation VALUES('OTHER','TRANSFORMATION_1M','TRANSFORMATION_1M',NULL,'EPSG','9615','NTv2','EPSG','4326','EPSG','4326',1.0,'EPSG','8656','Latitude and longitude difference file','nzgd2kgrid0005.gsb',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"#,
    r#"INSERT INTO usage VALUES('OTHER','grid_transformation_TRANSFORMATION_1M_usage','grid_transformation','OTHER','TRANSFORMATION_1M','EPSG','1262','EPSG','1024');"#,
    r#"INSERT INTO grid_transformation VALUES('OTHER','TRANSFORMATION_0.5M_DEPRECATED','TRANSFORMATION_0.5M_DEPRECATED',NULL,'EPSG','9615','NTv2','EPSG','4326','EPSG','4326',1.0,'EPSG','8656','Latitude and longitude difference file','nzgd2kgrid0005.gsb',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,1);"#,
    r#"INSERT INTO usage VALUES('OTHER','grid_transformation_TRANSFORMATION_0.5M_DEPRECATED_usage','grid_transformation','OTHER','TRANSFORMATION_0.5M_DEPRECATED','EPSG','1262','EPSG','1024');"#,
];

#[test]
fn factory_with_tmp_database_authority_factory_test_with_fake_epsg_database() {
    let tmp = TmpDb::new("fake-epsg");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    let db = tmp.database();
    let factory = AuthorityFactory::create(&db, "EPSG");
    let object = |code: &str| factory.create_object(code).expect(code);

    assert!(matches!(object("9001"), AuthorityObject::Unit(_)));
    assert!(matches!(object("1262"), AuthorityObject::Extent(_)));
    assert!(matches!(object("8901"), AuthorityObject::PrimeMeridian(_)));
    assert!(matches!(object("7030"), AuthorityObject::Ellipsoid(_)));
    match object("6326") {
        AuthorityObject::Datum(Datum::Geodetic(frame)) => {
            assert_eq!(frame.anchor(), Some("my anchor"));
        }
        _ => panic!("6326 is not a geodetic reference frame"),
    }
    match object("1027") {
        AuthorityObject::Datum(Datum::Vertical(frame)) => {
            assert_eq!(frame.anchor(), Some("my anchor"));
        }
        _ => panic!("1027 is not a vertical reference frame"),
    }
    assert!(matches!(
        object("4326"),
        AuthorityObject::Crs(Crs::Geographic(_))
    ));
    assert!(matches!(
        object("3855"),
        AuthorityObject::Crs(Crs::Vertical(_))
    ));
    assert!(matches!(
        object("16031"),
        AuthorityObject::Operation(CoordinateOperation::Conversion(_))
    ));
    assert!(matches!(
        object("32631"),
        AuthorityObject::Crs(Crs::Projected(_))
    ));
    assert!(matches!(
        object("MY_COMPOUND"),
        AuthorityObject::Crs(Crs::Compound(_))
    ));
    for code in [
        "DUMMY_HELMERT",
        "DUMMY_GRID_TRANSFORMATION",
        "DUMMY_OTHER_TRANSFORMATION",
    ] {
        assert!(matches!(
            object(code),
            AuthorityObject::Operation(CoordinateOperation::Transformation(_))
        ));
    }
    assert!(matches!(
        object("DUMMY_CONCATENATED"),
        AuthorityObject::Operation(CoordinateOperation::Concatenated(_))
    ));
}

#[test]
fn factory_with_tmp_database_authority_factory_test_inversion_first_and_last_steps_of_concat_op() {
    let tmp = TmpDb::new("inversion");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute_all(INVERSION_INSERTS);
    let db = tmp.database();
    let factory = AuthorityFactory::create(&db, "EPSG");
    assert!(matches!(
        factory.create_object("DUMMY_CONCATENATED_2"),
        Ok(AuthorityObject::Operation(
            CoordinateOperation::Concatenated(_)
        ))
    ));
}

#[test]
fn factory_with_tmp_database_authority_factory_test_with_fake_epsg_and_other_database() {
    let tmp = TmpDb::new("other");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute_all(OTHER_CRS_INSERTS);

    let db = tmp.database();
    let general = AuthorityFactory::create(&db, "");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let other = AuthorityFactory::create(&db, "OTHER");
    let keep = SupersededOperations::Keep;
    assert_eq!(
        general
            .create_from_coordinate_reference_system_codes_in(
                ("OTHER", "OTHER_4326"),
                ("OTHER", "OTHER_32631"),
                keep
            )
            .expect("general")
            .len(),
        1
    );
    assert_eq!(
        epsg.create_from_coordinate_reference_system_codes_in(
            ("OTHER", "OTHER_4326"),
            ("OTHER", "OTHER_32631"),
            keep
        )
        .expect("EPSG")
        .len(),
        1
    );
    // the conversion is in the EPSG space
    assert_eq!(
        other
            .create_from_coordinate_reference_system_codes("OTHER_4326", "OTHER_32631")
            .expect("OTHER")
            .len(),
        0
    );

    tmp.execute_all(OTHER_GRID_INSERTS);
    let db = tmp.database();
    let general = AuthorityFactory::create(&db, "");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let other = AuthorityFactory::create(&db, "OTHER");
    assert_eq!(
        general
            .create_from_coordinate_reference_system_codes_in(
                ("EPSG", "4326"),
                ("OTHER", "OTHER_4326"),
                keep
            )
            .expect("general, grid")
            .len(),
        1
    );
    assert_eq!(
        epsg.create_from_coordinate_reference_system_codes_in(
            ("EPSG", "4326"),
            ("OTHER", "OTHER_4326"),
            keep
        )
        .expect("EPSG, grid")
        .len(),
        0
    );
    assert_eq!(
        other
            .create_from_coordinate_reference_system_codes_in(
                ("EPSG", "4326"),
                ("OTHER", "OTHER_4326"),
                keep
            )
            .expect("OTHER, grid")
            .len(),
        1
    );
}

#[test]
fn factory_with_tmp_database_authority_factory_test_sorting_of_coordinate_operations() {
    let tmp = TmpDb::new("sorting");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    tmp.execute_all(SORTING_INSERTS);
    let db = tmp.database();
    let other = AuthorityFactory::create(&db, "OTHER");
    let res = other
        .create_from_coordinate_reference_system_codes_in(
            ("EPSG", "4326"),
            ("EPSG", "4326"),
            SupersededOperations::Keep,
        )
        .expect("sorted operations");
    let names: Vec<&str> = res.iter().map(CoordinateOperation::name).collect();
    assert_eq!(
        names,
        [
            "TRANSFORMATION_1M",
            "TRANSFORMATION_10M",
            "TRANSFORMATION_1M_SMALL_EXTENT",
        ]
    );
}

#[test]
fn factory_with_tmp_database_get_authorities() {
    let tmp = TmpDb::new("authorities");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    let authorities = tmp.database().authorities().expect("authorities");
    assert_eq!(authorities.len(), 2, "{authorities:?}");
    assert!(authorities.iter().any(|name| name == "EPSG"));
    assert!(authorities.iter().any(|name| name == "PROJ"));
}

/// `createSourceTargetPivotCRS` of `test_factory.cpp`: the datums and the geographic CRS
/// `NS_SOURCE`, `NS_TARGET` and `NS_PIVOT` that the pivot tests route through.
fn create_source_target_pivot_crs(tmp: &TmpDb) {
    for val in ["SOURCE", "TARGET", "PIVOT"] {
        tmp.execute(&format!(
            "INSERT INTO geodetic_datum VALUES('FOO','{val}','{val}','','EPSG','7030','EPSG','8901',\
             NULL,NULL,NULL,NULL,NULL,0);"
        ));
        tmp.execute(&format!(
            "INSERT INTO usage VALUES('FOO','geodetic_datum_{val}_usage','geodetic_datum',\
             'FOO','{val}','EPSG','1262','EPSG','1024');"
        ));
        tmp.execute(&format!(
            "INSERT INTO geodetic_crs VALUES('NS_{val}','{val}','{val}',NULL,'geographic 2D',\
             'EPSG','6422','FOO','{val}',NULL,0);"
        ));
        tmp.execute(&format!(
            "INSERT INTO usage VALUES('FOO','geodetic_crs_{val}_usage','geodetic_crs',\
             'NS_{val}','{val}','EPSG','1262','EPSG','1024');"
        ));
    }
}

/// `createTransformationForPivotTesting` of `test_factory.cpp`: a Helmert transformation
/// from `NS_{src}` to `NS_{dst}` in the `OTHER` authority.
fn create_transformation_for_pivot_testing(tmp: &TmpDb, src: &str, dst: &str) {
    tmp.execute(&format!(
        "INSERT INTO helmert_transformation VALUES('OTHER','{src}_{dst}',\
         'Transformation from {src} to {dst}',NULL,'EPSG','9603',\
         'Geocentric translations (geog2D domain)','NS_{src}','{src}','NS_{dst}','{dst}',\
         1.0,0,0,0,'EPSG','9001',\
         NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,0);"
    ));
    tmp.execute(&format!(
        "INSERT INTO usage VALUES('OTHER','helmert_transformation{src}_{dst}_usage',\
         'helmert_transformation','OTHER','{src}_{dst}','EPSG','1262','EPSG','1024');"
    ));
}

/// The kind of operation the C++ test asserts for a single result.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// `ConcatenatedOperation`.
    Concatenated,
    /// `Transformation`, the ballpark one of PROJ when no pivot is used.
    Transformation,
}

/// The one operation of `res`, checked to be of the kind `C++` asserts.
fn assert_one_operation(res: &[CoordinateOperation], kind: Kind) {
    assert_eq!(res.len(), 1);
    let kind_ok = match kind {
        Kind::Concatenated => matches!(res[0], CoordinateOperation::Concatenated(_)),
        Kind::Transformation => matches!(res[0], CoordinateOperation::Transformation(_)),
    };
    assert!(kind_ok, "expected {kind:?}, got {}", res[0].name());
}

/// `checkSourceToOther` of `test_factory.cpp`: the operations from `NS_SOURCE` to
/// `NS_TARGET`, through the pivot list of the `OTHER` authority, then through the general
/// one, and through a `CoordinateOperationContext` with a pivot list or `NEVER`.
fn check_source_to_other(tmp: &TmpDb) {
    let db = tmp.database();
    {
        let other = AuthorityFactory::create(&db, "OTHER");
        let source = ("NS_SOURCE", "SOURCE");
        let target = ("NS_TARGET", "TARGET");
        let res = other
            .create_from_crs_codes_with_intermediates(source, target, &[])
            .expect("OTHER");
        assert_one_operation(&res, Kind::Concatenated);

        let res = other
            .create_from_crs_codes_with_intermediates(source, target, &[("NS_PIVOT", "PIVOT")])
            .expect("OTHER, pivot");
        assert_one_operation(&res, Kind::Concatenated);

        let res = other
            .create_from_crs_codes_with_intermediates(
                source,
                target,
                &[("NS_PIVOT", "NOT_EXISTING")],
            )
            .expect("OTHER, unknown pivot");
        assert_eq!(res.len(), 0);

        let res = other
            .create_from_crs_codes_with_intermediates(source, target, &[("BAD_NS", "PIVOT")])
            .expect("OTHER, bad authority");
        assert_eq!(res.len(), 0);

        let res = other
            .create_from_crs_codes_with_intermediates(target, source, &[])
            .expect("OTHER, reversed");
        assert_one_operation(&res, Kind::Concatenated);
    }
    {
        let general = AuthorityFactory::create(&db, "");
        let res = general
            .create_from_crs_codes_with_intermediates(
                ("NS_SOURCE", "SOURCE"),
                ("NS_TARGET", "TARGET"),
                &[],
            )
            .expect("general");
        assert_one_operation(&res, Kind::Concatenated);

        let source = AuthorityFactory::create(&db, "NS_SOURCE")
            .create_coordinate_reference_system("SOURCE")
            .expect("source CRS");
        let target = AuthorityFactory::create(&db, "NS_TARGET")
            .create_coordinate_reference_system("TARGET")
            .expect("target CRS");
        let operations = CoordinateOperationFactory::create();

        let context = CoordinateOperationContext::create(&db, "");
        let res = operations
            .create_operations(&source, &target, &context)
            .expect("createOperations");
        assert_one_operation(&res, Kind::Concatenated);

        let mut context = CoordinateOperationContext::create(&db, "");
        context.set_intermediate_crs(vec![("NS_PIVOT".to_owned(), "PIVOT".to_owned())]);
        let res = operations
            .create_operations(&source, &target, &context)
            .expect("createOperations, pivot");
        assert_one_operation(&res, Kind::Concatenated);

        let mut context = CoordinateOperationContext::create(&db, "");
        context.set_intermediate_crs_use(IntermediateCrsUse::Never);
        let res = operations
            .create_operations(&source, &target, &context)
            .expect("createOperations, never");
        assert_one_operation(&res, Kind::Transformation);

        let mut context = CoordinateOperationContext::create(&db, "");
        context.set_intermediate_crs(vec![("NS_PIVOT".to_owned(), "NOT_EXISTING".to_owned())]);
        let res = operations
            .create_operations(&source, &target, &context)
            .expect("createOperations, unknown pivot");
        assert_one_operation(&res, Kind::Transformation);
    }
}

#[test]
fn factory_with_tmp_database_authority_factory_create_from_crs_codes_with_intermediates_source_equals_target(
) {
    let tmp = TmpDb::new("same-crs");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    let db = tmp.database();
    let factory = AuthorityFactory::create(&db, "");
    let res = factory
        .create_from_crs_codes_with_intermediates(("EPSG", "4326"), ("EPSG", "4326"), &[])
        .expect("same CRS");
    assert_eq!(res.len(), 0);
}

#[test]
fn factory_with_tmp_database_authority_factory_create_from_crs_codes_with_intermediates_case_source_pivot_target_pivot(
) {
    let tmp = TmpDb::new("source-pivot-target-pivot");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    create_source_target_pivot_crs(&tmp);
    create_transformation_for_pivot_testing(&tmp, "SOURCE", "PIVOT");
    create_transformation_for_pivot_testing(&tmp, "TARGET", "PIVOT");
    check_source_to_other(&tmp);
}

#[test]
fn factory_with_tmp_database_authority_factory_create_from_crs_codes_with_intermediates_case_source_pivot_pivot_target(
) {
    let tmp = TmpDb::new("source-pivot-pivot-target");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    create_source_target_pivot_crs(&tmp);
    create_transformation_for_pivot_testing(&tmp, "SOURCE", "PIVOT");
    create_transformation_for_pivot_testing(&tmp, "PIVOT", "TARGET");
    check_source_to_other(&tmp);
}

#[test]
fn factory_with_tmp_database_authority_factory_create_from_crs_codes_with_intermediates_case_pivot_source_pivot_target(
) {
    let tmp = TmpDb::new("pivot-source-pivot-target");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    create_source_target_pivot_crs(&tmp);
    create_transformation_for_pivot_testing(&tmp, "PIVOT", "SOURCE");
    create_transformation_for_pivot_testing(&tmp, "PIVOT", "TARGET");
    check_source_to_other(&tmp);
}

#[test]
fn factory_with_tmp_database_authority_factory_create_from_crs_codes_with_intermediates_case_pivot_source_target_pivot(
) {
    let tmp = TmpDb::new("pivot-source-target-pivot");
    tmp.create_structure();
    tmp.populate_with_fake_epsg();
    create_source_target_pivot_crs(&tmp);
    create_transformation_for_pivot_testing(&tmp, "PIVOT", "SOURCE");
    create_transformation_for_pivot_testing(&tmp, "TARGET", "PIVOT");
    check_source_to_other(&tmp);
}
