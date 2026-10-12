# `.padpack` format reference

A pack is one game in one file: TOML, in the same shapes the app writes to `config.toml`. This page describes **format 17**, the format the current app writes. For a walkthrough of building one, see [tutorial-deus-ex-pack.md](tutorial-deus-ex-pack.md). The shipped examples are in `packs/`.

The easiest way to write a pack is in the app, then **Details → Export…**. Read this page when you want to edit the file by hand, check a value, or understand what the export produced.

## Top level

```toml
format = 10                      # pack format version (required)
macros = []                      # optional, see Macros
overlay_font = "Rajdhani"        # optional, font for overlays, menus and keyboards
controllers = { dualsense = true } # optional, other controllers the game supports (see Controller identity)

[pack]                           # required
id = "744efa38-…"                # required, stable across versions
name = "Deus Ex"                 # required, becomes the game's name
version = "1.1"
author = "Padwight"
description = "…"
made_with = "Xbox controller"

[[rules]]                        # auto-switch rules (see Rules)
[[profiles]]                     # required, at least one
[[macros]]                       # optional
[[menus]]                        # optional
[[info_overlays]]                # optional
[[modes]]                        # optional
[[layers]]                       # optional
[keyboard_style]                 # optional, on-screen keyboard look
[numpad_style]                   # optional, on-screen numpad look
[media_style]                    # optional, media controls look
[menu_style]                     # optional, in-game menu and Edit Controls look
[motion]                         # optional, how each kind of overlay moves
```

Unknown top-level keys are an error, so a typo fails the import rather than being ignored. `log_overlays` is also accepted.

### `[pack]`

| Key | Meaning |
|---|---|
| `id` | A UUID. Keeps the game recognised across versions; a re-export keeps it, and a fork gets a new one. |
| `name` | The game name. |
| `version` | Free text, compared as versions (`1.2` is newer than `1.1`). |
| `author`, `description`, `made_with` | Shown in the import preview. `made_with` is informational, e.g. `"Xbox controller"`. |
| `based_on` | Only on forks: a table with `id`, `name`, `author`, `version` of the original. |

### Rules

Rules switch the game's profile when a matching window gets focus.

```toml
[[rules]]
kind = "executable"       # "executable" | "steam_app_id" | "window_class"
value = "DeusEx.exe"
profile = "Deus Ex"       # must be a profile of this pack
```

- `executable`: the program's file name. For Wine/Proton games this is the Windows `.exe`.
- `steam_app_id`: the Steam App ID, as a string.
- `window_class`: the window's class.
- `enabled = false` turns a rule off without deleting it. It is written only when false.

## Profiles

```toml
[[profiles]]
name = "Deus Ex"
combos = []
combo_window_ms = 120    # default 60
tap_window_ms = 250      # default 250: longest gap that still continues a double/triple tap
long_press_ms = 500      # default 500: hold time for a long press
requires = []            # optional, e.g. ["gyro"]; see Requirements

[profiles.buttons]       # one entry per button; unset buttons do nothing
South = "disabled"
```

Buttons are: `South`, `East`, `North`, `West` (the face buttons, A/B/Y/X or ✕/○/△/□), `LeftBumper`, `RightBumper`, `LeftStick`, `RightStick`, `Select`, `Start`, `Guide`, `DpadUp`, `DpadDown`, `DpadLeft`, `DpadRight`. The stick directions are also buttons: `LeftStickUp`, `LeftStickDown`, `LeftStickLeft`, `LeftStickRight`, and the same with `RightStick…`. Triggers are not buttons; they have their own section below.

### Actions

A button's value is an **action**. Each form is one TOML key:

