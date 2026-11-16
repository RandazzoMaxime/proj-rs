//! The network functions PROJ uses when the application gives none: its curl client
//! (`curl_open`, `curl_read_range`, `curl_get_header_value`, `curl_close` of
//! `src/networkfilemanager.cpp`). Each read is one `GET` with a `Range` header, so a grid is read
//! in the chunks [`crate::network_chunks::RemoteFile`] asks for, as PROJ reads it, not whole.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::network::{
    header_value, http_fetch, Callbacks, HttpReply, NetworkCallbacks, NetworkError, NetworkHandle,
    OpenReply,
};

/// The headers of a reply, as (name, value).
type Headers = Vec<(String, String)>;
/// The URL of each open handle and the headers of its last reply.
type OpenFiles = HashMap<u64, (String, Headers)>;

/// The open files: the URL of each handle and the headers of its last reply.
#[derive(Default)]
struct BuiltinHttp {
    next: AtomicU64,
    open: Mutex<OpenFiles>,
}

impl BuiltinHttp {
    /// `size_to_read` bytes of `url` from `offset`, asked with a `Range` header. As PROJ's curl
    /// client, any reply under `300` gives the first bytes of its body: a server that ignores the
    /// range sends no `Content-Range`, and the open fails on that.
    fn get(url: &str, offset: u64, size_to_read: usize) -> Option<(Vec<u8>, Headers)> {
        let last = offset.saturating_add(u64::try_from(size_to_read).ok()?.saturating_sub(1));
        let range = format!("bytes={offset}-{last}");
        let mut delay = MIN_RETRY_DELAY_MS;
        let mut reply = loop {
            delay = match http_fetch(url, Some(&range)) {
                Some(reply) if reply.status < 300 => break reply,
                Some(reply) if transient_reply(&reply) => next_delay(delay)?,
                None if crate::network::last_fetch_transient() => next_delay(delay)?,
                _ => return None,
            };
            std::thread::sleep(Duration::from_secs_f64(delay / 1000.0));
        };
        reply.body.truncate(size_to_read);
        Some((reply.body, reply.headers))
    }

    fn table(&self) -> std::sync::MutexGuard<'_, OpenFiles> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl NetworkCallbacks for BuiltinHttp {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        let (data, headers) = Self::get(url, offset, size_to_read)
            .ok_or_else(|| NetworkError::Open(format!("cannot open {url}")))?;
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        self.table().insert(id, (url.to_owned(), headers));
        Ok(OpenReply {
            handle: NetworkHandle(id),
            data,
        })
    }

    fn header(&self, handle: NetworkHandle, name: &str) -> Option<String> {
        let table = self.table();
        let (_, headers) = table.get(&handle.0)?;
        header_value(headers, &name.to_ascii_lowercase())
            .or_else(|| header_value(headers, name))
            .map(str::to_owned)
    }

    fn read_range(
        &self,
        handle: NetworkHandle,
        offset: u64,
        size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        let url = self
            .table()
            .get(&handle.0)
            .map(|(url, _)| url.clone())
            .ok_or_else(|| NetworkError::ReadRange("unknown handle".to_owned()))?;
        let (data, headers) = Self::get(&url, offset, size_to_read)
            .ok_or_else(|| NetworkError::ReadRange(format!("cannot read {url}")))?;
        if let Some(entry) = self.table().get_mut(&handle.0) {
            entry.1 = headers;
        }
        Ok(data)
    }

    fn close(&self, handle: NetworkHandle) {
        self.table().remove(&handle.0);
    }
}

const MIN_RETRY_DELAY_MS: f64 = 500.0;
const MAX_RETRY_DELAY_MS: f64 = 60_000.0;

/// A failed reply that a retry may cure, as PROJ's `GetNewRetryDelay` reads it.
fn transient_reply(reply: &HttpReply) -> bool {
    matches!(reply.status, 429 | 500 | 502..=504)
        || (reply.status == 400 && String::from_utf8_lossy(&reply.body).contains("RequestTimeout"))
}

/// The next wait of `GetNewRetryDelay`: twice the last one plus up to a quarter of it at random.
/// `None` to give up, at a wait of a minute or more.
fn next_delay(last: f64) -> Option<f64> {
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos());
    let delay = last * (2.0 + 0.5 * f64::from(jitter) / 1e9);
    (delay < MAX_RETRY_DELAY_MS).then_some(delay)
}

/// New built-in network functions, with an empty chunk cache.
pub(crate) fn builtin_callbacks() -> Callbacks {
    Callbacks::new(Arc::new(BuiltinHttp::default()))
}
