# Settings window polish

A list of visual inconsistencies, polish points and improvements found by going through every
page of the settings window (Overview, App settings, Manual, a setup's tabs and profile
sub-tabs, the add, import, export and delete dialogs, and dark mode). Work through it one item
at a time, with a before and after screenshot for each. Screenshots should fall in an untracked directory inside the repository.

Tick an item when it's done and merged.

## Broken layout

- [x] 1. Rows overflow their cards on the Sticks tab, Macros and Menus. Wide action editors ran
  off the card edge into a sideways scrollbar, and the right-hand controls were cut off. Editors
  now wrap onto a second line instead.
- [x] 2. The Mapping guide's columns don't line up. The D-pad row's text field is wider than the
  others, the "Lean" row has an "Add an input" list and a blue "Split" button, and the hold and
  ×2 rows have no "Share with" column. The Show and controls columns now have fixed widths on
  every row, and Split and Remove are grey buttons of one size.
- [x] 3. The "daemon not running" note on the Overview prints the backticks around
  `systemctl --user start padwight` and `padwight daemon` (`src/gui.rs`). The commands are now
  set in monospace instead.
- [x] 4. The Guide tab shows the raw token `{keyboard:apostrophe}` as the label for "X hold".
  A row's field now holds only text the author wrote, and the default shows as plain words in
  its placeholder ("Press “'”"), since a text field can't draw glyph tokens.

## Contradictory or misleading states

- [x] 5. With the daemon down, Overview says "No profile has switched since the daemon started."
  It now says "Needs the daemon.", like the two feeds above it.
- [x] 6. The "Remapping enabled" toggle is greyed out while the daemon is down, so it looks off,
  but its label still says "enabled". It now reads "Remapping on" or "Remapping off", and is off
  while the daemon is down, since nothing is remapped then.
- [x] 7. "Revert" looks clickable when there is nothing to revert, while "Save & apply" is greyed
  out next to it. Every disabled button now has one look, whatever its style: no fill, a faint
  edge and grey text.
- [x] 8. The disabled "Save & apply" is a pale blue that is hard to read, and "Find by pressing"
  barely looks different when disabled. Both use the disabled look from item 7, which keeps the
  text readable.
- [x] 9. The sidebar says "No setups yet. Use + Add setup…" while General is listed just above
  it. The search box shows even when there is nothing to search. It now says "No game setups
  yet", and the search box shows only once there is a game setup.
- [x] 10. On a profile with no bindings, the controller picture shows a lone "Layer "Guide""
  callout and nothing else. Guide's usual mapping (which the engine ignores) is no longer drawn,
  and a picture with no callouts says "Every input passes through unchanged" under it.

## The same thing worded differently

- [x] 11. "Needs the daemon" has four wordings in two sizes: "Live input needs the daemon",
  "Needs the daemon.", "Unavailable while the daemon is not running." and "Automatic switching
  needs the daemon to be running." The Controllers one is in a larger font. All four now say
  "Needs the daemon." in small grey text.
- [x] 12. An unbound input is "(nothing)" in the picture callouts but "Disabled" in the button
  list (`src/gui/profile.rs`). The callouts say "Disabled" too.
- [x] 13. The same key is written three ways: "SPACE", "Space" and "“;”". The key field now
  shows the keyboard names the summaries use ("Space", "Left Ctrl+C") and takes them typed in,
  as well as the old code names. Arrows read "Up", "Left" and so on, and keys whose name has a
  "+" (Vol +) keep their code name, so the field can be typed back. Summaries still quote a lone
  punctuation key (“;”) so it stays visible in a sentence.
- [x] 14. Button names follow different patterns: "A (South)", "Left Bumper (LB)", "Back (View)",
  "Left Stick Click", then "L3" and "R3" on the Guide tab and "Stick clicks" in log overlays.
  Names follow "Name (abbreviation)": "Left Stick Click (L3)", and "Stick clicks (L3, R3)" in log
  overlays. Glyphs stay short, like every other glyph.
- [x] 15. Stick names are capitalised differently: "Left Stick" in lists, "right stick" in hints.
  Sticks, bumpers, triggers and D-pad directions are capitalized everywhere in the window, the
  overlays, the controller editor and the manual.
- [x] 16. The app name has two casings: "Padwight" in the window title and page header,
  "padwight" in the manual and in "by padwight" and "Exported by padwight". It is "Padwight" in
  prose, the library packs' author included, and `padwight` only as a command.
