# Padwight in Heroic: feasibility

As of 2026-10-30. Heroic references are to `src/backend/launcher.ts` in the
[Heroic Games Launcher](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher) repo, commit 509a34fe
(2026-10-15).

## Summary

Running Padwight as a per-game session inside Heroic is feasible, and most of the pieces already exist. Heroic has
the hook it needs (game wrappers), and the daemon can already take and release controllers while it runs. The new
work is a session mode, `padwight run -- <game command>`, that sets up none of Padwight's desktop-wide parts.

Plan: a local fork of Heroic, off its main branch, with Padwight built in. Heroic starts the session as a wrapper,
ties each game's Padwight setup to its Heroic game settings, and gets a Controller tab in game settings. Rebuilding
all of Padwight's settings window in React would be large, so the tab carries the core settings and opens Padwight's
own editor for the rest. The main gap is gamescope, including Steam Deck Game Mode: remapping would work there, but
the overlays would not draw.

## The goal

Padwight would run as a server that a launcher such as Heroic uses: it improves the games that launcher starts and
otherwise leaves the system alone. Today Padwight runs as a long-lived daemon that takes every controller, watches
window focus across the desktop and picks a setup for whatever is in front.

In session mode, "leaves the system alone" would mean:

- Controllers are taken only while a Heroic game runs, and handed back when it exits.
- No focus tracking: no KWin script and no GNOME Shell extension.
- No tray icon and no systemd service.
- Each game keeps its own Padwight setup, tied to that game's Heroic settings. Starting or ending a session changes
  no saved config.
- The setup comes from the game Heroic launched, not from the focused window.

## What Heroic offers

Heroic has no plugin system. It does have two generic hooks per game, and wrappers are the better fit.

| Hook | When it runs | What it is told | Code |
|---|---|---|---|
| Wrapper | Wraps the game's command for its whole life, like MangoHud, gamemoderun and gamescope | `HEROIC_APP_NAME`, `HEROIC_APP_RUNNER`, `HEROIC_APP_SOURCE` (gog, epic, amazon, sideload, zoom), `STORE` | `setupWrappers` (line 1321), `setupWrapperEnvVars` (line 1072) |
| Before/after launch script | Before launch (Heroic waits for it), and after the game exits | `HEROIC_GAME_APP_NAME`, `HEROIC_GAME_EXEC`, `HEROIC_GAME_RUNNER`, `HEROIC_GAME_TITLE`, the game's info and settings as JSON | `runScriptForGame` (line 1989) |

A wrapper lives exactly as long as the game, so Padwight's session can start and end with it. It also knows which
game it is for from its environment and from its own child process. Heroic already finds MangoHud and GameMode on
PATH and adds them as wrappers when their toggles are on (lines 588–610).

Heroic's UI has controller navigation and a console mode, through Chromium's Gamepad API
(`src/frontend/helpers/gamepad.ts`, `src/frontend/screens/ConsoleMode/`). The Flatpak gets `--device=all`, the
Wayland, X11 and PulseAudio sockets, and `/run/udev` read-only.

## What Padwight already has

The parts that matter for a session already work in the daemon:

- **Taking and releasing controllers at runtime.** `SetEnabled(false)` releases every grabbed pad
  (`src/daemon/requests.rs`), and every grab checks `should_manage` first (`src/daemon.rs`).
- **Finding the game.** Library packs match on executable names, and the Wine/Proton `.exe` detection in
  `src/focus/identify.rs` already covers games launched from Heroic. `src/launchers.rs` reads Heroic's
  installed-games lists, Flatpak included.
- **Remote control.** The 1.0 [control protocol](../protocol.md) is what a launcher would use if it wanted more than
  "on for this game".

### A rough version with no code changes

You could try this today with Heroic before/after scripts that send `SetEnabled` and `Activate` over the socket with
`socat`. It falls short of the goal in three ways:

1. `SetEnabled` saves to `config.toml`.
2. The daemon still runs and tracks focus between games.
3. From the Heroic Flatpak, Padwight's socket isn't visible without a `flatpak override`.

## Proposed design: a session mode

A new process role, `padwight run -- <game command>`, which Heroic runs as a game wrapper. The engine, overlay
process, packs and settings window stay as they are.

