#!/usr/bin/env python3
"""Lädt das neueste offizielle Proxmox-Template „debian-<version>-standard“.

    fetch-base-template.py <zielverzeichnis> [--debian 13] [--arch amd64]

Die Liste der Templates (aplinfo) und die Dateien kommen von
download.proxmox.com. Die SHA-512-Prüfsumme aus der Liste wird kontrolliert.
Gibt den Pfad der heruntergeladenen Datei aus.
"""

import argparse
import hashlib
import re
import sys
import urllib.request
from pathlib import Path

APLINFO = "https://download.proxmox.com/images/aplinfo-pve-9.dat"
BASE_URL = "https://download.proxmox.com/images/"


def parse_aplinfo(text: str) -> list[dict[str, str]]:
    """aplinfo besteht aus Blöcken „Schlüssel: Wert“, getrennt durch Leerzeilen."""
    entries, current, last_key = [], {}, None
    for line in text.splitlines():
        if not line.strip():
            if current:
                entries.append(current)
            current, last_key = {}, None
        elif line[0].isspace() and last_key:
            current[last_key] += "\n" + line.strip()  # Fortsetzungszeile
        elif ":" in line:
            key, value = line.split(":", 1)
            last_key = key.strip().lower()
            current[last_key] = value.strip()
    if current:
        entries.append(current)
    return entries


def version_key(version: str) -> list:
    return [int(p) if p.isdigit() else p for p in re.split(r"[.\-]", version)]


def pick(entries: list[dict[str, str]], debian: str, arch: str) -> dict[str, str]:
    pattern = re.compile(rf"^system/debian-{debian}-standard_[^/]+_{arch}\.tar\.(zst|gz|xz)$")
    candidates = [e for e in entries if pattern.match(e.get("location", ""))]
    if not candidates:
        sys.exit(f"Kein Template debian-{debian}-standard ({arch}) in {APLINFO} gefunden")
    return max(candidates, key=lambda e: version_key(e.get("version", "0")))


def download(url: str, target: Path) -> None:
    with urllib.request.urlopen(url, timeout=120) as resp, open(target, "wb") as out:
        while chunk := resp.read(1 << 20):
            out.write(chunk)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("target_dir")
    ap.add_argument("--debian", default="13")
    ap.add_argument("--arch", default="amd64")
    ap.add_argument("--aplinfo", help="lokale aplinfo-Datei statt Download (für Tests)")
    ap.add_argument("--dry-run", action="store_true", help="nur anzeigen, nichts laden")
    args = ap.parse_args()

    if args.aplinfo:
        text = Path(args.aplinfo).read_text(encoding="utf-8")
    else:
        with urllib.request.urlopen(APLINFO, timeout=60) as resp:
            text = resp.read().decode("utf-8")
    entry = pick(parse_aplinfo(text), args.debian, args.arch)
    url = BASE_URL + entry["location"]
    target = Path(args.target_dir) / Path(entry["location"]).name
    print(f"Basis-Template: {entry['location']} (Version {entry.get('version')})", file=sys.stderr)
    if args.dry_run:
        print(target)
        return

    target.parent.mkdir(parents=True, exist_ok=True)
    download(url, target)

    expected = entry.get("sha512sum")
    if expected:
        actual = hashlib.sha512(target.read_bytes()).hexdigest()
        if actual != expected:
            target.unlink()
            sys.exit(f"Prüfsumme stimmt nicht für {url}")
        print("SHA-512 geprüft", file=sys.stderr)
    else:
        print("Warnung: keine Prüfsumme in aplinfo", file=sys.stderr)
    print(target)


if __name__ == "__main__":
    main()
