//! Compile Karney's geodesic library, the same C PROJ 9.9 calls.
//!
//! The `cc` crate picks the C compiler that matches the Rust target: `cl` on
//! Windows MSVC, `gcc` or `clang` on Linux. The bench links libproj only when
//! it finds one: in `PROJ_RS_LIBPROJ_DIR`, else in the active conda
//! environment (`CONDA_PREFIX`). Without one, the `ratio` column stays empty.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let src_dir = manifest.join("src").join("geodesic");
    let c_file = src_dir.join("geodesic.c");
    println!("cargo:rerun-if-changed={}", c_file.display());
    println!(
        "cargo:rerun-if-changed={}",
        src_dir.join("geodesic.h").display()
    );

    println!("cargo:rustc-check-cfg=cfg(link_libproj)");

    cc::Build::new()
        .file(&c_file)
        .include(&src_dir)
        .std("c11")
        .opt_level(2)
        .warnings(false)
        .compile("geodesic");

    println!("cargo:rerun-if-env-changed=PROJ_RS_LIBPROJ_DIR");
    println!("cargo:rerun-if-env-changed=CONDA_PREFIX");
    if let Some(dir) = libproj_dir() {
        println!("cargo:rustc-link-search=native={}", dir.display());
        println!("cargo:rustc-link-lib=dylib=proj");
        println!("cargo:rustc-cfg=link_libproj");
        if std::env::var("CARGO_CFG_TARGET_FAMILY").is_ok_and(|family| family == "unix") {
            println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{}", dir.display());
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
