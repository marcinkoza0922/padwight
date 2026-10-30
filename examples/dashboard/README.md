# Dashboard example

A browser dashboard for the Padwight daemon, as an example of a client of the [control protocol](../../docs/protocol.md) written in another language. It shows the daemon's state, the controllers, the last profile switches and the live state of the controller in use, and it can turn remapping on and off, switch, lock and step through profiles, change a few settings, and leave a controller alone.

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
| `GET /api/settings` | `"GetConfig"`, reduced to a few settings |
| `POST /api/settings` `{"rumble_strength": 0.5}` | `"GetConfig"`, the change, then `{"SetConfig": …}` |
| `POST /api/ignore` `{"id": "3537:1079", "ignored": true}` | The same, on `ignored_devices` |

`index.html` is one file of plain JavaScript with no libraries. It polls `/api/status` once a second, as the settings window does, and keeps `/api/input` open for the live view.

It starts by asking for `"Version"` and exits when the daemon's major version isn't 1.

## Keep it private

Anyone who can reach the bridge can change what your buttons do. So it listens on `127.0.0.1` only, forwards just the requests above, requires `Content-Type: application/json` on changes, and refuses a `Host` or `Origin` that isn't its own, which stops other web pages from reaching it. The page never sends a configuration: it sends values for the settings listed in `SETTINGS` in `server.py`, which the bridge checks before it puts them in. Keep those checks if you build on it.

## Changing settings

`change_config` in `server.py` is the whole recipe from the protocol's [Config](../../docs/protocol.md#config) section: `GetConfig`, edit the object it returns, send **all** of it back with `SetConfig`. It sends back everything it read, so settings the dashboard doesn't know about are kept. It isn't atomic: a change made elsewhere between the two requests is lost. To offer another setting, add a line to `SETTINGS` and a field to the page's `SETTINGS`.

`GetConfig` leaves out settings that have their default value, so the bridge reads a missing key as the default, listed beside each setting.

## Going further

Editing profiles works the same way, on `general` and `setups`. A client that has edits it hasn't saved yet should also report [`Unsaved`](../../docs/protocol.md#settings-window-cooperation), so stopping the daemon waits for it. The dashboard saves each change at once, so it has none.
