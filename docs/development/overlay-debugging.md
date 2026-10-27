# Debugging the overlay

Tools for seeing what the on-screen overlay does, frame by frame, without a monitor or a
controller. They answer these kinds of question:

- **Timing:** did the overlay get the frame the daemon sent, and when did it draw and present
  it? Use the [harness](#the-harness) and the [timing log](#the-timing-log).
- **Looks:** what does a menu look like part way through a fade? Use the
  [fade frames](#fade-frames).
- **Motion:** how does a motion style open, move, pick and close? Use the
  [motion runs](#motion-runs).
- **Stills:** how does a screen look, start to finish? Use the
  [screenshot and preview tools](#screenshots-and-previews).
- **Against iced:** does the overlay's own renderer draw a scene the way the old iced overlay
  did? Use the [compare tool](#compare-with-iced).

All output goes under `screenshots/`, which git ignores.

## How the overlay draws

The overlay process draws with its own renderer, `src/overlay/render/`. It has no iced in it.
A frame from the daemon goes through five stages, each in its own part of the code:

1. **Layout tree** (`render/layout.rs`, `Node`). Each panel is a tree of rows, columns, wrapping
   rows, stacks, text and painted leaves. It is measured at a width and then placed in a box. The
   panels' trees are built in `render/menus.rs`, `radial.rs`, `keyboard.rs`, `info.rs`, `log.rs`,
   `media.rs`, `figure.rs`, `item.rs` and `panel.rs`.
2. **Display list** (`render/op.rs`, `Op`). Placing a tree emits a flat list of rects, shadows,
   text runs, paths, pictures, clips and layers. Lists are plain data: they compare equal, and
   have a hash (`op::digest`). `render::screen_ops` (`render/screen.rs`) builds the whole screen's
   list, with each panel as one layer placed where it rests (`render/place.rs`).
3. **Panel pictures** (`render/screen/pictures.rs`, `tiles.rs`, `diff.rs`, `lay.rs`). A panel's
   list is drawn at full strength into its own picture, and the picture is kept while the list
   stays the same. A fade, slide or zoom only lays the picture down again, at its opacity, offset
   and scale, so every part of a panel fades by the same amount. When a list changes (a highlight
   gliding, an item fading in), `diff.rs` finds the ops that changed and only the 64-pixel tiles
   they reach are drawn again (`tiles.rs`, `under.rs`).
4. **Damage** (`render/screen/damage.rs`, `items.rs`, `repaint.rs`). `Screen::paint` works out the
   device pixels that changed since the buffer it draws into was last drawn. The buffer's age
   says how many frames back that was; the last four frames' damage is kept. Only those patches
   are cleared and drawn again.
5. **Window.** On Linux, `overlay/wayland/picture.rs` paints each buffer the compositor has
   released through `Screen::paint`. On Windows, `overlay/windows.rs` paints its DIB section the
   same way and gives the damage's bounds to `UpdateLayeredWindowIndirect`.

The rasterizer (`render/raster.rs` and `raster/`) draws the ops with tiny-skia, SVGs with resvg,
and text with cosmic-text (`render/text.rs`). Shadows, gradients and borders follow iced
tiny-skia's maths, so the numbers from `overlay/draw/` carry over. `render/screen.rs`, `pictures.rs`
and `damage.rs` start with module docs that say more.

The iced versions of the same panels are still in `overlay/draw/`. The settings window's
previews, the stage editor and the iced side of `debug overlay compare` use them. Until the
settings previews move to the new renderer (plan step 9.1), a change to how a panel looks has to
be made in both.

## Compare with iced

`padwight debug overlay compare` draws each scene twice: with iced (`overlay::snapshot_iced`) and
with the new renderer (`overlay::render::screen`). Both are settled, with motion off.

```sh
cargo run -q -- debug overlay compare all                 # every scene
cargo run -q -- debug overlay compare menu-list radial    # some scenes
cargo run -q -- debug overlay compare all --out <dir>     # default screenshots/compare
```

It takes the same options as `debug overlay` (`--size`, `--scale`, `--background`). For each
scene it writes `<out>/<name>.png`: iced on the left, the new renderer in the middle, and the
difference on the right. The difference is each channel's |iced − new| times 8, so a small
difference shows.

The report prints one line per scene, worst first:

```
scene                       over     mean   max  path
```

- **`over`:** the share of pixels where some channel (alpha included) differs by more than 8
  levels. This is the number to watch.
- **`mean`:** the average over all pixels of the largest channel difference, in levels.
- **`max`:** the largest difference anywhere, in levels.

Scenes come out "size differs" when the two pictures aren't the same size. Scenes the new
renderer can't draw are listed last under "not ported"; right now that is only `glyphs`, the glyph
sheets, which are still drawn by iced alone.

To read a difference, open the PNG:

- Differences only at letter edges and outlines are the text and anti-aliasing. The plan
  expects those, and a few levels per pixel.
- A whole box or area that differs points at a color or alpha. Compare the two in the same place
  in `overlay/draw/` and `render/`: both read the same `Style` and `Look`, and
  `draw/look/parity.rs` checks the resolved colors for the default and decorated styles.
- Grain in the filter scenes (`menu-filters`, `menu-plaid`) and the frame marks at rounded corners
  differ from iced for known reasons; the plan's step rows 3.4 and 4.1 say which.

## Fade frames

`cargo test fade_frames` draws six menus part way through a fade-in, with the overlay's own
renderer, and checks them. It needs no `--ignored` flag and no display.

The six scenes are a radial menu, the same menu with an item aimed at, the radial menu as boxes,
directional menus on the D-pad and the face buttons, and a list. Each is drawn at 10%, 25%, 50%,
75% and 100% opacity (set directly, with no motion), over black and over light grey. The test
asserts that every pixel is its full-strength color blended over the background by the same
share, within 2 levels. A part that ignores the fade, or parts that stack their transparency, fail
it.

To keep the pictures:

```sh
mkdir -p screenshots/fade
PADWIGHT_FADE_FRAMES=$PWD/screenshots/fade cargo test fade_frames
scripts/harness/fade-check.py screenshots/fade
```

The files are named `<scene>-<background>-<opacity>.png`, the opacity as a percentage with
three digits (`radial-black-010.png`). `scripts/harness/fade-check.py` reads them and, for each
scene and background, prints the pixels more than 12 levels off the blend, the worst of them, and
the box they fall in. It is for looking at a change in the pictures; the test is the check that
fails. To add a scene, edit `scenes()` in `src/overlay/fade_frames.rs`.

## Motion runs

`padwight debug motion` plays a scripted run of an overlay in one or more motion styles and
draws every moment of it in-process, with the overlay's own renderer (`overlay::film` in
`src/overlay/ui/shots.rs`). No display, compositor, daemon or controller is involved, and the clock
is made up, so every run draws the same frames.

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

## The harness

`scripts/overlay-harness.sh <scenario> [out dir]` runs the real daemon and overlay on a headless
KWin, attaches the debug controller (see [Debug mode](architecture.md#debug-mode)), runs a
scenario script, and records everything to `screenshots/harness/` (or `out dir`).

```sh
scripts/overlay-harness.sh scripts/harness/directional-pick.sh
scripts/harness/analyze.py            # or: analyze.py -v for every frame
```

It needs `kwin_wayland`, `dbus-run-session`, `bwrap` and `python3`, and builds the debug binary
first. Set `PADWIGHT_HARNESS_RELEASE=1` to build and run the optimized binary; use it for any
figure about CPU or memory.

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
- `pacing.sh`: opens the radial menu, sweeps its cursor round, holds it still and closes it, five
  times. Its rounds are marked `round N open|move|hold|close|idle`.
- `radial-sweep.sh`: holds the radial menu open while the cursor goes round and round without
  stopping, for about 5 s.
- `resources.sh`: hidden, a menu held, a menu being moved through, and hidden again.
- `scale.sh`: sets the first output's scale with `kscreen-doctor`, runs `pacing.sh`, then sets
  the scale back to 1. Needs `SCALE` (for example `SCALE=1.5`) and `kscreen-doctor`.

### Measuring

Each figure has its own command. Run the harness with `PADWIGHT_HARNESS_RELEASE=1` and a folder
of its own for each run.

```sh
# Frame pacing: intervals, missed refreshes, daemon-to-present, fade steps
PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh scripts/harness/pacing.sh screenshots/pacing/<name>
python3 -I scripts/harness/pacing.py screenshots/pacing/<name>/overlay.log           # --hz 60 by default

# Surface: every buffer committed at the size last configured, none at 1x1 while showing
python3 -I scripts/harness/surface.py screenshots/pacing/<name>/overlay.log [scale]

# Resources: CPU (share of one core), resident memory, and the memory's kinds
PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh scripts/harness/resources.sh <out>
PADWIGHT_HEADLESS_SIZE=3840x2160 PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh scripts/harness/resources.sh <out>

# Radial sweep: CPU while the cursor moves round without stopping
PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh scripts/harness/radial-sweep.sh <out>

# Scale: buffers at 1, 1.5 and 2 (kscreen-doctor sets the scale for the run)
SCALE=1.5 PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh scripts/harness/scale.sh <out>
python3 -I scripts/harness/surface.py <out>/overlay.log 1.5
```

`resources.sh` writes `resources.txt` in the out dir: one line per phase (hidden, menu-held,
navigating, hidden-after) with the overlay's and KWin's CPU and the overlay's RSS, then the
`smaps_rollup` lines (Pss, Pss_Anon, Pss_File, Pss_Shmem) that split the memory into the
overlay's own and the files it maps in, such as the graphics driver. `radial-sweep.sh` writes the
same file with phases hidden, radial-held, sweeping and hidden-after.

`pacing.py` prints the frame intervals against the refresh (median, p95, max and missed refreshes
per run), the frames shown twice or more, presents with no new build since the last one, the time
from a daemon frame to its first present, and how evenly a fade steps. `surface.py` prints each
round's configure and first commit, the commits made while showing, the buffer sizes it saw, and
`violations: none` when nothing broke the rules.

### What it records

| File | What's in it |
|---|---|
| `frames.log` | Every frame the daemon sent, as `<ns> <frame JSON>`, read from the daemon's socket as a second overlay watcher. |
| `overlay.log` | The overlay's [timing log](#the-timing-log), plus the scenario's `mark` lines. |
| `daemon.log` | The daemon's output. |
| `session.log`, `kwin.log` | The debug session's and KWin's output, for when a run fails. |
| `resources.txt`, `outputs.log` | From `resources.sh` / `radial-sweep.sh`, and from `scale.sh` (the outputs before and after). |

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
- **The rendering path isn't yours.** KWin's virtual screen doesn't render the way a real GPU
  does, so the timing it shows is close but not exact.
- **Only KWin is covered.** The harness runs one virtual output under KWin. Other compositors
  (Sway, Hyprland), full-screen games and real displays are checked by hand (plan step 8.3).

## The timing log

When `PADWIGHT_OVERLAY_TIMING` names a file, the overlay process appends lines to it: one for each
frame it receives, each buffer it draws and commits, and each menu it draws. The harness sets the
variable for the overlay. To log your real overlay, start the daemon with it set (the overlay
process inherits it). Stop the running daemon first:

```sh
PADWIGHT_OVERLAY_TIMING=$PWD/screenshots/overlay.log padwight daemon
```

The overlay opens the file on every line, so the log is for measuring only, not for normal use.
The code is in `src/overlay/timing.rs`, with the lines written where they happen (`ui.rs`,
`wayland/`, `windows.rs`). Times are nanoseconds since the Unix epoch unless a field says
otherwise. An example from a menu's opening:

```
recv 1793100000123456789 menu "Directional" selected=Some(0) picks=1
built 1793100000124000000 812000 1920x1080 list 90000 paint 700000 pictures 1 tiles 0 diff 40000 draw 600000
menu 1793100000124000000 "Directional" picks=1 selected=Some(0) opacity=0.42 lit=1.00 flash=0.00 since=open:0.012,closed:None,moved:0.012,picked:0.000
commit 1793100000124100000 1920x1080 Draw Screen slot 0 age 0 damage 1 2073600 buffer 1920x1080 Buffer(1)
present 1793100000124100000
```

The lines, in the order a frame goes through them (the fields are as the code prints them):

| Line | When | Fields |
|---|---|---|
| `recv <ns> <what>` | A frame arrives from the daemon. | `<what>` is `menu "<title>" selected=<Option> picks=<n>` for a menu, `none` when nothing shows, and otherwise the overlay's name for what it shows. |
| `built <ns> <total ns> <w>x<h> list <ns> paint <ns> pictures <n> tiles <n> diff <ns> draw <ns>` | Linux: a buffer is painted. | `<total ns>` is the whole paint. `<w>x<h>` is the buffer in device pixels. `list` is the time to build the display list, and `paint` the time to draw it into the buffer. `pictures` and `tiles` count the panel pictures drawn afresh and the tiles drawn again, both since the overlay started. `diff` and `draw` are the time spent diffing lists and drawing tiles since the last build. |
| `built <ns> <total ns> <w>x<h>` | Windows: a frame is painted into the DIB. | As above, with no breakdown. |
| `menu <ns> "<title>" picks=<n> selected=<Option> opacity=<f> lit=<f> flash=<f> since=open:<s>,closed:<Option>,moved:<s>,picked:<s>` | Each menu in each screen drawn. | The motion values of that screen. The `since` values are seconds ago: when it opened, when it started closing (`None` if it isn't), when its cursor last moved and when it was last picked. |
| `surface <ns> ask <Corner\|Screen> <w>x<h>` | Linux: the surface asks for a size. | `Corner` is 1×1 in a corner, `Screen` the whole screen. A `0` side means the compositor chooses. |
| `configure <ns> <w>x<h> <target> <next>` | Linux: a configure is acked. | `<next>` is what the loop does next: `Nothing`, `Grow`, `Shrink {..}`, `Blank` or `Draw((w, h))`. |
| `commit <ns> <w>x<h> <Draw\|Clear\|Blank> <target> slot <n> age <n> damage <patches> <pixels> buffer <dw>x<dh> <Viewport\|Buffer(n)>` | Linux: a buffer is committed. | `<Draw\|Clear\|Blank>` is why: a frame, the clear that shrinks the surface, or the clear for hiding. `slot` and `age` identify the buffer and how many frames back it is. `damage` is the patches committed and their total pixels. The last field is the scale mode. |
| `commit-unchanged <ns> <w>x<h> <why> slot <n> age <n>` | Linux: a frame came out the same as the screen. | Nothing is committed, and no callback is asked for. |
| `commit-skipped <ns> <w>x<h> <why> held <n>` | Linux: every buffer is held by the compositor. | The frame is drawn on the next release. |
| `commit-refused <ns> <w>x<h> <why>` | Linux: a buffer at a size the surface wasn't last configured to. | Shouldn't happen. A debug build asserts. |
| `pool <ns> <dw>x<dh> <format>` | Linux: a new shared-memory pool is made. | Made when the size, scale or format changes. |
| `present <ns>` | Linux: after each committed frame. | The same time as its `commit` line. |
| `callback <ns> <compositor ms> current\|stale` | Linux: a frame callback is done. | `stale` means it answers an older commit. |
| `shm-format <ns> <format>`, `scale <ns> <n>/120`, `scale <ns> buffer <n>` | Linux: the compositor offers a format or a scale. | |

`analyze.py`, `pacing.py` and `surface.py` read these lines. A `menu` line is written for each
menu each time the screen is built, so during a fade there is one per frame.

## How to debug a look that differs from iced

1. Compare the scene: `cargo run -q -- debug overlay compare <scene>`. Open
   `screenshots/compare/<scene>.png` and see where the difference is (see [Compare with
   iced](#compare-with-iced)).
2. Find the panel. The iced version is in `overlay/draw/` and the new one in `render/`, usually
   with the same name (for example `draw/menus.rs` and `render/menus.rs`). Check that both
   use the same numbers for sizes, gaps and font sizes. The numbers are meant to be copied, not
   recalculated.
3. Check the colors and style. `draw/look/parity.rs` compares the resolved styles. If the colors
   match and the picture doesn't, the difference is in the drawing (`render/raster/`).
4. Check the fade and the motion. A look that is right settled but wrong part way through is
   usually a layer or an opacity: run `cargo test fade_frames` (with `PADWIGHT_FADE_FRAMES`
   for the pictures), or `padwight debug motion <style> --trace` for the numbers.
5. Check the repaint. A look that is right when drawn afresh but wrong after something moves is
   a damage problem. `render/screen/repaint.rs` tests that a damaged redraw equals a full one;
   run `cargo nextest run overlay::render::screen` to check.
6. Look at the saved pictures of each panel. The ignored tests write them to
   `screenshots/render/` (for example `every_menu_scene_drawn_as_a_framed_panel`,
   `every_info_scene_drawn_as_an_info_panel`, `every_radial_scene_drawn`); run one with
   `cargo nextest run --run-ignored ignored-only <name>`.

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
