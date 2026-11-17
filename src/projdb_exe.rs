//! Reads the embedded `proj.db` from the executable file with `pread`, through a SQLite VFS.
//!
//! The 10 MiB database sits in the read-only data of the binary. Reading it through the
//! mapping makes every page that a query touches resident in this process, and the kernel
//! maps 16 pages around each fault. PROJ reads its `proj.db` with `read`, so those pages stay
//! in the page cache of the kernel and never count in the resident set of the process. This
//! VFS does the same: SQLite asks for a page, `pread` copies it from the file of the binary,
//! and the mapping is never touched. Linux only: the position of the bytes in the file comes
//! from `/proc/self/maps`. When anything does not check out, [`open`] returns `None` and the
//! caller reads the bytes in place.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::ptr;
use std::sync::OnceLock;

use rusqlite::ffi;
use rusqlite::{Connection, OpenFlags};

const VFS_NAME: &CStr = c"proj-rs-exe";
const FILE_NAME: &CStr = c"proj-rs-embedded";

/// The executable and where the embedded bytes start in it.
struct Source {
    file: File,
    offset: u64,
    len: u64,
}

static SOURCE: OnceLock<Option<Source>> = OnceLock::new();
static REGISTERED: OnceLock<bool> = OnceLock::new();

/// A connection that reads `bytes` from the executable file, or `None` when the bytes do not
/// map to a file we can read back identically.
pub(crate) fn open(bytes: &'static [u8]) -> Option<Connection> {
    let source = SOURCE.get_or_init(|| locate(bytes)).as_ref()?;
    if source.len != u64::try_from(bytes.len()).ok()? || !*REGISTERED.get_or_init(register) {
        return None;
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let uri = "file:proj-rs-embedded?immutable=1";
    let connection = Connection::open_with_flags_and_vfs(uri, flags, "proj-rs-exe").ok()?;
    connection
        .pragma_update(None, "cache_size", -CACHE_KIB)
        .ok()?;
    Some(connection)
}

/// Page cache of one connection, in KiB. The default is 2000. A creation reads about 2 MiB of
/// pages once, so a smaller cache costs few extra `pread` and saves heap: with 500, the peak of
/// `Proj::new_known_crs("EPSG:4326", "EPSG:2154")` is 1.4 MiB lower than with 2000.
const CACHE_KIB: i64 = 500;

/// The file mapped under `bytes`, and the offset of `bytes[0]` in it. The first and last page
/// are read back and compared, so a binary replaced on disk is not trusted.
fn locate(bytes: &'static [u8]) -> Option<Source> {
    #[expect(
        clippy::as_conversions,
        reason = "the address of a static, compared to /proc/self/maps"
    )]
    let start = bytes.as_ptr() as usize;
    let end = start.checked_add(bytes.len())?;
    let maps = {
        let mut text = String::with_capacity(1 << 16);
        File::open("/proc/self/maps")
            .ok()?
            .read_to_string(&mut text)
            .ok()?;
        text
    };
    let (path, offset, inode) = maps.lines().find_map(|line| {
        let mut fields = line.splitn(6, ' ');
        let (from, to) = fields.next()?.split_once('-')?;
        let from = usize::from_str_radix(from, 16).ok()?;
        let to = usize::from_str_radix(to, 16).ok()?;
        if from > start || end > to {
            return None;
        }
        let _perms = fields.next()?;
        let file_offset = u64::from_str_radix(fields.next()?, 16).ok()?;
        let _dev = fields.next()?;
        let inode: u64 = fields.next()?.parse().ok()?;
        let path = fields.next()?.trim_start();
        Some((
            path.to_owned(),
            file_offset + u64::try_from(start - from).ok()?,
            inode,
        ))
    })?;
    if inode == 0 || !path.starts_with('/') || path.ends_with(" (deleted)") {
        return None;
    }
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.ino() != inode {
        return None;
    }
    let len = u64::try_from(bytes.len()).ok()?;
    let page = bytes.len().min(4096);
    let mut buffer = vec![0u8; page];
    let first = file.read_exact_at(&mut buffer, offset).is_ok() && buffer == bytes[..page];
    let tail = len - u64::try_from(page).ok()?;
    let last = file.read_exact_at(&mut buffer, offset + tail).is_ok()
        && buffer == bytes[bytes.len() - page..];
    (first && last).then_some(Source { file, offset, len })
}

