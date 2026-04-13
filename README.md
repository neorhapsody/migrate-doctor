# migrate-doctor

A small CLI that lints **PostgreSQL-oriented `.sql` migration files** for patterns that often cause locks, downtime, or risky deploys. It is an MVP: one parser (`postgres-sql`), static analysis only, no database connection.

## Install

Requires [Rust](https://www.rust-lang.org/tools/install).

From a clone of this repository:

```bash
cargo install --path .
```

Or install directly from Git:

```bash
cargo install --git https://github.com/neorhapsody/migrate-doctor.git
```

The binary is named `migrate-doctor`.

## Usage

Lint one file or every `.sql` file under directories (recursive):

```bash
migrate-doctor check path/to/migration.sql
migrate-doctor check ./db/migrations
```

Print rule ids and default severities (for `[rules]` keys in TOML):

```bash
migrate-doctor list-rules
migrate-doctor list-rules --json
```

### Flags (`check`)


| Flag              | Meaning                                                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------ |
| `--json`          | Print a JSON array of findings to stdout (human-readable lines go to stderr unless this is set). |
| `--deny-warnings` | Exit with status 1 if any **warning** is present (default: only **errors** fail the run).        |
| `--config PATH`   | TOML file with per-rule enable/disable (see below).                                              |


Use `migrate-doctor check --help` for full CLI help.

If a file is not valid **PostgreSQL-oriented** SQL for this parser, `check` exits with an error that includes the **file path**, the **sqlparser** message, and a short hint (the tool does not run migrations against a database).

### Exit status

- `0` — No findings with severity **error**, and (unless `--deny-warnings`) no need to fail on warnings.
- `1` — At least one **error**, or any **warning** when `--deny-warnings` is set.

## GitHub Actions

This repository runs [`.github/workflows/ci.yml`](.github/workflows/ci.yml) on pushes to `main` and on pull requests. The job runs, in order:

1. `cargo fmt --all -- --check` (requires `rustfmt`)
2. `cargo clippy --locked --all-targets -- -D warnings` (requires `clippy`)
3. `cargo test --locked`
4. `cargo run --locked -- check examples/migrations/002_ok.sql`

The `examples/migrations/001_bad.sql` sample is intentionally full of violations and is not part of that check.

## Releases

[GitHub Releases](https://github.com/neorhapsody/migrate-doctor/releases) are produced by [cargo-dist](https://github.com/axodotdev/cargo-dist) via [`.github/workflows/release.yml`](.github/workflows/release.yml). Push a version tag such as `v0.1.0` (see that file for accepted patterns), with `version` in `Cargo.toml` matching the release. The workflow also runs `dist plan` on pull requests so release config stays valid. Release targets and `dist` version live in [`dist-workspace.toml`](dist-workspace.toml); after changing them, run `dist generate` and commit the updated workflow.

To lint migrations in your own repo, add a workflow (adjust `db/migrations` to your path). Example using `cargo install` from Git:

```yaml
name: Lint migrations

on:
  pull_request:

jobs:
  migrate-doctor:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install migrate-doctor
        run: cargo install --git https://github.com/neorhapsody/migrate-doctor.git --locked
      - name: Check SQL migrations
        run: migrate-doctor check db/migrations --deny-warnings
```

Use `--config path/to/migrate-doctor.toml` on the last line if you disable rules locally.

## Configuration

Optional TOML file passed with `--config`. Rules are **enabled** by default; set a rule to `false` to disable it.

Keys use the form `"parser-id/rule-suffix"` (quotes required in TOML when the key contains `/`).

```toml
[rules]
"postgres-sql/ban-drop" = false
"postgres-sql/prefer-foreign-key-not-valid" = false
```

An empty file is valid and leaves all rules enabled.

## Rules (parser `postgres-sql`)


| Rule id                                     | Default severity | What it flags                                                                                                            |
| ------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `postgres-sql/require-concurrent-index`     | error            | `CREATE INDEX` without `CONCURRENTLY`                                                                                    |
| `postgres-sql/ban-drop`                     | warning          | `DROP` (wording depends on object type)                                                                                  |
| `postgres-sql/adding-field-with-default`    | warning          | `ALTER TABLE ... ADD COLUMN ...` with a `DEFAULT`                                                                        |
| `postgres-sql/prefer-foreign-key-not-valid` | warning          | `ALTER TABLE ... ADD ... FOREIGN KEY` (PostgreSQL `NOT VALID` is not represented in the parser; this is a general nudge) |


## JSON output

With `--json`, each finding includes at least: `rule_id`, `severity`, `message`, `file`, `line` (optional), `parser_id`.

## Development

Before opening a PR, run the same checks as CI (install `rustfmt` and `clippy` with `rustup component add rustfmt clippy` if needed):

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --locked -- check examples/migrations/002_ok.sql
```

`examples/migrations/001_bad.sql` is intentionally full of violations; use it to see output, not for a green check.

Contributor notes for AI-assisted edits are in `[AGENTS.md](AGENTS.md)`.

## Scope

- **In scope:** `.sql` files interpreted with a PostgreSQL-oriented dialect via [sqlparser](https://crates.io/crates/sqlparser).
- **Out of scope for now:** live database stats, ORM-specific formats, non-Postgres engines, guaranteed absence of false positives/negatives.

## License

MIT. See [`LICENSE`](LICENSE) and `Cargo.toml`.