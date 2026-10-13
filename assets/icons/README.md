# Built-in icons

The icons `{icon:name}` can draw without a pack carrying its own (see `src/icons.rs`). `build.rs` embeds every
`*.svg` in this folder, named by its file name, so adding an icon is adding a file. Each is a 24-unit SVG
drawn with `currentColor`, so it takes the text's color.

These from [Lucide](https://lucide.dev) (ISC): `axe`, `bow`, `car`, `coin`, `gem`, `heart`, `key`, `map`,
`pda`, `potion`, `shield`, `skull`, `sword`. `wheel` is from [Tabler Icons](https://tabler.io/icons) (MIT).
Both licences need their notice kept with copies of the icons, so the notices are in `LICENSES.txt` beside
them.

Every other icon here was drawn for this app, first for the library packs (`packs/`), and is under the
app's own license. Draw new ones in the same way: a `0 0 24 24` viewBox, round caps and joins, a 1.8 stroke
in `currentColor`, and a general name (`dagger`, not `prince-dagger`) so any game can use it.
