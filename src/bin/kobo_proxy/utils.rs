use super::models::{ParsedChoice, SimplifiedState};
use vocabtrainer::models::{ChallengeState, RawQuestion };
use vocabtrainer::{VocabTrainerClient};

pub(crate) fn parse_text(document: &scraper::Html, selector_str: &str) -> String {
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

pub(crate) fn parse_choices(document: &scraper::Html) -> Vec<ParsedChoice> {
    let fallback_selectors = ["div.choices a", "a.choice", "a[accesskey]"];
    for selector_str in fallback_selectors {
        let selector = scraper::Selector::parse(selector_str).unwrap();
        let mut choices = Vec::new();
        for element in document.select(&selector) {
            if let Some(nonce) = element.value().attr("data-nonce") {
                let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();

                let mut image_url = None;
                if let Some(style_attr) = element.value().attr("style") {
                    image_url = extract_image_url(style_attr);
                }

                if !nonce.is_empty() {
                    choices.push(ParsedChoice {
                        nonce: nonce.to_string(),
                        text,
                        image_url
                    });
                }
            }
        }
        if !choices.is_empty() { return choices; }
    }
    Vec::new()
}

pub(crate) fn parse_image(document: &scraper::Html, selector_str: &str) -> Option<String> {
    let selector = scraper::Selector::parse(selector_str).unwrap();
    if let Some(element) = document.select(&selector).next() {
        if let Some(style_attr) = element.value().attr("style") {
            extract_image_url(style_attr)
        } else {
            None
        }
    } else {
        None
    }
}


/// Helper to extract raw image URLs from inline background-image style definitions
pub(crate) fn extract_image_url(style: &str) -> Option<String> {
    if let Some(start_idx) = style.find("url(") {
        let sub = &style[start_idx + 4..];
        let clean_sub = sub.trim_start_matches('\'').trim_start_matches('"');
        if let Some(end_idx) = clean_sub.find(|c| c == '\'' || c == '"' || c == ')') {
            return Some(clean_sub[..end_idx].to_string());
        }
    }
    None
}

pub(crate) fn build_simplified_state(session_token: &str, challenge: &ChallengeState) -> SimplifiedState {
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

    let image_url = parse_image(&document, "div.questionContent");
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
        image_url,
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
