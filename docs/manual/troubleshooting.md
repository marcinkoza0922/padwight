# Troubleshooting

## The game sees my controller twice

If Steam Input manages the controller, the game sees Steam's virtual controller as well as Padwight's. Turn off Steam Input for that controller.

PlayStation and Switch controllers can also be read directly, so for those you may need to turn off Steam Input, or set `SDL_JOYSTICK_HIDAPI=0` for the game.

## A Steam game doesn't react to the controller, or ignores its stick

A game that Steam starts and lists with full controller support gets the controller through Steam Input: Padwight lets go of it while the game is in front (see [Controller setups](setups.md)). Leave Steam Input on for such games.

If you use a setup for one anyway (**Use this setup even when Steam lists the game with full controller support**), turn off Steam Input for the game: in Steam, open the game's **Properties › Controller** and pick **Disable Steam Input**. If the buttons then work but the stick doesn't, the game is reading the stick through Steam, which sends nothing while Steam Input is off. Turn on **Hide Padwight's controller from Steam** in **App settings › Steam**, then quit and restart Steam. Games started from Steam still see Padwight's controller; Steam's own menus and Big Picture don't.

The setting adds a copy of Steam's menu entry to `~/.local/share/applications` (or a Flatpak override for Flatpak Steam), so it only applies when Steam is started from the menu. If Steam starts some other way, such as at login or from a terminal, start it with `SDL_GAMECONTROLLER_IGNORE_DEVICES=0x045e/0x028e,0x054c/0x09cc,0x054c/0x0ce6,0x057e/0x2009` set.

## My controller doesn't show up

Run `padwight status` in a terminal to see what the daemon has found. Your user needs access to the controller and to `/dev/uinput`. The README explains the udev rule, or joining the `input` group, that gives it.

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

Those controllers' motion sensors need a permission rule of their own, which the README explains. Install it, then press **Calibrate gyro**.

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

**My controller has no Guide button.** Open App settings, pick the Controller shortcuts tab, and set "Open the Guide overlay" to a combo such as Start + Select, a long press, or another button. Press Listen and then the button to choose it from the controller. Setups whose profiles already use that input show a ⚠ in the sidebar.

**Guide opens the Xbox Game Bar.** Turn off the Game Bar's controller button in Windows' Settings, under Gaming, Xbox Game Bar.

**My PlayStation or Switch controller doesn't show up.** On Windows, Padwight reads controllers through XInput, Windows' Xbox controller interface. Other controllers show up when something presents them as an Xbox controller, such as Steam Input or DS4Windows, without their gyro and touchpad.
