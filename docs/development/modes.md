# Modes

Status: implemented (2026-10-11).

## Goal

Layers stack, so toggled layers could pile up into combinations that don't fit together, and
leave the player without a button to get back out. Modes are for context switches (vehicles,
minigames, a game's own menus): mutually exclusive, so switching to one turns off the one before.
Layers stay for extending and fine-tuning the scheme they're held over (lean while a button is
held, a slower mouse while aiming).

## Decisions at a glance

| Topic | Decision |
|---|---|
| What a mode is | A named set of overrides belonging to a game, stored exactly like a layer (`config::Mode` is `Layer`). It applies over whichever of the game's profiles is active. |
| The default mode | No mode on: the profile itself. It has no entry of its own; it only shows (as "Default") once the game has modes. |
| Activation | `ButtonAction::Mode(name)`, on press. It switches to that mode, or back to the default if that mode is already on. Release does nothing. |
| Where it can be used | Every input a held action can go on, menu items (in the GUI, the in-game editor and radial menus), and inside a mode or layer. Not in macros, and not inside Toggle (it already switches back). |
| Layers | Applied on top of the mode, in the order they were held. They can't be toggled any more, and menu items can't hold them, even inside Toggle. |
| Scope | Per controller, like layers: each engine has its own mode (`Engine::mode`). The daemon reports the mode of the controller used last. |
| Lifetime | Survives menus, the on-screen keyboard and profile switches within the game (`release_all(keep_mode)`). Ends when another game takes over or the controller goes away. |
| Swallow | A mode or layer with `swallow_unbound` keeps any button whose action or gestures switch a mode or hold a layer, so there's always a way back. |
| Indicator | The same as a layer's (name, an info overlay, the bindings sheet, or nothing), and the `{mode}` token ("Default" when none). |
| Editor | A **Modes** tab before Layers, the same editor (`gui/layers.rs`, by `Shift::Mode`). "Switch mode…" in the action list offers the game's modes and **+ New mode**. |
| Packs | Format 17 adds `[[modes]]` and the `mode` action. |

## Built-in modes

`Guide` and `Mouse mode` (`Config::built_in_modes`) were built-in layers. They're never stored,
can't be edited per game or removed, and the Modes tab lists them read-only. Every game's scope
has them after its own modes. Their buttons come from the app-wide `Config::shortcuts`
(`Layer::guide_with`, `Layer::mouse_with`; see [guide.md](guide.md#movable-shortcuts)), so use
`Config::mode_for` / `Config::built_in_modes` rather than the static `built_in_mode(s)`, which
have the defaults only.

- The engine's `mode()` is the mode on now: Guide while Guide is held or its overlay is up, else
  mouse mode while it's on, else the mode switched to (`chosen_mode()`). A built-in mode stands in
  for the chosen one and gives it back when it ends, so opening the Guide overlay doesn't drop
  the player out of Driving.
- While a built-in mode is on, `layers()` is empty: held layers stay counted, and apply again
  after.
- `Mode("Mouse mode")` switches mouse mode on and off, so any button or menu item can enter it.
  Switching to any other mode ends it. The Guide mode's own mouse-mode entry is such a switch
  (it sits on R3 by default, and closes the overlay). `Mode("Guide")` does nothing; the Guide button owns it,
  and the GUI's mode pickers leave it out. Profiles' Guide buttons say `Mode("Guide")`, which
  the engine ignores.
- The Guide overlay's Mappings leave them out: Left and Right turn only through the game's own
  modes (`Scope::game_modes`). Guide mode is what's on screen, and Mouse mode is the same
  everywhere.

## Migration

When a setup is loaded or a pack is read (`modes::toggled_layers_to_modes`), every
`Toggle → Layer X` anywhere (profiles, menu items, layers, modes) becomes `Mode X`, and layer X
moves to the modes. If something still holds X as a layer, it's copied instead, so the held
uses keep working. `Layer("Guide")` and `Layer("Mouse mode")` (toggled or not) become switches
to the built-in modes. A toggle's "On when the game starts" is dropped with it. A game that was
unedited since its import has its import hashes updated after the upgrade, so it isn't shown as
edited.

## The Guide overlay's Mappings

`info::GuideView` builds the Notes and Mappings panels.

- The Mappings show the profile with the mode on show applied. Guide-tab edits (texts, merged and
  custom rows) about inputs the mode changes are dropped there, since they describe what those
  inputs do outside the mode.
- With the overlay open, D-pad or Left Stick Left and Right turn the shown mode (the daemon's
  `GuideMenu::mode`, starting at the controller's mode). The heading names it, "(not on)" when
  it isn't the one that's on, and shows the Left and Right glyphs.
- Each of the game's layers that an input of the shown mode holds (buttons, gestures, combos,
  trigger pulls) gets a section under the Mappings, titled with the layer and its holders
  ("Hotkeys layer · LB"), listing the rows the layer sets. Built-in layers get none.
- Pages hold 14 rows. Mapping rows fill pages first; sections go after them, each starting a new
  page when it doesn't fit in the room left.
