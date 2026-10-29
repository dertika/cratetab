//! Hashen und Prüfen von PINs und Passwörtern mit argon2.
//!
//! Gespeichert wird nur der Hash im „PHC-Format“ (`$argon2id$v=19$...`), der
//! Salt und Parameter enthält. Das Klartext-Passwort verlässt diese Funktionen nie.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

/// Hasht ein Passwort oder eine PIN mit zufälligem Salt.
pub fn hash(secret: &str) -> anyhow::Result<String> {
    let hash = Argon2::default()
        .hash_password(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    Ok(hash.to_string())
}

/// Prüft ein Passwort gegen einen gespeicherten Hash.
/// Ein kaputter Hash gilt als „falsches Passwort“.
pub fn verify(secret: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(secret.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
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
    }

    #[test]
    fn tokens_are_random() {
        let a = random_token(16);
        assert_eq!(a.len(), 32);
        assert_ne!(a, random_token(16));
    }
}
