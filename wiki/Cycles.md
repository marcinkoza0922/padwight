# Cycles

A cycle is a named list of inputs that one button steps through, one per press, starting over after the last. A setup's **Cycles** tab holds them, each in its own collapsible card. **+ New cycle** adds one at the top, with two empty steps.

Use one for a game's states that rule each other out: game speeds, standing / crouching / prone, fire modes.

## Steps

Each step presses one of:

- a key or key combination,
- a mouse button or the scroll wheel,
- a gamepad button, trigger or stick,
- a macro,
- several of these at once (**Several at once…**).

**+ Add step** adds one; the arrows move a step and **×** removes it. A closed card lists its steps in order (`2 → 3 → 1`).

## Using a cycle

Map a cycle with the **Cycle…** action, on a button, gesture, combo, trigger, zone or menu item.

- The first press presses step 1, the next step 2, and so on; after the last, step 1 again.
- A step stays pressed while the button is held.
- **Step back** presses the step before instead. Two buttons, one with **Step back**, share one place in the cycle.
- Every cycle goes back to its start when the game starts.

List the state the game starts in last, so the first press moves on from it.

A cycle only knows which step it pressed last. It works best with a key for each state; if the game has one key that steps through its states already, map that key instead.

Cycles can't be toggled or turboed. Renaming a cycle updates every mapping that uses it.

In a pack, cycles are `[[cycles]]` with `name` and `steps`, mapped with `cycle = { name = "…" }` (and `back = true` to step back). See `docs/pack-format.md`.
