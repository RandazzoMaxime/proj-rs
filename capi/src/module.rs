//! The file this library is loaded from, which PROJ takes `../share/proj` from
//! (`pj_get_relative_share_proj`, `src/filemanager.cpp`): `dladdr` on Unix,
//! `GetModuleHandleExW` and `GetModuleFileNameW` on Windows.

use std::sync::Once;

/// Tell the engine where this library is, once, before its first file lookup. A library
/// linked into the executable statically gives no path of its own: the engine then uses the
/// executable.
pub(crate) fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if let Some(path) = path() {
            proj_rs::info::set_module_path(&path);
        }
    });
}

/// An address inside this library.
#[cfg(any(unix, windows))]
fn anchor() -> *const std::ffi::c_void {
    static ANCHOR: u8 = 0;
    std::ptr::from_ref(&ANCHOR).cast()
}

#[cfg(unix)]
fn path() -> Option<String> {
    use std::ffi::{c_char, c_int, c_void, CStr};

    #[repr(C)]
    struct DlInfo {
        dli_fname: *const c_char,
        dli_fbase: *mut c_void,
        dli_sname: *const c_char,
        dli_saddr: *mut c_void,
    }
    unsafe extern "C" {
        fn dladdr(address: *const c_void, info: *mut DlInfo) -> c_int;
    }

    let mut info = DlInfo {
        dli_fname: std::ptr::null(),
        dli_fbase: std::ptr::null_mut(),
        dli_sname: std::ptr::null(),
        dli_saddr: std::ptr::null_mut(),
    };
    // SAFETY: `info` is a valid `Dl_info` for the call to fill, and the address is that of a
    // static of this library.
    let found = unsafe { dladdr(anchor(), &raw mut info) };
    if found == 0 || info.dli_fname.is_null() {
        return None;
    }
    // SAFETY: on success `dli_fname` is a NUL-terminated string that lives as long as the
    // library is loaded.
    let name = unsafe { CStr::from_ptr(info.dli_fname) }.to_str().ok()?;
    if !name.contains('/') {
        return None;
    }
    if name.starts_with('/') {
        Some(name.to_owned())
    } else {
        std::fs::canonicalize(name)
            .ok()
            .and_then(|absolute| absolute.to_str().map(str::to_owned))
    }
}

#[cfg(windows)]
fn path() -> Option<String> {
    use std::ffi::c_void;

    type Module = *mut c_void;
    const FROM_ADDRESS: u32 = 0x4;
    const UNCHANGED_REFCOUNT: u32 = 0x2;
    const INSUFFICIENT_BUFFER: u32 = 122;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleExW(flags: u32, address: *const c_void, module: *mut Module) -> i32;
        fn GetModuleFileNameW(module: Module, buffer: *mut u16, size: u32) -> u32;
        fn GetLastError() -> u32;
    }

    let mut module: Module = std::ptr::null_mut();
    // SAFETY: `module` is a valid out pointer, and the address is that of a static of this
    // library.
    let found =
        unsafe { GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT, anchor(), &raw mut module) };
    if found == 0 {
        return None;
    }
    let mut size: usize = 1024;
    loop {
        let mut buffer = vec![0_u16; size];
        let capacity = u32::try_from(size - 1).ok()?;
        // SAFETY: `buffer` holds `size` code units and the call writes at most `size - 1`.
        let written = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), capacity) };
        if written == 0 {
            return None;
        }
        let written = usize::try_from(written).ok()?;
        if written == size - 1 {
            // SAFETY: `GetLastError` reads the calling thread's error code.
            if unsafe { GetLastError() } != INSUFFICIENT_BUFFER {
                return None;
            }
            size *= 2;
            continue;
        }
        buffer.truncate(written);
        return Some(String::from_utf16_lossy(&buffer));
    }
}

#[cfg(not(any(unix, windows)))]
fn path() -> Option<String> {
    None
}
