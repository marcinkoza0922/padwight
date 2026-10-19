# Sticks and triggers

## What a stick can do

- **Gamepad stick**: passes its movement on to the virtual controller, possibly changed.
- **Mouse pointer**: moves the mouse. Its **Response** offers the same presets as gyro acceleration (see [Gyro](gyro.md)). Any preset except Linear shows its settings to fine-tune: the *Curve*, *Acceleration* (speeds up while you hold the stick at full push) with its *Ramp time*, and an *Outer boost* near the edge. Pick *Custom* to start your own from the response in use.
- **Scroll wheel**: scrolls up and down.
- **Flick stick**: push it to turn the camera in that direction at once. Turning it then turns the camera by the same angle. It turns left and right only, so aim up and down with gyro or the other stick.
- **Button ring**: splits the stick into 4, 8 or 12 sectors. Pointing into a sector holds that sector's action, such as W, A, S and D for movement.

## Tuning

Each stick has a box of its own: its mode, *Invert Y* and *Invert X* on the top line, then its settings, an *Advanced* fold for the finer ones (click its bar to open it; while shut it lists what it holds), a *Visualizers* panel with the stick's pictures, and last its direction buttons, *Rotate output* and zones.

### The deadzone, in order

A stick's raw position goes through these steps, in this order. Each later setting works on what the one before it left.

1. **Deadzone**: how far the stick must move before it counts. Raise it if the stick drifts at rest, and lower it until the drift comes back to find the smallest one that works. A worn stick needs more than a new one. The response starts only past it, rising smoothly from zero, so a bigger deadzone doesn't make the stick feel jumpy.
2. **Deadzone shape**: the outline of the dead middle.
   - **Circle** (the default) is the same in every direction. Choose it unless you have a reason not to.
   - **Square** is dead inside a square. A push counts once either axis passes the deadzone, and it keeps its direction. Diagonals need a bigger push than with a circle, and a full diagonal still reaches full output.
   - **Cross** gives each axis a deadzone of its own. Pushing straight up stays clean of sideways drift, because the sideways part is dropped while it's small. That suits a button ring or movement in straight lines. The small part of a gentle diagonal is dropped too, so avoid it for aiming.
3. **Anti-deadzone**: the least output once the stick is past the deadzone. Use it only for a game with a deadzone of its own that you can't change: the game ignores small pushes, so Padwight starts the output at the size of the game's deadzone instead. Leave it at 0 for mouse, scroll and ring modes, which have no deadzone of their own to beat. To find the right value, raise it from 0 in steps of 0.02 until the smallest push of the stick moves the game's view, then take one step back.
4. **Anti-deadzone hold**: extra travel past the deadzone where the output holds steady at the anti-deadzone before it starts to rise. It gives a small flat patch for pushes that must not creep, such as a sneak speed. Leave it at 0 unless you want that patch.
5. **Invert X** and **Invert Y** flip left and right, and up and down.
6. **Rotate output**: turns the whole stick by that many degrees clockwise. At 90, pushing up acts as pushing right. Use it for a stick whose directions don't match the game's, or to line a stick up with a tilted camera. Everything after it, including direction buttons and zones, sees the turned stick.

The *Stick travel* and *Response* pictures show the first four: the dead middle in its shape, and the response starting past it (with the anti-deadzone's step and flat patch if you set them). Push the stick with the daemon running and the dots follow it.

### Mode settings

- **Curve**: how quickly the output grows as you push further. A higher curve gives finer control near the center. 1 is straight; 2 is the usual starting point for aiming. A scroll stick has it on its own; a mouse stick's is part of its Response.
- **Speed visualizers**: for a mouse stick, *Pointer speed* shows the speed against how far the stick is pushed, and *Speed by direction* the speed in each direction. Hover the speed graph to read the speed at any push; the picture beside it follows it.
- **Directions press at**: how far the stick must go for its direction buttons (Left Stick Up, and so on) and for key modes to press. Lower it for a lighter touch; raise it if a resting thumb sets them off.

### Starting points

| For | Mode | Deadzone | Notes |
| --- | --- | --- | --- |
| Walking in a game with no controller support | Button ring, 8 sectors | just above the drift | Add a walk zone (see the [tutorial](tutorial-sticks.md)) |
| Mouse-look | Mouse pointer | just above the drift | Curve 2, speed near 1600; use Acceleration only to copy a console shooter |
| A game that already reads the gamepad but ignores small pushes | Gamepad stick | as small as drift allows | Raise the anti-deadzone to the game's own deadzone |
| Quick turns | Flick stick | the default | Pair with gyro for up and down |

## Stick directions as buttons

Each stick direction (Left Stick Up, and so on) can have an action of its own, and can take part in combos. For example, LB + Right Stick Right.

## Triggers

A trigger can send its pull as an analog value, for games that read how far it's pressed. Or it can work as a button, which fires once the pull passes a threshold you set.

## Zones

A zone adds an action while a stick or trigger is pushed part of the way. For example, holding Left Shift on a light stick push can mean walk, and a full push can mean run. A half pull and a full pull on a trigger can do different things.

Controllers whose triggers are only on or off don't have zones.
