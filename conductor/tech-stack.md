# Tech Stack

## Core Language & Runtime
- **Language:** Rust (Edition 2021)
- **Runtime:** Tokio (Full features) for asynchronous execution and non-blocking I/O.

## Build System & Package Management
- **Toolchain:** Cargo
- **Dependency Management:** `Cargo.toml` and `Cargo.lock`

## CLI & User Interface
- **CLI Framework:** `clap` (v4) with derive and env features for robust command-line parsing.
- **Terminal UI:** 
    - `termimad` for rich Markdown rendering in the terminal.
    - `colored` for ANSI color support.
    - `indicatif` for progress bars and spinners.
    - `requestty` for interactive prompts and menus.

## Networking & Data Handling
- **HTTP Client:** `reqwest` (v0.12) with rustls-tls and stream support.
- **Serialization:** `serde` (v1) and `serde_json` for JSON/TOML processing.
- **Configuration:** `toml` (v0.8) for file-based settings.

## Utilities & Architecture
- **Error Handling:** `anyhow` and `thiserror` for idiomatic and flexible error management.
- **Time/Date:** `chrono` for handling context TTL and timestamps.
- **Concurrency:** `futures` for stream manipulation.
- **System Access:** 
    - `dirs` for cross-platform configuration paths.
    - `shellexpand` for path expansion.
    - `arboard` for clipboard operations.

## Architecture Pattern
- **Modular CLI:** Separation of concerns between the parser, executor, configuration loader, and provider traits.
- **Pluggable Providers:** Abstracted AI provider interface allowing for easy integration of new LLM backends (Gemini, OpenAI, Anthropic).
