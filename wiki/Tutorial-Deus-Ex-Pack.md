# Tutorial: making a pack, step by step (Deus Ex, Xbox-style pad)

This walks through building `packs/deus-ex.padpack`, the pack that ships with the app. You'll build it in the settings window, one input at a time, then export it. The finished file is the reference: every mapping below is in it. It uses most of what the other tutorials teach, one feature at a time: [buttons](Tutorial-Buttons), [sticks](Tutorial-Sticks), [menus](Tutorial-Menus), [gyro](Tutorial-Gyro), [the Guide button](Tutorial-Guide) and [sharing](Tutorial-Sharing). Read the one for a step if you want it explained from scratch.

The pack targets a plain **XInput** pad (an Xbox-style controller) and the game's default keys. Deus Ex has no controller support of its own, so the whole scheme is built here. Its layout follows the [game setup guidelines](https://github.com/marcinkoza0922/padwight/blob/main/docs/development/game-setup-guidelines.md); each step says which guideline a choice comes from.

**About the keys.** Every key below is from the game's own `System/DefUser.ini`, with the game command it runs.

**About the screenshots.** They show the finished Deus Ex pack from the library, so each one is the end state of a step. Your own setup will look emptier until you've built it. Orange numbers mark the parts a step talks about. The live controller drawing shows labels but no button presses, since the pictures were taken without a controller connected.

## What you'll end up with

One setup, **Deus Ex**, with three profiles:

| Profile | For | Needs gyro? |
|---|---|---|
| **Deus Ex** | Plain pad, mouse-look on the right stick | No |
| **Deus Ex + Gyro** | The same, plus gyro mouse aiming, paused while LT is held | No (gyro is an extra) |
| **Deus Ex + Flick stick** | Right stick turns the camera in flicks; gyro aims up and down | Yes |

The setup also gets two radial menus (belt and augmentations) and a list of the game's screens. The Guide button needs no setup: it opens the Guide overlay in every game, and the pack's Mapping guide and notes describe the controls there.

## Before you start

