//! Verificação manual contra um servidor local: conecta, envia uma ideia cifrada e sincroniza.
//! Uso: CHECK_PASSWORD=... cargo run --example remote_check -- <url> <apelido>
use lume_lib::{db, remote, vault::MasterKey};
use std::sync::{Arc, Mutex};

fn main() {
    let mut a = std::env::args().skip(1);
    let (url, nick) = (a.next().unwrap(), a.next().unwrap());
    let pw = std::env::var("CHECK_PASSWORD").unwrap();
    let (cfg, kp) = remote::connect(&url, &nick, &pw, None, remote::crypto::generate_keypair()).expect("conectar");
    println!("conectado como {} em {}", cfg.nickname, cfg.url);

    // simula o celular: lacra uma ideia para a chave pública e envia
    let sealed = crypto_box::PublicKey::from(kp.public).seal(&mut rand::rngs::OsRng, "Ideia do teste: o farol acende sozinho.".as_bytes()).unwrap();
    let r = reqwest::blocking::Client::new()
        .post(format!("{}/api/inbox", cfg.url))
        .bearer_auth(&cfg.token)
        .header("x-lume-kind", "text")
        .header("content-type", "text/plain;charset=utf-8")
        .body(sealed)
        .send()
        .unwrap();
    println!("envio: {}", r.status());

    let dir = tempfile::tempdir().unwrap();
    let c = db::open(&dir.path().join("t.db"), &MasterKey([3; 32])).unwrap();
    let shared = Arc::new(Mutex::new(Some(c)));
    let api = remote::client::HttpApi { url: cfg.url.clone(), token: cfg.token.clone() };
    let n = remote::sync::sync(&remote::SharedLocal(shared.clone()), &api, &kp, 1);
    let g = shared.lock().unwrap();
    let items = db::inbox::inbox_list(g.as_ref().unwrap()).unwrap();
    println!("resultado: {n:?}; textos: {:?}", items.iter().filter_map(|i| i.text.clone()).collect::<Vec<_>>());
    println!("restam no servidor: {}", api_list_len(&cfg));
}

fn api_list_len(cfg: &remote::RemoteConfig) -> usize {
    use remote::sync::InboxApi;
    remote::client::HttpApi { url: cfg.url.clone(), token: cfg.token.clone() }.list().unwrap().len()
}
