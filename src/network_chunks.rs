//! Ranged reads of a remote file through the application's callbacks, as PROJ reads a grid
//! (`NetworkFile`, `DOWNLOAD_CHUNK_SIZE` in `src/filemanager.cpp`).
//!
//! The file is read in chunks of [`CHUNK_SIZE`] bytes. The chunks already read, and the properties
//! of each URL (size, `Last-Modified`, `ETag`), are kept in a cache shared by every file of the
//! same builder. A second creation or a second transform asks the callbacks for nothing it already
//! has, and every reply is compared with the properties of its URL. A range with missing chunks is
//! one call:
//! `open` when no handle is open for the file, `read_range` on the handle otherwise. The handle
//! stays open until the file is released, which is what lets a tile read after creation reuse
//! it. After each call the headers `Content-Range`, `Last-Modified` and `ETag` are asked, in
//! that order, as PROJ asks them.

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::network::{content_range_total, Callbacks, NetworkError, NetworkHandle};

/// Bytes of one chunk, `DOWNLOAD_CHUNK_SIZE` in PROJ 9.9.0.
pub(crate) const CHUNK_SIZE: usize = 16384;

/// The chunks kept for one context, `gridChunkCache.max_size` in PROJ 9.9.0 (300 MB). Past it
/// the cache is emptied before the next read. PROJ evicts by age, which is not reproduced.
const CACHE_MAX_BYTES: usize = 300 * 1024 * 1024;

/// The chunks read so far, by URL and chunk index, and the properties of each URL: the size and the
/// `Last-Modified` and `ETag` of the reply that set them (`gNetworkFileProperties` of PROJ 9.9.0,
/// kept per URL and shared by every `RemoteFile` of the builder). A second `RemoteFile` of the same
/// URL knows where the file ends without a call, and compares its replies with the properties.
#[derive(Default)]
pub(crate) struct ChunkCache {
    chunks: HashMap<String, HashMap<u64, Vec<u8>>>,
    properties: HashMap<String, Validators>,
    bytes: usize,
}

impl ChunkCache {
    fn get(&self, url: &str, index: u64) -> Option<&[u8]> {
        self.chunks.get(url)?.get(&index).map(Vec::as_slice)
    }

    /// Forgets the chunks of `url` when its file changed. PROJ empties every chunk in memory
    /// (`clearMemoryCache`); the chunks of the other URLs are kept here.
    fn forget_chunks(&mut self, url: &str) {
        if let Some(file) = self.chunks.remove(url) {
            let freed: usize = file.values().map(Vec::len).sum();
            self.bytes = self.bytes.saturating_sub(freed);
        }
    }

    /// Stores the chunks of `data`, which starts at chunk `first`.
    fn store(&mut self, url: &str, first: u64, data: &[u8]) {
        let file = self.chunks.entry(url.to_owned()).or_default();
        for (position, piece) in data.chunks(CHUNK_SIZE).enumerate() {
            let Ok(offset) = u64::try_from(position) else {
                break;
            };
            if let Some(old) = file.insert(first.saturating_add(offset), piece.to_vec()) {
                self.bytes = self.bytes.saturating_sub(old.len());
            }
            self.bytes = self.bytes.saturating_add(piece.len());
        }
    }

    /// Empties the cache once it is over its budget. Called before a read, never during one, so
    /// the chunks a read has just looked at are still there.
    fn trim(&mut self) {
        if self.bytes > CACHE_MAX_BYTES {
            self.chunks.clear();
            self.bytes = 0;
        }
    }
}

/// The properties of a file that its replies give: the size from `Content-Range`, and the
/// `Last-Modified` and `ETag` headers (empty when absent). PROJ's `FileProperties`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Validators {
    size: u64,
    last_modified: String,
    etag: String,
}

/// One remote file read through the callbacks. Dropping it closes its handle.
#[derive(Debug)]
pub(crate) struct RemoteFile {
    url: String,
    callbacks: Callbacks,
    handle: Option<NetworkHandle>,
    /// The size of the file, from `Content-Range`. A reply that ends early also sets it.
    total: Option<u64>,
    /// Set once a reply differs from the properties of the URL: the chunks of the URL were
    /// dropped, and this file is not compared again (PROJ's `m_hasChanged`).
    changed: bool,
    /// The chunk database (`cache.db`) of the context, when its cache is on.
    disk: Option<crate::network_cache::Settings>,
    /// Chunks the next request asks for, doubled on sequential reads (`m_nBlocksToDownload`).
    blocks: u64,
    /// Where the last request ended (`m_lastDownloadedOffset`).
    last_downloaded: Option<u64>,
}

/// The most chunks one request asks for (`MAX_CHUNKS` of PROJ).
const MAX_CHUNKS: u64 = 64;

