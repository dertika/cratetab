# Crate Tab – Deutsch (Standardsprache)
#
# Format: schluessel = Text
# Platzhalter stehen in geschweiften Klammern, z. B. { $name }.
# Diese Datei ist die Referenz: Jede andere Sprache muss dieselben Schlüssel haben.
# Doku zur Syntax: https://projectfluent.org/fluent/guide/

## Sprache und Formate

# Name dieser Sprache in ihr selbst (erscheint als Tooltip an der Flagge)
language-name = Deutsch
# Beträge: Dezimal- und Tausendertrennzeichen
fmt-decimal-separator = ,
fmt-thousands-separator = .
# Betrag mit Währung; $sign ist "-" oder leer
fmt-money = { $sign }{ $amount } { $currency }
# Datumsformate (strftime-Syntax: %d Tag, %m Monat, %Y Jahr, %H:%M Uhrzeit)
fmt-datetime = %d.%m.%Y %H:%M
fmt-date = %d.%m.%Y

## Allgemein

app-name = Crate Tab
app-tagline = Die Getränkekasse für euren Verein – geschrieben in Rust 🦀
language-switch = Sprache wählen
back = Zurück
cancel = Abbrechen
save = Speichern
saved = Gespeichert.
upload = Hochladen
status-active = aktiv
status-inactive = inaktiv
keypad-back = Letzte Ziffer löschen
keypad-clear = Eingabe löschen

## Kiosk – Startseite

user-search = Name suchen …
user-search-empty = Niemand gefunden.
users-empty = Noch keine Benutzer. Leg dich gleich selbst an!
letters-all = Alle
register-button = Neu hier?
pin-protected = mit PIN geschützt
admin-link = Admin-Bereich

## Kiosk – Registrierung

register-title = Neu anmelden
register-name = Name
register-pin-hint = Optional: Mit einer PIN (4–8 Ziffern) kann niemand sonst auf deinen Namen buchen.
register-submit = Anlegen

## Kiosk – PIN

pin-prompt = Bitte PIN eingeben.
pin-label = PIN
pin-submit = Weiter
pin-digits-only = Die PIN besteht nur aus Ziffern.
pin-wrong = PIN ist falsch.
pin-locked = Zu viele Fehlversuche. Bitte in { $minutes } Minuten erneut versuchen.
pin-new = Neue PIN
pin-repeat = PIN wiederholen
pin-optional = PIN (optional)
pin-rules = Die PIN besteht aus 4 bis 8 Ziffern.
pin-set-title = PIN festlegen
pin-change-title = PIN ändern
pin-remove = PIN entfernen
pin-remove-confirm = PIN wirklich entfernen? Dann kann jede Person auf deinen Namen buchen.
pin-saved = PIN gespeichert.
pin-removed = PIN entfernt.

## Kiosk – Benutzerseite

hello-user = Hallo { $name }!
done-button = Fertig
settings-button = Einstellungen
balance-label = Guthaben
balance-min-hint = Dein Guthaben ist im Minus. Erlaubt sind bis zu { $min }.
articles-title = Getränke & Snacks
articles-empty = Noch keine Artikel angelegt.
scan-camera = Barcode scannen
scan-hint = Tippe auf einen Artikel oder scanne seinen Barcode.
scan-select-user-first = Bitte tippe zuerst auf deinen Namen und scanne dann.
scan-unknown = Unbekannter Barcode { $code }. Bitte sag einem Admin Bescheid.
purchase-ok = { $article } gebucht ({ $price }). Prost!
deposit-title = Guthaben aufladen
deposit-hint = Lege das Geld in die Kasse und tippe dann auf den Betrag.
deposit-custom = Anderer Betrag
deposit-custom-label = Betrag (höchstens { $max })
deposit-submit = Aufladen
deposit-ok = { $amount } aufgeladen. Danke!
undo-button = Rückgängig: { $what }
undo-ok = Buchung storniert ({ $amount }).

## Kiosk – Einstellungen

settings-title = Einstellungen
language-preference = Bevorzugte Sprache
language-preference-hint = Nach der Auswahl deines Namens wechselt der Kiosk automatisch in diese Sprache.
language-device = Sprache des Geräts verwenden
language-saved = Sprache gespeichert.

