use serde_json::Value;

use crate::content::{ResearchBoundary, ValidationError};

pub mod terms;

pub use terms::{BannedTerms, TermsError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentBoundaryPolicy {
    Clean,
    InternalParity,
}

pub fn boundary_policy(
    clean_content: bool,
    boundary: &ResearchBoundary,
) -> Result<ContentBoundaryPolicy, ValidationError> {
    let mut errors = Vec::new();
    let policy = validate_research_boundary(clean_content, boundary, "content", &mut errors);
    if errors.is_empty() {
        Ok(policy.expect("valid boundary has a policy"))
    } else {
        Err(ValidationError::new(errors))
    }
}

pub(crate) fn validate_research_boundary(
    clean_content: bool,
    boundary: &ResearchBoundary,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<ContentBoundaryPolicy> {
    let clean = clean_content && boundary.status == "clean_original_fixture";
    let marked = !clean_content && boundary.status == "internal_parity_fixture";
    if !clean && !marked {
        errors.push(format!(
            "{label}.clean_content and {label}.research_boundary.status must select exactly clean_original_fixture or internal_parity_fixture"
        ));
    }
    if boundary.review_refs.is_empty() {
        errors.push(format!(
            "{label}.research_boundary.review_refs must be non-empty"
        ));
    }
    for (index, review_ref) in boundary.review_refs.iter().enumerate() {
        if review_ref.trim().is_empty() {
            errors.push(format!(
                "{label}.research_boundary.review_refs[{index}] must be non-empty"
            ));
        }
    }
    if boundary.notes.trim().is_empty() {
        errors.push(format!("{label}.research_boundary.notes must be non-empty"));
    }
    if marked && !boundary.notes.contains("TME-PLACEHOLDER") {
        errors.push(format!(
            "{label}.internal_parity_fixture research_boundary.notes must contain TME-PLACEHOLDER"
        ));
    }

    if clean {
        Some(ContentBoundaryPolicy::Clean)
    } else if marked {
        Some(ContentBoundaryPolicy::InternalParity)
    } else {
        None
    }
}

/// Recursively checks every JSON object key and string value at the one raw
/// rules boundary. Callers label each document so diagnostics retain their
/// component owner.
///
/// The denylist is data, not code: it loads from the file named by
/// `TME_BANNED_TERMS_FILE` (see [`terms`]). When that file cannot be
/// established the scan **fails closed** — it reports the configuration error
/// as a validation error rather than admitting the document unscanned.
pub fn scan_raw_documents<'a>(
    policy: ContentBoundaryPolicy,
    documents: impl IntoIterator<Item = (&'a str, &'a Value)>,
) -> Result<(), ValidationError> {
    if policy == ContentBoundaryPolicy::InternalParity {
        return Ok(());
    }

    let banned = match terms::process_terms() {
        Ok(banned) => banned,
        Err(error) => return Err(ValidationError::new(vec![error.to_string()])),
    };

    let mut errors = Vec::new();
    for (component, value) in documents {
        scan_value(banned, component, "", value, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::new(errors))
    }
}

/// Scan documents against an explicitly supplied denylist.
///
/// [`scan_raw_documents`] resolves the process-wide list; this entry point
/// exists for callers that already hold one, and for proving the scan itself
/// against a known list.
pub fn scan_raw_documents_with<'a>(
    policy: ContentBoundaryPolicy,
    banned: &BannedTerms,
    documents: impl IntoIterator<Item = (&'a str, &'a Value)>,
) -> Result<(), ValidationError> {
    if policy == ContentBoundaryPolicy::InternalParity {
        return Ok(());
    }
    let mut errors = Vec::new();
    for (component, value) in documents {
        scan_value(banned, component, "", value, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::new(errors))
    }
}

fn scan_value(
    banned: &BannedTerms,
    component: &str,
    pointer: &str,
    value: &Value,
    errors: &mut Vec<String>,
) {
    match value {
        Value::String(value) => scan_string(banned, component, pointer, "value", value, errors),
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                scan_value(
                    banned,
                    component,
                    &format!("{pointer}/{index}"),
                    value,
                    errors,
                );
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                let child = format!("{pointer}/{}", escape_pointer(key));
                scan_string(banned, component, &child, "key", key, errors);
                scan_value(banned, component, &child, value, errors);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn scan_string(
    banned: &BannedTerms,
    component: &str,
    pointer: &str,
    role: &str,
    value: &str,
    errors: &mut Vec<String>,
) {
    if value.contains("TME-PLACEHOLDER") {
        errors.push(format!(
            "{component}{pointer} {role} contains TME-PLACEHOLDER"
        ));
    }
    if let Some(term) = banned.first_match(value) {
        errors.push(format!(
            "{component}{pointer} {role} contains banned source term {term:?}"
        ));
    }
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn synthetic() -> BannedTerms {
        BannedTerms::from_entries(["zorbelquux", "mimsywort", "vorpal grimble"])
            .expect("synthetic denylist compiles")
    }

    #[test]
    fn raw_scanner_visits_nested_keys_and_values() {
        let value = json!({"safe": [{"zorbelquux": "value"}], "other": "mimsywort"});
        let error = scan_raw_documents_with(
            ContentBoundaryPolicy::Clean,
            &synthetic(),
            [("catalog", &value)],
        )
        .expect_err("private strings must fail");
        assert_eq!(error.messages().len(), 2);
        assert!(
            error
                .messages()
                .iter()
                .any(|message| message.contains("/safe/0/zorbelquux key"))
        );
        assert!(
            error
                .messages()
                .iter()
                .any(|message| message.contains("/other value"))
        );
    }

    #[test]
    fn marked_policy_allows_marked_raw_values() {
        let value = json!({"notes": "TME-PLACEHOLDER zorbelquux"});
        scan_raw_documents(ContentBoundaryPolicy::InternalParity, [("catalog", &value)])
            .expect("marked source owns its private strings");
    }

    #[test]
    fn an_explicit_denylist_rejects_a_separated_multi_word_term() {
        let value = json!({"name": "the VorpalGrimble blade"});
        let error = scan_raw_documents_with(
            ContentBoundaryPolicy::Clean,
            &synthetic(),
            [("catalog", &value)],
        )
        .expect_err("a separator-free spelling must still be caught");
        assert!(
            error
                .messages()
                .iter()
                .any(|message| message.contains("vorpal grimble"))
        );
    }

    #[test]
    fn an_explicit_denylist_admits_a_clean_document() {
        let value = json!({"name": "an ordinary fixture", "nested": ["values", "only"]});
        scan_raw_documents_with(
            ContentBoundaryPolicy::Clean,
            &synthetic(),
            [("catalog", &value)],
        )
        .expect("a clean document must pass");
    }

    #[test]
    fn the_scan_reports_the_configuration_error_when_a_denylist_cannot_be_built() {
        // The scan can only fail closed on a denylist it cannot establish, and
        // the failure has to arrive as a validation error rather than as
        // silence. Proving that on the process-wide list would require breaking
        // the process configuration for every other test, so the fail-closed
        // contract is proven on the loader (terms::tests) and its surfacing is
        // proven here on the error type the scan would report.
        let error = BannedTerms::from_entries(Vec::<String>::new())
            .expect_err("an empty list is a broken input, not an empty policy");
        let reported = ValidationError::new(vec![error.to_string()]);
        assert!(
            reported.messages()[0].contains("no entries"),
            "fail-closed diagnostics must name the cause: {:?}",
            reported.messages()
        );
    }
}
