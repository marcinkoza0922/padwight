# Games and profiles

Everything is organized into **setups**: one controller setup per game. A setup holds its own:

- profiles,
- [modes](Modes) and [layers](Layers),
- [macros](Macros), [menus](Menus), [info overlays](Info-Overlays) and log overlays,
- and the **auto-switch rules** that activate it.

All of a setup's profiles can use all of its items.

## General

**General** is a built-in setup with no rules. It covers the desktop and plain gamepad use, and is used when no other setup matches.

Names only need to be unique within a setup. To reuse items from another setup, use **Copy from another setup…**.

## The setup page

The sidebar lists General and your setups, with a search box. Each setup's page has these tabs:

- **Profiles**: edit the profiles (buttons, sticks and triggers, combos, gyro).
- **Modes** and **Layers**: the setup's modes and layers.
- **Macros**, **Menus**, **Info overlays**, **Log overlays**.
- **Details**: name, rules, export and delete.

On every tab except Profiles and Details, **Copy from another setup…** previews another setup's (or a library setup's) items and copies the ones you pick in, to adapt.

## Auto-switch rules

![The Details tab: the game's name, its auto-switch rules, and the on-screen keyboard settings](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/details-rules.png)

A setup's **Details** tab holds its rules. When the game's window gets focus, padwight switches to one of the setup's profiles. When the window loses focus, the **default profile** (set on the App settings page) takes over. You can also keep the current profile.

A rule matches on one of:

- **Executable**: the program's file name. For Wine and Proton games this is the Windows `.exe` (for example `eldenring.exe`), not the Wine loader.
- **Steam App ID**: taken from the environment Steam sets, or from Proton's `steam_app_<id>` window class.
- **Window class**.

Rules are checked setup by setup, in order. A rule can be switched off without deleting it.

The Details tab lists recently focused windows, with a one-click **+ Rule** for each.

Each switch, and each game launch, shows a short toast naming the active profile and its setup. Switching back to the default when a game loses focus doesn't show one.

Switching with Guide or from the GUI holds until focus changes again. Focusing the settings window never switches profiles.

Automatic switching depends on the desktop. See [Desktop support](Desktop-Support).

## Profiles

Each profile is a complete mapping: buttons, sticks, triggers, combos and gyro. To make a second profile in a setup, use **Duplicate** on the Profiles tab, or start from a template under **+ New profile…**.

Profiles don't mix with each other. If a game needs a different control scheme for part of play, such as driving, use a [mode](Modes) instead.

Switch profiles, or make a new one, with **Switch profile** on the [Guide menu](Guide-Button), in [Quick Settings](In-Game-Menu#quick-settings), or in the app. No controller button switches profiles, so each profile stays a complete scheme.

## Where setups are stored

Each setup is a folder under `~/.config/padwight/setups/` (General is `general/`), with its own `setup.toml` and a subfolder of files for its profiles, modes, layers, macros, menus, overlays and logs. Copy a folder to move it to another machine.

## Setups made from the in-game menu

Changing a setting while no setup matches the window in front, from [Quick Settings](In-Game-Menu#quick-settings), makes a setup for that window. Editing controls with a window in front does the same. The setup is named after the window's program, and gets a rule for that window. Its profile starts as a copy of General's. General itself is not changed.
