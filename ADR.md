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

## ADR-005: Automatic Intent Detection

**Status**: Accepted

**Context**: Users may not know if they want a command or an answer.

**Decision**: Use a lightweight classification prompt to detect intent (COMMAND vs QUESTION vs CODE).

**Implementation**:
- Send a quick classification request to the AI
- Use structured response for reliability
- Override with `-x` flag for explicit command mode

**Rationale**:
- Better UX - users don't need to think about mode
- Small token cost for classification
- Can be bypassed when needed

**Consequences**:
- Extra API call for intent detection
- Slight latency increase
- More intelligent default behavior

---

## ADR-006: Gemini as Default Provider

**Status**: Accepted

**Context**: Need to choose a default AI provider for `ask init`.

**Decision**: Use Google Gemini as the default provider with `gemini-2.0-flash` model.

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

## ADR-007: Simple Streaming with stdout flush

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

## ADR-008: Safety Detection for Commands

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

## ADR-009: Multi-layer Configuration Precedence

**Status**: Accepted

**Context**: Different contexts need different configurations.

**Decision**: Implement configuration hierarchy:
1. CLI arguments (highest)
2. Environment variables
3. Local config (`./ask.toml`)
4. Home config (`~/ask.toml`)
5. XDG config (`~/.config/ask/config.toml`)
6. Defaults (lowest)

**Rationale**:
- Project-specific settings possible
- Environment variables for CI/CD
- User defaults at home level
- Follows Unix conventions

**Consequences**:
- Flexible configuration
- May be confusing which config is active
- Config merging logic to maintain

---

## ADR-010: Boxed Callbacks for Streaming

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
