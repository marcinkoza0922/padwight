# The settings window

The sidebar lists the pages and your setups. The rest of the window shows the page you picked.

- **Overview**: your controller, labeled with what each button does in the profile in use. **What's happening** lists your latest presses, what Padwight sent to the game for each, and each profile switch with what caused it. **Controllers** lists the controllers Padwight has found, with **Calibrate gyro**, **Test rumble** and the rumble strength.
- **App settings**, **General** tab: the window's appearance (Auto, Light or Dark), automatic switching and the default profile, the overlays' font, colors, motion and sounds, the **Performance** switches, and the look of the on-screen keyboard, numpad, media controls and in-game menu. **Search settings…** finds a card by name.
- **App settings**, **Controller shortcuts** tab: what opens the Guide overlay, the Guide shortcuts and mouse mode's keys, for every setup. See [The Guide button](guide-button.md).
- **Manual**: this guide.
- **General**: the built-in setup for the desktop and plain gamepad use. It's used when no other setup matches.
- **Setups**: one for each game, each with its own profiles. **+ Add setup** adds one.
- **Top bar**: the *Remapping* switch turns all remapping on or off (it stays off while the daemon isn't running). **Active profile** switches the profile in use right away: pick the setup, then its profile. With more than one controller, the first box picks which controller to set, or **All controllers**.
- **Bottom bar**: changes wait until you press **Save & apply**. **Revert** throws them away.

## Button layout

A profile's **Profiles** tab has a **Button layout** card with the **Nintendo button layout** switch. It swaps the labels of A and B, and of X and Y, to match games that draw Nintendo prompts. It belongs to a controller, not to the profile: pick the controller in the top bar first. On a Nintendo pad the face buttons then send the letter on their label; remapping a face button's plain press turns that off. A setup can make its own choice on its **Details** tab.

## Performance

These switches turn things off in every overlay, for computers that struggle to draw them:

- **Motion**: off, overlays appear and vanish at once and redraw only when something changes. The biggest saving with a graphics card.
- **Decorations**: shadows, outlines, gradients and glows. Off is the biggest saving without a graphics card, such as in a virtual machine.
- **Sounds**: off, the audio device is never opened.
- **Looping animations**: off, glows stop pulsing and gradients stop drifting. Motion styles still play.

## Help

The **Help** card on App settings has **Take the tour**, which walks through the window again, and **System check**, which also shows as a strip on the Overview when something is missing. It lists what Padwight needs from your system (access to `/dev/uinput`, your controller and its gyro), and for anything missing the commands that fix it, with a **Copy** button. `padwight doctor` in a terminal does the same.

> **Tip:** you can edit while the daemon isn't running. Changes are saved and take effect once it starts.
