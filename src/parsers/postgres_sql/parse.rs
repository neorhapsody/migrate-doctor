use sqlparser::ast::Statement;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::path::Path;

pub(crate) fn parse_statements(sql: &str, path: &Path) -> anyhow::Result<Vec<Statement>> {
    let dialect = PostgreSqlDialect {};
    Parser::parse_sql(&dialect, sql).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse {} as PostgreSQL-oriented SQL (sqlparser): {e}\n\
             Hint: fix syntax or split statements; this tool does not execute SQL.",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_error_includes_path() {
        let err =
            parse_statements("NOT SQL AT ALL;;;", Path::new("db/migrations/bad.sql")).unwrap_err();
        let s = format!("{err:#}");
        assert!(s.contains("bad.sql"), "expected path in error, got: {s}");
    }
}
