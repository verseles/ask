# Architecture Decision Records

This document records the architectural decisions made for the `ask` CLI project.

## ADR-001: JSON Storage Instead of Native DB

**Status**: Accepted

**Context**: We need to store conversation context/history with low latency and good Rust integration.

**Decision**: Use simple JSON file storage instead of Native DB for the initial implementation.

**Rationale**:
- Native DB adds significant complexity and compile time
- JSON files are human-readable and debuggable
- The data model is simple (messages per directory)
- Easy to migrate to a database later if needed

**Consequences**:
- Simpler implementation and faster compilation
- Slightly higher I/O overhead for large histories
- No concurrent write protection (acceptable for CLI use)

---

## ADR-002: Flexible Argument Parsing

**Status**: Accepted

**Context**: Users want to type natural language without quotes.

**Decision**: Implement custom parser that allows flags before or after free text.

**Examples**:
```bash
ask --json what is the weather
ask what is the weather --json
ask -c -x list all files
```

**Rationale**:
- Standard CLI parsers require strict ordering
- Natural language benefits from flexibility
- Flags are unambiguous (start with `-`)

**Consequences**:
- Better user experience
- Custom parsing logic to maintain
- Values must immediately follow their flags

---

## ADR-003: Context is Opt-in

**Status**: Accepted

**Context**: Maintaining conversation context has costs (tokens, storage, potential confusion).

**Decision**: Context is opt-in via `-c` flag. By default, each query is stateless.

**Rationale**:
- Predictable behavior by default
- Token economy (no unnecessary context)
- User explicitly chooses when context matters

**Consequences**:
- Users must remember to use `-c` for conversations
- Simpler default behavior
- Clear separation of stateless vs stateful modes

---

## ADR-004: TOML for Configuration

**Status**: Accepted

**Context**: Need a configuration format for settings and API keys.

**Decision**: Use TOML instead of YAML or JSON.

**Rationale**:
- TOML is the Rust ecosystem standard (Cargo.toml)
- Fewer parsing gotchas than YAML (no "Norway problem")
- More human-readable than JSON
- Excellent Rust library support

**Consequences**:
- Users familiar with Rust will feel at home
- Some users may need to learn TOML syntax
- Good error messages from the `toml` crate

---

## ADR-005: Gemini as Default Provider

**Status**: Accepted

**Context**: Need to choose a default AI provider for `ask init`.

**Decision**: Use Google Gemini as the default provider. The default model is updated periodically to use the best free-tier option (currently `gemini-flash-lite-latest` for Quick Setup, `gemini-3-flash-preview` in templates).

**Rationale**:
- Free tier available for testing
- Fast response times
- Good quality for command generation
- Simple API key acquisition

**Consequences**:
- Users need a Google account for API key
- Good out-of-box experience
- Users can switch to OpenAI/Anthropic easily

---

## ADR-006: Simple Streaming with stdout flush

**Status**: Accepted

**Context**: Users expect real-time token streaming like ChatGPT.

**Decision**: Use `print!()` with `stdout.flush()` for streaming, not a TUI framework.

**Implementation**:
```rust
print!("{}", token);
io::stdout().flush()?;
```

**Rationale**:
- Minimal complexity
- Works with pipes and redirects
- Small binary size
- No terminal compatibility issues

**Consequences**:
- Simple, reliable streaming
- No fancy TUI features
- Output works with standard Unix tools

---

## ADR-007: Safety Detection for Commands

**Status**: Accepted

**Context**: Auto-executing commands is dangerous without safeguards.

**Decision**: Implement pattern-based detection for destructive commands.

**Safe commands** (auto-execute OK):
- `ls`, `cd`, `cat`, `grep`, `find`
- `git status`, `git log`, `git diff`
- `docker ps`, `docker images`

**Destructive commands** (require confirmation):
- `rm -rf`, `rm -r`
- `sudo *`
- `dd`, `mkfs`, `fdisk`
- `curl | sh`, `wget | bash`

**Rationale**:
- Prevents accidental data loss
- Pattern matching is fast and reliable
- User can override with `-y`

**Consequences**:
- Some false positives possible
- Safe by default
- Clear confirmation prompts

