use sqlparser::ast::Statement;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

pub(crate) fn parse_statements(sql: &str) -> anyhow::Result<Vec<Statement>> {
    let dialect = PostgreSqlDialect {};
    Parser::parse_sql(&dialect, sql).map_err(|e| anyhow::anyhow!(e))
}
