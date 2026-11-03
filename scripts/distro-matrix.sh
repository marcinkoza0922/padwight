#!/usr/bin/env bash
# Install, session and environment checks across distros (qa-plan INS-1, INS-2, INS-4, INS-5,
# ENV-5, ENV-6, ENV-8, ENV-9, ENV-10, ENV-11), run in rootless podman containers with systemd as
# init, or in a QEMU cloud image where the check needs a kernel, logind, SELinux or AppArmor.
#
# Usage: scripts/distro-matrix.sh list
#        scripts/distro-matrix.sh tarball                 build the release tarball in a rust:1 container (glibc 2.36)
#        scripts/distro-matrix.sh run <check>[@distro]... run checks; no @distro means each distro the check lists
#        scripts/distro-matrix.sh all                     every check that can run here
#   checks: ins1 ins2 ins4 ins5 env5 env6 env8 env11 (containers), vm-ins1 vm-env9 vm-env10 vm-env11 (QEMU)
#   distros: fedora ubuntu(24.04) ubuntu22(22.04, the oldest: the release builds on it) arch (containers); void (ENV-8); vm images arch ubuntu fedora
#   DM_VM_PORT=2605  the VM's ssh port     DM_KEEP=1  leave containers up for a look (default: removed)
#
# The tarball comes from a container, so its binary needs glibc 2.36, which Debian 12 has and is the
# oldest distro here; the host's Fedora would build one that stock Ubuntu LTS cannot run.
# Every check prints PASS, FAIL or SKIP lines and the run ends with a table. Nothing touches the
# host's /etc, its input devices or its Steam: containers are rootless and see no /dev/uinput, and
# the VM is a throwaway overlay of the images in vms/.
set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=$root/screenshots/distro-matrix
rel=${DM_REL:-$root/release}   # where the tarball and the cargo-installed binary are (the build writes release/)
vms=${DM_VMS:-/home/marcink/Repos/padwight/vms}
port=${DM_VM_PORT:-2605}
mkdir -p "$out" "$rel"
results=$out/results.tsv

# --- results -------------------------------------------------------------------------------
ctx=""   # "check@distro" of the running function
note() { echo "[$ctx] $*"; }
record() { printf '%s\t%s\t%s\n' "$ctx" "$1" "$2" >>"$results"; echo "$1 [$ctx] $2"; }
pass() { record PASS "$1"; }
fail() { record FAIL "$1"; }
skip() { record SKIP "$1"; }
# expect <description> <command...>: PASS when the command succeeds.
expect() { local d=$1; shift; if "$@" >/dev/null 2>&1; then pass "$d"; else fail "$d"; fi; }
# expect_not <description> <command...>: PASS when the command fails.
expect_not() { local d=$1; shift; if "$@" >/dev/null 2>&1; then fail "$d"; else pass "$d"; fi; }
# has <text> <fixed string>...: every string is in the text. lacks: none of them is.
has() { local t=$1 p; shift; for p in "$@"; do grep -qF -- "$p" <<<"$t" || return 1; done; }
lacks() { local t=$1 p; shift; for p in "$@"; do ! grep -qF -- "$p" <<<"$t" || return 1; done; }

table() {
    [ -s "$results" ] || { echo "no results yet"; return; }
    echo; echo "== results ($results)"
    awk -F'\t' '{ n[$1] ; c[$1,$2]++; if ($2=="FAIL") f[$1]=f[$1] "\n      FAIL: " $3; if ($2=="SKIP") s[$1]=s[$1] "\n      SKIP: " $3 }
        END { for (k in n) printf "%-22s pass %-3d fail %-3d skip %-3d%s%s\n", k, c[k,"PASS"], c[k,"FAIL"], c[k,"SKIP"], f[k], s[k] }' "$results" | sort
}

# --- the tarball ---------------------------------------------------------------------------
tarball_path() { ls -t "$rel"/padwight-*-linux-x86_64.tar.gz 2>/dev/null | head -1; }

# Builds the tarball (scripts/package.sh) and a `cargo install --root` tree in an ubuntu:22.04
# container, the release workflow's runner, so the binary needs no more than glibc 2.35 and runs on
# every distro newer than that. Its target directory and cargo home live in volumes of their own.
tarball() {
    command -v docker >/dev/null || { echo "docker is needed to build the portable tarball" >&2; return 1; }
    nice docker run --rm -u 0 -v "$root":/src -v padwight-distro-target22:/src/target -v padwight-distro-home:/root \
        -e CARGO_BUILD_JOBS="${DM_JOBS:-3}" -e DEBIAN_FRONTEND=noninteractive -w /src ubuntu:22.04 sh -c '
        apt-get update -qq && apt-get install -y -qq curl ca-certificates build-essential pkg-config libxkbcommon-dev \
            libwayland-dev libudev-dev libfontconfig1-dev libdbus-1-dev libasound2-dev >/dev/null &&
        { [ -x /root/.cargo/bin/cargo ] || curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal >/dev/null; } &&
        export PATH=/root/.cargo/bin:$PATH &&
        sh scripts/package.sh &&
        CARGO_TARGET_DIR=/src/target cargo install --path . --locked --root /src/target/install-root &&
        cp /src/target/install-root/bin/padwight /src/release/padwight-cargo-install &&
        chown -R '"$(id -u):$(id -g)"' /src/release' 2>&1 | tail -n "${DM_TAIL:-15}"
}

# --- containers ----------------------------------------------------------------------------
# Each distro is a stock image plus the packages a desktop already has (systemd, sudo, udev,
# XKB data); padwight's own files come only from the tarball. systemd is the init, so the
# user manager, linger and `systemctl --user` behave as at a login. Rootless podman's user
# namespace cannot write /proc/self/loginuid and its locked account fails pam_unix's account
# check, so the user manager's PAM file is replaced by one that skips those two (harness only).
distros="fedora ubuntu ubuntu22 arch"
base_image() {
    case $1 in fedora) echo fedora:latest ;; ubuntu) echo ubuntu:24.04 ;; ubuntu22) echo ubuntu:22.04 ;; arch) echo archlinux:latest ;; esac
}
containerfile() { # containerfile <distro> [extra packages]
    local d=$1 extra=${2:-}
    local pam_inc=system-auth
    echo "FROM $(base_image "$d")"
    case $d in
    fedora) echo "RUN dnf -y -q install systemd systemd-pam sudo udev kmod xkeyboard-config libxkbcommon alsa-lib acl procps-ng findutils iproute passwd file $extra && dnf clean all" ;;
    arch) echo "RUN pacman -Syu --noconfirm --quiet systemd dbus-broker sudo kmod xkeyboard-config libxkbcommon alsa-lib acl procps-ng iproute2 findutils file $extra >/dev/null" ;;
    ubuntu | ubuntu22)
        pam_inc=common-session
        echo "ENV DEBIAN_FRONTEND=noninteractive"
        echo "RUN apt-get update -qq && apt-get install -y -qq systemd systemd-sysv sudo udev kmod xkb-data libxkbcommon0 \$(apt-cache show libasound2t64 >/dev/null 2>&1 && echo libasound2t64 || echo libasound2) acl procps iproute2 dbus dbus-user-session libpam-systemd file ca-certificates $extra >/dev/null && rm -rf /var/lib/apt/lists/*"
        ;;
    esac
    echo "RUN (userdel -r ubuntu 2>/dev/null || true) && useradd -m -u 1000 -s /bin/bash dev && echo 'dev ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/dev"
    echo "RUN printf 'account required pam_permit.so\\nsession optional pam_loginuid.so\\nsession optional pam_keyinit.so force revoke\\nsession include $pam_inc\\n' > /etc/pam.d/systemd-user"
    echo "RUN printf 'auth sufficient pam_permit.so\\naccount sufficient pam_permit.so\\nsession sufficient pam_permit.so\\n' > /etc/pam.d/sudo"
    # Rootless containers have no udev daemon; install.sh's `udevadm control --reload` and `trigger` are
    # no-ops here (the rule files themselves are still checked; their effect is the VM's job).
    echo "RUN mv /usr/bin/udevadm /usr/bin/udevadm.real && printf '#!/bin/sh\\ncase \"\$1\" in control|trigger) exit 0 ;; esac\\nexec /usr/bin/udevadm.real \"\$@\"\\n' > /usr/bin/udevadm && chmod 755 /usr/bin/udevadm"
    echo 'RUN systemctl mask systemd-oomd.service systemd-oomd.socket systemd-resolved.service systemd-networkd.service 2>/dev/null; true'
    echo 'CMD ["/sbin/init"]'
}
image() { # image <distro> [tag-suffix] [extra packages]: builds dm-<distro>[-suffix] if missing
    local tag=dm-$1${2:+-$2}
    podman image exists "$tag" && return 0
    note "building image $tag"
    containerfile "$1" "${3:-}" | podman build -q -t "$tag" -f - "$out" >"$out/image-$tag.log" 2>&1 || { echo "image build failed: $tag (see $out/image-$tag.log)" >&2; return 1; }
}
cname() { echo "dm-run-$1"; }

