//! Gives the shared library the soname of libproj 9.9.0 (`libproj.so.25`), so that a program
//! linked against the installed `libproj.so` finds it by that name.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target.as_str() {
        "linux" | "android" | "freebsd" => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libproj.so.25");
        }
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libproj.25.dylib");
        }
        _ => {}
    }
}
