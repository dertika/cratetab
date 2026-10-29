//! Artikel, ihre Übersetzungen, Barcodes und Bilder.

use sqlx::SqlitePool;
use thiserror::Error;

use crate::db;

/// Ein Artikel mit dem Namen in der gewünschten Sprache.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Article {
    pub id: i64,
    /// Name in der angefragten Sprache (Fallback: Standardname)
    pub name: String,
    pub price_cents: i64,
    pub active: bool,
    pub sort_order: i64,
    /// Änderungszeitpunkt des Bildes, `None` = kein Bild.
    /// Wird als Cache-Buster an die Bild-URL gehängt.
    pub image_version: Option<String>,
    /// Anzahl der zugeordneten Barcodes
    pub barcode_count: i64,
}

impl Article {
    /// URL des Artikelbildes, falls vorhanden.
    pub fn image_url(&self) -> Option<String> {
        self.image_version.as_ref().map(|v| {
            let v: String = v.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            format!("/images/articles/{}?v={v}", self.id)
        })
    }
}

/// Eingabedaten aus dem Admin-Formular.
#[derive(Debug, Clone, Default)]
pub struct ArticleInput {
    /// Name in der Standardsprache (Pflicht)
    pub name: String,
    pub price_cents: i64,
    pub active: bool,
    pub sort_order: i64,
    /// Optionale Namen pro Sprache: (Sprachcode, Name)
    pub translations: Vec<(String, String)>,
    pub barcodes: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArticleError {
    #[error("Name fehlt")]
    NameEmpty,
    #[error("Preis ungültig")]
    InvalidPrice,
    #[error("ungültiger Barcode: {0}")]
    InvalidBarcode(String),
    #[error("Barcode {code} gehört schon zu „{article}“")]
    BarcodeTaken { code: String, article: String },
    #[error("Artikel nicht gefunden")]
    NotFound,
    #[error("Bildformat nicht unterstützt (PNG, JPEG, GIF oder WebP)")]
    UnsupportedImage,
    #[error("Bild ist zu groß")]
    ImageTooLarge,
    #[error("Datenbankfehler")]
    Db,
}

impl From<sqlx::Error> for ArticleError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!("Datenbankfehler: {e}");
        ArticleError::Db
    }
}

/// Liefert die SELECT-Spalten für [`Article`]; `?1` ist die Sprache.
macro_rules! article_select {
    () => {
        "SELECT a.id,
                COALESCE(tr.name, a.name) AS name,
                a.price_cents, a.active, a.sort_order,
                img.updated_at AS image_version,
                (SELECT COUNT(*) FROM barcodes b WHERE b.article_id = a.id) AS barcode_count
         FROM articles a
         LEFT JOIN article_translations tr ON tr.article_id = a.id AND tr.language = ?1
         LEFT JOIN article_images img ON img.article_id = a.id "
    };
}

/// Aktive Artikel in Sortierreihenfolge (für den Kiosk).
pub async fn list_active(pool: &SqlitePool, lang: &str) -> Result<Vec<Article>, sqlx::Error> {
    sqlx::query_as::<_, Article>(concat!(
        article_select!(),
        "WHERE a.active = 1 ORDER BY a.sort_order, name COLLATE NOCASE"
    ))
    .bind(lang)
    .fetch_all(pool)
    .await
}

/// Alle Artikel (für den Admin-Bereich), Namen in der Standardsprache.
pub async fn list_all(pool: &SqlitePool) -> Result<Vec<Article>, sqlx::Error> {
    sqlx::query_as::<_, Article>(concat!(
        article_select!(),
        "ORDER BY a.active DESC, a.sort_order, a.name COLLATE NOCASE"
    ))
    .bind("")
    .fetch_all(pool)
    .await
}

