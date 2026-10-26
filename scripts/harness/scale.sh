# Lists the outputs, sets the first one's scale to $SCALE (kscreen-doctor), then runs the pacing rounds.
sleep 1
kscreen-doctor -o >"$out/outputs.log" 2>&1
name=$(sed 's/\x1b\[[0-9;]*m//g' "$out/outputs.log" | awk '/^Output:/ {print $3; exit}')
kscreen-doctor "output.$name.scale.${SCALE:-1}" >>"$out/outputs.log" 2>&1
sleep 1
kscreen-doctor -o >>"$out/outputs.log" 2>&1
. "$(dirname "${BASH_SOURCE[0]}")/pacing.sh"
