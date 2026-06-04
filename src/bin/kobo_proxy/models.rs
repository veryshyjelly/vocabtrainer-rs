use std::{collections::HashMap};
use tokio::sync::RwLock;

use vocabtrainer::{VocabTrainerClient};

#[derive(serde::Serialize)]
pub struct ParsedChoice {
    pub nonce: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}


#[derive(serde::Serialize)]
pub struct SimplifiedState {
    pub session_token: String,
    pub is_spelling: bool,
    pub prompt: String,
    pub choices: Vec<ParsedChoice>,
    pub hints: Vec<String>,
    pub secret: String,
    pub total_points: u64,
    pub streak: u32,
    pub level_name: String,
}

pub(crate) struct AppState {
    pub sessions: RwLock<HashMap<String, VocabTrainerClient>>,
}

#[derive(serde::Deserialize)]
pub(crate) struct LoginPayload {
    pub email: Option<String>,
    pub password: Option<String>,
    pub guest: bool,
}

#[derive(serde::Deserialize)]
pub(crate) struct AnswerPayload {
    pub session_token: String,
    pub answer: String,
    pub response_time_ms: u64,
    pub secret: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct HintPayload {
    pub session_token: String,
    pub secret: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct NextPayload {
    pub session_token: String,
    pub secret: String,
}
