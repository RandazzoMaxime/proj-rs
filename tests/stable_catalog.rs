//! The stable catalogue of the native façade: `Context::crs_catalog`, `CrsFilter`, `CrsEntry`,
//! `Context::database_metadata` and `proj_version`.
//!
//! Every expected value comes from PROJ 9.9.0, never from this crate: `projinfo --list-crs` for
//! the listings and the areas of use (`projinfo EPSG:x -o WKT2:2019`), the `metadata` table of
//! PROJ's own `proj.db` (`target/proj-reference/build/data/proj.db`) for the versions, and
//! `cs2cs --version` for the PROJ version. The counts count the rows `projinfo` prints: a CRS
//! with two areas of use is one row per area.

use proj_rs::{BBox, Cache, CatalogKind, Context, CrsEntry, CrsFilter, DeprecatedCrs, ProjVersion};

/// `projinfo --list-crs projected --authority EPSG -q`, codes only, in its order.
const PROJINFO_PROJECTED_CODES: &str = include_str!("stable_catalog/projinfo_projected_codes.txt");

fn epsg(filter: CrsFilter) -> Vec<CrsEntry> {
    Context::default()
        .crs_catalog()
        .list(&filter.authority("EPSG"))
        .expect("the embedded database lists the EPSG CRS")
}

fn find<'a>(entries: &'a [CrsEntry], code: &str) -> Vec<&'a CrsEntry> {
    entries.iter().filter(|entry| entry.code == code).collect()
}

#[test]
fn default_filter_counts_match_projinfo_rows() {
    // projinfo --list-crs "" --authority EPSG -q                       -> 7485 rows
    assert_eq!(epsg(CrsFilter::new()).len(), 7485);
    // projinfo --list-crs "allow_deprecated" --authority EPSG -q       -> 8021 rows
    assert_eq!(
        epsg(CrsFilter::new().deprecated(DeprecatedCrs::Include)).len(),
        8021
    );
}

#[test]
fn per_kind_counts_match_projinfo_rows() {
    let count = |kind| epsg(CrsFilter::new().kinds([kind])).len();
    // projinfo --list-crs geographic_2d | projected | vertical | compound (non deprecated)
    assert_eq!(count(CatalogKind::Geographic2D), 637);
    assert_eq!(count(CatalogKind::Projected), 5434);
    assert_eq!(count(CatalogKind::Vertical), 308);
    assert_eq!(count(CatalogKind::Compound), 573);
    // projinfo --list-crs allow_deprecated,geographic_2d | projected (with deprecated)
    let with_deprecated = |kind| {
        epsg(
            CrsFilter::new()
                .kinds([kind])
                .deprecated(DeprecatedCrs::Include),
        )
        .len()
    };
    assert_eq!(with_deprecated(CatalogKind::Geographic2D), 718);
    assert_eq!(with_deprecated(CatalogKind::Projected), 5798);
}

#[test]
fn kinds_filter_keeps_only_the_kinds_asked() {
    let entries = epsg(CrsFilter::new().kinds([CatalogKind::Geographic2D, CatalogKind::Projected]));
    assert_eq!(entries.len(), 637 + 5434);
    assert!(entries.iter().all(|entry| matches!(
        entry.kind,
        CatalogKind::Geographic2D | CatalogKind::Projected
    )));
}

#[test]
fn projected_codes_are_the_ones_projinfo_lists_in_its_order() {
    let codes: Vec<String> = epsg(CrsFilter::new().kinds([CatalogKind::Projected]))
        .iter()
        .map(|entry| format!("{}:{}", entry.authority, entry.code))
        .collect();
    let expected: Vec<&str> = PROJINFO_PROJECTED_CODES
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    assert_eq!(codes.len(), 5434);
    assert_eq!(codes, expected);
}

#[test]
fn known_codes_have_the_kind_and_name_projinfo_gives() {
    let all = epsg(CrsFilter::new());
    let check = |code: &str, kind: CatalogKind, name: &str| {
        let rows = find(&all, code);
        assert_eq!(
            rows.len(),
            1,
            "{code} is listed once without its deprecated copies"
        );
        assert_eq!(rows[0].kind, kind, "kind of {code}");
        assert_eq!(rows[0].name, name, "name of {code}");
        assert!(!rows[0].deprecated, "{code} is not deprecated");
    };
    // projinfo --list-crs geographic_2d, projected, vertical, compound, geocentric, geographic_3d
    check("4326", CatalogKind::Geographic2D, "WGS 84");
    check("4979", CatalogKind::Geographic3D, "WGS 84");
    check("3857", CatalogKind::Projected, "WGS 84 / Pseudo-Mercator");
    check(
        "2154",
        CatalogKind::Projected,
        "ETRS89-FRA [RGF93 v1] / Lambert-93",
    );
    check("5773", CatalogKind::Vertical, "EGM96 height");
    check(
        "3901",
        CatalogKind::Compound,
        "KKJ / Finland Uniform Coordinate System + N60 height",
    );
    check("3822", CatalogKind::Geocentric, "TWD97");
}

#[test]
fn authority_column_is_set_for_every_entry() {
    let all = epsg(CrsFilter::new());
    assert!(all.iter().all(|entry| entry.authority == "EPSG"));
}

