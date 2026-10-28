# Doodle

A drawing tool over the game: the controller moves a cursor, and a pen, an eraser and a text tool
put marks on the screen. Doodles stay drawn over the game after the tool is closed, and they go
only when wiped.

## Entry and exit

- Doodle is a new entry on the **Tools** menu (Guide + right trigger), next to Calculator.
- Opening Doodle takes the controller, the same way the Tools menu does. The game gets no input
  while it's open.
- Leave Doodle by pressing **Guide**, which hands the controller back.
  The doodles stay on screen.
- East is undo. A tap undoes, and a hold of about half a second or longer clears everything. The
  tray's last cell says so.
- Doodles are visible over the game at all times, in or out of Doodle mode.
- Doodles are session-only. They are kept in memory and are lost when the daemon restarts.

## Layering

From the bottom up: the game, then doodles, then the magnifier lens, then Padwight's overlays and
menus (Guide menu, Tools menu, the on-screen keyboard, the Doodle HUD, and the rest).

- Doodles are always below every Padwight overlay and menu. A menu never gets drawn under a
  doodle.
- The magnifier lens is also always below Padwight menus, so a menu opened over the lens covers it.
- The magnifier lens is drawn above doodles, so the lens shows its enlarged picture without
  doodles drawn across it.

## Wipe

- **Hold East** clears every doodle and text box on screen.
- There is no other way to clear them, so re-entering Doodle mode doesn't clear anything.

## Controls

The left stick moves the cursor. **South** uses the current tool. **LB / RB** switch between
tools in the order Pen, Eraser, Text.

**East** (tap) undoes the last thing added (a stroke or a text box). It works in every tool.
**East** (hold) clears every doodle and text box. **Guide** leaves Doodle mode.

### Pen

Hold South to draw: press to put the pen down, move the stick to draw, release to lift.

| Control | Action |
| --- | --- |
| D-pad left / right | Change color (12-color palette) |
| D-pad up / down | Change size |
| North | Cycle the tip shape: round, square, chisel, diagonal |
| West | Cycle the kind: solid, brush, spray paint |

The brush paints like a bristle brush in a paint program: bristles across the width, each a
little lighter or darker than the paint, spreading out from a narrower start, with the outer ones
running dry in streaks. The spray is an airbrush: fine dots scattered over a disc, thickest at the
middle, as many for each stretch as the area it sweeps, so a wide spray is as dense as a thin one.

### Eraser

Hold South over a stroke or a text box to delete the whole item it touches.

| Control | Action |
| --- | --- |
| D-pad up / down | Change the cursor's size (the area that counts as touched) |

The eraser has no color, tip, or kind options.

### Text

**South** on empty space creates a text box at the cursor and opens the on-screen keyboard.
**South** on an existing text box opens it for editing, so it never makes a second box on top of
one.

Text boxes can be moved with South (select, then move the stick) but not resized.

| Control | Action |
| --- | --- |
| D-pad up / down | Change font size |
| D-pad left / right | Change font color (same 12-color palette as the pen) |
| West | Cycle the font (fixed list of about 8 fonts) |
| North | Toggle italic, bold, underline |

Italic, bold and underline are toggled on the text box being created or edited. They are not
set globally. North opens them as a checklist standing just above the tray, over North's cell:
the D-pad moves between them, South ticks one, and North or East closes it.

## Colors

The pen and text use one fixed 12-color palette, left and right on the D-pad.
The exact colors are not yet chosen. See open questions.

## Status

Built: the canvas and tools (`src/tools/doodle.rs`, `doodle/screen.rs`), the Doodle entry on the
Tools menu, the daemon-held canvas, the pen, eraser and undo controls, the wipe, East's tap and
hold, the drawing in the overlay (under the lens and the panels), the carousel across the top
showing the current tool with its bumper hints, the tray along the bottom with each setting and its
hint, the cursor (always shown on the Doodle screen, with a minimum size),
and the text tool. South on empty space opens the on-screen keyboard for a new box; South on a box
grabs it, moving it with the stick, and lifting South without moving opens it for editing. Enter
sets the text, and closing the keyboard keeps what was there. An emptied box is removed.

Not yet: chisel and diagonal tips draw with a butt end for now, and the brush ignores the tip. The fonts are placeholders, all drawn in the system font, and the palette
is a placeholder too.

Decisions made while building:
- Positions are shares of the screen (0 to 1), as the magnifier's place is, so the canvas doesn't
  depend on resolution. Sizes are shares of the screen's height.
- East's hold time is 0.5 seconds. It clears while held, and a shorter press undoes on release,
  so a long hold doesn't also undo.
- Undo takes back the last item added. Erased items are gone.
- Performance: a stroke is drawn in runs of 32 segments, an op each, so as it grows only its
  tail is repainted. The overlay keeps the finished items' ops, and the daemon sends the finished
  items only when they change; the stroke being drawn goes with every frame. A spray is an op for
  each 8 segments instead, as it has many more shapes to a segment. A path's steps are shared
  between its clones with their bounds kept, so the frame's list is cloned, compared and measured
  each frame without walking every point of every doodle. Measure with the ignored
  `doodles::tests::bench_drawing_over_doodles` (`--cargo-profile release`).

## Open questions

- **Palette**: the 12 colors are not yet chosen.
- **Fonts**: the list of about 8 fonts is not yet chosen, and it's unclear whether they ship with
  the app or come from the system.
- **Clear confirmation**: whether the hold-East wipe should ask for confirmation first.
- **Undo depth**: one step, or a stack of several steps?
- **Pen sizes**: the range and step for D-pad up / down are not yet set.
