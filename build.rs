//! The bench links libproj only when it finds one: in `PROJ_RS_LIBPROJ_DIR`, else in the active conda
//! environment (`CONDA_PREFIX`). Without one, the `ratio` column stays empty.

use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(link_libproj)");

    println!("cargo:rerun-if-env-changed=PROJ_RS_LIBPROJ_DIR");
    println!("cargo:rerun-if-env-changed=CONDA_PREFIX");
    if let Some(dir) = libproj_dir() {
        // The bench alone links libproj: a link-lib here would reach every crate linking
        // proj-rs, and libproj in proj-rs-capi clashes with the libproj it builds.
        println!("cargo:rustc-cfg=link_libproj");
        let bench = "cargo:rustc-link-arg-bin=proj-rs-bench";
        if std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc") {
            println!("{bench}=/LIBPATH:{}", dir.display());
            println!("{bench}=proj.lib");
        } else {
            println!("{bench}=-L{}", dir.display());
            println!("{bench}=-lproj");
            println!("{bench}=-Wl,-rpath,{}", dir.display());
        }
    }
}

/// The directory that holds libproj, if this machine has one.
fn libproj_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::var_os("PROJ_RS_LIBPROJ_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    if let Some(prefix) = std::env::var_os("CONDA_PREFIX") {
        let prefix = PathBuf::from(prefix);
        candidates.push(prefix.join("Library").join("lib"));
        candidates.push(prefix.join("lib"));
    }
    let names = ["proj.lib", "libproj.so", "libproj.dylib"];
    candidates
        .into_iter()
        .find(|dir| names.iter().any(|name| dir.join(name).is_file()))
}
