//! PROJ 9.9.0 `test/unit/test_crs.cpp`, identification of vertical CRS against the database.

use proj_rs::iso19111::{AuthorityFactory, Crs, DatabaseContext, VerticalCRS};

fn with_code(source: &VerticalCRS, name: &str, code_space: &str, code: &str) -> Crs {
    let datum = source.datum().cloned().expect("datum");
    let mut crs = VerticalCRS::create(name, datum, source.coordinate_system().clone());
    if !code.is_empty() {
        crs = crs.with_identifier(code_space, code);
    }
    Crs::Vertical(crs)
}

/// `TEST(crs, verticalCRS_identify_db)`
#[test]
fn crs_vertical_crs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let source = factory.create_vertical_crs("7651").expect("7651");
    {
        let found = Crs::Vertical(source.clone()).identify_with(Some(&factory));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, 100);
        assert!(Crs::Vertical(source.clone()).identify_with(None).is_empty());
    }
    {
        let crs = with_code(&source, source.name(), "EPSG", "1");
        assert!(crs.identify_with(Some(&factory)).is_empty());
    }
    {
        let crs = with_code(&source, source.name(), "EPSG", "7652");
        let found = crs.identify_with(Some(&factory));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, 25);
    }
    {
        let crs = with_code(&source, source.name(), "", "");
        let found = crs.identify_with(Some(&factory));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, 100);
    }
    {
        let crs = with_code(&source, "Kumul_34_height", "", "");
        let found = crs.identify_with(Some(&factory));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, 90);
    }
    {
        let other = factory.create_vertical_crs("7652").expect("7652");
        let crs = with_code(&other, "Kumul_34_height", "", "");
        let found = crs.identify_with(Some(&factory));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, 25);
    }
    {
        let crs = with_code(&source, "no match", "", "");
        assert!(crs.identify_with(Some(&factory)).is_empty());
    }
}
