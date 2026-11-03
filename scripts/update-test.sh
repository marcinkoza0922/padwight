#!/bin/bash
# Runs `padwight update`, the daemon's daily update check and `dist/get.sh` against a local fake
# release server (scripts/fake-release-server.py) in throwaway home directories. qa-plan INS-12
# and INS-13, except what needs a real published release, a systemd user session or a controller.
#
#   scripts/update-test.sh [update] [getsh] [daemon]      default: all three (daemon takes ~2.5 minutes)
#
# Needs a debug build of padwight (PADWIGHT_BIN, default target/debug/padwight), curl, wget, openssl,
# python3, util-linux `setsid`. Nothing outside the temp folder is touched: HOME and the
# XDG folders are the folder's, `systemctl`, `sudo`, `modprobe` and `udevadm` are shims that only log
# their arguments (so no real service is restarted and nothing is installed in /etc), and the only
# network is 127.0.0.1: https_proxy and CURL_CA_BUNDLE (and a wget shim) send github.com to the fake server.
# The "new release" holds a stub `padwight` (a shell script), so the update replaces a real binary
# with something recognisable. No padwight code is changed for the test.
set -u
cd "$(dirname "$0")/.."
root=$PWD
BIN=${PADWIGHT_BIN:-$root/target/debug/padwight}
SYSPATH=/usr/bin:/bin
SETSID=$(command -v setsid)
[ -x "$BIN" ] || { echo "build padwight first (cargo build), or set PADWIGHT_BIN" >&2; exit 2; }
for tool in openssl curl wget python3 sha256sum tar setsid; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 2; }
done
groups=${*:-update getsh daemon}

T=$(mktemp -d)
# Unix socket paths are limited to 107 bytes, so the runtime folders live under a short name.
RUNS=$(mktemp -d /tmp/pwr.XXXXXX)
# The debug build is hundreds of MB; a stripped copy, hard-linked into each home, works the same.
strip -o "$T/padwight-bin" "$BIN" || cp "$BIN" "$T/padwight-bin"
BIN=$T/padwight-bin
SERVER_PIDS=()
DAEMON_PIDS=()
cleanup() {
    for pid in "${DAEMON_PIDS[@]}" "${SERVER_PIDS[@]}"; do kill "$pid" 2>/dev/null; done
    wait 2>/dev/null
    rm -rf "$T" "$RUNS"
}
trap cleanup EXIT

pass=0 failed=0
check() { # check "description" command args...
    local what=$1; shift
    if "$@"; then pass=$((pass + 1)); echo "ok: $what"; else failed=$((failed + 1)); echo "FAIL: $what"; fi
}
has() { grep -q -- "$2" "$1" 2>/dev/null; }          # has file text
lacks() { ! grep -q -- "$2" "$1" 2>/dev/null; }
sameas() { cmp -s "$1" "$2"; }

# ---- the world a test runs in ---------------------------------------------------------------

new_world() { # new_world name: a fresh home with shims; sets W
    W=$T/$1
    mkdir -p "$W/home" "$W/shims" "$W/tmp" "$W/cache" "$W/data" "$W/state" "$W/config"
    mkdir -p -m 700 "$RUNS/$1"
    : > "$W/calls.log"
    for tool in systemctl sudo modprobe udevadm; do
        printf '#!/bin/sh\necho "%s $*" >> "$CALLS"\nexit 0\n' "$tool" > "$W/shims/$tool"
        chmod +x "$W/shims/$tool"
    done
    printf '#!/bin/sh\nif [ -n "${FAKE_ARCH:-}" ] && [ "${1:-}" = -m ]; then echo "$FAKE_ARCH"; else exec /usr/bin/uname "$@"; fi\n' > "$W/shims/uname"
    chmod +x "$W/shims/uname"
    # wget (2.x here) ignores WGETRC for trust, so the shim adds the test CA on the command line.
    printf '#!/bin/sh\nexec /usr/bin/wget --ca-certificate="$CURL_CA_BUNDLE" "$@"\n' > "$W/shims/wget"
    chmod +x "$W/shims/wget"
}

run_dir() { echo "$RUNS/$(basename "$W")"; }

