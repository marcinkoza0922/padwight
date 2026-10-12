# Tutorial: an on-screen menu on a bumper

A menu is a list or ring of choices that appears on screen. Choosing an item presses its action. You'll make a radial menu of weapon slots, and open it by holding LB.

## 1. Add the menu

![The Menus tab: the add-a-menu buttons, a radial menu's name, kind and close button, and its first items](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/menus-tab.png "The Belt menu of the Deus Ex setup.")

1. Open the setup's **Menus** tab. The **Add a menu** buttons (1) make a menu of each kind:
   - **Radial**: hold, aim with a stick, let go to choose. Best for many items.
   - **Directional**: four slots on the D-pad or the face buttons.
   - **List**: move with the D-pad or Left Stick and press A.
   - **Button menu**: a list whose items can also be chosen by their own button.
   - **Carousel**: a row you cycle through.
   - **Grid**: items in rows and columns.
2. Press **Radial**. The new menu opens. Give it a name, such as *Weapons*.
3. In **Kind** (2), choose which stick aims it. **Use boxes** draws the items in boxes at equal angles instead of arcs of the circle.
4. **Close button** sets the button that backs out. A radial menu has none unless you give it one: it goes when you let go of its input, or when you choose an item.

## 2. Add the items

The **Items** list (3) is the menu's contents. Each row has a label, a size (how much of the circle its arc takes), and an action: anything a button can do, such as a key. Add a few with **+ Add item**: *Pistol* on key `1`, *Shotgun* on key `2`, *Rifle* on key `3`. The arrows move an item up or down, **×** removes it.

The picture under the list previews the menu, so you can see where each item sits around the stick.

![A radial menu on screen with five items, one picked](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/menu-radial.png "A radial menu as it looks in a game.")

## 3. Open it with a button

1. On a profile's **Buttons** tab, open **LB**.
2. Choose **Open menu…** and pick *Weapons*.

The menu stays up while you hold LB. Aim with the stick the menu uses, and let go of LB to choose the item you aimed at.

## 4. Save and try it

Press **Save & apply**. Hold LB, aim, and let go.

## Other kinds

![A list menu on screen](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/menu-list.png "A list: move with the D-pad, press A.")

- A **list** with a close button (B by default for lists, grids and carousels) stays up when you let go of its input, so a quick tap opens it too. Choose with A, or close it with B or by pressing its input again.
- An item can open another menu, as long as both menus have the same close button. The close button goes back one level.
- **Stay open for more picks** keeps a menu up after each choice.
- On a DualShock or DualSense, the touchpad can pick from a menu: see [the touchpad tutorial](Tutorial-Touchpad).
- A list or carousel of **Switch mode…** items works well when a game has several [modes](Tutorial-Modes).

## Where next

- [On-screen menus](Menus) for the reference, including menu combos.
- The Deus Ex tutorial builds three menus on the shoulder buttons and Select: [Making a pack](Tutorial-Deus-Ex-Pack).
