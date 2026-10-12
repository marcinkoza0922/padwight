# On-screen menus

A menu is a list or ring of choices that appears on screen. Choosing an item presses its action, such as a key or a macro. You build menus on the setup's **Menus** tab.

## Opening one

Use **Open menu…** as the action of any button, gesture, combo, trigger or stick direction. The menu stays up while you hold that input, and closes when you let go. Lists with a close button are the exception (see below).

## Keeping one open

A **list** that has a close button stays up when you let go of its input, so a quick tap opens it too. It closes when you press its input again, or with its close button. Other menus go when you let go. A menu can't be put in a **Toggle**. A menu item can switch [modes](modes.md), but it can't hold a layer.

## Close button

Lists, grids and carousels close with B. Radial, directional and button menus have no close button unless you give them one: they go when you let go of their input, or when you choose an item. Each menu can have another close button, or none, on the **Menus** tab. A list without one closes when you let go, like any other menu.

## Submenus

An item can open another menu when both menus have the same close button. The new menu takes the place of the first, with the path to it shown above its title. The close button goes back one level; hold it to close every level at once. Radial menus can't open other menus.

## Choosing

Move with the Left Stick or D-pad, and press A to choose. The close button goes back one level, or closes the menu at the top; on a radial menu it closes without choosing. In a **Button menu** each item has its own button instead: press it to choose the item. Choosing an item fires it and closes the menu, whatever its kind. Opening a submenu doesn't count as choosing. To pick several items in one go, tick **Stay open for more picks** on the menu's card: the menu then stays up after each pick, until its input is let go (or pressed again, for a list with a close button) or its close button closes it. Radial menus choose as you let go, so they always close.

## Menu combos

When a button, a long press or a combo opens a directional or button menu, **+ Add menu combos** appears under its action. It adds a combo for each item: the opening input plus the item's button, doing what the item does. RB opens a D-pad menu, so RB + D-pad Up and RB + D-pad Down fire its Up and Down items without the menu showing. A combo that opens the menu gets longer (LB + RB + D-pad Up). Items set to Disabled, and button-menu items with no button, get none. Double and triple taps don't offer it.

The new combos are ordinary combos on the Combos tab, not linked to the menu: edit or delete them there. The button is off while any of them already exists, which also stops you adding them twice.

## Grids

On a grid, the arrow buttons beside an item merge it with the cells to its right or below, making one bigger item that the first cell's action fills. The items in the cells it takes are kept, hidden, until you split it again. **Cursor starts on** picks the cell the cursor is on when the menu opens: (1, 1), the top left, unless you choose another. Changing the number of columns splits every item back.

> **Tip:** a list with more than eight rows is shown in two columns. Left and right move between them, and in a list too long to show at once, LB and RB jump a page.
