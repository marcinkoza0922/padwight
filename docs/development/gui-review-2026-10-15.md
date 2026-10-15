# GUI review, third pass (2026-10-15)

A walk through every page, tab and dialog of the settings window, after the two passes in
[gui-polish.md](gui-polish.md) (items 1–85, all closed). This covers what's still wrong or has come
back since, and what's new.

How it was looked at: `padwight debug gui` with the StarCraft and Deus Ex library setups plus `demo`,
at 1300 × 1900 (and 1300 × 4200 for long pages), in light and dark mode, and at 900 × 1300 to check
a narrower window. Screenshots are in `screenshots/review-2026-10-15/` (untracked): `light/` and
`dark/`, named by page (`20-buttons.png`, `35-menu-open.png`, `61-narrow-buttons.png`, …).

The screenshot tool runs with the daemon up and a made-up Xbox controller, so "daemon down" states
weren't covered this time. Items marked *verify* may come from the tool rather than the app.

## Broken layout

- [x] 1. **Below about 1150 px wide, the setup tabs are cut off.** At 900 px, "Info overlays" wraps
  onto two lines, and "Log overlays" and "Details" are off the right edge, so there's no way to
  reach them. The tab bar doesn't wrap or scroll. The profile sub-tabs have the same problem:
  "Guide" touches the edge, and Tutorial, Expand all and Collapse all are gone
  (`61-narrow-buttons.png`, `63-narrow-details.png`). **Done:** The setup tabs and the profile sub-tabs wrap onto a second line (`segment_bar` in `widgets.rs`), and a tab's name never wraps inside it.
- [x] 2. **In a narrow window the controller picture's labels come loose from the picture.** The
  callout lines stop about 40 px short of their labels. The letters LT, RT, Y, X, B and A are drawn
  beside their buttons instead of on them, and right-hand labels are cut off ("Menu “Augmen"). The
  Overview picture has the same misplaced letters (`60-narrow-overview.png`,
  `61-narrow-buttons.png`). **Done:** The picture keeps its size and scrolls sideways in a narrow window, so the labels stay on their parts.
- [x] 3. **In a narrow window the Menus header breaks.** The "Grid" button is cut through, the
  blurb, "Copy from another setup…" and the Tutorial link are gone, and the card is about 180 px tall
  with the buttons centered in empty space (`62-narrow-menus.png`). On Details, the ✕ that deletes
  an auto-switch rule is pushed out of view (`63-narrow-details.png`). **Done:** The add buttons and the blurb under them share the width the copy button leaves, and wrap there. Auto-switch rules wrap too, so the ✕ stays in view. The menu buttons read "+ Radial" and so on.
- [x] 4. **The Guide tab's Notes box cuts off its last line.** On Deus Ex and StarCraft the last line
  sits on the box's bottom edge, half outside it, with no scrollbar. The Export dialog's description
  box had the same bug (item 46); the Notes box needs the same fix of growing to whole lines
  (`27-guide.png`, `40-sc-profiles.png`). **Done:** The Notes box grows with the notes in whole lines, up to 16 (`editor_lines`, shared with the Export box). The Deus Ex notes had an eighth line that was hidden.
- [x] 5. **In the Mapping guide, gesture rows show raw tokens that run into the next column.** Their
  label column reads "{gesture:hold}", "{gesture:double}", and "{gesture:double}" overlaps the text
  field ("{gesture:double}{icon:save} Quick save"). The label should show the gesture as a word or
  glyph ("Hold", "Double tap"), and fit its column (`27-guide.png`; tokens come from
  `RowShape::Gesture` in `src/config/guide/`). **Done:** Gesture rows show the gesture as a word ("Hold", "Double tap") in the label column.
- [x] 6. **The menu's "Close button" dropdown is too narrow for its value.** "None (the default)"
  runs under the arrow (`35-menu-open.png`). **Done:** The dropdown is as wide as its longest choice.
- [x] 7. **The radial menu preview spills past its frame.** The wheel is wider than the dark preview
  panel, so the panel cuts off its left, right and top edges (`53-menu-tall.png`). **Done:** No change: at full size the wheel fits its panel. The scaled-down screenshot was misread.
