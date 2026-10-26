# Tutorial: remapping buttons, gestures and combos

You'll make A click, give B a second job on a long press, and make a button a toggle. Open any profile's **Buttons** tab to follow along.

## 1. Change what a button does

Each button has a row. Click its name to open it.

![A (South) opened on the Buttons tab, showing its action and the gesture buttons](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/buttons-row.png "An open button row.")

1. The action (1) shows what a plain press does. Click it to open the **Action for A (South)** dialog, pick **Mouse click**, then choose **Left** in the dropdown beside it: A now clicks.
2. A closed row shows the button's glyph and what it does. Gestures are listed under it: in the picture (2), X (West) types Semicolon, and Apostrophe on a long press.
3. The **Tap window** and **Long press after** sliders under the list set the timing for every button in the profile. Leave them for now.

The action dialog groups every kind, each with a line on what it does: **Press** (keys, mouse, scroll wheel, gamepad buttons, triggers and sticks, Disabled), **Change how it presses** (Toggle, Turbo, Several at once, Macro), **Open or switch** (menus, layers, modes, cycles), **On screen** (keyboard, numpad, Quick Settings, media controls, info and log overlays), **Gyro** and **System**. Cancel, or a click beside the dialog, leaves the action as it was.

## 2. Give the same button a second job

Under the action are **+ Double tap**, **+ Triple tap** and **+ Long press** (3).

1. Open B and click **+ Long press**.
2. Choose what a long press does, for example **Keys** and **Esc**.
3. Press **Save & apply** and try it: a quick tap does B's normal action, and holding B does the second.

A button with gestures answers a little later, because Padwight waits to see whether another tap is coming. That's fine for a reload button that changes ammo on a long press, and bad for a jump button.

## 3. Hold it down with Toggle

Games that make you hold a button to crouch or run are easier with a toggle.

1. Open the button's row, click its action and choose **Toggle on / off…**.
2. Pick the key or button to hold. The first press holds it down; the next lets go.

**Turbo…** presses the action over and over, 2 to 30 times a second, while you hold the button. Put a Turbo inside a Toggle for auto-fire you switch on and off.

## 4. Make a combo

A combo is two or more buttons pressed together that act as an input of their own, such as LB + RB.

1. Open the profile's **Combos** tab and press **+ Add combo**.
2. Choose the buttons and the action.

After the first button goes down, Padwight waits a moment (60 ms by default) for the others, so every button in a combo reacts a little later. A button set to *Disabled* works as a modifier with no wait.

## 5. Save and check

Press **Save & apply**. **Live input** at the top of the profile (press **Show** if it's folded) draws your controller with each input labeled. A blue `+` after a label means the button also has a gesture; point at it to see what.

## Where next

- [Buttons](Buttons-and-Actions) for the full list, back paddles and gestures.
- [Layers](Tutorial-Layers) change many buttons at once while you hold one.