## Kamera-Scanner

camera-title = Barcode in die Kamera halten
camera-close = Schließen
camera-unavailable = Keine Kamera verfügbar oder Zugriff verweigert.
camera-insecure = Die Kamera funktioniert nur über HTTPS.

## Fehlermeldungen

error-internal = Da ist etwas schiefgelaufen. Bitte nochmal versuchen.
error-network = Keine Verbindung zum Server. Bitte nochmal versuchen.
error-not-found = Nicht gefunden.
error-name-empty = Bitte einen Namen eingeben.
error-name-too-long = Der Name darf höchstens { $max } Zeichen lang sein.
error-name-taken = Dieser Name ist schon vergeben.
error-pin-invalid = Die PIN muss aus 4 bis 8 Ziffern bestehen.
error-pin-mismatch = Die beiden PINs stimmen nicht überein.
error-user-not-found = Benutzer nicht gefunden oder deaktiviert.
error-insufficient-balance = Guthaben reicht nicht – bitte erst aufladen. Erlaubt ist ein Kontostand bis { $min }.
error-article-unavailable = Dieser Artikel ist nicht verfügbar.
error-amount-invalid = Bitte einen gültigen Betrag eingeben.
error-amount-too-large = Der Betrag ist zu hoch.
error-balance-max = Mit dieser Aufladung wäre das Guthaben über dem Höchstguthaben von { $max }.
error-deposit-max = Pro Aufladung sind höchstens { $max } möglich.
error-nothing-to-undo = Es gibt nichts mehr rückgängig zu machen.
error-reason-required = Bitte eine Begründung angeben.
error-price-invalid = Bitte einen gültigen Preis eingeben (z. B. 1,50).
error-barcode-invalid = Ungültiger Barcode: { $code }
error-barcode-taken = Der Barcode { $code } gehört schon zu „{ $article }“.
error-image-type = Bitte ein Bild im Format PNG, JPEG, WebP oder GIF hochladen.
error-image-size = Das Bild ist zu groß (höchstens { $kb } KB).
error-password-short = Das Passwort muss mindestens { $min } Zeichen lang sein.
error-password-mismatch = Die beiden Passwörter stimmen nicht überein.
error-password-wrong = Das aktuelle Passwort ist falsch.

## Admin – Allgemein

admin-title = Admin-Bereich
admin-nav-dashboard = Übersicht
admin-nav-articles = Artikel
admin-nav-users = Benutzer
admin-nav-transactions = Buchungen
admin-nav-stats = Statistik
admin-nav-admins = Admins
admin-nav-kiosk = Zum Kiosk
admin-logout = Abmelden ({ $name })
admin-login-title = Admin-Anmeldung
admin-login-submit = Anmelden
admin-login-failed = Benutzername oder Passwort falsch.
admin-username = Benutzername
admin-password = Passwort
admin-recent-transactions = Letzte Buchungen
admin-show-all = Alle anzeigen
admin-reversed = Buchung storniert.
admin-balance-mismatch =
    { $count ->
        [one] Bei einem Benutzer weicht das Guthaben von der Summe der Buchungen ab:
       *[other] Bei { $count } Benutzern weicht das Guthaben von der Summe der Buchungen ab:
    }

## Admin – Artikel

article-new = Neuer Artikel
article-name = Name
article-name-in = Name ({ $language })
article-name-fallback = leer = deutscher Name
article-price = Preis
article-sort = Sortierung
article-status = Status
article-active = Artikel ist aktiv (wird am Kiosk angezeigt)
article-barcodes = Barcodes
article-barcodes-hint = Ein Barcode pro Zeile. Mit dem USB-Scanner einfach ins Feld scannen.
article-image = Bild
article-image-none = Noch kein Bild.
article-image-upload = Bild hochladen (PNG, JPEG, WebP oder GIF, höchstens { $kb } KB)
article-image-delete = Bild entfernen
article-image-delete-confirm = Bild wirklich entfernen?
article-move-up = Nach oben
article-move-down = Nach unten

## Admin – Benutzer

