#!/bin/bash
# Runs the real `padwight doctor` binary, as an unprivileged user, on a made-up system in a throwaway
# Docker container, with each piece missing in turn, and asserts the message and the fix it prints
# (qa-plan INS-8). Also edits each installed rule and the module list by hand and runs
# `padwight doctor --files`.
#
#   scripts/doctor-test.sh            needs docker and a debug build (PADWIGHT_BIN, default target/debug/padwight)
#   DOCTOR_IMAGE=image                a Linux image with bash, setpriv and a glibc at least as new as the build's
#                                     (default padwight-gnome:45, which the desktop tests already use)
#
# The container's own /dev and /etc are its own; /sys/class/input is replaced by a tmpfs that the
# test fills with fake controllers, so nothing of the host's input devices is visible or touched. It
# runs as root inside (to make fake files and then drop to nobody) with no extra capabilities or devices;
# AppArmor is switched off for it, because the default profile forbids writing under /sys even on a tmpfs.
set -u
if [ "${1:-}" != inner ]; then
    cd "$(dirname "$0")/.."
    root=$PWD
    BIN=${PADWIGHT_BIN:-$root/target/debug/padwight}
    [ -x "$BIN" ] || { echo "build padwight first (cargo build), or set PADWIGHT_BIN" >&2; exit 2; }
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    strip -o "$tmp/padwight" "$BIN" 2>/dev/null || cp "$BIN" "$tmp/padwight"
    chmod 755 "$tmp" "$tmp/padwight"
    exec docker run --rm -u 0 --security-opt apparmor=unconfined --tmpfs /sys/class/input:rw,mode=755 \
        -v "$tmp/padwight:/opt/pw/padwight:ro" -v "$root/dist:/dist:ro" -v "$root/scripts/doctor-test.sh:/doctor-test.sh:ro" \
        "${DOCTOR_IMAGE:-padwight-gnome:45}" bash /doctor-test.sh inner
fi

# ---- inside the container -------------------------------------------------------------------

pass=0 failed=0
P=/opt/pw:/usr/bin:/bin
RULES="70-padwight-uinput.rules 70-padwight-motion.rules 72-padwight-hidraw.rules"

ok() { pass=$((pass + 1)); echo "ok: $1"; }
bad() { failed=$((failed + 1)); echo "FAIL: $1"; [ -n "${2:-}" ] && echo "$2" | sed 's/^/    | /'; }

