# Troubleshooting

## The game sees my controller twice

PlayStation and Switch controllers can also be read directly, by the game's SDL and by Steam, so the game may get their input as well as Padwight's. Set `SDL_JOYSTICK_HIDAPI=0` for the game, or turn off Steam Input for that controller.

## A game ignores the controller with Steam Input on

Some games, such as Slay the Spire 2, read only the first controller Steam Input lists. Padwight keeps its virtual controller ready while remapping is on, so it comes first. If the game still ignores it, Padwight probably started after the controller was connected: disconnect the controller and connect it again.

## My controller doesn't show up

Run `padwight doctor` in a terminal. It checks that you can open the controller, its motion sensor and `/dev/uinput`, and prints the commands that fix anything missing. `padwight status` shows what the running daemon has found.

## The calculator's Copy does nothing

Copy puts the answer on the clipboard through `wl-copy`, from your distro's `wl-clipboard` package. Install it; nothing needs restarting. **Send**, which types the answer into the game, and the keyboard's **Paste** work without it. `padwight doctor` says whether it's installed.

Padwight needs a Wayland session. On an X11 session (Xorg or XLibre) the overlays, including the calculator, don't work at all; `padwight doctor` says so.

## Why did my profile change?

The Overview page's **What's happening** lists the last profile switches, newest first, each with its cause: the rule that matched, the default profile taking over when a game lost focus, or your own pick. It also lists your latest presses next to what Padwight sent for them.

## The profile didn't switch when I opened a game

- Check the setup's rules on its **Details** tab, and that the rule is switched on.
- Check that no other rule also matches the game.
- Check that your desktop supports window tracking: KDE Plasma, GNOME, Cinnamon, Hyprland, Sway, niri, COSMIC, or a wlroots compositor such as labwc, Wayfire or Budgie 10.10 and later. COSMIC and wlroots compositors match by window class only, so a rule on the executable name won't match there.

## The keyboard, menus or overlays don't appear

On KDE Plasma, Sway, Hyprland, niri, COSMIC and wlroots compositors such as labwc, they are ordinary layer-shell windows. On GNOME and Cinnamon, padwight's desktop extension shows them instead. It's installed and enabled the first time the daemon runs there, and you need to log out and back in once before it works. `padwight status` says which method is in use.

## The on-screen keyboard shows the wrong letters

The keyboard is labeled for your desktop's first layout. To pick another, such as `de` or `fr(azerty)`, run `systemctl --user edit padwight`, add these lines, save, and run `systemctl --user restart padwight`:

```
[Service]
Environment=PADWIGHT_KEYBOARD_LAYOUT=de
```

Only the first layout counts. If it isn't Latin (Russian, Greek), the on-screen keyboard is labeled in that script, and text you send through it (the keyboard's typing and the calculator) can only use characters that layout types: Latin letters are dropped, while digits, `-` and `.` still work in Russian and Greek. Space, Tab and Enter work in every layout. Keys you map on a controller and macros are key positions, not characters, so they are not affected. To type Latin letters, set `PADWIGHT_KEYBOARD_LAYOUT=us` as above; the game receives the same key positions, so it types Latin letters when its own layout is Latin.

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
