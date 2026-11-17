//! The HTTP client of the `network` feature: plain `http` and `https` (system TLS through
//! `native-tls`), redirects, a `CONNECT` tunnel through `https_proxy`. PROJ has the same client
//! only when it is built with libcurl, see `docs/PORTING-network-feature.md`.

use std::cell::Cell;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::network::{is_http, nonempty_env, MAX_GRID_BYTES};

thread_local! {
    static LAST_TRANSIENT: Cell<bool> = const { Cell::new(false) };
}

/// Whether the last HTTP fetch of this thread failed on a connection reset or a timeout, the
/// transport errors PROJ's curl client tries again (`GetNewRetryDelay`).
pub(crate) fn last_fetch_transient() -> bool {
    LAST_TRANSIENT.with(std::cell::Cell::get)
}

/// `result` as an option, noting a transport error that a later attempt may not meet.
fn io_step<T>(result: std::io::Result<T>) -> Option<T> {
    result
        .map_err(|error| {
            use std::io::ErrorKind;
            let transient = matches!(
                error.kind(),
                ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::BrokenPipe
                    | ErrorKind::TimedOut
                    | ErrorKind::WouldBlock
            );
            if transient {
                LAST_TRANSIENT.with(|flag| flag.set(true));
            }
        })
        .ok()
}

/// A `GET` that follows the redirects, with the `Last-Modified` and `ETag` of the reply.
pub(crate) fn http_get_validated(url: &str) -> Option<crate::network_cache::Fetched> {
    let reply = http_fetch(url, None)?;
    if reply.status != 200 || reply.body.is_empty() {
        return None;
    }
    let validator = |name| header_value(&reply.headers, name).unwrap_or("").to_owned();
    Some(crate::network_cache::Fetched {
        last_modified: validator("last-modified"),
        etag: validator("etag"),
        body: reply.body,
    })
}

/// The last reply of `url` that is not a redirect, the redirects followed (at most four), with
/// `range` as the `Range` header when given. Sets `LAST_NOT_FOUND` for a 404 or a 410.
pub(crate) fn http_fetch(url: &str, range: Option<&str>) -> Option<HttpReply> {
    crate::network::record_status(None);
    LAST_TRANSIENT.with(|flag| flag.set(false));
    let mut url = url.to_owned();
    for _ in 0..4 {
        let target = parse_http_url(&url)?;
        let reply = http_exchange(&target, range)?;
        if !matches!(reply.status, 301 | 302 | 303 | 307 | 308) {
            crate::network::record_status(Some(reply.status));
            return Some(reply);
        }
        let location = header_value(&reply.headers, "location")?;
        url = resolve_location(&target, location)?;
    }
    None
}

struct HttpTarget {
    https: bool,
    host: String,
    port: u16,
    path: String,
}

fn parse_http_url(url: &str) -> Option<HttpTarget> {
    let (https, rest) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else {
        (false, url.strip_prefix("http://")?)
    };
    if rest.is_empty() || rest.contains(['\r', '\n', ' ']) {
        return None;
    }
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, "/".to_owned()),
    };
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let (host, port) = split_host_port(authority, if https { 443 } else { 80 })?;
    Some(HttpTarget {
        https,
        host,
        port,
        path,
    })
}

fn split_host_port(authority: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        if host.is_empty() {
            return None;
        }
        let port = if let Some(port) = tail.strip_prefix(':') {
            if port.is_empty() {
                return None;
            }
            port.parse().ok()?
        } else if tail.is_empty() {
            default_port
        } else {
            return None;
        };
        return Some((host.to_owned(), port));
    }
    if let Some((host, port)) = authority.rsplit_once(':') {
        if !host.is_empty() && !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some((host.to_owned(), port.parse().ok()?));
        }
    }
    if authority.is_empty() {
        return None;
    }
    Some((authority.to_owned(), default_port))
}

fn resolve_location(current: &HttpTarget, location: &str) -> Option<String> {
    if is_http(location) {
        return Some(location.to_owned());
    }
    let path = location.strip_prefix('/')?;
    let scheme = if current.https { "https" } else { "http" };
    Some(format!(
        "{scheme}://{}:{}/{}",
        current.host, current.port, path
    ))
}

