# Dashboard example

A browser dashboard for the Padwight daemon, as an example of a client of the [control protocol](../../docs/protocol.md) written in another language. It shows the daemon's state, the controllers, the last profile switches and the live state of the controller in use, and it can turn remapping on and off, switch, lock and step through profiles.

```sh
padwight daemon &                          # or the systemd service
python3 examples/dashboard/server.py       # --port 8765 by default
```

Open <http://127.0.0.1:8765/>. Python 3.9 or later; nothing to install.

## How it works

A web page can't open a Unix socket, so `server.py` stands between: it serves `index.html` and turns a few HTTP calls into protocol requests.

| HTTP | Protocol |
|---|---|
| `GET /api/status` | `"Status"` |
| `GET /api/profiles` | `"GetConfig"`, reduced to the profile names |
| `GET /api/input` | `"WatchInput"`, passed on as server-sent events |
| `POST /api/enabled` `{"enabled": true}` | `{"SetEnabled": true}` |
| `POST /api/profile` `{"setup": "Doom", "profile": "Default"}` | `{"Activate": {…}}` |
| `POST /api/lock` `{"locked": true}` | `{"SetProfileLock": true}` |
| `POST /api/next` | `"NextProfile"` |

`index.html` is one file of plain JavaScript with no libraries. It polls `/api/status` once a second, as the settings window does, and keeps `/api/input` open for the live view.

It starts by asking for `"Version"` and exits when the daemon's major version isn't 1.

## Keep it private

Anyone who can reach the bridge can change what your buttons do. So it listens on `127.0.0.1` only, forwards just the requests above (never `SetConfig`), requires `Content-Type: application/json` on changes, and refuses a `Host` or `Origin` that isn't its own, which stops other web pages from reaching it. Keep those checks if you build on it.

## Going further

This only reads and switches. Editing profiles means `GetConfig`, changing the object and sending **all** of it back with `SetConfig`; the protocol page's [Config](../../docs/protocol.md#config) section has the rules. A client that edits should also report [`Unsaved`](../../docs/protocol.md#settings-window-cooperation) so stopping the daemon waits for it.
