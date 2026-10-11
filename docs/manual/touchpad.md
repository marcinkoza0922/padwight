# Touchpad

DualShock 4 and DualSense controllers have a touchpad, which you can also press down like a big button. Set it up on a profile's **Touchpad** tab.

## What fingers do

The pad (or each half of a split pad) has a box laid out like a stick's, with its mode on the top line. Pick one of these:

- **Off**: fingers do nothing. The click still works.
- **Mouse pointer**: works like a laptop trackpad. **Speed** is how far the pointer moves when your finger crosses the whole pad, and **Response** offers the same presets as gyro (see [Gyro](gyro.md)); any preset except Linear shows the sensitivity for slow and fast fingers and the shape of the blend to fine-tune. There are three extras:
  - **Tap to click**: a quick tap runs the click action, so you don't have to press the pad down.
  - **Two-finger scroll**: move two fingers together to scroll.
  - **Keep gliding after a flick**: flick and lift your finger, and the pointer keeps sliding and slows to a stop, like a trackball. Touching the pad stops it.
- **Gamepad stick**: your finger pushes a stick of the virtual controller (picked beside the mode), on top of the real stick. With **Center where the finger lands**, the stick's center is wherever you put your finger, and moving **Full push at** away from there is a full push. Without it, the middle of the pad is center and its edges are a full push. **Deadzone** ignores small pushes.
- **Menu zones**: the pad is divided into the items of one of your menus (see [On-screen menus](menus.md)). While a finger is on the pad, the menu shows on screen with the item under your finger picked. The game keeps getting your input while the menu shows.
- **Swipes**: a quick stroke across the pad runs the action for its direction: up, down, left or right. **Swipe length** is how far the stroke has to go.

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

- **Click the pad**: press the pad down. The item's action is held for as long as you hold the click, so an item can hold a key.
- **Lift the finger**: let go of the pad over the item. Clicking still works too.

Untick **Show the menu while a finger is on the pad** once you know the zones by heart. They still work, without the menu on screen.

An item that opens another menu opens it as usual, and you then move through it with the D-pad or stick.

## The click

**Click** is what pressing the pad down does, and it takes any action, like a button. In menu zones it's what a click does when your finger isn't on an item.

## Splitting the pad

Tick **Split into a left and a right half** to give each half a job of its own, with its own click. For example, the left half could be a gamepad stick for moving while the right half is the mouse. Each finger belongs to the half it landed on, even if it slides over the middle, so you can use both halves at once with two thumbs. A click runs the click of the half your first finger is on.

## The picture

The *Visualizers* panel under the boxes has a picture of the pad. It shows each half's job and where a menu's items lie. Fingers on your controller's touchpad show up on it, and the item under each finger lights up.

## Profiles that need a touchpad

At the bottom of the Touchpad tab, tick *Can't be played without touchpad* if the profile depends on it, such as a build menu that's only on the pad. Players whose controller has no touchpad aren't offered that profile, and Guide skips it. Leave it unticked when the pad only adds a mouse or shortcuts to a scheme that works without it.

## In layers

A layer can override the touchpad like a stick or the gyro, for example to turn the pad into a menu while you hold a button. See [Layers](layers.md).

> **Tip:** with no controller to hand, `padwight debug` has a touchpad you can drag on with the mouse. See the development docs.
