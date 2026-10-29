//! Buchungslogik – das Herz der Getränkekasse.
//!
//! Regeln:
//! * Jede Geldbewegung ist eine Zeile in `transactions`. Zeilen werden nie
//!   gelöscht oder geändert.
//! * `users.balance_cents` wird in **derselben** Datenbank-Transaktion wie die
//!   Buchung angepasst. Es ist damit immer gleich der Summe aller Buchungen
//!   des Benutzers; [`verify_balances`] prüft das nach.
//! * Ein Storno ist eine neue Buchung vom Typ `reversal` mit umgekehrtem
//!   Vorzeichen, die per `reverses_id` auf die Originalbuchung zeigt.
//!
//! Alle schreibenden Funktionen öffnen ihre Transaktion mit `BEGIN IMMEDIATE`.
//! Damit sperrt SQLite sofort zum Schreiben, und zwei gleichzeitige Käufe
//! können nicht beide denselben alten Kontostand sehen.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{SqliteConnection, SqlitePool};
use thiserror::Error;

use crate::db;

/// Art einer Buchung. Wird in der Datenbank als Text gespeichert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TxKind {
    /// Kauf eines Artikels (negativer Betrag)
    Purchase,
    /// Aufladung des Guthabens (positiver Betrag)
    Deposit,
    /// Manuelle Korrektur durch einen Admin (mit Begründung)
    Correction,
    /// Storno einer anderen Buchung (Gegenbuchung)
    Reversal,
}

impl TxKind {
    pub const ALL: [TxKind; 4] = [
        TxKind::Purchase,
        TxKind::Deposit,
        TxKind::Correction,
        TxKind::Reversal,
    ];

    /// Name in der Datenbank, z. B. `"purchase"`
    pub fn as_str(self) -> &'static str {
        match self {
            TxKind::Purchase => "purchase",
            TxKind::Deposit => "deposit",
            TxKind::Correction => "correction",
            TxKind::Reversal => "reversal",
        }
    }

    pub fn parse(s: &str) -> Option<TxKind> {
        TxKind::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// Eine Buchung, wie sie in der Datenbank steht.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Transaction {
    pub id: i64,
    pub user_id: i64,
    pub amount_cents: i64,
    pub kind: TxKind,
    pub article_id: Option<i64>,
    pub article_name: Option<String>,
    pub note: Option<String>,
    pub admin_id: Option<i64>,
    pub reverses_id: Option<i64>,
    pub created_at: String,
}

/// Alles, was bei einer Buchung schiefgehen kann.
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Benutzer nicht gefunden")]
    UserNotFound,
    #[error("Benutzer ist deaktiviert")]
    UserInactive,
    #[error("Artikel nicht gefunden")]
    ArticleNotFound,
    #[error("Artikel ist nicht verfügbar")]
    ArticleInactive,
    #[error("Guthaben reicht nicht (Kontostand {balance}, Minimum {min})")]
    InsufficientBalance { balance: i64, min: i64 },
    #[error("ungültiger Betrag")]
    InvalidAmount,
    #[error("Betrag über dem erlaubten Höchstbetrag")]
    AmountTooLarge,
    #[error("Begründung fehlt")]
    ReasonRequired,
    #[error("keine Buchung zum Rückgängigmachen")]
    NothingToUndo,
    #[error("Buchung ist bereits storniert")]
    AlreadyReversed,
    #[error("diese Buchung kann nicht storniert werden")]
    NotReversible,
    #[error("Buchung nicht gefunden")]
    TransactionNotFound,
    #[error("Datenbankfehler: {0}")]
    Db(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, LedgerError>;

/// Interne Beschreibung einer neuen Buchung.
struct NewTx<'a> {
    user_id: i64,
    amount_cents: i64,
    kind: TxKind,
    article_id: Option<i64>,
    article_name: Option<&'a str>,
    note: Option<&'a str>,
    admin_id: Option<i64>,
    reverses_id: Option<i64>,
    created_at: String,
    legacy_id: Option<&'a str>,
}