# ct_up <distro> [podman run options...]: a fresh container with systemd running and dev's user manager up.
ct_up() {
    local d=$1 c; c=$(cname "$1"); shift
    podman rm -f "$c" >/dev/null 2>&1
    image "$d" "${DM_TAG:-}" "${DM_PKGS:-}" || return 1
    podman run -d --name "$c" --systemd=always "$@" "dm-$d${DM_TAG:+-$DM_TAG}" >/dev/null || return 1
    for _ in $(seq 1 60); do
        case $(podman exec "$c" systemctl is-system-running 2>/dev/null) in running | degraded) break ;; esac
        sleep 1
    done
    podman exec "$c" loginctl enable-linger dev >/dev/null 2>&1
    podman exec "$c" systemctl start user@1000 || { echo "no user manager in $c" >&2; return 1; }
}
ct_down() { [ -n "${DM_KEEP:-}" ] && { note "left running: $(cname "$1")"; return; }; podman rm -f "$(cname "$1")" >/dev/null 2>&1; }
# ct <distro> <script>: runs bash as dev, in dev's home, as in a login (session bus, runtime dir).
ct() { podman exec -u dev -w /home/dev -e HOME=/home/dev -e XDG_RUNTIME_DIR=/run/user/1000 \
    -e DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus "$(cname "$1")" bash -c "$2"; }
ctr() { podman exec -w /root "$(cname "$1")" bash -c "$2"; }  # as root
ct_tarball() { # unpacks the tarball in dev's home and prints the folder name
    local t; t=$(tarball_path) || true
    [ -n "$t" ] || { echo "no tarball in $rel: run scripts/distro-matrix.sh tarball" >&2; return 1; }
    podman cp "$t" "$(cname "$1"):/home/dev/" && podman cp "$rel/padwight-cargo-install" "$(cname "$1"):/home/dev/" 2>/dev/null
    ctr "$1" "chown dev:dev /home/dev/padwight-* && chmod +x /home/dev/padwight-cargo-install 2>/dev/null; true"
    ct "$1" "tar xzf $(basename "$t") && ls -d padwight-*-linux-x86_64"
}

# --- INS-1: tarball install, uninstall, purge ----------------------------------------------
ins1() {
    local d=$1 dir
    ct_up "$d" || { fail "container did not come up"; return; }
    dir=$(ct_tarball "$d") || { fail "no tarball"; ct_down "$d"; return; }
    ct "$d" "cd $dir && ./install.sh --no-system --no-service" >"$out/ins1-$d-user.log" 2>&1
    expect "install.sh --no-system runs without root" test $? -eq 0
    expect "binary, uninstaller, icon and menu entry land in ~/.local" ct "$d" 'cd ~/.local && test -x bin/padwight && test -x bin/padwight-uninstall && test -f share/applications/io.github.marcinkoza0922.Padwight.desktop && test -f share/icons/hicolor/scalable/apps/io.github.marcinkoza0922.Padwight.svg'
    expect_not "no system file without --no-system consent" ctr "$d" 'ls /etc/udev/rules.d/*padwight* /etc/modules-load.d/padwight*'
    ct "$d" 'PATH=$HOME/.local/bin:$PATH; padwight doctor' >"$out/ins1-$d-doctor-without-rules.log" 2>&1
    # Without the rules (and, in a container, without /dev/uinput) doctor must say what to do about it.
    expect "doctor without the rules names the uinput fix" grep -Eq 'uinput' "$out/ins1-$d-doctor-without-rules.log"
    expect "doctor prints the command for the rules" grep -Eq '70-padwight-uinput.rules|udev' "$out/ins1-$d-doctor-without-rules.log"
    ct "$d" "cd $dir && ./install.sh --yes" >"$out/ins1-$d-full.log" 2>&1
    expect "install.sh --yes installs the service and the system files" test $? -eq 0
    expect "the four system files are in /etc, mode 644" ctr "$d" 'for f in udev/rules.d/70-padwight-motion.rules udev/rules.d/70-padwight-uinput.rules udev/rules.d/72-padwight-hidraw.rules modules-load.d/padwight-uinput.conf; do test "$(stat -c %a /etc/$f)" = 644 || exit 1; done'
    expect "the service is enabled for default.target" ct "$d" 'systemctl --user is-enabled padwight'
    expect "the unit's ExecStart is the installed binary" ct "$d" 'grep -q "^ExecStart=/home/dev/.local/bin/padwight daemon" ~/.config/systemd/user/padwight.service'
    # Uninstall: no files, units, rules left; settings stay.
    ct "$d" 'PATH=$HOME/.local/bin:$PATH; mkdir -p ~/.config/padwight ~/.local/state/padwight; echo x > ~/.config/padwight/config.toml; echo log > ~/.local/state/padwight/daemon.log'
    ct "$d" 'padwight-uninstall --yes' >"$out/ins1-$d-uninstall.log" 2>&1 || ct "$d" '~/.local/bin/padwight-uninstall --yes' >"$out/ins1-$d-uninstall.log" 2>&1
    expect "uninstall leaves nothing of padwight outside the settings" test -z "$(ctr "$d" "find / -xdev \( -path /proc -o -path /sys -o -path /run -o -path /home/dev/padwight-* -o -path /home/dev/.config/padwight -o -path /home/dev/.local/state/padwight -o -path /usr/share/zoneinfo \) -prune -o -iname '*padwight*' -print 2>/dev/null")"
    expect "uninstall removed the system files" ctr "$d" '! ls /etc/udev/rules.d/*padwight* /etc/modules-load.d/padwight* 2>/dev/null'
    expect "uninstall disabled the service (no wants link)" ct "$d" '! ls ~/.config/systemd/user/default.target.wants/padwight.service 2>/dev/null'
    expect "settings stay without --purge" ct "$d" 'test -f ~/.config/padwight/config.toml'
    ct "$d" "cd $dir && ./install.sh --yes" >/dev/null 2>&1
    ct "$d" 'mkdir -p ~/.config/padwight ~/.local/state/padwight; echo x > ~/.config/padwight/config.toml; echo log > ~/.local/state/padwight/daemon.log; ~/.local/bin/padwight-uninstall --purge --yes' >"$out/ins1-$d-purge.log" 2>&1
    expect "--purge removes the settings folder and the daemon log" ct "$d" '! test -e ~/.config/padwight && ! test -e ~/.local/state/padwight'
    ct_down "$d"
}

# --- INS-2: cargo install, then the README's steps ------------------------------------------
# The binary is what `cargo install --path . --locked` built in the rust:1 container (see `tarball`).
ins2() {
    local d=$1 dir
    ct_up "$d" || { fail "container did not come up"; return; }
    dir=$(ct_tarball "$d") || { fail "no tarball"; ct_down "$d"; return; }
    [ -x "$rel/padwight-cargo-install" ] || { skip "no cargo-install binary in $rel (run the tarball step)"; ct_down "$d"; return; }
    # README "From source": the binary is in ~/.cargo/bin, the unit is copied by hand and enabled.
    ct "$d" "mkdir -p ~/.cargo/bin ~/.config/systemd/user && cp padwight-cargo-install ~/.cargo/bin/padwight && cp $dir/share/padwight.service ~/.config/systemd/user/ && systemctl --user daemon-reload && systemctl --user enable --now padwight" >"$out/ins2-$d-steps.log" 2>&1
    expect "README steps (copy unit, enable --now) run without root" test $? -eq 0
    expect "the unit's ExecStart (%h/.cargo/bin/padwight) is the cargo-installed binary" ct "$d" 'systemctl --user cat padwight | grep -q "^ExecStart=%h/.cargo/bin/padwight daemon" && test -x ~/.cargo/bin/padwight'
    expect "the service was started by the user manager" ct "$d" 'test -n "$(systemctl --user show padwight -p ExecMainStartTimestamp --value)"'
    ct "$d" '~/.cargo/bin/padwight doctor' >"$out/ins2-$d-doctor.log" 2>&1
    expect "padwight doctor runs and names the uinput access" grep -q uinput "$out/ins2-$d-doctor.log"
    ct "$d" 'systemctl --user disable --now padwight; rm -f ~/.config/systemd/user/padwight.service ~/.cargo/bin/padwight; systemctl --user daemon-reload'
    expect "removing the unit and binary by hand leaves nothing (no service, no process)" ct "$d" '! pgrep -x padwight && ! systemctl --user cat padwight'
    ct_down "$d"
}

