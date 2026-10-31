# GNOME Shell of one Fedora release, run headless to check the overlay extension against it.
# Usage: scripts/gnome-versions/run.sh <fedora release>...
ARG FEDORA=44
FROM fedora:${FEDORA}
ARG FEDORA
# Fedora's EOL releases move to the archive, which the image's metalinks no longer find.
RUN (dnf -y --setopt=install_weak_deps=False install gnome-shell mutter mesa-dri-drivers dbus-daemon dbus-tools python3-gobject gdk-pixbuf2 glib2 procps-ng \
    || (sed -i -e 's|^metalink=|#metalink=|' -e "s|^#baseurl=http://download.example/pub/fedora/linux|baseurl=https://dl.fedoraproject.org/pub/archive/fedora/linux|" /etc/yum.repos.d/*.repo \
        && dnf -y --setopt=install_weak_deps=False install gnome-shell mutter mesa-dri-drivers dbus-daemon dbus-tools python3-gobject gdk-pixbuf2 glib2 procps-ng)) \
    && dnf clean all
RUN useradd -m dev
COPY --chown=dev probe.py localed-stub.py run-in-container.sh /home/dev/
USER dev
WORKDIR /home/dev
CMD ["bash", "/home/dev/run-in-container.sh"]
