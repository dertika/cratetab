//! Mehrsprachigkeit mit [Project Fluent](https://projectfluent.org/).
//!
//! Alle Texte stehen in `locales/<sprache>.ftl`. Jede Datei in diesem
//! Verzeichnis wird beim Kompilieren eingebettet und automatisch als Sprache
//! angeboten – für eine neue Sprache genügt also eine neue Datei (siehe README).
//! Zusätzlich kann `CRATETAB_LOCALES_DIR` auf ein Verzeichnis zeigen, dessen
//! Dateien zur Laufzeit geladen werden und eingebettete Texte überschreiben.
//!
//! Neben normalen Texten enthält jede Datei einige `fmt-*`-Einträge, die das
//! Format von Beträgen und Datumsangaben in dieser Sprache festlegen.

use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono_tz::Tz;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use fluent_langneg::{NegotiationStrategy, negotiate_languages, parse_accepted_languages};
use fluent_syntax::{ast, parser};
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

use crate::{
    assets::Assets,
    db,
    money::{self, MoneyFormat},
};

/// Die eingebetteten Übersetzungsdateien.
#[derive(RustEmbed)]
#[folder = "locales/"]
#[include = "*.ftl"]
struct EmbeddedLocales;

/// Eine verfügbare Sprache (für die Flaggenleiste).
#[derive(Debug, Clone)]
pub struct Language {
    /// Sprachcode = Dateiname ohne `.ftl`, z. B. `de`
    pub code: String,
    /// Name der Sprache in ihr selbst, z. B. „Deutsch“
    pub name: String,
    /// URL der Flagge, falls `assets/flags/<code>.svg` existiert
    pub flag_url: Option<String>,
}

impl Language {
    /// Sprachkürzel in Großbuchstaben für die Anzeige neben der Flagge.
    pub fn label(&self) -> String {
        self.code.to_uppercase()
    }
}

struct Locale {
    bundle: FluentBundle<FluentResource>,
    money: MoneyFormat,
    datetime_format: String,
    date_format: String,
    /// Quelltexte der .ftl-Dateien (für die Vollständigkeitsprüfung)
    sources: Vec<String>,
}

/// Alle geladenen Sprachen.
pub struct Locales {
    locales: HashMap<String, Locale>,
    languages: Vec<Language>,
    default: String,
    currency: String,
    timezone: Tz,
}

