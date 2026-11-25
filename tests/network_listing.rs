//! The listing of the operations of `EPSG:4326` to `EPSG:5070` with the network on and none of
//! its grids on disk or on the network, as terra's `proj_pipelines` asks for it, and what a point
//! does when the grid of its operation does not open.
//!
//! The expected values were measured with the libproj of PROJ 9.9.0 built by
//! `tools/build-reference.sh` (no libcurl: every network open answers
//! `Network functionality not available`, `no_op_network_open`), `PROJ_DATA` its `build/data`
//! (no grid), through `capi/tests/network_listing.c`, which `capi/tests/run.sh network_listing`
//! compares with the C library of proj-rs. Here the same refusal comes from the application's
//! callbacks, so the test runs with and without the `network` feature, and nothing leaves the
//! process.
//!
//! PROJ sorts the candidates with `USED_FOR_SORTING` by opening each grid
//! (`lookForGridInfo` → `resolveFullFilename`, `factory.cpp:3441-3450, 3520-3522`): a grid that
//! does not open is unavailable, so the Helmert "NAD83 to WGS 84 (1)" (4 m) stays ahead of the
//! grid operations (1.5 m), and each failed open logs `Cannot open <url>: <error>`
//! (`networkfilemanager.cpp:1323`). `proj_coordoperation_is_instantiable` passes the network state
//! as `considerKnownGridsAsAvailable` (`c_api.cpp:7918-7929`) and opens nothing.

use std::cell::RefCell;
use std::sync::Arc;

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext, CrsExtentUse,
    DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};
use proj_rs::unstable::{replace_log_sink, LogSink};
use proj_rs::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply, ProjBuilder, ProjError};

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

#[derive(Clone, Copy, PartialEq)]
enum Network {
    On,
    Off,
}

