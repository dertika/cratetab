//! Demo-Daten: ein paar Mitglieder, Artikel mit Barcodes und eine kleine
//! Buchungshistorie, damit man die Oberfläche sofort ausprobieren kann.
//!
//! Aktivieren mit `CRATETAB_SEED_DEMO=true` (nur wenn noch keine Benutzer
//! existieren) oder einmalig mit `cratetab seed-demo`.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use crate::{
    articles::{self, ArticleInput},
    db,
    ledger::{self, ImportedTx, TxKind},
    users,
};

/// Demo-Artikel: (Name DE, Name EN, Name FR, Preis in Cent, Barcodes)
const ARTICLES: &[(&str, &str, &str, i64, &[&str])] = &[
    (
        "Club-Mate",
        "Club-Mate",
        "Club-Mate",
        150,
        &["4029764001807", "2000000000015"],
    ),
    ("Cola", "Cola", "Cola", 120, &["2000000000022"]),
    ("Wasser", "Water", "Eau", 80, &["2000000000039"]),
    (
        "Apfelschorle",
        "Apple spritzer",
        "Jus de pomme pétillant",
        120,
        &["2000000000046"],
    ),
    ("Bier", "Beer", "Bière", 180, &["2000000000053"]),
    (
        "Alkoholfreies Bier",
        "Alcohol-free beer",
        "Bière sans alcool",
        150,
        &["2000000000060"],
    ),
    ("Kaffee", "Coffee", "Café", 50, &["2000000000077"]),
    (
        "Schokoriegel",
        "Chocolate bar",
        "Barre chocolatée",
        100,
        &["20000011"],
    ),
];

/// Demo-Benutzer: (Name, PIN)
const USERS: &[(&str, Option<&str>)] = &[
    ("Ada", None),
    ("Ben", Some("1234")),
    ("Clara", None),
    ("Dmitri", None),
    ("Emma", Some("1234")),
    ("Ferris", None),
    ("Grace", None),
    ("Hamid", None),
    ("Ines", None),
    ("Jonas", None),
];

/// Legt die Demo-Daten an, aber nur in einer leeren Datenbank.
pub async fn seed_if_empty(pool: &SqlitePool) -> anyhow::Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(false);
    }
    seed(pool).await?;
    Ok(true)
}

/// Legt die Demo-Daten an.
pub async fn seed(pool: &SqlitePool) -> anyhow::Result<()> {
    let mut article_ids = Vec::new();
    for (i, (de, en, fr, price, codes)) in ARTICLES.iter().enumerate() {
        let id = articles::save(
            pool,
            None,
            &ArticleInput {
                name: de.to_string(),
                price_cents: *price,
                active: true,
                sort_order: (i as i64 + 1) * 10,
                translations: vec![("en".into(), en.to_string()), ("fr".into(), fr.to_string())],
                barcodes: codes.iter().map(|c| c.to_string()).collect(),
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("Demo-Artikel {de}: {e}"))?;
        article_ids.push((id, *de, *price));
    }

    let mut user_ids = Vec::new();
    for (name, pin) in USERS {
        let user = users::create(pool, name, *pin)
            .await
            .map_err(|e| anyhow::anyhow!("Demo-Benutzer {name}: {e}"))?;
        user_ids.push(user.id);
    }

    // Kleine Historie über die letzten ~90 Tage. Ein einfacher, fester
    // Pseudo-Zufall sorgt dafür, dass die Demo immer gleich aussieht.
    let mut seed: u64 = 42;
    let mut next = move |max: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % max
    };
    let now = Utc::now();
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    for (i, &user_id) in user_ids.iter().enumerate() {
        // Ein Demo-Benutzer (Dmitri) lädt nie nach und rutscht ins Minus
        let tops_up = i != 3;
        let mut when = now - Duration::days(85 + next(5) as i64);
        let mut balance = 0;
        let mut first = true;
        for _ in 0..(8 + next(15)) {
            let (article_id, name, price) = article_ids[next(article_ids.len() as u64) as usize];
            if first || (tops_up && balance < price + 200) {
                let amount = if tops_up {
                    [1000, 2000, 2000, 5000][next(4) as usize]
                } else {
                    500
                };
                ledger::record_imported(
                    &mut tx,
                    ImportedTx {
                        user_id,
                        amount_cents: amount,
                        kind: TxKind::Deposit,
                        article_id: None,
                        article_name: None,
                        note: None,
                        created_at: db::to_db_time(when),
                        legacy_id: None,
                    },
                )
                .await?;
                balance += amount;
                first = false;
            }
            when += Duration::hours(30 + next(120) as i64);
            if when > now - Duration::hours(1) {
                break;
            }
            ledger::record_imported(
                &mut tx,
                ImportedTx {
                    user_id,
                    amount_cents: -price,
                    kind: TxKind::Purchase,
                    article_id: Some(article_id),
                    article_name: Some(name),
                    note: None,
                    created_at: db::to_db_time(when),
                    legacy_id: None,
                },
            )
            .await?;
            balance -= price;
        }
    }
    tx.commit().await?;
    tracing::info!(
        "Demo-Daten angelegt: {} Benutzer, {} Artikel (PIN der Demo-Benutzer Ben und Emma: 1234)",
        user_ids.len(),
        article_ids.len()
    );
    Ok(())
}
