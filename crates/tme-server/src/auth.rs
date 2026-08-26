use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use tme_protocol as wire;
use tokio::sync::Semaphore;
use unicode_normalization::UnicodeNormalization;

pub const MIN_BLOCKLIST_ENTRIES: usize = 10_000;
pub const MAX_BLOCKLIST_ENTRIES: usize = 1_000_000;
pub const ARGON2_MEMORY_KIB: u32 = 65_536;
pub const ARGON2_PASSES: u32 = 3;
pub const ARGON2_LANES: u32 = 4;
pub const ARGON2_OUTPUT_BYTES: usize = 32;
pub const ARGON2_CONCURRENCY: usize = 2;
pub const MAX_LOGIN_SOURCE_BUCKETS: usize = 4_096;
pub const LOGIN_BURST: f64 = 5.0;
pub const LOGIN_REFILL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub(crate) struct AuthService {
    semaphore: Arc<Semaphore>,
    pub(crate) dummy_phc: Arc<String>,
}

pub(crate) struct AuthVerification {
    pub(crate) verified: bool,
    pub(crate) needs_rehash: bool,
}

impl AuthService {
    pub(crate) async fn new() -> Result<Self, String> {
        let service = Self {
            semaphore: Arc::new(Semaphore::new(ARGON2_CONCURRENCY)),
            dummy_phc: Arc::new(String::new()),
        };
        let dummy_phc = service
            .hash("tme dummy credential never authenticates")
            .await?;
        Ok(Self {
            dummy_phc: Arc::new(dummy_phc),
            ..service
        })
    }

    pub(crate) async fn hash(&self, password: &str) -> Result<String, String> {
        let normalized = normalize(password);
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|error| error.to_string())?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let salt = random_bytes::<16>()?;
            let salt = SaltString::encode_b64(&salt).map_err(|error| error.to_string())?;
            argon2_instance()?
                .hash_password(normalized.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?
    }

    pub(crate) async fn verify(
        &self,
        password: &str,
        phc: String,
    ) -> Result<AuthVerification, String> {
        let normalized = normalize(password);
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|error| error.to_string())?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let parsed = PasswordHash::new(&phc).map_err(|error| error.to_string())?;
            let verified = argon2_instance()?
                .verify_password(normalized.as_bytes(), &parsed)
                .is_ok();
            let mut salt_bytes = [0_u8; 64];
            let salt_len = parsed
                .salt
                .and_then(|salt| salt.decode_b64(&mut salt_bytes).ok())
                .map(<[u8]>::len);
            let needs_rehash = parsed.algorithm.as_str() != "argon2id"
                || parsed.version != Some(19)
                || parsed.params.get_decimal("m") != Some(ARGON2_MEMORY_KIB)
                || parsed.params.get_decimal("t") != Some(ARGON2_PASSES)
                || parsed.params.get_decimal("p") != Some(ARGON2_LANES)
                || salt_len != Some(16)
                || parsed.hash.as_ref().map(|hash| hash.len()) != Some(ARGON2_OUTPUT_BYTES);
            Ok(AuthVerification {
                verified,
                needs_rehash,
            })
        })
        .await
        .map_err(|error| error.to_string())?
    }
}

fn argon2_instance() -> Result<Argon2<'static>, String> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_PASSES,
        ARGON2_LANES,
        Some(ARGON2_OUTPUT_BYTES),
    )
    .map_err(|error| error.to_string())?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

pub(crate) fn validate_enrollment(
    username: &wire::Username,
    display_name: &wire::DisplayName,
    password: &wire::Password,
    blocklist: &BTreeSet<String>,
) -> Result<(), String> {
    let normalized = normalize(password.expose_for_verification());
    if !(15..=128).contains(&normalized.chars().count()) {
        return Err("password must contain 15-128 Unicode scalars".to_string());
    }
    // Service-name words are checked by containment, not equality. Passwords are
    // at least fifteen scalars, so an equality test against a six-letter word
    // can never fire — it would be a policy that reads as enforced and is not.
    let folded = normalized.to_lowercase();
    let username_folded = username.as_str().to_lowercase();
    if blocklist.contains(&normalized)
        || SERVICE_WORDS.iter().any(|word| folded.contains(word))
        || contains_username(&folded, &username_folded)
        || folded == normalize(display_name.as_str()).to_lowercase()
    {
        return Err("password is blocked".to_string());
    }
    Ok(())
}

/// The username is checked by containment from [`USERNAME_CONTAINMENT_FLOOR`]
/// scalars up, and not at all below it. Usernames may be three scalars, and
/// rejecting every password containing an arbitrary trigram would refuse an
/// enormous number of legitimate passwords for no real gain. Nothing is lost at
/// the short end: an equality test was already unreachable there, because a
/// password short enough to equal a sub-floor username dies at the length gate
/// first. See `docs/server-notes.md` for the ruling.
fn contains_username(folded_password: &str, folded_username: &str) -> bool {
    folded_username.chars().count() >= USERNAME_CONTAINMENT_FLOOR
        && folded_password.contains(folded_username)
}

