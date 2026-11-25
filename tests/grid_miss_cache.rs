//! A grid that did not open is not tried again by the same context while PROJ keeps its
//! `lookForGridInfo` record: the per-context LRU of 128 keys, allowed to grow to 138, of
//! `cacheGridInfo_` (`factory.cpp:860-871, 3398-3539`). The count of `Cannot open <url>: <error>`
//! lines of each listing therefore depends on the listings the context made before.
//!
//! The expected counts were measured with the libproj of PROJ 9.9.0 built by
//! `tools/build-reference.sh` (no libcurl: every network open answers
//! `Network functionality not available`), `PROJ_DATA` its `build/data` (no grid), through
//! `capi/tests/grid_miss_cache.c`, which `capi/tests/run.sh grid_miss_cache` compares with the C
//! library of proj-rs. Here the same refusal comes from the application's callbacks.

use std::cell::Cell;
use std::sync::Arc;

use proj_rs::iso19111::{
    create_from_user_input, BoundingBox, CoordinateOperationContext, CrsExtentUse, DatabaseContext,
    GridAvailabilityUse, ObjectDomain, SpatialCriterion,
};
use proj_rs::unstable::{replace_log_sink, LogSink};
use proj_rs::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply, ProjBuilder};

/// `no_op_network_open` of PROJ built without libcurl (`networkfilemanager.cpp`).
struct NoNetwork;

impl NetworkCallbacks for NoNetwork {
    fn open(&self, _: &str, _: u64, _: usize) -> Result<OpenReply, NetworkError> {
        Err(NetworkError::Open(
            "Network functionality not available".to_owned(),
        ))
    }

    fn read_range(&self, _: NetworkHandle, _: u64, _: usize) -> Result<Vec<u8>, NetworkError> {
        Err(NetworkError::ReadRange(
            "Network functionality not available".to_owned(),
        ))
    }

    fn close(&self, _: NetworkHandle) {}
}

thread_local! {
    static CANNOT_OPEN: Cell<usize> = const { Cell::new(0) };
}

fn record(_owner: usize, line: &str) {
    if line.starts_with("Cannot open ") {
        CANNOT_OPEN.with(|count| count.set(count.get() + 1));
    }
}

