pub mod config;
pub mod model;
pub mod parsers;

pub use model::{Finding, Severity};
use std::path::{Path, PathBuf};

pub fn lint_sql_file(path: &Path, sql: &str) -> anyhow::Result<Vec<Finding>> {
    parsers::lint_file_with_defaults(path, sql)
}

pub fn lint_sql_file_with_config(
    path: &Path,
    sql: &str,
    cfg: &config::Config,
) -> anyhow::Result<Vec<Finding>> {
    let findings = lint_sql_file(path, sql)?;
    Ok(cfg.filter_findings(findings))
}

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
