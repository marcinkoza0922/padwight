# Tutorial: aim with the gyro

On a DualShock 4, DualSense, Switch Pro controller or Joy-Con you can aim by turning the controller. You'll set up gyro as mouse aiming, with a button that switches it off while you reposition your hands.

**You'll need** a controller with gyro, and on PlayStation and Switch controllers the motion-sensor permission rule from [Troubleshooting](Troubleshooting). Without it the Overview says the daemon can't open the sensors.

## 1. Calibrate

Keep the controller still on a table, then press **Calibrate gyro** next to it in the controller list on the **Overview** page. It takes two seconds and sets the baseline that small drift is measured from.

## 2. Make a profile for it

Gyro belongs to a profile. A good habit is to keep your plain profile and make a copy for gyro: on the **Profiles** tab press **Duplicate**, and rename the copy, for example *Play + Gyro*. Then open its **Gyro** tab.

## 3. Turn it on

![The Gyro tab: the mode, the sensitivity, the Active row set to off while Left Trigger is held, and the box saying the profile needs gyro](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/gyro-panel.png "The Gyro tab of a copy of the profile.")

1. Choose **Mouse (gyro aiming)** in the first dropdown (2). The other modes push a virtual stick instead, or steer like a wheel.
2. **Sensitivity** (3) is how many pixels the pointer moves for each degree you turn. Start at 30. Tick **Real-world sensitivity** if you'd rather enter the game's full turn: then every game feels the same for the same setting.
3. **Turning read as** should be **Player space** for most people: turning left aims left however you hold the controller, flat or upright.
4. **Axes** lets gyro do both directions, or only up and down, which suits a profile where the stick turns and gyro adjusts the aim.
5. **Horizontal speed** and **Vertical speed** scale each direction.

## 4. Choose when it works

Gyro is on from the start. Tick **Starts switched off** if it should wait for you to turn it on. What switches it is an action, so you put it on whichever input suits you (a button, a gesture, a combo or a menu item) from the action list:

- **Gyro clutch (off while held)**: hold it to lift the controller and bring it back without the view turning.
- **Gyro on while held**: gyro only while you hold the aim button. Tick **Starts switched off** too.
- **Gyro on / off**: each press switches gyro on or off. **Rumble on gyro on / off** gives a short buzz when it does.
- **Recenter gyro**: counts the way the controller points now as straight ahead (steering and position modes), and clears leftover motion.

Most games already use every button, so a combo or gesture is the way to add a clutch without taking one from the layout. Back paddles also suit, on controllers that have them, but a setup can't rely on them.

**Pause while pushing** turns gyro off while you push a stick, so the stick and gyro don't fight.

## 5. Tune the feel

- **Response** changes sensitivity with how fast you turn. *Precision* is slow for small movements, for sniping; *Turn boost* leaves aiming alone and speeds up big quick turns.
- Under **Advanced**, **Smooth below** steadies slow, careful aiming without delaying fast turns, and **Re-measure drift whenever the controller is set down still** keeps the baseline right as the controller warms up.
- The **Visualizers** at the bottom draw the curve with your current turning speed on it. They show the active profile's saved settings, so make the profile active and press **Save & apply** to watch them move.

## 6. Aim a game that only reads the gamepad

If the game has no mouse support alongside the gamepad, pick **Gamepad stick (turn speed)** in the first dropdown and send it to the **Right stick**. Turning the controller then pushes a virtual stick, added to the real one. Set these in order:

1. **Full tilt at**: how fast you turn for a full push. Lower it to turn the view further with the same turn.
2. **Anti-deadzone**: raise it until slow turns move the view, and lower it if the view creeps with the controller still.
3. **Game's curve**: if turning the controller slowly turns the view less than you'd expect from turning it quickly, raise this a little at a time from 1 until slow and fast turns feel consistent. It undoes the game's own curve, so the right value is the game's.

Set the game's own stick sensitivity before any of these, since each of them depends on it.

## 7. Say if the profile needs gyro

Tick **Can't be played without gyro** (1) only if the profile depends on it, such as a flick-stick profile that turns left and right with the stick and up and down with gyro. Players whose controller has no gyro then aren't offered it when they import your pack. Leave it unticked when gyro is an extra.

## When it feels wrong

- The view creeps with the controller on the table: **Calibrate gyro** again, then raise *Ignore jitter below* under **Advanced**.
- Slow aiming is shaky: raise **Smooth below**.
- Turning the controller the same angle gives a different turn in each game: tick **Real-world sensitivity** and enter each game's full turn.
- Up and down feel backwards or too strong: **Invert vertical** and **Vertical speed**.

## Where next

- [Gyro](Gyro) for every setting.
- [Changing controls while you play](In-Game-Menu): Quick Settings changes gyro sensitivity or turns gyro on and off without leaving the game.
