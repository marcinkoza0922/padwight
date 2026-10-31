# Desktop support

Automatic profile switching needs to know which window has focus. padwight picks the right tracker for your desktop by itself, and logs which one it's using. `padwight status` shows it too.

| Desktop | Focus tracking | Automatic switching | On-screen keyboard, menus and overlays |
|---|---|---|---|
| **KDE Plasma** (Wayland or X11) | A small KWin script reports focus changes over D-Bus | Yes | Yes |
| **Sway** | The compositor's IPC socket (`$SWAYSOCK`) | Yes | Yes |
| **Hyprland** | Its event and command sockets | Yes | Yes |
| **GNOME** | GNOME Shell extension | Yes | Experimental, through the extension |
| **wlroots compositors** (labwc, Wayfire, river, …) | The `wlr-foreign-toplevel-management` protocol. Matches by window class, not executable | Yes | Yes |
| Other desktops | None | Only the fallback below | Only if the compositor supports layer-shell |

## GNOME

Focus tracking works through a small GNOME Shell extension, which padwight installs and enables itself. Mutter has no layer-shell, so the overlay can't be a window of its own: the extension shows the frames the overlay draws instead (experimental: checked on GNOME 51 in a VM: the keyboard, the Guide shortcuts and menus show over a fullscreen window and hide again, on the focused window's monitor, at fractional scales too. The extension was checked against GNOME Shell 45 to 51 headless; only 51 was run in a full session).

## Tested

The desktop checks last ran on 2026-10-09 with kernel 7.2.9. GNOME 51.0, KDE Plasma 6.7.5, Sway 1.12, Hyprland 0.56.2 and labwc 0.20.2 pass the focus, profile and input checks (the overlay is skipped on GNOME).

## Fallback

Without a focus tracker, a game's rules still apply while a process matching the rule is running. This is less precise than focus tracking: the game's profile is used whenever it's running, not only when its window is focused.

## Window identification

For each window, padwight works out the executable, the Steam App ID and the window class from `/proc`. Wine and Proton games are matched by their `.exe` name, not by the Wine loader. See [Games and profiles](Games-and-Profiles#auto-switch-rules).
