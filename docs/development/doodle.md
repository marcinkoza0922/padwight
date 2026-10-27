# Doodle

A drawing tool over the game: the controller moves a cursor, and a pen, an eraser and a text tool
put marks on the screen. Doodles stay drawn over the game after the tool is closed, and they go
only when wiped.

## Entry and exit

- Doodle is a new entry on the **Tools** menu (Guide + right trigger), next to Calculator.
- Opening Doodle takes the controller, the same way the Tools menu does. The game gets no input
  while it's open.
- Leave Doodle by pressing **Guide**, or by **holding East**. Either hands the controller back.
  The doodles stay on screen.
- East is also undo, so exit by holding it. A tap undoes, and a hold of about half a second or
  longer exits. The hold time is not yet set (see open questions).
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

- **L3 + R3** clears every doodle and text box on screen.
- There is no other way to clear them, so re-entering Doodle mode doesn't clear anything.

## Controls

The left stick moves the cursor. **South** uses the current tool. **LB / RB** switch between
tools in the order Pen, Eraser, Text.

**East** (tap) undoes the last thing added (a stroke or a text box). It works in every tool.
**East** (hold) leaves Doodle mode.

### Pen

Hold South to draw: press to put the pen down, move the stick to draw, release to lift.

| Control | Action |
| --- | --- |
| D-pad left / right | Change color (12-color palette) |
| D-pad up / down | Change size |
| North | Cycle the tip shape: round, square, chisel, diagonal |
| West | Cycle the kind: solid, brush, spray paint |

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
set globally.

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

Not yet: the brush and spray looks beyond a fainter, wider stroke. Chisel and diagonal tips draw
with a butt end for now. The fonts are placeholders, all drawn in the system font, and the palette
is a placeholder too.

Decisions made while building:
- Positions are shares of the screen (0 to 1), as the magnifier's place is, so the canvas doesn't
  depend on resolution. Sizes are shares of the screen's height.
- East's hold time is 0.5 seconds. It leaves while held, and a shorter press undoes on release,
  so a long hold doesn't also undo.
- Undo takes back the last item added. Erased items are gone.

## Open questions

- **Palette**: the 12 colors are not yet chosen.
- **Fonts**: the list of about 8 fonts is not yet chosen, and it's unclear whether they ship with
  the app or come from the system.
- **L3 + R3 conflict**: it is a combo in normal play. Confirm whether the wipe should also work
  outside Doodle mode, and whether it should ask for confirmation first.
- **Undo depth**: one step, or a stack of several steps?
- **Pen sizes**: the range and step for D-pad up / down are not yet set.
