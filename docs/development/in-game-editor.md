# In-game layout editing and Quick Settings

Status: Edit Controls covers what the settings window edits for a game, with live pictures (branch `in-game-editor`). What stays in the settings window is listed under "Comprehensive editor".

## Goal

A player can change the controls of the game they are playing without leaving it. Controller input drives the UI, and the UI draws over the game.

## Decisions

| Topic | Decision |
|---|---|
| Entry | Hold Guide and press Start (the Quick Settings shortcut; movable, see [guide.md](guide.md#movable-shortcuts)) opens a hardcoded menu over the game. |
| Menu items | "Quick Settings" first, "Edit Controls" last. |
| Quick Settings | Stick speed (for sticks that move the mouse), gyro sensitivity (gyro in mouse mode), Invert Y (every Y inversion in the profile at once), and the profile when the game has more than one. |
| Quick Settings target | The active profile of the active game. Edits are saved to the config and apply at once. |
| Editor scope | The active profile, and what its game shares. Other games, and other profiles of the game, are not edited (but the game's profiles can be renamed, copied, moved and deleted). |
| Editor overlay | Drawn over the game. Controller input goes to the UI while it is open. The game is not paused. |
| Editor layout | A list of categories, each opening a list. Pages of values stay in one column and scroll; others go to two columns past eight rows. |
| Editor apply | Each change applies live to the running mappings. Nothing reaches the config file until Save. Closing with unsaved changes asks: save and close, discard and close, or keep editing. Discard puts back the config as it was when the menu opened. |
| New pack | A game with no pack gets one. Outside any game, a Quick Settings change makes a pack for the window in front: the active profile is copied from General, the change goes into the copy, and General is left as it was. The pack is named after the executable. Without a window, the change stays in General. (Edit Controls is refused instead; see the phase 2a notes.) |
| Navigation | Left stick and D-pad move the cursor (on a left-stick page, only the D-pad). A chooses. B backs out. Left and right change a value. The bumpers jump a page. |
| Macros and turbo | Still in the app (turbo wraps a macro). Included in the editor. |

## Phase 1 (done)

- `src/system_menu.rs`: the menu state machine, the Quick Settings rows and value changes. Unit tests cover the menu order, stick-speed and Invert Y changes, and stick stepping.
- `src/daemon.rs`: `Active::Settings`. `chord_opens_menu` runs before input routing and opens the menu. `settings_input` applies the menu's outcome. The menu is drawn as a normal `MenuView`, so the overlay process does not change.
- Pack creation: `Config::add_game_for_window` (`src/config/window_game.rs`) and `Daemon::pack_from_general`.

## Conventions that carried over

- Choosing Edit Controls outside a game makes the pack first (`pack_from_general`), so the editor edits the game's own profile. With no window in front, it is refused: the menu goes back to its top page and says "Edit Controls needs a game" (`edit_controls` in `daemon/in_game_menu.rs`).
- Button rows carry the controller's glyphs for the controller in use; stick and trigger rows use the `{ls}`, `{rs}`, `{lt}`, `{rt}` tokens.
- In a two-column list, up and down follow the order the list reads in, and left and right move to the other column (`list_step_in`, `list_start_in` and `MenuView::columns` in `menu.rs`, shared by every list menu).
- Rows carry a tone (`Tone` in `menu.rs`): adding rows are lime, removing or deleting rows red; "Color-blind mode" (`Config::colorblind_tones`) makes them blue and orange with a + or −.
- Breadcrumbs: a page shows the pages it was opened from above its title.
- Deleting takes a second press of A.

## Switch Profile (done)

The Guide menu's "Next profile" row is now "Switch profile (now …)". It opens the in-game menu on `Page::Profiles` (`src/system_menu/profiles.rs`): the active game's profiles, then "New profile…". Picking a profile switches to it and leaves the menu (asking about unsaved changes first, if any). A switch to a profile that was saved updates the menu's saved copy, so it isn't an unsaved change on its own. "New profile…" offers a copy of the current profile, each `Template` (moved from the GUI to `config/templates.rs`), and in a game a copy of each General profile; the copy gets a `unique_name` in its game and is switched to (`Outcome::Added`). Discard puts back the profile the menu opened on.

## Comprehensive editor (done, branch `in-game-editor`)

Goal: anything the settings window can do to the current profile and its game, Edit Controls can do with the controller. Decisions from the interview (2026-10-14):

| Topic | Decision |
|---|---|
| Scope | The active profile's own settings, plus the game-wide ones (modes, layers, macros, menus, info and log overlays, the game's details). No other profiles in the editor. |
| Top page | A list of categories: Buttons, Sticks, Triggers, Gyro, Touchpad, Combos, Modes, Layers, Macros, Menus, Info overlays, Log overlays, Guide overlay, Profile, Game. List pages show their items; choosing one opens its settings. |
| Text | Names, labels, overlay text and window rules are typed on the on-screen keyboard, whose keys go into the field instead of the game. |
| Game details | Name and window rules, looks (styles, font, Nintendo layout), sounds, rumble and motion. Pack info (author, version…) stays in the settings window. |
| Profiles | Switch Profile (Guide menu) switches and makes profiles. Renaming, duplicating and deleting are under Game › Profiles. |
| Visuals | Live pictures beside the list for sticks, gyro and touchpad (the settings window's graphs, with the live input), and previews of the menu or info overlay being edited. |
| Delivery | One branch, a commit per phase. |

Progress:

- Done: Switch Profile; the category top page; the row structure; text entry on the keyboard (`Screen::Typing` in `system_menu`, `OverlayController::for_field`); buttons, gestures and combos for the profile or any mode or layer; sticks (every kind: gamepad, mouse with its response presets and settings, scroll, ring with its sectors and fills, flick), triggers, zones; the whole gyro page; the touchpad (every kind, split halves); Multi, Toggle and Turbo nested to any depth, the key picker over the full keyboard; modes and layers (name, overrides, combos turned off, indicator); macros (every step kind, timing, moving steps, fighting-game motions); menus (kind settings, close button, items with label, button, weight and span); info overlays (timing, heading, cells).
- Done too: log overlays and what input displays follow (shared with info overlays' current-input cells); the Guide overlay (notes line by line, Mappings rows: text, hiding, sharing, splitting, the author's own rows); the profile's page (name, what it needs, timings); the game's page (name, window rules, profiles: rename, copy, move, delete; Nintendo layout, font, the four looks, sounds, rumble, motion); look pages for every overlay with a style (position, size, colors by swatch or typed #rrggbb, text, every decoration and ornament). Renames follow references (`named.rs`, `Config::rename_profile`, `Config::rename_game`).
- Not offered in game: a look's background picture and a cue's pack sound file (both pick files), pack info (author, version), and importing or exporting packs.
- Pictures (done): `MenuView::figure` carries a `figures::Figure` beside the list, drawn by `overlay/draw/figure.rs`. The SVG pictures moved from the settings window to `src/figures/` (deadzone, mouse response, ring, gyro) so both draw the same ones, plus a trigger's travel. `layout_editor/pictures.rs` picks the picture for a page: a stick (travel, ring, response), a trigger (press point, zones), the gyro (curve, turning), the touchpad, or the menu being edited, smaller. The daemon keeps the menu's controller's input in `Daemon::menu_live` (`figures::Live`), feeds it gyro readings while the menu is up (the engine's gyro is paused then, so the picture shows the raw turning), and redraws for them at most every 33 ms. A left-stick page leaves the left stick to its picture; the D-pad moves the cursor. An info or log overlay being edited is shown where it goes (`daemon/previews.rs`), a log overlay with sample presses.

Structure: every page builds a list of `Row`s (`layout_editor/rows.rs`), each carrying its label and what A and left/right do, so row counts, drawing, choosing and stepping all come from one place. Input pages take an `Owner` (the profile, or one of the game's modes or layers), so a layer's sticks, triggers, gyro, touchpad, gestures and combos use the same pages as the profile's. An action is reached through an `ActionAt` (a slot plus a path into Multi, Toggle and Turbo), so actions nest.

## Open questions

- Not yet tried on a controller: the live pictures (especially the gyro's, which shows the raw turning because the engine's gyro is paused while the menu is up), typing on the keyboard in a field, and how the wider editor panel fits beside the game at large overlay sizes.

## User docs

The user-facing pages are in `wiki/`: `In-Game-Menu.md` (quick settings, saving, color-blind mode) and `Editing-Controls-in-Game.md` (every page of the editor). `Menus.md`, `Info-Overlays.md`, `Buttons-and-Actions.md`, `Sticks-and-Triggers.md`, `Layers.md`, `Macros.md`, `Gyro.md`, `Games-and-Profiles.md` and `Getting-Started.md` each have a short note with a link. The README has an "In-game menu" section. Update those when the editor changes.
