# `.padpack` format reference

A pack is one game in one file: TOML, in the same shapes the app writes to `config.toml`. This page describes **format 20**, the format the current app writes. For a walkthrough of building one, see [the Deus Ex tutorial](manual/tutorial-deus-ex.md). The shipped examples are in `packs/`.

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
[assets.icons]                   # optional, SVG icons for {icon:name} (see Icons)
[assets.sounds]                  # optional, WAV files for overlay cues (see Sounds a pack carries)
[assets.fonts]                   # optional, TrueType/OpenType files for {font:…} (see Fonts a pack carries)
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
  A value with folders in it (`Game Dev Tycoon/nw.exe`) matches the end of the program's path instead, whole folder names only, for a program whose name other games share. It wins over a rule naming only the file.
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
| `"gyro_toggle"`, `"gyro_clutch"`, `"gyro_hold"`, `"gyro_recenter"` | | Switch gyro on or off, turn it off while held, turn it on while held, or recenter it. |
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

- Gesture kinds: `double_tap`, `triple_tap`, `long_press`, `on_press`, `on_release`. `on_press` and `on_release` tap their action once as the button goes down and once as it's let go, beside its own action and without waiting on the other gestures. The same key on both makes a game's toggle a hold.
- A combo needs at least two buttons, and no two combos in a profile, layer or mode can have the same buttons: to do several things, give one combo a `multi` action. Members wait `combo_window_ms` for the rest of the combo before acting alone. A member whose own action is `"disabled"` works as a modifier with no time limit.

Key names are evdev names (`KEY_A`, `KEY_LEFTSHIFT`, `KEY_KPPLUS`, `KEY_RIGHTBRACE`, `KEY_APOSTROPHE`, …). The keyboard overlay in the app lists them.

## Sticks

```toml
[profiles.left_stick]
deadzone = 0.12          # radial deadzone, 0.0–1.0
curve = 1.0              # response exponent for mouse and scroll: 1.0 linear, higher = finer at small pushes (default 2.0)
deadzone_shape = "square"  # "circle" (default), "square" or "cross" (each axis on its own)
anti_deadzone = 0.1      # least output once past the deadzone, for games with their own deadzone (default 0)
anti_deadzone_buffer = 0.05  # travel past the deadzone where output holds at the anti-deadzone (default 0)
key_threshold = 0.25     # push at which direction keys and direction buttons press (default 0.5)
invert_x = true          # mirror the stick left to right (default false)
rotate = 90.0            # degrees clockwise the stick's output is turned: 90 makes up act as right (default 0)

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
| `region` | `region = { x = 0.5, y = 0.5, width = 0.5, height = 0.5, shape = "circle" }` | Mouse region: the stick's position puts the pointer at the matching spot in a part of the desktop (all monitors together). `x` and `y` are its middle and `width` and `height` its size, each a share (0..1) of the desktop; missing ones take these defaults. `shape = "circle"` fills the width and height, so it is an ellipse when they differ on screen, and a full push in any direction reaches its edge; with `"rectangle"`, a full push into a diagonal reaches a corner. Letting go puts the pointer back in the middle. |

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

A trigger acting as a button takes `on_press` and `on_release` too (no other gestures), here turning a game's toggled aim into a hold while the trigger holds a finer-aim layer:

```toml
[profiles.left_trigger.action.button]
threshold = 0.4
action = { layer = "Zoom" }

[profiles.left_trigger.gestures]
on_press = { mouse = "Right" }
on_release = { mouse = "Right" }
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
toggle_rumble = false     # optional: a short rumble when the gyro on / off action switches it
start_off = false         # optional: gyro waits for an action to turn it on

[profiles.gyro.mode.mouse]          # or: "off", stick = {...}, position = {...}, steering = {...}
sensitivity = 15.0                  # pixels per degree turned

