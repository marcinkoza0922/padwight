# GUI and overlay review, fourth pass (2026-10-18)

A pass over the user-facing text and controls of the settings window and the on-screen overlays,
the Guide overlay and its menus included. It follows [gui-review-2026-10-15.md](gui-review-2026-10-15.md),
[gui-polish.md](gui-polish.md) and [overlay-review.md](overlay-review.md). Items still open there
(55, 56, 57 and 60 of the third pass) aren't repeated here.

How it was looked at:

- The settings window: `padwight debug gui` with the Deus Ex library setup plus `demo`, at
  1300 × 2400, light mode. Screenshots are in `screenshots/review-2026-10-18/` (untracked), named by
  page (`10-buttons.png`, `16-guide.png`, `32-actions.png`, …). `crop-*.png` are the same pages
  without the sidebar.
- The overlays: `padwight debug overlay <scene>`, in `screenshots/overlay/` (`guide.png`,
  `switch-profile.png`, `edit-*.png`, `unsaved.png`, …).

Nothing was tried on hardware. Items marked *verify* may come from the debug scene's made-up data.

Tick an item once it's committed.

## Guide overlay and its menus

- [x] 1. **The Guide menu mixes Title Case and sentence case.** "Quick Settings" and "Edit Controls"
  are in Title Case, while "On-screen keyboard", "Mouse mode", "Media controls" and
  "Switch profile (now Gameplay)" are in sentence case. The page that "Switch profile" opens is titled
  "Switch Profile", and the page under it is "New profile". Pick one rule for these names, such as
  Title Case for panel names (Quick Settings, Edit Controls, Switch Profile) and sentence case for
  everything else, and give "Switch profile" the same casing as the page it opens (`guide.png`,
  `switch-profile.png`, `new-profile.png`).
- [x] 2. **The Guide's three panels don't line up.** Notes is centered vertically on the screen, the
  menu is a bit higher, and Mappings starts at the top of the menu. With a layer section, the
  Mappings column runs below the menu. Aligning the tops of the three panels, or centering all three,
  would look tidier (`guide.png`).
- [x] 3. **The Mappings header "· ◀ ▶ modes" is hard to read.** It shows two small D-pad crosses,
  each with one tiny blue arm, followed by "modes". At this size the arms can't be told apart.
  "Mappings · Default mode" with a separate small hint line ("◀ ▶ D-pad: other modes") would be
  clearer. The same D-pad pairs in the footers of Quick Settings and Edit Controls ("✚✚ change")
  have the same problem (`guide.png`, `edit-stick.png`, `unsaved.png`).
- [x] 4. **A layer is named in two ways in one view.** The Mappings row says
  "LB  Layer “Hotkeys”", and the panel under it is headed "Hotkeys layer · LB". Use one form, such as
  "Hotkeys layer" in both (`guide.png`).
- [x] 5. **"Driving mode (not on)" doesn't say how to turn it on.** When the Mappings panel is moved
  to a mode that isn't on, a hint such as "(preview; Y switches to it)" or just "(preview)" would
  say more than "not on" (`guide-mode.png`).
- [x] 6. **Submenu rows say "…" and "▸" at once.** "New profile… ▸", "Gamepad button… ▸",
  "Gestures… ▸", "Keys… ▸" and "Look… ▸" carry both marks. Other rows that open a page
  ("Quick Settings ▸", "Buttons ▸") have only the arrow. Drop the "…" in the overlays. The arrow
  also sits right after the text, so it lands at a different place on every row; right-aligning it
  would make the rows easier to scan (`switch-profile.png`, `edit-action.png`, `edit-menu.png`).
- [x] 7. **"New profile…" in Switch Profile is solid green.** No other row in any Guide page has a
  colored fill, and the green looks like the "selected" state of some other UI. A plain row with a
  "+" ("+ New profile") would match "+ New profile…" in the settings window
  (`switch-profile.png`).
- [x] 8. **The current profile isn't marked, only labeled.** "Gameplay (current)" looks like every
  other row. A check mark or the accent edge would show it at a glance (`switch-profile.png`).
- [x] 9. **"needs gyro" and "needs a gyro" sit one page apart.** Switch Profile says
  "Gyro aim (needs gyro)" (`system_menu/profiles.rs`), and New profile says "PC action with gyro
  aiming (needs a gyro)" (`config/templates.rs:293`). Use "needs a gyro" in both.