/// Schreibt eine Buchung und passt das Guthaben an. Muss innerhalb einer
/// offenen Datenbank-Transaktion aufgerufen werden.
async fn insert_tx(conn: &mut SqliteConnection, tx: NewTx<'_>) -> Result<Transaction> {
    let updated = sqlx::query(
        "UPDATE users SET balance_cents = balance_cents + ?, updated_at = ? WHERE id = ?",
    )
    .bind(tx.amount_cents)
    .bind(&tx.created_at)
    .bind(tx.user_id)
    .execute(&mut *conn)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(LedgerError::UserNotFound);
    }

    let row = sqlx::query_as::<_, Transaction>(
        "INSERT INTO transactions
            (user_id, amount_cents, kind, article_id, article_name, note, admin_id, reverses_id,
             created_at, legacy_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id, user_id, amount_cents, kind, article_id, article_name, note,
                   admin_id, reverses_id, created_at",
    )
    .bind(tx.user_id)
    .bind(tx.amount_cents)
    .bind(tx.kind)
    .bind(tx.article_id)
    .bind(tx.article_name)
    .bind(tx.note)
    .bind(tx.admin_id)
    .bind(tx.reverses_id)
    .bind(&tx.created_at)
    .bind(tx.legacy_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(row)
}

/// Lädt Kontostand und Aktiv-Status eines Benutzers.
async fn load_user(conn: &mut SqliteConnection, user_id: i64) -> Result<(i64, bool)> {
    let row: Option<(i64, bool)> =
        sqlx::query_as("SELECT balance_cents, active FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?;
    row.ok_or(LedgerError::UserNotFound)
}

async fn begin(pool: &SqlitePool) -> Result<sqlx::Transaction<'static, sqlx::Sqlite>> {
    Ok(pool.begin_with("BEGIN IMMEDIATE").await?)
}

/// Bucht den Kauf eines Artikels. Schlägt fehl, wenn der Kontostand danach
/// unter `min_balance_cents` fallen würde.
pub async fn purchase(
    pool: &SqlitePool,
    user_id: i64,
    article_id: i64,
    min_balance_cents: i64,
) -> Result<Transaction> {
    let mut tx = begin(pool).await?;

    let (balance, active) = load_user(&mut tx, user_id).await?;
    if !active {
        return Err(LedgerError::UserInactive);
    }

    let article: Option<(String, i64, bool)> =
        sqlx::query_as("SELECT name, price_cents, active FROM articles WHERE id = ?")
            .bind(article_id)
            .fetch_optional(&mut *tx)
            .await?;
    let (name, price, article_active) = article.ok_or(LedgerError::ArticleNotFound)?;
    if !article_active {
        return Err(LedgerError::ArticleInactive);
    }
    if balance - price < min_balance_cents {
        return Err(LedgerError::InsufficientBalance {
            balance,
            min: min_balance_cents,
        });
    }

    let row = insert_tx(
        &mut tx,
        NewTx {
            user_id,
            amount_cents: -price,
            kind: TxKind::Purchase,
            article_id: Some(article_id),
            article_name: Some(&name),
            note: None,
            admin_id: None,
            reverses_id: None,
            created_at: db::now(),
            legacy_id: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

/// Bucht eine Aufladung am Kiosk (Vertrauensbasis: Geld kommt in die Kasse).
pub async fn deposit(
    pool: &SqlitePool,
    user_id: i64,
    amount_cents: i64,
    max_deposit_cents: i64,
) -> Result<Transaction> {
    if amount_cents <= 0 {
        return Err(LedgerError::InvalidAmount);
    }
    if amount_cents > max_deposit_cents {
        return Err(LedgerError::AmountTooLarge);
    }
    let mut tx = begin(pool).await?;
    let (_, active) = load_user(&mut tx, user_id).await?;
    if !active {
        return Err(LedgerError::UserInactive);
    }
    let row = insert_tx(
        &mut tx,
        NewTx {
            user_id,
            amount_cents,
            kind: TxKind::Deposit,
            article_id: None,
            article_name: None,
            note: None,
            admin_id: None,
            reverses_id: None,
            created_at: db::now(),
            legacy_id: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

/// Manuelle Korrektur durch einen Admin. Der Betrag darf positiv oder negativ
/// sein, eine Begründung ist Pflicht. Das Minimalguthaben gilt hier nicht.
pub async fn correction(
    pool: &SqlitePool,
    user_id: i64,
    amount_cents: i64,
    reason: &str,
    admin_id: i64,
) -> Result<Transaction> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(LedgerError::ReasonRequired);
    }
    if amount_cents == 0 {
        return Err(LedgerError::InvalidAmount);
    }
    let mut tx = begin(pool).await?;
    load_user(&mut tx, user_id).await?;
    let row = insert_tx(
        &mut tx,
        NewTx {
            user_id,
            amount_cents,
            kind: TxKind::Correction,
            article_id: None,
            article_name: None,
            note: Some(reason),
            admin_id: Some(admin_id),
            reverses_id: None,
            created_at: db::now(),
            legacy_id: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

/// Storniert eine Buchung durch eine Gegenbuchung (innerhalb einer offenen Transaktion).
async fn reverse_in(
    conn: &mut SqliteConnection,
    original: &Transaction,
    admin_id: Option<i64>,
    note: Option<&str>,
) -> Result<Transaction> {
    if original.kind == TxKind::Reversal {
        return Err(LedgerError::NotReversible);
    }
    let already: Option<i64> =
        sqlx::query_scalar("SELECT id FROM transactions WHERE reverses_id = ?")
            .bind(original.id)
            .fetch_optional(&mut *conn)
            .await?;
    if already.is_some() {
        return Err(LedgerError::AlreadyReversed);
    }
    insert_tx(
        conn,
        NewTx {
            user_id: original.user_id,
            amount_cents: -original.amount_cents,
            kind: TxKind::Reversal,
            // Artikel übernehmen, damit die Statistik Käufe und Stornos verrechnen kann
            article_id: original.article_id,
            article_name: original.article_name.as_deref(),
            note,
            admin_id,
            reverses_id: Some(original.id),
            created_at: db::now(),
            legacy_id: None,
        },
    )
    .await
}

async fn load_tx(conn: &mut SqliteConnection, id: i64) -> Result<Option<Transaction>> {
    Ok(sqlx::query_as::<_, Transaction>(
        "SELECT id, user_id, amount_cents, kind, article_id, article_name, note,
                admin_id, reverses_id, created_at
         FROM transactions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?)
}

/// Sucht die Buchung, die der Benutzer am Kiosk gerade rückgängig machen dürfte:
/// seine allerletzte Buchung, falls sie ein Kauf oder eine Aufladung ist,
/// noch nicht storniert wurde und höchstens `window_secs` alt ist.
async fn undoable_in(
    conn: &mut SqliteConnection,
    user_id: i64,
    window_secs: i64,
    now: DateTime<Utc>,
) -> Result<Option<Transaction>> {
    let last = sqlx::query_as::<_, Transaction>(
        "SELECT id, user_id, amount_cents, kind, article_id, article_name, note,
                admin_id, reverses_id, created_at
         FROM transactions WHERE user_id = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(last) = last else { return Ok(None) };
    if !matches!(last.kind, TxKind::Purchase | TxKind::Deposit) {
        return Ok(None);
    }
    let Some(created) = db::parse_db_time(&last.created_at) else {
        return Ok(None);
    };
    let age = (now - created).num_seconds();
    if age > window_secs {
        return Ok(None);
    }
    Ok(Some(last))
}

/// Liefert die Buchung, die gerade rückgängig gemacht werden könnte (für die Anzeige).
pub async fn last_undoable(
    pool: &SqlitePool,
    user_id: i64,
    window_secs: i64,
    now: DateTime<Utc>,
) -> Result<Option<Transaction>> {
    let mut conn = pool.acquire().await?;
    undoable_in(&mut conn, user_id, window_secs, now).await
}

/// Macht die letzte Kiosk-Buchung des Benutzers rückgängig (Storno-Gegenbuchung).
pub async fn undo_last(
    pool: &SqlitePool,
    user_id: i64,
    window_secs: i64,
    now: DateTime<Utc>,
) -> Result<Transaction> {
    let mut tx = begin(pool).await?;
    let original = undoable_in(&mut tx, user_id, window_secs, now)
        .await?
        .ok_or(LedgerError::NothingToUndo)?;
    let row = reverse_in(&mut tx, &original, None, None).await?;
    tx.commit().await?;
    Ok(row)
}

/// Storniert eine beliebige Buchung im Admin-Bereich.
pub async fn reverse_by_admin(
    pool: &SqlitePool,
    transaction_id: i64,
    admin_id: i64,
    note: Option<&str>,
) -> Result<Transaction> {
    let mut tx = begin(pool).await?;
    let original = load_tx(&mut tx, transaction_id)
        .await?
        .ok_or(LedgerError::TransactionNotFound)?;
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    let row = reverse_in(&mut tx, &original, Some(admin_id), note).await?;
    tx.commit().await?;
    Ok(row)
}

/// Eine historische Buchung mit vorgegebenem Zeitstempel.
///
/// Gedacht für den späteren Import aus fnordcredit (Original-Zeitstempel und
/// alte IDs bleiben erhalten) und für Demo-Daten. Es gibt keine Prüfung des
/// Minimalguthabens – historische Daten werden so übernommen, wie sie sind.
pub struct ImportedTx<'a> {
    pub user_id: i64,
    pub amount_cents: i64,
    pub kind: TxKind,
    pub article_id: Option<i64>,
    pub article_name: Option<&'a str>,
    pub note: Option<&'a str>,
    /// Zeitpunkt im Datenbankformat, siehe [`db::to_db_time`]
    pub created_at: String,
    /// ID im Altsystem; verhindert doppelten Import (Spalte ist UNIQUE)
    pub legacy_id: Option<&'a str>,
}

/// Schreibt eine historische Buchung innerhalb einer offenen Transaktion.
pub async fn record_imported(
    conn: &mut SqliteConnection,
    tx: ImportedTx<'_>,
) -> Result<Transaction> {
    insert_tx(
        conn,
        NewTx {
            user_id: tx.user_id,
            amount_cents: tx.amount_cents,
            kind: tx.kind,
            article_id: tx.article_id,
            article_name: tx.article_name,
            note: tx.note,
            admin_id: None,
            reverses_id: None,
            created_at: tx.created_at,
            legacy_id: tx.legacy_id,
        },
    )
    .await
}

/// Ergebnis der Guthaben-Prüfung für einen Benutzer.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BalanceMismatch {
    pub user_id: i64,
    pub name: String,
    pub stored_cents: i64,
    pub computed_cents: i64,
}

/// Rechnet alle Guthaben aus den Buchungen nach und liefert Abweichungen.
/// Im Normalfall ist die Liste leer.
pub async fn verify_balances(pool: &SqlitePool) -> Result<Vec<BalanceMismatch>> {
    Ok(sqlx::query_as::<_, BalanceMismatch>(
        "SELECT u.id AS user_id, u.name, u.balance_cents AS stored_cents,
                COALESCE(SUM(t.amount_cents), 0) AS computed_cents
         FROM users u LEFT JOIN transactions t ON t.user_id = u.id
         GROUP BY u.id
         HAVING stored_cents <> computed_cents
         ORDER BY u.name",
    )
    .fetch_all(pool)
    .await?)
}

/// Setzt alle zwischengespeicherten Guthaben auf die Summe der Buchungen.
/// Gibt die Zahl der korrigierten Benutzer zurück.
pub async fn repair_balances(pool: &SqlitePool) -> Result<u64> {
    let res = sqlx::query(
        "UPDATE users SET balance_cents =
            (SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE user_id = users.id)
         WHERE balance_cents <>
            (SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE user_id = users.id)",
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
