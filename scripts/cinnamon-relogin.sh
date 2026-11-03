#!/usr/bin/env bash
# FOC-10, the log out and in: run after `scripts/desktop-test.sh cinnamon` (it leaves the extension
# freshly installed and a copy of its setup in /tmp). Restarts the VM's display manager, which ends
# the session and logs the autologin user in again, starts the daemon, and checks that the extension
# is still enabled and loaded without any setting touched, and that switching works (FOC-2, FOC-6).
# Usage: DESKTOP_TEST_HOST=dev@127.0.0.1 DESKTOP_TEST_SSH_OPTS="-p 2548 -i vms/id_ed25519" scripts/cinnamon-relogin.sh
set -euo pipefail
host=${DESKTOP_TEST_HOST:?set DESKTOP_TEST_HOST, for example dev@127.0.0.1}
read -ra ssh_opts <<<"${DESKTOP_TEST_SSH_OPTS:-}"
remote() { ssh "${ssh_opts[@]}" "$host" "$@"; }
remote 'sudo systemctl restart gdm'
sleep 25
until remote 'loginctl list-sessions --no-legend | grep -q seat0 && pgrep -x cinnamon' >/dev/null 2>&1; do sleep 3; done
sleep 20
remote 'bash -s' <<'POST'
set -u
config=$HOME/.config/padwight; mkdir -p "$config/setups"; rm -rf "$config"/setups/*; cp -r /tmp/desktop-test-setup-saved "$config/setups/001-Desktop test"
uid=$(id -u)
export XDG_RUNTIME_DIR=/run/user/$uid DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$uid/bus
export WAYLAND_DISPLAY=$(basename "$(ls /run/user/$uid/wayland-* | grep -v lock | head -1)")
bin=$HOME/controller_app/target/release/padwight
ext=padwight-focus@io.github.marcinkoza0922
ev() { for _ in $(seq 1 25); do "$@" && return 0; sleep 1; done; return 1; }
chk() { if ev "${@:2}"; then echo "PASS $1"; else echo "FAIL $1"; fi; }
echo "session: $(loginctl list-sessions --no-legend | tr '\n' ';')"
echo "enabled: $(gsettings get org.cinnamon enabled-extensions)"
chk "extension still installed and enabled after login" bash -c "ls ~/.local/share/cinnamon/extensions/$ext/metadata.json >/dev/null && gsettings get org.cinnamon enabled-extensions | grep -q $ext"
pgrep -x padwight >/dev/null && echo "daemon survived" || echo "daemon gone with the session"
pkill -x padwight; sleep 1
: >/tmp/post-daemon.log
(setsid nohup $bin daemon >/tmp/post-daemon.log 2>&1 </dev/null &)
chk "tracker is Cinnamon" grep -q "focus tracking: focused window (Cinnamon)" /tmp/post-daemon.log
chk "extension bus name owned" bash -c 'gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus -m org.freedesktop.DBus.NameHasOwner io.github.marcinkoza0922.Padwight.Shell | grep -q true'
on() { $bin status 2>&1 | grep -q "^profile: $1"; }
foc() { $bin status 2>&1 | grep -q "focused: .*(class $1)"; }
rm -f /tmp/desktop-test-key
(setsid nohup gnome-terminal -- bash -c 'sleep 120' >/dev/null 2>&1 </dev/null &)
chk "terminal focused" foc org.gnome.Terminal
chk "FOC-2 rule profile in ruled window" on "Console Pad"
(setsid nohup nemo >/dev/null 2>&1 </dev/null &)
chk "FOC-2 default profile in other window" on Gamepad
pkill -x nemo
chk "FOC-2 rule profile returns" on "Console Pad"
pkill -x padwight; sleep 1
(setsid nohup $bin daemon >>/tmp/post-daemon.log 2>&1 </dev/null &)
chk "FOC-6 restart picks ruled profile" on "Console Pad"
n=$(grep -c "no window has focus" /tmp/post-daemon.log)
(setsid nohup nemo >/dev/null 2>&1 </dev/null &); ev on Gamepad; pkill -x nemo; ev on "Console Pad"
[ "$(grep -c 'no window has focus' /tmp/post-daemon.log)" = "$n" ] && echo "PASS FOC-6 no 'no window has focus'" || echo "FAIL FOC-6 no window has focus"
pkill -x gnome-terminal-; pkill -x padwight
POST
