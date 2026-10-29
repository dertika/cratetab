-- Crate Tab – initiales Datenbankschema
--
-- Grundregeln:
--   * Alle Geldbeträge sind INTEGER in Cent (nie Fließkommazahlen!).
--   * Zeitstempel sind TEXT im Format "YYYY-MM-DDTHH:MM:SSZ" (UTC). So sind sie
--     lesbar, korrekt sortierbar und Importe können Original-Zeitstempel behalten.
--   * Buchungen (transactions) werden nie gelöscht oder geändert. Ein Storno ist
--     eine neue Gegenbuchung.
--   * legacy_id-Spalten nehmen beim späteren Import aus fnordcredit die alte ID auf.

-- Mitglieder, die am Kiosk Getränke buchen.
CREATE TABLE users (
    id                  INTEGER PRIMARY KEY,
    name                TEXT    NOT NULL COLLATE NOCASE UNIQUE,
    -- argon2-Hash der PIN im PHC-Format, NULL = keine PIN
    pin_hash            TEXT,
    -- Zwischengespeichertes Guthaben. Wird in derselben DB-Transaktion wie jede
    -- Buchung aktualisiert und lässt sich jederzeit aus `transactions` nachrechnen.
    balance_cents       INTEGER NOT NULL DEFAULT 0,
    -- bevorzugte Sprache (z. B. "de"), NULL = Sprache des Geräts verwenden
    language            TEXT,
    active              INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    -- Schutz gegen Durchprobieren von PINs
    failed_pin_attempts INTEGER NOT NULL DEFAULT 0,
    pin_locked_until    TEXT,
    legacy_id           TEXT UNIQUE,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL
);

-- Zugänge für den Admin-Bereich (getrennt von den Kiosk-Benutzern).
CREATE TABLE admins (
    id            INTEGER PRIMARY KEY,
    username      TEXT NOT NULL COLLATE NOCASE UNIQUE,
    password_hash TEXT NOT NULL,
    active        INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at    TEXT NOT NULL
);

-- Artikel (Getränke, Snacks …)
CREATE TABLE articles (
    id          INTEGER PRIMARY KEY,
    -- Name in der Standardsprache (Deutsch); Übersetzungen siehe article_translations
    name        TEXT    NOT NULL,
    price_cents INTEGER NOT NULL CHECK (price_cents >= 0),
    active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    sort_order  INTEGER NOT NULL DEFAULT 0,
    legacy_id   TEXT UNIQUE,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

-- Optionale Artikelnamen pro Sprache. Fehlt ein Eintrag, wird articles.name benutzt.
CREATE TABLE article_translations (
    article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    language   TEXT    NOT NULL,
    name       TEXT    NOT NULL,
    PRIMARY KEY (article_id, language)
);

-- Artikelbilder liegen direkt in der Datenbank. Dadurch besteht ein Backup aus
-- genau einer Datei.
CREATE TABLE article_images (
    article_id INTEGER PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE,
    mime_type  TEXT NOT NULL,
    data       BLOB NOT NULL,
    updated_at TEXT NOT NULL
);

-- Ein Artikel kann beliebig viele Barcodes haben, ein Barcode gehört zu genau einem Artikel.
CREATE TABLE barcodes (
    code       TEXT    PRIMARY KEY,
    article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE
);
CREATE INDEX idx_barcodes_article ON barcodes(article_id);

-- Das Buchungsjournal. Guthaben = Summe aller amount_cents eines Benutzers.
CREATE TABLE transactions (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id),
    -- Vorzeichenbehaftet: Kauf negativ, Aufladung positiv
    amount_cents INTEGER NOT NULL,
    kind         TEXT    NOT NULL CHECK (kind IN ('purchase', 'deposit', 'correction', 'reversal')),
    article_id   INTEGER REFERENCES articles(id),
    -- Artikelname zum Zeitpunkt der Buchung (bleibt erhalten, wenn der Artikel umbenannt wird)
    article_name TEXT,
    -- Freitext: Begründung bei Korrekturen, Beschreibung bei importierten Buchungen
    note         TEXT,
    -- ausführender Admin (bei Korrekturen)
    admin_id     INTEGER REFERENCES admins(id),
    -- bei Storno: die stornierte Buchung. UNIQUE verhindert doppeltes Stornieren.
    reverses_id  INTEGER UNIQUE REFERENCES transactions(id),
    legacy_id    TEXT UNIQUE,
    created_at   TEXT    NOT NULL
);
CREATE INDEX idx_transactions_user ON transactions(user_id, created_at);
CREATE INDEX idx_transactions_created ON transactions(created_at);
CREATE INDEX idx_transactions_article ON transactions(article_id);

-- Anmeldesitzungen (Admin-Login und entsperrte Kiosk-Benutzer).
CREATE TABLE sessions (
    token      TEXT PRIMARY KEY,
    -- genau eins von beiden ist gesetzt
    admin_id   INTEGER REFERENCES admins(id) ON DELETE CASCADE,
    user_id    INTEGER REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL,
    CHECK ((admin_id IS NULL) <> (user_id IS NULL))
);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);