| Form | Example | What it does |
|---|---|---|
| `"disabled"` | `DpadUp = "disabled"` | Nothing. Also what an unset button does. |
| `keys` | `keys = ["KEY_LEFTCTRL", "KEY_C"]` | Presses the evdev keys together, releases together. |
| `mouse` | `mouse = "Right"` | A mouse button: `Left`, `Right`, `Middle`, `Back`, `Forward`. |
| `wheel` | `wheel = "Up"` | One wheel notch per press, and keeps scrolling while held. `Up`, `Down`, `Left`, `Right`. |
| `gamepad` | `gamepad = "Guide"` | Sends a gamepad button to the virtual pad, with any button name above. |
| `"toggle_overlay"`, `"toggle_numpad"` | | Opens or closes the on-screen keyboard or numpad. |
| `"screenshot"`, `"toggle_recording"`, `"toggle_media"`, `"force_quit"` | | The Guide-layer actions: screenshot, start or stop recording, media controls, force-quit the focused game. |
| `multi` | `multi = [{ layer = "Hotkeys" }, { show_info = "Controls" }]` | Several actions at once, pressed in order and released in reverse. |
| `toggle` | `[profiles.buttons.LeftStick.toggle]` `keys = ["KEY_X"]` | First press holds the inner action, next press releases it. |
| `turbo` | `turbo = { action = { keys = ["KEY_F"] }, rate = 10.0 }` | Repeats the inner action 2–30 times a second while held. |
| `macro` | `macro = { name = "QCF", repeat = false }` | Plays a macro. `repeat = true` loops while held. |
| `layer` | `layer = "Hotkeys"` | Holds a layer on while the input is held. Can't be inside `toggle`, or in a menu item. |
| `mode` | `mode = "Driving"` | Switches to a mode, or back to the default mode if that mode is on. `"Mouse mode"` names the built-in mouse mode. |
| `show_info` | `show_info = "Controls"` | Shows an info overlay while held. |
| `show_log` | `show_log = "Inputs"` | Shows a log overlay while held. |
| `open_menu` | `open_menu = "Belt"` | Opens a menu while held. |

Toggle with a start state: `toggle = { action = { keys = ["KEY_X"] }, start_on = true }`. Without `start_on`, a toggle starts off and is written as the inner action only.

Gestures and combos are attached to a button:

```toml
[profiles.gestures.LeftBumper.double_tap]
keys = ["KEY_BACKSPACE"]

[profiles.gestures.Start.long_press]
keys = ["KEY_KPSLASH"]

[[profiles.combos]]        # buttons pressed together act as their own input
buttons = ["LeftBumper", "RightBumper"]
action = { keys = ["KEY_LEFTALT", "KEY_TAB"] }
```

- Gesture kinds: `double_tap`, `triple_tap`, `long_press`.
- A combo needs at least two buttons. Members wait `combo_window_ms` for the rest of the combo before acting alone. A member whose own action is `"disabled"` works as a modifier with no time limit.

Key names are evdev names (`KEY_A`, `KEY_LEFTSHIFT`, `KEY_KPPLUS`, `KEY_RIGHTBRACE`, `KEY_APOSTROPHE`, …). The keyboard overlay in the app lists them.

## Sticks

```toml
[profiles.left_stick]
deadzone = 0.12          # radial deadzone, 0.0–1.0
curve = 1.0              # response exponent for mouse and scroll: 1.0 linear, higher = finer at small pushes (default 2.0)
key_threshold = 0.25     # push at which direction keys and direction buttons press (default 0.5)

[profiles.left_stick.action.keys]
up = "KEY_W"
down = "KEY_S"
left = "KEY_A"
right = "KEY_D"
```

The `action` table picks what the stick does:

