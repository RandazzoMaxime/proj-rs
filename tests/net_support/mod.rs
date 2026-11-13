//! A local HTTP/1.1 server for the network tests: `std::net::TcpListener`, one thread, no
//! dependency. Nothing here reaches `cdn.proj.org`.
//!
//! A route is a path and a reply. A path with no route answers `404`. Every request line is
//! recorded, so a test can assert which paths were asked for, and how many times.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// A route: the status and body sent for one path.
pub(crate) struct Route {
    pub(crate) path: &'static str,
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

pub(crate) struct LocalServer {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl LocalServer {
    /// Serve `routes` on a free port of `127.0.0.1` until the server is dropped.
    pub(crate) fn spawn(routes: Vec<Route>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
        let port = listener.local_addr().expect("local address").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let requests_thread = Arc::clone(&requests);
        let stop_thread = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                if stop_thread.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    answer(stream, &routes, &requests_thread);
                }
            }
        });
        Self {
            url: format!("http://127.0.0.1:{port}"),
            requests,
            stop,
            handle: Some(handle),
        }
    }

    /// The base URL, `http://127.0.0.1:<port>`, with no trailing slash.
    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    /// The request paths received so far, in order.
    pub(crate) fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocking accept so the thread sees the stop flag.
        let _woken = TcpStream::connect(self.url.trim_start_matches("http://"));
        if let Some(handle) = self.handle.take() {
            let _joined = handle.join();
        }
    }
}

fn answer(mut stream: TcpStream, routes: &[Route], requests: &Mutex<Vec<String>>) {
    let _timeout = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buffer = [0_u8; 4096];
    let read = stream.read(&mut buffer).unwrap_or(0);
    let head = String::from_utf8_lossy(&buffer[..read]);
    let path = head.split_whitespace().nth(1).unwrap_or("").to_owned();
    if let Ok(mut log) = requests.lock() {
        log.push(path.clone());
    }
    let (status, body): (u16, &[u8]) = match routes.iter().find(|route| route.path == path) {
        Some(route) => (route.status, &route.body),
        None => (404, b""),
    };
    let reason = if status == 200 { "OK" } else { "Not Found" };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _sent = stream
        .write_all(header.as_bytes())
        .and_then(|()| stream.write_all(body))
        .and_then(|()| stream.flush());
}
