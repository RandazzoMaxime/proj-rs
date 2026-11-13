//! `test/unit/test_defmodel.cpp`: `defmodel.evaluator_horizontal_unit_degree`,
//! `evaluator_horizontal_unit_metre` and `evaluator_projected_crs`.
//!
//! The C++ tests inject synthetic grids through `GridPrototype`,
//! `GridSetPrototype` and `EvaluatorIfacePrototype`. The Rust tests do the
//! same through `proj_rs::defmodel::{Grid, GridSet, GridProvider}`: the grids
//! are the fixtures, the evaluator is the unit under test. Expected values are
//! PROJ 9.9.0's.

#[path = "common/defmodel_json.rs"]
#[expect(
    dead_code,
    reason = "this file uses neither the removal, the clear, nor the full model helpers"
)]
mod defmodel_json;

use std::f64::consts::PI;

use defmodel_json::{arr, min_valid, n, obj, s, set, Json};
use proj_rs::defmodel::{DefmodelError, Evaluator, Grid, GridProvider, GridSet, MasterFile};

/// `DEG_TO_RAD_CONSTANT` of `defmodel_impl.hpp`.
const DEG_TO_RAD_CONSTANT: f64 = PI / 180.0;

fn deg_to_rad(d: f64) -> f64 {
    d * DEG_TO_RAD_CONSTANT
}

fn rad_to_deg(d: f64) -> f64 {
    d / DEG_TO_RAD_CONSTANT
}

fn near(got: f64, expected: f64, tolerance: f64) {
    assert!(
        (got - expected).abs() <= tolerance,
        "{got} vs {expected} (tolerance {tolerance})"
    );
}

/// A grid set that serves one node grid for every point, as `GridSet` of the
/// C++ tests does.
#[derive(Clone)]
struct OneGrid<N>(N);

impl<N: Grid> GridSet for OneGrid<N> {
    type Node = N;

    fn grid_at(&self, _x: f64, _y: f64) -> Option<&N> {
        Some(&self.0)
    }
}

/// `EvaluatorIface` of the C++ tests: it opens `bla.tif` only.
struct Provider<N> {
    geographic: bool,
    node: N,
}

impl<N: Grid + Clone> GridProvider for Provider<N> {
    type Set = OneGrid<N>;

    fn open(&self, filename: &str) -> Option<OneGrid<N>> {
        (filename == "bla.tif").then(|| OneGrid(self.node.clone()))
    }

    fn is_geographic_crs(&self, _definition: &str) -> bool {
        self.geographic
    }
}

/// A component object with a `GeoTIFF` spatial model on `bla.tif`.
fn component(displacement: &str, bbox: [f64; 4], interpolation: &str, time_function: Json) -> Json {
    obj(vec![
        ("displacement_type", s(displacement)),
        ("uncertainty_type", s("none")),
        (
            "extent",
            obj(vec![
                ("type", s("bbox")),
                (
                    "parameters",
                    obj(vec![("bbox", arr(bbox.iter().map(|v| n(*v)).collect()))]),
                ),
            ]),
        ),
        (
            "spatial_model",
            obj(vec![
                ("type", s("GeoTIFF")),
                ("interpolation_method", s(interpolation)),
                ("filename", s("bla.tif")),
            ]),
        ),
        ("time_function", time_function),
    ])
}

/// A piecewise function, zero outside `[2010, 2020]`, with scale factor `0.5`.
fn half_piecewise() -> Json {
    obj(vec![
        ("type", s("piecewise")),
        (
            "parameters",
            obj(vec![
                ("before_first", s("zero")),
                ("after_last", s("zero")),
                (
                    "model",
                    arr(vec![
                        obj(vec![
                            ("epoch", s("2010-01-01T00:00:00Z")),
                            ("scale_factor", n(0.5)),
                        ]),
                        obj(vec![
                            ("epoch", s("2020-01-01T00:00:00Z")),
                            ("scale_factor", n(0.5)),
                        ]),
                    ]),
                ),
            ]),
        ),
    ])
}

fn constant() -> Json {
    obj(vec![("type", s("constant"))])
}

