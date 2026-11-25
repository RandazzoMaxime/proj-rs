//! The channel a message of PROJ's logger takes when it is sent from deep inside a call, where
//! the context is not at hand: `NetworkFile::open` answers a failed open with
//! `Cannot open <url>: <error>` (`networkfilemanager.cpp:293, 1323, 2586, 2741`), during the
//! sort of the candidates of a CRS pair as anywhere else.
//!
//! `pj_log` writes to the logger of the context of the caller, so the C library points this
//! thread's channel at its context for the time of each of its entry points
//! (`capi/src/context.rs`: `context`, `capi/src/util.rs`: `guard`). A library built on the Rust
//! facade installs none, and the messages go nowhere.

use std::cell::Cell;

/// The message channel of a context: `write(owner, line)` receives one line at a time. `owner`
/// names the context to the function that wrote the sink; this crate never reads it. A plain
/// function and an integer, so installing a sink allocates nothing.
#[derive(Clone, Copy, Debug)]
pub struct LogSink {
    owner: usize,
    write: fn(usize, &str),
}

impl LogSink {
    /// The sink that hands each line to `write`, with `owner`.
    #[must_use]
    pub const fn new(owner: usize, write: fn(usize, &str)) -> Self {
        Self { owner, write }
    }

    /// The `owner` the sink was made with.
    #[must_use]
    pub const fn owner(&self) -> usize {
        self.owner
    }
}

thread_local! {
    static SINK: Cell<Option<LogSink>> = const { Cell::new(None) };
}

/// Points the message channel of this thread at `sink` (`None`: nowhere), and gives back the
/// one it replaces, so a caller can put it back when its call ends.
pub fn replace_log_sink(sink: Option<LogSink>) -> Option<LogSink> {
    SINK.with(|channel| channel.replace(sink))
}

/// The sink of this thread's message channel.
#[must_use]
pub fn log_sink() -> Option<LogSink> {
    SINK.with(Cell::get)
}

/// A message PROJ writes to the logger of the calling context, from the depth of a call:
/// `Cannot open <url>: <error>` of `NetworkFile::open` (`networkfilemanager.cpp:1323`).
pub(crate) fn log_error(message: &str) {
    if let Some(sink) = log_sink() {
        (sink.write)(sink.owner, message);
    }
}

/// `f` with the message channel of this thread pointed nowhere, then put back. `createFromPROJString`
/// instantiates a PROJ string under a logger of its own that keeps the last error line alone
/// (`io.cpp:12761-12810`), and `proj_create` logs that line after its name: a
/// `Cannot open <url>` sent from inside that instantiation never reaches the context.
pub(crate) fn muted<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(Option<LogSink>);
    impl Drop for Restore {
        fn drop(&mut self) {
            replace_log_sink(self.0);
        }
    }
    let _restore = Restore(replace_log_sink(None));
    f()
}

#[cfg(test)]
mod tests {
    use super::{log_error, log_sink, muted, replace_log_sink, LogSink};
    use std::cell::RefCell;

    thread_local! {
        static SEEN: RefCell<Vec<(usize, String)>> = const { RefCell::new(Vec::new()) };
    }

    fn record(owner: usize, line: &str) {
        SEEN.with(|seen| seen.borrow_mut().push((owner, line.to_owned())));
    }

    #[test]
    fn a_line_reaches_the_installed_sink_and_none_after_it_is_put_back() {
        log_error("before");
        let previous = replace_log_sink(Some(LogSink::new(7, record)));
        assert!(previous.is_none());
        log_error("Cannot open https://cdn.proj.org/x.tif: Network functionality not available");
        muted(|| log_error("inside createFromPROJString"));
        assert_eq!(log_sink().map(|sink| sink.owner()), Some(7));
        replace_log_sink(previous);
        log_error("after");
        assert!(log_sink().is_none());
        let seen = SEEN.with(|seen| seen.borrow().clone());
        assert_eq!(
            seen,
            vec![(
                7,
                "Cannot open https://cdn.proj.org/x.tif: Network functionality not available"
                    .to_owned()
            )]
        );
    }
}
