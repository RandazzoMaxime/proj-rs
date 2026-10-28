//! Compile Karney's geodesic library, the same C PROJ 9.9 calls.
//!
//! The `cc` crate picks the C compiler that matches the Rust target: `cl` on
//! Windows MSVC, `gcc` or `clang` on Linux. The bench links libproj only when
//! an import library is already on this machine.

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

    let proj_lib = PathBuf::from(r"C:\Users\maxim\AppData\Local\Programs\OSGeo4W\lib");
    if proj_lib.join("proj.lib").is_file() {
        println!("cargo:rustc-link-search=native={}", proj_lib.display());
        println!("cargo:rustc-link-lib=dylib=proj");
        println!("cargo:rustc-cfg=link_libproj");
    }
}
