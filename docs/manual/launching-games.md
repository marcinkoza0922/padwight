# Launching games

The **Games** page lists what Steam, Heroic and Lutris have installed, including the games you added to them by hand. **Play** starts a game through its store and Padwight follows it from then on, so its setup applies as soon as its window is in front. **Stop** ends it.

Padwight starts every game through its store, so the store sets up the game's runner and prefix. To add a game of your own, add it to Steam (Add a Non-Steam Game), Heroic (Add game) or Lutris, then press **Import games**.

## Start through Padwight

Playing from the Games page is enough when you start games from Padwight. If you start them from the store itself, or want a game to get settings of its own, add `padwight run` to the game's launch options. It starts the game as its parent, so Padwight sees every process the game starts, even ones that detach, and knows the moment it ends.

Select the game on the Games page and copy the line under **Start through Padwight**:

| Store | Where it goes | The line |
|---|---|---|
| Steam | the game's Properties, Launch Options | `padwight run -- %command%` |
| Heroic | the game's wrapper command setting | `padwight run --title <id> --` |
| Lutris | the game's command prefix setting | `padwight run --title <id> --` |

Padwight never edits the store's settings for you. The Heroic and Lutris lines name the game by its id, which is how Padwight tells which game it started; copy the line from the Games page rather than typing it. Steam sets variables the wrapper reads, so it needs no id.

`padwight run` never keeps a game from starting. If Padwight isn't running, or doesn't know the game, the command runs as if it weren't there and a line on stderr says so.

For a program that isn't in any store, `padwight run --setup "My setup" -- ./game` plays it with that setup.

## Settings that need the wrapper

Under **Start through Padwight**:

- **Hide real controllers from the game** sets `SDL_GAMECONTROLLER_IGNORE_DEVICES` for this game, so SDL sees only Padwight's virtual controller and not the real one underneath. It leaves out a real controller whose USB ids Padwight's virtual one shares.
- **Variables** are set on this game only, written as `NAME=value` with spaces between, for example `PROTON_LOG=1`.

Both are saved with the game and apply the next time it starts through `padwight run`.

> **Tip:** the same "game sees my controller twice" cure without the wrapper is in [Troubleshooting](troubleshooting.md).
