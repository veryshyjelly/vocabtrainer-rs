use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use vocabtrainer::models::{ChallengeState, RawQuestion };
use vocabtrainer::{VocabTrainerClient};

// --- Simplified Kobo Payloads ---

#[derive(serde::Serialize)]
pub struct ParsedChoice {
    pub nonce: String,
    pub text: String,
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

// --- Server-Side Web State ---

struct AppState {
    sessions: RwLock<HashMap<String, VocabTrainerClient>>,
}

#[derive(serde::Deserialize)]
struct LoginPayload {
    email: Option<String>,
    password: Option<String>,
    guest: bool,
}

#[derive(serde::Deserialize)]
struct AnswerPayload {
    session_token: String,
    answer: String,
    response_time_ms: u64,
    secret: String,
}

#[derive(serde::Deserialize)]
struct HintPayload {
    session_token: String,
    secret: String,
}

#[derive(serde::Deserialize)]
struct NextPayload {
    session_token: String,
    secret: String,
}

// --- Endpoints ---

async fn handle_start(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    let client = match VocabTrainerClient::new(None) {
        Ok(c) => c,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build client").into_response()
        }
    };

    if !payload.guest {
        if let (Some(email), Some(password)) = (payload.email, payload.password) {
            if client.login(&email, &password).await.is_err() {
                return (StatusCode::UNAUTHORIZED, "Failed to login").into_response();
            }
        } else {
            return (StatusCode::BAD_REQUEST, "Missing credentials").into_response();
        }
    }

    let challenge = match client.start_challenge(None, None).await {
        Ok(state) => state,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("API Error: {:?}", e)).into_response(),
    };

    let session_token = Uuid::new_v4().to_string();
    state
        .sessions
        .write()
        .await
        .insert(session_token.clone(), client);

    let simplified = build_simplified_state(&session_token, &challenge);
    (StatusCode::OK, Json(simplified)).into_response()
}

async fn handle_answer(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AnswerPayload>,
) -> impl IntoResponse {
    let sessions = state.sessions.read().await;
    let client = match sessions.get(&payload.session_token) {
        Some(c) => c,
        None => return (StatusCode::NOT_FOUND, "Session expired").into_response(),
    };

    match client
        .save_answer(&payload.answer, payload.response_time_ms, &payload.secret)
        .await
    {
        Ok(res) => {
            let resp = serde_json::json!({
                "correct": res.answer.correct,
                "word": res.answer.word,
                "points_earned": res.answer.points,
                "definition": res.answer.sense.map(|s| s.definition).unwrap_or_default(),
                "context": res.answer.blurb.and_then(|b|
                    b.short.map(|x| x.replace("<i>", "").replace("</i>", ""))).unwrap_or_default(),
                "secret": res.secret,
                "total_points": res.pdata.and_then(|p| p.points).unwrap_or(0),
            });
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Error saving answer: {:?}", e),
        )
            .into_response(),
    }
}

async fn handle_hint(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HintPayload>,
) -> impl IntoResponse {
    let sessions = state.sessions.read().await;
    let client = match sessions.get(&payload.session_token) {
        Some(c) => c,
        None => return (StatusCode::NOT_FOUND, "Session expired").into_response(),
    };

    match client.get_hint("F", &payload.secret).await {
        Ok(res) => {
            let resp = serde_json::json!({
                "nonces_to_remove": res.nonces.unwrap_or_default(),
                "secret": res.secret
            });
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, format!("Hint failed: {:?}", e)).into_response(),
    }
}

async fn handle_next(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NextPayload>,
) -> impl IntoResponse {
    let sessions = state.sessions.read().await;
    let client = match sessions.get(&payload.session_token) {
        Some(c) => c,
        None => return (StatusCode::NOT_FOUND, "Session expired").into_response(),
    };

    match client.next_question(&payload.secret).await {
        Ok(challenge) => {
            let simplified = build_simplified_state(&payload.session_token, &challenge);
            (StatusCode::OK, Json(simplified)).into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Error going to next question: {:?}", e),
        )
            .into_response(),
    }
}

// --- HTML Parsing Helpers using Reused Library Decoders ---

fn parse_text(document: &scraper::Html, selector_str: &str) -> String {
    let selector = scraper::Selector::parse(selector_str).unwrap();
    if let Some(element) = document.select(&selector).next() {
        element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    } else {
        String::new()
    }
}

fn parse_choices(document: &scraper::Html) -> Vec<ParsedChoice> {
    let fallback_selectors = ["div.choices a", "a.choice", "a[accesskey]"];
    for selector_str in fallback_selectors {
        let selector = scraper::Selector::parse(selector_str).unwrap();
        let mut choices = Vec::new();
        for element in document.select(&selector) {
            if let Some(nonce) = element.value().attr("data-nonce") {
                let text = element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                if !nonce.is_empty() {
                    choices.push(ParsedChoice {
                        nonce: nonce.to_string(),
                        text,
                    });
                }
            }
        }
        if !choices.is_empty() {
            return choices;
        }
    }
    Vec::new()
}

fn build_simplified_state(session_token: &str, challenge: &ChallengeState) -> SimplifiedState {
    let default_raw = RawQuestion {
        code: "".to_string(),
        difficulty: 0.0,
        answerstats: None,
        hints: vec![],
        turn: 0,
        question_type: "D".to_string(),
    };
    let q = challenge.question.as_ref().unwrap_or(&default_raw);

    // Re-use library Client directly to translate the question code [Citations: index-1mp2a04.css]
    let client_builder = VocabTrainerClient::new(None).unwrap();
    let raw_html = client_builder
        .decode_question_html(&q.code)
        .unwrap_or_default();
    let document = scraper::Html::parse_document(&raw_html);

    let mut prompt = parse_text(&document, "div.instructions");
    let content = parse_text(&document, "div.sentence");
    if !content.is_empty() {
        prompt = format!("{}\n{}", prompt, content);
    }
    prompt = prompt.trim().into();

    let is_spelling = q.question_type == "T";
    let choices = if is_spelling {
        vec![]
    } else {
        parse_choices(&document)
    };

    SimplifiedState {
        session_token: session_token.to_string(),
        is_spelling,
        prompt,
        choices,
        hints: q.hints.clone(),
        secret: challenge.secret.clone(),
        total_points: challenge.pdata.as_ref().and_then(|p| p.points).unwrap_or(0),
        streak: challenge.round.as_ref().map(|r| r.streak).unwrap_or(0),
        level_name: challenge
            .pdata
            .as_ref()
            .and_then(|p| p.level.as_ref())
            .and_then(|l| l.name.clone())
            .unwrap_or_else(|| "Novice".to_string()),
    }
}

// --- Embedded Static Assets ---

async fn get_index() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}

async fn get_style() -> Response {
    let mut response = Response::new(axum::body::Body::from(include_str!(
        "../../static/style.css"
    )));
    response
        .headers_mut()
        .insert("CONTENT_TYPE", HeaderValue::from_static("text/css"));
    response
}

async fn get_app() -> Response {
    let mut response = Response::new(axum::body::Body::from(include_str!("../../static/app.js")));
    response.headers_mut().insert(
        "CONTENT_TYPE",
        HeaderValue::from_static("application/javascript"),
    );
    response
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        sessions: RwLock::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/", get(get_index))
        .route("/style.css", get(get_style))
        .route("/app.js", get(get_app))
        .route("/api/start", post(handle_start))
        .route("/api/answer", post(handle_answer))
        .route("/api/hint", post(handle_hint))
        .route("/api/next", post(handle_next))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("[+] Server running locally on http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}
