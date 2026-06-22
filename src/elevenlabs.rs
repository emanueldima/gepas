use {
    anyhow::{Context, Result, bail},
    reqwest::{Url, blocking::Client, header::HeaderMap},
    serde_json::{Value, json},
    std::time::Duration,
};

pub struct ElevenLabs {
    client: Client,
    api_key: String,
}

pub struct SynthesisRequest<'a> {
    pub voice_id: &'a str,
    pub text: &'a str,
    pub model: &'a str,
    pub language_code: &'a str,
    pub output_format: &'a str,
    pub speed: f64,
}

pub struct SynthesisResult {
    pub audio: Vec<u8>,
    pub character_count: Option<String>,
    pub request_id: Option<String>,
}

mod private {
    use super::*;

    impl ElevenLabs {
        pub fn new(api_key: String) -> Result<Self> {
            Ok(Self {
                client: Client::builder()
                    .timeout(Duration::from_secs(120))
                    .build()
                    .context("failed to create ElevenLabs HTTP client")?,
                api_key,
            })
        }

        pub fn synthesize_pcm(&self, request: SynthesisRequest<'_>) -> Result<SynthesisResult> {
            let (url, body) = build_request(&request)?;
            let response = self
                .client
                .post(url)
                .header("xi-api-key", &self.api_key)
                .json(&body)
                .send()
                .context("failed to call the ElevenLabs text-to-speech API")?;
            let status = response.status();
            if !status.is_success() {
                let body = response
                    .text()
                    .context("failed to read the ElevenLabs error response")?;
                bail!("ElevenLabs API returned {status}: {body}");
            }

            let (character_count, request_id) = response_metadata(response.headers());
            let audio = response
                .bytes()
                .context("failed to read ElevenLabs audio")?
                .to_vec();
            validate_pcm(&audio)?;
            Ok(SynthesisResult {
                audio,
                character_count,
                request_id,
            })
        }
    }

    fn build_request(request: &SynthesisRequest<'_>) -> Result<(Url, Value)> {
        let mut url = Url::parse("https://api.elevenlabs.io/v1/text-to-speech")?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("ElevenLabs API URL cannot contain path segments"))?
            .push(request.voice_id);
        url.query_pairs_mut()
            .append_pair("output_format", request.output_format);

        Ok((
            url,
            json!({
                "text": request.text,
                "model_id": request.model,
                "language_code": request.language_code,
                "voice_settings": {
                    "speed": request.speed
                }
            }),
        ))
    }

    fn validate_pcm(audio: &[u8]) -> Result<()> {
        if audio.is_empty() {
            bail!("ElevenLabs returned empty PCM audio");
        }
        if !audio.len().is_multiple_of(2) {
            bail!("ElevenLabs returned an odd number of bytes for 16-bit PCM audio");
        }
        Ok(())
    }

    fn response_metadata(headers: &HeaderMap) -> (Option<String>, Option<String>) {
        let value = |name| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        (value("x-character-count"), value("request-id"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn request_propagates_voice_language_model_format_and_speed() {
            let (url, body) = build_request(&SynthesisRequest {
                voice_id: "voice/id",
                text: "Guten Morgen.",
                model: "test-model",
                language_code: "de",
                output_format: "pcm_24000",
                speed: 0.9,
            })
            .unwrap();

            assert_eq!(url.path(), "/v1/text-to-speech/voice%2Fid");
            assert_eq!(url.query(), Some("output_format=pcm_24000"));
            assert_eq!(body["text"], "Guten Morgen.");
            assert_eq!(body["model_id"], "test-model");
            assert_eq!(body["language_code"], "de");
            assert_eq!(body["voice_settings"]["speed"], 0.9);
        }

        #[test]
        fn pcm_validation_rejects_empty_and_odd_responses() {
            assert!(validate_pcm(&[]).unwrap_err().to_string().contains("empty"));
            assert!(validate_pcm(&[0]).unwrap_err().to_string().contains("odd"));
            assert!(validate_pcm(&[0, 1]).is_ok());
        }

        #[test]
        fn response_metadata_includes_usage_and_request_id() {
            let mut headers = HeaderMap::new();
            headers.insert("x-character-count", "42".parse().unwrap());
            headers.insert("request-id", "request-123".parse().unwrap());

            assert_eq!(
                response_metadata(&headers),
                (Some("42".into()), Some("request-123".into()))
            );
        }
    }
}
