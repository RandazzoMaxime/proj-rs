//! The unsafe edge: C strings, raw pointers, and the panic guard.
//!
//! The C contract of `proj.h` is that every pointer argument is a valid pointer of its type or
//! `NULL` where the header allows it, and that a string is NUL-terminated. The helpers here
//! turn such a pointer into a reference once, so that the functions above them are safe code.

use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs `body`; a panic becomes `fallback`, so that it never crosses the C boundary.
pub(crate) fn guard<T>(fallback: T, body: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(fallback)
}

/// The text of a C string. `None` for `NULL` and for bytes that are not UTF-8.
pub(crate) fn text<'a>(pointer: *const c_char) -> Option<&'a str> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: the pointer is not null, and the C contract makes it a NUL-terminated string
    // that outlives the call.
    unsafe { CStr::from_ptr(pointer) }.to_str().ok()
}

/// The bytes of a C string, `None` for `NULL`.
pub(crate) fn bytes<'a>(pointer: *const c_char) -> Option<&'a [u8]> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: as in `text`.
    Some(unsafe { CStr::from_ptr(pointer) }.to_bytes())
}

/// A C string with its interior NULs removed, which C could not read past anyway.
pub(crate) fn c_string(value: &str) -> CString {
    CString::new(value.replace('\0', "")).unwrap_or_default()
}

/// The `count` elements at `pointer`, empty for `NULL` or a zero count.
pub(crate) fn slice<'a, T>(pointer: *const T, count: usize) -> &'a [T] {
    if pointer.is_null() || count == 0 {
        return &[];
    }
    // SAFETY: the C contract gives `count` valid, initialised elements at `pointer`.
    unsafe { std::slice::from_raw_parts(pointer, count) }
}

/// The `count` elements at `pointer`, writable. Empty for `NULL` or a zero count.
pub(crate) fn slice_mut<'a, T>(pointer: *mut T, count: usize) -> &'a mut [T] {
    if pointer.is_null() || count == 0 {
        return &mut [];
    }
    // SAFETY: the C contract gives `count` valid elements at `pointer`, owned by the caller
    // for the call.
    unsafe { std::slice::from_raw_parts_mut(pointer, count) }
}

/// Writes `value` through an out-parameter. Does nothing for `NULL`.
pub(crate) fn put<T>(pointer: *mut T, value: T) {
    if pointer.is_null() {
        return;
    }
    // SAFETY: the pointer is not null and, by the C contract, valid for a write of `T`.
    unsafe { pointer.write(value) };
}

/// Reads the value behind an in-parameter. `None` for `NULL`.
pub(crate) fn get<T: Copy>(pointer: *const T) -> Option<T> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: the pointer is not null and, by the C contract, points to a valid `T`.
    Some(unsafe { pointer.read() })
}

/// A shared reference behind a pointer. `None` for `NULL`.
pub(crate) fn deref<'a, T>(pointer: *const T) -> Option<&'a T> {
    // SAFETY: the pointer, when not null, is valid for reads for the whole call.
    unsafe { pointer.as_ref() }
}

/// A unique reference behind a pointer. `None` for `NULL`.
pub(crate) fn deref_mut<'a, T>(pointer: *mut T) -> Option<&'a mut T> {
    // SAFETY: the pointer, when not null, is valid and not used elsewhere during the call:
    // the objects of proj.h are not thread-safe, and the caller owns them for the call.
    unsafe { pointer.as_mut() }
}

/// Gives a value to C: a heap object that [`release`] takes back.
pub(crate) fn leak<T>(value: T) -> *mut T {
    Box::into_raw(Box::new(value))
}

/// Takes back an object made by [`leak`]. Does nothing for `NULL`.
pub(crate) fn release<T>(pointer: *mut T) {
    if pointer.is_null() {
        return;
    }
    // SAFETY: the pointer comes from `leak` and the caller destroys it once.
    drop(unsafe { Box::from_raw(pointer) });
}