/// Einen Artikel laden; `lang = ""` liefert den Standardnamen.
pub async fn get(pool: &SqlitePool, id: i64, lang: &str) -> Result<Option<Article>, sqlx::Error> {
    sqlx::query_as::<_, Article>(concat!(article_select!(), "WHERE a.id = ?2"))
        .bind(lang)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Übersetzte Namen eines Artikels: (Sprachcode, Name)
pub async fn translations(
    pool: &SqlitePool,
    id: i64,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT language, name FROM article_translations WHERE article_id = ? ORDER BY language",
    )
    .bind(id)
    .fetch_all(pool)
    .await
}

/// Barcodes eines Artikels.
pub async fn barcodes(pool: &SqlitePool, id: i64) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT code FROM barcodes WHERE article_id = ? ORDER BY code")
        .bind(id)
        .fetch_all(pool)
        .await
}

/// Sucht den Artikel zu einem Barcode.
pub async fn find_by_barcode(pool: &SqlitePool, code: &str) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar("SELECT article_id FROM barcodes WHERE code = ?")
        .bind(normalize_barcode(code))
        .fetch_optional(pool)
        .await
}

/// Entfernt Leerzeichen und unsichtbare Zeichen, die manche Scanner mitsenden.
pub fn normalize_barcode(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect()
}

