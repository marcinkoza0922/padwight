# Modes

A mode is another control scheme inside a game, which you switch to and stay in until you switch again. For example, a **Driving** mode where the sticks steer and the triggers accelerate, a minigame, or a **Menus** mode where the face buttons click and the D-pad presses the arrow keys.

Only one mode is on at a time. Switching to a mode turns off the one that was on, so incompatible schemes can't pile up. With no mode on, you're in the **default mode**: the profile itself. You won't see the default mode until a game has modes of its own.

Modes belong to a game. They're on its **Modes** tab, and they apply over whichever of its profiles is active. Anything a mode doesn't set stays as in the profile. A mode can override the same things a [layer](Layers) can: buttons and stick directions, gestures, sticks and triggers (with their zones), gyro, the touchpad and combos.

## Built-in modes

Every game also has two modes that can't be changed or removed. They're listed on the **Modes** tab.

- **Guide**: on while Guide is held or the [Guide overlay](Guide-Button) is up, with the system shortcuts. It belongs to the Guide button; nothing else switches to it.
- **Mouse mode**: the controller drives the mouse pointer. Turn it on from the Guide overlay, with Guide + R3, or with **Switch mode…** "Mouse mode" on any button. Guide (or that button again) leaves it.

While a built-in mode is on, it stands in for the game's mode, and held layers don't apply. When it ends, the mode you were in comes back.

## Switching modes

Map **Switch mode…** to any button, trigger, stick direction, zone, gesture, combo or menu item. Pressing it switches to that mode. Pressing it again while that mode is on goes back to the default mode.

- To get back out, give the mode the same **Switch mode…** on the same button, or any other way back.
- To move between several modes, a carousel or list [menu](Menus) of **Switch mode…** items works well.
- **Do nothing while the mode is on** (for unset buttons) never turns off buttons that switch modes or hold layers, so there's always a way back.

In the [in-game editor](Editing-Controls-in-Game), **Switch to a mode…** on a button's action list picks one of the game's modes.

## Modes, layers and profiles

- A **mode** is a change of context that you switch to and stay in: vehicles, minigames, the game's own menus.
- A [layer](Layers) extends or fine-tunes the mode you're in while a button is held: lean while LB is held, a slower mouse while aiming. Layers apply on top of the mode. A layer can't be toggled on, and menus can't hold layers; a layer that should stay on is a mode.
- A **profile** is a complete control scheme picked by the game's rules or from Quick Settings. Modes work over every profile of the game.

## Switching over

- An input that's down when the mode changes keeps doing what it started until it's released.
- Sticks, triggers and gyro switch right away.
- The mode stays on when a menu or the on-screen keyboard opens, and when the profile changes within the game. Switching to another game goes back to the default mode.
- Each controller has its own mode.

## Showing the mode

A mode can show its name on screen while it's on, with its own position, size and colours, or one of the game's info overlays, or nothing. The `{mode}` token in an [info overlay](Info-Overlays) shows the mode that's on, or "Default". The live controller drawing labels each input as it acts in the mode that's on.

The [Guide overlay](Guide-Button)'s Mappings show what the controls do in the mode you're in. Press **Left** or **Right** to see the other modes, the built-in ones included.

## Editing

The Modes tab works like the Layers tab: it uses the profile editor, shown over one of the game's profiles. Inputs a mode doesn't set read **Same as Gameplay: …** with an **Override** button; overridden ones have **Back to base**. **Copy from another game…** copies a mode with the macros, menus and info overlays it uses.

Older setups and packs that toggled a layer on and off are converted when they're loaded: the toggle becomes **Switch mode…**, and a layer that was only ever toggled becomes a mode.
