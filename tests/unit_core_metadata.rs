//! PROJ 9.9.0 `test/unit/test_metadata.cpp`, the 12 `TEST(metadata, ...)` cases. Expected values are
//! the ones the C++ asserts. The two `isEquivalentName` cases with a lone UTF-8 lead byte are not
//! written, because a Rust `&str` cannot hold them: see `docs/AUDIT-unit-core.md`, note 5.

use proj_rs::iso19111::{create_object_from_json, JsonObject, WktObject, WktParser};
use proj_rs::{
    equivalent_name_bigger_differences, BoundingBox, Citation, Extent, FactoryError, Identifier,
    WktConvention, WktFormatter,
};

fn extent(west: f64, south: f64, east: f64, north: f64) -> Extent {
    Extent::from_bbox(west, south, east, north).expect("valid bounding box")
}

fn equals(left: &Extent, right: &Extent) -> bool {
    left.contains(right) && right.contains(left)
}

/// The single geographic element of an extent, as `getBBox` reads it.
fn bbox_of(extent: &Extent) -> BoundingBox {
    assert_eq!(extent.geographic_elements().len(), 1);
    extent.geographic_elements()[0]
}

/// `TEST(metadata, citation)`
#[test]
fn citation() {
    let c = Citation::new("my citation");
    let c2 = c.clone();
    assert_eq!(c2.title(), Some("my citation"));
    assert_eq!(c2, c);
}