user-new = Benutzer anlegen
user-since = Angelegt
user-data = Stammdaten
user-active = Benutzer ist aktiv
user-active-hint = Deaktivierte Benutzer erscheinen nicht mehr am Kiosk. Ihre Buchungen bleiben erhalten.
user-has-pin = Der Benutzer hat eine PIN.
user-no-pin = Der Benutzer hat keine PIN.
pin-reset = PIN zurücksetzen
pin-reset-label = Neue PIN (leer lassen, um die PIN zu entfernen)
pin-reset-confirm = PIN dieses Benutzers wirklich zurücksetzen?
correction-title = Guthaben korrigieren
correction-hint = Positiver Betrag erhöht das Guthaben, negativer verringert es. Die Begründung erscheint in der Buchungsübersicht.
correction-amount = Betrag
correction-reason = Begründung (Pflicht)
correction-submit = Korrektur buchen

## Admin – Buchungen

tx-time = Zeit
tx-user = Benutzer
tx-kind = Art
tx-article = Artikel
tx-amount = Betrag
tx-note = Notiz
tx-sum = Summe
tx-empty = Keine Buchungen gefunden.
tx-reverse = Stornieren
tx-reverse-confirm = Buchung #{ $id } wirklich stornieren? Es wird eine Gegenbuchung angelegt.
tx-reverses = Storno von #{ $id }
tx-reversed-by = storniert durch #{ $id }
tx-count =
    { $count ->
        [one] 1 Buchung
       *[other] { $count } Buchungen
    }
tx-kind-purchase = Kauf
tx-kind-deposit = Aufladung
tx-kind-correction = Korrektur
tx-kind-reversal = Storno
filter-all = alle
filter-from = von
filter-to = bis
filter-apply = Filtern
filter-reset = Zurücksetzen
export-csv = CSV-Export
page-prev = Zurück
page-next = Weiter
page-current = Seite { $page }

## CSV-Export (Spaltenüberschriften)

csv-id = Nr
csv-time = Zeit
csv-user = Benutzer
csv-kind = Art
csv-article = Artikel
csv-amount = Betrag
csv-amount-cents = Betrag (Cent)
csv-note = Notiz
csv-admin = Admin
csv-reverses = Storno von

## Admin – Statistik

stats-total-balance = Summe aller Guthaben
stats-credit = davon Guthaben
stats-debt = davon Schulden
stats-users = Benutzer
stats-month-revenue = Umsatz diesen Monat
stats-month-deposits = Aufladungen diesen Monat
stats-year = Jahr
stats-year-revenue = Umsatz { $year }
stats-year-deposits = Aufladungen { $year }
stats-by-article = Umsatz pro Artikel { $year }
stats-by-month = Pro Monat
stats-quantity = Anzahl
stats-revenue = Umsatz
stats-deposits = Aufladungen

## Admin – Zugänge

admin-new = Neuen Admin anlegen
admin-me = du
admin-activate = Aktivieren
admin-deactivate = Deaktivieren
admin-change-password = Eigenes Passwort ändern
admin-password-current = Aktuelles Passwort
admin-password-new = Neues Passwort
admin-password-repeat = Neues Passwort wiederholen

## Admin – Import aus fnordcredit

admin-nav-import = Import
import-title = Import aus fnordcredit
import-intro = Übernimmt Benutzer, Guthaben, PINs, Produkte und die komplette Buchungshistorie aus dem alten fnordcredit. Nach dem Hochladen läuft zuerst ein Probelauf. Gespeichert wird erst, wenn du bestätigst.
import-howto = Wie man den Export erstellt, steht in der Anleitung:
import-not-empty = Die Datenbank ist nicht leer. Der Import ist nur möglich, wenn die vorhandenen Daten ersetzt werden. Vorhanden sind:
import-file = Export-Datei
import-file-hint = fnordcredit-export.tar.gz – oder users.json, transactions.json und products.json zusammen auswählen.
import-upload = Hochladen und prüfen
import-no-file = Bitte eine Datei auswählen.
import-error = Import nicht möglich: { $error }
import-expired = Der Upload ist abgelaufen oder wurde schon verwendet. Bitte die Datei erneut hochladen.
import-dry-run-title = Probelauf – es wurde noch nichts gespeichert. Bitte den Bericht prüfen und dann bestätigen.
import-done-title = Import abgeschlossen.
import-files = Gelesen
import-users = Benutzer
import-articles = Artikel
import-transactions = Buchungen
import-barcodes =
    { $count ->
        [one] 1 Barcode
       *[other] { $count } Barcodes
    }
