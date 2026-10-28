//! Deformation models: `+proj=deformation`, `+proj=defmodel`, `+proj=tinshift`.
//!
//! PROJ 9.9 sources: `src/transformations/deformation.cpp`,
//! `src/transformations/defmodel.cpp` with `defmodel_impl.hpp`, and
//! `src/transformations/tinshift.cpp` with the JSON and GeoPackage evaluators.
//! A missing model file is [`CreateError::Unsupported`], so a coordinate is
//! never invented for a grid this crate does not have.

#![allow(
    clippy::pedantic,
    reason = "the evaluators follow the PROJ deformation sources"
)]

mod defmodel;
mod deformation;
mod grid;
mod inflate;
mod json;
mod tinshift;

use std::path::{Path, PathBuf};

use crate::engine::{CreateError, Frame, Params, Step};

const GLOBAL_KEYS: &[&str] = &[
    "proj",
    "ellps",
    "datum",
    "a",
    "b",
    "rf",
    "f",
    "es",
    "e",
    "R",
    "R_A",
    "R_V",
    "R_a",
    "R_g",
    "R_h",
    "R_lat_a",
    "R_lat_g",
    "R_C",
    "lon_0",
    "lat_0",
    "x_0",
    "y_0",
    "z_0",
    "t_0",
    "k",
    "k_0",
    "units",
    "to_meter",
    "vunits",
    "vto_meter",
    "no_defs",
    "wktext",
    "type",
    "pm",
    "axis",
    "over",
    "geoc",
    "lon_wrap",
    "inv",
    "omit_fwd",
    "omit_inv",
];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "deformation" => Some(deformation::keys()),
        "defmodel" => Some(defmodel::keys()),
        "tinshift" => Some(tinshift::keys()),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    let extra = match name {
        "deformation" | "defmodel" | "tinshift" => extra_keys(name).unwrap_or(&[]),
        _ => return Ok(None),
    };
    for (key, _) in params.entries() {
        if !GLOBAL_KEYS.contains(&key) && !extra.contains(&key) {
            return Err(CreateError::UnknownParameter(key.to_owned()));
        }
    }
    let step = match name {
        "deformation" => deformation::build(params, frame)?,
        "defmodel" => defmodel::build(params, frame)?,
        "tinshift" => tinshift::build(params, frame)?,
        _ => return Ok(None),
    };
    Ok(Some(step))
}

fn resolve(name: &str) -> Result<PathBuf, CreateError> {
    let name = cdn_name(name).unwrap_or(name);
    if name.starts_with("http://") || name.starts_with("https://") {
        return Err(CreateError::IllegalValue("file"));
    }
    for candidate in candidates(name) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(CreateError::IllegalValue("file"))
}

/// `https://cdn.proj.org/<file>` is the file of that name under the grid search path.
fn cdn_name(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("https://cdn.proj.org/")?;
    if rest.is_empty() || rest.contains('/') || rest.contains('\\') || rest.contains('?') {
        return None;
    }
    Some(rest)
}

fn read_resource(name: &str) -> Result<Vec<u8>, CreateError> {
    let path = resolve(name)?;
    let bytes = std::fs::read(&path).map_err(|_| CreateError::IllegalValue("file"))?;
    if bytes.len() > 100 * 1024 * 1024 {
        return Err(CreateError::IllegalValue("file"));
    }
    Ok(bytes)
}