/// `TEST(metadata, extent)`
#[test]
fn extent_cases() {
    // An extent with nothing in it is valid.
    let _empty = Extent::create(None, Vec::new(), Vec::new(), Vec::new());

    let world = extent(-180.0, -90.0, 180.0, 90.0);
    assert!(world.is_equivalent_to(&world));
    assert!(world.contains(&world));

    let west_hemisphere = extent(-180.0, -90.0, 0.0, 90.0);
    assert!(!world.is_equivalent_to(&west_hemisphere));
    assert!(world.contains(&west_hemisphere));
    assert!(!west_hemisphere.contains(&world));

    let world_inter_world = world.intersection(&world).expect("world meets world");
    assert!(equals(&world_inter_world, &world));

    let france = extent(-5.0, 40.0, 12.0, 51.0);
    assert!(france.contains(&france));
    assert!(world.contains(&france));
    // We are only speaking about geography here.
    assert!(!france.contains(&world));
    assert!(world.intersects(&france));
    assert!(france.intersects(&world));

    let france_inter_france = france.intersection(&france).expect("france meets france");
    assert!(equals(&france_inter_france, &france));

    let france_inter_world = france.intersection(&world).expect("france meets world");
    assert!(equals(&france_inter_world, &france));

    let world_inter_france = world.intersection(&france).expect("world meets france");
    assert!(equals(&world_inter_france, &france));

    let france_shifted = extent(-5.0 + 5.0, 40.0 + 5.0, 12.0 + 5.0, 51.0 + 5.0);
    assert!(france.intersects(&france_shifted));
    assert!(france_shifted.intersects(&france));
    assert!(!france.contains(&france_shifted));
    assert!(!france_shifted.contains(&france));

    let europe = extent(-30.0, 25.0, 30.0, 70.0);
    assert!(europe.contains(&france));
    assert!(!france.contains(&europe));

    let france_inter_europe = france.intersection(&europe).expect("france meets europe");
    assert!(equals(&france_inter_europe, &france));

    let europe_intersects_france = europe.intersection(&france).expect("europe meets france");
    assert!(equals(&europe_intersects_france, &france));

    let nz = extent(155.0, -60.0, -170.0, -25.0);
    assert!(nz.contains(&nz));
    assert!(world.contains(&nz));
    assert!(nz.intersects(&world));
    assert!(world.intersects(&nz));
    assert!(!nz.contains(&world));
    assert!(!nz.contains(&france));
    assert!(!france.contains(&nz));
    assert!(!nz.intersects(&france));
    assert!(!france.intersects(&nz));

    {
        let nz_inter_world = nz.intersection(&world).expect("nz meets world");
        assert!(equals(&nz_inter_world, &nz));
    }
    {
        let nz_inter_world = bbox_of(&nz)
            .intersection(&bbox_of(&world))
            .expect("nz box meets world box");
        assert!(equals(
            &Extent::create(None, vec![nz_inter_world], Vec::new(), Vec::new()),
            &nz
        ));
    }
    {
        let world_inter_nz = world.intersection(&nz).expect("world meets nz");
        assert!(equals(&world_inter_nz, &nz));
    }
    {
        let world_inter_nz = bbox_of(&world)
            .intersection(&bbox_of(&nz))
            .expect("world box meets nz box");
        assert!(equals(
            &Extent::create(None, vec![world_inter_nz], Vec::new(), Vec::new()),
            &nz
        ));
    }

    assert!(nz.intersection(&france).is_none());
    assert!(france.intersection(&nz).is_none());

    let bbox_antimeridian_north = extent(155.0, 10.0, -170.0, 30.0);
    assert!(!nz.contains(&bbox_antimeridian_north));
    assert!(!bbox_antimeridian_north.contains(&nz));
    assert!(!nz.intersects(&bbox_antimeridian_north));
    assert!(!bbox_antimeridian_north.intersects(&nz));
    assert!(nz.intersection(&bbox_antimeridian_north).is_none());
    assert!(bbox_antimeridian_north.intersection(&nz).is_none());

    let nz_pos_long = extent(155.0, -60.0, 180.0, -25.0);
    assert!(nz.contains(&nz_pos_long));
    assert!(!nz_pos_long.contains(&nz));
    assert!(nz.intersects(&nz_pos_long));
    assert!(nz_pos_long.intersects(&nz));
    let nz_inter_nz_pos_long = nz.intersection(&nz_pos_long).expect("nz meets nz_pos_long");
    assert!(equals(&nz_inter_nz_pos_long, &nz_pos_long));
    let nz_pos_long_inter_nz = nz_pos_long.intersection(&nz).expect("nz_pos_long meets nz");
    assert!(equals(&nz_pos_long_inter_nz, &nz_pos_long));

    let nz_neg_long = extent(-180.0, -60.0, -170.0, -25.0);
    assert!(nz.contains(&nz_neg_long));
    assert!(!nz_neg_long.contains(&nz));
    assert!(nz.intersects(&nz_neg_long));
    assert!(nz_neg_long.intersects(&nz));
    let nz_inter_nz_neg_long = nz.intersection(&nz_neg_long).expect("nz meets nz_neg_long");
    assert!(equals(&nz_inter_nz_neg_long, &nz_neg_long));
    let nz_neg_long_inter_nz = nz_neg_long.intersection(&nz).expect("nz_neg_long meets nz");
    assert!(equals(&nz_neg_long_inter_nz, &nz_neg_long));

    let nz_smaller = extent(160.0, -55.0, -175.0, -30.0);
    assert!(nz.contains(&nz_smaller));
    assert!(!nz_smaller.contains(&nz));

    let nz_pos_long_shifted_west = extent(150.0, -60.0, 175.0, -25.0);
    assert!(!nz.contains(&nz_pos_long_shifted_west));
    assert!(!nz_pos_long_shifted_west.contains(&nz));
    assert!(nz.intersects(&nz_pos_long_shifted_west));
    assert!(nz_pos_long_shifted_west.intersects(&nz));

    let nz_smaller_shifted = extent(165.0, -60.0, -170.0, -25.0);
    assert!(!nz_smaller.contains(&nz_smaller_shifted));
    assert!(!nz_smaller_shifted.contains(&nz_smaller));
    assert!(nz_smaller.intersects(&nz_smaller_shifted));
    assert!(nz_smaller_shifted.intersects(&nz_smaller));

    let nz_shifted = extent(165.0, -60.0, -160.0, -25.0);
    let nz_intersect_nz_shifted = nz.intersection(&nz_shifted).expect("nz meets nz_shifted");
    assert!(equals(
        &nz_intersect_nz_shifted,
        &extent(165.0, -60.0, -170.0, -25.0)
    ));

    let nz_inter_nz_smaller = nz.intersection(&nz_smaller).expect("nz meets nz_smaller");
    assert!(equals(&nz_inter_nz_smaller, &nz_smaller));

    let nz_smaller_inter_nz = nz_smaller.intersection(&nz).expect("nz_smaller meets nz");
    assert!(equals(&nz_smaller_inter_nz, &nz_smaller));

    let world_smaller = extent(-179.0, -90.0, 179.0, 90.0);
    assert!(!world_smaller.contains(&nz));
    assert!(!nz.contains(&world_smaller));

    let nz_inter_world_smaller = nz
        .intersection(&world_smaller)
        .expect("nz meets world_smaller");
    assert!(equals(
        &nz_inter_world_smaller,
        &extent(155.0, -60.0, 179.0, -25.0)
    ));

    let world_smaller_inter_nz = world_smaller
        .intersection(&nz)
        .expect("world_smaller meets nz");
    assert!(equals(
        &world_smaller_inter_nz,
        &extent(155.0, -60.0, 179.0, -25.0)
    ));

    let world_smaller_east = extent(-179.0, -90.0, 150.0, 90.0);
    assert!(!world_smaller_east.contains(&nz));
    assert!(!nz.contains(&world_smaller_east));

    let nz_inter_world_smaller_east = nz
        .intersection(&world_smaller_east)
        .expect("nz meets world_smaller_east");
    assert_eq!(
        bbox_of(&nz_inter_world_smaller_east).west_bound_longitude(),
        -179.0
    );
    assert_eq!(
        bbox_of(&nz_inter_world_smaller_east).east_bound_longitude(),
        -170.0
    );
    assert!(equals(
        &nz_inter_world_smaller_east,
        &extent(-179.0, -60.0, -170.0, -25.0)
    ));

    let world_smaller_east_inter_nz = world_smaller_east
        .intersection(&nz)
        .expect("world_smaller_east meets nz");
    assert_eq!(
        bbox_of(&world_smaller_east_inter_nz).west_bound_longitude(),
        -179.0
    );
    assert_eq!(
        bbox_of(&world_smaller_east_inter_nz).east_bound_longitude(),
        -170.0
    );
    assert!(equals(
        &world_smaller_east_inter_nz,
        &extent(-179.0, -60.0, -170.0, -25.0)
    ));

    let east_hemisphere = extent(0.0, -90.0, 180.0, 90.0);
    let east_hemisphere_inter_nz = east_hemisphere
        .intersection(&nz)
        .expect("east_hemisphere meets nz");
    assert!(equals(
        &east_hemisphere_inter_nz,
        &extent(155.0, -60.0, 180.0, -25.0)
    ));

    let minus_180_to_156 = extent(-180.0, -90.0, 156.0, 90.0);
    let minus_180_to_156_inter_nz = minus_180_to_156
        .intersection(&nz)
        .expect("minus_180_to_156 meets nz");
    assert!(equals(
        &minus_180_to_156_inter_nz,
        &extent(-180.0, -60.0, -170.0, -25.0)
    ));
}