| Form | Keys | Meaning |
|---|---|---|
| `disabled` | | Nothing. |
| `gamepad` | `gamepad = { stick = "Left", invert_y = false }` | Feeds a virtual-pad stick. |
| `mouse` | `mouse = { speed = 1600.0 }` | Pointer movement; `speed` is pixels per second at full push. Optional: `accel`, `accel_ramp_ms`, `y_scale`, `outer_boost`, `smoothing_ms`, `invert_y`. |
| `scroll` | `scroll = { speed = 4.0, invert_y = false }` | Wheel notches per second at full push. `invert_y` flips the direction (stick up scrolls down). |
| `keys` | the table above | Direction keys (WASD, arrows). |
| `ring` | `ring = { sectors = 8, start_angle = 0.0, inner_radius = …, hysteresis = …, actions = [...] }` | The stick's angle picks one of `sectors` slices, and that slice's action is held. |
| `flick` | `flick = { full_turn_px = 8000.0, flick_threshold = 0.9, flick_time_ms = 100, rotate_smoothing_ms = 0, forward_deadzone = 0.0, vertical = "off", vertical_speed = 1200.0 }` | Flick stick for turning the camera. `vertical = "look"` adds up and down. |

**Zones** hold an extra action while the stick is within part of its travel:

```toml
[[profiles.left_stick.zones]]
min = 0.0        # 0.0–1.0 of travel
max = 0.6
action = { keys = ["KEY_LEFTSHIFT"] }
```

Triggers take zones too, under `[[profiles.left_trigger.zones]]`. The app hides zones for controllers whose triggers are on/off only.

## Triggers

```toml
[profiles.right_trigger.action.button]
threshold = 0.3                  # pull (0.0–1.0) that counts as pressed
action = { mouse = "Left" }      # any action a button can have
```

Or pass the trigger through as an analog axis: `[profiles.right_trigger.action] gamepad = "Right"` (`"Left"` or `"Right"`), or `"disabled"`.

## Gyro

Only meaningful for controllers with motion sensors. Written under `[profiles.gyro]`:

```toml
[profiles.gyro]
horizontal = "player_space" # "yaw" (turn) | "roll" (tilt) | "yaw_and_roll" | "player_space" | "world_space"
invert_x = false
invert_y = false
axes = "both"             # optional: "both" | "horizontal" | "vertical"
x_scale = 1.0             # optional: horizontal sensitivity, times the mode's
y_scale = 0.8             # optional: vertical sensitivity, times the mode's
noise_threshold = 1.0     # rotation slower than this (degrees/second) is scaled down
smoothing_threshold = 8.0 # optional: rotation slower than this (degrees/second) is smoothed; 0 is off
smoothing_ms = 125        # optional: over this long
pause_with_stick = "Right" # optional: gyro pauses while this stick is pushed
auto_calibrate = false    # optional: re-measure drift whenever the controller is set down still
toggle_rumble = false     # optional: a short rumble when a toggle switches gyro on or off

[profiles.gyro.mode.mouse]          # or: "off", stick = {...}, position = {...}, steering = {...}
sensitivity = 15.0                  # pixels per degree turned

[profiles.gyro.curve]               # optional acceleration curve (mouse and stick modes)
slow_gain = 0.5                     # sensitivity, times the mode's, up to slow_speed
fast_gain = 2.0                     # and from fast_speed
slow_speed = 5.0                    # degrees/second
fast_speed = 40.0
power = 1.0                         # shape of the blend between: 1 is straight

[profiles.gyro.activation.unless_held]
button = "DpadUp"                   # or: "always", while_held, toggle
```

- `mode`: `"off"`, `mouse = { sensitivity }`, `stick = { stick, full_rate, anti_deadzone }` (rotation speed deflects a virtual stick), `position = { stick, max_angle, anti_deadzone, lock_at_edge }` (how far the controller has turned deflects a virtual stick; `max_angle` degrees is full, and with `lock_at_edge` turning further is let go, so turning back leaves the edge at once), or `steering = { stick, max_angle }` (tilt steers a stick).
- `horizontal`: which turning aims left and right. `player_space` and `world_space` use gravity, so turning left aims left however the controller is held; `world_space` also reads up and down against the horizon.
- `full_turn_px` (optional, mouse mode): mouse pixels that turn the game a full circle. With it, `sensitivity` is real-world: 1.0 turns the game as far as the controller turned.
- `curve`: the app's presets are Linear (no curve), Gentle, Aggressive, Wide, Extra wide, Precision and Turn boost.
- `activation`: `"always"`, `while_held = { button = … }`, `unless_held = { button = … }` (a clutch: off while held), or `toggle = { button = … }`. The input can also be `left_trigger` or `right_trigger`, as in `while_held = "left_trigger"`.
- `recenter` (optional): an input that sets the current direction as straight ahead (steering and position modes) and clears leftover motion.

