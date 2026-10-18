# Per-controller setups

Status: design, from an interview, then checked against the code. Nothing is implemented.

## Goal

Several controllers at once should each get their own mapping, and a controller's mapping should
follow it when it is unplugged and plugged back in. The hierarchy is:

    Controller -> Setup -> Profile

- **Controller**: one physical device, identified by its unique id (serial / `uniq` / USB path), with
  vendor/product as the fallback when a device has none.
- **Setup**: a pack, as today. Packs stay global: imported once, shared by all controllers.
- **Profile**: a profile inside a pack, as today.

Problems this solves: different controller types needing different bindings for one game (DualSense
with gyro vs Xbox pad), local co-op with two or more pads, and swapping pads without reconfiguring.

## Decisions

| Topic | Decision |
|---|---|
| Ownership | Packs are shared. Each controller stores only *which profile is active per pack*. No per-controller copies of packs, no diff layers. |
| What differs per controller | Only profile choice, plus what tells pads apart: nickname, colour, player label. Everything under the Profile tab is fair game; global settings (identity, sounds, overlay style, ...) stay global. |
| Identity | Per physical device, so two identical pads can differ. |
| Retention | Remembered until the user forgets them. Disconnected pads stay listed, greyed out. |
| Window rules | A rule picks the pack for all pads. Each pad then resolves its own profile within it. |
| Overlays and menus | Opened by a pad, acting on that pad only (profile switch, modes, mappings overlay shows that pad's profile). |
| Capability | A pad is offered only profiles its hardware supports (`requires`). If its active profile is unsupported, fall back to the pack's default compatible profile. |
| Slots | Virtual pad order is connection order, as today. A pad may be pinned to a player number. |
| New pad | In-game overlay on the new pad: copy settings from another pad / use defaults / pass through. The choice is remembered. |
| GUI | A controller picker at the top of the existing views; the Setup and Profile screens below are scoped to the picked controller. |

## Data model (sketch)

A new per-controller table, keyed by device id, next to the packs:

    [[controllers]]
    id = "..."                 # unique id, or "vid:pid" fallback
    nickname = "Couch DualSense"
    color = "#4f8cff"
    player = 2                 # optional pinned slot
    [controllers.profiles]     # pack id -> active profile name
    "Deus Ex" = "Gyro aim"

Breaking changes are fine (see CLAUDE.md): the single active-profile-per-pack state moves into the
controller entry rather than staying as a second source of truth.

## Behaviour notes

- **Resolution order** for a pad in a focused game: the window rule picks the pack; the pad's entry
  names the profile; if unset or incompatible, use the pack's default profile that the pad supports.
- **Defaults for an unconfigured pad** exist only until the new-pad prompt is answered. Without an
  answer (nobody at the pad), pass through rather than guess.
- **Copy settings from another pad** copies the profile choices, not the packs.
- **Forget** removes the entry; the pad then counts as new on its next connection.
- **Virtual pad identity** (Xbox / DualShock / DualSense) stays derived from the physical pad and the
  pack's `controllers` list, as in `pack-format.md`. It is not a per-controller setting.

## What the code does today

Checked against the code (`config.rs`, `daemon.rs`, `daemon/*.rs`).

- **One global active profile.** `Config.active: ProfileRef` (`config.rs`) is the only active-profile
  state. `Config::active()` / `active_ref()` / `active_game()` read it, and the daemon calls
  `config.active()` in about 25 places (timers, `guide.rs`, `overlay.rs`, `in_game_menu.rs`,
  `info_overlays.rs`, `auto_switch.rs`, status). The GUI's `Status` carries one `active_profile` /
  `active_game`, and the layout editor edits through `system_menu::active_profile_mut`. This is the
  main refactor: every one of those sites needs "the active profile *of this device*".
- **Per-device state already exists, but only above the profile.** `Managed` (one per physical pad)
  holds its own `Engine`, `layered: Option<(Shifts, Profile)>` (the active profile with this pad's mode
  and layers on top), `number` (connection order, used by `{current_input_device_N}`) and `gyro_bias`.
  The engine is already called with `layered(&dev.layered, base)`, so swapping `base` for a per-device
  profile is a local change at those call sites, not a new mechanism. Modes and layers are the
  precedent for per-pad state that the pack defines and the pad selects.
- **The game-level scope is shared and can stay so.** `Daemon.scope` (menus, layers, info overlays,
  fonts, modes) comes from the active *game*. Window rules pick the game for everyone, so `scope`
  stays one value. Only the profile inside it becomes per pad.
- **Identity today is the device name.** `gyro_calibration: BTreeMap<String, [f32;3]>` and
  `ignored_devices` are keyed by `dev.name`. Two identical pads therefore share a calibration and
  can't be ignored separately. Linux has `Managed.uniq` (`#[cfg(target_os = "linux")]`) and `parent`
  (HID parent path), used only to pair motion/touchpad nodes with their gamepad. There is no stable
  id type yet, and `uniq` is absent on non-Linux builds (`xinput.rs`).
- **Overlays are mostly per device already.** `Active::Menu { device }` and `Active::Settings(_, device)`
  record the pad that opened them; `guide_current_mode` finds the opener by
  `engine.guide_overlay_open()`. But `guide_view()` shows `self.config.active()` (one profile) and
  `Active` is a single `Option`, so only one overlay is up at once, whichever pad opened it.
- **First-slot pad.** `Managed.first` marks the pad that took the kept `spare_pad`. Slot order in
  Steam follows virtual-pad creation order, which `first_pad.rs` already manages with rebuild delays.

## Answers to the open questions

1. **Where the active profile lives:** `Config.active`, see above. It becomes the *fallback* for pads
   with no entry, so existing flows (auto switch, "Next profile", GUI) keep working during a staged
   refactor. Per-pad choices go in a map on the controller entry.
2. **Identical pads without a unique id:** the id cannot be told apart by name alone. Proposal: id =
   `uniq` when present, else `vid:pid` plus the HID parent path as a session-only disambiguator.
   Two such pads share one saved entry (a known limitation to show in the GUI as "identical pads
   can't be told apart"). This also fixes `gyro_calibration` keying; migrate it into the controller
   entry (breaking changes are fine).
3. **First-slot pad:** it belongs to no controller (`spare_pad` exists before any pad does). Pinning
   a player number means creating or rebuilding virtual pads in that order, reusing the
   `first_pad.rs` rebuild-before-replace trick. Needs a hardware check with Steam; do it as a separate
   step after per-pad profiles work. Until then, `player` is a label only.
4. **Guide menu:** `guide_menu` is one value on `Daemon`, and `guide_view()` reads the global
   profile. It must take the opening device's profile (the opener is already found via
   `guide_overlay_open()`). One overlay at a time stays; see question 5.
5. **Co-op overlays:** `Active` is a single slot, so two pads can't both have a menu or mappings up.
   Keep one at a time (the first to open it holds it), which is today's behaviour. Simultaneous
   overlays are a separate, larger change.
6. **Pack export:** controller entries live in the config, not in packs. Export reads games and
   profiles only; add a test that a pack round-trip carries no controller data.
7. **New-pad prompt:** `gamepads` / `SeenGamepad` already distinguish seen from managed pads, so the
   prompt hooks where a pad becomes `Managed`. Only for pads that pass the existing gamepad
   detection. An `ignored_devices` match never prompts.

## Suggested order

1. Add a stable controller id and key `gyro_calibration` and `ignored_devices` by it. **Done**
   (`controller_id.rs`; `SeenGamepad.id`, `Managed.id`, `DeviceInfo.id`). Old name-keyed entries are
   not migrated: they just stop matching.
2. **Done** (`config/controllers.rs`, `Config::profile_for`; status, toast and info overlays still read
   `Config.active`). Add `controllers` to the config (id, nickname, colour, player label, `profiles` map). Resolve each
   `Managed`'s profile through it, falling back to `Config.active`. Window rules still set the game.
3. **Done** (`Config.acting`, `Daemon::guide_profile`). The Guide menu and Mappings read the profile of
   the pad that opened the Guide overlay. The Quick Settings / Edit Controls menu sets `Config.acting`
   to its pad's id when it opens (cleared in `close_overlay`), so `Config::active()` and the layout
   editor's `active_profile_mut` stand for that pad's profile without touching their ~150 call sites.
   Not covered: Switch Profile still sets the global `Config.active` rather than the pad's entry, and
   nothing writes `controllers` entries yet, so behaviour is unchanged until step 4.
4. **Done, first part** (`Request::ActivateFor`, `DeviceInfo.profile`, `header_pick::ControllerChoice`).
   The header gets a controller picker (managed pads, shown once any exists); with a pad picked, the
   profile box shows that pad's profile and sets it through the daemon, which writes its entry. The
   daemon owns `controllers` (`SetConfig` keeps its copy, as for `active`). `Status.active_profile` is
   still the global one. Also done: a "Remembered controllers" card on the Overview (`gui/controllers.rs`;
   `Status.controllers`, `Request::SetController` / `ForgetController`) with nickname, colour, player
   label, a pass-through switch and Forget, greyed out while the pad is away; the Overview's live view
   labels from the profile of the pad in use; and Switch Profile in the in-game menu sets the opening
   pad's entry (it is kept by that menu's Save, and Discard undoes it) when the profile is in the
   active setup. Still global: the status line and toast, which name the setup's active profile.
5. **Done** (`new_pad.rs`, `daemon/new_pads.rs`, `Active::NewPad`). The first controller is
   remembered without a question; one that joins while another is in use gets a `pending` entry and
   a list overlay on its own pad: use the defaults / copy settings from a remembered pad / pass
   through. Back (or 60 s idle) leaves it pending, passing through, and it is asked again every
   5 minutes. The answer is kept in the entry (`pass_through`, copied `profiles`). Not done: the
   question for identical pads without a unique id (they share one entry), a GUI way to undo
   "pass through" or forget a pad, and a hardware check.
6. **Done, untested on hardware** (`daemon/first_pad.rs`, `fillers_needed`). A pad with `player = N`
   (N > 1) gets placeholder virtual pads made ahead of its own until N-1 pads exist, so Steam places it
   in slot N. An unpinned pad takes the kept pad, then the lowest placeholder, before a new one is made.
   Placeholders go when remapping is turned off. Linux only. Not covered: the GUI has no way to set the
   number yet beyond the existing player label; pinning to 1 is the kept pad's behaviour; a pad
   connecting after a lower-numbered one left may land in a freed slot instead. Needs a check with Steam.