---

## ADR-008: Boxed Callbacks for Streaming

**Status**: Accepted

**Context**: Need to pass callbacks to async streaming functions while maintaining `dyn` compatibility.

**Decision**: Use `Box<dyn FnMut(&str) + Send>` for streaming callbacks.

**Rationale**:
- Traits with generic methods are not dyn-compatible
- Boxing the callback solves this
- Small runtime overhead acceptable

**Consequences**:
- Heap allocation for callbacks
- Works with trait objects
- Slightly more verbose call sites

---

## ADR-009: Clipboard Paste for Command Injection

**Status**: Accepted (supersedes original keystroke approach)

**Context**: Commands need to be injected into the terminal for user review/edit before execution. The original keystroke-by-keystroke approach had issues with international keyboard layouts (dead keys like ', `, ~ would combine with vowels, e.g., 'a becoming á on ABNT2).

**Decision**: Use clipboard paste instead of keystroke typing.

**Implementation**:
- All platforms: Copy command to clipboard, then simulate paste shortcut
- Linux: Ctrl+Shift+V (standard terminal paste)
- macOS: Cmd+V
- Windows: Ctrl+V
- Clipboard preservation: Save clipboard before, restore after 500ms delay
- Fallback: Interactive requestty prompt with editable text

**Rationale**:
- Fixes dead key issues with international keyboard layouts (ABNT2, AZERTY, etc.)
- Much faster than keystroke-by-keystroke (single action vs N keystrokes)
- Smaller window for focus-change issues
- Consistent behavior across all platforms

**Consequences**:
- Temporarily overwrites clipboard (restored after 500ms)
- Requires uinput permissions on Linux (input group or udev rule)
- Requires Accessibility permission on macOS
- Graceful fallback to interactive prompt if permissions unavailable

---

## ADR-010: Auto-Update via GitHub Releases

**Status**: Accepted

**Context**: Users need an easy way to keep the CLI updated without manual downloads.

**Decision**: Implement automatic update checking via GitHub Releases API with background process.

**Implementation**:
- Background check: Spawn detached process to check GitHub releases every 24h
- Notification: Save update info to file, display on next run
- Manual update: `ask --update` for interactive update with progress bar
- Download: Fetch platform-specific binary from release assets
- Replace: Atomic binary replacement (rename on Unix, backup-replace on Windows)

**Platform Assets**:
```
ask-linux-x86_64
ask-linux-aarch64
ask-darwin-x86_64
ask-darwin-aarch64
ask-windows-x86_64.exe
```

**Rationale**:
- No external update tools required (self-contained)
- Background check doesn't block CLI usage
- GitHub Releases is reliable and free
- Atomic replacement prevents corruption
- User notification respects their workflow

**Disable Options**:
- `ASK_NO_UPDATE=1` - Disable all update checks
- `ASK_UPDATE_AUTO_CHECK=false` - Disable background checks only
- Config: `[update] auto_check = false`

**Consequences**:
- Binary must be writable (may fail in system directories)
- Requires network access for updates
- ~10KB overhead for update notification file
- Windows may need admin for some install locations

---

## ADR-011: Custom Commands System

**Status**: Accepted

**Context**: Users want reusable shortcuts for common workflows (e.g., `git diff | ask cm` for commit messages).

**Decision**: Implement config-defined custom commands with full override capabilities.

**Configuration**:
```toml
[commands.cm]
system = "Generate concise git commit message based on diff"
type = "command"           # Forces command mode
auto_execute = false       # Don't auto-run
inherit_flags = true       # Respect -c, -t, etc.
provider = "anthropic"     # Optional: override provider
model = "claude-3-opus"    # Optional: override model
```

**Execution Flow**:
1. First word of query checked against `config.commands`
2. If match found:
   - Remaining words become the query
   - System prompt replaced with custom `system`
   - Provider/model overridden if specified
   - `type = "command"` forces command mode
   - `auto_execute` controls `-y` behavior
3. Piped input combined with query as usual

**Example Usage**:
```bash
git diff | ask cm              # Uses [commands.cm] config
cat code.rs | ask explain      # Uses [commands.explain] config
ask review src/main.rs         # Uses [commands.review] config
```

