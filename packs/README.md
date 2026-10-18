# Built-in game library

Each `*.padpack` file here is a game shipped with the app: the build embeds them, and
"+ Add game" offers them. Make one in the app (a game's **Details → Export…**); in a debug
build, tick **Library pack** to keep a library game's ID so users get it as an update.

`cargo test` checks every pack here: it must parse at the current format, have a unique ID
and name, no references to missing macros, menus or info overlays, rules that point at
existing profiles (at least one rule), and at least one profile that needs nothing beyond a plain pad. Profiles state what they need themselves (`requires = ["gyro"]`, never detected). Shooters ship a profile without gyro (the one the rules point to), one with gyro as an extra, and a flick stick one that requires gyro. Strategy games ship a plain profile and a gyro one (`<Game> + Gyro`) that moves the cursor as well.

Library packs carry no `[assets]`. The icons, sounds and fonts they use are built into the app (`assets/`), so every game can use them; a new one goes there, and a test checks that every `{icon:…}` a library pack names is built in.
