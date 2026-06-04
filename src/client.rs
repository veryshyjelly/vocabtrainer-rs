use crate::error::VocabError;
use crate::models::{ChallengeState, HintResponse, SaveAnswerResponse};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};

const DEFAULT_BASE_URL: &str = "https://www.vocabulary.com";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct VocabTrainerClient {
    http_client: reqwest::Client,
    base_url: String,
}

impl VocabTrainerClient {
    /// Initializes a generic trainer client with a built-in session cookie store
    pub fn new(custom_base_url: Option<String>) -> Result<Self, VocabError> {
        let base_url = custom_base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));

        let http_client = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http_client,
            base_url,
        })
    }

    /// Programmatic login function to authenticate credentials via the standard form endpoint
    pub async fn login(&self, username: &str, password: &str) -> Result<(), VocabError> {
        let url = format!("{}/login/vocabtrainer", self.base_url);
        let form_data = [
            ("username", username),
            ("password", password),
            ("autoLogon", "true"),
        ];

        let response = self.http_client.post(&url).form(&form_data).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(VocabError::BadStatus(response.status()))
        }
    }

    /// Initiates a standard game session
    pub async fn start_challenge(
        &self,
        wordlist_id: Option<&str>,
        saved_secret: Option<&str>,
    ) -> Result<ChallengeState, VocabError> {
        let url = format!("{}/challenge/start.json", self.base_url);

        let mut form_params = vec![
            ("activitytype".to_string(), "c".to_string()),
            ("v".to_string(), "4".to_string()),
        ];

        if let Some(id) = wordlist_id {
            form_params.push(("wordlistid".to_string(), id.to_string()));
        }
        if let Some(sec) = saved_secret {
            form_params.push(("secret".to_string(), sec.to_string()));
        }

        let body_payload = serde_urlencoded::to_string(&form_params)?;

        let response = self
            .http_client
            .post(&url)
            .header(
                CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(body_payload)
            .send()
            .await?;

        self.handle_response::<ChallengeState>(response).await
    }

    /// Evaluates and submits a chosen option index or text answer
    pub async fn save_answer(
        &self,
        answer: &str,
        response_time_ms: u64,
        secret: &str,
    ) -> Result<SaveAnswerResponse, VocabError> {
        let url = format!("{}/challenge/saveanswer.json", self.base_url);

        let form_params = [
            ("a", answer),
            ("rt", &response_time_ms.to_string()),
            ("secret", secret),
            ("v", "4"),
        ];

        let body_payload = serde_urlencoded::to_string(&form_params)?;

        let response = self
            .http_client
            .post(&url)
            .header(
                CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(body_payload)
            .send()
            .await?;

        self.handle_response::<SaveAnswerResponse>(response).await
    }

    /// Requests a dynamic hint
    pub async fn get_hint(
        &self,
        hint_type: &str,
        secret: &str,
    ) -> Result<HintResponse, VocabError> {
        let url = format!("{}/challenge/hint.json", self.base_url);

        let form_params = [("type", hint_type), ("secret", secret), ("v", "4")];

        let body_payload = serde_urlencoded::to_string(&form_params)?;

        let response = self
            .http_client
            .post(&url)
            .header(
                CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(body_payload)
            .send()
            .await?;

        self.handle_response::<HintResponse>(response).await
    }

    /// Requests the next play card once an evaluation has finished
    pub async fn next_question(&self, secret: &str) -> Result<ChallengeState, VocabError> {
        let url = format!("{}/challenge/nextquestion.json", self.base_url);

        let form_params = [("secret", secret), ("v", "4")];

        let body_payload = serde_urlencoded::to_string(&form_params)?;

        let response = self
            .http_client
            .post(&url)
            .header(
                CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(body_payload)
            .send()
            .await?;

        self.handle_response::<ChallengeState>(response).await
    }

    /// Utility method that decodes the base64-obfuscated question layout
    pub fn decode_question_html(&self, obfuscated_code: &str) -> Result<String, VocabError> {
        let decoded_bytes = STANDARD
            .decode(obfuscated_code)
            .map_err(|e| VocabError::DecodeError(e.to_string()))?;

        String::from_utf8(decoded_bytes).map_err(|e| VocabError::DecodeError(e.to_string()))
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, VocabError> {
        let status = response.status();
        let body_text = response.text().await?;

        #[cfg(debug_assertions)]
        println!("RESPONSE_TEXT {:?}", body_text);

        if status.is_success() {
            // Check if backend returned a standard API exception wrapper
            if let Ok(error_val) = serde_json::from_str::<serde_json::Value>(&body_text) {
                if let Some(err_type) = error_val.get("error") {
                    let message = error_val
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown exception occurred")
                        .to_string();
                    return Err(VocabError::ApiError {
                        error_type: err_type.to_string(),
                        message,
                    });
                }
            }
            let parsed_payload = serde_json::from_str::<T>(&body_text)?;
            Ok(parsed_payload)
        } else {
            Err(VocabError::BadStatus(status))
        }
    }
}
