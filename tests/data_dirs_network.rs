//! The grid cache of the `network` feature goes where PROJ puts it: `cache.db` in the user
//! directory, `proj_context_get_user_writable_directory` of PROJ, and not in the temporary
//! directory. The server is local.
#![cfg(feature = "network")]

use std::path::PathBuf;

use proj_rs::ProjBuilder;

#[path = "net_support/mod.rs"]
#[expect(dead_code, reason = "this file serves with spawn only")]
mod net_support;

use net_support::{LocalServer, Route};

const GRID: &str = "/proj_rs_dirs_grid.tif";

#[test]
fn chunk_database_and_downloads_are_kept_in_the_user_directory() {
    let home = std::env::temp_dir().join(format!("proj-rs-dirs-net-{}", std::process::id()));
    let _gone = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("home");
    let home = std::fs::canonicalize(&home).expect("canonical");
    for variable in [
        "PROJ_DATA",
        "PROJ_LIB",
        "PROJ_USER_WRITABLE_DIRECTORY",
        "PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY",
        "XDG_DATA_HOME",
    ] {
        std::env::remove_var(variable);
    }
    std::env::set_var("HOME", &home);
    std::env::set_var("LOCALAPPDATA", &home);
    let user: PathBuf = if cfg!(windows) {
        PathBuf::from(format!("{}/proj", home.display()))
    } else if cfg!(target_vendor = "apple") {
        home.join("Library/Application Support/proj")
    } else {
        home.join(".local/share/proj")
    };

    let fixture =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/tests/test_hgrid.tif"))
            .expect("fixture");
    let server = LocalServer::spawn(vec![Route {
        path: GRID,
        status: 200,
        body: fixture,
    }]);

    let mut builder = ProjBuilder::new();
    assert_eq!(builder.enable_network(true).expect("enable"), 1);
    builder.set_url_endpoint(server.url()).expect("endpoint");
    builder.grid_cache_enable(true).expect("cache on");
    assert_eq!(builder.user_writable_directory(), user);
    assert_eq!(builder.grid_cache_path(), user.join("cache.db"));

    builder
        .proj("+proj=hgridshift +grids=proj_rs_dirs_grid.tif")
        .expect("downloaded grid");
    assert!(
        user.join("cache.db").is_file(),
        "the chunk database is in the user directory"
    );
    let _gone = std::fs::remove_dir_all(&home);
}