**Rationale**:
- Reduces repetitive prompts
- Enables team-shared workflows via project config
- Full flexibility with provider/model per command
- Integrates naturally with piping

**Consequences**:
- Command names can shadow regular queries (use unique names)
- Config complexity increases
- No command-line definition (config only)
- Custom commands not visible in `--help`

---

## ADR-012: Web Search Integration Across Providers

**Status**: Accepted

**Context**: Users need real-time web information beyond the LLM's knowledge cutoff.

**Decision**: Implement web search as an opt-in feature across all three providers using their native APIs.

**Provider Implementations**:

| Provider | Tool | API Format |
|----------|------|------------|
| Gemini | `google_search` | `tools: [{ google_search: {} }]` |
| OpenAI | Responses API | `tools: [{ type: "web_search" }]` |
| Anthropic | `web_search_20250305` | `tools: [{ type: "web_search_20250305", name: "web_search" }]` |

**CLI Flags**:
- `-s` or `--search` - Enable web search for single query
- `--citations` - Show source URLs at end of response

**Config Options**:
```toml
[profiles.research]
web_search = true
allowed_domains = ["docs.rs", "stackoverflow.com"]  # Anthropic only
blocked_domains = ["pinterest.com"]                  # Anthropic only
```

**Citations**:
- Gemini: Extracted from `groundingMetadata.groundingChunks`
- OpenAI: Extracted from `output.content.annotations` (Responses API)
- Anthropic: Extracted from `content.citations`

**Rationale**:
- Opt-in by default (web search has additional costs and latency)
- Domain filtering only supported by Anthropic currently
- OpenAI Responses API used instead of Chat Completions for web search support

**Consequences**:
- Web search may increase response latency
- Each provider has different pricing for web search
- OpenAI web search only works with official API (not OpenAI-compatible endpoints)

---

## ADR-013: Unified Prompt System

**Status**: Accepted

**Context**: The original implementation used a separate `IntentClassifier` that made an additional API call to classify user intent before the main request. This doubled API usage and latency.

**Decision**: Replace the two-call approach with a unified prompt that handles intent detection inline.

**Previous Architecture**:
1. `IntentClassifier.classify()` → API call to determine COMMAND/QUESTION/CODE
2. Based on intent, call appropriate handler with specialized prompt
3. **Total: 2 API calls per user query**

**New Architecture**:
1. Unified prompt with inline intent detection rules
2. Single call handles all intents
3. Response detection identifies commands for execution
4. **Total: 1 API call per user query**

**Custom Prompts**:
- `ask.md` files can override the default prompt entirely
- Search order: `./ask.md` → `./.ask.md` → `~/ask.md` → `~/.config/ask/ask.md`
- Command-specific prompts: `ask.{command}.md` (e.g., `ask.cm.md`)
- Variables supported: `{os}`, `{shell}`, `{cwd}`, `{locale}`, `{now}`, `{format}`

**CLI Flags**:
- `--make-prompt` - Export default prompt template
- `--markdown[=bool]` - Control markdown formatting in responses
- `--color=bool` / `--no-color` - Control ANSI color formatting

**Rationale**:
- 50% reduction in API calls and latency
- LLMs are capable of inline intent detection
- Custom prompts allow project-specific behavior
- Simpler codebase without separate classifier

**Consequences**:
- Reduced API costs
- Faster response times
- Commands detected heuristically from response (may occasionally miss edge cases)
- Users can fully customize behavior via `ask.md` files

---

## ADR-014: Interactive Configuration Menu

**Status**: Accepted (Updated v0.25.0)

**Context**: The original `ask init` command was a simple linear wizard that only configured basic settings. Users couldn't easily manage multiple profiles, view current config, or edit specific settings without re-running the entire wizard.

**Decision**: Implement a full-featured interactive menu system for `ask init` / `ask config`.

**Menu Structure**:
```
Main Menu (existing config):
├── View current config
├── Manage profiles
│   ├── Create new profile
│   ├── Edit existing profile
│   ├── Delete profile
│   └── Set default profile
└── Exit

Quick Setup (new config):
└── Guided wizard for first-time setup (creates "main" profile)
```

