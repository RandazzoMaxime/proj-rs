//! PROJ 9.9.0 `test/unit/test_c_api.cpp`, `proj_trans_bounds` and `proj_trans_bounds_3D` on
//! the ESRI Spilhaus projection: a geographic CRS in degrees to `ESRI:54099`, which needs the
//! `unitconvert` from degrees to radians before the projection, and to `ESRI:102036`.

use proj_rs::iso19111::best_pipeline;
use proj_rs::{Direction, Proj};

fn near(got: f64, expect: f64, tol: f64) {
    assert!((got - expect).abs() <= tol, "{got} vs {expect} (tol {tol})");
}

/// Point transform from `OGC:CRS84` to `ESRI:54099`, as `cs2cs` 9.9.0 writes it. No C++ `TEST()`
/// names this point. The pipeline is `unitconvert` from degrees, then `+proj=spilhaus`.
#[test]
fn capi_convert_world_geodetic_to_spilhaus_origin() {
    let proj = Proj::new_known_crs("OGC:CRS84", "ESRI:54099", None).expect("pair");
    let (x, y, z) = proj.convert_xyz(0.0, 0.0, 0.0).expect("origin");
    near(x, -5_584_639.07, 0.01);
    near(y, 2_860_255.12, 0.01);
    near(z, 0.0, 0.01);
    let (x, y, _) = proj.convert_xyz(10.0, 10.0, 0.0).expect("10,10");
    near(x, -4_951_069.22, 0.01);
    near(y, 3_971_197.53, 0.01);
}

/// `TEST_F(CApi, proj_trans_bounds_world_geodetic_to_spilhaus)`
#[test]
fn capi_trans_bounds_world_geodetic_to_spilhaus() {
    let proj = Proj::new_known_crs("OGC:CRS84", "ESRI:54099", None).expect("pair");
    let [left, bottom, right, top] = proj
        .transform_bounds_in(Direction::Forward, -180.0, -90.0, 180.0, 90.0, 21)
        .expect("forward");
    near(left, -16_336_432.4, 1.0);
    near(bottom, -16_605_405.9, 1.0);
    near(right, 16_574_104.3, 1.0);
    near(top, 16_640_152.9, 1.0);
}

/// `TEST_F(CApi, proj_trans_bounds_ignore_inf)`
#[test]
fn capi_trans_bounds_ignore_inf() {
    let proj = Proj::new_known_crs("OGC:CRS84", "ESRI:102036", None).expect("pair");
    let [left, bottom, right, top] = proj
        .transform_bounds_in(Direction::Forward, -180.0, -90.0, 180.0, 1.3, 21)
        .expect("forward");
    near(left, -49_621_755.4, 1.0);
    near(bottom, -116_576_598.5, 1.0);
    near(right, 49_621_755.4, 1.0);
    near(top, 50_132_027.2, 1.0);
}

/// `TEST_F(CApi, proj_trans_bounds_3D_ignore_inf)`
#[test]
fn capi_trans_bounds_3d_ignore_inf() {
    let proj = Proj::new_known_crs("OGC:CRS84", "ESRI:102036", None).expect("pair");
    let (min, max) = proj
        .transform_bounds_3d(
            Direction::Forward,
            [-180.0, -90.0, 0.0],
            [180.0, 1.3, 0.0],
            21,
        )
        .expect("3D");
    near(min[0], -49_621_755.4, 1.0);
    near(min[1], -116_576_598.5, 1.0);
    near(max[0], 49_621_755.4, 1.0);
    near(max[1], 50_132_027.2, 1.0);
    near(min[2], 0.0, 1.0);
    near(max[2], 0.0, 1.0);
}

/// `TEST_F(CApi, proj_trans_bounds_ignore_inf_geographic)`
///
/// The source is an ESRI WKT1 Interrupted Goode Homolosine, read by the ISO 19111 selection
/// that `proj_create_crs_to_crs` uses. The pipeline is then transformed as a PROJ string.
/// The left bound is 0.79 degree from the C++ value on the same densified boundary, within
/// the tolerance of 1. The other three match to 1e-4.
#[test]
fn capi_trans_bounds_ignore_inf_geographic() {
    let wkt = "PROJCS[\"Interrupted_Goode_Homolosine\",\
        GEOGCS[\"GCS_unnamed ellipse\",DATUM[\"D_unknown\",\
        SPHEROID[\"Unknown\",6378137,298.257223563]],\
        PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.0174532925199433]],\
        PROJECTION[\"Interrupted_Goode_Homolosine\"],\
        UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]],\
        AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH]]";
    let (definition, _) = best_pipeline(wkt, "OGC:CRS84").expect("pipeline");
    let proj = Proj::new(&definition).expect("pipeline");
    let [left, bottom, right, top] = proj
        .transform_bounds_in(
            Direction::Forward,
            -15_028_000.0,
            7_515_000.0,
            -14_975_000.0,
            7_556_000.0,
            21,
        )
        .expect("forward");
    near(left, -179.2133, 1.0);
    near(bottom, 70.9345, 1.0);
    near(right, -177.9054, 1.0);
    near(top, 71.4364, 1.0);
}