The touchpad's mouse motion takes the same `curve`, with speeds in pad widths/second (`[profiles.touchpad.motion.mouse.curve]`). A finger's pad widths/second feel about like 40 times as many degrees/second of gyro.

Everything in this section beyond `mode`, `horizontal`, the inverts, `noise_threshold`, `activation` and `recenter` is format 15; older app versions can't read packs that use the new modes or spaces.

## Touchpad

DualShock 4 and DualSense touchpads. Left out, the pad works as a laptop trackpad: mouse motion at 1200 pixels across the pad, and the click is the left mouse button.

```toml
[profiles.touchpad]
click = "disabled"                  # what pressing the pad does, like any button

[profiles.touchpad.motion.menu]     # or: "off", mouse = {...}, joystick = {...}, swipe = {...}
menu = "Build"                      # the menu the pad is divided into
choose = "click"                    # or "lift"
overlay = true                      # show the menu while a finger is on the pad

[profiles.touchpad.right]           # optional: splits the pad, this is its right half
click = { mouse = "Left" }

[profiles.touchpad.right.motion.mouse]
speed = 1200.0                      # pixels across the whole pad
tap = true                          # a quick tap runs the click
scroll = true                       # two fingers scroll
inertia = true                      # a flick keeps gliding
```

- `motion`: what fingers do on the whole pad, or on its left half when `right` is set.
  - `"off"`.
  - `mouse = { speed, curve, tap, scroll, inertia }`. `curve` is as for gyro, in pad widths/second; `tap`, `scroll` and `inertia` default to off.
  - `joystick = { stick, relative, radius, deadzone }`: the finger pushes a virtual stick. With `relative`, center is where the finger lands and `radius` pad widths from there is full; otherwise the middle of the pad (or half) is center and its edges are full. Pushes under `deadzone` (0..1) are ignored.
  - `menu = { menu, choose, overlay }`: the pad is divided into the menu's items as the menu is drawn (a radial menu by direction from the middle, a grid by cells, a list by rows, a carousel from left to right). `choose = "click"` holds the item under the finger while the pad is clicked; `"lift"` taps it when the finger lifts (a click still works). `overlay = false` hides the menu.
  - `swipe = { up, down, left, right, distance }`: a quick stroke at least `distance` pad widths long (default 0.2) taps the action for its direction. Each direction is optional.
- `click`: any action. In menu mode it's what a click does with the finger off the items. With a split pad, a click runs the click of the half the first finger is on.
- `right` (optional): `{ motion, click }` for the right half. Each finger belongs to the half it lands on.

The touchpad modes other than mouse, mouse's `tap`, `scroll` and `inertia`, and `right` are format 16.

## Controller identity

`controllers` says which other controllers the game has explicit support for: `dualshock` (DualShock 4), `dualsense` and `switch_pro` (Switch Pro Controller). Each is optional and off by default. While the game is active, a physical controller of a ticked kind makes the virtual pad present itself as that kind: its USB IDs, name and button labels, so the game sees a DualShock, DualSense or Pro controller. Any other controller, or a kind left unticked, is seen as an Xbox 360 pad, as before.

The Mappings overlay draws each mapped output as the controller the pad presents itself as. Default text for a pad button is `{pad:south}` and so on (see Overlay info below), so an output that is a DualSense's bottom button shows ✕ in a DualSense game and A elsewhere.