fn build<P: GridProvider>(j: &Json, provider: &P, a: f64, b: f64) -> Evaluator<P> {
    let mf = MasterFile::parse(&j.dump()).expect("valid model");
    Evaluator::new(mf, provider, a, b).expect("evaluator")
}

// ---------------------------------------------------------------------------
// horizontal_unit_degree

/// The degree grid of `evaluator_horizontal_unit_degree`: longitude and
/// latitude offsets in degrees, a vertical offset in metres, on a grid from
/// (160, -50) with steps 2 and 0.5 degrees, 16 by 41 nodes.
#[derive(Clone)]
struct DegreeGrid;

const GRID_RES_X_DEG: f64 = 2.0;
const GRID_RES_Y_DEG: f64 = 0.5;

impl DegreeGrid {
    fn long_lat(ix: usize, iy: usize) -> Option<(f64, f64)> {
        let lon = match ix {
            1 => deg_to_rad(0.01),
            2 => deg_to_rad(0.02),
            _ => return None,
        };
        let lat = match iy {
            3 => deg_to_rad(0.03),
            4 => deg_to_rad(0.04),
            _ => return None,
        };
        Some((lon, lat))
    }

    fn z(ix: usize, iy: usize) -> Option<f64> {
        match (ix, iy) {
            (1, 3) => Some(10.0),
            (2, 3) => Some(11.0),
            (1, 4) => Some(11.0),
            (2, 4) => Some(12.0),
            _ => None,
        }
    }
}

impl Grid for DegreeGrid {
    fn min_x(&self) -> f64 {
        deg_to_rad(160.0)
    }
    fn min_y(&self) -> f64 {
        deg_to_rad(-50.0)
    }
    fn res_x(&self) -> f64 {
        deg_to_rad(GRID_RES_X_DEG)
    }
    fn res_y(&self) -> f64 {
        deg_to_rad(GRID_RES_Y_DEG)
    }
    fn width(&self) -> usize {
        // 1 + (0.5 + (190 - 160) / 2) in the C++ GridSet constructor.
        16
    }
    fn height(&self) -> usize {
        // 1 + (0.5 + (-30 - -50) / 0.5) in the C++ GridSet constructor.
        41
    }
    fn long_lat_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64)> {
        Self::long_lat(ix, iy)
    }
    fn long_lat_z_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64, f64)> {
        let (lon, lat) = Self::long_lat(ix, iy)?;
        Some((lon, lat, Self::z(ix, iy)?))
    }
    fn z_offset(&self, ix: usize, iy: usize) -> Option<f64> {
        Self::z(ix, iy)
    }
}

fn degree_model(displacement: &str, time_function: Json) -> Json {
    let mut j = min_valid();
    set(&mut j, &["horizontal_offset_method"], s("addition"));
    set(&mut j, &["horizontal_offset_unit"], s("degree"));
    set(&mut j, &["vertical_offset_unit"], s("metre"));
    set(
        &mut j,
        &["components"],
        arr(vec![component(
            displacement,
            [160.0, -50.0, 190.0, -30.0],
            "bilinear",
            time_function,
        )]),
    );
    j
}

