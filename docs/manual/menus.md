# On-screen menus

A menu is a list or ring of choices that appears on screen. Choosing an item runs its action, such as a key or a macro. Build menus on the setup's **Menus** tab.

## Opening one

Use **Open menu…** as the action of any button, gesture, combo, trigger or stick direction. The menu stays up while you hold that input and closes when you let go.

The exception is a **list** with a close button: it stays up after you let go, so a quick tap opens it. It closes when you press its input again, or with its close button.

A menu can't be put in a **Toggle**. A menu item can switch [modes](modes.md), but it can't hold a layer.

## Choosing

Move with the Left Stick or D-pad and press A to choose. In a **Button menu**, press an item's own button instead. Choosing an item runs it and closes the menu. A radial menu chooses the item you aim at when you let go.

Tick **Stay open for more picks** on the menu's card to keep it up after each pick, until you let go of its input or close it. Radial menus always close, since they choose as you let go.

## Close button

Lists, grids and carousels close with B. Radial, directional and button menus have none unless you give one. On the **Menus** tab, each menu can have another close button, or none. The close button goes back one level in a submenu, or closes the menu at the top; on a radial menu it closes without choosing.

## Submenus

An item can open another menu when both have the same close button. The new menu takes the place of the first, with the path to it above its title. The close button goes back one level; hold it to close every level. Radial menus can't open other menus.

## Menu combos

When a button, long press or combo opens a directional or button menu, **+ Add menu combos** appears under its action. It adds a combo for each item: the opening input plus the item's button. If RB opens a D-pad menu, RB + D-pad Up then fires its Up item without the menu showing. Disabled items, and button-menu items with no button, get none. Double and triple taps don't offer it.

The new combos are ordinary combos on the Combos tab, not linked to the menu: edit or delete them there. The button is off while any of them exists, so you can't add them twice.

## Grids

On a grid, the arrows beside an item merge it with the cells to its right or below into one bigger item, which takes the first cell's action. The merged cells' items are kept, hidden, until you split it again. Changing the number of columns splits every item. **Cursor starts on** picks the cell the cursor starts on, (1, 1) at the top left by default.

> **Tip:** a list with more than eight rows is shown in two columns: Left and Right move between them. In a list too long to show at once, LB and RB jump a page.