import-purchases =
    { $count ->
        [one] 1 Kauf
       *[other] { $count } Käufe
    }
import-deposits =
    { $count ->
        [one] 1 Aufladung
       *[other] { $count } Aufladungen
    }
import-images =
    { $count ->
        [one] 1 Bild
       *[other] { $count } Bilder
    }
import-hint-images = Bilder nicht übernommen
import-hint-images-text = Fehlende Bilder: den Export mit der aktuellen Version von scripts/fnordcredit-export.js wiederholen. Alle Bilder lassen sich auch später im Admin-Bereich beim Artikel hochladen.
import-image-missing = Bilddatei fehlt im Export
import-image-unsupported = Format nicht unterstützt (nur PNG, JPEG, GIF, WebP)
import-image-too-large = Bild ist zu groß
import-pins = PINs übernommen
import-pins-hint = werden beim ersten Login auf argon2 umgestellt
import-total-old = Summe aller Guthaben in fnordcredit
import-total-new = Summe aller Guthaben in Crate Tab
import-total-orphans = davon { $amount } bei Benutzern ohne alten Kontostand
import-total-ok = Die Summen aller Guthaben stimmen überein.
import-total-mismatch = Die Summen aller Guthaben weichen ab – bitte prüfen!
import-hints = Hinweise
import-hint-reconcile = Ausgleichsbuchungen
import-hint-reconcile-text = Bei diesen Benutzern passt die Summe der Buchungen nicht zum Kontostand in fnordcredit (z. B. Benutzer aus der Zeit vor der Buchungshistorie). Eine Korrekturbuchung gleicht die Differenz aus.
import-col-balance = Kontostand fnordcredit
import-col-sum = Summe der Buchungen
import-col-difference = Ausgleich
import-hint-renamed = Umbenannt (Name doppelt oder zu lang)
import-hint-orphans = Buchungen ohne Benutzer – als deaktivierte Benutzer angelegt
import-hint-pins-dropped = PIN in unbekanntem Format, nicht übernommen
import-hint-tokens = Hatten einen Token (entfällt)
import-hint-debt = Hatten eigene Schuldengrenzen (entfallen, es gilt das globale Minimalguthaben)
import-debt-none = keine Schulden erlaubt
import-debt-limit = Grenze { $amount }
import-hint-unknown-products = Produkte aus Buchungen, die es nicht mehr gibt (nur als Name übernommen)
import-hint-barcodes = Barcodes übersprungen
import-barcode-invalid = ungültig
import-barcode-taken = gehört schon zu { $article }
import-hint-unreadable = Nicht lesbare Datensätze
import-hint-zero =
    { $count ->
        [one] 1 Buchung ohne Betrag übersprungen.
       *[other] { $count } Buchungen ohne Betrag übersprungen.
    }
import-hint-no-time =
    { $count ->
        [one] 1 Buchung ohne Zeitstempel übersprungen.
       *[other] { $count } Buchungen ohne Zeitstempel übersprungen.
    }
import-hint-chain =
    { $count ->
        [one] 1 Buchung passte in fnordcredit nicht zum vorherigen Kontostand (Hinweis auf frühere Änderungen direkt in der Datenbank).
       *[other] { $count } Buchungen passten in fnordcredit nicht zum vorherigen Kontostand (Hinweis auf frühere Änderungen direkt in der Datenbank).
    }
