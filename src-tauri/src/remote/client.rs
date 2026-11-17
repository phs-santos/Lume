//! Cliente HTTP do servidor do Lume.

use super::sync::{EnvelopeMeta, InboxApi, RemoteError};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::blocking::{Client, Response};
use serde_json::{json, Value};
use std::time::Duration;

fn client() -> Client {
    Client::builder().timeout(Duration::from_secs(60)).build().expect("cliente http")
}

fn net(e: reqwest::Error) -> RemoteError {
    RemoteError::Network(e.to_string())
}

/// Converte respostas de erro em `Status(código, mensagem do servidor)`.
fn check(r: Response) -> Result<Response, RemoteError> {
    if r.status().is_success() {
        return Ok(r);
    }
    let code = r.status().as_u16();
    let msg = r.json::<Value>().ok().and_then(|v| v["error"].as_str().map(String::from)).unwrap_or_else(|| format!("Erro {code}"));
    Err(RemoteError::Status(code, msg))
}

pub fn base(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

pub fn get_salt(url: &str, nickname: &str) -> Result<Vec<u8>, RemoteError> {
    let v: Value = check(client().get(format!("{}/api/accounts/{}/salt", base(url), nickname)).send().map_err(net)?)?.json().map_err(net)?;
    STANDARD.decode(v["salt"].as_str().unwrap_or_default()).map_err(|e| RemoteError::Local(e.to_string()))
}

pub fn register(url: &str, nickname: &str, invite: &str, salt: &[u8], secret: &[u8]) -> Result<String, RemoteError> {
    let body = json!({"nickname": nickname, "invite": invite, "salt": STANDARD.encode(salt), "secret": STANDARD.encode(secret)});
    let v: Value = check(client().post(format!("{}/api/accounts", base(url))).json(&body).send().map_err(net)?)?.json().map_err(net)?;
    Ok(v["token"].as_str().unwrap_or_default().to_string())
}

pub fn login(url: &str, nickname: &str, secret: &[u8]) -> Result<String, RemoteError> {
    let body = json!({"nickname": nickname, "secret": STANDARD.encode(secret)});
    let v: Value = check(client().post(format!("{}/api/sessions", base(url))).json(&body).send().map_err(net)?)?.json().map_err(net)?;
    Ok(v["token"].as_str().unwrap_or_default().to_string())
}

pub fn put_public_key(url: &str, token: &str, public: &[u8; 32]) -> Result<(), RemoteError> {
    check(
        client()
            .put(format!("{}/api/accounts/me/public-key", base(url)))
            .bearer_auth(token)
            .json(&json!({"public_key": STANDARD.encode(public)}))
            .send()
            .map_err(net)?,
    )?;
    Ok(())
}

pub struct HttpApi {
    pub url: String,
    pub token: String,
}

impl InboxApi for HttpApi {
    fn list(&self) -> Result<Vec<EnvelopeMeta>, RemoteError> {
        check(client().get(format!("{}/api/inbox", base(&self.url))).bearer_auth(&self.token).send().map_err(net)?)?
            .json()
            .map_err(net)
    }
    fn fetch(&self, id: &str) -> Result<Vec<u8>, RemoteError> {
        let r = check(client().get(format!("{}/api/inbox/{id}", base(&self.url))).bearer_auth(&self.token).send().map_err(net)?)?;
        Ok(r.bytes().map_err(net)?.to_vec())
    }
    fn delete(&self, id: &str) -> Result<(), RemoteError> {
        check(client().delete(format!("{}/api/inbox/{id}", base(&self.url))).bearer_auth(&self.token).send().map_err(net)?)?;
        Ok(())
    }
}
