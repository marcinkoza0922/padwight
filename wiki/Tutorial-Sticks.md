# Tutorial: sticks as keys, mouse and walk zones

You'll turn the left stick into WASD movement with a walk zone, and the right stick into mouse-look. Open a profile's **Sticks & triggers** tab.

## 1. Left stick: a ring of keys

Each stick has a box. Its first dropdown is what the stick does: **Gamepad stick** (pass through), **Mouse pointer**, **Scroll wheel**, **Flick stick** or **Button ring**.

![The left stick set to Button ring, with its sliders, pictures and sector actions](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/sticks-ring.png "A button ring splits the stick into sectors. Each sector holds a key.")

1. Choose **Button ring** in the left stick's dropdown (1).
2. **Sectors** (2): 4, 8 or 12. Eight gives diagonals, which walking wants.
3. Under **Sector actions**, press **WASD** next to *Fill with* (5). Each sector gets the keys for its direction: W for up, W and D for up-right, and so on. **Arrows** fills in arrow keys instead.
4. The sliders (3) shape the ring. **First sector at** turns the ring. **Starts at** is how far out the sectors begin. **Stickiness** holds a sector a little past its edge, so a wobble doesn't flick between neighbours. **Deadzone** is how far the stick must move before it counts: raise it if the stick drifts at rest.
5. The **Visualizers** (4) show the ring, the stick's travel and its response. They follow your stick while this profile is active.

## 2. Walk and run with a zone

A zone adds an action while the stick is pushed part of the way.

![The Zones row with a Left Shift zone from 0.00 to 0.60, and the right stick set to Mouse pointer](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/sticks-zones.png "A zone for the first 60% of the stick's travel, and a mouse stick below it.")

1. At the bottom of the left stick's box, press **+ Walk modifier (Shift on partial push)** (2), which makes this zone for you. Or press **+ Add zone** and set it yourself:
2. **From 0.00 to 0.60**, with the action **Keys** and **Left Shift** (1).

Shift is held while the stick is in the first 60% of its travel. In most games a light push then walks and a full push runs.

## 3. Right stick: mouse-look

1. Choose **Mouse pointer** in the right stick's dropdown (3).
2. **Speed** is the pointer's speed at a full push, in pixels a second. Start near 1600.
3. **Response** (4) shapes how the push turns into speed. Pick a preset, or **Custom**:
   - **Curve** above 1 gives fine control near the middle and full speed at the edge.
   - **Acceleration** and **Ramp time** speed up the turn while you hold the stick at full push, as console shooters do.
   - **Outer boost** adds speed near the edge.
4. **Deadzone** hides drift. **Invert Y** flips up and down.
5. Under **Advanced**, **Vertical speed** sets up and down as a share of the horizontal speed.

## 4. Save and try it

Press **Save & apply** and walk around. The **Pointer speed** and **Speed by direction** pictures under the right stick show how the pointer moves; hover the graph to read the speed at any push.

## 5. Fit the deadzone to the stick and the game

Tune these in order, since each works on what the one before left:

1. **Deadzone**: with the stick untouched, raise it until the *Stick travel* dot stays inside the shaded middle, then stop.
2. **Deadzone shape**: leave it on **Circle**. Try **Cross** on a ring for clean straight-line movement.
3. **Anti-deadzone**: only for a **Gamepad stick** in a game that ignores small pushes. Raise it until the lightest push moves the game, then back off a step.
4. **Anti-deadzone hold**: only for a steady patch just past the deadzone, such as a sneak speed.
5. **Invert** and **Rotate output**: for a game that expects a flipped or turned stick.

Change one setting at a time, save, and test it in the game.

## Triggers

Triggers are on this tab too. A trigger can pass its pull on as an analog value, or work as a button that fires past a **threshold**. Zones work on triggers too, except on controllers whose triggers are only on or off.

## Where next

- [Sticks and triggers](Sticks-and-Triggers) for flick stick and scrolling.
- The game setup guidelines for shooters (in the repository's `docs/development`) list the curve and deadzone values packs start from.
- [Gyro](Tutorial-Gyro) to add motion aiming on top of the right stick.