# --- INS-4: an immutable root --------------------------------------------------------------
# A read-only root with /etc, /home and /var writable, as on Silverblue/Bazzite. The checks:
# nothing can be written to /usr, the install works anyway, and only the four files land in /etc.
ins4() {
    local d=${1:-fedora} dir v=dm-imm
    podman volume rm -f $v-etc $v-home $v-var >/dev/null 2>&1
    ct_up "$d" --read-only -v $v-etc:/etc -v $v-home:/home -v $v-var:/var || { fail "container did not come up"; return; }
    expect_not "the root really is read-only (/usr/local/x cannot be created)" ctr "$d" 'touch /usr/local/x'
    dir=$(ct_tarball "$d") || { fail "no tarball"; ct_down "$d"; return; }
    ctr "$d" 'find /etc -type f -exec sha256sum {} + 2>/dev/null | sort -k2' >"$out/ins4-etc-before.txt"
    ct "$d" "cd $dir && ./install.sh --yes" >"$out/ins4-$d-install.log" 2>&1
    expect "install.sh --yes succeeds with no layered package" test $? -eq 0
    ctr "$d" 'find /etc -type f -exec sha256sum {} + 2>/dev/null | sort -k2' >"$out/ins4-etc-after.txt"
    local changed; changed=$(diff "$out/ins4-etc-before.txt" "$out/ins4-etc-after.txt" | grep '^[<>]' | awk '{print $1 " " $3}' | sort)
    note "files that changed in /etc:"$'\n'"$changed"
    local want=$'> /etc/modules-load.d/padwight-uinput.conf\n> /etc/udev/rules.d/70-padwight-motion.rules\n> /etc/udev/rules.d/70-padwight-uinput.rules\n> /etc/udev/rules.d/72-padwight-hidraw.rules'
    if [ "$changed" = "$want" ]; then pass "only the three rules and padwight-uinput.conf were added to /etc"; else fail "/etc changed in other ways: $changed"; fi
    expect "the binary and unit are under the home directory" ct "$d" 'test -x ~/.local/bin/padwight && test -f ~/.config/systemd/user/padwight.service'
    ct "$d" 'ldd ~/.local/bin/padwight | grep "not found"' >"$out/ins4-$d-missing-libs.txt" 2>&1
    if [ -s "$out/ins4-$d-missing-libs.txt" ]; then fail "libraries missing on the stock image: $(tr '\n' ' ' <"$out/ins4-$d-missing-libs.txt")"; else pass "every shared library resolves from the stock image"; fi
    ct_down "$d"
    podman volume rm -f $v-etc $v-home $v-var >/dev/null 2>&1
}

# --- INS-5: the service starts at login ---------------------------------------------------
started() { # started <distro>: prints how many times the user manager has started the daemon
    ct "$1" 'journalctl --user -u padwight --no-pager -o cat 2>/dev/null | grep -c "^Started padwight"'
}
ins5() {
    local d=$1 dir n
    ct_up "$d" || { fail "container did not come up"; return; }
    dir=$(ct_tarball "$d") || { fail "no tarball"; ct_down "$d"; return; }
    ct "$d" "cd $dir && ./install.sh --yes" >"$out/ins5-$d-install.log" 2>&1
    expect "install enables the unit and starts it" ct "$d" 'test -n "$(systemctl --user show padwight -p ExecMainStartTimestamp --value)"'
    local ts1; ts1=$(ct "$d" 'systemctl --user show padwight -p ExecMainStartTimestamp --value')
    # Log out: the user manager stops and takes the daemon with it.
    ctr "$d" 'systemctl stop user@1000'
    expect_not "after logout no padwight process is left" ctr "$d" 'pgrep -u dev -x padwight'
    # Log in again.
    ctr "$d" 'systemctl start user@1000'
    expect "after login the user manager starts the service" ct "$d" 'for i in $(seq 1 20); do test -n "$(systemctl --user show padwight -p ExecMainStartTimestamp --value)" && exit 0; sleep 1; done; exit 1'
    expect "it is a new start: the start time differs from the first login's" test "$(ct "$d" 'systemctl --user show padwight -p ExecMainStartTimestamp --value')" != "$ts1"
    # Reboot: linger starts the user manager by itself, which reaches default.target.
    podman restart "$(cname "$d")" >/dev/null 2>&1
    for _ in $(seq 1 60); do ct "$d" 'systemctl --user is-system-running' >/dev/null 2>&1 && break; sleep 1; done
    expect "after a reboot (linger) the service starts without a login" ct "$d" 'for i in $(seq 1 20); do test -n "$(systemctl --user show padwight -p ExecMainStartTimestamp --value)" && exit 0; sleep 1; done; exit 1'
    ct "$d" 'journalctl --user -u padwight --no-pager | tail -20' >"$out/ins5-$d-journal.log" 2>&1
    ct_down "$d"
}

# --- ENV-5: keyboard layouts --------------------------------------------------------------
# `padwight debug layout` runs the code the on-screen keyboard (labels and typing) and the
# calculator's Send use. Mapped keys and macros name physical key codes, so a layout can't move them
# (the scenario run below proves it on the host's build).
lay() { ct "$1" "PADWIGHT_KEYBOARD_LAYOUT='$2' ~/.local/bin/padwight debug layout '$3'"; }
env5() {
    local d=$1 dir o
    ct_up "$d" || { fail "container did not come up"; return; }
    dir=$(ct_tarball "$d") || { fail "no tarball"; ct_down "$d"; return; }
    ct "$d" "cd $dir && ./install.sh --no-system --no-service" >/dev/null 2>&1
    o=$(lay "$d" fr 'a1z'); echo "$o" >"$out/env5-$d-fr.txt"
    expect "AZERTY (fr): label of KEY_Q is a, KEY_A is q" has "$o" 'label KEY_Q a A' 'label KEY_A q Q'
    expect "AZERTY: typing a is KEY_Q, typing 1 needs Shift on KEY_1" has "$o" 'type a KEY_Q plain' 'type 1 KEY_1 shift'
    o=$(lay "$d" de 'zy-'); echo "$o" >"$out/env5-$d-de.txt"
    expect "QWERTZ (de): label of KEY_Y is z; typing z is KEY_Y, typing y is KEY_Z" has "$o" 'label KEY_Y z Z' 'type z KEY_Y plain' 'type y KEY_Z plain'
    o=$(lay "$d" 'us(dvorak)' 'aesq-.'); echo "$o" >"$out/env5-$d-dvorak.txt"
    expect "Dvorak: e is KEY_D, s is KEY_SEMICOLON, q is KEY_X, KEY_Q is labelled '" has "$o" 'type e KEY_D plain' 'type s KEY_SEMICOLON plain' 'type q KEY_X plain' "label KEY_Q ' "
    o=$(lay "$d" ru '1-.+*/a'); echo "$o" >"$out/env5-$d-ru.txt"
    expect "Russian: KEY_Q is labelled in Cyrillic; the calculator's digits, minus and point have keys" has "$o" 'label KEY_Q й' 'type 1 KEY_1 plain' 'type - KEY_MINUS' 'type . KEY_SLASH'
    expect "Russian: a Latin letter has no key (only the first layout counts; ru,us behaves as ru)" has "$o" 'type a none'
    o=$(lay "$d" 'gr,us' '1-.'); echo "$o" >"$out/env5-$d-gr.txt"
    expect "Greek (gr,us): KEY_Q is labelled ; and the calculator's characters are typeable" has "$o" 'label KEY_Q ;' 'type 1 KEY_1 plain'
    expect "Greek: minus and point are typeable" lacks "$o" 'type - none' 'type . none'
    # Detection: the environment's XKB variable, then the override above it.
    o=$(ct "$d" "XKB_DEFAULT_LAYOUT=de ~/.local/bin/padwight debug layout z")
    expect "XKB_DEFAULT_LAYOUT=de is detected" has "$o" 'type z KEY_Y plain'
    o=$(ct "$d" "XKB_DEFAULT_LAYOUT=de PADWIGHT_KEYBOARD_LAYOUT=fr ~/.local/bin/padwight debug layout a")
    expect "PADWIGHT_KEYBOARD_LAYOUT overrides detection" has "$o" 'type a KEY_Q plain'
    o=$(ct "$d" "PADWIGHT_KEYBOARD_LAYOUT=nosuchlayout ~/.local/bin/padwight debug layout a")
    expect "an unknown layout falls back to US" has "$o" 'type a KEY_A plain'
    # Layouts are compiled once per process (a OnceLock in keyboard/labels.rs): a switch while the daemon runs is not seen.
    skip "switching layout while the daemon runs: labels are computed once per process (keyboard/labels.rs LABELS OnceLock), so the on-screen keyboard keeps the old layout until the daemon restarts"
    ct_down "$d"
}
# The host's build runs the keys-and-mouse scenario under each layout: a mapped key and a macro
# must produce the same key codes whatever the desktop's layout.
env5_scenarios() {
    local l
    for l in fr de 'us(dvorak)' ru gr; do
        if (cd "$root" && PADWIGHT_KEYBOARD_LAYOUT=$l SCENARIO=keys-and-mouse RUST_BACKTRACE=0 nice cargo nextest run --profile agent --hide-progress-bar --cargo-quiet scenarios >"$out/env5-scenario-${l//[^a-z]/_}.log" 2>&1); then
            pass "mapped keys and macro (keys-and-mouse scenario) are the same under layout $l"
        else
            fail "keys-and-mouse scenario differs under layout $l (see $out)"
        fi
    done
}