```
Play in Heroic ──► padwight run starts ──► Pads taken
(runs the game's    (or joins a running     (setup from HEROIC_APP_NAME)
 wrappers)           session)                    │
                                                 ▼
Pads released  ◄── Game's processes exit ◄── Game runs
(virtual devices    (padwight watches        (sees only the virtual pad)
 removed)            its child)
```

Padwight takes the pads before the game starts, so the game sees the virtual pad from its first frame. Nothing is
left behind when the session ends.

The work:

1. **The `run` role.** Start the daemon logic in-process, or join a session that's already running. Grab the pads,
   spawn the child, and release everything when the child's process tree exits.
2. **No desktop-wide setup in this mode.** No KWin script, no GNOME Shell extension (`src/focus/gnome.rs` installs
   one into the user's extensions folder), no tray, no systemd.
3. **The setup comes from the launcher.** Use `HEROIC_APP_NAME` and `HEROIC_APP_SOURCE`, or the child's executable.
   This may need a new rule kind for a launcher's app ID.
4. **Enabled state in memory only.** A session never saves its on/off state to the config.
5. **Two games, or a second launch.** A second `padwight run` attaches to the running session instead of fighting
   over the pads.

## The Heroic fork: Padwight built in

A local fork off Heroic's main branch, where Heroic starts Padwight's session itself and its game settings get a
Controller tab. Heroic's backend is Node, so it talks the control protocol over the socket directly, with no scripts
or `socat`.

What the fork changes:

1. **Launch.** When a game's Padwight switch is on, the backend adds `padwight run` to its wrappers in
   `setupWrappers`, the same way it adds MangoHud.
2. **A Controller tab.** A new tab in game settings, beside Wine, Other and Advanced
   (`src/frontend/screens/Settings/sections/GamesSettings/index.tsx`).
3. **Config tied to the Heroic game.** Heroic's `GamesConfig/<appName>.json` holds only the switch and the name of
   the Padwight setup. The setup stays in Padwight's config folder, matched by a new rule kind for a launcher's app
   ID (today `RuleKind` has Executable, SteamAppId and WindowClass). One copy means the engine, overlays and pack
   import keep reading one store, and the two can't drift apart.

### How much settings UI to rebuild

| Approach | What Heroic shows | Cost |
|---|---|---|
| Core tab plus Padwight's editor (recommended) | Switch, a setup picked from the library, its profiles, the controllers, a read-only button map; "Edit in Padwight" opens Padwight's own window on that setup | A few React components on `GetConfig`, `SetConfig` and `Status` |
| Full rewrite in React | Everything Padwight's settings window has | That window is about 20,000 lines of Rust in 66 files under `src/gui`, and every new feature would land twice |
| Launch only | The switch; all editing in Padwight's window | Smallest, but no settings in Heroic |

The core tab gets most of the benefit for little code, and the in-game editor already covers changes during play.
Opening Padwight's window on one setup needs a new argument: today `padwight gui` takes none.

Keep the fork's changes to a few places (the wrapper in `launcher.ts`, one settings tab, one small backend module
for the socket). Heroic's main branch moves often, so a small fork rebases cheaply.

## The ambition: Heroic ships with Padwight, nothing else to install

Only partly feasible today. Remapping with nothing extra installed works where `/dev/uinput` is already open to the
user, which is usually where Steam's device rules are installed. It fails on a stock desktop without them, in
Heroic's Flatpak on such a system, and on Windows. Overlays work on KDE Plasma and native wlroots compositors only.

### What Padwight needs from the system