impl Locales {
    /// Lädt die eingebetteten Sprachen und optional ein zusätzliches Verzeichnis.
    pub fn load(
        default: &str,
        currency: &str,
        timezone: Tz,
        extra_dir: Option<&Path>,
    ) -> Result<Self> {
        // Sprachcode -> Liste von Quelltexten (eingebettet zuerst, Überschreibungen danach)
        let mut sources: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for file in EmbeddedLocales::iter() {
            let data = EmbeddedLocales::get(&file).context("eingebettete Datei fehlt")?;
            let text = String::from_utf8(data.data.into_owned())
                .with_context(|| format!("{file} ist kein UTF-8"))?;
            sources
                .entry(code_from_filename(&file))
                .or_default()
                .push(text);
        }
        if let Some(dir) = extra_dir {
            for entry in
                std::fs::read_dir(dir).with_context(|| format!("{} lesen", dir.display()))?
            {
                let path = entry?.path();
                if path.extension().and_then(|e| e.to_str()) != Some("ftl") {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("{} lesen", path.display()))?;
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                tracing::info!("Zusätzliche Übersetzung geladen: {}", path.display());
                sources
                    .entry(code_from_filename(name))
                    .or_default()
                    .push(text);
            }
        }

        let mut locales = HashMap::new();
        let mut languages = Vec::new();
        for (code, texts) in sources {
            let langid: LanguageIdentifier = code
                .parse()
                .with_context(|| format!("„{code}“ ist kein gültiger Sprachcode"))?;
            let mut bundle = FluentBundle::new_concurrent(vec![langid]);
            // Keine unsichtbaren Unicode-Steuerzeichen um eingesetzte Werte
            bundle.set_use_isolating(false);
            for text in &texts {
                let res = FluentResource::try_new(text.clone())
                    .map_err(|(_, errs)| anyhow::anyhow!("Syntaxfehler in {code}.ftl: {errs:?}"))?;
                bundle.add_resource_overriding(res);
            }

            let get = |key: &str, fallback: &str| -> String {
                format_message(&bundle, key, None).unwrap_or_else(|| fallback.to_string())
            };
            let locale = Locale {
                money: MoneyFormat {
                    decimal_separator: get("fmt-decimal-separator", ","),
                    thousands_separator: get("fmt-thousands-separator", "."),
                },
                datetime_format: get("fmt-datetime", "%Y-%m-%d %H:%M"),
                date_format: get("fmt-date", "%Y-%m-%d"),
                sources: texts,
                bundle,
            };
            let name = get_plain(&locale.bundle, "language-name").unwrap_or_else(|| code.clone());
            let flag_path = format!("flags/{code}.svg");
            languages.push(Language {
                flag_url: Assets::get(&flag_path).map(|_| format!("/static/{flag_path}")),
                code: code.clone(),
                name,
            });
            locales.insert(code, locale);
        }

        anyhow::ensure!(
            locales.contains_key(default),
            "Standardsprache „{default}“ gibt es nicht (vorhanden: {})",
            languages
                .iter()
                .map(|l| l.code.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        // Standardsprache zuerst, danach alphabetisch
        languages.sort_by_key(|l| (l.code != default, l.code.clone()));

        Ok(Locales {
            locales,
            languages,
            default: default.to_string(),
            currency: currency.to_string(),
            timezone,
        })
    }

    pub fn languages(&self) -> &[Language] {
        &self.languages
    }

    pub fn default_language(&self) -> &str {
        &self.default
    }

    pub fn is_available(&self, code: &str) -> bool {
        self.locales.contains_key(code)
    }

    /// Wählt anhand des `Accept-Language`-Headers die beste verfügbare Sprache.
    pub fn negotiate(&self, accept_language: &str) -> String {
        let requested = parse_accepted_languages(accept_language);
        let available: Vec<LanguageIdentifier> = self
            .languages
            .iter()
            .filter_map(|l| l.code.parse().ok())
            .collect();
        let default: LanguageIdentifier = self.default.parse().unwrap_or_default();
        negotiate_languages(
            &requested,
            &available,
            Some(&default),
            NegotiationStrategy::Filtering,
        )
        .first()
        .map(|l| l.to_string())
        .unwrap_or_else(|| self.default.clone())
    }

    /// Prüft, ob alle Sprachen dieselben Schlüssel und Variablen haben wie die
    /// Standardsprache. Liefert eine Liste von Problemen (leer = alles gut).
    pub fn check_completeness(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let Some(reference) = self.locales.get(&self.default) else {
            return vec![format!("Standardsprache {} fehlt", self.default)];
        };
        let reference = collect_messages(&reference.sources);
        for (code, locale) in &self.locales {
            if *code == self.default {
                continue;
            }
            let messages = collect_messages(&locale.sources);
            for (key, vars) in &reference {
                match messages.get(key) {
                    None => problems.push(format!("{code}: Schlüssel „{key}“ fehlt")),
                    Some(other) => {
                        for var in other.difference(vars) {
                            problems.push(format!(
                                "{code}: „{key}“ benutzt die Variable ${var}, die es in {} nicht gibt",
                                self.default
                            ));
                        }
                    }
                }
            }
            for key in messages.keys() {
                if !reference.contains_key(key) {
                    problems.push(format!(
                        "{code}: Schlüssel „{key}“ gibt es in {} nicht",
                        self.default
                    ));
                }
            }
        }
        problems.sort();
        problems
    }

    /// Alle Schlüssel der Standardsprache (für Tests).
    pub fn keys(&self) -> BTreeSet<String> {
        self.locales
            .get(&self.default)
            .map(|l| collect_messages(&l.sources).into_keys().collect())
            .unwrap_or_default()
    }
}

/// `de.ftl` -> `de`
fn code_from_filename(name: &str) -> String {
    name.trim_end_matches(".ftl").to_string()
}

/// Formatiert eine Nachricht; `None`, wenn es den Schlüssel nicht gibt.
fn format_message(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    args: Option<&FluentArgs>,
) -> Option<String> {
    let msg = bundle.get_message(key)?;
    let pattern = msg.value()?;
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, args, &mut errors);
    if !errors.is_empty() {
        tracing::warn!("Übersetzung „{key}“: {errors:?}");
    }
    Some(text.into_owned())
}

fn get_plain(bundle: &FluentBundle<FluentResource>, key: &str) -> Option<String> {
    format_message(bundle, key, None)
}

/// Liest aus .ftl-Quelltexten alle Nachrichten und die darin benutzten Variablen.
fn collect_messages(sources: &[String]) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for src in sources {
        let resource = match parser::parse(src.as_str()) {
            Ok(r) => r,
            Err((r, _)) => r,
        };
        for entry in resource.body {
            if let ast::Entry::Message(msg) = entry {
                let mut vars = BTreeSet::new();
                if let Some(pattern) = &msg.value {
                    collect_pattern_vars(pattern, &mut vars);
                }
                for attr in &msg.attributes {
                    collect_pattern_vars(&attr.value, &mut vars);
                }
                out.insert(msg.id.name.to_string(), vars);
            }
        }
    }
    out
}

fn collect_pattern_vars(pattern: &ast::Pattern<&str>, vars: &mut BTreeSet<String>) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            collect_expression_vars(expression, vars);
        }
    }
}

fn collect_expression_vars(expr: &ast::Expression<&str>, vars: &mut BTreeSet<String>) {
    match expr {
        ast::Expression::Inline(inline) => collect_inline_vars(inline, vars),
        ast::Expression::Select { selector, variants } => {
            collect_inline_vars(selector, vars);
            for v in variants {
                collect_pattern_vars(&v.value, vars);
            }
        }
    }
}

