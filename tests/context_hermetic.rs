//! A hermetic context reads nothing from the machine: no `PROJ_NETWORK`, `PROJ_NETWORK_ENDPOINT`,
//! `PROJ_ONLY_BEST_DEFAULT`, no `proj.ini` (in `PROJ_DATA`, the user-writable directory, the
//! search path or the directory of the module), no `HOME`, no working directory. Its sources are
//! the search path and the settings given to its builder.
//!
//! The expected values are PROJ 9.9.0's. For the `proj.ini` and the environment, they follow
//! `pj_load_ini` (`src/filemanager.cpp`, lines 1988-2105 of the 9.9.0 sources): the environment
//! wins over the file, `cdn_endpoint` is read only when `PROJ_NETWORK_ENDPOINT` is not set, and
//! `only_best_default` only when `PROJ_ONLY_BEST_DEFAULT` is not set. For the ballpark
//! transformation, they are the output of `cs2cs EPSG:4267 EPSG:4326` (PROJ 9.9.0) with
//! `PROJ_DATA` set to a directory without `us_noaa_conus.tif`: unset, it prints
//! `39.999996882 -100.000415589`; with `PROJ_ONLY_BEST_DEFAULT=YES`, it fails with
//! "Grid us_noaa_conus.tif is not available".
//!
//! The variables are the process's, so every case takes the lock and clears them first.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use proj_rs::{Cache, Context, Crs, Error, Network, OnlyBest, Point, Transformer};

static ENVIRONMENT: Mutex<()> = Mutex::new(());

/// The variables a context may read the machine through, and the user-writable directory, which
/// is pointed at an empty directory so that no `proj.ini` of this machine is found.
const VARIABLES: [&str; 6] = [
    "PROJ_NETWORK",
    "PROJ_NETWORK_ENDPOINT",
    "PROJ_ONLY_BEST_DEFAULT",
    "PROJ_DATA",
    "PROJ_LIB",
    "PROJ_AUX_DB",
];

/// A `proj.ini` that turns everything on: the network, another endpoint, `ONLY_BEST`.
const TRAP_INI: &str =
    "network = on\ncdn_endpoint = https://trap.example.org/grids\nonly_best_default = on\n";

const TRAP_ENDPOINT: &str = "https://trap.example.org/grids";
const PROJ_ENDPOINT: &str = "https://cdn.proj.org";

/// `cs2cs EPSG:4267 EPSG:4326` of PROJ 9.9.0 for the input `40 -100` (lat, lon), no grid.
const BALLPARK_LAT: f64 = 39.999_996_882;
const BALLPARK_LON: f64 = -100.000_415_589;

/// Holds the lock, clears the variables and points the user-writable directory at `scratch`.
fn clean_environment(scratch: &Path) -> MutexGuard<'static, ()> {
    let guard = ENVIRONMENT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for variable in VARIABLES {
        std::env::remove_var(variable);
    }
    std::env::set_var(
        "PROJ_USER_WRITABLE_DIRECTORY",
        scratch.join("user-writable"),
    );
    guard
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("proj-rs-hermetic-{tag}-{}", std::process::id()));
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch");
    dir
}

/// A directory holding the trap `proj.ini`.
fn trap_directory(root: &Path) -> PathBuf {
    let dir = root.join("trap");
    fs::create_dir_all(&dir).expect("trap directory");
    fs::write(dir.join("proj.ini"), TRAP_INI).expect("trap proj.ini");
    dir
}

/// A directory with no grid and no `proj.ini`.
fn empty_directory(root: &Path) -> PathBuf {
    let dir = root.join("empty");
    fs::create_dir_all(&dir).expect("empty directory");
    dir
}

fn set(variable: &str, value: impl AsRef<std::ffi::OsStr>) {
    std::env::set_var(variable, value);
}

/// The forward transformation NAD27 to WGS 84 of the point `40 -100` (lat, lon), read as
/// `x = lon, y = lat`.
fn nad27_to_wgs84(context: &Context, only_best: OnlyBest) -> Result<Result<Point, Error>, Error> {
    let source = Crs::from_user_input(context, "EPSG:4267")?;
    let target = Crs::from_user_input(context, "EPSG:4326")?;
    let transformer = Transformer::builder(&source, &target)
        .normalize_axis_order()
        .only_best(only_best)
        .build()?;
    Ok(transformer.forward(Point::new(-100.0, 40.0)))
}

fn assert_ballpark(point: &Point) {
    assert!(
        (point.y - BALLPARK_LAT).abs() < 1e-7,
        "latitude {} is not PROJ's {BALLPARK_LAT}",
        point.y
    );
    assert!(
        (point.x - BALLPARK_LON).abs() < 1e-7,
        "longitude {} is not PROJ's {BALLPARK_LON}",
        point.x
    );
}

/// (1) Network and endpoint: a `proj.ini` on the search path, in `PROJ_DATA`, with
/// `PROJ_NETWORK=ON`, is not read by a hermetic context, and it is read by the others.
#[test]
fn hermetic_ignores_the_network_settings_of_the_environment_and_proj_ini() {
    let root = scratch("network");
    let _lock = clean_environment(&root);
    let trap = trap_directory(&root);
    set("PROJ_NETWORK", "ON");
    set("PROJ_DATA", &trap);

    let hermetic = Context::builder().hermetic().search_path(&trap).build();
    assert_eq!(hermetic.network(), Network::Disabled);
    assert_eq!(hermetic.endpoint(), PROJ_ENDPOINT);
    assert_eq!(hermetic.cache(), Cache::Disabled);
    assert_eq!(hermetic.search_paths(), std::slice::from_ref(&trap));

    // The default of PROJ without a `proj.ini`: the same endpoint, and `ONLY_BEST` unset.
    let bare = Context::builder().hermetic().build();
    assert_eq!(bare.endpoint(), PROJ_ENDPOINT);

    // The trap is real: a context that is not hermetic reads the endpoint of the file
    // (`PROJ_NETWORK_ENDPOINT` is not set, filemanager.cpp:2064), from the search path, and the
    // context of the environment takes the network too (`PROJ_NETWORK=ON` wins over the file).
    let searched = Context::builder().search_path(&trap).build();
    assert_eq!(searched.endpoint(), TRAP_ENDPOINT);
    let from_environment = Context::from_environment();
    assert_eq!(from_environment.network(), Network::Enabled);
    assert_eq!(from_environment.endpoint(), TRAP_ENDPOINT);
    let _gone = fs::remove_dir_all(&root);
}