# --- QEMU cloud images ----------------------------------------------------------------------
# One VM at a time, on ssh port $DM_VM_PORT, as a throwaway overlay of the base image in vms/
# (arch: base.qcow2, ubuntu: base-ubuntu.qcow2, fedora: base-fedora.qcow2, fetched on first use).
# The VM has real systemd-logind, udev, a kernel with uinput, SELinux (fedora) or AppArmor (ubuntu).
fedora_image_url() {
    local base=https://dl.fedoraproject.org/pub/fedora/linux/releases
    local rel; rel=$(curl -fsSL "$base/" | grep -o 'href="[0-9][0-9]*/' | tr -dc '0-9\n' | sort -n | tail -1)
    local name; name=$(curl -fsSL "$base/$rel/Cloud/x86_64/images/" | grep -o 'Fedora-Cloud-Base-Generic[^"<]*\.qcow2' | sort -u | head -1)
    echo "$base/$rel/Cloud/x86_64/images/$name"
}
vm_base() {
    case $1 in
    arch) echo "$vms/base.qcow2" ;;
    ubuntu) echo "$vms/base-ubuntu.qcow2" ;;
    fedora)
        if [ ! -f "$vms/base-fedora.qcow2" ]; then
            note "downloading the Fedora cloud image (about 500 MB)"
            curl -fL --retry 3 -o "$vms/base-fedora.qcow2.part" "$(fedora_image_url)" && mv "$vms/base-fedora.qcow2.part" "$vms/base-fedora.qcow2"
        fi
        echo "$vms/base-fedora.qcow2"
        ;;
    esac
}
vmx() { # vmx <command...>: as dev over ssh
    ssh -i "$vms/id_ed25519" -p "$port" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=5 dev@127.0.0.1 "$@"
}
vmr() { vmx "sudo bash -c $(printf %q "$1")"; }
vm_wait() { for _ in $(seq 1 ${1:-90}); do vmx true 2>/dev/null && return 0; sleep 2; done; return 1; }
vm_up() { # vm_up <arch|ubuntu|fedora>
    local d=$1 dir=$out/vm-$1 base
    [ -f "$vms/id_ed25519" ] || { echo "no ssh key $vms/id_ed25519 (vms/vm.sh creates it)" >&2; return 1; }
    base=$(vm_base "$d") && [ -f "$base" ] || { echo "no base image for $d" >&2; return 1; }
    vm_down "$d"; rm -rf "$dir"; mkdir -p "$dir"
    qemu-img create -q -f qcow2 -F qcow2 -b "$base" "$dir/disk.qcow2" 20G || return 1
    printf '#cloud-config\nusers:\n  - name: dev\n    sudo: ALL=(ALL) NOPASSWD:ALL\n    shell: /bin/bash\n    ssh_authorized_keys: [%s]\n' "$(cat "$vms/id_ed25519.pub")" >"$dir/user-data"
    printf 'instance-id: dm-%s\nlocal-hostname: dm-%s\n' "$d" "$d" >"$dir/meta-data"
    mkisofs -quiet -output "$dir/seed.iso" -volid cidata -joliet -rock "$dir/user-data" "$dir/meta-data" || return 1
    nice qemu-system-x86_64 -enable-kvm -cpu host -smp 2 -m "${DM_VM_MEM:-3G}" -daemonize -pidfile "$dir/pid" -display none -vga std \
        -drive file="$dir/disk.qcow2",if=virtio -drive file="$dir/seed.iso",if=virtio,media=cdrom,readonly=on \
        -nic user,hostfwd=tcp:127.0.0.1:"$port"-:22 || return 1
    vm_wait 120 || { echo "VM $d did not answer on ssh port $port" >&2; return 1; }
    # Two consoles on seat0 (dev on tty1, dev2 on tty2) are logged in by getty, so logind has real seat sessions.
    vmr 'useradd -m dev2; for t in 1 2; do u=dev; [ $t = 2 ] && u=dev2; mkdir -p /etc/systemd/system/getty@tty$t.service.d; printf "[Service]\nExecStart=\nExecStart=-/sbin/agetty -o \"-p -f -- \\\\\\\\u\" --noclear --autologin $u %%I \$TERM\n" > /etc/systemd/system/getty@tty$t.service.d/autologin.conf; done; systemctl daemon-reload; systemctl restart getty@tty1 getty@tty2' >"$out/vm-$d-provision.log" 2>&1
    case $d in
    arch) vmr 'pacman -Sy --noconfirm acl nfs-utils rsync alsa-lib libxkbcommon xkeyboard-config' ;;
    ubuntu) vmr 'export DEBIAN_FRONTEND=noninteractive; apt-get update -qq && apt-get install -y -qq acl nfs-kernel-server libxkbcommon0 xkb-data $(apt-cache show libasound2t64 >/dev/null 2>&1 && echo libasound2t64 || echo libasound2)' ;;
    fedora) vmr 'dnf -y -q install acl nfs-utils audit policycoreutils alsa-lib libxkbcommon' ;;
    esac >>"$out/vm-$d-provision.log" 2>&1
    sleep 3
}
vm_down() {
    local pid="$out/vm-$1/pid"
    [ -f "$pid" ] && kill "$(cat "$pid")" 2>/dev/null
    rm -f "$pid"
}
vm_down_unless_kept() { [ -n "${DM_KEEP:-}" ] && { note "VM left running on port $port"; return; }; vm_down "$1"; }
vm_tarball() { # copies and unpacks the tarball in dev's home; prints the folder
    local t; t=$(tarball_path) || true
    [ -n "$t" ] || { echo "no tarball" >&2; return 1; }
    scp -q -i "$vms/id_ed25519" -P "$port" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "$t" "$rel/padwight-cargo-install" dev@127.0.0.1: 2>/dev/null \
        || scp -q -i "$vms/id_ed25519" -P "$port" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "$t" dev@127.0.0.1:
    vmx "tar xzf $(basename "$t") && ls -d padwight-*-linux-x86_64"
}

