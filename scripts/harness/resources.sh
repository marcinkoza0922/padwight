# How much the overlay process (and KWin, which composites what it draws) use in each state:
# hidden, a menu held open, a menu being moved through, and hidden again. Writes resources.txt
# to the out dir: CPU as a share of one core, and the overlay's resident memory.
# Run with PADWIGHT_HARNESS_RELEASE=1, so the figures are for the optimized build.
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
navigate() {
    local end=$((SECONDS + $1))
    while [ $SECONDS -lt $end ]; do pad tap down 30 >/dev/null; sleep 0.15; done
}
: >"$out/resources.txt"
sleep 2
measure hidden 5
pad press south >/dev/null; sleep 1.5
measure menu-held 5
measure navigating 5 navigate 5
pad release south >/dev/null; sleep 1.5
measure hidden-after 5
# What the overlay's memory is: its own (anonymous) or files mapped in, such as the graphics
# driver's libraries, which other programs share.
grep -E '^(Rss|Pss|Pss_Anon|Pss_File|Pss_Shmem|Anonymous)' "/proc/$overlay/smaps_rollup" >>"$out/resources.txt"
