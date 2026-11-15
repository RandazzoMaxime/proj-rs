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
}

impl RemoteFile {
    pub(crate) fn new(url: &str, callbacks: Callbacks) -> Self {
        let total = callbacks
            .chunks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .properties
            .get(url)
            .map(|known| known.size);
        Self {
            url: url.to_owned(),
            callbacks,
            handle: None,
            total,
            changed: false,
        }
    }

    /// The size of the file, once a reply has given it.
    pub(crate) fn size(&self) -> Option<u64> {
        self.total
    }

    /// The `Last-Modified` and `ETag` the URL has now, empty when absent. A download stores them as
    /// its properties (`get_props_from_headers`), once its first range is read.
    pub(crate) fn validators(&self) -> (String, String) {
        self.cache()
            .properties
            .get(&self.url)
            .map(|known| (known.last_modified.clone(), known.etag.clone()))
            .unwrap_or_default()
    }

    /// Whether a reply showed that the file changed while it was open. The chunks of the reply
    /// that showed it are the new file's, the chunks read before are gone.
    pub(crate) fn has_changed(&self) -> bool {
        self.changed
    }

    fn cache(&self) -> MutexGuard<'_, ChunkCache> {
        self.callbacks
            .chunks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Up to `len` bytes from `offset`, fewer at the end of the file. The chunks the range
    /// touches come from the cache when they are there; the missing ones are asked in one call,
    /// rounded out to whole chunks.
    pub(crate) fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, NetworkError> {
        if len == 0 {
            return Ok(Vec::new());
        }
        self.cache().trim();
        let chunk = chunk_bytes();
        let end = offset.saturating_add(u64::try_from(len).unwrap_or(u64::MAX));
        let first = offset / chunk;
        let last = (end - 1) / chunk;
        // The chunks of the range as they are before the fetch: a fetch that finds the file
        // changed empties the cache, and this read must still get its chunks.
        let cached: Vec<Option<Vec<u8>>> = (first..=last)
            .map(|index| self.cache().get(&self.url, index).map(<[u8]>::to_vec))
            .collect();
        let fetched = match self.missing_span(first, last) {
            Some((low, high)) => Some((low, self.fetch(low, high)?)),
            None => None,
        };
        Ok(self.assemble(offset, end, first, last, fetched.as_ref(), &cached))
    }

    /// The first and the last missing chunk of `first..=last`, when there is one.
    fn missing_span(&self, first: u64, last: u64) -> Option<(u64, u64)> {
        let cache = self.cache();
        let is_missing = |index: u64| cache.get(&self.url, index).is_none();
        let low = (first..=last).find(|&index| is_missing(index))?;
        let high = (first..=last)
            .rev()
            .find(|&index| is_missing(index))
            .unwrap_or(low);
        Some((low, high))
    }

    /// Asks the callbacks for the chunks `low..=high`, stores them, and returns the bytes read.
    /// The request is whole chunks, past the end of the file too, as PROJ's `NetworkFile::read`
    /// asks it: the callbacks answer with what the file holds. A chunk that starts past the end
    /// is not asked.
    fn fetch(&mut self, low: u64, high: u64) -> Result<Vec<u8>, NetworkError> {
        let chunk = chunk_bytes();
        let start = low.saturating_mul(chunk);
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
        self.observe(handle);
        if self.total.is_none() && data.len() < size {
            // No `Content-Range`, and the reply ends before the size asked for: the file ends
            // there. With `Content-Range`, the size is that one, whatever the reply holds.
            self.total = Some(start.saturating_add(u64::try_from(data.len()).unwrap_or(0)));
        }
        self.cache().store(&self.url, low, &data);
        Ok(data)
    }

    /// The headers after a call, asked in PROJ's order: `Content-Range`, then `Last-Modified` and
    /// `ETag`. `Content-Range` gives the size of the file. A reply that gives a size is compared
    /// with the properties of the URL in the shared cache, as PROJ compares each reply with the
    /// properties of its file (`NetworkFile::read`). Without properties for the URL, the reply
    /// sets them. A reply that differs means the file changed while it was open: the chunks of
    /// the URL are dropped, the properties become the reply's, and this file is marked changed
    /// (PROJ's `gNetworkFileProperties.insert` and `clearMemoryCache`, then `m_hasChanged`).
    fn observe(&mut self, handle: NetworkHandle) {
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
        let differs = cache
            .properties
            .get(&self.url)
            .is_some_and(|known| *known != now);
        if !differs {
            cache.properties.insert(self.url.clone(), now);
        } else if !self.changed {
            self.changed = true;
            cache.forget_chunks(&self.url);
            cache.properties.insert(self.url.clone(), now);
        }
    }

    /// The bytes of `offset..end`, from the chunks `first..=last`. The chunks of the call that
    /// fetched them (`fetched`, from its first chunk) come first; the others are in `cached`, as
    /// they were before the call. Stops at the end of the file.
    fn assemble(
        &self,
        offset: u64,
        end: u64,
        first: u64,
        last: u64,
        fetched: Option<&(u64, Vec<u8>)>,
        cached: &[Option<Vec<u8>>],
    ) -> Vec<u8> {
        let chunk = chunk_bytes();
        let mut out = Vec::new();
        for index in first..=last {
            let piece: &[u8] = match fetched {
                Some((low, data)) if index >= *low && (index - low) * chunk < len_u64(data) => {
                    let from = usize::try_from((index - low) * chunk).unwrap_or(usize::MAX);
                    &data[from..(from + CHUNK_SIZE).min(data.len())]
                }
                _ => match usize::try_from(index - first)
                    .ok()
                    .and_then(|position| cached.get(position))
                    .and_then(Option::as_deref)
                {
                    Some(piece) => piece,
                    None => break,
                },
            };
            let chunk_start = index.saturating_mul(chunk);
            let from = offset.saturating_sub(chunk_start).min(len_u64(piece));
            let until = end.saturating_sub(chunk_start).min(len_u64(piece));
            if until > from {
                let (Ok(from), Ok(until)) = (usize::try_from(from), usize::try_from(until)) else {
                    break;
                };
                out.extend_from_slice(&piece[from..until]);
            }
            if piece.len() < CHUNK_SIZE {
                break;
            }
        }
        out
    }

    /// The whole file, read chunk by chunk, for the grids that are not tiled. Without a
    /// `Content-Range` size, the first reply is the whole file. `None` when the file cannot be
    /// read or is larger than `max`. The chunks already read are not asked again.
    pub(crate) fn read_whole(&mut self, max: u64) -> Option<Vec<u8>> {
        let chunk = chunk_bytes();
        let first = self.read_at(0, CHUNK_SIZE).ok()?;
        let total = self
            .total
            .unwrap_or_else(|| u64::try_from(first.len()).unwrap_or(u64::MAX));
        if total > max {
            return None;
        }
        let mut bytes = first;
        while u64::try_from(bytes.len()).ok()? < total {
            let offset = u64::try_from(bytes.len()).ok()?;
            let wanted = usize::try_from((total - offset).min(chunk)).ok()?;
            let part = self.read_at(offset, wanted).ok()?;
            if part.is_empty() {
                return None;
            }
            bytes.extend_from_slice(&part);
        }
        bytes.truncate(usize::try_from(total).ok()?);
        if bytes.is_empty() {
            None
        } else {
            Some(bytes)
        }
    }

    /// Closes the handle, if one is open. A later read opens the file again.
    pub(crate) fn release(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.callbacks.imp().close(handle);
        }
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
    let callbacks = crate::network::context().callbacks()?;
    let download = crate::network::decide_download(name)?;
    Some(RemoteFile::new(&download.url, callbacks))
}
