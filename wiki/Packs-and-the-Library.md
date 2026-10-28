# Packs and the library

A setup can be shared as a **pack**: a single `.padpack` file (TOML) holding the setup's profiles, modes, layers, macros, menus and overlays.

For a step-by-step walkthrough that builds a pack from scratch, see [Tutorial: making a pack](Tutorial-Deus-Ex-Pack).

The format itself is documented in the repository: [pack format reference](https://github.com/marcinkoza0922/padwight/blob/main/docs/pack-format.md). A step-by-step example is in the [Deus Ex pack tutorial](https://github.com/marcinkoza0922/padwight/blob/main/docs/manual/tutorial-deus-ex.md).

## Exporting

**Details → Export…** on a setup writes a `.padpack` file. The export dialog warns about references to missing items.

Each profile declares what it needs beyond a plain controller: gyro, back paddles or a touchpad. Tick **Profile requires gyro** on the profile's Gyro tab when the profile depends on it. The back paddles section of the Buttons tab and the Touchpad tab have the same box. Nothing is detected; the export dialog only lists what you've declared.

Re-exporting your own setup keeps its pack ID. Exporting someone else's makes a fork with its own ID, which credits the original.

## Importing

**+ Add setup → Import a file…** opens a preview before anything changes. The preview shows:

- what's inside the pack,
- what your controller lacks,
- whether a setup with the same name exists: add under a new name, or replace yours,
- other setups' rules for the same window. The imported setup takes over unless you keep yours.

Nothing changes until **Save & apply**.

Imported setups are ordinary, editable setups. They remember their pack, so:

- importing a newer version of the same pack, or
- an app update that ships a newer library version (marked **update** in the sidebar)

offers to update the setup, and lists what you've changed since.

A pack made by a newer version of padwight is refused, with a request to update.

Players whose controller lacks a feature are only offered the profiles that don't need it.

## The built-in library

**+ Add setup** also lists the built-in library. Games you have installed in Steam, Heroic or Lutris, or that are running, come first.

The library is the `packs/*.padpack` files in the repository, embedded at build time. See [`packs/README.md`](https://github.com/marcinkoza0922/padwight/blob/main/packs/README.md). Adding a file to `packs/` is enough for it to appear in the picker, as long as it passes the checks that `cargo test` runs.