wenv() { # wenv [VAR=value...] command args...: runs in the world W with a clean environment
    env -i PATH="${WPATH:-$W/shims:$SYSPATH}" HOME="$W/home" XDG_CONFIG_HOME="$W/config" XDG_CACHE_HOME="$W/cache" \
        XDG_DATA_HOME="$W/data" XDG_STATE_HOME="$W/state" XDG_RUNTIME_DIR="$(run_dir)" TMPDIR="$W/tmp" LANG=C \
        https_proxy="$PROXY" CURL_CA_BUNDLE="$CA" CALLS="$W/calls.log" PADWIGHT_VERSION="${PADWIGHT_VERSION:-}" FAKE_ARCH="${FAKE_ARCH:-}" "$@"
}

install_old() { # the user's installed copy: the real build, in ~/.local/bin, with a service file
    mkdir -p "$W/home/.local/bin" "$W/config/systemd/user"
    ln "$BIN" "$W/home/.local/bin/padwight"
    printf '[Service]\nExecStart=/old/path daemon\n' > "$W/config/systemd/user/padwight.service"
    ln "$BIN" "$W/old-binary"
}

installed() { echo "$W/home/.local/bin/padwight"; }
is_stub() { has "$(installed)" "stub $1"; }
is_old() { sameas "$(installed)" "$W/old-binary"; }
sudo_calls() { grep -c '^sudo' "$W/calls.log"; }
no_sudo() { [ "$(sudo_calls)" -eq 0 ]; }

# ---- releases and the server ----------------------------------------------------------------

build_release() { # build_release version...: stub tarballs in $T/rel
    local v
    for v in "$@"; do
        printf '#!/bin/sh\ncase "$1" in doctor) echo "stub %s doctor $*" ;; esac\n' "$v" > "$T/stub-$v"
        chmod +x "$T/stub-$v"
        PADWIGHT_PACKAGE_BINARY=$T/stub-$v PADWIGHT_PACKAGE_VERSION=$v PADWIGHT_PACKAGE_OUT=$T/rel \
            scripts/package.sh >/dev/null 2>&1 || { echo "package.sh failed for $v" >&2; exit 2; }
    done
}

serve() { # serve name server-args...: starts a server, sets PROXY, CA, SRV (its state folder)
    local name=$1; shift
    SRV=$T/srv-$name
    python3 -I scripts/fake-release-server.py --state "$SRV" "$@" &
    SERVER_PIDS+=($!)
    local i
    for i in $(seq 100); do [ -e "$SRV/ready.json" ] && break; sleep 0.1; done
    [ -e "$SRV/ready.json" ] || { echo "the fake server didn't start" >&2; exit 2; }
    PROXY=$(python3 -I -c 'import json,sys;print(json.load(open(sys.argv[1]))["proxy"])' "$SRV/ready.json")
    CA=$SRV/ca.pem
}

requests() { grep -c "$1" "$SRV/requests.log" 2>/dev/null; }

# ---- padwight update ------------------------------------------------------------------------

