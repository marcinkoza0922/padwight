# Tutorial: share a setup as a pack, and use someone else's

A setup can be shared as a pack, a `.padpack` file that holds its profiles, macros, menus, layers, overlays, rules and description. You'll export a setup, then import one.

## 1. Export

1. Open the setup's **Details** tab. In the **Pack** card, press **Export…**.
2. The **Export** window opens over the page.

![The Export window: what's in the pack, version, author, made with, a description and Save](../images/manual/sharing-export.png "The Export window.")

1. The summary (1) lists what the pack holds. Padwight also warns here about:
   - profiles that need a controller feature, such as gyro or a touchpad (a profile that "can't be played without gyro" isn't offered to players whose controller has none),
   - dangling references, such as a button that points at a macro or menu that doesn't exist.

   Fix any warnings before you save.
2. **Version**, **Author** and **Made with** (2). Versions compare number by number, so `1.10` is newer than `1.9`. A higher version gives installed copies the *update* mark.
3. The **Description** (3) is what people read before adding the pack. Say what the layout is and what the pad should be (an Xbox-style controller, say).
4. Press **Save…** (4) and choose where to put the `.padpack`.

Then press **Save & apply** to remember the export details in the setup.

## 2. Import

1. Click **+ Add setup** and **Import a file…**, then pick the pack.
2. A **preview** shows what's in the pack, which controller features it needs, what your own controller lacks (a profile that needs gyro isn't offered on a plain pad), and any clashes with your setups. If you already have a setup of that name, the preview asks you to rename or replace.
3. Nothing changes until you confirm and press **Save & apply**.

To test an export, import it back and use **Rename**, then delete the test copy.

## 3. The library

**+ Add setup** also lists the setups that come with Padwight. Games you have installed in Steam, Heroic or Lutris, and games that are running, come first. When a game the library has a setup for comes to the front, a note at the top of the screen says so: tap Guide to see its profiles and add it. *Not now* asks again the next time the game starts; *Never* stops asking for that game.

When a newer version of a library setup or a pack you added is available, the sidebar marks it with *update*. Before you update, Padwight lists the changes you've made, because an update replaces them.

> **Tip:** a pack made with a newer Padwight can't be imported. Update Padwight first.

## Where next

- [Sharing setups: packs and the library](sharing.md) for the reference.
- [Making a pack, step by step](tutorial-deus-ex.md) builds a whole pack, and the [pack format](https://github.com/marcinkoza0922/padwight/blob/main/docs/pack-format.md) documents the file.
