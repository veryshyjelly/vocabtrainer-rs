use inquire::{Password, PasswordDisplayMode, Select, Text};
use regex::Regex;
use scraper::{Html, Selector};
use std::time::Instant;
use vocabtrainer::{models::SaveAnswerResponse, VocabTrainerClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("======================================================");
    println!("          Vocabulary.com VocabTrainer CLI             ");
    println!("======================================================");

    let session_modes = vec![
        "Play Anonymously (Guest Session)",
        "Log in with Email & Password",
    ];
    let mode_choice = Select::new("Select session type:", session_modes).prompt()?;

    let client = VocabTrainerClient::new(None)?;

    // Handle credentials-based login directly [Citations: index-1mp2a04.css]
    if mode_choice.starts_with("Log in") {
        let email = Text::new("Email address:").prompt()?;
        let password = Password::new("Password:")
            .with_display_mode(PasswordDisplayMode::Masked)
            .prompt()?;

        println!("[*] Logging in...");
        client.login(&email, &password).await?;
        println!("[+] Successfully authenticated.");
    }


    println!("\n[*] Connecting to VocabTrainer...");
    let challenge = client.start_challenge(None, None).await?;

    println!("[+] Handshake successful.");
    let mut active_secret = challenge.secret;
    let mut current_question = challenge.question;

    // Main Gameplay Loop
    loop {
        let question = match current_question.clone() {
            Some(q) => q,
            None => {
                println!(
                    "[-] No more questions available. The session may have reached a trial limit."
                );
                break;
            }
        };

        // Decode HTML structure
        let raw_html = client.decode_question_html(&question.code)?;
        let document = Html::parse_document(&raw_html);

        // Extract and combine any instructional or contextual text
        let mut prompt_text = parse_text(&document, "div.instructions");
        let content_text = parse_text(&document, "div.questionContent");
        if !content_text.is_empty() {
            prompt_text = format!("{}\n{}", prompt_text, content_text);
        }
        if prompt_text.is_empty() {
            prompt_text = "Select the option that best fits:".to_string();
        }

        println!("QUESTION_TYPE = {}", question.question_type);

        if question.question_type == "T" {
            // --- Spelling Question Mode ---
            println!("\n=================== SPELLING QUESTION ===================");
            println!("{}", prompt_text);
            println!("=========================================================");

            let timer = Instant::now();
            let user_answer = Text::new("Your spelling guess:").prompt()?;
            let elapsed_time = timer.elapsed().as_millis() as u64;

            println!("\n[*] Validating answer...");
            let save_res = client
                .save_answer(&user_answer, elapsed_time, &active_secret)
                .await?;
            print_answer_result(&save_res);

            active_secret = save_res.secret;
            current_question = None; // Reset until next request is processed

            let _ = Text::new("Press [Enter] to load next question...").prompt()?;
            let next_state = client.next_question(&active_secret).await?;
            active_secret = next_state.secret;
            current_question = next_state.question;
        } else {
            // --- Multiple Choice Question Mode ---
            println!("\n=================== CHOICE QUESTION ===================");
            println!("{}", prompt_text);
            println!("=========================================================");

            let mut choices = parse_choices(&document);
            let mut used_hint = false;

            loop {
                let mut options_to_show: Vec<String> =
                    choices.iter().map(|(_, text)| text.clone()).collect();

                // Add Hint Option if available and not yet used
                if !used_hint && question.hints.contains(&"F".to_string()) {
                    options_to_show.push("[Use 50/50 Hint (Costs 50% points)]".to_string());
                }

                let timer = Instant::now();
                let selection = Select::new("Choose an option:", options_to_show).prompt()?;
                let elapsed_time = timer.elapsed().as_millis() as u64;

                if selection.starts_with("[Use 50/50 Hint") {
                    println!("[*] Contacting server for hints...");
                    match client.get_hint("F", &active_secret).await {
                        Ok(hint_res) => {
                            active_secret = hint_res.secret; // Sync with new state secret
                            used_hint = true;
                            if let Some(nonces_to_remove) = hint_res.nonces {
                                println!("[+] Hint applied. Two incorrect choices removed.");
                                choices.retain(|(nonce, _)| !nonces_to_remove.contains(nonce));
                            }
                        }
                        Err(e) => {
                            println!("[-] Hint failed: {}", e);
                        }
                    }
                    continue; // Re-prompt choices loop
                } else {
                    // Match choice to retrieve its target data-nonce value
                    if let Some((nonce, _)) = choices.iter().find(|(_, text)| text == &selection) {
                        println!("\n[*] Evaluating option...");
                        let save_res = client
                            .save_answer(nonce, elapsed_time, &active_secret)
                            .await?;
                        print_answer_result(&save_res);

                        active_secret = save_res.secret;

                        let _ = Text::new("Press [Enter] to load next question...").prompt()?;
                        let next_state = client.next_question(&active_secret).await?;
                        active_secret = next_state.secret;
                        current_question = next_state.question;
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Parses specific CSS selector contents from HTML document structures
fn parse_text(document: &Html, selector_str: &str) -> String {
    let selector = Selector::parse(selector_str).unwrap();
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

pub fn extract_image_url(style: &str) -> Option<String> {
    if let Some(start_idx) = style.find("url(") {
        let sub = &style[start_idx + 4..];
        let clean_sub = sub.trim_start_matches('\'').trim_start_matches('"');
        if let Some(end_idx) = clean_sub.find(|c| c == '\'' || c == '"' || c == ')') {
            return Some(clean_sub[..end_idx].to_string());
        }
    }
    None
}

/// Parses the nonces and text of choice elements using multiple fallback CSS Selectors
fn parse_choices(document: &Html) -> Vec<(String, String)> {
    let fallback_selectors = [
        "div.choices a",
        "a.choice",
        "a[accesskey]",
    ];

    for selector_str in fallback_selectors {
        let selector = Selector::parse(selector_str).unwrap();
        let mut choices = Vec::new();
        for element in document.select(&selector) {
            if let Some(nonce) = element.value().attr("data-nonce") {
                let mut text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();

                // Extract image url if present in style tags [Citations: index-1mp2a04.css]
                if let Some(style_attr) = element.value().attr("style") {
                    if let Some(img_url) = extract_image_url(style_attr) {
                        if text.is_empty() {
                            text = format!("[Image Option] ({})", img_url);
                        } else {
                            text = format!("{} [Image: {}]", text, img_url);
                        }
                    }
                }

                if !nonce.is_empty() {
                    choices.push((nonce.to_string(), text));
                }
            }
        }
        if !choices.is_empty() {
            return choices;
        }
    }
    Vec::new()
}

/// Prints a readable evaluation output after answer verification
fn print_answer_result(save_res: &SaveAnswerResponse) {
    println!("\n------------------- RESULTS -------------------");
    if save_res.answer.correct {
        println!("[+] CORRECT!");
    } else {
        println!("[-] INCORRECT.");
    }
    println!("[*] Word: {}", save_res.answer.word);
    if let Some(ref blurb) = save_res.answer.blurb {
        if let Some(ref short) = blurb.short {
            println!("[*] Definition Context: {}", html_to_ansi(short));
        }
    }
    println!("[*] Points Added: +{}", save_res.answer.points);
    if let Some(ref pdata) = save_res.pdata {
        if let Some(total_points) = pdata.points {
            println!("[*] Current Total Points: {}", total_points);
        }
    }
    println!("-----------------------------------------------\n");
}

fn html_to_ansi(html: &str) -> String {
    let html = Regex::new(r"(?i)<i>").unwrap().replace_all(html, "\x1b[3m");

    Regex::new(r"(?i)</i>")
        .unwrap()
        .replace_all(&html, "\x1b[0m")
        .into_owned()
}
