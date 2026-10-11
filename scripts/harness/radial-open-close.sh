# Holds East for the radial menu, then lets go without aiming; then aims and picks. Five times.
for i in $(seq 5); do
    mark "round $i open-close"
    pad press east; sleep 0.8
    pad release east; sleep 0.8
    mark "round $i pick"
    pad press east; sleep 0.5
    pad stick right 0 -1; sleep 0.3
    pad release east; pad stick right 0 0; sleep 1
done