#[test]
fn defmodel_evaluator_horizontal_unit_degree() {
    let t_factor = 0.5;
    let j = degree_model("horizontal", half_piecewise());
    let provider = Provider {
        geographic: true,
        node: DegreeGrid,
    };
    let eval = build(&j, &provider, 1.0, 1.0);

    let i_queried_x = 1.0;
    let i_queried_y = 3.0;
    let long_offset_x = 0.01;
    let long_offset_xp1 = 0.02;
    let lat_offset_y = 0.03;
    let lat_offset_yp1 = 0.04;
    let z_offset_xy = 10.0;
    let z_offset_xp1_y = 11.0;
    let z_offset_xy_p1 = 11.0;
    let z_offset_xp1_yp1 = 12.0;
    let eps = 1e-9;
    let zval = 100.0;
    let t_valid = 2018.0;
    let grid_min_x = 160.0;
    let grid_min_y = -50.0;
    let grid_max_x = 190.0;
    let grid_max_y = -30.0;
    let model_min_x = 158.0;
    let model_min_y = -58.0;
    let model_max_x = 194.0;
    let model_max_y = -25.0;

    // Query on exact grid intersection.
    {
        let longitude = grid_min_x + i_queried_x * GRID_RES_X_DEG;
        let lat = grid_min_y + i_queried_y * GRID_RES_Y_DEG;
        let [x, y, z] = eval
            .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
            .expect("inside");
        near(rad_to_deg(x), longitude + t_factor * long_offset_x, eps);
        near(rad_to_deg(y), lat + t_factor * lat_offset_y, eps);
        assert_eq!(z, zval);
    }

    // Query between grid points.
    {
        let alpha_x = 0.25;
        let alpha_y = 0.125;
        let longitude = grid_min_x + i_queried_x * GRID_RES_X_DEG + alpha_x * GRID_RES_X_DEG;
        let lat = grid_min_y + i_queried_y * GRID_RES_Y_DEG + alpha_y * GRID_RES_Y_DEG;
        let [x, y, z] = eval
            .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
            .expect("inside");
        near(
            rad_to_deg(x),
            longitude + t_factor * (long_offset_x + alpha_x * (long_offset_xp1 - long_offset_x)),
            eps,
        );
        near(
            rad_to_deg(y),
            lat + t_factor * (lat_offset_y + alpha_y * (lat_offset_yp1 - lat_offset_y)),
            eps,
        );
        assert_eq!(z, zval);
    }

    // Outside the model extent, in longitude, latitude, and time.
    let at = |lon: f64, lat: f64, t: f64| eval.evaluate(deg_to_rad(lon), deg_to_rad(lat), zval, t);
    let grid_lat = grid_min_y + i_queried_y * GRID_RES_Y_DEG;
    let grid_lon = grid_min_x + i_queried_x * GRID_RES_X_DEG;
    assert!(at(model_min_x - 1e-1, grid_lat, t_valid).is_none());
    assert!(at(model_max_x + 1e-1, grid_lat, t_valid).is_none());
    assert!(at(grid_lon, model_min_y - 1e-1, t_valid).is_none());
    assert!(at(grid_lon, model_max_y + 1e-1, t_valid).is_none());
    assert!(at(grid_lon, grid_lat, 1000.0).is_none());
    assert!(at(grid_lon, grid_lat, 3000.0).is_none());

    // Outside the grid (but inside the component's extent): the component is
    // skipped and the point is returned unchanged.
    let identity = |lon: f64, lat: f64| {
        let [x, y, z] = at(lon, lat, t_valid).expect("skipped component, inside model");
        near(rad_to_deg(x), lon, eps);
        near(rad_to_deg(y), lat, eps);
        assert_eq!(z, zval);
    };
    // The component extent is [160, 190] x [-50, -30]: outside it, skipped.
    identity(grid_min_x - 1e-1, grid_lat);
    identity(grid_max_x + 1e-1, grid_lat);
    identity(grid_lon, grid_min_y - 1e-1);
    identity(grid_lon, grid_max_y + 1e-1);

    // Time function values to zero.
    {
        let [x, y, z] = at(grid_lon, grid_lat, 2000.0).expect("inside");
        near(rad_to_deg(x), grid_lon, eps);
        near(rad_to_deg(y), grid_lat, eps);
        assert_eq!(z, zval);
    }

    // Test vertical.
    let mut j_vertical = degree_model("vertical", half_piecewise());
    set(&mut j_vertical, &["vertical_offset_unit"], s("metre"));
    let eval_vertical = build(&j_vertical, &provider, 1.0, 1.0);
    {
        let alpha_x = 0.25;
        let alpha_y = 0.125;
        let longitude = grid_min_x + i_queried_x * GRID_RES_X_DEG + alpha_x * GRID_RES_X_DEG;
        let lat = grid_min_y + i_queried_y * GRID_RES_Y_DEG + alpha_y * GRID_RES_Y_DEG;
        let [x, y, z] = eval_vertical
            .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
            .expect("inside");
        near(rad_to_deg(x), longitude, eps);
        near(rad_to_deg(y), lat, eps);
        let z_bottom = z_offset_xy + alpha_x * (z_offset_xp1_y - z_offset_xy);
        let z_top = z_offset_xy_p1 + alpha_x * (z_offset_xp1_yp1 - z_offset_xy_p1);
        near(
            z,
            zval + t_factor * (z_bottom + alpha_y * (z_top - z_bottom)),
            eps,
        );
    }

    // Test 3d.
    let j_3d = degree_model("3d", half_piecewise());
    let eval_3d = build(&j_3d, &provider, 1.0, 1.0);
    {
        let alpha_x = 0.25;
        let alpha_y = 0.125;
        let longitude = grid_min_x + i_queried_x * GRID_RES_X_DEG + alpha_x * GRID_RES_X_DEG;
        let lat = grid_min_y + i_queried_y * GRID_RES_Y_DEG + alpha_y * GRID_RES_Y_DEG;
        let [x, y, z] = eval_3d
            .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
            .expect("inside");
        near(
            rad_to_deg(x),
            longitude + t_factor * (long_offset_x + alpha_x * (long_offset_xp1 - long_offset_x)),
            eps,
        );
        near(
            rad_to_deg(y),
            lat + t_factor * (lat_offset_y + alpha_y * (lat_offset_yp1 - lat_offset_y)),
            eps,
        );
        let z_bottom = z_offset_xy + alpha_x * (z_offset_xp1_y - z_offset_xy);
        let z_top = z_offset_xy_p1 + alpha_x * (z_offset_xp1_yp1 - z_offset_xy_p1);
        near(
            z,
            zval + t_factor * (z_bottom + alpha_y * (z_top - z_bottom)),
            eps,
        );
    }
}

