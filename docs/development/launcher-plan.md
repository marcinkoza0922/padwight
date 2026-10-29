# Plan: launching games through Padwight

A plan for starting games from Padwight, so the daemon knows which game is running from the
moment it starts and can stop guessing from other programs' windows and processes. It covers
Steam, Heroic and Lutris games as well as games added by hand, without changes to the stores'
settings or files and without root. Nothing here is built yet.

## Why

Today the daemon works out which game is in front after the fact: a tracker for the desktop
reports the focused window, `focus/identify.rs` reads its process from `/proc` (with the sandbox
pid lookup in `host_pid`), and the setups' rules pick the profile. That has limits:

- **The Flatpak can't see host processes.** In the sandbox a window is matched by its class and
  title only, the process-scan fallback (Sway, labwc) finds nothing, and force quit can't reach
  the game.
- **Force quit walks the parent tree** (`quit.rs`), which misses processes that double-fork or
  are re-parented, such as wineserver and launcher stubs.
- **The profile switches after the first window gets focus**, not when the game starts.
- **Focus detection is intrusive.** It reads the environment and executable of whatever process
  is focused, matches every window against every setup's rules, and scans all processes where
  there is no tracker. Some people won't want that.
- **The game's environment is fixed by the time we see it**, so the variables that would hide
  the real controller from SDL and Proton, or give the game a mapping for the virtual pad, can't
  be set.

## What being the game's parent would give, and what we get without it

Most people start games from Steam, Heroic or Lutris, and those make the game their own child.
Being the parent would give four things. Three of them can be had another way:

| | How we get it |
|---|---|
| Which process is the game, with its whole tree | Steam's `reaper` names the game (below). Heroic and Lutris: what their processes carry, or the manifest's executable |
| The moment it exits | `pidfd_open` on the game's root process, which works for any process we can see, not only children |
| Force quit that catches everything | Kill the root's tree. Steam's `reaper` keeps the whole game under it |
| Changing the game's environment | Only with the optional wrapper. Its main use, hiding the real controller, has other routes (below) |

So the design is: **import the stores' games, start them through the store's own link, and
follow the game the store starts.** A wrapper (`padwight-run`) that a launch option puts in
front of the game is an optional upgrade, for what only the environment can do.

## What we build

1. **Titles**: games imported from Steam, Heroic and Lutris, each linked to a setup. Games the
   user added to those by hand (Steam's non-Steam shortcuts, Heroic's "Add game", Lutris's
   locally installed games) come in too.
2. **A Games page** in the settings window to import and launch them.
3. **Launch sessions in the daemon**: a game started from the Games page, found and followed
   through the store's own processes, with its setup, its exit, and a way to end it.
4. **`padwight-run`**, an optional wrapper for a store's launch options.
5. **A launched-games-only mode**: setups apply only to games started from the Games page or
   through `padwight-run`.

Not in this plan:

