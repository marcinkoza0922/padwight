# Tutorial: step through game speeds with one button

A cycle presses the next of its steps each time, and starts over after the last. You'll make a *Game speed* cycle for a strategy game with keys 1, 2 and 3 for normal, fast and fastest speed, and put it on a bumper, with the other bumper going back.

## 1. Make the cycle

1. Open the setup's **Cycles** tab and press **+ New cycle**. The new cycle opens at the top with two empty steps.
2. Type its name in the **Name** box: *Game speed*.
3. Press **+ Add step** for a third step.
4. Give each step its key, by clicking its keys field and pressing the key, or picking it on the on-screen keyboard:
   - step 1: **2** (fast),
   - step 2: **3** (fastest),
   - step 3: **1** (normal).

The game starts at normal speed, so normal goes last: the first press moves on from it. The card's header lists the steps in order when it's closed, here *2 → 3 → 1*.

A step can press more than a key: choose its action to press a gamepad button, play a [macro](macros.md), or press several inputs at once.

## 2. Put it on a button

1. On a profile's **Buttons** tab, open **RB**.
2. Click its action, choose **Cycle…** (under **Open or switch**) and pick *Game speed*.
3. Open **LB**, choose **Cycle…** again, pick *Game speed* and tick **Step back**.

RB now goes fast, fastest, normal, fast…; LB goes the other way from wherever RB left it.

## 3. Save and try it

Press **Save & apply**, start the game and press RB. Each press presses the next step's key, held for as long as you hold the button. Every cycle goes back to its start when the game starts.

If the game changes speed by itself, say after loading a save, the cycle doesn't know: press on until it matches again.

## Where next

- [Cycles](cycles.md) for the reference.
- For stances, a cycle of stand, crouch and prone works the same way, but a long press for prone is often handier: see [the buttons tutorial](tutorial-buttons.md).
