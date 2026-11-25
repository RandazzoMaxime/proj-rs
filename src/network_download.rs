//! Whether a grid file is downloaded again, and the download itself. A port of PROJ 9.9.0
//! `proj_is_download_needed` and `proj_download_file` (`src/networkfilemanager.cpp`).
//!
//! A file goes into the user-writable directory, under the last part of its URL, and its record
//! (size, `Last-Modified`, `ETag`, time of the last check) goes into the
//! `downloaded_file_properties` table of `cache.db`. The record is what the check compares.
//!
//! The check and the download go through the network functions, the application's or the built-in
//! client, as PROJ's go through `ctx->networking`: ranged reads, no chunk cache.

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::network::{self, GridContext};
use crate::network_cache::{unix_now, DiskChunkCache, FileProperties};
use crate::proj::{ProjError, TtlSetting};

/// `FULL_FILE_CHUNK_SIZE` of PROJ: the piece a download is written in, and the step of its progress.
const FULL_FILE_CHUNK_SIZE: u64 = 1024 * 1024;

/// Distinguishes the temporary files of downloads made by one process, as PROJ's `%d_%p` suffix does.
static TEMPORARY_COUNT: AtomicU64 = AtomicU64::new(0);

/// `proj_is_download_needed`: whether `name` must be downloaded.
///
/// The file is needed when it is not in the user-writable directory, or when it has no record in
/// `cache.db`. A record is checked against the server when the TTL of the grid cache has run out,
/// unless `ttl` is [`TtlSetting::Ignore`], which sends no request. A changed size, `Last-Modified`
/// or `ETag` makes the file needed; an unchanged one only moves the time of the last check.
///
/// # Errors
///
/// [`ProjError::Network`] when network access is off.
pub(crate) fn is_download_needed(
    ctx: &GridContext,
    name: &str,
    ttl: TtlSetting,
) -> Result<bool, ProjError> {
    if !ctx.enabled {
        return Err(ProjError::Network);
    }
    let url = network::build_url(ctx, name);
    let Some(file_name) = network::cache_file_name(&url) else {
        return Ok(false);
    };
    // A hermetic context with no directory has no file to find: the download is needed, and it
    // fails for want of a directory.
    let Some(directory) = ctx.writable_directory() else {
        return Ok(true);
    };
    let local = directory.join(file_name);
    if !network::is_file_with(ctx.file_api().as_deref(), &local) {
        return Ok(true);
    }
    let Some(cache) = open_cache(ctx) else {
        return Ok(false);
    };
    let Some(record) = cache.download_properties(&url) else {
        return Ok(true);
    };
    if ttl == TtlSetting::Ignore
        || !crate::network_cache::is_expired(&record, ctx.cache_ttl_seconds)
    {
        return Ok(false);
    }
    let Some(probe) = probe_properties(ctx, &url) else {
        return Ok(false);
    };
    let (size, last_modified, etag) = probe;
    if size != record.size || last_modified != record.last_modified || etag != record.etag {
        return Ok(true);
    }
    // The file is current: only the time of the check moves. PROJ answers `false` either way.
    let _checked = cache.set_download_last_checked(&url, unix_now());
    Ok(false)
}

/// The size and the validators of `url`, from a one-byte `open` (`open(url, 0, 1)` of PROJ's check).
/// `None` when the open fails or its reply has no `Content-Range`.
fn probe_properties(ctx: &GridContext, url: &str) -> Option<(u64, String, String)> {
    let callbacks = ctx.reading_callbacks()?;
    let imp = callbacks.imp();
    let handle = imp.open(url, 0, 1).ok()?.handle;
    let size = imp
        .header(handle, "Content-Range")
        .and_then(|value| network::content_range_total(&value));
    let last_modified = imp.header(handle, "Last-Modified").unwrap_or_default();
    let etag = imp.header(handle, "ETag").unwrap_or_default();
    imp.close(handle);
    Some((size?, last_modified, etag))
}

/// `proj_download_file`: downloads `name` into the user-writable directory, unless
/// [`is_download_needed`] says the file is current.
///
/// `report` gets the fraction of the file written after each piece, except the first, and stops
/// the download when it returns `false`. The file is put under its name only once it is complete,
/// and its record is stored then.
///
/// # Errors
///
/// [`ProjError::Network`] when network access is off, the server does not answer, the reply has no
/// size, or `report` stops the download. [`ProjError::Path`] when the file cannot be written.
pub(crate) fn download(
    ctx: &GridContext,
    name: &str,
    ttl: TtlSetting,
    report: &mut dyn FnMut(f64) -> bool,
) -> Result<(), ProjError> {
    if !is_download_needed(ctx, name, ttl)? {
        return Ok(());
    }
    let url = network::build_url(ctx, name);
    let file_name = network::cache_file_name(&url).ok_or(ProjError::Network)?;
    // `invalidateGridInfo` of the short name, before the download (networkfilemanager.cpp:
    // 2690-2705): the records of the file are looked up again once it is on disk.
    ctx.grid_info().invalidate(&file_name);
    let directory = ctx.writable_directory().ok_or(ProjError::Path)?;
    network::create_directory_all(ctx, &directory).map_err(|_| ProjError::Path)?;
    let local = directory.join(&file_name);
    let count = TEMPORARY_COUNT.fetch_add(1, Ordering::Relaxed);
    let temporary = directory.join(format!("{file_name}{}_{count}", std::process::id()));

    let mut out = network::create_for_write(ctx, &temporary).map_err(|_| ProjError::Path)?;
    let written = write_download(ctx, &url, out.as_mut(), report);
    drop(out);
    let record = match written {
        Ok(record) => record,
        Err(error) => {
            let _gone = network::remove_file(ctx, &temporary);
            return Err(error);
        }
    };
    let _replaced = network::remove_file(ctx, &local);
    if network::rename_file(ctx, &temporary, &local).is_err() {
        let _gone = network::remove_file(ctx, &temporary);
        return Err(ProjError::Path);
    }
    store_record(ctx, &url, &record)
}

