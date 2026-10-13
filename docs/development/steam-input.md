# Steam Input: putting Padwight in front of Steam

Status: investigation (2026-10-13). Nothing here is implemented yet. Today's code still hands the
pads to Steam Input for games Steam lists with full controller support (`src/daemon/steam_handoff.rs`)
and has a setting that edits Steam's menu entry (`src/steam_hide.rs`). Both would go.

## Goal

Steam games should work with Padwight while Steam Input is on, remapping included, and without
the user changing anything in Steam. **Padwight must not modify Steam in any way**: no
environment variables, no copies of its menu or autostart entries, no Flatpak overrides, no
settings, no files under `~/.local/share/Steam`.

The chain we want:

```
real pad ──▶ Padwight ──▶ Padwight's virtual pad ──▶ Steam Input ──▶ game
```

Steam must see Padwight's virtual pad, and must not see the real pad.

## Why Steam games break today

Steam's controller log (`~/.local/share/Steam/logs/controller.txt`) shows which devices it opened
and which slot each got (`Local Device Found`, `opened for index N`, `reserving XInput slot N`).

- Padwight grabs the real pad (`EVIOCGRAB`). A grab stops events but not enumeration, so Steam
  still lists the real pad. It sends nothing.
- Steam gives slots in arrival order. The real pad always arrives first, because Padwight only
  builds its virtual pad after it has grabbed a real one. So the dead real pad is controller 0 /
  XInput slot 0, and Padwight's pad is controller 1.
- A game that reads through Steam's input API (Slay the Spire 2) uses controller 0 and gets
  nothing. A replug doesn't help: the real pad wins the race again.
- Steam gives each game it starts its own `SDL_GAMECONTROLLER_IGNORE_DEVICES` covering nearly
  every known pad ID (Padwight's `045e:028e` and the user's GameSir `3537:1079` included), plus
  `SDL_GAMECONTROLLER_ALLOW_STEAM_VIRTUAL_GAMEPAD=1`. So under Steam Input a game reads only
  Steam's virtual pads (`28de:11ff`) or Steam's API, never the devices themselves. Steam's
  virtual pad disappeared while Slay the Spire 2 (an API game) was in front.
- The old fix, hiding *Padwight's* pad from Steam, is backwards. It keeps the dead real pad in
  slot 0 and stops Steam API games from ever seeing Padwight's output.

## Experiments (2026-10-13)

Hardware: GameSir G7 HE, wired (`3537:1079`, three GIP interfaces `ff/47/d0`, interface 0 has
endpoints `0x82` in and `0x02` out, driven by `xpad` in Xbox One mode). Game: Slay the Spire 2
(app 2868840) with Steam Input on. Padwight kept the pads in it (the setup has `over_steam`).

| Setup | Steam's slot 0 | Result |
|---|---|---|
| Stock Steam, Padwight grabs the pad | grabbed real pad | no input at all |
| Same, real pad unplugged and replugged | grabbed real pad (it arrives first again) | no input at all |
| Steam started with `SDL_GAMECONTROLLER_IGNORE_DEVICES=0x3537/0x1079` (invasive, rejected) | Padwight's pad | works, and a B→A remap reaches the game |
| Stock Steam, `xpad` detached through usbfs, pad fed from userspace via uinput | the userspace pad | works |

The last row is the approach to build on: a user process takes the pad's USB interface from the
kernel driver. The real pad's evdev node goes away for everyone, Steam included, while Steam
runs. Steam logs `Controller device closed after hid_read failure` and gives slot 0 to the
remaining pad. Steam isn't touched.

### Taking a USB pad from its driver

`scripts/usb-takeover.py` does it with raw usbfs ioctls, the calls libusb uses:

1. `USBDEVFS_DISCONNECT_CLAIM` on `/dev/bus/usb/BBB/DDD` for the interface. This unbinds the
   kernel driver and claims the interface in one call. No root is needed, only read-write access
   to the node.
2. xpad's init request: control transfer `bmRequestType 0xC1`, `bRequest 0x01`, `wValue 0x100`,
   20 bytes. Third-party pads stay silent without it. Then the LED command `01 03 02` on the OUT
   endpoint.