- [x] 10. **New profile mixes copies and templates.** The two-column grid reads "Copy of Gameplay",
  four templates, then "Copy of General's Gamepad" and "Copy of General's Desktop" at the end of the
  right column. Put the copies together, before or after the templates (`new-profile.png`).
- [x] 11. **"Gamepad passthrough" is one word only here.** The template is "Gamepad passthrough"
  (`config/templates.rs:290`), but the new-controller choice and the controller card say
  "Pass through" (`new_pad.rs:45`, `gui/controllers.rs:96`). Use "pass-through" as a noun and
  "pass through" as a verb, or just "Gamepad (pass through)".
- [x] 12. **The in-game menu opened with Guide + Start is titled "Menu".** It's called "In-game menu" on
  App settings, and its pages are titled "Quick Settings" and "Edit Controls". "Menu" is too generic
  to tell the user where they are; "In-game menu" or "Padwight" would say it
  (`in-game-menu.png`, `system_menu/view.rs:176`).
- [x] 13. **Dialogs over Quick Settings don't dim the menu under them.** In the unsaved-changes,
  disconnected and remapping-off prompts, half-hidden rows peek out above and below the dialog,
  and one is cut through its text ("Nintendo button layout: off"). A dark scrim over the menu would
  make the dialog the obvious focus (`unsaved.png`, `held-controller.png`, `held-remapping.png`).
- [x] 14. **The dialogs' buttons all look the same.** Save, Discard and Keep editing have one style,
  so nothing says which one is the safe default. Give Save the selected look, or put Keep editing
  first, since B is "back" everywhere else (`unsaved.png`).
- [x] 15. **"then the controller is let go" is jargon.** The remapping-off prompt ends with
  "Save or Discard, then the controller is let go". Something like "Save or Discard to hand the
  controller back to the game" says what happens (`held-remapping.png`).
- [x] 16. **The dialogs' footer lines have no period, and their body text does.** "You changed
  settings that aren't saved yet." ends with one, "Save or Discard, then back to the Guide" and "The
  menu waits where it was" don't (`unsaved.png`, `held-controller.png`).

## Edit Controls and Quick Settings

- [x] 17. **The same setting has different words in the overlay and the window.**
  - "Left stick dead zone" in Quick Settings vs "Deadzone" on the Sticks tab.
  - "Right stick" and "Left stick: the profile's" in Edit Controls vs "Left Stick" and
    "aim with Right Stick" in the window. Item 15 of the first pass capitalized sticks "everywhere",
    but the overlays weren't covered.
  - "Invert Y: off" in Quick Settings vs "Invert Y: no" in Edit Controls. Every yes/no row in Edit
    Controls says "no"/"yes" ("Real-world sensitivity: no", "Stays up after a choice: no"), and
    every row in Quick Settings says "on"/"off". Use one pair.

  (`unsaved.png`, `edit-stick.png`, `edit-gyro.png`, `edit-menu.png`, `12-sticks.png`)
- [x] 18. **The action page names actions differently from the window's action picker.**
  The overlay lists "Keys…", "Left click", "Right click", "Middle click", "Back click" and
  "Quick Settings"; the window's "Choose an action" dialog lists "Keyboard", "Mouse button" and
  "Quick Settings menu". App settings calls the shortcuts "Screenshot" and "Start or stop recording",
  and the action picker calls the actions "Take screenshot" and "Toggle recording". One name each
  would let someone who learned one find it in the other (`edit-action.png`, `32-actions.png`,
  `03-shortcuts.png`).
- [x] 19. **On a button's action page, "Keys…" is the last item.** It's the most common action but
  sits at the bottom of the right column, after Force quit and four gyro actions. Keys, Gamepad
  button and Mouse clicks first, then gyro and system actions, would match the window's picker
  order (Press, Change how it presses, Open or switch, …) (`edit-action.png`).
- [x] 20. **"Now: E" looks like an action.** The first row of the action page shows the current
  binding but is styled like the choices under it. A non-selectable line under the title
  ("Now: E"), or a muted row, would read as status (`edit-action.png`).