/// `TEST(metadata, extent_edge_cases)`: a NaN bound and a south bound north of the north bound are refused.
#[test]
fn extent_edge_cases() {
    let _empty = Extent::create(None, Vec::new(), Vec::new(), Vec::new());

    let refuses = |west: f64, south: f64, east: f64, north: f64| {
        matches!(
            Extent::from_bbox(west, south, east, north),
            Err(FactoryError::Invalid(_))
        )
    };
    assert!(refuses(f64::NAN, -90.0, 180.0, 90.0));
    assert!(refuses(-180.0, f64::NAN, 180.0, 90.0));
    assert!(refuses(-180.0, -90.0, f64::NAN, 90.0));
    assert!(refuses(-180.0, -90.0, 180.0, f64::NAN));

    // South > north.
    assert!(refuses(-100.0, 10.0, 100.0, 0.0));

    // Scenario of https://bugs.chromium.org/p/oss-fuzz/issues/detail?id=57328 and
    // https://bugs.chromium.org/p/oss-fuzz/issues/detail?id=60084
    {
        let a = extent(0.0, 1.0, 2.0, 3.0);
        let b = extent(200.0, -80.0, -100.0, 80.0);
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
        assert!(a.intersection(&b).is_none());
        assert!(b.intersection(&a).is_none());
    }
    {
        let a = extent(0.0, 1.0, 2.0, 3.0);
        let b = extent(100.0, -80.0, -200.0, 80.0);
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
        assert!(a.intersection(&b).is_none());
        assert!(b.intersection(&a).is_none());
    }

    // Degenerate bounding box on a point.
    {
        let a = extent(1.0, 2.0, 1.0, 2.0);
        let b = extent(-10.0, -10.0, 10.0, 10.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.contains(&b));
        assert!(b.contains(&a));
        assert!(a.intersection(&b).is_some());
        assert!(b.intersection(&a).is_some());
    }

    // Degenerate bounding box on a line at long=-180.
    {
        let a = extent(-180.0, 2.0, -180.0, 3.0);
        let b = extent(-180.0, -90.0, 180.0, 90.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.contains(&b));
        assert!(b.contains(&a));
        assert!(a.intersection(&b).is_some());
        assert!(b.intersection(&a).is_some());
    }

    // Degenerate bounding box on a line at long=180.
    {
        let a = extent(180.0, 2.0, 180.0, 3.0);
        let b = extent(-180.0, -90.0, 180.0, 90.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.contains(&b));
        assert!(b.contains(&a));
        assert!(a.intersection(&b).is_some());
        assert!(b.intersection(&a).is_some());
    }

    // Degenerate bounding box on a line at lat=90.
    {
        let a = extent(2.0, 90.0, 3.0, 90.0);
        let b = extent(-180.0, -90.0, 180.0, 90.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.contains(&b));
        assert!(b.contains(&a));
        assert!(a.intersection(&b).is_some());
        assert!(b.intersection(&a).is_some());
    }

    // Degenerate bounding box on a line at lat=-90.
    {
        let a = extent(2.0, -90.0, 3.0, -90.0);
        let b = extent(-180.0, -90.0, 180.0, 90.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.contains(&b));
        assert!(b.contains(&a));
        assert!(a.intersection(&b).is_some());
        assert!(b.intersection(&a).is_some());
    }
}

