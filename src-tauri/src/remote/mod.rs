//! Conexão com o servidor do Lume: cripto compatível com o celular, cliente HTTP e sincronização.
pub mod crypto;
pub mod sync;
pub mod client;

use crate::db::settings::{setting_get, setting_set};
use crate::state::SharedDb;
use base64::{engine::general_purpose::STANDARD, Engine};
use crypto::Keypair;
use rusqlite::Connection;
use sync::{EnvelopeMeta, LocalInbox, RemoteError};

/// Conexão com o servidor, guardada nas preferências (dentro do banco cifrado).
pub struct RemoteConfig {
    pub url: String,
    pub nickname: String,
    pub token: String,
}

fn local(e: impl std::fmt::Display) -> RemoteError {
    RemoteError::Local(e.to_string())
}

pub fn load_config(c: &Connection) -> Result<Option<RemoteConfig>, RemoteError> {
    let get = |k: &str| setting_get(c, k).map_err(local);
    match (get("remote_url")?, get("remote_nickname")?, get("remote_token")?) {
        (Some(url), Some(nickname), Some(token)) => Ok(Some(RemoteConfig { url, nickname, token })),
        _ => Ok(None),
    }
}

pub fn load_keypair(c: &Connection) -> Result<Option<Keypair>, RemoteError> {
    let dec = |k: &str| -> Result<Option<[u8; 32]>, RemoteError> {
        Ok(match setting_get(c, k).map_err(local)? {
            Some(s) => Some(STANDARD.decode(s).map_err(local)?.try_into().map_err(|_| local("chave inválida"))?),
            None => None,
        })
    };
    Ok(match (dec("remote_public")?, dec("remote_secret")?) {
        (Some(public), Some(secret)) => Some(Keypair { public, secret }),
        _ => None,
    })
}

pub fn save_connection(c: &Connection, cfg: &RemoteConfig, kp: &Keypair) -> Result<(), RemoteError> {
    for (k, v) in [
        ("remote_url", cfg.url.clone()),
        ("remote_nickname", cfg.nickname.clone()),
        ("remote_token", cfg.token.clone()),
        ("remote_public", STANDARD.encode(kp.public)),
        ("remote_secret", STANDARD.encode(kp.secret)),
    ] {
        setting_set(c, k, &v).map_err(local)?;
    }
    Ok(())
}

/// Esquece a sessão. O par de chaves fica, para abrir ideias que ainda estejam a caminho.
pub fn forget_session(c: &Connection) -> Result<(), RemoteError> {
    c.execute("DELETE FROM settings WHERE key IN ('remote_token')", []).map_err(local)?;
    Ok(())
}

/// Entra (ou cadastra, se vier convite e a conta não existir) e publica a chave pública.
/// Só rede: não toca no banco local.
pub fn connect(url: &str, nickname: &str, password: &str, invite: Option<&str>, kp: Keypair) -> Result<(RemoteConfig, Keypair), RemoteError> {
    let salt = client::get_salt(url, nickname)?;
    let secret = crypto::derive_login_secret(password, &salt).map_err(local)?;
    let token = match client::login(url, nickname, &secret) {
        Ok(t) => t,
        Err(RemoteError::Status(401, msg)) => match invite.filter(|i| !i.trim().is_empty()) {
            None => return Err(RemoteError::Status(401, msg)),
            Some(inv) => {
                let mut fresh = [0u8; 16];
                rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut fresh);
                let secret = crypto::derive_login_secret(password, &fresh).map_err(local)?;
                client::register(url, nickname, inv.trim(), &fresh, &secret)?
            }
        },
        Err(e) => return Err(e),
    };
    client::put_public_key(url, &token, &kp.public)?;
    Ok((RemoteConfig { url: client::base(url), nickname: nickname.into(), token }, kp))
}

/// Banco compartilhado, trancado só pelo instante de cada leitura ou gravação.
pub struct SharedLocal(pub SharedDb);

impl LocalInbox for SharedLocal {
    fn has(&self, id: &str) -> Result<bool, RemoteError> {
        let g = self.0.lock().unwrap();
        g.as_ref().ok_or_else(|| local("LOCKED"))?.has(id)
    }
    fn insert(&self, m: &EnvelopeMeta, body: &[u8], received_at: i64) -> Result<(), RemoteError> {
        let g = self.0.lock().unwrap();
        g.as_ref().ok_or_else(|| local("LOCKED"))?.insert(m, body, received_at)
    }
}

/// Traduz o erro da sincronização para o app. Sessão expirada (401): esquece o token,
/// para a tela de conectar aparecer de novo.
pub fn sync_error_code(c: &Connection, e: &RemoteError) -> String {
    if matches!(e, RemoteError::Status(401, _)) {
        let _ = forget_session(c);
        return "NOT_AUTHENTICATED".into();
    }
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn connected(c: &Connection) {
        let cfg = RemoteConfig { url: "https://x".into(), nickname: "henri".into(), token: "tok".into() };
        save_connection(c, &cfg, &crypto::generate_keypair()).unwrap();
    }

    #[test]
    fn expired_session_forgets_token_and_says_not_authenticated() {
        let (_d, c) = test_conn();
        connected(&c);
        assert_eq!(sync_error_code(&c, &RemoteError::Status(401, "Entre de novo.".into())), "NOT_AUTHENTICATED");
        assert!(load_config(&c).unwrap().is_none());
        assert!(load_keypair(&c).unwrap().is_some()); // a chave fica
    }

    #[test]
    fn other_errors_keep_the_session() {
        let (_d, c) = test_conn();
        connected(&c);
        assert!(sync_error_code(&c, &RemoteError::Network("x".into())).starts_with("NETWORK"));
        assert_eq!(sync_error_code(&c, &RemoteError::KeyMismatch), "KEY_MISMATCH");
        assert!(load_config(&c).unwrap().is_some());
    }
}
