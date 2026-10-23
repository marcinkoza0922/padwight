# Touchpad

DualShock 4 and DualSense controllers have a touchpad, which you can also press down like a big button. Set it up on a profile's **Touchpad** tab.

## What fingers do

The pad (or each half of a split pad) has a box like a stick's, with its mode on the top line:

- **Off**: fingers do nothing. The click still works.
- **Mouse pointer**: a laptop trackpad. **Speed** is how far the pointer moves when your finger crosses the whole pad, and **Response** offers the same presets as [gyro](gyro.md). Three extras:
  - **Tap to click**: a quick tap runs the click, so you needn't press the pad down.
  - **Two-finger scroll**: move two fingers together to scroll.
  - **Keep gliding after a flick**: flick and lift your finger, and the pointer keeps sliding and slows to a stop, like a trackball. Touching the pad stops it.
- **Gamepad stick**: your finger pushes a virtual stick (picked beside the mode), on top of the real one. Without **Center where the finger lands**, the middle of the pad is center and its edges a full push. With it, center is wherever your finger lands, and **Full push at** away from there is a full push. **Deadzone** ignores small pushes.
- **Menu zones**: the pad is divided into the items of one of your [menus](menus.md). While a finger is on the pad, the menu shows with the item under your finger picked. The game still gets your other input.
- **Swipes**: a quick stroke up, down, left or right runs that direction's action. **Swipe length** is how far the stroke must go.

## Menu zones

The pad is divided the way the menu is drawn:

| Menu kind | How the pad is divided |
| --- | --- |
| Radial | by direction from the middle of the pad; the very middle picks nothing |
| Directional | up, right, down and left from the middle |
| Grid | into the grid's cells |
| List, Button menu | into rows; a long list into two columns |
| Carousel | from left to right |

**Choose by** sets how you run the item under your finger:

- **Click the pad**: press the pad down. The item's action is held as long as the click, so an item can hold a key.
- **Lift the finger**: let go over the item. Clicking works too.

Untick **Show the menu while a finger is on the pad** once you know the zones by heart; they still work.

An item that opens another menu opens it as usual; move through that one with the D-pad or stick.

## The click

**Click** is what pressing the pad down does. It takes any action, like a button. In menu zones it's what a click does away from the items.

## Splitting the pad

Tick **Split into a left and a right half, each with its own job** to give each half its own mode and click, such as a gamepad stick on the left for moving and a mouse on the right. A finger belongs to the half it landed on even if it slides over the middle, so both thumbs can work at once. A click runs the click of the half your first finger is on.

## The picture

The *Visualizers* panel under the boxes draws the pad: each half's job, where a menu's items lie, and your fingers, with the item under each lit up.

## Profiles that need a touchpad

At the top of the Touchpad tab, tick *Can't be played without touchpad* if the profile depends on it, such as a build menu that's only on the pad. Players whose controller has no touchpad then aren't offered the profile. Leave it unticked when the pad only adds extras.

## In layers

A [layer](layers.md) can override the touchpad, for example to turn the pad into a menu while you hold a button.

> **Tip:** with no controller to hand, `padwight debug` has a touchpad you can drag on with the mouse. See the development docs.
