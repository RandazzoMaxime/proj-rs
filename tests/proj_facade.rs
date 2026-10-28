//! georust `proj` 0.31 facade against one `builtins.gie` point and named EPSG codes.

use proj_rs::{Coord, Ellipsoid, Hemisphere, Lcc, Mercator, Proj, ProjBuilder, Tmerc, UtmZone};

const AEA: &str = "+proj=aea +ellps=GRS80 +lat_1=0 +lat_2=2";
const AEA_EAST: f64 = 222_571.608_757_106;
const AEA_NORTH: f64 = 110_653.326_743_030;

fn metres(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-4
}

#[test]
fn builtins_aea_project_and_convert_match_gie() {
    let proj = Proj::new(AEA).expect("aea builds");
    let radians = (2.0_f64.to_radians(), 1.0_f64.to_radians());
    let projected = proj.project(radians, false).expect("project");
    let converted = proj.convert(radians).expect("convert");
    assert!(metres(projected.x(), AEA_EAST), "{}", projected.x());
    assert!(metres(projected.y(), AEA_NORTH), "{}", projected.y());
    assert!(metres(converted.x(), AEA_EAST), "{}", converted.x());
    assert!(metres(converted.y(), AEA_NORTH), "{}", converted.y());
    let mut slice = [radians];
    proj.convert_array(&mut slice).expect("array");
    assert!(metres(slice[0].x(), AEA_EAST));
    let back = proj.project(projected, true).expect("inverse");
    assert!((back.x() - radians.x()).abs() < 1e-10);
    assert!((back.y() - radians.y()).abs() < 1e-10);
}

#[test]
fn unknown_proj_is_an_error() {
    let error = Proj::new("+proj=not_a_real_projection").expect_err("unknown");
    let text = error.to_string();
    assert!(text.contains("not_a_real_projection"), "{text}");
}

#[test]
fn epsg_4326_to_2154_matches_lambert93() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("pair");
    let xy = proj.convert((2.0_f64, 49.0_f64)).expect("forward");
    let typed = Lcc::lambert93().forward(2.0, 49.0).expect("lcc");
    assert!(metres(xy.0, typed.east), "{} vs {}", xy.0, typed.east);
    assert!(metres(xy.1, typed.north), "{} vs {}", xy.1, typed.north);
    let projected = proj.project((2.0_f64, 49.0_f64), false).expect("project");
    assert!(metres(projected.0, typed.east));
    let back = proj.project(xy, true).expect("inverse");
    assert!((back.0 - 2.0).abs() < 1e-8, "{}", back.0);
    assert!((back.1 - 49.0).abs() < 1e-8, "{}", back.1);
}

#[test]
fn utm_and_web_mercator_codes() {
    let zone = UtmZone::try_new(31).expect("zone");
    let utm = Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm");
    let typed = utm.forward(3.0, 46.5).expect("forward");
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32631", None).expect("32631");
    let xy = proj.convert((3.0_f64, 46.5_f64)).expect("convert");
    assert!(metres(xy.0, typed.east), "{} vs {}", xy.0, typed.east);
    assert!(metres(xy.1, typed.north), "{} vs {}", xy.1, typed.north);

    let south = Proj::new_known_crs("EPSG:4326", "EPSG:32731", None).expect("32731");
    let origin = south.convert((3.0_f64, 0.0_f64)).expect("equator");
    assert!(metres(origin.0, 500_000.0), "{}", origin.0);
    assert!(metres(origin.1, 10_000_000.0), "{}", origin.1);

    let web = Mercator::web().forward(2.0, 49.0).expect("web");
    let merc = Proj::new_known_crs("EPSG:4326", "EPSG:3857", None).expect("3857");
    let xy = merc.convert((2.0_f64, 49.0_f64)).expect("convert");
    assert!(metres(xy.0, web.east), "{} vs {}", xy.0, web.east);
    assert!(metres(xy.1, web.north), "{} vs {}", xy.1, web.north);
}

#[test]
fn network_and_grid_methods_fail() {
    let mut builder = ProjBuilder::new();
    assert!(builder.enable_network(true).is_err());
    assert!(builder.set_url_endpoint("https://example.invalid").is_err());
    assert!(builder.set_search_paths("C:/no/grids").is_err());
    assert!(builder.grid_cache_enable(true).is_err());
    assert!(!builder.network_enabled());
    let proj = builder
        .proj_known_crs("EPSG:4326", "EPSG:2154", None)
        .expect("pair");
    assert!(proj.area_of_use().is_err());
    assert!(proj.to_projjson(None, None, None).is_err());
}

#[test]
fn single_crs_is_not_a_transform() {
    let source = Proj::new("EPSG:4326").expect("4326");
    let target = Proj::new("EPSG:2154").expect("2154");
    let proj = source
        .create_crs_to_crs_from_pj(&target, None, None)
        .expect("pair");
    let xy = proj.convert((2.0_f64, 49.0_f64)).expect("convert");
    let typed = Lcc::lambert93().forward(2.0, 49.0).expect("lcc");
    assert!(metres(xy.0, typed.east));
    assert!(proj_rs::Proj::new("EPSG:4326")
        .expect("crs")
        .convert((2.0_f64, 49.0_f64))
        .is_err());
}

#[cfg(feature = "geo-types")]
#[test]
fn geo_types_point_uses_convert() {
    use geo_types::Point;
    use proj_rs::Transform;

    let proj = Proj::new("+proj=axisswap +order=2,1").expect("axisswap");
    let point = Point::new(-36.508_f64, -54.2815_f64);
    let swapped = point.transformed(&proj).expect("transform");
    assert!((swapped.x() - -54.2815).abs() < 1e-9);
    assert!((swapped.y() - -36.508).abs() < 1e-9);
    assert!((point.x() - -36.508).abs() < 1e-9);
}