- [x] 8. **Menu items' Name fields cut off their text.** At about 150 px wide, "{icon:key} Nanokey
  ring" shows as "{icon:key} Nanoke" and "{icon:holster} Holster" as "{icon:holster} Hol", while
  the row has plenty of empty space to its right (`35-menu-open.png`). **Done:** The label field is 230 px and the number column narrower.
- [x] 9. *verify* **The open menu's preview shows through the Save & apply bar.** In
  `35-menu-open.png` the gray preview background and the dark wheel go on under Revert and Save &
  apply, as though the footer were transparent there. Check in a real window. **Done:** No change: the tiny-skia renderer the screenshot tool uses draws shadows without the clip mask. A real window draws with wgpu, which clips them.
- [x] 10. **Button summaries with several gestures get cut off at the right edge.** In a narrow
  window, Start's "Long press: Keypad Slash" is cut through the middle instead of wrapping
  (`61-narrow-buttons.png`). **Done:** A button's summary segments wrap onto another line, each gesture kept whole.

## Contradictory or misleading states

- [x] 11. **The Overlay motion "Preview" dropdown doesn't follow the styles above it.** On Deus Ex
  every overlay is set to "Swift", but the Preview dropdown says "Subtle", so the preview shows a
  style the setup doesn't use. It should start on the style in use, or say what it previews
  ("Preview a style:") (`10-details.png`). **Done:** The preview starts on the page's menu style whenever the page or tab opens (`sync_motion_demo`).
- [x] 12. *verify* **The Overlay motion preview is an empty white box,** about 670 × 180 px, in both
  App settings and Details. It may be animated and blank only in a still render, but if so, a still
  frame would look better than a blank panel (`03-settings.png`, `10-details.png`). **Done:** No change: the preview is animated. A still taken as it starts catches the menu closed.
- [x] 13. **Disabled ↑ / ↓ buttons look more active than the enabled ones.** The first step's ↑ and
  the last step's ↓ (in macros, menu items and info-overlay rows) are white with an edge, while the
  usable arrows are gray-filled. That's the opposite of what a reader expects (`33-macro-open.png`,
  `35-menu-open.png`, `37-info-open.png`). **Done:** Disabled buttons' text is fainter (40%), so a disabled arrow reads as out of use.
- [x] 14. **Placeholder action dropdowns still look disabled in some places.** "Insert motion…" in
  the macro editor uses gray placeholder text, although item 67 changed the other action dropdowns
  to the text color (`33-macro-open.png`). **Done:** "Insert motion…" and the combo's "+ Add button…" use `style::action_dropdown`.
- [x] 15. **The header's "Active profile" doesn't say which setup it belongs to.** On the Deus Ex
  page it reads "Gamepad", which is General's profile, with nothing to say so. The padlock beside it
  is an icon with no label (`20-buttons.png`). **Done:** Profiles are named with their setup everywhere, General too: "General › Gamepad". The padlock already has a tooltip.
- [x] 16. **The Gyro tab asks "Can't be played without gyro" while gyro is Off.** The requirement
  and its three-line explanation come before the setting itself and mean nothing until gyro is on.
  Show them under the Gyro dropdown, and only when it isn't Off (`25-gyro.png`). **Done:** The requirement sits after the Gyro setting, and shows only while gyro isn't Off (or the box is ticked).
- [x] 17. **An unmapped button shows only its own glyph in the Buttons list** (General: A shows
  the green "A" disc and nothing else). It isn't clear whether that means "passes through as A" or
  is just the button's icon. Paddles say "Disabled" in words; pass-through could say "Same button"
  or be dimmed with text (`02-general.png`). **Done:** A button left as itself reads "Same button" in gray.

## The same thing worded differently

- [x] 18. **Button names follow different patterns on Controller shortcuts.** That tab has
  "X / Square (West)", "Guide / Home", "Select / Back", "Start" and "Left Bumper", while the Buttons
  list has "X (West)", "Back (View)", "Start (Menu)" and "Left Bumper (LB)". Item 14 set
  "Name (abbreviation)"; the shortcut pickers don't follow it (`04-settings-shortcuts.png`). **Done:** The shortcut pickers name buttons for the controller in use, as the Buttons tab does (`info::button_name`, which now names Guide too).