pub(crate) struct HttpReply {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

fn http_exchange(target: &HttpTarget, range: Option<&str>) -> Option<HttpReply> {
    let stream = connect(target)?;
    if target.https {
        let connector = native_tls::TlsConnector::new().ok()?;
        let tls = connector.connect(&target.host, stream).ok()?;
        read_http(tls, target, range)
    } else {
        read_http(stream, target, range)
    }
}

/// The TCP stream to `target`: direct, or through the `CONNECT` tunnel of the HTTPS proxy when
/// [`https_proxy`] names one. Both ways have the timeouts of a request.
fn connect(target: &HttpTarget) -> Option<TcpStream> {
    let proxy = https_proxy(target);
    let (host, port) = match &proxy {
        Some((host, port)) => (host.as_str(), *port),
        None => (target.host.as_str(), target.port),
    };
    let stream = io_step(TcpStream::connect((host, port)))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .ok()?;
    match proxy {
        Some(_) => open_tunnel(stream, target),
        None => Some(stream),
    }
}

/// The HTTPS proxy an `https` request goes through, as `(host, port)`: `https_proxy`, else
/// `HTTPS_PROXY`, as curl reads them. Nothing when the host is listed in `no_proxy` or `NO_PROXY`,
/// or when the proxy URL carries credentials, which are not sent. A plain `http` request is never
/// sent through the proxy.
fn https_proxy(target: &HttpTarget) -> Option<(String, u16)> {
    if !target.https || bypasses_proxy(&target.host) {
        return None;
    }
    let value = nonempty_env("https_proxy").or_else(|| nonempty_env("HTTPS_PROXY"))?;
    let rest = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or(&value);
    let authority = rest.split('/').next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    split_host_port(authority, 80)
}

/// Whether `no_proxy` (or `NO_PROXY`) lists `host`: `*`, the host itself, or a domain it is in.
fn bypasses_proxy(host: &str) -> bool {
    let Some(list) = nonempty_env("no_proxy").or_else(|| nonempty_env("NO_PROXY")) else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    list.split(',')
        .map(|entry| entry.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .any(|entry| entry == "*" || host == entry || host.ends_with(&format!(".{entry}")))
}

/// Asks the proxy for a `CONNECT` tunnel to `target`. The reply head is read one byte at a time,
/// so no byte of the TLS handshake that follows is read here. `None` unless the proxy answers 200.
fn open_tunnel(mut stream: TcpStream, target: &HttpTarget) -> Option<TcpStream> {
    let request = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nUser-Agent: proj-rs\r\n\r\n",
        host = target.host,
        port = target.port
    );
    io_step(stream.write_all(request.as_bytes()))?;
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 16_384 || io_step(stream.read(&mut byte))? == 0 {
            return None;
        }
        head.push(byte[0]);
    }
    let status_line = std::str::from_utf8(&head).ok()?.lines().next()?;
    (status_line.split_whitespace().nth(1)? == "200").then_some(stream)
}

fn read_http(
    mut stream: impl Read + Write,
    target: &HttpTarget,
    range: Option<&str>,
) -> Option<HttpReply> {
    let host = if (target.https && target.port == 443) || (!target.https && target.port == 80) {
        target.host.clone()
    } else {
        format!("{}:{}", target.host, target.port)
    };
    let range = range.map_or_else(String::new, |range| format!("Range: {range}\r\n"));
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: proj-rs\r\n{range}Connection: close\r\n\r\n",
        path = target.path
    );
    io_step(stream.write_all(request.as_bytes()))?;
    let mut raw = Vec::new();
    io_step(
        stream
            .take(MAX_GRID_BYTES.saturating_add(65_536))
            .read_to_end(&mut raw),
    )?;
    let reply = split_http(&raw)?;
    if u64::try_from(reply.body.len()).ok()? > MAX_GRID_BYTES {
        return None;
    }
    Some(reply)
}

fn split_http(raw: &[u8]) -> Option<HttpReply> {
    let end = raw.windows(4).position(|mark| mark == b"\r\n\r\n")?;
    let header_text = std::str::from_utf8(&raw[..end]).ok()?;
    let mut lines = header_text.split("\r\n");
    let status = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    let mut body = raw[end + 4..].to_vec();
    if header_value(&headers, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        body = decode_chunks(&body)?;
    } else if let Some(length) = header_value(&headers, "content-length") {
        let length = length.parse::<usize>().ok()?;
        if body.len() < length {
            return None;
        }
        body.truncate(length);
    }
    Some(HttpReply {
        status,
        headers,
        body,
    })
}

fn decode_chunks(mut bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        let line_end = bytes.windows(2).position(|mark| mark == b"\r\n")?;
        let size_text = std::str::from_utf8(&bytes[..line_end]).ok()?;
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        bytes = &bytes[line_end + 2..];
        if size == 0 {
            break;
        }
        if bytes.len() < size + 2 {
            return None;
        }
        out.extend_from_slice(&bytes[..size]);
        if &bytes[size..size + 2] != b"\r\n" {
            return None;
        }
        bytes = &bytes[size + 2..];
    }
    Some(out)
}

pub(crate) fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}