fn collect_inline_vars(expr: &ast::InlineExpression<&str>, vars: &mut BTreeSet<String>) {
    match expr {
        ast::InlineExpression::VariableReference { id } => {
            vars.insert(id.name.to_string());
        }
        ast::InlineExpression::Placeable { expression } => {
            collect_expression_vars(expression, vars)
        }
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for a in &arguments.positional {
                collect_inline_vars(a, vars);
            }
            for a in &arguments.named {
                collect_inline_vars(&a.value, vars);
            }
        }
        _ => {}
    }
}

/// Übersetzt Texte in einer bestimmten Sprache. Wird jeder Seite mitgegeben.
///
/// In Templates: `{{ t.t("schluessel") }}`, `{{ t.money(betrag) }}` usw.
#[derive(Clone)]
pub struct Translator {
    locales: Arc<Locales>,
    lang: String,
}

impl Translator {
    /// Erzeugt einen Übersetzer; unbekannte Sprachen fallen auf die Standardsprache zurück.
    pub fn new(locales: Arc<Locales>, lang: &str) -> Self {
        let lang = if locales.is_available(lang) {
            lang.to_string()
        } else {
            locales.default.clone()
        };
        Translator { locales, lang }
    }

    /// Aktueller Sprachcode, z. B. `de`
    pub fn lang(&self) -> &str {
        &self.lang
    }

    pub fn languages(&self) -> &[Language] {
        self.locales.languages()
    }

    /// Text mit Argumenten. Fehlt der Schlüssel, wird die Standardsprache
    /// versucht und zuletzt der Schlüssel selbst angezeigt.
    pub fn ta(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let locales = &self.locales;
        if let Some(text) = locales
            .locales
            .get(&self.lang)
            .and_then(|l| format_message(&l.bundle, key, args))
        {
            return text;
        }
        if let Some(text) = locales
            .locales
            .get(&locales.default)
            .and_then(|l| format_message(&l.bundle, key, args))
        {
            return text;
        }
        tracing::warn!("Übersetzungsschlüssel „{key}“ fehlt");
        key.to_string()
    }

    /// Einfacher Text ohne Argumente.
    pub fn t(&self, key: &str) -> String {
        self.ta(key, None)
    }

    /// Text mit einem Text-Argument, z. B. `t1("welcome", "name", &user.name)`.
    pub fn t1(&self, key: &str, name: &str, value: impl AsRef<str>) -> String {
        let mut args = FluentArgs::new();
        args.set(name, FluentValue::from(value.as_ref().to_string()));
        self.ta(key, Some(&args))
    }

    /// Text mit einem Zahl-Argument (für Mehrzahlformen), z. B. `tn("items", "count", 3)`.
    pub fn tn(&self, key: &str, name: &str, n: impl Borrow<i64>) -> String {
        let mut args = FluentArgs::new();
        args.set(name, FluentValue::from(*n.borrow()));
        self.ta(key, Some(&args))
    }

    fn locale(&self) -> &Locale {
        self.locales
            .locales
            .get(&self.lang)
            .or_else(|| self.locales.locales.get(&self.locales.default))
            .expect("Standardsprache ist immer geladen")
    }

    /// Betrag mit Währung in der Schreibweise der Sprache, z. B. „1,50 €“ / „€1.50“.
    pub fn money(&self, cents: impl Borrow<i64>) -> String {
        let cents = *cents.borrow();
        let mut args = FluentArgs::new();
        args.set("sign", if cents < 0 { "-" } else { "" });
        args.set("amount", money::format_number(cents, &self.locale().money));
        args.set("currency", self.locales.currency.clone());
        self.ta("fmt-money", Some(&args))
    }

    /// Betrag ohne Währung, z. B. „-1,50“ (für CSV-Export).
    pub fn number(&self, cents: impl Borrow<i64>) -> String {
        let cents = *cents.borrow();
        let sign = if cents < 0 { "-" } else { "" };
        format!(
            "{sign}{}",
            money::format_number(cents, &self.locale().money)
        )
    }

    /// Dezimaltrennzeichen der Sprache („,“ oder „.“)
    pub fn decimal_separator(&self) -> &str {
        &self.locale().money.decimal_separator
    }

    /// Zeitstempel aus der Datenbank in Ortszeit und Sprachformat.
    pub fn datetime(&self, ts: impl AsRef<str>) -> String {
        self.format_time(ts.as_ref(), &self.locale().datetime_format)
    }

    /// Nur das Datum eines Zeitstempels.
    pub fn date(&self, ts: impl AsRef<str>) -> String {
        self.format_time(ts.as_ref(), &self.locale().date_format)
    }

    fn format_time(&self, ts: &str, fmt: &str) -> String {
        match db::parse_db_time(ts) {
            Some(t) => t
                .with_timezone(&self.locales.timezone)
                .format(fmt)
                .to_string(),
            None => ts.to_string(),
        }
    }
}
