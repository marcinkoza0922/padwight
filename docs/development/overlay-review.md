# Overlay and menu review

A pass through every on-screen system: the on-screen keyboard and numpad, action menus (all six
kinds, held, toggled and unwatched, plus touchpad menus), the Guide overlay and its menu, the
in-game menu (Guide + Start: Quick Settings, Edit Controls, the unsaved-changes prompt), media
controls, the library offer, toasts and the force-quit hold, and the overlay process and drawing
code. Findings come from reading the code on `main` (as of `ade6c7b`) and from the overlay
screenshots. Nothing here has been tried on hardware yet. Items marked *verify* are inferred from
the code and need a quick test before fixing.

Tick an item when it's done and merged.

## Bugs

- [x] 1. **Two-column lists can't switch columns.** `list_step` handles left and right (and it's
  tested), but `MenuSession::list_button` only sends D-pad Left/Right for grids
  (`src/menu.rs:418`), and a list's stick step only reads `LeftY`. A list of 9+ items is drawn in
  two columns, but reaching the right column means scrolling down through the whole left one.
  Edit Controls does support left and right on its two-column pages, so action menus are the odd
  one out.
- [x] 2. **In a Buttons menu, a quick-select button can take over navigation.** Quick-select is
  checked before navigation (`src/menu.rs:391`), and the GUI offers every button as a quick-select
  (`src/gui/items.rs:1205`), including A, the D-pad and B. The sample `menu-buttons.png` shows
  this: "A Quick save" means pressing A always runs Quick save, never the highlighted "Crouch".
  B as a quick-select in an unwatched menu never fires because cancel wins. Two items with the
  same button: the first one always wins. The GUI should warn about (or leave out) A, B (or the
  menu's cancel button), the D-pad directions that move the cursor, the opener's own buttons, and
  duplicates.

    - My note: The quick-select / buttons menu shouldn't allow you to move the cursor at all. The entire point is to open a menu then press a single button to select the desired action. The only real conflict might be with the opener's own buttons; we could add a check to the gui that warns the user when tries to assign a button menu to an input that warns him that the button menu has an input that conflicts with the input.
  
- [x] 3. **B doesn't go back in held or toggled menus.** The cancel button only works in
  `Mode::Unwatched` (`src/menu.rs:378`). In a held menu you can open a submenu but you can't back
  out of it. The only way out is letting go, which closes everything. The manual says "B goes
  back one level, or closes the menu at the top" (`docs/manual/menus.md:15`). Either make cancel
  go back one level in every mode, closing only at the top in toggled mode, or fix the manual.
  The hint line never mentions B for held menus either.
- [x] 4. **Quick Settings cursor runs past the end after a profile switch.** The Profile row is
  last. If the next profile has fewer rows (no mouse stick, no gyro mouse), `self.cursor` points
  past the end, so nothing is highlighted and left, right and A do nothing until the cursor moves
  (`src/system_menu.rs:413`, `quick_step`). Clamp the cursor after `Outcome::Switch`, or keep it
  on the Profile row.
- [x] 5. **"Keep editing" lands on a page the player never opened.** Opened from the Guide
  overlay (Quick Settings or Edit Controls), leaving with unsaved changes shows the prompt, and
  both "Keep editing" and B call `back_to_main()` (`src/system_menu.rs:214`). That drops you on
  the Guide + Start "Menu" page instead of the page you were on. Remember the screen (and the
  editor's state) before `Confirm` and return to it.
- [x] 6. **Edit Controls from the Guide overlay, with nothing to edit, also lands on "Menu".**
  `edit_controls()` refuses outside a game with no focused window and calls `back_to_main()`
  (`src/daemon.rs:2071`). From the Guide overlay that opens the Guide + Start main page. It
  should go back to the Guide overlay, with the toast.
- [x] 7. **Breadcrumbs say "Menu" even when you came from the Guide overlay.**
  (`src/system_menu.rs:272`, `283`, `293`). They should say "Guide" there.
- [x] 8. **Toggling the Nintendo layout outside a game is probably lost** (*verify*). The first
  Quick Settings change outside a game moves the edited *profile* into a new pack
  (`pack_from_general`, `src/daemon.rs:2083`) and puts General back as it was. The Nintendo
  layout is set on the *game*, so General's copy is undone and the new pack never gets it.
- [x] 9. **The offer's text doesn't match what "No" does.** `OfferOutcome::No` says "ask again
  next time the game is started" (`src/offer.rs:64`). The hint says "To be asked again, tap Guide
  in a game Padwight doesn't know yet" (`src/offer.rs:161`). In fact "No" adds the packs to
  `offers.declined` for the rest of the daemon's life (`src/daemon.rs:2129`), so neither is true
  until a restart. Leaving the offer alone for 30 s counts as "No" too (`src/daemon.rs:743`). Pick
  one behavior. For example, clear `declined` for a game when its process restarts, and say that
  in the hint.
- [x] 10. **Carousels with an even number of items aren't centered.** `offset in
  -(shown/2)..=(shown-1-shown/2)` (`src/overlay/draw.rs:584`) gives -1..=0 for 2 items and
  -2..=1 for 4, so the selected item sits right of center. With 2 items, "previous" and "next"
  are the same item but only one side is shown. Pad the short side with an empty slot so the
  selected item is always in the middle.
- [x] 11. **Carousels on the touchpad don't match what's drawn.** `item_at` maps the pad left to
  right across all items (`src/menu/touch.rs:57`), but the overlay draws a ring that rotates to
  keep the selected item centered. As a finger slides, the items move under it. Either draw
  touchpad carousels as a fixed strip, or map the finger's movement relative to where it landed.
- [x] 12. **Unlatching Shift lets go of it while LT still holds it.** Picking a latched Shift on
  the keyboard again sends `Key(LEFTSHIFT, false)` unconditionally (`src/overlay.rs:350`), even
  while the trigger holds Shift (`trigger_shift`). Releasing the trigger later sends nothing. Skip
  the release while `trigger_shift` is on, as `release_latched` already does.
- [x] 13. **Key repeat catches up in bursts after a stall.** The keyboard and menus repeat with
  `while now >= next { step; next += EVERY }` (`src/overlay.rs:393`, `src/menu.rs:569`). If the
  daemon's tick is late (load, suspend, a slow D-Bus call), the cursor jumps several keys or rows
  at once. Step once and set `next = now + EVERY`, as media and the Guide menu already do.
- [x] 14. **A second controller can close or drive another controller's menu** (*verify*).
  `overlay_input` sends every device's events to the active overlay, and `MenuSession` keeps one
  `held` set. If pad 2 presses and releases the button pad 1 is holding to keep its menu up, that
  menu closes (or chooses, for a radial). Items still run on the opener's device, so if that pad
  disconnects, picks quietly do nothing. Route overlay input from the opening device only (the
  Settings menu already stores it), or keep held state per device.
- [x] 15. **The left stick may move the game while it scrolls the Guide menu** (*verify*).
  `guide_nav` takes the left stick's *direction buttons* for the menu, but nothing stops the
  stick's analog output, so a stick mapped to the gamepad's stick probably still reaches the game.
- [x] 16. **Edit Controls lists Guide, but Guide can't be remapped.** `top_rows` uses
  `Button::ALL` (`src/layout_editor.rs:37`), which includes Guide. The engine handles Guide before
  the profile sees it, so whatever is set there does nothing. On the other hand, back paddles
  aren't in `Button::ALL`, so they can't be edited in game even on pads that have them. Leave out
  Guide, and add paddles when the pad has them.
- [x] 17. **Small doc-comment slips** that make the code harder to read:
  - `Daemon.overlay_cursors` has two stacked doc lines, an old one and a new one.
  - "Makes sure the (normally resident) overlay window process is running." sits on top of
    `start_services`.
  - "Moves what General's active profile is into a new pack…" belongs to `pack_from_general` but
    sits on `edit_controls`.
  - "Difference between two angles in degrees…" sits on top of `guide_nav` in `src/engine.rs`.

## Inconsistencies

- [x] 18. **Hints and legends always use Xbox letters.** Menus draw item glyphs in the pad's own
  family (and Nintendo layout), but every hint and badge is fixed text: `button_badge` gives
  A/B/X/Y and LB/RB; the keyboard and numpad legends, the media `HINT` and the offer answers are
  literal strings. On a PlayStation pad the overlay says "A choose" next to ✕ glyphs. With the
  Nintendo layout on, "A" names the wrong button. `info::button_name(b, family)` and
  `button_glyph` already exist: build hints from segments (glyph + word) so they match the pad.
  Quick-select badges (`ItemView.button`, light-grey text chips) should be glyphs too, like
  `ItemView.buttons`.
- [x] 19. **Five repeat timings and four copies of stick-as-D-pad.** Keyboard 350/90 ms, action
  menus 350/110, media 400/120, Guide menu 400/125. The in-game menu and the offer don't repeat
  at all, so changing a stick speed from 1000 to 3000 takes about twelve separate presses. Stick
  hysteresis (0.6/0.45) is written out again in `overlay.rs`, `menu.rs`, `system_menu.rs` and the
  engine. One small `nav` module (a direction source with hysteresis plus repeat) would make them
  all behave the same.
- [x] 20. **Closing works differently everywhere:**

  | Overlay | How it closes |
  |---|---|
  | Keyboard and numpad | Hold B, Guide, or Guide + X / Guide + Y |
  | Media | Tap B, Guide, or Guide + LB |
  | Action menus | Let go of the opener, or the cancel button when unwatched |
  | Guide + Start menu | B, step by step |
  | Guide menu | B, or any button that isn't navigation |
  | Offer | B ("No") |

  Guide closes the keyboard, numpad and media (`src/daemon.rs:2218`) but not menus, the Guide +
  Start menu or the offer. Suggestion: Guide closes *any* overlay, and B means "back" everywhere,
  with the keyboard keeping hold-B so a mistap can't lose what you typed.
- [x] 21. **The offer ignores the user's styles.** It's always drawn with
  `OverlayStyle::default()`, centered (`src/offer.rs:132`), and `overlay_view` doesn't override it
  as it does for the keyboard, numpad and media. Use the menu style, or add an offer style, so
  scale, colors and fonts apply.
- [x] 22. **Tones only show in lists.** Add and remove rows are tinted (and get +/− in
  color-blind mode) only in `list()`. Grid, carousel, radial and directional ignore `tone`.
  Directional slots also never show the cursor or a pick flash: in a held directional menu,
  choosing a slot gives no visual sign that it fired.
- [x] 23. **Badges and glyphs are mixed in Edit Controls.** Button rows draw the pad's glyph,
  stick and trigger rows draw text chips ("LS", "LT"), and the radial hub writes "LS"/"RS" as
  text. `info::trigger_glyph` exists, and a stick glyph would match.
- [x] 24. **Menu-like overlays only partly share code.** The Guide menu, in-game menu, offer and
  media each have their own cursor, hints and close handling. The Guide menu's state lives in the
  daemon (`GuideMenu`), its presses come from the engine (`take_guide_nav`), and its view comes
  from `guide_menu.rs`. The offer has a cursor list drawn with its own `offer_panel` instead of
  `menu_panel`. Moving the offer's pickers and the Guide menu onto `MenuView` / list drawing would
  get the motion, sounds, tones and scrolling for free.
- [x] 25. **Hints don't show every control.**
  - The keyboard legend leaves out moving (D-pad or stick), Guide closing it, and that picking a
    latched modifier again turns it off. It says "Shift, Ctrl and Alt" though Super and AltGr
    latch too.
  - The numpad legend leaves out Guide + Y.
  - The list hint "↑↓ move" doesn't mention left and right for two columns (see 1).
  - Quick Settings says "A toggle or next", but A on a number row *raises* it.

## Improvements

- [x] 26. **A way to cancel a radial menu.** A radial menu's aim sticks after the stick springs
  back (`aim`, `src/menu.rs:497`), so letting go always chooses the last item aimed at. The
  touchpad version already treats the middle as "nothing". Options: a center dead zone held for
  about 150 ms clears the aim, or the cancel button closes held radials without choosing.
- [x] 27. **Show where the stick points on a radial menu.** Draw a needle or dot in the hub (or
  on the ring) following the live stick angle, so you can see which arc you'll land on before it
  lights up. It helps most with weighted arcs, where small arcs are hard to hit.
- [x] 28. **The keyboard needs arrow keys and editing keys.** The controller keyboard has only
  the main block: no arrows, Home/End, Delete or Page Up/Down, so you can't move the text cursor
  without retyping. Ideas: LB/RB for ← → (the D-pad is taken by the cursor), the right stick for
  arrows, or add the nav block to the layout.
  - An idea might be an option for whether the user wants a full keyboard or a basic one handling in game fields, with the basic one being the default 
  
- [x] 29. **Keyboard keycaps should follow Shift and Caps Lock.** With Shift latched or LT held,
  show "!" over "1" and lower-case letters otherwise. Light up Caps Lock while it's on (the
  overlay can't see it now; the daemon could track it, since it sends the key).
- [x] 30. **Hold X to repeat backspace** on the keyboard and numpad, at the D-pad's repeat rate.
  Deleting a word now takes one tap per letter.
- [x] 31. **The numpad needs a minus key** (negative numbers, dates), and Backspace and Enter
  keys a mouse or touchpad user could press. Now only X and Start do those.
- [x] 32. **Non-US keyboard layouts.** Keys are drawn with US labels and sent as evdev codes, so
  on AZERTY or QWERTZ the labels are wrong (Q types A). At least note this in the manual. Better:
  read the XKB layout and label keys from it.
  - Yes, we should probably add layouts and match the user's locale and local settings
- [x] 33. **Long grids don't scroll with the cursor.** Lists scroll with `list_start`, but a grid
  taller than its max height only gets the box's scrollbar, so the cursor can move off screen.
  - No change: the editor caps grids at 6×6 (`GRID_MAX`), and six rows fit at any scale up to 2.
- [x] 34. **The Guide overlay's Mappings can't be scrolled.** A long Mappings panel is cut off by
  its max height, and there's no input to scroll it while the Guide menu is up. The right stick
  or the bumpers could page through it.
- [x] 35. **Show progress for the force-quit hold.** "Keep holding to force quit…" is a plain
  toast that lingers 2.5 s after you let go. A countdown bar (like the keyboard's hold-B bar),
  removed as soon as you let go, would make it clear how long to hold and that letting go
  cancelled it.
- [x] 36. **Quick Settings number rows step by only 10%.** Repeat (see 19), plus bigger steps
  with a bumper held, would make big changes practical. Showing the saved value next to the
  current one ("1200 px/s, was 1000") would also help.
- [x] 37. **Deleting in Edit Controls is a single A press.** "Delete combo" and the macro delete
  row act at once. Discard on exit can undo it, but only by throwing away every other change too.
  Ask for a second press ("Press A again to delete").
- [x] 38. **Unwatched menus never time out.** A menu opened from the command line or an analog
  zone stays up (holding the controller) until B. Media and the offer close themselves when idle;
  menus could do the same.
- [x] 39. **Overlays can cover each other.** Info and log overlays at the same spot stack up, but
  the keyboard or a menu is drawn on top of whatever shares its position, and toasts go at the
  top regardless. Panels at the same spot could stack instead of overlapping, or at least the
  editor's preview could warn.
  - I think a warning is sufficient; overlap might be intentional, or the user might be aware that those two overlays/menus will never both be visible at the same time.

## New features

- [x] 40. **Edit menus and info overlays in game.** Edit Controls covers buttons, sticks,
  triggers, gyro, combos, layers and macros, but not menus (add, rename or reorder items) or
  info overlays. Being able to add the item you just realized you need without alt-tabbing fits
  the in-game editor's goal.
- [x] 41. **More Quick Settings rows:** rumble strength, gyro on/off (and its button), trigger
  dead zones, stick dead zones, overlay scale or opacity, and "Remapping on/off" (a panic switch
  to pass the raw pad through).
  - Done except trigger dead zones: triggers have no dead zone setting to change.
- [x] 42. **System volume in media controls.** Now ▲▼ change the *player's* volume over MPRIS,
  which some players ignore. Offer the system's volume (PipeWire/Pulse) as an option or a second
  row. Album art next to the title would make the panel easier to recognize at a glance.
  - System volume and mute are done (through `wpctl`, or `pactl`). Album art isn't: drawing
    images needs iced's `image` feature, which brings new crates into the build.
- [x] 43. **Recent items in the Guide menu.** Show the last few menu items you chose, or the
  last macro run, so a common action is one press away.
- [x] 44. **Typed-text preview and suggestions on the keyboard.** A line above the keys showing
  what's been typed this session, which helps when the game's text field is hidden behind the
  overlay. Word suggestions on the bumpers could come later.
  - The typed line is done. Word suggestions aren't: the bumpers now move the text cursor (28).
- [x] 45. **Search or jump-to in long lists.** In Edit Controls' key picker and long action menus,
  a letter jump (hold a bumper and move a letter wheel) or the on-screen keyboard as a filter.
  - Done as page jumps: LB and RB move a page through a long list menu and Edit Controls' pages.
    A letter wheel or filter isn't there.
- [x] 46. **Haptics on navigation.** The sound effects give feedback for each step and pick. A
  short rumble pulse on picks (optional, per overlay like the sounds) would help with the sound
  off.
  - Done as one setting for every overlay (Controllers page), rather than one per overlay.
- [x] 47. **Pin a menu.** A toggle (say, R3 while the menu is up) to keep a held menu open after
  letting go, for when you want several picks without holding the opener.