impl RemoteFile {
    pub(crate) fn new(url: &str, callbacks: Callbacks) -> Self {
        let total = callbacks
            .chunks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .properties
            .get(url)
            .map(|known| known.size);
        let ctx = crate::network::context();
        let disk = ctx.cache_enabled.then(|| ctx.cache_settings());
        let mut file = Self {
            url: url.to_owned(),
            callbacks,
            handle: None,
            total,
            changed: false,
            disk,
            blocks: 1,
            last_downloaded: None,
        };
        if file.total.is_none() {
            file.properties_from_disk();
        }
        file
    }

    /// `NetworkFilePropertiesCache::tryGet`: the properties `cache.db` holds for the URL, when
    /// they were checked less than the time-to-live ago, become the properties in memory.
    fn properties_from_disk(&mut self) {
        let Some(settings) = &self.disk else {
            return;
        };
        let Some(cache) = crate::network_cache::DiskChunkCache::open(&settings.path) else {
            return;
        };
        let Some(props) = cache.properties(&self.url) else {
            return;
        };
        if crate::network_cache::is_expired(&props, settings.ttl_seconds) {
            return;
        }
        self.total = Some(props.size);
        self.cache().properties.insert(
            self.url.clone(),
            Validators {
                size: props.size,
                last_modified: props.last_modified,
                etag: props.etag,
            },
        );
    }

    /// `NetworkChunkCache::get` past the memory: the chunks of `first..=last` that `cache.db`
    /// holds join the memory cache. Only once the URL has properties in memory, from a fresh
    /// entry of the database or from a reply, so a stale entry is checked before it is read.
    fn load_from_disk(&self, first: u64, last: u64) {
        let Some(settings) = &self.disk else {
            return;
        };
        let missing: Vec<u64> = {
            let cache = self.cache();
            if !cache.properties.contains_key(&self.url) {
                return;
            }
            (first..=last)
                .filter(|&index| cache.get(&self.url, index).is_none())
                .collect()
        };
        if missing.is_empty() {
            return;
        }
        let Some(disk) = crate::network_cache::DiskChunkCache::open(&settings.path) else {
            return;
        };
        for index in missing {
            if let Some(chunk) = disk.chunk(&self.url, index) {
                self.cache().store(&self.url, index, &chunk);
            }
        }
    }

    /// `NetworkChunkCache::insert` on the disk: each chunk read goes to `cache.db`.
    fn store_on_disk(&self, first: u64, data: &[u8]) {
        let Some(settings) = &self.disk else {
            return;
        };
        let Some(disk) = crate::network_cache::DiskChunkCache::open(&settings.path) else {
            return;
        };
        for (position, piece) in data.chunks(CHUNK_SIZE).enumerate() {
            let index = first.saturating_add(u64::try_from(position).unwrap_or(u64::MAX));
            let _stored = disk.insert_chunk(&self.url, index, piece, settings.max_bytes);
        }
    }

    /// The size of the file, once a reply has given it.
    pub(crate) fn size(&self) -> Option<u64> {
        self.total
    }

    pub(crate) fn has_changed(&self) -> bool {
        self.changed
    }

    fn cache(&self) -> MutexGuard<'_, ChunkCache> {
        self.callbacks
            .chunks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Up to `len` bytes from `offset`, fewer at the end of the file, as PROJ's
    /// `NetworkFile::read`: chunk after chunk, each from memory, then `cache.db`, else downloaded
    /// with the chunks after it ([`Self::fetch`]); the next chunks then come from the cache.
    pub(crate) fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, NetworkError> {
        if len == 0 {
            return Ok(Vec::new());
        }
        self.cache().trim();
        let chunk = chunk_bytes();
        let mut out = Vec::new();
        let mut at = offset;
        let mut left = u64::try_from(len).unwrap_or(u64::MAX);
        while left > 0 {
            let index = at / chunk;
            self.load_from_disk(index, index);
            let cached = self.cache().get(&self.url, index).map(<[u8]>::to_vec);
            let region = match cached {
                Some(piece) => piece,
                None => self.fetch(index, (at + left - 1) / chunk)?,
            };
            let skip = at - index * chunk;
            let available = len_u64(&region).saturating_sub(skip);
            if available == 0 {
                break;
            }
            let count = left.min(available);
            let (Ok(from), Ok(until)) = (usize::try_from(skip), usize::try_from(skip + count))
            else {
                break;
            };
            out.extend_from_slice(&region[from..until]);
            at += count;
            left -= count;
            if region.len() < CHUNK_SIZE && left != 0 {
                break;
            }
        }
        Ok(out)
    }