// ---------------------------------------------------------------------------
// horizontal_unit_metre

/// `DeltaLongLatToEastingNorthing` of `test_defmodel.cpp`: the easting and
/// northing offsets of a longitude and latitude offset at latitude `phi`.
fn delta_long_lat_to_easting_northing(
    phi: f64,
    dlam: f64,
    dphi: f64,
    a: f64,
    b: f64,
) -> (f64, f64) {
    let sinphi = phi.sin();
    let cosphi = phi.cos();
    let a2 = a * a;
    let b2 = b * b;
    let x = a2 * (cosphi * cosphi) + b2 * (sinphi * sinphi);
    let sqrt_x = x.sqrt();
    (dlam * (a2 * cosphi) / sqrt_x, dphi * a2 * b2 / (sqrt_x * x))
}

/// The metre grid of `evaluator_horizontal_unit_metre`: nodes `(ix, iy)` of a
/// grid whose first row and column are one node before the model. Nodes 0 and
/// 1 of each axis both map to the first grid node, as the C++ mock does.
#[derive(Clone)]
struct MetreGrid {
    min_x: f64,
    min_y: f64,
    res_x: f64,
    res_y: f64,
}

/// The C++ mock stores float literals (`0.4f`) in doubles: `f64::from` of the
/// `f32` value gives the same double.
fn metre_easting_northing(slot: usize) -> (f64, f64) {
    let table = [(0.4_f32, -0.2_f32), (0.5, -0.25), (0.8, -0.4), (1.0, -0.3)];
    let (e, n) = table[slot];
    (f64::from(e), f64::from(n))
}

fn metre_z(slot: usize) -> f64 {
    f64::from([0.84_f32, 0.75, 0.36, 0.0][slot])
}

/// Index of the entry for `(ix, iy)` in the tables above, after the C++
/// `ix -= 1` and clamp to 0 for `-1`.
fn metre_slot(ix: usize, iy: usize) -> Option<usize> {
    let x = ix.saturating_sub(1);
    let y = iy.saturating_sub(1);
    match (x, y) {
        (0, 0) => Some(0),
        (1, 0) => Some(1),
        (0, 1) => Some(2),
        (1, 1) => Some(3),
        _ => None,
    }
}

