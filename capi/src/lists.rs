//! The static lists of `proj.h`: projections, ellipsoids, units, angular units and prime
//! meridians (`proj_list_*`), with the tables of PROJ 9.9.0.

use std::ffi::{c_char, CString};
use std::sync::OnceLock;

use crate::ffi::*;
use crate::lists_data::{ANGULAR_UNITS, ELLIPSOIDS, LINEAR_UNITS, OPERATIONS, PRIME_MERIDIANS};
use crate::util::guard;

/// The arrays handed to C, and the strings they point to. They live for the whole process.
struct Tables {
    operations: Vec<PJ_LIST>,
    ellipsoids: Vec<PJ_ELLPS>,
    units: Vec<PJ_UNITS>,
    angular_units: Vec<PJ_UNITS>,
    prime_meridians: Vec<PJ_PRIME_MERIDIANS>,
    // Kept so that the pointers of the tables stay valid.
    _strings: Vec<CString>,
    _descriptions: Vec<Box<*const c_char>>,
}

// SAFETY: a `Tables` is built once, never changed, and only read through the pointers it holds,
// which point into data it owns.
unsafe impl Send for Tables {}
// SAFETY: as above: shared reads of immutable data.
unsafe impl Sync for Tables {}

/// A constructor slot of `PJ_LIST::proj`: this port has no per-projection constructor to give.
extern "C" fn no_constructor(_: *mut PJ) -> *mut PJ {
    std::ptr::null_mut()
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(build)
}

fn build() -> Tables {
    let mut strings: Vec<CString> = Vec::new();
    let mut keep = |text: &str| -> *const c_char {
        let owned = CString::new(text.replace('\0', "")).unwrap_or_default();
        let pointer = owned.as_ptr();
        strings.push(owned);
        pointer
    };
    let mut descriptions: Vec<Box<*const c_char>> = Vec::new();
    let mut operations = Vec::with_capacity(OPERATIONS.len() + 1);
    for (id, description) in OPERATIONS {
        let slot = Box::new(keep(description));
        let slot_pointer: *const *const c_char = &raw const *slot;
        descriptions.push(slot);
        operations.push(PJ_LIST {
            id: keep(id),
            proj: Some(no_constructor),
            descr: slot_pointer,
        });
    }
    operations.push(PJ_LIST {
        id: std::ptr::null(),
        proj: None,
        descr: std::ptr::null(),
    });
    let mut ellipsoids: Vec<PJ_ELLPS> = ELLIPSOIDS
        .iter()
        .map(|(id, major, ell, name)| PJ_ELLPS {
            id: keep(id),
            major: keep(major),
            ell: keep(ell),
            name: keep(name),
        })
        .collect();
    ellipsoids.push(PJ_ELLPS {
        id: std::ptr::null(),
        major: std::ptr::null(),
        ell: std::ptr::null(),
        name: std::ptr::null(),
    });
    let mut units_of = |table: &[(&str, &str, &str, f64)]| -> Vec<PJ_UNITS> {
        let mut list: Vec<PJ_UNITS> = table
            .iter()
            .map(|(id, to_meter, name, factor)| PJ_UNITS {
                id: keep(id),
                to_meter: keep(to_meter),
                name: keep(name),
                factor: *factor,
            })
            .collect();
        list.push(PJ_UNITS {
            id: std::ptr::null(),
            to_meter: std::ptr::null(),
            name: std::ptr::null(),
            factor: 0.0,
        });
        list
    };
    let units = units_of(LINEAR_UNITS);
    let angular_units = units_of(ANGULAR_UNITS);
    let mut prime_meridians: Vec<PJ_PRIME_MERIDIANS> = PRIME_MERIDIANS
        .iter()
        .map(|(id, defn)| PJ_PRIME_MERIDIANS {
            id: keep(id),
            defn: keep(defn),
        })
        .collect();
    prime_meridians.push(PJ_PRIME_MERIDIANS {
        id: std::ptr::null(),
        defn: std::ptr::null(),
    });
    Tables {
        operations,
        ellipsoids,
        units,
        angular_units,
        prime_meridians,
        _strings: strings,
        _descriptions: descriptions,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_operations() -> *const PJ_OPERATIONS {
    guard(std::ptr::null(), || tables().operations.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_ellps() -> *const PJ_ELLPS {
    guard(std::ptr::null(), || tables().ellipsoids.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_units() -> *const PJ_UNITS {
    guard(std::ptr::null(), || tables().units.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_angular_units() -> *const PJ_UNITS {
    guard(std::ptr::null(), || tables().angular_units.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_prime_meridians() -> *const PJ_PRIME_MERIDIANS {
    guard(std::ptr::null(), || tables().prime_meridians.as_ptr())
}
