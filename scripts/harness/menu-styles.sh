# OVL-3: every menu style (Directional on South, Radial on East, List on West, Buttons on North,
# Carousel on LB, Grid on RB) opened and closed quickly many times, then one held open still for a
# minute, with the overlay's CPU use measured while it is held still and again once it is hidden.
# Run by scripts/menu-styles-check.sh, which judges the logs. Marks are `<style> <n> open|close|idle`
# and `hold start|still|end`; cpu.txt gets `still <percent of one core>` and `hidden <percent>`.
#   PADWIGHT_CYCLES=4           open-close rounds per style
#   PADWIGHT_HOLD_SECONDS=60    how long the radial menu is held open
cycles=${PADWIGHT_CYCLES:-4}
hold=${PADWIGHT_HOLD_SECONDS:-60}
ticks=$(getconf CLK_TCK)
cpu() { overlay=$(pgrep -n -f "$bin overlay"); [ -n "$overlay" ] || { echo "no overlay process" >&2; exit 1; }; awk '{ print $14 + $15 }' "/proc/$overlay/stat"; }
# measure <label> <seconds>: the overlay process's CPU over that long, as a share of one core.
measure() {
    local c0 c1
    c0=$(cpu); sleep "$2"; c1=$(cpu)
    printf '%s %.2f\n' "$1" "$(echo "($c1 - $c0) * 100 / $ticks / $2" | bc -l)" >>"$out/cpu.txt"
}
: >"$out/cpu.txt"
sleep 1

# open <style> <n>, then choose and close it. Held menus are chosen by their own input.
cycle() {
    local style=$1 n=$2
    mark "$style $n open"
    case $style in
        Directional) pad press south; sleep 0.35; mark "$style $n close"; pad tap right 30; pad release south ;;
        Radial) pad press east; sleep 0.35; mark "$style $n close"; pad stick right 1 0; sleep 0.15; pad release east; pad stick right 0 0 ;;
        List) pad press west; sleep 0.35; mark "$style $n close"; pad tap south 30; pad release west ;;
        Buttons) pad press north; sleep 0.35; mark "$style $n close"; pad tap east 30; pad release north ;;
        Carousel) pad tap lb 30; sleep 0.35; mark "$style $n close"; pad tap south 30 ;;
        Grid) pad tap rb 30; sleep 0.35; mark "$style $n close"; pad tap south 30 ;;
    esac >/dev/null
    sleep 0.7
    mark "$style $n idle"
    sleep 0.3
}
for n in $(seq "$cycles"); do
    for style in Directional Radial List Buttons Carousel Grid; do cycle "$style" "$n"; done
done

mark "hold start"
pad press east >/dev/null
sleep 3
mark "hold still"
measure still $((hold > 15 ? 10 : 5))
sleep $((hold > 15 ? hold - 13 : 0))
mark "hold end"
pad release east >/dev/null
sleep 1.5
mark "hidden"
measure hidden 5
