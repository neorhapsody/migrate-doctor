use crate::model::{Finding, Severity};
use crate::rules::postgres as rules;
use sqlparser::ast::{
    AlterTableOperation, ColumnOption, ObjectType, Spanned, Statement, TableConstraint,
};
use std::path::Path;

pub(crate) const PARSER_ID: &str = "postgres-sql";

pub(crate) fn stmt_line(stmt: &Statement) -> Option<u32> {
    let line = stmt.span().start.line;
    if line == 0 {
        None
    } else {
        Some(line as u32)
    }
}

pub(crate) fn lint_statement(stmt: &Statement, file: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    let line = stmt_line(stmt);

    match stmt {
        Statement::CreateIndex(create) => {
            if !create.concurrently {
                out.push(Finding {
                    rule_id: rules::REQUIRE_CONCURRENT_INDEX,
                    severity: Severity::Error,
                    message: "CREATE INDEX without CONCURRENTLY can lock writes on large tables; use CREATE INDEX CONCURRENTLY in a separate transaction.".into(),
                    file: file.to_path_buf(),
                    line,
                    parser_id: PARSER_ID,
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
                rule_id: rules::BAN_DROP,
                severity: Severity::Warning,
                message: msg.into(),
                file: file.to_path_buf(),
                line,
                parser_id: PARSER_ID,
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
                                rule_id: rules::ADDING_FIELD_WITH_DEFAULT,
                                severity: Severity::Warning,
                                message: "ADD COLUMN with DEFAULT may rewrite the whole table on PostgreSQL < 11 or be expensive on large tables; consider add nullable → backfill → set default.".into(),
                                file: file.to_path_buf(),
                                line,
                                parser_id: PARSER_ID,
                            });
                        }
                    }
                    AlterTableOperation::AddConstraint(constraint) => {
                        if matches!(constraint, TableConstraint::ForeignKey { .. }) {
                            out.push(Finding {
                                rule_id: rules::PREFER_FOREIGN_KEY_NOT_VALID,
                                severity: Severity::Warning,
                                message: "ADD FOREIGN KEY often validates all rows and can lock; prefer NOT VALID then VALIDATE CONSTRAINT in a follow-up migration.".into(),
                                file: file.to_path_buf(),
                                line,
                                parser_id: PARSER_ID,
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

#[cfg(test)]
mod tests {
    use super::super::parse::parse_statements;
    use super::*;
    use std::path::Path;

    fn lint_sql(path: &Path, sql: &str) -> Vec<Finding> {
        let stmts = parse_statements(sql).unwrap();
        let mut findings = Vec::new();
        for stmt in &stmts {
            findings.extend(lint_statement(stmt, path));
        }
        findings
    }

    #[test]
    fn flags_non_concurrent_index() {
        let sql = "CREATE INDEX idx_users_email ON users (email);";
        let f = lint_sql(Path::new("t.sql"), sql);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, rules::REQUIRE_CONCURRENT_INDEX);
    }

    #[test]
    fn allows_concurrent_index() {
        let sql = "CREATE INDEX CONCURRENTLY idx_users_email ON users (email);";
        let f = lint_sql(Path::new("t.sql"), sql);
        assert!(f.is_empty());
    }

    #[test]
    fn flags_add_column_with_default() {
        let sql = "ALTER TABLE users ADD COLUMN legacy_id text DEFAULT '';";
        let f = lint_sql(Path::new("t.sql"), sql);
        assert!(f
            .iter()
            .any(|x| x.rule_id == rules::ADDING_FIELD_WITH_DEFAULT));
    }
}
