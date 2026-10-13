# Code review plan

Written 2026-10-12, at commit `01e5f4a`. A plan for reviewing the whole codebase one area at a time, with the findings of each pass and the tools that help check them.

The codebase is about 72,000 lines of Rust in 258 files. A single review of all of it only finds the most obvious problems. When asked to review `src/` with a clean working tree, the review tool fell back to the latest commit and never looked at the rest. So the review goes area by area. Each pass names the files it covers, reads them as they are now (not a diff), and records what it finds here.

## How a pass runs

1. **Scope.** Name the files (the table below). If a pass is over about 6,000 lines, split it.
2. **Read for correctness first.** Use the checklist below. Cleanups come second and are only reported if they make a bug less likely.
3. **Check every finding against the code** before recording it. A finding the code doesn't support is dropped, and the reason is noted.
4. **Fix, with a test that fails without the fix.** Put the fix back to its old form once to check that the test catches it (see pass 0). If no test is possible without hardware, add the case to the hardware checks below and to `qa-plan.md` section 3.
5. **Record** each finding in the pass's table: ID, file, what's wrong, status (fixed / open / not a bug / needs hardware).
6. **Leave the tree clean.** `cargo clippy --all-targets` has no warnings, the tests pass, and files the pass grew over 400 lines are split (`scripts/long-files.sh`).

### Checklist

- **Device ownership.** Every path that grabs a controller asks the same question. Every grab has a release on each exit path: error, unplug, disable, quit, signal. Nothing stays pressed.
- **Event-loop stalls.** Nothing slow runs on the daemon thread: file reads, process scans, D-Bus calls, subprocesses. If it has to, it's cached and the cost is measured.
- **State after a change.** When the config, focus, the enabled switch or a device changes, everything derived from the old state is worked out again (scope, hand-off, layers, overlays).
- **Untrusted input.** Packs, config files, `/proc`, Steam's files and IPC messages can be malformed. Parsing can't panic, loop forever or index out of bounds.
- **Platform split.** Linux and Windows paths make the same decisions (`#[cfg]` twins such as `daemon/evdev.rs` and `daemon/xinput.rs`).
- **Concurrency.** Threads, channels and mutexes: no lock held across blocking calls, no poisoned-lock panics, threads stop when their device goes.
- **Error reporting.** Failures reach the log or the GUI, not `let _ =`, unless ignoring them is the point (and a comment says so).
- **Tests.** The behaviour the code promises in its doc comments has a test.

## Passes

Sizes are lines including tests, at the date above. Passes are ordered by the cost of a bug: one that leaves a controller grabbed or a key held down is worse than one that draws a label wrong.

| Pass | Area | Files | Lines | Status |
|---|---|---|---|---|
| 0 | Steam hand-off and game detection (commit `b6111fd`) | `daemon/steam_handoff.rs`, `focus/`, `steam_appinfo.rs`, `steam_hide.rs`, `library.rs` | ~2,600 | Done, 2026-10-12 |
| 1 | Device ownership | `daemon.rs`, `daemon/`, `input/`, `output/`, `rumble/` | ~6,900 | Done, 2026-10-12 |
| 2 | The engine | `engine.rs`, `engine/`, `motion/` | ~6,500 | Done, 2026-10-13 |
| 3 | Game detection and launchers (rest of) | `focus/` (backends), `launchers.rs`, `offer/`, `opener_watch.rs`, `library.rs` | ~2,700 | Done, 2026-10-13 |
| 4 | Config and packs | `config.rs`, `config/`, `pack.rs`, `keycode/` | ~10,600 (split: `config.rs` alone, then the rest) | Done, 2026-10-13 |
| 5 | IPC and process lifecycle | `ipc.rs`, `monitor.rs`, `main.rs`, `tray/`, `quit/`, `record/`, `capture/`, `media/` | ~2,900 | Done, 2026-10-13 |
| 6 | Overlays and menus | `overlay/`, `menu/`, `info/`, `keyboard/`, `system_menu/`, `radial.rs`, `guide_menu.rs`, `nav.rs`, `layout_editor/` | ~14,300 (split in three) | Done, 2026-10-13 |
| 7 | Sound, drawing and fonts | `sound/`, `pad_svg/`, `font.rs`, `style.rs`, `icons.rs`, the widgets | ~4,400 | Done, 2026-10-13 |
| 8 | The GUI | `gui.rs`, `gui/` | ~16,100 (split in three) | Done, 2026-10-13 (state and messages; the editor views searched only) |
| 9 | Debug tools and integration tests | `debug/`, `tests/` | ~3,200 + tests | Done, 2026-10-13 (the profiling suite searched only) |

## Remaining issues

Everything the passes left unsolved, gathered here as of commit `72f0c59` (2026-10-13). Each pass's table below keeps the full finding; when one of these is settled, update its row there and remove it here.

### Open bugs

None.

### Needs Windows or hardware to settle

| ID | What's left | Check |
|---|---|---|
| R5-12 | On Windows nothing ends ffmpeg when the daemon dies; it may record until the disk is full. A job object would kill it but leave the MP4 unplayable. | H-9 |
| R0-7 | Whether a Steam game with Steam Input off inherits an `SDL_GAMECONTROLLER_IGNORE_DEVICES` that hides Padwight's controller. Checked only with Steam Input on. | H-3 |
| R9-8 | The `hotplug_`, `daemon_`, `idle_` and profiling tests should stop at once when a daemon runs on the host; not yet tried with one running. | Run them with the host daemon up |

