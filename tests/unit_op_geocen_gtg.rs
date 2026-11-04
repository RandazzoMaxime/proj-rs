//! PROJ 9.9.0 `test/unit/test_operation.cpp`, a geocentric translation with NEU velocities
//! from the EPSG database, exported as a PROJ string.

use proj_rs::{AlternativeGridNames, AuthorityFactory, DatabaseContext, ProjLines};

/// `TEST(operation, operation_geocen_translations_NEU_velocities_gtg)`
#[test]
fn operation_geocen_translations_neu_velocities_gtg() {
    let db = DatabaseContext::create().expect("proj.db");
    let op = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("10809", AlternativeGridNames::Official)
        .expect("10809");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +inv +proj=deformation +t_epoch=2000 +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80"
    );
}

/// `TEST(operation, operation_Position_Vector_geocen_and_geocen_translations_NEU_velocities_gtg_with_source_epoch)`
#[test]
fn operation_position_vector_geocen_and_geocen_translations_neu_velocities_gtg_with_source_epoch() {
    let db = DatabaseContext::create().expect("proj.db");
    let op = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("10814", AlternativeGridNames::Official)
        .expect("10814");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +proj=helmert +x=-0.05027 +y=-0.11595 +z=0.03012 +rx=-0.00310814 +ry=0.00457237 +rz=0.00472406 +s=0.003191 +convention=position_vector +step +proj=deformation +dt=-2.44 +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80"
    );
    let inverse = op.inverse();
    assert_eq!(
        inverse
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +inv +proj=deformation +dt=-2.44 +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80 +step +inv +proj=helmert +x=-0.05027 +y=-0.11595 +z=0.03012 +rx=-0.00310814 +ry=0.00457237 +rz=0.00472406 +s=0.003191 +convention=position_vector"
    );
}

/// `TEST(operation, operation_geocen_translations_by_grid_and_geocen_translations_NEU_velocities_gtg)`
#[test]
fn operation_geocen_translations_by_grid_and_geocen_translations_neu_velocities_gtg() {
    let db = DatabaseContext::create().expect("proj.db");
    let op = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("10811", AlternativeGridNames::Official)
        .expect("10811");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +proj=xyzgridshift +grids=no_kv_NKGETRF14_EPSG7922_2000.tif +step +proj=deformation +dt=-5 +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80"
    );
    let inverse = op.inverse();
    assert_eq!(
        inverse
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +inv +proj=deformation +dt=-5 +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80 +step +inv +proj=xyzgridshift +grids=no_kv_NKGETRF14_EPSG7922_2000.tif"
    );
}
