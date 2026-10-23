# Tutorial: a cheat sheet on screen, and the on-screen keyboard

Padwight draws on top of a game, even a fullscreen one, without taking keyboard focus. You'll make an info overlay (a cheat sheet with button pictures), then use the on-screen keyboard.

Overlays need KDE Plasma, Sway, Hyprland or labwc; GNOME doesn't support them. See [On-screen keyboard, numpad and overlays](overlays.md).

## 1. Make an info overlay

![The Info overlays tab: a cheat sheet with its name, heading, always-shown switch, three rows of cells and a preview](../images/manual/info-tab.png "A cheat sheet with three rows.")

1. Open the setup's **Info overlays** tab and press **+ New info overlay** (1). The new overlay opens.
2. Give it a **Name** and a **Heading**, the title above the grid.
3. **Shown** (2): tick **Always, while this setup is active** for a permanent cheat sheet. Untick it to show the overlay when the game starts (for as many seconds as you set), and with the **Info overlay…** action on a button: while it's held, or until pressed again if wrapped in **Toggle on / off…**.
4. **Cells** (3) is the grid; cells line up in columns. A cell is text, or a `{token}`: `{south}` draws the A button as your controller labels it. **Insert…** opens the token picker, with every token and a preview of each: button glyphs, icons, formatting tags (`{b}` bold, `{i}` italic, `{color:red}`…) and live values such as the time, CPU load or current mode. Put a button in the first cell and what it does in the second.
5. **+ Cell** adds a cell to a row and **+ Add row** adds a row.
6. The **Preview** (4) shows the result as you type.

![An info overlay on screen over a game](../images/manual/info-overlay.png "An info overlay with live values and glyphs.")

Under **Appearance**, or with **Edit on screen**, each overlay gets its own position, size and colors. For a whole setup's look, use the Details tab.

## 2. Show a log of your presses

The **Log overlays** tab makes a small list of your latest presses and what each did, for checking a mapping. It's made like an info overlay, and shown with **Log overlay…**.

## 3. The on-screen keyboard

Open the keyboard with Guide + X, or a button set to **On-screen keyboard**. Then:

![The on-screen keyboard, with Shift on and the cursor on H](../images/manual/keyboard-overlay.png "The on-screen keyboard.")

- **D-pad or Left Stick** moves between keys. **A** presses the selected key (hold A to hold the key down).
- **Shift, Ctrl, Alt, AltGr and Super** stay on for the next key only, so you can type a shortcut like Ctrl + C.
- **X** is Backspace, **Y** is Space, **Start** is Enter, **LB** and **RB** move the text cursor, and holding **LT** holds Shift.
- **Hold B**, or press **Guide**, closes it.

On **App settings** you can choose **Basic** or **Full** keys. The **numpad** (Guide + Y) is a pad of digits for codes, and **media controls** (Guide + LB) drive the music or video player that's running.

## 4. Make overlays fit the game

On its **Details** tab, a setup can have its own font, motion, sounds and colors for every overlay, so a fantasy game's menus can glow gold while a shooter's stay flat. If overlays make your computer stutter, the **Performance** card on **App settings** turns decorations, motion and sounds off everywhere.

## Where next

- [On-screen keyboard, numpad and overlays](overlays.md) for the reference.
- A layer or mode can show an info overlay while it's on: see [layers](tutorial-layers.md).