fn candidates(name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let given = Path::new(name);
    if given.is_absolute() {
        out.push(given.to_path_buf());
    } else {
        out.push(PathBuf::from(name));
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        out.push(root.join("data").join(name));
        out.push(root.join(name));
        for directory in crate::gridpath::directories() {
            out.push(directory.join(name));
            if let Some(file_name) = given.file_name() {
                if file_name != given.as_os_str() {
                    out.push(directory.join(file_name));
                }
            }
        }
    }
    for variable in ["PROJ_DATA", "PROJ_LIB"] {
        if let Some(value) = std::env::var_os(variable) {
            let text = value.to_string_lossy();
            for directory in text.split([';', '|']) {
                if !directory.is_empty() {
                    out.push(Path::new(directory).join(name));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::engine::{Coord, Operation};

    fn metres(got: Coord, expect: (f64, f64, f64)) -> f64 {
        let dx = got.x - expect.0;
        let dy = got.y - expect.1;
        let dz = got.z - expect.2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[test]
    fn tinshift_json_matches_the_gie_points() {
        let implicit = Operation::create("+proj=tinshift +file=tests/tinshift_crs_implicit.json")
            .expect("implicit");
        let got = implicit.forward(Coord::xy(2.0, 49.0)).expect("inside");
        assert!(metres(got, (2.1, 49.1, 0.0)) < 1e-9, "{got:?}");
        assert!(implicit.forward(Coord::xy(0.0, 0.0)).is_err());
        let side =
            Operation::create("+proj=tinshift +file=tests/tinshift_fallback_nearest_side.json")
                .expect("side");
        let got = side.forward(Coord::xy(2.0, 3.0)).expect("side point");
        assert!(metres(got, (4.0, 6.0, 0.0)) < 1e-9, "{got:?}");
        let centroid =
            Operation::create("+proj=tinshift +file=tests/tinshift_fallback_nearest_centroid.json")
                .expect("centroid");
        let got = centroid
            .forward(Coord::xy(3.0, 0.0))
            .expect("centroid point");
        assert!(metres(got, (3.0, 0.0, 0.0)) < 1e-9, "{got:?}");
        let kkj = Operation::create("+proj=tinshift +file=tests/tinshift_simplified_kkj_etrs.json")
            .expect("kkj");
        let got = kkj
            .forward(Coord::xy(3_210_000.0, 6_700_000.0))
            .expect("kkj point");
        assert!(
            metres(got, (209_948.321_7, 6_697_187.000_9, 0.0)) < 0.0001,
            "{got:?}"
        );
        let height =
            Operation::create("+proj=tinshift +file=tests/tinshift_simplified_n60_n2000.json")
                .expect("n60");
        let got = height
            .forward(Coord {
                x: 3_210_000.0,
                y: 6_700_000.0,
                z: 10.0,
                t: 0.0,
            })
            .expect("n60 point");
        assert!(
            metres(got, (3_210_000.0, 6_700_000.0, 10.2886)) < 0.0001,
            "{got:?}"
        );
    }

    #[test]
    fn tinshift_gpkg_matches_the_json_cases() {
        let implicit = Operation::create("+proj=tinshift +file=tests/tinshift_crs_implicit.gpkg")
            .expect("gpkg");
        let got = implicit.forward(Coord::xy(2.0, 49.0)).expect("inside");
        assert!(metres(got, (2.1, 49.1, 0.0)) < 1e-6, "{got:?}");
        assert!(Operation::create("+proj=tinshift +file=tests/tinshift_empty_file.gpkg").is_err());
        assert!(Operation::create("+proj=tinshift").is_err());
    }

    #[test]
    fn defmodel_degree_and_vertical_match_the_gie_points() {
        let horizontal =
            Operation::create("+proj=defmodel +model=tests/simple_model_degree_horizontal.json")
                .expect("horizontal");
        let input = Coord {
            x: 2.0_f64.to_radians(),
            y: 49.0_f64.to_radians(),
            z: 30.0,
            t: 2020.0,
        };
        let got = horizontal.forward(input).expect("forward");
        let dx = (got.x - 3.0_f64.to_radians()) * 6_378_137.0 * 49.0_f64.to_radians().cos();
        let dy = (got.y - 51.0_f64.to_radians()) * 6_378_137.0;
        assert!(
            (dx * dx + dy * dy).sqrt() < 0.0001 && (got.z - 30.0).abs() < 1e-6,
            "{got:?} dx={dx} dy={dy}"
        );
        assert!(horizontal
            .forward(Coord {
                x: input.x,
                y: input.y,
                z: 30.0,
                t: f64::INFINITY,
            })
            .is_err());
        let vertical =
            Operation::create("+proj=defmodel +model=tests/simple_model_metre_vertical.json")
                .expect("vertical");
        let got = vertical.forward(input).expect("vertical forward");
        assert!((got.z - 33.0).abs() < 1e-4, "{got:?}");
        let wrapped = vertical
            .forward(Coord {
                x: 362.0_f64.to_radians(),
                y: 49.0_f64.to_radians(),
                z: 30.0,
                t: 2020.0,
            })
            .expect("wrap");
        assert!((wrapped.z - 33.0).abs() < 1e-4, "{wrapped:?}");
    }

    #[test]
    fn deformation_extract_matches_the_gie_pipeline() {
        let pipeline = Operation::create(
            "+proj=pipeline +step +proj=cart +ellps=GRS80 +step +proj=deformation +grids=tests/nkgrf03vel_realigned_extract.tif +ellps=GRS80 +dt=1 +step +inv +proj=cart +ellps=GRS80",
        )
        .expect("pipeline");
        let input = Coord {
            x: 21.5_f64.to_radians(),
            y: 63.0_f64.to_radians(),
            z: 0.0,
            t: 0.0,
        };
        let got = pipeline.forward(input).expect("forward");
        let expect = Coord {
            x: 21.500_000_004_9_f64.to_radians(),
            y: 62.999_999_993_7_f64.to_radians(),
            z: 0.0083,
            t: 0.0,
        };
        let (a, es) = pipeline.distance_ellipsoid();
        let distance = crate::engine::lpz_dist(a, es, expect, got);
        assert!(distance < 5e-5, "tif distance {distance} got {got:?}");
        let split = Operation::create(
            "+proj=pipeline +step +proj=cart +ellps=GRS80 +step +proj=deformation +xy_grids=tests/nkgrf03vel_realigned_xy_extract.ct2 +z_grids=tests/nkgrf03vel_realigned_z_extract.gtx +ellps=GRS80 +dt=1 +step +inv +proj=cart +ellps=GRS80",
        )
        .expect("split");
        let got = split.forward(input).expect("split forward");
        let distance = crate::engine::lpz_dist(a, es, expect, got);
        assert!(distance < 5e-5, "ct2 distance {distance} got {got:?}");
        let back = pipeline.inverse(got).expect("roundtrip");
        let distance = crate::engine::lpz_dist(a, es, input, back);
        assert!(distance < 5e-5, "roundtrip {distance}");
    }

    fn ang(lon: f64, lat: f64, z: f64, t: f64) -> Coord {
        Coord {
            x: lon.to_radians(),
            y: lat.to_radians(),
            z,
            t,
        }
    }

    fn angular_close(operation: &Operation, got: Coord, lon: f64, lat: f64, z: f64) {
        let expect = ang(lon, lat, z, got.t);
        let (a, es) = operation.distance_ellipsoid();
        let distance = crate::engine::lpz_dist(a, es, expect, got);
        assert!(distance < 1e-4, "distance {distance} got {got:?}");
    }

    #[test]
    fn defmodel_remaining_gie_points() {
        let three = Operation::create("+proj=defmodel +model=tests/simple_model_degree_3d.json")
            .expect("3d");
        angular_close(
            &three,
            three
                .forward(ang(2.0, 49.0, 30.0, 2020.0))
                .expect("forward"),
            3.0,
            51.0,
            33.0,
        );
        let metre = Operation::create(
            "+proj=pipeline +step +inv +proj=merc +step +proj=defmodel +model=tests/simple_model_metre_horizontal.json +step +proj=merc",
        )
        .expect("metre");
        let got = metre
            .forward(Coord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                t: 2020.0,
            })
            .expect("forward");
        assert!(metres(got, (11.0, 22.0, 30.0)) < 1e-4, "{got:?}");
        let three_m = Operation::create(
            "+proj=pipeline +step +inv +proj=merc +step +proj=defmodel +model=tests/simple_model_metre_3d.json +step +proj=merc",
        )
        .expect("metre 3d");
        let got = three_m
            .forward(Coord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                t: 2020.0,
            })
            .expect("forward");
        assert!(metres(got, (11.0, 22.0, 33.0)) < 1e-4, "{got:?}");
        let geo = Operation::create(
            "+proj=pipeline +step +inv +proj=merc +step +proj=defmodel +model=tests/simple_model_metre_3d_geocentric.json +step +proj=merc",
        )
        .expect("geocentric");
        let got = geo
            .forward(Coord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                t: 2020.0,
            })
            .expect("forward");
        assert!(metres(got, (11.0, 22.0, 33.0)) < 1e-4, "{got:?}");
        let projected = Operation::create(
            "+proj=pipeline +step +proj=defmodel +model=tests/simple_model_projected.json",
        )
        .expect("projected");
        let cases = [
            (
                1_500_200.0,
                5_400_400.0,
                30.0,
                1_500_200.588,
                5_400_399.722,
                30.6084,
            ),
            (
                1_500_000.0,
                5_400_000.0,
                30.0,
                1_500_000.4,
                5_399_999.8,
                30.84,
            ),
            (
                1_501_000.0,
                5_400_000.0,
                30.0,
                1_501_000.5,
                5_399_999.75,
                30.75,
            ),
            (
                1_500_000.0,
                5_401_000.0,
                30.0,
                1_500_000.8,
                5_400_999.6,
                30.36,
            ),
            (
                1_501_000.0,
                5_401_000.0,
                30.0,
                1_501_001.0,
                5_400_999.7,
                30.0,
            ),
        ];
        for (x, y, z, ex, ey, ez) in cases {
            let got = projected
                .forward(Coord { x, y, z, t: 2020.0 })
                .expect("forward");
            assert!(
                metres(got, (ex, ey, ez)) < 1e-4,
                "{got:?} vs {ex} {ey} {ez}"
            );
        }
        let east = Operation::create("+proj=defmodel +model=tests/simple_model_wrap_east.json")
            .expect("east");
        angular_close(
            &east,
            east.forward(ang(165.9, -37.3, 10.0, 2020.0))
                .expect("forward"),
            165.9,
            -37.3,
            10.4525,
        );
        let west = Operation::create("+proj=defmodel +model=tests/simple_model_wrap_west.json")
            .expect("west");
        angular_close(
            &west,
            west.forward(ang(165.9, -37.3, 10.0, 2020.0))
                .expect("forward"),
            165.9,
            -37.3,
            10.4525,
        );
        let polar = Operation::create("+proj=defmodel +model=tests/simple_model_polar.json")
            .expect("polar");
        let checks = [
            (20.0, -90.0, 15.0, 27.474_324_536_5, -89.999_974_772_1, 18.0),
            (
                120.0,
                -90.0,
                15.0,
                27.473_793_409_8,
                -89.999_974_771_8,
                18.0,
            ),
            (
                235.0,
                -89.5,
                15.0,
                -124.998_663_857_1,
                -89.500_022_370_8,
                17.375,
            ),
            (
                45.0,
                -89.5,
                15.0,
                44.999_129_539_2,
                -89.499_975_943_8,
                18.5469,
            ),
        ];
        for (lon, lat, z, elon, elat, ez) in checks {
            angular_close(
                &polar,
                polar.forward(ang(lon, lat, z, 2020.0)).expect("forward"),
                elon,
                elat,
                ez,
            );
        }
    }

    #[test]
    fn tinshift_gpkg_remaining_and_missing_files() {
        let kkj = Operation::create("+proj=tinshift +file=tests/tinshift_simplified_kkj_etrs.gpkg")
            .expect("kkj gpkg");
        let got = kkj
            .forward(Coord::xy(3_210_000.0, 6_700_000.0))
            .expect("forward");
        assert!(
            metres(got, (209_948.321_7, 6_697_187.000_9, 0.0)) < 1e-4,
            "{got:?}"
        );
        let n60 =
            Operation::create("+proj=tinshift +file=tests/tinshift_simplified_n60_n2000.gpkg")
                .expect("n60 gpkg");
        let got = n60
            .forward(Coord {
                x: 3_210_000.0,
                y: 6_700_000.0,
                z: 10.0,
                t: 0.0,
            })
            .expect("forward");
        assert!(
            metres(got, (3_210_000.0, 6_700_000.0, 10.2886)) < 1e-4,
            "{got:?}"
        );
        let side =
            Operation::create("+proj=tinshift +file=tests/tinshift_fallback_nearest_side.gpkg")
                .expect("side gpkg");
        let got = side.forward(Coord::xy(2.0, 3.0)).expect("forward");
        assert!(metres(got, (4.0, 6.0, 0.0)) < 1e-6, "{got:?}");
        assert!(Operation::create("+proj=tinshift +file=i_do_not_exist").is_err());
        assert!(Operation::create("+proj=defmodel +model=proj.ini").is_err());
        assert!(Operation::create("+proj=deformation +xy_grids=alaska +dt=1").is_err());
    }
}
