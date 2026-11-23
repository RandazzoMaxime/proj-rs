//! Reads the embedded `proj.db` through a SQLite VFS, one page at a time.
//!
//! The database is stored in `data/proj.pages`: every 4 KiB page compressed on its own with zlib
//! (`tools/proj-db/pack_pages.py`), so the binary carries about a quarter of the 10 MiB file. A
//! read decompresses the pages it touches into a stack buffer; SQLite's own page cache keeps the
//! ones it reuses. Creation pays the decompression once per process (about 10 ms); the conversions
//! do not read the database at all.
//!
//! On Linux the compressed streams are read with `pread` from the executable file, as the
//! uncompressed bytes were before: a page read through the mapping would count in the resident set
//! of the process. Elsewhere, or when the file cannot be checked, they are read in place.

use std::ffi::{c_char, c_int, c_void, CStr};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{FileExt, MetadataExt};
use std::ptr;
use std::sync::OnceLock;

use miniz_oxide::inflate::decompress_slice_iter_to_slice;
use rusqlite::ffi;
use rusqlite::{Connection, OpenFlags};

const VFS_NAME: &CStr = c"proj-rs-exe";
const FILE_NAME: &CStr = c"proj-rs-embedded";
const MAGIC: &[u8; 4] = b"PRJP";
const HEADER: usize = 20;

/// The largest zlib stream of a page: a 4 KiB page does not compress to more than this.
const MAX_STREAM: usize = 8192;

/// The compressed pages of the embedded database, parsed from the header of `proj.pages`. The
/// streams come from the executable file when `file` is set, else from `bytes`.
struct Pages {
    bytes: &'static [u8],
    #[cfg(target_os = "linux")]
    file: Option<(File, u64)>,
    page: usize,
    len: usize,
    data: usize,
}

impl Pages {
    fn parse(bytes: &'static [u8]) -> Option<Self> {
        let header = bytes.get(..HEADER)?;
        if &header[..4] != MAGIC {
            return None;
        }
        let page = usize::try_from(u32::from_le_bytes(header[4..8].try_into().ok()?)).ok()?;
        let count = usize::try_from(u32::from_le_bytes(header[8..12].try_into().ok()?)).ok()?;
        let len = usize::try_from(u64::from_le_bytes(header[12..20].try_into().ok()?)).ok()?;
        let data = HEADER + 4 * (count + 1);
        if page == 0 || count * page < len || bytes.len() < data {
            return None;
        }
        Some(Self {
            bytes,
            #[cfg(target_os = "linux")]
            file: locate(bytes),
            page,
            len,
            data,
        })
    }

    /// Decompress page `index` into `out`, which holds exactly the bytes of that page.
    fn decode(&self, index: usize, out: &mut [u8]) -> bool {
        let mut scratch = [0u8; MAX_STREAM];
        let Some(stream) = self.stream(index, &mut scratch) else {
            return false;
        };
        decompress_slice_iter_to_slice(out, std::iter::once(stream), true, false)
            .is_ok_and(|written| written == out.len())
    }

    /// The zlib stream of page `index`: read from the executable file when there is one.
    fn stream<'a>(&'a self, index: usize, scratch: &'a mut [u8]) -> Option<&'a [u8]> {
        let at = |i: usize| -> Option<usize> {
            let start = HEADER + 4 * i;
            let raw = self.bytes.get(start..start + 4)?;
            usize::try_from(u32::from_le_bytes(raw.try_into().ok()?)).ok()
        };
        let (from, to) = (self.data + at(index)?, self.data + at(index + 1)?);
        #[cfg(target_os = "linux")]
        if let Some((file, base)) = &self.file {
            let buffer = scratch.get_mut(..to.checked_sub(from)?)?;
            file.read_exact_at(buffer, base + u64::try_from(from).ok()?)
                .ok()?;
            return Some(&*buffer);
        }
        let _ = scratch;
        self.bytes.get(from..to)
    }

    /// Length of page `index`: the page size, but less for the last one.
    fn page_len(&self, index: usize) -> usize {
        self.len.saturating_sub(index * self.page).min(self.page)
    }
}

