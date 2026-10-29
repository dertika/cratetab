//! # Crate Tab
//!
//! Die Getränkekasse für euren Verein – geschrieben in Rust 🦀
//!
//! Überblick über die Module:
//!
//! | Modul        | Aufgabe                                                   |
//! |--------------|-----------------------------------------------------------|
//! | [`config`]   | Einstellungen aus Umgebungsvariablen                      |
//! | [`db`]       | SQLite-Verbindung, Migrationen, Backup, Zeitstempel       |
//! | [`money`]    | Beträge in Cent einlesen und formatieren                  |
//! | [`ledger`]   | Buchungslogik: Kauf, Aufladung, Korrektur, Storno         |
//! | [`users`]    | Mitglieder und PINs                                       |
//! | [`articles`] | Artikel, Übersetzungen, Barcodes, Bilder                  |
//! | [`admins`]   | Admin-Zugänge                                             |
//! | [`sessions`] | Anmeldesitzungen (Cookies)                                |
//! | [`password`] | argon2-Hashing                                            |
//! | [`i18n`]     | Übersetzungen (Fluent) und sprachabhängige Formate        |
//! | [`assets`]   | eingebettete statische Dateien                            |
//! | [`web`]      | HTTP-Routen und Seiten                                    |
//! | [`seed`]     | Demo-Daten                                                |

pub mod admins;
pub mod articles;
pub mod assets;
pub mod config;
pub mod db;
pub mod i18n;
pub mod ledger;
pub mod money;
pub mod password;
pub mod seed;
pub mod sessions;
pub mod users;
pub mod web;

use std::sync::Arc;

/// Baut den Anwendungszustand: Datenbank öffnen, Sprachen laden, Admin anlegen.
pub async fn build_state(config: config::Config) -> anyhow::Result<web::AppState> {
    let pool = db::connect(&config.database_path).await?;
    state_with_pool(config, pool).await
}

/// Wie [`build_state`], aber mit einer vorhandenen Datenbankverbindung (für Tests).
pub async fn state_with_pool(
    config: config::Config,
    pool: sqlx::SqlitePool,
) -> anyhow::Result<web::AppState> {
    let locales = i18n::Locales::load(
        &config.default_language,
        &config.currency_symbol,
        config.timezone,
        config.locales_dir.as_deref(),
    )?;
    for problem in locales.check_completeness() {
        tracing::warn!("Übersetzung unvollständig: {problem}");
    }
    admins::ensure_initial_admin(&pool, config.admin_initial_password.as_deref()).await?;
    if config.seed_demo {
        seed::seed_if_empty(&pool).await?;
    }
    Ok(web::AppState {
        pool,
        config: Arc::new(config),
        locales: Arc::new(locales),
    })
}