/// The record of a download, as `proj_download_file`: the file is written into `out` in pieces,
/// one `open` then a `read_range` per piece, on the network functions themselves (the
/// application's, else the built-in client), not through the chunk cache. The first reply gives
/// the size (`Content-Range`) and the validators, or the download fails.
fn write_download(
    ctx: &GridContext,
    url: &str,
    out: &mut dyn Write,
    report: &mut dyn FnMut(f64) -> bool,
) -> Result<FileProperties, ProjError> {
    let chunk = chunk_size();
    let callbacks = ctx.reading_callbacks().ok_or(ProjError::Network)?;
    let imp = callbacks.imp();
    let opened = imp
        .open(url, 0, usize_of(chunk))
        .map_err(|_| ProjError::Network)?;
    let handle = opened.handle;
    // PROJ takes the time of the download after the first reply, before the headers.
    let checked = unix_now();
    let written = (|| {
        let total = imp
            .header(handle, "Content-Range")
            .and_then(|value| network::content_range_total(&value))
            .ok_or(ProjError::Network)?;
        let last_modified = imp.header(handle, "Last-Modified").unwrap_or_default();
        let etag = imp.header(handle, "ETag").unwrap_or_default();
        copy_pieces(out, total, opened.data, chunk, report, |offset, len| {
            imp.read_range(handle, offset, usize_of(len)).ok()
        })?;
        Ok(FileProperties {
            size: total,
            last_checked: checked,
            last_modified,
            etag,
        })
    })();
    imp.close(handle);
    written
}

/// Writes the `total` bytes of a file: `first` is its first piece, and `read(offset, len)` gives
/// the next ones. After each piece after the first, `report` gets the fraction written.
fn copy_pieces(
    out: &mut dyn Write,
    total: u64,
    first: Vec<u8>,
    chunk: u64,
    report: &mut dyn FnMut(f64) -> bool,
    mut read: impl FnMut(u64, u64) -> Option<Vec<u8>>,
) -> Result<(), ProjError> {
    if total == 0 || first.is_empty() {
        return Err(ProjError::Network);
    }
    let mut written = write_piece(out, &first)?;
    while written < total {
        let wanted = chunk.min(total - written);
        let part = read(written, wanted)
            .filter(|part| u64::try_from(part.len()).ok() == Some(wanted))
            .ok_or(ProjError::Network)?;
        written += write_piece(out, &part)?;
        if !report(fraction(written, total)) {
            return Err(ProjError::Network);
        }
    }
    Ok(())
}

/// `written / total`, the progress PROJ reports (`double(totalDownloaded) / props.size`). Exact
/// below 2^53 bytes, which every file a grid can be is.
#[expect(
    clippy::as_conversions,
    reason = "byte counts below 2^53 are exact as f64, as PROJ's double division takes them"
)]
fn fraction(written: u64, total: u64) -> f64 {
    written as f64 / total as f64
}

fn write_piece(out: &mut dyn Write, bytes: &[u8]) -> Result<u64, ProjError> {
    out.write_all(bytes).map_err(|_| ProjError::Path)?;
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

/// Stores the record of `url` in `cache.db`, replacing the one it had (PROJ's select, then update
/// or insert).
fn store_record(ctx: &GridContext, url: &str, record: &FileProperties) -> Result<(), ProjError> {
    let cache = open_cache(ctx).ok_or(ProjError::Network)?;
    cache
        .set_download_properties(url, record)
        .map_err(|_| ProjError::Network)
}

/// The record database, when the grid cache is on (PROJ's `DiskChunkCache::open`). Its directory is
/// made through the file API of `ctx`, as PROJ makes the user-writable directory.
fn open_cache(ctx: &GridContext) -> Option<DiskChunkCache> {
    let path = ctx.chunk_cache()?.path;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        let _made = network::create_directory_all(ctx, parent);
    }
    DiskChunkCache::open(&path)
}

/// The size of a download piece: `PROJ_FULL_FILE_CHUNK_SIZE` when it is a positive number, else
/// 1 MiB. PROJ reads the variable at each download, for its own tests.
fn chunk_size() -> u64 {
    std::env::var("PROJ_FULL_FILE_CHUNK_SIZE")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(FULL_FILE_CHUNK_SIZE)
}

fn usize_of(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
