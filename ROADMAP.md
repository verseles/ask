---
feature: "Profile-Only Architecture + Ad-hoc Mode"
spec: |
  Simplify configuration by removing [default] and [providers] sections.
  All configuration lives in [profiles.*]. Ad-hoc mode via -P for execution without config.
---

## Feature 13: Profile-Only Architecture

Description: Remove [default] and [providers], centralize everything in [profiles.*]

### Phase 1: Struct Changes

- [x] 13.01 Remove `DefaultConfig` struct from `src/config/mod.rs`
- [x] 13.02 Remove `ProviderConfig` struct from `src/config/mod.rs`
- [x] 13.03 Simplify `Config` struct (remove `default` and `providers` fields)
- [x] 13.04 Ensure `ProfileConfig` has all required fields with proper defaults

### Phase 2: Loader Changes

- [x] 13.05 Remove merge logic for `[default]` section in `src/config/loader.rs`
- [x] 13.06 Remove merge logic for `[providers]` section in `src/config/loader.rs`
- [x] 13.07 Simplify overlay to only merge: profiles, behavior, context, update, commands, aliases

### Phase 3: Access Method Changes

- [x] 13.08 Refactor `api_key()` to fetch from active profile (with ENV fallback)
- [x] 13.09 Refactor `base_url()` to fetch from active profile (with ENV fallback)
- [x] 13.10 Refactor `active_provider()` to fetch from active profile
- [x] 13.11 Refactor `active_model()` to fetch from active profile
- [x] 13.12 Remove or refactor `apply_profile()` method
- [x] 13.13 Refactor `with_cli_overrides()` for new architecture

### Phase 4: Default Profile Logic

- [x] 13.14 Remove `default_profile = "first"` from `DEFAULT_CONFIG_TEMPLATE`
- [x] 13.15 Remove `default_profile` generation in Quick Setup (`init_config`)
- [x] 13.16 Add logic to clear `default_profile` when deleting the default profile
- [x] 13.17 Verify `active_profile()` fallback logic works correctly

### Phase 5: CLI Validation & Ad-hoc Mode

- [x] 13.18 Add validation: `-p` and `-P` are mutually exclusive
- [x] 13.19 Add validation: `-P` requires `-k` or `ASK_{PROVIDER}_API_KEY`
- [x] 13.20 Implement ad-hoc mode: create virtual profile when `-P` is used
- [x] 13.21 Add `ASK_PROFILE` env var support (equivalent to `-p`)
- [x] 13.22 Add validation: `ASK_PROFILE` and `ASK_PROVIDER` are mutually exclusive

### Phase 6: Interactive Menu Updates

- [x] 13.23 Update "Edit default settings" to edit default/first profile
- [x] 13.24 Update "Manage API keys" to edit api_key within profiles
- [x] 13.25 Update "View current config" to not show [default]/[providers]

### Phase 7: Template Updates

- [x] 13.26 Update `DEFAULT_CONFIG_TEMPLATE` with new structure
- [x] 13.27 Update `--make-config` output

### Phase 8: Tests

- [x] 13.28 Remove tests for `[providers]` and `[default]`
- [x] 13.29 Add tests for ad-hoc mode (`-P` + `-k`)
- [x] 13.30 Add tests for `-p` + `-P` error
- [x] 13.31 Add tests for implicit default_profile (first profile)
- [x] 13.32 Add tests for ENV validation (`ASK_PROFILE` + `ASK_PROVIDER` error)

### Phase 9: Documentation

- [x] 13.33 Update README.md with new config structure
- [x] 13.34 Update CODEBASE.md with architectural changes
- [x] 13.35 Add ADR-021: Profile-Only Architecture
- [x] 13.36 Update `--help-env` with new ENV list

## Legend

- [x] Complete
- [~] In progress
- [ ] Not started

## Notes

### Environment Variables (Final)

| ENV                       | Description                              |
| ------------------------- | ---------------------------------------- |
| `ASK_PROFILE`             | Select profile (like `-p`)               |
| `ASK_PROVIDER`            | Ad-hoc mode (like `-P`)                  |
| `ASK_MODEL`               | Override model                           |
| `ASK_GEMINI_API_KEY`      | Gemini API key                           |
| `ASK_OPENAI_API_KEY`      | OpenAI API key                           |
| `ASK_ANTHROPIC_API_KEY`   | Anthropic API key                        |
| `ASK_GEMINI_BASE_URL`     | Gemini base URL                          |
| `ASK_OPENAI_BASE_URL`     | OpenAI base URL                          |
| `ASK_ANTHROPIC_BASE_URL`  | Anthropic base URL                       |
| `ASK_STREAM`              | Override streaming                       |
| `ASK_TIMEOUT`             | Override timeout                         |

### Precedence Order

```
CLI flags (-p, -P, -m, -k)
    ↓
ENVs (ASK_PROFILE, ASK_PROVIDER, ASK_MODEL, ASK_*_API_KEY)
    ↓
Profile config ([profiles.*])
    ↓
Hardcoded defaults
```

### Final Config Structure

```toml
# Optional: only when user explicitly sets it
# default_profile = "personal"

[profiles.personal]
provider = "gemini"
model = "gemini-3-flash-preview"
api_key = "AIza..."
stream = true

[profiles.work]
provider = "openai"
model = "gpt-5"
api_key = "sk-..."

[behavior]
auto_execute = false
confirm_destructive = true
timeout = 30

[context]
max_age_minutes = 30
max_messages = 20

[update]
auto_check = true

[aliases]
q = "--raw --no-color"

[commands.cm]
system = "Generate commit message"
```
