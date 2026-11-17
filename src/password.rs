//! Hashen und Prüfen von PINs und Passwörtern mit argon2.
//!
//! Gespeichert wird nur der Hash im „PHC-Format“ (`$argon2id$v=19$...`), der
//! Salt und Parameter enthält. Das Klartext-Passwort verlässt diese Funktionen nie.
//!
//! Aus fnordcredit übernommene PINs liegen noch im alten Format des npm-Pakets
//! `password-hash` vor (`sha1$<salt>$<iterationen>$<hmac>`). Sie werden hier
//! ebenfalls geprüft und beim ersten erfolgreichen Login durch einen
//! argon2-Hash ersetzt (siehe [`needs_rehash`]).

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;

/// Hasht ein Passwort oder eine PIN mit zufälligem Salt.
pub fn hash(secret: &str) -> anyhow::Result<String> {
    let hash = Argon2::default()
        .hash_password(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    Ok(hash.to_string())
}

/// Prüft ein Passwort gegen einen gespeicherten Hash (argon2 oder altes
/// fnordcredit-Format). Ein kaputter Hash gilt als „falsches Passwort“.
pub fn verify(secret: &str, stored_hash: &str) -> bool {
    if is_legacy_hash(stored_hash) {
        return verify_legacy(secret, stored_hash);
    }
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(secret.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Muss der Hash nach einem erfolgreichen Login durch argon2 ersetzt werden?
pub fn needs_rehash(stored_hash: &str) -> bool {
    is_legacy_hash(stored_hash)
}

/// Erkennt einen Hash aus fnordcredit: `sha1$<salt>$<iterationen>$<hmac>`
/// oder das noch ältere Format ohne Iterationen `sha1$<salt>$<hmac>`.
pub fn is_legacy_hash(stored_hash: &str) -> bool {
    parse_legacy(stored_hash).is_some()
}

/// Zerlegt einen alten Hash in (Salt, Iterationen, erwarteter HMAC in Hex).
fn parse_legacy(stored_hash: &str) -> Option<(&str, u32, &str)> {
    let parts: Vec<&str> = stored_hash.split('$').collect();
    let (salt, iterations, digest) = match parts.as_slice() {
        ["sha1", salt, iterations, digest] => (*salt, iterations.parse().ok()?, *digest),
        ["sha1", salt, digest] => (*salt, 1, *digest),
        _ => return None,
    };
    let valid_digest = digest.len() == 40 && digest.bytes().all(|b| b.is_ascii_hexdigit());
    (valid_digest && !salt.is_empty() && (1..=10_000).contains(&iterations))
        .then_some((salt, iterations, digest))
}

/// Rechnet wie `password-hash` 1.2.2: Der Salt ist der HMAC-Schlüssel, jede
/// Iteration hasht die Hex-Ausgabe der vorherigen.
fn verify_legacy(secret: &str, stored_hash: &str) -> bool {
    let Some((salt, iterations, expected)) = parse_legacy(stored_hash) else {
        return false;
    };
    let mut current = secret.to_string();
    for _ in 0..iterations {
        let Ok(mut mac) = Hmac::<Sha1>::new_from_slice(salt.as_bytes()) else {
            return false;
        };
        mac.update(current.as_bytes());
        current = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
    }
    current.eq_ignore_ascii_case(expected)
}

/// Erzeugt ein zufälliges Token als Hex-String (für Sitzungs-Cookies und Startpasswörter).
pub fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::fill(&mut buf[..]);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let h = hash("1234").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify("1234", &h));
        assert!(!verify("4321", &h));
        assert!(!verify("1234", "kaputt"));
        assert!(!needs_rehash(&h));
    }

    /// Testvektoren, erzeugt mit dem npm-Paket password-hash 1.2.2 (wie fnordcredit).
    #[test]
    fn legacy_fnordcredit_hashes() {
        let one = "sha1$2b19e86b$1$7fccf43a2e1bd530ca8c1e573cb6b5b5059ba7d7";
        assert!(verify("1234", one));
        assert!(!verify("1235", one));
        assert!(needs_rehash(one));

        let three_iterations = "sha1$bd25e160$3$313d6a95bc3feca3113c3a8a2de37bd344636b0f";
        assert!(verify("geheim42", three_iterations));

        // Noch älteres Format ohne Iterationsangabe
        let old_format = "sha1$9ecc3aeb$fa242ae8d34898952d124eddf870596a46e07677";
        assert!(verify("0815", old_format));

        assert!(!is_legacy_hash("md5$abc$1$0123"));
        assert!(!is_legacy_hash(
            "sha1$$1$7fccf43a2e1bd530ca8c1e573cb6b5b5059ba7d7"
        ));
    }

    #[test]
    fn tokens_are_random() {
        let a = random_token(16);
        assert_eq!(a.len(), 32);
        assert_ne!(a, random_token(16));
    }
}
