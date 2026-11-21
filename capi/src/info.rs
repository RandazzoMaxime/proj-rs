//! `proj_info`, `proj_pj_info`, `proj_grid_info`, `proj_init_info` (`info.cpp`, `grids.cpp`).

use std::ffi::{c_char, c_int, CString};
use std::sync::Mutex;

use crate::context::context;
use crate::ffi::*;
use crate::iso::Obj;
use crate::pj::pj;
use crate::util::{self, guard};
use proj_rs::iso19111::CoordinateOperation;

/// The strings `proj_info` points to, kept until the next call (`info.cpp` keeps them in a
/// static, under a lock).
#[derive(Default)]
struct InfoStrings {
    release: Option<CString>,
    version: Option<CString>,
    searchpath: Option<CString>,
    paths: Vec<CString>,
    pointers: Vec<*const c_char>,
}

// SAFETY: the pointers in `pointers` point into the `CString`s of the same value, which are
// only read, and the whole value lives behind a `Mutex`.
unsafe impl Send for InfoStrings {}

static INFO: Mutex<Option<InfoStrings>> = Mutex::new(None);

fn empty_info() -> PJ_INFO {
    PJ_INFO {
        major: 0,
        minor: 0,
        patch: 0,
        release: std::ptr::null(),
        version: std::ptr::null(),
        searchpath: std::ptr::null(),
        paths: std::ptr::null(),
        path_count: 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_info() -> PJ_INFO {
    guard(empty_info(), || {
        let ctx = context(std::ptr::null());
        let Ok(found) = ctx.builder().lib_info() else {
            return empty_info();
        };
        let mut strings = InfoStrings {
            release: Some(util::c_string(crate::misc::RELEASE)),
            version: Some(util::c_string(&found.version)),
            searchpath: Some(util::c_string(&found.searchpath)),
            ..InfoStrings::default()
        };
        let separator = if cfg!(windows) { ';' } else { ':' };
        strings.paths = found
            .searchpath
            .split(separator)
            .filter(|part| !part.is_empty())
            .map(util::c_string)
            .collect();
        strings.pointers = strings.paths.iter().map(|path| path.as_ptr()).collect();
        let info = PJ_INFO {
            major: found.major,
            minor: found.minor,
            patch: found.patch,
            release: strings
                .release
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr()),
            version: strings
                .version
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr()),
            searchpath: strings
                .searchpath
                .as_ref()
                .map_or(std::ptr::null(), |s| s.as_ptr()),
            paths: strings.pointers.as_ptr(),
            path_count: strings.pointers.len(),
        };
        let mut slot = INFO
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *slot = Some(strings);
        info
    })
}

fn empty_pj_info() -> PJ_PROJ_INFO {
    PJ_PROJ_INFO {
        id: std::ptr::null(),
        description: std::ptr::null(),
        definition: std::ptr::null(),
        has_inverse: 0,
        accuracy: -1.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_pj_info(p: *mut PJ) -> PJ_PROJ_INFO {
    guard(empty_pj_info(), || {
        let Some(object) = pj(p) else {
            return empty_pj_info();
        };
        if let Some(info) = crate::iso::operation_info(object) {
            return PJ_PROJ_INFO {
                id: object.arena.put_opt(info.id.as_deref()),
                description: object.arena.put(&info.description),
                definition: object.arena.put(&info.definition),
                has_inverse: c_int::from(info.has_inverse),
                accuracy: info.accuracy,
            };
        }
        let Some(proj) = object.proj.as_ref() else {
            return empty_pj_info();
        };
        let mut found = proj.proj_info();
        if let Some(operation) = object.obj.as_ref().and_then(Obj::operation) {
            // `info.cpp:200-231`: the name of the ISO object, and the accuracy of an operation
            // (0 for a conversion). A PROJ-based operation is not a Conversion in PROJ.
            found.description = Some(operation.name().to_owned());
            found.accuracy = match operation {
                CoordinateOperation::Conversion(conversion)
                    if !conversion
                        .method()
                        .name()
                        .starts_with("PROJ-based operation method") =>
                {
                    0.0
                }
                other => other.accuracy().unwrap_or(-1.0),
            };
        }
        PJ_PROJ_INFO {
            id: object.arena.put_opt(found.id.as_deref()),
            description: object.arena.put_opt(found.description.as_deref()),
            definition: object.arena.put_opt(found.definition.as_deref()),
            has_inverse: c_int::from(found.has_inverse),
            accuracy: found.accuracy,
        }
    })
}

/// Copies `text` into a fixed C array, cut to fit, always NUL-terminated.
fn fill(target: &mut [c_char], text: &str) {
    let room = target.len().saturating_sub(1);
    for (slot, byte) in target.iter_mut().zip(text.bytes().take(room)) {
        *slot = c_char::from_ne_bytes([byte]);
    }
}

fn empty_grid_info() -> PJ_GRID_INFO {
    PJ_GRID_INFO {
        gridname: [0; 32],
        filename: [0; 260],
        format: [0; 8],
        lowerleft: PJ_LP { lam: 0.0, phi: 0.0 },
        upperright: PJ_LP { lam: 0.0, phi: 0.0 },
        n_lon: 0,
        n_lat: 0,
        cs_lon: 0.0,
        cs_lat: 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_grid_info(gridname: *const c_char) -> PJ_GRID_INFO {
    guard(empty_grid_info(), || {
        let mut out = empty_grid_info();
        let Some(name) = util::text(gridname) else {
            return out;
        };
        let ctx = context(std::ptr::null());
        let Some(found) = ctx.builder().grid_info(name) else {
            return out;
        };
        fill(&mut out.gridname, &found.gridname);
        if let Some(path) = &found.filename {
            fill(&mut out.filename, &path.to_string_lossy());
        }
        fill(&mut out.format, &found.format);
        out.lowerleft = PJ_LP {
            lam: found.lower_left.lon,
            phi: found.lower_left.lat,
        };
        out.upperright = PJ_LP {
            lam: found.upper_right.lon,
            phi: found.upper_right.lat,
        };
        out.n_lon = c_int::try_from(found.n_lon).unwrap_or(c_int::MAX);
        out.n_lat = c_int::try_from(found.n_lat).unwrap_or(c_int::MAX);
        out.cs_lon = found.cs_lon;
        out.cs_lat = found.cs_lat;
        out
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_init_info(initname: *const c_char) -> PJ_INIT_INFO {
    guard(empty_init_info(), || {
        let mut out = empty_init_info();
        let Some(name) = util::text(initname) else {
            return out;
        };
        let Some(found) = proj_rs::info::init_info(name) else {
            return out;
        };
        fill(&mut out.name, &found.name);
        if let Some(path) = &found.filename {
            fill(&mut out.filename, &path.to_string_lossy());
        }
        fill(&mut out.version, &found.version);
        fill(&mut out.origin, &found.origin);
        fill(&mut out.lastupdate, &found.lastupdate);
        out
    })
}

fn empty_init_info() -> PJ_INIT_INFO {
    PJ_INIT_INFO {
        name: [0; 32],
        filename: [0; 260],
        version: [0; 32],
        origin: [0; 32],
        lastupdate: [0; 16],
    }
}
