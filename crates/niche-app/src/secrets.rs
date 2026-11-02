//! Reversible encryption of stored third-party credentials (GBIF accounts of users and
//! organisations, ADR-0110). AES-256-GCM with a server key (`NICHE_SECRET_KEY`, 64 hex
//! characters) that never touches the database. Each value is bound to its row (`aad`, e.g.
//! `user:<id>`), so a sealed password copied to another row does not open.
//!
//! Stored form: hex(nonce ‖ ciphertext ‖ tag).

use ring::aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};

use crate::AppError;

/// The server key. `Debug` never prints it.
#[derive(Clone)]
pub struct SecretKey([u8; 32]);

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey(<redacted>)")
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

impl SecretKey {
    /// 32 bytes as 64 hexadecimal characters (`openssl rand -hex 32`).
    pub fn from_hex(s: &str) -> Result<Self, AppError> {
        let bytes = unhex(s).filter(|b| b.len() == 32).ok_or_else(|| {
            AppError::Invalid("the secret key must be 64 hexadecimal characters (32 bytes)".into())
        })?;
        let mut k = [0u8; 32];
        k.copy_from_slice(&bytes);
        Ok(Self(k))
    }

    fn key(&self) -> Result<LessSafeKey, AppError> {
        UnboundKey::new(&AES_256_GCM, &self.0)
            .map(LessSafeKey::new)
            .map_err(|_| AppError::Internal("secret key rejected".into()))
    }

    /// Encrypt `plain` for the row named by `aad`.
    pub fn seal(&self, aad: &str, plain: &str) -> Result<String, AppError> {
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce)
            .map_err(|e| AppError::Internal(format!("random nonce: {e}")))?;
        let mut buf = plain.as_bytes().to_vec();
        self.key()?
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(aad.as_bytes()),
                &mut buf,
            )
            .map_err(|_| AppError::Internal("encryption failed".into()))?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&buf);
        Ok(hex(&out))
    }

    /// Decrypt a value sealed for the same row with this key.
    pub fn open(&self, aad: &str, sealed: &str) -> Result<String, AppError> {
        let bad = || {
            AppError::Invalid("stored credentials cannot be decrypted with this server key".into())
        };
        let bytes = unhex(sealed).ok_or_else(bad)?;
        if bytes.len() < NONCE_LEN {
            return Err(bad());
        }
        let (nonce, rest) = bytes.split_at(NONCE_LEN);
        let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| bad())?;
        let mut buf = rest.to_vec();
        let plain = self
            .key()?
            .open_in_place(nonce, Aad::from(aad.as_bytes()), &mut buf)
            .map_err(|_| bad())?;
        String::from_utf8(plain.to_vec()).map_err(|_| bad())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn round_trip_bound_to_row() -> Result<(), AppError> {
        let k = SecretKey::from_hex(K)?;
        let s = k.seal("user:1", "hunter2-password")?;
        assert!(!s.contains("hunter2"));
        assert_eq!(k.open("user:1", &s)?, "hunter2-password");
        assert!(
            k.open("user:2", &s).is_err(),
            "another row must not open it"
        );
        let other = SecretKey::from_hex(&K.replace("00", "ff"))?;
        assert!(
            other.open("user:1", &s).is_err(),
            "another key must not open it"
        );
        // Fresh nonce each time.
        assert_ne!(s, k.seal("user:1", "hunter2-password")?);
        Ok(())
    }

    #[test]
    fn rejects_bad_keys() {
        assert!(SecretKey::from_hex("abc").is_err());
        assert!(SecretKey::from_hex(&"zz".repeat(32)).is_err());
        assert!(format!("{:?}", SecretKey::from_hex(K)).contains("redacted"));
    }
}