impl Grid for MetreGrid {
    fn min_x(&self) -> f64 {
        self.min_x
    }
    fn min_y(&self) -> f64 {
        self.min_y
    }
    fn res_x(&self) -> f64 {
        self.res_x
    }
    fn res_y(&self) -> f64 {
        self.res_y
    }
    fn width(&self) -> usize {
        3
    }
    fn height(&self) -> usize {
        3
    }
    fn easting_northing_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64)> {
        Some(metre_easting_northing(metre_slot(ix, iy)?))
    }
    fn easting_northing_z_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64, f64)> {
        let (e, n) = self.easting_northing_offset(ix, iy)?;
        Some((e, n, self.z_offset(ix, iy)?))
    }
    fn z_offset(&self, ix: usize, iy: usize) -> Option<f64> {
        Some(metre_z(metre_slot(ix, iy)?))
    }
}

#[test]
fn defmodel_evaluator_horizontal_unit_metre() {
    let t_factor = 0.5;
    let grid_min_x = 165.8;
    let grid_min_y = -37.5;
    let grid_max_x = 166.2;
    let grid_max_y = -37.2;
    let grid_res_x = grid_max_x - grid_min_x;
    let grid_res_y = grid_max_y - grid_min_y;
    let extra_point_x = 1.0;
    let extra_point_y = 1.0;

    let grid = MetreGrid {
        min_x: deg_to_rad(grid_min_x - extra_point_x * grid_res_x),
        min_y: deg_to_rad(grid_min_y - extra_point_y * grid_res_y),
        res_x: deg_to_rad(grid_res_x),
        res_y: deg_to_rad(grid_res_y),
    };
    let a = 6_378_137.0;
    let b = 6_356_752.314_140;
    let t_valid = 2018.0;
    let zval = 100.0;

    let bbox = [
        grid_min_x - extra_point_x * grid_res_x,
        grid_min_y - extra_point_y * grid_res_y,
        grid_max_x,
        grid_max_y,
    ];
    let mut j = min_valid();
    set(&mut j, &["horizontal_offset_method"], s("addition"));
    set(&mut j, &["horizontal_offset_unit"], s("metre"));
    set(&mut j, &["vertical_offset_unit"], s("metre"));
    set(
        &mut j,
        &["components"],
        arr(vec![component(
            "horizontal",
            bbox,
            "XXXXXXX",
            half_piecewise(),
        )]),
    );
    let provider = Provider {
        geographic: true,
        node: grid.clone(),
    };

    // (longitude, lat, expected de, expected dn, expected dz, displacement,
    // interpolation), as in the C++ table.
    let test_points: [(f64, f64, f64, f64, f64, &str, &str); 14] = [
        (
            grid_min_x - extra_point_x * grid_res_x - 1e-11,
            grid_min_y - extra_point_y * grid_res_y - 1e-11,
            0.4,
            -0.2,
            0.0,
            "horizontal",
            "bilinear",
        ),
        (
            grid_min_x,
            grid_min_y,
            0.4,
            -0.2,
            0.0,
            "horizontal",
            "bilinear",
        ),
        (
            grid_max_x,
            grid_min_y,
            0.5,
            -0.25,
            0.0,
            "horizontal",
            "bilinear",
        ),
        (
            grid_min_x,
            grid_max_y,
            0.8,
            -0.4,
            0.0,
            "horizontal",
            "bilinear",
        ),
        (
            grid_max_x,
            grid_max_y,
            1.0,
            -0.3,
            0.0,
            "horizontal",
            "bilinear",
        ),
        (
            grid_max_x + 1e-11,
            grid_max_y + 1e-11,
            1.0,
            -0.3,
            0.0,
            "horizontal",
            "bilinear",
        ),
        (
            165.9,
            -37.3,
            0.70833334,
            -0.32083334,
            0.0,
            "horizontal",
            "bilinear",
        ),
        (
            165.9,
            -37.3,
            0.70833334,
            -0.32083334,
            0.4525,
            "3d",
            "bilinear",
        ),
        (
            grid_min_x,
            grid_min_y,
            0.4,
            -0.2,
            0.0,
            "horizontal",
            "geocentric_bilinear",
        ),
        (
            grid_max_x,
            grid_min_y,
            0.5,
            -0.25,
            0.0,
            "horizontal",
            "geocentric_bilinear",
        ),
        (
            grid_min_x,
            grid_max_y,
            0.8,
            -0.4,
            0.0,
            "horizontal",
            "geocentric_bilinear",
        ),
        (
            grid_max_x,
            grid_max_y,
            1.0,
            -0.3,
            0.0,
            "horizontal",
            "geocentric_bilinear",
        ),
        (
            165.9,
            -37.3,
            0.708_369_204_460_884_6,
            -0.320_964_233_971_140_5,
            0.0,
            "horizontal",
            "geocentric_bilinear",
        ),
        (
            165.9,
            -37.3,
            0.708_369_204_460_884_6,
            -0.320_964_233_971_140_5,
            0.4525,
            "3d",
            "geocentric_bilinear",
        ),
    ];

    for (longitude, lat, expected_de, expected_dn, expected_dz, displacement, interpolation) in
        test_points
    {
        let mut jp = j.clone();
        set(
            &mut jp,
            &["components", "0", "displacement_type"],
            s(displacement),
        );
        set(
            &mut jp,
            &["components", "0", "spatial_model", "interpolation_method"],
            s(interpolation),
        );
        let eval = build(&jp, &provider, a, b);
        let [new_long, new_lat, new_z] = eval
            .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
            .unwrap_or_else(|| panic!("{longitude} {lat} {displacement} {interpolation}"));
        near(new_z - zval, t_factor * expected_dz, 1e-8);
        let (de, dn) = delta_long_lat_to_easting_northing(
            deg_to_rad(lat),
            new_long - deg_to_rad(longitude),
            new_lat - deg_to_rad(lat),
            a,
            b,
        );
        near(de, t_factor * expected_de, 1e-8);
        near(dn, t_factor * expected_dn, 1e-8);

        if longitude == grid_min_x && lat == grid_min_y {
            // Redo the exact same evaluation, then after shifts, to test caching.
            let again = eval
                .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
                .expect("inside");
            assert_eq!(again, [new_long, new_lat, new_z]);

            assert!(eval
                .evaluate(
                    deg_to_rad(longitude - grid_res_x / 2.0),
                    deg_to_rad(lat),
                    zval,
                    t_valid
                )
                .is_some());
            let back = eval
                .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
                .expect("inside");
            assert_eq!(back, [new_long, new_lat, new_z]);

            assert!(eval
                .evaluate(
                    deg_to_rad(longitude),
                    deg_to_rad(lat - grid_res_y / 2.0),
                    zval,
                    t_valid
                )
                .is_some());
            let back = eval
                .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
                .expect("inside");
            assert_eq!(back, [new_long, new_lat, new_z]);
        }
    }

    // Test inverse().
    {
        let mut jp = j.clone();
        set(&mut jp, &["components", "0", "displacement_type"], s("3d"));
        set(
            &mut jp,
            &["components", "0", "spatial_model", "interpolation_method"],
            s("bilinear"),
        );
        let eval = build(&jp, &provider, a, b);
        let longitude = 165.9;
        let lat = -37.3;
        let [new_long, new_lat, new_z] = eval
            .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
            .expect("inside");
        let [inv_long, inv_lat, inv_z] = eval
            .evaluate_inverse(new_long, new_lat, new_z, t_valid)
            .expect("converges");
        near(rad_to_deg(inv_long), longitude, 1e-10);
        near(rad_to_deg(inv_lat), lat, 1e-10);
        near(inv_z, zval, 1e-4);
    }

    // Test horizontal_offset_method = geocentric.
    {
        let mut jp = j.clone();
        set(&mut jp, &["horizontal_offset_method"], s("geocentric"));
        set(&mut jp, &["components", "0", "displacement_type"], s("3d"));
        set(
            &mut jp,
            &["components", "0", "spatial_model", "interpolation_method"],
            s("bilinear"),
        );
        let eval = build(&jp, &provider, a, b);
        let longitude = grid_min_x;
        let lat = grid_min_y;
        let [new_long, new_lat, new_z] = eval
            .evaluate(deg_to_rad(longitude), deg_to_rad(lat), zval, t_valid)
            .expect("inside");
        let expected_de = 0.400_000_009_480_813_27;
        let expected_dn = -0.199_999_998_105_426_82;
        let (de, dn) = delta_long_lat_to_easting_northing(
            deg_to_rad(lat),
            new_long - deg_to_rad(longitude),
            new_lat - deg_to_rad(lat),
            a,
            b,
        );
        near(de, t_factor * expected_de, 1e-10);
        near(dn, t_factor * expected_dn, 1e-9);
        near(new_z - zval, t_factor * 0.84, 1e-4);
    }
}

