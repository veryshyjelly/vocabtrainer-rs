
# VocabTrainer Suite

An asynchronous Rust library, interactive command-line interface, and high-contrast, e-ink-optimized web proxy designed for playing Vocabulary.com's **VocabTrainer** on desktop terminals or low-powered e-readers like the **Kobo Clara BW**.

---

## Key Features

*   **Asynchronous Engine**: A robust, type-safe client that handles standard credential-based logins, guest play, state maintenance, and game-saving API routines.
*   **HTML Base64 Decoding**: Decodes the base64-obfuscated HTML templates served by the backend into standard text strings.
*   **Interactive Terminal Game (`trainer`)**: A console-based app using `inquire` select lists, secure password masks, and `scraper` selector logic to parse multiple-choice options and spelling questions natively.
*   **E-Ink Web Proxy Server (`kobo_proxy`)**: An Axum-based web proxy that handles computationally heavy HTML parsing on the server side and feeds clean, optimized JSON to the Kobo.
*   **Monochromatic Frontend**: Features a simple ES5-compliant frontend with a high-contrast style guide, no transitions or animations, and large touch targets designed to prevent ghosting on e-ink panels.

---

## Directory Layout

```text
.
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library exports
│   ├── client.rs           # Core HTTP API logic & decoding
│   ├── error.rs            # Library-specific error types
│   ├── models.rs           # Serialized JSON representations
│   └── bin/
│       ├── trainer.rs      # Interactive terminal application
│       └── kobo_proxy.rs   # Server for E-Reader web browser
└── static/
    ├── index.html          # Lightweight frontend layout
    ├── style.css           # Monochromatic high-contrast stylesheet
    └── app.js              # Standard ES5 XMLHttp engine
```

---

## Getting Started

### 1. Prerequisites
Make sure you have [Rust and Cargo](https://rustup.rs/) installed.

### 2. Standard Configuration
Clone the repository and compile the binaries:
```bash
cargo build --release
```

---

## Usage

### Option A: Playing in the Terminal

Launch the interactive console application:
```bash
cargo run --bin trainer
```

1. Select whether to play anonymously as a guest or log in with your account.
2. If logging in, securely input your Vocabulary.com email and password at the prompt.
3. Use your terminal **arrow keys** and **Enter** to navigate multiple-choice selections or type answers for spelling prompts.

---

### Option B: Playing on a Kobo E-Reader

Launch the proxy server locally on your computer:
```bash
cargo run --bin kobo_proxy
```
This spins up a local web server at `http://localhost:8080` which serves the embedded, lightweight e-ink pages.

#### Connecting Your Kobo Clara BW
1. **Expose the Local Port**: Because e-readers cannot easily connect to localhost, expose your local port `8080` using a Cloudflare Tunnel or local reverse-proxy tool to map it to a public domain:
   ```bash
   cloudflared tunnel --url http://localhost:8080
   ```
2. **Launch the Browser on Kobo**: On your Kobo Clara BW, navigate to:
   * **More** $\rightarrow$ **Beta Features** $\rightarrow$ **Web Browser**
3. Navigate to your tunnel's domain (e.g., `https://vcom.yourdomain.com`).
4. Log in using your email and password, or proceed in guest mode to play.

---

## Technical Information

*   **API Protocol Version**: Uses client parameter configuration `v=4`.
*   **State Coordination**: Automatically tracks game sequences via transient session tokens (`secret`) on consecutive `/challenge/saveanswer.json` and `/challenge/nextquestion.json` requests.
*   **DOM Parsing Strategy**: Implements fallback queries over `div.choices a`, `a.choice`, and `a[accesskey]` to maintain compatibility if class selectors change on the server.

---

## License

This project is intended for personal study and educational utility. All game rules and assets remain the property of Vocabulary.com, Inc.
