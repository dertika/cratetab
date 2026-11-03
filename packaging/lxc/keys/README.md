# Proxmox-Release-Schlüssel

Mit diesen öffentlichen Schlüsseln prüft `fetch-base-template.py` die Signatur der
Proxmox-Template-Liste (`aplinfo-pve-9.dat.asc`), genau wie `pveam` auf einem
Proxmox-Host. Sie stammen aus
[pve-manager/aplinfo](https://github.com/proxmox/pve-manager/tree/master/aplinfo).

| Datei | Fingerabdruck |
|---|---|
| `proxmox-release-trixie.gpg` | `24B3 0F06 ECC1 836A 4E5E  FECB A7BC D142 0BFE 778E` |
| `proxmox-release-bookworm.gpg` | `F4E1 36C6 7CDC E41A E6DE  6FC8 1140 AF8F 639E 0C39` |

Prüfen: `gpg --show-keys --with-fingerprint proxmox-release-trixie.gpg`
