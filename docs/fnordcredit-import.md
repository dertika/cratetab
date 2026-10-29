# Import aus fnordcredit (geplant)

Der Import aus dem alten fnordcredit (RethinkDB) ist **noch nicht umgesetzt**.
Das Schema von Crate Tab ist aber schon darauf vorbereitet. Diese Seite hält
fest, wie der Import später funktionieren soll.

## Was schon vorbereitet ist

| Vorbereitung | Wo |
|---|---|
| Spalte `legacy_id` (UNIQUE) in `users`, `articles` und `transactions`, damit alte IDs erhalten bleiben und ein zweiter Import nichts doppelt anlegt | `migrations/0001_initial.sql` |
| Zeitstempel als Text (`YYYY-MM-DDTHH:MM:SSZ`), Original-Zeitstempel lassen sich direkt übernehmen | `migrations/0001_initial.sql`, `db::to_db_time` |
| Spalte `transactions.note` für die alte Beschreibung einer Buchung | `migrations/0001_initial.sql` |
| `ledger::record_imported()` schreibt eine historische Buchung mit vorgegebenem Zeitstempel und `legacy_id` und passt das Guthaben in derselben DB-Transaktion an (ohne Prüfung des Minimalguthabens) | `src/ledger.rs` |
| `cratetab check-balances` prüft nach dem Import, ob alle Guthaben zur Buchungshistorie passen | `src/main.rs` |

## Geplanter Ablauf

1. In fnordcredit die Tabellen als JSON exportieren, z. B. mit
   `rethinkdb export -e fnordcredit.users -e fnordcredit.transactions --format json`.
2. Neuer Befehl `cratetab import-fnordcredit <export-verzeichnis>`:
   * Benutzer anlegen: `name` → `users.name`, RethinkDB-`id` → `users.legacy_id`.
     Die alten PINs liegen bei fnordcredit nicht als argon2-Hash vor. Betroffene
     Benutzer bekommen deshalb keine PIN und setzen sie am Kiosk neu.
   * Buchungen in zeitlicher Reihenfolge mit `ledger::record_imported()` übernehmen:
     `delta` (Euro, Kommazahl) wird **exakt** in Cent umgerechnet (Rundung prüfen!),
     `time` → `created_at`, `description` → `note`, `id` → `legacy_id`.
     Positive Beträge werden zu `deposit`, negative zu `purchase` (ohne Artikel-ID).
   * Weicht das Ergebnis vom Kontostand `credit` in fnordcredit ab, wird pro
     Benutzer eine `correction`-Buchung mit Begründung „Import fnordcredit:
     Ausgleich“ angelegt, damit das Guthaben exakt stimmt.
   * Alles läuft in einer einzigen DB-Transaktion: Bei einem Fehler bleibt die
     Datenbank unverändert.
3. Danach `cratetab check-balances` ausführen.

Offene Fragen für später: Welche fnordcredit-Version genau im Einsatz ist
(Feldnamen unterscheiden sich zwischen Versionen) und ob Produkte/Barcodes aus
fnordcredit ebenfalls übernommen werden sollen.