reset() { # a clean system: no padwight files, no controllers, no /dev/uinput
    rm -f /etc/udev/rules.d/*padwight* /usr/lib/udev/rules.d/*padwight* /etc/modules-load.d/padwight-uinput.conf
    rm -rf /sys/class/input/* /dev/input /dev/uinput /opt/stub /tmp/padwight-* /tmp/nh
    mkdir -p /etc/udev/rules.d /etc/modules-load.d /dev/input /tmp/nh /opt/stub
    chmod 777 /tmp/nh
    unset EXTRA_ENV
}

install_files() { # install_files rules... : the shipped files, as install.sh puts them
    for f in "$@"; do
        case $f in
            *.conf) cp "/dist/$f" /etc/modules-load.d/ ;;
            *) cp "/dist/$f" /etc/udev/rules.d/ ;;
        esac
    done
}

uinput() { # uinput none|denied|writable
    case $1 in
        denied) mknod /dev/uinput c 10 223 && chmod 000 /dev/uinput ;;
        writable) : > /dev/uinput && chmod 666 /dev/uinput ;;
    esac
}

pad() { # pad event kind driver mode: a fake controller; kind pad|motion; mode of its /dev/input node
    local d=/sys/class/input/$1/device
    mkdir -p "$d/capabilities"
    if [ "$2" = pad ]; then
        echo "Fake $3 pad" > "$d/name"; echo "1000000000000 0 0 0 0" > "$d/capabilities/key"; echo 3 > "$d/capabilities/abs"; echo 0 > "$d/properties"
    else
        echo "Fake $3 motion" > "$d/name"; echo 0 > "$d/capabilities/key"; echo 0 > "$d/capabilities/abs"; echo 40 > "$d/properties"
    fi
    ln -s "../../drivers/$3" "/sys/class/input/$1/driver"
    : > "/dev/input/$1"; chmod "$4" "/dev/input/$1"
}

doctor() { # doctor args...: as the unprivileged user; output in $out, exit status in $rc
    out=$(setpriv --reuid=65534 --regid=65534 --clear-groups env -i PATH="$P" HOME=/tmp/nh ${EXTRA_ENV:-} padwight doctor "$@" 2>&1)
    rc=$?
}

expect() { # expect "description" needle...: every needle is in $out
    local what=$1; shift
    local n
    for n in "$@"; do
        if ! printf '%s\n' "$out" | grep -qF -- "$n"; then bad "$what (missing: $n)" "$out"; return; fi
    done
    ok "$what"
}
refute() { # refute "description" needle: it is not in $out
    if printf '%s\n' "$out" | grep -qF -- "$2"; then bad "$1 (found: $2)" "$out"; else ok "$1"; fi
}
status_is() { if [ "$rc" -eq "$2" ]; then ok "$1 (exit $2)"; else bad "$1 (exit $rc, wanted $2)" "$out"; fi; }

# 1. Nothing installed.
reset
doctor
expect "no /dev/uinput: named, with the modprobe and the module list as the fix" "✗ Virtual devices: /dev/uinput doesn't exist" "sudo modprobe uinput" "sudo tee /etc/modules-load.d/padwight-uinput.conf"
expect "no daemon: named, with how to start it" "• Daemon: not running" "padwight daemon"
expect "no controller" "• No controller is connected"
expect "no motion rule: named, with its install command" "• Gyro: 70-padwight-motion.rules isn't installed" "sudo tee /etc/udev/rules.d/70-padwight-motion.rules"
expect "no hidraw rule: named, with its install command" "• Raw controller access: 72-padwight-hidraw.rules isn't installed" "sudo tee /etc/udev/rules.d/72-padwight-hidraw.rules"
expect "no wl-copy: named" "• Clipboard: wl-copy isn't installed"
expect "no desktop session: a note" "• Desktop session: none found here"
status_is "a problem was found" 1
expect "the problem count and where to read more" "Run \`padwight doctor --explain\`" "1 problem found"

# 2. uinput present but this user can't write it.
reset; uinput denied
doctor
expect "uinput denied, no rule: says the user can't write and gives the uaccess rule" "✗ Virtual devices: no write access to /dev/uinput" "Your user can't write to /dev/uinput" "sudo tee /etc/udev/rules.d/70-padwight-uinput.rules" "sudo udevadm control --reload"
install_files 70-padwight-uinput.rules
doctor
expect "uinput denied, rule installed: asks to apply it, no second install" "✗ Virtual devices: no write access to /dev/uinput" "is installed, but not yet applied to your session" "sudo udevadm control --reload"
refute "uinput denied, rule installed: does not offer to write it again" "sudo tee /etc/udev/rules.d/70-padwight-uinput.rules"
echo 'KERNEL=="uinput", MODE="0666"' >> /etc/udev/rules.d/70-padwight-uinput.rules
doctor
expect "uinput rule edited by hand: noted as differing, with the fix that restores it" "• Virtual devices: the installed 70-padwight-uinput.rules differs from this version's" "sudo tee /etc/udev/rules.d/70-padwight-uinput.rules"

# 3. uinput fine; the desktop session.
reset; uinput writable
doctor
expect "writable uinput: ok" "✓ Virtual devices: /dev/uinput is writable"
EXTRA_ENV="DISPLAY=:0"; doctor
expect "an X11 session: a problem that says Wayland is needed" "✗ Desktop session: X11, which isn't supported" "Log in to your desktop's Wayland session"
status_is "an X11 session fails the check" 1
EXTRA_ENV="WAYLAND_DISPLAY=wayland-0"; doctor
expect "a Wayland session: ok" "✓ Desktop session: Wayland"
printf '#!/bin/sh\n' > /opt/stub/wl-copy; chmod 755 /opt/stub/wl-copy
P=/opt/pw:/opt/stub:/usr/bin:/bin doctor
expect "wl-copy on the path: found" "✓ Clipboard: wl-copy found"
P=/opt/pw:/usr/bin:/bin

# 4. Controllers.
reset; uinput writable
pad event5 pad xpad 644
doctor
expect "a readable pad" "✓ Controller: Fake xpad pad (event5) is readable"
reset; uinput writable
pad event5 pad xpad 000
doctor
expect "an unreadable pad: points at the input group" "✗ Controller: no read access to Fake xpad pad (event5)" "sudo usermod -aG input \$USER"
reset; uinput writable
pad event5 pad playstation 644; pad event6 motion playstation 000
doctor
expect "an unreadable PlayStation motion sensor, no rule: names it and gives the rule" "✗ Gyro: no read access to the motion sensor of Fake playstation motion (event6)" "The motion-sensor rule isn't installed." "sudo tee /etc/udev/rules.d/70-padwight-motion.rules"
expect "a connected PlayStation pad is mentioned for the missing motion and hidraw rules" "A PlayStation or Switch controller is connected."
install_files 70-padwight-motion.rules
doctor
expect "motion rule installed but not applied: says to reconnect" "The rule is installed, but not yet applied to this controller." "✓ Gyro: 70-padwight-motion.rules is installed"
reset; uinput writable
pad event5 pad playstation 644; pad event6 motion playstation 644
doctor
expect "a readable motion sensor" "✓ Gyro: Fake playstation motion (event6) is readable"
install_files 70-padwight-motion.rules 72-padwight-hidraw.rules
doctor
expect "both optional rules installed: ok" "✓ Gyro: 70-padwight-motion.rules is installed" "✓ Raw controller access: 72-padwight-hidraw.rules is installed"
echo '# old' > /etc/udev/rules.d/72-padwight-hidraw.rules
doctor
expect "a hidraw rule replaced by an old one: noted as differing, with the fix" "• Raw controller access: the installed 72-padwight-hidraw.rules differs from this version's" "sudo tee /etc/udev/rules.d/72-padwight-hidraw.rules"

# 5. Everything in place.
reset; uinput writable; install_files 70-padwight-uinput.rules 70-padwight-motion.rules 72-padwight-hidraw.rules padwight-uinput.conf
pad event5 pad xpad 644
printf '#!/bin/sh\n' > /opt/stub/wl-copy; chmod 755 /opt/stub/wl-copy
EXTRA_ENV="WAYLAND_DISPLAY=wayland-0"; P=/opt/pw:/opt/stub:/usr/bin:/bin doctor
status_is "a complete setup passes" 0
expect "a complete setup says so" "Everything padwight needs is in place."
refute "a complete setup has no problem marks" "✗"

# 6. doctor --files.
reset; uinput writable
doctor --files
status_is "--files with nothing installed" 0
[ -z "$out" ] && ok "--files with nothing installed prints nothing" || bad "--files with nothing installed prints nothing" "$out"
install_files 70-padwight-uinput.rules 70-padwight-motion.rules 72-padwight-hidraw.rules padwight-uinput.conf
doctor --files
[ -z "$out" ] && ok "--files with all current prints nothing" || bad "--files with all current prints nothing" "$out"
for f in 70-padwight-uinput.rules 70-padwight-motion.rules 72-padwight-hidraw.rules; do echo "# a comment I added" >> "/etc/udev/rules.d/$f"; done
echo "# a comment I added" >> /etc/modules-load.d/padwight-uinput.conf
doctor --files
[ -z "$out" ] && ok "--files ignores comments added by hand" || bad "--files ignores comments added by hand" "$out"
for f in $RULES; do echo 'KERNEL=="x", MODE="0666"' >> "/etc/udev/rules.d/$f"; done
doctor --files
status_is "--files with edited rules" 0
expect "--files lists each edited rule with its fix and says why" "This version changes system files that need root" \
    "the installed 70-padwight-uinput.rules differs" "sudo tee /etc/udev/rules.d/70-padwight-uinput.rules" \
    "the installed 70-padwight-motion.rules differs" "sudo tee /etc/udev/rules.d/70-padwight-motion.rules" \
    "the installed 72-padwight-hidraw.rules differs" "sudo tee /etc/udev/rules.d/72-padwight-hidraw.rules"
refute "--files does not mention the module list while it is unchanged" "padwight-uinput.conf differs"
echo "something else" > /etc/modules-load.d/padwight-uinput.conf
doctor --files
expect "--files lists a hand-edited module list" "the installed padwight-uinput.conf differs" "sudo tee /etc/modules-load.d/padwight-uinput.conf"
refute "--files lists only files that changed (no 'Everything' summary, no healthy checks)" "✓"
for f in $RULES; do cp "/dist/$f" /etc/udev/rules.d/; done; cp /dist/padwight-uinput.conf /etc/modules-load.d/
doctor --files
[ -z "$out" ] && ok "--files prints nothing again once the files are restored" || bad "--files prints nothing again once the files are restored" "$out"

echo
echo "$pass passed, $failed failed"
[ "$failed" -eq 0 ]
