//! Import aus dem alten fnordcredit (RethinkDB-Version, bis Anfang 2017).
//!
//! Eingabe ist ein Verzeichnis von `rethinkdb export --format json`. Darin
//! liegen je Tabelle eine Datei `users.json`, `transactions.json` und
//! optional `products.json` (meist unter `<export>/fnordcredit/`).
//!
//! Datenmodell in fnordcredit:
//! * `users`: `name` (Primärschlüssel), `credit` (Euro als Kommazahl),
//!   `lastchanged`, `pincode` (Hash des npm-Pakets password-hash), `token`,
//!   `debtAllowed`, `debtHardLimit`
//! * `transactions`: `id`, `username`, `delta` (Euro), `credit` (Kontostand
//!   danach), `time`, `description`, `product` (Produktname oder `null`)
//! * `products`: `name`, `price`, `description`, `ean` (mehrere durch `|`
//!   getrennt), `order`
//!
//! Regeln des Imports:
//! * Läuft nur in eine Datenbank ohne Benutzer, Artikel und Buchungen.
//! * Alles passiert in einer Datenbank-Transaktion: Bei einem Fehler bleibt
//!   die Datenbank unverändert. Beim Probelauf wird am Ende zurückgerollt.
//! * Beträge werden auf Cent gerundet. Weicht die Summe der Buchungen eines
//!   Benutzers vom alten Kontostand ab (z. B. Benutzer aus der Zeit vor den
//!   Buchungen), gleicht eine Korrekturbuchung die Differenz aus.
//! * PINs bleiben gültig und werden beim ersten Login auf argon2 umgestellt
//!   (siehe [`crate::password`]). Tokens und Schuldengrenzen pro Benutzer
//!   gibt es in Crate Tab nicht; der Bericht nennt die betroffenen Benutzer.

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use sqlx::{SqliteConnection, SqlitePool};

use crate::{
    articles, db,
    ledger::{self, ImportedTx, TxKind},
    password, users,
};

/// Notiz der Ausgleichsbuchungen
pub const RECONCILE_NOTE: &str = "Import fnordcredit: Ausgleich auf den alten Kontostand";

// ---------------------------------------------------------------------------
// Einlesen des Exports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LegacyUser {
    pub name: String,
    pub credit_cents: Option<i64>,
    pub pincode: Option<String>,
    pub has_token: bool,
    pub debt_allowed: Option<bool>,
    pub debt_hard_limit_cents: Option<i64>,
    pub lastchanged: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct LegacyTx {
    pub id: Option<String>,
    pub username: String,
    pub delta_cents: i64,
    pub balance_after_cents: Option<i64>,
    pub time: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub product: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LegacyProduct {
    pub id: Option<String>,
    pub name: String,
    pub price_cents: i64,
    pub eans: Vec<String>,
    pub order: Option<i64>,
}

/// Der eingelesene Export.
#[derive(Debug, Default)]
pub struct Export {
    pub users: Vec<LegacyUser>,
    pub transactions: Vec<LegacyTx>,
    pub products: Vec<LegacyProduct>,
    /// Dokumente, die nicht gelesen werden konnten: (Tabelle, Grund)
    pub unreadable: Vec<(String, String)>,
    /// Gelesene Dateien (für die Anzeige)
    pub files: Vec<String>,
}

/// Sucht eine Tabellendatei (z. B. `users.json`) im Exportverzeichnis.
fn find_table(dir: &Path, table: &str) -> Result<Option<PathBuf>> {
    let wanted = format!("{table}.json");
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).with_context(|| format!("{} lesen", d.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some(wanted.as_str()) {
                found.push(path);
            }
        }
    }
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.pop()),
        _ => bail!(
            "{wanted} kommt mehrfach vor ({}). Bitte nur den Export einer Datenbank angeben.",
            found
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Liest den Inhalt einer Tabellendatei: ein JSON-Array oder ein Dokument pro Zeile.
fn parse_documents(name: &str, data: &[u8]) -> Result<Vec<Value>> {
    let text = std::str::from_utf8(data).with_context(|| format!("{name} ist kein UTF-8-Text"))?;
    let trimmed = text.trim_start_matches('\u{feff}').trim();
    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed)
            .with_context(|| format!("{name} ist kein gültiges JSON"));
    }
    trimmed
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line.trim().trim_end_matches(','))
                .with_context(|| format!("{name} Zeile {}: kein gültiges JSON", i + 1))
        })
        .collect()
}

