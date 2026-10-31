# Control protocol (version 1.0)

The Padwight daemon (`padwight daemon`) is controlled over a local socket. The settings window, the tray and the command line (`padwight status`, `padwight profile <name>`, …) are all clients of it, and so can be a program you write, in any language that can open a Unix socket and read JSON. [examples/dashboard/](../examples/dashboard/) is a small one: a web page with a tiny Python bridge.

This page is the protocol's specification. The Rust types behind it are in [src/ipc.rs](../src/ipc.rs), and tests there fail if the wire format changes.

## Versions

The protocol has a version, `major.minor`, and this page describes **1.0**. Ask the daemon for its version first with [`Version`](#reading).

- Within one major version, requests and fields are only ever **added**. Nothing is removed, renamed or changes meaning.
- A minor version is a release that added something. A client written for 1.0 works with a 1.3 daemon.
- A client must **ignore fields it doesn't know** in replies and streamed lines, and must cope with **enum values it doesn't know** (a new `FocusBackend`, a new `Button`): show them as they are rather than failing.
- A client written for 1.3 can find it is talking to a 1.0 daemon. Check `protocol_minor` before sending a request that came later. A daemon that doesn't know a request answers `{"Error":"bad request: …"}`.
- A new major version may change anything. A client should refuse to run against a major version it doesn't know.

What the version covers: everything on this page except what is marked *internal* or *debug*. Those parts exist for the app's own windows and for testing; they can change in any release.

## Transport

| | |
|---|---|
| Socket | A Unix stream socket named `padwight.sock` in `$XDG_RUNTIME_DIR` (`/run/user/1000/` on most systems). Without a runtime directory, `/tmp/padwight-<uid>/padwight.sock`. |
| Access | Mode `0600`: only your own user can connect. There is no other authentication. |
| Encoding | JSON, UTF-8, one value per line (`\n`). |
| Requests | **One request per connection.** Connect, write one line, read one line, close. |
| Watching | The `Watch…` requests are the exception: the connection stays open and the daemon writes one JSON line per update until you close it. |

Timing:

- Send the request line within **2 seconds** of connecting, or the daemon hangs up.
- The daemon answers within **2 seconds**. If its event loop is busy it replies `{"Error":"the daemon is busy and didn't answer in time; the change may still be made"}`; the request stays queued and may still happen. Give your own read timeout a little more than 2 seconds so you hear that.
- A watcher that doesn't read for **10 seconds** is dropped. Reconnect when a stream ends: it ends when the daemon restarts.

A request the daemon can't read gets `{"Error":"bad request: <why>"}`, and any failure is a reply of the form `{"Error":"<message>"}`. The messages are for people; don't match on them.

## JSON shapes

The JSON is what Rust's serde produces by default:

| Rust | JSON | Example |
|---|---|---|
| Unit variant | The name as a string | `"Status"` |
| Variant with one value | `{"Name": value}` | `{"SetEnabled":true}` |
| Variant with named fields | `{"Name": {fields…}}` | `{"SetNintendoLayout":{"id":"3537:1079","on":true}}` |
| `Option<T>` | The value, or `null` | `{"WatchFeed":null}` |
| Struct | An object. Fields with their default value may be missing. | |

Numbers are plain JSON numbers (sticks `-1.0` to `1.0`, with **Y positive = down**; triggers `0.0` to `1.0`).

## Requests

Every request below is a JSON value you send. The reply is `"Ok"` unless the table says otherwise, or `{"Error": "…"}` when it failed.

### Reading

| Request | Reply | |
|---|---|---|
| `"Version"` | `{"Version": {"protocol_major":1, "protocol_minor":0, "app":"0.1.0"}}` | The protocol version, and the app build (for bug reports, not for checking compatibility). |
| `"Status"` | `{"Status": `[Status](#status)`}` | Everything about now. Cheap; polling it about once a second is what the settings window does. |
| `"GetConfig"` | `{"Config": `[Config](#config)`}` | The whole configuration. |
| `"WatchInput"` | A [stream](#watchinput) | The live state of the controller used last. |

### Changing the daemon's state

These take effect at once and last until the daemon stops, unless noted.

| Request | |
|---|---|
| `{"SetEnabled": true}` | Turn remapping on or off. Saved to the config. |
| `{"SetProfile": "Doom"}` | Switch to the profile with this name (the first one found; fails if there is none). Anything that switches profiles (rules, the default profile) may switch away again, unless it is locked. |
| `{"Activate": {"setup": "Doom", "profile": "Default"}}` | Switch to exactly this profile. `setup` is the game's setup name; leave it out for the General setup. |
| `"NextProfile"` | Switch to the next profile of the active setup. |
| `{"SetProfileLock": true}` | Lock the active profile so rules and the default profile can't switch it away. Not saved; any switch of profile clears it. |
| `"Reload"` | Read the config files from disk again (after editing them by hand). |
| `"ToggleOverlay"`, `"ToggleNumpad"` | Open or close the on-screen keyboard or numpad. |
| `{"OpenMenu": "Name"}` | Show the menu with this name on the controller used last. |
| `"CloseOverlays"` | Close every overlay that has the controller. |

### Changing the configuration

| Request | |
|---|---|
| `{"SetConfig": `[Config](#config)`}` | Replace the profiles and settings with this configuration, and save it. See [Config](#config) for what the daemon keeps of its own. |
| `{"SetController": `[ControllerInfo](#controllerinfo)`}` | Change a remembered controller's nickname, color, player number and pass-through. Fails when the controller isn't remembered. The other fields are ignored. |
| `{"ForgetController": "<id>"}` | Forget a remembered controller; on its next connection it counts as new. |
| `{"SetNintendoLayout": {"id": "<id>", "on": true}}` | Turn the Nintendo button layout on or off for a controller. |
| `{"ActivateFor": {"id": "<id>", "profile": "Default"}}` | Make a profile of the active setup the one this controller uses. |

A controller's `id` is the `id` field of its entry in `Status.controllers` or `Status.devices`.

### Testing a controller

| Request | |
|---|---|
| `{"TestRumble": "/dev/input/event10"}` | A short rumble pattern, on the controller at this `path` (from `Status.devices`). Fails when it has no rumble. |
| `{"CalibrateGyro": "/dev/input/event10"}` | Measure the gyro's drift for a moment; keep the controller still. Fails when it has no gyro or isn't managed. |
| `{"TestTurn": 400}` | After 3 seconds, move the mouse this many pixels to the right over a second, to see how far it turns a game's camera. |

### Settings window cooperation

A client that edits the configuration should tell the daemon when it has changes it hasn't saved, because stopping the daemon (a signal, or Quit in the tray) then **waits** for the client to save or discard them.

| Request | |
|---|---|
| `{"Unsaved": true}` | You hold unsaved edits. Send it about once a second while you do: after **3 seconds** without it the daemon decides you've gone away. Send `{"Unsaved": false}` (or just stop) when you have none. |
| `"CancelStop"` | You keep your unsaved edits: a stop the daemon is waiting on is dropped, and it keeps running. |

`Status.stop_pending` is `true` while the daemon waits. Then show the user a question (save, discard, cancel), act on the answer, and send `{"Unsaved": false}` or `"CancelStop"`.

### Internal and debug requests

Not covered by the version; they can change in any release.

- `"WatchOverlay"`, `{"WatchFeed": …}`: the streams of what the overlay window and the Overview page draw. Their lines are drawing instructions in the app's own renderer types.
- `DebugAttach`, `DebugInput`, `DebugOutput`, `DebugDetach`, `DebugIdentify`: debug mode only (the daemon must have been started with `--debug`); they add and drive controllers no hardware backs.

## Replies in detail

### Status

```json
{"Status": {
  "enabled": true,
  "active_profile": "Gamepad",
  "active_game": null,
  "active_layers": [],
  "devices": [ … ],
  "controllers": [ … ],
  "focus_backend": "Kwin",
  "focused": {"class": "kitty", "title": "…", "pid": 24750, "exe": "kitty", "path": "/usr/bin/kitty", "steam_app_id": null},
  "recent_windows": [ … ],
  "motion_access_denied": [],
  "raw_readers": [],
  "overlay_visible": false,
  "numpad_visible": false,
  "stop_pending": false,
  "profile_locked": false,
  "switches": [ … ]
}}
```

| Field | Meaning |
|---|---|
| `enabled` | Remapping is on. |
| `active_profile`, `active_game` | The profile in use and the game (setup) it belongs to. `active_game` is `null` for General. |
| `active_layers` | Layers held right now, oldest first. |
| `active_mode` | The mode switched to; missing in the default mode. |
| `devices` | The controllers connected. Each has `name`, `id`, `path`, `profile` (the one it uses; empty if unmanaged), `managed` (grabbed and remapped), `ignored`, and the features `analog_triggers`, `rumble`, `gyro`, `paddles`, `touchpad`, plus `family` and `model` (strings when the model is recognized, else `null`). |
| `controllers` | Every controller remembered, connected or not: [ControllerInfo](#controllerinfo). |
| `focus_backend` | How the daemon learns the active game: `Kwin`, `GnomeShell`, `Cinnamon`, `Cosmic`, `Sway`, `Hyprland`, `Wlroots` or `ProcessScan`. |
| `focused`, `recent_windows` | The focused window, and recent ones newest first. Each has `class`, `title`, `pid`, `exe`, `path`, `steam_app_id`. Window titles can be private; show them with care. `focused` is `null` when unknown. |
| `motion_access_denied`, `raw_readers` | Problems to show the user: motion sensors the daemon may not open, and other programs reading a managed controller directly (`{"controller", "programs"}`). |
| `overlay_visible`, `numpad_visible` | The on-screen keyboard or numpad is up. |
| `stop_pending` | The daemon is ending and waits for [`Unsaved`](#settings-window-cooperation) edits. |
| `profile_locked` | The active profile is [locked](#changing-the-daemons-state). |
| `switches` | Recent profile switches, newest first: `{"age_ms", "game", "profile", "why"}`, `why` being a sentence such as `Doom (doom.exe) matches Executable “doom.exe”`. |

### ControllerInfo

```json
{"id": "3537:1079", "name": "Generic X-Box pad", "nickname": "", "color": null, "player": null,
 "pass_through": false, "nintendo_layout": false, "pending": false, "connected": true}
```

`nickname` is what the user calls it; show it, else `name`, else `id`. `pending`: its set-up question is still unanswered. `pass_through`: the daemon leaves it alone.

### Config

`GetConfig` returns the whole configuration as one JSON object, and `SetConfig` takes one back. It is the same data as the config folder's `config.toml` and the setup files beside it, with the fields named the same way.

The top level, in the fields a client is most likely to read:

| Field | Meaning |
|---|---|
| `enabled` | Remapping is on (set it with `SetEnabled`, not here). |
| `active` | The profile in use, as `{"setup": "Doom", "profile": "Default"}` (`setup` missing: General). |
| `general` | The General setup: a game object. |
| `setups` | The games' setups: an array of game objects. |
| `ignored_devices`, `auto_switch`, `controllers`, and the overlay styles | Settings. |

A game object has `name`, `rules` (how its window is recognized), `profiles` (an array, each with a `name` and the buttons, sticks, triggers and gyro), and its macros, menus and overlays. Profile names are what `SetProfile` and `Activate` take:

```python
names = [(None, p["name"]) for p in config["general"]["profiles"]] + \
        [(g["name"], p["name"]) for g in config["setups"] for p in g["profiles"]]
```

The rest of the shape is defined by the types in [src/config.rs](../src/config.rs) and described by the [pack format](pack-format.md), whose profile data it shares. It is large and grows with each feature, which is allowed in 1.x (fields are only added), so a client that edits a configuration must follow two rules:

1. **Read, change, write back.** Take the object from `GetConfig`, change the part you edit, and send **all** of it back. The daemon builds its configuration from what you send; anything you leave out goes back to its default. Don't build a configuration from scratch.
2. **Keep fields you don't understand** as they are, for the same reason.

`GetConfig` leaves out a setting that has its default value, so read a missing key as the default. The first `SetConfig` after the daemon starts can fill in what was unset (an empty `active.profile` becomes the profile in use); the configuration is otherwise handed back as it was sent.

The daemon refuses a configuration it finds a problem with (`{"Error":"not saved: …"}`, such as a General setup with no profile) and keeps what it has. For what it accepts, it keeps these of its own, whatever you send: `enabled`, the active profile, the remembered `controllers` (change those with `SetController` and the like), and the list of files it couldn't read.

#### Writing the config files instead

A client may also edit the config folder directly, and with no daemon running that is the only way. The folder is `~/.config/padwight/` (`$XDG_CONFIG_HOME` can name another). `config.toml` holds the app's settings, and `general/` and `setups/<name>/` hold the profiles, macros, menus and so on, one TOML file each. [src/config/store.rs](../src/config/store.rs) describes the layout.

- With the daemon running, send `"Reload"` after writing, so it reads the folder again. Without it the daemon keeps its own copy, and its next save **rewrites the whole folder** over your changes.
- A file that doesn't load is left out of the configuration and kept on disk; `Reload` answers `{"Error": …}` when the folder as a whole can't be read.
- The version of this protocol covers the socket only. The files are the app's own storage, and their layout can change between releases without a new protocol version. Prefer `SetConfig` when the daemon is up.

### WatchInput

Send `"WatchInput"` and keep reading. The daemon writes a line for each change, **at most about 60 a second** (a burst is merged into the latest state). The first line comes at once and is the current state. Each line is `null`, meaning no controller is active, or a snapshot of the one used last:

```json
{"device": "Generic X-Box pad", "model": null, "family": null,
 "buttons": ["South", "LeftBumper"],
 "left_stick": [0.5, -1.0], "right_stick": [0.0, 0.0],
 "left_trigger": 0.25, "right_trigger": 0.0}
```

This is the controller's **physical** state, before any mapping. `buttons` are the ones held. Where present, `gyro` is `[pitch, yaw, roll]` in degrees per second, `touches` is `[x, y]` for each finger on the touchpad (0 to 1 from its top left corner) and `touch_click` says it is clicked; `gyro_out` is what the gyro does under the active profile.

Button names: `South`, `East`, `North`, `West` (the face buttons by position, so `South` is A on an Xbox pad and ✕ on a PlayStation pad), `LeftBumper`, `RightBumper`, `Select`, `Start`, `Guide`, `LeftStick`, `RightStick`, `DpadUp`, `DpadDown`, `DpadLeft`, `DpadRight`, the four paddles `LeftPaddle`, `RightPaddle`, `LeftPaddle2`, `RightPaddle2`, and a stick pushed far enough counts as a button: `LeftStickUp` … `RightStickRight`. More may come.

## Trying it

With the daemon running, `socat` is enough:

```sh
echo '"Version"' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/padwight.sock
echo '"Status"'  | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/padwight.sock
echo '"WatchInput"' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/padwight.sock   # Ctrl-C to stop
```

A client in Python needs only the standard library:

```python
import json, os, socket

def request(req):
    with socket.socket(socket.AF_UNIX) as s:
        s.settimeout(5)
        s.connect(os.environ["XDG_RUNTIME_DIR"] + "/padwight.sock")
        s.sendall(json.dumps(req).encode() + b"\n")
        reply = json.loads(s.makefile().readline())
    if isinstance(reply, dict) and "Error" in reply:
        raise RuntimeError(reply["Error"])
    return reply

print(request("Version"))
print(request({"SetProfileLock": True}))
```

## Security

Anyone who can connect can change what every button does, including making a button type text into a window, so the socket is private to your user (mode `0600`) and you should keep it that way. A program that exposes the protocol on a network port, or to web pages, **is** giving that power away: restrict what it forwards, bind to the loopback address only, and check the `Host` and `Origin` headers against web pages that try to reach it from elsewhere (DNS rebinding). The example dashboard does all of these, and forwards only a handful of requests.
