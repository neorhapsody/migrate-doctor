mod postgres_sql;

use crate::model::Finding;
use std::path::Path;

pub trait MigrationParser: Send + Sync {
    fn id(&self) -> &'static str;

    fn extensions(&self) -> &'static [&'static str];

    fn lint(&self, path: &Path, source: &str) -> anyhow::Result<Vec<Finding>>;
}

pub fn default_parsers() -> Vec<Box<dyn MigrationParser>> {
    vec![Box::new(postgres_sql::PostgresSqlParser)]
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
}

pub fn lint_file(
    path: &Path,
    source: &str,
    parsers: &[Box<dyn MigrationParser>],
) -> anyhow::Result<Vec<Finding>> {
    let ext = extension_of(path).unwrap_or_default();
    let parser = parsers
        .iter()
        .find(|p| p.extensions().contains(&ext.as_str()))
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

pub fn lint_file_with_defaults(path: &Path, source: &str) -> anyhow::Result<Vec<Finding>> {
    let parsers = default_parsers();
    lint_file(path, source, &parsers)
}