3. Read the IN endpoint (`USBDEVFS_BULK` also works for interrupt endpoints) and decode GIP:
   `0x20` packets are input reports (buttons in bytes 4–5, triggers as u16 at 6 and 8, sticks as
   i16 at 10, 12, 14 and 16 with Y up). `0x07` packets carry the Guide button. `0x03` is a heartbeat.
4. On exit: `USBDEVFS_RELEASEINTERFACE`, then `USBDEVFS_IOCTL` with `USBDEVFS_CONNECT`. The
   kernel driver binds again.

The first attempt read nothing until the init request was added, and then dropped every packet
because it expected Xbox 360 reports: under `xpad` this pad speaks GIP (Xbox One).

### What happens when the process dies

| How it ends | Driver afterwards | Pad |
|---|---|---|
| SIGTERM / Ctrl+C / timeout (the script reattaches) | `xpad` (or `xone`) again within a second | back for Steam and Padwight |
| SIGKILL, nothing else | **none**: the kernel does not rebind | dead to everything |
| SIGKILL, then `scripts/usb-reattach.py` from another process | `xpad` again, same device number | back |
| SIGKILL, nothing else, ~35 s later | the GameSir dropped off the bus and re-enumerated by itself | back |

The last row is this pad's firmware giving up when nobody polls it. Don't rely on it: other pads
may stay without a driver until replugged. It also means Padwight must keep reading a pad it holds,
or the pad will reset.

Compare today's evdev grab: the kernel ends it whenever the process dies, so nothing can stay stuck.

### Other drivers

Tested in the `sway` test VM (Arch, kernel 7.2.9) with the GameSir passed through over QMP
(`device_add usb-host`), running unprivileged with a udev rule giving the user its USB node:

- **xone** (out-of-tree GIP driver; `xone-dkms` from AUR, installed without the dongle firmware
  it depends on): claims the GameSir by vendor and GIP class. It names the pad "Microsoft Xbox
  Controller" on its own `gip0` bus. Detach, reading reports (1260 in one run), reattach after
  SIGTERM, and reattach after SIGKILL all work. `xone` resets the USB device each time it binds,
  so a reattach takes about a second.
- **xpad**: claims the GameSir too (by vendor, both the 360 and GIP classes). Tested on the host
  as above. When both drivers are installed, whichever loads first wins.
- **xpadneo**: Bluetooth only. All its aliases are HID bus `0005`, so it never binds a USB pad and
  the usbfs route doesn't apply. Untested on hardware: it needs a Bluetooth Xbox pad.
- **hid-nintendo** over Bluetooth: see [Bluetooth](#bluetooth-an-8bitdo-ultimate-in-switch-mode)
  below.

The usbfs route doesn't care which driver is bound: `DISCONNECT_CLAIM` unbinds whatever has the
interface, and `CONNECT` lets the kernel pick a driver again. What changes per pad is the protocol
Padwight then has to speak.

### Bluetooth: an 8BitDo Ultimate in Switch mode

The pad shows up as a Pro Controller (`057e:2009`, bus `0005`). BlueZ creates it through uhid and
`hid-nintendo` drives it. **Steam reads it through `/dev/hidraw7`, not evdev** (seen in Steam's
open files), so Padwight's grab doesn't stop it. Steam gets the pad's live, unremapped input in
its own slot, next to Padwight's virtual pad.

`scripts/bt-takeover.py` tried to take the pad from BlueZ the way the USB route takes it from the
kernel, as a normal user: get BlueZ to let go, open the pad's HID channels (L2CAP PSM 17 control,
19 interrupt) on a plain socket with `BT_SECURITY_MEDIUM` (the kernel already has the link key),
and decode the `0xA1 0x3F` reports into a uinput pad.

| Run | How BlueZ was made to let go | Result |
|---|---|---|
| 1 | `bluetoothctl disconnect`, connect 1 s later | Got in on the 4th try. Steam dropped `hidraw7` and opened the test pad; 115 reports arrived. Pressing Home made the pad connect to BlueZ again, alongside the script's channels. |
| 2 | Same | "Host is down" for the whole 10 s retry window: the pad had powered off. |
| 3 | `Device1.DisconnectProfile` for the HID UUID only | The link dropped anyway. When the pad came back it refused our channels ("Connection refused", then timeouts), and the link bounced up and down until the run ended. BlueZ then reconnected it normally. |

Conclusions:

- A Switch-style pad treats a disconnect from the host as "power off". Run 1 won a race in the
  second or so before it shut down.
