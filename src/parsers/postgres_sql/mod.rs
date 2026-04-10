//! PostgreSQL migration files parsed as SQL (`sqlparser`, PostgreSQL dialect).
//!
//! Add new rules in [`lint`] and wire any new statement shapes there. Rule ids use
//! the form `postgres-sql/{suffix}` (same stem as [`MigrationParser::id`](crate::parsers::MigrationParser::id)).

mod lint;
mod parse;

use crate::model::Finding;
use crate::parsers::MigrationParser;
use std::path::Path;

/// Linter for `.sql` files using the PostgreSQL dialect.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostgresSqlParser;

impl MigrationParser for PostgresSqlParser {
    fn id(&self) -> &'static str {
        lint::PARSER_ID
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".sql"]
    }

    fn lint(&self, path: &Path, source: &str) -> anyhow::Result<Vec<Finding>> {
        let stmts = parse::parse_statements(source)?;
        let mut findings = Vec::new();
        for stmt in &stmts {
            findings.extend(lint::lint_statement(stmt, path));
        }
        Ok(findings)
    }
}
