use std::path::Path;

use serde::Serialize;

use crate::loading::{ValidationBatchContext, load_simulation_with_context};

pub const CONTENT_VALIDATION_SCHEMA_VERSION: u32 = 1;
pub const CONTENT_VALIDATION_KIND: &str = "tme_content_validation_result";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentValidationReport {
    pub schema_version: u32,
    pub kind: &'static str,
    pub results: Vec<ContentValidationInputResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentValidationInputResult {
    pub input: String,
    pub valid: bool,
    pub diagnostics: Vec<ContentValidationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentValidationDiagnostic {
    pub component: &'static str,
    pub pointer: String,
    pub message: String,
}

pub fn validate_content_paths(inputs: impl IntoIterator<Item = String>) -> ContentValidationReport {
    let mut context = ValidationBatchContext::default();
    let results = inputs
        .into_iter()
        .map(
            |input| match load_simulation_with_context(Path::new(&input), &mut context) {
                Ok(_) => ContentValidationInputResult {
                    input,
                    valid: true,
                    diagnostics: Vec::new(),
                },
                Err(error) => ContentValidationInputResult {
                    input,
                    valid: false,
                    diagnostics: error
                        .diagnostics
                        .into_iter()
                        .map(|diagnostic| ContentValidationDiagnostic {
                            component: diagnostic.component.as_str(),
                            pointer: diagnostic.pointer,
                            message: diagnostic.message,
                        })
                        .collect(),
                },
            },
        )
        .collect();
    ContentValidationReport {
        schema_version: CONTENT_VALIDATION_SCHEMA_VERSION,
        kind: CONTENT_VALIDATION_KIND,
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_preserves_input_order_and_validity_invariant() {
        let valid = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-corpus/first_room.json")
            .display()
            .to_string();
        let missing = "definitely/missing.json".to_string();
        let report = validate_content_paths([valid.clone(), missing.clone()]);
        assert_eq!(report.results[0].input, valid);
        assert!(report.results[0].valid);
        assert!(report.results[0].diagnostics.is_empty());
        assert_eq!(report.results[1].input, missing);
        assert!(!report.results[1].valid);
        assert!(!report.results[1].diagnostics.is_empty());
    }
}
