# How much the overlay process (and KWin) use while the radial menu's cursor sweeps round and
# round without stopping: East holds the radial menu, and the right stick steps through 16
# directions, a new one every 50 ms or so. Writes resources.txt to the out dir, as resources.sh
# does. Run with PADWIGHT_HARNESS_RELEASE=1, so the figures are for the optimized build.
overlay=$(pgrep -n -f "$bin overlay")
kwin=$(pgrep -n -f "kwin_wayland --virtual --socket $WAYLAND_DISPLAY")
ticks=$(getconf CLK_TCK)
cpu() { awk '{ print $14 + $15 }' "/proc/$1/stat"; }
rss() { awk '/^VmRSS/ { print $2 }' "/proc/$1/status"; }
# measure <phase> <seconds> [command run meanwhile]
measure() {
    local o0 k0 o1 k1
    o0=$(cpu "$overlay"); k0=$(cpu "$kwin")
    if [ $# -gt 2 ]; then "${@:3}"; else sleep "$2"; fi
    o1=$(cpu "$overlay"); k1=$(cpu "$kwin")
    printf '%-12s overlay %5.1f%%  kwin %5.1f%%  overlay rss %6d kB\n' "$1" \
        "$(echo "($o1 - $o0) * 100 / $ticks / $2" | bc -l)" "$(echo "($k1 - $k0) * 100 / $ticks / $2" | bc -l)" \
        "$(rss "$overlay")" >>"$out/resources.txt"
}
# Sixteen directions round the circle, x and y on the unit circle.
directions=("1 0" "0.92 -0.38" "0.71 -0.71" "0.38 -0.92" "0 -1" "-0.38 -0.92" "-0.71 -0.71" "-0.92 -0.38"
    "-1 0" "-0.92 0.38" "-0.71 0.71" "-0.38 0.92" "0 1" "0.38 0.92" "0.71 0.71" "0.92 0.38")
sweep() {
    local end=$((SECONDS + $1))
    while [ $SECONDS -lt $end ]; do
        for dir in "${directions[@]}"; do pad stick right $dir >/dev/null; sleep 0.05; done
    done
}
: >"$out/resources.txt"
sleep 2
measure hidden 5
mark "radial open"
pad press east >/dev/null; sleep 1.5
measure radial-held 5
mark "radial sweep"
measure sweeping 5 sweep 5
pad stick right 0 0 >/dev/null
mark "radial close"
pad release east >/dev/null; sleep 1.5
measure hidden-after 5
