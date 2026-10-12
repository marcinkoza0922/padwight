# Tutorial: a Driving mode you switch to and stay in

A mode is another control scheme inside a game. You switch to it, and you stay there until you switch again. You'll make a **Driving** mode where the sticks and triggers steer and accelerate, and a button that switches in and out of it.

## 1. Make the mode and the switch

1. On the profile's **Buttons** tab, click **Y (North)**.
2. Choose **Switch mode…**, then **+ New mode** in the picker. A mode is made, and Y switches to it.
3. Open the setup's **Modes** tab and rename the mode to *Driving*.

## 2. Set the mode's controls

![The Modes tab with a mode picker, the built-in modes, the name, the on-screen indicator and the unset-buttons switch](../images/manual/modes-tab.png "A mode's settings. Below them, the mode has the same tabs as a profile.")

1. The picker (1) chooses which mode to edit, **+ New mode** adds one.
2. **Built-in modes** (2) lists the two every setup has: **Guide**, on while the Guide overlay is up, and **Mouse mode**, where the controller drives the pointer. They can't be changed or removed.
3. **On screen while on** (3) works as for a layer: the mode's name, nothing, an info overlay or a generated bindings sheet.
4. **Unset buttons: Do nothing while the mode is on** (4) turns off every button the mode doesn't set, except the ones that switch modes or hold layers, so there's always a way back.

Now give the mode its own controls on its tabs. A setting the mode doesn't change reads **Same as** the profile; **Override** lets the mode set its own value.

1. On the **Sticks & triggers** tab, override the sticks and triggers the game should read differently. If the profile maps the left stick to keys, for example, override it to a **Gamepad stick** so the game gets analog steering.
2. On the **Buttons** tab, override what you need, such as A for handbrake.

## 3. Add the way back

Inside the Driving mode, give **Y (North)** the action **Switch mode…** *Driving* as well. Pressing the switch while that mode is on goes back to the default mode. (To go from one mode straight to another, give the first mode a **Switch mode…** for the second.)

## 4. Save and try it

Press **Save & apply**. Tap Y: the mode's name shows on screen and the controls change. Tap Y again to leave.

## Good to know

- Only one mode is on at a time, on top of the profile. With none on, you're in the *default mode*: the profile itself.
- A mode stays on when a menu or the on-screen keyboard opens, and when the profile changes within the same game. It goes back to the default when you switch to another game.
- For a mode that changes every time you pick from a list, put **Switch mode…** items in a list or carousel [menu](tutorial-menus.md).
- Mode or layer? A mode is for a change of context (driving, flying, a minigame). A [layer](tutorial-layers.md) is for extra controls while you hold a button.

## Where next

- [Modes](modes.md) for the reference.
- The [Guide overlay](tutorial-guide.md) shows what each mode does.
