# Troubleshooting

## A controller shows up twice (or the game sees two pads)

PlayStation and Switch controllers are also readable through their raw HID node (`/dev/hidraw*`), which SDL, Steam and Chromium browsers use directly. padwight's exclusive hold covers only the input node, so they may see the real controller as well as padwight's, and Steam may act on its buttons. When padwight finds a program doing this, it says so in a desktop notification and on screen, and the settings window names it in a banner across the top of every page.

Install `72-padwight-hidraw.rules` (`padwight doctor` prints the commands), then turn the controller off and on. It closes the raw node to every program, so they read the input node, which padwight holds. While padwight is off, Steam then has no gyro or lightbar control for those controllers. Without the rule, turn off Steam Input for the pad, or set `SDL_JOYSTICK_HIDAPI=0` for the game.

padwight skips virtual uinput devices, such as Steam Input's pad and its own.

## A game ignores the controller with Steam Input on

Some games read only the first controller Steam Input lists, such as Slay the Spire 2. Steam lists controllers in the order it found them, and the controller padwight takes over has nothing to say to the game, so padwight keeps its own virtual controller ready while remapping is on, ahead of any controller you connect. If the game still ignores the controller, padwight was probably started after the controller was connected: unplug the controller (or turn it off) and connect it again.

## The controller isn't picked up

Check that your user can open `/dev/uinput` and the gamepad device nodes. If not, add a udev rule or join the `input` group. See [Installation](Installation#permissions).

Run `padwight status` to see which controllers the daemon has found.

## X and Y are swapped on an xpad-driver pad

Pads on the `xpad` driver report X and Y by label rather than by position. padwight corrects this automatically.

## Motion or gyro doesn't work on a PlayStation or Switch pad

Install the motion-sensor udev rule. See [Installation](Installation#gyro).

## The profile didn't change when I focused a game

- Check the setup's rules on its **Details** tab. Rules match on the executable, Steam App ID or window class, and for Wine and Proton games the match is on the `.exe`.
- Check that the desktop supports focus tracking. See [Desktop support](Desktop-Support). On GNOME, the extension must be enabled. A log out and back in may be needed after the first install.
- Check that the rule is switched on, and that no earlier rule matches the same game.

## The on-screen keyboard, menus or overlays don't appear

These need a compositor with layer-shell support: KDE Plasma, Sway, Hyprland and similar. GNOME doesn't have it.

## The profile changed after a controller was unplugged or swapped

A disconnect releases everything that controller was holding and closes any menu it had open. A toast names the controller that left or arrived.

When the controller comes back, the profile you were on returns, unless you've changed it since. If the replacement controller lacks a feature the profile needs, such as gyro, the setup's first profile that doesn't need it takes over, or the default profile if none does. A toast says so.

The change is made once the controllers have looked the same for two 2-second scans. This stops a motion sensor that appears a moment after its pad from making the profile flicker.

## I edited the config file by hand and nothing changed

Run `padwight reload`. The daemon is the only writer of the file while it runs, and it re-reads the file only on reload.

## Rumble doesn't reach the controller

Rumble from games is forwarded to the physical controller only if it supports force feedback. Use **Test rumble** on the Overview page to check it.