pub(crate) const USERNAME_CONTAINMENT_FLOOR: usize = 5;

/// Context-specific words a password may not contain, per NIST SP 800-63B's
/// rule against the name of the service. Compared against the case-folded NFC
/// form of the password.
pub(crate) const SERVICE_WORDS: [&str; 2] = ["mortal", "estate"];

pub(crate) fn normalize(value: &str) -> String {
    value.nfc().collect()
}

pub(crate) fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
    Ok(bytes)
}

#[derive(Default)]
pub(crate) struct LoginLimits {
    sources: BTreeMap<IpAddr, TokenBucket>,
    accounts: BTreeMap<wire::AccountId, TokenBucket>,
}

impl LoginLimits {
    pub(crate) fn allow_source(&mut self, source: IpAddr) -> bool {
        let now = Instant::now();
        if !self.sources.contains_key(&source) {
            self.sources
                .retain(|_, bucket| !bucket.refresh_is_full(now));
            if self.sources.len() >= MAX_LOGIN_SOURCE_BUCKETS {
                return false;
            }
        }
        self.sources
            .entry(source)
            .or_insert_with(TokenBucket::new)
            .allow(now)
    }

    pub(crate) fn refund_source(&mut self, source: IpAddr) {
        if let Some(bucket) = self.sources.get_mut(&source) {
            bucket.tokens = (bucket.tokens + 1.0).min(LOGIN_BURST);
            if bucket.tokens == LOGIN_BURST {
                self.sources.remove(&source);
            }
        }
    }

    pub(crate) fn allow_account(&mut self, account_id: wire::AccountId) -> bool {
        self.accounts
            .entry(account_id)
            .or_insert_with(TokenBucket::new)
            .allow(Instant::now())
    }

    pub(crate) fn clear_account(&mut self, account_id: wire::AccountId) {
        self.accounts.remove(&account_id);
    }
}

struct TokenBucket {
    tokens: f64,
    last: Instant,
}

impl TokenBucket {
    fn new() -> Self {
        Self {
            tokens: LOGIN_BURST,
            last: Instant::now(),
        }
    }

    fn refresh_is_full(&mut self, now: Instant) -> bool {
        self.tokens = (self.tokens
            + now.duration_since(self.last).as_secs_f64() / LOGIN_REFILL.as_secs_f64())
        .min(LOGIN_BURST);
        self.last = now;
        self.tokens == LOGIN_BURST
    }

    fn allow(&mut self, now: Instant) -> bool {
        self.refresh_is_full(now);
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn successful_verification_identifies_current_policy_rehash() {
        let auth = AuthService::new().await.unwrap();
        let password = "correct horse battery staple";
        let current = auth.hash(password).await.unwrap();
        let second_current = auth.hash(password).await.unwrap();
        assert!(
            current != second_current,
            "fresh salts must change the PHC string"
        );
        let current_verification = auth.verify(password, current).await.unwrap();
        assert!(current_verification.verified);
        assert!(!current_verification.needs_rehash);

        let composed = auth.hash("caf\u{e9} password phrase").await.unwrap();
        let equivalent = auth
            .verify("cafe\u{301} password phrase", composed)
            .await
            .unwrap();
        assert!(equivalent.verified);

        let old_params = Params::new(32_768, 2, 1, Some(16)).unwrap();
        let old_argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, old_params);
        let salt = SaltString::encode_b64(&[7_u8; 16]).unwrap();
        let old_phc = old_argon
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        let old_verification = auth.verify(password, old_phc).await.unwrap();
        assert!(old_verification.verified);
        assert!(old_verification.needs_rehash);
    }

    #[test]
    fn enrollment_normalizes_and_rejects_exact_context_or_blocklist_matches() {
        let username = wire::Username::new("test_account").unwrap();
        let display_name = wire::DisplayName::new("Visible Adventurer").unwrap();
        let allowed = wire::Password::new("aaaaaaaaaaaaaaa").unwrap();
        assert!(validate_enrollment(&username, &display_name, &allowed, &BTreeSet::new()).is_ok());

        let blocked_text = "cafe\u{301} password phrase";
        let blocklist = BTreeSet::from([normalize("caf\u{e9} password phrase")]);
        let blocked = wire::Password::new(blocked_text).unwrap();
        assert!(validate_enrollment(&username, &display_name, &blocked, &blocklist).is_err());
        let display_match = wire::Password::new(display_name.as_str()).unwrap();
        assert!(
            validate_enrollment(&username, &display_name, &display_match, &BTreeSet::new())
                .is_err()
        );
    }

