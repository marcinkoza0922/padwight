# Modes

A mode is another control scheme inside a game that you switch to and stay in, such as a **Driving** mode where the sticks steer and the triggers accelerate. Anything a mode doesn't change keeps its normal mapping.

Only one mode is on at a time. With none on, you're in the **default mode**: the profile itself.

## Built-in modes

Every setup also has two built-in modes, listed on the **Modes** tab, which can't be changed or removed:

- **Guide**: on while you hold Guide or its overlay is up, with the [Guide shortcuts](guide-button.md). Only the Guide button (or what you moved it to) turns it on.
- **Mouse mode**: the controller drives the mouse pointer. Turn it on from the Guide overlay, with Guide + Right Stick, or with **Switch mode…** "Mouse mode" on any button. Press Guide to leave it.

While a built-in mode is on, held layers don't apply. When it ends, the mode you were in comes back.

## Switching modes

Choose **Switch mode…** as the action of any button, trigger, stick direction, zone, gesture, combo or menu item. Pressing it switches to that mode; pressing it again while that mode is on goes back to the default mode. So to leave Driving with the same Y that entered it, give Y the same **Switch mode…** "Driving" inside the mode too. A list or carousel [menu](menus.md) of modes suits a game with several.

A mode stays on when a menu or the on-screen keyboard opens, and when the profile changes within the same game. Switching to another game goes back to the default.

## Modes or layers

- A **mode** is for a change of context: driving, flying, a minigame, the game's menus. You switch to it and stay there.
- A [layer](layers.md) changes some controls while you hold a button, on top of the mode: lean while LB is held, or slow the mouse while aiming. Layers can't be toggled on, so a layer that should stay on is a mode.

## Modes or profiles

A profile is a complete control scheme, picked by the setup's rules or by hand. A mode is a change over whichever profile is active, switched from the controller, and works the same over each of the setup's profiles.

## Showing the mode

While it's on, a mode can show its name, one of the setup's info overlays, a generated sheet of its bindings, or nothing. The `{mode}` token in an info overlay shows the mode that's on, or "Default".

The [Guide overlay](guide-button.md)'s Mappings show the mode you're in; press Left or Right to see the others.

## Editing

Modes are edited on the setup's **Modes** tab. A setting the mode doesn't change reads "Same as" the profile shown under **Shown over**; **Override** gives the mode its own value, and **Back to base** removes it.

**Unset buttons: Do nothing while the mode is on** turns off every button the mode doesn't set, except those that switch modes or hold layers, so there's always a way back.
