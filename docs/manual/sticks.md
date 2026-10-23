# Sticks and triggers

## What a stick can do

- **Gamepad stick**: passes its movement on to the virtual controller, with the settings below applied.
- **Mouse pointer**: moves the mouse. **Response** offers the same presets as gyro (see [Gyro](gyro.md)). Any preset but Linear shows its settings: *Curve*, *Acceleration* (speeds up while you hold the stick at full push) with its *Ramp time*, and *Outer boost* near the edge. *Custom* starts your own from the response in use.
- **Scroll wheel**: scrolls up and down.
- **Flick stick**: push it to turn the camera to face that way at once; rotating it then turns the camera by the same angle. It turns left and right only, so aim up and down with gyro.
- **Button ring**: splits the stick into 4, 8 or 12 sectors. Pointing into a sector holds its action, such as W, A, S and D for movement.

## Tuning

Each stick has a box: its mode and *Invert* switches on the top line, then its settings, an *Advanced* fold for the finer ones, a *Visualizers* panel with the stick's pictures, and last its direction buttons, *Rotate output* and zones.

### The deadzone, in order

A stick's position goes through these steps in order, each working on what the one before left.

1. **Deadzone**: how far the stick must move before it counts. Raise it if the stick drifts at rest; the smallest value that stops the drift is best. A worn stick needs more than a new one. The output rises smoothly from zero past it, so a bigger deadzone doesn't make the stick jumpy.
2. **Deadzone shape**: the outline of the dead middle.
   - **Circle** (the default) is the same in every direction. Choose it unless you have a reason not to.
   - **Square**: a push counts once either axis passes the deadzone. Diagonals need a bigger push than with a circle.
   - **Cross** gives each axis its own deadzone, so a push straight up has no sideways drift. It suits a button ring or movement in straight lines, but not aiming, since gentle diagonals lose their small part too.
3. **Anti-deadzone**: the least output once the stick is past the deadzone, for a game that ignores small pushes and won't let you change that. Raise it in small steps until the lightest push moves the game's view, then take one step back. Leave it at 0 for mouse, scroll and ring modes.
4. **Anti-deadzone hold**: extra travel past the deadzone where the output stays at the anti-deadzone before it rises. It gives a steady patch for pushes that must not creep, such as a sneak speed. Usually 0.
5. **Invert X** and **Invert Y** flip left and right, and up and down.
6. **Rotate output**: turns the whole stick clockwise by that many degrees. At 90, pushing up acts as pushing right. Direction buttons and zones see the turned stick.

The *Stick travel* and *Response* pictures show the first four steps, and follow the stick while the daemon runs.

### Mode settings

- **Curve**: how quickly the output grows as you push further. Higher gives finer control near the center; 1 is straight, and 2 is a good start for aiming. For a mouse stick it's part of the Response.
- **Speed visualizers**: for a mouse stick, *Pointer speed* plots speed against push, and *Speed by direction* the speed each way. Hover the graph to read the speed at any push.
- **Directions press at**: how far the stick must go to press its direction buttons (Left Stick Up, and so on). Raise it if a resting thumb sets them off.

### Starting points

| For | Mode | Deadzone | Notes |
| --- | --- | --- | --- |
| Walking in a game with no controller support | Button ring, 8 sectors | just above the drift | Add a walk zone (see the [tutorial](tutorial-sticks.md)) |
| Mouse-look | Mouse pointer | just above the drift | Curve 2, speed near 1600; use Acceleration only to copy a console shooter |
| A game that already reads the gamepad but ignores small pushes | Gamepad stick | as small as drift allows | Raise the anti-deadzone to the game's own deadzone |
| Quick turns | Flick stick | the default | Pair with gyro for up and down |

## Stick directions as buttons

Each stick direction (Left Stick Up, and so on) can have its own action and can be part of a combo, such as LB + Right Stick Right.

## Triggers

A trigger can pass its pull on as an analog value, for games that read how far it's pressed, or work as a button that fires once the pull passes a threshold.

## Zones

A zone adds an action while a stick or trigger is pushed part of the way. Holding Left Shift on a light stick push can mean walk, and a full push run; a half pull and a full pull of a trigger can do different things. Triggers that are only on or off have no zones.
