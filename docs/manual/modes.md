# Modes

A mode is another control scheme inside a game, which you switch to and stay in until you switch again. For example, a **Driving** mode where the sticks steer and the triggers accelerate, or a **Menus** mode where the face buttons click and the D-pad presses the arrow keys. Anything a mode doesn't change keeps its normal mapping.

Only one mode is on at a time. Switching to a mode turns off the one that was on. With no mode on, you're in the **default mode**: the profile itself. You won't notice the default mode until a setup has modes of its own.

## Built-in modes

Every setup also has two modes of its own, which can't be changed or removed. They're listed on the **Modes** tab.

- **Guide**: on while you hold Guide or its overlay is up, with the [Guide shortcuts](guide-button.md), which you can move on App settings. It belongs to the Guide button (or whatever you moved that to), so nothing else switches to it.
- **Mouse mode**: the controller drives the mouse pointer. Turn it on from the Guide overlay, with Guide + Right Stick, or with **Switch mode…** set to "Mouse mode" on any button. Press Guide (or whatever opens the overlay, or that button again) to leave it. Its keys can be moved on App settings; see [The Guide button](guide-button.md).

While a built-in mode is on, it stands in for the setup's mode, and held layers don't apply. When it ends, the mode you were in comes back.

## Switching modes

Choose **Switch mode…** as the action of any button, trigger, stick direction, zone, gesture, combo or menu item. Pressing it switches to that mode. Pressing it again while that mode is on goes back to the default mode.

To get from one mode straight to another, give the mode a **Switch mode…** of its own, such as Y for "Driving" in the default mode and Y for "Driving" inside the Driving mode to get out again. A carousel or list [menu](menus.md) of modes works well when a game has several.

A mode stays on when a menu or the on-screen keyboard opens, and when the profile changes within the same game. It goes back to the default when you switch to another game.

## Modes or layers

- Use a **mode** for a change of context: driving, flying, a minigame, the game's menus. You switch to it and stay there.
- Use a [layer](layers.md) to extend or fine-tune whichever mode you're in while you hold a button: lean while LB is held, or slow the mouse down while aiming. Layers apply on top of the mode.

Layers can't be toggled on, and menus can't hold them. A layer that should stay on is a mode.

## Modes or profiles

Profiles are complete control schemes picked by the game's rules or from Quick Settings. A mode is a change over whichever profile is active, switched from the controller. A "hold Y to lean" layer or a "Driving" mode works the same over each of the setup's profiles.

## Showing the mode

Like a layer, a mode can show its name on screen while it's on, show one of the setup's info overlays (such as a cheat sheet), or show nothing. The `{mode}` token in an info overlay shows the mode that's on, or "Default".

The [Guide overlay](guide-button.md)'s Mappings show the mode you're in. Press Left or Right to look at the other modes, the built-in ones included.

## Editing

Modes are edited on the setup's **Modes** tab, the same way as layers. A setting the mode doesn't change reads "Same as" the profile it's shown over, with an **Override** button. **Override** lets the mode set its own value, and **Back to base** removes that value again.

**Unset buttons: Do nothing while the mode is on** turns off every button the mode doesn't set, except the ones that switch modes or hold layers, so there's always a way back.