/// Zeitwert aus RethinkDB: `{"$reql_type$": "TIME", "epoch_time": …}`,
/// ein ISO-8601-Text oder eine Zahl (Sekunden oder Millisekunden).
fn value_time(v: &Value) -> Option<DateTime<Utc>> {
    match v {
        Value::Object(o) => {
            let epoch = o.get("epoch_time")?.as_f64()?;
            Utc.timestamp_millis_opt((epoch * 1000.0).round() as i64)
                .single()
        }
        Value::String(s) => DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|t| t.with_timezone(&Utc))
            .or_else(|| s.parse::<DateTime<Utc>>().ok()),
        Value::Number(n) => {
            let x = n.as_f64()?;
            // Werte über 10^11 sind Millisekunden (ab dem Jahr 1973)
            let millis = if x.abs() > 1e11 { x } else { x * 1000.0 };
            Utc.timestamp_millis_opt(millis.round() as i64).single()
        }
        _ => None,
    }
}

/// Euro-Betrag (Zahl oder Text) in Cent, kaufmännisch gerundet.
fn value_cents(v: &Value) -> Option<i64> {
    let x = match v {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => s.trim().replace(',', ".").parse::<f64>().ok()?,
        _ => return None,
    };
    let cents = (x * 100.0).round();
    (x.is_finite() && cents.abs() < 1e10).then_some(cents as i64)
}

