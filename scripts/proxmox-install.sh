#!/usr/bin/env bash
# Legt auf einem Proxmox-Host einen LXC-Container mit Crate Tab an – ohne Docker.
#
# Auf der Proxmox-Shell (als root):
#   bash -c "$(curl -fsSL https://raw.githubusercontent.com/dertika/cratetab/main/scripts/proxmox-install.sh)"
#
# Einstellungen per Umgebungsvariable (alle optional), z. B.
#   CTID=120 IP=192.168.1.50/24 GATEWAY=192.168.1.1 bash proxmox-install.sh
#
#   CTID              Container-ID                  (Standard: nächste freie)
#   CT_HOSTNAME       Hostname                      (cratetab)
#   STORAGE           Speicher für die Disk         (local-lvm)
#   TEMPLATE_STORAGE  Speicher für Templates        (local)
#   DISK              Diskgröße in GB               (4)
#   MEMORY            RAM in MB                     (512)
#   CORES             CPU-Kerne                     (1)
#   BRIDGE            Netzwerkbrücke                (vmbr0)
#   IP                "dhcp" oder z. B. 192.168.1.50/24   (dhcp)
#   GATEWAY           Gateway bei fester IP
#   VLAN              VLAN-Tag (optional)
#   VERSION           Release, z. B. v0.2.0         (neueste)
#   CLUB_NAME         Vereinsname                   (Mein Verein e. V.)
#   REPO              GitHub-Repository             (dertika/cratetab)
set -euo pipefail

REPO="${REPO:-dertika/cratetab}"
VERSION="${VERSION:-latest}"
CT_HOSTNAME="${CT_HOSTNAME:-cratetab}"
STORAGE="${STORAGE:-local-lvm}"
TEMPLATE_STORAGE="${TEMPLATE_STORAGE:-local}"
DISK="${DISK:-4}"
MEMORY="${MEMORY:-512}"
CORES="${CORES:-1}"
BRIDGE="${BRIDGE:-vmbr0}"
IP="${IP:-dhcp}"
CLUB_NAME="${CLUB_NAME:-}"
ASSET="cratetab-lxc-debian-13-amd64.tar.zst"

say() { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31mFehler:\033[0m %s\n' "$*" >&2; exit 1; }

command -v pct >/dev/null || die "Dieses Skript muss auf einem Proxmox-Host laufen."
[ "$(id -u)" = 0 ] || die "Bitte als root ausführen."
[ "$(uname -m)" = x86_64 ] || die "Das Template gibt es nur für amd64."

CTID="${CTID:-$(pvesh get /cluster/nextid)}"
pct status "$CTID" >/dev/null 2>&1 && die "Container $CTID existiert bereits."

if [ "$VERSION" = latest ]; then
    BASE="https://github.com/$REPO/releases/latest/download"
else
    BASE="https://github.com/$REPO/releases/download/$VERSION"
fi

# Template in den Template-Speicher laden (ein vorhandenes wird ersetzt)
TEMPLATE="$(pvesm path "$TEMPLATE_STORAGE:vztmpl/$ASSET")"
say "Lade Template nach $TEMPLATE"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fL --progress-bar -o "$tmp/$ASSET" "$BASE/$ASSET"
curl -fsSL -o "$tmp/$ASSET.sha256" "$BASE/$ASSET.sha256"
(cd "$tmp" && sha256sum -c "$ASSET.sha256") || die "Prüfsumme stimmt nicht."
mkdir -p "$(dirname "$TEMPLATE")"
mv "$tmp/$ASSET" "$TEMPLATE"

net="name=eth0,bridge=$BRIDGE,ip=$IP"
[ -n "${GATEWAY:-}" ] && net="$net,gw=$GATEWAY"
[ -n "${VLAN:-}" ] && net="$net,tag=$VLAN"

say "Lege Container $CTID ($CT_HOSTNAME) an"
pct create "$CTID" "$TEMPLATE_STORAGE:vztmpl/$ASSET" \
    --hostname "$CT_HOSTNAME" \
    --ostype debian \
    --unprivileged 1 \
    --features nesting=1 \
    --cores "$CORES" \
    --memory "$MEMORY" \
    --swap 256 \
    --rootfs "$STORAGE:$DISK" \
    --net0 "$net" \
    --onboot 1 \
    --description "Crate Tab – Die Getränkekasse für euren Verein 🦀
https://github.com/$REPO"

say "Starte Container"
pct start "$CTID"

# Warten, bis Crate Tab antwortet
for _ in $(seq 1 30); do
    if pct exec "$CTID" -- cratetab healthcheck >/dev/null 2>&1; then break; fi
    sleep 2
done
pct exec "$CTID" -- cratetab healthcheck >/dev/null || die "Crate Tab startet nicht. Logs: pct exec $CTID -- journalctl -u cratetab"

if [ -n "$CLUB_NAME" ]; then
    escaped=$(printf '%s' "$CLUB_NAME" | sed 's/[\/&"]/\\&/g')
    pct exec "$CTID" -- sed -i "s/^CRATETAB_CLUB_NAME=.*/CRATETAB_CLUB_NAME=\"$escaped\"/" /etc/cratetab/cratetab.env
    pct exec "$CTID" -- systemctl restart cratetab
fi

# Admin-Passwort neu setzen und anzeigen
password="$(tr -dc 'A-Za-z0-9' </dev/urandom | head -c 16)"
pct exec "$CTID" -- cratetab set-admin-password admin "$password" >/dev/null

addr="$(pct exec "$CTID" -- hostname -I | awk '{print $1}')"
cat <<EOF

  Crate Tab läuft in Container $CTID.

  Kiosk:        http://$addr:8080/
  Admin:        http://$addr:8080/admin
  Benutzer:     admin
  Passwort:     $password      (bitte notieren und nach dem ersten Login ändern)

  Konfiguration: pct exec $CTID -- nano /etc/cratetab/cratetab.env
  Update:        pct exec $CTID -- cratetab-update
  Konsole:       pct enter $CTID

  Für Kamera-Scanner und Handys braucht es HTTPS über einen Reverse Proxy (siehe README).

EOF
