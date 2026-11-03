#!/usr/bin/env python3
"""Runs examples/dashboard/server.py's socket client, and its HTTP bridge, against a live daemon.

Called by tests/dashboard_client.rs with XDG_RUNTIME_DIR pointing at a throwaway daemon's socket.
Checks what docs/protocol.md promises a client in another language: Version, Status and GetConfig
replies have the documented shape, `WatchInput` streams one JSON line (`null` or a snapshot) at
once, and the dashboard's own endpoints pass those on. Prints "ok" on success; any failure raises.
"""

import http.client
import importlib.util
import json
import sys
import threading
from http.server import ThreadingHTTPServer
from pathlib import Path

SERVER = Path(__file__).resolve().parents[2] / "examples/dashboard/server.py"
spec = importlib.util.spec_from_file_location("dashboard_server", SERVER)
server = importlib.util.module_from_spec(spec)
spec.loader.exec_module(server)

STATUS_FIELDS = {
    "enabled": bool, "active_profile": str, "active_layers": list, "devices": list, "controllers": list,
    "focus_backend": str, "recent_windows": list, "overlay_visible": bool, "numpad_visible": bool,
    "stop_pending": bool, "profile_locked": bool, "switches": list,
}
SNAPSHOT_FIELDS = ["device", "buttons", "left_stick", "right_stick", "left_trigger", "right_trigger"]


def check(condition, what):
    if not condition:
        raise AssertionError(what)


def check_snapshot(line):
    check(line is None or all(f in line for f in SNAPSHOT_FIELDS), f"WatchInput line is neither null nor a snapshot: {line!r}")


def watch_lines(count):
    """Opens WatchInput the way the protocol describes and reads up to `count` lines (with no
    controller the daemon sends one `null` and then nothing, so only the first is required)."""
    with server.connect() as s:
        s.settimeout(1)
        s.sendall(b'"WatchInput"\n')
        reader = s.makefile()
        lines = [json.loads(reader.readline())]
        for _ in range(count - 1):
            try:
                lines.append(json.loads(reader.readline()))
            except (TimeoutError, OSError):
                break
        return lines


def main():
    version = server.request("Version")["Version"]
    check(version["protocol_major"] == 1 and isinstance(version["protocol_minor"], int), f"Version: {version}")
    check(isinstance(version["app"], str) and version["app"], f"Version: {version}")

    status = server.request("Status")["Status"]
    for field, kind in STATUS_FIELDS.items():
        check(isinstance(status.get(field), kind), f"Status.{field} should be {kind.__name__}: {status.get(field)!r}")
    check(status["enabled"] is True, "a fresh daemon has remapping on")

    config = server.request("GetConfig")["Config"]
    check(config["general"]["profiles"], "GetConfig: General has a profile")
    first = {"setup": None, "profile": config["general"]["profiles"][0]["name"]}
    check(server.profiles(config)[0] == first, "profiles()")

    try:
        server.request({"SetProfile": "no such profile"})
    except RuntimeError as e:
        check("no profile" in str(e), f"error message: {e}")
    else:
        raise AssertionError("SetProfile of a missing profile should be an {Error}")
    try:
        server.request({"NoSuchRequest": 1})
    except RuntimeError as e:
        check(str(e).startswith("bad request"), f"unknown request: {e}")
    else:
        raise AssertionError("an unknown request should be an {Error}")

    for line in watch_lines(3):
        check_snapshot(line)

    # The dashboard's own HTTP endpoints, on a throwaway port.
    http_server = ThreadingHTTPServer(("127.0.0.1", 0), server.Handler)
    http_server.daemon_threads = True
    port = http_server.server_address[1]
    http_server.hosts = {f"127.0.0.1:{port}"}
    threading.Thread(target=http_server.serve_forever, daemon=True).start()
    try:
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        conn.request("GET", "/api/status")
        reply = conn.getresponse()
        check(reply.status == 200 and json.loads(reply.read())["active_profile"] == status["active_profile"], "GET /api/status")

        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        conn.request("GET", "/api/input")
        reply = conn.getresponse()
        check(reply.status == 200 and reply.getheader("Content-Type") == "text/event-stream", "GET /api/input")
        line = reply.fp.readline().decode()
        check(line.startswith("data: "), f"first SSE line: {line!r}")
        check_snapshot(json.loads(line[len("data: "):]))
        conn.close()

        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        conn.request("GET", "/", headers={"Host": "evil.example"})
        check(conn.getresponse().status == 403, "a foreign Host is refused")
    finally:
        http_server.shutdown()
    print("ok")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"FAILED: {e!r}", file=sys.stderr)
        sys.exit(1)
