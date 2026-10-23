# Tutorial: aim with the gyro

On a DualShock 4, DualSense, Switch Pro controller or Joy-Con you can aim by turning the controller. You'll set up gyro mouse aiming, with a button that pauses it while you reposition your hands.

**You'll need** a controller with gyro. PlayStation and Switch controllers also need the motion-sensor permission rule from [Troubleshooting](Troubleshooting).

## 1. Calibrate

Put the controller still on a table and press **Calibrate gyro** next to it on the **Overview** page. It takes two seconds and sets the baseline drift is measured from.

## 2. Make a profile for it

Gyro belongs to a profile. Keep your plain profile and make a copy for gyro: on the **Profiles** tab press **Duplicate**, and rename the copy, say *Play + Gyro*. Then open its **Gyro** tab.

## 3. Turn it on

![The Gyro tab: the mode, the sensitivity, the Active row set to off while Left Trigger is held, and the box saying the profile needs gyro](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/gyro-panel.png "The Gyro tab of a copy of the profile.")

1. Choose **Mouse (gyro aiming)** in the first dropdown (2). The other modes push a virtual stick, or steer like a wheel.
2. **Sensitivity** (3) is how many pixels the pointer moves per degree you turn. Start at 30. Or tick **Real-world sensitivity** and enter the game's full turn, so every game feels the same.
3. **Turning read as**: **Player space** suits most people. Turning left aims left however you hold the controller.
4. **Axes**: both directions, or only up and down, for a profile where the stick turns and gyro fine-tunes.
5. **Horizontal speed** and **Vertical speed** scale each direction.

## 4. Choose when it works

The **Active** row (4) sets how gyro starts: on, unless you tick **Starts switched off**. What switches it is an action, which you put on any button, gesture, combo or menu item:

- **Gyro clutch (off while held)**: hold it to lift the controller and bring it back without the view turning.
- **Gyro on while held**: gyro only while you hold the aim button. Tick **Starts switched off** too.
- **Gyro on / off**: each press switches gyro on or off. **Rumble on gyro on / off** gives a short buzz when it does.
- **Recenter gyro**: counts the way the controller points now as straight ahead (steering and position modes), and clears leftover motion.

Most games use every button already, so a combo, a gesture or a back paddle makes a good clutch.

**Pause while pushing** turns gyro off while you push a stick, so the stick and gyro don't fight.

## 5. Tune the feel

- **Response** changes sensitivity with how fast you turn. *Precision* is slow for small movements, for sniping; *Turn boost* leaves aiming alone and speeds up big quick turns.
- Under **Advanced**, **Smooth below** steadies slow, careful aiming without delaying fast turns, and **Re-measure drift whenever the controller is set down still** keeps the baseline right as the controller warms up.
- The **Visualizers** draw the curve with your current turning speed on it. They follow the active profile's saved settings, so make this profile active and save to watch them.

## 6. Aim a game that only reads the gamepad

If the game doesn't read the mouse alongside the gamepad, pick **Gamepad stick (turn speed)** and send it to the **Right stick**. Turning the controller then pushes a virtual stick, on top of the real one. Set the game's own stick sensitivity first, then:

1. **Full tilt at**: how fast you turn for a full push. Lower it to turn the view further.
2. **Anti-deadzone**: raise it until slow turns move the view; lower it if the view creeps with the controller still.
3. **Game's curve**: if slow turns move the view less than fast ones, raise it a little at a time from 1 until they match.

## 7. Say if the profile needs gyro

Tick **Can't be played without gyro** (1) only if the profile depends on it, such as a flick-stick profile that aims up and down with gyro. Players without gyro then aren't offered it. Leave it unticked when gyro is an extra.

## When it feels wrong

- The view creeps with the controller on the table: **Calibrate gyro** again, then raise *Ignore jitter below* under **Advanced**.
- Slow aiming is shaky: raise **Smooth below**.
- The same turn of the controller turns each game differently: tick **Real-world sensitivity** and enter each game's full turn.
- Up and down feel backwards or too strong: **Invert vertical** and **Vertical speed**.

## Where next

- [Gyro](Gyro) for every setting.
- [Changing controls while you play](In-Game-Menu): Quick Settings changes gyro sensitivity or turns gyro on and off without leaving the game.
