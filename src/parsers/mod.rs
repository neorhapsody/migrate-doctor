//! Pluggable migration parsers.
//!
//! ## Adding a parser
//!
//! 1. Create a submodule directory, e.g. `parsers/my_format/`, with:
//!    - `mod.rs` — a type that implements [`MigrationParser`]
//!    - optional `parse.rs`, `lint.rs`, etc.
//! 2. Declare the module here and append your type to [`default_parsers`].
//!
//! Parsers are selected by file extension (first match in [`default_parsers`] order).

mod postgres_sql;

use crate::model::Finding;
use std::path::Path;

/// One migration format (SQL dialect, ORM export, etc.).
pub trait MigrationParser: Send + Sync {
    /// Stable id for diagnostics and JSON output (e.g. `postgres-sql`).
    fn id(&self) -> &'static str;

    /// File extensions this parser handles, including the dot (e.g. `.sql`, `.prisma`).
    fn extensions(&self) -> &'static [&'static str];

    /// Parse `source` and return findings for `path`.
    fn lint(&self, path: &Path, source: &str) -> anyhow::Result<Vec<Finding>>;
}

/// Built-in parsers, in precedence order (first extension match wins).
pub fn default_parsers() -> Vec<Box<dyn MigrationParser>> {
    vec![Box::new(postgres_sql::PostgresSqlParser::default())]
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
}

/// Run the first parser whose [`MigrationParser::extensions`] matches `path`.
pub fn lint_file(
    path: &Path,
    source: &str,
    parsers: &[Box<dyn MigrationParser>],
) -> anyhow::Result<Vec<Finding>> {
    let ext = extension_of(path).unwrap_or_default();
    let parser = parsers
        .iter()
        .find(|p| p.extensions().iter().any(|e| *e == ext.as_str()))
        .ok_or_else(|| {
            let ids: Vec<_> = parsers.iter().map(|p| p.id()).collect();
            anyhow::anyhow!(
                "no parser registered for extension {:?} (path: {}); known parsers: {:?}",
                ext,
                path.display(),
                ids
            )
        })?;
    parser.lint(path, source)
}

/// Lint using [`default_parsers`].
pub fn lint_file_with_defaults(path: &Path, source: &str) -> anyhow::Result<Vec<Finding>> {
    let parsers = default_parsers();
    lint_file(path, source, &parsers)
}