/// Strings handed to C as `const char *`. A string stays valid until its owner is destroyed.
#[derive(Debug, Default)]
pub(crate) struct Arena {
    strings: Vec<CString>,
}

impl Arena {
    /// Keeps `value` and returns a pointer to its C form.
    pub(crate) fn put(&mut self, value: &str) -> *const c_char {
        let owned = c_string(value);
        let pointer = owned.as_ptr();
        self.strings.push(owned);
        pointer
    }

    /// Like [`Arena::put`], `NULL` for `None`.
    pub(crate) fn put_opt(&mut self, value: Option<&str>) -> *const c_char {
        value.map_or(std::ptr::null(), |text| self.put(text))
    }
}

/// A `NULL`-terminated array of C strings allocated for the caller, freed by
/// `proj_string_list_destroy`.
pub(crate) fn string_list(items: &[String]) -> *mut *mut c_char {
    let mut pointers: Vec<*mut c_char> =
        items.iter().map(|item| c_string(item).into_raw()).collect();
    pointers.push(std::ptr::null_mut());
    let boxed = pointers.into_boxed_slice();
    Box::into_raw(boxed).cast::<*mut c_char>()
}

/// Frees a list made by [`string_list`]. The length is found from the `NULL` terminator.
pub(crate) fn free_string_list(list: *mut *mut c_char) {
    if list.is_null() {
        return;
    }
    let mut count = 0_usize;
    // The list comes from `string_list`: a NULL-terminated array of pointers.
    while get(list.wrapping_add(count).cast_const()).is_some_and(|item| !item.is_null()) {
        count += 1;
    }
    let whole = std::ptr::slice_from_raw_parts_mut(list, count + 1);
    // SAFETY: the array has `count + 1` pointers, as `string_list` boxed it.
    let pointers = unsafe { &*whole };
    for pointer in pointers.iter().take(count) {
        // SAFETY: each non-NULL pointer comes from `CString::into_raw` in `string_list`.
        drop(unsafe { CString::from_raw(*pointer) });
    }
    // SAFETY: the array was made by `Box::into_raw` on a boxed slice of `count + 1` pointers.
    drop(unsafe { Box::from_raw(whole) });
}

/// A heap string given to C, freed by `proj_string_destroy`: the pointer is made by
/// [`give_string`] and returned by [`take_string`].
pub(crate) fn give_string(value: &str) -> *mut c_char {
    c_string(value).into_raw()
}

/// Frees a string made by [`give_string`]. Does nothing for `NULL`.
pub(crate) fn take_string(pointer: *mut c_char) {
    if pointer.is_null() {
        return;
    }
    // SAFETY: the pointer comes from `CString::into_raw` in `give_string`.
    drop(unsafe { CString::from_raw(pointer) });
}

/// An array of `int` given to C, freed by `proj_int_list_destroy`. The length sits just before
/// the first element, so that the array can be freed from its pointer alone.
pub(crate) fn give_ints(values: &[std::ffi::c_int]) -> *mut std::ffi::c_int {
    let mut stored: Vec<std::ffi::c_int> = Vec::with_capacity(values.len() + 1);
    stored.push(std::ffi::c_int::try_from(values.len()).unwrap_or(0));
    stored.extend_from_slice(values);
    let start = Box::into_raw(stored.into_boxed_slice()).cast::<std::ffi::c_int>();
    start.wrapping_add(1)
}

/// Frees an array made by [`give_ints`]. Does nothing for `NULL`.
pub(crate) fn release_array(pointer: *mut std::ffi::c_int) {
    if pointer.is_null() {
        return;
    }
    let start = pointer.wrapping_sub(1);
    let length = usize::try_from(get(start.cast_const()).unwrap_or(0)).unwrap_or(0) + 1;
    // SAFETY: the pointer comes from `give_ints`: it follows the length of a boxed slice of
    // `length` elements, which starts one element before it.
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(start, length)) });
}
