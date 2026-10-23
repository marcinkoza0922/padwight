# Tutorial: a layer for hotkeys while you hold a button

A layer is a second set of mappings that's on while you hold a button. You'll make a **Hotkeys** layer: hold LB and the face buttons press F1 to F4, with everything else unchanged.

## 1. Make the layer and the button that holds it

1. Open the profile's **Buttons** tab and click **LB (Left Bumper)**.
2. Click its action and choose **Hold layer…**, then **+ New layer** in the layer picker. A layer is made and LB holds it.
3. Open the setup's **Layers** tab. Rename the layer to *Hotkeys*.

## 2. Set what the layer changes

![The Layers tab with a layer picker, the name, the on-screen indicator and what the layer is shown over](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/layers-tab.png "A layer's settings. Its buttons are listed below, as on a profile.")

1. The picker at the top (1) chooses which layer you're editing, and **+ New layer** adds another.
2. **On screen while on** (2): **Its name**, **Nothing**, one of the setup's info overlays, or **Its bindings (generated)**, a cheat sheet of the layer's own buttons. **Show after** delays it, so a quick tap doesn't flash it.
3. **Unset buttons: Do nothing while the layer is on** (3) turns off every button the layer doesn't set. Leave it unticked for hotkeys, so the other buttons keep working.
4. **Shown over** (4) is the profile shown below for comparison. The layer applies over whichever profile is active.

Below, the layer has the same tabs as a profile. A setting the layer doesn't change reads **Same as** the profile, with an **Override** button.

1. On the layer's **Buttons** tab, press **Override** on **A (South)**, and choose **Keys** and **F1**.
2. Do the same for B (F2), X (F3) and Y (F4).

**Back to base** removes an override again.

## 3. Save and try it

Press **Save & apply**. Hold LB and press A: the game gets F1. Let go of LB and A does what it did before.

## Good to know

- Layers stack. If two disagree about a button, the one turned on last wins.
- A layer can't be toggled on, and a menu item can't hold one. Controls that should stay on are a [mode](Tutorial-Modes).
- A layer can also override a stick, the gyro or the touchpad, to slow the mouse stick while aiming, say.
- The [Guide overlay](Tutorial-Guide) lists each layer a button holds in a section of its own.

## Where next

- [Layers](Layers) for the reference.
- [Modes](Tutorial-Modes) for control schemes you switch to and stay in.