## Requirements

A profile says what it cannot be played without, so the app can offer only profiles a player's controller supports:

```toml
requires = ["gyro", "back_paddles", "touchpad"]
```

`gyro`, `back_paddles` and `touchpad` exist today. Don't set `requires` for a profile that only *adds* gyro, paddle or touchpad inputs to a scheme that works without them: leave it out. The export dialog lists declared needs and warns about gyro, paddle or touchpad use that isn't declared (a touchpad set up differently from the default counts as use), but doesn't block. `touchpad` is format 16. Paddle buttons (`LeftPaddle`, `RightPaddle`, `LeftPaddle2`, `RightPaddle2`) are format 13 and can't be read by older app versions.

## Menus

```toml
[[menus]]
name = "Belt"
cancel = "East"               # optional: button that backs out (default East)
keep_open = true              # optional: stay up after a pick (default false: a pick closes it)

[menus.kind.radial]
stick = "Right"

[[menus.items]]
label = "Belt 1"
action = { keys = ["KEY_1"] }

[menus.style]
position = "center"           # a 3×3 grid: top_left … bottom_right
scale = 1.0

[menus.style.background]
color = "#16181c"
opacity = 0.92
```

Menu kinds (`menus.kind`):

| Kind | TOML | Notes |
|---|---|---|
| Radial | `[menus.kind.radial] stick = "Right"` | Aim the stick at an item; release to pick it. Items are arcs of a circle, each sized by its `weight` (default 1). `boxes = true` puts them in boxes at equal angles instead. |
| Directional | `[menus.kind.directional] cluster = "DPad"` (or `"FaceButtons"`) | Four slots, up/right/down/left. |
| List | `kind = "list"` | D-pad or left stick moves; A picks. |
| Button | `kind = "buttons"` | A list whose items can also be picked with their own `button`. |
| Carousel | `[menus.kind.carousel] controls = "Bumpers"` | Cycles with `Bumpers`, `Triggers`, `DPad`, `LeftStick` or `RightStick`. |
| Grid | `[menus.kind.grid] columns = 4` | Up to 6 columns and 6 rows. |

A menu item's `action` can be `open_menu = "Name"`, which opens a submenu. Radial menus can't open submenus. `style` also has `items` and `selected` paints, as in the example above. `corners` (0–1, default 0.4) sets how round boxes, keys and panels are: 0 is square and 1 is a circle wherever the shape allows.

### Decorations

Any overlay style (menus, the keyboard and numpad, media controls, info and log overlays, layer indicators) can also have decorations that give a game's overlays a look of their own. Each is optional, and a field left out of one takes the default shown. Sizes are pixels at scale 1 and grow with `scale`.

```toml
[menus.style.border]          # the panel's outline (default: a faint 1 px line)
width = 2                     # 0 draws none
color = "#c8a040"
opacity = 1.0

[menus.style.shadow]          # the panel's shadow (default: soft, dark, 6 px low)
color = "#000000"
opacity = 0.5                 # 0 draws none
blur = 16
x = 0                         # moves it right
y = 0                         # moves it down

[menus.style.gradient]        # the background fades into this color (default: one color)
color = "#3a1010"
opacity = 1.0
angle = 180                   # degrees: 0 bottom to top, 90 left to right, 180 top to bottom

[menus.style.item_border]     # every item's and key's outline (default: faint, 2 px on the selected one)
width = 2                     # the selected item's turns its text color; 0 draws none at all
color = "#806030"

[menus.style.glow]            # a glow around the selected item (default: none)
color = "#ffcc40"             # fields as for shadow; leave x and y at 0 for a glow
opacity = 0.9
blur = 30
```

`text = "#e8d8a8"` sets the color of the panel's own text (titles, hints, info overlay text) instead of white or near-black chosen to suit the background. Items keep their own. Radial menus drawn as arcs take the panel decorations but no item outline or glow. Decorations are format 14; older app versions can't read packs that use them. The player can turn them all off on the App settings page (Performance → Plain overlays).

