//! Geldbeträge.
//!
//! Beträge werden überall als `i64` in **Cent** behandelt. Fließkommazahlen
//! kommen nie vor – weder beim Rechnen noch beim Einlesen von Benutzereingaben.

use thiserror::Error;

/// Fehler beim Einlesen eines Betrags.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseAmountError {
    #[error("leerer Betrag")]
    Empty,
    #[error("ungültiger Betrag")]
    Invalid,
    #[error("höchstens zwei Nachkommastellen erlaubt")]
    TooManyDecimals,
    #[error("Betrag zu groß")]
    TooLarge,
}

/// Größter Betrag, den wir annehmen (1 Million in Euro). Schützt vor Überläufen.
const MAX_ABS_CENTS: i64 = 100_000_000;

/// Liest einen Betrag wie `"1,50"`, `"1.5"`, `"-10"` oder `"20 €"` ein und gibt Cent zurück.
///
/// Komma und Punkt werden beide als Dezimaltrennzeichen akzeptiert, damit die
/// Eingabe in allen Sprachen funktioniert. Tausendertrennzeichen sind nicht erlaubt.
pub fn parse_cents(input: &str) -> Result<i64, ParseAmountError> {
    // Währungszeichen und Leerzeichen entfernen
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '€' | '$' | '£'))
        .collect();
    if cleaned.is_empty() {
        return Err(ParseAmountError::Empty);
    }

    let (negative, rest) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };

    let (whole, frac) = match rest.find([',', '.']) {
        Some(pos) => (&rest[..pos], &rest[pos + 1..]),
        None => (rest, ""),
    };
    if whole.is_empty() && frac.is_empty() {
        return Err(ParseAmountError::Invalid);
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(ParseAmountError::Invalid);
    }
    if frac.len() > 2 {
        return Err(ParseAmountError::TooManyDecimals);
    }
    if whole.len() > 9 {
        return Err(ParseAmountError::TooLarge);
    }

    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| ParseAmountError::Invalid)?
    };
    // "5" -> 50 Cent, "05" -> 5 Cent, "" -> 0 Cent
    let frac: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().map_err(|_| ParseAmountError::Invalid)? * 10,
        _ => frac.parse().map_err(|_| ParseAmountError::Invalid)?,
    };

    let cents = whole * 100 + frac;
    if cents > MAX_ABS_CENTS {
        return Err(ParseAmountError::TooLarge);
    }
    Ok(if negative { -cents } else { cents })
}

/// Formatierungsregeln einer Sprache (kommen aus der jeweiligen .ftl-Datei).
#[derive(Debug, Clone)]
pub struct MoneyFormat {
    pub decimal_separator: String,
    pub thousands_separator: String,
}

/// Formatiert den Zahlenteil eines Betrags ohne Vorzeichen und Währung,
/// z. B. `123456` → `"1.234,56"` (deutsch) bzw. `"1,234.56"` (englisch).
pub fn format_number(cents: i64, fmt: &MoneyFormat) -> String {
    let abs = cents.unsigned_abs();
    let whole = (abs / 100).to_string();
    let frac = abs % 100;

    // Tausendertrennzeichen von rechts einfügen
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push_str(&fmt.thousands_separator);
        }
        grouped.push(ch);
    }
    format!("{grouped}{}{frac:02}", fmt.decimal_separator)
}

/// Formatiert einen Betrag maschinenlesbar mit Punkt, z. B. `-150` → `"-1.50"`.
/// Wird für Formularfelder benutzt.
pub fn format_plain(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_inputs() {
        assert_eq!(parse_cents("1,50"), Ok(150));
        assert_eq!(parse_cents("1.5"), Ok(150));
        assert_eq!(parse_cents("1.05"), Ok(105));
        assert_eq!(parse_cents("20"), Ok(2000));
        assert_eq!(parse_cents("20 €"), Ok(2000));
        assert_eq!(parse_cents("-10"), Ok(-1000));
        assert_eq!(parse_cents("+3,2"), Ok(320));
        assert_eq!(parse_cents(",5"), Ok(50));
        assert_eq!(parse_cents("0"), Ok(0));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_cents(""), Err(ParseAmountError::Empty));
        assert_eq!(parse_cents("abc"), Err(ParseAmountError::Invalid));
        assert_eq!(parse_cents("1,234"), Err(ParseAmountError::TooManyDecimals));
        assert_eq!(parse_cents("1.000,00"), Err(ParseAmountError::Invalid));
        assert_eq!(parse_cents("."), Err(ParseAmountError::Invalid));
        assert_eq!(parse_cents("99999999999"), Err(ParseAmountError::TooLarge));
    }

    #[test]
    fn formats_numbers() {
        let de = MoneyFormat {
            decimal_separator: ",".into(),
            thousands_separator: ".".into(),
        };
        let en = MoneyFormat {
            decimal_separator: ".".into(),
            thousands_separator: ",".into(),
        };
        assert_eq!(format_number(150, &de), "1,50");
        assert_eq!(format_number(-5, &de), "0,05");
        assert_eq!(format_number(123_456_789, &de), "1.234.567,89");
        assert_eq!(format_number(100_000, &en), "1,000.00");
        assert_eq!(format_plain(-150), "-1.50");
        assert_eq!(format_plain(7), "0.07");
    }
}
