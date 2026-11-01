#!/bin/sh
# Downloads the latest padwight release, checks its checksum and runs its installer.
#
#   curl -fsSL https://github.com/marcinkoza0922/padwight/releases/latest/download/get.sh | sh
#   curl -fsSL .../get.sh | sh -s -- --no-system     options go to install.sh (see its --help)
#   PADWIGHT_VERSION=0.1.0 sh get.sh                 install a specific version
#
# This only fetches; the installer inside the tarball does the work, so you can also download the
# tarball yourself, read install.sh and run it by hand. Everything is inside main, which runs on the
# last line, so a download that is cut short can't run half a script.
set -eu

main() {
    repo=marcinkoza0922/padwight
    arch=$(uname -m)
    [ "$(uname -s)" = Linux ] || { echo "padwight runs on Linux only" >&2; exit 1; }
    [ "$arch" = x86_64 ] || { echo "no padwight release for $arch yet: build it from source (see the README)" >&2; exit 1; }

    if command -v curl >/dev/null 2>&1; then
        fetch() { curl -fsSL -o "$2" "$1"; }
        latest() { curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$repo/releases/latest"; }
    elif command -v wget >/dev/null 2>&1; then
        fetch() { wget -q -O "$2" "$1"; }
        latest() { wget -q --max-redirect=0 -S -O /dev/null "https://github.com/$repo/releases/latest" 2>&1 | sed -n 's/^ *[Ll]ocation: *//p' | tr -d '\r' | head -1; }
    else
        echo "need curl or wget to download padwight" >&2; exit 1
    fi
    command -v sha256sum >/dev/null 2>&1 || { echo "need sha256sum to check the download" >&2; exit 1; }
    command -v tar >/dev/null 2>&1 || { echo "need tar to unpack the download" >&2; exit 1; }

    version=${PADWIGHT_VERSION:-}
    if [ -z "$version" ]; then
        tag=$(latest | sed 's|.*/||')
        version=${tag#v}
        [ -n "$version" ] && [ "$version" != latest ] || { echo "couldn't find the latest padwight release" >&2; exit 1; }
    fi
    version=${version#v}
    name=padwight-$version-linux-$arch
    base=https://github.com/$repo/releases/download/v$version

    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT INT TERM
    echo "downloading padwight $version"
    fetch "$base/$name.tar.gz" "$work/$name.tar.gz" || { echo "couldn't download $base/$name.tar.gz" >&2; exit 1; }
    fetch "$base/$name.tar.gz.sha256" "$work/$name.tar.gz.sha256" || { echo "couldn't download the checksum" >&2; exit 1; }
    (cd "$work" && sha256sum -c "$name.tar.gz.sha256" >/dev/null) || { echo "the download doesn't match its checksum: not installing" >&2; exit 1; }
    tar -C "$work" -xzf "$work/$name.tar.gz"

    # The installer asks questions (sudo for the system files). When this script itself came through
    # a pipe, stdin is the script, so questions go to the terminal, or the system files are skipped.
    if [ ! -t 0 ]; then
        if ( : </dev/tty ) 2>/dev/null; then
            "$work/$name/install.sh" "$@" </dev/tty
        else
            case " $* " in
                *" --yes "*|*" -y "*|*" --no-system "*) "$work/$name/install.sh" "$@" ;;
                *)
                    echo "no terminal to ask about the system files, so skipping them (add --yes to install them without asking)"
                    "$work/$name/install.sh" --no-system "$@"
                    ;;
            esac
        fi
    else
        "$work/$name/install.sh" "$@"
    fi
}

main "$@"