thread_local! {
    static LOGGED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn record(_owner: usize, line: &str) {
    LOGGED.with(|logged| logged.borrow_mut().push(line.to_owned()));
}

/// What `body` sends to the message channel, as the logger of a context receives it.
fn logged<T>(body: impl FnOnce() -> T) -> (T, Vec<String>) {
    LOGGED.with(|logged| logged.borrow_mut().clear());
    let previous = replace_log_sink(Some(LogSink::new(0, record)));
    let result = body();
    replace_log_sink(previous);
    (result, LOGGED.with(|logged| logged.borrow().clone()))
}

fn builder(network: Network) -> ProjBuilder {
    let empty =
        std::env::temp_dir().join(format!("proj-rs-network-listing-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("empty search path");
    let mut builder = ProjBuilder::new();
    builder.replace_search_paths(vec![empty]);
    builder.set_network_callbacks(Arc::new(NoNetwork));
    builder
        .enable_network(network == Network::On)
        .expect("network switch");
    builder
}

fn listing(network: Network, extent: CrsExtentUse) -> (Vec<CoordinateOperation>, Vec<String>) {
    let builder = builder(network);
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input("EPSG:4326", &db).expect("source");
    let target = create_from_user_input("EPSG:5070", &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_crs_extent_use(extent);
    let (operations, lines) = logged(|| builder.create_operations(&source, &target, &context));
    (operations.expect("operations"), lines)
}

/// `Inverse of NAD83 to WGS 84 (<n>) + Conus Albers` by its `<n>`, the other names whole.
fn short_names(operations: &[CoordinateOperation]) -> Vec<String> {
    operations
        .iter()
        .map(|operation| {
            let name = operation.name();
            name.strip_prefix("Inverse of NAD83 to WGS 84 (")
                .and_then(|rest| rest.strip_suffix(") + Conus Albers"))
                .unwrap_or(name)
                .to_owned()
        })
        .collect()
}

const SMALLEST: [&str; 48] = [
    "1", "38", "39", "18", "19", "41", "26", "6", "43", "48", "14", "30", "32", "31", "54", "15",
    "34", "42", "16", "20", "44", "55", "28", "50", "40", "52", "17", "36", "23", "47", "22", "33",
    "13", "27", "21", "12", "51", "25", "49", "56", "57", "29", "37", "46", "24", "53", "8", "7",
];

/// terra's default (`use = "NONE"`): the operations of Hawaii and Alaska are kept.
const NONE: [&str; 53] = [
    "1",
    "3",
    "2",
    "6",
    "8",
    "7",
    "38",
    "39",
    "18",
    "19",
    "41",
    "26",
    "43",
    "48",
    "14",
    "30",
    "32",
    "31",
    "54",
    "15",
    "34",
    "42",
    "16",
    "20",
    "44",
    "55",
    "28",
    "50",
    "40",
    "52",
    "17",
    "36",
    "23",
    "45",
    "47",
    "22",
    "33",
    "13",
    "27",
    "21",
    "12",
    "51",
    "25",
    "49",
    "56",
    "57",
    "29",
    "37",
    "46",
    "24",
    "53",
    "35",
    "Ballpark geographic offset from WGS 84 to NAD83 + Conus Albers",
];

/// The grids the sort tries, in the order PROJ logs their failed opens.
const TRIED: [&str; 47] = [
    "ca_nrc_NA83SCRS.tif",
    "ca_nrc_ABCSRSV4.tif",
    "ca_nrc_SK83-98.tif",
    "us_noaa_ethpgn.tif",
    "us_noaa_wthpgn.tif",
    "us_noaa_FL.tif",
    "us_noaa_emhpgn.tif",
    "us_noaa_WO.tif",
    "us_noaa_mihpgn.tif",
    "us_noaa_cnhpgn.tif",
    "us_noaa_nvhpgn.tif",
    "us_noaa_mnhpgn.tif",
    "us_noaa_nmhpgn.tif",
    "us_noaa_azhpgn.tif",
    "us_noaa_nyhpgn.tif",
    "us_noaa_cshpgn.tif",
    "us_noaa_mohpgn.tif",
    "us_noaa_okhpgn.tif",
    "us_noaa_WI.tif",
    "us_noaa_cohpgn.tif",
    "us_noaa_wmhpgn.tif",
    "us_noaa_wyhpgn.tif",
    "us_noaa_nchpgn.tif",
    "us_noaa_nbhpgn.tif",
    "us_noaa_uthpgn.tif",
    "us_noaa_vahpgn.tif",
    "us_noaa_ilhpgn.tif",
    "us_noaa_gahpgn.tif",
    "us_noaa_sdhpgn.tif",
    "us_noaa_lahpgn.tif",
    "us_noaa_kshpgn.tif",
    "us_noaa_kyhpgn.tif",
    "us_noaa_ndhpgn.tif",
    "us_noaa_iahpgn.tif",
    "us_noaa_mshpgn.tif",
    "us_noaa_alhpgn.tif",
    "us_noaa_arhpgn.tif",
    "us_noaa_wvhpgn.tif",
    "us_noaa_mehpgn.tif",
    "us_noaa_ohhpgn.tif",
    "us_noaa_schpgn.tif",
    "us_noaa_pahpgn.tif",
    "us_noaa_nehpgn.tif",
    "us_noaa_TN.tif",
    "us_noaa_inhpgn.tif",
    "us_noaa_MD.tif",
    "us_noaa_njhpgn.tif",
];

fn cannot_open(grid: &str) -> String {
    format!("Cannot open https://cdn.proj.org/{grid}: Network functionality not available")
}

#[test]
fn the_helmert_stays_first_when_no_grid_opens() {
    for network in [Network::Off, Network::On] {
        let (operations, _) = listing(network, CrsExtentUse::Smallest);
        assert_eq!(short_names(&operations), SMALLEST);
        let (operations, _) = listing(network, CrsExtentUse::Ignore);
        assert_eq!(short_names(&operations), NONE);
    }
}

#[test]
fn each_grid_that_does_not_open_logs_cannot_open_once() {
    let (_, lines) = listing(Network::On, CrsExtentUse::Smallest);
    let expected: Vec<String> = TRIED.iter().map(|grid| cannot_open(grid)).collect();
    assert_eq!(lines, expected);
    let (_, lines) = listing(Network::On, CrsExtentUse::Ignore);
    assert_eq!(lines.len(), 49);
    let (_, lines) = listing(Network::Off, CrsExtentUse::Ignore);
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn a_known_grid_makes_an_operation_instantiable_with_the_network_on() {
    let db = DatabaseContext::create().expect("database");
    let (operations, _) = listing(Network::Off, CrsExtentUse::Ignore);
    // `proj_coordoperation_is_instantiable` network off: only the operations without a grid and
    // the ballpark (the first three Helmerts and the last one).
    let off: String = operations
        .iter()
        .map(|operation| {
            if operation.is_proj_instantiable(&db, GridAvailabilityUse::UseForSorting) {
                '1'
            } else {
                '0'
            }
        })
        .collect();
    assert_eq!(off, "11100000000000000000000000000000000000000000000000001");
    let (on, lines) = logged(|| {
        operations.iter().all(|operation| {
            operation.is_proj_instantiable(&db, GridAvailabilityUse::KnownAvailable)
        })
    });
    assert!(on, "every operation is instantiable with the network on");
    assert!(lines.is_empty(), "no open is attempted: {lines:?}");
}

#[test]
fn a_point_whose_grid_does_not_open_is_a_network_error() {
    let (pair, lines) =
        logged(|| builder(Network::On).proj_known_crs("EPSG:4267", "EPSG:4326", None));
    assert!(
        lines.is_empty(),
        "the creation of the pair opens nothing: {lines:?}"
    );
    let pair = pair.expect("pair");
    let (converted, lines) = logged(|| pair.convert((-100.0_f64, 40.0_f64)));
    assert!(
        matches!(converted, Err(ProjError::Network)),
        "{converted:?}"
    );
    assert_eq!(lines, [cannot_open("us_noaa_conus.tif")]);

    let pair = builder(Network::Off)
        .proj_known_crs("EPSG:4267", "EPSG:4326", None)
        .expect("pair");
    let (converted, lines) = logged(|| pair.convert((-100.0_f64, 40.0_f64)));
    let (lon, lat) = converted.expect("network off: an operation without a grid");
    assert!(lon.is_finite() && lat.is_finite());
    assert!(lines.is_empty(), "{lines:?}");
}
