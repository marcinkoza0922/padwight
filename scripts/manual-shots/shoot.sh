#!/bin/sh
# Takes the raw screenshots for the manual into screenshots/manual/ (untracked), with `padwight debug gui`
# and `padwight debug overlay`. annotate.py then marks them up, from specs/*.json, into docs/images/manual/.
# Run scripts/manual-shots/build.sh to do both.
cd "$(dirname "$0")/../.." || exit 1
cargo build -q || exit 1
pw=target/debug/padwight
out=screenshots/manual

# Library setups for every picture, plus the macro and info overlay the tutorials build ("demo").
"$pw" debug gui library StarCraft library "Deus Ex" game "Deus Ex" demo --size 1280x1500 --out $out \
    page overview shot overview game "Deus Ex" \
    picture hide tab details shot details \
    dialog add shot add-setup close \
    tab profiles sub buttons shot buttons-list \
    picture show shot buttons-picture picture hide \
    open South shot buttons \
    sub sticks shot sticks \
    size 1280x3000 shot sticks-tall size 1280x1500 \
    profile "Deus Ex + Gyro" sub gyro shot gyro \
    profile "Deus Ex + Flick stick" sub sticks size 1280x3000 shot flick size 1280x1500 \
    profile "Deus Ex" sub touchpad shot touchpad \
    sub guide shot guide-tab \
    tab menus menu 0 shot menus \
    tab macros macro 0 shot macros \
    tab info info 0 shot info \
    game StarCraft tab layers sub buttons shot layers \
    tab modes shot modes \
    game StarCraft tab details dialog export shot export close \
    page settings settings shortcuts shortcuts-demo shot shortcuts-tab \
    >/dev/null
"$pw" debug overlay guide menu-radial info in-game-menu keyboard menu-list --out $out/ov >/dev/null
