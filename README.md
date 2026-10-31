# padwight

Ever wanted to play a PC game with a controller, only to find it ignores your pad, or only half-listens to it? padwight is for that. It sits between your gamepad and the desktop and turns what the pad does into something the game understands: keyboard keys, mouse movement, clicks, or a virtual controller of its own.

It's built for two kinds of game:

- **Games with no controller support, or only partial support.** Point a stick at the mouse or at WASD, put a macro on a button, give a trigger a mouse click, or send a whole combo of keys on one press. Missing a button the game never bound? Want gyro aiming on a pad that has a gyro? padwight can fill those gaps. Games that play well with a pad often still have a launcher that wants a mouse, or a name field that assumes you're sitting at a keyboard, and padwight covers those too (see [Mouse and keyboard from the couch](#mouse-and-keyboard-from-the-couch)).
- **Games that already work with a controller, but could go further.** Remap buttons on the fly, in the middle of a game, without leaving it. Where a game allows it, add gyro aiming or a flick-stick setup on a controller that has the sensors. Use macros to pull off complex inputs, like fighting-game motions, with one press. And control media playback from the pad while you play.

It runs quietly in the background and has a settings window for the fiddly parts. Your setups are plain files, so they're easy to back up, share, or copy to another machine.

Here's how it's put together:

- **daemon**: grabs physical gamepads (evdev) so nothing else sees them, runs the active profile, and emits through virtual uinput devices (an Xbox 360-style pad plus a keyboard and a mouse).
- **gui**: edits profiles and talks to the daemon over `$XDG_RUNTIME_DIR/padwight.sock`.
- **config**: `~/.config/padwight/`. `config.toml` holds the app settings. Each setup is a folder under `setups/` (General is `general/`), with a `setup.toml` and one file per profile, mode, layer, macro, cycle, menu, overlay and log. To move a setup to another machine, copy its folder. The daemon is the only writer while it runs, so after editing by hand, run `padwight reload`.

## Documentation

For users:

- **Manual**: a plain-language guide, shown on the Manual page of the settings window. Its source is [docs/manual/](docs/manual/), plain Markdown that is also built into the app, so the two can't drift apart. It has a step-by-step tutorial with annotated screenshots for every major feature (the `tutorial-*.md` chapters).
- [docs/manual/tutorial-deus-ex.md](docs/manual/tutorial-deus-ex.md): a tutorial that walks through making a pack, using Deus Ex on an Xbox-style pad, with the reasons behind the less obvious mappings.
- [docs/pack-format.md](docs/pack-format.md): reference for the `.padpack` format (format 20).
- [docs/protocol.md](docs/protocol.md): the daemon's control protocol (version 1.0), for writing your own client, with [examples/dashboard/](examples/dashboard/), a browser dashboard in plain HTML and Python.

For developers, in [docs/development/](docs/development/):

- [architecture.md](docs/development/architecture.md): how the processes, input pipeline, config and focus tracking fit together, with a module map and test commands.
- [specs.md](docs/development/specs.md), [modes.md](docs/development/modes.md), [layers.md](docs/development/layers.md), [sticks.md](docs/development/sticks.md), [guide.md](docs/development/guide.md): design notes for individual features.
- [game-setup-guidelines.md](docs/development/game-setup-guidelines.md): where actions usually go when setting up a game's bindings (sticks, buttons, hotkeys, system buttons).
- [overlay-debugging.md](docs/development/overlay-debugging.md): tools for checking the on-screen overlay's timing and fades without a monitor or controller: a headless harness, the overlay's timing log, and fade pictures.
- [manual-screenshots.md](docs/development/manual-screenshots.md): how the manual's annotated screenshots are made, and made again when the window changes: `padwight debug gui` and `scripts/annotate.py`.

## Setups

Everything is organized into setups: one controller setup per game. Each game keeps its own bindings and switches to them when you start it. A setup holds its own profiles, macros, cycles, menus, info overlays, and the auto-switch rules that activate it. All of its profiles can use all of its items. **General** is a built-in setup with no rules, for the desktop and a plain gamepad. Names only need to be unique within a setup. To reuse items from another setup, use **Copy from another setup…** on its Modes, Layers, Macros, Cycles, Menus or Info overlays tab.

The sidebar lists General and your setups (with a search). Each setup's page has nine tabs: **Profiles**, **Modes**, **Layers**, **Macros**, **Cycles**, **Menus**, **Info overlays**, **Log overlays** and **Details** (name, rules, export, delete). On every tab except Profiles and Details, "Copy from another setup…" previews another setup's items (or one from the library) and copies the ones you want in, so you can adapt them.

