//! Konfiguration über Umgebungsvariablen.
//!
//! Alle Variablen beginnen mit `CRATETAB_`. Eine vollständige Liste mit
//! Erklärungen steht in der README.

use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono_tz::Tz;

use crate::money::parse_cents;

#[derive(Debug, Clone)]
pub struct Config {
    /// Name des Vereins, wird neben „Crate Tab“ angezeigt
    pub club_name: String,
    /// Währungssymbol, z. B. „€“
    pub currency_symbol: String,
    /// Tiefster erlaubter Kontostand in Cent (z. B. -1000 = bis -10 € Schulden)
    pub min_balance_cents: i64,
    /// Sprache, wenn weder Cookie noch Browser etwas Passendes liefern
    pub default_language: String,
    /// Passwort für den ersten Admin-Zugang (nur beim allerersten Start benutzt)
    pub admin_initial_password: Option<String>,
    /// Pfad zur SQLite-Datei
    pub database_path: PathBuf,
    /// Adresse, auf der der Server lauscht
    pub listen: SocketAddr,
    /// Dürfen sich neue Benutzer am Kiosk selbst anlegen?
    pub self_registration: bool,
    /// Nach so vielen Sekunden ohne Eingabe springt der Kiosk zur Startseite
    pub idle_timeout_secs: u64,
    /// So lange (Sekunden) kann die letzte Buchung am Kiosk rückgängig gemacht werden
    pub undo_window_secs: i64,
    /// Höchstbetrag einer einzelnen Aufladung am Kiosk in Cent
    pub max_deposit_cents: i64,
    /// Schnellwahl-Beträge für die Aufladung in Cent
    pub deposit_presets_cents: Vec<i64>,
    /// Zeitzone für die Anzeige von Zeitstempeln
    pub timezone: Tz,
    /// Optionales Verzeichnis mit zusätzlichen/überschreibenden .ftl-Dateien
    pub locales_dir: Option<PathBuf>,
    /// Beim Start Demo-Daten anlegen, wenn die Datenbank leer ist
    pub seed_demo: bool,
    /// Maximale Größe eines Artikelbildes in Bytes
    pub max_image_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            club_name: "Mein Verein e. V.".into(),
            currency_symbol: "€".into(),
            min_balance_cents: -1000,
            default_language: "de".into(),
            admin_initial_password: None,
            database_path: PathBuf::from("data/cratetab.db"),
            listen: SocketAddr::from(([0, 0, 0, 0], 8080)),
            self_registration: true,
            idle_timeout_secs: 60,
            undo_window_secs: 60,
            max_deposit_cents: 10_000,
            deposit_presets_cents: vec![500, 1000, 2000],
            timezone: chrono_tz::Europe::Berlin,
            locales_dir: None,
            seed_demo: false,
            max_image_bytes: 2 * 1024 * 1024,
        }
    }
}

/// Liest eine Umgebungsvariable; leere Werte zählen als „nicht gesetzt“.
fn var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn parse_bool(name: &str, value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "ja" | "on" => Ok(true),
        "0" | "false" | "no" | "nein" | "off" => Ok(false),
        _ => bail!("{name}: „{value}“ ist kein Wahrheitswert (true/false)"),
    }
}

fn parse_amount(name: &str, value: &str) -> Result<i64> {
    parse_cents(value).map_err(|e| anyhow::anyhow!("{name}: „{value}“: {e}"))
}

impl Config {
    /// Baut die Konfiguration aus den Umgebungsvariablen; nicht gesetzte Werte
    /// bekommen die Vorgaben aus [`Config::default`].
    pub fn from_env() -> Result<Self> {
        let mut c = Config::default();

        if let Some(v) = var("CRATETAB_CLUB_NAME") {
            c.club_name = v;
        }
        if let Some(v) = var("CRATETAB_CURRENCY") {
            c.currency_symbol = v;
        }
        if let Some(v) = var("CRATETAB_MIN_BALANCE") {
            c.min_balance_cents = parse_amount("CRATETAB_MIN_BALANCE", &v)?;
            if c.min_balance_cents > 0 {
                bail!("CRATETAB_MIN_BALANCE darf nicht positiv sein (z. B. -10 oder 0)");
            }
        }
        if let Some(v) = var("CRATETAB_DEFAULT_LANGUAGE") {
            c.default_language = v.to_ascii_lowercase();
        }
        c.admin_initial_password = var("CRATETAB_ADMIN_PASSWORD");
        if let Some(v) = var("CRATETAB_DATABASE") {
            c.database_path = PathBuf::from(v);
        }
        if let Some(v) = var("CRATETAB_LISTEN") {
            c.listen = v.parse().with_context(|| {
                format!("CRATETAB_LISTEN: „{v}“ ist keine Adresse wie 0.0.0.0:8080")
            })?;
        }
        if let Some(v) = var("CRATETAB_SELF_REGISTRATION") {
            c.self_registration = parse_bool("CRATETAB_SELF_REGISTRATION", &v)?;
        }
        if let Some(v) = var("CRATETAB_IDLE_TIMEOUT") {
            c.idle_timeout_secs = v
                .parse()
                .context("CRATETAB_IDLE_TIMEOUT: Sekunden als Zahl")?;
        }
        if let Some(v) = var("CRATETAB_UNDO_WINDOW") {
            c.undo_window_secs = v
                .parse()
                .context("CRATETAB_UNDO_WINDOW: Sekunden als Zahl")?;
        }
        if let Some(v) = var("CRATETAB_MAX_DEPOSIT") {
            c.max_deposit_cents = parse_amount("CRATETAB_MAX_DEPOSIT", &v)?;
        }
        if let Some(v) = var("CRATETAB_DEPOSIT_PRESETS") {
            c.deposit_presets_cents = v
                .split([';', ' '])
                .filter(|s| !s.is_empty())
                .map(|s| parse_amount("CRATETAB_DEPOSIT_PRESETS", s))
                .collect::<Result<_>>()?;
        }
        if let Some(v) = var("CRATETAB_TIMEZONE") {
            c.timezone = v
                .parse()
                .map_err(|_| anyhow::anyhow!("CRATETAB_TIMEZONE: unbekannte Zeitzone „{v}“"))?;
        }
        c.locales_dir = var("CRATETAB_LOCALES_DIR").map(PathBuf::from);
        if let Some(v) = var("CRATETAB_SEED_DEMO") {
            c.seed_demo = parse_bool("CRATETAB_SEED_DEMO", &v)?;
        }
        if let Some(v) = var("CRATETAB_MAX_IMAGE_KB") {
            let kb: usize = v
                .parse()
                .context("CRATETAB_MAX_IMAGE_KB: Zahl in Kilobyte")?;
            c.max_image_bytes = kb * 1024;
        }

        Ok(c)
    }
}
