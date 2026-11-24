//! Einstellungen, die der Verein im Admin-Bereich ändern kann.
//!
//! Startwerte kommen aus der Konfiguration ([`Config`], also Umgebungsvariablen).
//! Sobald ein Wert im Admin-Bereich gespeichert wurde, steht er in der Tabelle
//! `settings` und hat Vorrang. Die Werte liegen damit in der Datenbank und sind
//! in jedem Backup enthalten; Änderungen gelten sofort, ohne Neustart.

use std::collections::HashSet;

use sqlx::SqlitePool;

use crate::{config::Config, db, money};

/// Schlüssel in der Tabelle `settings`.
pub mod keys {
    pub const CLUB_NAME: &str = "club_name";
    pub const MIN_BALANCE: &str = "min_balance_cents";
    pub const MAX_BALANCE: &str = "max_balance_cents";
    pub const MAX_DEPOSIT: &str = "max_deposit_cents";
    pub const DEPOSIT_PRESETS: &str = "deposit_presets_cents";
    pub const SELF_REGISTRATION: &str = "self_registration";
    pub const IDLE_TIMEOUT: &str = "idle_timeout_secs";
    pub const UNDO_WINDOW: &str = "undo_window_secs";
    pub const DEFAULT_LANGUAGE: &str = "default_language";

    pub const ALL: [&str; 9] = [
        CLUB_NAME,
        MIN_BALANCE,
        MAX_BALANCE,
        MAX_DEPOSIT,
        DEPOSIT_PRESETS,
        SELF_REGISTRATION,
        IDLE_TIMEOUT,
        UNDO_WINDOW,
        DEFAULT_LANGUAGE,
    ];
}

/// Die aktuell gültigen Einstellungen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Name des Vereins in der Kopfzeile
    pub club_name: String,
    /// Tiefster erlaubter Kontostand in Cent (≤ 0)
    pub min_balance_cents: i64,
    /// Höchster erlaubter Kontostand nach einer Aufladung in Cent; `None` = keine Grenze
    pub max_balance_cents: Option<i64>,
    /// Höchstbetrag einer einzelnen Aufladung in Cent
    pub max_deposit_cents: i64,
    /// Schnellwahl-Beträge der Aufladung in Cent
    pub deposit_presets_cents: Vec<i64>,
    /// Dürfen sich neue Benutzer am Kiosk selbst anlegen?
    pub self_registration: bool,
    /// Sekunden ohne Eingabe, bis der Kiosk zur Startseite springt
    pub idle_timeout_secs: u64,
    /// Sekunden, in denen die letzte Buchung rückgängig gemacht werden kann
    pub undo_window_secs: i64,
    /// Sprache, wenn weder Cookie noch Browser etwas Passendes liefern
    pub default_language: String,
    /// Welche Werte im Admin-Bereich gespeichert sind (Rest: aus der Konfiguration)
    pub stored: HashSet<String>,
}

impl Settings {
    /// Die Startwerte aus der Konfiguration.
    pub fn from_config(config: &Config) -> Self {
        Settings {
            club_name: config.club_name.clone(),
            min_balance_cents: config.min_balance_cents,
            max_balance_cents: config.max_balance_cents,
            max_deposit_cents: config.max_deposit_cents,
            deposit_presets_cents: config.deposit_presets_cents.clone(),
            self_registration: config.self_registration,
            idle_timeout_secs: config.idle_timeout_secs,
            undo_window_secs: config.undo_window_secs,
            default_language: config.default_language.clone(),
            stored: HashSet::new(),
        }
    }

    /// Ist der Wert im Admin-Bereich gespeichert (statt aus der Konfiguration)?
    pub fn is_stored(&self, key: &str) -> bool {
        self.stored.contains(key)
    }

