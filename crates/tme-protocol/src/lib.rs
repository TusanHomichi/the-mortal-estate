use std::collections::BTreeSet;
use std::fmt;

use serde::de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Number, Value};
use uuid::Uuid;

#[cfg(test)]
mod client_fixture_tests;

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 8;
pub const WEBSOCKET_SUBPROTOCOL: &str = "tme.v1";
pub const MAX_INPUT_BYTES: usize = 32 * 1024;
pub const MAX_JSON_NESTING: usize = 32;
pub const MAX_SUPPORTED_MINORS: usize = 8;
pub const MAX_MOVE_PATH_STEPS: usize = 3;
pub const MAX_SERVER_ENVELOPE_BYTES: usize = 256 * 1024;
pub const MAX_STATIC_SCENE_TILES: usize = 225;
pub const MAX_STATIC_SCENE_PROPS: usize = 128;
pub const MAX_STATIC_TRANSITION_APERTURES: usize = 64;
pub const MAX_STATIC_TERRAINS_PER_TILE: usize = 8;
pub const MAX_OBSERVER_TILES: usize = 225;
pub const MAX_OBSERVER_ACTORS: usize = 128;
pub const MAX_OBSERVER_CORPSES: usize = 64;
pub const MAX_OBSERVER_GROUND_ITEMS: usize = 128;
pub const MAX_OBSERVER_GOLD_PILES: usize = 64;
pub const MAX_OBSERVER_ACTION_OPTIONS: usize = 256;
pub const MAX_SOCIAL_BODY_SCALARS: usize = 280;
pub const MAX_SOCIAL_BODY_BYTES: usize = 1024;
pub const MAX_FEEDBACK_TEXT_SCALARS: usize = 280;
pub const MAX_FEEDBACK_TEXT_BYTES: usize = 1024;
pub const MAX_FEEDBACK_TRANSACTION_COSTS: usize = 64;
pub const MAX_FEEDBACK_TRANSACTION_REWARDS: usize = 64;
pub const MAX_MERCHANT_PURCHASE_ITEMS: usize = 128;
pub const MAX_OBSERVED_EVENTS: usize = 64;
pub const CONTROL_API_VERSION: u16 = 3;
pub const MAX_CONTROL_INPUT_BYTES: usize = 16 * 1024;
pub const MAX_CONTROL_JSON_NESTING: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolError(String);

impl ProtocolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ProtocolError {}

macro_rules! uuid_id {
    ($name:ident, $non_nil:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new(value: Uuid) -> Result<Self, ProtocolError> {
                if $non_nil && value.is_nil() {
                    return Err(ProtocolError::new(concat!(
                        stringify!($name),
                        " must be non-nil"
                    )));
                }
                Ok(Self(value))
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", self.0.hyphenated())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let text = String::deserialize(deserializer)?;
                let parsed = Uuid::parse_str(&text).map_err(serde::de::Error::custom)?;
                if parsed.hyphenated().to_string() != text {
                    return Err(serde::de::Error::custom(
                        "UUID must be canonical lowercase hyphenated text",
                    ));
                }
                Self::new(parsed).map_err(serde::de::Error::custom)
            }
        }
    };
}

uuid_id!(ConnectionId, false);
uuid_id!(FacetId, false);
uuid_id!(CommandId, true);
uuid_id!(PreviewId, true);
uuid_id!(MessageId, true);
uuid_id!(AccountId, true);
uuid_id!(SessionId, true);
uuid_id!(CharacterId, true);
uuid_id!(PlayerKillMarkId, true);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecimalU64(u64);

impl DecimalU64 {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for DecimalU64 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl Serialize for DecimalU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        if text.is_empty()
            || (text.len() > 1 && text.starts_with('0'))
            || !text.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(serde::de::Error::custom(
                "counter must be canonical decimal text",
            ));
        }
        text.parse::<u64>()
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecimalI64(i64);