    /// Asks the callbacks for the chunks `low..=high`, stores them, and returns the bytes read.
    /// The request is whole chunks, past the end of the file too, as PROJ's `NetworkFile::read`
    /// asks it: the callbacks answer with what the file holds. A chunk that starts past the end
    /// is not asked.
    fn fetch(&mut self, low: u64, high: u64) -> Result<Vec<u8>, NetworkError> {
        let chunk = chunk_bytes();
        let start = low.saturating_mul(chunk);
        // `NetworkFile::read`: a read that starts where the last one ended doubles the request
        // (up to 100 chunks before the cap), any other read asks one chunk; the request covers
        // the range asked for, stops before a chunk already cached, and is capped at 64 chunks;
        // the caller's loop asks the rest.
        if self.last_downloaded == Some(start) {
            if self.blocks < 100 {
                self.blocks *= 2;
            }
        } else {
            self.blocks = 1;
        }
        let needed = high.saturating_sub(low).saturating_add(1);
        self.blocks = self.blocks.max(needed);
        // `gNetworkChunkCache.get` looks in memory, then in `cache.db`.
        let cached_at = (1..self.blocks).find(|&step| {
            let index = low.saturating_add(step);
            self.load_from_disk(index, index);
            self.cache().get(&self.url, index).is_some()
        });
        if let Some(step) = cached_at {
            self.blocks = step;
        }
        if self.blocks > MAX_CHUNKS {
            self.blocks = MAX_CHUNKS;
        }
        let high = low.saturating_add(self.blocks).saturating_sub(1);
        let stop = high.saturating_add(1).saturating_mul(chunk);
        if self.total.is_some_and(|total| start >= total) || stop <= start {
            return Ok(Vec::new());
        }
        let size = usize::try_from(stop - start).unwrap_or(usize::MAX);
        let (handle, data) = match self.handle {
            Some(handle) => (
                handle,
                self.callbacks.imp().read_range(handle, start, size)?,
            ),
            None => {
                let reply = self.callbacks.imp().open(&self.url, start, size)?;
                self.handle = Some(reply.handle);
                (reply.handle, reply.data)
            }
        };
        let first_open = self.total.is_none();
        self.observe(handle);
        if first_open && self.total.is_none() {
            // `NetworkFile::open`: a file whose first reply gives no `Content-Range` size is not
            // opened, and its handle is closed.
            self.release();
            return Err(NetworkError::Open(format!(
                "{}: no Content-Range in the reply",
                self.url
            )));
        }
        if self.total.is_none() && data.len() < size {
            // No `Content-Range`, and the reply ends before the size asked for: the file ends
            // there. With `Content-Range`, the size is that one, whatever the reply holds.
            self.total = Some(start.saturating_add(u64::try_from(data.len()).unwrap_or(0)));
        }
        self.last_downloaded = Some(start.saturating_add(len_u64(&data)));
        self.cache().store(&self.url, low, &data);
        self.store_on_disk(low, &data);
        Ok(data)
    }

    /// The headers after a call, asked in PROJ's order: `Content-Range`, then `Last-Modified` and
    /// `ETag`, and not at all once this file has seen the change (`m_hasChanged`). `Content-Range`
    /// gives the size of the file. A reply that gives a size is compared
    /// with the properties of the URL in the shared cache, as PROJ compares each reply with the
    /// properties of its file (`NetworkFile::read`). Without properties for the URL, the reply
    /// sets them. A reply that differs means the file changed while it was open: the chunks of
    /// the URL are dropped, the properties become the reply's, and this file is marked changed
    /// (PROJ's `gNetworkFileProperties.insert` and `clearMemoryCache`, then `m_hasChanged`).
    fn observe(&mut self, handle: NetworkHandle) {
        if self.changed {
            return;
        }
        let imp = self.callbacks.imp();
        let total = imp
            .header(handle, "Content-Range")
            .and_then(|value| content_range_total(&value));
        let last_modified = imp.header(handle, "Last-Modified");
        let etag = imp.header(handle, "ETag");
        let Some(size) = total else {
            return;
        };
        self.total = Some(size);
        let now = Validators {
            size,
            last_modified: last_modified.unwrap_or_default(),
            etag: etag.unwrap_or_default(),
        };
        let shared = Arc::clone(&self.callbacks.chunks);
        let mut cache = shared.lock().unwrap_or_else(PoisonError::into_inner);
        let known = cache.properties.get(&self.url).cloned();
        let differs = known.as_ref().is_some_and(|known| *known != now);
        if !differs {
            cache.properties.insert(self.url.clone(), now.clone());
        } else if !self.changed {
            self.changed = true;
            cache.forget_chunks(&self.url);
            cache.properties.insert(self.url.clone(), now.clone());
        }
        drop(cache);
        // `NetworkFilePropertiesCache::insert` writes the properties to `cache.db` too, which
        // invalidates the chunks it holds for an older file.
        if known.is_none() || differs {
            self.properties_to_disk(&now);
        }
    }

