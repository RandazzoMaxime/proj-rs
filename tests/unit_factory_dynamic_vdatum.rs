//! PROJ 9.9.0 `test/unit/test_factory.cpp`, `factory.AuthorityFactory_createDynamicVerticalDatum`.
//!
//! The C++ `TEST()` sits under `#ifdef no_more_dynamic_vertical_datum`, so PROJ
//! 9.9.0 does not compile it. The expectation it held (EPSG:1096 is a dynamic
//! vertical frame with reference epoch 2000) does not hold in the 9.9.0 database.
//! The values below are what the reference `projinfo` 9.9.0 prints for EPSG:1096:
//!
//! ```text
//! projinfo -k datum EPSG:1096 -o PROJJSON
//! {"type": "VerticalReferenceFrame", "name": "Norway Normal Null 2000:2018", ...}
//! ```
//!
//! So this port asserts the 9.9.0 behaviour: a plain `VerticalReferenceFrame`
//! with no frame reference epoch.

use proj_rs::iso19111::{AuthorityFactory, DatabaseContext};

/// `factory.AuthorityFactory_createDynamicVerticalDatum`, as 9.9.0 behaves.
#[test]
fn factory_authority_factory_create_dynamic_vertical_datum() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let datum = factory
        .create_vertical_datum("1096")
        .expect("EPSG:1096 Norway Normal Null 2000");
    assert_eq!(datum.name(), "Norway Normal Null 2000:2018");
    assert_eq!(datum.frame_reference_epoch(), None);
}
