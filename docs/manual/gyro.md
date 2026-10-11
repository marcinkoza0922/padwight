# Gyro (motion aiming)

Some controllers have motion sensors, called gyro. Padwight can use them to aim or steer: turn or tilt the controller to move the mouse or a stick. DualShock 4, DualSense, Switch Pro and Joy-Con controllers have gyro.

- **Mouse**: turning the controller moves the mouse, like aiming with a mouse.
- **Gamepad stick (turn speed)**: the faster you turn, the further a virtual stick is pushed. It adds to the real stick.
- **Gamepad stick (turn position)**: how far you've turned the controller pushes a virtual stick, and it stays pushed while you hold it there. **Lock at edge** lets go of any turning past a full push, so turning back moves the stick off the edge straight away, as in Steam Input.
- **Steering**: tilt the controller like a steering wheel.

## How turning is read

- **Turning read as**: **Player space** turns the aim left and right however you hold the controller, flat or upright, and suits most people. **World space** also reads up and down against the horizon. **Yaw**, **Roll** and **Yaw + roll** use the controller's own axes.
- **Axes**: use gyro for both directions, or only one. *Vertical only* suits turning with a stick while gyro aims up and down.
- **Horizontal speed** and **Vertical speed** set each direction's sensitivity separately.

## Sensitivity and acceleration

In Mouse mode, sensitivity is pixels per degree turned. Tick **Real-world sensitivity** and enter the game's full turn, the mouse pixels that turn the game all the way round, as for a flick stick. Then 1× turns the game exactly as far as you turned the controller, so the same setting feels the same in every game.

**Acceleration** changes sensitivity with how fast you turn:

| Preset | Feel |
| --- | --- |
| Linear | the same at every speed |
| Gentle | a little slower when slow, a little faster when fast |
| Aggressive | slow for aiming, quickly much faster for turning |
| Wide | speeds up gradually over a wide range |
| Extra wide | speeds up very gradually, from still to fast turns |
| Precision | much slower for small movements, for sniping |
| Turn boost | unchanged when aiming, faster only for big quick turns |

Any preset except Linear shows its settings to fine-tune: the sensitivity when slow and when fast, the speeds where each starts, and the shape of the blend. A mouse stick's **Response** offers the same names on the Sticks & triggers tab.

The **Visualizers** under the settings draw the curve with your current turning speed on it, where the aim has been moving, and in the stick modes, the virtual stick.

## When gyro is on

- always
- only while you hold a button, such as LT to aim down the sights
- off while you hold a button, which lets you reposition the controller without turning the view
- switched on and off with a toggle, which can give a short rumble each time

**Pause while pushing** turns gyro off while you push a stick, so the stick and gyro don't fight.

## Drift and jitter

Very small movements are ignored, to hide drift. To set the baseline, keep the controller still and press **Calibrate gyro** in the controller list on the Overview page. It takes 2 seconds. Tick **Re-measure drift whenever the controller is set down still** to keep it right as the controller warms up.

**Smooth turning below** steadies slow, careful aiming. Faster turns aren't smoothed, so they aren't delayed.

## Profiles that need gyro

On a profile's **Gyro** tab, tick *Can't be played without gyro* if the profile depends on it. When someone imports a pack, players whose controller has no gyro aren't offered that profile.

> **Tip:** PlayStation and Switch controllers also need a motion-sensor permission rule. See [Troubleshooting](troubleshooting.md).
