use crate::model::Finding;
use crate::rules::postgres as rules;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct ListedRule {
    pub id: &'static str,
    pub severity: &'static str,
    pub summary: &'static str,
}

pub const RULE_CATALOG: &[ListedRule] = &[
    ListedRule {
        id: rules::REQUIRE_CONCURRENT_INDEX,
        severity: "error",
        summary: "CREATE INDEX without CONCURRENTLY",
    },
    ListedRule {
        id: rules::BAN_DROP,
        severity: "warning",
        summary: "DROP objects (wording depends on object type)",
    },
    ListedRule {
        id: rules::ADDING_FIELD_WITH_DEFAULT,
        severity: "warning",
        summary: "ALTER TABLE ... ADD COLUMN ... with DEFAULT",
    },
    ListedRule {
        id: rules::PREFER_FOREIGN_KEY_NOT_VALID,
        severity: "warning",
        summary: "ALTER TABLE ... ADD ... FOREIGN KEY",
    },
];

pub const KNOWN_RULE_IDS: &[&str] = rules::RULE_IDS;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    rules: HashMap<String, bool>,
}

impl Config {
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

    pub fn rule_enabled(&self, rule_id: &str) -> bool {
        self.rules.get(rule_id).copied().unwrap_or(true)
    }

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
    use std::collections::HashSet;

    #[test]
    fn rule_catalog_matches_known_rule_ids() {
        let a: HashSet<_> = RULE_CATALOG.iter().map(|r| r.id).collect();
        let b: HashSet<_> = KNOWN_RULE_IDS.iter().copied().collect();
        assert_eq!(
            a, b,
            "RULE_CATALOG and KNOWN_RULE_IDS must list the same rule ids"
        );
    }

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
        assert!(!cfg.rule_enabled(rules::BAN_DROP));
        assert!(cfg.rule_enabled(rules::REQUIRE_CONCURRENT_INDEX));
        assert!(cfg.rule_enabled(rules::ADDING_FIELD_WITH_DEFAULT));
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
                rule_id: rules::BAN_DROP,
                severity: crate::model::Severity::Warning,
                message: "x".into(),
                file: "a.sql".into(),
                line: None,
                parser_id: "postgres-sql",
            },
            Finding {
                rule_id: rules::REQUIRE_CONCURRENT_INDEX,
                severity: crate::model::Severity::Error,
                message: "y".into(),
                file: "a.sql".into(),
                line: None,
                parser_id: "postgres-sql",
            },
        ];
        let kept = cfg.filter_findings(findings);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].rule_id, rules::REQUIRE_CONCURRENT_INDEX);
    }
}