- [x] 17. The settings page has two names: "Same as Settings" on the Details tab, and
  "App settings" in the sidebar and hints. It is "App settings" everywhere.
- [x] 18. British and American spelling are mixed: "Menu colours" and "Colour-blind mode" in the
  UI, "color" in `src/gui/widgets.rs` and `src/gui/tab_icons.rs`. Everything is American now
  (color, center, gray, labeled), in the UI, the docs and the code. The config file keeps its
  `colourblind_tones` key, so existing files still load.
- [x] 19. Unit spacing varies: "250 ms" on sliders, "40ms" in the log preview, and "0 ° from up"
  with a space before the degree sign. Units take a space ("40 ms") and the degree sign doesn't
  ("0° from up", "120°/s").

## Controls that look or behave differently for the same job

- [x] 20. "Use its own appearance in this setup" is a checkbox on four cards on the Details tab,
  while Button layout is a segmented control, Overlay font a dropdown, and Overlay sounds a full
  editor.
- [x] 21. Toggles and checkboxes are mixed for on/off settings: toggles for "Controllers this
  setup supports" and Overlay sounds, checkboxes for "Can't be played without…" and rule enables.
- [x] 22. Button sizes vary: "Make active", "Hide", "Expand all" and "Collapse all" are small or
  unstyled, while "Duplicate", "Find by pressing" and "+ New macro" are full size.
- [x] 23. The top of each items tab differs: Layers has a bare "+ New layer" button, Macros, Info
  and Log overlays have a card with a description, and Menus has an "Add a menu:" row. The
  "Copy from another setup…" link sits in a different place on each.
- [x] 24. Disclosure triangles come in two sizes: a large ▾ on item cards and a small ▸ on
  "Appearance" and "Shared … usable here (0)".
- [x] 25. "Remove" is shown three ways: a small red underlined link under a gesture label, "×"
  buttons on rules, cells and macros, and "×" on the card corner to delete a whole item.
- [x] 26. Some cards lack the ⓘ icon. "Controller requirements" and the four appearance cards on
  Details have none, and every other card has one.
- [x] 27. "Expand all" and "Collapse all" appear on tabs with nothing to expand (Combos, Guide)
  but are hidden on Gyro.
- [x] 28. On the Details tab, "Name" sits outside any card, while everything else is in one.
- [x] 29. The profile dropdown and the rename field next to it are both unlabelled and both
  read "Deus Ex".
- [x] 30. Mapping summaries use colours that look like errors or links with no legend: "Menu" in
  red, "Toggle" in green, "Right click" in teal.

## Spacing and density

- [x] 31. The header (title, remapping toggle, active profile, daemon note) repeats on every page,
  including the Manual. With the daemon down it pushes content about 230 px down. The header is
  now one row (the window title already names the app), with "○ Daemon not running" beside the
  switch it explains. The note on starting the daemon shows on the Overview only, and the Manual
  has no header.
- [x] 32. The radial menu preview is about 750 × 800 px and pushes the item list well below the
  fold. Menu previews are drawn at 80%, and sit after the items, or under the Appearance editor
  while it's open, so each change shows.
- [x] 33. Overlay sounds is a long wall of small sliders: four sounds with three sliders each, for
  every overlay type, repeated on App settings and on every setup's Details tab. Each overlay is
  a switch with an "Adjust" disclosure, closed at first, and each sound is one line: kind, Play,
  volume, pitch and length.
- [x] 34. Sound labels mix case ("Step", "Pick" against "volume", "pitch", "length") and use a
  smaller font. They're all capitalized and at the body size; only the percentages are small.
- [x] 35. Long explanations are in very small grey text: the controller-requirements note, the
  "Shown" note on info overlays, and the export notes. They use a new `note` style: 14 px, in the
  theme's text color softened, rather than the faint caption grey.
- [x] 36. Dropdowns have different widths on the same page: 280, 200 and 160 px on App settings.
  Settings dropdowns and the header's share one width (`SETTING_WIDTH`, 260 px).
- [x] 37. In the Add a setup dialog the "installed" tags touch the scrollbar, and the right pane
  is mostly empty. The list leaves a gap for the scrollbar, and the first entry is shown until
  another is picked.
- [x] 38. In the Add and Export dialogs the item counts include zeros ("0 layers · 0 macros …")
  and wrap so that "overlays" sits alone on a line. Kinds with none are left out, and each count
  is held together by no-break spaces.