## Packs and the library

Found a setup worth sharing? Export it as a **pack**: **Details → Export…** writes a `.padpack` file (TOML). The export dialog warns you about references to missing items.

Whether a pack needs more than a plain XInput pad (gyro, back paddles, or a touchpad) is something you declare on each **profile**, not on the pack. On the profile's Gyro tab, tick *Profile requires gyro* when it depends on gyro (a flick stick setup that turns up and down with gyro, say). Leave it unticked when gyro only adds to a scheme that works without it. The back paddles section of the Buttons tab and the Touchpad tab have the same box. It's your word, nothing is detected, and the export dialog just lists what you've declared. Players whose controller lacks a feature only see the profiles that don't need it, and Guide skips the others. Re-exporting your own setup keeps its pack ID. Exporting someone else's makes a fork with its own ID that credits the original.

**+ Add setup** opens the picker. It offers the built-in library first (setups for games you have installed in Steam, Heroic or Lutris, or that are running), an empty setup from a template, or **Import a file…**. Importing shows a preview before anything changes: what's inside, what your controller lacks, whether a setup with the same name already exists (add under a new name, or replace yours), and other setups' rules for the same window (the imported game takes over unless you keep yours). Nothing changes until you choose Save & apply.

Imported setups are ordinary, editable setups that remember their pack. When a newer version of the same pack turns up, or an app update ships a newer library version (marked "update" in the sidebar), you're offered the update, along with a list of what you've changed since. A pack made by a newer app version is refused, with a request to update the app.

The library is `packs/*.padpack` in this repository, embedded at build time. See `packs/README.md`.

## Install

From a release tarball (Linux, x86_64):

```sh
tar xzf padwight-*-linux-x86_64.tar.gz
cd padwight-*-linux-x86_64
./install.sh
```

It installs padwight for your user, starts the background service at login, and, after asking, copies three small system files with `sudo` (see Permissions below). `./install.sh --uninstall` removes it. `scripts/package.sh` builds the tarball; build it on the oldest distro you want to support, as the binary needs at least the glibc it was built against.

From source:

```sh
cargo install --path .
mkdir -p ~/.config/systemd/user
cp dist/padwight.service ~/.config/systemd/user/
systemctl --user enable --now padwight
padwight            # open the GUI
```

For an application-menu entry and icon for the settings window:

```sh
install -Dm644 dist/io.github.marcinkoza0922.Padwight.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.marcinkoza0922.Padwight.svg
install -Dm644 dist/io.github.marcinkoza0922.Padwight.desktop ~/.local/share/applications/io.github.marcinkoza0922.Padwight.desktop
```

Controllers can be unplugged, reconnected or swapped while padwight runs, and it copes. A disconnect releases everything that controller was holding and closes every overlay it was driving (unsaved in-game menu changes are put aside until the menu opens again). A toast names the controller that left or arrived. Your profile stays put, so reconnecting the same pad picks up where you left off.

If the pad that comes back (or replaces it) lacks something the active profile needs, such as gyro, the setup's first profile that doesn't need it takes over. If none does, the default profile takes over, and a toast tells you so. When a pad with the feature is connected again, your profile returns, unless you've changed it since. The switch only happens once the controllers have looked the same for two 2-second scans, so a motion sensor that appears a moment after its pad won't make the profile flicker.

### Permissions

`padwight doctor` checks all of this and prints the exact commands for whatever is missing; `padwight doctor --explain` says what each permission is for. The settings window also lists what's missing while the daemon isn't running, and the daemon logs it if it can't start.

