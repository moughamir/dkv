# AGENTS.md

## What this is

DKV (Dev Knowledge Vault): an offline-first developer knowledge platform, implemented as a Rust CLI.
Not an AI project — AI/RAG is out of scope until every other phase is complete.

Status: Phase 1 (project understanding) partially implemented. Do not redesign or rename modules;
extend what exists. The plan's later modules (`providers/`, `scanner/`, `storage/`, `index/`,
`search/`) do not exist yet — create them under `src/` when their phase starts.

## Toolchain

- Rust edition 2024, `rust-version = 1.97.1` (that toolchain is installed; build with >= 1.97.1).
- Single crate `dkv` with a lib (`src/lib.rs`) and a thin bin (`src/main.rs` dispatching into
  `dkv::commands`).
- Strict lints live in `[lints]` of `Cargo.toml`: `unsafe_code = "forbid"`, clippy all/pedantic/
  nursery deny, `unwrap_used`/`expect_used`/`panic`/`todo` deny. Write code compatible with these
  (prefer `?`, `ok_or_else`, `map_err`).

## Quality gates — end of every milestone, in order

```bash
cargo fmt --check
cargo check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
cargo doc --no-deps
```

All five gates are currently clean (`cargo fmt --check`, `cargo check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test`,
`cargo doc --no-deps`). Keep them green when touching code.

## Architecture

```
src/
  cli.rs        # clap definitions (bin-only)
  main.rs       # dispatch + tracing init (-v/-vv/-vvv, --quiet are global)
  lib.rs        # pub mods; exports DkvError, Result, VERSION, APP_NAME
  commands/     # one module per CLI command
  config/       # load/save/init of ~/.config/dkv/config.toml (XDG via `directories`)
  error.rs      # DkvError + Result (thiserror)
  project/      # manifest parsing + detection (cargo.rs, npm.rs, detect.rs)
  scanner/      # filesystem traversal + `.gitignore`/`.dkvignore` exclusions (mod.rs, ignore.rs)
  types/        # shared data structures (Config, Dependency, ProjectManifest)
  util/         # shared utilities (empty stubs)
```

- The spec says `utils/`; the repo uses `util/` (singular). Keep `util/`.
- `project/` only understands projects (parsing/detection). It must never download documentation.
- `detect.rs` delegates traversal to `src/scanner/`, which reads `.gitignore` / `.dkvignore` and
  applies config exclusions. Excluded directories (e.g. `target/`, `node_modules/`, `.git`) are
  pruned from the walk, so detection no longer descends into build-artifact dirs.

## CLI state (real vs stub)

- Implemented: `init` (writes default config), `scan` (prints detected languages/frameworks/packages).
- Stubs printing placeholder text in `main.rs`: `sync`, `deps`, `search`, `open`, `update`, `doctor`, `stats`.
- `Commands::Scan` takes `SyncArgs`; `ScanArgs` (`cli.rs:101`) is dead code. Harmonize when touching `cli.rs`.
- `project/bun.rs` is an empty, unregistered stub — no phase owns it yet.

## Dependencies

Used: anyhow, clap, directories, owo-colors, serde, serde_json, serde_yaml (error.rs only), thiserror,
toml, tracing, tracing-subscriber, walkdir. Declared but unused (planned for later phases):
cargo_metadata, semver, toml_edit. Dev: tempfile. Spec rule: avoid unnecessary dependencies — only
add a dep when the current milestone actually needs it.

## Testing

No tests exist yet; `tests/` does not exist. Fixtures belong under `tests/fixtures/` (spec requires
workspace-detection fixtures). Use `tempfile` for scratch dirs.

## Workflow rules (from the project spec)

- Complete one milestone, run all five gates, then STOP and wait for review. Never chain milestones.
- Never rewrite working modules, introduce breaking changes, rename public APIs, create duplicate
  types, move files, or swap the architecture without explicit instruction.
- Every public item gets rustdoc; every `Result`-returning fn gets a `# Errors` section.
- Milestone deliverable: summary, files created/modified, public API changes, tests added, remaining
  work, exact commands executed, verification results.

## Repo hygiene

- `.gitignore` only lists `/target`. Don't commit untracked `repomix-output.xml` (regenerate via
  `repomix`; config: `repomix.config.json`) or `test.txt` (stray `dkv scan` debug output).
- No git remotes; work happens on feature branches (currently `feature/milestone-2-config`).
- Config quirk: default vault path is the literal string `~/Documents/DevKnowledgeVault` — `~` is
  never expanded anywhere.
