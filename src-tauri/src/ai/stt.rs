//! Transcrição de áudio pela Inworld (STT). Literal: nada além de `trim`.

use super::provider::ProviderError;
use serde_json::Value;

pub const STT_URL: &str = "https://api.inworld.ai/stt/v1/transcribe";
pub const STT_MODEL: &str = "inworld/inworld-stt-1";
/// Limite da chamada síncrona da Inworld.
pub const STT_MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub text: String,
    pub audio_ms: i64,
}

pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio: &[u8], mime: &str, vocabulary: &[String]) -> Result<Transcript, ProviderError>;
}

pub fn stt_config(vocabulary: &[String]) -> Value {
    let mut c = serde_json::json!({
        "modelId": STT_MODEL,
        // o celular manda mp4 (iPhone) ou webm (Android); a Inworld lê o codec do arquivo
        "audioEncoding": "AUTO_DETECT",
        "language": "pt",
    });
    if !vocabulary.is_empty() {
        c["prompts"] = serde_json::json!(vocabulary);
    }
    c
}

pub fn parse_stt_response(body: &str) -> Result<Transcript, ProviderError> {
    let v: Value = serde_json::from_str(body).map_err(|e| ProviderError::Bad(format!("resposta inválida: {e}")))?;
    let text = v["transcription"]["transcript"].as_str().unwrap_or("").trim().to_string();
    let ms = &v["usage"]["transcribedAudioMs"];
    let audio_ms = ms.as_i64().or_else(|| ms.as_str().and_then(|s| s.parse().ok())).unwrap_or(0);
    Ok(Transcript { text, audio_ms })
}

pub struct InworldStt {
    pub key: String,
}

impl Transcriber for InworldStt {
    fn transcribe(&self, audio: &[u8], mime: &str, vocabulary: &[String]) -> Result<Transcript, ProviderError> {
        use reqwest::blocking::multipart::{Form, Part};
        let net = |e: reqwest::Error| ProviderError::Network(e.to_string());
        let config = Part::text(stt_config(vocabulary).to_string()).mime_str("application/json").map_err(net)?;
        let file = Part::bytes(audio.to_vec()).file_name("ideia").mime_str(mime.split(';').next().unwrap_or(mime)).map_err(net)?;
        // a configuração vai antes do arquivo, como a Inworld pede
        let form = Form::new().part("transcribeConfig", config).part("file", file);
        let res = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(net)?
            .post(STT_URL)
            .header("Authorization", format!("Basic {}", self.key.trim()))
            .multipart(form)
            .send()
            .map_err(net)?;
        let status = res.status().as_u16();
        let body = res.text().map_err(net)?;
        if !(200..300).contains(&status) {
            return Err(super::provider::status_error(status, &body));
        }
        parse_stt_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_literal_portuguese_with_vocabulary() {
        let c = stt_config(&["Teodoro".into(), "Itaipava".into()]);
        assert_eq!(c["modelId"], STT_MODEL);
        assert_eq!(c["audioEncoding"], "AUTO_DETECT");
        assert_eq!(c["language"], "pt");
        assert_eq!(c["prompts"], serde_json::json!(["Teodoro", "Itaipava"]));
    }

    #[test]
    fn config_without_vocabulary_has_no_prompts() {
        assert!(stt_config(&[]).get("prompts").is_none());
    }

    #[test]
    fn parses_text_and_duration_trimmed() {
        let t = parse_stt_response(r#"{"transcription":{"transcript":"  A Marina abre a carta. ","isFinal":true},"usage":{"transcribedAudioMs":4210,"modelId":"inworld/inworld-stt-1"}}"#).unwrap();
        assert_eq!(t, Transcript { text: "A Marina abre a carta.".into(), audio_ms: 4210 });
    }

    #[test]
    fn missing_usage_means_zero_ms_and_missing_text_means_empty() {
        assert_eq!(parse_stt_response(r#"{"transcription":{"transcript":"oi"}}"#).unwrap().audio_ms, 0);
        assert_eq!(parse_stt_response(r#"{"transcription":{}}"#).unwrap().text, "");
    }

    #[test]
    fn usage_ms_may_come_as_string() {
        // a Inworld serializa int64 como string em algumas respostas
        assert_eq!(parse_stt_response(r#"{"transcription":{"transcript":"x"},"usage":{"transcribedAudioMs":"1500"}}"#).unwrap().audio_ms, 1500);
    }

    #[test]
    fn broken_json_is_bad() {
        assert!(matches!(parse_stt_response("<html>"), Err(ProviderError::Bad(_))));
    }

    #[test]
    #[ignore]
    fn live_inworld_stt() {
        let key = std::env::var("INWORLD_KEY").expect("INWORLD_KEY");
        let audio = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../test-fixtures/fala-pt.m4a")).unwrap();
        let t = InworldStt { key }.transcribe(&audio, "audio/mp4", &["Teodoro".into()]).unwrap();
        assert!(t.text.contains("Marina"), "{t:?}");
    }
}
