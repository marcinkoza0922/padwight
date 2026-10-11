# Sticks and triggers

## What a stick can do

- **Gamepad stick**: passes its movement on to the virtual controller, possibly changed.
- **Mouse pointer**: moves the mouse. Its **Response** offers the same presets as gyro acceleration (see [Gyro](gyro.md)). Any preset except Linear shows its settings to fine-tune: the *Curve*, *Acceleration* (speeds up while you hold the stick at full push) with its *Ramp time*, and an *Outer boost* near the edge. Pick *Custom* to start your own from the response in use.
- **Scroll wheel**: scrolls up and down.
- **Flick stick**: push it to turn the camera in that direction at once. Turning it then turns the camera by the same angle. It turns left and right only, so aim up and down with gyro or the other stick.
- **Button ring**: splits the stick into 4, 8 or 12 sectors. Pointing into a sector holds that sector's action, such as W, A, S and D for movement.

## Tuning

Each stick has a box of its own: its mode and Invert Y on the top line, then its settings, an *Advanced* fold for the finer ones (click its bar to open it; while shut it lists what it holds), a *Visualizers* panel with the stick's pictures, and last its direction buttons and zones.

- **Deadzone**: how far the stick must move before it counts. Raise it if the stick drifts when resting. The *Stick travel* and *Response* pictures show the dead middle of the stick's travel, and how the response starts only past it.
- **Curve**: how quickly the output grows as you push further. A higher curve gives finer control near the center. A scroll stick has it on its own; a mouse stick's is part of its Response.
- **Speed visualizers**: for a mouse stick, *Pointer speed* shows the speed against how far the stick is pushed, and *Speed by direction* the speed in each direction. Hover the speed graph to read the speed at any push; the picture beside it follows it.
- **Invert Y**: flips up and down.

## Stick directions as buttons

Each stick direction (Left Stick Up, and so on) can have an action of its own, and can take part in combos. For example, LB + Right Stick Right.

## Triggers

A trigger can send its pull as an analog value, for games that read how far it's pressed. Or it can work as a button, which fires once the pull passes a threshold you set.

## Zones

A zone adds an action while a stick or trigger is pushed part of the way. For example, holding Left Shift on a light stick push can mean walk, and a full push can mean run. A half pull and a full pull on a trigger can do different things.

Controllers whose triggers are only on or off don't have zones.
