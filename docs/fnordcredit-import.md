# Umzug von fnordcredit

Crate Tab übernimmt Benutzer, Guthaben, PINs, Produkte und die komplette
Buchungshistorie (mit Original-Zeitstempeln) aus dem alten **fnordcredit mit
RethinkDB** (Versionen bis Anfang 2017).

## 1. Daten aus fnordcredit exportieren

> Tipp: fnordcredit vorher stoppen, damit während des Exports niemand bucht.

**Variante A – mit dem Skript aus diesem Repository (empfohlen, kein Python nötig).**
Es nutzt den RethinkDB-Treiber und die `config.js` der vorhandenen
fnordcredit-Installation, passt also immer zur Version eures Servers:

```sh
curl -fsSL -o /root/fnordcredit-export.js \
  https://raw.githubusercontent.com/dertika/cratetab/main/scripts/fnordcredit-export.js
cd /pfad/zu/fnordcredit            # dort, wo config.js und node_modules liegen
node /root/fnordcredit-export.js /root/fnordcredit-export
```

Wo fnordcredit liegt, verrät z. B. `find / -name config.js -path '*fnordcredit*' -not -path '*/node_modules/*'`.

**Variante B – mit `rethinkdb export`.** Das braucht den Python-Treiber von
RethinkDB (sonst: „The rethinkdb-export command depends on the RethinkDB Python
driver“). Der aktuelle Treiber spricht nur mit RethinkDB ab Version 2.3:

```sh
python3 -m venv /root/rdb && /root/rdb/bin/pip install rethinkdb
PATH=/root/rdb/bin:$PATH rethinkdb export -e fnordcredit --format json -d /root/fnordcredit-export
```

Beide Varianten liefern dasselbe: `fnordcredit/users.json`, `transactions.json`
und (falls vorhanden) `products.json`. Danach einpacken:

```sh
cd /root && tar czf fnordcredit-export.tar.gz fnordcredit-export
```

## 2. Leere Datenbank oder „Ersetzen“

Am einfachsten importiert man in eine **leere** Datenbank, also ohne Benutzer,
Artikel und Buchungen. Admin-Zugänge dürfen schon existieren. Demo-Daten deshalb
ausschalten (`CRATETAB_SEED_DEMO=false`).

Enthält die Datenbank schon Daten, z. B. von Tests, gibt es **Ersetzen**:
- **Im Browser:** beim Hochladen „Vorhandene Daten ersetzen“ ankreuzen.
- **Auf der Kommandozeile:** `--replace` anhängen.

Was dabei passiert:
- Benutzer, Artikel (mit Bildern und Barcodes) und Buchungen werden gelöscht und durch den Import ersetzt. Admin-Zugänge bleiben.
- Der Probelauf zeigt, wie viel gelöscht würde, löscht aber noch nichts.
- Vor dem echten Import sichert Crate Tab die bisherige Datenbank nach `backups/vor-import-<Datum>.db` neben der Datenbankdatei (Docker: `/data/backups`, LXC: `/var/lib/cratetab/backups`).
- Löschen und Import laufen in einer Datenbank-Transaktion: Schlägt der Import fehl, bleibt alles, wie es war.

## 3. Probelauf und Import im Browser (empfohlen)

Im Admin-Bereich unter **Import** die Datei `fnordcredit-export.tar.gz` hochladen
(oder `users.json`, `transactions.json` und `products.json` zusammen auswählen).
Crate Tab macht sofort einen Probelauf und zeigt den Bericht. Erst ein Klick
auf **Jetzt importieren** übernimmt die Daten. Der Upload wird nur im
Arbeitsspeicher gehalten und verfällt nach 30 Minuten.

Läuft Crate Tab hinter nginx, muss das Upload-Limit groß genug sein
(`client_max_body_size 64m;`, siehe README).

## 3b. Probelauf auf der Kommandozeile (Alternative)

Der Probelauf liest alles, führt den kompletten Import durch und rollt ihn am
Ende zurück. Es wird nichts gespeichert, aber ihr seht genau, was passieren würde.

**LXC-Template** (auf dem Proxmox-Host, Container-ID 120):

```sh
pct push 120 fnordcredit-export.tar.gz /tmp/fnordcredit-export.tar.gz
pct exec 120 -- tar xzf /tmp/fnordcredit-export.tar.gz -C /tmp
pct exec 120 -- cratetab import-fnordcredit /tmp/fnordcredit-export --dry-run
```

**Docker:**

```sh
tar xzf fnordcredit-export.tar.gz -C data/      # landet in ./data, also /data im Container
docker compose exec cratetab cratetab import-fnordcredit /data/fnordcredit-export --dry-run
```