- **Write access to `/dev/uinput`**, to create the virtual gamepad, keyboard and mouse. On most desktops (systemd-logind with `uaccess`) you already have it. If not, install `dist/70-padwight-uinput.rules`, which tags the node for `uaccess` so the user at the machine gets access. If `/dev/uinput` doesn't exist, the `uinput` kernel module isn't loaded: `dist/padwight-uinput.conf` in `/etc/modules-load.d/` loads it at boot.
- **Read access to the controller's `/dev/input/event*` node.** `uaccess` covers joysticks. If yours doesn't, add yourself to the `input` group (`sudo usermod -aG input $USER`) and log in again.
- **Read access to the controller's motion sensor**, for gyro on PlayStation and Switch controllers. It's a separate node that `uaccess` doesn't cover. Install `dist/70-padwight-motion.rules`, which grants it for the `playstation`, `sony` and `nintendo` drivers only, so a laptop's tilt sensor is untouched. Without it, everything but gyro works.
- **Optional: no access to PlayStation and Switch controllers' raw HID node** (`/dev/hidraw*`). Steam, SDL games and Chromium browsers read those controllers there, past padwight's hold on the input node, so they see every press as well as padwight's virtual pad. `dist/72-padwight-hidraw.rules` closes the raw node to every program (padwight doesn't use it), so they read the input node, which padwight holds. Turn the controller off and on after installing it. The cost: while padwight is off, Steam has no gyro or lightbar control for those controllers either. When padwight finds a program reading a controller this way, it says so in a desktop notification (with a button that opens the settings window) and on screen, and the settings window names it in a banner across the top.

Rules go in `/etc/udev/rules.d/`, and apply after:

```sh
sudo udevadm control --reload && sudo udevadm trigger
```

### Immutable and atomic distros

Padwight installs entirely into your home directory plus three small files in `/etc`, so it works on Silverblue, Kinoite, Bazzite, Aeon and similar systems without layering packages. The binary and the systemd user unit go in `~/.local`; the udev rules and `dist/padwight-uinput.conf` go in `/etc/udev/rules.d/` and `/etc/modules-load.d/`, which stay writable. On NixOS, declare the rules and the `uinput` module in your configuration instead. A Flatpak isn't offered: the sandbox would block `/dev/uinput`, the process list that game detection reads, and the overlay's layer-shell surface.

## Profiles

A profile is the heart of a setup: the mappings you use while playing. Each profile maps:

- **buttons**: to a gamepad button, a key or key combo (`Left Ctrl+C`), a mouse button, the scroll wheel (one notch per press, continuous while held), or **several of these at once**.
- **back paddles**: on controllers that have them (Xbox Elite, DualSense Edge, Steam Deck), the paddles get their own section under the buttons and map like any other button. Each profile says whether it needs them, just like gyro.
- **sticks**: to a gamepad stick, the mouse pointer, the scroll wheel, a flick stick, or a button ring (its sectors can press keys, e.g. WASD or the arrows). Each has a deadzone, a response curve and an adjustable press threshold. Mouse sticks also get a **response preset** (the same names as gyro acceleration) that sets the curve, **acceleration** (speed grows while held at full push) with its ramp time, and an outer-edge boost, all fine-tunable; **Invert Y**; and under *Advanced*, a vertical speed ratio and smoothing. Scroll sticks get **Invert Y** too. Each stick's **directions also act as buttons** of their own (Left Stick Up, …): they can have any action or gesture, and can be part of combos, e.g. LB + Right Stick Right. Gamepad outputs include stick directions too, so a button (or the D-pad) can push the virtual stick, and two directions make a diagonal.
- **button rings**: a stick can instead act as a ring of 4, 8 or 12 sectors. Pointing it into a sector holds that sector's action (any action a button can have). You can adjust the angle of the first sector, how far out the stick must go, and how sticky the sector boundaries are.
- **flick stick**: push a stick out and the camera turns that way at once (spread over about 0.1 s). Rotate the stick and the camera turns by the same angle. It's horizontal only, so aim up and down with gyro or the other stick. Set *Turn size* to the mouse pixels one full turn takes in the game, then press **Test turn**, which moves the mouse one turn after 3 seconds so you can check. Only one stick per profile can be a flick stick. Advanced options: flick time, turning smoothing, a dead angle straight ahead, and optional looking up and down.
- **triggers**: to an analog gamepad trigger, or to anything a button can do (a gamepad button, key, mouse button, macro, …), pressed once the trigger is pulled past a threshold.
- **touchpad** (DualShock 4, DualSense): the mouse pointer (with tap to click, two-finger scrolling and gliding after a flick), a gamepad stick, swipes, or **menu zones**: the pad is divided into the items of one of your menus, which shows on screen while your finger is on the pad, and clicking runs the item under it. Split the pad to give each half its own job. Its click takes any action. Each profile says whether it needs a touchpad, just like gyro.

On top of that, you get some handy extras:

