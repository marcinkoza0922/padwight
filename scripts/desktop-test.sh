#!/usr/bin/env bash
# Checks focus tracking and profile switching inside a real desktop session on a test machine.
# It syncs the checkout, builds the release binary, starts the daemon in the session, opens a
# terminal, and checks that the daemon reports the right tracker and window, switches to a rule's
# profile while the window has focus, and falls back to the default profile after it closes.
#
# Only ever point this at a throwaway VM. The daemon grabs gamepads and creates virtual input
# devices, which would take over your own desktop.
#
# Usage: DESKTOP_TEST_HOST=dev@127.0.0.1 DESKTOP_TEST_SSH_OPTS="-p 2457 -i vms/id_ed25519" \
#        scripts/desktop-test.sh <gnome|cinnamon|budgie|cosmic|kde|sway|hyprland|niri|labwc|wayfire>
#   DESKTOP_TEST_QMP=<qmp socket>  optional: sends Escape through QEMU first, to close GNOME's
#                                  Overview after login (VMs started by vms/vm.sh have one).
#
# It also covers the focus checks of qa-plan.md: switching to a window no rule matches and back
# (FOC-1, FOC-2), restarting the daemon with a ruled window in front and switching windows (FOC-6,
# FOC-7), an empty workspace on Sway (FOC-8), the Cinnamon extension installing itself on a fresh
# start (FOC-10), and on Budgie the wlroots tracker with no GNOME extension and no overlay errors
# (FOC-3), and on KDE, GNOME and Hyprland a native Wayland and an Xwayland window with class and
# executable rules and Force Quit (FOC-11). A step whose other application isn't installed in the VM prints SKIP, not PASS.
#
# Re-run it after changing how the daemon talks to the system, and when a desktop, the kernel or
# the input stack gets a new release. Record the versions it prints with the result.
set -euo pipefail

desktop=${1:?usage: scripts/desktop-test.sh gnome|cinnamon|budgie|cosmic|kde|sway|hyprland|niri|labwc|wayfire}
case $desktop in
gnome | cinnamon | budgie | cosmic | kde | sway | hyprland | niri | labwc | wayfire) ;;
*)
    echo "unknown desktop: $desktop" >&2
    exit 2
    ;;
esac
host=${DESKTOP_TEST_HOST:?set DESKTOP_TEST_HOST, for example dev@127.0.0.1}
read -ra ssh_opts <<<"${DESKTOP_TEST_SSH_OPTS:-}"
root=$(cd "$(dirname "$0")/.." && pwd)
profile=$(mktemp)
trap 'rm -f "$profile"' EXIT

remote() { ssh "${ssh_opts[@]}" "$host" "$@"; }

# The rule's profile is the default Gamepad profile under another name, so it maps the same buttons.
python3 - "$root/tests/fixtures/default-config.toml" >"$profile" <<'PY'
import sys
lines, seen, out = open(sys.argv[1]).read().splitlines(), False, []
for line in lines:
    if line.startswith("[[general.profiles]]"):
        if seen:
            break
        seen = True
        continue
    if seen and line.startswith("[") and not line.startswith("[general.profiles."):
        break
    if seen:
        out.append(line.replace("[general.profiles.", "["))
text = "\n".join(out).replace('name = "Gamepad"', 'name = "Console Pad"', 1)
south = '[buttons.South]\ngamepad = "South"'
assert south in text, "the default Gamepad profile maps South to a gamepad button"
print(text.replace(south, '[buttons.South]\nkeys = ["KEY_A"]', 1))
PY

echo "== syncing the checkout to $host"
rsync -az --delete --exclude target --exclude vms --exclude .git -e "ssh ${DESKTOP_TEST_SSH_OPTS:-}" "$root/" "$host:controller_app/"

echo "== building the release binary"
remote 'cd controller_app && cargo build --release'

if [ "$desktop" = gnome ] && [ -n "${DESKTOP_TEST_QMP:-}" ]; then
    echo "== closing the GNOME Overview"
    printf '%s\n' '{"execute":"qmp_capabilities"}' \
        '{"execute":"input-send-event","arguments":{"events":[{"type":"key","data":{"down":true,"key":{"type":"qcode","data":"esc"}}},{"type":"key","data":{"down":false,"key":{"type":"qcode","data":"esc"}}}]}}' |
        timeout 5 socat - "UNIX-CONNECT:$DESKTOP_TEST_QMP" >/dev/null
    sleep 2
