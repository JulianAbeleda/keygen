#![forbid(unsafe_code)]
//! Stable diagnostics for importers, compilers, and hosts.
//!
//! Diagnostic codes and source identifiers are deliberately strings so that
//! adding a new producer does not require a breaking enum change. Callers
//! should use the documented constants where possible.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SOURCE_IMPORT: &str = "import";
pub const SOURCE_PACKAGE: &str = "package";
pub const SOURCE_RENDER: &str = "render";
pub const SOURCE_RUNTIME: &str = "runtime";

pub const CODE_INVALID_INPUT: &str = "KGD-001";
pub const CODE_UNSUPPORTED: &str = "KGD-002";
pub const CODE_MISSING_INPUT: &str = "KGD-003";
pub const CODE_INTEGRITY: &str = "KGD-004";
pub const CODE_INTERNAL: &str = "KGD-999";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub source: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

impl Diagnostic {
    pub fn new(
        code: impl Into<String>,
        severity: Severity,
        source: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            source: source.into(),
            message: message.into(),
            fields: BTreeMap::new(),
        }
    }

    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Serialize only stable, non-path-bearing diagnostic data.
    ///
    /// Fields that look like local paths or environment values are omitted;
    /// this makes diagnostics safe to attach to reports and CI artifacts.
    pub fn redacted_json(&self) -> Result<String, serde_json::Error> {
        let mut redacted = self.clone();
        redacted.fields.retain(|key, _| !is_sensitive_key(key));
        serde_json::to_string(&redacted)
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("path") || key.contains("home") || key.contains("token") || key.contains("secret")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_json_round_trip() {
        let diagnostic = Diagnostic::new(
            CODE_UNSUPPORTED,
            Severity::Warning,
            SOURCE_IMPORT,
            "unsupported object",
        )
        .field("object_type", "Animator")
        .field("path", ["", "Users", "private", "recovery"].join("/"));
        let json = diagnostic.redacted_json().unwrap();
        let private_prefix = ["", "Users", "private"].join("/");
        assert!(!json.contains(&private_prefix));
        let parsed: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, CODE_UNSUPPORTED);
        assert_eq!(
            parsed.fields.get("object_type").map(String::as_str),
            Some("Animator")
        );
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let result = serde_json::from_str::<Diagnostic>(
            r#"{"code":"KGD-001","severity":"error","source":"import","message":"x","extra":true}"#,
        );
        assert!(result.is_err());
    }
}
