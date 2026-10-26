# Cycles

A cycle steps through a list of inputs, one per press, and starts over after the last. It puts a game's states that rule each other out on one button: game speeds, standing, crouching and lying down, or fire modes.

## Making one

1. Open the setup's **Cycles** tab, and click **+ New cycle**. A new cycle has two empty steps.
2. Give each step what it presses: a key or key combination, a mouse button, the scroll wheel, a gamepad button, trigger or stick, a [macro](macros.md), or **Several at once…** to press inputs together.
3. **+ Add step** adds another. The arrows beside a step move it, and **×** removes it.

## Using one

Map it with the **Cycle…** action, on a button, gesture, combo, trigger, zone or menu item.

- The first press presses step 1, the next step 2, and so on. After the last step, the next press presses step 1 again.
- A step stays pressed while you hold the button, and is let go when you do.
- **Step back** presses the step before instead. Put it on a second button to go both ways, such as RB for faster and LB for slower; both share the same place in the cycle.
- Every cycle goes back to its start when the game starts.

The first press presses step 1, so list the state the game starts in last. For a strategy game that starts at normal speed, with keys 1, 2 and 3 for normal, fast and fastest, the steps are 2, 3, 1: the first press goes fast, the next fastest, and the third back to normal.

## When to use one

A cycle suits a game with a key for each state (1, 2 and 3 for each speed; C to crouch and Z to go prone). If the game already has one key that steps through its states, map that key instead.

The cycle only knows which step it pressed last. If the game changes state by itself (a menu resets the speed), the next press may not do what you expect; press on until it does, or let the game start over.

For one state you hold rather than step through, such as lying down, a long press on the crouch button is often nicer: see the gestures in [Buttons](buttons.md).

> **Tip:** renaming a cycle updates every button that uses it.
