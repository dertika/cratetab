//! Tests der Übersetzungen:
//! * alle Sprachen haben dieselben Schlüssel wie Deutsch (und keine Platzhalter, die es dort nicht gibt)
//! * jeder im Code oder in Templates benutzte Schlüssel existiert
//! * Beträge und Datumsangaben werden je Sprache richtig formatiert

use std::{collections::BTreeSet, path::Path, sync::Arc};

use cratetab::i18n::{Locales, Translator};

fn locales() -> Arc<Locales> {
    Arc::new(
        Locales::load("de", "€", chrono_tz::Europe::Berlin, None).expect("Übersetzungen laden"),
    )
}

#[test]
fn all_languages_are_complete() {
    let locales = locales();
    let codes: Vec<&str> = locales
        .languages()
        .iter()
        .map(|l| l.code.as_str())
        .collect();
    for expected in ["de", "en", "fr"] {
        assert!(codes.contains(&expected), "Sprache {expected} fehlt");
    }
    let problems = locales.check_completeness();
    assert!(
        problems.is_empty(),
        "Übersetzungen unvollständig:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_language_has_a_flag() {
    for lang in locales().languages() {
        assert!(
            lang.flag_url.is_some(),
            "Flagge für {} fehlt (assets/flags/{}.svg)",
            lang.code,
            lang.code
        );
    }
}

/// Sucht in einem Quelltext nach `t("…")`, `t1("…"`, `tn("…"` und `ta("…"`.
fn used_keys(text: &str, is_rust: bool, out: &mut BTreeSet<String>) {
    for marker in [".t(\"", ".t1(\"", ".tn(\"", ".ta(\"", "t.t(\""] {
        let mut rest = text;
        while let Some(pos) = rest.find(marker) {
            rest = &rest[pos + marker.len()..];
            if let Some(end) = rest.find('"') {
                out.insert(rest[..end].to_string());
            }
        }
    }
    if !is_rust {
        return;
    }
    // Schlüssel, die als Konstanten im Code stehen (z. B. Buchungsarten, CSV-Spalten)
    for prefix in ["\"tx-kind-", "\"csv-", "\"error-", "\"pin-"] {
        let mut rest = text;
        while let Some(pos) = rest.find(prefix) {
            rest = &rest[pos + 1..];
            if let Some(end) = rest.find('"') {
                out.insert(rest[..end].to_string());
            }
        }
    }
}

fn scan_dir(dir: &Path, out: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            scan_dir(&path, out);
        } else {
            let ext = path.extension().and_then(|e| e.to_str());
            if matches!(ext, Some("rs" | "html")) {
                used_keys(
                    &std::fs::read_to_string(&path).unwrap(),
                    ext == Some("rs"),
                    out,
                );
            }
        }
    }
}

#[test]
fn all_used_keys_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut used = BTreeSet::new();
    scan_dir(&root.join("src"), &mut used);
    scan_dir(&root.join("templates"), &mut used);
    // Platzhalter aus Doku-Kommentaren
    used.retain(|k| !k.is_empty() && !k.contains(' ') && !k.ends_with('-') && k != "schluessel");
    let defined = locales().keys();
    let missing: Vec<_> = used.difference(&defined).collect();
    assert!(
        missing.is_empty(),
        "Diese Schlüssel werden benutzt, fehlen aber in de.ftl: {missing:?}"
    );
}

#[test]
fn money_formatting_per_language() {
    let l = locales();
    let de = Translator::new(l.clone(), "de");
    let en = Translator::new(l.clone(), "en");
    let fr = Translator::new(l.clone(), "fr");
    assert_eq!(de.money(150), "1,50 €");
    assert_eq!(de.money(-1050), "-10,50 €");
    assert_eq!(de.money(123_456), "1.234,56 €");
    assert_eq!(en.money(150), "€1.50");
    assert_eq!(en.money(-1050), "-€10.50");
    assert_eq!(en.money(123_456), "€1,234.56");
    assert_eq!(fr.money(150), "1,50\u{a0}€");
    assert_eq!(fr.money(123_456), "1\u{202f}234,56\u{a0}€");
}

#[test]
fn date_formatting_per_language() {
    let l = locales();
    // 14:05 UTC = 16:05 in Berlin (Sommerzeit)
    let ts = "2026-07-01T14:05:00Z";
    assert_eq!(
        Translator::new(l.clone(), "de").datetime(ts),
        "01.07.2026 16:05"
    );
    assert_eq!(
        Translator::new(l.clone(), "en").datetime(ts),
        "2026-07-01 16:05"
    );
    assert_eq!(
        Translator::new(l.clone(), "fr").datetime(ts),
        "01/07/2026 16:05"
    );
}

#[test]
fn arguments_and_plurals() {
    let l = locales();
    let de = Translator::new(l.clone(), "de");
    assert_eq!(de.t1("hello-user", "name", "Ferris"), "Hallo Ferris!");
    assert_eq!(de.tn("tx-count", "count", 1), "1 Buchung");
    assert_eq!(de.tn("tx-count", "count", 3), "3 Buchungen");
    let en = Translator::new(l.clone(), "en");
    assert_eq!(en.tn("tx-count", "count", 1), "1 booking");
}

#[test]
fn language_negotiation_and_fallback() {
    let l = locales();
    assert_eq!(l.negotiate("fr-CH, fr;q=0.9, en;q=0.8"), "fr");
    assert_eq!(l.negotiate("en-US,en;q=0.9"), "en");
    assert_eq!(
        l.negotiate("ja, es;q=0.5"),
        "de",
        "unbekannte Sprachen -> Standard"
    );
    assert_eq!(l.negotiate(""), "de");
    // Unbekannte Sprache im Übersetzer -> Deutsch
    assert_eq!(Translator::new(l.clone(), "xx").lang(), "de");
    // Unbekannter Schlüssel -> Schlüssel selbst
    assert_eq!(Translator::new(l, "en").t("gibt-es-nicht"), "gibt-es-nicht");
}
