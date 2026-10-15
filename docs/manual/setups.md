# Controller setups and profiles

A **controller setup** holds everything Padwight keeps for one game: its profiles, modes, layers, macros, menus and overlays, and the rules that pick it when that game's window has focus. In the sidebar, a setup is listed under **Setups**. **General** is the setup for the desktop and plain controller use. It has no rules, so it's used when no other setup matches.

## Profiles

A profile is one complete set of mappings inside a setup, for example one for a whole game or one for a different way of playing it. Profiles don't mix with each other. If a game needs a different control scheme for a context inside play, such as driving, use a [mode](modes.md) instead. On the **Profiles** tab, **Duplicate** copies the profile you're on, and **New from template** starts from a ready-made one.

## Switching profiles

- **Automatically:** a rule in the setup picks its profile when the game's window has focus.
- **By hand:** choose a profile in the top bar's *Active profile* list, or press Guide + Start and pick one in Quick Settings.
- **Locked:** the padlock beside the *Active profile* list keeps the active profile on, so rules and the Default Profile don't switch it. Switching to another profile by hand unlocks it. The lock isn't kept when Padwight restarts.
- **When no setup matches:** the *Default Profile* setting under Per-setup profiles on the App settings page picks the profile.

## Rules

A rule matches a window by its program name, its Steam App ID, or its window class. Wine and Proton games match by their `.exe` name.

Some games share a program name, such as `game.exe` or `gpp.exe`. To tell them apart, put the folder in an **Executable** rule: `Prince of Persia - The Sands of Time/gpp.exe` matches only a `gpp.exe` in a folder of that name. Add more folders if one isn't enough (`Ubisoft/Sands of Time/gpp.exe`). A rule that names a folder wins over one that names only the program. **Recently focused windows** adds the folder for you when another setup already uses the program's name.

To make a rule quickly, focus the game and use **Recently focused windows** on the setup's Details tab. It adds a rule for the profile you're editing.

> **Tip:** automatic switching needs a desktop that reports which window has focus: KDE Plasma, GNOME, Hyprland, Sway, or a wlroots compositor such as labwc. See [Troubleshooting](troubleshooting.md).

## Reusing items

Each setup's modes, layers, macros, menus and overlays are its own, and names only need to be unique within one setup. To reuse one from another setup, click **Copy from another setup…** on its tab. The copy brings along anything the item uses.
