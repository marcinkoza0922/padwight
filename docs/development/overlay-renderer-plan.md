# Plan: the overlay's own renderer

A plan for drawing the overlays (menus, the keyboard and numpad, media controls, offers, info
and log panels) without iced: our own layout, drawn on the CPU, on a Wayland surface we drive
ourselves. The settings window keeps iced. Work happens on the branch `native-overlay`.

## Why

Two problems with the overlay as it is:

1. **It flickers and draws in stages.** Four things cause it:
   - **iced fades each widget on its own.** Part way through a fade, overlapping boxes,
     outlines and labels stack their transparency, so outlines and boxes show before their
     labels. The `whole-panel-fade` prototype works around this for some panels, by drawing the
     panel off-screen while it fades. It doesn't cover info or log panels, and it needed
     workarounds for wgpu's late uploads, blending that differs between tiny-skia and wgpu, and
     sub-pixel alignment.
   - **Frames run on a 16 ms timer** (`overlay/layershell.rs`), not on the compositor's frame
     callbacks. They drift against the display, so some frames show twice and others are
     dropped.
   - **The surface grows from 1×1 when something opens.** The first frame can be laid out at
     1×1 (the harness shows the first menu stalling this way).
   - **Buffers are iced's and wgpu's to present and reuse**, so we can't control when or how.
2. **It uses a lot of memory.** Most of it is the GPU driver, in a process that sits hidden
   nearly all the time.

### Baseline

Measured on 2026-10-26 with `PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh
scripts/harness/resources.sh` (headless KWin, 1920×1080, three runs each, at commit `909ea81`).
CPU is the share of one core.

| | iced on wgpu (today) | iced on tiny-skia (`ICED_BACKEND=tiny-skia`) |
|---|---|---|
| Memory, hidden | 193 MB | 22 MB |
| Memory, menu shown | 216 MB | 45–47 MB |
| CPU, hidden | 0.4% | 0.0% |
| CPU, menu held still | 0.4–0.6% | 0.0% |
| CPU, moving through a menu | 1.2–1.4% | 4.2–5.2% |

About 170 MB is the graphics driver. Drawing on the CPU removes it, but costs more while
things move: iced redraws its whole widget tree on every tick.

### Baseline: frame pacing

