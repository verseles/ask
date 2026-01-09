---
feature: "CLI ask - Initial Implementation"
spec: |
  AI-powered CLI that accepts plain text questions without quotes.
  Supports Gemini, OpenAI, and Anthropic providers with streaming.
---

## Task List

### Feature 1: Core Infrastructure

Description: Basic setup, config loading, CLI parsing

- [x] 1.01 Setup Rust project with Cargo.toml - Commit: dd3ef64
- [x] 1.02 Implement flexible argument parser (flags before/after text) - Commit: dd3ef64
- [x] 1.03 Config loader with TOML hierarchy (CLI > Env > Local > Home > XDG) - Commit: dd3ef64
- [x] 1.04 Default values and constants - Commit: dd3ef64

### Feature 2: Provider Integrations

Description: Integrate OpenAI, Anthropic, Gemini APIs

- [x] 2.01 Provider trait definition with async methods - Commit: dd3ef64
- [x] 2.02 Gemini integration with streaming (default provider) - Commit: dd3ef64
- [x] 2.03 OpenAI integration with streaming - Commit: dd3ef64
- [x] 2.04 Anthropic Claude integration with streaming - Commit: dd3ef64
- [!] 2.05 Intent classifier for COMMAND/QUESTION/CODE - Commit: dd3ef64 - Will be removed by Feature 11

### Feature 3: Context System

Description: JSON storage for conversation history

- [x] 3.01 JSON file storage backend - Commit: dd3ef64
- [x] 3.02 Context manager with directory-based keys - Commit: dd3ef64
- [x] 3.03 Context commands (--clear, --history) - Commit: dd3ef64
- [x] 3.04 Automatic TTL cleanup - Commit: dd3ef64
- [x] 3.05 Optional TTL in context flag
  - **Syntax**: `-c60`, `-c=60`, `--context=120` (minutes)
  - **Default**: 30 minutes (when just `-c` is used)
  - **Permanent**: `-c0` or `--context=0` means no expiration
  - **No max limit**: User's choice, LLM context limit will error naturally if exceeded
  - **Echo context size**: When context > 500 chars OR > 3 messages (whichever first), show:
    `"(using context: X messages, Y chars - use --clear to reset)"`
  - **Clap config**: Use `num_args = 0..=1` with `default_missing_value = "30"`

### Feature 4: Command Execution

Description: Safe command detection and execution

- [x] 4.01 Safety detector for destructive commands (regex patterns) - Commit: dd3ef64
- [x] 4.02 Command executor with follow-up echo - Commit: dd3ef64
- [x] 4.03 Confirmation prompts for destructive commands - Commit: dd3ef64
- [x] 4.04 Retry with sudo suggestion on permission denied - Commit: 8d1a7f6

### Feature 5: Output & Streaming

Description: Streaming, formatting, colors

- [x] 5.01 SSE streaming with stdout flush - Commit: dd3ef64
- [x] 5.02 JSON output format (--json) - Commit: dd3ef64
- [x] 5.03 Raw output format (--raw) - Commit: dd3ef64
- [x] 5.04 Markdown rendering in terminal - Commit: dd3ef64
- [x] 5.05 Color scheme utilities (implemented but not fully used) - Commit: dd3ef64

### Feature 6: Advanced Features

Description: Auto-update, custom commands, piping

- [x] 6.01 Piping support (stdin detection) - Commit: dd3ef64
- [x] 6.02 Auto-update from GitHub releases - Commit: 8d1a7f6
- [x] 6.03 Custom commands from config ([commands.cm] etc.) - Commit: 8d1a7f6
- [x] 6.04 Update check notification - Commit: 8d1a7f6

### Feature 7: Documentation & CI/CD

Description: Documentation, install scripts, GitHub Actions

- [x] 7.01 README.md with examples and configuration - Commit: dd3ef64
- [x] 7.02 ADR.md with architecture decisions - Commit: dd3ef64
- [x] 7.03 CODEBASE.md with structure documentation - Commit: dd3ef64
- [x] 7.04 install.sh for Unix systems - Commit: dd3ef64
- [x] 7.05 install.ps1 for Windows - Commit: fe21368
- [x] 7.06 GitHub Actions CI pipeline - Commit: dd3ef64
- [x] 7.07 GitHub Actions release pipeline - Commit: dd3ef64
- [x] 7.08 GitHub Actions test pipeline - Commit: dd3ef64
- [x] 7.09 Complete env vars documentation
  - **New flag**: `--help-env` shows all environment variables (separate from --help)
  - **README.md**: Full env var list with `<details>` collapsible sections
  - **Variables to document** (~14 missing from --help):
    - `ASK_STREAM`, `ASK_GEMINI_BASE_URL`, `ASK_OPENAI_BASE_URL`, `ASK_ANTHROPIC_BASE_URL`
    - `ASK_AUTO_EXECUTE`, `ASK_CONFIRM_DESTRUCTIVE`, `ASK_TIMEOUT`
    - `ASK_CONTEXT_MAX_AGE`, `ASK_CONTEXT_MAX_MESSAGES`, `ASK_CONTEXT_PATH`
    - `ASK_UPDATE_AUTO_CHECK`, `ASK_UPDATE_INTERVAL`, `ASK_UPDATE_CHANNEL`, `ASK_NO_UPDATE`