## Macros

```toml
[[macros]]
name = "Dodge"
steps = [
    { press = { keys = ["KEY_LEFTSHIFT"] } },
    { wait = 120 },
    { tap = { action = { keys = ["KEY_SPACE"] }, hold_ms = 50 } },
    { release = { keys = ["KEY_LEFTSHIFT"] } },
    { stick = { stick = "Left", x = 0.0, y = -1.0 } },
]
```

Steps: `tap` (press, hold `hold_ms`, release), `press` (keep held until a matching `release` or the macro's end), `release`, `wait` (milliseconds), and `stick` (move a virtual stick to `x`, `y` in −1..1, y positive down; it recenters when the macro ends). Macros are referenced from buttons with `macro = { name = "…" }`.

## Info overlays

```toml
[[info_overlays]]
name = "Controls"
title = "This game's controls"    # optional heading; may hold {tokens}
always = false                    # true: shown whenever the game is active
on_start = 6.0                    # optional: seconds shown when the game starts
linger = 2.0                      # optional: seconds it stays after release
rows = [
    ["{south}", "Jump", "{east}", "Use"],
    ["{lt} {rt}", "Scope · fire", "{lb}", "Belt (hold)"],
]

[info_overlays.style]
position = "center"
scale = 0.9
```

- Each row is a list of cells, and cells line up in columns. A cell is text with `{tokens}`.
- Button glyphs: `{south} {east} {west} {north} {lb} {rb} {lt} {rt} {select} {start} {guide} {ls} {rs} {l3} {r3} {dpad} {up} {down} {left} {right}`. They are drawn for the controller in use.
- Output glyphs: `{pad:south}` (any button name from the list above, after `pad:`) is drawn for the controller the pad presents itself as, `{keyboard:a}` is a key by its name without `KEY_` in lower case (`{keyboard:leftshift}`, `{keyboard:f1}`), and `{mouse:leftclick}` is `rightclick`, `middleclick`, `backclick` or `forwardclick`. A key or mouse token that names nothing stays as text.
- Live values: `{time} {time12} {date} {profile} {layer} {mode} {app} {title} {pid} {cpu} {ram} {gpu} {wifi} {system_battery} {controller_battery}`, plus `{current_input}` (with the `current_input` settings).
- An overlay is shown by a `show_info` action, or always when `always = true`.

## Modes

A mode is a control scheme the player switches to and stays in, such as driving. Only one is on at a time; with none on, the profile itself is the default mode. A mode is written exactly like a layer (see below), under `[[modes]]`, and switched to with the `mode` action:

```toml
[profiles.buttons.North]
mode = "Driving"

[[modes]]
name = "Driving"
indicator = "name"

[modes.buttons.North]            # Y again goes back to on foot
mode = "Driving"

[modes.right_trigger.action.button.action]
keys = ["KEY_W"]
```

Layers apply on top of the mode that's on.

## Layers

A layer changes some of the profile's (and the mode's) mappings while it is held. Everything it doesn't set falls through to the mode and the profile. A layer can't be toggled: one that should stay on is a mode.

```toml
[[layers]]
name = "Guide"
indicator = "bindings"           # "name" | "bindings" | "off" | { info = "Name" }
indicator_title = "Hold {guide} and press"
indicator_delay_ms = 100         # appears only if held this long
swallow_unbound = true           # buttons the layer doesn't set do nothing

[layers.buttons]
West = "toggle_overlay"

[layers.buttons.DpadDown]
keys = ["KEY_TAB"]

[layers.right_stick.action.mouse]
speed = 1600.0
```