Every fix with a hardware check (H-1 to H-19, [below](#hardware-checks)) is also unconfirmed on real devices until that check passes.

### Long files still to split

R0-10, R1-4, R2-7, R4-9 and R8-8 shrank the worst files, but these are still well over 400 lines (`scripts/long-files.sh`):

| File | Lines | Next step named by a pass |
|---|---|---|
| `config.rs` | 2,146 | The stick and trigger types (R4-9) |
| `gui/items.rs` | 1,643 | |
| `daemon.rs` | 1,519 | |
| `gui.rs` | 1,429 | |
| `gui/profile.rs` | 1,272 | |
| `engine.rs` | 1,186 | `emit`, which predates the size lints (R2-7) |
| `gui/packs.rs` | 1,025 | |
| `pack.rs` | 728 | |
| `gui/actions.rs`, `gui/checks.rs` | 616, 538 | |

### Not read in full

These parts were only searched (for unwraps, indexing, clamps and slow work), not read line by line:

- Pass 6: the rest of `overlay/draw/`, `overlay/windows.rs`, `info/catalog.rs`, `opener.rs`, `pad_model.rs`, `keyboard/xkb.rs`, `labels.rs`, `layout_editor/buttons.rs`, `values.rs`, `views.rs`, `menu/suite/`.
- Pass 7: the drawing data in `pad_svg/models.rs` and `parts.rs`.
- Pass 8: the GUI's editor views (`actions.rs`, `sticks.rs`, `triggers.rs`, `gyro.rs`, `touchpad.rs`, `style_editor.rs`, `overlays.rs`, `settings.rs`, the graphs), `manual.rs`, `tour.rs`, `shoot.rs`.
- Pass 9: the profiling suite, `tests/profile/`.

### Tools not yet built

- A replay of a recorded input stream through the daemon's scan and dispatch, and a container test that a uhid pad appearing while `steam_has_pads` is set isn't grabbed (R0-1).
- A sweep that loads, saves and compares every config folder in `tests/fixtures` and every library pack.

## Pass 0: Steam hand-off and game detection

Reviewed commit `b6111fd` and the code it touches. Findings were checked against the code before fixing.

| ID | File | Finding | Status |
|---|---|---|---|
| R0-1 | `daemon/evdev.rs` | A controller plugged in, or woken from sleep, while Steam had the controllers was grabbed anyway. The check for a pad seen before asked whether Padwight takes pads, but the check for a new one only asked whether remapping was on. | **Fixed.** Every scan path, evdev and XInput, now asks `Daemon::should_manage` (`daemon/steam_handoff.rs`). No automated test: the scan needs `/dev/input`. Hardware check H-1. |
| R0-2 | `focus/identify.rs` | A sandboxed game's window can report its pid inside the sandbox (X11's `_NET_WM_PID`). That pid nearly always names an unrelated process here, and the code took it as it was, so the game wasn't recognised. | **Fixed.** When the process here doesn't look like the window (class against program name or Steam app), the processes with that pid in a namespace below are tried, and the one that looks like the window wins. `pick_pid` and `fits` have unit tests. `identifies_a_window_from_a_real_sandbox` runs a real pid namespace with `unshare`. With the old rule put back, it fails (the window resolved to pid 1, systemd). |
| R0-3 | `steam_appinfo.rs` | Controller support was cached for the daemon's lifetime, including "not listed". So a game installed after the daemon started was never handed to Steam. Each new app also read the whole `appinfo.vdf`. | **Fixed.** The whole file is indexed once and indexed again only when its modification time changes. Test: `a_game_steam_learns_about_later_is_found`. Measured with `padwight debug steam`: 31 ms for a 10 MB cache of 1,838 apps, on the first Steam game focused after a change. |
| R0-4 | `focus/identify.rs` | A Wine program started by its bare name (`gpp.exe`, `.\game.exe`) had no folder in its path, so rules naming a folder never matched it. | **Fixed.** A bare name is replaced by the matching `.exe` mapped into the Wine loader, which has the full path. Test: `wine_programs_started_by_bare_name_report_the_mapped_path`. |
| R0-5 | `daemon.rs` | Saving the config (ticking "Use this setup even when Steam lists the game…") didn't change who had the controllers until the game lost and regained focus. | **Fixed.** `replace_config` calls `recheck_steam_handoff`, which handles the window in front again when the answer changed. Taking a game back also switches to its profile. Hardware check H-2. The original report also named `SetEnabled`, but that is not a bug: `should_manage` already covers the enabled switch. |
| R0-6 | `daemon.rs` | Handing a game to Steam left the previous game's profile active. | **Fixed.** The hand-off leaves the previous game the way any other window does (`leave_game`). The Steam game's launch is still not recorded. That's deliberate: launch handling (info overlays, start toggles) only applies to games Padwight handles. |
| R0-7 | `steam_hide.rs` | Concern: Steam's games might inherit Steam's `SDL_GAMECONTROLLER_IGNORE_DEVICES`, which hides Padwight's controller, and so ignore Padwight's controller too. | **Partly checked.** With Steam Input on (Slay the Spire 2, 2026-10-12), the game runs with Steam's own list of 794 IDs, not the inherited one, as the module comment says. That list also hides Padwight's IDs, which doesn't matter, because Steam Input has the controllers then. Not yet checked with Steam Input off. Hardware check H-3. |
| R0-8 | `focus/mod.rs` | The process scan tried every rule against every process even after a match nothing could beat. | **Fixed.** `best` stops at a match with the highest specificity. Test: `the_search_stops_at_a_match_nothing_can_beat`. |
| R0-9 | `library.rs` | A test repeated the private `names_folder` check, so it could drift from the matching code. | **Fixed.** It calls `focus::names_folder`. A misplaced doc comment in `focus/mod.rs` was moved too. |
| R0-10 | `daemon.rs`, `config.rs`, `pack.rs`, `gui/games.rs` | The commit grew files that are already far over the 400-line limit. | **Partly done.** Passes 1 and 4 shrank `daemon.rs`, `config.rs` and `pack.rs` (see R1-4, R4-9); pass 8 split `gui/games.rs` (R8-8). This pass split the file it pushed over the limit: the `focus` tests moved to `focus/tests.rs`. |
| R0-11 | `focus/mod.rs` | Seen with `debug focus processes`: a Steam App ID rule matches Steam's `reaper` wrapper as well as the game, and the scan picks the first, so the launch is recorded with reaper's pid. | **Not a bug.** Reaper lives exactly as long as the game. Noted in case launch tracking ever needs the game's own pid. |

The status reported by the daemon now includes `steam_has_pads`, so tools and the GUI can show who has the controllers.

## Pass 1: Device ownership

Read all of `daemon/`, `input/`, `output/` and `rumble.rs`, and in `daemon.rs` the event loop and every grab and release path: the scans, `device_gone`, `release_devices`, `release_all`, profile switches, identity rebuilds, shutdown, and the signal handlers on both platforms.

| ID | File | Finding | Status |
|---|---|---|---|
| R1-1 | `rumble.rs` | A single failed rumble write to the controller (it was unplugged, or Bluetooth dropped for a moment) ended the forwarding thread. Nothing then answered the game's next rumble upload, so the game waited 30 s for the kernel to time it out. A game that rumbles from its main thread freezes for that long, each time it rumbles. | **Fixed.** Failed plays, stops and gain changes are logged once until rumble works again, and forwarding goes on. Kernel test `a_controller_that_stops_rumbling_doesnt_stall_the_game`, which unplugs a stand-in controller. On the old code it failed after waiting 30.4 s. It runs on a host with uinput access: `cargo test -- --ignored rumble`. |
| R1-2 | `daemon.rs`, `config/store.rs` | Every profile switch, including each automatic switch on alt-tab, rewrote the whole config folder from the daemon's thread. The rewrite deleted `setups/` and then renamed the new copy into place, so a crash, kill or power cut between the two lost every setup. The next save then deleted the copy left in `.saving/`. | **Fixed, both halves.** Profile switches, the remapping switch and gyro calibration now write only `config.toml` (`Config::save_settings`). A full save sets the old folder aside as `.setups.old` (and `.general.old`) until the new one is in place, and loading reads the old one if a save stopped in between. Test: `a_save_cut_short_loses_nothing`, which fails on the old load. Side effect: an automatic switch while the in-game editor has unsaved changes no longer writes them to disk, so Discard really discards them. |
| R1-3 | `daemon/evdev.rs`, `daemon/touchpad.rs` | Letting go of a controller stops its reader threads, but they hold the grab for up to 200 ms (their poll timeout). A scan in that window (taking the pads back from Steam right after giving them, or switching remapping off and on quickly) can't grab the pad and gets it on the next scan, 2 s later. If the pad's grab works but its touchpad's doesn't, the touchpad is marked as attached and never retried, so the desktop keeps it as a mouse until the pad reconnects. | **Fixed, 2026-10-13.** The daemon keeps its own copy of each grabbed node's file descriptor (`devices::Grab`, the pad's and the touchpad's), and dropping the controller lets go of the grab through it at once, without waiting for the reader. A reader still running for a moment after its controller was let go of has its input ignored; before, it could make itself the overlay's driver. A touchpad another program holds is still not retried. Test: `hotplug_switching_remapping_off_and_on_takes_the_controller_back_at_once` (container; on the old code the pad's grab fails with EBUSY in round 1). Touchpad: H-16. |
| R1-4 | `daemon.rs` | 3,019 lines. | **Partly done.** Focus handling and auto-switching moved to `daemon/auto_switch.rs`, and the control-socket requests to `daemon/requests.rs`, with no change in behaviour. `daemon.rs` is now 2,739 lines. Pass 6 moved the overlay and menu handling (R6-9); it is now 1,523 lines. |

Checked and found sound:

- Every scan path asks `should_manage` (after R0-1).
- A failed grab, or a failed virtual pad, drops the device, which releases it.
- Readers report `Gone` when a device errors, and the next scan takes it again.
- `release_devices` and `device_gone` release the engine's outputs before dropping the virtual pad.
- When a virtual device is destroyed, even by SIGKILL, the kernel releases its keys.
- `set_profile` rebuilds virtual pads for a new identity before it resyncs.
- `VirtualKbm` sends each key as it changes, so key order is kept. After a failed write it catches up in code order, which only matters if a write failed in the middle of a key combination.
- SIGTERM, SIGINT and the console events on Windows all reach `shut_down`, which releases everything.

## Pass 2: The engine

Read all of `engine.rs` and `engine/` (buttons, combos, gestures, the Guide, actions, toggles, turbo, macros, triggers, zones, sticks, rings, flick, gyro, the touchpad) and `motion/`. Each fix has a test that failed before it.

| ID | File | Finding | Status |
|---|---|---|---|
| R2-1 | `engine.rs` | A trigger mapped to a button, or a stick direction, let go 0.05 below where it pressed. At 0.05, the lowest the settings page offers, that's 0, the resting value, so the action never let go: a key or mouse button stayed held until something else released everything. A pack or config file with a threshold of 0 also pressed every such action at rest. | **Fixed.** `analog_pressed` decides both: nothing presses at rest, and everything lets go there. Tests: `a_trigger_pressing_at_the_lowest_setting_lets_go_at_rest`, `a_stick_direction_pressing_at_the_lowest_setting_lets_go_at_rest`, `nothing_presses_at_rest_whatever_the_threshold`. |
| R2-2 | `engine.rs` | A macro that plays itself, directly or through another macro, recursed without end on the first press: the stack overflowed and the daemon aborted. A pack can contain such a macro. | **Fixed.** `set_macros` finds the macros that reach themselves (`looping_macros`), logs them, and makes them do nothing. A macro that plays another one still works. Test: `a_macro_that_plays_itself_is_refused_rather_than_crashing` (on the old code: `stack overflow`, SIGABRT). |
| R2-3 | `engine/touchpad.rs` | The finger that counts on a side (it moves the pointer, steers the joystick, picks the menu item) was meant to be the first one down, but the code took the lowest contact slot. The pad gives a lifted finger's slot to the next finger down, so after lifting the first of two fingers and putting another down, the new finger took over. | **Fixed.** `primary` picks the finger down longest, and so does the click's side. Test: `the_finger_first_down_keeps_the_pointer_when_another_lands_in_a_lower_slot`. Hardware check H-5. |
| R2-4 | `engine/touchpad.rs` | A layer that switched a touchpad side out of joystick mode while a finger was down left the virtual stick pushed until the finger lifted. Physical sticks recenter in that case. | **Fixed.** Every touchpad event first lets go of sticks whose side is no longer a joystick. Test: `a_joystick_side_switched_to_another_mode_lets_its_stick_go`. |
| R2-5 | `engine.rs` | With the Nintendo face swap on, a face button that has gestures, held past the tap window, sent the button in its position rather than the letter on its label. The other paths swap. | **Fixed.** That path uses `plain_action` too. Test: `the_face_swap_holds_for_a_face_button_with_gestures`. |
| R2-6 | `engine.rs` | A combo that lists the same button twice (`[A, A]`) fires on one press of that button. Only a hand-edited file or a pack can contain one. | **Fixed in pass 4** (R4-5). |
| R2-7 | `engine.rs` | 3,095 lines, over half of them tests. | **Partly done.** Tests moved to `engine/tests.rs`, macros and turbo to `engine/macros.rs`, the Guide and mouse mode to `engine/guide.rs`. `engine.rs` is now 1,186 lines; `emit` (marked as predating the size lints) is the next thing to move. Two doc comments on the wrong functions were moved, and one in `config.rs`. |

Checked and found sound:

- `release_all` lets go of everything that can be held: held inputs, toggles, turbos, macros (and their sticks), layers, info and log holds, ForceQuit, pushed stick directions, touchpad clicks and sticks. `release_tests.rs` checks this with random input.
- Release always uses the action recorded at press time (`held`), so a layer or profile change between press and release can't leave the old action held.
- Held combo members and stick directions stay quiet after `release_all` until let go, instead of firing under the new profile.
- Turbo can't spin: its rate is clamped, and a macro turbo waits at least 17 ms. Turbo and macros run on `tick`, which only runs while `needs_tick` says so.
- A toggled gyro goes off when everything is released (profile switch, an overlay taking the controller). That matches toggled actions, which are let go then too.
- Ring sectors, flick and zones ignore a stick at rest, so their thresholds can't hold anything at 0.
- `motion/` only shapes drawing; its styles are fixed presets, so nothing in it comes from a file.

## Pass 3: Game detection and launchers (the rest)

Read the focus backends (`focus/dbus.rs`, `kwin.rs`, `gnome.rs` and the GNOME extension, `sway.rs`, `hyprland.rs`, `wlroots.rs`, `windows.rs`), `launchers.rs`, `offer.rs`, `opener_watch.rs` and `library.rs`, and the daemon code that calls them (`daemon/auto_switch.rs`, the offer handling in `daemon.rs`). `focus/identify.rs` and the rule matching in `focus/mod.rs` were pass 0's.

| ID | File | Finding | Status |
|---|---|---|---|
| R3-1 | `library.rs`, `daemon.rs` | The daemon parsed every library pack on its own thread each time a window of a new process came to the front, to decide whether to hint at a library setup: 33 ms in a release build (200 ms in debug), during which controller input waited. | **Fixed.** `library::entries` parses once and hands out the same list. Test: `the_library_is_parsed_once`. |
| R3-2 | `focus/wlroots.rs` | On wlroots compositors (labwc, Wayfire, river), moving focus sends the old window's `done` before the new one's, and each was reported. So every focus change was "nothing in front" and then the new window: the game was left (and the controllers taken back from Steam) for a moment between two windows of the same game or two Steam games. | **Fixed.** The events read in one go are reported as where focus ended up. Test: `moving_focus_settles_on_the_new_window_without_unfocusing_in_between`. |
| R3-3 | `focus/wlroots.rs` | The window in front when tracking started was learned during `connect`, but reported only after the next window event, since `follow` waited for new events first. A game already running when the daemon started kept the default profile until something changed. | **Fixed.** `follow` reports what it knows before waiting. No automated test: it needs a wlroots compositor. Desktop check H-6. |
| R3-4 | GNOME extension | The extension reports only when focus changes. A daemon started or restarted after it (an update, a crash, a login where the extension loads first) didn't know the window in front until focus moved. KWin already had this handled by reloading its script. | **Fixed.** The extension watches for the daemon's bus name and reports when it appears (which also covers the first report). Checked with `gjs` that the callback fires when the name is taken. An installed extension keeps its old code until the next log in, as before. Desktop check H-7. |
| R3-5 | `focus/sway.rs` | Going to an empty workspace sends Sway's `workspace` event but no `window` event, so the game behind stayed "in front": its profile stayed on, and Steam kept the controllers. | **Fixed.** The tracker also follows workspace events and reports nothing in front when the focused workspace is empty. Test: `going_to_an_empty_workspace_reports_unfocused`. |
| R3-6 | `opener_watch.rs` | With a double or triple tap opening the Guide overlay, the panels counted the taps within one tap window of the first press, while the engine allows a tap window from each release to the next press. A double tap that opened the overlay could fail to close a panel; a triple tap of three 50 ms taps 200 ms apart never did. | **Fixed.** Taps are timed from the last release, as in the engine. Test: `taps_are_timed_from_the_last_release_as_in_the_engine`. |
| R3-7 | `launchers.rs` | A pack rule naming a program's folder (`Prince of Persia - The Sands of Time/gpp.exe`, since pass 0) never counted as installed, so the library picker didn't put that game first. | **Fixed.** The scan keeps the paths of the programs it finds (`.exe` files, and files marked executable) and matches folder rules against them the way focus does. Test: `a_rule_naming_the_folder_finds_the_game_in_that_folder`. |
| R3-8 | `offer.rs` | The offer closes itself after 30 s without input, so it can't hold the controller forever, but any event counted as input, including the small changes a stick or trigger resting off its middle sends all the time. | **Fixed.** Only buttons and steps of the stick keep it up. Test: `a_restless_stick_doesnt_keep_the_offer_up`. |
| R3-9 | `focus/wlroots.rs` | Closed windows' handles were never destroyed, as the protocol asks, so each window ever opened stayed allocated in the daemon and the compositor. | **Fixed.** The handle is destroyed on `closed`. No automated test (needs a compositor). |

Checked and found sound:

- Every backend runs on the focus thread or zbus's, never the daemon's; a stalled compositor socket (Hyprland's command socket has no timeout) only delays focus reports.
- Each Sway, Hyprland and wlroots tracker reports the window in front when it connects (wlroots after R3-3), and the watchdog reconnects within 10 s when the compositor restarts.
- Compositor replies that don't parse are skipped or end that connection; nothing indexes them unchecked. A D-Bus caller can only send a class, title and pid, which `identify` reads like any other.
- `opener_watch` is fed every event, whoever has the controller, so its held buttons don't go stale; a reconnected pad gets a new one.
- The offer's indexes stay in range: the daemon opens it only with candidates, a pack with several usable profiles goes to the profile list, and the cursor wraps by the list on show.
- `offers.hinted` grows by one entry per new window process. Not worth bounding.

## Pass 4: Config and packs

Read all of `config.rs`, `config/` (the store, shortcuts, gyro, curves, touchpad, assets, modes, the window game; the looks, Guide rows, logs and summaries more quickly, as they only describe), `pack.rs` and `keycode/`, and the paths that load them: `Config::load_from`, `pack::parse`, the daemon's `refresh_layers`, and the GUI's load and save.

| ID | File | Finding | Status |
|---|---|---|---|
| R4-1 | `config.rs` (`Layer::apply`) | The Guide mode and mouse mode swallow the inputs they don't set ("the game underneath gets nothing"), but only the 15 face, shoulder, stick-click, D-pad and menu buttons. A profile's back paddles and stick-direction buttons, with their gestures, and all its combos still reached the game: in mouse mode a paddle mapped to A pressed A, moving the pointer with the right stick pressed whatever `RightStickUp` sent, and LB + RB (mouse back and forward there) fired the profile's LB + RB combo. Holding Guide had the same leaks. | **Fixed.** A layer that swallows covers every button (`Button::EVERY` and the paddles) and drops the profile's combos, keeping, as before, those that hold a layer or switch modes. Test: `the_built_in_modes_let_nothing_of_the_profile_through_but_a_way_back`. |
| R4-2 | `config.rs` (`load_from`) | With `config.toml` missing (deleted to reset the settings, or lost) but the setup folders beside it, loading wrote the default config, including its setup folders, over them: every setup was gone. | **Fixed.** The setups are read from their folders whether or not `config.toml` is there; only the settings start from the defaults. Test: `a_lost_config_file_keeps_the_setups_saved_beside_it`. |
| R4-3 | `gui.rs` | When the config doesn't load (one setup or item file that doesn't parse is enough), the daemon doesn't start, and the settings window shows the defaults with an error. Saving there, with no daemon to send to, wrote the defaults over every setup. | **Fixed.** The window won't save while the config on disk is unreadable, and says why. Test: `a_config_that_failed_to_load_is_never_saved_over`. See R4-7 for loading what can be read. |
| R4-4 | `pack.rs`, `config.rs` (`mode_for`) | A pack could carry a layer or mode named "Guide" or "Mouse mode". Loading strips those from setups, but an import doesn't load: until the daemon restarted, the pack's mode took the place of the built-in one, so holding Guide applied the pack's overrides instead of the system shortcuts. | **Fixed.** `pack::parse` drops them as loading does, and `mode_for` always gives the built-in mode for its name. Tests: `a_pack_can_t_bring_its_own_built_in_modes_or_repeat_a_combo_button`, `a_game_s_mode_named_like_a_built_in_one_never_replaces_it`. |
| R4-5 | `config.rs`, `pack.rs`, `config/shortcuts.rs` | R2-6: a combo listing a button twice fired on one press. Shortcut inputs read from `config.toml` could list a button twice too, which made a one-button "combo" opener that never fired. | **Fixed.** Loading and `pack::parse` drop repeated buttons from combos and from the combos layers switch off (`tidy_combos`), and `Shortcuts::sanitized` tidies each input. Tests: `combos_listing_a_button_twice_are_tidied_when_loaded`, `an_input_listing_a_button_twice_is_tidied_when_loaded`, and the pack test above. |
| R4-6 | `config/window_game.rs` | Changing a setting outside any game makes a setup for the window in front, copying General's items so references resolve, but not General's icons and sounds, so `{icon:…}` tokens and sound cues in the copied menus and overlays found nothing. | **Fixed.** The assets are copied too. Test: `the_new_game_carries_general_s_icons_and_sounds`. |
| R4-7 | `config.rs`, `config/store.rs` | One file that doesn't parse (a hand-edited profile, or a folder copied from a newer version, which the store's module comment invites) stops the whole config from loading, so the daemon won't start at all. | **Fixed, 2026-10-13.** Loading leaves out an item file that doesn't load, or a setup's whole folder when its `setup.toml` doesn't, and keeps their bytes (`Config::unreadable`, `config/store/unreadable.rs`). Saving writes them back into the new tree; when a saved item has taken a left-out file's name, the left-out file goes beside it as `name (2).toml`. The daemon logs each one when it loads. The settings window names the first and says how many more there are. The daemon keeps its own list when the window sends a config. General's `setup.toml` gives the default General's settings, and its items load as usual. Saving is refused until that file is fixed, since a save would write over it. General gets the default profiles if all of its own were left out. A `config.toml` that doesn't parse still stops the load. Tests: `a_file_that_doesn_t_load_is_left_out_and_kept_on_disk`, `generals_settings_that_don_t_load_keep_its_items_and_stop_saves`, `general_keeps_a_profile_when_its_own_are_left_out`, `left_out_files_go_back_where_a_save_reads_them`, `files_left_out_of_the_config_are_named_on_load`. Later: what can't be saved can't be loaded. The save checks moved from `gui/checks.rs` to `config/problems.rs`; a setup that fails them is left out and kept the same way (General too, which then holds up saving), a pack that fails them is refused, and the daemon refuses such a config over IPC. Tests: `a_setup_saving_would_refuse_is_left_out_and_kept`, `a_general_saving_would_refuse_is_left_out_and_holds_up_saving`. |
| R4-8 | `config.rs` (`Layer::mouse`, `Layer::guide`) | Swallowing covers buttons and combos only. While Guide is held the profile's sticks, triggers, touchpad and gyro still reach the game, and in mouse mode its touchpad and gyro do (a touchpad side mapped to keys, or gyro aiming while LT, mouse mode's right click, is held). | **Fixed, 2026-10-13.** Decided: nothing reaches the game in the built-in modes but what they map. They also switch off the sticks, triggers, gyro and touchpad they don't set (`Layer::swallow_analog`); a stick keeps its deadzone and thresholds, so Guide shortcuts on stick directions still work. A game's own modes with "Do nothing while the mode is on" keep the profile's analog inputs, as three library packs rely on that. Test: `the_built_in_modes_let_none_of_the_profile_s_sticks_triggers_gyro_or_touchpad_through`. Two app-wide options under mouse mode's keys (`Config::mouse_mode`, off by default) let the gyro and the touchpad move the pointer there; test: `mouse_mode_can_let_the_gyro_and_touchpad_move_the_pointer`. |
| R4-9 | `config.rs`, `pack.rs` | 3,656 and 1,102 lines. | **Partly done.** Menus moved to `config/menus.rs`, layers and the built-in modes to `config/layers.rs`, the profile templates to `config/templates.rs`, and the tests to `config/tests.rs` (with `tests/actions.rs` and `tests/behaviour.rs`) and `pack/tests.rs`, with no change in behaviour. `config.rs` is now 2,144 lines and `pack.rs` 727. The stick and trigger types are the next block to move. |