fi

echo "== running the $desktop checks"
{
    printf 'PROFILE_B64=%s\n' "$(base64 -w0 "$profile")"
    cat <<'SCENARIO'
set -u
uid=$(id -u)
export XDG_RUNTIME_DIR=/run/user/$uid DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$uid/bus
export WAYLAND_DISPLAY=$(basename "$(ls /run/user/$uid/wayland-* | grep -v lock | head -1)")
bin=("$HOME/controller_app/target/release/padwight")
config=$HOME/.config/padwight
desktop=$1

# The window runs bash (dash has no read -n) that reads one key and records it, so a pad press shows where it went.
read_key='read -n1 key; printf %s "$key" > /tmp/desktop-test-key; sleep 120'
case $desktop in
gnome) window=kgx; class=org.gnome.Console; tracker="GNOME Shell"; launch=(kgx -- bash -c "$read_key") ;;
# The window belongs to the terminal's server (its process name is cut to 15 characters), not to the command that opened it.
cinnamon) window=gnome-terminal; closer=gnome-terminal-; class=org.gnome.Terminal; tracker=Cinnamon; launch=(gnome-terminal -- bash -c "$read_key") ;;
# Budgie 10.10 is labwc underneath, so the generic wlroots tracker serves it.
budgie) window=gnome-terminal; closer=gnome-terminal-; class=org.gnome.Terminal; tracker=wlroots; launch=(gnome-terminal -- bash -c "$read_key") ;;
cosmic) window=cosmic-term; class=com.system76.CosmicTerm; tracker=COSMIC; launch=(cosmic-term -e bash -c "$read_key") ;;
kde) window=konsole; class=org.kde.konsole; tracker=KWin; launch=(konsole -e bash -c "$read_key") ;;
sway) window=foot; class=foot; tracker=Sway; launch=(foot bash -c "$read_key") ;;
hyprland) window=kitty; class=kitty; tracker=Hyprland; launch=(kitty bash -c "$read_key") ;;
niri) window=foot; class=foot; tracker=niri; launch=(foot bash -c "$read_key") ;;
labwc) window=foot; class=foot; tracker=wlroots; launch=(foot bash -c "$read_key") ;;
wayfire) window=foot; class=foot; tracker=wlroots; launch=(foot bash -c "$read_key") ;;
esac

# A second window that no rule matches, and the program name that closes it. The first installed
# candidate is used. On the wlroots desktops the terminal itself serves, under another app id.
case $desktop in
gnome) other_candidates=("nautilus" "gnome-text-editor" "gnome-calculator") ;;
cinnamon) other_candidates=("nemo" "xed" "gnome-calculator") ;;
budgie) other_candidates=("nautilus" "nemo" "gnome-calculator" "gnome-text-editor") ;;
cosmic) other_candidates=("cosmic-files" "cosmic-edit") ;;
kde) other_candidates=("dolphin" "kate" "kwrite") ;;
sway | niri | labwc | wayfire) other_candidates=("foot --app-id padwight-other") ;;
hyprland) other_candidates=("kitty --class padwight-other") ;;
esac
other_cmd=()
for candidate in "${other_candidates[@]}"; do
    read -ra words <<<"$candidate"
    if command -v "${words[0]}" >/dev/null; then
        other_cmd=("${words[@]}")
        break
    fi
done
# The terminals are told to stay open a while; the others stay until closed.
case ${other_cmd[0]:-} in
foot) other_cmd+=(sleep 120) ;;
kitty) other_cmd+=(sleep 120) ;;
esac

failures=0
pass() { echo "PASS $1"; }
fail() { echo "FAIL $1"; failures=$((failures + 1)); }
skip() { echo "SKIP $1"; }
# Polls until the command succeeds, for up to 15 seconds.
eventually() { for _ in $(seq 1 15); do "$@" && return 0; sleep 1; done; return 1; }
status() { "${bin[@]}" status 2>&1; }
on_profile() { status | grep -q "^profile: $1"; }
focused_is() { status | grep -q "focused: .*(class $1)"; }
log_has() { grep -q "$1" /tmp/desktop-test-daemon.log 2>/dev/null; }
stop_daemon() { pkill -x padwight 2>/dev/null || true; sleep 1; }

