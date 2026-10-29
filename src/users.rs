//! Kiosk-Benutzer (Mitglieder) und ihre PINs.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use thiserror::Error;

use crate::{db, password};

/// Nach so vielen falschen PIN-Eingaben wird der Benutzer kurz gesperrt.
pub const MAX_PIN_ATTEMPTS: i64 = 5;
/// Dauer der Sperre nach zu vielen Fehlversuchen.
pub const PIN_LOCK_MINUTES: i64 = 5;
/// Höchstlänge eines Benutzernamens (in Zeichen).
pub const MAX_NAME_LEN: usize = 40;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub pin_hash: Option<String>,
    pub balance_cents: i64,
    pub language: Option<String>,
    pub active: bool,
    pub created_at: String,
}

impl User {
    pub fn has_pin(&self) -> bool {
        self.pin_hash.is_some()
    }

    /// Anfangsbuchstabe für die Buchstabenleiste am Kiosk.
    pub fn initial(&self) -> String {
        self.name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserError {
    #[error("Name fehlt")]
    NameEmpty,
    #[error("Name zu lang")]
    NameTooLong,
    #[error("Name bereits vergeben")]
    NameTaken,
    #[error("PIN muss aus 4 bis 8 Ziffern bestehen")]
    PinInvalid,
    #[error("PINs stimmen nicht überein")]
    PinMismatch,
    #[error("Benutzer nicht gefunden")]
    NotFound,
    #[error("Datenbankfehler")]
    Db,
}

impl From<sqlx::Error> for UserError {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &e
            && db.is_unique_violation()
        {
            return UserError::NameTaken;
        }
        tracing::error!("Datenbankfehler: {e}");
        UserError::Db
    }
}

/// Spaltenliste für `SELECT`s auf `users` (als Makro, damit `concat!` daraus
/// einen festen SQL-Text bauen kann).
macro_rules! user_columns {
    () => {
        "id, name, pin_hash, balance_cents, language, active, created_at"
    };
}

/// Prüft und normalisiert einen Benutzernamen.
pub fn validate_name(name: &str) -> Result<String, UserError> {
    // Mehrfache Leerzeichen zusammenfassen
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(UserError::NameEmpty);
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(UserError::NameTooLong);
    }
    Ok(name)
}

/// Prüft eine PIN: 4 bis 8 Ziffern.
pub fn validate_pin(pin: &str) -> Result<(), UserError> {
    if (4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(UserError::PinInvalid)
    }
}

/// Alle aktiven Benutzer, alphabetisch (für die Kiosk-Startseite).
pub async fn list_active(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE active = 1 ORDER BY name COLLATE NOCASE"
    ))
    .fetch_all(pool)
    .await
}

/// Alle Benutzer inkl. deaktivierter (für den Admin-Bereich).
pub async fn list_all(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users ORDER BY active DESC, name COLLATE NOCASE"
    ))
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Legt einen neuen Benutzer an, optional mit PIN.
pub async fn create(pool: &SqlitePool, name: &str, pin: Option<&str>) -> Result<User, UserError> {
    let name = validate_name(name)?;
    let pin_hash = match pin {
        Some(pin) => {
            validate_pin(pin)?;
            Some(password::hash(pin).map_err(|_| UserError::Db)?)
        }
        None => None,
    };
    let now = db::now();
    let user = sqlx::query_as::<_, User>(concat!(
        "INSERT INTO users (name, pin_hash, created_at, updated_at) VALUES (?, ?, ?, ?)
         RETURNING ",
        user_columns!(),
        ""
    ))
    .bind(&name)
    .bind(pin_hash)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(user)
}

/// Setzt oder entfernt (`None`) die PIN eines Benutzers.
pub async fn set_pin(pool: &SqlitePool, id: i64, pin: Option<&str>) -> Result<(), UserError> {
    let pin_hash = match pin {
        Some(pin) => {
            validate_pin(pin)?;
            Some(password::hash(pin).map_err(|_| UserError::Db)?)
        }
        None => None,
    };
    let res = sqlx::query(
        "UPDATE users SET pin_hash = ?, failed_pin_attempts = 0, pin_locked_until = NULL,
                          updated_at = ? WHERE id = ?",
    )
    .bind(pin_hash)
    .bind(db::now())
    .bind(id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(UserError::NotFound);
    }
    Ok(())
}