- **Toggle and Turbo**: wrap any action in a toggle (press once to hold it down, press again to release, e.g. for hold-to-crouch games), or in turbo (repeats 2–30 times a second while held). A macro inside a turbo plays every set number of milliseconds, never faster than the macro takes. They combine nicely: Toggle → Turbo is auto-fire you can switch on and off.
- **Combos**: buttons pressed together (e.g. LB+RB) act as their own input. Combo members wait a short window (default 60 ms) for the rest of the combo. A member whose own action is *Disabled* works as a modifier, with no time limit.
- **Gestures**: double tap, triple tap and long press, per button. The final tap of a sequence fires on press and holds, so "double-tap and hold" works. A quick tap on a button with gestures fires once the gesture is decided. Holding past the tap window presses the button's own action right away and holds it until release, unless a long press is set.
- **Zones**: extra actions that are held while a stick or trigger is within part of its travel. For example, Left Shift on a partial stick push gives you walk and run with WASD, and a half versus full trigger pull can do two different things. Zones are hidden for controllers whose triggers are simply on or off (e.g. Switch pads).

General's defaults are **Gamepad** (1:1 passthrough) and **Desktop** (left stick moves the mouse, right stick scrolls, A/B click, LB+RB = Alt+Tab). Tapping Guide (or whatever you moved that to) opens the Guide overlay. Profiles switch from Quick Settings (Guide + Start) or the app, never from the controller alone.

Want a head start? These are under "Create New Profile…" in the GUI:

- **PC action**: WASD on the left stick (Shift at full push to sprint), mouse look on the right stick, Mouse 1/2 on RT/LT, A = E (use), B = Space, X = R, Y = F, LB/RB = Q/G, L3/R3 = Ctrl/V, D-pad = 1–4, Start/Select = Esc/Tab.
- **Strategy**: mouse pointer on the left stick, arrow-key camera on the right, A/B/X = left/right/middle click, RT = left click for drag-select, L3/R3 = zoom (wheel up/down), LB/RB = held Ctrl/Shift, D-pad = control groups 1–4.
- **Retro / platformer**: arrows on the D-pad and left stick, A/B/X/Y = Z/X/C/V, for keyboard-only indie games and emulators.

## Modes

A mode is another control scheme inside a game that you switch to and stay in, such as driving, a minigame or the game's own menus. Only one mode is on at a time, so switching to one turns off the one that was on; with none on, you're in the profile's default mode. Modes belong to a setup (on its **Modes** tab), apply over whichever of its profiles is active, and override the same things a layer can.

- **Switching**: map "Switch mode…" to any button, trigger, stick direction, zone, gesture, combo or menu item. It switches to that mode, or back to the default if that mode is already on. A carousel menu of modes suits games with several.
- **Staying on**: the mode stays when a menu or keyboard opens and when the profile changes within the game, and goes back to the default when another game takes over. Buttons that switch modes keep working even in a mode that turns off its unset buttons, so there's always a way back.
- **Built in**: every setup also has the **Guide** mode (while Guide is held or its overlay is up) and **Mouse mode**, which can't be changed or removed. While one is on it stands in for the setup's mode, held layers pause, and the mode you were in comes back after. "Switch mode…" can turn on Mouse mode.
- **Seeing them**: the Guide overlay's Mappings show the mode you're in; Left and Right turn to the other modes. A mode can show its name on screen like a layer, and `{mode}` shows it in info overlays.

## Layers

A layer changes some of your mappings while you hold a button, then changes them back when you let go. For example, hold LB and the face buttons type F1–F4 and the right stick scrolls, or hold the aim button and the mouse slows down. Layers extend or fine-tune whichever mode is on; for a scheme that stays on, use a mode. Layers belong to a setup (on its **Layers** tab) and apply over whichever of its profiles and modes are on. Anything a layer doesn't set stays as it was. A layer can override buttons and stick directions, gestures, sticks, triggers (with their zones), gyro and the touchpad. It can also add combos, or switch the profile's combos off.

- **Holding one**: map "Hold layer…" to any button, trigger, stick direction, zone, gesture or combo. The layer is on while that input is held. A layer can't be toggled, and menu items can't hold layers. Layers can hold further layers.
- **Several at once**: layers stack, and the newest one wins where they disagree.
- **Switching over**: an input that's held down when a layer comes on or goes off keeps doing what it started until released. Sticks, triggers and gyro switch right away. Held layers end when the profile changes or an on-screen menu or keyboard opens.
- **Showing it**: each layer can show its name on screen while it's on (with its own position, size and colors), or show one of the setup's info overlays (e.g. a cheat sheet), or nothing at all for quick ones like "hold Y to lean". The live controller drawing labels inputs with the layers that are on, as they act.
- **Editing**: the Layers tab uses the profile editor, shown over one of the setup's profiles. Inputs the layer doesn't set read "Same as Gameplay: …" with an **Override** button. Overridden ones get **Back to base** to undo. Layers use their profile's timings. "Copy from another setup…" copies a layer along with the macros, menus and info overlays it uses, which is handy for sequels.

