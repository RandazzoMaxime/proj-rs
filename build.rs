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
        // No fused multiply-add: clang fuses `a * b + c` by default on aarch64, and the
        // reference PROJ (x86-64) does not, so the geodesics would differ in the last bits.
        .flag_if_supported("-ffp-contract=off")
        .warnings(false)
        .compile("geodesic");

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
