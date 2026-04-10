use anyhow::Context;
use clap::{Parser, Subcommand};
use migrate_doctor::config::Config;
use migrate_doctor::{collect_sql_files, lint_sql_file, lint_sql_file_with_config, Severity};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "migrate-doctor")]
#[command(about = "Minimal PostgreSQL migration safety linter (MVP)", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint SQL migration files
    Check {
        /// Files or directories to scan (.sql only)
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Emit JSON array of findings to stdout
        #[arg(long)]
        json: bool,

        /// Treat warnings as errors (non-zero exit)
        #[arg(long)]
        deny_warnings: bool,

        /// TOML config with [rules] table: "{parser-id}/{rule}" = true | false (omit = enabled)
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check {
            paths,
            json,
            deny_warnings,
            config,
        } => {
            let files = collect_sql_files(&paths).context("collect sql files")?;
            if files.is_empty() {
                anyhow::bail!("no .sql files found under given paths");
            }

            let cfg = config
                .as_ref()
                .map(|p| Config::from_path(p))
                .transpose()
                .context("load config file")?;

            let mut all = Vec::new();
            for path in &files {
                let sql =
                    fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
                let findings = match &cfg {
                    Some(c) => lint_sql_file_with_config(path, &sql, c),
                    None => lint_sql_file(path, &sql),
                }
                .with_context(|| format!("lint {}", path.display()))?;
                all.extend(findings);
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&all)?);
            } else if all.is_empty() {
                eprintln!("OK — no issues in {} file(s)", files.len());
            } else {
                for f in &all {
                    let sev = match f.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                    };
                    let loc = f
                        .line
                        .map(|l| format!(":{}:", l))
                        .unwrap_or_else(|| ":".to_string());
                    eprintln!(
                        "{} [{}] {}{} {}",
                        sev,
                        f.rule_id,
                        f.file.display(),
                        loc,
                        f.message
                    );
                }
            }

            let has_error = all.iter().any(|f| f.severity == Severity::Error);
            let has_warn = all.iter().any(|f| f.severity == Severity::Warning);
            if has_error || (deny_warnings && has_warn) {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
