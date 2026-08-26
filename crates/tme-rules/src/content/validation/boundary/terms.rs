//! The content boundary's denylist: a public mechanism over a private list.
//!
//! The predecessor shipped its source-lineage denylist as a literal array in
//! public code, which named in public the very lineage the denylist existed to
//! keep out of public surfaces. This module inverts that: the loading, the
//! matching rule, and the fail-closed behavior are public; the terms are data.
//!
//! **Where the terms come from.** `TME_BANNED_TERMS_FILE` names the data file.
//! With the variable unset, the loader looks for `.boundary/banned-terms.txt`
//! by walking up from the current directory. The workspace's
//! `.cargo/config.toml` points cargo-run processes at the tracked synthetic
//! fixture (invented nonsense terms), which is what lets a clean clone build
//! and test with the private `.boundary/` root absent.
//!
//! **Fail closed.** A missing, unreadable, non-UTF-8, or entry-less file is an
//! error at every scan. It is never a skip and never a pass. A boundary check
//! that goes quiet when its input disappears is worse than no check.
//!
//! **Scope.** This scanner guards content at load time, including content that
//! never enters the repository. Repository-wide enforcement over carried files
//! is `tools/check_boundary_terms.py`, which reads the same file format and
//! applies the same matching rule.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Environment variable naming the denylist data file.
pub const TERMS_PATH_ENV: &str = "TME_BANNED_TERMS_FILE";

/// Path searched when the environment variable is unset.
pub const DEFAULT_TERMS_PATH: &str = ".boundary/banned-terms.txt";

/// A compiled denylist. Each term is stored as its alphanumeric parts so that
/// matching tolerates any separator between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannedTerms {
    terms: Vec<CompiledTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledTerm {
    display: String,
    parts: Vec<String>,
}

impl BannedTerms {
    /// Compile a denylist from raw entries.
    ///
    /// An entry with no alphanumeric content, or an empty entry list, is a
    /// broken input rather than an empty policy.
    pub fn from_entries<I, S>(entries: I) -> Result<Self, TermsError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut terms = Vec::new();
        for entry in entries {
            let display = entry.as_ref().to_string();
            let parts = alphanumeric_parts(&display);
            if parts.is_empty() {
                return Err(TermsError::EmptyTerm(display));
            }
            terms.push(CompiledTerm { display, parts });
        }
        if terms.is_empty() {
            return Err(TermsError::NoEntries);
        }
        Ok(Self { terms })
    }

    /// Load a denylist from a one-entry-per-line data file. `#` starts a
    /// comment anywhere on a line.
    pub fn load_from_file(path: &Path) -> Result<Self, TermsError> {
        let raw = fs::read_to_string(path)
            .map_err(|error| TermsError::Unreadable(path.to_path_buf(), error.to_string()))?;
        let entries = raw
            .lines()
            .map(|line| line.split('#').next().unwrap_or("").trim())
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Err(TermsError::Unreadable(
                path.to_path_buf(),
                "file has no entries once comments and blank lines are removed".to_string(),
            ));
        }
        Self::from_entries(entries)
    }

    /// Return the first term the value carries, if any.
    ///
    /// Matching is case-insensitive with word-ish boundaries: a match must not
    /// be flanked by an ASCII alphanumeric, so a short term never fires from
    /// inside a longer word. Inside a term, any run of separators matches any
    /// run of non-alphanumeric characters *including none*, so a two-word term
    /// such as `"vorpal grimble"` also catches `Vorpal.Grimble`,
    /// `vorpal_grimble`, and `VorpalGrimble`.
    pub fn first_match(&self, value: &str) -> Option<&str> {
        let lowered = value.to_ascii_lowercase();
        let haystack = lowered.as_bytes();
        self.terms
            .iter()
            .find(|term| term_matches(haystack, &term.parts))
            .map(|term| term.display.as_str())
    }

    /// The number of compiled terms. Diagnostics only; the terms themselves are
    /// deliberately not exposed.
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

/// Why a denylist could not be established. Every variant is fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermsError {
    /// No data file was found at the configured or default location.
    Missing(PathBuf),
    /// The data file exists but could not be read as UTF-8 text, or carried no
    /// entries once comments and blank lines were removed.
    Unreadable(PathBuf, String),
    /// A term carried no alphanumeric content, so it could never match.
    EmptyTerm(String),
    /// The entry list was empty.
    NoEntries,
}

impl std::fmt::Display for TermsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(path) => write!(
                formatter,
                "content boundary denylist is missing: {} (set {TERMS_PATH_ENV} or provide {DEFAULT_TERMS_PATH})",
                path.display()
            ),
            Self::Unreadable(path, detail) => write!(
                formatter,
                "content boundary denylist is unreadable: {} ({detail})",
                path.display()
            ),
            Self::EmptyTerm(term) => write!(
                formatter,
                "content boundary denylist term has no alphanumeric content: {term:?}"
            ),
            Self::NoEntries => write!(formatter, "content boundary denylist has no entries"),
        }
    }
}

impl std::error::Error for TermsError {}

static PROCESS_TERMS: OnceLock<Result<BannedTerms, TermsError>> = OnceLock::new();

/// The process-wide denylist, loaded from the configured data file on first
/// use. The result — success or failure — is cached, so a broken configuration
/// keeps failing rather than intermittently passing.
pub fn process_terms() -> Result<&'static BannedTerms, &'static TermsError> {
    PROCESS_TERMS
        .get_or_init(|| resolve_terms_path().and_then(|path| BannedTerms::load_from_file(&path)))
        .as_ref()
}

