# Buttons

Every button in a profile has an action. Click the button's name to open it, then click the action to change it.

## What a button can do

- press a gamepad button (A can act as B, for example), or pull a gamepad trigger or push a stick
- press a key, or a key combination such as Left Ctrl + C
- press a mouse button, or turn the scroll wheel
- do nothing, with *Disabled*
- run a [macro](macros.md), step through a [cycle](cycles.md), open a menu, switch [modes](modes.md), hold a [layer](layers.md), or show an overlay
- do several of these at once

## Toggle and Turbo

Wrap an action in one or both to change how it presses:

- **Toggle**: press once to hold the action down, and again to let go. Good for crouching or running.
- **Turbo**: repeats the action 2 to 30 times a second while you hold the button.
- **Both**: a Turbo inside a Toggle gives auto-fire you switch on and off.

## Combos

Buttons pressed together can make an input of their own, such as LB + RB. After the first button goes down, Padwight waits a moment (60 ms by default) for the others before it treats the press as a single button.

A button set to *Disabled* works as a modifier, with no time limit.

## Gestures

A button can do something else on a double tap, a triple tap or a long press. Each gesture is listed as a row under the button's action. A tap on a button with gestures fires once Padwight is sure no second tap is coming; hold the button past that moment and its normal action starts at once.

> **Tip:** because Padwight waits to see if another tap follows, a button with gestures responds a little later.

### On press and On release

*On press* taps its action once as the button goes down, and *On release* taps its action once as it's let go. Holding the button in between does nothing more. Both work beside the button's own action and don't wait for the other gestures, so they add no delay. A trigger set to act as a button has them too, under its action on the Sticks & triggers tab.

Together they turn a game's toggle into a hold. If a game's aim down sights is a toggle on the right mouse button, set both to the right mouse button: pulling the trigger aims, letting go stops. The trigger's own action can then hold a layer with a finer aim for as long as the game aims. If the game drops the aim on its own (when you sprint, say), the two get out of step; tap the trigger once to bring them back.

## Back paddles

Xbox Elite, DualSense Edge and Steam Deck controllers have extra buttons on the back. They're in the **Back paddles** box under the buttons on the Buttons tab, and map like any other button.

Tick *Can't be played without back paddles* only if the profile depends on them. Players whose controller has none then aren't offered the profile, and the Guide overlay's Switch Profile skips it.

See also [Sticks and triggers](sticks.md) for zones, and [Modes](modes.md) and [Layers](layers.md) for changing many buttons at once.
