//! Import aus fnordcredit mit einem nachgebauten RethinkDB-Export.

use std::path::{Path, PathBuf};

use cratetab::{
    articles, db,
    fnordcredit::{self, RECONCILE_NOTE},
    ledger::{self, TxKind},
    users::{self, PinCheck},
};
use serde_json::{Value, json};
use sqlx::SqlitePool;

/// Zeitwert wie in `rethinkdb export`
fn t(epoch: f64) -> Value {
    json!({"$reql_type$": "TIME", "epoch_time": epoch, "timezone": "+00:00"})
}

/// PIN „1234“, erzeugt mit password-hash 1.2.2 (wie fnordcredit)
const PIN_1234: &str = "sha1$2b19e86b$1$7fccf43a2e1bd530ca8c1e573cb6b5b5059ba7d7";

fn write_export(dir: &Path) {
    let db_dir = dir.join("fnordcredit");
    std::fs::create_dir_all(&db_dir).unwrap();

    // users.json als JSON-Array
    let users = json!([
        {"name": "Max", "credit": 2.9000000000000004, "lastchanged": t(1_480_000_500.0),
         "pincode": PIN_1234, "token": "karte-17", "debtAllowed": true, "debtHardLimit": -20},
        {"name": "max", "credit": 0, "lastchanged": t(1_470_000_000.0), "pincode": null},
        {"name": "Alt", "credit": 12.34, "lastchanged": t(1_420_000_000.0)},
        {"name": "Knapp", "credit": -1.0, "lastchanged": t(1_480_000_000.0), "debtAllowed": false},
        {"credit": 1}
    ]);
    std::fs::write(
        db_dir.join("users.json"),
        serde_json::to_string_pretty(&users).unwrap(),
    )
    .unwrap();

    let products = json!([
        {"id": "p-cola", "name": "Cola", "price": 1.2, "ean": "", "order": 2, "description": "Cola 0,33"},
        {"id": "p-mate", "name": "Club-Mate", "price": 1.5, "ean": "4029764001807|40297640", "order": 1,
         "description": "Mate"},
        {"id": "p-kaputt", "name": "Kaputt", "price": 1, "ean": "4029764001807|ab cd!", "order": 3}
    ]);
    std::fs::write(
        db_dir.join("products.json"),
        serde_json::to_string(&products).unwrap(),
    )
    .unwrap();

    // transactions.json als ein Dokument pro Zeile
    let txs = [
        json!({"id": "t1", "username": "Max", "delta": 5.0, "credit": 5.0, "time": t(1_480_000_000.0),
               "description": "+5,00 €", "product": null}),
        json!({"id": "t2", "username": "Max", "delta": -1.5, "credit": 3.5, "time": t(1_480_000_100.0),
               "description": "Mate", "product": "Club-Mate"}),
        json!({"id": "t3", "username": "Max", "delta": -0.5, "credit": 3.0, "time": t(1_480_000_200.0),
               "description": "-0,50 €", "product": null}),
        json!({"id": "t4", "username": "Max", "delta": "-0.1", "credit": 2.9, "time": t(1_480_000_300.0),
               "description": "Mate (alt)", "product": "Mate alt"}),
        json!({"id": "t5", "username": "Geist", "delta": -1.2, "credit": -1.2, "time": t(1_400_000_000.0),
               "description": "Cola", "product": "cola"}),
        json!({"id": "t6", "username": "Max", "delta": 0, "credit": 2.9, "time": t(1_480_000_400.0)}),
        json!({"id": "t7", "username": "Knapp", "delta": -0.5, "credit": -1.0, "time": t(1_480_000_000.0),
               "description": "-0,50 €", "product": null}),
    ];
    let lines: Vec<String> = txs.iter().map(|v| v.to_string()).collect();
    std::fs::write(db_dir.join("transactions.json"), lines.join("\n")).unwrap();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cratetab-fc-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn user_by_legacy(pool: &SqlitePool, legacy: &str) -> users::User {
    let id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE legacy_id = ?")
        .bind(legacy)
        .fetch_one(pool)
        .await
        .unwrap();
    users::get(pool, id).await.unwrap().unwrap()
}

#[tokio::test]
async fn full_import() {
    let dir = temp_dir("full");
    write_export(&dir);
    let export = fnordcredit::load_export(&dir).unwrap();
    assert_eq!(export.users.len(), 4);
    assert_eq!(export.transactions.len(), 7);
    assert_eq!(export.products.len(), 3);
    assert_eq!(export.unreadable.len(), 1, "Benutzer ohne Namen");

    let pool = db::connect_memory().await.unwrap();

    // Probelauf ändert nichts
    let dry = fnordcredit::import(&pool, &export, true).await.unwrap();
    assert!(dry.dry_run);
    assert_eq!(dry.transactions, 6);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "Probelauf darf nichts speichern");

    // Echter Import
    let report = fnordcredit::import(&pool, &export, false).await.unwrap();
    println!("{report}");
    assert_eq!(report.users, 5, "4 Benutzer + 1 deaktiviert angelegter");
    assert_eq!(report.articles, 3);
    assert_eq!(
        report.barcodes, 2,
        "doppelter und ungültiger Barcode übersprungen"
    );
    assert_eq!(report.barcodes_skipped.len(), 2);
    assert_eq!(report.transactions, 6);
    assert_eq!(report.skipped_zero, 1);
    assert_eq!(report.deposits, 1);
    assert_eq!(report.purchases, 5);
    assert_eq!(report.users_with_pin, 1);
    assert_eq!(report.users_with_token, vec!["Max".to_string()]);
    assert_eq!(report.users_with_debt_limits.len(), 2);
    assert_eq!(
        report.users_renamed,
        vec![("max".to_string(), "max (2)".to_string())]
    );
    assert_eq!(report.users_created_for_orphans, vec!["Geist".to_string()]);
    assert_eq!(report.unknown_products.get("Mate alt"), Some(&1));
    // Knapps Buchung nennt -1,00 € als neuen Kontostand, gerechnet sind es -0,50 €
    assert_eq!(report.balance_chain_mismatches, 1);
    // Alt (keine Buchungen) und Knapp (Kontostand passt nicht zur Buchung) brauchen einen Ausgleich
    assert_eq!(report.reconciliations.len(), 2);
    assert_eq!(report.old_total_cents, 290 + 1234 - 100);
    assert_eq!(user_by_legacy(&pool, "Knapp").await.balance_cents, -100);
    assert_eq!(
        report.new_total_cents,
        report.old_total_cents - 120,
        "plus Geist mit -1,20 €"
    );
    assert_eq!(report.orphans_total_cents, -120);
    assert!(report.to_string().contains(" ✓"), "Summen stimmen überein");

    // Guthaben exakt in Cent
    let max = user_by_legacy(&pool, "Max").await;
    assert_eq!(max.balance_cents, 290);
    assert!(max.has_pin());
    let alt = user_by_legacy(&pool, "Alt").await;
    assert_eq!(alt.balance_cents, 1234);
    let geist = user_by_legacy(&pool, "Geist").await;
    assert!(!geist.active);
    assert_eq!(geist.balance_cents, -120);
    assert_eq!(user_by_legacy(&pool, "max").await.name, "max (2)");
    assert!(ledger::verify_balances(&pool).await.unwrap().is_empty());

    // Original-Zeitstempel und Zuordnung zu Artikeln
    let (created, article, kind): (String, Option<i64>, TxKind) = sqlx::query_as(
        "SELECT created_at, article_id, kind FROM transactions WHERE legacy_id = 't2'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(created, "2016-11-24T15:08:20Z");
    assert_eq!(kind, TxKind::Purchase);
    let mate = articles::find_by_barcode(&pool, "40297640").await.unwrap();
    assert_eq!(article, mate);
    // Produktname in anderer Schreibweise („cola“) wird trotzdem zugeordnet
    let cola_tx: Option<i64> =
        sqlx::query_scalar("SELECT article_id FROM transactions WHERE legacy_id = 't5'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(cola_tx.is_some());
    // Ausgleichsbuchung
    let note: String = sqlx::query_scalar(
        "SELECT note FROM transactions WHERE user_id = ? AND kind = 'correction'",
    )
    .bind(alt.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(note, RECONCILE_NOTE);

    // Alte PIN funktioniert und wird dabei auf argon2 umgestellt
    assert_eq!(
        users::check_pin(&pool, max.id, "1111").await.unwrap(),
        PinCheck::Wrong
    );
    assert_eq!(
        users::check_pin(&pool, max.id, "1234").await.unwrap(),
        PinCheck::Ok
    );
    let hash: String = sqlx::query_scalar("SELECT pin_hash FROM users WHERE id = ?")
        .bind(max.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$"));
    assert_eq!(
        users::check_pin(&pool, max.id, "1234").await.unwrap(),
        PinCheck::Ok
    );

    // Zweiter Import wird abgelehnt
    let err = fnordcredit::import(&pool, &export, false)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("nicht leer"), "{err}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn missing_files_are_reported() {
    let dir = temp_dir("missing");
    assert!(
        fnordcredit::load_export(&dir)
            .unwrap_err()
            .to_string()
            .contains("users.json")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn upload_variants() {
    let users = br#"[{"name": "Ada", "credit": 1}]"#.to_vec();
    let txs =
        br#"[{"id": "1", "username": "Ada", "delta": 1, "time": "2016-01-01T00:00:00Z"}]"#.to_vec();

    // Einzelne JSON-Dateien; unbekannte Dateien werden ignoriert
    let export = fnordcredit::load_upload(vec![
        ("users.json".into(), users.clone()),
        ("transactions.json".into(), txs.clone()),
        ("notizen.txt".into(), b"egal".to_vec()),
    ])
    .unwrap();
    assert_eq!(export.users.len(), 1);
    assert_eq!(export.transactions.len(), 1);
    assert!(export.products.is_empty());

    // Fehlende Tabelle
    let err = fnordcredit::load_upload(vec![("users.json".into(), users.clone())]).unwrap_err();
    assert!(err.to_string().contains("transactions.json"), "{err}");

    // Doppelte Tabelle
    let err = fnordcredit::load_upload(vec![
        ("users.json".into(), users.clone()),
        ("a/users.json".into(), users),
        ("transactions.json".into(), txs),
    ])
    .unwrap_err();
    assert!(err.to_string().contains("mehrfach"), "{err}");

    // Kaputtes Archiv
    let err =
        fnordcredit::load_upload(vec![("x.tar.gz".into(), vec![0x1f, 0x8b, 0, 1, 2])]).unwrap_err();
    assert!(!err.to_string().is_empty());
}