- **Games added in Padwight itself** (a program, its arguments, Wine or Proton). The stores
  already add games by hand, with their runners, Proton versions and prefixes, and a store link
  starts them on the host even from the Flatpak, which Padwight can't do for a program of its
  own (see [The Flatpak](#the-flatpak)). The Games page points there instead. A Padwight-only
  title could come later, for the native install, if people without any store want it.
- **Launching from the controller** (a games list in the Guide overlay). It needs a UI of its own
  and is a project in itself.
- **A Steam compatibility tool** (a tool in `~/.steam/root/compatibilitytools.d/`, picked as
  Steam's default, that would put the wrapper in front of every Proton game, as Steam Tinker
  Launch does). It adds a file to Steam's folder and has to find and run Proton itself. Maybe
  later.
- **Artwork** for titles. Names are enough until the approach is proven. Steam keeps covers in
  `appcache/librarycache/<id>/` and Heroic in its images cache, if it's wanted later.
- **Starting Steam with the SDL variables** (they'd change Steam Input too, and Steam is usually
  running already) and **`LD_PRELOAD`** or any other code in the game (anti-cheat).

## Titles

A **title** is something that can be launched. A **setup** (`config::Game`) is how Padwight
plays it. They are separate because an import brings in many titles that need no setup of their
own (they play on General), and one setup can cover the same game from two stores.

```toml
[[titles]]
id = "steam-1245620"           # the store and its id
name = "Elden Ring"
source = "steam"               # steam | heroic | lutris
store_id = "1245620"           # the id for the store's link
runner = ""                    # heroic: legendary | gog | nile | sideload
setup = "Elden Ring"           # empty: matched through setups' rules, else General
env = { }                      # set on the game through the wrapper only
hidden = false                 # imported but not wanted in the list
```

The code calls them `Title` (`src/titles.rs`), not `Game`, which is already the setup. In the UI,
"Games" is the page of titles and "Setups" stays the sidebar list, so the two words mean two
things.

### Importing

`launchers.rs` already reads Steam's app manifests, Heroic's `installed.json` files and Lutris's
game files, but only for ids and executable names. The import extends that to names and launch
details:

- **Steam**: `appmanifest_*.acf` gives the id, name and install folder. Leave out tools such as
  Proton, the Steam Linux Runtime and Steamworks redistributables (a short list of known ids to
  start with). The link is `steam://rungameid/<id>`.
- **Steam's non-Steam shortcuts**: `userdata/<account>/config/shortcuts.vdf` (binary VDF) gives
  each shortcut's `appid`, `AppName`, `Exe` and `LaunchOptions`. Read the account in
  `loginusers.vdf` marked most recent. The link is `steam://rungameid/<id>`, where the id is the
  shortcut's `appid` shifted into the top 32 bits with `0x02000000` in the low ones. Shortcuts
  that start another store's game (`heroic://…` or `lutris:…` in `Exe` or `LaunchOptions`, which
  is common) are that store's titles: skip them so the game isn't listed twice.
- **Heroic**: Legendary's and GOG's `installed.json`, Nile's (Amazon), and
  `sideload_apps/library.json` for games added by hand give the app name, title, runner and
  executable. The link is `heroic://launch?appName=<name>&runner=<runner>`.
- **Lutris**: `pga.db` (SQLite) lists every game, locally installed ones included, with its id,
  name, runner and executable (the `games` table). The link is `lutris:rungameid/<id>`.

The import runs on demand ("Import games") and when the page opens, off the UI thread as
`launchers::scan` already does. Games found again keep their settings, games that are gone are
marked uninstalled rather than removed, and the same game in two stores stays two titles that can
share a setup. Each new title is matched to a setup through the setups' rules, which
`Installed::has` already does for the library picker. Titles that match a library pack the user
doesn't have get "Add setup", the same offer the daemon makes for a focused game today.

The Flatpak already reads Steam's and Lutris's libraries (`--filesystem=…:ro` in the manifest).
Heroic's folders and Lutris's `pga.db` need adding there, read-only.

### Packs

Titles live in the user's config only. **Third-party packs carry nothing about launching**: no
titles, commands, arguments or environment variables, not even as suggestions. Importing a pack
that has such fields fails with a message that names them. **Library packs** (the ones built into
Padwight) may suggest environment variables for a game, from a fixed allowlist (`SDL_*`,
`PROTON_*`, `WINE*`), which the user turns on per title. Nothing else about launching comes from
a pack.

## The Games page

A new sidebar entry, **Games**, above Setups. The page shows:

- The titles, installed ones first, with a search (the virtualized list in `add_game.rs` handles
  thousands of rows and can be reused), each with its store, its setup, whether it's running,
  and whether it came through the wrapper last time.
- **Play** / **Stop** per title. Stop ends the game (see [Ending a game](#ending-a-game)).
- A title's details: its setup (a picker, or "match by rules") and the **wrapper upgrade**: what
  it adds, the launch-option line with a Copy button, and a link to the guide.
- **Import games**, and **+ Add game**, which explains how to add a game to Steam (Add a
  Non-Steam Game), Heroic (Add game) or Lutris, and imports again when the user comes back.

## Launch sessions

Play opens the store's link, and the daemon opens a
**launch session** expecting that title. A `LaunchSession` holds the title, the setup, how the
game is followed (below), the game's root process once found, the window classes seen for it,
and when it started. A store game started by the user from the store itself, with the wrapper in
its launch options, opens a session too.

While a session is open:

- Its setup is active while one of its windows is in front. A focused window belongs to a
  session when its pid is in the game's tree, or, where the pid can't be checked (the Flatpak),
  when its class is the session's (`steam_app_<id>` for Proton games) or a class already seen
  for it. With several sessions, the one in front wins. When another window is in front, the
  default profile comes back, as when a game loses focus today.
- `game_started` (`daemon/auto_switch.rs`) runs when the game's root is found, rather than on
  the game's first focus, so start-on toggles and start-up info overlays happen on time.
- When the game exits, the session closes.

A session whose game doesn't show up within a couple of minutes (the store asked to update, or
the user cancelled) closes on its own, and the Games page says so.

### Following a Steam game

Steam starts every game as `reaper SteamLaunch AppId=<id> -- …` (the string is in
`steamclient.so`). `reaper` is a subreaper: everything the game starts stays under it, double
forks included. The native daemon looks for the `reaper` with the expected App ID among new
processes, takes it as the game's root, and gets its exit through `pidfd_open`. That reads only
`/proc/<pid>/cmdline` of new processes named `reaper`, nothing else.

In the Flatpak, which can't see host processes, Steam's `~/.steam/registry.vdf` may say which game
is running (`RunningAppID`, to check). With read-only access to that one file, the session opens
when it names the expected game and closes when it changes. Proton games' windows carry the App
ID in their class, so they're matched exactly. Native Linux games are matched by the first new
window after the game starts, and then by its class.

### Following a Heroic or Lutris game

Neither has a process like `reaper` that names the game. In order of preference:

1. A variable they set on the game's processes that names it (umu's `GAMEID`, or their own;
   to check), read from `/proc/<pid>/environ` of new processes only.
2. The executable named in the game's install manifest, among processes that appear after Play.

The root is the topmost process of the game's that isn't the store's own. Its tree is followed
by pid, so a double fork can escape it. In the Flatpak, a Heroic or Lutris game is matched by the
first new window after Play, and then by its class, and its exit isn't known until its windows
are gone.

### Through the wrapper

With `padwight-run` in a game's launch options (see [The wrapper](#the-wrapper)), the session
gets everything from the wrapper: the exact tree, the exit, the environment, and in the Flatpak,
the host's view of the game's processes. This makes Heroic, Lutris and Flatpak sessions as exact
as native Steam ones.

### Ending a game

Stop and force quit ask the game to close (SIGTERM to the tree), then kill it after
`quit::GRACE`. Native: the root's tree (for Steam, everything under `reaper`). Through the
wrapper: the wrapper does it, through its scope or process group, which also works from the
Flatpak. Flatpak without the wrapper: not possible, so Stop isn't offered there.

## Hiding the real controller

The game reading the real pad alongside the virtual one (double input, or the wrong pad in the
first slot) is the main reason to want the game's environment. Two routes don't need it:

- **The hidraw rule** (`dist/72-padwight-hidraw.rules`, see the README's Permissions), a one-time
  root install that already exists. It stops SDL, Steam and browsers from reading the pad
  through hidraw.
- **Detaching the pad's USB driver** through usbfs, so no program sees the real pad at all. It
  needs no root and works live. Bluetooth pads aren't covered. See the `steam-input-investigation`
  branch (`docs/development/steam-input.md` there). It's its own feature, not part of this plan,
  but it means the wrapper isn't needed for this.

With the wrapper, a title can also turn on **Hide real controllers**, which sets
`SDL_GAMECONTROLLER_IGNORE_DEVICES` (with the grabbed pads' vendor and product ids) and Proton's
hidraw switch for that game only.

## The wrapper

`padwight-run [--title <id>] [--setup <name>] -- <command>…`, used in Steam's launch options
(`padwight-run -- %command%`), Heroic's per-game wrapper setting and Lutris's command prefix.
The user sets launch options themselves, following the guide.
**Padwight never writes them** (no edits to Steam's `localconfig.vdf` or other store files).

1. Connects to the daemon's socket and sends a new `Request::Launch` with its pid, the command,
   the working directory, the title or setup if given, and the store ids it inherited
   (`SteamAppId`, `SteamGameId`, `STEAM_COMPAT_APP_ID`, Heroic's and Lutris's variables).
2. Gets back the session's setup and the environment to add (the title's variables and Hide
   real controllers).
3. Becomes a child subreaper (`PR_SET_CHILD_SUBREAPER`), so everything the game starts stays
   under it. It then starts the command with that environment, in a process group of its own
   (and, where systemd is there, a transient scope from `systemd-run --user --scope`, so ending
   the game is one `cgroup.kill`).
4. Stays running as the parent. It passes on signals, keeps the socket open (the daemon sees the
   game end when the socket closes), answers whether a pid belongs to the game's tree, ends the
   tree when the daemon asks, and exits with the game's exit status.

If the daemon isn't running, the wrapper starts the game anyway with no changes and says so on
stderr: a launch option must never stop a game from starting. If the daemon restarts, the
wrapper reconnects and registers again.

### The guide

A manual page (`docs/manual/`), with annotated screenshots from the tutorial pipeline, that shows
how to add the line to a Steam game's launch options, Heroic's wrapper setting and Lutris's
command prefix, and what it adds: per-game variables, Hide real controllers, and exact tracking
for Heroic, Lutris and the Flatpak. The Games page links to it from each store title.

## Launched-games-only mode

A setting, **Detect games**, with two choices:

- **By focused window** (today, the default): every focused window is matched against the setups'
  rules.
- **Only games launched through Padwight**: setups apply only to launch sessions, from Play or
  from the wrapper.

In the second mode the focus tracker still runs, because a game's setup must turn off while
another window is in front. What changes is what the daemon does with the focused window: it
checks whether the window is a session's, and nothing else. It doesn't read the focused process's
`/proc` entries (only the new processes a session is waiting for, as above), doesn't match rules
against windows, doesn't scan processes, and makes no library offers for games it didn't start.
The compositor piece stays: the KWin script, the GNOME Shell extension, or the Hyprland or Sway
socket.

Where there is no focus tracker at all (the Flatpak on Sway, labwc and other wlroots desktops,
see below), the mode can't tell when the game is in front. There, the setting is greyed out with
the reason, and points at the native install.

Other features use the focused window today. In the second mode:

| Feature | By focused window | Only launched games |
|---|---|---|
| Auto switch (`auto_switch.rs`) | Focused window's setup, by rules | The session's setup while its window is in front |
| Game start (`game_started`) | First focus after launch | When the session finds the game |
| Force quit (`quit.rs`) | Focused window's process tree | The session's game, if its window is in front. Nothing otherwise |
| Close overlays on focus change (`close_all.rs`) | When focus moves away | The same |
| Library offer (`offers.rs`) | When a game with a library pack gets focus | When a title is imported or added |
| In-game menu's "add setup for this game" (`in_game_menu.rs`) | Focused window | The session's title. Not shown for other windows |
| Window details in info overlays (`info_overlays.rs`) | Focused window | The session's title |

## The Flatpak

### What the sandbox can see

| Desktop | Focused window (class, title, pid) | The window's process |
|---|---|---|
| KDE Plasma | Yes, from the KWin script over `org.kde.KWin` | No |
| GNOME | Yes, from the Shell extension over `org.gnome.Shell` | No |
| Hyprland | Yes, from its socket (`xdg-run/hypr`) | No |
| Sway, labwc, other wlroots | No: the IPC socket isn't shared, and the compositor hides its toplevel protocols from sandboxed apps | No |

The pid the compositor reports is the host's, which names nothing inside the sandbox.

### What works there

| | Flatpak, no wrapper | Flatpak, with the wrapper |
|---|---|---|
| Import | Yes (read-only library access) | Yes |
| Play | Yes: the store's link opens through the desktop portal (`org.freedesktop.portal.OpenURI`), as a browser opens Steam links | Yes |
| Which game, start and exit | Steam: `registry.vdf` and the window class. Heroic, Lutris: first new window, then its class | Exact, from the wrapper |
| Stop and force quit | No | Yes, through the wrapper |
| Per-game variables, Hide real controllers | No | Yes |

**Why every title goes through a store.** Every program Padwight starts runs inside its
sandbox: with the Freedesktop runtime's libraries instead of the host's, without the host's
`/usr`, Wine or `umu-run`, without the folders the game lives in, and with Padwight's own
permissions. A store link is different because the portal hands it to Steam, Heroic or Lutris,
which run on the host and start the game there. Every way to start a program on the host from
the sandbox (`flatpak-spawn --host` with `org.freedesktop.Flatpak`, or a transient unit through
`org.freedesktop.systemd1`) lets the sandbox run anything on the host, a full escape. The
portal's own launchers (`OpenFile`, `DynamicLauncher`) won't run a program, or run it back
inside the sandbox. So a game Padwight doesn't import from a store gets added to one.

### The wrapper on the host

The wrapper has to run on the host. `flatpak run … -- %command%` would start the game inside
Padwight's sandbox, with its runtime and permissions, and break it.

- Link `padwight-run` statically (musl), so it needs nothing from the Flatpak runtime. The
  native install ships the same binary, so there is one wrapper.
- The host can run it from the Flatpak's own install
  (`~/.local/share/flatpak/app/io.github.marcinkoza0922.Padwight/current/active/files/bin/padwight-run`,
  or `/var/lib/flatpak/…` for a system install). That path survives updates, so the launch-option
  line stays the same, and no file has to be written to the host.
- It finds the socket at `$XDG_RUNTIME_DIR/app/io.github.marcinkoza0922.Padwight/`, which
  `ipc::shared_runtime_dir` already uses because the host can see it.
- **Flatpak Steam, Heroic or Lutris** run launch options inside their own sandbox, which can see
  neither the wrapper's path nor the socket. The user would have to give the store's Flatpak
  access with `flatpak override`. Padwight doesn't do that for them. The guide says the wrapper
  is unsupported there for now. Import and Play still work.

## Windows

Same shape, later. Steam on Windows has no `reaper`: the game is a child of `steam.exe`, found
by its executable from the manifest. `padwight-run.exe` works in Steam's launch options, with a
Job Object in place of the subreaper and cgroup (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, and the
job's process list for the tree), and a named pipe in place of the socket. Hiding the real
controller on Windows is HidHide's job, which is out of scope.

## Phases

Each phase can be merged on its own.

0. **Check the assumptions** (see [To check first](#to-check-first)) on the test VMs and on a
   Steam install, before writing the real code.
1. **Titles, import and the Games page.** The `[[titles]]` config, the import from Steam (with
   its shortcuts), Heroic (with sideloaded games) and Lutris, matching to setups, the library
   offer. Play opens the store's link. No sessions yet: setups still switch by focus as today.
2. **Launch sessions.** Following Steam games through `reaper`, Heroic and Lutris games by their
   variables or executables, `pidfd` exits, setup activation while the session's window is in
   front, `game_started` when the game is found, Stop and force quit through the session.
   Tests: a fake store that starts a fake `reaper` with a game that double-forks, and the daemon
   tests in Docker.
3. **The wrapper.** `padwight-run`, `Request::Launch`, sessions from the wrapper, per-game
   variables, Hide real controllers. The guide, with its screenshots.
4. **Launched-games-only mode.** The setting, the session-only focus handling, the table above.
5. **The Flatpak.** Play through the portal, `registry.vdf`, window-class matching for sessions,
   the static wrapper and its host path, the manifest's new read-only paths.
6. **Windows.**

## To check first

- Steam writes `RunningAppID` (or a per-app `Running` key) into `~/.steam/registry.vdf` while a
  game runs, and clears it when the game exits, promptly enough to follow.
- `reaper`'s command line has the App ID for every game: native, Proton, and non-Steam shortcuts
  (which would have the shortcut's `appid`).
- `steam://rungameid/<id>` with the id worked out from a shortcut's `appid` starts that shortcut,
  and Heroic's `runner=sideload` link starts a game added by hand.
- What Heroic (Legendary, GOG, Nile) and Lutris set on a game's processes that names the game.
- The desktop portal opens `steam://`, `heroic://` and `lutris:` links from the sandbox, and what
  it asks the user the first time.
- The pid KWin, GNOME and Hyprland report for a Proton game's window (an X11 window through
  XWayland) is in `reaper`'s tree, and the same for native games on Wayland.
- With the wrapper: Steam's launch-option prefix runs outside the Steam Linux Runtime container,
  the variables it sets reach the game inside pressure-vessel and Proton, and Steam still tracks
  the game as running with our subreaper between `reaper` and the game.
- `SDL_GAMECONTROLLER_IGNORE_DEVICES` with the real pad's ids hides it from SDL's HIDAPI driver
  as well as from evdev, and Proton's hidraw path respects it or has its own switch.
- Heroic's wrapper setting and Lutris's command prefix take a command with arguments, and pass
  on the variables.
- A static binary inside the Flatpak's install folder runs from the host, from native Steam's
  launch options.
