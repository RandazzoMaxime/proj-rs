//! Whether a grid file is downloaded again, and the download itself. A port of PROJ 9.9.0
//! `proj_is_download_needed` and `proj_download_file` (`src/networkfilemanager.cpp`).
//!
//! A file goes into the user-writable directory, under the last part of its URL, and its record
//! (size, `Last-Modified`, `ETag`, time of the last check) goes into the
//! `downloaded_file_properties` table of `cache.db`. The record is what the check compares.
//!
//! Where this differs from PROJ, the reason is in `docs/PORTING-network.md`: the built-in client
//! sends no `Range`, so the check and the download each read the whole file in one request.

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::network::{self, GridContext};
use crate::network_cache::{unix_now, DiskChunkCache, Fetched, FileProperties};
use crate::network_chunks::RemoteFile;
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
    let local = ctx.cache_directory().join(file_name);
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
    let Some(fetched) = network::http_get_validated(&url) else {
        return Ok(false);
    };
    let size = u64::try_from(fetched.body.len()).unwrap_or(u64::MAX);
    if size != record.size
        || fetched.last_modified != record.last_modified
        || fetched.etag != record.etag
    {
        return Ok(true);
    }
    // The file is current: only the time of the check moves. PROJ answers `false` either way.
    let _checked = cache.set_download_last_checked(&url, unix_now());
    Ok(false)
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
    let directory = ctx.cache_directory();
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

/// The record of a download: the file is written into `out` in pieces, through the callbacks when
/// the application set some, else with one request of the built-in client.
fn write_download(
    ctx: &GridContext,
    url: &str,
    out: &mut dyn Write,
    report: &mut dyn FnMut(f64) -> bool,
) -> Result<FileProperties, ProjError> {
    let chunk = chunk_size();
    // PROJ takes the time of the download after the first reply, before the headers.
    let checked = unix_now();
    match ctx.callbacks() {
        Some(callbacks) => {
            let mut remote = RemoteFile::new(url, callbacks);
            let first = remote
                .read_at(0, usize_of(chunk))
                .map_err(|_| ProjError::Network)?;
            let total = remote.size().ok_or(ProjError::Network)?;
            let (last_modified, etag) = remote.validators();
            copy_pieces(out, total, first, chunk, report, |offset, len| {
                remote.read_at(offset, usize_of(len)).ok()
            })?;
            Ok(FileProperties {
                size: total,
                last_checked: checked,
                last_modified,
                etag,
            })
        }
        None => {
            let Fetched {
                body,
                last_modified,
                etag,
            } = network::http_get_validated(url).ok_or(ProjError::Network)?;
            let total = u64::try_from(body.len()).unwrap_or(u64::MAX);
            let first = slice(&body, 0, chunk.min(total)).ok_or(ProjError::Network)?;
            copy_pieces(out, total, first, chunk, report, |offset, len| {
                slice(&body, offset, len)
            })?;
            Ok(FileProperties {
                size: total,
                last_checked: checked,
                last_modified,
                etag,
            })
        }
    }
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

/// The bytes `offset..offset + len` of `body`, when they are all there.
fn slice(body: &[u8], offset: u64, len: u64) -> Option<Vec<u8>> {
    let start = usize::try_from(offset).ok()?;
    let end = start.checked_add(usize::try_from(len).ok()?)?;
    body.get(start..end).map(<[u8]>::to_vec)
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
    if !ctx.cache_enabled {
        return None;
    }
    let path = ctx.cache_settings().path;
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
