//! TOML configuration for enabling or disabling lint rules.
//!
//! Example `migrate-doctor.toml`:
//!
//! ```toml
//! [rules]
//! "postgres-sql/ban-drop" = false
//! "postgres-sql/require-concurrent-index" = true
//! ```
//!
//! Omitted rules default to **enabled**. Set a rule to `false` to disable it.

use crate::model::Finding;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Rule ids shipped by built-in parsers (for reference and tests).
pub const KNOWN_RULE_IDS: &[&str] = &[
    "postgres-sql/require-concurrent-index",
    "postgres-sql/ban-drop",
    "postgres-sql/adding-field-with-default",
    "postgres-sql/prefer-foreign-key-not-valid",
];

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    rules: HashMap<String, bool>,
}

impl Config {
    /// Load from a TOML file. An empty file yields the default config (all rules on).
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let s = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("read config {}: {e}", path.display()))?;
        if s.trim().is_empty() {
            return Ok(Self::default());
        }
        toml::from_str(&s).map_err(|e| {
            anyhow::anyhow!(
                "parse config {}: {e}\n\
                 Expected a [rules] table with rule ids as keys and booleans (true = on, false = off).",
                path.display()
            )
        })
    }

    /// Whether this rule should run. Missing entries default to `true`.
    pub fn rule_enabled(&self, rule_id: &str) -> bool {
        self.rules.get(rule_id).copied().unwrap_or(true)
    }

    /// Drop findings for disabled rules.
    pub fn filter_findings(&self, findings: Vec<Finding>) -> Vec<Finding> {
        findings
            .into_iter()
            .filter(|f| self.rule_enabled(f.rule_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rules_table() {
        let cfg: Config = toml::from_str(
            r#"
            [rules]
            "postgres-sql/ban-drop" = false
            "postgres-sql/require-concurrent-index" = true
            "#,
        )
        .unwrap();
        assert!(!cfg.rule_enabled("postgres-sql/ban-drop"));
        assert!(cfg.rule_enabled("postgres-sql/require-concurrent-index"));
        assert!(cfg.rule_enabled("postgres-sql/adding-field-with-default"));
    }

    #[test]
    fn filter_drops_disabled() {
        let cfg: Config = toml::from_str(
            r#"
            [rules]
            "postgres-sql/ban-drop" = false
            "#,
        )
        .unwrap();
        let findings = vec![
            Finding {
                rule_id: "postgres-sql/ban-drop",
                severity: crate::model::Severity::Warning,
                message: "x".into(),
                file: "a.sql".into(),
                line: None,
                parser_id: "postgres-sql",
            },
            Finding {
                rule_id: "postgres-sql/require-concurrent-index",
                severity: crate::model::Severity::Error,
                message: "y".into(),
                file: "a.sql".into(),
                line: None,
                parser_id: "postgres-sql",
            },
        ];
        let kept = cfg.filter_findings(findings);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].rule_id, "postgres-sql/require-concurrent-index");
    }
}
