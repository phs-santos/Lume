//! Cripto compatível com o libsodium do celular.

use crate::vault::VaultError;

use zeroize::Zeroize;

/// Mesmo Argon2id do cofre (64 MiB, t=3, p=1, 32 bytes) — igual ao `crypto_pwhash` do libsodium.
pub fn derive_login_secret(password: &str, salt: &[u8]) -> Result<[u8; 32], VaultError> {
    if salt.len() != 16 {
        return Err(VaultError::Crypto);
    }
    crate::vault::kdf(password.as_bytes(), salt)
}

pub struct Keypair {
    pub public: [u8; 32],
    pub secret: [u8; 32],
}

impl Drop for Keypair {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

pub fn generate_keypair() -> Keypair {
    let sk = crypto_box::SecretKey::generate(&mut rand::rngs::OsRng);
    Keypair { public: *sk.public_key().as_bytes(), secret: sk.to_bytes() }
}

/// Abre um envelope `crypto_box_seal`. `None` se não for para esta chave ou se foi adulterado.
pub fn seal_open(sealed: &[u8], kp: &Keypair) -> Option<Vec<u8>> {
    crypto_box::SecretKey::from(kp.secret).unseal(sealed).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};

    fn fx(key: &str) -> Vec<u8> {
        let v: serde_json::Value = serde_json::from_str(include_str!("../../../test-fixtures/crypto.json")).unwrap();
        let s = v[key].as_str().unwrap();
        if key == "password" { s.as_bytes().to_vec() } else { STANDARD.decode(s).unwrap() }
    }

    fn fixture_kp() -> Keypair {
        Keypair { public: fx("public_key").try_into().unwrap(), secret: fx("secret_key").try_into().unwrap() }
    }

    #[test]
    fn login_secret_matches_libsodium() {
        let pw = String::from_utf8(fx("password")).unwrap();
        assert_eq!(derive_login_secret(&pw, &fx("salt")).unwrap().to_vec(), fx("login_secret"));
    }

    #[test]
    fn opens_envelope_sealed_by_libsodium() {
        assert_eq!(seal_open(&fx("sealed"), &fixture_kp()).unwrap(), fx("plaintext"));
    }

    #[test]
    fn tampered_envelope_is_none() {
        let mut s = fx("sealed");
        let i = s.len() / 2;
        s[i] ^= 1;
        assert!(seal_open(&s, &fixture_kp()).is_none());
    }

    #[test]
    fn generated_keypair_opens_its_own_seal() {
        let kp = generate_keypair();
        let pk = crypto_box::PublicKey::from(kp.public);
        let sealed = pk.seal(&mut rand::rngs::OsRng, b"oi").unwrap();
        assert_eq!(seal_open(&sealed, &kp).unwrap(), b"oi");
        assert_ne!(generate_keypair().public, kp.public);
    }
}
