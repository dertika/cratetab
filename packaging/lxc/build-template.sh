#!/usr/bin/env bash
# Baut ein Proxmox-LXC-Template mit Crate Tab.
#
#   sudo packaging/lxc/build-template.sh <basis-template.tar.zst> <cratetab-binärdatei> <ausgabe.tar.zst>
#
# Basis ist das offizielle Debian-Template von Proxmox (debian-13-standard).
# Darauf kommen:
#   /usr/local/lib/cratetab/cratetab     die Binärdatei
#   /usr/local/bin/cratetab              Wrapper für Hilfsbefehle (lädt die Konfiguration)
#   /usr/local/sbin/cratetab-backup      tägliches Backup (systemd-Timer)
#   /usr/local/sbin/cratetab-update      Update auf eine neue Version
#   /etc/cratetab/cratetab.env           Konfiguration
#   /var/lib/cratetab/                   Datenbank und Backups (Benutzer „cratetab“)
#
# Muss als root auf einem amd64-Linux laufen (wegen chroot und Dateirechten).
set -euo pipefail

if [ $# -ne 3 ]; then
    echo "Aufruf: $0 <basis-template.tar.zst> <cratetab-binärdatei> <ausgabe.tar.zst>" >&2
    exit 2
fi
BASE="$(realpath "$1")"
BINARY="$(realpath "$2")"
OUTPUT="$(realpath -m "$3")"
HERE="$(cd "$(dirname "$0")" && pwd)"

[ "$(id -u)" = 0 ] || { echo "Bitte als root ausführen" >&2; exit 1; }

WORK="$(mktemp -d)"
ROOT="$WORK/rootfs"
cleanup() {
    for m in dev/pts dev proc sys; do umount "$ROOT/$m" 2>/dev/null || true; done
    rm -rf "$WORK"
}
trap cleanup EXIT

echo "==> Basis-Template entpacken"
mkdir -p "$ROOT"
tar --zstd -xpf "$BASE" --numeric-owner --xattrs --xattrs-include='*' -C "$ROOT"

if [ "${SKIP_APT:-0}" = 1 ]; then
    echo "==> SKIP_APT=1: Paket-Updates übersprungen (nur für lokale Tests)"
else
    echo "==> Pakete aktualisieren (Sicherheitsupdates, curl für cratetab-update)"
    if [ -e "$ROOT/etc/resolv.conf" ] || [ -L "$ROOT/etc/resolv.conf" ]; then
        mv "$ROOT/etc/resolv.conf" "$WORK/resolv.conf.orig"
    fi
    cp -L /etc/resolv.conf "$ROOT/etc/resolv.conf"
    mount --bind /dev "$ROOT/dev"
    mount --bind /dev/pts "$ROOT/dev/pts"
    mount -t proc proc "$ROOT/proc"
    mount -t sysfs sys "$ROOT/sys"
    chroot "$ROOT" /usr/bin/env DEBIAN_FRONTEND=noninteractive sh -c '
        set -e
        apt-get update -q
        apt-get -y -q -o Dpkg::Options::=--force-confold upgrade
        apt-get -y -q install --no-install-recommends curl ca-certificates
        apt-get clean
        rm -rf /var/lib/apt/lists/*
    '
    for m in dev/pts dev proc sys; do umount "$ROOT/$m"; done
    rm -f "$ROOT/etc/resolv.conf"
    if [ -e "$WORK/resolv.conf.orig" ] || [ -L "$WORK/resolv.conf.orig" ]; then
        mv "$WORK/resolv.conf.orig" "$ROOT/etc/resolv.conf"
    fi
fi

echo "==> Benutzer und Verzeichnisse anlegen"
chroot "$ROOT" sh -c '
    getent group cratetab >/dev/null || groupadd --system cratetab
    getent passwd cratetab >/dev/null || useradd --system --gid cratetab \
        --home-dir /var/lib/cratetab --shell /usr/sbin/nologin cratetab
'
install -d -m 755 "$ROOT/usr/local/lib/cratetab" "$ROOT/etc/cratetab"
install -d -m 750 "$ROOT/var/lib/cratetab" "$ROOT/var/lib/cratetab/backups"
chroot "$ROOT" chown -R cratetab:cratetab /var/lib/cratetab

echo "==> Crate Tab installieren"
install -m 755 "$BINARY" "$ROOT/usr/local/lib/cratetab/cratetab"
install -m 755 "$HERE/cratetab" "$ROOT/usr/local/bin/cratetab"
install -m 755 "$HERE/cratetab-backup" "$ROOT/usr/local/sbin/cratetab-backup"
install -m 755 "$HERE/cratetab-update" "$ROOT/usr/local/sbin/cratetab-update"
install -m 640 "$HERE/cratetab.env" "$ROOT/etc/cratetab/cratetab.env"
chroot "$ROOT" chown root:cratetab /etc/cratetab/cratetab.env
install -m 644 "$HERE/cratetab.service" "$HERE/cratetab-backup.service" \
    "$HERE/cratetab-backup.timer" "$ROOT/etc/systemd/system/"
# Dienste aktivieren (dasselbe wie „systemctl enable“, braucht aber kein laufendes systemd)
install -d "$ROOT/etc/systemd/system/multi-user.target.wants" "$ROOT/etc/systemd/system/timers.target.wants"
ln -sf /etc/systemd/system/cratetab.service "$ROOT/etc/systemd/system/multi-user.target.wants/cratetab.service"
ln -sf /etc/systemd/system/cratetab-backup.timer "$ROOT/etc/systemd/system/timers.target.wants/cratetab-backup.timer"

# Hinweis bei der Anmeldung im Container
cat > "$ROOT/etc/motd" <<'EOF'

  Crate Tab – Die Getränkekasse für euren Verein 🦀

  Konfiguration:  nano /etc/cratetab/cratetab.env && systemctl restart cratetab
  Status/Logs:    systemctl status cratetab   |   journalctl -u cratetab -f
  Backup:         cratetab-backup   (täglich automatisch, /var/lib/cratetab/backups)
  Update:         cratetab-update
  Hilfe:          cratetab help

EOF

# Versionsdatei, damit man im Container sieht, was installiert ist
"$BINARY" --version > "$ROOT/etc/cratetab/template-version" 2>/dev/null \
    || echo "unbekannt" > "$ROOT/etc/cratetab/template-version"

echo "==> Aufräumen"
rm -f "$ROOT/root/.bash_history"
rm -rf "$ROOT/tmp/"* "$ROOT/var/tmp/"*
# Proxmox erzeugt beim Anlegen des Containers eine neue machine-id
: > "$ROOT/etc/machine-id"

echo "==> Template packen: $OUTPUT"
mkdir -p "$(dirname "$OUTPUT")"
tar --zstd -cpf "$OUTPUT" --numeric-owner --xattrs --xattrs-include='*' -C "$ROOT" .
ls -lh "$OUTPUT"