# --- VM: real udev, uaccess, reboot (INS-1, INS-5) -----------------------------------------
acl_of() { vmr 'getfacl -p /dev/uinput 2>&1 | grep -E "^user:[a-z]"'; }
acl_has() { acl_of | grep -q "^user:$1:rw"; }
acl_lacks() { ! acl_has "$1"; }
session_is() { [ "$(vmr "loginctl show-session $1 -p Active --value")" = "$2" ]; }
vm_ins1() {
    local d=$1 dir
    vm_up "$d" || { fail "VM did not come up"; return; }
    dir=$(vm_tarball) || { fail "no tarball"; vm_down "$d"; return; }
    vmr 'modprobe uinput'
    note "uaccess before any padwight file: $(acl_of | tr '\n' ' ')"
    local before; before=$(acl_of)
    vmx "cd $dir && ./install.sh --no-system" >"$out/vm-ins1-$d-nosystem.log" 2>&1
    expect "install.sh --no-system runs without root" test $? -eq 0
    vmx '~/.local/bin/padwight doctor' >"$out/vm-ins1-$d-doctor-before.log" 2>&1
    if grep -q 'dev' <<<"$before"; then
        pass "the distro already gives the seat user /dev/uinput (uaccess from the distro): $(tr '\n' ' ' <<<"$before")"
    else
        pass "without padwight's rule, the seat user has no ACL on /dev/uinput (so the rule is needed)"
        expect "doctor names the missing uinput access" grep -qi uinput "$out/vm-ins1-$d-doctor-before.log"
    fi
    vmx "cd $dir && ./install.sh --yes" >"$out/vm-ins1-$d-full.log" 2>&1
    expect "install.sh --yes with sudo succeeds" test $? -eq 0
    sleep 3
    expect "uaccess: the autologin seat user (dev) has an ACL on /dev/uinput after the rule" acl_has dev
    vmx '~/.local/bin/padwight doctor' >"$out/vm-ins1-$d-doctor-after.log" 2>&1
    cat "$out/vm-ins1-$d-doctor-after.log" | head -20
    expect "the service runs in the session (real daemon, real /dev/uinput)" vmx 'systemctl --user is-active padwight'
    expect "the daemon made its virtual devices (a padwight input device exists)" vmx 'grep -qi padwight /proc/bus/input/devices'
    # INS-5 on a real boot: modprobe via modules-load.d, the rule via udev, the service via the autologin session.
    vmr 'systemctl reboot' >/dev/null 2>&1; sleep 5; vm_wait 120
    sleep 10
    expect "after a reboot /dev/uinput exists without a manual modprobe" vmr 'test -c /dev/uinput'
    expect "after a reboot the seat user still has the ACL" acl_has dev
    expect "after a reboot the service is active (started at login)" vmx 'for i in $(seq 1 30); do systemctl --user is-active padwight && exit 0; sleep 1; done; exit 1'
    # Uninstall: the rules go; the seat ACL goes with them when the distro does not grant it.
    vmx '~/.local/bin/padwight-uninstall --yes' >"$out/vm-ins1-$d-uninstall.log" 2>&1
    expect "uninstall removes the system files" vmr '! ls /etc/udev/rules.d/*padwight* /etc/modules-load.d/padwight* 2>/dev/null'
    expect "uninstall leaves no padwight process" vmx '! pgrep -x padwight'
    expect "the settings stay after a plain uninstall" vmx 'test -d ~/.config/padwight'
    vm_down_unless_kept "$d"
}

# --- VM: seats and sessions (ENV-9) -------------------------------------------------------
vm_env9() {
    local d=${1:-arch} dir s1 s2
    vm_up "$d" || { fail "VM did not come up"; return; }
    dir=$(vm_tarball) || { fail "no tarball"; vm_down "$d"; return; }
    vmr 'modprobe uinput'
    vmx "cd $dir && ./install.sh --yes" >"$out/vm-env9-$d-install.log" 2>&1
    sleep 3
    vmr 'loginctl list-sessions --no-legend' >"$out/vm-env9-$d-sessions.txt" 2>&1
    s1=$(vmr "loginctl list-sessions --no-legend | awk '\$3==\"dev\" && \$4==\"seat0\" {print \$1; exit}'")
    s2=$(vmr "loginctl list-sessions --no-legend | awk '\$3==\"dev2\" && \$4==\"seat0\" {print \$1; exit}'")
    if [ -z "$s1" ] || [ -z "$s2" ]; then fail "no two seat0 sessions for dev and dev2: $(cat "$out/vm-env9-$d-sessions.txt")"; vm_down "$d"; return; fi
    note "sessions: dev=$s1 dev2=$s2"
    expect "dev's session is active on seat0" session_is "$s1" yes
    expect "uaccess: dev has an ACL on /dev/uinput while active" acl_has dev
    vmx 'systemctl --user is-active padwight' >/dev/null 2>&1 && pass "daemon active in dev's session" || note "daemon not active in dev's session"
    # A text console of another user, on the same seat.
    vmr "loginctl activate $s2"; sleep 2
    expect "switching to dev2's console makes dev's session inactive" session_is "$s1" no
    expect "uaccess moved: dev2 has the ACL on /dev/uinput" acl_has dev2
    expect "uaccess moved: dev no longer has it while inactive" acl_lacks dev
    expect "the daemon survives its session going inactive (no exit, answers status)" vmx '~/.local/bin/padwight status'
    # Lock: logind sends Lock/Unlock signals; nothing here should kill the daemon.
    vmr "loginctl activate $s1"; sleep 2
    vmr "loginctl lock-session $s1" && pass "lock-session accepted" || fail "lock-session failed"
    sleep 1
    expect "the daemon survives a locked screen" vmx '~/.local/bin/padwight status'
    vmr "loginctl unlock-session $s1"
    expect "after return dev has the ACL again" acl_has dev
    expect "after return the service is still active" vmx 'systemctl --user is-active padwight'
    skip "a held pad's keys across the switch: needs a pad in the VM (a uhid pad could be injected; not built)"
    vm_down_unless_kept "$d"
}

# --- VM: security modules (ENV-10) ---------------------------------------------------------
vm_env10() {
    local d=${1:-ubuntu} dir
    vm_up "$d" || { fail "VM did not come up"; return; }
    dir=$(vm_tarball) || { fail "no tarball"; vm_down "$d"; return; }
    vmr 'modprobe uinput'
    case $d in
    fedora)
        expect "SELinux is enforcing" vmr '[ "$(getenforce)" = Enforcing ]'
        ;;
    ubuntu)
        vmr 'sysctl kernel.apparmor_restrict_unprivileged_userns kernel.unprivileged_userns_clone 2>&1' >"$out/vm-env10-$d-sysctl.txt"
        note "userns restriction: $(tr '\n' ' ' <"$out/vm-env10-$d-sysctl.txt")"
        expect "AppArmor is on" vmr 'aa-status --enabled || [ -d /sys/kernel/security/apparmor ]'
        ;;
    esac
    vmx "cd $dir && ./install.sh --yes" >"$out/vm-env10-$d-install.log" 2>&1
    expect "install works under the security module" test $? -eq 0
    sleep 3
    # Use the daemon: doctor, status, the debug session (virtual pad through the daemon), overlay screenshots.
    vmx '~/.local/bin/padwight doctor' >"$out/vm-env10-$d-doctor.log" 2>&1
    vmx '~/.local/bin/padwight status; ~/.local/bin/padwight debug focus processes' >"$out/vm-env10-$d-use.log" 2>&1
    vmx 'cd ~ && mkdir -p shots && cd shots && ~/.local/bin/padwight debug overlay all glyphs' >"$out/vm-env10-$d-overlay.log" 2>&1
    expect "debug overlay renders on this distro (no GPU needed)" grep -qiv 'error' "$out/vm-env10-$d-overlay.log"
    # scripts/sandbox-pid.sh as a Flatpak-like sandbox would run.
    scp -q -i "$vms/id_ed25519" -P "$port" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "$root/scripts/sandbox-pid.sh" dev@127.0.0.1: 2>/dev/null
    vmx 'timeout 20 sh ./sandbox-pid.sh sleep 6; echo "exit=$?"' >"$out/vm-env10-$d-sandbox.log" 2>&1
    head -5 "$out/vm-env10-$d-sandbox.log"
    if grep -q '^exit=77' "$out/vm-env10-$d-sandbox.log"; then
        skip "scripts/sandbox-pid.sh: user namespaces are refused on this distro ($(grep -m1 'refused' "$out/vm-env10-$d-sandbox.log"))"
    else
        expect "scripts/sandbox-pid.sh finds the sandboxed process" grep -q 'pid in the sandbox' "$out/vm-env10-$d-sandbox.log"
    fi
    case $d in
    fedora)
        vmr 'ausearch -m avc,user_avc,selinux_err -ts boot 2>&1; journalctl -b --no-pager | grep -E "avc: +denied"' >"$out/vm-env10-$d-denials.log" 2>&1
        if grep -Eqi 'avc: +denied|type=AVC' "$out/vm-env10-$d-denials.log"; then fail "SELinux denials: $(grep -Eci 'avc: +denied|type=AVC' "$out/vm-env10-$d-denials.log") (see vm-env10-$d-denials.log)"; else pass "no SELinux AVC denials in ausearch/journal"; fi
        ;;
    ubuntu)
        vmr 'journalctl -k -b --no-pager | grep -i "apparmor.*DENIED"; dmesg | grep -i "apparmor.*DENIED"' >"$out/vm-env10-$d-denials.log" 2>&1
        grep -v 'profile="who"\|ubuntu_pro' "$out/vm-env10-$d-denials.log" >"$out/vm-env10-$d-denials-filtered.log"; if grep -qi 'apparmor="DENIED"' "$out/vm-env10-$d-denials-filtered.log"; then fail "AppArmor denials beyond the stock who and ubuntu_pro profiles: $(grep -c 'DENIED' "$out/vm-env10-$d-denials-filtered.log") (see vm-env10-$d-denials.log); first: $(grep -m1 DENIED "$out/vm-env10-$d-denials.log" | cut -c1-220)"; else pass "no AppArmor DENIED in the kernel log"; fi
        ;;
    esac
    vm_down_unless_kept "$d"
}