impl DecimalI64 {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for DecimalI64 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl Serialize for DecimalI64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalI64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        let digits = text.strip_prefix('-').unwrap_or(&text);
        if digits.is_empty()
            || (digits.len() > 1 && digits.starts_with('0'))
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
            || text == "-0"
        {
            return Err(serde::de::Error::custom(
                "signed quantity must be canonical decimal text",
            ));
        }
        text.parse::<i64>()
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

fn is_canonical_sequence_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty()
            && !suffix.starts_with('0')
            && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

macro_rules! printable_ascii_string {
    ($name:ident, $max:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
                let value = value.into();
                if value.is_empty()
                    || value.len() > $max
                    || !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
                {
                    return Err(ProtocolError::new(concat!(
                        stringify!($name),
                        " must be bounded printable ASCII"
                    )));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

printable_ascii_string!(ActorId, 64);
printable_ascii_string!(WireLabel, 64);
printable_ascii_string!(ItemInstanceId, 128);
printable_ascii_string!(CorpseId, 128);
printable_ascii_string!(ActionId, 192);
printable_ascii_string!(ActionLabel, 256);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocialBody(String);

impl SocialBody {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        let scalars = value.chars().count();
        if !(1..=MAX_SOCIAL_BODY_SCALARS).contains(&scalars)
            || value.len() > MAX_SOCIAL_BODY_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(ProtocolError::new(
                "social body must contain 1-280 non-control scalars and at most 1024 bytes",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SocialBody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SocialBody([REDACTED])")
    }
}

impl Serialize for SocialBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SocialBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FeedbackText(String);

impl FeedbackText {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        let scalars = value.chars().count();
        if !(1..=MAX_FEEDBACK_TEXT_SCALARS).contains(&scalars)
            || value.len() > MAX_FEEDBACK_TEXT_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(ProtocolError::new(
                "feedback text must contain 1-280 non-control scalars and at most 1024 bytes",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for FeedbackText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FeedbackText([REDACTED])")
    }
}

impl Serialize for FeedbackText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for FeedbackText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Username(String);

impl Username {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        let valid = (3..=32).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && value
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric);
        if !valid {
            return Err(ProtocolError::new("username is not canonical"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Username {
    type Error = ProtocolError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DisplayName(String);

impl DisplayName {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        let count = value.chars().count();
        if !(1..=64).contains(&count) || value.chars().any(char::is_control) {
            return Err(ProtocolError::new(
                "display name must contain 1-64 non-control scalars",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DisplayName {
    type Error = ProtocolError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DisplayName> for String {
    fn from(value: DisplayName) -> Self {
        value.0
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Password(String);

impl Password {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        if value.len() > MAX_CONTROL_INPUT_BYTES {
            return Err(ProtocolError::new(
                "raw password exceeds the control-body bound",
            ));
        }
        Ok(Self(value))
    }

    pub fn expose_for_verification(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Password([REDACTED])")
    }
}

impl TryFrom<String> for Password {
    type Error = ProtocolError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Password> for String {
    fn from(value: Password) -> Self {
        value.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AdmissionTicket(String);

impl AdmissionTicket {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        if value.len() != 43
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            || value
                .as_bytes()
                .last()
                .and_then(|byte| base64url_value(*byte))
                .is_none_or(|index| index % 4 != 0)
        {
            return Err(ProtocolError::new(
                "ticket must be 43-character unpadded base64url text",
            ));
        }
        Ok(Self(value))
    }

    pub fn expose_for_admission(&self) -> &str {
        &self.0
    }
}

fn base64url_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

impl fmt::Debug for AdmissionTicket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AdmissionTicket([REDACTED])")
    }
}

impl Serialize for AdmissionTicket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AdmissionTicket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CsrfToken(String);

impl CsrfToken {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        AdmissionTicket::new(value.clone())?;
        Ok(Self(value))
    }

    pub fn expose_for_validation(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CsrfToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CsrfToken([REDACTED])")
    }
}

impl Serialize for CsrfToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CsrfToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

mod control;
pub use control::*;
mod observer;
pub use observer::*;
mod services;
pub use services::*;
mod feedback;
pub use feedback::*;
mod actions;
pub use actions::*;
mod envelopes;
pub use envelopes::*;

mod codec;
pub use codec::decode_document;
#[cfg(target_arch = "wasm32")]
mod wasm;
