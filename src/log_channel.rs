//! The channel a message of PROJ's logger takes when it is sent from deep inside a call, where
//! the context is not at hand: `NetworkFile::open` answers a failed open with
//! `Cannot open <url>: <error>` (`networkfilemanager.cpp:293, 1323, 2586, 2741`), during the
//! sort of the candidates of a CRS pair as anywhere else.
//!
//! `pj_log` writes to the logger of the context of the caller, so the C library installs the
//! sink of its context on each of its entry points (`capi/src/context.rs`: `context`). A
//! library built on the Rust facade installs none, and the messages go nowhere.

use std::sync::Arc;

/// The message channel of a context: a function that receives one line at a time.
pub type LogSink = Arc<dyn Fn(&str) + Send + Sync>;

thread_local! {
    static SINK: std::cell::RefCell<Option<LogSink>> = const { std::cell::RefCell::new(None) };
}

/// Points the message channel of this thread at `sink`: the C library does so on each of its
/// entry points, with the logger of the context the call came with, so a message from inside
/// the call reaches the same logger `pj_log` writes to.
pub fn set_log_sink(sink: LogSink) {
    SINK.with(|channel| *channel.borrow_mut() = Some(sink));
}

/// A message PROJ writes to the logger of the calling context, from the depth of a call:
/// `Cannot open <url>: <error>` of `NetworkFile::open` (`networkfilemanager.cpp:1323`).
pub(crate) fn log_error(message: &str) {
    SINK.with(|channel| {
        if let Some(sink) = channel.borrow().as_ref() {
            sink(message);
        }
    });
}
