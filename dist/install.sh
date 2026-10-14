#!/bin/sh
# Installs padwight from the release tarball this script sits in.
#
#   ./install.sh                install for your user (~/.local), then the system files with sudo
#   ./install.sh --no-system    skip the system files (udev rules, uinput module list)
#   ./install.sh --no-service   don't enable the background service
#   ./install.sh --yes          don't ask before using sudo
#   ./install.sh --uninstall    remove everything this script installs
#
# Your own files go in ~/.local and ~/.config/systemd/user. Only the udev rules and the module list
# need root, and only for the file copies: /etc/udev/rules.d and /etc/modules-load.d.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
bin_dir=${XDG_BIN_HOME:-$HOME/.local/bin}
data_dir=${XDG_DATA_HOME:-$HOME/.local/share}
config_dir=${XDG_CONFIG_HOME:-$HOME/.config}
app_id=io.github.marcinkoza0922.Padwight
rules="70-padwight-motion.rules 70-padwight-uinput.rules"

system=1 service=1 yes=0 uninstall=0
for arg in "$@"; do
    case $arg in
        --no-system) system=0 ;;
        --no-service) service=0 ;;
        --yes|-y) yes=1 ;;
        --uninstall) uninstall=1 ;;
        -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

# Runs a command as root: directly if we are root, else through sudo.
as_root() {
    if [ "$(id -u)" -eq 0 ]; then "$@"; else sudo "$@"; fi
}

# Says what the system files are for and asks before using sudo. Returns 1 for no.
confirm_system() {
    [ "$yes" -eq 1 ] && return 0
    cat <<'TEXT'

padwight needs three small system files, which take root to copy:
  /etc/udev/rules.d/70-padwight-uinput.rules   lets you create the virtual gamepad, keyboard and mouse
                                               games and your mappings go through (/dev/uinput)
  /etc/udev/rules.d/70-padwight-motion.rules   lets you read PlayStation and Switch controllers' gyro
  /etc/modules-load.d/padwight-uinput.conf     loads the uinput kernel module at boot
The rules only give access to you, the user at the machine. `padwight doctor --explain` says more.
TEXT
    printf 'Install them with sudo? [Y/n] '
    read -r answer || answer=n
    case $answer in n|N|no|No) return 1 ;; *) return 0 ;; esac
}

install_user() {
    mkdir -p "$bin_dir" "$data_dir/applications" "$data_dir/icons/hicolor/scalable/apps"
    install -m755 "$here/padwight" "$bin_dir/padwight"
    install -m644 "$here/share/$app_id.svg" "$data_dir/icons/hicolor/scalable/apps/$app_id.svg"
    sed "s|^Exec=.*|Exec=$bin_dir/padwight|" "$here/share/$app_id.desktop" > "$data_dir/applications/$app_id.desktop"
    echo "installed $bin_dir/padwight, its icon and its menu entry"
    case ":$PATH:" in *":$bin_dir:"*) ;; *) echo "note: $bin_dir isn't in your PATH, so add it to run \`padwight\` from a terminal" ;; esac
}

install_service() {
    mkdir -p "$config_dir/systemd/user"
    sed "s|^ExecStart=.*|ExecStart=$bin_dir/padwight daemon|" "$here/share/padwight.service" > "$config_dir/systemd/user/padwight.service"
    systemctl --user daemon-reload
    systemctl --user enable padwight
    echo "installed the background service, which starts at login"
}

install_system() {
    for rule in $rules; do
        as_root install -Dm644 "$here/share/$rule" "/etc/udev/rules.d/$rule"
    done
    as_root install -Dm644 "$here/share/padwight-uinput.conf" /etc/modules-load.d/padwight-uinput.conf
    as_root modprobe uinput || echo "note: couldn't load the uinput module now; it will load at the next boot"
    as_root udevadm control --reload
    as_root udevadm trigger
    echo "installed the udev rules and the module list"
}

uninstall() {
    systemctl --user disable --now padwight 2>/dev/null || true
    rm -f "$bin_dir/padwight" "$config_dir/systemd/user/padwight.service" \
        "$data_dir/applications/$app_id.desktop" "$data_dir/icons/hicolor/scalable/apps/$app_id.svg"
    systemctl --user daemon-reload 2>/dev/null || true
    echo "removed the user files. Your settings in $config_dir/padwight are kept."
    if [ "$system" -eq 1 ] && [ -e /etc/modules-load.d/padwight-uinput.conf -o -e /etc/udev/rules.d/70-padwight-uinput.rules ] && confirm_remove; then
        for rule in $rules; do as_root rm -f "/etc/udev/rules.d/$rule"; done
        as_root rm -f /etc/modules-load.d/padwight-uinput.conf
        as_root udevadm control --reload
        echo "removed the udev rules and the module list"
    fi
}

confirm_remove() {
    [ "$yes" -eq 1 ] && return 0
    printf 'Remove the system files from /etc too (needs sudo)? [Y/n] '
    read -r answer || answer=n
    case $answer in n|N|no|No) return 1 ;; *) return 0 ;; esac
}

if [ "$uninstall" -eq 1 ]; then
    uninstall
    exit 0
fi

[ -x "$here/padwight" ] || { echo "run this from the unpacked release folder: padwight is missing next to it" >&2; exit 1; }
install_user
if [ "$system" -eq 1 ]; then
    if confirm_system; then install_system; else echo "skipped the system files; \`padwight doctor\` prints the commands for them"; fi
fi
if [ "$service" -eq 1 ]; then
    install_service
    systemctl --user restart padwight || echo "note: the service didn't start; \`padwight doctor\` says why"
fi

echo
"$bin_dir/padwight" doctor || true