#[cfg(target_os = "linux")]
/// The file mapped under `bytes`, and the offset of `bytes[0]` in it. The first and last page
/// are read back and compared, so a binary replaced on disk is not trusted.
fn locate(bytes: &'static [u8]) -> Option<(File, u64)> {
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
    (first && last).then_some((file, offset))
}

static SOURCE: OnceLock<Option<Pages>> = OnceLock::new();
static REGISTERED: OnceLock<bool> = OnceLock::new();

/// A connection to the embedded database, or `None` when the VFS cannot be registered.
pub(crate) fn open(pages: &'static [u8]) -> Option<Connection> {
    SOURCE.get_or_init(|| Pages::parse(pages)).as_ref()?;
    if !*REGISTERED.get_or_init(register) {
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
/// pages once, so a smaller cache costs few extra decompressions and saves heap: with 500, the
/// peak of `Proj::new_known_crs("EPSG:4326", "EPSG:2154")` is 1.4 MiB lower than with 2000.
const CACHE_KIB: i64 = 500;

/// The whole database in memory, decompressed: for the one path that copies it
/// (`DatabaseContext::create_with_auxiliary` without a path), and for when the VFS is unavailable.
pub(crate) fn decode_all(pages: &'static [u8]) -> Option<Vec<u8>> {
    let parsed = Pages::parse(pages)?;
    let mut image = vec![0u8; parsed.len];
    for (index, out) in image.chunks_mut(parsed.page).enumerate() {
        if !parsed.decode(index, out) {
            return None;
        }
    }
    Some(image)
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
    let (Some(Some(source)), Ok(amount), Ok(offset)) = (
        SOURCE.get(),
        usize::try_from(amount),
        usize::try_from(offset),
    ) else {
        return ffi::SQLITE_IOERR_READ;
    };
    // SAFETY: SQLite gives a buffer of `amount` writable bytes.
    let buffer = unsafe { std::slice::from_raw_parts_mut(buffer.cast::<u8>(), amount) };
    let available = source.len.saturating_sub(offset);
    let count = amount.min(available);
    let (head, tail) = buffer.split_at_mut(count);
    let mut scratch = [0u8; 4096];
    let mut done = 0;
    while done < count {
        let at = offset + done;
        let index = at / source.page;
        let inside = at % source.page;
        let page_len = source.page_len(index);
        let page = &mut scratch[..page_len];
        if !source.decode(index, page) {
            return ffi::SQLITE_IOERR_READ;
        }
        let take = (page_len - inside).min(count - done);
        head[done..done + take].copy_from_slice(&page[inside..inside + take]);
        done += take;
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

    const WHOLE: &[u8] = include_bytes!("../data/proj.db");
    const PAGES: &[u8] = include_bytes!("../data/proj.pages");

    fn count(connection: &Connection) -> i64 {
        connection
            .query_row("SELECT count(*) FROM geodetic_crs, crs_view", [], |row| {
                row.get(0)
            })
            .expect("count")
    }

    #[test]
    fn every_page_decodes_to_the_bytes_of_the_database() {
        assert_eq!(decode_all(PAGES).expect("decode"), WHOLE);
    }

    #[test]
    fn reads_the_same_rows_as_the_whole_database() {
        let from_pages = open(PAGES).expect("the VFS opens the pages");
        let mut in_memory = Connection::open_in_memory().expect("memory");
        in_memory
            .deserialize_bytes(rusqlite::MAIN_DB, WHOLE)
            .expect("deserialize");
        assert_eq!(count(&from_pages), count(&in_memory));
        let size: i64 = from_pages
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .expect("page_count");
        let page: i64 = from_pages
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .expect("page_size");
        assert_eq!(usize::try_from(size * page).expect("size"), WHOLE.len());
    }

    #[test]
    fn refuses_writes() {
        let from_pages = open(PAGES).expect("open");
        assert!(from_pages
            .execute_batch("CREATE TABLE written (a)")
            .is_err());
    }

    #[test]
    fn threads_read_through_one_file() {
        let threads: Vec<_> = (0..4)
            .map(|_| std::thread::spawn(|| count(&open(PAGES).expect("open"))))
            .collect();
        let counts: Vec<i64> = threads
            .into_iter()
            .map(|thread| thread.join().expect("thread"))
            .collect();
        assert!(counts.windows(2).all(|pair| pair[0] == pair[1]));
    }
}
