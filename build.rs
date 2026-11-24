//! The bench compares with libproj only when it finds one: in `PROJ_RS_LIBPROJ_DIR`, else in the
//! active conda environment (`CONDA_PREFIX`). It then compiles `src/bin/support/libproj_timer.c`
//! against it, a separate program that times libproj for the bench: no Rust code calls libproj,
//! so the bench has no `unsafe`. Without a libproj, or if the C compiler fails (a warning), the
//! `ratio` column stays empty and nothing else changes.

use std::path::{Path, PathBuf};

const TIMER_SOURCE: &str = "src/bin/support/libproj_timer.c";

fn main() {
    println!("cargo:rustc-check-cfg=cfg(link_libproj)");
    // The targets with a batch path (`src/simd/`): AVX2 (4 lanes) or AVX-512 (8 lanes, feature
    // `avx512`) on x86-64, NEON (4 lanes) on aarch64.
    println!("cargo:rustc-check-cfg=cfg(simd_lanes)");
    if matches!(
        std::env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
        Ok("x86_64" | "aarch64")
    ) {
        println!("cargo:rustc-cfg=simd_lanes");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PROJ_RS_LIBPROJ_DIR");
    println!("cargo:rerun-if-env-changed=CONDA_PREFIX");
    println!("cargo:rerun-if-changed={TIMER_SOURCE}");
    if let Some(dir) = libproj_dir() {
        match compile_timer(&dir) {
            Ok(timer) => {
                println!("cargo:rustc-cfg=link_libproj");
                println!("cargo:rustc-env=PROJ_RS_LIBPROJ_TIMER={}", timer.display());
            }
            Err(error) => println!(
                "cargo:warning=libproj found in {}, but its timer did not compile ({error}): \
                 the bench has no libproj column",
                dir.display()
            ),
        }
    }
}

/// Compiles the libproj timer of the bench into `OUT_DIR`, linked to the libproj of `dir`, with
/// the C compiler of the target (the `cc` crate finds it, MSVC included).
fn compile_timer(dir: &Path) -> Result<PathBuf, String> {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("no OUT_DIR")?);
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let exe = out.join(if windows {
        "libproj-timer.exe"
    } else {
        "libproj-timer"
    });
    let compiler = cc::Build::new()
        .opt_level(2)
        .cargo_metadata(false)
        .try_get_compiler()
        .map_err(|error| error.to_string())?;
    let mut command = compiler.to_command();
    command.arg(TIMER_SOURCE);
    if msvc {
        command
            .arg(format!("/Fe{}", exe.display()))
            .arg(format!("/Fo{}\\", out.display()))
            .arg("/link")
            .arg(format!("/LIBPATH:{}", dir.display()))
            .arg("proj.lib");
    } else {
        command
            .arg("-o")
            .arg(&exe)
            .arg(format!("-L{}", dir.display()))
            .arg("-lproj")
            .arg("-lm")
            .arg(format!("-Wl,-rpath,{}", dir.display()));
    }
    let status = command.status().map_err(|error| error.to_string())?;
    if status.success() {
        Ok(exe)
    } else {
        Err(format!("{command:?} exited with {status}"))
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