- [x] 21. **Rows with glyph chips are taller than plain rows.** In Edit Controls, "LS RS Sticks" and
  "LT RT Triggers" are a few pixels taller than "Buttons" and "Gyro", and in a layer's page
  "LS Left stick: the profile's" rows are taller than "Combos (0)". The list looks uneven
  (`edit-controls.png`, `edit-layer.png`).
- [x] 22. **Only Sticks and Triggers have glyphs** on the Edit Controls page. Either give Buttons, Gyro
  and Touchpad an icon too, or drop the chips, which repeat what the word says (`edit-controls.png`).
- [x] 23. **Counts are written two ways.** "Macros (1)", "Combos (0)", but "Buttons (0 bound)" on a
  layer page. And a "(0)" count on an empty list doesn't add much: "Modes (0) ▸" could read
  "Modes: none ▸" or be dimmed (`edit-controls.png`, `edit-layer.png`).
- [x] 24. **"the app's" and "the profile's" are awkward values.** "Font: the app's",
  "On-screen keyboard look: the app's", "Nintendo button layout: as the controller's",
  "Left stick: the profile's" and "Closes with: B (its kind's)". The window says "Following the …
  set on the App settings page". Short words such as "Font: App settings", "Left stick: same as
  profile" and "Closes with: B (default)" would read better (`edit-game.png`, `edit-layer.png`,
  `edit-menu.png`).
- [x] 25. **Some stick and gyro values have no unit or meaning.** "Outer boost: 0.00", "Curve: 2.00",
  "Acceleration: 0% faster", "Turning read as: Yaw (turn)" and the graph caption
  "50°/s now: 1.00×". The window's sliders have explanations beside them; the overlay could add a
  one-line hint for the highlighted row at the bottom of the panel (`edit-stick.png`,
  `edit-gyro.png`).
- [x] 26. **The "▼ more below" mark is a lone triangle at the bottom-left.** It's small, sits under
  the first column only and doesn't say how many rows are hidden. Center it, or show "▼ 4 more"
  (`edit-action.png`, `edit-game.png`, `edit-menu.png`).
- [x] 27. **The visualizer panels float in the middle of the right column.** On the stick and gyro
  pages the "Stick travel" and "Pointer speed" panels start well below the first row, leaving a gap
  above them. Top-aligning them with the first row would tie them to the list (`edit-stick.png`,
  `edit-gyro.png`).