/// Ergebnis einer PIN-Prüfung.
#[derive(Debug, PartialEq, Eq)]
pub enum PinCheck {
    Ok,
    Wrong,
    /// Zu viele Fehlversuche – Benutzer ist bis zu diesem Zeitpunkt gesperrt
    Locked,
}

/// Prüft die PIN und zählt Fehlversuche. Nach [`MAX_PIN_ATTEMPTS`]
/// Fehlversuchen wird der Benutzer für [`PIN_LOCK_MINUTES`] gesperrt.
pub async fn check_pin(pool: &SqlitePool, id: i64, pin: &str) -> Result<PinCheck, sqlx::Error> {
    let row: Option<(Option<String>, i64, Option<String>)> = sqlx::query_as(
        "SELECT pin_hash, failed_pin_attempts, pin_locked_until FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some((hash, attempts, locked_until)) = row else {
        return Ok(PinCheck::Wrong);
    };
    let Some(hash) = hash else {
        // Ohne PIN gibt es nichts zu prüfen
        return Ok(PinCheck::Ok);
    };
    let now = Utc::now();
    if let Some(until) = locked_until.as_deref().and_then(db::parse_db_time)
        && until > now
    {
        return Ok(PinCheck::Locked);
    }

    if password::verify(pin, &hash) {
        sqlx::query(
            "UPDATE users SET failed_pin_attempts = 0, pin_locked_until = NULL WHERE id = ?",
        )
        .bind(id)
        .execute(pool)
        .await?;
        return Ok(PinCheck::Ok);
    }

    let attempts = attempts + 1;
    if attempts >= MAX_PIN_ATTEMPTS {
        let until = db::to_db_time(now + Duration::minutes(PIN_LOCK_MINUTES));
        sqlx::query("UPDATE users SET failed_pin_attempts = 0, pin_locked_until = ? WHERE id = ?")
            .bind(until)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(PinCheck::Locked)
    } else {
        sqlx::query("UPDATE users SET failed_pin_attempts = ? WHERE id = ?")
            .bind(attempts)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(PinCheck::Wrong)
    }
}

/// Speichert die bevorzugte Sprache (`None` = Sprache des Geräts verwenden).
pub async fn set_language(
    pool: &SqlitePool,
    id: i64,
    lang: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET language = ?, updated_at = ? WHERE id = ?")
        .bind(lang)
        .bind(db::now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Stammdaten durch einen Admin ändern.
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    active: bool,
    lang: Option<&str>,
) -> Result<(), UserError> {
    let name = validate_name(name)?;
    let res = sqlx::query(
        "UPDATE users SET name = ?, active = ?, language = ?, updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(active)
    .bind(lang)
    .bind(db::now())
    .bind(id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(UserError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation() {
        assert_eq!(validate_name("  Anna   Lena "), Ok("Anna Lena".into()));
        assert_eq!(validate_name("   "), Err(UserError::NameEmpty));
        assert_eq!(validate_name(&"x".repeat(41)), Err(UserError::NameTooLong));
    }

    #[test]
    fn pin_validation() {
        assert!(validate_pin("1234").is_ok());
        assert!(validate_pin("12345678").is_ok());
        assert!(validate_pin("123").is_err());
        assert!(validate_pin("123456789").is_err());
        assert!(validate_pin("12a4").is_err());
    }

    #[tokio::test]
    async fn pin_lockout() {
        let pool = db::connect_memory().await.unwrap();
        let u = create(&pool, "Paula", Some("1234")).await.unwrap();
        for _ in 0..MAX_PIN_ATTEMPTS - 1 {
            assert_eq!(
                check_pin(&pool, u.id, "0000").await.unwrap(),
                PinCheck::Wrong
            );
        }
        assert_eq!(
            check_pin(&pool, u.id, "0000").await.unwrap(),
            PinCheck::Locked
        );
        // Auch die richtige PIN hilft während der Sperre nicht
        assert_eq!(
            check_pin(&pool, u.id, "1234").await.unwrap(),
            PinCheck::Locked
        );
        // Admin setzt PIN zurück -> Sperre aufgehoben
        set_pin(&pool, u.id, Some("4321")).await.unwrap();
        assert_eq!(check_pin(&pool, u.id, "4321").await.unwrap(), PinCheck::Ok);
    }

    #[tokio::test]
    async fn duplicate_names_rejected() {
        let pool = db::connect_memory().await.unwrap();
        create(&pool, "Max", None).await.unwrap();
        assert_eq!(
            create(&pool, "max", None).await.unwrap_err(),
            UserError::NameTaken
        );
    }
}