[profiles.gyro.curve]               # optional acceleration curve (mouse and stick modes)
slow_gain = 0.5                     # sensitivity, times the mode's, up to slow_speed
fast_gain = 2.0                     # and from fast_speed
slow_speed = 5.0                    # degrees/second
fast_speed = 40.0
power = 1.0                         # shape of the blend between: 1 is straight
```

- `mode`: `"off"`, `mouse = { sensitivity }`, `stick = { stick, full_rate, anti_deadzone, game_curve }` (rotation speed deflects a virtual stick; `game_curve`, default 1, is the exponent of the game's own stick response, which the deflection undoes so the game turns as fast as the controller does), `position = { stick, max_angle, anti_deadzone, lock_at_edge }` (how far the controller has turned deflects a virtual stick; `max_angle` degrees is full, and with `lock_at_edge` turning further is let go, so turning back leaves the edge at once), or `steering = { stick, max_angle }` (tilt steers a stick).
- `horizontal`: which turning aims left and right. `player_space` and `world_space` use gravity, so turning left aims left however the controller is held; `world_space` also reads up and down against the horizon.
- `full_turn_px` (optional, mouse mode): mouse pixels that turn the game a full circle. With it, `sensitivity` is real-world: 1.0 turns the game as far as the controller turned.
- `curve`: the app's presets are Linear (no curve), Gentle, Aggressive, Wide, Extra wide, Precision and Turn boost.
- `start_off` (optional): gyro starts switched off. What switches it is an action on any button, gesture, combo or menu item: `"gyro_toggle"` (on or off), `"gyro_hold"` (on while held), `"gyro_clutch"` (off while held) and `"gyro_recenter"` (sets the current direction as straight ahead in steering and position modes, and clears leftover motion). A trigger takes them as `action = "gyro_clutch"` inside its `button` action.

The touchpad's mouse motion takes the same `curve`, with speeds in pad widths/second (`[profiles.touchpad.motion.mouse.curve]`). A finger's pad widths/second feel about like 40 times as many degrees/second of gyro.

Everything in this section beyond `mode`, `horizontal`, the inverts, `noise_threshold` is format 15; older app versions can't read packs that use the new modes or spaces.

## Touchpad

DualShock 4 and DualSense touchpads. Left out, the pad works as a laptop trackpad: mouse motion at 1200 pixels across the pad, and the click is the left mouse button.

```toml
[profiles.touchpad]
click = "disabled"                  # what pressing the pad does, like any button

[profiles.touchpad.motion.menu]     # or: "off", mouse = {...}, joystick = {...}, region = {...}, swipe = {...}
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
  - `region = { x, y, width, height, shape }`: mouse region. The pad (or half) is laid over a part of the desktop, as for a stick's `region`, and the pointer goes where the finger is. With `shape = "circle"` (the default), a finger outside the circle, such as in a corner of the pad, puts the pointer on its edge; the settings window starts a pad on `"rectangle"`. Lifting the finger leaves the pointer where it is.
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
label = "{icon:potion} Belt 1"   # a label may hold {tokens}: icons, glyphs and formatting (see Text)
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

A grid item's `span = {columns = 2, rows = 1}` merges it over the cells to its right and below. Item `i` sits in cell `i` (left to right, then down), and the items whose cells it covers are hidden until it's split again, so a pack fills them with copies of the merged item. The grid has as many cells as items, so a span can't reach past the last item.

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