- [x] 19. **"Listen" and "Find by pressing" do the same job.** The shortcut rows say "Listen" and
  the profile editor says "Find by pressing". Use one word for both (`04-settings-shortcuts.png`). **Done:** "Listen" is "Set by pressing", next to "Find by pressing" (they do different jobs: one sets, one finds). The manual says so too.
- [x] 20. **Labels switch to Title Case in a few places:** "+ Create New Profile…", "Default
  Profile", "Quick Settings menu", while everything else is in sentence case ("Duplicate", "Find by
  pressing", "Overlay font") (`02-general.png`, `03-settings.png`). **Done:** "Default profile", and "+ New profile…", like "+ New layer" and "+ New macro". "Quick Settings" stays: it's the name of a panel.
- [x] 21. **"Mouse mode keys" lists things that aren't keys:** Middle click, Mouse back and Mouse
  forward. Maybe call it "Mouse mode buttons" or "In mouse mode" (`04-settings-shortcuts.png`). **Done:** "Mouse mode buttons".
- [x] 22. **Labels in the same card repeat their heading on one card but not another.** "On-screen
  numpad buttons" has "Numpad backspace", "Numpad enter", "Numpad close", but "Media controls
  buttons" has "Next track" and "Close", not "Media close" (`04-settings-shortcuts.png`). **Done:** Rows in a card leave out the card's name ("Backspace", "Enter", "Close"); warnings, read out of context, keep it.
- [x] 23. **Three red treatments for Delete.** Profile and setup Delete are solid red, item Delete
  ("Delete menu", "Delete macro", "Delete info overlay") is pale pink in light mode and dark red in
  dark mode, and rows use ✕. After a8ef0ee the item buttons are pink on every closed card again, so
  the Menus list is three pink buttons stacked (item 1 had moved them inside or quieted them)
  (`34-menus.png`). **Done:** Every Delete on a page (profile, setup, item) is the light red `quiet_danger`, dark red on hover; only a dialog's final Delete is solid red. An item's Delete shows only while its card (or combo row) is open.
