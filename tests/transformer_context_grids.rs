//! A `Transformer` built under a context finds the grids of that context's search path, whichever
//! way the candidates are filtered, and `Ballpark::Forbidden` leaves no ballpark operation.
//!
//! `EPSG:4979` to `EPSG:4326+5773` (EGM96) has a grid operation (`us_nga_egm96_15.tif`) and a
//! ballpark one. With the grid in the search path and ballparks forbidden, one candidate remains
//! and `build` creates it; that creation must run under the context, or the grid is not found
//! (`no operation: illegal value for +grids`). With no grid anywhere, the ballpark operation is
//! the only one left and `Forbidden` refuses it, as PROJ's `--no-ballpark` does.

use proj_rs::{Ballpark, Context, Crs, Network, Point, Transformer};
use std::fs;
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("proj-rs-ctx-grids-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

/// A directory with a vertical GeoTIFF grid under the name the database gives the EGM96 grid.
/// The content is a 430-byte test grid that opens as a vertical shift: what matters is that the
/// operation is created, which opens the file by that name in the search path of the context.
fn with_egm96_grid(name: &str) -> PathBuf {
    let dir = scratch(name);
    let extract =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("data/tests/test_vgrid_pixelisarea.tif");
    fs::copy(extract, dir.join("us_nga_egm96_15.tif")).expect("copy the grid");
    dir
}

fn names(search_path: &Path, ballpark: Ballpark) -> Result<Vec<String>, String> {
    let ctx = Context::builder()
        .search_path(search_path)
        .network(Network::Disabled)
        .build();
    let source = Crs::from_user_input(&ctx, "EPSG:4979").expect("source");
    let target = Crs::from_user_input(&ctx, "EPSG:4326+5773").expect("target");
    let transformer = Transformer::builder(&source, &target)
        .normalize_axis_order()
        .allow_ballpark(ballpark)
        .build()
        .map_err(|error| error.to_string())?;
    let operations = transformer
        .operations()
        .map_err(|error| error.to_string())?;
    Ok(operations
        .iter()
        .map(|operation| operation.name().unwrap_or_default().to_owned())
        .collect())
}

fn is_ballpark(name: &str) -> bool {
    name.to_ascii_lowercase().contains("ballpark")
}

#[test]
fn a_single_remaining_candidate_is_created_under_the_context() {
    let grids = with_egm96_grid("forbidden-with-grid");
    let names = names(&grids, Ballpark::Forbidden).expect("the grid is in the search path");
    assert!(!names.is_empty());
    assert!(
        names.iter().all(|name| !is_ballpark(name)),
        "no ballpark operation: {names:?}"
    );
    assert!(
        names.iter().any(|name| name.contains("EGM96 height")),
        "the grid operation: {names:?}"
    );
}

#[test]
fn ballpark_forbidden_without_the_grid_never_falls_back_to_a_ballpark() {
    let empty = scratch("forbidden-no-grid");
    let ctx = Context::builder()
        .search_path(&empty)
        .network(Network::Disabled)
        .build();
    let source = Crs::from_user_input(&ctx, "EPSG:4979").expect("source");
    let target = Crs::from_user_input(&ctx, "EPSG:4326+5773").expect("target");
    let built = Transformer::builder(&source, &target)
        .normalize_axis_order()
        .allow_ballpark(Ballpark::Forbidden)
        .build();
    // PROJ keeps the grid operation, whose grid is missing, and fails when it is used.
    let Ok(transformer) = built else {
        return;
    };
    let names: Vec<String> = transformer
        .operations()
        .expect("operations")
        .iter()
        .map(|operation| operation.name().unwrap_or_default().to_owned())
        .collect();
    assert!(
        names.iter().all(|name| !is_ballpark(name)),
        "a forbidden ballpark is not a candidate: {names:?}"
    );
    let point = transformer.forward(Point::xyz(9.0, 42.0, 0.0));
    assert!(
        point.is_err(),
        "the grid is missing and a ballpark is forbidden, so no result: {point:?}"
    );
}

#[test]
fn ballpark_allowed_without_the_grid_uses_the_ballpark_operation() {
    let empty = scratch("allowed-no-grid");
    let names = names(&empty, Ballpark::Allowed).expect("a ballpark operation");
    assert!(names.iter().any(|name| is_ballpark(name)), "{names:?}");
}

#[test]
fn ballpark_allowed_with_the_grid_chooses_the_grid_operation() {
    let grids = with_egm96_grid("allowed-with-grid");
    let names = names(&grids, Ballpark::Allowed).expect("the grid is in the search path");
    assert!(
        names
            .first()
            .is_some_and(|name| name.contains("EGM96 height") && !is_ballpark(name)),
        "the best candidate has the grid: {names:?}"
    );
}

/// A point inside the extract of the grid in the test file, and one far outside it.
const INSIDE: (f64, f64) = (5.0, 53.0);
const OUTSIDE: (f64, f64) = (-179.0, -89.0);

fn transformer(search_path: &Path, ballpark: Ballpark) -> Transformer {
    let ctx = Context::builder()
        .search_path(search_path)
        .network(Network::Disabled)
        .build();
    let source = Crs::from_user_input(&ctx, "EPSG:4979").expect("source");
    let target = Crs::from_user_input(&ctx, "EPSG:4326+5773").expect("target");
    Transformer::builder(&source, &target)
        .normalize_axis_order()
        .allow_ballpark(ballpark)
        .build()
        .expect("the pair builds")
}

#[test]
fn the_single_operation_is_named_and_not_a_ballpark() {
    // `Forbidden` with the grid leaves one operation: the application refuses a ballpark
    // conversion by reading what the last point went through.
    let grids = with_egm96_grid("single-named");
    let t = transformer(&grids, Ballpark::Forbidden);
    let info = t
        .last_used_operation()
        .expect("the single operation is known at once");
    assert_eq!(info.name(), Some("WGS 84 to EGM96 height (1)"));
    assert!(!info.is_ballpark());
    t.forward(Point::xyz(INSIDE.0, INSIDE.1, 0.0))
        .expect("a point of the grid");
    let info = t.last_used_operation().expect("after a conversion");
    assert_eq!(info.name(), Some("WGS 84 to EGM96 height (1)"));
    assert!(!info.is_ballpark());
    assert!(info
        .proj_string()
        .is_some_and(|text| text.contains("vgridshift")));
}

#[test]
fn a_ballpark_conversion_says_so() {
    // `Allowed`: a point outside the grid goes through the ballpark operation, a point inside it
    // through the grid.
    let grids = with_egm96_grid("ballpark-says-so");
    let t = transformer(&grids, Ballpark::Allowed);
    t.forward(Point::xyz(OUTSIDE.0, OUTSIDE.1, 0.0))
        .expect("a point outside the grid");
    assert!(t.last_used_operation().expect("an operation").is_ballpark());
    t.forward(Point::xyz(INSIDE.0, INSIDE.1, 0.0))
        .expect("a point of the grid");
    let info = t.last_used_operation().expect("an operation");
    assert!(!info.is_ballpark(), "{:?}", info.name());
}