fn value_string(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn value_bool(v: Option<&Value>) -> Option<bool> {
    match v? {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => Some(n.as_f64()? != 0.0),
        Value::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn parse_user(doc: &Value) -> Result<LegacyUser, String> {
    let name = value_string(doc.get("name")).ok_or("Feld „name“ fehlt")?;
    Ok(LegacyUser {
        credit_cents: doc.get("credit").and_then(value_cents),
        pincode: value_string(doc.get("pincode")),
        has_token: value_string(doc.get("token")).is_some(),
        debt_allowed: value_bool(doc.get("debtAllowed")),
        debt_hard_limit_cents: doc.get("debtHardLimit").and_then(value_cents),
        lastchanged: doc.get("lastchanged").and_then(value_time),
        name,
    })
}

fn parse_transaction(doc: &Value) -> Result<LegacyTx, String> {
    let username = value_string(doc.get("username")).ok_or("Feld „username“ fehlt")?;
    let delta_cents = doc
        .get("delta")
        .and_then(value_cents)
        .ok_or("Feld „delta“ fehlt oder ist keine Zahl")?;
    Ok(LegacyTx {
        id: value_string(doc.get("id")),
        username,
        delta_cents,
        balance_after_cents: doc.get("credit").and_then(value_cents),
        time: doc.get("time").and_then(value_time),
        description: value_string(doc.get("description")),
        product: value_string(doc.get("product")),
    })
}

fn parse_product(doc: &Value) -> Result<LegacyProduct, String> {
    let name = value_string(doc.get("name")).ok_or("Feld „name“ fehlt")?;
    let price_cents = doc
        .get("price")
        .and_then(value_cents)
        .ok_or("Feld „price“ fehlt oder ist keine Zahl")?;
    let eans = value_string(doc.get("ean"))
        .map(|e| {
            e.split('|')
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect()
        })
        .unwrap_or_default();
    Ok(LegacyProduct {
        id: value_string(doc.get("id")),
        name,
        price_cents: price_cents.abs(),
        eans,
        order: doc.get("order").and_then(Value::as_f64).map(|o| o as i64),
    })
}

/// Liest ein Exportverzeichnis von `rethinkdb export` (bzw. scripts/fnordcredit-export.js).
pub fn load_export(dir: &Path) -> Result<Export> {
    let mut files = Vec::new();
    for table in TABLES {
        if let Some(path) = find_table(dir, table)? {
            let data = std::fs::read(&path).with_context(|| format!("{} lesen", path.display()))?;
            files.push((path.display().to_string(), data));
        }
    }
    load_from_files(files)
}

/// Die Tabellen, die der Import liest.
const TABLES: [&str; 3] = ["users", "transactions", "products"];

/// Höchstgröße des entpackten Inhalts eines hochgeladenen Archivs.
pub const MAX_UNPACKED_BYTES: u64 = 256 * 1024 * 1024;

/// Liest hochgeladene Dateien: ein Archiv (`.tar.gz`/`.tgz`/`.tar`) mit dem
/// Export oder die JSON-Dateien einzeln. Das Archiv wird nur im Speicher
/// gelesen, nicht entpackt; andere Dateien darin werden ignoriert.
pub fn load_upload(uploads: Vec<(String, Vec<u8>)>) -> Result<Export> {
    let mut files = Vec::new();
    for (name, data) in uploads {
        if data.starts_with(&[0x1f, 0x8b]) {
            let gz = flate2::read::GzDecoder::new(data.as_slice());
            read_tar(&name, gz, &mut files)?;
        } else if data.len() > 262 && &data[257..262] == b"ustar" {
            read_tar(&name, data.as_slice(), &mut files)?;
        } else {
            files.push((name, data));
        }
    }
    load_from_files(files)
}

/// Sammelt die Tabellendateien aus einem tar-Archiv.
fn read_tar(
    archive_name: &str,
    reader: impl std::io::Read,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<()> {
    use std::io::Read;
    let mut archive = tar::Archive::new(reader.take(MAX_UNPACKED_BYTES));
    let mut total: u64 = 0;
    for entry in archive
        .entries()
        .with_context(|| format!("{archive_name} ist kein gültiges Archiv"))?
    {
        let mut entry = entry.with_context(|| format!("{archive_name} ist beschädigt"))?;
        let path = entry.path()?.to_string_lossy().into_owned();
        let file_name = path.rsplit('/').next().unwrap_or_default();
        if !TABLES.iter().any(|t| file_name == format!("{t}.json")) {
            continue;
        }
        total += entry.size();
        if total > MAX_UNPACKED_BYTES {
            bail!("{archive_name} ist zu groß");
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        files.push((format!("{archive_name}: {path}"), data));
    }
    Ok(())
}

/// Ordnet Dateien den Tabellen zu (am Dateinamen) und liest sie ein.
fn load_from_files(files: Vec<(String, Vec<u8>)>) -> Result<Export> {
    let mut export = Export::default();
    let mut by_table: HashMap<&str, (String, Vec<u8>)> = HashMap::new();
    for (name, data) in files {
        let file_name = name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_string();
        let Some(table) = TABLES.iter().find(|t| file_name == format!("{t}.json")) else {
            continue;
        };
        if let Some((other, _)) = by_table.get(table) {
            bail!(
                "{table}.json kommt mehrfach vor ({other}, {name}). Bitte nur den Export einer Datenbank angeben."
            );
        }
        by_table.insert(table, (name, data));
    }
    if !by_table.contains_key("users") {
        bail!("users.json nicht gefunden");
    }
    if !by_table.contains_key("transactions") {
        bail!("transactions.json nicht gefunden");
    }

    for table in TABLES {
        let Some((name, data)) = by_table.remove(table) else {
            continue;
        };
        for doc in parse_documents(&name, &data)? {
            let result = match table {
                "users" => parse_user(&doc).map(|u| export.users.push(u)),
                "transactions" => parse_transaction(&doc).map(|t| export.transactions.push(t)),
                _ => parse_product(&doc).map(|p| export.products.push(p)),
            };
            if let Err(e) = result {
                export
                    .unreadable
                    .push((table.to_string(), format!("{e}: {doc}")));
            }
        }
        export.files.push(name);
    }
    Ok(export)
}

// ---------------------------------------------------------------------------
// Bericht
// ---------------------------------------------------------------------------

/// Ausgleichsbuchung für einen Benutzer.
#[derive(Debug, Clone)]
pub struct Reconciliation {
    pub name: String,
    pub expected_cents: i64,
    pub from_transactions_cents: i64,
}

/// Eigene Schuldengrenze eines Benutzers in fnordcredit (entfällt beim Import).
#[derive(Debug, Clone)]
pub struct DebtLimit {
    pub name: String,
    /// `None` = keine Schulden erlaubt, sonst die Grenze in Cent
    pub limit_cents: Option<i64>,
}

/// Ein Barcode, der nicht übernommen wurde.
#[derive(Debug, Clone)]
pub struct SkippedBarcode {
    pub code: String,
    pub article: String,
    /// Artikel, dem der Barcode schon gehört; `None` = ungültiger Barcode
    pub owner: Option<String>,
}

/// Was mit vorhandenen Daten passiert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportMode {
    /// Nur in eine leere Datenbank importieren (Standard)
    #[default]
    EmptyOnly,
    /// Vorhandene Benutzer, Artikel und Buchungen löschen und durch den
    /// Import ersetzen. Admin-Zugänge bleiben erhalten.
    Replace,
}

/// Ergebnis des Imports (bzw. Probelaufs).
#[derive(Debug, Default)]
pub struct ImportReport {
    pub dry_run: bool,
    /// Beim Ersetzen gelöschte Datensätze
    pub replaced_users: i64,
    pub replaced_articles: i64,
    pub replaced_transactions: i64,
    /// Sicherung, die vor dem Ersetzen angelegt wurde
    pub backup: Option<String>,
    pub users: usize,
    pub users_created_for_orphans: Vec<String>,
    pub users_renamed: Vec<(String, String)>,
    pub users_with_pin: usize,
    pub pins_dropped: Vec<String>,
    pub users_with_token: Vec<String>,
    pub users_with_debt_limits: Vec<DebtLimit>,
    pub articles: usize,
    pub barcodes: usize,
    pub barcodes_skipped: Vec<SkippedBarcode>,
    pub transactions: usize,
    pub purchases: usize,
    pub deposits: usize,
    pub skipped_zero: usize,
    pub skipped_without_time: usize,
    pub unknown_products: BTreeMap<String, usize>,
    pub balance_chain_mismatches: usize,
    pub reconciliations: Vec<Reconciliation>,
    pub unreadable: Vec<(String, String)>,
    pub old_total_cents: i64,
    pub new_total_cents: i64,
    /// Anteil von `new_total_cents` bei Benutzern, die nur in Buchungen vorkamen
    pub orphans_total_cents: i64,
    pub first_time: Option<DateTime<Utc>>,
    pub last_time: Option<DateTime<Utc>>,
}

fn euro(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!(
        "{sign}{},{:02} €",
        cents.unsigned_abs() / 100,
        cents.unsigned_abs() % 100
    )
}

fn list(f: &mut fmt::Formatter<'_>, title: &str, items: &[String]) -> fmt::Result {
    if items.is_empty() {
        return Ok(());
    }
    writeln!(f, "  {title} ({}):", items.len())?;
    for item in items.iter().take(50) {
        writeln!(f, "    - {item}")?;
    }
    if items.len() > 50 {
        writeln!(f, "    … und {} weitere", items.len() - 50)?;
    }
    Ok(())
}

impl ImportReport {
    /// Erster Buchungszeitpunkt im Datenbankformat (für die Anzeige)
    pub fn first_time_db(&self) -> Option<String> {
        self.first_time.map(db::to_db_time)
    }

    /// Letzter Buchungszeitpunkt im Datenbankformat (für die Anzeige)
    pub fn last_time_db(&self) -> Option<String> {
        self.last_time.map(db::to_db_time)
    }

    /// Stimmt die neue Summe aller Guthaben mit der alten überein?
    pub fn totals_match(&self) -> bool {
        self.old_total_cents == self.new_total_cents - self.orphans_total_cents
    }

    /// Gibt es Hinweise, die man sich ansehen sollte?
    pub fn has_hints(&self) -> bool {
        !(self.reconciliations.is_empty()
            && self.users_renamed.is_empty()
            && self.users_created_for_orphans.is_empty()
            && self.pins_dropped.is_empty()
            && self.users_with_token.is_empty()
            && self.users_with_debt_limits.is_empty()
            && self.unknown_products.is_empty()
            && self.barcodes_skipped.is_empty()
            && self.unreadable.is_empty())
            || self.skipped_zero > 0
            || self.skipped_without_time > 0
            || self.balance_chain_mismatches > 0
    }
}

impl fmt::Display for ImportReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dry_run {
            writeln!(f, "PROBELAUF – es wurde nichts gespeichert.\n")?;
        } else {
            writeln!(f, "Import abgeschlossen.\n")?;
        }
        if self.replaced_users + self.replaced_articles + self.replaced_transactions > 0 {
            writeln!(
                f,
                "ERSETZT: {} Benutzer, {} Artikel und {} Buchungen {} gelöscht.",
                self.replaced_users,
                self.replaced_articles,
                self.replaced_transactions,
                if self.dry_run { "würden" } else { "wurden" }
            )?;
        }
        if let Some(backup) = &self.backup {
            writeln!(f, "Sicherung vorher: {backup}")?;
        }
        let date = |t: Option<DateTime<Utc>>| {
            t.map(|t| t.format("%d.%m.%Y").to_string())
                .unwrap_or("–".into())
        };
        writeln!(f, "Benutzer:   {}", self.users)?;
        writeln!(
            f,
            "Artikel:    {} (mit {} Barcodes)",
            self.articles, self.barcodes
        )?;
        writeln!(
            f,
            "Buchungen:  {} ({} Käufe, {} Aufladungen), {} bis {}",
            self.transactions,
            self.purchases,
            self.deposits,
            date(self.first_time),
            date(self.last_time)
        )?;
        writeln!(f, "Ausgleichsbuchungen: {}", self.reconciliations.len())?;
        let matches = self.totals_match();
        write!(
            f,
            "Summe aller Guthaben: alt {}, neu {}",
            euro(self.old_total_cents),
            euro(self.new_total_cents)
        )?;
        if self.orphans_total_cents != 0 {
            write!(
                f,
                " (davon {} bei Benutzern ohne alten Kontostand)",
                euro(self.orphans_total_cents)
            )?;
        }
        writeln!(f, "{}", if matches { " ✓" } else { "  ← ABWEICHUNG" })?;
        writeln!(
            f,
            "PINs übernommen: {} (werden beim ersten Login auf argon2 umgestellt)\n",
            self.users_with_pin
        )?;

        writeln!(f, "Hinweise:")?;
        let recon: Vec<String> = self
            .reconciliations
            .iter()
            .map(|r| {
                format!(
                    "{}: Kontostand {}, Summe der Buchungen {}, Ausgleich {}",
                    r.name,
                    euro(r.expected_cents),
                    euro(r.from_transactions_cents),
                    euro(r.expected_cents - r.from_transactions_cents)
                )
            })
            .collect();
        list(f, "Ausgleichsbuchungen", &recon)?;
        let renamed: Vec<String> = self
            .users_renamed
            .iter()
            .map(|(a, b)| format!("„{a}“ → „{b}“"))
            .collect();
        list(f, "Umbenannt (Name doppelt oder zu lang)", &renamed)?;
        list(
            f,
            "Buchungen ohne Benutzer – deaktiviert angelegt",
            &self.users_created_for_orphans,
        )?;
        list(
            f,
            "PIN in unbekanntem Format, nicht übernommen",
            &self.pins_dropped,
        )?;
        list(f, "Hatten einen Token (entfällt)", &self.users_with_token)?;
        let limits: Vec<String> = self
            .users_with_debt_limits
            .iter()
            .map(|d| match d.limit_cents {
                None => format!("{}: keine Schulden erlaubt", d.name),
                Some(l) => format!("{}: Grenze {}", d.name, euro(l)),
            })
            .collect();
        list(
            f,
            "Hatten eigene Schuldengrenzen (entfallen, es gilt CRATETAB_MIN_BALANCE)",
            &limits,
        )?;
        let unknown: Vec<String> = self
            .unknown_products
            .iter()
            .map(|(p, n)| format!("{p} ({n}×)"))
            .collect();
        list(
            f,
            "Produkte aus Buchungen, die es nicht mehr gibt (nur als Name übernommen)",
            &unknown,
        )?;
        let barcodes: Vec<String> = self
            .barcodes_skipped
            .iter()
            .map(|b| match &b.owner {
                None => format!("„{}“ bei {}: ungültig", b.code, b.article),
                Some(o) => format!("{} bei {}: gehört schon zu {o}", b.code, b.article),
            })
            .collect();
        list(f, "Barcodes übersprungen", &barcodes)?;
        let unreadable: Vec<String> = self
            .unreadable
            .iter()
            .map(|(t, e)| format!("{t}: {e}"))
            .collect();
        list(f, "Nicht lesbare Dokumente", &unreadable)?;
        if self.skipped_zero > 0 {
            writeln!(
                f,
                "  Buchungen über 0,00 € übersprungen: {}",
                self.skipped_zero
            )?;
        }
        if self.skipped_without_time > 0 {
            writeln!(
                f,
                "  Buchungen ohne Zeitstempel übersprungen: {}",
                self.skipped_without_time
            )?;
        }
        if self.balance_chain_mismatches > 0 {
            writeln!(
                f,
                "  {} Buchungen passten in fnordcredit nicht zum vorherigen Kontostand \
                 (Hinweis auf frühere Änderungen direkt in der Datenbank)",
                self.balance_chain_mismatches
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/// Vergibt Namen, die in Crate Tab gültig und (ohne Groß-/Kleinschreibung) eindeutig sind.
#[derive(Default)]
struct NameAllocator {
    taken: std::collections::HashSet<String>,
}

impl NameAllocator {
    fn allocate(&mut self, original: &str) -> String {
        let mut base = original.split_whitespace().collect::<Vec<_>>().join(" ");
        if base.is_empty() {
            base = "Unbenannt".into();
        }
        // Platz für einen Zusatz wie " (12)" lassen
        if base.chars().count() > users::MAX_NAME_LEN {
            base = base
                .chars()
                .take(users::MAX_NAME_LEN - 5)
                .collect::<String>()
                .trim_end()
                .to_string();
        }
        let mut candidate = base.clone();
        let mut n = 2;
        while self.taken.contains(&candidate.to_lowercase()) {
            candidate = format!("{base} ({n})");
            n += 1;
        }
        self.taken.insert(candidate.to_lowercase());
        candidate
    }
}

async fn insert_user(
    conn: &mut SqliteConnection,
    name: &str,
    pin_hash: Option<&str>,
    active: bool,
    legacy_id: &str,
    created_at: &str,
) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO users (name, pin_hash, active, legacy_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(name)
    .bind(pin_hash)
    .bind(active)
    .bind(legacy_id)
    .bind(created_at)
    .bind(created_at)
    .fetch_one(&mut *conn)
    .await?)
}

/// Führt den Import aus. Mit `dry_run` wird am Ende alles zurückgerollt.
pub async fn import(
    pool: &SqlitePool,
    export: &Export,
    dry_run: bool,
    mode: ImportMode,
) -> Result<ImportReport> {
    let mut report = ImportReport {
        dry_run,
        unreadable: export.unreadable.clone(),
        ..Default::default()
    };
    let now = db::now();
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;

    // Vorhandene Daten: abbrechen oder (beim Ersetzen) löschen. Das Löschen
    // gehört zur selben Transaktion – schlägt der Import fehl, bleibt alles erhalten.
    let (users_n, articles_n, tx_n): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM users), (SELECT COUNT(*) FROM articles),
                (SELECT COUNT(*) FROM transactions)",
    )
    .fetch_one(&mut *tx)
    .await?;
    if users_n + articles_n + tx_n > 0 {
        if mode == ImportMode::EmptyOnly {
            bail!(
                "Die Datenbank ist nicht leer ({users_n} Benutzer, {articles_n} Artikel, {tx_n} Buchungen). \
                 Der Import läuft nur in eine neue Datenbank (ohne Demo-Daten) – oder mit „Ersetzen“."
            );
        }
        for table in [
            "transactions",
            "barcodes",
            "article_translations",
            "article_images",
            "articles",
            "sessions WHERE user_id IS NOT NULL",
            "users",
        ] {
            // Feste Tabellennamen, keine Benutzereingaben
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
                .execute(&mut *tx)
                .await?;
        }
        report.replaced_users = users_n;
        report.replaced_articles = articles_n;
        report.replaced_transactions = tx_n;
    }

    // --- Artikel -----------------------------------------------------------
    let mut products: Vec<&LegacyProduct> = export.products.iter().collect();
    products.sort_by_key(|p| p.order.unwrap_or(i64::MAX));
    let mut article_by_name: HashMap<String, i64> = HashMap::new();
    let mut barcode_owner: HashMap<String, String> = HashMap::new();
    for (i, p) in products.iter().enumerate() {
        let name = p.name.split_whitespace().collect::<Vec<_>>().join(" ");
        let legacy_id = p.id.clone().unwrap_or_else(|| format!("product:{name}"));
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO articles (name, price_cents, active, sort_order, legacy_id, created_at, updated_at)
             VALUES (?, ?, 1, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&name)
        .bind(p.price_cents)
        .bind((i as i64 + 1) * 10)
        .bind(&legacy_id)
        .bind(&now)
        .bind(&now)
        .fetch_one(&mut *tx)
        .await?;
        article_by_name.entry(name.clone()).or_insert(id);
        report.articles += 1;

        for raw in &p.eans {
            let code = articles::normalize_barcode(raw);
            if !articles::validate_barcode(&code) {
                report.barcodes_skipped.push(SkippedBarcode {
                    code: raw.clone(),
                    article: name.clone(),
                    owner: None,
                });
            } else if let Some(owner) = barcode_owner.get(&code) {
                report.barcodes_skipped.push(SkippedBarcode {
                    code: code.clone(),
                    article: name.clone(),
                    owner: Some(owner.clone()),
                });
            } else {
                sqlx::query("INSERT INTO barcodes (code, article_id) VALUES (?, ?)")
                    .bind(&code)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                barcode_owner.insert(code, name.clone());
                report.barcodes += 1;
            }
        }
    }
    let article_by_lower: HashMap<String, i64> = article_by_name
        .iter()
        .map(|(n, id)| (n.to_lowercase(), *id))
        .collect();

    // --- Benutzer ----------------------------------------------------------
    // Erste Buchung je Benutzer als Anlagedatum
    let mut first_tx: HashMap<&str, DateTime<Utc>> = HashMap::new();
    for t in &export.transactions {
        if let Some(time) = t.time {
            first_tx
                .entry(t.username.as_str())
                .and_modify(|e| *e = (*e).min(time))
                .or_insert(time);
        }
    }

    let mut names = NameAllocator::default();
    let mut user_ids: HashMap<String, i64> = HashMap::new();
    let mut legacy_users: Vec<&LegacyUser> = export.users.iter().collect();
    legacy_users.sort_by(|a, b| a.name.cmp(&b.name));
    for u in &legacy_users {
        if user_ids.contains_key(&u.name) {
            report.unreadable.push((
                "users".into(),
                format!("Benutzer „{}“ doppelt im Export", u.name),
            ));
            continue;
        }
        let name = names.allocate(&u.name);
        if name != u.name {
            report.users_renamed.push((u.name.clone(), name.clone()));
        }
        let pin = match u.pincode.as_deref() {
            Some(p) if password::is_legacy_hash(p) => {
                report.users_with_pin += 1;
                Some(p)
            }
            Some(_) => {
                report.pins_dropped.push(name.clone());
                None
            }
            None => None,
        };
        if u.has_token {
            report.users_with_token.push(name.clone());
        }
        if u.debt_allowed == Some(false) || u.debt_hard_limit_cents.is_some_and(|l| l != 0) {
            report.users_with_debt_limits.push(DebtLimit {
                name: name.clone(),
                limit_cents: if u.debt_allowed == Some(false) {
                    None
                } else {
                    u.debt_hard_limit_cents
                },
            });
        }
        let created = [first_tx.get(u.name.as_str()).copied(), u.lastchanged]
            .into_iter()
            .flatten()
            .min()
            .map(db::to_db_time)
            .unwrap_or_else(|| now.clone());
        let id = insert_user(&mut tx, &name, pin, true, &u.name, &created).await?;
        user_ids.insert(u.name.clone(), id);
        report.users += 1;
        report.old_total_cents += u.credit_cents.unwrap_or(0);
    }

    // --- Buchungen ---------------------------------------------------------
    let mut transactions: Vec<&LegacyTx> = export.transactions.iter().collect();
    transactions.sort_by(|a, b| a.time.cmp(&b.time).then_with(|| a.id.cmp(&b.id)));
    let mut running: HashMap<i64, i64> = HashMap::new();
    let mut orphan_ids: Vec<i64> = Vec::new();
    let mut last_time: HashMap<i64, DateTime<Utc>> = HashMap::new();
    for t in transactions {
        let Some(time) = t.time else {
            report.skipped_without_time += 1;
            continue;
        };
        if t.delta_cents == 0 {
            report.skipped_zero += 1;
            continue;
        }
        // Buchung zu einem Benutzer, den es nicht (mehr) gibt: deaktiviert anlegen
        let user_id = match user_ids.get(&t.username) {
            Some(id) => *id,
            None => {
                let name = names.allocate(&t.username);
                let id = insert_user(
                    &mut tx,
                    &name,
                    None,
                    false,
                    &t.username,
                    &db::to_db_time(time),
                )
                .await?;
                user_ids.insert(t.username.clone(), id);
                report.users_created_for_orphans.push(name);
                orphan_ids.push(id);
                report.users += 1;
                id
            }
        };

        let (kind, article_id) = if t.delta_cents > 0 {
            report.deposits += 1;
            (TxKind::Deposit, None)
        } else {
            report.purchases += 1;
            let article = t.product.as_ref().and_then(|p| {
                let p = p.split_whitespace().collect::<Vec<_>>().join(" ");
                article_by_name
                    .get(&p)
                    .or_else(|| article_by_lower.get(&p.to_lowercase()))
                    .copied()
            });
            if let (Some(p), None) = (&t.product, article) {
                *report.unknown_products.entry(p.clone()).or_default() += 1;
            }
            (TxKind::Purchase, article)
        };

        let before = running.get(&user_id).copied().unwrap_or(0);
        let after = before + t.delta_cents;
        if t.balance_after_cents.is_some_and(|b| b != after) {
            report.balance_chain_mismatches += 1;
        }
        running.insert(user_id, after);
        last_time.insert(user_id, time);

        ledger::record_imported(
            &mut tx,
            ImportedTx {
                user_id,
                amount_cents: t.delta_cents,
                kind,
                article_id,
                article_name: if t.delta_cents < 0 {
                    t.product.as_deref()
                } else {
                    None
                },
                note: t.description.as_deref(),
                created_at: db::to_db_time(time),
                legacy_id: t.id.as_deref(),
            },
        )
        .await
        .with_context(|| format!("Buchung {:?} von {}", t.id, t.username))?;
        report.transactions += 1;
        report.first_time = Some(report.first_time.map_or(time, |f| f.min(time)));
        report.last_time = Some(report.last_time.map_or(time, |l| l.max(time)));
    }

    // --- Ausgleich auf den alten Kontostand --------------------------------
    for u in &legacy_users {
        let (Some(expected), Some(&user_id)) = (u.credit_cents, user_ids.get(&u.name)) else {
            continue;
        };
        let current = running.get(&user_id).copied().unwrap_or(0);
        if current == expected {
            continue;
        }
        let when = [last_time.get(&user_id).copied(), u.lastchanged]
            .into_iter()
            .flatten()
            .max()
            .map(db::to_db_time)
            .unwrap_or_else(|| now.clone());
        ledger::record_imported(
            &mut tx,
            ImportedTx {
                user_id,
                amount_cents: expected - current,
                kind: TxKind::Correction,
                article_id: None,
                article_name: None,
                note: Some(RECONCILE_NOTE),
                created_at: when,
                legacy_id: None,
            },
        )
        .await?;
        report.reconciliations.push(Reconciliation {
            name: names_for(&user_ids, &u.name, &mut tx).await?,
            expected_cents: expected,
            from_transactions_cents: current,
        });
    }

    // --- Kontrolle ---------------------------------------------------------
    let (total, mismatches): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(balance_cents), 0),
                (SELECT COUNT(*) FROM users u WHERE u.balance_cents <>
                    (SELECT COALESCE(SUM(amount_cents), 0) FROM transactions t WHERE t.user_id = u.id))
         FROM users",
    )
    .fetch_one(&mut *tx)
    .await?;
    report.new_total_cents = total;
    report.orphans_total_cents = orphan_ids
        .iter()
        .map(|id| running.get(id).copied().unwrap_or(0))
        .sum();
    if mismatches > 0 {
        bail!(
            "Interner Fehler: {mismatches} Guthaben passen nicht zu den Buchungen – nichts gespeichert"
        );
    }

    if dry_run {
        tx.rollback().await?;
    } else {
        tx.commit().await?;
    }
    Ok(report)
}

/// Sichert die Datenbank vor dem Ersetzen nach `<db-verzeichnis>/backups/`.
/// Gibt den Pfad der Sicherung zurück.
pub async fn backup_before_replace(pool: &SqlitePool, database_path: &Path) -> Result<PathBuf> {
    let dir = database_path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .join("backups");
    std::fs::create_dir_all(&dir).with_context(|| format!("{} anlegen", dir.display()))?;
    let target = dir.join(format!(
        "vor-import-{}.db",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    db::backup(pool, &target).await?;
    Ok(target)
}

/// Aktueller Name eines importierten Benutzers (für den Bericht).
async fn names_for(
    user_ids: &HashMap<String, i64>,
    legacy: &str,
    conn: &mut SqliteConnection,
) -> Result<String> {
    let Some(id) = user_ids.get(legacy) else {
        return Ok(legacy.to_string());
    };
    Ok(sqlx::query_scalar("SELECT name FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *conn)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_reql_times_and_money() {
        let t = value_time(
            &json!({"$reql_type$": "TIME", "epoch_time": 1_456_823_470.5, "timezone": "+00:00"}),
        );
        // Sekundenbruchteile werden beim Speichern abgeschnitten
        assert_eq!(db::to_db_time(t.unwrap()), "2016-03-01T09:11:10Z");
        assert!(value_time(&json!("2016-03-01T09:11:10.000Z")).is_some());
        assert!(value_time(&json!(1_456_823_470_000_i64)).is_some());

        assert_eq!(value_cents(&json!(1.5)), Some(150));
        assert_eq!(value_cents(&json!(0.1 + 0.2)), Some(30));
        assert_eq!(value_cents(&json!(-0.29)), Some(-29));
        assert_eq!(value_cents(&json!("-1,20")), Some(-120));
        assert_eq!(value_cents(&json!(null)), None);
    }

    #[test]
    fn names_are_made_unique() {
        let mut n = NameAllocator::default();
        assert_eq!(n.allocate("Max"), "Max");
        assert_eq!(n.allocate("max"), "max (2)");
        assert_eq!(n.allocate("  Anna   Lena "), "Anna Lena");
        assert_eq!(n.allocate(""), "Unbenannt");
        assert!(n.allocate(&"x".repeat(60)).chars().count() <= users::MAX_NAME_LEN);
    }
}