- [x] 24. **The "You decide this; it isn't detected…" note is written three times,** under back
  paddles, gyro and touchpad, with slightly different wording ("lacks a ticked feature" and "has no
  paddles"). Each is two or three lines. One shared sentence, or an ⓘ, would do
  (`20-buttons.png`, `25-gyro.png`, `26-touchpad.png`). **Done:** One sentence for all three, behind an ⓘ beside the box.
- [x] 25. **The ellipsis is written two ways.** Placeholders such as "Search setups…" use "…", but
  "Search the library" in the Add dialog has none. Check that every "…" is the single character
  (`11-dialog-add.png`). **Done:** "Search the library…".
- [x] 26. **Card titles on the profile sub-tabs don't match the tab.** The "Guide" tab opens on cards
  called "Notes" and "Mapping guide", and the "Sticks & triggers" tab opens on "Sticks"
  (`27-guide.png`, `23-sticks.png`). **Done:** "Guide notes". "Sticks" and "Triggers" are the two cards of their tab, so they stay.
- [x] 27. **Info overlay summary wording is awkward:** "3 rows · always shown (not by actions)".
  Something like "3 rows · always on screen" reads better (`37-info-open.png`). **Done:** "3 rows · always on screen".
- [x] 28. **The info overlays blurb uses "e.g."** ("Text and button glyphs on screen, e.g. a game's
  controls."). No other text in the app abbreviates; use "such as" (`36-info.png`). **Done:** "such as a game's controls".
- [x] 29. **Quotes are mixed in one paragraph.** The Info overlay "Shown" note has straight quotes
  ("Toggle → Show info overlay…") where the rest of the UI uses curly ones (“Deus Ex + Gyro”)
  (`37-info-open.png`). **Done:** Curly quotes.
- [x] 30. **"held for" is the only lowercase field label** in the macro editor; other labels start
  with a capital ("Name", "Steps", "Add step:") (`33-macro-open.png`). **Done:** "Held for", and a wait's slider is labeled "For".
- [x] 31. **Total time is shown twice in a macro:** "3 steps · 130 ms" in the header and "Plays for
  130 ms" at the bottom right of the same card (`33-macro-open.png`). **Done:** "Plays for" is gone; the header keeps the total.
- [x] 32. **The live picture's note repeats the feeds' note.** On Overview, "Press a button on a
  managed controller" is shown three times: under the picture and in two of the three activity
  feeds. The picture's note ends without a period, the feeds' end with one (`01-overview.png`). **Done:** The feeds say "Nothing pressed yet." and "Nothing sent yet.".
- [x] 33. **The Manual's contents list starts 12 entries with "Tutorial:".** A "Tutorials"
  subheading would shorten them and make the list easier to scan. One also has a stray capital:
  "Tutorial: a Driving mode…" (`05-manual.png`). **Done:** Unsearched, the tutorials sit under a "Tutorials" heading without the prefix.

## Controls that look or behave differently for the same job

- [x] 34. **The Overlay font dropdown has its own arrow,** a small black ▾ where every other
  dropdown has the accent-colored chevron (it's the font picker that draws names in their own
  font) (`03-settings.png`, `10-details.png`). **Done:** The font picker draws its arrow in the accent color, and itself as a field (`style::field_button`).
- [x] 35. **The Tutorial ↗ link moves around.** It's at the far right of the sub-tab row, then jumps
  left when Expand all and Collapse all show. It's under the Copy button on items tabs, beside
  "+ Add rule" on Details, and above the list in the Add dialog. Pick one place, such as next to the
  card or tab title (`20-buttons.png`, `23-sticks.png`, `30-modes.png`, `10-details.png`). **Done:** The link is always at the right end of its row: after Expand all and Collapse all, and at the far right of "+ Add rule".
- [x] 36. **The "Add a menu:" buttons look like a segmented filter, not like add buttons.** "Radial
  · Directional · List · Button menu · Carousel · Grid" have no "+", unlike "+ New macro",
  "+ New layer" and so on (`34-menus.png`). **Done:** "+ Radial", "+ Directional" and so on.
- [x] 37. **The ✕ for a gesture is at the far right edge of the row,** about 400 px from the field
  it removes (X (West) → Long press, LB → Double tap). It's easy to miss which row it belongs to
  (`22-buttons-expanded.png`). **Done:** The ✕ sits right after the action it removes.
- [x] 38. **The preview is placed differently on each kind of item.** On an info overlay it's above
  "Cells"; on a menu it's below "Items" (`35-menu-open.png`, `37-info-open.png`). **Done:** As on a menu: under the Appearance editor while it's open, after the cells otherwise.
- [x] 39. **The Add a setup dialog's main button sits in the middle of the right pane,** right under
  the description ("Preview & add…"), while Cancel is alone in the dialog's bottom right. Other
  dialogs put both at the bottom right (`11-dialog-add.png`). **Done:** "Preview & add…" (or "Open it") sits beside Cancel.
- [x] 40. **The System check card's "Check…" button sits at the card's right edge,** while every
  other card's buttons ("Open it now", "Take the tour", "Turn everything off") sit on the left
  (`50-settings-tall.png`). **Done:** System check is a line in the Help card, its button first (see 58).
- [x] 41. **The "Info overlays" card on App settings has no ⓘ by its title,** but it has one beside
  the Fallback glyphs dropdown. Every other card has the ⓘ by its title (`50-settings-tall.png`).
  The "Guide overlay" card on Controller shortcuts also has none, and puts its long explanation in
  the card instead (`04-settings-shortcuts.png`). **Done:** Its ⓘ is by the title. The Guide overlay card's paragraph is its ⓘ now.
- [x] 42. **Pages start differently.** A setup's page has a title ("Deus Ex"), General has one with
  an ⓘ, Deus Ex's has none, and App settings has no title at all and starts with its tabs
  (`02-general.png`, `03-settings.png`, `20-buttons.png`). **Done:** App settings has a title, and every setup's title an ⓘ.
- [x] 43. **The Overview uses horizontal rules between sections, and every other page uses cards.**
  "Live input" has a "Hide" button on profile pages but not on Overview (`01-overview.png`). **Done:** The Overview's parts are cards, and its Live input can be hidden.
- [x] 44. **Columns don't line up between the cards on Controller shortcuts.** Rows with a kind
  dropdown ("Button") put the button picker at x≈583; Media controls and numpad rows have no kind
  dropdown, so their picker starts at x≈445, under the other cards' kind dropdowns
  (`04-settings-shortcuts.png`). **Done:** Rows with no kind dropdown leave its width, so every button picker starts at the same place.
- [x] 45. **The macro step sliders don't line up.** "held for" sliders start at x≈476 and the Wait
  slider at x≈418, and each has a number field too, so the length shows twice
  (`33-macro-open.png`). **Done:** Wait and tap sliders share a label width and length. The number field stays: a slider can't hit 17 ms.
- [x] 46. **Text fields look like gray buttons.** Name, search and key fields have the same gray fill
  and height as secondary buttons, with no edge, so "Search this profile…" next to "Find by
  pressing" reads as two buttons. The Export dialog's Description box is the only white field with
  an edge (`20-buttons.png`, `12-dialog-export.png`). **Done:** Fields and dropdowns have the page's color with an edge, so they don't look like the filled buttons.

## Picture callouts and summaries

- [x] 47. **Leader lines still cross the face buttons.** The lines for X ("“;” +") and A ("Right
  click") pass through the B and A circles, and the left stick's line crosses the D-pad's. Item 43
  stopped lines crossing each other, but not crossing other buttons (`20-buttons.png`). **Done:** A leader line takes the shortest path that keeps clear of every other part, bending around them where it must (`route` in `pad_svg/labels.rs`); labels are swapped until the fewest lines cross. With every input mapped at once, one crossing can remain.
- [x] 48. **The picture shows only stick clicks, not what the sticks do.** On Sticks & triggers,
  Deus Ex's left stick is a "Button ring" (WASD), but the picture's only label for the left stick is
  L3's "Toggle X" (`23-sticks.png`). **Done:** A stick's mode joins its click's label: "Ring · Toggle X", "Mouse · “[” +".
- [x] 49. **Mouse actions are drawn two ways in summaries.** "Right click" is a teal chip, but "Scroll
  up" and "Scroll down" are plain text. Use one look for mouse outputs (`20-buttons.png`). **Done:** Wheel turns are teal chips like mouse buttons (`Piece::Wheel`).
- [x] 50. **A lone apostrophe key is still close to invisible** in the summary chip (X (West) →
  Long press: `'`), and so is the `;`. Item 48 squared the chips, but the mark itself is still a
  sliver (`20-buttons.png`). **Done:** The apostrophe, backtick, comma and period are named in chips ("Apostrophe").
- [x] 51. **D-pad glyph pairs in the Mapping guide are hard to read.** "✚, ✚" for "Previous / next
  belt item" only shows which arm is meant by a small blue highlight (`27-guide.png`). **Done:** An arrow follows each D-pad glyph.
- [x] 52. **The Guide tab and menu items show raw tokens with no rendering:** "{start} (Escape)
  backs out…", "{icon:bag} Inventory, goals", "{south}". The info overlay has a preview to check
  them against; the Notes and Mapping guide have none (`27-guide.png`, `37-info-open.png`). **Done:** The Guide notes have a preview drawn as the overlay draws them (`info::notes_panel`). Menu items already had one.

## Spacing, density and type

- [x] 53. **Controller shortcuts opens with two walls of text:** a two-line intro, then a six-line
  paragraph in the Guide overlay card. Most of it could go behind an ⓘ like the other cards
  (`04-settings-shortcuts.png`). **Done:** The intro is one line with an ⓘ; the Guide overlay paragraph is the card's ⓘ.
- [x] 54. **Small hints sit in narrow columns.** "Opens, moves, picks and closes, then pauses and
  repeats." wraps onto three lines in a 160 px column beside the preview, at the smallest size
  (`03-settings.png`). The keyboard appearance editor's formatting hint ("Bold, italic, underline
  and color go inside the text…") is in the smallest gray type too (`52-details-tall.png`). **Done:** The motion hint is an ⓘ; the formatting hint is a `note`.
- [ ] 55. **Too many nested panels.** On Sticks: a gray card holds a white card, which holds a gray
  "Visualizers" panel and gray "Several at once" panels. On Gyro, one dropdown sits in a white card
  of its own inside the gray card. Item 68 made some of these the same style, but the nesting is
  still three deep (`23-sticks.png`, `25-gyro.png`). **Open:** No change: the card, input box and panel nesting is the shared `device_box` layout, so Sticks, Gyro and Touchpad read alike.
- [ ] 56. **Each menu item takes two lines and a card,** with the key field alone on the second
  line. Eleven items fill about 1050 px. The key field could sit on the first line next to its kind
  (`35-menu-open.png`). **Open:** Partly: the label is wider. The key field still wraps under its dropdown; a row can't fit both, the size slider and the arrows at 1280 px.
- [ ] 57. **Info overlay cells are crowded.** Each cell is a field plus "Insert…", ☺ and ✕, so a
  two-cell row has eight controls, then "+ Cell" and the row arrows (`37-info-open.png`). **Open:** No change: each control has its own job (tokens, characters, removing).
- [x] 58. **Many App settings cards hold one control:** Button layout (one switch), Menu colors (one
  switch), Media controls and In-game menu (one "▸ Appearance" each), Help (one button). Grouping
  them would make the page shorter (`50-settings-tall.png`). **Done:** System check and the tour share the Help card. Button layout and Menu colors stay apart: Details mirrors the one, and the other is about overlays, not the window.
- [x] 59. **The visualizers don't match each other.** On Sticks, "Ring" is a bare circle with no
  frame, while "Stick travel" and "Response" are dark squares, and the captions are tiny. The ring
  doesn't label its sectors with their keys (W, D, …) (`23-sticks.png`). **Done:** The ring has the dark panel the graphs have, its wedges a step lighter. Labeling sectors with their keys isn't done.
- [ ] 60. **Mapping guide rows repeat a "Share with" dropdown and a "Show" checkbox on every row,**
  about 20 of each. The page is a grid of identical controls (`27-guide.png`). **Open:** No change: each row can be shared or hidden on its own.
- [x] 61. **The Export dialog's fields have three widths** (Version narrow, Author and Made with
  medium, Description to the dialog's edge). The "Description" label is centered on the tall box
  instead of at its top (`12-dialog-export.png`). **Done:** The Description label sits at the top of its box. The widths stay: a version needs less room than a name.
- [x] 62. **The System check dialog says "padwight doctor --explain" twice,** once in the gyro item
  and once at the bottom, in body text rather than code type. The ✗ / • legend at the top is hard
  to follow (`13-dialog-system.png`). **Done:** Once, at the bottom, in code type; the legend reads "✗ doesn't work yet · • worth knowing · ✓ in place".
- [x] 63. **The header row's label and dropdown are uneven.** "Active profile" is body text, the
  dropdown is 260 px, and the unlabeled padlock sits on its own. The sidebar's selected entry is a
  heavy lavender fill, strong next to the subtle selected tab (`01-overview.png`). **Done:** The open page is a softer tint. The padlock has a tooltip.
- [x] 64. **The sidebar's blue dot beside "General" has no legend.** It presumably marks the active
  setup, but nothing says so, not even a tooltip (`01-overview.png`). **Done:** The dot, ⚠ and "update" notes say what they mean on hover.
- [x] 65. **The sidebar keeps its full 210 px in a narrow window,** while the content loses its tabs
  (item 1). Narrowing or collapsing the sidebar below about 1100 px would leave room for them. **Done:** No change needed: with the tabs wrapping (1), a narrow window keeps every tab in reach.

## Tooling

- [x] 66. **`padwight debug gui --dark` paints the window white.** `src/gui/shoot.rs:252` always
  passes `Color::WHITE` as the background, so in dark mode the sidebar, page title, header and
  footer show light text on white and can't be read (`dark/*.png`). Use the theme's background
  color. Dark mode was judged here only on cards and dialogs, which look right. **Done:** The tool paints the theme's background.
- [x] 67. **The made-up controller has no device path,** so the Overview line under its name starts
  with a stray "·" ("· remapping · gyro"). Real controllers show `/dev/input/eventN` there, which is
  itself more technical than the line needs (`src/gui/settings.rs:226`). **Done:** An empty device path is left out.