Checked and found sound:

- A save sets the old folders aside until the new ones are in place, and loading reads whichever is whole (pass 1, R1-2). Settings are written to a temporary file and renamed.
- `pack::parse` is whole or nothing: unknown keys and newer formats are refused, assets are checked (clean SVG, WAV size and format, fonts that load), two items of a kind with one name are refused, and so are rules naming a missing profile.
- Nothing in `config/`, `pack.rs` or `keycode/` outside tests unwraps or indexes data from a file. Numbers that could break drawing or timing are kept in range where they're used: grid columns, ring sectors, rumble cues, and the thresholds pass 2 fixed.
- The key table other platforms use matches evdev's names and numbers. New test: `keycode::tests::the_portable_table_matches_evdev`, which reads the table's source on Linux.
- Item files are numbered `001-`: a list of over 999 items would load out of order. Not worth fixing.

For later passes: a menu's `start` and a grid item's `span` come from files unchecked (pass 6 should check the overlays clamp them). Pass 6 did: both are safe.

## Pass 5: IPC and process lifecycle

Read all of `ipc.rs`, `monitor.rs`, `main.rs`, `tray.rs` and `tray/windows.rs`, `quit.rs` and `quit/windows.rs`, `record.rs` and `record/windows.rs`, `capture.rs` and `capture/windows.rs`, `media.rs` and `media/`, and the daemon's side of them: the socket server, `daemon/requests.rs`, and the force-quit, screenshot and recording paths in `daemon.rs`. The Windows files were checked with `cargo clippy --target x86_64-pc-windows-msvc --all-targets`, which builds here; their tests compile but only run on Windows.