| Need | What a system gives without extra installs | What Padwight ships today |
|---|---|---|
| Read and grab the controller (evdev) | Yes: systemd's uaccess rule covers joysticks | Nothing needed |
| Create the virtual pad, keyboard and mouse (`/dev/uinput`) | Root only, unless a package added a uaccess rule. Steam's `steam-devices` adds one (`60-steam-input.rules`); this machine has it from Fedora's `steam-devices` package | `dist/70-padwight-uinput.rules` |
| Gyro (the controller's motion sensors) | No: uaccess covers joysticks, not the separate accelerometer node | `dist/70-padwight-motion.rules` |
| Stop games reading the pad directly through hidraw (double input) | No | `dist/72-padwight-hidraw.rules` |
| Overlays over the game | Wayland layer-shell: KDE Plasma and wlroots yes; GNOME, X11 and gamescope no; wlroots hides it from Flatpak apps | Nothing more can be shipped |

uinput is the gate: without it, Linux gives Padwight no way to send a game anything. Every other row degrades a
feature rather than stopping remapping.

### By Heroic's channel

| Channel | Can it set up access at install? | Result with nothing else installed |
|---|---|---|
| deb, rpm, pacman | Yes: they install as root, so they can ship Padwight's udev rules | Full remapping and gyro. Overlays depend on the desktop |
| Flatpak (Flathub) | No: a Flatpak can't install udev rules | Works only where `steam-devices` or similar is already there (Steam Deck, gaming distros, most systems with native Steam). Flathub's Steam already asks its users to install `steam-devices` |
| AppImage, tar.xz | No | Same as Flatpak; a one-time admin prompt (pkexec) could install the rules |
| Windows (NSIS installer) | It runs as admin, so it could bundle the drivers | Needs ViGEmBus for the virtual pad (archived since 2023, a kernel driver) and HidHide to hide the real pad (see [windows-dependencies.md](windows-dependencies.md)). Keyboard and mouse work without them |
| macOS | n/a | Padwight doesn't run on macOS |

### What would close the gaps

1. **Double input without the hidraw rule.** As the game's wrapper, Padwight sets its environment, so it can tell
   SDL to leave hidraw alone (`SDL_JOYSTICK_HIDAPI=0`). Whether that covers Proton's own hidraw path needs testing.
2. **Gyro without the motion rule.** Steam's rules open PlayStation and Nintendo hidraw nodes to the user, so
   Padwight could read motion there. That's a new input path, and it conflicts with the hidraw rule above.
3. **Overlays everywhere.** Drawing inside the game's own frames, as MangoHud does with a Vulkan layer, would work on
   GNOME, X11 and gamescope. It's a large piece of work, and it replaces the layer-shell window rather than adding to
   it.
4. **Steam running alongside.** Steam reads controllers through hidraw even for games it didn't launch.
   Without the hidraw rule, that conflict is the user's to sort out.

Licensing is no obstacle: Heroic and Padwight are both GPL-3.0.

## Risks and open questions

Gamescope is the biggest gap. The others are things to check, not blockers.

| Risk | What happens | What to do |
|---|---|---|
| Gamescope (Heroic's gamescope option, Steam Deck Game Mode) | Gamescope has no layer-shell, so remapping works but the overlays (menus, keyboard, Guide) don't draw. The [overlay renderer plan](overlay-renderer-plan.md)'s gamescope test (step 8.3) hasn't been done. | Open question: a fallback way to show the overlay inside gamescope, or ship without overlays there. |
| Steam Input on the Deck | Heroic games in Game Mode usually run as non-Steam shortcuts, so Steam Input also wants the controller through hidraw. | Same conflict as Padwight's existing Steam notes; reuse that approach. |
| Heroic Flatpak sandbox | The wrapper runs inside Heroic's sandbox. `--device=all` makes evdev and uinput reachable, and Wayland and PulseAudio are there, so a sandboxed session looks possible. | Check that the padwight binary's libraries load in Heroic's runtime, and decide how users make the binary visible in the sandbox (a visible path, or a Flatpak extension). |
| No focus tracking | Alt-tabbing to the desktop keeps the game's mapping until the game exits. | Accept it as session behaviour, or keep a light focus check that only switches off. |
| Heroic's own controller navigation | While Padwight holds the pad, Heroic's UI sees the virtual Xbox pad; afterwards it sees the real one again. | Probably fine; confirm Heroic doesn't react to Guide while a game runs. |
| Startup time | The overlay's font scan took about 1.5 s from a cold cache in the renderer plan. | Measure; the session must be ready before it starts the game. |

## Next steps

The fork comes first; offering a smaller "Use Padwight" toggle upstream, like the MangoHud and GameMode ones, can
wait until the session mode has been used for a while.

- [ ] Build `padwight run` (session mode) against the daemon code
- [ ] Add a rule kind for a launcher's app ID, so a setup is tied to a Heroic game
- [ ] Add a setup argument to `padwight gui`
- [ ] Fork Heroic locally off main; add the wrapper and a per-game Padwight switch
- [ ] Add the Controller tab: core settings and "Edit in Padwight"
- [ ] Test under gamescope and on a Steam Deck in Game Mode
- [ ] Measure the time from launch to the game's first frame with and without the wrapper
