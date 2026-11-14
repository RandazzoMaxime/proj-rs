//! PROJ `test/unit/test_util.cpp` (`util.NameFactory`, `util.NameFactory2`)
//! through `proj_rs::names`. Expected strings and flags are PROJ 9.9.0's.

use proj_rs::names::{LocalName, NameSpace};

#[test]
fn util_name_factory() {
    let localname = LocalName::new("foo", None);
    let scope = localname.scope();
    assert!(scope.is_global());
    assert_eq!(scope.name().to_fully_qualified_name().to_string(), "global");
    assert_eq!(localname.to_fully_qualified_name().to_string(), "foo");
}

#[test]
fn util_name_factory2() {
    let namespace = NameSpace::new(LocalName::new("bar", None), "/");
    let localname = LocalName::new("foo", Some(&namespace));
    let scope = localname.scope();
    assert!(!scope.is_global());
    let fully_qualified = scope.name().to_fully_qualified_name();
    assert_eq!(fully_qualified.to_string(), "bar");
    assert!(fully_qualified.scope().is_global());
    assert!(fully_qualified.scope().name().scope().is_global());
    assert_eq!(localname.to_fully_qualified_name().to_string(), "bar/foo");
}
