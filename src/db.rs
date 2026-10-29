//! Datenbankverbindung, Migrationen und Zeitstempel.

use std::{path::Path, str::FromStr, time::Duration};

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

/// Öffnet (bzw. erstellt) die SQLite-Datei und spielt alle Migrationen ein.
pub async fn connect(path: &Path) -> Result<SqlitePool> {
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Verzeichnis {} anlegen", dir.display()))?;
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // WAL: Lesen und Schreiben gleichzeitig, robust bei Stromausfall
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .with_context(|| format!("Datenbank {} öffnen", path.display()))?;
    migrate(&pool).await?;
    Ok(pool)
}

/// Erstellt eine leere In-Memory-Datenbank (für Tests).
pub async fn connect_memory() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
    // Eine einzige Verbindung, sonst hätte jede Verbindung ihre eigene leere Datenbank.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// Spielt die SQL-Dateien aus `migrations/` ein (sind in die Binärdatei eingebettet).
pub async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("Datenbank-Migrationen")?;
    Ok(())
}

/// Erstellt eine konsistente Kopie der Datenbank, während der Server läuft.
pub async fn backup(pool: &SqlitePool, target: &Path) -> Result<()> {
    if target.exists() {
        anyhow::bail!("{} existiert bereits", target.display());
    }
    let target = target
        .to_str()
        .context("Pfad enthält ungültige Zeichen")?
        .to_string();
    sqlx::query("VACUUM INTO ?")
        .bind(target)
        .execute(pool)
        .await
        .context("VACUUM INTO")?;
    Ok(())
}

/// Wandelt einen Zeitpunkt in das Speicherformat `YYYY-MM-DDTHH:MM:SSZ` um.
pub fn to_db_time(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Der aktuelle Zeitpunkt im Speicherformat.
pub fn now() -> String {
    to_db_time(Utc::now())
}

/// Liest einen gespeicherten Zeitstempel wieder ein.
pub fn parse_db_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}