A layer or mode can override `buttons`, `gestures`, `combos`, `disabled_combos`, `left_stick`, `right_stick`, `left_trigger`, `right_trigger`, `gyro` and `touchpad`. Each override replaces the profile's setting for that input only. With `swallow_unbound`, buttons it doesn't set do nothing, except those that hold a layer or switch modes.

**Guide** and **Mouse mode** are built-in modes. Every game has them, fixed, so a pack never carries them (a stored copy is dropped when read). The Guide button is the app's own: whatever a profile maps it to is ignored, and the library packs write `mode = "Guide"` there. Older packs' `layer = "Guide"` (and toggles of `"Mouse mode"`) are read as the built-in modes. The Guide mode's shortcuts: `+ X` keyboard, `+ Y` numpad, `+ LB` media controls, `+ RB` screenshot, `+ Left stick` recording, `+ B` force-quit, `+ RT` / `+ LT` clicks, `+ Right stick` mouse, `+ D-pad` Enter, Tab and Escape, and so on. Its bindings are listed in [the Guide design notes](development/guide.md).

## Keyboard, numpad and font

```toml
overlay_font = "Rajdhani"

[keyboard_style]
position = "bottom_center"
scale = 1.0

[keyboard_style.background]
color = "#080c0d"
opacity = 0.94
```

`keyboard_style`, `numpad_style`, `media_style` and `menu_style` use the same shape as an overlay's `style`. `overlay_font` names a bundled font (`assets/fonts`) or an installed family.

## Overlay sounds

`[sounds.<overlay>]` sets the sounds an overlay makes as its cursor moves (`step`), when something is picked (`pick`), and as it pops in (`open`) and out (`close`). Each overlay has its own table, and every key is optional. A missing overlay uses the defaults: a click and a pop, with a quiet bubble as it opens and closes, except the Guide menus, which default to soft electronic tones (`pulse`, `glass`, `rise` and `fall`).

```toml
[sounds.menu]
enabled = true                # false silences all four cues
step = { kind = "knock", volume = 0.07, pitch = 1.0, length = 0.8 }
pick = { kind = "thud", volume = 0.3, pitch = 0.8, length = 1.4 }
open = { kind = "swish", volume = 0.04, pitch = 1.0, length = 1.0 }
close = { kind = "swish", volume = 0.03, pitch = 1.0, length = 1.2 }
```