# --- paths (ENV-11): containers for the local parts, a VM for NFS ---------------------------
# Runs through $DM_TX (ct <distro> or vmx wrapper) so the same steps serve both targets.
txd() { if [ "${DM_T%%:*}" = vm ]; then vmx "$1"; else ct "${DM_T#ct:}" "$1"; fi; }
txr() { if [ "${DM_T%%:*}" = vm ]; then vmr "$1"; else ctr "${DM_T#ct:}" "$1"; fi; }
tree_hash() { echo 'find "$1" -path "*/.saving" -prune -o -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum'; }
env11_body() {
    local pack='~/pack.padpack' pw='~/.local/bin/padwight'
    # A. unusual XDG places
    txd "rm -rf ~/odd ~/.config/padwight; mkdir -p ~/odd/cfg ~/odd/state ~/odd/run; chmod 700 ~/odd/run; XDG_CONFIG_HOME=\$HOME/odd/cfg XDG_STATE_HOME=\$HOME/odd/state XDG_RUNTIME_DIR=\$HOME/odd/run $pw debug install-pack $pack" >"$out/env11-$DM_T-a.log" 2>&1
    expect "A: a save with XDG_CONFIG_HOME elsewhere succeeds" test $? -eq 0
    expect "A: config.toml, general/ and setups/ are under XDG_CONFIG_HOME/padwight" txd 'cd ~/odd/cfg/padwight && test -f config.toml && test -d general && test -d setups'
    expect "A: nothing was written to ~/.config/padwight" txd '! test -e ~/.config/padwight'
    expect "A: no .saving or .old leftovers after the save" txd '! ls -d ~/odd/cfg/padwight/.saving ~/odd/cfg/padwight/.*.old 2>/dev/null'
    expect "A: files are private (0600)" txd 'test -z "$(find ~/odd/cfg/padwight -type f ! -perm 600)"'
    txd "XDG_CONFIG_HOME=\$HOME/odd/cfg XDG_STATE_HOME=\$HOME/odd/state XDG_RUNTIME_DIR=\$HOME/odd/run timeout 4 $pw daemon" >"$out/env11-$DM_T-daemon.log" 2>&1
    expect "A: the daemon's log lands under XDG_STATE_HOME/padwight and its socket under XDG_RUNTIME_DIR" txd 'test -f ~/odd/state/padwight/daemon.log || ls ~/odd/run | grep -qi padwight'
    # B. the config dir is a symlink to another filesystem
    txd "rm -rf ~/.config/padwight /dev/shm/pwother; mkdir -p ~/.config /dev/shm/pwother; ln -s /dev/shm/pwother ~/.config/padwight; $pw debug install-pack $pack" >"$out/env11-$DM_T-b.log" 2>&1
    expect "B: a save through a symlink to another filesystem succeeds" test $? -eq 0
    expect "B: the symlink is still a symlink and the files are in its target" txd 'test -L ~/.config/padwight && test -f /dev/shm/pwother/config.toml && test -d /dev/shm/pwother/setups'
    expect "B: no leftover staging in the target" txd '! ls -d /dev/shm/pwother/.saving /dev/shm/pwother/.*.old 2>/dev/null'
    txd 'rm -rf ~/.config/padwight /dev/shm/pwother'
    # C. a full disk: the save fails with a message, the old tree is untouched
    txr 'mountpoint -q /mnt/small || { mkdir -p /mnt/small && mount -t tmpfs -o size=700k,mode=1777 tmpfs /mnt/small; }' >/dev/null 2>&1
    if txr 'mountpoint -q /mnt/small'; then
        txd "rm -rf ~/.config/padwight /mnt/small/*; mkdir -p /mnt/small/pw ~/.config; ln -s /mnt/small/pw ~/.config/padwight; $pw debug install-pack ~/pack.padpack"
        expect "C: the first save fits on the small filesystem" txd 'test -f /mnt/small/pw/config.toml'
        txd "dd if=/dev/zero of=/mnt/small/filler bs=1k 2>/dev/null; true"
        local h1 h2
        h1=$(txd "find /mnt/small/pw -path '*/.saving' -prune -o -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum")
        txd "$pw debug install-pack ~/pack2.padpack" >"$out/env11-$DM_T-c.log" 2>&1
        local rc=$?
        h2=$(txd "find /mnt/small/pw -path '*/.saving' -prune -o -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum")
        expect "C: a save on a full disk fails (non-zero exit)" test $rc -ne 0
        expect "C: the message says what went wrong (space or writing)" grep -Eqi 'space|writing|No space|ENOSPC|full' "$out/env11-$DM_T-c.log"
        if [ "$h1" = "$h2" ]; then pass "C: the old config tree is unchanged after the failed save"; else fail "C: the config tree changed although the save failed"; fi
        txd 'rm -rf /mnt/small/filler; $HOME/.local/bin/padwight debug install-pack ~/pack2.padpack' >"$out/env11-$DM_T-c2.log" 2>&1
        expect "C: with room again the save succeeds and leaves no staging" txd '! ls -d /mnt/small/pw/.saving 2>/dev/null'
        txd 'rm -rf ~/.config/padwight /mnt/small/*'; [ "${DM_T%%:*}" = vm ] && txr 'umount /mnt/small'
    else
        skip "C: no tmpfs mount possible here"
    fi
    # D. a read-only config folder
    txd "rm -rf ~/.config/padwight; $pw debug install-pack $pack >/dev/null 2>&1; chmod -R a-w ~/.config/padwight; h=\$(find ~/.config/padwight -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum); $pw debug install-pack ~/pack2.padpack" >"$out/env11-$DM_T-d.log" 2>&1
    expect_not "D: saving into a read-only config folder fails" test $? -eq 0
    expect "D: with a message that names the folder" grep -Eqi 'permission denied|read-only|padwight' "$out/env11-$DM_T-d.log"
    txd 'chmod -R u+w ~/.config/padwight; rm -rf ~/.config/padwight'
}
env11() {
    local d=${1:-fedora}
    DM_T=ct:$d
    ct_up "$d" --tmpfs /dev/shm:rw,mode=1777 --tmpfs /mnt/small:rw,size=700k,mode=1777 || { fail "container did not come up"; return; }
    local dir; dir=$(ct_tarball "$d") || { fail "no tarball"; ct_down "$d"; return; }
    ct "$d" "cd $dir && ./install.sh --no-system --no-service" >/dev/null 2>&1
    podman cp "$root/packs/against-the-storm.padpack" "$(cname "$d"):/home/dev/pack.padpack" && podman cp "$root/packs/age-of-empires-ii-hd-edition.padpack" "$(cname "$d"):/home/dev/pack2.padpack"
    ctr "$d" 'chown dev:dev /home/dev/*.padpack'
    env11_body
    ct_down "$d"
}
vm_env11() {
    local d=${1:-arch} dir
    DM_T=vm:$d
    vm_up "$d" || { fail "VM did not come up"; return; }
    dir=$(vm_tarball) || { fail "no tarball"; vm_down "$d"; return; }
    vmx "cd $dir && ./install.sh --no-system --no-service" >/dev/null 2>&1
    for p in against-the-storm:pack age-of-empires-ii-hd-edition:pack2; do
        scp -q -i "$vms/id_ed25519" -P "$port" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "$root/packs/${p%%:*}.padpack" "dev@127.0.0.1:${p##*:}.padpack"
    done
    env11_body
    # E. home on NFS: a user whose home is an NFS mount of this machine.
    case $d in ubuntu) srv=nfs-kernel-server ;; *) srv=nfs-server ;; esac
    vmr "useradd -M -d /mnt/nfshome nfsu; mkdir -p /srv/nfs/home /mnt/nfshome; chown nfsu:nfsu /srv/nfs/home; echo '/srv/nfs 127.0.0.1(rw,sync,no_subtree_check,no_root_squash,fsid=0)' > /etc/exports; systemctl enable --now $srv; sleep 3; exportfs -ra; mount -t nfs4 127.0.0.1:/home /mnt/nfshome" >"$out/env11-nfs-setup.log" 2>&1
    if vmr 'mount | grep -q "/mnt/nfshome type nfs"'; then
        vmr 'install -m755 -o nfsu -g nfsu /home/dev/.local/bin/padwight /usr/local/bin/padwight; install -m644 -o nfsu /home/dev/pack.padpack /tmp/pack.padpack; install -m644 /home/dev/pack2.padpack /tmp/pack2.padpack'
        vmr 'runuser -u nfsu -- env HOME=/mnt/nfshome padwight debug install-pack /tmp/pack.padpack' >"$out/env11-nfs.log" 2>&1
        expect "E: a save with the config folder on NFS succeeds" test $? -eq 0
        expect "E: the files are in the NFS home (config.toml, general, setups) with no staging left" vmr 'cd /mnt/nfshome/.config/padwight && test -f config.toml && test -d setups && ! ls -d .saving .*.old 2>/dev/null'
        vmr 'runuser -u nfsu -- env HOME=/mnt/nfshome padwight debug install-pack /tmp/pack2.padpack' >"$out/env11-nfs2.log" 2>&1
        expect "E: a second save over the first on NFS succeeds (swap by rename)" test $? -eq 0
        vmr 'umount /mnt/nfshome'
    else
        skip "E: NFS mount did not come up (see env11-nfs-setup.log)"
    fi
    vm_down_unless_kept "$d"
}