echo "== versions"
echo "kernel $(uname -r)"
for tool in gnome-shell cinnamon budgie-desktop cosmic-comp plasmashell sway Hyprland niri labwc wayfire; do
    command -v "$tool" >/dev/null && echo "$tool: $("$tool" --version 2>/dev/null | head -1)"
done

# Test VMs don't always have the udev rule applied (see the notes in vms/provision.sh).
if [ ! -w /dev/uinput ]; then
    sudo -n chmod 0666 /dev/uinput || { echo "/dev/uinput is not writable and sudo needs a password"; exit 1; }
fi
if [ ! -w /dev/uhid ]; then
    sudo -n chmod 0666 /dev/uhid || { echo "/dev/uhid is not writable and sudo needs a password"; exit 1; }
fi

# A simulated gamepad, made before the daemon starts so the daemon finds it on its first scan.
pad=/tmp/desktop-test-pad.fifo
pad_command() { timeout 3 sh -c "echo $1 > '$pad'" 2>/dev/null; }
stop_pad() { pad_command quit || true; sleep 1; }
stop_pad
rm -f "$pad"
(setsid nohup python3 "$HOME/controller_app/scripts/fake-pad.py" "$pad" >/tmp/desktop-test-pad.log 2>&1 </dev/null &)
if eventually grep -q "pad created" /tmp/desktop-test-pad.log; then
    pass "a simulated gamepad is created through uhid"
else
    fail "a simulated gamepad is created through uhid"
fi

# A fresh Cinnamon install has no extension of ours: remove it, so the daemon has to put it in
# place and have Cinnamon load it (FOC-10). The log out and in after that stays a manual step.
if [ "$desktop" = cinnamon ]; then
    stop_daemon
    ext=padwight-focus@io.github.marcinkoza0922
    rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/cinnamon/extensions/$ext"
    gsettings set org.cinnamon enabled-extensions "$(gsettings get org.cinnamon enabled-extensions | sed "s/, *'$ext'//; s/'$ext', *//; s/'$ext'//")"
    if ! gsettings get org.cinnamon enabled-extensions | grep -q "$ext"; then
        pass "the Cinnamon extension is removed before the daemon starts"
    else
        fail "the Cinnamon extension is removed before the daemon starts"
    fi
fi

# Budgie must run without our GNOME extension: drop one an older build left behind (FOC-3).
if [ "$desktop" = budgie ]; then
    stop_daemon
    rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}"/gnome-shell/extensions/padwight*
fi

# The daemon makes the config on its first start, and that start would clear setups written before
# it, so make the config first.
stop_daemon
if [ ! -f "$config/config.toml" ]; then
    (setsid nohup "${bin[@]}" daemon >/tmp/desktop-test-daemon.log 2>&1 </dev/null &)
    eventually log_has "daemon started" || true
    stop_daemon