| ID | File | Finding | Status |
|---|---|---|---|
| R5-1 | `tray/windows.rs` | The tray icon's 1-bit mask was `size * size / 8` bytes, but each of its rows is padded to whole 16-bit words. At 125%, 150% or 175% scaling (icons of 20, 24 or 28 pixels) `CreateIcon` read past the end of the buffer. | **Fixed.** `mask_len` pads the rows. Test: `the_mask_covers_every_padded_row` (runs on Windows). |
| R5-2 | `tray/windows.rs` | Every tooltip update (each profile switch) made a new icon handle and never freed it. A long session of switching games used up GDI handles. | **Fixed.** The icon is made once on the tray's thread. Needs Windows: I-11. |
| R5-3 | `quit/windows.rs` | Force quit ends the game and its descendants, found by parent ID. Windows keeps a process's parent ID after the parent exits and soon gives that ID to new processes, so if the game's ID had once belonged to another process's parent, that unrelated process (Explorer, say, whose parent `userinit.exe` exits at login) was asked to close and then terminated. | **Fixed.** A child counts only when it started after its parent (`started_after_parents`, by process start time); links whose times can't be read are dropped. Test: `a_parent_id_left_by_an_exited_parent_is_no_link`. |
| R5-4 | `quit/windows.rs` | After the 3 second grace, processes were terminated by ID. Windows reuses IDs quickly, so one that had exited in the meantime could be a new, unrelated process by then. | **Fixed.** The processes are opened before they are asked to close and terminated through those handles, which keep their IDs from being reused. On Linux, `pid_max` is 4,194,304 under systemd, so an ID coming back within 3 seconds isn't a real risk; left as it is. |
| R5-5 | `quit.rs`, `quit/windows.rs` | Reading every process's parent ran on the daemon thread, on each force quit. The tree could also include Padwight itself, when a game or launcher had started it. | **Fixed.** The walk runs on the force-quit thread, and the tree leaves out this process and what it started. Test: `a_tree_never_holds_this_program_or_what_it_started`. |
| R5-6 | `capture.rs` | Captures go in a folder named after the game. On Windows a name with `:`, `?`, `*`, `"`, `<`, `>` or `\|`, a trailing dot, or a reserved name (`CON`, `COM1`…) can't be a folder, so every screenshot and recording of "Prince of Persia: The Sands of Time" and "Thief II: The Metal Age" (both library packs) failed. | **Fixed.** On Windows those characters become `_` (`: ` becomes ` - `) and reserved names get a `_`. Other platforms keep their names, so existing folders are still used. Test: `folder_names_are_ones_windows_accepts`. |
| R5-7 | `capture.rs`, `record.rs` | Files are named by the second. A second screenshot in the same second, from a double press, was written over the first. | **Fixed.** `claim_media_path` adds `-2`, `-3`… when the file exists or another capture is being written to it. Test: `two_captures_in_one_second_get_a_file_each`. |
| R5-8 | `daemon/serve.rs`, `ipc.rs` | When the event loop didn't answer a request within 2 seconds, the daemon closed the connection without a word, while the request stayed queued and was still carried out. The client, waiting 2 seconds too, showed "EOF while parsing a value" or a read timeout. | **Fixed.** The daemon answers with an error saying it was busy and the change may still be made, and the client waits 2 seconds longer than the daemon, so it hears that. A connection closed without a reply says so. Tests: `a_request_the_event_loop_doesn_t_answer_gets_an_error_not_silence`, `a_daemon_that_hangs_up_without_replying_is_reported_as_such`. |
| R5-9 | `daemon/serve.rs` | Watch streams had no write timeout. A GUI that stopped reading (suspended with Ctrl+Z in a terminal, or hung) held a daemon thread forever, and the event loop kept queueing a snapshot for it on every input event, without bound. | **Fixed.** A watcher that takes nothing for 10 seconds is let go. The GUI connects again when it can, and the overlay is restarted when next needed. Tests: the first test above checks that the timeout is set; `a_watcher_that_stops_reading_is_let_go` checks that a timed-out write ends the stream. |
| R5-10 | `media/mixer.rs` | Through `pactl` (PulseAudio without `wpctl`), D-pad up at full volume changed the player's volume instead, unlike `wpctl` and Windows, and a step from 98% went to 103%. | **Fixed.** The volume is set outright and capped at full; at full there is nothing to do. Test: `pactl_raises_the_volume_no_further_than_full`. |
| R5-11 | `record/windows.rs` | A recorder that exited by itself left its stdin pipe in the list `stop` reads, for good. | **Fixed.** Ended recorders' pipes are dropped when the next one starts. |
| R5-12 | `record/windows.rs` | Linux ends the recorder with the daemon (`PR_SET_PDEATHSIG`). Windows has nothing like it: if the daemon dies, ffmpeg's stdin closes, and ffmpeg may not take that as a reason to stop, so it would record until the disk is full. | **Open: needs Windows.** I-12. A job object would kill it, but then the MP4 isn't finished and won't play. |
| R5-13 | `daemon.rs` (`focused_target`) | Without focus tracking, force quit and recording scan every process on the daemon thread, twice per force-quit hold. | **Fixed, 2026-10-13.** `focused_target` (now in `daemon/auto_switch.rs`) takes the active game's process as the 2-second scan last recorded it (`launches`), while it still runs. With automatic switching off that scan doesn't run, so the processes are still read then. Test: `without_focus_tracking_the_target_is_the_scanned_launch_while_it_runs`. Desktop check H-17. |

