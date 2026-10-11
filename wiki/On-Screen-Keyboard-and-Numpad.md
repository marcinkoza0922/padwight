# On-screen keyboard and numpad

Both pads open over everything, including fullscreen games. They are layer-shell surfaces that never take keyboard focus, so keys go to the window underneath.

They need a compositor with layer-shell support: KDE Plasma, Sway, Hyprland and similar. GNOME doesn't have it. See [Desktop support](Desktop-Support).

## Opening them

- **On-screen keyboard**: the "On-screen keyboard" action, the button on the Overview page, or `padwight overlay-toggle`. Hold Guide and press X, or open the Guide overlay and press X. See [The Guide button](Guide-Button).
- **On-screen numpad**: the "On-screen numpad" action, or `padwight numpad-toggle`. Hold Guide and press Y, or open the Guide overlay and press Y.

Opening one closes the other.

## Keyboard controls

| Control | Action |
|---|---|
| D-pad / left stick | Move between keys (repeats while held) |
| A | Press the selected key. Holding A holds the key. |
| Shift / Ctrl / Alt / AltGr / Super | Latch for the next key; press it again to let go |
| X | Backspace (repeats while held) |
| Y / Start | Space / Enter |
| LB / RB | Move the text cursor left / right |
| Hold LT | Hold Shift |
| Hold B, or press Guide | Close |

The hints at the bottom show your controller's own buttons.

### Basic or full

The **Settings** page chooses the keys. **Basic**, the default, has what typing into a game needs: letters, digits and symbols, Tab, Enter, Backspace, Esc, Ctrl, Alt, AltGr and the arrows. **Full** adds the function keys, the editing keys (Insert, Delete, Home, End, Page Up and Down) and the rest of a whole keyboard.

### Your keyboard layout

Keys are labelled with what they type in your keyboard layout, so a German layout shows Z where a US one shows Y, and the labels change while Shift is on. Caps Lock lights up when the keyboard turned it on. The layout is the first one your desktop is set to (KDE, GNOME, Sway, Hyprland, or the system's X11 or console setting), else the one your locale suggests. Set `PADWIGHT_KEYBOARD_LAYOUT` (for example `de` or `fr(azerty)`) in the daemon's environment to choose another.

## Numpad controls

The numpad has the digits 0 to 9, a dot, a comma and a minus, plus Backspace and Enter keys, for codes and number fields.

| Control | Action |
|---|---|
| D-pad / left stick | Move |
| A | Press the selected key |
| X | Backspace (repeats while held) |
| Start | Enter |
| Hold B, or press Guide | Close |

The numpad types the top-row number keys, so it works whatever the Num Lock state is. On a layout that types digits with Shift (AZERTY), it holds Shift for them.

## Appearance

Both pads have their own position, size and colours on the **Settings** page. A game can use its own look for either one, from its Details tab. The [media controls](Buttons-and-Actions) and the [in-game menu](In-Game-Menu) have the same kind of Appearance section, on the same pages.