## Macros

A setup's Macros tab holds named input sequences, each in its own collapsible card ("+ New macro" adds one at the top). Each step is one of:

- **tap**: press, hold for N ms, release
- **hold down** or **release**
- **wait**: N ms
- **move stick**: sends a virtual-pad stick to a direction or custom position, held until the next stick step, and recentered when the macro ends

Steps can press a key or key combo, a mouse button, the scroll wheel or a gamepad button. "Insert motion…" adds fighting-game motions one frame apart: quarter circles, dragon punch and half circles, written facing right.

Map a macro with the "Macro…" action, and pick how it plays:

- **Play once**: each press plays it through to the end, even if you let go early.
- **Repeat while held**: loops until you let go, then stops at once.

Each pass of a macro releases anything it still holds when it ends. Wrap a repeating macro in a Toggle to loop it hands-free. Renaming a macro updates its mappings, and profile switches stop any macros that are running.

## Cycles

A setup's Cycles tab holds named lists of steps that one button steps through, a step per press, starting over after the last: for a game's states that rule each other out, such as game speeds or stances. Each step presses a key or key combo, a mouse button, the scroll wheel, a gamepad button, trigger or stick, a macro, or several inputs at once, and stays pressed while the button is held.

Map a cycle with the "Cycle…" action. "Step back" presses the step before instead, so two buttons can go both ways through one cycle. The first press presses the first step, so list the state the game starts in last. Every cycle goes back to its start when the game starts. The strategy and life-sim packs with a speed menu step through the speeds on a tap of that button, and open the menu on a long press.

## Mouse and keyboard from the couch

A game can work fine with a controller and still trip you up. Its launcher may need a mouse to pick settings or a server, or its name entry may assume you're at a desk with a keyboard. For those moments, tap Guide to open its overlay: the notes the game's author wrote, a menu, and what each button does. The menu has the on-screen keyboard and numpad, media controls, Quick Settings and, while Steam runs, the Steam overlay.

Hold Guide and press X, Y, LB, RB, L3 or Select for a shortcut (keyboard, numpad, media, screenshot, recording, force quit). Hold Guide and press right stick, or press right stick with the overlay open, for mouse mode: the controller moves the pointer, clicks, scrolls and sends Enter, Tab, Escape and modifier keys.

The full list is in the manual's [Guide button](docs/manual/guide-button.md) page. The shortcuts are the same in every game, and can be moved on App settings' **Controller shortcuts** tab: what opens the overlay (Guide unless you move it, to Start + Select on a controller without a Guide button, say), each Guide shortcut and each of mouse mode's keys can sit on a button, a double tap, triple tap or long press, or a combo, or be switched off. These are app-wide and can't be set per game. The buttons that choose and go back in the menus and panels (A and B) can be moved there too. The tab warns when two share an input, when nothing opens the overlay, and (with a ⚠ in the setup list) when a profile already uses the input the overlay opens with. Guide itself can't be given to anything else. Each profile's **Guide** tab sets the overlay's notes and how its mapping list reads.

## On-screen overlay

The "On-screen keyboard" action (also a button on the Overview tab, and `padwight overlay-toggle`) opens a keyboard over everything, including fullscreen games. It's a Wayland layer-shell surface that never takes keyboard focus, so your keys still go to the window underneath. While it's open, the controller drives it:

| Control | Action |
|---|---|
| D-pad / left stick | move between keys (repeats while held) |
| A | press the selected key (holding A holds the key) |
| Shift / Ctrl / Alt / AltGr / Super | latch for the next key |
| X / Y / Start | Backspace / Space / Enter |
| LB / RB | move the text cursor left / right |
| hold LT | hold Shift |
| hold B, or press Guide | close |

Guide + X, or X with the Guide overlay open, opens the keyboard. It needs a compositor with layer-shell (KDE Plasma, Sway, Hyprland, …).

The "On-screen numpad" action (or `padwight numpad-toggle`) opens a smaller pad with the digits 0–9, a dot, a comma and a minus, for codes and number fields. Move with the D-pad or left stick, press A for a number, X for backspace, Start for Enter, and hold B to close. It types the top-row number keys, so it works whatever the Num Lock state. Both pads have their own position, size and colors on the App settings page, and opening one closes the other.