Text color is white or near-black, whichever reads on what the text sits on; to color some of the text, use `{color:…}` tags (see [Text](#text)). Radial menus drawn as arcs take the panel decorations but no item outline or glow. Decorations are format 14; older app versions can't read packs that use them. The player can turn them all off on the App settings page (Performance → Plain overlays).

`pulse = true` on the `glow` makes it breathe, and `drift = true` on the `gradient` makes its direction sway slowly. Both are looping effects, see [Looping effects](#looping-effects).

### Typography

```toml
[menus.style.type]
case = "upper"                 # "none" (as written), "upper" or "lower"
letter_spacing = 2             # pixels between letters at scale 1, 0 to 12
size = 1.1                     # a factor on every text size, 0.5 to 2
```

Titles, labels, hints and the offer's answers all follow the typography. Keyboard keys keep their own size. Letter spacing is laid out letter by letter, so long labels still wrap. Weight, slant, underline and color are not style settings: they go inside the text, as [formatting tags](#text). Older packs' `bold`, `italic`, `color` and `text` keys are ignored.

### Ornaments

```toml
[menus.style.ornament]
dividers = { width = 1, color = "#c8a040", opacity = 0.6 }   # a line between a list's rows
frame = { kind = "brackets", size = 16, width = 2, color = "#c8a040" }   # or "chamfer", "art_deco", …
scanlines = { spacing = 4, opacity = 0.2, color = "#000000", drift = false }   # a filter
grain = { opacity = 0.15, size = 1, animated = false }       # a filter: film noise
pixels = { size = 6, opacity = 0.25, shimmer = false }       # a filter: a mosaic of blocks
camo = { scheme = "woodland", opacity = 0.3, size = 60 }     # a filter: camouflage blotches
plaid = { colors = ["#a8202a", "#1f2f4a", "#e8c547"], opacity = 0.25, size = 80 }  # a filter: a tartan
argyle = { colors = ["#2f4a6b", "#c9b48a", "#e8e2d0"], opacity = 0.25, size = 40 } # a filter: diamonds
stripes = { colors = ["#1f3a5f", "#e8e2d0", "#b0202a"], direction = "diagonal", opacity = 0.25, size = 16 }
checkerboard = { colors = ["#1a1a1a", "#e8e2d0"], opacity = 0.25, size = 24 }   # a filter: squares
polka_dots = { colors = ["#b0202a", "#f4efe4"], opacity = 0.25, size = 28 }     # a filter: dots on a ground
tint = { color = "#ffa040", opacity = 0.12, on = "items" }   # a filter: a wash of color, on the items and keys alone
vignette = { color = "#000000", opacity = 0.6, reach = 25 }  # reach: percent of the panel
paper = { opacity = 0.5 }                                    # or brushed_metal / concrete / cloth / denim / silk / leather / steel / wood = { opacity = 0.5 }
bevel = { depth = 0.5 }                                      # raised, 3D-looking items and keys
```

- `dividers` goes between the rows of list and button menus. Grid, directional, carousel and radial menus have no rows to divide.
- `frame` marks each corner: `brackets` is an L along the edges, and `chamfer` a line across the corner. The ornaments are `art_deco` (a stepped corner with a sunburst), `art_nouveau` (a vine with tendrils and a leaf), `runic` (a band of runes), `gothic` (a cusped arch with a roundel), `baroque` (scrolls and leaves), `circuit` (traces with pads), `riveted` (an iron plate), `celtic` (two strands woven over and under, through a ring at the corner) `greek_key` (a running key between two rails), `egyptian` (a frieze of lotus flowers and buds between two rails, a sun disc at the corner) and `floral` (a blossom at the corner, a leafy vine running from it along each edge); they want a `size` of 32 to 72 for their detail to show, and the ones that run along the edges sit best on square corners (`corners = 0`). `size` is how far each mark reaches, at scale 1.
- `scanlines` are thin lines `spacing` pixels apart. `drift` rolls them slowly down (see [Looping effects](#looping-effects)).
- `vignette` darkens each edge, fading out `reach` percent of the panel's size in from it.
- `scanlines`, `grain`, `pixels`, `paper`, `brushed_metal`, `concrete`, `cloth`, `denim`, `silk`, `leather`, `steel`, `wood`, `camo`, `plaid`, `argyle`, `stripes`, `checkerboard`, `polka_dots` and `tint` are the filters (the style editor groups them under Filters). Each takes `on`: `background` (the panel's background), `items` (each item and keyboard key) or `both`; the materials default to `background` and the rest to `both`. Filters always sit under the text, and on an item they take its shape (a radial menu's arcs included); a filter's pattern runs on unbroken across the panel and every item it covers. `grain` is noise in blocks `size` pixels across; `animated` reshuffles it a dozen times a second. `pixels` is a mosaic of blocks `size` pixels across, each a little lighter or darker with a dark seam between them; `shimmer` makes each block slowly brighten and dim, out of step with its neighbours. It is a pattern, not a real pixelation of what's under it. `camo` is soft-edged blotches in three colors on a base, `size` pixels across; `scheme` is `woodland`, `desert`, `urban` or `arctic`. `plaid`, `argyle` and `stripes` each take three `colors`. `plaid` is a tartan: a ground (the first color) crossed both ways by wide bands (the second) and thin lines (the third), woven in a twill, the pattern repeating every `size` pixels. `argyle` is diamonds `size` pixels wide in the first two colors, alternating, with thin lines of the third through their middles. `stripes` are bands `size` pixels wide in the first two colors, with a pinstripe of the third between each two; `direction` is `horizontal`, `vertical` or `diagonal`. `checkerboard` and `polka_dots` each take two `colors`. `checkerboard` is squares `size` pixels across in the two colors, alternating. `polka_dots` is round dots in the second color on a ground of the first, `size` pixels apart, each row offset by half a step. `tint` is a flat wash of `color`.
- `paper` (grain, blotches and fibres), `brushed_metal` (streaks and a soft sheen) `concrete` (stains, sandy grain, pits, shuttering seams and hairline cracks), `cloth` (a plain weave of uneven threads, like linen), `denim` (a twill in diagonal ribs), `silk` (smooth, flowing folds with a soft sheen), `leather` (pebbled grain, creases and wrinkles), `steel` (tread plate: raised lugs in a herringbone) and `wood` (planks with wavy grain and knots) lay a material over the panel's background or its items. Each only lightens and darkens, so the color under it still shows through; `opacity` (0 to 1) is how strong the grain is.
- `bevel` makes items and keyboard keys look like raised buttons: a lit top, a darker bottom and a drop shadow, with the selected item pressed in. `depth` (0 to 1) is how pronounced that is. An item's own outline (`item_border`) replaces the lit edge.

Ornaments are decorations, so the player's Plain overlays setting turns them all off.

### Background pictures

```toml
[menus.style.background_image]
file = "crest.png"             # in the backgrounds folder, or a full path
opacity = 0.6                  # 0 to 1
fit = "cover"                  # "cover" (crops), "contain" (leaves bars) or "stretch"
```

The picture is drawn behind the panel's contents and over its background color. PNG, JPEG and SVG files work. A bare file name is looked up in the backgrounds folder, and a name with folders in it (`themes/crest.png`) is a path inside that folder. A full path is used as it is. A picture doesn't travel with the pack: whoever imports it needs the same file in the same place.

Plain overlays turns the picture off too.

### Looping effects

Some effects keep moving while the overlay is up (animated grain flickers and shimmering pixel blocks pulse like drifting scanlines do): a glow that pulses (`pulse = true` on `glow`), a gradient that sways (`drift = true` on `gradient`), and scanlines that roll (`drift = true` on `scanlines`). They cost a redraw every frame, so the player can turn them all off on the App settings page: Performance → **No looping animations**. Turning them off freezes each effect where it is, and the motion styles still play. Nothing moves in the overlay's own motion (Performance → No motion) stops them too.

### Text

Any text an overlay draws (menu labels, hints, info overlay headings and cells) can hold `{tokens}` and formatting tags in braces. Tags work in pairs: put the opening tag before the text and its closing tag after it. A tag left open holds to the end of the text. Anything in braces that isn't a token or tag stays as written.

| Tag | Does |
|---|---|
| `{b}…{/b}` | bold (a font with no bold face draws it regular) |
| `{i}…{/i}` | italic |
| `{u}…{/u}` | underlined |
| `{box}…{/box}` | a light outline around the text, in the text's color |
| `{font:Cinzel}…{/font}` | another font: a bundled one, an installed family, or one the pack carries (see [Fonts a pack carries](#fonts-a-pack-carries)) |
| `{color:red}…{/color}` | colored: `red`, `orange`, `yellow`, `green`, `cyan`, `blue`, `purple`, `pink`, `gray`, `white`, or a hex color, `{color:#ff8800}` |

Tags nest, so `{b}{color:orange}Quick save{/color}{/b}` is bold and orange. Use them for legibility and emphasis: bold the action's name, color the dangerous ones, underline a shortcut letter, box a key to hold.

The app's editors have an **Insert…** button beside every text that takes tokens. It opens the token picker: every token and tag in groups, each with a preview of what it shows, buttons drawn for Xbox, PlayStation and Nintendo controllers side by side, and every icon.

### Icons

`{icon:name}` draws an icon the height of the text, in the text's color. Use one in front of a menu item's label, or anywhere in an info overlay, so a glance says what an item does. The token picker shows every icon. The built-in ones, usable in any game, are the files in `assets/icons`:

`advisor`, `anchor`, `arrow`, `artwork`, `atom`, `attack`, `automate`, `axe`, `badge`, `bag`, `beaver`, `bell`, `bolt`, `book`, `bow`, `box`, `brain`, `broom`, `bus`, `camera`, `cancel`, `car`, `cassette`, `chart`, `chip`, `civics`, `clock`, `cloud`, `coaster`, `coin`, `compass`, `cottage`, `cross`, `crosshair`, `crystal`, `dagger`, `distribution`, `drop`, `drop-item`, `eraser`, `eye`, `factory`, `farm`, `fast`, `fire-truck`, `fist`, `flag`, `flame`, `flip`, `footprints`, `forbid`, `forest`, `fortify`, `gate`, `gear`, `gem`, `globe`, `god`, `great-person`, `grenade`, `grid`, `guests`, `hammer`, `hand`, `handshake`, `heart`, `height`, `holster`, `home`, `hospital`, `hourglass`, `key`, `layers`, `leaf`, `lens-resources`, `load`, `load-in`, `lockpick`, `map`, `mayor`, `medkit`, `messages`, `migration`, `money`, `money-bag`, `monument`, `moon`, `mountain`, `move`, `nav`, `palace`, `palette`, `path`, `pause`, `paw`, `pda`, `people`, `person`, `pick`, `pill`, `pillage`, `pipe`, `pistol`, `plant`, `population`, `potion`, `radiation`, `rail`, `rally`, `ranged`, `region`, `religion`, `research`, `rifle`, `road`, `rocket`, `rope`, `rotate`, `rotate-left`, `save`, `school`, `scope`, `scroll`, `search`, `see-through`, `sentry`, `settler`, `shield`, `shop`, `skull`, `sleep`, `sound`, `speed1`, `speed2`, `speed3`, `spy`, `star`, `stethoscope`, `stop`, `storage`, `subway`, `supports`, `sword`, `sword-cross`, `syringe`, `tech`, `temple`, `terrain`, `theater`, `ticket`, `trade`, `train`, `trap`, `trash`, `tree`, `trophy`, `underground`, `unload`, `upgrade`, `water-toggle`, `waves`, `wellbeing`, `wheat`, `wheel`, `workplace`, `wrench`.

A pack can carry its own icons, in `[assets.icons]`. Each is an SVG, by name, and works only in that pack's game. A pack's icon with a built-in's name replaces it in that game.

```toml
[assets.icons]
flame = "<svg viewBox='0 0 24 24'><path d='M12 2c1 4 6 6 6 12a6 6 0 0 1-12 0c0-3 2-4 2-7 2 1 3 2 4-5z' fill='currentColor'/></svg>"
```

Then `{icon:flame}` works in that game's menus and overlays. Wherever the SVG says `currentColor` it is drawn in the text's color, so it follows the style and the selection; anything else keeps its own color. Names use letters, digits, `-` and `_`; an SVG is at most 64 KB and may not hold scripts, links or embedded pictures (a pack with one is refused). An `{icon:…}` that names nothing shows as written, so a mistake is easy to see. In the app, **Details → Pack files → Add icon…** adds an SVG file to the setup, and it goes into the pack when exported.

Draw icons on a square `viewBox` (24 by 24 works well), simple enough to read at text size. If none of the built-in icons fits an item, make one that does: a pack with its own, fitting icons reads better than one that stretches the built-in set.

### Fonts a pack carries

`{font:name}…{/font}` sets some of the text in another font. `name` is a bundled font (`{font:Cinzel}`, `{font:Press Start 2P}`), any font installed on the player's machine (`{font:Fira Sans}`; this falls back to the default where it isn't installed), or a font the pack carries. The same names work for `overlay_font`.

A pack carries a font in `[assets.fonts]`: a TrueType or OpenType file (`.ttf` or `.otf`) as base64, at most 1 MB, under any name made of letters, digits, spaces, `-`, `_` and `.`. It works only in that pack's game, and takes precedence over a bundled font of the same name there.

```toml
[assets.fonts]
Runic = "AAEAAAAQAQAABAAAR0RFRgBk…"

[[menus.items]]
label = "{font:Runic}Rune of Fire{/font} {icon:flame}"
```

Only carry a font whose license lets you share it (the bundled ones are under the SIL Open Font License; so are most Google Fonts). A font with no bold or italic face draws the same under `{b}` and `{i}`. In the app, **Details → Pack files → Add font…** adds one, and it then appears in the font dropdowns and the token picker.

### Gyro and touchpad

`{gyro:p}`, `{gyro:r}` and `{gyro:y}` show how fast the controller turns about each axis in degrees a second (pitch, roll and yaw, such as `+12°/s`), and `{trackpad}` shows where the touchpad is touched, 0 to 1 across and down from its top left corner, and "pressed" while it is clicked (`(0.42, 0.58) pressed`; `(–, –)` with no finger down). A controller without a gyro or touchpad shows `n/a`. Overlays holding these refresh about twenty times a second, rather than once a second like the clock.

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
- Button glyphs: `{south} {east} {west} {north} {lb} {rb} {lt} {rt} {select} {start} {guide} {ls} {rs} {l3} {r3} {dpad} {up} {down} {left} {right} {leftright} {updown}`. `{leftright}` and `{updown}` are one D-pad with both arms of that axis lit. They are drawn for the controller in use. `{confirm}` and `{back}` are the buttons that choose and go back in menus, A and B unless the player moved them. `{guide}` is whatever opens the Guide overlay on the player's machine: the Guide button, or the input they moved that to.
- Output glyphs: `{pad:south}` (any button name from the list above, after `pad:`) is drawn for the controller the pad presents itself as, `{keyboard:a}` is a key by its name without `KEY_` in lower case (`{keyboard:leftshift}`, `{keyboard:f1}`), and `{mouse:leftclick}` is `rightclick`, `middleclick`, `backclick` or `forwardclick`. A key or mouse token that names nothing stays as text.
- Live values: `{time} {time12} {date} {profile} {layer} {mode} {app} {title} {pid} {cpu} {ram} {gpu} {wifi} {system_battery} {controller_battery}`, plus `{current_input}` (with the `current_input` settings), and the gyro and touchpad tokens (see [Gyro and touchpad](#gyro-and-touchpad)).
- Icons and formatting: `{icon:bow}`, `{b}`, `{i}`, `{u}`, `{box}` and `{color:…}`, see [Text](#text) and [Icons](#icons).
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

`keyboard_style`, `numpad_style`, `media_style` and `menu_style` use the same shape as an overlay's `style`. `overlay_font` names a bundled font or an installed family. The bundled fonts, under the SIL Open Font License (Special Elite under Apache 2.0; see `assets/fonts`), are Inter, Cantarell, Comfortaa, Source Code Pro, Rajdhani, Press Start 2P, VT323, Silkscreen, Monoton, Creepster, Nosifer, Eater, UnifrakturMaguntia, Cinzel, Bangers, Kreon, Orbitron, Titillium Web and Special Elite.

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

A cue can play a sound the pack carries instead of one of these kinds: `step = { file = "clank", volume = 0.2 }` (see [Sounds a pack carries](#sounds-a-pack-carries)). The `kind` beside it is what plays when the pack isn't in use.

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
| `typewriter` | A typewriter keystroke (a recording). |
| `piano` | A piano key struck: a warm note that dies away. |
| `beep` | A clean, flat electronic beep. |
| `drop` | A drop falling into water: a round note that leaps up. |
| `marimba` | A wooden bar struck with a soft mallet. |
| `glass` | A crystal glass tapped: a pure, long note that shimmers. |
| `pulse` | A soft electronic tone with a glassy edge that melts away. |
| `rise` | A soft tone gliding up. |
| `fall` | A soft tone gliding down. |
| `coin` | A retro pickup: two quick square notes. |
| `bong` | A round, soft bong, like a muted gong (recording). |
| `select` | A tiny, crisp tap (recording). |
| `toggle` | A soft switch flicked over (recording). |
| `pluck` | A plucked string, short and bright (recording). |
| `confirm` | A bright, friendly two-tone yes (recording). |
| `error` | A low, short buzz of refusal (recording). |
| `glitch` | A tiny electronic stutter (recording). |
| `rollover` | A light hover over a button (recording). |
| `switch` | A chunky switch thrown (recording). |
| `latch` | A metal latch clicking home (recording). |
| `page` | A page of a book turned (recording). |
| `wood` | A light knock on a wooden surface (recording). |
| `plate` | A metal plate struck, ringing out (recording). |
| `tin` | A tin can tapped (recording). |
| `bell` | A heavy bell struck, with a long ring (recording). |
| `geiger` | A Geiger counter's crackle: a few uneven ticks (sample). |
| `nano` | Two quick electronic shimmers (sample). |
| `radio` | A burst of radio static (sample). |
| `heartbeat` | Two low, soft thumps (sample). |
| `casing` | A spent brass casing bouncing on a hard floor (sample). |
| `lockpick` | Two small, dry metal ticks: a lock's pin giving way (sample). |
| `shing` | A blade drawn from its sheath: a bright metal ring (sample). |
| `fault` | A harsh square-wave stutter, like a failing terminal (sample). |
| `sand` | A soft hiss of grains pouring: an hourglass turned (sample). |
| `boing` | A cartoon spring: a wobbling tone that bends up and settles (sample). |
| `shutter` | A camera shutter: two crisp clicks (sample). |
| `zap` | An electric zap: a buzzing tone diving down, with crackle (sample). |
| `purr` | A cat's purr: a low, soft, pulsing rumble (sample). |
| `clank` | A machine part landing on steel: a thump with a short metal ring (sample). |
| `card` | A playing card flicked down on a table: a papery snap (sample). |
| `carriage` | A typewriter carriage thrown back: a quick ratchet and a clunk at the stop (sample). |

`volume` is 0–1, `pitch` is 0.5–2 (an octave either way) and `length` is 0.5–2 (half as long, or twice as long). All three are relative to the kind's own sound. `volume` scales the sound's level, so 0.1 is −20 dB and 0.01 is −40 dB (the settings show it in dB, and −60 dB is off). Equal volumes aren't equally loud: a short click needs far more than a pop to be heard as well, and a swish of noise far less. `scripts/sound-audit.py` measures cues' loudness to balance them. A longer cue is the same sound played slower, not a longer tail. The recordings and samples (marked above) are CC0 files that come with the app: `pitch` plays them higher or lower, and `length` stretches or squeezes them without changing their pitch.

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


### Sounds a pack carries

A pack can carry its own sound files, in `[assets.sounds]`: WAV files (8, 16, 24 or 32 bit PCM, or 32 bit float; any sample rate; mono or stereo, which is mixed to mono), as base64, by name. They are short cues: at most 600 KB and 8 seconds each. A cue names one with `file`:

```toml
[assets.sounds]
clank = "UklGRi4AAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQoAAAA…"

[sounds.menu]
pick = { file = "clank", volume = 0.3, pitch = 1.0 }
```

`volume`, `pitch` (which plays the file faster or slower) and the effects (`bass`, `treble`, `drive`, `echo`, `reverb`) apply to a file as they do to a built-in kind; `length` doesn't, a file being as long as it is. The sound plays only while the pack's game is in use. In the app, **Details → Pack files → Add sound…** adds a WAV file, and each cue's sound picker then lists it (marked ♪).

If none of the built-in kinds suits the game (a clank for a medieval menu, a radio click for a military one), record or synthesize a short sound that does and carry it in the pack.
## Overlay rumble

A setup can make the controller buzz as an overlay responds, in the same places its sounds play. Each overlay has its own table, and every cue is optional:

```toml
[rumble.menu]
step = { strength = 0.2, length_ms = 20 }    # as the cursor moves
pick = { strength = 0.6, length_ms = 40 }    # as something is picked
open = { strength = 0.1, length_ms = 30 }    # as the overlay comes up
close = { strength = 0.1, length_ms = 30 }   # as it goes away
mode = { strength = 0.3, length_ms = 60 }    # as a Guide mode is switched
```

The overlays are `keyboard`, `numpad`, `menu` (every menu layout), `guide` (the Guide menus), `media` and `offer`, as for [sounds](#overlay-sounds). `strength` is 0 to 1 and `length_ms` 10 to 500. With no cue set, an overlay is silent, so existing packs don't buzz. The buzz goes to the controller driving the overlay, and only if that controller can rumble. The App settings' rumble strength scales every cue. A pick cue replaces the App settings' faint pulse on picks for its overlay.

## Overlay motion

`[motion]` sets how each kind of overlay moves: as it opens and closes, as the cursor moves and as something is picked. Each key is one of `off`, `subtle`, `playful`, `stagger`, `swift`, `dreamy`, `retro`, `grim` or `brutal` (`swift`, `dreamy` and `retro` need format 19), and every key is optional (a missing one is `subtle`).

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
| `playful` | Panels spring in and bounce into place, and the item the cursor lands on boings. |
| `stagger` | Items cascade in one after another, sliding into place. |
| `swift` | Panels whoosh in from the side and on out the other, items trailing. |
| `dreamy` | Panels drift down and settle softly, and picks glow gently. |
| `retro` | Chunky steps like an old console: panels grow in blocks and picks blink. |
| `grim` | Slow fades that rise into place, and a dim, heavy pulse on a pick. |
| `brutal` | Snaps in and out, the highlight jumps, and a pick jolts and flashes. |

The kinds are `keyboard`, `numpad`, `menu` (every menu layout), `media` (the media controls), `offer` (library offers), `info` (info overlays) and `log` (input logs).

## Versioning

- `format` is an integer. The app writes `20` and reads up to `20`. A pack with a newer `format` is refused with "update the app to import it". Older formats are upgraded when read.
- A pack is refused if its setup couldn't be saved in the app: a missing macro, menu or overlay that something refers to, two items or profiles of one name, two combos of the same buttons, an unknown key and the like. A setup folder in the config that breaks these rules is left out when the config loads, and kept on disk to be fixed.
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
  - 18 added menu item icons, the typography (`type`), ornaments, background pictures, looping effects (pulsing glows and drifting gradients and scanlines) and rumble cues (`[rumble.<overlay>]`).
  - 19 replaced the menu item `icon` with the `{icon:name}` token, replaced the typography's bold, italic and color and the style's `text` color with inline formatting tags (`{b}`, `{i}`, `{u}`, `{box}`, `{color:…}`), added the `{gyro:…}` and `{trackpad}` tokens, added the `{font:…}` tag, and added the icons, sounds and fonts a pack carries (`[assets]`).
  - 20 added `native_controller` to `[pack]` (since removed).
- Bump `format` only for a breaking change to the shape. Adding an optional key with a default doesn't need it.

## Checks

The repository's test suite parses every file in `packs/` and checks it: the format, unique IDs and names, no dangling references (macros, menus, info overlays, modes, layers, profiles), that each rule points at an existing profile, and that a plain pad can play at least one profile. A pack you add to `packs/` is checked the same way, and it is embedded in the next build.
