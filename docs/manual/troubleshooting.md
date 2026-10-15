# Troubleshooting

## The game sees my controller twice

PlayStation and Switch controllers can also be read directly, by the game's SDL and by Steam, so the game may get their input as well as Padwight's. Set `SDL_JOYSTICK_HIDAPI=0` for the game, or turn off Steam Input for that controller.

## A game ignores the controller with Steam Input on

Some games read only the first controller Steam Input lists, such as Slay the Spire 2. Steam lists controllers in the order it found them, and the controller Padwight takes over has nothing to say to the game, so Padwight keeps its own virtual controller ready while remapping is on, ahead of any controller you connect. If the game still ignores the controller, Padwight was probably started after the controller was connected: unplug the controller (or turn it off) and connect it again.

## My controller doesn't show up

Run `padwight doctor` in a terminal. It checks that your user can open the controller, its motion sensor and `/dev/uinput`, and for anything missing it says why padwight needs it and prints the commands that fix it. `padwight status` shows what the running daemon has found.

## Why did my profile change?

The Overview page's **What's happening** section lists the last few profile switches, newest first. Each one says what made it, such as the rule that matched the window in front, the default profile taking over when a game lost focus, or a profile you picked yourself. Under the controller picture, the same section shows your latest presses and what each did, next to the buttons Padwight sent for them. If the two differ, the mapping is doing something other than what the label says.

## The profile didn't switch when I opened a game

- Check the setup's rules on its **Details** tab, and that the rule is switched on.
- Check that no other rule also matches the game.
- Check that your desktop supports window tracking: KDE Plasma, GNOME, Hyprland, Sway, or a wlroots compositor such as labwc (which matches by window class only).

## The keyboard, menus or overlays don't appear

They need a desktop that supports them: KDE Plasma, Sway, Hyprland or labwc. GNOME doesn't have the layer-shell they use. In a Flatpak, they don't show on Sway, labwc or Hyprland.

## The on-screen keyboard shows the wrong letters

The keyboard is labeled for the first layout your desktop is set to use. If it picks the wrong one, tell it yours, such as `de` or `fr(azerty)`. Run `systemctl --user edit padwight`, add these lines, save, and run `systemctl --user restart padwight`:

```
[Service]
Environment=PADWIGHT_KEYBOARD_LAYOUT=de
```

## Gyro does nothing on a PlayStation or Switch controller

Those controllers' motion sensors need a permission rule of their own. `padwight doctor` tells you whether it's installed and prints the commands if not. Install it, reconnect the controller, then press **Calibrate gyro**.

## The top bar says the daemon isn't running

Start it with `systemctl --user start padwight`, or on Windows with `padwight daemon`. You can still edit in the meantime. Your changes are kept, and they take effect once it starts.

## I edited the config file by hand, and nothing changed

Run `padwight reload`. The daemon reads the file only when it's told to.

## Rumble doesn't reach my controller

Only controllers with force feedback vibrate. Press **Test rumble** on the Overview page to check.

## The profile changed after I unplugged my controller

Unplugging releases everything the controller was holding, and a message says which controller left. When the same controller comes back, the profile you were using returns, unless you've changed it since.

If a replacement controller lacks something the profile needs, such as gyro, a profile that doesn't need it takes over, and a message says so.

## On Windows

**The game sees my controller twice.** Windows can't keep the controller to Padwight alone, so games see it next to Padwight's virtual one. HidHide hides it: add the controller to its hidden devices, and Padwight to the programs still allowed to see it.

**Games don't see Padwight's controller.** The virtual controller needs the ViGEmBus driver. Keyboard and mouse mappings work without it.

**Keys and the mouse don't reach a game.** Windows doesn't let a program send input to one that runs as administrator. Run Padwight as administrator too.

**My controller has no Guide button.** Open App settings, pick the Controller shortcuts tab, and set "Open the Guide overlay" to a combo such as Start + Select, a long press, or another button. Press Set by pressing and then the button to choose it from the controller. Setups whose profiles already use that input show a ⚠ in the sidebar.

**Guide opens the Xbox Game Bar.** Turn off the Game Bar's controller button in Windows' Settings, under Gaming, Xbox Game Bar.

**My PlayStation or Switch controller doesn't show up.** On Windows, Padwight reads controllers through XInput, Windows' Xbox controller interface. Other controllers show up when something presents them as an Xbox controller, such as Steam Input or DS4Windows, without their gyro and touchpad.