/// Ein Barcode darf aus Ziffern, Buchstaben und `-` bestehen (max. 64 Zeichen).
pub fn validate_barcode(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Legt einen Artikel an (`id = None`) oder speichert Änderungen.
/// Übersetzungen und Barcodes werden komplett ersetzt.
pub async fn save(
    pool: &SqlitePool,
    id: Option<i64>,
    input: &ArticleInput,
) -> Result<i64, ArticleError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(ArticleError::NameEmpty);
    }
    if input.price_cents < 0 {
        return Err(ArticleError::InvalidPrice);
    }
    let mut codes: Vec<String> = Vec::new();
    for raw in &input.barcodes {
        let code = normalize_barcode(raw);
        if code.is_empty() {
            continue;
        }
        if !validate_barcode(&code) {
            return Err(ArticleError::InvalidBarcode(code));
        }
        if !codes.contains(&code) {
            codes.push(code);
        }
    }

    let now = db::now();
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;

    let id = match id {
        Some(id) => {
            let res = sqlx::query(
                "UPDATE articles SET name = ?, price_cents = ?, active = ?, sort_order = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(name)
            .bind(input.price_cents)
            .bind(input.active)
            .bind(input.sort_order)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            if res.rows_affected() == 0 {
                return Err(ArticleError::NotFound);
            }
            id
        }
        None => sqlx::query_scalar(
            "INSERT INTO articles (name, price_cents, active, sort_order, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(name)
        .bind(input.price_cents)
        .bind(input.active)
        .bind(input.sort_order)
        .bind(&now)
        .bind(&now)
        .fetch_one(&mut *tx)
        .await?,
    };

    // Übersetzungen ersetzen
    sqlx::query("DELETE FROM article_translations WHERE article_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for (lang, tr_name) in &input.translations {
        let tr_name = tr_name.trim();
        if tr_name.is_empty() {
            continue;
        }
        sqlx::query(
            "INSERT INTO article_translations (article_id, language, name) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(lang)
        .bind(tr_name)
        .execute(&mut *tx)
        .await?;
    }

    // Barcodes ersetzen; ein Barcode, der zu einem anderen Artikel gehört, ist ein Fehler
    sqlx::query("DELETE FROM barcodes WHERE article_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for code in &codes {
        let owner: Option<String> = sqlx::query_scalar(
            "SELECT a.name FROM barcodes b JOIN articles a ON a.id = b.article_id WHERE b.code = ?",
        )
        .bind(code)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(article) = owner {
            return Err(ArticleError::BarcodeTaken {
                code: code.clone(),
                article,
            });
        }
        sqlx::query("INSERT INTO barcodes (code, article_id) VALUES (?, ?)")
            .bind(code)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(id)
}

/// Verschiebt einen Artikel in der Sortierung eine Position nach oben oder unten.
/// Dabei werden alle Artikel neu durchnummeriert (10, 20, 30 …).
pub async fn move_article(pool: &SqlitePool, id: i64, up: bool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM articles ORDER BY sort_order, name COLLATE NOCASE")
            .fetch_all(&mut *tx)
            .await?;
    if let Some(pos) = ids.iter().position(|&x| x == id) {
        if up && pos > 0 {
            ids.swap(pos, pos - 1);
        } else if !up && pos + 1 < ids.len() {
            ids.swap(pos, pos + 1);
        }
    }
    for (i, article_id) in ids.iter().enumerate() {
        sqlx::query("UPDATE articles SET sort_order = ? WHERE id = ?")
            .bind((i as i64 + 1) * 10)
            .bind(article_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await
}

/// Erkennt das Bildformat an den ersten Bytes (nicht an der Dateiendung).
/// SVG wird bewusst nicht unterstützt, weil SVGs Skripte enthalten können.
pub fn detect_image_type(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF8") {
        Some("image/gif")
    } else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Speichert das Bild eines Artikels (ersetzt ein vorhandenes).
pub async fn set_image(
    pool: &SqlitePool,
    id: i64,
    data: &[u8],
    max_bytes: usize,
) -> Result<(), ArticleError> {
    if data.len() > max_bytes {
        return Err(ArticleError::ImageTooLarge);
    }
    let mime = detect_image_type(data).ok_or(ArticleError::UnsupportedImage)?;
    // Zeitstempel mit Nanosekunden, damit der Browser ein neues Bild sofort lädt
    let version = chrono::Utc::now().format("%Y%m%d%H%M%S%f").to_string();
    sqlx::query(
        "INSERT INTO article_images (article_id, mime_type, data, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(article_id) DO UPDATE SET mime_type = excluded.mime_type,
             data = excluded.data, updated_at = excluded.updated_at",
    )
    .bind(id)
    .bind(mime)
    .bind(data)
    .bind(version)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_image(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM article_images WHERE article_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Lädt ein Artikelbild: (MIME-Typ, Daten)
pub async fn get_image(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<(String, Vec<u8>)>, sqlx::Error> {
    sqlx::query_as("SELECT mime_type, data FROM article_images WHERE article_id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, codes: &[&str]) -> ArticleInput {
        ArticleInput {
            name: name.into(),
            price_cents: 150,
            active: true,
            sort_order: 0,
            translations: vec![("en".into(), format!("{name} (EN)"))],
            barcodes: codes.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn barcodes_and_translations() {
        let pool = db::connect_memory().await.unwrap();
        let mate = save(
            &pool,
            None,
            &input("Mate", &["4029764001807", " 40297640 "]),
        )
        .await
        .unwrap();
        assert_eq!(
            find_by_barcode(&pool, "4029764001807").await.unwrap(),
            Some(mate)
        );
        assert_eq!(
            find_by_barcode(&pool, "40297640").await.unwrap(),
            Some(mate)
        );
        assert_eq!(find_by_barcode(&pool, "123").await.unwrap(), None);

        // Barcode schon vergeben
        let err = save(&pool, None, &input("Cola", &["4029764001807"]))
            .await
            .unwrap_err();
        assert!(matches!(err, ArticleError::BarcodeTaken { .. }));

        let en = get(&pool, mate, "en").await.unwrap().unwrap();
        assert_eq!(en.name, "Mate (EN)");
        let fr = get(&pool, mate, "fr").await.unwrap().unwrap();
        assert_eq!(fr.name, "Mate", "Fallback auf den Standardnamen");
    }

    #[test]
    fn image_detection() {
        assert_eq!(detect_image_type(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
        assert_eq!(detect_image_type(b"\xFF\xD8\xFF\xE0"), Some("image/jpeg"));
        assert_eq!(detect_image_type(b"<svg></svg>"), None);
    }
}
