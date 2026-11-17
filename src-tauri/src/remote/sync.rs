//! Baixa os envelopes do servidor, decifra e grava no banco local; só então apaga no servidor.

use super::crypto::{seal_open, Keypair};
use crate::db::inbox;
use rusqlite::Connection;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EnvelopeMeta {
    pub id: String,
    pub kind: String,
    pub mime: String,
    pub size: i64,
    pub created_at: i64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RemoteError {
    #[error("NETWORK: {0}")]
    Network(String),
    #[error("{1}")]
    Status(u16, String),
    #[error("SYNC_PARTIAL: {0} ideia(s) não puderam ser recebidas")]
    Partial(usize),
    #[error("UNDECRYPTABLE: {0} ideia(s) foram cifradas para outra chave")]
    Undecryptable(usize),
    #[error("KEY_MISMATCH")]
    KeyMismatch,
    #[error("{0}")]
    Local(String),
}

pub trait InboxApi {
    fn list(&self) -> Result<Vec<EnvelopeMeta>, RemoteError>;
    fn fetch(&self, id: &str) -> Result<Vec<u8>, RemoteError>;
    fn delete(&self, id: &str) -> Result<(), RemoteError>;
    /** Chave pública que o servidor entrega ao celular (`None` se ainda não houver). */
    fn public_key(&self) -> Result<Option<[u8; 32]>, RemoteError>;
}

/// Acesso ao banco local, um passo de cada vez (não segura o banco durante a rede).
pub trait LocalInbox {
    fn has(&self, id: &str) -> Result<bool, RemoteError>;
    fn insert(&self, meta: &EnvelopeMeta, body: &[u8], received_at: i64) -> Result<(), RemoteError>;
}

impl LocalInbox for Connection {
    fn has(&self, id: &str) -> Result<bool, RemoteError> {
        inbox::inbox_has(self, id).map_err(|e| RemoteError::Local(e.to_string()))
    }
    fn insert(&self, m: &EnvelopeMeta, body: &[u8], received_at: i64) -> Result<(), RemoteError> {
        inbox::inbox_insert(self, &m.id, &m.kind, &m.mime, body, m.created_at, received_at)
            .map_err(|e| RemoteError::Local(e.to_string()))
    }
}

/// Devolve quantas ideias novas foram gravadas. Se alguma falhar, processa as outras
/// e só no fim devolve `Partial`.
pub fn sync(local: &dyn LocalInbox, api: &dyn InboxApi, kp: &Keypair, now: i64) -> Result<usize, RemoteError> {
    // Se o servidor entrega ao celular outra chave, o celular está cifrando para outro dono.
    if let Some(pk) = api.public_key()? {
        if pk != kp.public {
            return Err(RemoteError::KeyMismatch);
        }
    }
    let mut received = 0;
    let mut failed = 0;
    let mut undecryptable = 0;
    for meta in api.list()? {
        let step = || -> Result<bool, RemoteError> {
            let new = !local.has(&meta.id)?;
            if new {
                let sealed = api.fetch(&meta.id)?;
                let plain = seal_open(&sealed, kp).ok_or(RemoteError::Undecryptable(1))?;
                local.insert(&meta, &plain, now)?;
            }
            // só apaga no servidor depois de a ideia estar no banco local;
            // 404 = outra sincronização já apagou, então está feito
            match api.delete(&meta.id) {
                Ok(()) | Err(RemoteError::Status(404, _)) => Ok(new),
                Err(e) => Err(e),
            }
        };
        match step() {
            Ok(true) => received += 1,
            Ok(false) => {}
            Err(RemoteError::Undecryptable(_)) => undecryptable += 1,
            Err(_) => failed += 1,
        }
    }
    if undecryptable > 0 {
        Err(RemoteError::Undecryptable(undecryptable))
    } else if failed > 0 {
        Err(RemoteError::Partial(failed))
    } else {
        Ok(received)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use crate::remote::crypto::generate_keypair;
    use std::cell::RefCell;
    use std::collections::{BTreeMap, HashSet};

    #[derive(Default)]
    struct FakeApi {
        envelopes: RefCell<BTreeMap<String, (EnvelopeMeta, Vec<u8>)>>,
        fail_fetch: HashSet<String>,
        fail_delete: RefCell<HashSet<String>>,
        deleted: RefCell<Vec<String>>,
        gone_on_delete: RefCell<HashSet<String>>,
        server_key: RefCell<Option<[u8; 32]>>,
    }

    impl FakeApi {
        fn put(&self, id: &str, kind: &str, plain: &[u8], kp: &Keypair) {
            let pk = crypto_box::PublicKey::from(kp.public);
            let sealed = pk.seal(&mut rand::rngs::OsRng, plain).unwrap();
            let meta = EnvelopeMeta { id: id.into(), kind: kind.into(), mime: "text/plain".into(), size: sealed.len() as i64, created_at: 1 };
            self.envelopes.borrow_mut().insert(id.into(), (meta, sealed));
        }
    }

    impl InboxApi for FakeApi {
        fn list(&self) -> Result<Vec<EnvelopeMeta>, RemoteError> {
            Ok(self.envelopes.borrow().values().map(|(m, _)| m.clone()).collect())
        }
        fn fetch(&self, id: &str) -> Result<Vec<u8>, RemoteError> {
            if self.fail_fetch.contains(id) {
                return Err(RemoteError::Network("caiu".into()));
            }
            Ok(self.envelopes.borrow()[id].1.clone())
        }
        fn delete(&self, id: &str) -> Result<(), RemoteError> {
            if self.fail_delete.borrow().contains(id) {
                return Err(RemoteError::Network("caiu".into()));
            }
            if self.gone_on_delete.borrow().contains(id) {
                self.envelopes.borrow_mut().remove(id);
                return Err(RemoteError::Status(404, "Não encontrado.".into()));
            }
            self.envelopes.borrow_mut().remove(id);
            self.deleted.borrow_mut().push(id.into());
            Ok(())
        }
        fn public_key(&self) -> Result<Option<[u8; 32]>, RemoteError> {
            Ok(*self.server_key.borrow())
        }
    }

    #[test]
    fn two_valid_envelopes_are_saved_then_deleted() {
        let (_d, c) = test_conn();
        let kp = generate_keypair();
        let api = FakeApi::default();
        api.put("a", "text", b"ideia A", &kp);
        api.put("b", "text", b"ideia B", &kp);
        assert_eq!(sync(&c, &api, &kp, 99).unwrap(), 2);
        assert_eq!(*api.deleted.borrow(), ["a", "b"]);
        let texts: Vec<_> = inbox::inbox_list(&c).unwrap().into_iter().map(|i| i.text.unwrap()).collect();
        assert!(texts.contains(&"ideia A".to_string()) && texts.contains(&"ideia B".to_string()));
    }

    #[test]
    fn repeating_after_failed_delete_does_not_duplicate() {
        let (_d, c) = test_conn();
        let kp = generate_keypair();
        let api = FakeApi::default();
        api.put("a", "text", b"ideia A", &kp);
        api.fail_delete.borrow_mut().insert("a".into());
        assert!(matches!(sync(&c, &api, &kp, 1), Err(RemoteError::Partial(1))));
        api.fail_delete.borrow_mut().clear();
        assert_eq!(sync(&c, &api, &kp, 2).unwrap(), 0); // já estava salva: só apaga
        assert_eq!(*api.deleted.borrow(), ["a"]);
        assert_eq!(inbox::inbox_list(&c).unwrap().len(), 1);
    }

    #[test]
    fn failed_fetch_keeps_that_one_on_server_and_saves_the_rest() {
        let (_d, c) = test_conn();
        let kp = generate_keypair();
        let mut api = FakeApi::default();
        api.put("a", "text", b"A", &kp);
        api.put("b", "text", b"B", &kp);
        api.put("c", "text", b"C", &kp);
        api.fail_fetch.insert("b".into());
        assert!(matches!(sync(&c, &api, &kp, 1), Err(RemoteError::Partial(1))));
        assert_eq!(*api.deleted.borrow(), ["a", "c"]);
        assert!(api.envelopes.borrow().contains_key("b"));
        assert_eq!(inbox::inbox_list(&c).unwrap().len(), 2);
    }

    #[test]
    fn envelope_for_another_key_is_not_saved_nor_deleted() {
        let (_d, c) = test_conn();
        let kp = generate_keypair();
        let other = generate_keypair();
        let api = FakeApi::default();
        api.put("x", "text", b"de outra chave", &other);
        assert!(matches!(sync(&c, &api, &kp, 1), Err(RemoteError::Undecryptable(1))));
        assert!(api.deleted.borrow().is_empty());
        assert!(inbox::inbox_list(&c).unwrap().is_empty());
    }

    #[test]
    fn server_with_another_public_key_stops_before_touching_anything() {
        let (_d, c) = test_conn();
        let kp = generate_keypair();
        let api = FakeApi::default();
        api.put("a", "text", b"A", &kp);
        *api.server_key.borrow_mut() = Some(generate_keypair().public);
        assert_eq!(sync(&c, &api, &kp, 1), Err(RemoteError::KeyMismatch));
        assert!(api.deleted.borrow().is_empty());
        assert!(inbox::inbox_list(&c).unwrap().is_empty());
    }

    #[test]
    fn server_with_our_key_syncs_normally() {
        let (_d, c) = test_conn();
        let kp = generate_keypair();
        let api = FakeApi::default();
        api.put("a", "text", b"A", &kp);
        *api.server_key.borrow_mut() = Some(kp.public);
        assert_eq!(sync(&c, &api, &kp, 1), Ok(1));
    }

    #[test]
    fn delete_404_counts_as_done() {
        // outra sincronização já apagou: não é erro
        let (_d, c) = test_conn();
        let kp = generate_keypair();
        let api = FakeApi::default();
        api.put("a", "text", b"A", &kp);
        api.gone_on_delete.borrow_mut().insert("a".into());
        assert_eq!(sync(&c, &api, &kp, 1), Ok(1));
    }
}
