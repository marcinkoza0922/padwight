# Tutorial: your first setup for a game

You'll give one game its own controls: add a setup, tell Padwight which window is the game, and change a button. It takes about five minutes.

**You'll need:** the daemon running (the Overview lists your controller), and the game installed.

## 1. Add the setup

1. Click **+ Add setup** at the bottom of the sidebar.
2. Pick where to start from:

![The Add a setup window: the template dropdown, Import a file, the library list and Preview & add](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/first-setup-add.png "Add a setup. Games you have installed are listed first.")

   1. **Empty setup from a template…** starts from a ready-made profile, for a game that isn't in the library.
   2. **Import a file…** adds a `.padpack` someone sent you. See [the sharing tutorial](Tutorial-Sharing).
   3. The **library** lists the setups that ship with Padwight. Click one to read about it.
   4. **Preview & add…** shows what the setup holds before anything changes.

If the game is in the library, add it and press **Save & apply**: the setup comes with its rules and profiles, and you're done. The rest of this tutorial starts from an empty setup.

An empty setup opens on its **Details** tab, named *New setup*. Type the game's name in the **Name** box.

## 2. Tell Padwight which window is the game

Padwight switches to a setup when its window has focus. **Auto-switch rules**, on the **Details** tab, say which window that is.

![The Auto-switch rules card with one rule and an Add rule button](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/first-setup-rules.png "A rule reads: When [what to look at] is [value] use [profile].")

1. **+ Add rule** (3) adds a row:
   1. The first box (1) says what to match: the program's name (**Executable**), its **Steam App ID**, or its window class. Wine and Proton games match by their `.exe` name, like `DeusEx.exe`.
   2. The last box (2) picks the profile to switch to.
2. Don't know the name? Start the game, focus it, and look under **Recently focused windows** below the rules. Its **+ Rule** buttons make the row for you.

> **Tip:** automatic switching needs a desktop that reports which window has focus. See [Troubleshooting](Troubleshooting).

## 3. Check for native controller support

Many games support a DualShock, DualSense or Switch Pro Controller themselves. If this one does, tell Padwight, so the game sees your controller as that kind, with its own button names and glyphs.

1. Check the game's controls menu, store page or wiki. Count only explicit support: a store badge alone isn't enough, and games from before 2006 rarely get it right.
2. On the **Details** tab, under **Controllers this setup supports**, tick each kind the game supports.
3. While this setup is active, a controller of a ticked kind is seen as itself. Any other is seen as an Xbox controller.

## 4. Change a button

1. Open the setup's **Profiles** tab.
2. Click a button's name, say **A (South)**, then its action, and pick what it does: a key, a mouse button, a macro and so on. The [Buttons tutorial](Tutorial-Buttons) goes through the options.
3. Not sure which row is which button? Press **Find by pressing**, then press the button on your controller.

## 5. Save and try it

1. Press **Save & apply** at the bottom. The change works straight away.
2. Switch to the game. **Active profile** in the top bar changes to the setup's profile, and the Overview's **What's happening** shows the switch.
3. If it doesn't switch, check the rule against **Recently focused windows**, then see [Troubleshooting](Troubleshooting).

You can always pick a profile by hand under **Active profile**.

## Where next

- [Buttons](Tutorial-Buttons), [sticks](Tutorial-Sticks) and [gyro](Tutorial-Gyro) for the controls themselves.
- [Controller setups and profiles](Games-and-Profiles) for how setups, profiles and rules fit together.