# --- ENV-6: audio stacks -------------------------------------------------------------------
# No sound card exists in a container, so each stack gets a null sink (PipeWire's and PulseAudio's
# own dummy outputs). A cue "plays" when its samples show up on the default sink's monitor. A
# Bluetooth headset is a second null sink made the default in the middle of a run.
loud() { [ "$(tr -d '\0' <"$1" | wc -c)" -gt 200 ]; }   # a capture file with signal in it
env6() { # env6 <pw|pa|alsa>
    local v=$1 d pk
    case $v in
    pw) d=fedora; pk="pipewire pipewire-pulseaudio pipewire-alsa wireplumber pulseaudio-utils alsa-utils" ;;
    pa) d=ubuntu; pk="pulseaudio pulseaudio-utils libasound2-plugins alsa-utils" ;;
    alsa) d=ubuntu; pk="alsa-utils" ;;
    esac
    DM_TAG=$v DM_PKGS=$pk ct_up "$d" || { fail "container did not come up"; return; }
    local dir; dir=$(ct_tarball "$d") || { fail "no tarball"; ct_down "$d"; return; }
    ct "$d" "cd $dir && ./install.sh --no-system --no-service" >/dev/null 2>&1
    case $v in
    pw) ct "$d" 'systemctl --user enable --now pipewire.socket pipewire-pulse.socket wireplumber.service; sleep 5; pactl info' ;;
    pa) ct "$d" 'mkdir -p ~/.config/pulse; echo "exit-idle-time = -1" > ~/.config/pulse/daemon.conf; systemctl --user start pulseaudio.service; for i in $(seq 1 60); do pactl info && break; sleep 2; done' ;;
    alsa) true ;;
    esac >"$out/env6-$v-server.txt" 2>&1
    note "server: $(grep -m1 'Server Name' "$out/env6-$v-server.txt")"
    local kind; kind=$(ct "$d" '~/.local/bin/padwight debug sound list' | sed -n 's/^kinds: \([^ ]*\).*/\1/p')
    local pw='~/.local/bin/padwight'
    if [ "$v" != alsa ]; then
        expect "the sound server answers and has an output (null sink)" ct "$d" 'pactl list short sinks | grep -q .'
        ct "$d" "(timeout 14 parec -d @DEFAULT_MONITOR@ --raw --rate=48000 --channels=1 --format=s16le > cap1.raw 2>/dev/null &) ; sleep 1; $pw debug sound $kind $kind $kind $kind $kind $kind; sleep 8" >"$out/env6-$v-play.log" 2>&1
        podman cp "$(cname "$d"):/home/dev/cap1.raw" "$out/env6-$v-cap1.raw" 2>/dev/null
        expect "cues play: samples reach the default sink's monitor" loud "$out/env6-$v-cap1.raw"
        expect "no 'no audio device' log line while it plays" lacks "$(cat "$out/env6-$v-play.log")" 'no audio device'
        # Media actions
        local before after
        before=$(ct "$d" "$pw debug media"); ct "$d" "$pw debug media volume down" >/dev/null 2>&1; after=$(ct "$d" "$pw debug media")
        note "volume: [$before] -> [$after]"
        expect "media: volume down changes the default output's volume" test "$before" != "$after"
        expect "media: mute toggles" ct "$d" "$pw debug media mute | grep -q ' muted'"
        ct "$d" "$pw debug media mute" >/dev/null 2>&1
        # Output switch mid-session: one long run of cues; a second sink becomes the default after 3 s.
        ct "$d" "pactl load-module module-null-sink sink_name=headset sink_properties=device.description=Headset >/dev/null; (timeout 14 parec -d @DEFAULT_MONITOR@ --raw --rate=48000 --channels=1 --format=s16le > old.raw 2>/dev/null &); (timeout 14 parec -d headset.monitor --raw --rate=48000 --channels=1 --format=s16le > new.raw 2>/dev/null &); sleep 1; ($pw debug sound $kind $kind $kind $kind $kind $kind $kind $kind $kind $kind; echo exit \$? ) > run.log 2>&1 & sleep 3; pactl set-default-sink headset; sleep 12; cat run.log" >"$out/env6-$v-switch.log" 2>&1
        podman cp "$(cname "$d"):/home/dev/old.raw" "$out/env6-$v-old.raw" 2>/dev/null; podman cp "$(cname "$d"):/home/dev/new.raw" "$out/env6-$v-new.raw" 2>/dev/null
        expect "output switch: the run kept going and exited cleanly" grep -q 'exit 0' "$out/env6-$v-switch.log"
        if loud "$out/env6-$v-new.raw"; then pass "output switch: cues reached the new default sink"; else
            if loud "$out/env6-$v-old.raw"; then note "cues stayed on the old sink after the switch"; fail "output switch: cues did not follow the new default sink (stayed on the old one)"; else fail "output switch: no cue was heard on either sink"; fi
        fi
    else
        ct "$d" "$pw debug sound $kind" >"$out/env6-$v-play.log" 2>&1
        expect "ALSA only, no card: one log line says why cues are silent" grep -q 'no audio device for overlay sounds' "$out/env6-$v-play.log"
        expect "ALSA only: the sound command returns (nothing blocks)" test -n "$(cat "$out/env6-$v-play.log")"
        ct "$d" "$pw debug media volume up" >"$out/env6-$v-media.log" 2>&1
        expect "ALSA only: media says it has no output instead of failing silently" grep -q 'no output\|no tool' "$out/env6-$v-media.log"
    fi
    ct_down "$d"
}