The overlays are `keyboard` (the on-screen keyboard), `numpad`, `menu` (every menu layout), `guide` (the Guide overlay's menu and the Guide + Start menu), `media` (the media controls) and `offer` (library offers). Input logs and info overlays have no cursor, so they make no sounds, and their appearing and going make none either.

| `kind` | Sounds like |
|---|---|
| `click` | A dry tick. |
| `tick` | A small, bright wooden tick. |
| `pop` | A soft bubble that drops in pitch. |
| `blip` | A short retro bleep that rises. |
| `chime` | A bright two-note bell. |
| `clink` | A high metal ping with a rattle. |
| `thud` | A low, short thump. |
| `knock` | A knuckle on wood. |
| `crunch` | A few crisp bursts of noise. |
| `swish` | A breath of air that brightens as it passes. |
| `keys` | A key going down on a mechanical keyboard. |
| `typewriter` | A typewriter keystroke: the key going down, the typebar's hard clack on the platen, and the carriage ticking along. |
| `piano` | A piano key struck: a warm note that dies away. |
| `beep` | A clean, flat electronic beep. |
| `drop` | A drop falling into water: a round note that leaps up. |
| `marimba` | A wooden bar struck with a soft mallet. |
| `glass` | A crystal glass tapped: a pure, long note that shimmers. |
| `pulse` | A soft electronic tone with a glassy edge that melts away. |
| `rise` | A soft tone gliding up. |
| `fall` | A soft tone gliding down. |
| `coin` | A retro pickup: two quick square notes. |

`volume` is 0–1, `pitch` is 0.5–2 (an octave either way) and `length` is 0.5–2 (half as long, or twice as long). All three are relative to the kind's own sound. A longer cue is the same sound played slower, not a longer tail.

Five effects change a cue after it's made, and all of them are off (0) unless set:

| Key | Range | Does |
|---|---|---|
| `bass` | -12 to 12 | Turns the lows down or up, in dB. |
| `treble` | -12 to 12 | Turns the highs down or up, in dB. |
| `drive` | 0–1 | Pushes the cue into soft clipping: thicker and grittier, and it carries better on small speakers. |
| `echo` | 0–1 | Echoes after the cue, louder and more of them as it grows. Each is a little darker than the last. |
| `reverb` | 0–1 | A room the cue rings in, more of it and bigger as it grows. |

Echo and reverb add a tail after the cue, up to about two seconds each at 1, so keep them low on a step that repeats as a direction is held.

```toml
[sounds.menu]
pick = { kind = "chime", volume = 0.3, echo = 0.3, reverb = 0.2, bass = -3 }
```

## Overlay motion

`[motion]` sets how each kind of overlay moves: as it opens and closes, as the cursor moves and as something is picked. Each key is one of `off`, `subtle`, `playful`, `stagger`, `grim` or `brutal`, and every key is optional (a missing one is `subtle`).

```toml
[motion]
keyboard = "grim"
numpad = "grim"
menu = "grim"
media = "grim"
offer = "grim"
info = "grim"
log = "grim"
```

| Style | Feel |
|---|---|
| `off` | Nothing moves. |
| `subtle` | A quick fade, and the highlight glides from item to item. |
| `playful` | Panels drop in with a bounce, and the highlight overshoots its item. |
| `stagger` | Items fade in one after another. |
| `grim` | Slow fades that rise into place, and a dim, heavy pulse on a pick. |
| `brutal` | Snaps in and out, the highlight jumps, and a pick jolts and flashes. |

The kinds are `keyboard`, `numpad`, `menu` (every menu layout), `media` (the media controls), `offer` (library offers), `info` (info overlays) and `log` (input logs).

## Versioning

- `format` is an integer. The app writes `17` and reads up to `17`. A pack with a newer `format` is refused with "update the app to import it". Older formats are upgraded when read.
- Format history:
  - 2 added layers.
  - 3 added toggles that start on.
  - 4 added keyboard and numpad styles.
  - 5 added the overlay font.
  - 6 added grid menus.
  - 7 added the Guide actions (`screenshot`, `toggle_recording`, `force_quit`), layer indicators with generated bindings, extra info overlays and a delay.
  - 8 moved controller needs from the pack to each profile (`requires`).
  - 9 added the media controls and in-game menu looks (`media_style`, `menu_style`).
  - 10 added menu sounds, radial arcs and corner rounding.
  - 11 added how each overlay moves (`motion`).
  - 12 replaced the menu sounds with a set of sounds for each overlay (`[sounds.<overlay>]`).
  - 13 added back paddles (their button names and the `back_paddles` feature).
  - 14 added overlay decorations (outlines, shadows, gradients, glows and text colors).
  - 15 added gyro acceleration curves, axes, player and world space, the position mode, smoothing and pausing, and the touchpad's curve.
  - 16 added the touchpad's joystick, menu and swipe modes, tapping, scrolling and gliding, the split pad, and layers' touchpads.
  - 17 added modes (`[[modes]]` and the `mode` action), and stopped layers from being toggled. When an older pack is read, each `toggle` of a layer becomes a `mode` switch, and a layer that was only ever toggled becomes a mode.
- Bump `format` only for a breaking change to the shape. Adding an optional key with a default doesn't need it.

## Checks

The repository's test suite parses every file in `packs/` and checks it: the format, unique IDs and names, no dangling references (macros, menus, info overlays, modes, layers, profiles), that each rule points at an existing profile, and that a plain pad can play at least one profile. A pack you add to `packs/` is checked the same way, and it is embedded in the next build.