- [x] 28. **The typing footer is laid out differently from every other footer.** It has no "·"
  separators and wider gaps ("✚ move  A type  X delete (hold to repeat)  Y space  Menu done  hold
  LT shift  hold B to cancel"), and "hold B to cancel" has a "to" that the others don't. Match the
  other footers: "… · hold LT shift · hold B cancel" (`edit-typing.png`).

## Other overlays

- [x] 29. **The library offer draws its buttons as plain letters.** "A Add", "B Not now",
  "Y Never" use a plain "A" where the unsaved-changes prompt uses the colored glyph disc. The
  footer "A add · B not now · Y never" then repeats the buttons word for word. Use glyphs on the
  buttons and drop the footer (`offer.png`).
- [x] 30. **The library hint has two unpunctuated lines and a plain "Guide".** "Padwight has controls
  for Deus Ex / Tap Guide to set them up" could end each line with a period and draw Guide as its
  glyph, like every other hint. The offer says "The library has controls for this game." for the
  same fact; one wording would do (`library-hint.png`).
- [x] 31. **A new, empty info overlay is a tiny dark pill** in the top-right corner, with nothing in
  it. While it has no cells it could show its name and "No rows yet", or nothing at all
  (`info-empty.png`).
- [x] 32. **The media controls footer is very long and very small.** Seven hints in about 11 px
  type. "Sound 45%" and "Player 70%" are also unclear: "System 45%" and "Player 70%", or a speaker
  icon for each, would say which volume is which. "Artist · Music" shows the player's name as if
  it were the album (`media.png`).

## Settings window: wrong or misleading text

- [x] 33. **Details → Button layout points to the wrong page.** While the setup has no choice of its
  own, the card says "Following the button layout set on the App settings page." (`own_rows` in
  `gui/widgets.rs:240`). App settings has no button layout: it's set per controller on the Profiles
  tab, as the card's own ⓘ says. Let `own_rows` take the place it follows, and say "Following each
  controller's setting on the Profiles tab." here (`27-details.png`).
- [x] 34. **The Profiles tab's "Button layout" card looks like part of the profile.** It sits between
  Edit profile and Live input, on every profile, but it's a per-controller setting and doesn't
  change with the profile. With "All controllers" picked in the header it's greyed out with no
  visible reason (the hint is behind the ⓘ). Move it to the controller card on the Overview, or say
  on the card itself "For: Xbox Wireless Controller" / "Pick a controller in the header"
  (`06-general.png`, `10-buttons.png`, `gui/profile.rs:817`).
- [x] 35. **The Overview's picture says "Every input passes through unchanged"** while remapping is
  on with General's Gamepad profile. That's true of that profile, but on the Overview it reads like
  remapping is doing nothing. Name the profile ("General › Gamepad passes every input through") or
  leave the line out on the Overview (`01-overview.png`).
- [x] 36. **"since the daemon started"** is the one place the Overview uses the word daemon while
  it's running: "No profile has switched since the daemon started." "No profile switches yet." is
  enough (`01-overview.png`).
- [x] 37. **The sidebar's empty-state note repeats the button under it.** "No game setups yet. Use
  + Add setup to add one." sits right above "+ Add setup". "No game setups yet." would do
  (`01-overview.png`).
- [x] 38. **The Overview's "What's happening" card explains itself twice:** a gray sentence under
  the title and an ⓘ beside it. Keep one (`01-overview.png`).
- [x] 39. **The rumble checkbox's label is unclear.** "A faint pulse as an overlay item is picked"
  is on the Rumble strength row, so it reads like a strength. "Pulse when picking an overlay item"
  says it's an on/off feature (`01-overview.png`).
- [x] 40. **The info overlay's "Shown" note is one long sentence with a bracket inside.** "Stays on
  screen whenever the game is active, so actions can't show it. To show it only sometimes, untick
  this and map “Toggle → Show info overlay…” to a button (the Toggle can start on, to show the
  overlay at launch until dismissed)." Split it, and move the bracketed part to the ⓘ. The checkbox
  says "while this setup is active" and the note "whenever the game is active"; use one
  (`25-info.png`).
- [x] 41. **Tautological descriptions in the action picker.** "Mouse button: A mouse button.",
  "Gamepad button: A controller button.", "Info overlay: Shows an info overlay.", "Log overlay:
  Shows a log overlay.", "Quick Settings menu: Opens the Quick Settings menu." Say what each is for,
  as Turbo and Several at once do ("Left, right, middle, back or forward click.", "Text and glyphs
  that stay on screen, such as a cheat sheet.") (`32-actions.png`).
- [x] 42. **The action picker doesn't say which button it's for.** The title is "Choose an action";
  "Action for A (South)" or a subtitle would help when it's opened from a gesture or a sector
  (`32-actions.png`).
- [x] 43. **"Controllers this setup supports" lists three pads with no Xbox.** It isn't clear whether
  an Xbox pad is always supported, or what ticking one changes. The ⓘ may say it, but a short line
  under the title ("Xbox-style pads always work. Tick the others you've tested.") would make the
  card readable without it (`27-details.png`).
- [x] 44. **"Installed ones first" isn't shown in the list.** The Add a setup dialog says
  "Built-in games (installed ones first):", but nothing in the list marks which games are installed
  or where the installed ones end. A small "installed" tag or a divider would show it
  (`30-add.png`).
- [x] 45. **The Guide notes preview breaks a line after a leading glyph.** The first note,
  "{start} (Escape) backs out of every in-game screen…", draws the Menu chip alone on a line and
  "(Escape) backs out…" on the next. The last note wraps with a leading space (" match the F-keys…"),
  so its second line is indented one space. Check `info::notes_panel`'s wrapping: a glyph at the
  start of a paragraph should stay on the line with the text after it, and a wrapped line shouldn't
  start with a space. The Guide overlay draws the notes the same way, so it likely has the same bug
  (`16-guide.png`).
- [x] 46. **{r3} is drawn as "R↓" in the notes and as "R3" everywhere else.** The preview's "click
  {r3} to look through it" shows a round "R↓" glyph, while the Mapping guide, the button list and
  the overlays show "R3". Use one glyph for stick clicks (`16-guide.png`).
- [x] 47. **The Mapping guide's text fields still show raw tokens** ("{icon:bag} Inventory, goals").
  Item 52 of the third pass added a preview for the notes, but the Mapping guide has none, and menu
  item names ("{icon:key} Nanokey ring") and info overlay cells ("{south}") show the same raw text in
  their fields. A small rendered chip beside each field, or rendering the token inline when the
  field isn't focused, would show what it will look like (`16-guide.png`, `23-menus.png`,
  `25-info.png`).

## Settings window: inconsistent words and controls

- [x] 48. **"New" and "Add" are both used for creating things.** "+ New mode", "+ New layer",
  "+ New macro", "+ New info overlay", "+ New profile…" vs "+ Add combo", "+ Add rule",
  "+ Add item", "+ Add row", "+ Add output", "+ Add setup". One reading: "New" for a top-level item
  with its own card, "Add" for a row inside one. If that's the rule, "+ Add combo" should be
  "+ New combo". Otherwise pick one word (`13-combos.png`, `20-modes.png`, `27-details.png`).
  *Kept that rule: a combo has no name and is a row in the Combos card, so "+ Add combo" fits it, and
  "+ Add setup" picks an existing game from the library.*
- [x] 49. **Empty states are drawn two ways.** On Combos the "This profile has no combos yet." line
  is inside the card, under the title and above the add button. On Modes, Layers and Log overlays,
  "This setup has no … yet." sits outside, under the card with the add button (`13-combos.png`,
  `20-modes.png`, `21-layers.png`, `26-logs.png`).
- [x] 50. **Tab blurbs are written in different forms.** "A mode swaps in another control scheme
  until you switch back.", "A layer changes some of the mappings while you hold it." and "A macro
  plays a sequence of inputs." are sentences; "The buttons just pressed, and what each did." (Log
  overlays) and "Text and button glyphs on screen, such as a game's controls." (Info overlays) are
  fragments. "A log overlay shows the buttons just pressed and what each did." and "An info overlay
  shows text and button glyphs on screen, such as a game's controls." match the others
  (`26-logs.png`, `25-info.png`).