Checked and found sound:

- The daemon reads one request line per connection with a 2 second read timeout, on a thread of its own, so a client that connects and sends nothing can't stall it. Debug requests are refused unless the daemon runs with `--debug`. A request that doesn't parse gets an error reply.
- `monitor`'s lock is only held while writing to stderr, with errors ignored, so it can't be poisoned.
- The Linux recorder is ended with the daemon by `PR_SET_PDEATHSIG`, which fires when the spawning thread ends, not the process. The thread that starts the recorder waits on it for the whole recording, so this holds. `record::start` now says so.
- The media worker runs on its own thread, so a player that doesn't answer over D-Bus holds up only the media overlay.
- The tray (`ksni` on Linux, a window of its own on Windows) only sends commands to the event loop.
- `capture::folder_name` already kept names from leaving the folder (`../`).

## Pass 6: Overlays and menus

Read in full: `menu.rs` and `menu/grid.rs`, `menu/touch.rs`, `radial.rs`, `nav.rs`, `guide_menu.rs`, `system_menu.rs`, `layout_editor.rs` and `layout_editor/menus.rs`, `zones.rs`, `macros.rs`, `info.rs` and `info/guide.rs`, `keyboard.rs` and `keyboard/boards.rs`, `overlay.rs` and `overlay/ui.rs`, `fit.rs`, `layershell.rs`, `overlay/draw/menus.rs` and `radial.rs`, the timing parts of `inputlog/view.rs`, and the daemon's side: opening and closing overlays, the frames it sends, the Guide menu, the in-game menu and overlay input. Searched for unwraps, indexing and clamps only: the rest of `overlay/draw/`, `overlay/windows.rs`, `info/catalog.rs`, `opener.rs`, `pad_model.rs`, `windows.rs`, `keyboard/xkb.rs`, `labels.rs`, `windows.rs`, `layout_editor/buttons.rs`, `values.rs`, `views.rs`, and `menu/suite/`. The pass ran in one go rather than three.

