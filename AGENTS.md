# Agent Rules for `ask`

## Release Process

### CRITICAL: Version Synchronization

**BEFORE creating any git tag for release:**

1. Update `Cargo.toml` version field to match the new tag
2. Run `make precommit` to ensure everything passes
3. Commit the version bump
4. Then create the tag

**Why this matters:**
- `env!("CARGO_PKG_VERSION")` is resolved at compile time
- The auto-update feature compares this compiled version against GitHub releases
- If Cargo.toml is behind, users will see "update available" forever

**Example workflow for releasing v0.27.0:**
```bash
# 1. Edit Cargo.toml: version = "0.27.0"
# 2. Run checks
make precommit
# 3. Commit
git add Cargo.toml && git commit -m "chore: bump version to 0.27.0"
# 4. Tag and push
git tag -a v0.27.0 -m "Release v0.27.0" && git push && git push origin v0.27.0
```

## Development Guidelines

- Always run `make precommit` before committing
- Update CODEBASE.md when architecture changes
- Create ADR entries for significant decisions
- Test on multiple platforms when touching executor/injector code

## Testing Environments

The command injection system should be tested on:
- Linux with Wayland
- Linux with X11
- macOS
- Windows
- SSH (headless) with tmux
- SSH (headless) with screen
- SSH (headless) without multiplexer (fallback)