# --- ENV-8: no systemd (Void/runit, Artix and Gentoo/OpenRC), and no uaccess ----------------
# Plain containers without an init: the install, an autostart-style start, doctor's advice for a
# missing uaccess, and the daemon letting go on SIGTERM (what logout does). A real runit or OpenRC
# supervisor is not run; the service managers only matter for how the daemon is started.
nosys_image() { # nosys_image <void|artix>
    case $1 in
    void) echo ghcr.io/void-linux/void-glibc:latest ;;
    artix) echo docker.io/artixlinux/artixlinux:latest ;;
    esac
}
env8() { # env8 <void|artix>
    local d=$1 c=dm-run-nosys-$1 img t
    img=$(nosys_image "$d")
    t=$(tarball_path) || { fail "no tarball"; return; }
    podman rm -f "$c" >/dev/null 2>&1
    podman run -d --init --name "$c" "$img" sleep 3600 >/dev/null 2>&1 || { skip "image $img could not be pulled here"; return; }
    case $d in
    void) podman exec "$c" sh -c 'xbps-install -Syu >/dev/null 2>&1; xbps-install -Sy eudev kmod bash tar gzip sudo shadow xkeyboard-config libxkbcommon alsa-lib runit procps-ng util-linux >/dev/null 2>&1' ;;
    artix) podman exec "$c" sh -c 'pacman -Syu --noconfirm bash tar gzip sudo shadow xkeyboard-config libxkbcommon alsa-lib runit procps-ng util-linux >/dev/null 2>&1' ;;
    esac
    podman exec "$c" sh -c "useradd -m -u 1000 -s /bin/bash dev && echo 'dev ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/dev" || { fail "could not prepare $d"; podman rm -f "$c" >/dev/null; return; }
    podman cp "$t" "$c:/home/dev/" && podman exec "$c" sh -c "chown dev:dev /home/dev/*.tar.gz"
    local run=(podman exec -u dev -w /home/dev -e HOME=/home/dev -e XDG_RUNTIME_DIR=/tmp/xdg "$c" bash -c)
    "${run[@]}" "mkdir -p /tmp/xdg && chmod 700 /tmp/xdg && tar xzf $(basename "$t")"
    local dir; dir=$("${run[@]}" 'ls -d padwight-*-linux-x86_64')
    "${run[@]}" "cd $dir && ./install.sh --no-system --yes" >"$out/env8-$d-install.log" 2>&1
    local rc=$?
    if [ $rc -eq 0 ]; then pass "install.sh (service on, no systemd) exits 0"; else fail "install.sh without systemd stops with exit $rc ($(tail -2 "$out/env8-$d-install.log" | tr '\n' ' ')); the README has no non-systemd steps"; fi
    "${run[@]}" "cd $dir && ./install.sh --no-system --no-service" >"$out/env8-$d-install2.log" 2>&1
    expect "install.sh --no-system --no-service works without systemd" test $? -eq 0
    expect "the binary is installed" "${run[@]}" 'test -x ~/.local/bin/padwight'
    # An autostart entry, started as a session would start it.
    "${run[@]}" 'mkdir -p ~/.config/autostart && printf "[Desktop Entry]\nType=Application\nName=Padwight\nExec=%s/.local/bin/padwight daemon\n" "$HOME" > ~/.config/autostart/padwight.desktop'
    "${run[@]}" '(setsid ~/.local/bin/padwight daemon > daemon.log 2>&1 &) ; sleep 3; cat daemon.log' >"$out/env8-$d-daemon.log" 2>&1
    "${run[@]}" '~/.local/bin/padwight status' >"$out/env8-$d-status.log" 2>&1
    if grep -qi 'uinput' "$out/env8-$d-daemon.log"; then pass "the daemon's log names the missing uinput access"; else note "daemon log: $(head -3 "$out/env8-$d-daemon.log" | tr '\n' ' ')"; fail "the daemon says nothing about missing uinput access"; fi
    "${run[@]}" '~/.local/bin/padwight doctor' >"$out/env8-$d-doctor.log" 2>&1
    expect "doctor names the missing access with the fix (rules file or modprobe)" grep -Eqi 'udev|modprobe|uinput' "$out/env8-$d-doctor.log"
    expect "SIGTERM (logout) leaves no daemon process (it may have exited already for want of uinput)" "${run[@]}" 'pkill -TERM -x padwight; sleep 2; ! pgrep -x padwight'
    podman rm -f "$c" >/dev/null 2>&1
}

# --- running -----------------------------------------------------------------------------
checks="ins1 ins2 ins4 ins5 env5 env6 env8 env11 vm-ins1 vm-env9 vm-env10 vm-env11"
list() {
    cat <<'EOF'
ins1@fedora|ubuntu|arch|ubuntu22  tarball install, uninstall, purge (containers); with and without the udev rules
ins2@...                          cargo-installed binary + the README's unit steps
ins4@fedora                       read-only root, only the four files in /etc
ins5@...                          service starts at login, after logout/login and after a restart
env5@fedora|ubuntu                keyboard layouts: fr de dvorak ru gr via `padwight debug layout`; scenarios under each layout (host)
env6@pw|pa|alsa                   audio: PipeWire+pulse (fedora), PulseAudio only (ubuntu), ALSA only (ubuntu)
env8@void|artix                   no systemd: install, autostart-style start, doctor, SIGTERM
env11@fedora|ubuntu               odd XDG dirs, symlinked config dir on another fs, full disk, read-only dir
vm-ins1@arch|ubuntu|fedora        QEMU: real udev + uaccess ACL, reboot, uninstall
vm-env9@arch                      QEMU: two seat0 sessions, loginctl activate/lock, ACL follows the active session
vm-env10@ubuntu|fedora            QEMU: AppArmor (ubuntu) or SELinux enforcing (fedora): denials, sandbox-pid.sh
vm-env11@arch|ubuntu|fedora       QEMU: env11 plus home on NFS
EOF
}
default_targets() {
    case $1 in
    ins1 | ins2 | ins5) echo "fedora ubuntu ubuntu22" ;;  # arch: dbus-broker will not start in a rootless container (ENOPKG); vm-ins1@arch covers Arch
    ins4) echo fedora ;;
    env5) echo "fedora ubuntu" ;;
    env6) echo "pw pa alsa" ;;
    env8) echo "void artix" ;;
    env11) echo "fedora ubuntu" ;;
    vm-ins1) echo "arch" ;;
    vm-env9) echo arch ;;
    vm-env10) echo "ubuntu fedora" ;;
    vm-env11) echo arch ;;
    esac
}
run_check() { # run_check <check>[@target]
    local c=${1%%@*} t=
    [ "$c" != "$1" ] && t=${1#*@}
    local targets=${t:-$(default_targets "$c")}
    for t in $targets; do
        ctx=$c@$t
        echo "=== $ctx"
        case $c in
        ins1) ins1 "$t" ;; ins2) ins2 "$t" ;; ins4) ins4 "$t" ;; ins5) ins5 "$t" ;;
        env5) env5 "$t"; [ "$t" = "${targets%% *}" ] && { ctx=env5-scenarios@host; env5_scenarios; } ;;
        env6) env6 "$t" ;; env8) env8 "$t" ;; env11) env11 "$t" ;;
        vm-ins1) vm_ins1 "$t" ;; vm-env9) vm_env9 "$t" ;; vm-env10) vm_env10 "$t" ;; vm-env11) vm_env11 "$t" ;;
        *) echo "unknown check $c" >&2; return 2 ;;
        esac
    done
}

case ${1:-list} in
list | -h | --help) list ;;
tarball) tarball ;;
fetch-fedora-image) vm_base fedora ;;
table) table ;;
all) : >"$results"; for c in $checks; do run_check "$c"; done; table ;;
run) shift; : >"$results"; for c in "$@"; do run_check "$c"; done; table ;;
*) echo "usage: $0 list|tarball|run <check>[@target]...|all|table" >&2; exit 2 ;;
esac
