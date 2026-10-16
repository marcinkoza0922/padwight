# Gyro (motion aiming)

Some controllers have motion sensors, called gyro. Padwight can use them to aim or steer: turn or tilt the controller to move the mouse or a stick. DualShock 4, DualSense, Switch Pro and Joy-Con controllers have gyro.

- **Mouse**: turning the controller moves the mouse, like aiming with a mouse.
- **Gamepad stick (turn speed)**: the faster you turn, the further a virtual stick is pushed. It adds to the real stick.
- **Gamepad stick (turn position)**: how far you've turned the controller pushes a virtual stick, and it stays pushed while you hold it there. **Lock at edge** lets go of any turning past a full push, so turning back moves the stick off the edge straight away, as in Steam Input.
- **Steering**: tilt the controller like a steering wheel.

The gyro's settings sit in a box laid out like a stick's: the mode on the top line, with the stick it sends (and *Invert*, for steering) beside it, then its settings, an *Advanced* fold with jitter, smoothing and drift, and a *Visualizers* panel.

## How turning is read

- **Turning read as**: **Player space** turns the aim left and right however you hold the controller, flat or upright, and suits most people. **World space** also reads up and down against the horizon. **Yaw**, **Roll** and **Yaw + roll** use the controller's own axes.
- **Axes**: use gyro for both directions, or only one. *Vertical only* suits turning with a stick while gyro aims up and down.
- **Horizontal speed** and **Vertical speed** set each direction's sensitivity separately.

## Sensitivity and acceleration

In Mouse mode, sensitivity is pixels per degree turned. Tick **Real-world sensitivity** and enter the game's full turn, the mouse pixels that turn the game all the way round, as for a flick stick. Then 1× turns the game exactly as far as you turned the controller, so the same setting feels the same in every game.

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

Any preset except Linear shows its settings to fine-tune: the sensitivity when slow and when fast, the speeds where each starts, and the shape of the blend. A mouse stick's and the touchpad's **Response** offer the same names.

The **Visualizers** at the bottom of the box draw the curve with your current turning speed on it, where the aim has been moving, and in the stick modes, the virtual stick.

## Stick modes: getting the setting right

The two stick modes send a virtual stick to a game that reads the gamepad, so they have settings the mouse mode doesn't:

- **Full tilt at** (turn speed) or **Full push at** (turn position): how fast, or how far, you turn for a full push. A smaller number makes the same turn count for more. The app starts turn speed at 360°/s and turn position at 30°.
- **Anti-deadzone**: the least deflection sent. A game ignores small stick pushes, so slow turns would do nothing; this starts the stick just past the game's own deadzone. If slow turns do nothing, raise it a little at a time; if the view creeps with the controller still, lower it. A game's deadzone is often around 0.1 to 0.2 of a full push.
- **Game's curve** (turn speed only): the game's own stick response, where 1 is linear and higher makes small pushes turn slowly. Most games use a curve, so a gentle turn of the controller turns the view less than you expect. Set this to the game's, and Padwight undoes it so that turning the controller turns the view by the same amount at every speed, as a mouse would. Leave it at 1 if you don't know the game's curve. If slow turns of the controller turn the view less than fast ones through the same angle, raise it a little at a time; past the game's real curve, slow turns become too strong.
- **Lock at edge** (turn position): see above.

Set the game's own stick sensitivity first, then *Full tilt at*, then the anti-deadzone, then the game's curve. Changing an earlier one moves the later ones.

## When gyro is on

- always
- only while you hold a button, such as LT to aim down the sights
- off while you hold a button, which lets you reposition the controller without turning the view
- switched on and off with a toggle, which can give a short rumble each time

**Pause while pushing** turns gyro off while you push a stick, so the stick and gyro don't fight.

## Drift and jitter

Very small movements are ignored, to hide drift. *Ignore jitter below* (in the *Advanced* fold) sets how small: raise it if the view creeps with the controller on a table, lower it if slow, careful aiming gets lost. To set the baseline, keep the controller still and press **Calibrate gyro** in the controller list on the Overview page. It takes 2 seconds. In the *Advanced* fold, tick **Re-measure drift whenever the controller is set down still** to keep it right as the controller warms up.

**Smooth below**, also in the *Advanced* fold, steadies slow, careful aiming. Faster turns aren't smoothed, so they aren't delayed.

## Profiles that need gyro

On a profile's **Gyro** tab, tick *Can't be played without gyro* if the profile depends on it. When someone imports a pack, players whose controller has no gyro aren't offered that profile.

> **Tip:** PlayStation and Switch controllers also need a motion-sensor permission rule. See [Troubleshooting](troubleshooting.md).

## Where to start

| For | Mode | Notes |
| --- | --- | --- |
| A shooter that takes the mouse and the gamepad together | Mouse, Player space | Real-world sensitivity at 1×, then adjust; set *Active* to only while a button is held if it's too much |
| A gamepad-only game | Gamepad stick (turn speed) on the right stick | Match the game's curve; set the anti-deadzone to the game's deadzone |
| Vertical aiming with the stick turning | Mouse, Axes: *Vertical only* | Pair with a flick stick |
| Racing | Steering | Full lock at the angle you like to tilt |
| A sniper scope | A layer on the aim button with the *Precision* response | See [Layers](layers.md) |

If aiming feels wrong, change one thing at a time in this order: *Turning read as*, sensitivity, response, smoothing, then jitter. Use the *Visualizers* to see what the controller is doing before touching a setting.
