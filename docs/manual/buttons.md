# Buttons

Every button in a profile has an action. Click the button's name to open it, then click the action to change it.

## What a button can do

- press a gamepad button (A can act as B, for example), or pull a gamepad trigger or push a stick
- press a key, or a key combination such as Left Ctrl + C
- press a mouse button, or turn the scroll wheel
- do nothing, with *Disabled*
- run a macro, open a menu, switch [modes](modes.md), hold a [layer](layers.md), or show an overlay
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

## Back paddles

Xbox Elite, DualSense Edge and Steam Deck controllers have extra buttons on the back. They're in the **Back paddles** box under the buttons on the Buttons tab, and map like any other button.

Tick *Can't be played without back paddles* only if the profile depends on them. Players whose controller has none then aren't offered the profile, and the Guide overlay's Switch Profile skips it.

See also [Sticks and triggers](sticks.md) for zones, and [Modes](modes.md) and [Layers](layers.md) for changing many buttons at once.