- [x] 51. **"+ New profile…" is drawn as a dropdown.** It has the dropdown's field look and chevron,
  next to Duplicate and Delete, which are buttons. If it opens a list of templates that's right, but
  then its label could say so ("+ New profile from…"); otherwise make it a button like "+ New macro"
  (`06-general.png`).
- [x] 52. **The profile's Name field is outside its card.** "Name" sits on the page between the Edit
  profile card and the Button layout card. On Details, a setup's Name is inside the Setup card, and a
  macro's or menu's Name is inside its card. Move it under the Profile dropdown (`06-general.png`).
- [x] 53. **Live input is a card on the Overview but loose on a setup's page.** Item 43 of the third
  pass made the Overview's parts cards; on the Profiles tab "Live input" and its Hide button sit on
  the page background, between two cards (`01-overview.png`, `10-buttons.png`).
- [x] 54. **Three styles side by side for an overlay's actions.** On App settings, the On-screen
  keyboard card reads "[Open it now] ▸ Appearance [Edit on screen]": a button, a disclosure link and
  another button in one row. Moving "▸ Appearance" to its own line below the buttons (as on a
  menu's card) would separate "do" from "show more" (`02-settings.png`).
- [x] 55. **The App settings toggles in Performance are negatives.** "No motion", "No sounds", "No
  looping animations" and "Plain overlays" are on when the feature is off. "No sounds" sits under
  an Overlay sounds card full of toggles that are on, so turning on "No sounds" makes six "on"
  toggles above it meaningless. Either name them positively and default them on ("Overlay sounds"),
  or grey out the Overlay sounds card while "No sounds" is on (`02-settings.png`).
- [x] 56. **The overlay lists differ between Overlay motion and Overlay sounds.** Motion lists
  On-screen keyboard, numpad, Menus, Media controls, Library offers, Info overlays and Log overlays;
  Sounds lists keyboard, numpad, Menus, Guide menus, Media controls and Library offers. Guide menus
  have sounds but no motion setting, and info and log overlays the other way round. If that's
  because they don't apply, a line in the ⓘ would say so; if not, the lists should match
  (`02-settings.png`, `27-details.png`).
- [x] 57. **"Turn everything off" vs "Turn every sound off".** App settings → Performance has
  "Turn everything off"; Details → Overlay sounds has "Turn every sound off", and App settings' own
  Overlay sounds card has no such button (`02-settings.png`, `27-details.png`).
- [x] 58. **"Show the overlay being edited" wraps onto two lines** in the label column on Controller
  shortcuts → Menu buttons, and its dropdown starts in the kind-dropdown column (x≈371) while every
  other row in the card puts its button picker at x≈486 (item 44 of the third pass lined these up)
  (`03-shortcuts.png`).
- [x] 59. **The mouse-mode gyro checkbox is hard to parse.** "Gyro moves the pointer, as the
  profile's gyro does (at a default speed if it aims a stick or is off)". Something like "Gyro moves
  the pointer" with the bracketed part in an ⓘ would do (`03-shortcuts.png`).
- [x] 60. **"Quick Settings menu" on Controller shortcuts, "Quick Settings" in the overlay.** Item 20
  of the third pass kept "Quick Settings" as the panel's name; the shortcut row and the action
  picker add "menu" (`03-shortcuts.png`, `32-actions.png`).
- [ ] 61. **"Play or pause" in the window, "play / pause" in the overlay.** The media shortcut row says
  "Play or pause" and the media overlay's footer says "play / pause" (`03-shortcuts.png`,
  `media.png`).