// ---------------------------------------------------------------------------
// projected_crs

#[test]
fn defmodel_evaluator_projected_crs() {
    let grid_min_x = 10_000.0;
    let grid_min_y = 20_000.0;
    let grid_max_x = 30_000.0;
    let grid_max_y = 40_000.0;
    #[derive(Clone)]
    struct ProjectedGrid;
    impl Grid for ProjectedGrid {
        fn min_x(&self) -> f64 {
            10_000.0
        }
        fn min_y(&self) -> f64 {
            20_000.0
        }
        fn res_x(&self) -> f64 {
            20_000.0
        }
        fn res_y(&self) -> f64 {
            20_000.0
        }
        fn width(&self) -> usize {
            2
        }
        fn height(&self) -> usize {
            2
        }
        fn easting_northing_offset(&self, ix: usize, iy: usize) -> Option<(f64, f64)> {
            match (ix, iy) {
                (0, 0) => Some((0.4, -0.2)),
                (1, 0) => Some((0.5, -0.25)),
                (0, 1) => Some((0.8, -0.4)),
                (1, 1) => Some((1.0, -0.3)),
                _ => None,
            }
        }
    }

    let a = 6_378_137.0;
    let b = 6_356_752.314_140;
    let t_valid = 2018.0;
    let zval = 100.0;

    let bbox = [grid_min_x, grid_min_y, grid_max_x, grid_max_y];
    let mut j = min_valid();
    set(
        &mut j,
        &["extent", "parameters", "bbox"],
        arr(bbox.iter().map(|v| n(*v)).collect()),
    );
    set(&mut j, &["horizontal_offset_method"], s("addition"));
    set(&mut j, &["horizontal_offset_unit"], s("metre"));
    set(&mut j, &["vertical_offset_unit"], s("metre"));
    set(
        &mut j,
        &["components"],
        arr(vec![component("horizontal", bbox, "bilinear", constant())]),
    );
    let provider = Provider {
        geographic: false,
        node: ProjectedGrid,
    };

    let eval = build(&j, &provider, a, b);
    let [new_x, new_y, new_z] = eval
        .evaluate(grid_min_x, grid_min_y, zval, t_valid)
        .expect("inside");
    near(new_x - grid_min_x, 0.4, 1e-8);
    near(new_y - grid_min_y, -0.2, 1e-8);
    near(new_z - zval, 0.0, 1e-8);

    // Projected CRS with the degree unit: refused.
    {
        let mut jc = j.clone();
        set(&mut jc, &["horizontal_offset_unit"], s("degree"));
        let mf = MasterFile::parse(&jc.dump()).expect("valid model");
        assert!(matches!(
            Evaluator::new(mf, &provider, a, b),
            Err(DefmodelError::Evaluator(_))
        ));
    }
    // Projected CRS with the geocentric method: refused.
    {
        let mut jc = j.clone();
        set(&mut jc, &["horizontal_offset_method"], s("geocentric"));
        let mf = MasterFile::parse(&jc.dump()).expect("valid model");
        assert!(matches!(
            Evaluator::new(mf, &provider, a, b),
            Err(DefmodelError::Evaluator(_))
        ));
    }
    // Projected CRS with the geocentric bilinear interpolation: refused.
    {
        let mut jc = j.clone();
        set(
            &mut jc,
            &["components", "0", "spatial_model", "interpolation_method"],
            s("geocentric_bilinear"),
        );
        let mf = MasterFile::parse(&jc.dump()).expect("valid model");
        assert!(matches!(
            Evaluator::new(mf, &provider, a, b),
            Err(DefmodelError::Evaluator(_))
        ));
    }
}
