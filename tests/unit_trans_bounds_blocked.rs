//! The five `proj_trans_bounds` `TEST()`s of PROJ `test/unit/test_c_api.cpp`
//! that `docs/PORTING-c-api-unit.md` listed as blocked: `OGC:CRS84`, `ESRI:`
//! codes, inline WKT and a compound CRS. Tolerances and expected values are
//! PROJ 9.9.0's.
//!
//! Axis order: a pair built from CRS names is longitude, latitude. PROJ's
//! `EPSG:4326` and `EPSG:9707` tests pass latitude first, so the box is
//! written in longitude, latitude order here, as in the other ports.

use proj_rs::{Direction, Proj};

fn near(got: f64, expect: f64, tol: f64) {
    assert!((got - expect).abs() <= tol, "{got} vs {expect} (tol {tol})");
}

#[test]
fn c_api_proj_trans_bounds_ignore_inf() {
    // Before the ellipsoidal gnomonic projection the WGS84 ellipsoid was a sphere
    // of radius 6378137 m. The north edge is moved to lat 1.3 so the boundary
    // stays inside the horizon.
    let proj = Proj::new_known_crs("OGC:CRS84", "ESRI:102036", None).expect("pair");
    let [left, bottom, right, top] = proj
        .transform_bounds_in(Direction::Forward, -180.0, -90.0, 180.0, 1.3, 21)
        .expect("forward");
    near(left, -49_621_755.4, 1.0);
    near(bottom, -116_576_598.5, 1.0);
    near(right, 49_621_755.4, 1.0);
    near(top, 50_132_027.2, 1.0);
}

#[test]
fn c_api_proj_trans_bounds_ignore_inf_geographic() {
    let wkt = "PROJCS[\"Interrupted_Goode_Homolosine\",\
        GEOGCS[\"GCS_unnamed ellipse\",DATUM[\"D_unknown\",\
        SPHEROID[\"Unknown\",6378137,298.257223563]],\
        PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.0174532925199433]],\
        PROJECTION[\"Interrupted_Goode_Homolosine\"],\
        UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]],\
        AXIS[\"Easting\",EAST],AXIS[\"Northing\",NORTH]]";
    let proj = Proj::new_known_crs(wkt, "OGC:CRS84", None).expect("pair");
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

#[test]
fn c_api_proj_trans_bounds_to_compound_crs() {
    // EPSG:9707 is "WGS 84 + EGM96 height": a compound CRS whose horizontal
    // part is EPSG:4326, so the box comes back unchanged.
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:9707", None).expect("pair");
    let [left, bottom, right, top] = proj
        .transform_bounds_in(Direction::Forward, -120.0, 40.0, -80.0, 64.0, 0)
        .expect("forward");
    near(left, -120.0, 1e-8);
    near(bottom, 40.0, 1e-8);
    near(right, -80.0, 1e-8);
    near(top, 64.0, 1e-8);
}

#[test]
fn c_api_proj_trans_bounds_world_geodetic_to_spilhaus() {
    let proj = Proj::new_known_crs("OGC:CRS84", "ESRI:54099", None).expect("pair");
    let [left, bottom, right, top] = proj
        .transform_bounds_in(Direction::Forward, -180.0, -90.0, 180.0, 90.0, 21)
        .expect("forward");
    near(left, -16_336_432.4, 1.0);
    near(bottom, -16_605_405.9, 1.0);
    near(right, 16_574_104.3, 1.0);
    near(top, 16_640_152.9, 1.0);
}

#[test]
fn c_api_proj_trans_bounds_3d_ignore_inf() {
    // The 3D walk over the two height planes zmin = 0 and zmax = 0.
    let proj = Proj::new_known_crs("OGC:CRS84", "ESRI:102036", None).expect("pair");
    let (min, max) = proj
        .transform_bounds_3d(
            Direction::Forward,
            [-180.0, -90.0, 0.0],
            [180.0, 1.3, 0.0],
            21,
        )
        .expect("forward");
    near(min[0], -49_621_755.4, 1.0);
    near(min[1], -116_576_598.5, 1.0);
    near(max[0], 49_621_755.4, 1.0);
    near(max[1], 50_132_027.2, 1.0);
    near(min[2], 0.0, 1.0);
    near(max[2], 0.0, 1.0);
}
