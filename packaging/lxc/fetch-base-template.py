#!/usr/bin/env python3
"""Lädt das neueste offizielle Proxmox-Template „debian-<version>-standard“.

    fetch-base-template.py <zielverzeichnis> [--debian 13] [--arch amd64]

Genau wie Proxmox' eigenes `pveam`:
  1. Template-Liste (aplinfo-pve-9.dat.gz) und ihre Signatur (.asc) laden.
     download.proxmox.com bietet nur HTTP an (das HTTPS-Zertifikat passt nicht).
  2. Signatur mit den Proxmox-Release-Schlüsseln prüfen (keys/*.gpg, stammen aus
     pve-manager/aplinfo/ – Fingerabdrücke siehe keys/README.md).
  3. Template laden und gegen die SHA-512-Summe aus der signierten Liste prüfen.
Gibt den Pfad der heruntergeladenen Datei aus.
"""

import argparse
import gzip
import hashlib
import re
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

BASE_URL = "http://download.proxmox.com/images/"
APLINFO = BASE_URL + "aplinfo-pve-9.dat"
KEY_DIR = Path(__file__).resolve().parent / "keys"


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
    pattern = re.compile(rf"(^|/)system/debian-{debian}-standard_[^/]+_{arch}\.tar\.(zst|gz|xz)$")
    candidates = [e for e in entries if pattern.search(e.get("location", ""))]
    if not candidates:
        sys.exit(f"Kein Template debian-{debian}-standard ({arch}) in {APLINFO} gefunden")
    return max(candidates, key=lambda e: version_key(e.get("version", "0")))


def download(url: str, target: Path) -> None:
    with urllib.request.urlopen(url, timeout=120) as resp, open(target, "wb") as out:
        while chunk := resp.read(1 << 20):
            out.write(chunk)


def fetch_verified_aplinfo() -> str:
    """Lädt die Template-Liste und prüft ihre Signatur."""
    keyrings = sorted(KEY_DIR.glob("*.gpg"))
    if not keyrings:
        sys.exit(f"Keine Schlüssel in {KEY_DIR}")
    with tempfile.TemporaryDirectory() as tmp:
        data, sig = Path(tmp) / "aplinfo.dat", Path(tmp) / "aplinfo.dat.asc"
        with urllib.request.urlopen(APLINFO + ".gz", timeout=60) as resp:
            data.write_bytes(gzip.decompress(resp.read()))
        download(APLINFO + ".asc", sig)
        cmd = ["gpgv"]
        for k in keyrings:
            cmd += ["--keyring", str(k)]
        cmd += [str(sig), str(data)]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            sys.exit(f"Signatur der Template-Liste ungültig:\n{result.stderr}")
        print("Signatur der Template-Liste geprüft", file=sys.stderr)
        return data.read_text(encoding="utf-8")


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
        text = fetch_verified_aplinfo()
    entry = pick(parse_aplinfo(text), args.debian, args.arch)
    location = entry["location"]
    url = location if "://" in location else BASE_URL + location
    target = Path(args.target_dir) / Path(entry["location"]).name
    print(f"Basis-Template: {entry['location']} (Version {entry.get('version')})", file=sys.stderr)
    if args.dry_run:
        print(target)
        return

    target.parent.mkdir(parents=True, exist_ok=True)
    download(url, target)

    expected = entry.get("sha512sum", "").lower()
    if not expected:
        target.unlink()
        sys.exit("Keine SHA-512-Summe in der Template-Liste")
    actual = hashlib.sha512(target.read_bytes()).hexdigest()
    if actual != expected:
        target.unlink()
        sys.exit(f"Prüfsumme stimmt nicht für {url}")
    print("SHA-512 des Templates geprüft", file=sys.stderr)
    print(target)


if __name__ == "__main__":
    main()