| ID | File | Finding | Status |
|---|---|---|---|
| R6-1 | `info/timers.rs`, `inputlog/view.rs` | An info overlay's `on_start` or `linger`, or a log overlay's `fade_after`, of `inf` or a number too big for a duration (TOML reads both, and a pack can carry either) panicked the daemon in `Duration::from_secs_f32`: when the game started, when the overlay was let go, or each time a log overlay was drawn. A `{current_input:…}` token's two timings, added together, could overflow too. | **Fixed.** `info::seconds` keeps a time between 0 and a year, and the token's timings add without overflowing. Tests: `a_time_too_long_for_a_duration_lasts_for_good_rather_than_crashing`, `a_fade_too_long_for_a_duration_keeps_lines_rather_than_crashing` (both panic on the old code). The info timers moved to `info/timers.rs`, with their tests. |
| R6-2 | `config/finite.rs`, `pack.rs`, `config.rs`, `config/store.rs` | A `nan` or `inf` anywhere in a pack or config file reached the drawing and timing unchecked. Seen with `debug overlay` on a pack whose keyboard style had `scale = nan`: the keyboard was drawn as one colored block over the whole screen. | **Fixed.** Packs and every config file are refused when a number in them isn't finite, naming where (`keyboard_style.scale isn't a number Padwight can use`). Nothing Padwight writes is ever one. Tests: `nan_and_inf_are_refused_wherever_they_are`, `a_config_with_a_number_that_isn_t_finite_is_refused_naming_where`, `a_pack_with_a_number_that_isn_t_finite_is_refused`. |
| R6-3 | `layout_editor/zones.rs` | Moving a zone's edge in Edit Controls clamped it between the zone's other edge and the travel. A zone read from a file that starts above 1, ends below 0, or has a `nan` edge made those bounds cross, and `f32::clamp` panicked: the daemon crashed. | **Fixed.** The edge moved is kept inside the travel and on its side of the other edge without `clamp`. Test: `a_zone_read_outside_the_travel_comes_back_inside_when_moved` (panics on the old code). |
| R6-4 | `daemon/requests.rs` | Saving in the settings window while the in-game menu was open replaced the config under it, but the menu kept the old config as "saved". Closing it then asked about changes made in the window, and Discard put the old config back in the daemon, while the file had the new one. | **Fixed.** A new config closes the in-game menu, as it already closed a menu. No automated test: the daemon needs uinput. Desktop check H-12. |
| R6-5 | `daemon/auto_switch.rs`, `layout_editor/menus.rs` | A focus change while the in-game menu was open switched the profile under it (another game's window, a launcher, the desktop), so the menu went on editing the other game. Edit Controls' page for a menu item keeps the item's position, and in the other game's menu of the same position that could be past the end: Move up, Move down, Remove and the button row then panicked. Handing a Steam game to Steam Input also let go of the controller driving the menu, which closed it without asking about unsaved changes. | **Fixed, both halves.** Focus changes and the process scan wait while the in-game menu is open, and the window in front is handled once it closes (only if focus moved, so a profile chosen in the menu stays). The item page does nothing for an item that isn't there. Test: `an_item_page_left_past_the_end_of_its_menu_does_nothing` (panics on the old code). Desktop check H-11. |
| R6-6 | `daemon.rs` (`set_profile`) | A profile switch writes `config.toml`, which holds the rumble strength and other settings Quick Settings changes. Switching profile from Quick Settings' Profile row wrote those unsaved changes to disk, so Discard reverted them in the daemon only, and the next start brought them back. | **Fixed.** Nothing is written while the in-game menu is open; its Save writes the whole config. Desktop check H-11. |
| R6-7 | `daemon/overlay_input.rs`, `daemon/in_game_menu.rs` | When the in-game menu closes without its question (the controller driving it is unplugged, remapping is switched off from the tray, the overlay window dies), its unsaved changes stay applied but unsaved, and the next save writes them. | **Fixed, 2026-10-13.** The menu no longer closes on the first two. Its controller gone, it waits on hold ("Controller disconnected") and the next controller to press a button, a reconnected one included, takes it over where it was. Remapping turned off from the tray with changes not saved keeps the menu's controller and asks Save or Discard ("Remapping is off"), then lets it go; without changes it closes as before, and remapping turned back on carries on. When nothing can answer (the overlay window dies, or a menu waiting for a controller while remapping goes off), the changes are discarded and a toast says so. Tests: `on_hold_for_a_controller_the_menu_ignores_input_and_carries_on_where_it_was`, `on_hold_with_remapping_off_a_saves_and_x_discards`; scenes `held-controller`, `held-remapping`. Desktop check H-19. |
| R6-8 | `guide_menu.rs` | Opening the Guide overlay reads every process's name from `/proc` on the daemon thread, to know whether Steam runs: about 3 ms for 526 processes (timed with an equivalent Python loop). | **Fixed, 2026-10-13.** The 2-second scan doesn't run on desktops with focus tracking, so it couldn't be reused. The menu opens without the Steam overlay row, a thread looks for Steam, and the row is added when it answers (`Msg::SteamRunning`); the cursor stays in range if the row goes. No automated test (the daemon needs uinput). Desktop check H-18. |
| R6-9 | `daemon.rs` | R1-4 left the overlay and menu handling to this pass. | **Done.** Moved, with no change in behaviour, to `daemon/overlay.rs`, `overlay_input.rs`, `info_overlays.rs`, `guide.rs`, `in_game_menu.rs` and `offers.rs`. `daemon.rs` is now 1,523 lines. |

Checked and found sound:

- A menu's session refers to menus by position; it is closed when the game changes or the config is replaced. Opening refuses an empty menu, submenus open only with items, and the cursor steps within the item count.
- A grid's `start` past its cells falls back to the first item (`GridLayout::owner`), and a span is cut to the free cells, so neither can index out of range (pass 4's question).
- Radial weights: a weight that isn't finite counts as 1, and a small one as a quarter, so every item keeps an arc to aim at.
- The drawing clamps the list cursor and its scroll, draws no slots for a carousel without items, and clamps a style's scale (now always a number, after R6-2).
- The keyboard lets go of everything it holds on close: the pressed key and the Shift it added, latched modifiers, and the trigger's Shift, counted once when the trigger and a latch both hold it.
- Edit Controls reads the config through `get` everywhere but the item page fixed above, so a stale position shows an empty page; fixed pages index tables within their row counts.
- The overlay window exits when the daemon's stream ends, and the daemon closes whatever was up when the window exits (`check_overlay_process`).

## Pass 7: Sound, drawing and fonts

Read in full: `sound.rs` and `sound/` (`player.rs`, `wav.rs`, `effects.rs`, `synth.rs`, `library.rs`, `kind.rs`), `pad_svg.rs` and `pad_svg/` but for the drawing data in `models.rs` and `parts.rs` (searched for clamps and input values only), `font.rs`, `icons.rs`, `style.rs`, `icon.rs`, `keyword_icon.rs`, `pad_widget.rs`, `touchpad_widget.rs` and `gui/widgets.rs`, and where the daemon and the overlay call into them: the pack assets (`config/assets.rs`), the frame's fonts and icons (`daemon/overlay.rs`, `overlay/ui.rs`). The Windows build was checked with `cargo clippy --target x86_64-pc-windows-msvc --all-targets`.

| ID | File | Finding | Status |
|---|---|---|---|
| R7-1 | `icons.rs` | A pack's icon was checked for `<image>`, scripts and `http` links, but the SVG renderer (usvg, under iced) also reads `<feImage href="…">` as a file from disk, whole. A shared pack could draw any picture at a known path, or name `/dev/zero` and run the settings window or the overlay out of memory. Checked with resvg: an icon with an `feImage` naming a local file passed `check_svg` and drew the file. | **Fixed.** Every `href` must name a part of the file itself (`#id`), so an icon can still `<use>` its own shapes. A pack with such an icon is refused on import, and a game file already holding one (imported before, or edited by hand) has the icon left out when its icons are put to use (`set_pack_icons`). Tests: `an_icon_can_t_read_a_file_from_disk`, `a_game_s_icon_that_fails_the_check_is_never_drawn`. |
| R7-2 | `sound/player.rs` | The sound thread ended for good the first time the audio device didn't open (no output plugged in, the sound server restarting, Bluetooth headphones the only output and off), so overlay sounds stayed silent until the daemon or settings window restarted, and nothing said why. | **Fixed.** That cue is dropped and the next one tries again; the first failure is logged. Test: `a_device_that_won_t_open_drops_that_cue_and_the_next_tries_again`. Desktop check H-13. |
| R7-3 | `font.rs`, `overlay/ui.rs` | Every frame names the pack's font files, and the overlay read and hashed each one (up to 1 MB) on every frame it took, which is every input event while an overlay is up, on its drawing thread. Only games whose pack carries fonts. | **Fixed.** The files are read again only when the frame names different ones. Test: `the_overlay_reads_a_pack_s_font_files_once_not_every_frame`. |
| R7-4 | `font.rs` | The daemon shares a pack's fonts with the overlay as files in the runtime folder. Windows has none unless `XDG_RUNTIME_DIR` is set, so on Windows the overlays never drew a pack's own fonts. | **Fixed.** On Windows they go in the user's local app data. Needs Windows: H-14. |
| R7-5 | `font.rs` | A shared font file was used as it was whenever it existed, so one cut short (the daemon ending while writing it) stayed broken until logout cleared the runtime folder. | **Fixed.** A file of the wrong length is written again. Test: `a_shared_font_file_cut_short_is_written_again`. |
| R7-6 | `touchpad_widget.rs` | The Touchpad tab drew a radial menu of one item as an arc from a point back to itself, which SVG leaves out, so the side was blank and a finger on it lit nothing. (The overlay's ring already draws this case as a circle.) | **Fixed.** One item fills its side. Test: `a_ring_of_one_item_fills_its_side_and_lights_up`. `gui/ring_preview.rs` drew its wedges the same way: see R8-7. |
| R7-7 | `sound/player.rs` | `Sounds::play` said cues queue up behind each other. Each plays on its own player, so they overlap. | **Fixed:** the comment now says so. Overlapping is what a fast-moving cursor wants. |
| R7-8 | `daemon/overlay.rs` | Each frame carries the pack's icons as SVG text (up to 64 KB each), and the overlay compares them with the last ones. | **Fixed, 2026-10-13.** Each overlay stream sends them only when they differ from the last ones it sent (`leave_out_icons_sent`); a frame without them keeps the overlay's. A new connection gets them with its first frame, and `debug overlay live` carries them forward. Test: `the_overlay_is_sent_a_pack_s_icons_only_when_they_change`. |

Checked and found sound:

- `decode_wav` reads chunk sizes from the file but slices within it, needs a format and data chunk, refuses a zero rate or channel count and anything over 8 seconds, and does the length check in seconds, so a file claiming a rate of 1 Hz can't make a huge buffer. Resampling stays inside the samples for every rate a file can claim (the rounding error is a few hundredths of a step at most), and an empty result plays as silence.
- Every cue setting is clamped where it's used, and `nan` and `inf` can't reach them (R6-2). The echo and reverb tails stay finite for settings near 0.
- Pack sounds and fonts are size-checked when a pack is loaded (`PackAssets::check`), and fonts again when loaded from a file.
- The controller drawing writes no text into the SVG, so a mapping label can't break it; stick and trigger values are clamped before drawing. Label rows fit both columns with every input labeled, and the leader-line uncrossing is bounded.
- Locks in `font.rs`, `icons.rs` and `sound/wav.rs` are never held across anything slow but a font load, and recover from poisoning.

## Pass 8: The GUI

Read in full: `gui.rs` (the app's state, `update`, undo, saving and closing), the message handlers in `items.rs`, `profile.rs`, `games.rs`, `packs.rs`, `logs.rs`, `keys.rs`, `tokens.rs`, `sound_picker.rs`, `menu_grid.rs` and `shortcuts_tab.rs`, and all of `layers.rs`, `guide_tab.rs`, `pack_files.rs`, `font_menu.rs`, `ring_preview.rs` and the save checks in `checks.rs`. Searched for unwraps, indexing, clamps, casts and file or process work on the window's thread only: the editor views (`actions.rs`, `sticks.rs`, `triggers.rs`, `gyro.rs`, `touchpad.rs`, `style_editor.rs`, `overlays.rs`, `settings.rs`, the graphs), `manual.rs`, `tour.rs`, `shoot.rs` and the rest. The pass ran in one go rather than three, since the views only build widgets from state the handlers keep in range.

| ID | File | Finding | Status |
|---|---|---|---|
| R8-1 | `gui.rs`, `gui/games.rs` | Ticking "Ignore" on a controller saves the config as last saved, with only that change, but the reply set the saved copy to everything on screen. Unsaved edits then showed as saved: no "Unsaved changes", no question on closing, and the daemon never had them. A save that was still on its way when another edit was made did the same to that edit. | **Fixed.** The reply carries the config that was sent, and that becomes the saved copy. Unsaved renames are kept until they are saved. Tests: `ignoring_a_controller_leaves_other_edits_unsaved`, `an_edit_made_while_saving_stays_unsaved`. |
| R8-2 | `gui/games.rs` | Saving waits while the config file couldn't be read, since the page then shows defaults, but ticking "Ignore" on a controller saved them anyway. With the file fixed and the daemon started after the window opened, that wrote the defaults over every setup. | **Fixed.** It refuses as Save does. Test: `ignoring_a_controller_never_saves_over_an_unreadable_config`. |
| R8-3 | `gui/guide_tab.rs`, `gui.rs` | The Guide tab's notes editor was loaded only when the tab or a profile was picked. Another game's page shown with the tab open, a new or deleted profile, Undo or Revert left it showing the old notes, and typing wrote them, plus the new letter, over the shown profile's notes. | **Fixed.** After each edit the editor is loaded again when it no longer holds the shown profile's notes. Test: `the_notes_editor_follows_the_profile_shown`. |
| R8-4 | `gui.rs` | A shortcut given its button by pressing it on the controller comes in as live input, which `update` doesn't count as an edit: it had no undo step, so Ctrl+Z took it back together with the edit before it, and the overlay previews kept the old shortcut labels until the next edit. | **Fixed.** Live input counts as an edit while a shortcut is listening. Test: `a_shortcut_set_by_pressing_is_its_own_undo_step`. |
| R8-5 | `gui.rs` | Ctrl+Z waited while the key picker or a dialog was open, but not the token, character or sound picker. The token and character pickers name their text by position, so undoing "+ New info overlay" under one made the next pick write into another overlay. | **Fixed.** Undo waits while any popup is open. Test: `undo_waits_while_a_picker_is_open`. |
| R8-6 | `gui/pack_files.rs` | An icon, sound or font picked in the file dialog was added to whichever setup was shown when the dialog closed, which can be another one: the window stays usable while the dialog is open. | **Fixed.** It goes to the setup it was added from, or, if that one was renamed or deleted meanwhile, nowhere, and the message says so. Test: `a_pack_file_goes_to_the_setup_it_was_added_from`. |
| R8-7 | `gui/ring_preview.rs` | A button ring of one sector (a config file can hold one; the editor offers 4, 8 and 12) was drawn as an arc from a point back to itself, which SVG leaves out, so the preview was blank (pass 7's question, R7-6). | **Fixed.** One sector is drawn as a circle. Test: `a_ring_of_one_sector_is_drawn_whole_and_lights_up`. |
| R8-8 | `gui/games.rs`, `gui.rs`, `gui/shortcuts_tab.rs` | R0-10 left `gui/games.rs` (769 lines) to this pass, and this pass grew `shortcuts_tab.rs` past 400. | **Done.** The Details tab moved to `gui/details.rs` and the App settings overlay cards to `gui/overlay_cards.rs` (`games.rs` is now 381 lines). The tests of `gui.rs` and `shortcuts_tab.rs` moved to `gui/tests.rs` and `gui/shortcuts_tab/tests.rs`, keeping their module paths. `items.rs` (1,643), `gui.rs` (1,429), `profile.rs` (1,425), `packs.rs` (1,025), `actions.rs` and `checks.rs` are still over 400 lines. |
| R8-9 | `gui.rs`, `gui/items.rs`, `gui/profile.rs` | Which cards and rows are open is kept by position (`open_menus`, `expanded` and the like). Undo and Revert moved none of them along, adding, deleting or importing an info or log overlay left its open input groups in place, and removing a combo closed every combo row, so afterwards other cards than before were open. (Zones aren't folded, so removing one moved nothing.) | **Fixed.** `gui/open_cards.rs` moves every set kept for a list (cards, Appearance sections, input groups, combo rows) when an item is added, deleted or imported, and after Undo and Revert finds each open item again by name (combos by their buttons), or keeps its place if the list kept its length. Tests: `undo_keeps_the_same_cards_open`, `revert_keeps_the_same_cards_open`, `input_groups_move_with_their_overlay`, `open_combos_follow_removal_and_undo`. |

Checked and found sound:

- Every handler that takes a position (an item, step, row, cell, rule, combo, zone) reaches it with `get` or checks the length first, so a stale position does nothing. Removing a profile keeps the edited one in range, and `settle_config` does after Undo, Revert and loading.
- The layer editor writes back only what changed in its working copy, after the message, and only into the layer shown before and after it; messages that pick another layer, mode or profile change nothing in the copy, so they write nothing. The settings a layer can't hold (tap and combo windows, what the profile requires, the Guide tab) aren't shown in it.
- Renaming a macro, menu, overlay, layer or mode follows references within its game, which is all that can refer to it (`Config::scope_of`). Copying items from another game renames clashes and copies what they refer to; the root is always first in the list.
- Files are read and written, and the file dialogs run, off the window's thread. Pack files are checked for names, size and content when added (`PackAssets`).
- Saving refuses an unreadable config and anything the daemon would refuse, and goes to the problem.

## Pass 9: Debug tools and integration tests

Read in full: `debug.rs` and `debug/` (`protocol.rs`, `server.rs`, `bridge.rs`, `pad.rs`, `window.rs`, `focus.rs`, `sound.rs`, `sound_panel.rs`, `glyphs.rs`, `overlay_shot.rs`, `pack_scenes.rs`, `motion_shot.rs`, the frame building in `scenes.rs`), the daemon's side of `--attach` (`daemon/inject.rs`), `snapshot.rs`, and `tests/` but for the profiling suite (`tests/profile/`, searched for how it finds its devices and controllers only). The `debug_`, `hotplug_`, `daemon_` and `idle_` container tests were run before and after (`scripts/kernel-test-docker.sh`), and the Windows build was checked with `cargo clippy --target x86_64-pc-windows-msvc --all-targets`.

| ID | File | Finding | Status |
|---|---|---|---|
| R9-1 | `debug.rs` | Ctrl+C (or SIGTERM) on a `padwight debug --attach` session ended it without a word, so its controller stayed in the daemon with whatever it held. With `--live`, a key held at the time stayed down on the desktop until `padwight debug detach-all`. | **Fixed.** Ctrl+C ends the session the way `quit` does, which removes the controller and lets go of what it held. The daemon's signal handling (`stop_on_signals`) now takes what to do, and the session uses it. Test: `debug_ctrl_c_detaches_the_controller_and_lets_go_of_what_it_held` (container). Windows: H-15. |
| R9-2 | `debug.rs` | An attached session attached to the daemon before taking its socket. A second `--attach` session, refused because the first had the socket, left its controller in the daemon. | **Fixed.** The socket comes first, and is removed again when attaching fails. Test: `debug_a_second_attached_session_leaves_no_controller_behind` (container). |
| R9-3 | `debug/server.rs` | `tap`, `pad-click <ms>` and `swipe` returned as soon as a send to the daemon failed, before letting go. The session then showed the button, click or finger as held, and the next change sent its release on to a daemon that never had the press. | **Fixed.** The release always happens; the first error is still reported. Test: `debug_a_tap_the_daemon_never_got_leaves_nothing_held` (container). |
| R9-4 | `snapshot.rs` | A zero, negative or `nan` size or scale (`debug overlay --size 0x0`, `--scale 0`, the same in `debug motion` and `debug gui`) panicked inside the renderer ("Create clip mask"), and a huge one ran out of memory. The Windows overlay draws its window through the same function, with the display's size. | **Fixed.** Refused with an error; each side must be 1 to 16,384 pixels. Test: `an_empty_or_huge_image_is_refused_not_a_panic`. |
| R9-5 | `debug/sound.rs` | `padwight debug sound length=nan` (or `pitch=nan` for a recorded kind) panicked working out how long the cue lasts. | **Fixed.** A setting must be a finite number. Test: `a_setting_that_is_not_a_finite_number_is_refused`. |
| R9-6 | `debug/protocol.rs` | `stick left nan 0` parsed, then JSON wrote the `nan` as `null`, and the session answered "bad request: invalid type: null". | **Fixed.** Refused as "not a number". Test: `mistakes_are_explained`. |
| R9-7 | `debug/motion_shot.rs` | The animated PNG's frame time was `fps × 1000` over 1000 in 16 bits, which tops out at 65.5 fps: `--fps 100` played at 65.5. | **Fixed.** Written in ten-thousandths of a second. Test: `the_animation_plays_at_the_frame_rate_asked_for`. |
| R9-8 | `tests/common/input.rs`, `tests/debug_session` | The daemon tests picked the test daemon's virtual keyboard as the one with the highest event number. The kernel reuses free numbers, so with a daemon running on the host they could watch its keyboard and time out (the flake in `qa-plan.md`). The tests with a uhid controller can't work then at all: the host daemon grabs their controller first (`grabbed` counts its grab as the test daemon's) and types their keys into the desktop. | **Fixed.** The `debug_` tests read the keyboard that appeared after their daemon started (`Reader::open_new`). The `hotplug_`, `daemon_`, `idle_` and profiling tests stop at once and say a daemon is running (`assert_no_other_daemon`). All pass in the container; the stop wasn't tried with a host daemon running. |
| R9-9 | `tests/daemon_exit.rs`, `tests/common/mod.rs` | The SIGTERM failure message said the daemon has no SIGTERM handler (it has one), and `default_config`'s comment sat on `install_config`. | **Fixed.** |
| R9-10 | `tests/debug_session.rs` | 619 lines, and this pass added three tests. | **Done.** Now `tests/debug_session/main.rs` (the session tests, 346 lines) and `daemon.rs` (the tests that need a daemon, 319). The test binary's name and the tests' names are unchanged. |
| R9-11 | `debug/bridge.rs` | When the daemon restarts, an attached session's controller is gone and every change fails ("no injected controller debug:0") until the session is started again. A change whose send failed is dropped, so the two can disagree until that input changes again. | **Fixed, 2026-10-13.** A failed send, or an output request the daemon can't answer, replaces the controller with a fresh one given the pad's whole state (`Session::reattach`); while the daemon can't take one, each change tries again. So a change made while the daemon was down reaches it once it is back. Test: `debug_a_session_carries_on_with_a_restarted_daemon` (container; on the old code the press after the restart fails). |
| R9-12 | `debug/pack_scenes.rs` | Two menus or info overlays whose names give the same file name (`Items` and `items!`, or two names with no letters or digits) are drawn to one file, and the second overwrites the first. | **Fixed, 2026-10-13.** A name that gives a file name already taken is numbered (`items`, `items-2`), and a name with no letters or digits is drawn as `<file>-menu` or `<file>-info`, numbered the same way. Test: `names_that_give_the_same_file_name_are_numbered`. |

Checked and found sound:

- The session's locks are taken link then pad, everywhere, and poisoning is recovered from. `flush` holds the link across the request to the daemon on purpose: it keeps the window's changes and the socket's in order.
- Script and command parsing can't panic: every number goes through a checked parse, a script is parsed whole before anything is sent, and the client waits at most 30 seconds for a reply.
- `debug focus` reads `/proc/<pid>/environ` for any pid given and shows nothing when it can't; `debug steam` parses app IDs and reads each cache file once. `debug overlay live` waits no longer than `--wait` and skips frames that show nothing.
- The integration tests kill their daemons and sessions when they fail (`Drop`), each test runs in its own temp folder with its own `XDG_RUNTIME_DIR`, and the daemon tests run one at a time (`serial`). The default config they write ignores every real gamepad on the machine.

## Tools for checking findings

Tools added in pass 0:

| Tool | What it answers |
|---|---|
| `padwight debug focus` | What the running daemon sees in front: the focus backend, the active profile, whether Steam Input has the controllers, and, for the focused window, its program and path, the Steam app and its controller support, the rule it matches, and the SDL controller variables the process runs with. A long `SDL_GAMECONTROLLER_IGNORE_DEVICES` list is summarised as its length and which of Padwight's controller IDs it hides. |
| `padwight debug focus <pid> [class] [title]` | The same for any window, without the daemon: identifies the pid as the daemon would now, including finding a sandboxed process from its inner pid (it says so when it does). |
| `padwight debug focus processes` | For desktops without focus tracking: which running processes each rule matches, and which one the process scan picks. |
| `padwight debug steam [app id]...` | Reads Steam's app cache as the hand-off does: each cache file's size, how many apps list their controller support, how long reading took, and the support listed for each app ID given. |
| `scripts/sandbox-pid.sh [program [args]]` | Runs a program in its own pid namespace (as a Flatpak or the Steam runtime does) and prints its pid there and here, with the `debug focus` line to try. Needs unprivileged user namespaces. |

Earlier tools that later passes will need are listed in `CLAUDE.md` (overlay and motion screenshots, sound traces, long files), in `qa-plan.md` (project checks, container kernel tests with `scripts/kernel-test-docker.sh`) and in `overlay-debugging.md`.

Tools later passes are likely to want, to be built when a finding needs them:

- **Pass 1 (still worth having):** a way to replay a recorded input stream through the daemon's scan and dispatch without hardware. `daemon::kernel_tests` and `tests/hotplug.rs` cover parts of this in a container. Check that R0-1 has a container test there: a uhid pad that appears while `steam_has_pads` is set must not be grabbed.
- **Pass 4:** a sweep that loads every config folder in `tests/fixtures` and every library pack, saves them and compares the result (A-5 and A-6 cover most of this).

## Hardware checks

These can't be checked without real controllers and Steam. When they pass, they move to `qa-plan.md` section 3.

| ID | Check | From |
|---|---|---|
| H-1 | With a Steam game with full controller support in front (`debug focus` says Steam Input has the controllers), turn a Bluetooth pad off and on, or plug in a second one. Steam must keep it: Padwight's log must not show `managing …`, and the game must respond to the pad. | R0-1 |
| H-2 | With that game in front, tick "Use this setup even when Steam lists the game…" and save. Padwight takes the controllers and the game's profile becomes active straight away, without alt-tabbing. Untick it: Steam gets them back. | R0-5 |
| H-3 | With "Hide Padwight's controller from Steam" on, turn Steam Input off for Slay the Spire 2 and start it. Run `padwight debug focus <the game's pid>`. If the `SDL_GAMECONTROLLER_IGNORE_DEVICES` line says it hides Padwight's IDs, the game can't see Padwight's controller, and the concern in R0-7 holds. | R0-7 |
| H-4 | Start a game from Flatpak Heroic, on each desktop with focus tracking (KWin, GNOME, Sway, Hyprland). `debug focus` names the game's process, and its setup switches on. Where the desktop passes on the pid an X11 window gives itself, this exercises the sandbox lookup. | R0-2 |
| H-5 | On a DualSense or Steam Deck touchpad in mouse mode with scrolling off, put two fingers down, lift the first, put a finger down again, and move each in turn. Only the finger that stayed down moves the pointer. | R2-3 |
| H-6 | On labwc or Wayfire, with a game that has a rule in front, restart the daemon. The game's profile comes on straight away, without touching any window. Then switch between two windows: the log shows no `no window has focus` in between. | R3-2, R3-3 |
| H-7 | On GNOME, after logging out and in once to load the updated extension, with a game that has a rule in front, restart the daemon. The game's profile comes on straight away. | R3-4 |
| H-8 | On Windows at 125% or 150% scaling, look at the tray icon, then switch profiles 50 times. Task Manager's GDI objects column for Padwight stays flat. | R5-1, R5-2 |
| H-9 | On Windows, start a recording, then end the daemon from Task Manager. Check whether ffmpeg is still running and whether the file plays. | R5-12 |
| H-10 | On Windows, force quit a game started from a launcher (Steam, Epic): the game and its helpers end, the launcher and Explorer don't. Take a screenshot in Prince of Persia: The Sands of Time: it is saved under `Prince of Persia - The Sands of Time`. | R5-3, R5-4, R5-6 |
| H-11 | With a game that has a rule in front, open the in-game menu (Guide + Start) and change the rumble strength in Quick Settings. Bring another window or the desktop to the front with the mouse. The menu keeps showing the game's values, and the log shows no profile switch until the menu closes; then it switches. Open the menu again, change the rumble, switch profile on the Profile row, and Discard: `config.toml` still has the old rumble strength. | R6-5, R6-6 |
| H-12 | Open the in-game menu, then save a change in the settings window. The in-game menu closes; opened again, it shows the settings window's values, and closing it asks nothing. | R6-4 |
| H-13 | Turn off or unplug every audio output, open a menu overlay and move its cursor, then plug one back in and move it again. The daemon's log says once that there is no audio device; the cues sound again once the output is back, without a restart. | R7-2 |
| H-14 | On Windows, with a game whose pack carries a font used by an info overlay, bring the game up. The overlay draws in the pack's font. | R7-4 |
| H-15 | On Windows, press Ctrl+C in a console running `padwight daemon`: it releases what it holds and exits. Then run `padwight debug --attach --live` against a `--debug` daemon, hold a button mapped to a key, and press Ctrl+C: `padwight status` no longer lists the debug controller, and the key comes up. | R9-1 |
| H-16 | On a DualSense or DualShock 4, run `padwight disable; padwight enable` in one line, then move a finger on the touchpad. The desktop pointer doesn't move, and the log has no `touchpad: cannot take`. | R1-3 |
| H-17 | On a desktop without focus tracking (`padwight status` says process scan), with automatic switching on and a game with a rule running, hold Force Quit. The toast names the game, and the game ends. Close the game, wait under 2 seconds and hold it again: "Nothing to force quit". | R5-13 |
| H-18 | With Steam running, open the Guide overlay: the Steam overlay row is there at once. Quit Steam and open it again: the row isn't. | R6-8 |
| H-19 | Open the in-game menu and change a Quick Settings value. Unplug the controller: the menu stays with "Controller disconnected"; plug it back and press a button: the menu carries on, the change still shown. Turn remapping off from the tray: the menu asks Save or Discard, and the controller is let go once answered (Discard puts the value back). With no changes, turning remapping off closes the menu at once. Kill the `padwight overlay` process with a change made: the toast says the changes were discarded. | R6-7 |
