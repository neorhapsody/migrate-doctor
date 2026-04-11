# Agent notes: migrate-doctor

Context for humans and coding agents working in this repository.

## What this is

`migrate-doctor` is a Rust CLI that lints PostgreSQL-oriented `.sql` migration files using static analysis only (no database connection). It targets startup-style workflows: local runs and CI.

## Layout


| Path                        | Role                                                                            |
| --------------------------- | ------------------------------------------------------------------------------- |
| `src/main.rs`               | `clap` CLI (`check`, `list-rules`), exit codes, JSON/human output               |
| `src/lib.rs`                | Public API: `lint_sql_file`, `lint_sql_file_with_config`, `collect_sql_files`   |
| `src/model.rs`              | `Finding`, `Severity`                                                           |
| `src/rules/postgres/`       | Rule ids (`ids.rs`); add per-rule modules here as the set grows                 |
| `src/config.rs`             | TOML `[rules]`, `RULE_CATALOG` / `KNOWN_RULE_IDS` (use ids from `rules::postgres`) |
| `src/parsers/mod.rs`        | `MigrationParser` trait, `default_parsers()`, `lint_file` dispatch by extension |
| `src/parsers/postgres_sql/` | Parser `postgres-sql`: `parse.rs` (sqlparser), `lint.rs` (rules)                |


New migration formats should be new modules under `src/parsers/`, implement `MigrationParser`, register in `default_parsers()`, and use rule ids `{parser-id}/{suffix}`.

## Conventions

- **Rule ids** for `postgres-sql` live in [`rules/postgres/ids.rs`](src/rules/postgres/ids.rs); add a `pub const`, append `RULE_IDS`, add a `ListedRule` in `config.rs`, and implement the check under `parsers/postgres_sql/` (or split into `rules/postgres/<name>.rs` when large). TOML keys with `/` need quotes.
- **Findings** are produced for all rules first; `Config::filter_findings` removes disabled rules afterward (simple; optimize later if needed).
- **Tests**: parser/rule tests live in `src/parsers/postgres_sql/lint.rs` under `#[cfg(test)]`; config tests in `src/config.rs`.
- **Comments in code**: keep them **minimal**. Prefer clear names and structure over narration. Reserve comments for non-obvious rationale, invariants, or `///` on small public API surfaces where it helps callers. Avoid restating what the code already says, section banners, and large doc blocks on obvious helpers.

## Commands

Match `[.github/workflows/ci.yml](.github/workflows/ci.yml)` before pushing (install components if needed: `rustup component add rustfmt clippy`):

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --locked -- check examples/migrations/002_ok.sql
```

`examples/migrations/001_bad.sql` is intentionally full of violations (for manual demos). CI only runs the linter on `002_ok.sql`.

## CI

On pushes to `main` and on pull requests, `[.github/workflows/ci.yml](.github/workflows/ci.yml)` runs, in order: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, then `cargo run --locked -- check examples/migrations/002_ok.sql`. The toolchain step includes `rustfmt` and `clippy` components.

For user-facing install and rules documentation, prefer `[README.md](README.md)`.

## Dependencies (high level)

- `sqlparser` + PostgreSQL dialect for parsing (AST upgrades may require lint changes)
- `clap` CLI, `serde`/`serde_json`, `toml` for config, `walkdir` for directory walks

## Scope boundaries

Do not imply guarantees for engines other than what the active parser documents. New rules should stay dialect-accurate; avoid vague “generic SQL” safety claims without explicit heuristics and docs.