fn builder() -> ProjBuilder {
    let empty =
        std::env::temp_dir().join(format!("proj-rs-grid-miss-cache-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("empty search path");
    let mut builder = ProjBuilder::new();
    builder.replace_search_paths(vec![empty]);
    builder.set_network_callbacks(Arc::new(NoNetwork));
    builder.enable_network(true).expect("network on");
    builder
}

#[derive(Clone, Copy)]
enum Area {
    None,
    Texas,
}

/// The operations of `source` to `target` (crs extent none, partial intersection) and the
/// `Cannot open` lines of the call.
fn listing(
    builder: &ProjBuilder,
    source: &str,
    target: &str,
    grids: GridAvailabilityUse,
    area: Area,
) -> (usize, usize) {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_crs_extent_use(CrsExtentUse::Ignore)
        .set_grid_availability_use(grids);
    if let Area::Texas = area {
        context.set_area_of_interest(Some(ObjectDomain::from_bounding_box(BoundingBox::new(
            -100.0, 30.0, -90.0, 40.0,
        ))));
    }
    CANNOT_OPEN.with(|count| count.set(0));
    let previous = replace_log_sink(Some(LogSink::new(0, record)));
    let operations = builder.create_operations(&source, &target, &context);
    replace_log_sink(previous);
    (
        operations.expect("operations").len(),
        CANNOT_OPEN.with(Cell::get),
    )
}

const SORTING: GridAvailabilityUse = GridAvailabilityUse::UseForSorting;
const DISCARD: GridAvailabilityUse = GridAvailabilityUse::DiscardIfMissing;

/// The listings of `capi/tests/grid_miss_cache.c` on one context: the second sort retries the 33
/// grids whose records the area-of-interest listing and the rejection pushed out of the cache.
#[test]
fn a_context_retries_a_grid_once_its_record_is_evicted() {
    let context = builder();
    let measured = [
        listing(&context, "EPSG:5070", "EPSG:4326", SORTING, Area::None),
        listing(&context, "EPSG:5070", "EPSG:4326", DISCARD, Area::None),
        listing(&context, "EPSG:5070", "EPSG:4326", SORTING, Area::Texas),
        listing(&context, "EPSG:5070", "EPSG:4326", SORTING, Area::None),
        listing(&context, "EPSG:4326", "EPSG:5070", SORTING, Area::None),
    ];
    assert_eq!(measured, [(53, 49), (4, 0), (11, 1), (53, 33), (53, 0)]);
}

/// The rejection of `DISCARD_OPERATION_IF_MISSING_GRID` looks the grids up with the network off
/// (`DisableNetwork`, `factory.cpp:4333-4359`): it opens nothing, and the sort that follows on the
/// same context still tries each grid once.
#[test]
fn the_rejection_opens_nothing() {
    let context = builder();
    assert_eq!(
        listing(&context, "EPSG:5070", "EPSG:4326", DISCARD, Area::None),
        (4, 0)
    );
    assert_eq!(
        listing(&context, "EPSG:5070", "EPSG:4326", SORTING, Area::None),
        (53, 49)
    );
}

/// A builder with its records reset is a new `DatabaseContext`: it tries each grid again; its
/// clones share its records, as the objects of one PROJ context share its database context.
#[test]
fn records_belong_to_the_builder_and_its_clones() {
    let context = builder();
    assert_eq!(
        listing(&context, "EPSG:4326", "EPSG:5070", SORTING, Area::None),
        (53, 49)
    );
    let clone = context.clone();
    assert_eq!(
        listing(&clone, "EPSG:4326", "EPSG:5070", SORTING, Area::None),
        (53, 0)
    );
    let mut fresh = context.clone();
    fresh.reset_grid_records();
    assert_eq!(
        listing(&fresh, "EPSG:4326", "EPSG:5070", SORTING, Area::None),
        (53, 49)
    );
    assert_eq!(
        listing(&context, "EPSG:4326", "EPSG:5070", SORTING, Area::None),
        (53, 0)
    );
}

/// `proj_create_crs_to_crs_from_pj` with `ONLY_BEST=NO` and the network on asks
/// `KNOWN_AVAILABLE` (`crs_to_crs.cpp:566-573`): the pair is built without opening a grid, the
/// grid operations stay candidates, and a candidate opens its grid at its first point only
/// (`defer_grid_opening`, `hgridshift.cpp:30-41`). Measured with `capi/tests/grid_miss_cache.c`
/// (the `only_best_default = off` of the reference `proj.ini`, no option): the points of the
/// north edge of `EPSG:5070` take `NAD83 to WGS 84 (6)` (`ca_nrc_NA83SCRS.tif`), fail with a
/// network error, and only the first one logs.
#[test]
fn a_pair_opens_a_missing_grid_once() {
    let context = builder();
    let source = context.clone().proj("EPSG:5070").expect("source");
    let target = context.clone().proj("EPSG:4326").expect("target");
    let (pair, created) = counted(|| {
        context.clone().proj_create_crs_to_crs_from_pj(
            &source,
            &target,
            None,
            Some(vec!["ONLY_BEST=NO"]),
        )
    });
    let pair = pair.expect("pair");
    assert_eq!(created, 0, "the pair opens no grid");
    let points = [
        (958_000.0, 4_470_000.0),
        (1_500_000.0, 4_470_000.0),
        (2_978_000.0, 4_470_000.0),
        (1_000_000.0, 1_000_000.0),
        (1_100_000.0, 1_000_000.0),
    ];
    let measured: Vec<(bool, usize)> = points
        .iter()
        .map(|&(x, y)| {
            let (result, lines) = counted(|| pair.convert((x, y)));
            (result.is_err(), lines)
        })
        .collect();
    assert_eq!(
        measured,
        [(true, 1), (true, 0), (true, 0), (true, 1), (true, 1)]
    );
}

fn counted<T>(body: impl FnOnce() -> T) -> (T, usize) {
    CANNOT_OPEN.with(|count| count.set(0));
    let previous = replace_log_sink(Some(LogSink::new(0, record)));
    let result = body();
    replace_log_sink(previous);
    (result, CANNOT_OPEN.with(Cell::get))
}
