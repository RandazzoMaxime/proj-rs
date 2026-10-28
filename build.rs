//! Compile Karney's geodesic library, the same C PROJ 9.9 calls.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let src_dir = manifest.join("src").join("geodesic");
    let c_file = src_dir.join("geodesic.c");
    println!("cargo:rerun-if-changed={}", c_file.display());
    println!(
        "cargo:rerun-if-changed={}",
        src_dir.join("geodesic.h").display()
    );

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    let obj = out.join("geodesic.obj");
    let lib = out.join("geodesic.lib");
    let vcvars = find_vcvars();
    let batch = out.join("build-geodesic.bat");
    let script = format!(
        "@echo off\r\ncall \"{}\" >nul\r\nif errorlevel 1 exit /b 1\r\ncl /nologo /c /O2 /std:c11 /I \"{}\" /Fo\"{}\" \"{}\"\r\nif errorlevel 1 exit /b 1\r\nlib /nologo /OUT:\"{}\" \"{}\"\r\n",
        vcvars.display(),
        src_dir.display(),
        obj.display(),
        c_file.display(),
        lib.display(),
        obj.display()
    );
    std::fs::write(&batch, script).expect("write geodesic build script");
    let output = Command::new("cmd")
        .arg("/c")
        .arg(&batch)
        .output()
        .expect("spawn the MSVC compiler");
    assert!(
        output.status.success(),
        "geodesic.c failed to compile\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=geodesic");

    let proj_lib = PathBuf::from(r"C:\Users\maxim\AppData\Local\Programs\OSGeo4W\lib");
    if proj_lib.join("proj.lib").is_file() {
        println!("cargo:rustc-link-search=native={}", proj_lib.display());
    }
}

fn find_vcvars() -> PathBuf {
    let vswhere =
        PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if vswhere.is_file() {
        if let Ok(output) = Command::new(&vswhere)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-find",
                r"VC\Auxiliary\Build\vcvars64.bat",
            ])
            .output()
        {
            if let Some(line) = String::from_utf8_lossy(&output.stdout).lines().next() {
                let path = PathBuf::from(line.trim());
                if path.is_file() {
                    return path;
                }
            }
        }
    }
    PathBuf::from(
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Professional\VC\Auxiliary\Build\vcvars64.bat",
    )
}
