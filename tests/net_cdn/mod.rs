//! PROJ's `initial_check` (`test/unit/test_network.cpp:90`), shared by the cases that need the real
//! CDN file: `download_whole_files` and `file_api`. Each case includes this file with `#[path]`.
//!
//! `cdn_reachable` is `GET https://cdn.proj.org/fr_ign_ntf_r93.tif` with `Range: bytes=0-1`, and
//! the CDN is reachable when it answers `206`. Through `HTTPS_PROXY` when it is set, as curl goes
//! through it. The probe makes its own request, so a broken download cannot pass for an
//! unreachable CDN.

use std::io::{Read, Write};
use std::net::TcpStream;

const CDN_HOST: &str = "cdn.proj.org";

/// Whether the CDN answers the `initial_check` request with `206`.
pub(crate) fn cdn_reachable() -> bool {
    let Some(stream) = tunnel_to_cdn() else {
        return false;
    };
    let Ok(connector) = native_tls::TlsConnector::new() else {
        return false;
    };
    let Ok(mut tls) = connector.connect(CDN_HOST, stream) else {
        return false;
    };
    let request = format!(
        "GET /fr_ign_ntf_r93.tif HTTP/1.1\r\nHost: {CDN_HOST}\r\nRange: bytes=0-1\r\n\
         User-Agent: proj-rs-test\r\nConnection: close\r\n\r\n"
    );
    if tls.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while head.len() < 512 {
        match tls.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => break,
        }
        if head.ends_with(b"\r\n") {
            break;
        }
    }
    head.starts_with(b"HTTP/1.1 206")
}

/// A TCP stream to the CDN: direct, or through the `CONNECT` tunnel of `HTTPS_PROXY`.
fn tunnel_to_cdn() -> Option<TcpStream> {
    let proxy = ["https_proxy", "HTTPS_PROXY"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()));
    let Some(proxy) = proxy else {
        return TcpStream::connect((CDN_HOST, 443)).ok();
    };
    let authority = proxy
        .strip_prefix("http://")
        .unwrap_or(&proxy)
        .trim_end_matches('/');
    let mut stream = TcpStream::connect(authority).ok()?;
    let connect = format!("CONNECT {CDN_HOST}:443 HTTP/1.1\r\nHost: {CDN_HOST}:443\r\n\r\n");
    stream.write_all(connect.as_bytes()).ok()?;
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 4096 || stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        head.push(byte[0]);
    }
    head.starts_with(b"HTTP/1.1 200").then_some(stream)
}
