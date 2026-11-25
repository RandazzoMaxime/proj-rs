#!/usr/bin/env python3
"""Local proxy for clients that give one proxy to every URL scheme (tools/clients/pdal.sh).

`split_proxy.py <port file>` listens on 127.0.0.1 (a free port, written to <port file>) and:
- tunnels each CONNECT (HTTPS) through the upstream proxy of $HTTPS_PROXY, unchanged;
- sends each plain-HTTP request (absolute form) straight to its host, as a client with
  HTTPS_PROXY set and no HTTP_PROXY would.

PDAL's HTTP layer (arbiter) takes HTTPS_PROXY as CURLOPT_PROXY for http:// URLs too, so an
HTTPS-only (CONNECT-only) proxy refuses its plain-HTTP requests (the JSON schemas of STAC
catalogs, http://json-schema.org/...). Standard library only.
"""
import socket
import socketserver
import sys
import threading
import urllib.parse


def pipe(a, b):
    try:
        while True:
            data = a.recv(65536)
            if not data:
                break
            b.sendall(data)
    except OSError:
        pass
    finally:
        for s in (a, b):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def relay(a, b, first=b""):
    if first:
        b.sendall(first)
    t = threading.Thread(target=pipe, args=(b, a), daemon=True)
    t.start()
    pipe(a, b)
    t.join()


class Handler(socketserver.BaseRequestHandler):
    upstream = None

    def handle(self):
        client = self.request
        head = b""
        while b"\r\n\r\n" not in head:
            chunk = client.recv(65536)
            if not chunk:
                return
            head += chunk
        line = head.split(b"\r\n", 1)[0].decode("latin-1")
        method, target, _ = line.split(" ", 2)
        if method == "CONNECT":
            server = socket.create_connection(self.upstream)
            relay(client, server, head)
            return
        url = urllib.parse.urlsplit(target)
        if url.scheme != "http" or not url.hostname:
            client.sendall(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            return
        path = (url.path or "/") + ("?" + url.query if url.query else "")
        rest = head.split(b"\r\n", 1)[1]
        lines = [l for l in rest.split(b"\r\n") if not l.lower().startswith(b"proxy-")]
        request = f"{method} {path} HTTP/1.1\r\n".encode("latin-1") + b"\r\n".join(lines)
        server = socket.create_connection((url.hostname, url.port or 80))
        relay(client, server, request)


class Server(socketserver.ThreadingTCPServer):
    daemon_threads = True
    allow_reuse_address = True


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: split_proxy.py <port file>")
    import os

    proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
    if not proxy:
        sys.exit("split_proxy.py: no HTTPS_PROXY")
    up = urllib.parse.urlsplit(proxy if "://" in proxy else "http://" + proxy)
    Handler.upstream = (up.hostname, up.port or 3128)
    with Server(("127.0.0.1", 0), Handler) as server:
        with open(sys.argv[1], "w") as f:
            f.write(str(server.server_address[1]))
        server.serve_forever()


if __name__ == "__main__":
    main()