- [x] 39. In the Export dialog the Description field is one line and cuts off its text, the
  Author placeholder repeats its label, and "Library pack: keep the library's ID" uses an
  internal concept. The description is a box that wraps, the placeholders are examples ("Your
  name"), and the box reads "Release it as the built-in Deus Ex: people who added the built-in
  one get it as an update".

## Dark mode

- [x] 40. With the daemon down, the controller picture is barely visible against the background,
  and its callout lines disappear. The leader lines are a layer of their own, never dimmed, and
  a little stronger. The controller is dimmed less in dark mode, since it's drawn dark.
- [x] 41. The selected tab is only slightly darker than the tab bar, so it is hard to see which
  tab is active. The selected segment is lighter than the bar in both themes, with an accent
  edge.
- [x] 42. The menu and overlay previews are always dark navy: fine in dark mode, but a heavy slab
  in light mode. The "screen" behind a preview follows the theme; the overlay itself keeps its
  real colors.

## Picture callouts

- [x] 43. Leader lines cross each other around the face buttons and the right stick. Each label
  sits as near its part's height as the rows allow, and labels whose lines cross swap places
  until none do (a test checks every input on each layout).
- [x] 44. Long labels are cut on the right ("Menu "Augme…") but not on the left
  ("Menu "Belt" +"). The label columns are wider, so most names fit, and a name that's still
  cut keeps its closing quote ("Menu “Augmentat…”").
- [x] 45. A trailing "+" ("Esc +", "(nothing) +") means there is more, but nothing explains it.
  When a callout has one, a line under the picture says what it means.

## Second pass (2026-10-11)

Found by stepping through every page, tab and dialog with the daemon down, in light and dark
mode, at 1300 × 1900. Screenshots are in `screenshots/review-2026-10-11/` (untracked), named
by page.

### Broken layout

- [x] 46. The Export dialog's Description box cuts its last visible line through the middle of
  the letters and shows no scrollbar, so it isn't clear the text goes on. The box grows with the text up to ten whole lines, then scrolls, so no line is cut through.
- [x] 47. "Copy from another setup…" wraps onto two lines on the Menus tab, where the six
  "Add a menu" buttons crowd it. On Layers, Macros, Info and Log overlays it fits on one line. The blurb takes the free width and wraps instead, so the copy button stays on one line.
- [x] 48. Lone punctuation keys are close to invisible. The Guide tab's "X hold" placeholder
  reads `Press “'”`, which looks like empty quotes, and in the Buttons list X (West) shows the
  `;` and `'` key caps as thin slivers. The picture callouts quote them (“;”), but the list
  doesn't. One-character key caps are square and a lone punctuation mark is drawn larger. The apostrophe and backtick are named ("Press Apostrophe") instead of quoted.

### Contradictory or misleading states

- [x] 49. The Combos tab with no combos shows the "Combo window" slider, which means nothing
  until there is a combo, and has no "No combos yet" line like the items tabs have. The tab says "This profile has no combos yet.", and the window slider shows once there is a combo.
- [x] 50. "● active" shows next to "Edit profile" for kitty's Gamepad while the daemon is down
  and remapping is off. Other profiles show nothing, because "Make active" is hidden then. "● active" shows only while the daemon runs.
- [x] 51. A disabled "Delete" (a setup's only profile) gives no reason. A tooltip on the disabled Delete says a setup needs at least one profile.
- [x] 52. In light mode the controller picture's "LT" and "RT" labels are white on light gray and
  can't be read. In dark mode the picture isn't dimmed while the daemon is down, so the
  "not live" cue only shows in light mode. Light gray letters on the dimmed picture turn dark gray in light mode, and the dark-mode picture is dimmed to 50% so the cue shows there too.
- [x] 53. Overview's "What's happening" has a heading "Exported by Padwight", which reads like a
  pack export rather than the output feed. Its two-column grid leaves "Profile switches" alone
  on the left with an empty right half. The feed is "Sent to the game", and the three feeds sit side by side.
- [x] 54. The picture's callouts draw "Disabled" as a key cap, the same dark pill as "Space" or
  "F1", while the list below shows it as gray text. "Disabled" callouts have gray text and a gray edge.

### The same thing worded differently

- [x] 55. One overlay has several names: "Input logs" under Overlay motion but "Log overlays" on
  the tab, and "Numpad" in the motion and sound lists but "On-screen numpad" on its card. The
  motion list has Info overlays and Input logs but no Guide menus, and the sound list has Guide
  menus but no Info or Log overlays. Nothing says why. The lists say "On-screen numpad" and "Log overlays". The motion help says Guide menus move as Menus do, and the sounds help says info and log overlays make no sound.
- [x] 56. Similar values show in different forms on the Sticks tab: Deadzone "0.12" next to the
  graph's "Flat for the first 12%", Starts at "0.25 of the way out", Stickiness "0.10 of a
  sector". Choose either percentages or fractions. Every share is a percentage: "12%", "25% of the way out", "10% of a sector", and the trigger's "Presses at" and gyro anti-deadzone too (`fraction_editor`).
- [x] 57. "Two fingers scroll" should be "Two-finger scroll". The Response hint "the same
  sensitivity at every speed" starts in lowercase, where other hints are sentences. "Two-finger scroll", and the response presets' descriptions are sentences.
- [x] 58. Rules run the kind into the value: "Switches on for: Executable Against the
  Storm.exe" in the Add dialog, "Rules: Executable DeusEx.exe, Steam App ID 6910" in the
  Delete dialog. Rule values are quoted: Executable “DeusEx.exe”, as the daemon's switch reasons already were.
- [x] 59. The Copy from another setup dialog closes with "Close". Every other dialog uses
  "Cancel". No change: copying applies at once, so there's nothing to cancel, and "Close" is right.
- [x] 60. The appearance editor repeats its labels: "Width  ☐ Set width", "Height  ☐ Set
  height". The checkboxes read "Fixed".
- [x] 61. The Deus Ex library pack capitalizes its Guide text unevenly: "holster" and "quick
  save" next to "Quick load" and "Jump". "Holster" and "Quick save" in every library pack. Setups already added keep their text.

### Controls that look or behave differently for the same job

- [x] 62. Toggles and checkboxes are still mixed for on/off (see item 21). Toggles are used on
  App settings and for the Details tab's "Use its own…". Checkboxes are used for "Controllers
  this setup supports", the touchpad options (Split, Tap to click, Two fingers scroll, Keep
  gliding), the menu's "Use boxes", the appearance Decorations, rule enables and the Guide's
  "Show". Kept, as one rule: switches for page-level settings on App settings and Details; checkboxes inside editors, for picking from a set, for optional values and for statements about the profile ("Can't be played without…"). The touchpad options and "Use boxes" sit in editors, so they stay checkboxes.
- [x] 63. Delete has four looks: a red "Delete" for a profile, a red "Delete setup", gray
  "Delete menu" text in the card's corner that doesn't read as a button, and "×" for rows. The
  "×" also moves around. It's at the card's far right edge for a button's extra gestures, right
  after the key field in "Several at once…", and right after the dropdown on auto-switch rules. Three kinds, by what goes: red buttons for a profile or a setup, an outlined quiet button (red on hover) for an item, ✕ for a row. "Delete menu" and the other item deletes now have a button's edge. The ✕ positions are left as they are.
- [x] 64. "Delete setup" sits in the "Pack" card next to "Export…". Deleting a setup has
  nothing to do with packs, and kitty, which isn't from a pack, still gets a "Pack" card. "Delete setup" sits on the Setup card's Name row. The Pack card holds only the pack and Export, and has an ⓘ.
- [x] 65. Button sizes still vary (see item 22). "+ Add rule", "+ Add item", "+ Add output",
  "+ Double tap", "Turn every sound off", "Turn everything off" and the Add a menu buttons are
  small. "+ New layer", "+ Add combo", "Open it now" and "Hide" are full size. One rule: page and tab level buttons (the items header, profile bar, Live input) are full size, buttons inside a card are small. The Add a menu buttons are full size now, "+ Add combo" and "Open it now" small, and the Guide's "+ Add a row" a secondary button.
- [x] 66. "Copy from another setup…" is plain body text with no link or button look. It's a secondary button.
- [x] 67. Dropdowns with a gray placeholder ("+ From template…", "Empty setup from a
  template…", "Share with", "Pick a setup…") look disabled. Dropdowns that run an action ("+ From template…", "Empty setup from a template…", "Pick a setup…", "Share with", "Add an input") draw their label in the text color (`style::action_dropdown`).
- [x] 68. Nested editors each have their own background. Expanded button rows sit in white
  cards, "Several at once…" in a lavender box, Visualizers in a gray card inside a gray card,
  and the Gyro tab's single dropdown in a white card of its own. A "Several at once" list, and the action a Toggle or Turbo wraps, use the panel style, like the Visualizers.
- [x] 69. "Can't be played without back paddles" sits inside the Back paddles card, while the
  same choice for gyro and touchpad gets a separate "Controller requirements" card. The gyro's and touchpad's "Can't be played without…" sit in their own cards, under the missing-hardware note, as the paddles' does.
- [x] 70. The ⓘ is still missing on some cards (see item 26): the "Setup" and "Pack" cards on
  Details, the Sticks "Visualizers" (the Touchpad's has one) and a menu's "Items". Setup, Pack, Visualizers on Sticks and a menu's Items have one.
- [x] 71. Each menu item has a small slider (its weight) with no label and no value. The slider shows its value ("Size 1.00×"), and the Items ⓘ explains it.
- [x] 72. In the appearance editor, the opacity sliders next to each color have no label, and
  Corners shows a word ("Rounded") where every other slider shows a number. Each opacity slider is labeled "Opacity", and Corners shows its percentage before the name ("10% · Rounded").
- [x] 73. On App settings, "When no setup matches, use" puts its label and dropdown at the
  right end of the row. Every other setting has a label column and a dropdown at a fixed
  position. "No setup matches" is a labeled row under the switch, in the label column.
- [x] 74. Shared cards come in a different order on each page. App settings has Appearance,
  Button layout, Overlay font, Overlay motion, Overlay sounds, then the keyboard, numpad and
  so on. Details has Overlay font, keyboard, numpad, media controls, in-game menu, Button
  layout, Controllers, Overlay sounds, then Overlay motion. Details follows App settings' order: Setup, Auto-switch rules, Button layout, Controllers, Overlay font, Overlay motion, Overlay sounds, the overlay cards, then Pack.
- [x] 75. Two controls turn every sound off: the "Turn every sound off" button under Overlay
  sounds and the "No sounds" switch under Performance. App settings drops "Turn every sound off" and points to Performance's "No sounds" in the help. A setup's own sounds keep the button.

### Spacing and type

- [x] 76. Explanations still use three styles (see item 35). The Performance descriptions and
  the "You decide this; it isn't detected…" text are full-contrast 14 px, "None of your
  managed controllers…" is the faint caption gray, and the Response hint is smaller again. The missing-hardware notes use `note` too. Short hints beside a control stay captions.
- [x] 77. Field labels come in different sizes. "Profile" and "Name" on the profile bar and
  "When", "is" and "use" on rules are small, while labels elsewhere are body size. Field labels on the profile bar and on rules are body size.
- [x] 78. The appearance editor's rows are indented, but its "Decorations" and "Preview"
  captions sit at the card's left edge. "Decorations" is indented like the editor's labels. "Preview" is full width, so it stays at the edge.
- [x] 79. Visualizers are large dark slabs in light mode. The touchpad's "Pad (live)" is about
  420 × 235 px of near-black with one word ("Mouse") in it. The stick's "Stick travel" and
  "Response" graphs are dark too. Item 42 changed only the menu and overlay previews. Partly: the touchpad picture is 320 px wide instead of 420. The instrument pictures stay dark, like the controller.
- [x] 80. The radial menu preview is still about 630 × 680 px at 80%, taller than the
  10-item list above it. No change: below 80% the item labels can't be read (see `MENU_PREVIEW_SCALE`).
- [x] 81. The Delete dialog uses three text sizes: the title, "This removes:", then a smaller
  list. "This removes:" and the list are both 14 px.
- [x] 82. Revert and Save & apply show on the Manual page, where nothing can be edited. The footer hides on the Manual unless there's something to save or a message.
- [x] 83. The sidebar's "Search setups" field has the same gray fill and height as the
  "+ Add setup" button, so the field and the button look alike. "+ Add setup" is a sidebar entry in the accent color, and the field reads "Search setups…".

### Dark mode

- [x] 84. Off switches and unticked checkboxes are dark gray on dark cards and hard to see: the
  Details tab's "Use its own…" switches, the header's "Remapping off" switch and "Controllers
  this setup supports". Switches and checkboxes go through `style::switch` and `style::tickbox`. Off switches have an edge and, in dark mode, a light knob; empty checkboxes have an edge drawn from the text color.
- [x] 85. A modal dialog has the same color as the cards, and the dimming behind it is light, so
  the dialog barely stands apart from the page (Add a setup). Dialogs and the key picker use `style::dialog`: an edge, a shadow and, in dark mode, a surface a step lighter than the cards.