![The Overview page: the sidebar of setups, what's happening, and the controller list](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/deus-overview.png "The Overview page. Your controller is in the list, and the sidebar shows your setups.")

1. Make sure the daemon is running (`padwight status` should list your controller, as does the Overview page). Open the settings window with `padwight`.
2. Launch Deus Ex once. Padwight needs to see the game to fill in the rule values, and the setup's **Details** tab lists the windows it has seen under **Recently focused windows**.

## Step 1: Create the setup

1. In the sidebar, click **+ Add setup**, then open **Empty setup from a template…** in the picker and choose **Gamepad pass-through**, so you start with no preset mappings.
2. The new setup opens on its **Details** tab, named *New setup*. Name it **Deus Ex**. The setup's name becomes the pack's name when you export it.

![The Add a setup picker, with the Empty setup from a template dropdown and the library's built-in setups](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/first-setup-add.png "The picker. The dropdown (1) holds the blank template. The list (3) is the library, which is where the finished pack comes from in these screenshots.")

The [first setup tutorial](Tutorial-First-Setup) covers the picker and the rules.

## Step 2: Set up the profile

Open the setup's **Profiles** tab.

1. Rename the profile to **Deus Ex** in the **Name** box.
2. Set the timing to the pack's values on the Buttons tab: **Tap window 250 ms**, **Long press after 500 ms**. These are the defaults, and the pack writes them out explicitly.

*Why these timings:* the tap window is how long padwight waits after a tap to see whether a second tap follows. A button with a double-tap gesture holds its tap for that window, so a shorter window is snappier but makes double taps harder to land. The long press at 500 ms is long enough that a normal tap never counts as one.

**Combo window** is set to 120 ms but the pack has **no combos**. A combo makes each of its buttons wait for the rest of the combo, which delays every press. The pack avoids combos on purpose. The window does nothing here.

## Step 3: Face buttons and the D-pad

![The controller drawing at the top of the Deus Ex profile, labeling each input with its action](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/deus-picture.png "The drawing labels each input with its action. A trailing + means the button also has a gesture.")

![The Buttons tab of the Deus Ex profile: each button's row with a summary of its mapping](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/deus-buttons.png "The Buttons tab. A closed row shows a one-line summary.")

On the profile's **Buttons** sub-tab, set each button. The keys are the game's defaults, from its `DefUser.ini`:

| Button | Action | Game command |
|---|---|---|
| **A** (South) | **Mouse Right** | Use / pick up (`ParseRightClick`). |
| **B** (East) | Keyboard **Space** | Jump. |
| **X** (West) | Keyboard **`;`** (`KEY_SEMICOLON`) | Reload. Gets a long press in Step 7. |
| **Y** (North) | Keyboard **Tab** | Drop / throw (`DropItem`). |
| **Start** | Keyboard **Esc** | Main menu. Gets gestures in Step 7. |
| **D-pad Up** | Mouse wheel **Up** | Previous belt item. |
| **D-pad Down** | Mouse wheel **Down** | Next belt item. |
| **D-pad Left** | Keyboard **Q** | Lean left. |
| **D-pad Right** | Keyboard **E** | Lean right. |

Select gets a menu in Step 4. Guide is the app's own and needs nothing.

**Why these** (from the [game setup guidelines](https://github.com/marcinkoza0922/padwight/blob/main/docs/development/game-setup-guidelines.md)):

- **Use on A, Jump on B.** Deus Ex's Use is the right mouse button. Whatever is on the right mouse button normally goes on LT, but when it's Use, A or X is preferred. When a game has both Use and Jump, Use takes A and Jump takes B.
- **Drop on Y.** It's a deliberate action that wants its own button, and Y is the face button left over.
- **Lean on D-pad left and right, belt cycling on up and down.** Lean is a mirrored pair, so it takes the D-pad's sides. The game cycles the belt with the mouse wheel, and scrolling can go on the D-pad.

## Step 4: Menus

![The Belt menu on the Menus tab: the add-a-menu buttons, the menu's kind and its items](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/menus-tab.png "The Belt menu, expanded. The list sets each slot's key, and a preview under it shows where each slot sits around the right stick.")

The [menus tutorial](Tutorial-Menus) goes through the kinds.

1. **LB → Open menu "Belt"**, **RB → Open menu "Augmentations"**, **Select → Open menu "Screens"** (on the Buttons tab, pick "Open menu…").
2. On the **Menus** tab, press a button next to **Add a menu:** for each one:
   - **Belt**: **Radial**, aimed with the **Right Stick**. Items `Belt 1` to `Belt 9` on keys `1` to `9`, `Nanokey ring` on `0`, and `Holster` on **Backspace**.
   - **Augmentations**: **Radial**, aimed with the **Right Stick**. Ten items, `Aug 1 · F3` to `Aug 10 · F12`, on keys `F3` to `F12`.
   - **Screens**: **List**. `Inventory` on **F1** and `Goals and notes` on **F2**.

**Why this design:**

- **Radials on the shoulder buttons for the number and function keys.** The game uses all of 0 to 9 for the belt and F3 to F12 for augmentations, which is what radial menus are for. Every hotkey is an item. Hold the bumper, aim with the right stick, let go to pick.
- **Holster on the belt too.** Holstering belongs on the input that took the item out. Step 7 explains why that's a double tap rather than a long press, and the menu item makes it easy to find.
- **Slot 0 is named.** The nanokey ring always sits in belt slot 0, so the label says so. The other slots change with what you carry.
- **A list on Select for the game's screens.** The game has separate keys for the inventory and the goals screen, so Select opens a list of them. Tap Select, move with the D-pad and press A, which opens the screen and closes the list. The list has a close button, so it stays up after the tap; Select or B closes it without picking. The other screens (health, augmentations, skills, conversations, images, logs) are tabs inside the inventory and have no default keys.

## Step 5: The sticks

![The Sticks & triggers tab, showing the left stick set to a button ring of eight sectors that hold W, A, S and D](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/sticks-ring.png "The left stick as a button ring. The zone and the right stick's settings sit below.")

The [sticks tutorial](Tutorial-Sticks) explains each control.

### Left stick: walking and running

For the **left stick**:

- Action: **Button ring** with **8** sectors. Press **WASD** next to *Fill with*: up is **W**, down **S**, left **A**, right **D**, and the diagonals press two keys at once.
- **Deadzone** 12%, **Starts at** 25% of the way out, **Stickiness** 10% of a sector.
- Add a **zone** from **0.00 to 0.60** with action **Keyboard** and **Left Shift**.

![The left stick's zone, from 0.00 to 0.60 with Left Shift, and the right stick set to Mouse pointer](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/sticks-zones.png "The walk zone (1), and below it the right stick's mouse settings (3, 4).")

The zone is the walk/run split. The game's Shift is `Walking`, so while the stick is in the first 60% of its travel Shift is held: a light push walks and a full push runs. Deus Ex has no sprint, so A has no long press.

- **Starts at 25%.** A sector's keys press once the stick passes this point, so movement starts early. The response curve only affects mouse and scroll output, so it does nothing for keys.

### Right stick: mouse look

For the **right stick**: **Mouse pointer**, **Speed** **2600** px/s, **Deadzone** 12%, and under **Response**, **Custom** with **Curve** **2.20**, **Acceleration** 50% with a **Ramp time** of **350 ms**. Open **Advanced** and set **Vertical speed** to **0.75**.

This is shaped like a console shooter's look, as the guidelines ask:

- **Radial deadzone and a power curve.** Curve 2.2 keeps small pushes slow for fine aim and gives full speed at full push.
- **Acceleration at full deflection.** Held all the way over, the turn speeds up by half again over 350 ms, the way console shooters ramp up a turn.
- **Slower vertical.** Up and down at 0.75 of the horizontal speed, as most console shooters do.
- **Speed.** 2600 px/s turns roughly 120 degrees a second at the game's default mouse sensitivity, before the ramp. *(Estimated from the flick profile's 8000 px per turn; adjust to taste.)*

On the game's own screens the right stick is the mouse cursor, so the inventory, computers and conversations work with it and RT and A as the clicks.

### Stick clicks

- **Left stick click (L3)**: a **toggle** of keyboard **X** (`Duck`). Crouch is a hold in the game, and the toggle makes one click crouch and the next stand. To wrap a button's action in a toggle, pick **Toggle on / off…** on that button.
- **Right stick click (R3)**: keyboard **`[`** (`ToggleScope`). Its long press, in Step 7, is the laser sight.

**Why the scope is on R3:** zoom goes on R3 or D-pad Up when it isn't on the right mouse button. The game's scope is a toggle, so the button is a plain key and the game does the toggling. The laser sight is a related aiming aid, so it shares the button as a long press.

## Step 6: The triggers

- **Right trigger (RT)**: works as a button, with a **threshold** of 0.30, and the action **Mouse button** **Left** (fire).
- **Left trigger (LT)**: **Disabled**.

Deus Ex has no secondary fire, and its right mouse button is Use, which went on A. Rather than invent a job for LT, the plain profile leaves it free. The gyro profiles use it as the gyro clutch (Step 11).

## Step 7: Gestures (one button, several jobs)

To add a gesture, open the button's row on the Buttons tab and press **+ Double tap**, **+ Triple tap** or **+ Long press** under its plain action. Then choose what the gesture does, the same way as for the plain press.

![A button's row with its action and the add-a-gesture buttons](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/buttons-row.png "The gesture buttons (3) sit under a button's action.")

| Button | Gesture | Action | Mapping guide text |
|---|---|---|---|
| **X** | Long press | Keyboard `'` (`SwitchAmmo`) | "Change ammo" |
| **R3** | Long press | Keyboard `]` (`ToggleLaser`) | "Laser sight" |
| **LB** | Double tap | Keyboard **Backspace** (`PutInHand`) | "Holster" |
| **Start** | Double tap | Keyboard **Keypad Plus** (`QuickSave`) | "Quick save" |
| **Start** | Long press | Keyboard **Keypad Slash** (`QuickLoad`) | "Quick load" |

**Why these:**

- **Change ammo on a long press of Reload.** The guidelines pair them: reload is the tap, changing ammo the hold.
- **Holster on a double tap of LB.** The guidelines put holster on a long press of the button that took the item out. On a button that holds a menu, a long press would stop the menu from staying up while held, so the belt button gets a double tap instead, and the belt menu has a Holster item.
- **Start: save on double tap, load on long press.** The guidelines' system buttons. Loading throws away your progress, so it takes the deliberate gesture.

> **Latency cost.** A tap on a button with a gesture waits to see which gesture it is: up to the tap window (250 ms) for a double tap, or until release for a long press. Reload, the scope and Start fire on release, and the belt menu opens after the tap window.

## Step 8: The Guide button, the Mapping guide and the notes

Nothing to set up for the Guide button itself. It works the same in every game: tap it for the Guide overlay, hold it with another button for a shortcut (keyboard on X, numpad on Y, and so on). See [The Guide button](Guide-Button) and [the Guide tutorial](Tutorial-Guide). The pack maps Guide to the built-in Guide mode, as every library pack does.

The pack describes its controls in the **Mapping guide**, on the profile's **Guide** tab, which is what the overlay's Mappings box shows. The Lean and belt-cycling rows each merge both D-pad sides, with **Share with**.

![The Guide tab of the Deus Ex profile: the notes, and the Mapping guide's rows with their text](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/guide-tab.png "The notes (1) and the Mapping guide (2).")

The same tab holds the **notes**, for what a player wouldn't guess from the layout:

- Start (Escape) backs out of every in-game screen: the inventory, goals, computers, keypads, robot recharging and repair bots. Without the note, a player hunting for a "back" button on B would be stuck.
- On those screens the right stick is the cursor, RT a left click and A a right click.
- Tap Select for the list of the inventory and goals screens; Select or B closes it.
- Keypads take Guide + Y (numpad), computer logins Guide + X (keyboard).
- A light push of the left stick walks, a full push runs.
- The scope is a toggle, slot 0 is the nanokey ring (double tap LB, or pick Holster on the belt, to put the item away), and the augmentation slots follow the order you installed them.

## Step 9: Look and sound

Overlays should look and sound like the game. The pack uses the Rajdhani font and a dark steel-blue palette with square corners and a thin light outline, in the spirit of the game's menus, on the menus, the keyboard, the numpad, the media controls and the in-game menu. Its menus click and beep like the game's computers, and overlays snap in and out (`brutal` motion), the way the game's own windows do.

## Step 10: Check every input in the game

1. Watch the live controller drawing in the profile editor, or use **Find by pressing** there. Press each button and confirm it shows the right action.
2. Walk and run: a light push walks, a full push runs.
3. Aim: the right stick should feel like a console shooter, and speed up when held all the way over.
4. Fire and scope: RT fires, R3 raises and lowers the scope, holding R3 toggles the laser.
5. Menus: hold LB and RB, aim, and release. Tap Select and pick Inventory with A; tap Select again and close the list with B.
6. Gestures: double tap LB, double tap Start, long press Start, and long press X.
7. Screens: open a keypad, use Guide + Y, and back out with Start.

If something doesn't work, check the rule first (is the executable right?), then that the profile is active (the Overview shows which), and then the mapping.

## Step 11: The gyro profile: "Deus Ex + Gyro"

![The Gyro tab of the Deus Ex + Gyro profile: the mode, the sensitivity and the clutch](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/gyro-panel.png "The Gyro tab of the finished Deus Ex + Gyro profile. The base profile's gyro is off; the steps below set these controls on the copy. See the gyro tutorial for each control.")

Shooters always get gyro and flick stick profiles to try, and they're never the default: the rules start on the plain profile.

1. On the **Profiles** tab, duplicate the **Deus Ex** profile, and rename the copy to **Deus Ex + Gyro**.
2. On its **Gyro** tab:
   - **Gyro**: **Mouse (gyro aiming)**, **Sensitivity** **30** (pixels per degree turned).
   - **Turning read as**: **Player space (any grip)** (turning left aims left however you hold the controller).
   - Under **Advanced**, **Ignore jitter below**: 1.0 °/s.
3. On its **Buttons** tab, set the **Left Trigger** to **Gyro clutch (off while held)**.
4. Leave **Can't be played without gyro** unticked.

**Why:** gyro adds fine aim on top of the right stick, so the profile still plays without it and doesn't declare it. The clutch turns gyro off while held, to reposition the controller, and LT is the input this scheme leaves free.

## Step 12: The flick stick profile: "Deus Ex + Flick stick"

A flick stick turns the camera by flicking the right stick, instead of holding it over to aim. It only turns left and right, so looking up and down needs gyro, which is why this profile **requires** gyro.

1. Duplicate the **Deus Ex + Gyro** profile, and rename the copy to **Deus Ex + Flick stick**.
2. On its **Sticks & triggers** tab, set the **right stick** to **Flick stick**: **Turn size** **8000** px per 360°, **Flick at** 90% of the way out, **Deadzone** 10%. Under **Advanced flick**, set the flick time to **100 ms**, smoothing to **0 ms**, the dead angle to **0°** and looking up and down to off. Use **Test turn** to check that 8000 px turns the camera a full circle, and adjust.

![The right stick set to Flick stick: turn size and Test turn, flick point and deadzone, and the Advanced flick fold](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/deus-flick.png "The flick stick's settings.")
3. On its **Gyro** tab: sensitivity **22.0**, which is about one to one with that turn size. Keep the LT clutch.
4. Tick **Can't be played without gyro**.

> **Calibration.** Turn size is the one value that depends on your game settings. Set it with Test turn, and expect to change it if you change the game's sensitivity.

## Step 13: Describe the pack and export it

![The Auto-switch rules card of the Details tab: DeusEx.exe and Steam App ID 6910, each using the Deus Ex profile](../images/manual/first-setup-rules.png "The auto-switch rules on the Details tab are what Padwight matched when you launched the game in \"Before you start\".")

1. Open the setup's **Details** tab and check the rules, then press **Export…** in the **Pack** card.
2. The **Export** window lists what's in the pack (here, 3 profiles, 3 menus), the profiles' declared requirements (the flick profile needs gyro, and the gyro profile is playable without it), and any dangling references (a button pointing at a missing macro or menu). Fix any warnings before saving. Then fill in:
   - **Version**: `1.0` for a first release (the library pack is at `2.0`). Later, bump it. Versions compare number by number (`1.10` is newer than `1.9`), and a higher version gives installed copies the *update* mark.
   - **Author**: your name.
   - **Made with**: "Xbox controller".
   - **Description**: what the pack does and how it's laid out. The pack's own description is: "The original Deus Ex (2000) on an Xbox-style pad, for the game's default keys. A uses, B jumps, RT fires…"

![The Export window with the pack's contents, version, author, made-with and description](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/sharing-export.png "The Export window (shown for the StarCraft setup).")

3. Press **Save…** and save as `deus-ex.padpack`.
4. Press **Save & apply** to remember the export details in the setup.

The [sharing tutorial](Tutorial-Sharing) covers this window and importing.

### Test the export by importing it

1. In the sidebar, click **+ Add setup**, then **Import a file…**, and choose the file.
2. The preview shows the contents, your controller's gaps (on a plain pad, the flick profile is not offered), and any clashes. Since you already have a Deus Ex setup, the preview asks you to rename or replace. Use **Rename** to test, then delete the test copy.

## Step 14 (maintainers): add it to the library

The library is the set of packs that ship with the app, in `packs/`.

1. In a debug build, tick **Release it as the built-in …** on export, so the setup keeps its library ID and installed copies get it as an update.
2. Copy the file into `packs/`, and run `cargo test`. The tests check the format, IDs, names, references, that a rule points at an existing profile, and that at least one profile works on a plain pad.
3. Rebuild. The build embeds every `.padpack` in `packs/` into the binary. Developers can find the details in the [architecture notes](https://github.com/marcinkoza0922/padwight/blob/main/docs/development/architecture.md).

## Summary: the less obvious choices

| Choice | Guideline |
|---|---|
| Use (right mouse) on A, Jump on B | Right mouse goes on LT unless it's Use; with Use and Jump, Use takes A. |
| LT free in the plain profile | No secondary fire, and right mouse is Use. The gyro profiles clutch on it. |
| Scope on R3, laser on its long press | Zoom goes on R3 or D-pad Up; related actions share a button. |
| Crouch toggle on L3 | Crouch is a toggle, on L3 (B is taken by Jump). |
| Walk on a 60% Shift zone | Walk is a left-stick zone over the first 50 to 70%. |
| Lean on D-pad left/right, belt cycling on up/down | Mirrored actions and scrolling go on the D-pad. |
| Change ammo on a long press of Reload | They share a button. |
| Radials on LB and RB | Number and function key hotkeys go on radial menus on the shoulders, every hotkey an item. |
| Holster on a double tap of LB, and in the belt | Holster goes on the input that took the item out; a long press would break the held menu. |
| Inventory and goals in a list on Select | Separate screen keys go in a list on Select; a list with a close button stays up when Select is let go. |
| Start: Esc, double tap save, long press load | The system buttons. |
| Notes on backing out with Start | Notes say what a player wouldn't guess. |
| Console-like right stick | Shooters get a radial deadzone, a power curve and console-ish speed. |
| Gyro and flick stick profiles, never the default | Always offered for shooters; the rules start on the plain profile. |

## Troubleshooting

- **The rule doesn't switch to the profile.** Check the executable name (for Proton games, the Windows `.exe`, e.g. `DeusEx.exe`) and the Steam App ID. The **Details** tab's recently focused windows show what padwight saw.
- **The game sees two controllers.** A PlayStation or Switch controller can reach the game directly too. Set `SDL_JOYSTICK_HIDAPI=0` (see [Troubleshooting](Troubleshooting)).
- **A button feels late.** It has a gesture, so it waits for the tap window. See the latency note in Step 7.
- **Guide's shortcuts don't work.** Hold Guide and press the shortcut, or tap Guide to open the overlay and then press it. See [The Guide button](Guide-Button).
- **A row in the Mapping guide reads wrong.** Fix the text on the profile's **Guide** tab.
- **The flick profile is missing.** It needs gyro, so with a plain pad it isn't offered. The import preview lists profiles it won't offer.