**Key Features**:
- `ConfigManager` struct for state management
- Proper TOML editing that preserves existing settings
- Backup before any changes (`ask.toml.bak`)
- Per-profile settings: provider, model, API key, base URL, web search, thinking, fallback
- All configuration lives in profiles (Profile-Only Architecture per ADR-018)

**Rationale**:
- Users need to manage multiple profiles for different use cases
- Editing specific settings shouldn't require full reconfiguration
- Viewing current config helps with debugging
- Profile management is the central configuration concept

**Consequences**:
- Simplified menu with profile-centric approach
- Better user experience for configuration management
- `ask config` now works as alias for `ask init`

---

## ADR-015: Command-Line Aliases

**Status**: Accepted

**Context**: Users frequently use the same flag combinations and want shortcuts.

**Decision**: Add `[aliases]` section to config for defining flag shortcuts that expand before argument parsing.

**Configuration**:
```toml
[aliases]
q = "--raw --no-color"
fast = "-P fast --no-fallback"
deep = "-t --search"
```

**Implementation**:
1. `Config::load_aliases_only()` - Fast alias loading (no full config parse)
2. `Args::expand_aliases()` - Expands aliases before parsing
3. Aliases are merged from all config sources (local overrides global)

**Usage**:
```bash
ask q what is rust           # Expands to: ask --raw --no-color what is rust
ask deep explain quantum     # Expands to: ask -t --search explain quantum
```

**Rationale**:
- Reduces typing for common workflows
- User-definable (not hardcoded)
- Transparent expansion (aliases become real flags)

**Consequences**:
- Alias names cannot conflict with subcommands
- Expansion happens once (no recursive aliases)
- Fast path avoids full config load for alias expansion

---

## ADR-016: Non-Interactive Init

**Status**: Accepted

**Context**: Users need to configure `ask` in scripts, CI/CD, and automation without interactive prompts.

**Decision**: Add `-n`/`--non-interactive` flag with `-k`/`--api-key` for scripted configuration.

**Usage**:
```bash
# Explicit API key
ask init -n -p gemini -m gemini-2.5-flash -k YOUR_KEY

# From environment variable
GEMINI_API_KEY=xxx ask init -n

# Minimal (uses defaults)
ask init -n -k YOUR_KEY
```

**API Key Resolution**:
1. `-k`/`--api-key` flag (highest priority)
2. `{PROVIDER}_API_KEY` environment variable
3. `ASK_{PROVIDER}_API_KEY` environment variable
4. Error if none found

**Rationale**:
- Enables Docker/CI configuration
- Complements `--make-config` for template-based setup
- Follows 12-factor app principles

**Consequences**:
- Creates minimal config (no custom commands, profiles)
- Always writes to XDG config path
- For complex configs, use `--make-config` + manual edit

---

## ADR-017: Verbose Mode and Profiles Subcommand

**Status**: Accepted

**Context**: Users need visibility into which profile/provider is being used and want to list available profiles.

**Decision**: Add `-v`/`--verbose` flag and `ask profiles` subcommand.

**Verbose Output**:
- Displays active provider, model, profile, and thinking settings.
- **Update (v0.18.0)**: Includes a full dump of all internal CLI flag statuses (context, json, raw, search, etc.) for improved observability and to facilitate deep integration testing.

**Profiles Subcommand**:
```
$ ask profiles
Profiles

  personal anthropic claude-sonnet-4 [fallback: any] [think:high]
  work openai gpt-4o [search]

Default profile: personal
```

**Rationale**:
- Debugging which config is active
- Discovery of available profiles
- Consistent with other CLI tools (`docker ps`, `kubectl get`)
- Flag dump allows integration tests to verify actual application of arguments beyond just parsing success

