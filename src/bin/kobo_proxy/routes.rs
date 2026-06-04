use super::models::*;
use std::sync::Arc;
use axum::extract::{State, Json};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::http::header::CONTENT_TYPE;
use uuid::Uuid;
use vocabtrainer::VocabTrainerClient;
use crate::utils::build_simplified_state;

pub(crate) async fn handle_start(
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

pub(crate) async fn handle_answer(
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
                "progress": (100. * res.answer.progress.map(|p| p.progress.max(0.0)).unwrap_or(0.0)).round(),
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

pub(crate) async fn handle_hint(
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

pub(crate) async fn handle_next(
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

pub(crate) async fn health() -> StatusCode {
    StatusCode::OK
}

pub(crate) async fn get_index() -> Html<&'static str> {
    Html(include_str!("../../../static/index.html"))
}

pub(crate) async fn get_style() -> Response {
    let mut response = Response::new(axum::body::Body::from(include_str!(
        "../../../static/style.css"
    )));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("text/css"));
    response
}

pub(crate) async fn get_app() -> Response {
    let mut response = Response::new(axum::body::Body::from(include_str!("../../../static/app.js")));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/javascript"),
    );
    response
}