    #[test]
    fn enrollment_rejects_the_service_words_at_any_case_and_position() {
        let username = wire::Username::new("test_account").unwrap();
        let display_name = wire::DisplayName::new("Visible Adventurer").unwrap();

        // Both words, upper/lower/mixed, embedded in an otherwise long password.
        // The length floor is fifteen scalars, so every case here is one the
        // old equality check silently let through.
        for candidate in [
            "aaaaaaaaaamortal",
            "aaaaaaaaaaMORTAL",
            "aaaaaaaaaaMoRtAl",
            "mortalaaaaaaaaaa",
            "aaaaamortalaaaaa",
            "aaaaaaaaaaestate",
            "aaaaaaaaaaESTATE",
            "aaaaaaaaaaEsTaTe",
            "estateaaaaaaaaaa",
            "aaaaathemortalestateaaaaa",
        ] {
            let password = wire::Password::new(candidate).unwrap();
            assert!(
                validate_enrollment(&username, &display_name, &password, &BTreeSet::new()).is_err(),
                "accepted a password containing a service word: {candidate}"
            );
        }

        // A password that merely resembles them stays allowed.
        for candidate in ["aaaaaaaaaamortars", "aaaaaaaaaaestates"] {
            let password = wire::Password::new(candidate).unwrap();
            let verdict =
                validate_enrollment(&username, &display_name, &password, &BTreeSet::new());
            assert_eq!(
                verdict.is_err(),
                candidate.contains("estate"),
                "wrong verdict for {candidate}"
            );
        }
    }

    #[test]
    fn enrollment_rejects_a_password_containing_a_long_enough_username() {
        // Containment, not equality: the username sits inside a longer password.
        let username = wire::Username::new("test_account").unwrap();
        let display_name = wire::DisplayName::new("Visible Adventurer").unwrap();
        assert!(username.as_str().chars().count() >= USERNAME_CONTAINMENT_FLOOR);
        for candidate in [
            "carry test_account home",
            "CARRY TEST_ACCOUNT HOME",
            "test_account rides out",
            "the road to test_account",
        ] {
            assert!(
                candidate.chars().count() >= 15,
                "{candidate:?} must clear the length gate for this test to mean anything"
            );
            let password = wire::Password::new(candidate).unwrap();
            assert!(
                validate_enrollment(&username, &display_name, &password, &BTreeSet::new()).is_err(),
                "accepted a password containing the username: {candidate}"
            );
        }
    }

    #[test]
    fn a_username_below_the_floor_does_not_poison_passwords_containing_it() {
        // Usernames may be three scalars. Rejecting every password containing an
        // arbitrary trigram would refuse enormous numbers of legitimate
        // passwords, so containment starts at USERNAME_CONTAINMENT_FLOOR.
        let display_name = wire::DisplayName::new("Visible Adventurer").unwrap();
        for name in ["abc", "ab1", "a_b2"] {
            let username = wire::Username::new(name).unwrap();
            assert!(username.as_str().chars().count() < USERNAME_CONTAINMENT_FLOOR);
            let candidate = format!("{name}defghijklmnopqr");
            assert!(candidate.chars().count() >= 15);
            let password = wire::Password::new(candidate.clone()).unwrap();
            assert!(
                validate_enrollment(&username, &display_name, &password, &BTreeSet::new()).is_ok(),
                "a {}-scalar username blocked a legitimate password: {candidate}",
                name.chars().count()
            );
        }

        // At the floor exactly, containment is live again.
        let username = wire::Username::new("abcde").unwrap();
        assert_eq!(
            username.as_str().chars().count(),
            USERNAME_CONTAINMENT_FLOOR
        );
        let password = wire::Password::new("abcdefghijklmnopqr").unwrap();
        assert!(
            validate_enrollment(&username, &display_name, &password, &BTreeSet::new()).is_err(),
            "a username exactly at the floor was not enforced"
        );
    }

    #[test]
    fn enrollment_rejects_a_long_username_or_display_name_at_any_case() {
        let username = wire::Username::new("averylongusername_here").unwrap();
        let display_name = wire::DisplayName::new("A Very Long Display Name").unwrap();
        for candidate in [
            username.as_str().to_uppercase(),
            display_name.as_str().to_lowercase(),
        ] {
            let password = wire::Password::new(candidate.clone()).unwrap();
            assert!(
                validate_enrollment(&username, &display_name, &password, &BTreeSet::new()).is_err(),
                "accepted a cased context match: {candidate}"
            );
        }
    }

    #[test]
    fn source_failure_bucket_store_is_bounded() {
        let mut limits = LoginLimits::default();
        for index in 0..MAX_LOGIN_SOURCE_BUCKETS {
            let source = IpAddr::from([10, (index >> 16) as u8, (index >> 8) as u8, index as u8]);
            assert!(limits.allow_source(source));
        }
        assert_eq!(limits.sources.len(), MAX_LOGIN_SOURCE_BUCKETS);
        assert!(!limits.allow_source(IpAddr::from([192, 0, 2, 1])));
    }
}