/// (2) `PROJ_ONLY_BEST_DEFAULT=YES` is ignored by a hermetic context: the ballpark transformation
/// is built, as PROJ does with the variable unset. The context of the environment is refused,
/// as PROJ refuses with the variable set.
#[test]
fn hermetic_ignores_proj_only_best_default() {
    let root = scratch("only-best");
    let _lock = clean_environment(&root);
    let empty = empty_directory(&root);
    set("PROJ_ONLY_BEST_DEFAULT", "YES");
    set("PROJ_DATA", &empty);

    let hermetic = Context::builder().hermetic().search_path(&empty).build();
    let point = nad27_to_wgs84(&hermetic, OnlyBest::Default)
        .expect("the context builds the crs")
        .expect("the ballpark transformation is built and transforms");
    assert_ballpark(&point);

    // PROJ with the variable: the best operation needs `us_noaa_conus.tif`, which is missing.
    let from_environment = Context::from_environment();
    match nad27_to_wgs84(&from_environment, OnlyBest::Default).expect("the crs") {
        Err(Error::MissingGrid(_)) => {}
        other => panic!("expected a missing grid, got {:?}", other.map(|_| ())),
    }
    let _gone = fs::remove_dir_all(&root);
}

/// (3) `search_path` and `hermetic()` commute: the same settings give the same context in either
/// order, and the search path replaces `PROJ_DATA`, which is not read.
#[test]
fn search_path_and_hermetic_give_the_same_context_in_either_order() {
    let root = scratch("order");
    let _lock = clean_environment(&root);
    let trap = trap_directory(&root);
    let empty = empty_directory(&root);
    set("PROJ_NETWORK", "ON");
    set("PROJ_ONLY_BEST_DEFAULT", "YES");
    set("PROJ_DATA", &trap);

    let first = Context::builder().search_path(&empty).hermetic().build();
    let second = Context::builder().hermetic().search_path(&empty).build();
    for context in [&first, &second] {
        assert_eq!(context.network(), Network::Disabled);
        assert_eq!(context.endpoint(), PROJ_ENDPOINT);
        assert_eq!(context.search_paths(), std::slice::from_ref(&empty));
        assert_eq!(context.cache(), Cache::Disabled);
    }
    let point_first = nad27_to_wgs84(&first, OnlyBest::Default)
        .expect("crs")
        .expect("ballpark");
    let point_second = nad27_to_wgs84(&second, OnlyBest::Default)
        .expect("crs")
        .expect("ballpark");
    assert_eq!(point_first, point_second);
    assert_ballpark(&point_first);

    // A non-hermetic context with the same search path: PROJ replaces `PROJ_DATA` by the search
    // path, so the trap in `PROJ_DATA` is not read either.
    let searched = Context::builder().search_path(&empty).build();
    assert_eq!(searched.endpoint(), PROJ_ENDPOINT);
    let _gone = fs::remove_dir_all(&root);
}

/// The embedded `proj.db` is the crate's own data: a hermetic context opens it with `PROJ_DATA`
/// pointing nowhere, and with no search path.
#[test]
fn hermetic_context_uses_the_embedded_database() {
    let root = scratch("database");
    let _lock = clean_environment(&root);
    set("PROJ_DATA", root.join("missing"));
    let hermetic = Context::builder().hermetic().build();
    let crs = Crs::from_user_input(&hermetic, "EPSG:4326").expect("embedded proj.db");
    assert_eq!(crs.name(), "WGS 84");
    let _gone = fs::remove_dir_all(&root);
}

/// The explicit settings of the builder are honored by a hermetic context.
#[test]
fn hermetic_context_keeps_its_explicit_settings() {
    let root = scratch("explicit");
    let _lock = clean_environment(&root);
    let user = root.join("user");
    set("PROJ_NETWORK", "ON");

    let context = Context::builder()
        .hermetic()
        .endpoint("https://example.org/grids")
        .user_directory(&user)
        .cache(Cache::Enabled)
        .build();
    assert_eq!(context.endpoint(), "https://example.org/grids");
    assert_eq!(context.user_directory(), user);
    assert_eq!(context.cache(), Cache::Enabled);
    assert_eq!(context.network(), Network::Disabled);

    // Without a directory a hermetic context has nowhere to keep a cache: it stays off, and
    // its directory is empty rather than the user-writable directory of the machine.
    let no_directory = Context::builder().hermetic().cache(Cache::Enabled).build();
    assert_eq!(no_directory.cache(), Cache::Disabled);
    assert!(no_directory.user_directory().as_os_str().is_empty());

    let network = Context::builder()
        .hermetic()
        .network(Network::Enabled)
        .build();
    assert_eq!(network.network(), Network::Enabled);
    assert_eq!(network.endpoint(), PROJ_ENDPOINT);
    let _gone = fs::remove_dir_all(&root);
}