### Feature 8: Polish & Testing

Description: Additional tests, cleanup, optimization

- [x] 8.01 Integration tests for CLI (basic) - Commit: 3436645
- [x] 8.03 Remove dead code warnings - Commit: 3436645
- [x] 8.04 Add more unit tests for config loading - Commit: 8d1a7f6
- [x] 8.06 Shell completions generation - Commit: 8d1a7f6

### Feature 9: Multi-Profile System

Description: Named profiles (like rclone) with fallback support for resilient AI queries

- [x] 9.01 Config structure for named profiles
  - **Syntax**: `[profiles.work]`, `[profiles.personal]`, etc.
  - **Inheritance**: Profiles inherit from default config (partial configs allowed)
  - **Example**:
    ```toml
    [profiles.work]
    model = "gpt-5"  # inherits provider, api_key from default
    
    [profiles.personal]
    provider = "anthropic"
    model = "claude-haiku-4-5"
    api_key = "sk-ant-..."  # can override everything
    ```
- [x] 9.02 Profile slug flag (`--profile`, `-P`) to select active profile
  - **Syntax**: `-P work`, `--profile=personal`
- [x] 9.03 Default profile setting in config
  - **Config**: `default_profile = "work"` or first profile in file if not specified
  - **Fallback order**: First profile defined in config file is the default fallback
- [x] 9.04 Profile inheritance (profiles extend base/default settings)
  - Profiles only need to specify what they override
  - All unspecified values come from `[default]` and `[providers.*]` sections
- [x] 9.05 Fallback config option per profile
  - **Syntax**: `fallback = "other-profile"` or `fallback = "any"` or `fallback = "none"`
  - **Default behavior**: Use first profile as fallback unless `fallback = "none"`
  - **Profile can prohibit fallback**: `fallback = "none"` disables fallback for that profile
- [ ] 9.06 Auto-fallback on provider errors (429, timeout, API errors)
  - Automatic retry with fallback profile on: 429, 500, 502, 503, 504, timeout, connection errors
  - Show message: `"Provider error, retrying with fallback profile..."`
- [ ] 9.07 Interactive fallback setup in `ask init`
  - Prompt: "Fallback behavior?" with options:
    1. Use any available profile
    2. Use specific profile (shows select)
    3. No fallback
- [x] 9.08 CLI flag to disable fallback for single query (`--no-fallback`)
- [x] 9.09 OpenAI-compatible endpoint support via profile base_url (Ollama, LM Studio)
  - **Example**:
    ```toml
    [profiles.local]
    provider = "openai"
    base_url = "http://localhost:11434/v1"
    model = "llama3"
    api_key = "ollama"  # some local servers require dummy key
    ```
- [x] 9.10 Review and update tests for Feature 9
- [x] 9.11 Review and update ADR.md if needed for Feature 9
- [x] 9.12 Review and update README.md if needed for Feature 9
  - Use `<details>` sections for profile examples and fallback configuration

### Feature 10: Web Search Integration

Description: Native web search support for all providers (Gemini, OpenAI, Anthropic)

- [x] 10.01 Gemini: Google Search grounding (`tools: [{ google_search: {} }]`)
- [x] 10.02 OpenAI: Web Search tool (`tools: [{ type: "web_search" }]` via Responses API)
- [x] 10.03 Anthropic: Web Search tool (`web_search_20250305` for claude models)
- [x] 10.04 Config option to enable web search per profile (`web_search = true`)
  - Default: `false` (opt-in)
- [x] 10.05 CLI flag to enable web search (`--search`, `-s`)
  - `-s` or `--search` enables for single query
- [x] 10.06 Parse and display citations from search results
  - **Default**: Citations hidden
  - **Flag**: `--citations` to show for single query
  - **Format**: Numbered inline with URL list at end
- [x] 10.07 Domain filtering support for Anthropic (allowed_domains/blocked_domains)
  - Config: `allowed_domains = ["docs.rs", "stackoverflow.com"]`
  - Config: `blocked_domains = ["pinterest.com"]`
- [ ] 10.08 Interactive web search setup in `ask init`
  - **Note**: Consider renaming `ask init` to `ask config` (keep `init` as alias for compatibility)
  - Both commands lead to same interactive config CLI
- [x] 10.09 Review and update tests for Feature 10
- [x] 10.10 Review and update ADR.md if needed for Feature 10
- [x] 10.11 Review and update README.md if needed for Feature 10
  - Document pricing differences per provider
  - Use `<details>` for citation format examples

### Feature 11: Unified Prompt System

Description: Consolidate prompts and remove redundant intent classifier API call

- [x] 11.01 Create unified prompt template (handles command/question detection in single call)
  - **Template structure**:
    ```
    Answer in the user's language based on locale ({locale}).
    If the user asks for a shell command, return ONLY the command - no explanation,
    no markdown, no code blocks. Use && for multiple commands.
    If it's a question, be brief (1-3 sentences).
    
    use markdown={true|false}
    {do not} use terminal colors and formatting
    
    Context: OS={os}, shell={shell}, cwd={cwd}, locale={locale}, now={now}
    ```
  - **Result**: Reduces from 2 API calls to 1 per query
