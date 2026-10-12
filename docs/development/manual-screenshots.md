# Manual screenshots

The tutorials in [docs/manual/](../manual/) show the settings window and the overlays, with numbered
marks on the parts a step talks about. The pictures are made by tools, not by hand, so they can be
made again when the window changes. Nothing needs a display, a daemon or a controller.

```sh
scripts/manual-shots/build.sh
```

That runs the two steps below. Raw screenshots go to `screenshots/manual/`, which git ignores; the
marked-up ones go to `docs/images/manual/`, which is committed.

## 1. Raw screenshots: `padwight debug gui`

`scripts/manual-shots/shoot.sh` draws the real settings window with the offscreen renderer (the one
`debug overlay` uses), in the light theme, with a made-up controller connected. It never reads or writes
your own config. `padwight debug gui` is a list of steps, run in order, with a picture saved at each `shot`:

```sh
padwight debug gui library "Deus Ex" game "Deus Ex" tab profiles sub sticks shot sticks
```

The steps pick a library setup to add, a page, a game, a tab, a profile or one of its sub-tabs, open a
button's row or a card, open a dialog, hide the live controller picture, and set the picture size.
`demo` adds the macro and the info overlay that the macro and overlay tutorials build, so those tabs
aren't empty. `padwight debug help` lists them all. The overlay pictures come from `padwight debug overlay`.

## 2. Marked-up pictures: `scripts/annotate.py`

Each `scripts/manual-shots/specs/*.json` is a list of pictures: a raw screenshot, a crop, and marks in the
screenshot's own pixels (boxes with numbered badges, arrows, notes, a spotlight, a blur). The numbers are
what the tutorial text refers to, so keep the two in step. To find coordinates, run
`scripts/annotate.py grid screenshots/manual/<name>.png [x y w h]`, which writes the screenshot with a
labeled 50-pixel grid into `screenshots/manual/grid/`.

## Adding a picture

1. Add a `shot` to `shoot.sh` and an entry to a spec.
2. Run `scripts/manual-shots/build.sh`, and look at the result.
3. List the file's name in `IMAGES` in `src/gui/manual.rs`, and put it in a chapter as
   `![alt](../images/manual/<name>.png "caption")`. A test checks that the folder, the list and the
   chapters agree, and the app draws the pictures from the list (the window draws SVG, so each PNG is
   wrapped in one when the Manual is drawn).

When the window changes, run `build.sh` again and check the marks still sit on the right controls.

## The wiki copies

`scripts/sync-wiki-tutorials.py` writes the tutorials into `wiki/` as `Tutorial-*.md` pages, with links and
pictures rewritten for the wiki, and rebuilds the sidebar's tutorial list. Run it after editing a tutorial.