**Direkt** (ohne Container): `CRATETAB_DATABASE=… cratetab import-fnordcredit <ordner> --dry-run`

## 4. Bericht prüfen

Beispiel:

```
PROBELAUF – es wurde nichts gespeichert.

Benutzer:   42
Artikel:    9 (mit 14 Barcodes)
Buchungen:  18317 (16890 Käufe, 1427 Aufladungen), 02.06.2014 bis 24.02.2017
Ausgleichsbuchungen: 3
Summe aller Guthaben: alt 412,30 €, neu 412,30 € ✓
PINs übernommen: 17 (werden beim ersten Login auf argon2 umgestellt)

Hinweise:
  Ausgleichsbuchungen (3):
    - Ada: Kontostand 12,34 €, Summe der Buchungen 0,00 €, Ausgleich 12,34 €
  …
```

Was die Hinweise bedeuten:

| Hinweis | Bedeutung |
|---|---|
| **Ausgleichsbuchungen** | Die Summe der Buchungen passt nicht zum Kontostand in fnordcredit, z. B. bei Benutzern aus der Zeit vor der Buchungshistorie oder nach Änderungen direkt in der Datenbank. Eine Korrekturbuchung „Import fnordcredit: Ausgleich auf den alten Kontostand“ gleicht die Differenz aus. Danach stimmt jedes Guthaben auf den Cent. |
| **Umbenannt** | fnordcredit unterscheidet „Max“ und „max“, Crate Tab nicht. Der zweite Name bekommt einen Zusatz („max (2)“). Namen über 40 Zeichen werden gekürzt. Umbenennen geht danach im Admin-Bereich. |
| **Buchungen ohne Benutzer** | Buchungen zu Benutzern, die in fnordcredit gelöscht wurden. Sie werden als deaktivierte Benutzer angelegt, damit die Historie vollständig bleibt. |
| **Hatten einen Token** | Die Anmeldung per Karte/Token gibt es in Crate Tab nicht, der Token entfällt. |
| **Eigene Schuldengrenzen** | `debtAllowed`/`debtHardLimit` pro Benutzer entfallen. Es gilt das globale `CRATETAB_MIN_BALANCE`. |
| **Produkte, die es nicht mehr gibt** | Käufe eines gelöschten oder umbenannten Produkts. Sie bleiben mit Produktname, aber ohne Artikel-Verknüpfung erhalten. In der Statistik erscheinen sie daher nicht pro Artikel. |
| **Passten nicht zum vorherigen Kontostand** | Rein informativ: In fnordcredit stand bei diesen Buchungen ein Kontostand, der nicht zur Rechnung passt. |

## 5. Import

Im Browser: **Jetzt importieren**. Auf der Kommandozeile: denselben Befehl ohne `--dry-run` ausführen. Alles läuft in einer einzigen
Datenbank-Transaktion: Bei einem Fehler bleibt die Datenbank unverändert. Ein
zweiter Import wird abgelehnt.

Danach:

```sh
cratetab check-balances        # bzw. mit pct exec / docker compose exec davor
```

und den Export wieder löschen (er enthält PIN-Hashes): `rm -rf /tmp/fnordcredit-export*`.

## Was wie übernommen wird

| fnordcredit | Crate Tab |
|---|---|
| `users.name` | Benutzername, alter Name als `legacy_id` |
| `users.credit` | Guthaben (über Buchungen + ggf. Ausgleichsbuchung) |
| `users.pincode` | PIN bleibt gültig. Beim ersten Login wird sie auf argon2 umgestellt. Alte PINs dürfen länger als 8 Ziffern sein. Crate Tab akzeptiert nur Ziffern: Wer in fnordcredit eine PIN mit Buchstaben hatte, bekommt im Admin-Bereich eine neue (*Benutzer → PIN zurücksetzen*). |
| `transactions` mit `delta > 0` | Aufladung |
| `transactions` mit `delta < 0` | Kauf, mit Artikel, wenn `product` zu einem Produkt passt (Groß-/Kleinschreibung egal) |
| `transactions.description` | Notiz der Buchung |
| `transactions.time` / `id` | Original-Zeitstempel / `legacy_id` |
| `products` | Artikel mit Preis, Sortierung nach `order`, `ean` (durch `\|` getrennt) als Barcodes |
| Buchungen über 0,00 € | übersprungen (werden gezählt) |

Beträge sind in fnordcredit Kommazahlen. Sie werden einzeln auf Cent gerundet.
Die Ausgleichsbuchungen sorgen dafür, dass am Ende jedes Guthaben genau dem
alten Kontostand entspricht.
