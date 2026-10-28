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

    // The bench compares against libproj when it can link it. Point
    // PROJ_LIB_DIR at the folder holding proj.lib or libproj.so; the OSGeo4W
    // folder is still tried on the maintainer's Windows machine.
    println!("cargo:rerun-if-env-changed=PROJ_LIB_DIR");
    let mut candidates: Vec<PathBuf> = std::env::var_os("PROJ_LIB_DIR")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    candidates.push(PathBuf::from(
        r"C:\Users\maxim\AppData\Local\Programs\OSGeo4W\lib",
    ));
    for proj_lib in candidates {
        if proj_lib.join("proj.lib").is_file() || proj_lib.join("libproj.so").is_file() {
            println!("cargo:rustc-link-search=native={}", proj_lib.display());
            println!("cargo:rustc-link-lib=dylib=proj");
            println!("cargo:rustc-cfg=link_libproj");
            break;
        }
    }
}
