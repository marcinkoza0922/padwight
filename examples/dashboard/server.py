#!/usr/bin/env python3
"""A browser dashboard for the Padwight daemon: the bridge between a web page and the control socket.

A web page can't open a Unix socket, so this serves index.html and turns a few HTTP calls into
requests of the protocol in docs/protocol.md. Standard library only.

    python3 examples/dashboard/server.py [--port 8765]

Then open http://127.0.0.1:8765/.

Anyone who can reach this server can change what your buttons do, so it listens on the loopback
address only, forwards just the requests below (never SetConfig), and refuses requests whose Host
or Origin isn't itself, which keeps other web pages from reaching it (DNS rebinding, CSRF).
"""

import argparse
import json
import os
import socket
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

PAGE = Path(__file__).with_name("index.html")


def socket_path():
    runtime = os.environ.get("XDG_RUNTIME_DIR")
    if runtime:
        return os.path.join(runtime, "padwight.sock")
    return f"/tmp/padwight-{os.getuid()}/padwight.sock"


def connect():
    s = socket.socket(socket.AF_UNIX)
    s.settimeout(5)  # the daemon answers within 2 seconds
    s.connect(socket_path())
    return s


def request(req):
    """One request, one reply. Raises RuntimeError with the daemon's message on {"Error": ...}."""
    with connect() as s:
        s.sendall(json.dumps(req).encode() + b"\n")
        line = s.makefile().readline()
    if not line:
        raise RuntimeError("the daemon closed the connection without replying")
    reply = json.loads(line)
    if isinstance(reply, dict) and "Error" in reply:
        raise RuntimeError(reply["Error"])
    return reply


def profiles(config):
    """Every profile as {"setup": name or None, "profile": name}, General first."""
    found = [{"setup": None, "profile": p["name"]} for p in config["general"]["profiles"]]
    for game in config["setups"]:
        found += [{"setup": game["name"], "profile": p["name"]} for p in game["profiles"]]
    return found


def activate(body):
    ref = {"profile": str(body["profile"])}
    if body.get("setup"):  # General has no "setup" key
        ref["setup"] = str(body["setup"])
    return {"Activate": ref}


# What the page may ask for. Each takes the page's JSON body and returns the daemon's request.
ACTIONS = {
    "enabled": lambda body: {"SetEnabled": bool(body["enabled"])},
    "lock": lambda body: {"SetProfileLock": bool(body["locked"])},
    "profile": activate,
    "next": lambda body: "NextProfile",
}


class Handler(BaseHTTPRequestHandler):
    server_version = "padwight-dashboard"

    def log_message(self, fmt, *args):
        pass

    def reply(self, code, body, kind="application/json"):
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def allowed(self):
        """Only this server's own page may ask: its Host, and its Origin when the browser sends one."""
        host = self.headers.get("Host", "")
        if host not in self.server.hosts:
            self.reply(403, {"Error": "unexpected Host"})
            return False
        origin = self.headers.get("Origin")
        if origin is not None and origin != f"http://{host}":
            self.reply(403, {"Error": "unexpected Origin"})
            return False
        return True

    def do_GET(self):
        if not self.allowed():
            return
        try:
            if self.path == "/":
                self.reply(200, PAGE.read_bytes(), "text/html; charset=utf-8")
            elif self.path == "/api/status":
                self.reply(200, request("Status")["Status"])
            elif self.path == "/api/profiles":
                self.reply(200, profiles(request("GetConfig")["Config"]))
            elif self.path == "/api/input":
                self.stream_input()
            else:
                self.reply(404, {"Error": "not found"})
        except (OSError, RuntimeError, ValueError) as e:
            self.reply(502, {"Error": f"{e}"})

    def do_POST(self):
        if not self.allowed():
            return
        # A page on another site can't send this content type without asking permission first.
        if self.headers.get("Content-Type", "").split(";")[0] != "application/json":
            return self.reply(415, {"Error": "send application/json"})
        action = ACTIONS.get(self.path.removeprefix("/api/"))
        if action is None:
            return self.reply(404, {"Error": "not found"})
        try:
            body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
            request(action(body))
            self.reply(200, {"ok": True})
        except (KeyError, TypeError, ValueError) as e:
            self.reply(400, {"Error": f"bad body: {e}"})
        except (OSError, RuntimeError) as e:
            self.reply(502, {"Error": f"{e}"})

    def stream_input(self):
        """WatchInput as server-sent events: the daemon's lines, passed on as they come."""
        with connect() as s:
            s.settimeout(None)
            s.sendall(b'"WatchInput"\n')
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            try:
                for line in s.makefile():
                    self.wfile.write(f"data: {line.strip()}\n\n".encode())
                    self.wfile.flush()
            except OSError:
                pass  # the page went away


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--port", type=int, default=8765)
    port = parser.parse_args().port

    try:
        version = request("Version")["Version"]
    except (OSError, RuntimeError, ValueError, KeyError) as e:
        sys.exit(f"No Padwight daemon at {socket_path()}: {e}\nStart it with `padwight daemon`.")
    if version["protocol_major"] != 1:
        sys.exit(f"The daemon speaks protocol {version['protocol_major']}.{version['protocol_minor']}; this dashboard is for 1.x.")

    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    server.daemon_threads = True
    server.hosts = {f"127.0.0.1:{port}", f"localhost:{port}"}
    print(f"Padwight {version['app']}, protocol {version['protocol_major']}.{version['protocol_minor']}")
    print(f"Dashboard on http://127.0.0.1:{port}/ (Ctrl-C to stop)")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