fn register() -> bool {
    // SAFETY: `sqlite3_vfs_find(NULL)` returns the default VFS, valid for the process.
    let default = unsafe { ffi::sqlite3_vfs_find(ptr::null()) };
    if default.is_null() {
        return false;
    }
    // SAFETY: `default` is non-null and points to a live `sqlite3_vfs`.
    let default_vfs = unsafe { &*default };
    let (version, file_size, max_path) = (
        default_vfs.iVersion,
        default_vfs.szOsFile,
        default_vfs.mxPathname,
    );
    if version < 1 {
        return false;
    }
    let own_size = c_int::try_from(size_of::<ffi::sqlite3_file>()).unwrap_or(c_int::MAX);
    // SAFETY: an all-zero `sqlite3_vfs` is valid: integers, null pointers and `None`.
    let mut vfs: ffi::sqlite3_vfs = unsafe { std::mem::zeroed() };
    vfs.iVersion = 1;
    vfs.szOsFile = file_size.max(own_size);
    vfs.mxPathname = max_path;
    vfs.zName = VFS_NAME.as_ptr();
    vfs.pAppData = default.cast();
    vfs.xOpen = Some(x_open);
    vfs.xDelete = Some(x_delete);
    vfs.xAccess = Some(x_access);
    vfs.xFullPathname = Some(x_full_pathname);
    vfs.xRandomness = Some(x_randomness);
    vfs.xSleep = Some(x_sleep);
    vfs.xCurrentTime = Some(x_current_time);
    let leaked = Box::into_raw(Box::new(vfs));
    // SAFETY: `leaked` is never freed, as SQLite requires of a registered VFS.
    unsafe { ffi::sqlite3_vfs_register(leaked, 0) == ffi::SQLITE_OK }
}

/// The default VFS that `vfs` stands in front of, as a pointer for the call and as a reference
/// to read its methods.
///
/// # Safety
///
/// `vfs` is the VFS built by [`register`].
unsafe fn inner(vfs: *mut ffi::sqlite3_vfs) -> (*mut ffi::sqlite3_vfs, &'static ffi::sqlite3_vfs) {
    // SAFETY: the caller passes our VFS, whose `pAppData` is the default VFS.
    let app = unsafe { (*vfs).pAppData };
    let raw = app.cast::<ffi::sqlite3_vfs>();
    // SAFETY: the default VFS is never freed or moved, and SQLite does not write to it.
    (raw, unsafe { &*raw })
}

/// Whether `name` is the name of the embedded file.
///
/// # Safety
///
/// `name` is NULL or NUL-terminated.
unsafe fn is_embedded(name: *const c_char) -> bool {
    // SAFETY: the caller promises a NUL-terminated name.
    !name.is_null() && unsafe { CStr::from_ptr(name) } == FILE_NAME
}

unsafe extern "C" fn x_open(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    out_flags: *mut c_int,
) -> c_int {
    // SAFETY: SQLite passes a NUL-terminated name or NULL.
    if unsafe { is_embedded(name) } {
        if flags & ffi::SQLITE_OPEN_READONLY == 0 {
            return ffi::SQLITE_CANTOPEN;
        }
        // SAFETY: SQLite passes a file of `szOsFile` bytes, which holds a `sqlite3_file`.
        unsafe { (*file).pMethods = &raw const IO_METHODS };
        if !out_flags.is_null() {
            // SAFETY: a non-null `pOutFlags` is a valid output pointer.
            unsafe { *out_flags = ffi::SQLITE_OPEN_READONLY };
        }
        return ffi::SQLITE_OK;
    }
    // SAFETY: SQLite calls this function with the VFS registered by `register`.
    let (raw, default) = unsafe { inner(vfs) };
    match default.xOpen {
        // SAFETY: forwards the arguments SQLite gave, to the default VFS.
        Some(open) => unsafe { open(raw, name, file, flags, out_flags) },
        None => ffi::SQLITE_CANTOPEN,
    }
}

unsafe extern "C" fn x_delete(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    sync: c_int,
) -> c_int {
    // SAFETY: SQLite calls this function with the VFS registered by `register`.
    let (raw, default) = unsafe { inner(vfs) };
    match default.xDelete {
        // SAFETY: forwards the arguments SQLite gave, to the default VFS.
        Some(delete) => unsafe { delete(raw, name, sync) },
        None => ffi::SQLITE_IOERR_DELETE,
    }
}

