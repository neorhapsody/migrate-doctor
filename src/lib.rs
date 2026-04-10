//! Core lint logic for PostgreSQL-oriented migration SQL.

use serde::Serialize;
use sqlparser::ast::{
    AlterTableOperation, ColumnOption, ObjectType, Spanned, Statement, TableConstraint,
};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    /// 1-based line when known
    pub line: Option<u32>,
}

fn parse_statements(sql: &str) -> anyhow::Result<Vec<Statement>> {
    let dialect = PostgreSqlDialect {};
    Parser::parse_sql(&dialect, sql).map_err(|e| anyhow::anyhow!(e))
}

fn stmt_line(stmt: &Statement) -> Option<u32> {
    let line = stmt.span().start.line;
    if line == 0 {
        None
    } else {
        Some(line as u32)
    }
}

fn lint_statement(stmt: &Statement, file: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    let line = stmt_line(stmt);

    match stmt {
        Statement::CreateIndex(create) => {
            if !create.concurrently {
                out.push(Finding {
                    rule_id: "require-concurrent-index",
                    severity: Severity::Error,
                    message: "CREATE INDEX without CONCURRENTLY can lock writes on large tables; use CREATE INDEX CONCURRENTLY in a separate transaction.".into(),
                    file: file.to_path_buf(),
                    line,
                });
            }
        }
        Statement::Drop { object_type, .. } => {
            let msg = match object_type {
                ObjectType::Table => {
                    "DROP TABLE can cause downtime or break running app versions; prefer expand/contract or multi-step rollout."
                }
                ObjectType::Index => {
                    "DROP INDEX can block queries; on PostgreSQL prefer CONCURRENTLY variants where applicable."
                }
                _ => {
                    "DROP can cause downtime or break running app versions; review carefully before deploy."
                }
            };
            out.push(Finding {
                rule_id: "ban-drop",
                severity: Severity::Warning,
                message: msg.into(),
                file: file.to_path_buf(),
                line,
            });
        }
        Statement::AlterTable { operations, .. } => {
            for op in operations {
                match op {
                    AlterTableOperation::AddColumn { column_def, .. } => {
                        let has_default = column_def
                            .options
                            .iter()
                            .any(|o| matches!(o.option, ColumnOption::Default(_)));
                        if has_default {
                            out.push(Finding {
                                rule_id: "adding-field-with-default",
                                severity: Severity::Warning,
                                message: "ADD COLUMN with DEFAULT may rewrite the whole table on PostgreSQL < 11 or be expensive on large tables; consider add nullable → backfill → set default.".into(),
                                file: file.to_path_buf(),
                                line,
                            });
                        }
                    }
                    AlterTableOperation::AddConstraint(constraint) => {
                        if matches!(constraint, TableConstraint::ForeignKey { .. }) {
                            // sqlparser 0.54 does not represent PostgreSQL NOT VALID on FKs; nudge for safer two-step flow.
                            out.push(Finding {
                                rule_id: "prefer-foreign-key-not-valid",
                                severity: Severity::Warning,
                                message: "ADD FOREIGN KEY often validates all rows and can lock; prefer NOT VALID then VALIDATE CONSTRAINT in a follow-up migration.".into(),
                                file: file.to_path_buf(),
                                line,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    out
}

/// Lint the contents of a single migration file.
pub fn lint_sql_file(path: &Path, sql: &str) -> anyhow::Result<Vec<Finding>> {
    let stmts = parse_statements(sql)?;
    let mut findings = Vec::new();
    for stmt in &stmts {
        findings.extend(lint_statement(stmt, path));
    }
    Ok(findings)
}

/// Collect `.sql` files under `root` (non-recursive if `root` is a file).
pub fn collect_sql_files(paths: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for p in paths {
        if p.is_file() {
            if p.extension().and_then(|e| e.to_str()) == Some("sql") {
                files.push(p.clone());
            }
        } else if p.is_dir() {
            for entry in walkdir::WalkDir::new(p)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path().to_path_buf();
                if path.extension().and_then(|e| e.to_str()) == Some("sql") {
                    files.push(path);
                }
            }
        } else {
            anyhow::bail!("path not found: {}", p.display());
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_non_concurrent_index() {
        let sql = "CREATE INDEX idx_users_email ON users (email);";
        let f = lint_sql_file(Path::new("t.sql"), sql).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "require-concurrent-index");
    }

    #[test]
    fn allows_concurrent_index() {
        let sql = "CREATE INDEX CONCURRENTLY idx_users_email ON users (email);";
        let f = lint_sql_file(Path::new("t.sql"), sql).unwrap();
        assert!(f.is_empty());
    }

    #[test]
    fn flags_add_column_with_default() {
        let sql = "ALTER TABLE users ADD COLUMN legacy_id text DEFAULT '';";
        let f = lint_sql_file(Path::new("t.sql"), sql).unwrap();
        assert!(f.iter().any(|x| x.rule_id == "adding-field-with-default"));
    }
}
