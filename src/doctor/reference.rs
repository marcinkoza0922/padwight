//! The text of `padwight doctor --explain`.

/// What the daemon needs and why, for `padwight doctor --explain`.
pub const REFERENCE: &str = "\
What padwight needs from the system, and why

1. Write access to /dev/uinput.
   padwight reads your real controller and shows games a virtual gamepad, plus a virtual keyboard
   and mouse for the keys and clicks your mappings send. The kernel makes virtual input devices
   only through /dev/uinput, which needs the uinput kernel module loaded.
   Granted by: dist/70-padwight-uinput.rules (systemd-logind's `uaccess` tag gives the user at
   the machine access and nobody else), and dist/padwight-uinput.conf (loads the module at boot).
   Most desktops already do both.

2. Read access to the controller's /dev/input/eventN node.
   That is where its buttons and sticks come from. padwight also takes the node exclusively, so
   games see only the virtual gamepad and not both. systemd's own rules give the logged-in user
   access to joysticks. Without that, `sudo usermod -aG input $USER` and a new login does.

3. Read access to the controller's motion sensor, for gyro (PlayStation and Switch controllers).
   The motion sensor is a separate input node, classed as an accelerometer, which systemd's rules
   leave to root. dist/70-padwight-motion.rules tags it for the logged-in user, only for the
   playstation, sony and nintendo drivers, so a laptop's tilt sensor is untouched.
   Without it, everything but gyro works.

4. Optional: no access to PlayStation and Switch controllers' raw HID node (/dev/hidrawN).
   Steam and SDL games read those controllers there instead of through the input node, so taking
   the input node doesn't stop them: they see every press as well as the virtual gamepad.
   dist/72-padwight-hidraw.rules closes the raw node to every program (padwight doesn't use it),
   so they read the input node, which padwight holds. While padwight is off they then read it
   there too, which costs Steam's own gyro and lightbar control for those controllers.

5. A Wayland session, for the settings window, the on-screen keyboard and the overlays.
   X11 sessions (Xorg, XLibre) aren't supported; games running under Xwayland are fine.
   The overlays need a Wayland compositor with layer-shell (KDE Plasma, COSMIC, Sway, Hyprland,
   labwc and other wlroots compositors). GNOME and Cinnamon have none: their overlays are shown by
   padwight's shell extension instead (experimental).

6. Optional: wl-copy, from the wl-clipboard package, for the calculator's Copy. padwight has no
   window of its own to own the clipboard with, so it hands the text to wl-copy. Everything else
   works without it, Paste included.

7. Optional: the daemon running. `systemctl --user enable --now padwight` starts it at login.

All udev rules go in /etc/udev/rules.d/ and need root once. `padwight doctor` prints the
exact commands for whatever is missing.";
