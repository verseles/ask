---
feature: "CLI ask - Initial Implementation"
spec: |
  AI-powered CLI that accepts plain text questions without quotes.
  Supports Gemini, OpenAI, and Anthropic providers with streaming.
---

## Task List

### Feature 1: Core Infrastructure

Description: Basic setup, config loading, CLI parsing

- [x] 1.01 Setup Rust project with Cargo.toml
- [x] 1.02 Implement flexible argument parser (flags before/after text)
- [x] 1.03 Config loader with TOML hierarchy (CLI > Env > Local > Home > XDG)
- [x] 1.04 Default values and constants

### Feature 2: Provider Integrations

Description: Integrate OpenAI, Anthropic, Gemini APIs

- [x] 2.01 Provider trait definition with async methods
- [x] 2.02 Gemini integration with streaming (default provider)
- [x] 2.03 OpenAI integration with streaming
- [x] 2.04 Anthropic Claude integration with streaming
- [x] 2.05 Intent classifier for COMMAND/QUESTION/CODE

### Feature 3: Context System

Description: JSON storage for conversation history

- [x] 3.01 JSON file storage backend
- [x] 3.02 Context manager with directory-based keys
- [x] 3.03 Context commands (--clear, --history)
- [ ] 3.04 Automatic TTL cleanup (implemented but untested in production)

### Feature 4: Command Execution

Description: Safe command detection and execution

- [x] 4.01 Safety detector for destructive commands (regex patterns)
- [x] 4.02 Command executor with follow-up echo
- [x] 4.03 Confirmation prompts for destructive commands
- [ ] 4.04 Retry with sudo suggestion on permission denied

### Feature 5: Output & Streaming

Description: Streaming, formatting, colors

- [x] 5.01 SSE streaming with stdout flush
- [x] 5.02 JSON output format (--json)
- [x] 5.03 Raw output format (--raw)
- [x] 5.04 Markdown rendering in terminal
- [x] 5.05 Color scheme utilities (implemented but not fully used)

### Feature 6: Advanced Features

Description: Auto-update, custom commands, piping

- [x] 6.01 Piping support (stdin detection)
- [ ] 6.02 Auto-update with self_update crate
- [ ] 6.03 Custom commands from config ([commands.cm] etc.)
- [ ] 6.04 Update check notification

### Feature 7: Documentation & CI/CD

Description: Documentation, install scripts, GitHub Actions

- [x] 7.01 README.md with examples and configuration
- [x] 7.02 ADR.md with architecture decisions
- [x] 7.03 CODEBASE.md with structure documentation
- [x] 7.04 install.sh for Unix systems
- [x] 7.05 install.ps1 for Windows
- [x] 7.06 GitHub Actions CI pipeline
- [x] 7.07 GitHub Actions release pipeline
- [x] 7.08 GitHub Actions test pipeline

### Feature 8: Polish & Testing

Description: Additional tests, cleanup, optimization

- [x] 8.01 Integration tests for CLI (basic)
- [ ] 8.02 Integration tests for providers (mock server)
- [x] 8.03 Remove dead code warnings
- [ ] 8.04 Add more unit tests for config loading
- [ ] 8.05 Benchmark binary size optimization
- [ ] 8.06 Shell completions generation

## Legend

- [x] Complete and functional
- [~] In progress recently
- [/] Partially implemented but not functional
- [ ] Not started

## Notes

### Completed in Initial Implementation

1. **Core CLI functionality** - Flexible argument parsing allowing flags before/after text
2. **Three AI providers** - Gemini (default), OpenAI, Anthropic with streaming support
3. **Context system** - Per-directory conversation history with JSON storage
4. **Safety detection** - Pattern-based detection of destructive commands
5. **Output formatting** - JSON, raw, and markdown modes
6. **Documentation** - README, ADR, and CODEBASE files
7. **CI/CD** - GitHub Actions for testing, linting, and releases
8. **Install scripts** - Cross-platform installation

### Completed in v0.2.0

1. **Makefile** - Added precommit workflow (fmt, clippy, test, audit)
2. **AI assistant docs** - Added CLAUDE.md and GEMINI.md
3. **Integration tests** - Basic CLI tests (help, version, flags)
4. **Code quality** - Removed dead code warnings, replaced deprecated atty crate
5. **Cross-compilation** - Switched to rustls-tls for OpenSSL-free builds
6. **Updated models** - Gemini 3 Flash Preview, GPT-5 Mini, Claude Haiku 4.5
7. **Thinking mode** - Added thinking/reasoning support for all providers
8. **Interactive init** - Model selection and thinking mode during `ask init`
9. **Improved installers** - Updated artifact names, prompt to run init after install

### Known Limitations

1. **No auto-update** - Self-update feature not implemented
2. **No custom commands** - Config-defined commands not parsed
3. **Limited tests** - Only basic unit tests implemented

### Future Considerations

1. Consider switching to Native DB for context if JSON becomes a bottleneck
2. Add OpenAI-compatible endpoint support (Ollama, LM Studio)
3. Consider adding TUI mode for interactive sessions
4. Add shell completion scripts (bash, zsh, fish, PowerShell)
