#!/usr/bin/env python3
"""A local stand-in for github.com's release pages, for testing `padwight update`, the daemon's
daily check and `dist/get.sh` without a published release (qa-plan INS-12, INS-13).

padwight and get.sh only use two URLs, both on https://github.com/<repo>:
  /releases/latest                    a redirect to /releases/tag/v<version> (curl -w %{url_effective})
  /releases/download/v<version>/<f>   the tarball and its .sha256
so this serves exactly those, over TLS with a throwaway CA, and a plain-HTTP CONNECT proxy that
tunnels every https request to it. Nothing in padwight changes: the test points curl and wget at
the proxy (https_proxy) and trusts the CA (CURL_CA_BUNDLE, WGETRC ca_certificate). Nothing leaves
the machine and no system file is touched.

    fake-release-server.py --dir release/ --latest 0.2.0 --state state/

--dir        files served under /releases/download/v<any>/ (the output of scripts/package.sh)
--latest V   what /releases/latest redirects to; omitted: a redirect to /releases (no release yet)
--state D    where it writes ca.pem, requests.log and ready.json (proxy port, CA path); the
             caller waits for ready.json
--fault F    truncate:<substr>  send half of the matching file, then cut the connection
             status:<code>:<substr>  answer matching paths with that HTTP status
"""

import argparse
import http.server
import json
import os
import select
import socket
import socketserver
import ssl
import subprocess
import sys
import threading

args = None
log_lock = threading.Lock()


def log(line):
    with log_lock, open(os.path.join(args.state, "requests.log"), "a") as f:
        f.write(line + "\n")


def make_certs(state):
    ca_key, ca_pem = os.path.join(state, "ca.key"), os.path.join(state, "ca.pem")
    key, csr, crt = (os.path.join(state, n) for n in ("leaf.key", "leaf.csr", "leaf.pem"))
    ext = os.path.join(state, "leaf.ext")
    with open(ext, "w") as f:
        f.write("subjectAltName=DNS:github.com\nbasicConstraints=CA:FALSE\n")
    run = lambda *a: subprocess.run(a, check=True, capture_output=True)
    run("openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", ca_key, "-out", ca_pem,
        "-days", "2", "-subj", "/CN=padwight test CA")
    run("openssl", "req", "-newkey", "rsa:2048", "-nodes", "-keyout", key, "-out", csr, "-subj", "/CN=github.com")
    run("openssl", "x509", "-req", "-in", csr, "-CA", ca_pem, "-CAkey", ca_key, "-CAcreateserial",
        "-out", crt, "-days", "2", "-extfile", ext)
    return ca_pem, crt, key


class Release(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def do_HEAD(self):
        self.answer(head=True)

    def do_GET(self):
        self.answer(head=False)

    def answer(self, head):
        log(f"{self.command} {self.path}")
        for fault in args.fault:
            kind, _, rest = fault.partition(":")
            if kind == "status":
                code, _, substr = rest.partition(":")
                if substr in self.path:
                    return self.reply(int(code), b"", head)
        path = self.path.split("?")[0]
        if path.endswith("/releases/latest"):
            target = path.removesuffix("/latest") + (f"/tag/v{args.latest}" if args.latest else "")
            self.send_response(302)
            self.send_header("Location", f"https://github.com{target}")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        if path.endswith("/releases") or "/releases/tag/" in path:
            return self.reply(200, b"no releases yet", head)
        if "/releases/download/" in path:
            name = os.path.basename(path)
            full = os.path.join(args.dir, name)
            if not args.dir or not os.path.isfile(full):
                return self.reply(404, b"not found", head)
            with open(full, "rb") as f:
                data = f.read()
            for fault in args.fault:
                kind, _, substr = fault.partition(":")
                if kind == "truncate" and name.endswith(substr):
                    self.send_response(200)
                    self.send_header("Content-Length", str(len(data)))
                    self.end_headers()
                    if not head:
                        self.wfile.write(data[: len(data) // 2])
                        self.wfile.flush()
                    self.close_connection = True
                    return
            return self.reply(200, data, head)
        self.reply(404, b"not found", head)

    def reply(self, code, body, head):
        self.send_response(code)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if not head:
            self.wfile.write(body)


class Tunnel(http.server.BaseHTTPRequestHandler):
    """A proxy that accepts CONNECT to any host and connects it to the local TLS server."""

    def log_message(self, *a):
        pass

    def do_CONNECT(self):
        log(f"CONNECT {self.path}")
        upstream = socket.create_connection(("127.0.0.1", args.tls_port))
        self.send_response(200, "Connection established")
        self.end_headers()
        client = self.connection
        try:
            while True:
                ready, _, _ = select.select([client, upstream], [], [], 30)
                if not ready:
                    break
                for s in ready:
                    data = s.recv(65536)
                    if not data:
                        return
                    (upstream if s is client else client).sendall(data)
        except OSError:
            pass
        finally:
            upstream.close()


class Threaded(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True


def main():
    global args
    p = argparse.ArgumentParser()
    p.add_argument("--dir", default="")
    p.add_argument("--latest", default="")
    p.add_argument("--state", required=True)
    p.add_argument("--fault", action="append", default=[])
    args = p.parse_args()
    os.makedirs(args.state, exist_ok=True)
    ca, crt, key = make_certs(args.state)
    tls = Threaded(("127.0.0.1", 0), Release)
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(crt, key)
    tls.socket = ctx.wrap_socket(tls.socket, server_side=True)
    args.tls_port = tls.server_address[1]
    proxy = Threaded(("127.0.0.1", 0), Tunnel)
    for server in (tls, proxy):
        threading.Thread(target=server.serve_forever, daemon=True).start()
    ready = os.path.join(args.state, "ready.json")
    with open(ready + ".tmp", "w") as f:
        json.dump({"proxy": f"http://127.0.0.1:{proxy.server_address[1]}", "ca": ca}, f)
    os.rename(ready + ".tmp", ready)
    try:
        threading.Event().wait()
    except KeyboardInterrupt:
        sys.exit(0)


if __name__ == "__main__":
    main()