**Consequences**:
- Verbose output goes to stderr (doesn't pollute stdout)
- Profiles shows all settings at a glance
- Slightly more verbose output when using `-v`, but significantly better for debugging and testing

---

## ADR-018: Unified Configuration Architecture (Profile-Only)

**Status**: Accepted

**Context**: The original configuration had three separate sections: `[default]` for default provider/model, `[providers.*]` for API keys, and `[profiles.*]` for named configurations. This created confusion about where settings should go and required complex inheritance logic.

**Decision**: Simplify to a profile-only architecture where all configuration lives in `[profiles.*]`. Remove `[default]` and `[providers]` sections entirely.

**Configuration Structure**:
```toml
# First profile is default unless default_profile is set
# default_profile = "work"

[profiles.main]
provider = "gemini"
model = "gemini-3-flash-preview"
api_key = "AIza..."
stream = true

[profiles.work]
provider = "openai"
model = "gpt-5"
api_key = "sk-..."
fallback = "personal"  # retry with this profile on error

[profiles.local]
provider = "openai"
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama"
```

**Precedence Hierarchy** (highest to lowest):
1. CLI flags (`-p`, `-P`, `-m`, `-k`, `-t`)
2. Environment variables (`ASK_PROFILE`, `ASK_PROVIDER`, `ASK_MODEL`, `ASK_*_API_KEY`)
3. Profile config (selected via `-p`, `default_profile`, or first available)
4. Local config (`./ask.toml`) - discovered recursively upward
5. Home config (`~/ask.toml` - legacy, still supported)
6. XDG config (`~/.config/ask/ask.toml` - recommended)
7. Hardcoded defaults

**CLI Flags**:
- `-p work` or `--profile=work` - Select active profile
- `-P gemini` or `--provider=gemini` - Ad-hoc provider override
- `-m model` or `--model=model` - Override model
- `-k key` or `--api-key=key` - Override API key
- `--no-fallback` - Disable fallback for single query

**Ad-hoc Mode**: Use `-P provider -k key` for one-off queries without any config file.

**Mutual Exclusivity**:
- `-p` (profile) and `-P` (provider) cannot be used together
- `ASK_PROFILE` and `ASK_PROVIDER` cannot be set together

**Fallback Logic**:
1. Select profile from CLI `-p`, then `default_profile`, then first available.
2. Load all settings from selected profile.
3. Apply CLI overrides (`-P`, `-m`, `-t`).
4. On provider error, attempt fallback to next profile in chain.
5. Circular fallback chains are prevented by tracking visited profiles.

**Rationale**:
- Simpler mental model: everything is a profile
- No confusion about inheritance between sections
- Ad-hoc mode enables use without config file
- Cleaner codebase with less merge logic
- Fallback provides resilience (429 errors, timeouts)

**Consequences**:
- Breaking change for existing configs using `[default]`/`[providers]`
- Migration path: move settings into `[profiles.main]`
- `ActiveConfig` struct holds runtime-resolved configuration
- First profile is used by default (no need to set `default_profile` for single-profile configs)

---

## ADR-019: Unified Thinking Levels

**Status**: Accepted

**Context**: Different providers implement "thinking" or "reasoning" capabilities with different parameters:
- **Gemini**: `thinking_level` (none, low, medium, high) or `thinking_budget` (tokens)
- **OpenAI**: `reasoning_effort` (low, medium, high)
- **Anthropic**: `thinking.budget_tokens` (integer)

This inconsistency makes it difficult for users to switch providers without changing their configuration or CLI flags.

**Decision**: Unify the thinking configuration to use abstract levels (`low`, `medium`, `high`) across all providers, while still allowing raw values for advanced users.

**Mappings**:

| Level | Gemini (Level) | OpenAI (Effort) | Anthropic (Tokens) |
|-------|---------------|-----------------|-------------------|
| `minimal` | `minimal` | `minimal` | 2048 |
| `low` | `low` | `low` | 4096 |
| `medium` | `medium` | `medium` | 8192 |
| `high` | `high` | `high` | 16384 |
| `xhigh` | - | - | 32768 |

**Implementation**:
- **CLI**: `-t`/`--think` accepts both booleans and values (e.g., `-t`, `-t high`, `--think=low`)
- **Config**: `thinking_level` in profiles is the primary configuration knob
- **Providers**: Each provider implements normalization logic to map these abstract levels to their specific API parameters
- **Fallbacks**: If a provider supports specific numeric values (like Anthropic), users can still provide raw numbers (e.g., `--think=5000`)

**Rationale**:
- **Consistency**: Users learn one set of values that works everywhere
- **Portability**: Profiles can be switched between providers without breaking thinking settings
- **Simplicity**: Abstract levels are easier to reason about than raw token counts

**Consequences**:
- Anthropic users can now use "low"/"medium"/"high" instead of just numbers
- Default token budgets for Anthropic are opinionated but reasonable
- Advanced users can still use specific values if needed

---

## ADR-020: Recursive Configuration Discovery

**Status**: Accepted

**Context**: Users working in subdirectories of a project expect the project-level configuration (API keys, aliases, custom commands) and prompts (`ask.md`) to be active without having to copy them to every subfolder.

**Decision**: Implement recursive upward search for local configuration and prompt files, similar to how Git searches for `.git` or Cargo searches for `Cargo.toml`.

**Discovery Logic**:
1. Start at the current working directory.
2. Search for `ask.toml` or `.ask.toml` (for config) or `ask.md`/`.ask.md` (for prompts).
3. If not found, move to the parent directory and repeat.
4. Stop when the file is found or the root directory is reached.

**Rationale**:
- **Workflow Efficiency**: Configuration defined at the project root applies to all subfolders.
- **Convention**: Matches the behavior of most modern developer tools.
- **Simplicity**: Avoids the need for complex global configuration management for project-specific needs.

**Consequences**:
- Configuration files in parent directories are now discovered automatically.
- Performance impact is negligible as the number of directory levels is typically small.
- Users can still override project-wide settings with a local `ask.toml` in a specific subfolder.

---

## ADR-021: Loading Indicator with Blinking ● Symbol

**Status**: Accepted

**Context**: Users had no visual feedback while waiting for AI responses, making it unclear if the tool was working or frozen.

**Decision**: Implement a loading indicator using the ● symbol that blinks while waiting and appears at the end of streaming text.

**Implementation**:
- `Spinner`: Blinks ● (500ms visible, 500ms hidden) while waiting for first chunk or full response
- `StreamingIndicator`: Shows ● at the end of text during streaming, updates position with each chunk
- Only active in terminal mode (not in raw, json, or piped output)
- Uses ANSI backspace (`\x08`) for cursor manipulation

**Rationale**:
- Minimal visual footprint (single character)
- Clear indication of "thinking" (blinking) vs "receiving" (at end of text)
- Doesn't interfere with output content
- Automatically disabled when output is piped or formatted

**Consequences**:
- Additional thread for spinner (minimal overhead)
- Requires terminal that supports backspace control character
- Gracefully does nothing in non-terminal environments

---

## ADR-022: Throttled Update Checks

**Status**: Accepted

**Context**: The "aggressive" update mode was checking for updates on every single execution. For users who use the CLI frequently, this resulted in excessive GitHub API calls, potential rate limiting, and unnecessary process spawning overhead.

**Decision**: Implement a minimum cooldown period for update checks even in aggressive mode.

**Implementation**:
- **Aggressive Mode**: Minimum 1-hour interval between background checks.
- **Normal Mode**: Respects the user-configured `check_interval_hours` (default 24h).
- The check happens by verifying the timestamp in `~/.local/share/ask/last_update_check`.

**Rationale**:
- Prevents GitHub API rate limiting.
- Reduces system overhead for frequent CLI users.
- 1 hour is more than sufficient for "aggressive" discovery of new releases.

**Consequences**:
- Users won't see an update immediately if they just checked less than an hour ago.
- Significant reduction in background process spawning.

---

## ADR-023: Safe Command Flattening

**Status**: Accepted

**Context**: LLMs sometimes return multi-line command responses when a single line was expected. The initial implementation (`flatten_command`) blindly joined all lines with `&&`, which caused problems:
1. **Broke line continuations**: `docker run \` followed by options became invalid syntax
2. **Broke heredocs**: Commands with `<<EOF` were corrupted
3. **Changed semantics**: Joining with `&&` changes execution flow (second command only runs if first succeeds)
4. **Shell compatibility**: The comment claimed `&&` was compatible with fish, but fish < 3.0 doesn't support it

**Decision**: Replace unconditional flattening with safe flattening that returns `None` when it's unsafe to flatten.

**Implementation**:
```rust
pub fn flatten_command_if_safe(text: &str) -> Option<String> {
    // Returns Some(flattened) only when ALL conditions are met:
    // - No line continuations (lines ending with \)
    // - No heredocs (lines containing <<)
    // - All lines are < 120 chars (long lines = likely wrapped single command)
    // - All lines start with a known command
}
```

**Safety Checks**:
| Pattern | Action | Reason |
|---------|--------|--------|
| Line ends with `\` | Return None | Line continuation |
| Line contains `<<` | Return None | Heredoc |
| Line > 120 chars | Return None | Likely wrapped single command |
| Line doesn't start with known command | Return None | Not a command sequence |

**Rationale**:
- Preserves original text when unsafe to modify
- Prevents silent corruption of complex commands
- Users see the original multi-line response and can decide themselves
- Flattening still works for simple sequential commands

**Consequences**:
- Multi-line responses that can't be safely flattened are shown as-is
- Users may need to manually combine some commands
- No risk of corrupting heredocs, continuations, or non-command text

---

## ADR-024: Command Injection Method Priority

**Status**: Accepted

**Context**: The command injection system (`src/executor/injector.rs`) supports multiple methods: GUI paste (clipboard + key simulation), tmux send-keys, GNU screen stuff, and an enhanced fallback. The question arose: what should be the detection order when multiple methods are available (e.g., running tmux inside a Wayland session)?

**Decision**: Prioritize GUI paste when a display server is available, even inside terminal multiplexers.

**Detection Order**:
1. If `$DISPLAY` or `$WAYLAND_DISPLAY` is set → **GuiPaste**
2. If macOS with Accessibility permission → **GuiPaste**
3. If Windows → **GuiPaste**
4. If `$TMUX` is set (no GUI) → **TmuxSendKeys**
5. If `$STY` is set (no GUI) → **ScreenStuff**
6. Otherwise → **Enhanced Fallback** (visual print + editable prompt)

**Rationale**:
- GUI paste works reliably in all terminals, including tmux/screen running inside a graphical session
- tmux/screen send-keys is primarily useful in headless environments (SSH without X11 forwarding)
- Users with GUI rarely need the multiplexer-specific injection
- GUI paste is the battle-tested method with better UX (no command echoing issues)

**Consequences**:
- Users in tmux with GUI get the same behavior as outside tmux
- tmux/screen injection only activates in truly headless environments
- The enhanced fallback provides a usable experience even without any injection method
- Headless SSH users benefit from automatic command injection via their multiplexer

---

## ADR-025: Safe-by-Default CLI Navigation

**Status**: Accepted

**Context**: Interactive configuration menus (`ask init`) often cause accidental repeated actions or misconfiguration if the first option is always selected by default after a step. Furthermore, when editing existing profiles, users often want to preserve current values rather than resetting them to a fixed default.

**Decision**: Implement "Safe-by-Default" navigation and "Smart Persistence" in the interactive configuration wizard.

**Implementation**:
1. **Safe-by-Default**: After performing an action in the main menu or submenus, the cursor automatically pre-selects "Back" or "Exit". This requires intentional movement to repeat an action.
2. **Smart Persistence**: When editing an existing profile, the wizard pre-loads and pre-selects current values (Provider, Model, Thinking Level, etc.) in the prompts.
3. **Type-Agnostic Reading**: Implemented `get_any_str` in `ConfigManager` to handle TOML values of various types (String, Integer, Boolean) as strings for menu pre-selection (e.g., reading `thinking_budget = 16384` as `"16384"`).

**Rationale**:
- Reduces accidental changes to configuration.
- Improves ergonomic flow for users wanting to "Exit" or "Go Back" after a quick change.
- Consistent with modern CLI wizard patterns (e.g., `npm init`, `git init` style interactions).
- Prevents data loss during profile editing by preserving existing settings.

**Consequences**:
- Users must press arrow keys more often to perform multiple consecutive actions.
- Much higher confidence during profile editing as existing values are visible and pre-selected.
- `thinking_budget` (Gemini 2.5/Anthropic) is now correctly persisted and pre-selected.