fn resolve_terms_path() -> Result<PathBuf, TermsError> {
    if let Some(configured) = env::var_os(TERMS_PATH_ENV) {
        let path = PathBuf::from(configured);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(TermsError::Missing(path))
        };
    }
    let start = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for directory in start.ancestors() {
        let candidate = directory.join(DEFAULT_TERMS_PATH);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(TermsError::Missing(start.join(DEFAULT_TERMS_PATH)))
}

fn alphanumeric_parts(term: &str) -> Vec<String> {
    term.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn term_matches(haystack: &[u8], parts: &[String]) -> bool {
    let first = parts[0].as_bytes();
    for start in 0..=haystack.len().saturating_sub(first.len()) {
        if start > 0 && haystack[start - 1].is_ascii_alphanumeric() {
            continue;
        }
        if let Some(end) = match_parts_at(haystack, start, parts)
            && (end == haystack.len() || !haystack[end].is_ascii_alphanumeric())
        {
            return true;
        }
    }
    false
}

fn match_parts_at(haystack: &[u8], start: usize, parts: &[String]) -> Option<usize> {
    let mut cursor = start;
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            while cursor < haystack.len() && !haystack[cursor].is_ascii_alphanumeric() {
                cursor += 1;
            }
        }
        let bytes = part.as_bytes();
        if haystack.len() < cursor + bytes.len() || &haystack[cursor..cursor + bytes.len()] != bytes
        {
            return None;
        }
        cursor += bytes.len();
    }
    Some(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms() -> BannedTerms {
        BannedTerms::from_entries(["zorbelquux", "vorpal grimble"]).expect("fixture compiles")
    }

    #[test]
    fn matches_are_case_insensitive() {
        assert_eq!(terms().first_match("a ZorBelQuux here"), Some("zorbelquux"));
    }

    #[test]
    fn a_term_never_fires_from_inside_a_longer_word() {
        assert_eq!(terms().first_match("zorbelquuxen"), None);
        assert_eq!(terms().first_match("prezorbelquux"), None);
    }

    #[test]
    fn separators_inside_a_term_are_tolerated_including_none() {
        for value in [
            "vorpal grimble",
            "Vorpal.Grimble",
            "vorpal_grimble",
            "VorpalGrimble",
            "vorpal   --  grimble",
        ] {
            assert_eq!(
                terms().first_match(value),
                Some("vorpal grimble"),
                "{value:?} must match"
            );
        }
    }

    #[test]
    fn a_multi_part_term_still_respects_the_outer_boundary() {
        assert_eq!(terms().first_match("xvorpalgrimble"), None);
        assert_eq!(terms().first_match("vorpalgrimbles"), None);
    }

    #[test]
    fn a_clean_value_matches_nothing() {
        assert_eq!(terms().first_match("an ordinary fixture name"), None);
    }

    #[test]
    fn an_entry_list_without_alphanumeric_content_is_rejected() {
        assert_eq!(
            BannedTerms::from_entries(["---"]),
            Err(TermsError::EmptyTerm("---".to_string()))
        );
        assert_eq!(
            BannedTerms::from_entries(Vec::<String>::new()),
            Err(TermsError::NoEntries)
        );
    }

    #[test]
    fn a_missing_data_file_fails_closed() {
        let missing = Path::new("/nonexistent/tme/banned-terms.txt");
        let error = BannedTerms::load_from_file(missing).expect_err("missing file must fail");
        assert!(matches!(error, TermsError::Unreadable(_, _)));
    }

    #[test]
    fn the_tracked_synthetic_fixture_loads_and_matches() {
        // Loaded by explicit path, not through the process-wide resolver: this
        // test proves the file format and the matcher, and must not depend on
        // which denylist the surrounding process happens to be configured with.
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/synthetic-terms.txt");
        let terms = BannedTerms::load_from_file(&fixture).expect("tracked fixture must load");
        assert_eq!(terms.len(), 5, "the fixture carries five synthetic terms");
        assert_eq!(terms.first_match("zorbelquux"), Some("zorbelquux"));
        assert_eq!(terms.first_match("Vorpal Grimble"), Some("vorpal grimble"));
        assert_eq!(terms.first_match("an ordinary name"), None);
    }

    /// A tripwire for test runners, not for the code under test.
    ///
    /// `.cargo/config.toml` supplies `TME_BANNED_TERMS_FILE` through cargo's
    /// `[env]` table, which cargo injects only into processes **it** launches.
    /// A runner that executes the compiled test binaries directly bypasses it,
    /// and the tests then run against whatever denylist the machine happens to
    /// have — which produced a red with nothing to do with the code once
    /// already. This asserts the variable arrived, so any runner that stops
    /// going through cargo fails here, loudly, instead of somewhere confusing.
    #[test]
    fn cargos_env_table_reaches_this_test_process() {
        assert!(
            std::env::var_os(TERMS_PATH_ENV).is_some(),
            "{TERMS_PATH_ENV} is unset: this test process was not launched by cargo, so \
             .cargo/config.toml's [env] table never applied. Run the workspace through \
             `cargo test` (tools/run_rust_tests.py does), not by executing the test binaries."
        );
    }

    #[test]
    fn the_process_denylist_resolves_under_the_configured_environment() {
        // Which list is configured is deliberately not asserted — that is
        // deployment configuration. What must hold everywhere is that the
        // resolver establishes a non-empty list rather than falling open.
        let terms = process_terms().expect("the configured denylist must resolve");
        assert!(!terms.is_empty());
    }
}
