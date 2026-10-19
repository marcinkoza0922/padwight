# Tutorial: a layer for hotkeys while you hold a button

A layer is a second set of mappings that's on only while you hold a button. You'll make a **Hotkeys** layer: hold LB and the face buttons press F1 to F4, and everything else stays as it was.

## 1. Make the layer and the button that holds it

1. Open the profile's **Buttons** tab and click **LB (Left Bumper)**.
2. Click its **Press** button and choose **Hold layer**, then **+ New layer** in the layer picker. A layer is made, with a name Padwight picks, and the button holds it. (The same works from the **Layers** tab with **+ New layer**.)
3. Open the setup's **Layers** tab. Rename the layer to *Hotkeys*.

## 2. Set what the layer changes

![The Layers tab with a layer picker, the name, the on-screen indicator and what the layer is shown over](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/layers-tab.png "A layer's settings. Its buttons are listed below, as on a profile.")

1. The picker at the top (1) chooses which layer you're editing, and **+ New layer** adds another.
2. **On screen while on** (2) says what shows while the layer is held: **Its name**, **Nothing**, one of the setup's info overlays, or **Its bindings (generated)**, a cheat sheet built from the layer's own buttons. **Show after** delays it, so a quick tap of the button doesn't flash it.
3. **Unset buttons: Do nothing while the layer is on** (3) turns off every button the layer doesn't set. Leave it unticked for hotkeys, so you can keep playing with the other buttons.
4. **Shown over** (4) is the profile whose buttons you see below, to compare with. The layer applies on top of whichever profile is active.

Below that, the layer has the same tabs as a profile: Buttons, Sticks & triggers, Combos, Gyro and Touchpad. A setting the layer doesn't change reads **Same as** the profile, with an **Override** button.

1. On the layer's **Buttons** tab, press **Override** on **A (South)**, and choose **Keys** and **F1**.
2. Do the same for B (F2), X (F3) and Y (F4).

**Back to base** removes an override again.

## 3. Save and try it

Press **Save & apply**. Hold LB and press A: the game gets F1. Let go of LB and A does what it did before.

## Good to know

- Layers stack. If two of them disagree about a button, the one turned on most recently wins.
- A layer can't be toggled on, and a menu item can't hold one. A set of controls that should stay on until you switch back is a [mode](Tutorial-Modes).
- A layer can also override a stick, the gyro or the touchpad: hold a button to slow the mouse stick down while aiming, say.
- The [Guide overlay](Tutorial-Guide) lists each layer a button holds in a section of its own.

## Where next

- [Layers](Layers) for the reference.
- [Modes](Tutorial-Modes) for control schemes you switch to and stay in.
