# Layers

A layer changes some of a controller's mappings while you hold a button, then changes them back when you let go. For example: hold LB, and the face buttons type F1 to F4 and the right stick scrolls instead of moving the mouse. Or hold the aim button, and the mouse stick slows down. It works like Shift on a keyboard.

Layers belong to a game. They're on its **Layers** tab, and they apply over whichever of its profiles and [modes](Modes) are on. Anything a layer doesn't set stays as it was.

A layer can override:

- buttons and stick directions,
- gestures,
- sticks and triggers (with their zones),
- gyro and the touchpad,
- combos (add combos, or switch the profile's combos off).

## Holding a layer

Map **Hold layer…** to any button, trigger, stick direction, zone, gesture or combo. The layer is on while that input is held.

Layers can hold further layers.

A layer can't be toggled on, and menu items can't hold layers. If a set of mappings should stay on until you switch back, make it a [mode](Modes) instead.

In the [in-game editor](Editing-Controls-in-Game), **Add layer** makes a layer, and **Hold a layer…** on a button's action list makes that button hold it.

## Layers or modes

Use a layer to extend or fine-tune the controls while a button is held, such as a Lean layer that makes the left stick lean. Use a [mode](Modes) for a different control scheme for part of play, such as driving or using a vehicle. Layers apply on top of the mode that's on.

## Several layers at once

Layers stack. Where two layers disagree, the one turned on most recently wins.

## Switching over

- An input that's down when a layer comes on or goes off keeps doing what it started until it's released.
- Sticks, triggers and gyro switch right away.
- Held layers end when the profile changes, or when an on-screen menu or keyboard opens.

## Showing a layer

Each layer can show its name on screen while it's on, with its own position, size and colours. A layer can instead show one of the game's info overlays, such as a cheat sheet. Quick layers, such as "hold Y to lean", can show nothing at all.

The live controller drawing labels each input with the mapping of the layers that are on. See [Info overlays](Info-Overlays). The [Guide overlay](Guide-Button)'s Mappings give each layer a button holds a section of its own, with what it changes.

## Editing

The Layers tab uses the same profile editor as the Profiles tab, shown over one of the game's profiles.

- Inputs a layer doesn't set read **Same as Gameplay: …**. Click **Override** to set one.
- Overridden inputs have **Back to base** to undo that.

Layers use their profile's timings (combo window, tap window and so on).

**Copy from another game…** copies a layer along with the macros, menus and info overlays it uses. This is handy for sequels.

Shared items can't use layers, because layers always belong to one game.

The Guide button's shortcuts aren't a layer and don't appear on this tab. See [The Guide button](Guide-Button).
