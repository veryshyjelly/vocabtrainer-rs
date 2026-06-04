use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Level {
    pub id: Option<String>,
    pub name: Option<String>,
    pub milestone: Option<u32>,
    pub progress: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListProgress {
    pub current: bool,
    #[serde(rename = "listId")]
    pub list_id: u64,
    pub name: String,
    pub priority: Option<i32>,
    pub progress: Option<f64>,
    pub wordcount: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileData {
    pub lists: Option<Vec<ListProgress>>,
    pub level: Option<Level>,
    pub nummastered: Option<u32>,
    pub numplayed: Option<u32>,
    pub points: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Round {
    pub number: u32,
    pub played_count: u32,
    pub streak: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RawQuestion {
    pub code: String, // Base64 HTML question payload
    pub difficulty: f64,
    #[serde(default)]
    pub answerstats: Option<AnswerStats>,
    #[serde(default)]
    pub hints: Vec<String>,
    pub turn: u32,
    #[serde(rename = "type")]
    pub question_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnswerStats {
    pub correct: u32,
    pub total: u32,
}

/// Returned by `/challenge/start.json` and `/challenge/nextquestion.json`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChallengeState {
    pub action: String,
    pub pdata: Option<ProfileData>,
    pub question: Option<RawQuestion>,
    pub round: Option<Round>,
    pub secret: String,
    pub v: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blurb {
    pub long: Option<String>,
    pub short: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Frequency {
    pub band: u32,
    pub rank: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgressStats {
    pub correct_count: u32,
    pub incorrect_count: u32,
    pub play_count: u32,
    pub played_at: Option<String>,
    pub priority: i32,
    pub progress: f64,
    pub scheduled_at: Option<String>,
    pub value: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PronunciationVariant {
    #[serde(rename = "audioId")]
    pub audio_id: Option<String>,
    pub ipa: String,
    pub region: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WordSense {
    pub audio: Vec<String>,
    pub definition: String,
    pub id: u64,
    pub ordinal: String,
    #[serde(rename = "part_of_speech")]
    pub part_of_speech: String,
    pub pos: String,
    #[serde(rename = "videoHost")]
    pub video_host: Option<String>,
    #[serde(rename = "imageQuestionRoot")]
    pub image_question_root: Option<String>,
    #[serde(rename = "pronunciationVariants")]
    pub pronunciation_variants: Option<Vec<PronunciationVariant>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnswerDetails {
    pub accepted_answers: Option<Vec<String>>,
    pub achievements: Option<Vec<serde_json::Value>>,
    pub blurb: Option<Blurb>,
    pub bonus: u32,
    pub correct: bool,
    pub correct_choice: Option<u32>,
    pub frequency: Option<Frequency>,
    pub hints: serde_json::Value,
    pub lists_mastered: Option<Vec<u64>>,
    pub lists_progress: Option<Vec<u64>>,
    pub pagefreq: Option<f64>,
    pub played_at: String,
    pub points: u32,
    pub progress: Option<ProgressStats>,
    pub question_mode: String,
    pub question_type: String,
    pub response_time: u64,
    pub round_streak: u32,
    pub round_turn: u32,
    pub sense: Option<WordSense>,
    pub session_time: u64,
    pub streak: u32,
    pub turn: u32,
    pub user_choice: Option<serde_json::Value>,
    pub word: String,
}

/// Returned by `/challenge/saveanswer.json`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveAnswerResponse {
    pub answer: AnswerDetails,
    pub pdata: Option<ProfileData>,
    pub round: Option<Round>,
    pub secret: String,
}

/// Returned by `/challenge/hint.json`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HintResponse {
    pub def: Option<String>,
    pub nonces: Option<Vec<String>>,
    pub secret: String,
    pub word: Option<String>,
    pub pdata: Option<ProfileData>,
}
