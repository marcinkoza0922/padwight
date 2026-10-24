# How much the settings window uses while it sits open and nothing changes, and its resident
# memory. Writes gui.txt to the out dir. Run with PADWIGHT_HARNESS_RELEASE=1.
"$bin" gui >"$out/gui.log" 2>&1 &
gui=$!
sleep 6
ticks=$(getconf CLK_TCK)
a=$(awk '{ print $14 + $15 }' "/proc/$gui/stat")
sleep 10
b=$(awk '{ print $14 + $15 }' "/proc/$gui/stat")
printf 'settings window idle: %.2f%% of a core, rss %s kB\n' "$(echo "($b - $a) * 100 / $ticks / 10" | bc -l)" "$(awk '/^VmRSS/ { print $2 }' "/proc/$gui/status")" >"$out/gui.txt"
grep -E '^(Pss_Anon|Pss_File)' "/proc/$gui/smaps_rollup" >>"$out/gui.txt"
kill $gui
