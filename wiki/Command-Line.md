# Command line

Every command except `padwight` (which opens the settings window) and `padwight daemon` talks to the running service, sends one request, and exits.

| Command | What it does |
|---|---|
| `padwight` or `padwight gui` | Open the settings window |
| `padwight daemon` | Run the service (the systemd unit runs this) |
| `padwight status` | Show the daemon's status and the controllers it has found |
| `padwight enable` / `padwight disable` | Turn remapping on or off |
| `padwight profile <name>` | Switch to a profile |
| `padwight next-profile` | Switch to the next profile |
| `padwight reload` | Re-read the config file |
| `padwight menu <name>` | Open a menu |
| `padwight overlay-toggle` | Open or close the on-screen keyboard |
| `padwight numpad-toggle` | Open or close the on-screen numpad |
| `padwight close-overlays` | Close every overlay and menu. Bind it to a key in your desktop's shortcut settings to close them from the keyboard |

## Finding profiles and menus

`padwight profile <name>` looks in the active setup first, then in General, then in the first setup that has a profile of that name.

`padwight menu <name>` opens one of the active setup's menus.

## Live status

When `padwight daemon` runs in a terminal, it shows a live status line with each controller's input and what it's outputting.

## After editing the config by hand

The daemon is the only writer of `~/.config/padwight/` (`config.toml` and the setup folders) while it runs. After editing the file yourself, run `padwight reload`.
