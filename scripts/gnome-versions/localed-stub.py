"""Stands in for systemd-localed on the container's private system bus: GNOME Shell 45 and 47
(libgnome-desktop) crash on start when its reply is missing."""
import sys
from gi.repository import Gio, GLib

XML = """<node><interface name="org.freedesktop.locale1">
<property name="Locale" type="as" access="read"/>
<property name="X11Layout" type="s" access="read"/><property name="X11Model" type="s" access="read"/>
<property name="X11Variant" type="s" access="read"/><property name="X11Options" type="s" access="read"/>
<property name="VConsoleKeymap" type="s" access="read"/><property name="VConsoleKeymapToggle" type="s" access="read"/>
</interface></node>"""
VALUES = {"Locale": GLib.Variant("as", ["LANG=en_US.UTF-8"]), "X11Layout": GLib.Variant("s", "us")}

conn = Gio.DBusConnection.new_for_address_sync(
    sys.argv[1], Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION, None, None)
info = Gio.DBusNodeInfo.new_for_xml(XML).interfaces[0]
conn.register_object("/org/freedesktop/locale1", info, None, lambda c, s, p, i, name: VALUES.get(name, GLib.Variant("s", "")), None)
Gio.bus_own_name_on_connection(conn, "org.freedesktop.locale1", Gio.BusNameOwnerFlags.NONE, None, None)
GLib.MainLoop().run()