    fn properties_to_disk(&self, now: &Validators) {
        let Some(settings) = &self.disk else {
            return;
        };
        let Some(disk) = crate::network_cache::DiskChunkCache::open(&settings.path) else {
            return;
        };
        let _stored = disk.insert_properties(
            &self.url,
            &crate::network_cache::FileProperties {
                size: now.size,
                last_checked: crate::network_cache::unix_now(),
                last_modified: now.last_modified.clone(),
                etag: now.etag.clone(),
            },
        );
    }

    /// The whole file, for the grids and models read whole. The first chunk is read as the open
    /// of PROJ reads it, then the file in one read, as PROJ's readers do (`seek` to the end,
    /// `tell`, one `read`). `None` when the file cannot be read or is larger than `max`.
    pub(crate) fn read_whole(&mut self, max: u64) -> Option<Vec<u8>> {
        self.read_at(0, CHUNK_SIZE).ok()?;
        let total = self.total?;
        if total > max {
            return None;
        }
        let bytes = self.read_at(0, usize::try_from(total).ok()?).ok()?;
        (!bytes.is_empty() && len_u64(&bytes) == total).then_some(bytes)
    }

    /// Closes the handle, if one is open. A later read opens the file again.
    pub(crate) fn release(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.callbacks.imp().close(handle);
        }
        // A later read is a new `NetworkFile` in PROJ: no sequential history.
        self.blocks = 1;
        self.last_downloaded = None;
    }
}

impl Drop for RemoteFile {
    fn drop(&mut self) {
        self.release();
    }
}

/// [`CHUNK_SIZE`] as a file offset.
fn chunk_bytes() -> u64 {
    u64::try_from(CHUNK_SIZE).unwrap_or(u64::MAX)
}

fn len_u64(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

/// `Read` and `Seek` over a [`RemoteFile`] shared with the grid that reads it, so that the TIFF
/// decoder reads only the ranges it asks for.
///
/// A window (`set_window`) replaces the file for one range that the caller has read already: a
/// read inside it is served from its bytes, and a read past it is the end of the data. A chunk
/// decoded from a window is read once from the callbacks, one `read_range` per strip or tile, as
/// PROJ's measured reads are.
#[derive(Clone, Debug)]
pub(crate) struct RemoteReader {
    file: Arc<Mutex<RemoteFile>>,
    position: u64,
    /// The file offset of `bytes[0]`, and the bytes of the window.
    window: Option<(u64, Vec<u8>)>,
}

impl RemoteReader {
    pub(crate) fn new(file: Arc<Mutex<RemoteFile>>) -> Self {
        Self {
            file,
            position: 0,
            window: None,
        }
    }

    /// Serves the reads from `bytes`, which are the file from `start`, instead of the callbacks.
    pub(crate) fn set_window(&mut self, start: u64, bytes: Vec<u8>) {
        self.window = Some((start, bytes));
    }

    fn file(&self) -> MutexGuard<'_, RemoteFile> {
        self.file.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The bytes of the window at the position, up to `len`; `None` when there is no window.
    fn window_read(&mut self, len: usize) -> Option<Vec<u8>> {
        let (start, bytes) = self.window.as_ref()?;
        let from = self
            .position
            .checked_sub(*start)
            .and_then(|offset| usize::try_from(offset).ok())
            .unwrap_or(usize::MAX);
        let served = bytes
            .get(from..)
            .map(|rest| rest[..rest.len().min(len)].to_vec())
            .unwrap_or_default();
        Some(served)
    }
}

impl Read for RemoteReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if let Some(served) = self.window_read(buf.len()) {
            let count = served.len();
            buf[..count].copy_from_slice(&served);
            self.position = self.position.saturating_add(len_u64(&served));
            return Ok(count);
        }
        let bytes = self
            .file()
            .read_at(self.position, buf.len())
            .map_err(io::Error::other)?;
        let count = bytes.len().min(buf.len());
        buf[..count].copy_from_slice(&bytes[..count]);
        self.position = self.position.saturating_add(len_u64(&bytes[..count]));
        Ok(count)
    }
}

impl Seek for RemoteReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let next = match position {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => {
                let size = self
                    .file()
                    .size()
                    .ok_or_else(|| io::Error::other("the size of the remote file is unknown"))?;
                size.checked_add_signed(delta)
            }
        };
        let next = next.ok_or_else(|| io::Error::other("seek before the start of the file"))?;
        self.position = next;
        Ok(next)
    }
}

/// The file of a grid that the callbacks serve: `name` is one that
/// [`crate::network::decide_download`] gives a URL for, and callbacks are set. `None` otherwise,
/// and the grid is read as before.
pub(crate) fn callback_file(name: &str) -> Option<RemoteFile> {
    let callbacks = crate::network::context().reading_callbacks()?;
    let download = crate::network::decide_download(name)?;
    Some(RemoteFile::new(&download.url, callbacks))
}