test_update() {
    echo "== padwight update =="
    build_release 0.2.0
    local out

    serve ok --dir "$T/rel" --latest 0.2.0
    new_world u-check; install_old
    out=$(wenv "$(installed)" update --check 2>&1); rc=$?
    check "update --check names the new version (exit 0)" test $rc -eq 0 -a "$(echo "$out" | grep -c 'padwight 0.2.0 is available')" -eq 1
    check "update --check installs nothing" is_old

    new_world u-same; install_old
    serve same --dir "$T/rel" --latest 0.1.0
    out=$(wenv "$(installed)" update --check 2>&1)
    check "update --check at the current version says so" test "$(echo "$out" | grep -c 'is the latest version')" -eq 1

    new_world u-none; install_old
    serve none --dir "$T/rel"
    out=$(wenv "$(installed)" update --check 2>&1); rc=$?
    check "no release yet: a clear error, exit 1" test $rc -eq 1 -a "$(echo "$out" | grep -c 'no release yet')" -eq 1

    new_world u-down; install_old
    out=$(PROXY=http://127.0.0.1:1 wenv "$(installed)" update --check 2>&1); rc=$?
    check "server down: couldn't reach GitHub, exit 1" test $rc -eq 1 -a "$(echo "$out" | grep -c "couldn't reach GitHub")" -eq 1

    new_world u-other; install_old
    serve ok2 --dir "$T/rel" --latest 0.2.0
    mkdir -p "$W/src"; ln "$BIN" "$W/src/padwight"
    out=$(wenv "$W/src/padwight" update --check 2>&1); rc=$?
    check "a source build: --check gives advice" test $rc -eq 0 -a "$(echo "$out" | grep -c 'rebuild it from source')" -eq 1
    out=$(wenv "$W/src/padwight" update 2>&1); rc=$?
    check "a source build: update refuses with advice, exit 1" test $rc -eq 1 -a "$(echo "$out" | grep -c "isn't replaced")" -eq 1
    check "a source build: nothing downloaded" test "$(requests download)" -eq 0

    # The update proper.
    new_world u-full; install_old
    serve full --dir "$T/rel" --latest 0.2.0
    local before after
    before=$(cd "$W" && find home data config -type f | sort)
    out=$(wenv "$(installed)" update 2>&1); rc=$?
    echo "$out" | sed 's/^/    | /'
    check "update exits 0 and says updated to 0.2.0" test $rc -eq 0 -a "$(echo "$out" | grep -c 'updated to 0.2.0')" -eq 1
    check "the installed binary is the new one" is_stub 0.2.0
    check "never asks for sudo" no_sudo
    check "the service is restarted last (systemctl --user try-restart padwight)" has "$W/calls.log" 'systemctl --user try-restart padwight'
    check "doctor --files is run from the new binary at the end" test "$(echo "$out" | grep -c 'stub 0.2.0 doctor doctor --files')" -eq 1
    check "the service file now points at the installed binary" has "$W/config/systemd/user/padwight.service" "ExecStart=$W/home/.local/bin/padwight daemon"
    after=$(cd "$W" && find home data config -type f | sort)
    check "only the user files were added (binary, uninstaller, icon, menu entry)" test "$(diff <(echo "$before") <(echo "$after") | tee "$W/diff" | grep -c '^[<>]')" -eq 3
    check "the scratch folder is gone" test -z "$(ls "$W/cache/padwight" 2>/dev/null)"

    # Failures leave the old version installed and the service alone.
    local mode want
    for mode in checksum truncated missing; do
        new_world "u-$mode"; install_old
        case $mode in
            checksum) want="the download doesn.t match its checksum"
                mkdir -p "$T/rel-bad"; cp "$T/rel"/*0.2.0*.tar.gz* "$T/rel-bad/"
                echo "0000000000000000000000000000000000000000000000000000000000000000  padwight-0.2.0-linux-x86_64.tar.gz" > "$T/rel-bad/padwight-0.2.0-linux-x86_64.tar.gz.sha256"
                serve "$mode" --dir "$T/rel-bad" --latest 0.2.0 ;;
            truncated) want="download failed"; serve "$mode" --dir "$T/rel" --latest 0.2.0 --fault truncate:x86_64.tar.gz ;;
            missing) want="download failed"; serve "$mode" --dir "$T/rel" --latest 0.2.0 --fault status:404:/download/ ;;
        esac
        out=$(wenv "$(installed)" update 2>&1); rc=$?
        echo "$out" | sed 's/^/    | /'
        check "$mode: update fails (exit 1) with the reason: $want" test $rc -ne 0 -a "$(echo "$out" | grep -c "$want")" -eq 1
        check "$mode: the old binary is still installed" is_old
        check "$mode: the service is not restarted" lacks "$W/calls.log" try-restart
        check "$mode: no sudo" no_sudo
        check "$mode: nothing left in the scratch folder" test -z "$(ls "$W/cache/padwight" 2>/dev/null)"
    done
}

# ---- dist/get.sh ----------------------------------------------------------------------------

stdin_pipe() { # no terminal at all: the script arrives through a pipe, like `ssh -T host 'curl ... | sh'`
    cat dist/get.sh | wenv "$@" "$SETSID" -w sh -s -- "${GET_ARGS[@]}"
}

# Bin dir with just the tools get.sh and install.sh use, for the missing-tool cases.
tool_path() { # tool_path dir tools-to-leave-out...
    local dir=$1; shift
    mkdir -p "$dir"
    local tool skip
    for tool in sh mktemp sed tr head cat id install mkdir rm basename dirname gzip tar sha256sum curl cp chmod grep date sleep; do
        for skip in "$@"; do [ "$skip" = "$tool" ] && continue 2; done
        ln -sf "$(command -v "$tool")" "$dir/$tool"
    done
    cp "$W/shims"/* "$dir/"
    for skip in "$@"; do [ "$skip" = wget ] && rm -f "$dir/wget"; done
}

test_getsh() {
    echo "== dist/get.sh =="
    build_release 0.1.0 0.2.0
    local out rc
    serve get --dir "$T/rel" --latest 0.2.0

    new_world g-notty
    GET_ARGS=()
    out=$(stdin_pipe 2>&1); rc=$?
    check "no terminal: installs, skips the system files and says how to add them" test $rc -eq 0 -a "$(echo "$out" | grep -c 'no terminal to ask about the system files, so skipping them (add --yes')" -eq 1
    check "no terminal: the latest version (0.2.0) is installed" is_stub 0.2.0
    check "no terminal: no sudo" no_sudo
    check "no terminal: the service was enabled and started" has "$W/calls.log" 'systemctl --user enable padwight'

    new_world g-yes
    GET_ARGS=(--yes)
    out=$(stdin_pipe 2>&1); rc=$?
    check "no terminal, --yes: installs the system files through sudo without asking" test $rc -eq 0 -a "$(sudo_calls)" -ge 4
    check "no terminal, --yes: the rules go to /etc/udev/rules.d and the module list to /etc/modules-load.d" test "$(grep -c '^sudo install -Dm644 .*/etc/\(udev/rules.d\|modules-load.d\)/' "$W/calls.log")" -eq 4

    new_world g-nosys
    GET_ARGS=(--no-system)
    out=$(stdin_pipe 2>&1); rc=$?
    check "no terminal, --no-system: installs, no sudo, no complaint about the terminal" test $rc -eq 0 -a "$(sudo_calls)" -eq 0 -a "$(echo "$out" | grep -c 'no terminal')" -eq 0
    check "--no-system: the binary is installed" is_stub 0.2.0

    new_world g-tty-no   # a terminal, the script run from a file: asks, answer no
    out=$(wenv python3 -I scripts/pty-run.py n 'Install them with sudo' -- sh $root/dist/get.sh 2>&1); rc=$?
    check "terminal, answer n: asks before the system files and skips them" test "$(echo "$out" | grep -c 'Install them with sudo')" -eq 1 -a "$(sudo_calls)" -eq 0
    check "terminal, answer n: the binary is installed" is_stub 0.2.0

    new_world g-tty-yes  # the script through a pipe, but a terminal exists: the question goes to it
    out=$(wenv python3 -I scripts/pty-run.py y 'Install them with sudo' -- sh -c "cat $root/dist/get.sh | sh" 2>&1); rc=$?
    check "piped in a terminal, answer y: asks, then uses sudo for the four files" test "$(echo "$out" | grep -c 'Install them with sudo')" -eq 1 -a "$(grep -c '^sudo install -Dm644' "$W/calls.log")" -eq 4

    new_world g-old
    GET_ARGS=(--no-system)
    out=$(PADWIGHT_VERSION=0.1.0 stdin_pipe 2>&1)
    check "PADWIGHT_VERSION=0.1.0 installs that release, not the latest" is_stub 0.1.0
    new_world g-oldv
    out=$(PADWIGHT_VERSION=v0.1.0 stdin_pipe 2>&1)
    check "PADWIGHT_VERSION=v0.1.0 (with the v) works too" is_stub 0.1.0

    local bad
    for bad in "1.0;touch /tmp/pwn" "../0.2.0" "abc" "0.2.0 x" '$(id)'; do
        new_world g-bad
        out=$(PADWIGHT_VERSION=$bad stdin_pipe 2>&1); rc=$?
        check "PADWIGHT_VERSION='$bad' is refused before any download" test $rc -ne 0 -a "$(echo "$out" | grep -c 'not a padwight version')" -eq 1 -a ! -e "$(installed)"
    done
    new_world g-9
    out=$(PADWIGHT_VERSION=9.9.9 stdin_pipe 2>&1); rc=$?
    check "PADWIGHT_VERSION=9.9.9 (no such release): stops with a download error, nothing installed" test $rc -ne 0 -a "$(echo "$out" | grep -c "couldn't download")" -eq 1 -a ! -e "$(installed)"

    # Tools and architecture.
    new_world g-wget; tool_path "$W/bin-nocurl" curl
    out=$(WPATH=$W/bin-nocurl stdin_pipe 2>&1); rc=$?
    check "only wget: finds the latest version and installs it" test $rc -eq 0 -a "$(grep -c "stub 0.2.0" "$(installed)")" -eq 1
    new_world g-nodl; tool_path "$W/bin" curl wget
    out=$(WPATH=$W/bin stdin_pipe 2>&1); rc=$?
    check "neither curl nor wget: says so, installs nothing" test $rc -ne 0 -a "$(echo "$out" | grep -c 'need curl or wget')" -eq 1 -a ! -e "$(installed)"
    new_world g-nosha; tool_path "$W/bin" sha256sum
    out=$(WPATH=$W/bin stdin_pipe 2>&1); rc=$?
    check "no sha256sum: says so, installs nothing" test $rc -ne 0 -a "$(echo "$out" | grep -c 'need sha256sum')" -eq 1 -a ! -e "$(installed)"
    new_world g-notar; tool_path "$W/bin" tar
    out=$(WPATH=$W/bin stdin_pipe 2>&1); rc=$?
    check "no tar: says so, installs nothing" test $rc -ne 0 -a "$(echo "$out" | grep -c 'need tar')" -eq 1 -a ! -e "$(installed)"
    new_world g-arm
    out=$(FAKE_ARCH=aarch64 stdin_pipe 2>&1); rc=$?
    check "aarch64: says there is no release for it, installs nothing" test $rc -ne 0 -a "$(echo "$out" | grep -c 'no padwight release for aarch64')" -eq 1 -a ! -e "$(installed)"

    # Failures.
    new_world g-sum
    mkdir -p "$T/rel-bad"; cp "$T/rel"/*0.2.0*.tar.gz* "$T/rel-bad/"
    echo "0000000000000000000000000000000000000000000000000000000000000000  padwight-0.2.0-linux-x86_64.tar.gz" > "$T/rel-bad/padwight-0.2.0-linux-x86_64.tar.gz.sha256"
    serve getbad --dir "$T/rel-bad" --latest 0.2.0
    out=$(stdin_pipe 2>&1); rc=$?
    check "bad checksum: refuses, installs nothing" test $rc -ne 0 -a "$(echo "$out" | grep -c "doesn't match its checksum")" -eq 1 -a ! -e "$(installed)"
    new_world g-cut
    serve getcut --dir "$T/rel" --latest 0.2.0 --fault truncate:x86_64.tar.gz
    out=$(stdin_pipe 2>&1); rc=$?
    check "cut download: stops, installs nothing" test $rc -ne 0 -a ! -e "$(installed)"
    new_world g-down
    out=$(PROXY=http://127.0.0.1:1 stdin_pipe 2>&1); rc=$?
    check "server down: couldn't find the latest release, installs nothing" test $rc -ne 0 -a "$(echo "$out" | grep -c "couldn't find the latest")" -eq 1 -a ! -e "$(installed)"
    new_world g-norel
    serve getnone --dir "$T/rel"
    out=$(stdin_pipe 2>&1); rc=$?
    check "no release published yet: says it found no release, installs nothing" test $rc -ne 0 -a "$(echo "$out" | grep -c "find the latest")" -eq 1 -a ! -e "$(installed)"
    echo "    (message with no release: $(echo "$out" | tail -1))"
}

# ---- the daemon's daily check ---------------------------------------------------------------

status_json() { # status_json socket-dir: the daemon's Status reply, one line
    python3 -I -c '
import socket, sys
s = socket.socket(socket.AF_UNIX)
s.settimeout(5)
s.connect(sys.argv[1] + "/padwight.sock")
s.sendall(sys.argv[2].encode() + b"\n")
print(s.makefile().readline().strip())' "$(run_dir)" "$1"
}
update_of() { python3 -I -c 'import json,sys;print(json.dumps(json.loads(sys.stdin.read())["Status"].get("update")))'; }

start_daemon() { # start_daemon binary: in the world W
    local line=${2:-}
    wenv "$1" daemon --debug --fake-clock > "$W/daemon.log" 2>&1 &
    DAEMON_PIDS+=($!)
    local i
    for i in $(seq 100); do [ -S "$(run_dir)/padwight.sock" ] && break; sleep 0.1; done
    [ -S "$(run_dir)/padwight.sock" ] || { echo "the daemon didn't start:"; cat "$W/daemon.log"; return 1; }
    # The daemon wrote its default config on its first start; a setting is added to it and re-read.
    if [ -n "$line" ]; then
        { echo "$line"; cat "$W/config/padwight/config.toml"; } > "$W/config.new" && mv "$W/config.new" "$W/config/padwight/config.toml"
        status_json '"Reload"' > /dev/null
    fi
}

test_daemon() {
    echo "== the daily check (the daemon looks 120 s after it starts) =="
    build_release 0.2.0
    # Three daemons side by side, so the wait is shared.
    serve don --dir "$T/rel" --latest 0.2.0; local srv_on=$SRV
    new_world d-on; install_old; local w_on=$W
    start_daemon "$(installed)" || return
    SRV=$srv_on

    serve doff --dir "$T/rel" --latest 0.2.0; local srv_off=$SRV
    new_world d-off; install_old; local w_off=$W
    start_daemon "$(installed)" 'check_updates = false' || return

    serve dsrc --dir "$T/rel" --latest 0.2.0; local srv_src=$SRV
    new_world d-src; install_old; local w_src=$W
    mkdir -p "$W/src"; ln "$BIN" "$W/src/padwight"
    start_daemon "$W/src/padwight" || return

    echo "waiting 130 s for the first check..."
    sleep 130

    W=$w_on; SRV=$srv_on
    check "check on: the log says a newer version is available" has "$W/daemon.log" 'padwight 0.2.0 is available'
    check "check on: Status carries the update, which this copy can install (the settings banner and tray entry read this)" \
        test "$(status_json '"Status"' | update_of)" = '{"version": "0.2.0", "self_updating": true, "advice": "run `padwight update`"}'
    check "check on: exactly one request for the latest page" test "$(requests 'releases/latest')" -eq 1

    W=$w_off; SRV=$srv_off
    check "check off: no request reached the server at all" test ! -s "$SRV/requests.log"
    check "check off: Status has no update" test "$(status_json '"Status"' | update_of)" = null

    W=$w_src; SRV=$srv_src
    check "source build: Status says the update can't be installed here, with advice" \
        test "$(status_json '"Status"' | update_of | grep -c '"self_updating": false.*rebuild it from source')" -eq 1
    check "source build: Update (tray / settings window) is refused" test "$(status_json '"InstallUpdate"' | grep -c 'not updated')" -eq 1

    # Update from the tray or the settings window is the InstallUpdate request.
    W=$w_on; SRV=$srv_on
    check "InstallUpdate is accepted" test "$(status_json '"InstallUpdate"')" = '"Ok"'
    local i
    for i in $(seq 100); do is_stub 0.2.0 && break; sleep 0.2; done
    check "Update from the daemon replaces the binary" is_stub 0.2.0
    check "Update from the daemon asks the service manager to restart" has "$W/calls.log" 'systemctl --user try-restart padwight'
    check "Update from the daemon never uses sudo" no_sudo
    check "the daemon keeps running until the service manager restarts it (shim here)" kill -0 "${DAEMON_PIDS[0]}"
}

for g in $groups; do
    case $g in
        update) test_update ;;
        getsh) test_getsh ;;
        daemon) test_daemon ;;
        *) echo "unknown group: $g" >&2; exit 2 ;;
    esac
done
echo
echo "$pass passed, $failed failed"
[ "$failed" -eq 0 ]
