# Controller setups and profiles

A **controller setup** holds everything Padwight keeps for one game: its profiles, modes, layers, macros, menus and overlays, and the rules that pick it when the game's window has focus. Setups are listed in the sidebar. **General**, the setup for the desktop, has no rules, so it's used when no other setup matches.

## Profiles

A profile is one complete set of mappings, such as one way of playing a game. Only one is in use at a time. For a change of controls during play, such as driving, use a [mode](modes.md) instead. On the **Profiles** tab, **Duplicate** copies the profile you're on, and **+ New profile from…** starts from a ready-made one.

## Switching profiles

- **Automatically:** a rule in the setup picks its profile when the game's window has focus.
- **By hand:** pick one under *Active profile* in the top bar, or from **Switch Profile** on the Guide overlay.
- **Per controller:** with more than one controller, pick a controller in the top bar first. Each one can use its own profile.
- **Locked:** the padlock beside *Active profile* stops rules from switching away. Picking another profile by hand unlocks it, and so does restarting Padwight.
- **When no setup matches:** the *Default profile* on App settings (under Per-setup profiles) is used.

## Rules

A rule matches a window by its program name, its Steam App ID, or its window class. Wine and Proton games match by their `.exe` name.

Some games share a program name, such as `game.exe`. To tell them apart, put the folder in an **Executable** rule: `Prince of Persia - The Sands of Time/gpp.exe` matches only a `gpp.exe` in a folder of that name. Add more folders if one isn't enough (`Ubisoft/Sands of Time/gpp.exe`). A rule that names a folder wins over one that names only the program.

The quickest way to make a rule: focus the game, then use **Recently focused windows** on the setup's Details tab. It adds a rule for the profile you're editing, with the folder when another setup already uses the program's name.

> **Tip:** automatic switching needs a desktop that reports which window has focus: KDE Plasma, GNOME, Hyprland, Sway, or a wlroots compositor such as labwc. See [Troubleshooting](troubleshooting.md).

## Reusing items

Each setup's modes, layers, macros, menus and overlays are its own. To reuse one from another setup, click **Copy from another setup…** on its tab. The copy brings along anything the item uses.
