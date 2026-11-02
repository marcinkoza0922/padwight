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
// Larger than any screen: keeps width * height * 4 well inside what a number holds exactly.
const MAX_SIDE = 32768;
const OVERLAY_BUS = 'io.github.marcinkoza0922.Padwight.Shell';
const OVERLAY_PATH = '/Overlay';

// Any program on the bus can call SetBuffer, and one that shrinks its memfd after it is mapped
// would kill the Shell with SIGBUS on the next read. The overlay process seals its memfd against
// shrinking (F_SEAL_SHRINK, which can't be undone). GJS has no fcntl to read the seals, so this
// tries to shrink the file by a byte: it fails only on a sealed one. On an unsealed memfd it
// succeeds, which refuses the buffer (and cuts only the sender's own file).
function requireShrinkSeal(path) {
    const file = Gio.File.new_for_path(path).open_readwrite(null);
    try {
        const size = file.query_info('standard::size', null).get_size();
        let shrank = false;
        try {
            shrank = file.truncate(size - 1, null);
        } catch (e) {
            // Refused (EPERM): the file is sealed against shrinking.
        }
        if (shrank)
            throw new Error('the picture\'s memfd is not sealed against shrinking');
    } finally {
        file.close(null);
    }
}

// Any program of the user's on the bus can call this object, and the overlay process is the only
// one that should. The bus can say which process a sender is, so a call is served only when that
// process runs the same executable as the daemon (the one that owns the Focus name, and starts
// the overlay process from its own binary). A copy of the file, or any other program, is refused.
const DBUS_NAME = 'org.freedesktop.DBus';
function exeOf(name) {
    try {
        const reply = Gio.DBus.session.call_sync(DBUS_NAME, '/org/freedesktop/DBus', DBUS_NAME,
            'GetConnectionCredentials', new GLib.Variant('(s)', [name]), null, Gio.DBusCallFlags.NONE, -1, null);
        return GLib.file_read_link(`/proc/${reply.recursiveUnpack()[0].ProcessID}/exe`);
    } catch (e) {
        return null;
    }
}

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

        this._trusted = new Set();
        this._dbus = Gio.DBusExportedObject.wrapJSObject(OVERLAY_XML, this);
        this._dbus.export(Gio.DBus.session, OVERLAY_PATH);
        this._ownerId = Gio.bus_own_name_on_connection(Gio.DBus.session, OVERLAY_BUS, Gio.BusNameOwnerFlags.NONE, null, null);
    }

    // True when the caller is padwight's own overlay process; otherwise answers the call with an error.
    _allowed(invocation) {
        const sender = invocation.get_sender();
        if (!this._trusted.has(sender)) {
            const daemon = exeOf(DAEMON_BUS);
            if (daemon === null || daemon !== exeOf(sender)) {
                invocation.return_dbus_error('org.freedesktop.DBus.Error.AccessDenied', 'only padwight draws on its overlay');
                return false;
            }
            // A unique name is never handed out twice, so the answer holds for as long as it lives.
            this._trusted.add(sender);
        }
        return true;
    }

    destroy() {
        Gio.bus_unown_name(this._ownerId);
        this._dbus.unexport();
        Main.layoutManager.removeChrome(this._actor);
        this._actor.destroy();
        this._actor = null;
    }

    // `parameters` is [index into the message's Gio.UnixFDList, width, height]. Served as
    // an async handler to see who the sender is.
    SetBufferAsync([fd, width, height], invocation) {
        if (!this._allowed(invocation))
            return;
        try {
            const real = invocation.get_message().get_unix_fd_list().get(fd);
            // Reopening the memfd through /proc maps it, without copying it into JS. The mapping
            // is kept: every frame is drawn into it.
            let picture;
            try {
                requireShrinkSeal(`/proc/self/fd/${real}`);
                picture = new GLib.MappedFile(`/proc/self/fd/${real}`, false).get_bytes();
            } finally {
                GLib.close(real);
            }
            // Any program on the bus can call this: refuse sizes the buffer can't back, since
            // the picture is read as width x height x 4 bytes.
            if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0 ||
                width > MAX_SIDE || height > MAX_SIDE || picture.get_size() < width * height * 4)
                throw new Error(`picture of ${picture.get_size()} bytes is too small for ${width}x${height}`);
            this._picture = picture;
            this._size = [width, height];
            invocation.return_value(null);
        } catch (e) {
            global.logError(`padwight overlay buffer: ${e}`);
            invocation.return_dbus_error('org.freedesktop.DBus.Error.InvalidArgs', String(e.message ?? e));
        }
    }

    PresentAsync(_parameters, invocation) {
        if (!this._allowed(invocation))
            return;
        if (!this._picture) {
            invocation.return_value(null);
            return;
        }
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
        // Answered only now: the daemon paints on the picture again once this returns.
        invocation.return_value(null);
    }

    GeometryAsync(_parameters, invocation) {
        if (!this._allowed(invocation))
            return;
        const layout = Main.layoutManager;
        const focused = global.display.focus_window?.get_monitor() ?? -1;
        this._monitor = focused >= 0 && focused < layout.monitors.length ? focused : layout.primaryIndex;
        const monitor = layout.monitors[this._monitor];
        const scale = global.display.get_monitor_scale?.(this._monitor) ?? 1;
        invocation.return_value(new GLib.Variant('(iid)', [monitor.width, monitor.height, scale]));
    }

    HideAsync(_parameters, invocation) {
        if (!this._allowed(invocation))
            return;
        invocation.return_value(null);
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
