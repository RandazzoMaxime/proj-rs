//! `proj_create_crs_to_crs_from_pj` between CRS written `+init=epsg:CODE`, under the PROJ 4 init
//! rules that pyproj turns on (`proj_context_use_proj4_init_rules`). The pair reads both sides
//! again under the rules of its context: under the ISO 19111 rules `init=epsg:` is refused, and
//! no candidate was built. Expected values: C PROJ 9.9.0, `proj_create_crs_to_crs_from_pj`, then
//! `proj_normalize_for_visualization` and `proj_trans`.

use proj_rs::unstable::InitRules;
use proj_rs::{Proj, ProjBuilder};

fn proj4_builder() -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder.set_init_rules(InitRules::Proj4);
    builder
}

fn pair(source: &str, target: &str) -> Proj {
    let source = proj4_builder().proj(source).expect("source");
    let target = proj4_builder().proj(target).expect("target");
    proj4_builder()
        .proj_create_crs_to_crs_from_pj(&source, &target, None, None)
        .expect("pair")
}

fn assert_close(got: (f64, f64), want: (f64, f64), tolerance: f64) {
    assert!(
        (got.0 - want.0).abs() < tolerance && (got.1 - want.1).abs() < tolerance,
        "got {got:?}, want {want:?}"
    );
}

#[test]
fn init_wgs84_to_init_british_national_grid() {
    let proj = pair("+init=epsg:4326 +type=crs", "+init=epsg:27700 +type=crs");
    let out = proj.convert((-2.0_f64, 50.0_f64)).expect("forward");
    assert_close(out, (400_096.027_881_023, 11_092.453_027_401), 1e-6);
}

#[test]
fn init_merchich_lambert_to_init_wgs84() {
    let proj = pair("+init=epsg:26191 +type=crs", "+init=epsg:4326 +type=crs");
    let out = proj
        .convert((336_900.0_f64, 383_000.0_f64))
        .expect("forward");
    assert_close(out, (-7.165_118_951, 34.033_518_563), 1e-9);
}
