// Shows padwight's overlay on GNOME, which has no layer-shell for a client window to use.
// The overlay process still draws every frame on the CPU, into a memfd; it hands the fd to
// this D-Bus object, and the frame is shown as a click-through actor in Shell's chrome, above
// fullscreen windows and without taking focus or input.

import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

export const BUS_NAME = 'io.github.marcinkoza0922.Padwight.Shell';
const OBJECT_PATH = '/Overlay';

const INTERFACE_XML = `
<node>
  <interface name="io.github.marcinkoza0922.Padwight.Overlay">
    <!-- The shared picture frames are drawn into: width x height pixels, premultiplied RGBA in
         memory order, rows width * 4 bytes apart, in the memfd. It covers the monitor Geometry chose.
         The sender paints into it only between a Present's return and its next call. -->
    <method name="SetBuffer">
      <arg type="h" direction="in" name="fd"/>
      <arg type="i" direction="in" name="width"/>
      <arg type="i" direction="in" name="height"/>
    </method>
    <!-- The picture holds a finished frame: show it. -->
    <method name="Present"/>
    <!-- Picks the monitor the overlay covers from now on, the one the focused window is on (the
         primary one when nothing has focus), and returns its logical size and its scale, which
         may be fractional. -->
    <method name="Geometry">
      <arg type="i" direction="out" name="width"/>
      <arg type="i" direction="out" name="height"/>
      <arg type="d" direction="out" name="scale"/>
    </method>
    <!-- Nothing to show: the overlay is closed. -->
    <method name="Hide"/>
  </interface>
</node>`;

export class Overlay {
    constructor() {
        this._actor = new St.Widget({reactive: false, can_focus: false, track_hover: false, visible: false});
        // Added as chrome that doesn't shrink the work area; it stays above fullscreen
        // windows because it doesn't track them, and takes no input because it isn't reactive.
        Main.layoutManager.addChrome(this._actor, {affectsStruts: false, trackFullscreen: false});
        this._content = St.ImageContent.new_with_preferred_size(1, 1);
        this._actor.set_content(this._content);
        this._actor.set_content_scaling_filters(Clutter.ScalingFilter.NEAREST, Clutter.ScalingFilter.NEAREST);

        this._dbus = Gio.DBusExportedObject.wrapJSObject(INTERFACE_XML, this);
        this._dbus.export(Gio.DBus.session, OBJECT_PATH);
        this._ownerId = Gio.bus_own_name_on_connection(Gio.DBus.session, BUS_NAME, Gio.BusNameOwnerFlags.NONE, null, null);
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
            logError(e, 'padwight overlay buffer');
        }
    }

    Present() {
        if (!this._picture)
            return;
        try {
            const [width, height] = this._size;
            this._content.set_bytes(global.stage.context.get_backend().get_cogl_context(), this._picture,
                Cogl.PixelFormat.RGBA_8888_PRE, width, height, width * 4);
            // The monitor chosen by the last Geometry call, unless it has gone since.
            const layout = Main.layoutManager;
            const monitor = layout.monitors[this._monitor ?? -1] ?? layout.primaryMonitor;
            this._actor.set_position(monitor.x, monitor.y);
            this._actor.set_size(monitor.width, monitor.height);
            this._actor.show();
        } catch (e) {
            logError(e, 'padwight overlay frame');
        }
    }

    Geometry() {
        const layout = Main.layoutManager;
        const focused = global.display.focus_window?.get_monitor() ?? -1;
        this._monitor = focused >= 0 && focused < layout.monitors.length ? focused : layout.primaryIndex;
        const monitor = layout.monitors[this._monitor];
        const scale = global.display.get_monitor_scale?.(this._monitor) ?? monitor.geometry_scale;
        return [monitor.width, monitor.height, scale];
    }

    Hide() {
        this._actor.hide();
        this._picture = null;
    }
}