unsafe extern "C" fn x_access(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    flags: c_int,
    out: *mut c_int,
) -> c_int {
    // SAFETY: SQLite calls this function with the VFS registered by `register`.
    let (raw, default) = unsafe { inner(vfs) };
    match default.xAccess {
        // SAFETY: forwards the arguments SQLite gave, to the default VFS.
        Some(access) => unsafe { access(raw, name, flags, out) },
        None => ffi::SQLITE_IOERR_ACCESS,
    }
}

unsafe extern "C" fn x_full_pathname(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    size: c_int,
    out: *mut c_char,
) -> c_int {
    // SAFETY: SQLite passes a NUL-terminated name or NULL.
    if unsafe { is_embedded(name) } {
        let bytes = FILE_NAME.to_bytes_with_nul();
        if usize::try_from(size).is_ok_and(|room| room >= bytes.len()) {
            // SAFETY: `out` holds `size` bytes, at least `bytes.len()`, and does not overlap.
            unsafe { ptr::copy_nonoverlapping(bytes.as_ptr().cast(), out, bytes.len()) };
            return ffi::SQLITE_OK;
        }
        return ffi::SQLITE_CANTOPEN;
    }
    // SAFETY: SQLite calls this function with the VFS registered by `register`.
    let (raw, default) = unsafe { inner(vfs) };
    match default.xFullPathname {
        // SAFETY: forwards the arguments SQLite gave, to the default VFS.
        Some(full) => unsafe { full(raw, name, size, out) },
        None => ffi::SQLITE_CANTOPEN,
    }
}

unsafe extern "C" fn x_randomness(
    vfs: *mut ffi::sqlite3_vfs,
    size: c_int,
    out: *mut c_char,
) -> c_int {
    // SAFETY: SQLite calls this function with the VFS registered by `register`.
    let (raw, default) = unsafe { inner(vfs) };
    match default.xRandomness {
        // SAFETY: forwards the arguments SQLite gave, to the default VFS.
        Some(random) => unsafe { random(raw, size, out) },
        None => 0,
    }
}

unsafe extern "C" fn x_sleep(vfs: *mut ffi::sqlite3_vfs, micros: c_int) -> c_int {
    // SAFETY: SQLite calls this function with the VFS registered by `register`.
    let (raw, default) = unsafe { inner(vfs) };
    match default.xSleep {
        // SAFETY: forwards the arguments SQLite gave, to the default VFS.
        Some(sleep) => unsafe { sleep(raw, micros) },
        None => 0,
    }
}

unsafe extern "C" fn x_current_time(vfs: *mut ffi::sqlite3_vfs, out: *mut f64) -> c_int {
    // SAFETY: SQLite calls this function with the VFS registered by `register`.
    let (raw, default) = unsafe { inner(vfs) };
    match default.xCurrentTime {
        // SAFETY: forwards the arguments SQLite gave, to the default VFS.
        Some(time) => unsafe { time(raw, out) },
        None => ffi::SQLITE_ERROR,
    }
}

static IO_METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    iVersion: 1,
    xClose: Some(io_ok),
    xRead: Some(io_read),
    xWrite: Some(io_write),
    xTruncate: Some(io_truncate),
    xSync: Some(io_sync),
    xFileSize: Some(io_size),
    xLock: Some(io_lock),
    xUnlock: Some(io_lock),
    xCheckReservedLock: Some(io_reserved),
    xFileControl: Some(io_control),
    xSectorSize: None,
    xDeviceCharacteristics: Some(io_characteristics),
    xShmMap: None,
    xShmLock: None,
    xShmBarrier: None,
    xShmUnmap: None,
    xFetch: None,
    xUnfetch: None,
};

unsafe extern "C" fn io_ok(_file: *mut ffi::sqlite3_file) -> c_int {
    ffi::SQLITE_OK
}

