# Debugging the overlay

Tools for seeing what the on-screen overlay does, frame by frame, without a monitor or a
controller. They answer two kinds of question:

- **Timing:** did the overlay get the frame the daemon sent, and when did it draw what? Use the
  [harness](#the-harness) and the [timing log](#the-timing-log).
- **Looks:** what does a menu look like part way through a fade? Use the
  [fade frames](#fade-frames).
- **Motion:** how does a motion style open, move, pick and close? Use the
  [motion runs](#motion-runs).

- **Stills:** how does a screen look, start to finish? Use the
  [screenshot and preview tools](#screenshots-and-previews).

All output goes under `screenshots/`, which git ignores.

## The harness

`scripts/overlay-harness.sh <scenario> [out dir]` runs the real daemon and overlay on a headless
KWin, attaches the debug controller (see [Debug mode](architecture.md#debug-mode)), runs a
scenario script, and records everything to `screenshots/harness/` (or `out dir`).

```sh
scripts/overlay-harness.sh scripts/harness/directional-pick.sh
scripts/harness/analyze.py            # or: analyze.py -v for every frame
```

It needs `kwin_wayland`, `dbus-run-session`, `bwrap` and `python3`, and builds the debug binary
first.

### What it keeps away from your desktop

The daemon in the harness can't touch your real session:

- It gets its own `XDG_RUNTIME_DIR`, so it can't reach the real daemon's socket, the desktop's
  session bus or the sound server (menu sounds go nowhere).
- It runs under `bwrap` with an empty `/dev/input` and every `/dev/hidraw*` hidden, so it can't
  take your controllers. The debug controller reaches it over the socket, not through
  `/dev/input`.
- It gets a copy of a config folder, never your own. The default is
  `scripts/harness/config`. In it, holding **South** opens a directional menu (`Directional`,
  on the D-pad) and holding **East** opens a radial one (`Radial`, aimed with the right stick).
  Set `PADWIGHT_HARNESS_CONFIG=<dir>` to use another folder.
- Without `--live`, nothing the debug controller does reaches the real virtual keyboard, mouse
  or pad.

### Writing a scenario

A scenario is a bash script, sourced once the controller is attached. It has three helpers:

| Helper | What it does |
|---|---|
| `pad <command>` | Runs `padwight debug <command>` against the session: `pad press south`, `pad tap up 40`, `pad stick right 0 -1`, `pad release east`. `padwight debug help` lists the commands. |
| `mark <text>` | Writes `mark <ns> <text>` into `overlay.log`, so the analysis can split the run into rounds. |
| `shot <name>` | Saves a screenshot as `<name>.png`. Use it to check layout only (see [Limits](#limits)). |

Leave time for the overlay to react (`sleep 0.5` and up). Each `pad` command returns once the
daemon has handled the input, but the overlay draws after that. The scenarios in
`scripts/harness/`:

- `directional-pick.sh`: opens the directional menu and picks its top slot, ten times.
- `radial-open-close.sh`: opens and closes the radial menu, then aims and picks, five times.

### What it records

| File | What's in it |
|---|---|
| `frames.log` | Every frame the daemon sent, as `<ns> <frame JSON>`, read from the daemon's socket as a second overlay watcher. |
| `overlay.log` | The overlay's [timing log](#the-timing-log), plus the scenario's `mark` lines. |
| `daemon.log` | The daemon's output. |
| `session.log`, `kwin.log` | The debug session's and KWin's output, for when a run fails. |

`frames.log` and `overlay.log` share a clock (nanoseconds since the Unix epoch). Comparing them
shows what the daemon sent and what the overlay got: a frame that's in `frames.log` but has no
`recv` in `overlay.log` was lost on the way.

`scripts/harness/analyze.py [overlay.log]` sums up `overlay.log` for each `mark`:

- the frames the overlay received;
- the longest gap between two builds, and the window size it laid out at;
- how soon after opening it drew the first frame part way into the fade;
- for a menu closed by a pick, how many frames it held the pick fully shown, and the strongest
  flash.

With `-v` it also lists every build and every drawn menu under its mark.

### Limits

- **Screenshots lag.** KWin's virtual screen shows the overlay's frames late and updates parts
  of them at a time, so a `shot` taken during a fade doesn't show what the overlay drew. To see a
  fade, use the [fade frames](#fade-frames).
- **The first menu stalls.** In every run so far, the first menu after the overlay starts is
  laid out at 1×1, then nothing more happens until the next menu opens. Its other frames arrive
  only then. So round 1 of a scenario shows nothing useful. Whether this happens on real
  hardware too isn't known yet.
- **The rendering path isn't yours.** KWin's virtual screen doesn't render the way a real GPU
  does, so the timing it shows is close but not exact.

## The timing log

When `PADWIGHT_OVERLAY_TIMING` names a file, the overlay process appends a line to it for every
frame it receives, every time it builds its widgets, and every menu it draws:

```
recv <ns> menu "Directional" selected=Some(0) picks=1
built <ns> <build ns> 1920x1080
menu <ns> "Directional" picks=1 selected=Some(0) opacity=1.00 lit=1.00 flash=0.80 since=open:0.517,closed:Some(0.075),moved:0.013,picked:0.013
```

- **`recv`:** the frame's menu title, cursor and pick count; `none` when nothing is shown, or
  the overlay's kind for the keyboard, media controls and the offer.
- **`built`:** how long the build took, and the window size it laid out at. A build at `1x1`
  happened before the window grew to the whole screen.
- **`menu`:** the menu's motion values in that build, and how many seconds ago it opened,
  closed, moved its cursor and was picked from. A `closed` value means the menu is fading out.

The overlay opens the file on every line, so the log is for measuring only, not for normal use.
The code is in `src/overlay/timing.rs`.

To log your real overlay, start the daemon with the variable set; the overlay process inherits
it. Stop the running daemon first:

```sh
PADWIGHT_OVERLAY_TIMING=$PWD/screenshots/overlay.log padwight daemon
```

`scripts/harness/analyze.py` reads this log too. Without `mark` lines it has no rounds to split
by, so add some by hand, for example `echo "mark $(date +%s%N) pick" >> screenshots/overlay.log`.

## Fade frames

The fade frames test draws menus part way through their fade-in, in-process, with the renderer
the overlay itself picks: wgpu on a machine with a GPU, otherwise tiny-skia (the file names end
in `-wgpu` or `-tiny-skia`). No compositor is involved, so each picture is exactly what the
overlay would draw at that point.

```sh
mkdir -p screenshots/fade
PADWIGHT_FADE_FRAMES=$PWD/screenshots/fade cargo test fade_frames -- --ignored
scripts/harness/fade-check.py screenshots/fade
```

It draws six scenes: a radial menu, the same menu with an item aimed at, the radial menu as boxes,
directional menus on the D-pad and the face buttons, and a list. Each is drawn at 10%, 25%,
50%, 75% and 100% opacity under the Subtle motion style, over black and over light grey. The
files are named `<scene>-<background>-<opacity>-<renderer>.png`. To add a scene, edit
`scenes()` in `src/overlay/fade_frames.rs`.

### Reading the check

A menu faded as one picture follows a simple rule: at opacity `o`, each pixel is the background
plus `o` of the way to that pixel fully shown. `scripts/harness/fade-check.py` compares every
picture with that rule and prints, per scene, background and opacity:

- how many pixels are more than 12 levels (of 255) off;
- the worst of them;
- the box they fall in.

Two kinds of fault show up, and the two backgrounds tell them apart:

- **A part that ignores the fade** is drawn at full strength from the first frame, so it shows
  up even on black. For example, a color with a fixed alpha (`Color { a: 0.35, .. }`) instead
  of one scaled from the faded color (`c.scale_alpha(0.35)`). Over black, the worst value should
  stay low, around 20 to 50.
- **Layers stacking.** Each widget fades on its own: the panel, an item, its label. Where they
  overlap, the result looks more solid than the rule says. This barely shows over black, but
  over light grey it covers large areas. iced can't fade a group of widgets as one, so this only
  goes away for parts drawn into a single image, such as the radial menu's ring, which is one
  SVG faded with the widget's own opacity.

The check doesn't fail the build. It's for comparing before and after a change: render the
frames on both versions, then run the check on each folder.

## Motion runs

`padwight debug motion` plays a scripted run of an overlay in one or more motion styles and
draws every moment of it in-process, with the overlay's own layer code (`overlay::film` in
`src/overlay/ui.rs`) and the CPU renderer. No display, compositor, daemon or controller is
involved, and the clock is made up, so every run draws the same frames.

```sh
cargo run -q -- debug motion playful              # one style
cargo run -q -- debug motion all                  # every style but off
cargo run -q -- debug motion swift dreamy --scene keyboard
```

The run is the same for every style:

| Time (s) | What happens |
|---|---|
| 0 | The overlay opens, the cursor on the first item (on the keyboard, H). |
| 0.55 | The cursor moves to the second item. |
| 0.7 | It moves to the third, before the last glide has finished. |
| 1.15 | A pick that leaves the overlay open. |
| 1.6 | The cursor moves to the fourth item. |
| 2.0 | A pick that closes a menu: it holds the picked item for 0.3 s, then plays its close. The keyboard just lets the key go and closes. |

The run ends 0.15 s after the close has played. For each style it writes two files to
`screenshots/motion/`, named `<scene>-<style>`:

- **`<scene>-<style>.png`:** an animated PNG of the whole run, looping. Open it in a browser
  (most image viewers show only its first frame).
- **`<scene>-<style>-strip.png`:** frames side by side, six to a row, left to right and top to
  bottom, for looking at one moment against the next. By default it covers the whole run,
  every 40 ms.

Every picture is cropped to the area anything was drawn in during the run, so a panel that
slides or grows stays in frame and every frame lines up.

| Option | What it does |
|---|---|
| `--scene <name>` | What to play: a menu scene from `padwight debug overlay list` (default `menu-list`), `keyboard` or `numpad`. |
| `--from <s>`, `--until <s>` | The part of the run the strip covers, in seconds. For example `--until 0.6` for the opening, or `--from 1.98 --until 2.62` for the pick and close. |
| `--every <ms>` | Time between the strip's frames (default 40). |
| `--fps <n>` | The animated PNG's frame rate (default 50). Lower it to draw faster. |
| `--size <w>x<h>` | The virtual screen (default 960x540). |
| `--out <dir>` | Where the files go (default `screenshots/motion`). |
| `--trace` | Prints the numbers the drawing reads instead of drawing anything. |

`--trace` prints one line every 20 ms: the overlay's `opacity`, its offset `x` and `y` in
pixels, its `scale`, how `lit` the cursor's item is, that item's own scale (`item`), the pick's
`flash`, the first item's fade (`fade0`), and how far the first and fourth items are still off
their places (`shift0`, `shift3`). Once the overlay has finished closing, the line says
`(gone)`. The trace is quick, so check numbers there first (a spring's overshoot, when a style
settles) and draw only to see how it looks.

Drawing is slow, about 0.2 s a frame. A whole run at the default 50 fps takes about half a
minute per style, so pick the styles you need, lower `--fps`, or run several styles as separate
commands at once. The radial menu's items are one SVG picture, so it shows the panel's motion
but not the items' own (their pop and slide).

## Screenshots and previews

### Running the app headless

`scripts/headless.sh [padwight arguments...]` runs any `padwight` command (the settings window
by default) on a virtual display, with its own config, state, runtime and temp folders. Your
desktop session and daemon are left alone.

```sh
scripts/headless.sh                       # the settings window
scripts/headless.sh overlay               # the overlay process, on its own
PADWIGHT_HEADLESS_KEEP=1 scripts/headless.sh gui
```

| Setting | What it does |
|---|---|
| `PADWIGHT_HEADLESS_BACKEND=kwin` | The default. KWin's own Wayland session on a virtual output: the overlays are layer-shell surfaces and show as they do on KDE. Needs `kwin_wayland` and `dbus-run-session`. |
| `PADWIGHT_HEADLESS_BACKEND=xvfb` | An X server, for the settings window only: Xvfb has no layer-shell, so the overlays can't show. Needs `xvfb-run`. |
| `PADWIGHT_HEADLESS_SIZE=1920x1080` | The virtual screen's size, for KWin. |
| `PADWIGHT_HEADLESS_SCREEN=1920x1080x24` | The virtual screen's size and color depth, for Xvfb. |
| `PADWIGHT_HEADLESS_KEEP=1` | Keeps the folders afterwards and prints where they are. |

Neither backend starts a daemon. The script sets `PADWIGHT_NO_DAEMON`, which stops the settings
window starting one as it usually does: that daemon would outlive the virtual display and take
the real controllers. For the window to talk to a daemon, start one with the same
`XDG_RUNTIME_DIR`, as `tests/gui_smoke.rs` does, or use the [harness](#the-harness).

### Menu screenshots

`scripts/menu-screens.sh [out dir]` draws a set of menu scenes with the real overlay on a
headless KWin and screenshots each one, into `screenshots/menus/` by default. No daemon runs:
a stand-in on the overlay's socket hands it one frame per scene and captures the screen once it
has settled, so these are stills, not motion. It needs `kwin_wayland`, `dbus-run-session`,
`spectacle` and `python3`, and takes `PADWIGHT_HEADLESS_SIZE` like `headless.sh`.

The scenes come from `scenes()` in `src/menu/suite/screens.rs`: merged grid cells, a start
cell, submenus one and two deep, each menu kind's default close button, and a touchpad menu.
The script gets them by running the ignored test `menu::suite::screens`, which writes each
scene's frame to the file `PADWIGHT_SCREEN_FRAMES` names. To add a scene, add it to `scenes()`.

### Controller drawings

The ignored test `pad_svg::tests::dump` writes every controller model's drawing, with the
letters the settings window draws over it, to `padwight-pad-svg/` in the temp folder:

```sh
cargo test --bin padwight pad_svg::tests::dump -- --ignored
```

`padwight debug svg [model]` prints one model's drawing at rest, without the letters.