/// `TEST(metadata, identifier_empty)`
#[test]
fn identifier_empty() {
    let id = Identifier::create("");
    let id2 = id.clone();
    assert_eq!(id2, id);
    assert!(id2.authority().is_none());
    assert!(id2.code().is_empty());
    assert!(id2.code_space().is_empty());
    assert!(id2.version().is_none());
    assert!(id2.description().is_none());
}

/// `TEST(metadata, identifier_properties)`
#[test]
fn identifier_properties() {
    let id = Identifier::create("my code")
        .with_code_space("codespace")
        .with_authority(Citation::new("authority"))
        .with_version("version")
        .with_description("description");
    let id2 = id.clone();
    assert_eq!(id2, id);
    assert_eq!(
        id2.authority().expect("authority").title(),
        Some("authority")
    );
    assert_eq!(id2.code(), "my code");
    assert_eq!(id2.code_space(), "codespace");
    assert_eq!(id2.version(), Some("version"));
    assert_eq!(id2.description(), Some("description"));
}

/// The `id` of a PROJJSON `PrimeMeridian`, the runtime entry point for an identifier with untyped members.
fn prime_meridian_id(id_json: &str) -> Result<Identifier, FactoryError> {
    let text = format!(r#"{{"type":"PrimeMeridian","name":"x","longitude":0,"id":{id_json}}}"#);
    match create_object_from_json(&text, None)? {
        JsonObject::PrimeMeridian(meridian) => meridian
            .identifiers()
            .first()
            .cloned()
            .ok_or_else(|| FactoryError::Invalid("no identifier".to_owned())),
        _ => Err(FactoryError::Invalid("not a prime meridian".to_owned())),
    }
}

/// `TEST(metadata, identifier_code_integer)`: an integer code is kept as its text.
#[test]
fn identifier_code_integer() {
    let id = prime_meridian_id(r#"{"authority":"EPSG","code":1234}"#).expect("identifier");
    assert_eq!(id.code(), "1234");
}

/// `TEST(metadata, identifier_code_string)`
#[test]
fn identifier_code_string() {
    let id = prime_meridian_id(r#"{"authority":"EPSG","code":"1234"}"#).expect("identifier");
    assert_eq!(id.code(), "1234");
}

/// `TEST(metadata, identifier_code_invalid_type)`: a boolean code is refused.
#[test]
fn identifier_code_invalid_type() {
    assert!(prime_meridian_id(r#"{"authority":"EPSG","code":true}"#).is_err());
}

/// `TEST(metadata, identifier_authority_citation)`
#[test]
fn identifier_authority_citation() {
    let id = Identifier::create("").with_authority(Citation::new("authority"));
    assert_eq!(
        id.authority().expect("authority").title(),
        Some("authority")
    );
}

/// `TEST(metadata, identifier_authority_invalid_type)`: a boolean authority is refused.
#[test]
fn identifier_authority_invalid_type() {
    assert!(prime_meridian_id(r#"{"authority":true,"code":"1234"}"#).is_err());
}

/// `TEST(metadata, id)`: a standalone `ID[...]` reads its parts, and writes back the same text.
#[test]
fn id() {
    let in_wkt = "ID[\"EPSG\",4946,1.5,\n    CITATION[\"my citation\"],\n    URI[\"urn:ogc:def:crs:EPSG::4946\"]]";
    let id = match WktParser::new().create_from_wkt(in_wkt).expect("ID") {
        WktObject::Identifier(id) => id,
        _ => panic!("expected an identifier"),
    };

    assert_eq!(
        id.authority().expect("authority").title(),
        Some("my citation")
    );
    assert_eq!(id.code_space(), "EPSG");
    assert_eq!(id.code(), "4946");
    assert_eq!(id.version(), Some("1.5"));
    assert_eq!(id.uri(), Some("urn:ogc:def:crs:EPSG::4946"));

    let got_wkt = id
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
        .expect("WKT");
    assert_eq!(got_wkt, in_wkt);
}

/// `TEST(metadata, Identifier_isEquivalentName)`: the name rules of `Identifier::isEquivalentName`.
#[test]
fn identifier_is_equivalent_name() {
    assert!(equivalent_name_bigger_differences("", ""));
    assert!(equivalent_name_bigger_differences("x", "x"));
    assert!(equivalent_name_bigger_differences("x", "X"));
    assert!(equivalent_name_bigger_differences("X", "x"));
    assert!(!equivalent_name_bigger_differences("x", ""));
    assert!(!equivalent_name_bigger_differences("", "x"));
    assert!(!equivalent_name_bigger_differences("x", "y"));
    assert!(equivalent_name_bigger_differences(
        "Central_Meridian",
        "Central_- ()/Meridian"
    ));

    // Accented letters are written as ASCII.
    assert!(equivalent_name_bigger_differences("\u{e1}", "a"));
    assert!(equivalent_name_bigger_differences("a", "\u{e1}"));
    assert!(equivalent_name_bigger_differences("\u{e4}", "\u{e1}"));

    assert!(equivalent_name_bigger_differences(
        "Unknown based on International 1924 (Hayford 1909, 1910) ellipsoid",
        "Unknown_based_on_International_1924_Hayford_1909_1910_ellipsoid"
    ));

    assert!(equivalent_name_bigger_differences("foo + ", "foo + "));
    assert!(equivalent_name_bigger_differences("foo + bar", "foo + bar"));
    assert!(equivalent_name_bigger_differences("foo + bar", "foobar"));

    assert!(equivalent_name_bigger_differences(
        "foo_IntlFeet",
        "foo_Feet"
    ));
    assert!(equivalent_name_bigger_differences(
        "foo_Feet",
        "foo_IntlFeet"
    ));
    assert!(!equivalent_name_bigger_differences(
        "foo_IntlFeet_bar",
        "foo_Feet_bar"
    ));
    assert!(!equivalent_name_bigger_differences(
        "foo_Feet_bar",
        "foo_IntlFeet_bar"
    ));
}
