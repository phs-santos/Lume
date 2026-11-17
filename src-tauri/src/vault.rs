//! Cofre local: chave-mestra aleatória embrulhada pela senha e pela frase de recuperação.

use serde::{Deserialize, Serialize};
use std::path::Path;
use zeroize::Zeroize;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum VaultError {
    #[error("WRONG_PASSWORD")]
    WrongPassword,
    #[error("INVALID_PHRASE")]
    InvalidPhrase,
    #[error("IO: {0}")]
    Io(String),
    #[error("CRYPTO")]
    Crypto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Wrapped {
    #[serde(with = "b64")]
    pub nonce: Vec<u8>,
    #[serde(with = "b64")]
    pub ct: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultFile {
    pub version: u32,
    pub nickname: String,
    #[serde(with = "b64")]
    pub pw_salt: Vec<u8>,
    #[serde(with = "b64")]
    pub rec_salt: Vec<u8>,
    pub by_password: Wrapped,
    pub by_recovery: Wrapped,
}

pub struct MasterKey(pub [u8; 32]);

impl Drop for MasterKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

const KDF_M_KIB: u32 = 65536;
const KDF_T: u32 = 3;
const KDF_P: u32 = 1;

fn random<const N: usize>() -> [u8; N] {
    use rand::RngCore;
    let mut b = [0u8; N];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}

fn kdf(secret: &[u8], salt: &[u8]) -> Result<[u8; 32], VaultError> {
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(KDF_M_KIB, KDF_T, KDF_P, Some(32)).map_err(|_| VaultError::Crypto)?;
    let mut out = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(secret, salt, &mut out)
        .map_err(|_| VaultError::Crypto)?;
    Ok(out)
}

fn wrap(kek: &[u8; 32], master: &[u8; 32]) -> Result<Wrapped, VaultError> {
    use chacha20poly1305::{aead::Aead, KeyInit, XChaCha20Poly1305, XNonce};
    let nonce: [u8; 24] = random();
    let ct = XChaCha20Poly1305::new(kek.into())
        .encrypt(XNonce::from_slice(&nonce), master.as_slice())
        .map_err(|_| VaultError::Crypto)?;
    Ok(Wrapped { nonce: nonce.to_vec(), ct })
}

/// `None` quando a chave de embrulho não abre o envelope.
fn unwrap_key(kek: &[u8; 32], w: &Wrapped) -> Option<MasterKey> {
    use chacha20poly1305::{aead::Aead, KeyInit, XChaCha20Poly1305, XNonce};
    if w.nonce.len() != 24 {
        return None;
    }
    let mut pt = XChaCha20Poly1305::new(kek.into())
        .decrypt(XNonce::from_slice(&w.nonce), w.ct.as_slice())
        .ok()?;
    let key: [u8; 32] = pt.as_slice().try_into().ok()?;
    pt.zeroize();
    Some(MasterKey(key))
}

fn password_envelope(password: &str, master: &MasterKey) -> Result<(Vec<u8>, Wrapped), VaultError> {
    let salt: [u8; 16] = random();
    let mut kek = kdf(password.as_bytes(), &salt)?;
    let w = wrap(&kek, &master.0);
    kek.zeroize();
    Ok((salt.to_vec(), w?))
}

pub fn create(nickname: &str, password: &str) -> Result<(VaultFile, MasterKey, String), VaultError> {
    let master = MasterKey(random());
    let mut entropy: [u8; 32] = random();
    let mnemonic = bip39::Mnemonic::from_entropy(&entropy).map_err(|_| VaultError::Crypto)?;
    let rec_salt: [u8; 16] = random();
    let mut rec_kek = kdf(&entropy, &rec_salt)?;
    entropy.zeroize();
    let by_recovery = wrap(&rec_kek, &master.0);
    rec_kek.zeroize();
    let (pw_salt, by_password) = password_envelope(password, &master)?;
    let v = VaultFile {
        version: 1,
        nickname: nickname.to_string(),
        pw_salt,
        rec_salt: rec_salt.to_vec(),
        by_password,
        by_recovery: by_recovery?,
    };
    Ok((v, master, mnemonic.to_string()))
}

pub fn unlock(v: &VaultFile, password: &str) -> Result<MasterKey, VaultError> {
    let mut kek = kdf(password.as_bytes(), &v.pw_salt)?;
    let r = unwrap_key(&kek, &v.by_password).ok_or(VaultError::WrongPassword);
    kek.zeroize();
    r
}

pub fn recover(v: &VaultFile, phrase: &str, new_password: &str) -> Result<(VaultFile, MasterKey), VaultError> {
    let mnemonic = bip39::Mnemonic::parse_in(bip39::Language::English, normalize_phrase(phrase))
        .map_err(|_| VaultError::InvalidPhrase)?;
    let mut entropy = mnemonic.to_entropy();
    let mut kek = kdf(&entropy, &v.rec_salt)?;
    entropy.zeroize();
    let master = unwrap_key(&kek, &v.by_recovery);
    kek.zeroize();
    let master = master.ok_or(VaultError::InvalidPhrase)?;
    let (pw_salt, by_password) = password_envelope(new_password, &master)?;
    let nv = VaultFile { pw_salt, by_password, ..v.clone() };
    Ok((nv, master))
}

pub fn normalize_phrase(input: &str) -> String {
    input.split_whitespace().map(|w| w.to_lowercase()).collect::<Vec<_>>().join(" ")
}

pub fn load(path: &Path) -> Result<Option<VaultFile>, VaultError> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| VaultError::Io(e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(VaultError::Io(e.to_string())),
    }
}

/// Escrita atômica e durável: guarda a versão anterior em `.bak`, grava num temporário,
/// força para o disco (`sync_all`), renomeia e sincroniza a pasta.
pub fn save(path: &Path, v: &VaultFile) -> Result<(), VaultError> {
    use std::io::Write;
    let io = |e: std::io::Error| VaultError::Io(e.to_string());
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    std::fs::create_dir_all(&dir).map_err(io)?;
    if path.exists() {
        std::fs::copy(path, path.with_extension("json.bak")).map_err(io)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(v).map_err(|e| VaultError::Io(e.to_string()))?;
    {
        let mut f = std::fs::File::create(&tmp).map_err(io)?;
        f.write_all(&json).map_err(io)?;
        f.sync_all().map_err(io)?;
    }
    std::fs::rename(&tmp, path).map_err(io)?;
    #[cfg(unix)]
    if let Ok(d) = std::fs::File::open(&dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

mod b64 {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_then_unlock_returns_same_key() {
        let (v, k, _) = create("ana", "senha-boa").unwrap();
        assert_eq!(unlock(&v, "senha-boa").unwrap().0, k.0);
    }

    #[test]
    fn wrong_password_is_wrongpassword() {
        let (v, _, _) = create("ana", "senha-boa").unwrap();
        assert!(matches!(unlock(&v, "errada"), Err(VaultError::WrongPassword)));
    }

    #[test]
    fn phrase_has_24_words() {
        let (_, _, p) = create("ana", "x").unwrap();
        assert_eq!(p.split(' ').count(), 24);
    }

    #[test]
    fn recover_sets_new_password_and_keeps_key() {
        let (v, k, p) = create("ana", "velha").unwrap();
        let (v2, k2) = recover(&v, &p, "nova").unwrap();
        assert_eq!(k.0, k2.0);
        assert_eq!(unlock(&v2, "nova").unwrap().0, k.0);
        assert!(unlock(&v2, "velha").is_err());
    }

    #[test]
    fn messy_phrase_is_accepted() {
        let (v, k, p) = create("ana", "x").unwrap();
        let messy = format!("  {}\n", p.to_uppercase().replace(' ', "  "));
        assert_eq!(recover(&v, &messy, "y").unwrap().1 .0, k.0);
    }

    #[test]
    fn phrase_from_other_vault_is_invalidphrase() {
        let (v, _, _) = create("ana", "x").unwrap();
        let (_, _, other) = create("bia", "x").unwrap();
        assert!(matches!(recover(&v, &other, "y"), Err(VaultError::InvalidPhrase)));
    }

    #[test]
    fn garbage_phrase_is_invalidphrase() {
        let (v, _, _) = create("ana", "x").unwrap();
        assert!(matches!(recover(&v, "isso nao e uma frase", "y"), Err(VaultError::InvalidPhrase)));
    }

    #[test]
    fn save_keeps_previous_version_as_bak() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.json");
        let (v1, _, _) = create("ana", "x").unwrap();
        let (v2, _, _) = create("ana", "y").unwrap();
        save(&path, &v1).unwrap();
        save(&path, &v2).unwrap();
        assert_eq!(load(&path).unwrap().unwrap(), v2);
        assert_eq!(load(&path.with_extension("json.bak")).unwrap().unwrap(), v1);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.json");
        assert!(load(&path).unwrap().is_none());
        let (v, _, _) = create("ana", "x").unwrap();
        save(&path, &v).unwrap();
        assert_eq!(load(&path).unwrap().unwrap(), v);
    }
}
