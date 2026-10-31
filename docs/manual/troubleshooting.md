# Troubleshooting

## The game sees my controller twice

PlayStation and Switch controllers can also be read directly, by the game's SDL and by Steam, so the game may get their input as well as Padwight's. Set `SDL_JOYSTICK_HIDAPI=0` for the game, or turn off Steam Input for that controller.

## A game ignores the controller with Steam Input on

Some games, such as Slay the Spire 2, read only the first controller Steam Input lists. Padwight keeps its virtual controller ready while remapping is on, so it comes first. If the game still ignores it, Padwight probably started after the controller was connected: disconnect the controller and connect it again.

## My controller doesn't show up

Run `padwight doctor` in a terminal. It checks that you can open the controller, its motion sensor and `/dev/uinput`, and prints the commands that fix anything missing. `padwight status` shows what the running daemon has found.

## Why did my profile change?

The Overview page's **What's happening** lists the last profile switches, newest first, each with its cause: the rule that matched, the default profile taking over when a game lost focus, or your own pick. It also lists your latest presses next to what Padwight sent for them.

## The profile didn't switch when I opened a game

- Check the setup's rules on its **Details** tab, and that the rule is switched on.
- Check that no other rule also matches the game.
- Check that your desktop supports window tracking: KDE Plasma, GNOME, Hyprland, Sway, or a wlroots compositor such as labwc (which matches by window class only).

## The keyboard, menus or overlays don't appear

They need KDE Plasma, Sway, Hyprland or labwc; GNOME lacks the layer-shell they use.

## The on-screen keyboard shows the wrong letters

The keyboard is labeled for your desktop's first layout. To pick another, such as `de` or `fr(azerty)`, run `systemctl --user edit padwight`, add these lines, save, and run `systemctl --user restart padwight`:

```
[Service]
Environment=PADWIGHT_KEYBOARD_LAYOUT=de
```

## Gyro does nothing on a PlayStation or Switch controller

Those controllers' motion sensors need a permission rule of their own. `padwight doctor` tells you whether it's installed and prints the commands if not. Install it, reconnect the controller, then press **Calibrate gyro**.

## The top bar says the daemon isn't running

Start it with `systemctl --user start padwight`, or `padwight daemon`. You can still edit in the meantime. Your changes are kept, and they take effect once it starts.

## I edited the config file by hand, and nothing changed

Run `padwight reload`. The daemon reads the file only when it's told to.

## Rumble doesn't reach my controller

Only controllers with force feedback vibrate. Press **Test rumble** on the Overview page to check.

## The profile changed after I unplugged my controller

When the same controller comes back, the profile you were using returns, unless you've changed it since. If a different controller lacks something the profile needs, such as gyro, a profile that doesn't need it takes over, and a message says so.