- [x] 11.02 Remove IntentClassifier API call (saves 1 API call per request)
  - Delete `IntentClassifier` struct and related code
  - Update tests accordingly
- [x] 11.03 Add `use markdown={true|false}` to prompt based on `--markdown[=bool]` flag
  - `--markdown` or `--markdown=true` → `use markdown=true`
  - `--markdown=false` or default → `use markdown=false`
- [x] 11.04 Add `{do not} use terminal colors and formatting` based on `--no-color`/`--color=bool`
  - Default: `use terminal colors and formatting`
  - `--no-color` or `--color=false` → `do not use terminal colors and formatting`
- [x] 11.05 Refactor `-x` to add command-mode emphasis to unified prompt (not separate prompt)
  - When `-x` flag: prepend `"IMPORTANT: User explicitly requested command mode. Return ONLY the shell command, nothing else."`
- [x] 11.06 Load custom prompts from `ask.md` files (same hierarchy as ask.toml)
  - **Search order**: `./ask.md` → `./.ask.md` → `~/ask.md` → `~/.config/ask/ask.md`
  - **Behavior**: If `ask.md` exists, it REPLACES the default prompt entirely
  - **Variables**: Support `{os}`, `{shell}`, `{cwd}`, `{locale}`, `{now}` replacements in custom prompts
- [x] 11.07 Load command-specific prompts from `ask.{command}.md` (e.g., `ask.cm.md`, `ask.explain.md`)
  - **Priority**: `ask.cm.md` > `[commands.cm].system` in TOML (user file always wins)
  - **Same variable support** as `ask.md`
- [x] 11.08 Add `ask --make-prompt` command to export default prompt template
  - Outputs the current default unified prompt to stdout
  - User can redirect: `ask --make-prompt > ask.md` to customize
- [x] 11.09 Review and update tests for Feature 11
- [x] 11.10 Review and update ADR.md if needed for Feature 11
  - Document the decision to remove IntentClassifier
  - Document prompt priority order
- [x] 11.11 Review and update README.md if needed for Feature 11
  - Add section on custom prompts with examples
  - Use `<details>` for prompt template reference

## Legend

- [x] Complete and functional
- [~] In progress recently
- [/] Partially implemented but not functional
- [!] Refused or will not be done (with reason/commit reference)
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

### Completed in v0.2.0 - Commit: 3436645

1. **Makefile** - Added precommit workflow (fmt, clippy, test, audit)
2. **AI assistant docs** - Added CLAUDE.md and GEMINI.md
3. **Integration tests** - Basic CLI tests (help, version, flags)
4. **Code quality** - Removed dead code warnings, replaced deprecated atty crate
5. **Cross-compilation** - Switched to rustls-tls for OpenSSL-free builds
6. **Updated models** - Gemini 3 Flash Preview, GPT-5 Mini, Claude Haiku 4.5
7. **Thinking mode** - Added thinking/reasoning support for all providers
8. **Interactive init** - Model selection and thinking mode during `ask init`
9. **Improved installers** - Updated artifact names, prompt to run init after install

### Completed in v0.3.0 - Commit: 8b0e670

1. **Command injection** - Commands are pasted directly to terminal via clipboard + Ctrl+Shift+V
2. **Context-aware prompts** - System prompts include OS, shell, cwd, locale, datetime
3. **Improved UX** - `ask init` reads existing config and shows current values as defaults
4. **Brief responses** - Questions get concise 1-3 sentence answers with terminal colors

### Completed in v0.6.0 - Commit: 8d1a7f6

1. **Auto-update** - Background update checks with GitHub releases, notification on next run
2. **Custom commands** - Config-defined commands with custom system prompts
3. **Sudo retry** - Suggests retry with sudo on permission denied errors
4. **Shell completions** - Bash, Zsh, Fish, PowerShell, Elvish support via --completions
5. **Config tests** - Comprehensive unit tests for config loading and parsing
6. **Full env var support** - All TOML options available as ASK_* environment variables

### Completed in v0.7.0 - Commit: 0e22fff

1. **macOS config fix** - Now checks ~/.config/ask/config.toml on macOS for Unix compatibility
2. **Config merge fix** - Fixed bug where explicit default values were ignored in overlay configs

### Known Limitations

1. **Limited tests** - Provider integration tests with mock server not yet implemented
2. **Binary size** - Could be optimized further

### Future Considerations

1. Consider switching to Native DB for context if JSON becomes a bottleneck
2. Consider adding TUI mode for interactive sessions
3. ~~Add shell completion scripts~~ - Done in 8.06
4. ~~Model/Provider rotation~~ - Covered by Feature 9 (Multi-Profile System)
5. ~~OpenAI-compatible endpoint~~ - Moved to Feature 9 (9.09)
6. Integration tests for providers (mock server) - Low priority
7. Benchmark binary size optimization - Low priority