fi
# The first setup whose rule matches the window wins, so remove the other setups (this VM is
# throwaway). The daemon names a setup's folder after the setup, so clear them all.
for old in "$config"/setups/*; do
    [ -d "$old" ] && rm -rf "$old"
done
setup="$config/setups/001-Desktop test"
mkdir -p "$setup/profiles"
printf 'name = "Desktop test"\n\n[[rules]]\nkind = "window_class"\nvalue = "%s"\nprofile = "Console Pad"\n' "$class" >"$setup/setup.toml"
echo "$PROFILE_B64" | base64 -d >"$setup/profiles/001-Console-Pad.toml"

: >/tmp/desktop-test-daemon.log
(setsid nohup "${bin[@]}" daemon >/tmp/desktop-test-daemon.log 2>&1 </dev/null &)

# The daemon retries the tracker every 10 seconds, so give it a little longer than that.
expected="focus tracking: focused window ($tracker)"
tracker_up() { for _ in $(seq 1 40); do log_has "$expected" && return 0; sleep 1; done; return 1; }
msg="daemon tracks focus through $tracker"
if tracker_up; then
    pass "$msg"
else
    fail "$msg"
    grep "focus tracking" /tmp/desktop-test-daemon.log | tail -1 >&2 || true
fi

if [ "$desktop" = gnome ]; then
    if eventually bash -c 'gdbus call --session -d org.gnome.Shell -o /org/gnome/Shell -m org.gnome.Shell.Extensions.GetExtensionInfo padwight-focus@io.github.marcinkoza0922 | grep -q "state.: <1.0>"'; then
        pass "the GNOME extension is enabled"
    else
        fail "the GNOME extension is enabled (a log out and back in may be needed after an update)"
    fi
fi

if [ "$desktop" = cinnamon ]; then
    if eventually gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus -m org.freedesktop.DBus.NameHasOwner io.github.marcinkoza0922.Padwight.Shell | grep -q true; then
        pass "the Cinnamon extension is loaded"
    else
        fail "the Cinnamon extension is loaded (a log out and back in may be needed after an update)"
    fi
fi

if [ "$desktop" = cinnamon ]; then
    if [ -f "${XDG_DATA_HOME:-$HOME/.local/share}/cinnamon/extensions/$ext/metadata.json" ] && gsettings get org.cinnamon enabled-extensions | grep -q "$ext"; then
        pass "the daemon installed and enabled its Cinnamon extension on a fresh start"
    else
        fail "the daemon installed and enabled its Cinnamon extension on a fresh start"
    fi
fi

# Budgie 10.10 is labwc underneath: the wlroots tracker serves it, not GNOME's extension (FOC-3).
if [ "$desktop" = budgie ]; then
    if ls "${XDG_DATA_HOME:-$HOME/.local/share}"/gnome-shell/extensions/padwight* >/dev/null 2>&1; then
        fail "no GNOME extension is installed on Budgie"
    else
        pass "no GNOME extension is installed on Budgie"
    fi
fi

(setsid nohup "${launch[@]}" >/tmp/desktop-test-window.log 2>&1 </dev/null &)

if eventually focused_is "$class"; then
    pass "a focused $window window is reported"
else
    fail "a focused $window window is reported (is the overview or a dialog holding focus?)"
fi

if eventually on_profile "Console Pad"; then
    pass "the rule switches to Console Pad while $window has focus"
else
    fail "the rule switches to Console Pad while $window has focus"
fi

# A press is dropped until the daemon has the pad, so press again until the key arrives.
pressed_key() { [ "$(cat /tmp/desktop-test-key 2>/dev/null)" = a ]; }
press_until_key() { for _ in $(seq 1 8); do pad_command press; sleep 2; pressed_key && return 0; done; return 1; }
if press_until_key; then
    pass "a pad press reaches the focused $window window as the mapped key"
else
    fail "a pad press reaches the focused $window window as the mapped key"
fi

# Switching to a window no rule matches and back (FOC-1, FOC-2): the default profile outside the
# ruled window, its own profile in it. Closing the other window hands focus back to the first.
other_proc=${other_cmd[0]:-}
open_other() { (setsid nohup "${other_cmd[@]}" >/tmp/desktop-test-other.log 2>&1 </dev/null &); }
# The terminals share their program name with the ruled window, so those are found by their app id.
close_other() {
    case "${other_cmd[*]}" in
    *padwight-other*) pkill -f padwight-other 2>/dev/null || true ;;
    *) pkill -x "${other_proc:0:15}" 2>/dev/null || true ;;
    esac
}
if [ ${#other_cmd[@]} -eq 0 ]; then
    skip "switching to another window and back (none of ${other_candidates[*]} is installed)"
else
    open_other
    if eventually on_profile "Gamepad"; then
        pass "the default profile comes back in a window no rule matches (${other_cmd[0]})"
    else
        fail "the default profile comes back in a window no rule matches (${other_cmd[0]})"
    fi
    close_other
    if eventually on_profile "Console Pad"; then
        pass "the rule's profile returns when focus comes back to $window"
    else
        fail "the rule's profile returns when focus comes back to $window"
    fi
fi

# A restart with the ruled window in front picks its profile up at once, and switching windows
# afterwards never reports an empty desktop (FOC-6 on labwc and Wayfire, FOC-7 on GNOME).
stop_daemon
(setsid nohup "${bin[@]}" daemon >>/tmp/desktop-test-daemon.log 2>&1 </dev/null &)
at_once() { for _ in $(seq 1 15); do on_profile "Console Pad" && return 0; sleep 1; done; return 1; }
if at_once; then
    pass "a restarted daemon picks the profile of the window in front"
else
    fail "a restarted daemon picks the profile of the window in front"
fi
if [ ${#other_cmd[@]} -gt 0 ]; then
    empty_before=$(grep -c "no window has focus" /tmp/desktop-test-daemon.log || true)
    open_other
    eventually on_profile "Gamepad" || true
    close_other
    eventually on_profile "Console Pad" || true
    if [ "$(grep -c "no window has focus" /tmp/desktop-test-daemon.log || true)" = "$empty_before" ]; then
        pass "switching windows never reports that no window has focus"
    else
        fail "switching windows never reports that no window has focus"
    fi
fi

# An empty workspace (FOC-8): Sway has no window there, so the default profile applies, and the
# game's returns with the workspace.
if [ "$desktop" = sway ]; then
    export SWAYSOCK=$(ls /run/user/"$uid"/sway-ipc.*.sock 2>/dev/null | head -1)
    swaymsg workspace number 9 >/dev/null
    if eventually on_profile "Gamepad"; then
        pass "the default profile applies on an empty workspace"
    else
        fail "the default profile applies on an empty workspace"
    fi
    swaymsg workspace back_and_forth >/dev/null
    if eventually on_profile "Console Pad"; then
        pass "the rule's profile returns with the workspace"
    else
        fail "the rule's profile returns with the workspace"
    fi
fi

# The on-screen keyboard is a layer-shell surface; GNOME has none, so there the overlay process
# hands its frames to the Shell extension (this checks the process, not what the extension shows).
overlay_running() { pgrep -f 'padwight overlay$' >/dev/null; }
"${bin[@]}" overlay-toggle >/dev/null 2>&1 || true
if eventually overlay_running; then
    sleep 3
    if overlay_running; then
        pass "the on-screen overlay opens and stays up"
    else
        fail "the on-screen overlay opens and stays up"
    fi
else
    fail "the on-screen overlay opens and stays up"
fi
"${bin[@]}" overlay-toggle >/dev/null 2>&1 || true
# The overlay process stays running and goes back to idle, so the daemon's log is what says it closed.
if eventually log_has "on-screen keyboard closed"; then
    pass "the on-screen overlay closes again"
else
    fail "the on-screen overlay closes again"
fi

pkill -x "${closer:-$window}" 2>/dev/null || true
if eventually on_profile "Gamepad"; then
    pass "the profile falls back to Gamepad after $window closes"
else
    fail "the profile falls back to Gamepad after $window closes"
fi

# A native Wayland window and an Xwayland one (FOC-11): a class rule and an executable rule each
# match both, and Force Quit ends the one in front, not the other. Only desktops where the same
# terminal can run either way: GTK and Qt pick the backend from the environment, kitty from a flag.
exe_name=${exe_name:-$window}
wayland_launch=() x11_launch=()
# The session starts Xwayland on demand and tells only its own children where it is.
xwayland=$(pgrep -a Xwayland | head -1)
xenv=(env "DISPLAY=$(grep -o ' :[0-9]*' <<<"$xwayland" | head -1 | tr -d ' ')" "XAUTHORITY=$(sed -n 's/.*-auth \([^ ]*\).*/\1/p' <<<"$xwayland")")
case $desktop in
gnome) wayland_launch=(env GDK_BACKEND=wayland "${launch[@]}"); x11_launch=(dbus-run-session -- "${xenv[@]}" GDK_BACKEND=x11 "${launch[@]}") ;;  # own session bus: kgx is single-instance and would reuse the Wayland process
kde) wayland_launch=(env QT_QPA_PLATFORM=wayland konsole --separate -e bash -c "$read_key"); x11_launch=("${xenv[@]}" QT_QPA_PLATFORM=xcb konsole --separate -e bash -c "$read_key") ;;
hyprland) wayland_launch=(kitty --override linux_display_server=wayland bash -c "$read_key"); x11_launch=("${xenv[@]}" kitty --override linux_display_server=x11 bash -c "$read_key") ;;
esac
if [ ${#x11_launch[@]} -eq 0 ] || ! command -v Xwayland >/dev/null; then
    skip "native Wayland and Xwayland windows ($desktop has no way to run $window both ways here, or no Xwayland)"
else
    focused_class() { status | sed -n 's/^focused: .*(class \(.*\))$/\1/p'; }
    focused_exe() { status | sed -n 's/^focused: \(.*\) (class .*$/\1/p'; }
    in_front() { [ "$(focused_exe)" = "$exe_name" ] && [ -n "$(focused_class)" ]; }
    write_rule() { printf 'name = "Desktop test"\n\n[[rules]]\nkind = "%s"\nvalue = "%s"\nprofile = "%s"\n' "$1" "$2" "$3" >"$setup/setup.toml"; }
    restart_daemon() { stop_daemon; (setsid nohup "${bin[@]}" daemon >>/tmp/desktop-test-daemon.log 2>&1 </dev/null &); sleep 4; }
    close_windows() { pkill -x "${exe_name:0:15}" 2>/dev/null || true; sleep 2; }
    # The profile that makes South the Force Quit hold: the Console Pad one with South changed.
    quit_profile="$setup/profiles/002-Quit-Pad.toml"
    sed -e '/^\[buttons.South\]/,/^$/d' -e 's/^name = "Console Pad"/name = "Quit Pad"/' "$setup/profiles/001-Console-Pad.toml" >"$quit_profile"
    printf '\n[buttons]\nSouth = "force_quit"\n' >>"$quit_profile"

    declare -A window_class
    for variant in wayland x11; do
        close_windows
        if [ $variant = wayland ]; then (setsid nohup "${wayland_launch[@]}" >/dev/null 2>&1 </dev/null &); else (setsid nohup "${x11_launch[@]}" >/dev/null 2>&1 </dev/null &); fi
        if eventually in_front; then
            window_class[$variant]=$(focused_class)
            pass "a $variant $window window is in front, class ${window_class[$variant]}"
        else
            fail "a $variant $window window is in front"
            continue
        fi
        write_rule window_class "${window_class[$variant]}" "Console Pad"
        restart_daemon
        if eventually on_profile "Console Pad"; then
            pass "a class rule matches the $variant window"
        else
            fail "a class rule matches the $variant window"
        fi
        write_rule executable "$exe_name" "Console Pad"
        restart_daemon
        if eventually on_profile "Console Pad"; then
            pass "an executable rule matches the $variant window"
        else
            fail "an executable rule matches the $variant window"
        fi
    done

    # Both windows open, the Xwayland one in front: Force Quit ends it and leaves the other.
    close_windows
    (setsid nohup "${wayland_launch[@]}" >/dev/null 2>&1 </dev/null &)
    eventually in_front || true
    wayland_pid=$(pgrep -n -x "${exe_name:0:15}" || true)
    sleep 2
    (setsid nohup "${x11_launch[@]}" >/dev/null 2>&1 </dev/null &)
    for _ in $(seq 1 15); do
        x11_pid=$(pgrep -n -x "${exe_name:0:15}" || true)
        [ -n "$x11_pid" ] && [ "$x11_pid" != "$wayland_pid" ] && break
        sleep 1
    done
    sleep 3
    write_rule executable "$exe_name" "Quit Pad"
    restart_daemon
    eventually on_profile "Quit Pad" || true
    pad_command "hold 4"
    sleep 6
    if ! kill -0 "$x11_pid" 2>/dev/null && kill -0 "$wayland_pid" 2>/dev/null; then
        pass "Force Quit ends the Xwayland window in front and leaves the Wayland one"
    else
        fail "Force Quit ends the Xwayland window in front and leaves the Wayland one (Wayland pid $wayland_pid, Xwayland pid $x11_pid)"
    fi
    # Focus goes to the Wayland window; the second hold ends that one.
    eventually in_front || true
    eventually on_profile "Quit Pad" || true
    pad_command "hold 4"
    sleep 6
    if ! kill -0 "$wayland_pid" 2>/dev/null; then
        pass "Force Quit then ends the Wayland window"
    else
        fail "Force Quit then ends the Wayland window (pid $wayland_pid)"
    fi
    close_windows
fi

if grep -q "overlay: " /tmp/desktop-test-daemon.log; then
    fail "no overlay errors in the daemon's log"
    grep "overlay: " /tmp/desktop-test-daemon.log | head -3 >&2
else
    pass "no overlay errors in the daemon's log"
fi

if grep -q panicked /tmp/desktop-test-daemon.log; then
    fail "the daemon didn't panic"
else
    pass "the daemon didn't panic"
fi

stop_daemon
stop_pad
# A copy stays in /tmp for scripts/cinnamon-relogin.sh, which checks FOC-10 after a log out and in.
rm -rf /tmp/desktop-test-setup-saved
cp -r "$setup" /tmp/desktop-test-setup-saved
rm -rf "$setup"
echo "== $failures failure(s)"
exit "$failures"
SCENARIO
} | remote "bash -s -- $desktop"
