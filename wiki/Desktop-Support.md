# Desktop support

padwight needs a Wayland session. X11 sessions (Xorg, XLibre) aren't supported, but games running under Xwayland, which includes every Proton game, work as usual.

Automatic profile switching needs to know which window has focus. padwight picks the right tracker for your desktop by itself, and logs which one it's using. `padwight status` shows it too.

| Desktop | Focus tracking | Rules that match | On-screen keyboard, menus and overlays |
|---|---|---|---|
| **KDE Plasma** | A small KWin script reports focus changes over D-Bus | All | Yes |
| **Sway** | The compositor's IPC socket (`$SWAYSOCK`) | All | Yes |
| **Hyprland** | Its event and command sockets | All | Yes |
| **niri** | Its IPC socket's event stream | All | Yes |
| **GNOME** | GNOME Shell extension | All | Experimental, through the extension |
| **Cinnamon** (Wayland) | Cinnamon extension | All | Experimental, through the extension |
| **COSMIC** | The `cosmic-toplevel-info` protocol | Window class only | Yes |
| **wlroots compositors** (labwc, Wayfire, river, …) and **Budgie** 10.10 and later, which runs on labwc | The `wlr-foreign-toplevel-management` protocol | Window class only | Yes |
| Other desktops | None | Only the fallback below | Only if the compositor supports layer-shell |

"Window class only" means the desktop doesn't say which process owns a window, so executable and Steam App ID rules don't match there. Give the game a window class rule instead (a setup's Details tab offers one for each recent window).

## GNOME and Cinnamon

Focus tracking works through a small extension, which padwight installs and enables itself. A log out and back in may be needed the first time. Mutter and Muffin have no layer-shell, so the overlay can't be a window of its own: the extension shows the frames the overlay draws instead. This is experimental. It was checked in VMs on GNOME 51 and Cinnamon 6.6: the keyboard, the Guide shortcuts and menus show over a fullscreen window and hide again, on the focused window's monitor, at fractional scales too. The GNOME extension was also checked against GNOME Shell 45 to 51 headless.

Uninstalling padwight (`padwight-uninstall`) removes the extension too.

## Tested

The desktop checks run the daemon in a VM of each desktop and check that the tracker is found, a window's rule switches the profile, the profile falls back when the window closes, and the overlay runs.

| Desktop | Version | Last passed |
|---|---|---|
| GNOME | 51.0 | 2026-10-09 |
| KDE Plasma | 6.7.5 | 2026-10-09 |
| Sway | 1.12 | 2026-10-09 |
| Hyprland | 0.56.2 | 2026-10-09 |
| labwc | 0.20.2 | 2026-10-09 |
| Cinnamon | 6.6.9 | 2026-10-30 |
| COSMIC | cosmic-comp 1.10.0 | 2026-10-31 |
| Budgie | 10.10.2 (also Ubuntu Budgie 26.04.1) | 2026-10-31 |
| niri | 26.04 | 2026-10-31 |

## Fallback

Without a focus tracker, a game's rules still apply while a process matching the rule is running. This is less precise than focus tracking: the game's profile is used whenever it's running, not only when its window is focused.

## Window identification

For each window, padwight works out the executable, the Steam App ID and the window class from `/proc`. Wine and Proton games are matched by their `.exe` name, not by the Wine loader. See [Games and profiles](Games-and-Profiles#auto-switch-rules).
