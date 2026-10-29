# Gyro (motion aiming)

DualShock 4, DualSense, Switch Pro and Joy-Con controllers have motion sensors, called gyro. Padwight can use them to aim or steer by turning or tilting the controller:

- **Mouse**: turning the controller moves the mouse.
- **Gamepad stick (turn speed)**: the faster you turn, the further a virtual stick is pushed, on top of the real stick.
- **Gamepad stick (turn position)**: how far you've turned pushes a virtual stick, which stays pushed while you hold the controller there. **Lock at edge** ignores turning past a full push, so turning back moves the stick off the edge at once, as in Steam Input.
- **Steering**: tilt the controller like a steering wheel.

The settings sit in a box laid out like a stick's: the mode on the top line, then its settings, an *Advanced* fold (jitter, smoothing and drift) and a *Visualizers* panel.

## How turning is read

- **Turning read as**: **Player space** turns the aim left and right however you hold the controller, and suits most people. **World space** also reads up and down against the horizon. **Yaw**, **Roll** and **Yaw + roll** use the controller's own axes.
- **Axes**: both directions, or only one. *Vertical only* suits turning with a stick while gyro aims up and down.
- **Horizontal speed** and **Vertical speed** scale each direction.

## Sensitivity and acceleration

In Mouse mode, sensitivity is pixels per degree turned. Or tick **Real-world sensitivity** and enter the game's full turn (the mouse pixels that turn the view all the way round): then 1× turns the view exactly as far as you turned the controller, in every game.

**Response** changes sensitivity with how fast you turn:

| Preset | Feel |
| --- | --- |
| Linear | the same at every speed |
| Gentle | a little slower when slow, a little faster when fast |
| Aggressive | slow for aiming, quickly much faster for turning |
| Wide | speeds up gradually over a wide range |
| Extra wide | speeds up very gradually, from still to fast turns |
| Precision | much slower for small movements, for sniping |
| Turn boost | unchanged when aiming, faster only for big quick turns |

Any preset but Linear shows its settings: the sensitivity when slow and when fast, the speeds where each applies, and the shape of the blend. A mouse stick and the touchpad offer the same presets.

The **Visualizers** draw the curve with your current turning speed on it, the aim's recent path, and in the stick modes, the virtual stick.

## Stick modes: getting the setting right

The stick modes push a virtual stick for games that only read the gamepad, so they have settings of their own:

- **Full tilt at** (turn speed) or **Full push at** (turn position): how fast, or how far, you turn for a full push. Smaller makes the same turn count for more. They start at 360°/s and 30°.
- **Anti-deadzone**: the least push sent, so slow turns get past the game's own deadzone (often 0.1 to 0.2). Raise it if slow turns do nothing; lower it if the view creeps with the controller still.
- **Game's curve** (turn speed only): the game's own stick curve, where 1 is linear. Padwight undoes it, so the view turns by the same amount at every speed, as with a mouse. Leave it at 1 if you don't know it. If slow turns move the view less than fast ones through the same angle, raise it a little at a time.

Set the game's own stick sensitivity first, then *Full tilt at*, the anti-deadzone and the game's curve, in that order. Changing an earlier one moves the later ones.

## When gyro is on

**Enabled** at the top says whether the profile uses gyro at all; until it is ticked, no input can be given a gyro action, and a profile that has one can't be saved with gyro disabled. **Starts active** says whether gyro is on from the start; untick it and gyro waits for a button. Unticking Enabled keeps your aim settings for later. These actions, on any button, gesture, combo or menu item, change that:

- **Gyro on while held**: only while you hold it, such as LT to aim down the sights
- **Gyro clutch (off while held)**: lets you reposition the controller without turning the view
- **Gyro on / off**: switches it on and off; **Rumble on gyro on / off** buzzes each time
- **Recenter gyro**: counts the way the controller points now as straight ahead

**Pause while pushing** turns gyro off while you push a stick, so the stick and gyro don't fight.

## Drift and jitter

To set the baseline, put the controller down still and press **Calibrate gyro** on the Overview page. It takes 2 seconds. In the *Advanced* fold:

- **Ignore jitter below** hides drift. Raise it if the view creeps with the controller on a table; lower it if slow, careful aiming gets lost.
- **Re-measure drift whenever the controller is set down still** keeps the baseline right as the controller warms up.
- **Smooth below** steadies slow aiming without delaying faster turns.

## Profiles that need gyro

On the **Gyro** tab, tick *Profile requires gyro* if the profile depends on it. Players whose controller has no gyro then aren't offered it.

> **Tip:** PlayStation and Switch controllers also need a motion-sensor permission rule. See [Troubleshooting](troubleshooting.md).

## Where to start

| For | Mode | Notes |
| --- | --- | --- |
| A shooter that takes the mouse and the gamepad together | Mouse, Player space | Real-world sensitivity at 1×, then adjust; set *Active* to only while a button is held if it's too much |
| A gamepad-only game | Gamepad stick (turn speed) on the right stick | Match the game's curve; set the anti-deadzone to the game's deadzone |
| Vertical aiming with the stick turning | Mouse, Axes: *Vertical only* | Pair with a flick stick |
| Racing | Steering | Full lock at the angle you like to tilt |
| A sniper scope | A layer on the aim button with the *Precision* response | See [Layers](layers.md) |

If aiming feels wrong, change one thing at a time, in this order: *Turning read as*, sensitivity, response, smoothing, jitter. Check the *Visualizers* before changing anything.
