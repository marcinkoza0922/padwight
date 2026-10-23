# Tutorial: an on-screen menu on a bumper

A menu is a list or ring of choices on screen; choosing an item runs its action. You'll make a radial menu of weapon slots, opened by holding LB.

## 1. Add the menu

![The Menus tab: the add-a-menu buttons, a radial menu's name, kind and close button, and its first items](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/menus-tab.png "The Belt menu of the Deus Ex setup.")

1. Open the setup's **Menus** tab. The **Add a menu** buttons (1) make each kind:
   - **Radial**: hold, aim with a stick, let go to choose. Best for many items.
   - **Directional**: four slots on the D-pad or the face buttons.
   - **List**: move with the D-pad or Left Stick and press A.
   - **Button menu**: a list whose items can also be chosen by their own button.
   - **Carousel**: a row you cycle through.
   - **Grid**: items in rows and columns.
2. Press **+ Radial** and name the new menu, such as *Weapons*.
3. In **Kind** (2), choose the stick that aims it. **Use boxes** draws the items as boxes instead of arcs.
4. **Close button**: a radial menu has none by default. It closes when you let go of its input.

## 2. Add the items

The **Items** list (3) is the menu's contents. Each row has a label, a size (how much of the circle it takes) and an action, such as a key. Add *Pistol* on key `1`, *Shotgun* on `2` and *Rifle* on `3` with **+ Add item**. The arrows move an item, and **×** removes it.

The preview under the list shows where each item sits around the stick.

![A radial menu on screen with five items, one picked](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/menu-radial.png "A radial menu as it looks in a game.")

## 3. Open it with a button

1. On a profile's **Buttons** tab, open **LB**.
2. Choose **Open menu…** and pick *Weapons*.

## 4. Save and try it

Press **Save & apply**. Hold LB, aim with the stick, and let go to choose.

## Other kinds

![A list menu on screen](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/menu-list.png "A list: move with the D-pad, press A.")

- A **list** has a close button (B by default), so it stays up after you let go and a tap opens it. Choose with A; close it with B or its input again.
- An item can open another menu with the same close button. The close button goes back one level.
- **Stay open for more picks** keeps a menu up after each choice.
- On a DualShock or DualSense, the touchpad can pick from a menu: see [the touchpad tutorial](Tutorial-Touchpad).
- A list or carousel of **Switch mode…** items works well when a game has several [modes](Tutorial-Modes).

## Where next

- [On-screen menus](Menus) for the reference, including menu combos.
- The Deus Ex tutorial builds three menus on the shoulder buttons and Select: [Making a pack](Tutorial-Deus-Ex-Pack).
