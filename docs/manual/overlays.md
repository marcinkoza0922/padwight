# On-screen keyboard, numpad and overlays

Padwight can draw on top of a game, even a fullscreen one, without taking keyboard focus. Your key presses still go to the game underneath.

This works on KDE Plasma, Sway, Hyprland and labwc, but not GNOME. In a Flatpak, only on KDE Plasma.

## On-screen keyboard

Open it with Guide + X, or with the **On-screen keyboard** action on any button. Then:

- **D-pad or Left Stick**: moves between keys, repeating while held.
- **A**: presses the selected key. Hold A to hold the key down.
- **Shift, Ctrl, Alt, AltGr, Super**: stay on for the next key only, so you can type a shortcut like Ctrl + C. Press one again to let go of it.
- **X**: Backspace. Hold it to keep deleting.
- **Y, Start**: Space and Enter.
- **LB, RB**: move the text cursor left and right.
- **Hold LT**: holds Shift.
- **Hold B, or press Guide**: closes the keyboard.

The keys are labeled in your keyboard layout. On App settings, choose **Basic** keys (enough for typing into a game, the default) or **Full** ones (with function and editing keys).

## On-screen numpad

Guide + Y opens a small pad of digits, a dot, a comma, a minus, Backspace and Enter, for codes and number fields. Move with the D-pad or Left Stick; A presses a key, X is Backspace, Start is Enter, and holding B or pressing Guide closes it. Num Lock doesn't matter.

## Info overlays

Text and button pictures on screen, such as a cheat sheet. One can be always on, shown for a while when the game starts, or shown by an action, a layer or a mode. Button pictures match the controller in use: Xbox, PlayStation or Nintendo.

## Log overlays

A small list of the buttons you pressed lately and what each did, for checking a mapping.

## Media controls

Guide + LB controls the music or video player that's running: A plays or pauses, the bumpers change track, D-pad left and right seek, up and down change the volume, X mutes, Y picks the player, and B closes.

> **Tip:** each overlay has its own position, size and colors. Set them on App settings, or drag it into place with **Edit on screen**.

## Editing an overlay on screen

**Edit on screen**, beside an overlay's or menu's **Appearance**, shows it full size where it will appear in game, with a settings panel at one side.

- **Move and resize**: drag the overlay to move it; it takes the nearest of the nine screen spots and grows from there. Drag an edge or corner to resize it. With **Snap to the grid** on, edges land on the grid (8 to 64 px); hold Alt to place freely. The arrow keys nudge it (with Shift, by 10 px).
- **Look** has the same settings as Appearance. **Content** has a menu's items or an info overlay's cells, and what you type shows on the overlay at once. Click an item or cell on screen to open it there.
- **Layout**: tick other overlays under **Also show** to draw them faintly where they go, so you can place this one around them. The setup's info and log overlays are ticked at first; one at the same spot as this one isn't listed, since the two stack. **Flash** makes the overlay pulse yellow for a moment, to find it on a dark wallpaper.
- **Undo** and **Save & apply** work as in the settings window. **Done** or Esc closes the editor, and Ctrl+S saves. **–** folds the panel and **⇆** moves it to the other side.

A setup's own look for the keyboard, numpad, media controls or in-game menu (on its **Details** tab) has an **Edit on screen** too. On a desktop without compositing, the screen behind the editor shows black.

## Giving a game's overlays their own look

Under each overlay's colors, **Decorations** lists extras that are off until you tick them: an outline and shadow for the panel, a gradient background, outlines on items, a glow around the selected item, and a text color. A setup can have its own look on its Details tab, so a fantasy game's menus can glow gold while a shooter's stay flat and square.

If overlays make your computer stutter, the **Performance** card on App settings turns decorations, motion and sounds off everywhere.
