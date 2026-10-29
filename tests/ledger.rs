//! Tests der Buchungslogik: Guthaben, Minimalguthaben, Storno, Korrekturen.

use chrono::{Duration, Utc};
use cratetab::{
    articles::{self, ArticleInput},
    db,
    ledger::{self, LedgerError, TxKind},
    users,
};
use sqlx::SqlitePool;

/// Minimalguthaben für die Tests: -10,00 €
const MIN: i64 = -1000;
/// Höchstbetrag einer Aufladung: 100,00 €
const MAX_DEPOSIT: i64 = 10_000;
/// Rückgängig-Fenster in Sekunden
const WINDOW: i64 = 60;

async fn setup() -> (SqlitePool, i64, i64) {
    let pool = db::connect_memory().await.unwrap();
    let user = users::create(&pool, "Testperson", None).await.unwrap();
    let mate = articles::save(
        &pool,
        None,
        &ArticleInput {
            name: "Mate".into(),
            price_cents: 150,
            active: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    (pool, user.id, mate)
}

async fn balance(pool: &SqlitePool, user: i64) -> i64 {
    users::get(pool, user).await.unwrap().unwrap().balance_cents
}

#[tokio::test]
async fn deposit_and_purchase_change_balance() {
    let (pool, user, mate) = setup().await;
    ledger::deposit(&pool, user, 1000, MAX_DEPOSIT)
        .await
        .unwrap();
    assert_eq!(balance(&pool, user).await, 1000);

    let tx = ledger::purchase(&pool, user, mate, MIN).await.unwrap();
    assert_eq!(tx.kind, TxKind::Purchase);
    assert_eq!(tx.amount_cents, -150);
    assert_eq!(tx.article_name.as_deref(), Some("Mate"));
    assert_eq!(balance(&pool, user).await, 850);

    assert!(ledger::verify_balances(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn minimum_balance_is_enforced() {
    let (pool, user, mate) = setup().await;
    // 0 € Guthaben, Minimum -10 €, Preis 1,50 € -> 6 Käufe gehen (-9,00 €)
    for _ in 0..6 {
        ledger::purchase(&pool, user, mate, MIN).await.unwrap();
    }
    assert_eq!(balance(&pool, user).await, -900);
    // der 7. würde -10,50 € ergeben
    let err = ledger::purchase(&pool, user, mate, MIN).await.unwrap_err();
    assert!(matches!(
        err,
        LedgerError::InsufficientBalance {
            balance: -900,
            min: MIN
        }
    ));
    assert_eq!(
        balance(&pool, user).await,
        -900,
        "fehlgeschlagener Kauf ändert nichts"
    );

    // genau auf dem Minimum landen ist erlaubt
    let exact = articles::save(
        &pool,
        None,
        &ArticleInput {
            name: "Kaffee".into(),
            price_cents: 100,
            active: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    ledger::purchase(&pool, user, exact, MIN).await.unwrap();
    assert_eq!(balance(&pool, user).await, -1000);

    // Minimum 0: ohne Guthaben kein Kauf
    let other = users::create(&pool, "Ohne Kredit", None).await.unwrap();
    assert!(matches!(
        ledger::purchase(&pool, other.id, mate, 0).await,
        Err(LedgerError::InsufficientBalance { .. })
    ));
}

#[tokio::test]
async fn deposit_limits() {
    let (pool, user, _) = setup().await;
    assert!(matches!(
        ledger::deposit(&pool, user, 0, MAX_DEPOSIT).await,
        Err(LedgerError::InvalidAmount)
    ));
    assert!(matches!(
        ledger::deposit(&pool, user, -500, MAX_DEPOSIT).await,
        Err(LedgerError::InvalidAmount)
    ));
    assert!(matches!(
        ledger::deposit(&pool, user, MAX_DEPOSIT + 1, MAX_DEPOSIT).await,
        Err(LedgerError::AmountTooLarge)
    ));
    ledger::deposit(&pool, user, MAX_DEPOSIT, MAX_DEPOSIT)
        .await
        .unwrap();
    assert_eq!(balance(&pool, user).await, MAX_DEPOSIT);
}

#[tokio::test]
async fn inactive_user_and_article_cannot_book() {
    let (pool, user, mate) = setup().await;
    // Artikel deaktivieren
    articles::save(
        &pool,
        Some(mate),
        &ArticleInput {
            name: "Mate".into(),
            price_cents: 150,
            active: false,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        ledger::purchase(&pool, user, mate, MIN).await,
        Err(LedgerError::ArticleInactive)
    ));
    assert!(matches!(
        ledger::purchase(&pool, user, 9999, MIN).await,
        Err(LedgerError::ArticleNotFound)
    ));

    users::update(&pool, user, "Testperson", false, None)
        .await
        .unwrap();
    assert!(matches!(
        ledger::deposit(&pool, user, 500, MAX_DEPOSIT).await,
        Err(LedgerError::UserInactive)
    ));
}

#[tokio::test]
async fn undo_creates_reversal_within_window() {
    let (pool, user, mate) = setup().await;
    ledger::deposit(&pool, user, 500, MAX_DEPOSIT)
        .await
        .unwrap();
    let purchase = ledger::purchase(&pool, user, mate, MIN).await.unwrap();
    assert_eq!(balance(&pool, user).await, 350);

    let now = Utc::now();
    let undoable = ledger::last_undoable(&pool, user, WINDOW, now)
        .await
        .unwrap();
    assert_eq!(undoable.map(|t| t.id), Some(purchase.id));

    let reversal = ledger::undo_last(&pool, user, WINDOW, now).await.unwrap();
    assert_eq!(reversal.kind, TxKind::Reversal);
    assert_eq!(reversal.amount_cents, 150);
    assert_eq!(reversal.reverses_id, Some(purchase.id));
    assert_eq!(
        reversal.article_id,
        Some(mate),
        "Storno behält den Artikel für die Statistik"
    );
    assert_eq!(balance(&pool, user).await, 500);

    // Die letzte Buchung ist jetzt ein Storno -> nichts mehr rückgängig zu machen
    assert!(matches!(
        ledger::undo_last(&pool, user, WINDOW, now).await,
        Err(LedgerError::NothingToUndo)
    ));

    // Buchungen werden nie gelöscht: Kauf und Storno sind beide noch da
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE user_id = ?")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 3);
    assert!(ledger::verify_balances(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn undo_expires_after_window() {
    let (pool, user, mate) = setup().await;
    ledger::purchase(&pool, user, mate, MIN).await.unwrap();
    let later = Utc::now() + Duration::seconds(WINDOW + 5);
    assert!(
        ledger::last_undoable(&pool, user, WINDOW, later)
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        ledger::undo_last(&pool, user, WINDOW, later).await,
        Err(LedgerError::NothingToUndo)
    ));
    assert_eq!(balance(&pool, user).await, -150);
}

#[tokio::test]
async fn undo_of_deposit() {
    let (pool, user, _) = setup().await;
    ledger::deposit(&pool, user, 2000, MAX_DEPOSIT)
        .await
        .unwrap();
    let r = ledger::undo_last(&pool, user, WINDOW, Utc::now())
        .await
        .unwrap();
    assert_eq!(r.amount_cents, -2000);
    assert_eq!(balance(&pool, user).await, 0);
}

#[tokio::test]
async fn corrections_need_reason_and_ignore_minimum() {
    let (pool, user, _) = setup().await;
    let admin = cratetab::admins::create(&pool, "kassenwart", "geheim123")
        .await
        .unwrap();

    assert!(matches!(
        ledger::correction(&pool, user, 500, "   ", admin).await,
        Err(LedgerError::ReasonRequired)
    ));
    assert!(matches!(
        ledger::correction(&pool, user, 0, "Null", admin).await,
        Err(LedgerError::InvalidAmount)
    ));

    let tx = ledger::correction(&pool, user, -5000, "Kasse gezählt, Fehlbetrag", admin)
        .await
        .unwrap();
    assert_eq!(tx.kind, TxKind::Correction);
    assert_eq!(tx.admin_id, Some(admin));
    assert_eq!(tx.note.as_deref(), Some("Kasse gezählt, Fehlbetrag"));
    assert_eq!(
        balance(&pool, user).await,
        -5000,
        "Korrekturen dürfen unter das Minimum gehen"
    );

    // Korrekturen kann der Benutzer am Kiosk nicht rückgängig machen
    assert!(matches!(
        ledger::undo_last(&pool, user, WINDOW, Utc::now()).await,
        Err(LedgerError::NothingToUndo)
    ));
}

#[tokio::test]
async fn admin_reversal_once_only() {
    let (pool, user, mate) = setup().await;
    let admin = cratetab::admins::create(&pool, "admin2", "geheim123")
        .await
        .unwrap();
    let p = ledger::purchase(&pool, user, mate, MIN).await.unwrap();

    let r = ledger::reverse_by_admin(&pool, p.id, admin, Some("falsch gebucht"))
        .await
        .unwrap();
    assert_eq!(r.admin_id, Some(admin));
    assert_eq!(r.note.as_deref(), Some("falsch gebucht"));
    assert_eq!(balance(&pool, user).await, 0);

    assert!(matches!(
        ledger::reverse_by_admin(&pool, p.id, admin, None).await,
        Err(LedgerError::AlreadyReversed)
    ));
    // Ein Storno selbst kann nicht storniert werden
    assert!(matches!(
        ledger::reverse_by_admin(&pool, r.id, admin, None).await,
        Err(LedgerError::NotReversible)
    ));
    assert!(matches!(
        ledger::reverse_by_admin(&pool, 4242, admin, None).await,
        Err(LedgerError::TransactionNotFound)
    ));
}

#[tokio::test]
async fn balances_are_recomputable() {
    let (pool, user, mate) = setup().await;
    ledger::deposit(&pool, user, 1000, MAX_DEPOSIT)
        .await
        .unwrap();
    ledger::purchase(&pool, user, mate, MIN).await.unwrap();
    assert!(ledger::verify_balances(&pool).await.unwrap().is_empty());

    // Zwischengespeichertes Guthaben absichtlich verfälschen …
    sqlx::query("UPDATE users SET balance_cents = 12345 WHERE id = ?")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let mismatches = ledger::verify_balances(&pool).await.unwrap();
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].stored_cents, 12345);
    assert_eq!(mismatches[0].computed_cents, 850);

    // … und aus den Buchungen reparieren
    assert_eq!(ledger::repair_balances(&pool).await.unwrap(), 1);
    assert_eq!(balance(&pool, user).await, 850);
}

#[tokio::test]
async fn concurrent_purchases_respect_minimum() {
    // Datei-Datenbank, damit mehrere Verbindungen gleichzeitig schreiben können
    let dir = std::env::temp_dir().join(format!("cratetab-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("concurrent.db");
    let _ = std::fs::remove_file(&path);
    let pool = db::connect(&path).await.unwrap();
    let user = users::create(&pool, "Parallel", None).await.unwrap().id;
    let mate = articles::save(
        &pool,
        None,
        &ArticleInput {
            name: "Mate".into(),
            price_cents: 100,
            active: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // 20 gleichzeitige Käufe à 1 €, erlaubt sind nur 10 (Minimum -10 €)
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let pool = pool.clone();
            tokio::spawn(async move { ledger::purchase(&pool, user, mate, MIN).await })
        })
        .collect();
    let mut ok = 0;
    for h in handles {
        if h.await.unwrap().is_ok() {
            ok += 1;
        }
    }
    assert_eq!(ok, 10);
    assert_eq!(balance(&pool, user).await, -1000);
    assert!(ledger::verify_balances(&pool).await.unwrap().is_empty());
    pool.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}
