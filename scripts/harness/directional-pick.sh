# Holds South for the directional menu and picks its top slot, ten times.
for i in $(seq 10); do
    mark "round $i"
    pad press south; sleep 0.5
    pad tap up 40; sleep 0.2
    pad release south; sleep 1
done
