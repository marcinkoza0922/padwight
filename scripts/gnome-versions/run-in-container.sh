#!/usr/bin/env bash
# Runs inside the image: starts GNOME Shell headless with the extension enabled, then probes it.
set -u
export XDG_RUNTIME_DIR=/tmp/xdg XDG_CONFIG_HOME=$HOME/.config GSETTINGS_BACKEND=keyfile
mkdir -p "$XDG_RUNTIME_DIR" "$XDG_CONFIG_HOME/glib-2.0/settings" && chmod 700 "$XDG_RUNTIME_DIR"
uuid=padwight-focus@io.github.marcinkoza0922
mkdir -p "$HOME/.local/share/gnome-shell/extensions"
cp -r /extension "$HOME/.local/share/gnome-shell/extensions/$uuid"
printf "[org/gnome/shell]\nenabled-extensions=['%s']\nwelcome-dialog-last-shown-version='999'\n" "$uuid" >"$XDG_CONFIG_HOME/glib-2.0/settings/keyfile"
echo "gnome-shell $(gnome-shell --version)"
# GNOME Shell wants a system bus and aborts without one; an empty private bus is enough.
export DBUS_SYSTEM_BUS_ADDRESS=$(dbus-daemon --session --fork --print-address)
# Shell 45 and 47 crash on start without a reply from localed (see localed-stub.py).
python3 /home/dev/localed-stub.py "$DBUS_SYSTEM_BUS_ADDRESS" &
exec dbus-run-session -- bash -c '
    gnome-shell --headless --wayland --no-x11 --unsafe-mode --virtual-monitor 1280x800 >/tmp/shell.log 2>&1 &
    python3 /home/dev/probe.py; status=$?
    echo "--- shell log (padwight and errors)"; grep -ai "padwight\|JS ERROR\|overlay.js\|extension" /tmp/shell.log | tail -15
    kill %1 2>/dev/null; exit $status'
