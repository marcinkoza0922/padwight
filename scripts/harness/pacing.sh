# Frame pacing: opens the radial menu on East, sweeps the cursor round it, holds it still, and
# closes it, five times, with the overlay's timing log on (it has `present` lines). Run with
# PADWIGHT_HARNESS_RELEASE=1, then `python3 -I scripts/harness/pacing.py <out>/overlay.log`.
# Each round's marks are `round N open|move|hold|close`.
sleep 1
for i in $(seq 5); do
    mark "round $i open"
    pad press east >/dev/null; sleep 1.2
    mark "round $i move"
    for dir in "1 0" "0 -1" "-1 0" "0 1" "1 0" "0 -1" "-1 0" "0 1"; do
        pad stick right $dir >/dev/null; sleep 0.2
    done
    pad stick right 0 0 >/dev/null
    mark "round $i hold"
    sleep 1.5
    mark "round $i close"
    pad release east >/dev/null; sleep 1.2
    mark "round $i idle"
    sleep 0.8
done
