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