## Menus

A setup's Menus tab holds on-screen **action menus**. Open one from any button, gesture, combo, trigger or stick direction with the "Open menu…" action (or `padwight menu <name>`). A menu stays on screen only while its input is held, and letting go closes it, except a list with a close button, which stays up until you press the input again or its close button. Choosing an item taps its action like a button press, so items can be keys, macros, toggles or the keyboard, and closes the menu, unless the menu is set to stay open for more picks.

Pick the style that suits the job:

- **Radial**: aim a stick at an item. Whatever you're aiming at when you let go is chosen.
- **Directional**: four slots on the D-pad or face buttons.
- **List**: move with the D-pad or left stick, A chooses.
- **Button menu**: a list whose items can also be chosen directly with their own button.
- **Carousel**: cycle with the bumpers, triggers, D-pad or a stick, A chooses.
- **Grid**: a list laid out in up to 6 columns and 6 rows. Move in all four directions with the D-pad or left stick, A chooses.

Under a button, long press or combo that opens a directional or button menu, "+ Add menu combos" adds a combo for each item: the opening input plus the item's button (RB opens the menu, so RB + D-pad Up fires its Up item without the menu showing). They are ordinary combos, and the option is off while any of them exists.

An item can open another menu of the same kind (radial menus can't open menus). The new menu is a child of the first, so letting go closes both. While a menu is open, the controller drives it, and anything the mappings were holding is released. A quick tap can't hold a menu up, so a menu opened by a tap (e.g. on a double tap that isn't the button's last gesture) won't show. Use a list with a close button, which stays up after the tap. A menu can't be put in a Toggle. Menus opened from the command line or an analog zone can't be held, so they stay until you choose an item or press B (Select for face-button menus). The overlay window stays running invisibly between uses, so menus appear instantly.

Each menu is a collapsible card on the Menus tab, with a live preview. "Add a menu" adds another at the top. Under **Appearance**, each menu (and the keyboard) gets its own screen position (a 3×3 grid of corners, edges and center, so it fits any monitor size or aspect ratio), a size (50–200%), a width and height as percentages of the screen (or fitted to the contents), an x and y offset, a max width and max height that stop it stretching off screen, and a color and opacity for the background, items and selected item. Text switches between light and dark to stay readable. Optional decorations give a game's overlays a look of their own: a panel outline, shadow and gradient, item outlines, a glow on the selected item and a text color. A Performance card on the App settings page turns decorations, motion and sounds off everywhere.

## In-game menu

Hold Guide and press Start (the Quick Settings shortcut, which can be moved on App settings like the other Guide shortcuts) to open a menu over the game in front. **Quick Settings** changes a few handy values, such as stick and gyro sensitivity, Invert Y and the profile. **Edit Controls** changes almost anything the settings window does for that setup: buttons, sticks, triggers, gyro, touchpad, combos, modes, layers, macros, menus, info and log overlays, the Guide overlay, looks, sounds and window rules, with names typed on the on-screen keyboard and live pictures of sticks, gyro and touchpad as you use them. **Switch profile** on the Guide menu switches profiles or makes a new one. Changes apply right away and are saved when you choose Save. With no window in front, Edit Controls is refused. When a window is in front but no setup matches it, a change makes a new setup for that window.

The wiki has the details: [the in-game menu](https://github.com/marcinkoza0922/padwight/wiki/In-Game-Menu) and [editing controls in game](https://github.com/marcinkoza0922/padwight/wiki/Editing-Controls-in-Game).

## Info overlays

Info overlays put text on screen without taking over the controller, mostly to show a game's controls. Each one is a grid of cells (rows of cells that line up in columns), with its own position, size and colors. Cells can hold tokens:

- **Button glyphs**, drawn the way the controller in use labels them. `{south}` is A on Xbox, ✕ on PlayStation and B on Nintendo. Also `{east}` `{west}` `{north}` `{lb}` `{rb}` `{lt}` `{rt}` `{select}` `{start}` `{guide}` (whatever opens the Guide overlay, which is the Guide button unless you moved it) `{confirm}` and `{back}` (the buttons that choose and go back in menus, A and B unless moved) `{ls}` `{rs}` `{l3}` `{r3}` `{dpad}` `{up}` `{down}` `{left}` `{right}`. Controllers that can't be recognized get the "Fallback glyphs" kind chosen on the App settings page.
- **Output glyphs**, for what a mapping outputs: `{pad:south}` is drawn for the controller the pad presents itself as (see the Details tab's *Controllers this setup supports*), `{keyboard:a}` or `{keyboard:leftshift}` for a key, and `{mouse:leftclick}` for a mouse button. The Mappings overlay uses these by default, so a key output is never mistaken for a button.
- **Live values**, updated every second: `{time}` `{time12}` `{date}` `{profile}` `{layer}` (the layers on, e.g. "Hotkeys + Build"), `{mode}` (the mode on, or "Default"), `{app}` (the focused program's executable), `{title}`, `{pid}`, `{cpu}`, `{ram}`, `{gpu}` (AMD only), `{wifi}` (signal strength, with its icon), `{system_battery}` (a laptop's or handheld's charge), and `{controller_battery}` (the controller in use's charge, where it reports one; `~` when it's estimated from a coarse level).
- **Icons**: `{pc}` (a monitor, or a laptop or handheld shape) and `{controller}` (the controller in use, drawn for its kind), meant to sit beside a battery. Add `:icon` to `{system_battery}` or `{controller_battery}` for a gauge instead of the number, and use `{controller:name}` for the controller's name.

An info overlay can appear in three ways: always while its setup is active, for a few seconds when the game starts (the first time it's focused after launching, e.g. a "config loaded" note), or while an input is held via the "Show info overlay…" action (wrap it in Toggle to keep it up). An always-shown overlay can't also be mapped to an action. For one that appears only sometimes, use an action, typically a Toggle. An overlay shown by an action can **linger** for a few seconds after it's let go or toggled off, and timed ones fade out at the end.

For an overlay that should stay up until you dismiss it, map "Toggle → Show info overlay…" to a button and tick **On when the game starts**. It shows at launch and goes away when you press the button. Any Toggle can start on that way (an overlay, a held key, …). Overlays at the same screen position stack.

## Gyro

Controllers with motion sensors (DualShock 4, DualSense, Switch Pro, Joy-Cons) can use their gyro in each profile:

- **Mouse**: gyro aiming, in pixels per degree turned.
- **Gamepad stick**: rotation speed deflects a virtual-pad stick, added to the physical stick. An anti-deadzone gets past the game's own stick deadzone.
- **Steering**: tilt the controller like a wheel to move a stick.

Horizontal aim can come from yaw (turning), roll (tilting) or both, and either axis can be inverted. Gyro is enabled per profile, and either active from the start or waiting for a button. A profile with gyro disabled can't hold gyro actions. Switching it is an action you assign to any button, gesture, combo or menu item: **Gyro active / inactive**, **Gyro active while held** (e.g. LT to aim down sights), **Gyro inactive while held** (for repositioning the controller) and **Recenter gyro**, which sets the current tilt as straight for steering. Slow movement below a jitter threshold is scaled down to hide drift. "Calibrate gyro" in the controller list measures the drift while the controller sits still.

The PC action template turns on gyro mouse aiming while LT is held (LT is Gyro active while held beside the right click). StarCraft's Gameplay profile has a "+ Gyro" twin that moves the cursor by tilting, as an extra to the left stick. The library's shooters (Deus Ex, F.E.A.R., Max Payne 1 and 2, Thief Gold, Thief II and System Shock 2) each ship three profiles: one without gyro, one with gyro as an extra, and a flick stick one that needs gyro. Their rules start on the plain one. If you pick another when the Guide offer appears, that one becomes the setup's.

## Automatic profile switching

Each setup's rules (on its Details tab) switch to one of its profiles when the game's window gets focus. When the game loses focus, the default profile takes over. You set that on the App settings page: General › Gamepad unless you change it, or "keep current profile". Each switch, and each game launch, shows a short toast at the top of the screen naming the controller profile now active and its setup. Switching back to the default when a game loses focus doesn't toast.

Rules are checked setup by setup, in order, and you can switch a rule off without deleting it. A rule matches on:

- **Executable**: the program's file name. For Wine/Proton games it's the Windows `.exe` (e.g. `eldenring.exe`), not the Wine loader.
- **Steam App ID**: taken from the environment Steam sets, or from Proton's `steam_app_<id>` window class.
- **Window class**.

A setup's Details tab lists recently focused windows, each with a one-click "+ Rule". Switching with Guide or the GUI stays in effect until focus changes again. Focusing the settings window never switches profiles.

Focus tracking depends on your desktop. The daemon picks the right method by itself and logs which one it uses (`padwight status` shows it):

- **KDE Plasma** (Wayland or X11): a small KWin script.
- **Sway**: the compositor's IPC socket (`$SWAYSOCK`).
- **Hyprland**: its event and command sockets.
- **wlroots compositors** (labwc, Wayfire, river, and others that offer `wlr-foreign-toplevel-management`): the protocol's window list. It doesn't say which process owns a window, so these match windows by class only, and executable-name rules don't match.
- **COSMIC**: its toplevel info protocol (`cosmic-toplevel-info`); class rules only, like wlroots. The overlays are ordinary layer-shell windows there.
- **GNOME** (Mutter): a small GNOME Shell extension, which the daemon installs and enables itself. A log out and back in may be needed the first time.
- **Cinnamon** (Muffin, Wayland or X11): the same, as a Cinnamon extension, enabled through the `enabled-extensions` setting.

GNOME and Cinnamon can't show the on-screen keyboard, menus or overlays as windows, because Mutter and Muffin have no layer-shell; the same extension shows them instead (experimental). Focus tracking works, through the extension. On other desktops, rules apply while a matching process is running.

## GUI

- **Overview**: a live controller drawing labeled with the active profile's mappings, *What's happening* (the latest presses as the controller labels them, the buttons padwight exported for them, and what each press did, plus each profile switch and the rule or action behind it), and the controller list (Manage, Test rumble, Calibrate gyro).
- **Settings**: automatic switching and its default profile, the on-screen keyboard and numpad, fallback glyphs, and color-blind mode for menu colors.
- **A setup's Profiles tab**: the profile you're editing, with its own labeled drawing and sub-tabs for Buttons, Sticks & triggers, Combos and Gyro. Mappings collapse to one-line summaries ("A ▸ Left click"). Click a name to edit it. **Find by pressing** jumps to whatever you press or push on the controller.
- **A setup's Modes tab**: the setup's modes, edited like a profile (see Modes).
- **A setup's Layers tab**: the setup's layers, edited like a profile (see Layers).
- **A setup's Macros tab**: a step editor with exact millisecond fields.
- **A setup's Menus, Info overlays and Log overlays tabs**: the setup's on-screen menus, text overlays, and lists of recent presses.

Problems that would block saving (unknown keys, missing macros, incomplete combos) are flagged on the row and with ⚠ on its tab. Section explanations sit behind ⓘ tooltips, and keys can be picked from an on-screen keyboard.

## CLI

`padwight status | enable | disable | profile <name> | next-profile | reload | overlay-toggle | numpad-toggle | menu <name> | close-overlays | daemon`

`profile <name>` looks in the active setup first, then General, then the first setup with a profile of that name. `menu <name>` opens one of the active setup's menus. `close-overlays` closes every overlay and menu: bind it to a key in your desktop's shortcut settings for a keyboard way out.

When `padwight daemon` runs in a terminal, it keeps a live status line showing each controller's input and what's being output.

Opening the GUI starts the daemon in the background if it isn't already running. Its output goes to `~/.local/state/padwight/daemon.log`, and the daemon keeps running after the window closes. Stopping the daemon closes the window.

Outside systemd (in a terminal, or from an autostart entry), the daemon also puts an icon in the system tray. Clicking it opens the settings window, and its menu has a Remapping switch, Open settings, Close overlays, and Quit. Under systemd, the unit manages the daemon, so no icon is shown. Desktops without a tray (such as GNOME without an extension) show no icon, and the daemon works fine without one.

## Good to know

- PlayStation and Switch controllers are also readable through hidraw, which SDL, Steam and Chromium browsers use directly, so they may see the real controller as well as ours. `dist/72-padwight-hidraw.rules` (see [Permissions](#permissions)) stops that.
- Virtual uinput devices, such as Steam Input's pad and padwight's own, are skipped.
- While remapping is on, padwight keeps one virtual pad even with no controller connected, so it comes ahead of the real controllers in Steam Input. Some games, such as Slay the Spire 2, read only the first controller Steam lists.
- Pads on the `xpad` driver report X/Y by label rather than position. This is corrected automatically.
- Rumble from games is forwarded to the physical controller when it supports force feedback. The end-to-end check needs `/dev/uinput`, so it's opt-in: `cargo test -- --ignored rumble`.

## License

GPL-3.0. See [LICENSE](LICENSE).