#[test]
fn deprecated_crs_is_excluded_by_default_and_listed_on_request() {
    // EPSG:2008 is deprecated: projinfo --list-crs projected lacks it, allow_deprecated has it.
    let default = epsg(CrsFilter::new().kinds([CatalogKind::Projected]));
    assert!(find(&default, "2008").is_empty());
    assert!(default.iter().all(|entry| !entry.deprecated));

    let included = epsg(
        CrsFilter::new()
            .kinds([CatalogKind::Projected])
            .deprecated(DeprecatedCrs::Include),
    );
    let rows = find(&included, "2008");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "NAD27(CGQ77) / SCoPQ zone 2");
    assert!(rows[0].deprecated);
}

#[test]
fn area_of_use_is_the_one_projinfo_reports() {
    let all = epsg(CrsFilter::new().deprecated(DeprecatedCrs::Include));

    // projinfo EPSG:4326 -o WKT2:2019 and -o PROJJSON say AREA["World."], BBOX[-90,-180,90,180].
    // Their name is not the database's: the usage reader of PROJ (factory.cpp, ~4222) replaces
    // extent 2830 by extent 1262 for WKT and PROJJSON. The listing (getCRSInfoList,
    // factory.cpp ~8959) reads extent.description with no such replacement, and the description
    // of extent 2830 in PROJ's proj.db is the long "World: Afghanistan; ... Zimbabwe." list.
    let world = find(&all, "4326");
    assert_eq!(world.len(), 1);
    let name = world[0]
        .area_name
        .as_deref()
        .expect("EPSG:4326 has an area name");
    assert!(name.starts_with("World: Afghanistan; Albania; "), "{name}");
    assert!(name.ends_with("Zimbabwe."), "{name}");
    assert_eq!(
        world[0].area_of_use,
        Some(BBox::new(-180.0, -90.0, 180.0, 90.0))
    );

    // projinfo EPSG:2008 -o WKT2:2019: AREA["Canada - Quebec - east of 57°W."], BBOX[46.6,-57,53.76,-54]
    let quebec = find(&all, "2008");
    assert_eq!(
        quebec[0].area_name.as_deref(),
        Some("Canada - Quebec - east of 57°W.")
    );
    assert_eq!(
        quebec[0].area_of_use,
        Some(BBox::new(-57.0, 46.6, -54.0, 53.76))
    );
}

#[test]
fn crs_with_two_usages_is_one_entry_per_usage() {
    // projinfo EPSG:22213 -o WKT2:2019 has two USAGE blocks, and --list-crs prints two rows.
    let all = epsg(CrsFilter::new());
    let rows = find(&all, "22213");
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .all(|entry| entry.name == "NAD83(CSRS)v2 / UTM zone 13N"));

    let mut areas: Vec<(&str, BBox)> = rows
        .iter()
        .map(|entry| {
            (
                entry.area_name.as_deref().unwrap_or_default(),
                entry.area_of_use.expect("both usages have a box"),
            )
        })
        .collect();
    areas.sort_by(|a, b| a.0.cmp(b.0));
    assert_eq!(
        areas,
        vec![
            (
                "Canada - Saskatchewan.",
                BBox::new(-110.0, 49.0, -101.34, 60.01)
            ),
            (
                "Canada between 108°W and 102°W onshore and offshore - Northwest Territories, Nunavut, Saskatchewan.",
                BBox::new(-108.0, 48.99, -102.0, 84.0)
            ),
        ]
    );
}

#[test]
fn unknown_authority_gives_an_empty_list() {
    // projinfo --list-crs "" --authority NOPE -q prints nothing and exits with 0.
    let entries = Context::default()
        .crs_catalog()
        .list(&CrsFilter::new().authority("NOPE"))
        .expect("an unknown authority is an empty list, not an error");
    assert!(entries.is_empty());
}

#[test]
fn filters_are_plain_values() {
    assert_eq!(CrsFilter::default(), CrsFilter::new());
    assert_eq!(DeprecatedCrs::default(), DeprecatedCrs::Exclude);
}

#[test]
fn database_metadata_is_the_one_of_proj_db() {
    // `metadata` table of target/proj-reference/build/data/proj.db, PROJ 9.9.0.
    let meta = Context::default()
        .database_metadata()
        .expect("the embedded database has its metadata");
    assert_eq!(meta.layout_version, Some((1, 7)));
    assert_eq!(meta.epsg_version.as_deref(), Some("v13.102"));
    assert_eq!(meta.proj_version.as_deref(), Some("9.9.0"));
    assert_eq!(meta.proj_data_version.as_deref(), Some("1.25"));
}

#[test]
fn proj_version_is_the_one_ported() {
    // cs2cs --version: "Rel. 9.9.0 (1df2be6), September 15th, 2026"
    let version: ProjVersion = proj_rs::proj_version();
    assert_eq!((version.major, version.minor, version.patch), (9, 9, 0));
    assert_eq!(version.to_string(), "9.9.0");
}

#[test]
fn grid_cache_mode_is_the_cache_setting_of_the_builder() {
    // The stable `Cache` is the grid cache mode: off by default, on when asked.
    assert_eq!(Context::default().cache(), Cache::Disabled);
    assert_eq!(
        Context::builder().cache(Cache::Enabled).build().cache(),
        Cache::Enabled
    );
}

#[test]
fn catalog_and_entries_are_send_and_sync() {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<proj_rs::CrsCatalog>();
    is_send_sync::<CrsEntry>();
    is_send_sync::<CrsFilter>();
}