    /// Schnellwahl-Beträge als Text, z. B. „5 10 20“ (Format wie in der Konfiguration).
    pub fn presets_text(&self) -> String {
        self.deposit_presets_cents
            .iter()
            .map(|&c| short_amount(c))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Ein gespeicherter Wert als Text (für die Datenbank).
    fn value_of(&self, key: &str) -> String {
        match key {
            keys::CLUB_NAME => self.club_name.clone(),
            keys::MIN_BALANCE => self.min_balance_cents.to_string(),
            keys::MAX_BALANCE => self
                .max_balance_cents
                .map(|c| c.to_string())
                .unwrap_or_default(),
            keys::MAX_DEPOSIT => self.max_deposit_cents.to_string(),
            keys::DEPOSIT_PRESETS => self
                .deposit_presets_cents
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            keys::SELF_REGISTRATION => self.self_registration.to_string(),
            keys::IDLE_TIMEOUT => self.idle_timeout_secs.to_string(),
            keys::UNDO_WINDOW => self.undo_window_secs.to_string(),
            keys::DEFAULT_LANGUAGE => self.default_language.clone(),
            _ => String::new(),
        }
    }

    /// Übernimmt einen gespeicherten Wert. Unlesbare Werte werden ignoriert
    /// (dann gilt weiter der Wert aus der Konfiguration).
    fn apply(&mut self, key: &str, value: &str) -> bool {
        let ok = match key {
            keys::CLUB_NAME => {
                self.club_name = value.to_string();
                true
            }
            keys::MIN_BALANCE => value.parse().map(|v| self.min_balance_cents = v).is_ok(),
            keys::MAX_BALANCE if value.is_empty() => {
                self.max_balance_cents = None;
                true
            }
            keys::MAX_BALANCE => value
                .parse()
                .map(|v| self.max_balance_cents = Some(v))
                .is_ok(),
            keys::MAX_DEPOSIT => value.parse().map(|v| self.max_deposit_cents = v).is_ok(),
            keys::DEPOSIT_PRESETS => value
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<Vec<i64>, _>>()
                .map(|v| self.deposit_presets_cents = v)
                .is_ok(),
            keys::SELF_REGISTRATION => value.parse().map(|v| self.self_registration = v).is_ok(),
            keys::IDLE_TIMEOUT => value.parse().map(|v| self.idle_timeout_secs = v).is_ok(),
            keys::UNDO_WINDOW => value.parse().map(|v| self.undo_window_secs = v).is_ok(),
            keys::DEFAULT_LANGUAGE => {
                self.default_language = value.to_string();
                true
            }
            _ => false,
        };
        if ok {
            self.stored.insert(key.to_string());
        } else {
            tracing::warn!(
                "Einstellung „{key}“ hat einen ungültigen Wert „{value}“ und wird ignoriert"
            );
        }
        ok
    }
}

/// Betrag ohne unnötige Nachkommastellen: 500 → „5“, 250 → „2.50“.
fn short_amount(cents: i64) -> String {
    if cents % 100 == 0 {
        (cents / 100).to_string()
    } else {
        money::format_plain(cents)
    }
}

/// Lädt die Einstellungen: Konfiguration als Grundlage, gespeicherte Werte darüber.
pub async fn load(pool: &SqlitePool, config: &Config) -> Result<Settings, sqlx::Error> {
    let mut settings = Settings::from_config(config);
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await?;
    for (key, value) in rows {
        settings.apply(&key, &value);
    }
    Ok(settings)
}

/// Speichert alle Werte, die sich gegenüber der Konfiguration unterscheiden
/// oder schon gespeichert waren.
pub async fn save(
    pool: &SqlitePool,
    new: &Settings,
    config: &Config,
    admin_id: i64,
) -> Result<(), sqlx::Error> {
    let defaults = Settings::from_config(config);
    let current = load(pool, config).await?;
    let now = db::now();
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    for key in keys::ALL {
        let value = new.value_of(key);
        // Unveränderte Startwerte nicht speichern, damit Änderungen an der
        // Konfiguration weiter wirken
        if value == defaults.value_of(key) && !current.is_stored(key) {
            continue;
        }
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at, admin_id) VALUES (?, ?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                 updated_at = excluded.updated_at, admin_id = excluded.admin_id",
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .bind(admin_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// Löscht alle gespeicherten Werte: Danach gilt wieder die Konfiguration.
pub async fn reset(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM settings").execute(pool).await?;
    Ok(())
}

/// Fehler bei der Eingabe im Admin-Formular (Übersetzungsschlüssel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsError {
    ClubNameEmpty,
    MinBalancePositive,
    MaxBalanceTooLow,
    MaxDepositInvalid,
    PresetInvalid,
    IdleTimeoutRange,
    UndoWindowRange,
    LanguageUnknown,
    AmountInvalid,
}

impl SettingsError {
    pub fn key(self) -> &'static str {
        match self {
            SettingsError::ClubNameEmpty => "settings-error-club-name",
            SettingsError::MinBalancePositive => "settings-error-min-balance",
            SettingsError::MaxBalanceTooLow => "settings-error-max-balance",
            SettingsError::MaxDepositInvalid => "settings-error-max-deposit",
            SettingsError::PresetInvalid => "settings-error-presets",
            SettingsError::IdleTimeoutRange => "settings-error-idle-timeout",
            SettingsError::UndoWindowRange => "settings-error-undo-window",
            SettingsError::LanguageUnknown => "settings-error-language",
            SettingsError::AmountInvalid => "error-amount-invalid",
        }
    }
}

/// Rohwerte aus dem Admin-Formular.
#[derive(Debug, Default, Clone)]
pub struct SettingsInput {
    pub club_name: String,
    pub min_balance: String,
    pub max_balance: String,
    pub max_deposit: String,
    pub deposit_presets: String,
    pub self_registration: bool,
    pub idle_timeout: String,
    pub undo_window: String,
    pub default_language: String,
}

impl SettingsInput {
    /// Formularwerte aus den aktuellen Einstellungen (zum Anzeigen).
    pub fn from_settings(s: &Settings) -> Self {
        SettingsInput {
            club_name: s.club_name.clone(),
            min_balance: short_amount(s.min_balance_cents),
            max_balance: s.max_balance_cents.map(short_amount).unwrap_or_default(),
            max_deposit: short_amount(s.max_deposit_cents),
            deposit_presets: s.presets_text(),
            self_registration: s.self_registration,
            idle_timeout: s.idle_timeout_secs.to_string(),
            undo_window: s.undo_window_secs.to_string(),
            default_language: s.default_language.clone(),
        }
    }

    /// Prüft die Eingaben und baut daraus neue Einstellungen.
    pub fn validate(
        &self,
        current: &Settings,
        is_language: impl Fn(&str) -> bool,
    ) -> Result<Settings, SettingsError> {
        let amount = |s: &str| money::parse_cents(s).map_err(|_| SettingsError::AmountInvalid);

        let club_name = self
            .club_name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if club_name.is_empty() || club_name.chars().count() > 80 {
            return Err(SettingsError::ClubNameEmpty);
        }
        let min_balance_cents = amount(&self.min_balance)?;
        if min_balance_cents > 0 {
            return Err(SettingsError::MinBalancePositive);
        }
        let max_balance_cents = match self.max_balance.trim() {
            "" => None,
            v => {
                let max = amount(v)?;
                if max <= 0 || max <= min_balance_cents {
                    return Err(SettingsError::MaxBalanceTooLow);
                }
                Some(max)
            }
        };
        let max_deposit_cents = amount(&self.max_deposit)?;
        if max_deposit_cents <= 0 {
            return Err(SettingsError::MaxDepositInvalid);
        }
        let deposit_presets_cents = self
            .deposit_presets
            .split([' ', ';'])
            .filter(|p| !p.trim().is_empty())
            .map(|p| amount(p.trim()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| SettingsError::PresetInvalid)?;
        if deposit_presets_cents.len() > 8
            || deposit_presets_cents
                .iter()
                .any(|&p| p <= 0 || p > max_deposit_cents)
        {
            return Err(SettingsError::PresetInvalid);
        }
        let idle_timeout_secs: u64 = self
            .idle_timeout
            .trim()
            .parse()
            .ok()
            .filter(|v| (10..=3600).contains(v))
            .ok_or(SettingsError::IdleTimeoutRange)?;
        let undo_window_secs: i64 = self
            .undo_window
            .trim()
            .parse()
            .ok()
            .filter(|v| (0..=3600).contains(v))
            .ok_or(SettingsError::UndoWindowRange)?;
        let default_language = self.default_language.trim().to_string();
        if !is_language(&default_language) {
            return Err(SettingsError::LanguageUnknown);
        }

        Ok(Settings {
            club_name,
            min_balance_cents,
            max_balance_cents,
            max_deposit_cents,
            deposit_presets_cents,
            self_registration: self.self_registration,
            idle_timeout_secs,
            undo_window_secs,
            default_language,
            stored: current.stored.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> SettingsInput {
        SettingsInput {
            club_name: "  Hackspace   e. V. ".into(),
            min_balance: "-15".into(),
            max_balance: "50".into(),
            max_deposit: "40,00".into(),
            deposit_presets: "5 10 20".into(),
            self_registration: false,
            idle_timeout: "90".into(),
            undo_window: "30".into(),
            default_language: "en".into(),
        }
    }

    #[test]
    fn validates_input() {
        let current = Settings::from_config(&Config::default());
        let s = input()
            .validate(&current, |l| l == "de" || l == "en")
            .unwrap();
        assert_eq!(s.club_name, "Hackspace e. V.");
        assert_eq!(s.min_balance_cents, -1500);
        assert_eq!(s.max_balance_cents, Some(5000));
        assert_eq!(s.max_deposit_cents, 4000);
        assert_eq!(s.deposit_presets_cents, vec![500, 1000, 2000]);
        assert!(!s.self_registration);
        assert_eq!(s.idle_timeout_secs, 90);
        assert_eq!(s.undo_window_secs, 30);
        assert_eq!(s.default_language, "en");

        let check = |f: fn(&mut SettingsInput)| {
            let mut i = input();
            f(&mut i);
            i.validate(&current, |l| l == "de" || l == "en")
                .unwrap_err()
        };
        assert_eq!(
            check(|i| i.club_name = "  ".into()),
            SettingsError::ClubNameEmpty
        );
        assert_eq!(
            check(|i| i.min_balance = "5".into()),
            SettingsError::MinBalancePositive
        );
        assert_eq!(
            check(|i| i.max_balance = "-20".into()),
            SettingsError::MaxBalanceTooLow
        );
        assert_eq!(
            check(|i| i.max_deposit = "0".into()),
            SettingsError::MaxDepositInvalid
        );
        assert_eq!(
            check(|i| i.deposit_presets = "5 50".into()),
            SettingsError::PresetInvalid
        );
        assert_eq!(
            check(|i| i.idle_timeout = "5".into()),
            SettingsError::IdleTimeoutRange
        );
        assert_eq!(
            check(|i| i.undo_window = "x".into()),
            SettingsError::UndoWindowRange
        );
        assert_eq!(
            check(|i| i.default_language = "xx".into()),
            SettingsError::LanguageUnknown
        );
        // Leeres Höchstguthaben = keine Grenze
        let mut i = input();
        i.max_balance = " ".into();
        assert_eq!(
            i.validate(&current, |_| true).unwrap().max_balance_cents,
            None
        );
    }

    #[tokio::test]
    async fn stored_values_override_config() {
        let pool = db::connect_memory().await.unwrap();
        let config = Config::default();
        let admin = crate::admins::create(&pool, "admin", "geheim123")
            .await
            .unwrap();

        let start = load(&pool, &config).await.unwrap();
        assert_eq!(start.club_name, config.club_name);
        assert!(start.stored.is_empty());

        // Nur geänderte Werte werden gespeichert
        let mut changed = start.clone();
        changed.club_name = "Neuer Name".into();
        changed.max_balance_cents = Some(5000);
        save(&pool, &changed, &config, admin).await.unwrap();
        let loaded = load(&pool, &config).await.unwrap();
        assert_eq!(loaded.club_name, "Neuer Name");
        assert_eq!(loaded.max_balance_cents, Some(5000));
        assert!(loaded.is_stored(keys::CLUB_NAME));
        assert!(
            !loaded.is_stored(keys::MIN_BALANCE),
            "unveränderter Wert bleibt bei der Konfiguration"
        );

        // Eine geänderte Konfiguration wirkt für nicht gespeicherte Werte weiter
        let other = Config {
            min_balance_cents: -2000,
            ..Config::default()
        };
        assert_eq!(load(&pool, &other).await.unwrap().min_balance_cents, -2000);
        assert_eq!(load(&pool, &other).await.unwrap().club_name, "Neuer Name");

        // Zurücksetzen
        reset(&pool).await.unwrap();
        assert_eq!(
            load(&pool, &config).await.unwrap(),
            Settings::from_config(&config)
        );
    }
}