unsafe extern "C" fn io_read(
    _file: *mut ffi::sqlite3_file,
    buffer: *mut c_void,
    amount: c_int,
    offset: i64,
) -> c_int {
    let (Some(Some(source)), Ok(amount), Ok(offset)) =
        (SOURCE.get(), usize::try_from(amount), u64::try_from(offset))
    else {
        return ffi::SQLITE_IOERR_READ;
    };
    // SAFETY: SQLite gives a buffer of `amount` writable bytes.
    let buffer = unsafe { std::slice::from_raw_parts_mut(buffer.cast::<u8>(), amount) };
    let available = usize::try_from(source.len.saturating_sub(offset)).unwrap_or(usize::MAX);
    let count = amount.min(available);
    let (head, tail) = buffer.split_at_mut(count);
    if source
        .file
        .read_exact_at(head, source.offset + offset)
        .is_err()
    {
        return ffi::SQLITE_IOERR_READ;
    }
    if tail.is_empty() {
        return ffi::SQLITE_OK;
    }
    tail.fill(0);
    ffi::SQLITE_IOERR_SHORT_READ
}

unsafe extern "C" fn io_write(
    _file: *mut ffi::sqlite3_file,
    _buffer: *const c_void,
    _amount: c_int,
    _offset: i64,
) -> c_int {
    ffi::SQLITE_IOERR_WRITE
}

unsafe extern "C" fn io_truncate(_file: *mut ffi::sqlite3_file, _size: i64) -> c_int {
    ffi::SQLITE_IOERR_TRUNCATE
}

unsafe extern "C" fn io_sync(_file: *mut ffi::sqlite3_file, _flags: c_int) -> c_int {
    ffi::SQLITE_OK
}

unsafe extern "C" fn io_size(_file: *mut ffi::sqlite3_file, out: *mut i64) -> c_int {
    let Some(Some(source)) = SOURCE.get() else {
        return ffi::SQLITE_IOERR_FSTAT;
    };
    let Ok(len) = i64::try_from(source.len) else {
        return ffi::SQLITE_IOERR_FSTAT;
    };
    // SAFETY: SQLite gives a valid output pointer.
    unsafe { *out = len };
    ffi::SQLITE_OK
}

unsafe extern "C" fn io_lock(_file: *mut ffi::sqlite3_file, _level: c_int) -> c_int {
    ffi::SQLITE_OK
}

unsafe extern "C" fn io_reserved(_file: *mut ffi::sqlite3_file, out: *mut c_int) -> c_int {
    // SAFETY: SQLite gives a valid output pointer.
    unsafe { *out = 0 };
    ffi::SQLITE_OK
}

unsafe extern "C" fn io_control(
    _file: *mut ffi::sqlite3_file,
    _op: c_int,
    _argument: *mut c_void,
) -> c_int {
    ffi::SQLITE_NOTFOUND
}

unsafe extern "C" fn io_characteristics(_file: *mut ffi::sqlite3_file) -> c_int {
    ffi::SQLITE_IOCAP_IMMUTABLE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(connection: &Connection) -> i64 {
        connection
            .query_row("SELECT count(*) FROM geodetic_crs, crs_view", [], |row| {
                row.get(0)
            })
            .expect("count")
    }

    #[test]
    fn reads_the_same_rows_as_the_bytes_in_place() {
        let bytes = crate::projdb::embedded_bytes();
        let from_file = open(bytes).expect("the test binary maps proj.db from its own file");
        let mut in_place = Connection::open_in_memory().expect("memory");
        in_place
            .deserialize_bytes(rusqlite::MAIN_DB, bytes)
            .expect("deserialize");
        assert_eq!(count(&from_file), count(&in_place));
        let size: i64 = from_file
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .expect("page_count");
        let page: i64 = from_file
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .expect("page_size");
        assert_eq!(usize::try_from(size * page).expect("size"), bytes.len());
    }

    #[test]
    fn refuses_writes() {
        let from_file = open(crate::projdb::embedded_bytes()).expect("open");
        assert!(from_file.execute_batch("CREATE TABLE written (a)").is_err());
    }

    #[test]
    fn threads_read_through_one_file() {
        let bytes = crate::projdb::embedded_bytes();
        let threads: Vec<_> = (0..4)
            .map(|_| std::thread::spawn(move || count(&open(bytes).expect("open"))))
            .collect();
        let counts: Vec<i64> = threads
            .into_iter()
            .map(|thread| thread.join().expect("thread"))
            .collect();
        assert!(counts.windows(2).all(|pair| pair[0] == pair[1]));
    }
}