- The pad reconnects by calling the host itself, and those calls go to BlueZ, which owns the HID
  channels. A normal user can't take them over. Taking over a Bluetooth pad isn't reliable, so
  Bluetooth pads stay outside the design below.
- Not tried: other Bluetooth pads (Xbox over xpadneo, DualSense) may behave differently.
  Running the pad off its 2.4 GHz USB dongle makes it a USB device, so the USB route would apply.

## Proposed design

1. **Take USB pads over usbfs.** When Padwight manages a USB pad and can open its USB node, it
   detaches the driver and reads the pad itself. One reader per protocol: GIP (Xbox One/Series
   and their clones), Xbox 360, and later the HID pads (DualShock 4, DualSense, Switch Pro).
   SDL's HIDAPI drivers are the reference for those. When the node can't be opened, it falls back
   to today's grab.
2. **Hand pads back on purpose.** Disabling remapping, setting a pad aside (`ignored_devices`) or
   quitting reattaches the driver, and Steam picks the pad up live.
3. **Survive hard kills.**
   - A watchdog child forked at start holds one end of a pipe. When the daemon dies by any means,
     the pipe closes and the child reattaches every interface the daemon detached.
   - On start, the daemon reattaches gamepad interfaces it finds without a driver.
     `padwight.service` restarts after 2 s (`Restart=on-failure`), so a crash costs about 2 s.
4. **USB access.** The user's own `/etc/udev/rules.d/70-vm-gamepad.rules` (written for QEMU
   passthrough) is what let the tests open the USB node. A stock system needs a rule of
   Padwight's own giving `uaccess` to gamepad USB devices. That is a one-time install step like
   `dist/70-padwight-motion.rules`, not a change to Steam.
5. **Retire the hand-off.** `left_to_steam`, `steam_has_pads`, `Game::over_steam`, the library's
   `native_controller` filtering for Steam launches, `steam_hide.rs` and its settings card all go.
   The manual's "Games Steam looks after" section and the Steam part of Troubleshooting get rewritten.

## Gaps and open questions

- **Bluetooth pads.** No usbfs, and taking the HID channels from BlueZ failed (see above). Steam
  reads them through hidraw, so a Steam game sees the raw pad as well as Padwight's virtual pad.
  The likely fallback is the grab plus a warning that Steam games may ignore Padwight's remaps.
  It's untested in a game which of the two pads the game follows, or whether it takes input from both.
- **HID pads over USB** (DualShock 4, DualSense, Switch Pro). The detach should work the same way,
  but each needs its own protocol code. Gyro and touchpad would then reach Steam only if Padwight's
  virtual pad carries them. Steam reads Sony pads through hidraw, which a uinput pad doesn't have.
  A uhid virtual pad would. Untested.
- **Games that read Steam's virtual pad instead of the API.** They should work the same way (Steam
  wraps whatever is in slot 0), but none has been tested.
- **The Xbox Wireless Adapter** (`xone`'s dongle) needs firmware loaded by the driver. Taking it
  over is a different job from a wired pad.
- **Suspend and resume.** The kernel may try to rebind driverless interfaces on resume. Untested.
- **Claude Code's auto mode** classes the driver detach as "security weaken" and blocks some
  commands around it. Expect to approve these steps by hand when testing.

## Reproducing

```sh
# Find the pad's USB node and interface (here 1-8.4, interface 0, IN endpoint 0x82)
d=/sys/bus/usb/devices/1-8.4; n=$(printf /dev/bus/usb/%03d/%03d $(cat $d/busnum) $(cat $d/devnum))
scripts/usb-takeover.py $n 0 0x82 120   # takes the pad for up to 2 minutes
scripts/usb-reattach.py $n 0            # gives it back after a SIGKILL
grep -a -E 'Local Device Found|type:|opened for index|XInput slot|closed' ~/.local/share/Steam/logs/controller.txt | tail
```

For a Bluetooth pad: `scripts/bt-takeover.py MAC SECONDS [full|profile]`. If the pad ends up
disconnected, press Home to reconnect it to BlueZ.

For other drivers, pass the pad to a test VM (`vms/vm.sh`) through its QMP socket:
`{"execute":"device_add","arguments":{"driver":"usb-host","vendorid":13623,"productid":4217,"id":"pad"}}`,
and `device_del` to give it back. The pad is gone from the host meanwhile.
