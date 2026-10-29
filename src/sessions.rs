//! Anmeldesitzungen.
//!
//! Der Browser bekommt nur ein zufälliges Token als Cookie; was dazu gehört
//! (welcher Admin bzw. welcher Kiosk-Benutzer, Ablaufzeit), steht in der
//! Tabelle `sessions`. Sitzungen verlängern sich bei jeder Nutzung.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use crate::{db, password};

/// Wem eine Sitzung gehört.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Admin(i64),
    User(i64),
}

/// Legt eine neue Sitzung an und liefert das Token für das Cookie.
pub async fn create(pool: &SqlitePool, owner: Owner, ttl: Duration) -> Result<String, sqlx::Error> {
    let token = password::random_token(32);
    let (admin_id, user_id) = match owner {
        Owner::Admin(id) => (Some(id), None),
        Owner::User(id) => (None, Some(id)),
    };
    sqlx::query("INSERT INTO sessions (token, admin_id, user_id, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&token)
        .bind(admin_id)
        .bind(user_id)
        .bind(db::to_db_time(Utc::now() + ttl))
        .execute(pool)
        .await?;
    Ok(token)
}

/// Prüft ein Token. Ist die Sitzung gültig, wird sie um `ttl` verlängert.
pub async fn validate(
    pool: &SqlitePool,
    token: &str,
    ttl: Duration,
) -> Result<Option<Owner>, sqlx::Error> {
    let now = Utc::now();
    let row: Option<(Option<i64>, Option<i64>)> =
        sqlx::query_as("SELECT admin_id, user_id FROM sessions WHERE token = ? AND expires_at > ?")
            .bind(token)
            .bind(db::to_db_time(now))
            .fetch_optional(pool)
            .await?;
    let owner = match row {
        Some((Some(admin), _)) => Owner::Admin(admin),
        Some((None, Some(user))) => Owner::User(user),
        _ => return Ok(None),
    };
    sqlx::query("UPDATE sessions SET expires_at = ? WHERE token = ?")
        .bind(db::to_db_time(now + ttl))
        .bind(token)
        .execute(pool)
        .await?;
    Ok(Some(owner))
}

pub async fn delete(pool: &SqlitePool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Löscht abgelaufene Sitzungen (wird regelmäßig im Hintergrund aufgerufen).
pub async fn cleanup(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(db::now())
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
