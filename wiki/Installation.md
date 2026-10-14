# Installation

padwight runs on Linux. It needs a Wayland or X11 desktop for the on-screen overlays, and a desktop with focus tracking (KDE Plasma, GNOME, Hyprland, Sway, or a wlroots compositor such as labwc) for automatic profile switching. See [Desktop support](Desktop-Support).

## Install from a release

```sh
tar xzf padwight-*-linux-x86_64.tar.gz
cd padwight-*-linux-x86_64
./install.sh
```

The script installs padwight for your user (`~/.local/bin`, the menu entry and icon, and a background service that starts at login). Then it explains three small system files and asks before copying them with `sudo`: two udev rules and a list that loads the `uinput` kernel module. `./install.sh --uninstall` removes everything it installed. Your settings stay.

## Build from source

```sh
cargo install --path .
mkdir -p ~/.config/systemd/user
cp dist/padwight.service ~/.config/systemd/user/
systemctl --user enable --now padwight
padwight            # open the settings window
```

The service runs `padwight daemon`. It starts with your user session and restarts itself if it fails.

## Permissions

Run `padwight doctor`. It checks each item below, and for anything missing it prints what's wrong, why padwight needs it, and the exact commands. `padwight doctor --explain` prints the full list of what padwight needs and why. If the daemon can't start, it logs the same report, and the settings window shows it while the daemon isn't running.

| What | Why | Granted by |
| --- | --- | --- |
| Write access to `/dev/uinput` | The kernel makes virtual input devices only through it. padwight shows games a virtual gamepad and sends your keys and clicks through a virtual keyboard and mouse. | `dist/70-padwight-uinput.rules` (systemd-logind's `uaccess` tag: the user at the machine, nobody else). Most desktops already allow it. |
| The `uinput` kernel module | `/dev/uinput` doesn't exist without it. | `dist/padwight-uinput.conf` in `/etc/modules-load.d/`, or `sudo modprobe uinput` for now. |
| Read access to the controller's `/dev/input/event*` node | Its buttons and sticks come from there, and padwight takes the node exclusively so a game doesn't see both pads. | systemd's `uaccess` covers joysticks. Otherwise `sudo usermod -aG input $USER` and log in again. |
| Read access to the controller's motion sensor | Gyro comes from a separate node, classed as an accelerometer, that `uaccess` leaves to root. | `dist/70-padwight-motion.rules`, which covers the `playstation`, `sony` and `nintendo` drivers only, so a laptop's tilt sensor is untouched. Without it everything but gyro works. |

Rules go in `/etc/udev/rules.d/` and apply after `sudo udevadm control --reload && sudo udevadm trigger`.

## Gyro

PlayStation and Switch controllers keep their motion sensors on a separate device that `uaccess` doesn't cover, so gyro needs `70-padwight-motion.rules` from the table above. `padwight doctor` says whether it's installed. See [Gyro](Gyro) for how to use it.

## Controllers that come and go

Controllers can be unplugged, reconnected or swapped while padwight runs. See [Troubleshooting](Troubleshooting) if a pad isn't picked up.