## Settings window: pictures and summaries

- [ ] 62. **Callouts on the controller picture are still cut off.** "Gyro clutch (off w…" and
  "Menu “Augmentat…”" at 1300 px wide. For the gyro clutch a shorter name ("Gyro clutch") would fit;
  for menus, the menu name alone in quotes, without "Menu", fits more (`10-buttons.png`).
- [ ] 63. **The "+" legend under the picture is cryptic.** "“+”: also does something on a double tap,
  triple tap or long press". A label ending in "+" ("Esc +", "“;” +") is not an obvious sign for
  that. A small superscript gesture glyph, or "…", or a dot, would be easier to learn; at least
  start the legend with a word ("A “+” after a label: …") (`10-buttons.png`).
- [ ] 64. **The Buttons summary has no Triple tap column.** The columns are Press, Double tap and Long
  press, but every row offers "+ Triple tap". A triple tap, when set, has nowhere to go
  (`10-buttons.png`, `11-expanded.png`) — *verify* where it's drawn.
- [ ] 65. **Lone punctuation keys are still slivers in the chips.** Item 50 of the third pass named
  the apostrophe, backtick, comma and period ("Apostrophe"), but ";" (X), "[" and "]" (R3) are still
  bare marks, and X's row now shows one key as a mark and the other as a word. Name these too
  ("Semicolon", "Left bracket") (`10-buttons.png`).
- [ ] 66. **The keyboard field's tiny ⌨ button has no label.** On every keyboard row the gray button
  after the key field shows only a keyboard icon. A tooltip ("Pick a key from a keyboard") would
  explain it, if it doesn't have one (`11-expanded.png`, `12-sticks.png`).
- [ ] 67. **The Sticks tab's labels wrap.** "Anti-deadzone buffer" takes two lines in the label
  column; the sector rows read only "↑ Sector 1", "↗ Sector 2". Naming the direction
  ("Sector 1 · Up") would help on an 8-way ring where the arrow glyphs are small (`12-sticks.png`).
- [ ] 68. **The action picker's sections end with a lone card.** Each of five sections fills a row of
  three and leaves one card alone (Disabled, Macro, Recenter gyro, Media controls). Four columns, or
  regrouping, would make the grid even. Also, some icons are colored (Toggle, Turbo, Macro, Open
  menu, Hold layer, Switch mode, Info, Log) and the rest gray, with no pattern a reader can use
  (`32-actions.png`).

## Not changed, worth a look

- The Overlay motion preview is still an empty white box in a still render (item 12 of the third
  pass says it's animated). A first frame showing the menu open would look better in screenshots and
  the manual.
- Every row on Controller shortcuts has its own "Set by pressing" button (about 30). That's open item
  60's pattern again; one "Set by pressing" that then asks which row would quiet the page.
