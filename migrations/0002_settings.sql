-- Einstellungen, die im Admin-Bereich geändert werden können.
-- Fehlt ein Schlüssel, gilt der Wert aus der Konfiguration (Umgebungsvariable
-- bzw. /etc/cratetab/cratetab.env). Beträge in Cent, Zeiten in Sekunden.
CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Admin, der den Wert zuletzt geändert hat
    admin_id   INTEGER REFERENCES admins(id) ON DELETE SET NULL
);