import-replace = Vorhandene Daten ersetzen
import-replace-hint = Benutzer, Artikel und Buchungen werden gelöscht und durch den Import ersetzt. Admin-Zugänge bleiben erhalten. Vorher wird automatisch eine Sicherung angelegt. Im Probelauf wird noch nichts gelöscht.
import-replace-will-delete = Beim Import werden gelöscht:
import-replace-deleted = Gelöscht und ersetzt:
import-replace-admins-kept = Admin-Zugänge bleiben erhalten.
import-replace-ack = Ich habe verstanden, dass die vorhandenen Benutzer, Artikel und Buchungen gelöscht werden.
import-replace-backup-hint = Vor dem Ersetzen wird die bisherige Datenbank im Ordner „backups“ neben der Datenbankdatei gesichert.
import-replace-confirm = Daten ersetzen und importieren
import-replace-confirm-question = Vorhandene Daten jetzt wirklich löschen und durch den Import ersetzen?
import-replace-ack-missing = Bitte bestätigen, dass die vorhandenen Daten gelöscht werden.
import-backup-created = Sicherung der bisherigen Datenbank:
import-backup-failed = Die Sicherung vor dem Ersetzen ist fehlgeschlagen, es wurde nichts geändert: { $error }
import-confirm = Jetzt importieren
import-confirm-question = Die Daten jetzt übernehmen?
import-confirm-hint = Der Upload bleibt 30 Minuten gültig. Danach bitte erneut hochladen.
import-to-users = Zu den Benutzern

## Admin – Einstellungen

admin-nav-settings = Einstellungen
settings-page-title = Einstellungen
settings-intro = Änderungen gelten sofort. Die Werte werden in der Datenbank gespeichert und sind damit in jedem Backup enthalten.
settings-section-general = Allgemein
settings-section-balance = Guthaben und Aufladen
settings-section-kiosk = Kiosk
settings-club-name = Vereinsname
settings-default-language = Standardsprache
settings-self-registration = Neue Benutzer dürfen sich am Kiosk selbst anlegen
settings-min-balance = Minimalguthaben
settings-min-balance-hint = Tiefster erlaubter Kontostand, z. B. -10. Mit 0 sind keine Schulden möglich.
settings-max-balance = Höchstguthaben
settings-max-balance-hint = Höchster Kontostand nach einer Aufladung am Kiosk. Leer lassen für keine Grenze. Korrekturen im Admin-Bereich sind ausgenommen.
settings-max-deposit = Höchstbetrag pro Aufladung
settings-deposit-presets = Schnellwahl-Beträge zum Aufladen
settings-deposit-presets-hint = Durch Leerzeichen getrennt, z. B. „5 10 20“. Höchstens 8 Beträge, jeder bis zum Höchstbetrag pro Aufladung.
settings-idle-timeout = Inaktivität bis zur Startseite (Sekunden)
settings-undo-window = Zeitfenster für „Rückgängig“ (Sekunden)
settings-undo-window-hint = 0 schaltet „Rückgängig“ am Kiosk ab.
settings-source-stored = im Admin-Bereich gespeichert
settings-source-config = aus der Konfiguration ({ $variable })
settings-reset = Alle Einstellungen auf die Konfiguration zurücksetzen
settings-reset-confirm = Alle hier gespeicherten Werte löschen? Danach gelten wieder die Werte aus der Konfiguration.
settings-file-hint = Datenbankpfad, Port, Zeitzone und das Startpasswort des ersten Admins stehen weiterhin in der Konfiguration (/etc/cratetab/cratetab.env bzw. docker-compose.yml).
settings-error-club-name = Bitte einen Vereinsnamen eingeben (höchstens 80 Zeichen).
settings-error-min-balance = Das Minimalguthaben darf nicht positiv sein (z. B. -10 oder 0).
settings-error-max-balance = Das Höchstguthaben muss positiv und größer als das Minimalguthaben sein.
settings-error-max-deposit = Der Höchstbetrag pro Aufladung muss positiv sein.
settings-error-presets = Die Schnellwahl-Beträge müssen positiv sein und dürfen den Höchstbetrag pro Aufladung nicht übersteigen (höchstens 8).
settings-error-idle-timeout = Die Inaktivität muss zwischen 10 und 3600 Sekunden liegen.
settings-error-undo-window = Das Zeitfenster für „Rückgängig“ muss zwischen 0 und 3600 Sekunden liegen.
settings-error-language = Diese Sprache gibt es nicht.