Measured on 2026-10-26 with `PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh
scripts/harness/pacing.sh <out>` then `python3 -I scripts/harness/pacing.py <out>/overlay.log`
(headless KWin, 60 Hz, 1920×1080, three runs each, five rounds per run of open, sweep the radial
menu's cursor, hold still, close; at commit `909ea81` plus the `present` log). For tiny-skia run
with `ICED_BACKEND=tiny-skia`.

| | iced on wgpu (today) | iced on tiny-skia |
|---|---|---|
| Frame interval, median / p95 | 16.2 / 17.2–17.3 ms | 16.2–16.3 / 17.3 ms |
| Frame interval, max | 18.3–19.5 ms | 23.6–49.0 ms |
| Intervals of 1.5 refreshes or more, per run of ~520 | 0 | 0, 2 and 4 (up to 5 refreshes missed) |
| Frames presented while held still | 0 | 0 |
| Presents with no new build (same content) | 10–12 of ~580 | 8–9 of ~570 |
| Frame from daemon to its first `present`, median / p95 | 0.2 / 0.3–0.8 ms | 0.2 / 0.3 ms |
| ... worst (the very first open after start) | 52 ms | 51 ms |
| Open fade: partly faded frames, step median, max/median | 12, 0.089, 1.03–1.15 | 11–12, 0.090, 1.2–1.9 |
| Close fade: partly faded frames, step median, max/median | 8, 0.135, 1.02–1.04 | 8, 0.135, 1.3–1.4 |

What it says: the 16 ms timer runs about 3% faster than a 60 Hz refresh (median 16.2 ms), so a
display locked to the refresh would drop about one frame in 30 (inferred; see below). Fades step
evenly on wgpu and less so on tiny-skia, where some frames take longer. A frame from the
daemon is drawn at the next redraw, within 1 ms.

Limits: a `present` line is stamped when iced starts the redraw (`RedrawRequested`, read from a
subscription, using iced's own instant), not at the commit. iced_layershell lays out, draws and
presents inside that one call and has no hook on the commit, so the buffer is committed the
time drawing takes later, about 1–4 ms in the logs. Intervals between stamps are good; what the
compositor shows is not in the log, so frames shown twice or dropped at the display are inferred
from the interval against the refresh. The fade steps use the opacity of the last `menu` line, in
the build just before the redraw.

Run: `PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh scripts/harness/pacing.sh
screenshots/pacing/<name>` (`--hz` on `pacing.py` for another refresh).

## Goals

- **Every panel fades, slides and zooms as one picture.** No stacked transparency and no
  staged drawing, for every kind of panel, info and log panels included.
- **Frames come when the compositor asks for them** (frame callbacks), with the animation clock
  read then. No fixed timer.
- **Nothing is shown at the wrong size.** The first frame after opening is drawn only once the
  compositor has given the surface its size.
- **Memory near the tiny-skia figure:** at most 25 MB hidden and 50 MB shown at 1080p.
  (Expect more at 4K, where each screen buffer is 33 MB. Buffers are freed while hidden.)
- **CPU while animating no higher than today's 1.2–1.4%**, through caching: a panel whose
  contents don't change is drawn once and only laid down again.
- **The same look.** Every style, ornament, filter, font and motion style draws as it does now,
  within a few levels per pixel. Text may differ slightly at its edges.
- **No GPU use at all in the overlay process.**

## Non-goals (for now)

- **The settings window's previews stay on iced** (`overlay/draw/`, the stage editor's click
  targets). Moving them onto the new renderer, so there is one source for how panels look, is
  a later, separate task (step 9.1).
- **No new looks or motion styles.** The motion maths in `motion/` stays as it is: the renderer
  only reads `Anim`.
- **No GNOME support.** Without wlr-layer-shell, the overlay keeps failing with a clear error,
  as now.

## Approach

```
OverlayFrame (from the daemon)
   │  ui.rs: Presence/Anim per panel, unchanged
   ▼
screen.rs ── for each panel: build a Node tree (layout.rs) ── place ──▶ display list (Vec<Op>)
   │                                                                      │ same as last frame?
   │                                                          yes: reuse its picture / no: raster.rs
   ▼                                                                      ▼
composite each panel's picture at its opacity, offset and scale ──▶ screen buffer (damage only)
   ▼
wayland.rs: wl_shm buffer, damage, commit, frame callback ──▶ compositor
```

- **Libraries:** tiny-skia (shapes, paths, masks, gradients), cosmic-text with swash (shaping
  and glyphs), resvg (SVG icons, frame marks, figures), and `image` (background pictures). All of
  them are already in the lock file through iced, so the vendored Flatpak sources don't change.
- **Layout:** a small tree of our own: row, column, wrapping row, stack, pinned children,
  fixed/shrink/fill sizes, padding, alignment, clipping. It is measured at a width, then placed.
  It only needs to do what the overlay's panels use, not everything iced does.
- **The display list** is flat, comparable data: rounded rects, shadows, text runs, paths,
  pictures, clips and layers. If a panel's list equals last frame's, its stored picture is
  reused. A list with an effect (an item's stagger fade, its bounce) draws that subtree as a
  layer of its own.
- **Panels are drawn at full strength**, into a picture as big as the panel plus its shadow.
  Their opacity, slide and zoom are applied only when the picture is laid onto the screen. At
  scale 1, panels are placed on whole device pixels so text doesn't shimmer as it slides.
- **Matching iced:** shadows use iced tiny-skia's formula (a rounded-box distance through a
  smoothstep), gradients use iced's angle-to-endpoints maths, borders are drawn inside the box,
  and line height is 1.3 times the font size, as in iced. The texture and frame-mark code moves
  into an iced-free module that both renderers call, so filters stay identical.
- **The window:** wayland-client directly, with zwlr_layer_shell_v1 on the overlay layer, no
  keyboard focus and an empty input region. Buffers are wl_shm (memfd), two or three of them,
  reused only once released, and drawn into directly when the compositor offers ABGR8888.
  Damage covers only what changed. Scale comes from wp_fractional_scale_v1 with wp_viewporter,
  else the surface's preferred buffer scale. One loop polls the Wayland socket and an eventfd
  that the thread reading frames from the daemon wakes.

## Steps

Each step is meant to be done, checked and committed on its own. **Model** is a suggestion:

- **Opus:** design work, or code where subtle mistakes (blending, timing, protocol state)
  are hard to spot later.
- **Sonnet:** porting or building to a pattern that's already set, with a clear reference to
  match.
- **Haiku:** mechanical edits, running commands, and docs.

Unless a step says otherwise, check it with `cargo check`, clippy (zero warnings) and the tests
of the module touched.

### Phase 0: groundwork

| # | Step | Model | Done when |
|---|---|---|---|
| 0.1 | Record the baseline. | done | The table above. |
| 0.2 | **Optional quick win, separate from the rewrite:** have the overlay process use iced's tiny-skia renderer, either by setting the backend before iced starts or through iced's settings. This alone cuts about 170 MB, and can go to main now. | done | On branch `overlay-tiny-skia`: 21.8 MB hidden, 47 MB shown. |
| 0.3 | Add a harness scenario and analysis for frame pacing. Log a `present <ns>` line for each buffer committed, then report the frame intervals, frames shown twice, the time from a frame arriving to its first draw, and how evenly a fade steps. Run it on today's overlay as the second baseline. | done | The second baseline table above; run it with `scripts/harness/pacing.sh` and `pacing.py`. |
| 0.4 | Add the dependencies: `cosmic-text = "0.15"`, `tiny-skia = "0.11"`, and `wayland-protocols` with `staging` (stashed as "native-overlay deps"). | done | `cargo check` passes and the lock file gains no new packages. |

### Phase 1: pieces both renderers share (iced-free)

| # | Step | Model | Done when |
|---|---|---|---|
| 1.1 | Move the texture generators (`draw/texture/materials.rs`, the pixel code in `patterns.rs` and `texture.rs`, `hash`/`smooth`/`blotch`, and `Shape::cut`) to a new `overlay/art/` module. Each returns a `Pattern { key, opacity, build }`, and `draw/texture` turns that into an iced element. | done | `art::Pattern` and one function per generator; `draw/texture.rs` has `layer` and `wash`. |
| 1.2 | Move the choice and order of a style's filters (`filtered::filters`) into `art` as a list of `Filter`s (pattern, or tint). `draw/filtered.rs` maps that list to elements. | done | `art::filters(style, items, clock) -> Vec<Filter>`, bottom up. |
| 1.3 | Move the frame marks' SVG (`frame::markup` and its designs) and the SVGs for info icons and play states into `art`, taking colors as `[u8; 3]` and an alpha. Thin wrappers stay for iced callers. | done | `art::frame::markup`, `art::icons::{info_svg, play_state_svg}`. |
| 1.4 | `font.rs`: resolve a font setting to a family name without iced (`family(name) -> Option<&'static str>`). Keep a list of the font files loaded, so each renderer can load them into its own font system. Read a file's family with fontdb, not iced's font system. `resolve` stays for iced. | done | `font::family` and `font::files`; families read with fontdb. |

### Phase 2: the renderer's core (`overlay/render/`)

| # | Step | Model | Done when |
|---|---|---|---|
| 2.1 | Colors: an `Rgba` type, `blend`, `text_on`, `paint`. The style resolved for drawing (today's `Colors` and `Look`): background, items, selected, muted, border, shadow, gradient, glow (pulse), bevel, item outline, corners, typography. | done | `render::{Rgba, blend, paint, text_on, Style, Look, Outline, Shadow, Type}`; `draw/look/parity.rs` checks them against `draw.rs`/`look.rs` for the default and decorated styles. |
| 2.2 | Text: a thread-local cosmic-text font system with the bundled fonts and the pack's (synced from 1.4), and a cache of shaped runs. Handles case, letter spacing, size scaling, bold, italic, underline, wrapping at a width, and line alignment. Glyphs are drawn from swash's masks and color glyphs at the device scale and sub-pixel position. | done | `render::text::{shape, draw, Spec, Shaped, Canvas}` with a per-thread engine (`text/engine.rs`, kept in step through `font::version`), runs and glyphs cached; sizes equal iced's paragraphs for every font, and the ignored `a_page_of_every_font_drawn_by_both` page (`screenshots/render/text-*`) matches iced pixel for pixel, within 1 level for see-through text. |
| 2.3 | The display list: an `Op` enum (rect, shadow, text, path, picture, svg, pattern, clip on/off, layer on/off) with equality, a hash and ink bounds. | done | `render::{Op, bounds, balanced, digest}` with `Point`, `Rect`, `Corners`, `Layer`, `Path`, `Fill`/`Linear` (iced's angle maths), `Stroke`, `Fit`, `Svg`/`Markup`, `PatternRef` and `RunKey`; tests for equality, hashes and bounds. |
| 2.4 | The rasterizer. Covers: rounded rects with borders inside them (as iced draws them), linear gradients (iced's angle maths), box shadows (iced's formula), paths and strokes, clip masks (rounded), and nested layers with opacity and scale about a point. SVG goes through resvg, cached by content and size, and pictures through `image`, fitted (cover, contain, stretch) and cached by path. | done | `render::raster::{draw, picture, Picture, forget}`: quads and shadows ported from iced's `draw_quad`, layers drawn at the scale they're seen at; pictures, SVGs and pattern parts kept in byte-budgeted caches. Golden PNGs in `raster/golden/` for each op; shadowed boxes match iced tiny-skia exactly (0 levels) at scales 1, 1.5 and 2. |
| 2.5 | The layout tree (`Node`): space, text, leaf painter, row, column, wrapping row, stack (first child sets the size), pinned children, fixed/shrink/fill sizes, max size, padding, alignment, clip, and effects (opacity, scale, shift) drawn as layers. Measured at a width, then placed into a rect, emitting ops. | done | `render::layout::{Node, Length, Padding, Pin, Painter, Align}`: `Node::{space, text, leaf, row, column, wrap, container, stack}` with builder methods, `measure(width)`, `place(rect, ops)` and `ops(at, width)`, following iced's flex, fill, shrink and compression where the panels rely on them; tests for spacing, alignment, fill sharing, stretching, stacks, pins, wrapping (rows, and text at a width), nested measurement, clips and effects. |
| 2.6 | A test helper that draws a node to an `Image` and saves it under `screenshots/render/`, for the panel steps to use. | done | `render::sample::{draw, save}` (test only): `draw(node, width, scale)` makes a picture of the node's ink with `raster::picture`, and `save(name, pixmap)` writes `screenshots/render/<name>.png`; a test draws a shadowed, rounded card with text and checks it saves. |

### Phase 3: the panels

Each panel step is checked the same way: draw every scene in `debug overlay list` that uses
that panel with both renderers, and compare (step 3.10 automates this). Port from the
`overlay/draw/` file named, keeping its numbers (sizes, gaps, font sizes) as they are.

| # | Step | Model | Done when |
|---|---|---|---|
| 3.1 | The panel's frame (`framed` in `draw.rs`): background or gradient (and its drift), border, shadow, background filters, picture, vignette, frame marks, clip to the rounded box, max size, and contents cut off past the max height. | done | `render::framed(body, &Style, &OverlayStyle, padding, &Fit) -> Node` (`render/panel.rs`, layers in `panel/ornament.rs`); unit tests for size, clip, cut-off and layer order; the ignored `every_ornament_and_filter_scene_drawn_as_a_framed_panel` saves `screenshots/render/panel-<scene>.png`, which match iced's backgrounds, filters, vignette and frames (the clip is rounded where iced's is square, so a mark at a rounded corner is cut). |
| 3.2 | Rich text (`marked.rs`, `glyphs.rs`, `rich_text`): text, button glyphs (discs and tags), the D-pad glyph, icons, `{icon:…}` pictures, bars, `{b}`, `{i}`, `{u}`, `{color}`, `{font}`, `{box}`, and wrapping between words. | Sonnet | The token scenes match. |
| 3.3 | Item boxes (`look.rs`, `item_box`): lit and flash blending, stagger fade, outline, glow and its pulse, bevel, filters on items, add/remove/note tones, color-blind tints. | Sonnet | The styled-menu scenes match, including part way through a glide. |
| 3.4 | Menus (`menus.rs`): list (one or two columns, ▲/▼ more, dividers, ▸ at the right), buttons, directional, grid with merged cells, carousel (and touch), crumbs, about text, hint, a prompt over the menu (dimmed under it), and the editor's figure beside it. | Sonnet | Every menu scene matches. |
| 3.5 | Radial menus (`radial.rs`): arcs as paths with the lines between them, filters cut to each arc, the hub, the aim dot, and the boxes variant. | Sonnet | The radial scenes match. |
| 3.6 | The keyboard and numpad (`keyboard.rs`): keys, cursor and pressed key, latched keys and Caps Lock, the closing bar, the legends, and the typed line or field name. A set width scales the keys. | Sonnet | The keyboard and numpad scenes match. |
| 3.7 | Info panels (a table with columns as wide as their widest cell, a title, wrapping held to the max width) and log panels (lines of cells, labels, hold times, faded released inputs). | Sonnet | The info and log scenes match. |
| 3.8 | The media and offer panels (`draw.rs`). | Haiku | Their scenes match. |
| 3.9 | The editor's figures (`figure.rs`): SVG pictures with captions and notes, and the live touchpad picture with its labels (`touchpad_widget.rs`: draw its SVG with our own text over it). | done | `render::{figure, touchpad}` (`render/figure.rs`): `figure(&Figure, &Style, scale) -> Node` and `touchpad(&PadPicture) -> Node`, labels from `PadPicture::labels` shared with iced; SVGs match iced exactly, text within a pixel; the ignored `every_editor_figure_drawn_in_a_framed_panel` saves `screenshots/render/figure-*.png`. |
| 3.10 | `padwight debug overlay compare`: draws each scene with both renderers and writes side-by-side PNGs and a difference score to `screenshots/compare/`. Can be built before 3.1 and used from then on. | done | `debug overlay compare` writes iced \| new \| difference PNGs to `screenshots/compare/` and lists scenes worst first (share over 8 levels, mean, max); unported scenes are listed as not ported; panels hook in at `draw_new` in `src/debug/overlay_compare.rs`. |

### Phase 4: the screen

| # | Step | Model | Done when |
|---|---|---|---|
| 4.1 | Port `ui::layers` to `render/screen.rs`: placing panels by position, margin, style offset and animation (`place.rs`), the Guide overlay's row of Notes, menu and Mappings (`beside`), the stacks of info and log panels, the editor's backdrop, `peek`, and a prompt over a menu. | Sonnet | `debug overlay all` matches iced for whole screens. |
| 4.2 | Panel pictures: draw each panel's list once at full strength, reuse it while the list is unchanged (a few kept, least recently used dropped), and composite it at its opacity, offset and scale. Snap to whole pixels at scale 1, and filter smoothly while zooming. | Opus | A test where a fade's frames redraw nothing but the composite. The fade check reports even steps for every panel, info and log included. |
| 4.3 | Screen damage: the union of what each panel covered on the buffer before and what it covers now. Only that is cleared and redrawn. | Opus | A test: moving the cursor in a corner info panel touches only that corner. |
| 4.4 | Switch `overlay::snapshot` and `overlay::film` (used by `debug overlay` and `debug motion`) to the new renderer. Port the `fade_frames` test so it asserts that fades are even. | Sonnet | `debug motion all` strips look as before, without staged drawing. |

### Phase 5: the Linux window (`overlay/wayland.rs`)

| # | Step | Model | Done when |
|---|---|---|---|
| 5.1 | The surface. Bind the globals (compositor, shm, layer shell, fractional scale, viewporter) and create a layer surface on the overlay layer with no keyboard focus and an empty input region. It is 1×1 while hidden. On showing, it asks for the whole screen and draws nothing until that configure arrives. On hiding, it goes back to 1×1 once the last fade has been presented. A missing layer shell gives a clear error. | Opus | It runs in the harness, and the first frame of every round is drawn at full size. |
| 5.2 | Buffers: a memfd pool, two or three buffers reused only after `release`, drawn into directly in ABGR8888 or converted into ARGB8888, freed while hidden, and remade when the size or scale changes. | Opus | No frame is drawn into a buffer the compositor holds (checked by assertion). Memory hidden is within the goal. |
| 5.3 | Frame pacing: draw only on a frame callback, read the animation clock then, and request the next callback while anything moves. If callbacks stop (output off, overlay hidden behind something), a fallback timer of about 100 ms keeps pruning and closing going. | Opus | The pacing scenario (0.3) shows one draw per refresh while animating and no draws while still. |
| 5.4 | Scale: wp_fractional_scale_v1 with a viewport, else the preferred buffer scale, else 1. Redraw at the new scale when it changes. | Sonnet | Checked in the harness at 1, 1.5 and 2. |
| 5.5 | The loop. A thread reads frames from the daemon (blocking socket, as on Windows) and wakes the loop through an eventfd. The loop polls both, keeps only the newest of a burst of frames that `stands_in_for` the ones before it, and exits when the daemon goes away. | Sonnet | It runs with the daemon. Killing the daemon ends the overlay. |
| 5.6 | The timing log: keep the `recv`, `built` and `menu` lines and add `present`, so `analyze.py` and the pacing scenario keep working. | Haiku | `analyze.py` runs on a new log. |

### Phase 6: Windows

| # | Step | Model | Done when |
|---|---|---|---|
| 6.1 | Port `overlay/windows.rs` from iced's snapshot to the new renderer: the screen picture, premultiplied, converted to BGRA in the DIB section for `UpdateLayeredWindow`, damage only. Keep the 16 ms timer: layered windows have no frame callbacks. | Sonnet | `cargo check --target x86_64-pc-windows-msvc` passes. **It can't be run here; it needs testing on Windows.** |

### Phase 7: take iced out of the overlay process

| # | Step | Model | Done when |
|---|---|---|---|
| 7.1 | Delete `overlay/layershell.rs`, and with it the `iced_layershell` dependency (nothing else uses it). Check what that leaves of the pin comments in `Cargo.toml` (`winit-core`, `xkbcommon`, `wayland-*`). | Sonnet | It builds, and the lock file only loses packages. |
| 7.2 | Make sure the overlay process never starts iced's font system or renderer (fonts from 1.4, snapshots from 4.4). | Haiku | `resources.sh` memory is within the goal. |

### Phase 8: check and measure

| # | Step | Model | Done when |
|---|---|---|---|
| 8.1 | Re-run `resources.sh` at 1920×1080 and 3840×2160 (`PADWIGHT_HEADLESS_SIZE`) and add the figures to this doc. | Haiku | The goals for memory and CPU are met, or the gap is written down. |
| 8.2 | Re-run the pacing scenario (0.3) and the fade check, and compare them with the baseline. | Haiku | Written down here. |
| 8.3 | Test on real hardware: KWin, a wlroots compositor (Sway or Hyprland), a game running full screen, a display at fractional scale, and Steam's gamescope if it's used. | the user | Flicker gone, nothing staged, no stutter. |
| 8.4 | Docs: `overlay-debugging.md` (renderer, compare tool, pacing scenario) and `architecture.md` (the overlay process). | Haiku | Written. |

### Phase 9: later

| # | Step | Model | Done when |
|---|---|---|---|
| 9.1 | Draw the settings window's previews with the new renderer, shown as images, so a look is defined in one place. Needs click targets from the layout tree for the stage editor (`tag.rs`, `found_at`). Then delete `overlay/draw/`. | Opus | The GUI's previews and stage editor work as before. |

## Order and parallel work

Phase 1 and steps 2.1–2.5 come first. Steps 2.2, 2.4 and 2.5 can be done side by side once the
op types from 2.3 are fixed. In phase 3, step 3.1 comes first, and the rest can be done in any
order or side by side. Phase 5 needs only phase 4's composite: step 5.1 can start with a test
picture as soon as 2.4 is done. Phase 6 is independent once phase 4 is done. Step 0.2 can ship
to main at any time.

## Risks

- **The look drifts from iced** (text metrics, edge blending). Mitigated by porting numbers
  as they are, the compare tool (3.10), and the iced formulas named above. Until 9.1, there are
  two renderers to keep in line: a change to how a panel looks must be made in both.
- **CPU at 4K while animating.** Four times the pixels. Mitigated by panel pictures and damage
  (4.2, 4.3). Measured in 8.1.
- **Compositor differences:** whether ABGR8888 is offered (ARGB8888 conversion is the
  fallback), callbacks stopping while hidden (the fallback timer), and how fractional scale is
  rounded. Covered by 8.3.
- **Windows can't be run here.** Step 6.1 is only type-checked until someone tests it.
