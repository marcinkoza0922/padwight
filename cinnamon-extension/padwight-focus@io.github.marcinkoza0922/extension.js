// Cinnamon's counterpart of the GNOME Shell extension (gnome-extension/…): it reports every
// focus change to padwight's daemon, which matches the window against the game rules, and shows
// the overlay's frames. Muffin has no layer-shell, so the overlay process draws each frame on the
// CPU into a memfd, hands the fd to the D-Bus object below, and the frame is shown as a
// click-through actor in Cinnamon's chrome, above fullscreen windows, without focus or input.

const Clutter = imports.gi.Clutter;
const Cogl = imports.gi.Cogl;
const Gio = imports.gi.Gio;
const GLib = imports.gi.GLib;
const St = imports.gi.St;
const Main = imports.ui.main;

const FOCUS_BUS = 'io.github.marcinkoza0922.Padwight';
const FOCUS_PATH = '/Focus';
const FOCUS_INTERFACE = 'io.github.marcinkoza0922.Padwight.Focus';
const OVERLAY_BUS = 'io.github.marcinkoza0922.Padwight.Shell';
const OVERLAY_PATH = '/Overlay';

const OVERLAY_XML = `
<node>
  <interface name="io.github.marcinkoza0922.Padwight.Overlay">
    <method name="SetBuffer">
      <arg type="h" direction="in" name="fd"/>
      <arg type="i" direction="in" name="width"/>
      <arg type="i" direction="in" name="height"/>
    </method>
    <method name="Present"/>
    <method name="Geometry">
      <arg type="i" direction="out" name="width"/>
      <arg type="i" direction="out" name="height"/>
      <arg type="d" direction="out" name="scale"/>
    </method>
    <method name="Hide"/>
  </interface>
</node>`;

class Overlay {
    constructor() {
        this._actor = new St.Widget({reactive: false, can_focus: false, track_hover: false, visible: false});
        Main.layoutManager.addChrome(this._actor, {visibleInFullscreen: true, affectsStruts: false, affectsInputRegion: false});
        this._image = new Clutter.Image();
        this._actor.set_content(this._image);
        this._actor.set_content_scaling_filters(Clutter.ScalingFilter.NEAREST, Clutter.ScalingFilter.NEAREST);

        this._dbus = Gio.DBusExportedObject.wrapJSObject(OVERLAY_XML, this);
        this._dbus.export(Gio.DBus.session, OVERLAY_PATH);
        this._ownerId = Gio.bus_own_name_on_connection(Gio.DBus.session, OVERLAY_BUS, Gio.BusNameOwnerFlags.NONE, null, null);
    }

    destroy() {
        Gio.bus_unown_name(this._ownerId);
        this._dbus.unexport();
        Main.layoutManager.removeChrome(this._actor);
        this._actor.destroy();
        this._actor = null;
    }

    // `fdList` is the message's Gio.UnixFDList; `fd` is an index into it.
    SetBuffer(fd, width, height, fdList) {
        try {
            const real = fdList.get(fd);
            // Reopening the memfd through /proc maps it, without copying it into JS. The mapping
            // is kept: every frame is drawn into it.
            this._picture = new GLib.MappedFile(`/proc/self/fd/${real}`, false).get_bytes();
            GLib.close(real);
            this._size = [width, height];
        } catch (e) {
            global.logError(`padwight overlay buffer: ${e}`);
        }
    }

    Present() {
        if (!this._picture)
            return;
        try {
            const [width, height] = this._size;
            this._image.set_bytes(this._picture, Cogl.PixelFormat.RGBA_8888_PRE, width, height, width * 4);
            // The monitor chosen by the last Geometry call, unless it has gone since.
            const layout = Main.layoutManager;
            const monitor = layout.monitors[this._monitor ?? -1] ?? layout.primaryMonitor;
            this._actor.set_position(monitor.x, monitor.y);
            this._actor.set_size(monitor.width, monitor.height);
            this._actor.show();
        } catch (e) {
            global.logError(`padwight overlay frame: ${e}`);
        }
    }

    Geometry() {
        const layout = Main.layoutManager;
        const focused = global.display.focus_window?.get_monitor() ?? -1;
        this._monitor = focused >= 0 && focused < layout.monitors.length ? focused : layout.primaryIndex;
        const monitor = layout.monitors[this._monitor];
        const scale = global.display.get_monitor_scale?.(this._monitor) ?? 1;
        return [monitor.width, monitor.height, scale];
    }

    Hide() {
        this._actor.hide();
        this._picture = null;
    }
}

class PadwightFocus {
    enable() {
        this._overlay = new Overlay();
        this._display = global.display;
        this._focusId = this._display.connect('notify::focus-window', () => this._report());
        // A daemon that starts (or restarts) after this hears nothing until focus moves, so
        // report to it as soon as it is on the bus. This also covers the first report.
        this._watchId = Gio.bus_watch_name(Gio.BusType.SESSION, FOCUS_BUS, Gio.BusNameWatcherFlags.NONE,
            () => this._report(), null);
    }

    disable() {
        this._overlay.destroy();
        this._overlay = null;
        Gio.bus_unwatch_name(this._watchId);
        this._display.disconnect(this._focusId);
        this._display = null;
    }

    _report() {
        const win = global.display.focus_window;
        if (!win) {
            // Nothing has focus, such as the desktop.
            Gio.DBus.session.call(FOCUS_BUS, FOCUS_PATH, FOCUS_INTERFACE, 'FocusCleared', new GLib.Variant('()', []),
                null, Gio.DBusCallFlags.NONE, -1, null, null);
            return;
        }
        // Everything is sent as a string, as the daemon expects (see focus/dbus.rs).
        const args = GLib.Variant.new_tuple([
            GLib.Variant.new_string(win.get_wm_class() ?? ''),
            GLib.Variant.new_string(String(win.get_pid())),
            GLib.Variant.new_string(win.get_title() ?? ''),
        ]);
        // No reply is needed. If the daemon isn't running, the call just fails.
        Gio.DBus.session.call(FOCUS_BUS, FOCUS_PATH, FOCUS_INTERFACE, 'WindowActivated', args,
            null, Gio.DBusCallFlags.NONE, -1, null, null);
    }
}

let extension = null;

function init(_meta) {
    extension = new PadwightFocus();
}

function enable() {
    extension.enable();
}

function disable() {
    extension.disable();
}
