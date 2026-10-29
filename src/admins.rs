//! Admin-Zugänge.

use sqlx::SqlitePool;

use crate::{db, password};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Admin {
    pub id: i64,
    pub username: String,
    pub active: bool,
    pub created_at: String,
}

/// Mindestlänge für Admin-Passwörter.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Legt beim ersten Start den Zugang `admin` an, falls es noch keinen Admin gibt.
///
/// Ist `CRATETAB_ADMIN_PASSWORD` nicht gesetzt, wird ein zufälliges Passwort
/// erzeugt und einmalig ins Log geschrieben.
pub async fn ensure_initial_admin(
    pool: &SqlitePool,
    initial_password: Option<&str>,
) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(());
    }
    let pw = match initial_password {
        Some(pw) => pw.to_string(),
        None => {
            let pw = password::random_token(8);
            tracing::warn!(
                "Kein CRATETAB_ADMIN_PASSWORD gesetzt. Admin-Zugang: Benutzer „admin“, Passwort „{pw}“ – bitte nach dem ersten Login ändern!"
            );
            pw
        }
    };
    create(pool, "admin", &pw).await?;
    tracing::info!("Admin-Zugang „admin“ angelegt");
    Ok(())
}

pub async fn create(pool: &SqlitePool, username: &str, pw: &str) -> anyhow::Result<i64> {
    let username = username.trim();
    anyhow::ensure!(!username.is_empty(), "Benutzername fehlt");
    let hash = password::hash(pw)?;
    let id = sqlx::query_scalar(
        "INSERT INTO admins (username, password_hash, created_at) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(username)
    .bind(hash)
    .bind(db::now())
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Prüft Benutzername und Passwort; liefert den Admin bei Erfolg.
pub async fn login(
    pool: &SqlitePool,
    username: &str,
    pw: &str,
) -> Result<Option<Admin>, sqlx::Error> {
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, password_hash FROM admins WHERE username = ? AND active = 1")
            .bind(username.trim())
            .fetch_optional(pool)
            .await?;
    let Some((id, hash)) = row else {
        // Trotzdem einmal hashen, damit man an der Antwortzeit nicht erkennt,
        // ob es den Benutzernamen gibt.
        static DUMMY: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| password::hash("dummy").unwrap_or_default());
        let _ = password::verify(pw, &DUMMY);
        return Ok(None);
    };
    if !password::verify(pw, &hash) {
        return Ok(None);
    }
    get(pool, id).await
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Admin>, sqlx::Error> {
    sqlx::query_as("SELECT id, username, active, created_at FROM admins WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Admin>, sqlx::Error> {
    sqlx::query_as("SELECT id, username, active, created_at FROM admins ORDER BY username")
        .fetch_all(pool)
        .await
}

pub async fn set_password(pool: &SqlitePool, id: i64, pw: &str) -> anyhow::Result<()> {
    let hash = password::hash(pw)?;
    sqlx::query("UPDATE admins SET password_hash = ? WHERE id = ?")
        .bind(hash)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_active(pool: &SqlitePool, id: i64, active: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE admins SET active = ? WHERE id = ?")
        .bind(active)
        .bind(id)
        .execute(pool)
        .await?;
    if !active {
        sqlx::query("DELETE FROM sessions WHERE admin_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}
