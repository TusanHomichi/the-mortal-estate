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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequestV1 {
    pub username: Username,
    pub password: Password,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogoutRequestV1 {
    pub csrf_token: CsrfToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterSelectRequestV1 {
    pub csrf_token: CsrfToken,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocketTicketRequestV1 {
    pub csrf_token: CsrfToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgivePlayerKillMarkRequestV1 {
    pub request_id: CommandId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgivePlayerKillMarkResultV1 {
    pub control_api_version: u16,
    pub mark_id: PlayerKillMarkId,
    pub replay_status: ReplayStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountSummaryV1 {
    pub account_id: AccountId,
    pub display_name: DisplayName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSummaryV1 {
    pub session_id: SessionId,
    pub idle_timeout_seconds: DecimalU64,
    pub absolute_timeout_seconds: DecimalU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterSummaryV1 {
    pub character_id: CharacterId,
    pub slot: u8,
    pub display_name: DisplayName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionBootstrapV1 {
    pub control_api_version: u16,
    pub account: AccountSummaryV1,
    pub session: SessionSummaryV1,
    pub csrf_token: CsrfToken,
    pub characters: Vec<CharacterSummaryV1>,
    pub selected_character_id: Option<CharacterId>,
    pub player_kill_marks: PlayerKillMarkStateV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerKillMarkSummaryV1 {
    pub mark_id: PlayerKillMarkId,
    pub victim_character_id: CharacterId,
    pub victim_display_name: DisplayName,
    pub assessed_at: WireLabel,
    pub expires_at: Option<WireLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgivablePlayerKillMarkV1 {
    pub mark_id: PlayerKillMarkId,
    pub killer_character_id: CharacterId,
    pub killer_display_name: DisplayName,
    pub assessed_at: WireLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerKillMarkStateV1 {
    pub active_count: u32,
    pub gameplay_locked: bool,
    pub active_marks: Vec<PlayerKillMarkSummaryV1>,
    pub forgivable_marks: Vec<ForgivablePlayerKillMarkV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterSelectionV1 {
    pub control_api_version: u16,
    pub character: CharacterSummaryV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocketTicketV1 {
    pub ticket: AdmissionTicket,
    pub protocol_major: u16,
    pub supported_minors: Vec<u16>,
    pub expires_in_seconds: DecimalU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlErrorCode {
    MalformedRequest,
    InvalidCredentials,
    RateLimited,
    AuthenticationRequired,
    CsrfRejected,
    CharacterNotOwned,
    CharacterNotSelected,
    GameplayMarkLocked,
    ForgivenessUnavailable,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlErrorV1 {
    pub code: ControlErrorCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitTraversalKind {
    StairsUp,
    StairsDown,
    ClimbUp,
    ClimbDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationKind {
    Walk,
    Swim,
    Door,
    Stairs { direction: VerticalDirection },
    Pit,
    Climb { direction: VerticalDirection },
    Passage,
    Portal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Player,
    Monster,
    Npc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeState {
    Alive,
    Ghost,
    AwaitingResurrection,
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub realm: WireLabel,
    pub level: WireLabel,
    pub position: Coord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub navigation: NavigationKind,
    pub target: Position,
    pub door_open: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverTile {
    pub position: Coord,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_id: Option<WireLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_name: Option<WireLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_cost: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverActor {
    pub actor_id: ActorId,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub character_id: Option<CharacterId>,
    pub name: WireLabel,
    pub kind: ActorKind,
    pub position: Position,
    pub life_state: LifeState,
    pub hp: i32,
    pub max_hp: i32,
    pub attack_safety: AttackSafety,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackSafety {
    Invalid,
    Protected,
    OpenSelfDefense,
    OpenEvilPlayer,
    OpenHostile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverItemBinding {
    Unbound,
    Bound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverItem {
    pub item_instance_id: ItemInstanceId,
    pub item_definition_id: WireLabel,
    pub name: WireLabel,
    pub quantity: u32,
    pub binding: ObserverItemBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LootOwner {
    Character(CharacterId),
    TransientActor(ActorId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LootClaimBasis {
    KillingBlow,
    CharacterDeathPile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootClaim {
    pub owner: LootOwner,
    pub basis: LootClaimBasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverCorpse {
    pub corpse_id: CorpseId,
    pub origin_actor_id: ActorId,
    pub origin_kind: ActorKind,
    pub origin_name: WireLabel,
    pub location: Position,
    pub sequence: DecimalU64,
    pub searched: bool,
    pub loot_claim: Option<LootClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverGroundItem {
    #[serde(flatten)]
    pub item: ObserverItem,
    pub location: Position,
    pub loot_claim: Option<LootClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverGoldPile {
    pub gold_pile_id: WireLabel,
    pub amount: DecimalI64,
    pub location: Position,
    pub loot_claim: Option<LootClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarriedPosition {
    LeftHand,
    RightHand,
    #[serde(rename = "left_finger_1")]
    LeftFinger1,
    #[serde(rename = "left_finger_2")]
    LeftFinger2,
    #[serde(rename = "left_finger_3")]
    LeftFinger3,
    #[serde(rename = "left_finger_4")]
    LeftFinger4,
    #[serde(rename = "right_finger_1")]
    RightFinger1,
    #[serde(rename = "right_finger_2")]
    RightFinger2,
    #[serde(rename = "right_finger_3")]
    RightFinger3,
    #[serde(rename = "right_finger_4")]
    RightFinger4,
    #[serde(rename = "belt_1")]
    Belt1,
    #[serde(rename = "belt_2")]
    Belt2,
    #[serde(rename = "belt_3")]
    Belt3,
    #[serde(rename = "belt_4")]
    Belt4,
    BeltBack,
    #[serde(rename = "sack_item_1")]
    SackItem1,
    #[serde(rename = "sack_item_2")]
    SackItem2,
    #[serde(rename = "sack_item_3")]
    SackItem3,
    #[serde(rename = "sack_item_4")]
    SackItem4,
    #[serde(rename = "sack_item_5")]
    SackItem5,
    #[serde(rename = "sack_item_6")]
    SackItem6,
    #[serde(rename = "sack_item_7")]
    SackItem7,
    #[serde(rename = "sack_item_8")]
    SackItem8,
    #[serde(rename = "sack_item_9")]
    SackItem9,
    #[serde(rename = "sack_item_10")]
    SackItem10,
    #[serde(rename = "sack_item_11")]
    SackItem11,
    #[serde(rename = "sack_item_12")]
    SackItem12,
    #[serde(rename = "sack_item_13")]
    SackItem13,
    #[serde(rename = "sack_item_14")]
    SackItem14,
    #[serde(rename = "sack_item_15")]
    SackItem15,
    #[serde(rename = "sack_item_16")]
    SackItem16,
    #[serde(rename = "sack_item_17")]
    SackItem17,
    #[serde(rename = "sack_item_18")]
    SackItem18,
    #[serde(rename = "sack_item_19")]
    SackItem19,
    #[serde(rename = "sack_item_20")]
    SackItem20,
    Head,
    Neck,
    LeftArm,
    RightArm,
    Gloves,
    InnerArmor,
    OuterArmor,
    Boots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarriedGoldPosition {
    LeftHand,
    RightHand,
    Sack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMember {
    pub character_id: CharacterId,
    pub joined_order: DecimalU64,
    pub membership_epoch: DecimalU64,
    pub connected: bool,
    pub absent_since: Option<DecimalU64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupView {
    pub group_id: DecimalU64,
    pub leader_character_id: CharacterId,
    pub members: Vec<GroupMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupInvitation {
    pub invitation_id: DecimalU64,
    pub issuer_character_id: CharacterId,
    pub target_character_id: CharacterId,
    pub group_id: Option<DecimalU64>,
    pub expires_at: DecimalU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialView {
    pub character_id: CharacterId,
    pub group: Option<GroupView>,
    pub incoming_invitations: Vec<GroupInvitation>,
    pub outgoing_invitations: Vec<GroupInvitation>,
    pub following_character_id: Option<CharacterId>,
    pub pages_enabled: bool,
    pub blocked_character_ids: Vec<CharacterId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemOffer {
    pub item: OwnedItem,
    pub sender_character_id: CharacterId,
    pub recipient_character_id: CharacterId,
    pub source_position: CarriedPosition,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterAlignment {
    Lawful,
    Neutral,
    Chaotic,
    Evil,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterIdentity {
    pub base_class_id: WireLabel,
    pub current_class_id: WireLabel,
    pub display_class: WireLabel,
    pub nationality_id: WireLabel,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub sex_or_gender_display: Option<WireLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterAttributes {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterResources {
    pub hp: i32,
    pub max_hp: i32,
    pub peak_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub stamina: i32,
    pub max_stamina: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterProgression {
    pub level: i32,
    pub experience: DecimalI64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub pending_target_level: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalAttributeAdds {
    pub strength_adds: i32,
    pub dexterity_adds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionEntry {
    pub from_class_id: WireLabel,
    pub to_class_id: WireLabel,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownSpell {
    pub spell_id: WireLabel,
    pub lane: WireLabel,
    pub learned_at_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillEntry {
    pub track_id: WireLabel,
    pub level: u8,
    pub critique_rank: u8,
    pub practice_points: DecimalU64,
    pub learning_rate: DecimalU64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub track_display: Option<WireLabel>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub level_title: Option<WireLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlledCharacter {
    pub identity: CharacterIdentity,
    pub alignment: CharacterAlignment,
    pub karma_points: u32,
    pub attributes: CharacterAttributes,
    pub resources: CharacterResources,
    pub progression: CharacterProgression,
    pub physical_attribute_adds: PhysicalAttributeAdds,
    pub promotion_history: Vec<PromotionEntry>,
    pub known_spells: Vec<KnownSpell>,
    pub skill_ledger: Vec<SkillEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnedItemBinding {
    Unrestricted,
    BindOnFirstCharacterTouch,
    Bound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowReadiness {
    Unnocked,
    Nocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemPlacementKind {
    Hand,
    RingFinger,
    BeltSide,
    BeltBack,
    Sack,
    Head,
    Neck,
    Arm,
    Gloves,
    InnerArmor,
    OuterArmor,
    Boots,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedItem {
    pub item_instance_id: ItemInstanceId,
    pub item_definition_id: WireLabel,
    pub name: WireLabel,
    pub quantity: u32,
    pub identified: bool,
    pub appraised: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub known_unit_value_gold: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub known_stack_value_gold: Option<DecimalU64>,
    pub unit_burden: DecimalU64,
    pub stack_burden: DecimalU64,
    pub binding: OwnedItemBinding,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub bow_readiness: Option<BowReadiness>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionedItem {
    pub position: CarriedPosition,
    pub item: OwnedItem,
    pub valid_placements: Vec<ItemPlacementKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriedGold {
    pub left_hand: DecimalI64,
    pub right_hand: DecimalI64,
    pub sack: DecimalI64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriedLayout {
    pub items: Vec<PositionedItem>,
    pub gold: CarriedGold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarmedSpellStatus {
    Warming,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarmedSpell {
    pub spell_id: WireLabel,
    pub warmed_at: DecimalU64,
    pub ready_at: DecimalU64,
    pub status: WarmedSpellStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCastingMethod {
    Direct,
    WarmThenCast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCastClass {
    Character,
    Path,
    PathOrCharacter,
    #[serde(rename = "self")]
    SelfTarget,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellTargetKind {
    Actor,
    Area,
    Coordinate,
    Direction,
    Door,
    Item,
    None,
    #[serde(rename = "self")]
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellActionState {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub blocked_reason: Option<WireLabel>,
    pub requires_target_selection: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub intent: Option<Intent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellAction {
    pub spell_id: WireLabel,
    pub spell_name: WireLabel,
    pub casting_method: SpellCastingMethod,
    pub cast_class: SpellCastClass,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub target_kind: Option<SpellTargetKind>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub mp_cost: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_cost: Option<i32>,
    pub hostile_act: bool,
    pub town_law_violation: bool,
    pub warm: SpellActionState,
    pub cast: SpellActionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionRequirement {
    CurrentClass {
        class_id: WireLabel,
    },
    MinimumLevel {
        level: i32,
    },
    ExactKarma {
        karma_points: u32,
    },
    ExactAlignment {
        alignment: CharacterAlignment,
    },
    MinimumSkillLevel {
        track_id: WireLabel,
        level: u8,
    },
    MinimumCarriedGold {
        amount: DecimalI64,
    },
    CarriedItem {
        item_definition_id: WireLabel,
        quantity: u32,
    },
    CarriedPositionEmpty {
        position: CarriedPosition,
    },
    SpellUnknown {
        spell_id: WireLabel,
    },
    QuestUnstarted {
        quest_id: WireLabel,
    },
    QuestAtStage {
        quest_id: WireLabel,
        stage_id: WireLabel,
    },
    NpcAccompanying {
        npc_actor_id: ActorId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionCost {
    CarriedGold { amount: DecimalI64 },
    SelectedCarriedItem { quantity: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionReward {
    Experience {
        amount: i32,
    },
    Item {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        position: CarriedPosition,
    },
    Class {
        to_class_id: WireLabel,
        to_class_display: WireLabel,
    },
    Spell {
        spell_id: WireLabel,
    },
    QuestStage {
        quest_id: WireLabel,
        stage_id: WireLabel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceTransaction {
    pub transaction_id: WireLabel,
    pub label: WireLabel,
    pub requirements: Vec<TransactionRequirement>,
    pub costs: Vec<TransactionCost>,
    pub rewards: Vec<TransactionReward>,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantListingOrigin {
    AuthoredStock,
    PawnPool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantListing {
    pub item: OwnedItem,
    pub origin: MerchantListingOrigin,
    pub price_gold: DecimalI64,
    pub purchase: ObserverActionOption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemServiceOperationKind {
    Appraise,
    Identify,
    EnchantWeapon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemServiceOperation {
    pub operation: ItemServiceOperationKind,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Hp,
    Mp,
    Stamina,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorationStatusKind {
    Blindness,
    Poison,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RestorationOutcome {
    RestoreResource { resource: ResourceKind },
    CureStatus { status: RestorationStatusKind },
    PriestResurrection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorationOperation {
    pub operation_id: WireLabel,
    pub label: WireLabel,
    pub requirements: Vec<TransactionRequirement>,
    pub costs: Vec<TransactionCost>,
    pub outcome: RestorationOutcome,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServiceCapability {
    SkillTraining {
        capability_id: WireLabel,
        offered_track_ids: Vec<WireLabel>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        selected_track_id: Option<WireLabel>,
        actions: Vec<ObserverActionOption>,
    },
    SkillCritique {
        capability_id: WireLabel,
        actions: Vec<ObserverActionOption>,
    },
    SpellTeaching {
        capability_id: WireLabel,
        spell_ids: Vec<WireLabel>,
        actions: Vec<ObserverActionOption>,
    },
    ClassPromotion {
        capability_id: WireLabel,
        target_class_id: WireLabel,
        actions: Vec<ObserverActionOption>,
    },
    ServiceTransaction {
        capability_id: WireLabel,
        transactions: Vec<ServiceTransaction>,
    },
    Merchant {
        capability_id: WireLabel,
        listings: Vec<MerchantListing>,
        buy_all: ObserverActionOption,
        sales: Vec<ObserverActionOption>,
    },
    ItemService {
        capability_id: WireLabel,
        operations: Vec<ItemServiceOperation>,
    },
    Restoration {
        capability_id: WireLabel,
        operations: Vec<RestorationOperation>,
    },
    Bank {
        capability_id: WireLabel,
        bank_id: WireLabel,
        balance_gold: DecimalI64,
        transaction_cap_gold: DecimalI64,
        deposit_actions: Vec<ObserverActionOption>,
        withdrawal_actions: Vec<ObserverActionOption>,
    },
    Locker {
        capability_id: WireLabel,
        vault_id: WireLabel,
        capacity: u32,
        item_count: u32,
        items: Vec<OwnedItem>,
        deposit_actions: Vec<ObserverActionOption>,
        withdrawal_actions: Vec<ObserverActionOption>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub service_id: WireLabel,
    pub name: WireLabel,
    pub position: Position,
    pub capabilities: Vec<ServiceCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcInteractionOutcome {
    Speak,
    BeginFollow,
    EndFollow,
    CompleteEscort { npc_actor_id: ActorId },
    Climb { direction: VerticalDirection },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcInteraction {
    pub interaction_id: WireLabel,
    pub label: WireLabel,
    pub requirements: Vec<TransactionRequirement>,
    pub costs: Vec<TransactionCost>,
    pub rewards: Vec<TransactionReward>,
    pub outcome: NpcInteractionOutcome,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub actor_id: ActorId,
    pub name: WireLabel,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub following_character_id: Option<CharacterId>,
    pub interactions: Vec<NpcInteraction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestState {
    pub quest_id: WireLabel,
    pub quest_title: WireLabel,
    pub stage_id: WireLabel,
    pub stage_label: WireLabel,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverFrame {
    pub contract_version: u32,
    pub logical_time: DecimalU64,
    pub ready_at: DecimalU64,
    pub observer_actor_id: ActorId,
    pub observation_center: Position,
    pub observation_radius: u32,
    pub can_act: bool,
    pub tiles: Vec<ObserverTile>,
    pub actors: Vec<ObserverActor>,
    pub corpses: Vec<ObserverCorpse>,
    pub corpses_truncated: bool,
    pub ground_items: Vec<ObserverGroundItem>,
    pub ground_items_truncated: bool,
    pub gold_piles: Vec<ObserverGoldPile>,
    pub gold_piles_truncated: bool,
    pub character: ControlledCharacter,
    pub carried: CarriedLayout,
    pub burden: Burden,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub warmed_spell: Option<WarmedSpell>,
    pub spell_actions: Vec<SpellAction>,
    pub services_here: Vec<Service>,
    pub npcs_here: Vec<Npc>,
    pub quest_log: Vec<QuestState>,
    pub action_options: Vec<ObserverActionOption>,
    pub action_options_truncated: bool,
    pub social: SocialView,
    pub incoming_item_offers: Vec<ItemOffer>,
    pub outgoing_item_offers: Vec<ItemOffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticSceneRole {
    Overworld,
    CombatSpace,
    Interior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    OverworldTown,
    CombatSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneSite {
    pub realm: WireLabel,
    pub level: WireLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneBounds {
    pub min: Coord,
    pub max: Coord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneTile {
    pub position: Coord,
    pub terrain_ids: Vec<WireLabel>,
    pub walkable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneProp {
    pub id: WireLabel,
    pub visual_family: WireLabel,
    pub anchor: Coord,
    pub layer: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticTransitionAperture {
    pub at: Coord,
    pub navigation: NavigationKind,
    pub target: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneContext {
    pub contract_version: u32,
    pub site: StaticSceneSite,
    pub bounds: StaticSceneBounds,
    pub content_digest: WireLabel,
    pub visual_manifest_digest: WireLabel,
    pub scene_role: StaticSceneRole,
    pub presentation_mode: PresentationMode,
    pub world_zoom: [u32; 2],
    pub tiles: Vec<StaticSceneTile>,
    pub walkable_mask: Vec<Coord>,
    pub static_props: Vec<StaticSceneProp>,
    pub transition_apertures: Vec<StaticTransitionAperture>,
}

impl StaticSceneContext {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.contract_version != 1 {
            return Err(ProtocolError::new(
                "static scene context contract version is not current",
            ));
        }
        if self.bounds.min.x > self.bounds.max.x || self.bounds.min.y > self.bounds.max.y {
            return Err(ProtocolError::new(
                "static scene context bounds are inverted",
            ));
        }
        let width = i64::from(self.bounds.max.x) - i64::from(self.bounds.min.x) + 1;
        let height = i64::from(self.bounds.max.y) - i64::from(self.bounds.min.y) + 1;
        let area = width
            .checked_mul(height)
            .ok_or_else(|| ProtocolError::new("static scene context area overflows"))?;
        if area <= 0
            || usize::try_from(area).ok() != Some(self.tiles.len())
            || self.tiles.len() > MAX_STATIC_SCENE_TILES
        {
            return Err(ProtocolError::new(
                "static scene context tile rectangle is invalid or exceeds its bound",
            ));
        }
        if self.walkable_mask.len() > self.tiles.len()
            || self.static_props.len() > MAX_STATIC_SCENE_PROPS
            || self.transition_apertures.len() > MAX_STATIC_TRANSITION_APERTURES
        {
            return Err(ProtocolError::new(
                "static scene context vector exceeds its bound",
            ));
        }
        if self.world_zoom[0] == 0 || self.world_zoom[1] == 0 {
            return Err(ProtocolError::new(
                "static scene context world zoom must be positive",
            ));
        }
        if !matches!(
            (self.scene_role, self.presentation_mode),
            (StaticSceneRole::Overworld, PresentationMode::OverworldTown)
                | (StaticSceneRole::CombatSpace, PresentationMode::CombatSpace)
                | (StaticSceneRole::Interior, _)
        ) {
            return Err(ProtocolError::new(
                "static scene role and presentation mode disagree",
            ));
        }
        for digest in [&self.content_digest, &self.visual_manifest_digest] {
            if digest.as_str().len() != 64
                || !digest
                    .as_str()
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(ProtocolError::new(
                    "static scene context digest must be lowercase SHA-256",
                ));
            }
        }
        let expected_positions = self
            .tiles
            .iter()
            .map(|tile| (tile.position.x, tile.position.y))
            .collect::<std::collections::BTreeSet<_>>();
        let position_in_bounds = |position: &Coord| {
            position.x >= self.bounds.min.x
                && position.x <= self.bounds.max.x
                && position.y >= self.bounds.min.y
                && position.y <= self.bounds.max.y
        };
        if expected_positions.len() != self.tiles.len()
            || self.tiles.iter().any(|tile| {
                !position_in_bounds(&tile.position)
                    || tile.terrain_ids.is_empty()
                    || tile.terrain_ids.len() > MAX_STATIC_TERRAINS_PER_TILE
            })
            || self.walkable_mask.iter().any(|position| {
                !expected_positions.contains(&(position.x, position.y))
                    || !self
                        .tiles
                        .iter()
                        .any(|tile| tile.position == *position && tile.walkable)
            })
            || self
                .tiles
                .iter()
                .any(|tile| tile.walkable != self.walkable_mask.contains(&tile.position))
        {
            return Err(ProtocolError::new(
                "static scene context walkable mask differs from tile walkability",
            ));
        }
        let mut prop_ids = std::collections::BTreeSet::new();
        if self
            .static_props
            .iter()
            .any(|prop| !position_in_bounds(&prop.anchor) || !prop_ids.insert(prop.id.as_str()))
            || self
                .transition_apertures
                .iter()
                .any(|aperture| !position_in_bounds(&aperture.at))
        {
            return Err(ProtocolError::new(
                "static scene context prop or aperture is invalid",
            ));
        }
        Ok(())
    }
}

impl ObserverFrame {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.contract_version != 7 {
            return Err(ProtocolError::new(
                "observer frame contract version is not current",
            ));
        }
        if self.observation_radius != 7
            || self.tiles.len() > MAX_OBSERVER_TILES
            || self.actors.len() > MAX_OBSERVER_ACTORS
            || self.corpses.len() > MAX_OBSERVER_CORPSES
            || self.ground_items.len() > MAX_OBSERVER_GROUND_ITEMS
            || self.gold_piles.len() > MAX_OBSERVER_GOLD_PILES
            || self.action_options.len() > MAX_OBSERVER_ACTION_OPTIONS
        {
            return Err(ProtocolError::new(
                "observer frame exceeds R7 storage bounds",
            ));
        }
        if self.gold_piles.iter().any(|pile| pile.amount.get() < 0) {
            return Err(ProtocolError::new(
                "observer gold pile amount must be non-negative",
            ));
        }
        if self.carried.gold.left_hand.get() < 0
            || self.carried.gold.right_hand.get() < 0
            || self.carried.gold.sack.get() < 0
        {
            return Err(ProtocolError::new(
                "observer carried gold must be non-negative",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverActionOption {
    pub id: ActionId,
    pub label: ActionLabel,
    pub enabled: bool,
    pub blocked_reason: Option<WireLabel>,
    pub intent: Option<Intent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverInspectExitStatus {
    Walkable,
    BlockedTerrain,
    Door { open: bool, target: Position },
    OutOfBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverInspectExit {
    pub direction: Direction,
    pub location: Position,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub terrain: Option<WireLabel>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub move_cost: Option<i32>,
    pub status: ObserverInspectExitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverInspectActor {
    pub direction: Direction,
    pub actor_id: ActorId,
    pub actor: WireLabel,
    pub kind: ActorKind,
    pub location: Position,
    pub hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverInspectGroundItem {
    #[serde(flatten)]
    pub item: ObserverItem,
    pub location: Position,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub direction: Option<Direction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackActor {
    pub actor_id: ActorId,
    pub name: WireLabel,
    pub kind: ActorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackWoundState {
    Unhurt,
    Wounded,
    BadlyWounded,
    NearDeath,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackWeaponFumbleResult {
    Dropped,
    BowUnnocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackDeathCause {
    Physical,
    Poison,
    Fire,
    OtherMagic,
    Hazard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackResurrectionMethod {
    Gods,
    Priest,
    Thaumaturge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackPhysicalOutcome {
    Hit {
        damage: i32,
        armor_reduction: i32,
        wound_before: FeedbackWoundState,
        wound_after: FeedbackWoundState,
        target_hp: i32,
    },
    Missed {},
    Blocked {},
    NoSight {},
    NotReady {
        current_time: DecimalU64,
        ready_at: DecimalU64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSpellFizzleReason {
    Replaced,
    Canceled,
    Rest,
    HealingBalm,
    Damage,
    Defeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSpellFailureReason {
    InvalidPath,
    AboveSkillAttempt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackSpellLifecycleState {
    Warmed {
        warmed_at: DecimalU64,
        ready_at: DecimalU64,
    },
    Ready {
        ready_at: DecimalU64,
    },
    Cast {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        mp_cost: Option<i32>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        stamina_cost: Option<i32>,
    },
    Fizzled {
        reason: FeedbackSpellFizzleReason,
    },
    Failed {
        reason: FeedbackSpellFailureReason,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        mp_cost: Option<i32>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        stamina_cost: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackSpellImpactOutcome {
    Damaged { damage: i32, target_hp: i32 },
    Healed { amount: i32, target_hp: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackEffectChange {
    Applied {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        remaining_rounds: Option<u32>,
    },
    Ticked {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        remaining_rounds: Option<u32>,
    },
    Expired {},
    Removed {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackResourceReason {
    MovementSpend,
    PhysicalSpend,
    SpellCost,
    Regenerated,
    Restored,
    Balm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackTransactionSource {
    SkillTraining {
        service_id: WireLabel,
        capability_id: WireLabel,
        track_id: WireLabel,
    },
    SpellLearning {
        service_id: WireLabel,
        capability_id: WireLabel,
        spell_id: WireLabel,
    },
    ClassPromotion {
        service_id: WireLabel,
        capability_id: WireLabel,
        transaction_id: WireLabel,
        target_class_id: WireLabel,
    },
    ServiceTransaction {
        service_id: WireLabel,
        capability_id: WireLabel,
        transaction_id: WireLabel,
    },
    MerchantPurchase {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_ids: Vec<ItemInstanceId>,
    },
    MerchantSale {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
    },
    ItemService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation: ItemServiceOperationKind,
        item_instance_id: ItemInstanceId,
    },
    RestorationService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        corpse_id: Option<CorpseId>,
    },
    NpcInteraction {
        npc_actor_id: ActorId,
        interaction_id: WireLabel,
    },
    BankDeposit {
        service_id: WireLabel,
        capability_id: WireLabel,
        bank_id: WireLabel,
        gold_pile_id: WireLabel,
    },
    BankWithdrawal {
        service_id: WireLabel,
        capability_id: WireLabel,
        bank_id: WireLabel,
        amount: DecimalI64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackTransactionCost {
    CarriedGold {
        amount: DecimalI64,
        position: CarriedGoldPosition,
        before: DecimalI64,
        after: DecimalI64,
    },
    GroundGoldPile {
        gold_pile_id: WireLabel,
        amount: DecimalI64,
    },
    BankBalance {
        bank_id: WireLabel,
        amount: DecimalI64,
        before: DecimalI64,
        after: DecimalI64,
    },
    SelectedCarriedItem {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        consumed_quantity: u32,
        remaining_quantity: u32,
    },
    MerchantItem {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        quantity: u32,
        pawn_listing_price_gold: DecimalI64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackTransactionReward {
    LearningRate {
        track_id: WireLabel,
        before: DecimalU64,
        after: DecimalU64,
    },
    Experience {
        amount: i32,
        total_xp: DecimalI64,
    },
    Item {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        position: CarriedPosition,
        quantity: u32,
    },
    Class {
        from_class_id: WireLabel,
        from_class_display: WireLabel,
        to_class_id: WireLabel,
        to_class_display: WireLabel,
    },
    Spell {
        spell_id: WireLabel,
        learned_at_level: i32,
    },
    CarriedGold {
        amount: DecimalI64,
        position: CarriedGoldPosition,
        before: DecimalI64,
        after: DecimalI64,
    },
    BankBalance {
        bank_id: WireLabel,
        amount: DecimalI64,
        before: DecimalI64,
        after: DecimalI64,
    },
    GroundGoldPile {
        gold_pile_id: WireLabel,
        amount: DecimalI64,
    },
    MerchantItem {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        quantity: u32,
        listing_price_gold: DecimalI64,
    },
    ItemAppraised {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        unit_value_gold: DecimalU64,
        total_value_gold: DecimalU64,
    },
    ItemIdentified {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
    },
    ItemEnchanted {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        enchantment_instance_id: WireLabel,
        combat_add_rating_bonus: i32,
        tags: Vec<WireLabel>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        remaining_rounds: Option<u32>,
    },
    ResourceRestored {
        resource: ResourceKind,
        before: i32,
        after: i32,
        maximum: i32,
    },
    StatusCured {
        status: RestorationStatusKind,
        removed_count: u32,
    },
    PriestResurrection {
        corpse_id: CorpseId,
        method: FeedbackResurrectionMethod,
        current_hp: i32,
        current_stamina: i32,
    },
    NpcInteraction {
        npc_actor_id: ActorId,
        interaction_id: WireLabel,
        outcome: NpcInteractionOutcome,
    },
    QuestStage {
        quest_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        before_stage_id: Option<WireLabel>,
        after_stage_id: WireLabel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackCorpseChange {
    Created {},
    Removed { method: FeedbackResurrectionMethod },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackCue {
    PhysicalCombat {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        source: Option<FeedbackActor>,
        target: FeedbackActor,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        location: Option<Position>,
        mode: PhysicalAttackMode,
        outcome: FeedbackPhysicalOutcome,
    },
    WeaponFumbled {
        actor: FeedbackActor,
        mode: PhysicalAttackMode,
        result: FeedbackWeaponFumbleResult,
    },
    SpellLifecycle {
        actor: FeedbackActor,
        spell_id: WireLabel,
        spell_name: WireLabel,
        state: FeedbackSpellLifecycleState,
    },
    SpellImpact {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        source: Option<FeedbackActor>,
        spell_id: WireLabel,
        spell_name: WireLabel,
        target: FeedbackActor,
        location: Position,
        outcome: FeedbackSpellImpactOutcome,
    },
    ActorEffect {
        actor: FeedbackActor,
        location: Position,
        effect_id: WireLabel,
        effect_kind: WireLabel,
        change: FeedbackEffectChange,
    },
    TileEffect {
        location: Position,
        effect_id: WireLabel,
        effect_kind: WireLabel,
        change: FeedbackEffectChange,
    },
    EffectDamage {
        actor: FeedbackActor,
        location: Position,
        effect_id: WireLabel,
        effect_kind: WireLabel,
        damage: i32,
        actor_hp: i32,
    },
    Resource {
        actor: FeedbackActor,
        resource: ResourceKind,
        reason: FeedbackResourceReason,
        amount: i32,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        current: Option<i32>,
        maximum: i32,
    },
    Transaction {
        actor: FeedbackActor,
        source: FeedbackTransactionSource,
        costs: Vec<FeedbackTransactionCost>,
        rewards: Vec<FeedbackTransactionReward>,
    },
    Quest {
        quest_id: WireLabel,
        quest_title: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        before_stage_id: Option<WireLabel>,
        after_stage_id: WireLabel,
        after_stage_label: WireLabel,
        terminal: bool,
    },
    NpcMessage {
        npc_actor_id: ActorId,
        npc_name: WireLabel,
        interaction_id: WireLabel,
        response: FeedbackText,
    },
    Defeat {
        actor: FeedbackActor,
        location: Position,
        cause: FeedbackDeathCause,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        credited_source: Option<FeedbackActor>,
    },
    Corpse {
        corpse_id: CorpseId,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        origin: Option<FeedbackActor>,
        location: Position,
        change: FeedbackCorpseChange,
    },
    LifeState {
        actor: FeedbackActor,
        from: LifeState,
        to: LifeState,
    },
    Resurrection {
        actor: FeedbackActor,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        corpse_id: Option<CorpseId>,
        method: FeedbackResurrectionMethod,
        destination: Position,
        current_hp: i32,
        current_stamina: i32,
    },
}

impl FeedbackCue {
    fn validate(&self) -> Result<(), ProtocolError> {
        if let Self::Transaction {
            costs,
            rewards,
            source,
            ..
        } = self
        {
            if costs.len() > MAX_FEEDBACK_TRANSACTION_COSTS
                || rewards.len() > MAX_FEEDBACK_TRANSACTION_REWARDS
            {
                return Err(ProtocolError::new(
                    "feedback transaction exceeds receipt bound",
                ));
            }
            if let FeedbackTransactionSource::MerchantPurchase {
                item_instance_ids, ..
            } = source
                && item_instance_ids.len() > MAX_MERCHANT_PURCHASE_ITEMS
            {
                return Err(ProtocolError::new(
                    "feedback merchant purchase exceeds item bound",
                ));
            }
        }
        let valid_gold_pile_id =
            |value: &WireLabel| is_canonical_sequence_id(value.as_str(), "gold:");
        let valid_corpse_id =
            |value: &CorpseId| is_canonical_sequence_id(value.as_str(), "corpse:");
        let sequence_ids_are_valid = match self {
            Self::Transaction {
                source,
                costs,
                rewards,
                ..
            } => {
                let source_is_valid = match source {
                    FeedbackTransactionSource::RestorationService { corpse_id, .. } => {
                        corpse_id.as_ref().is_none_or(valid_corpse_id)
                    }
                    FeedbackTransactionSource::BankDeposit { gold_pile_id, .. } => {
                        valid_gold_pile_id(gold_pile_id)
                    }
                    _ => true,
                };
                let costs_are_valid = costs.iter().all(|cost| match cost {
                    FeedbackTransactionCost::GroundGoldPile { gold_pile_id, .. } => {
                        valid_gold_pile_id(gold_pile_id)
                    }
                    _ => true,
                });
                let rewards_are_valid = rewards.iter().all(|reward| match reward {
                    FeedbackTransactionReward::GroundGoldPile { gold_pile_id, .. } => {
                        valid_gold_pile_id(gold_pile_id)
                    }
                    FeedbackTransactionReward::PriestResurrection { corpse_id, .. } => {
                        valid_corpse_id(corpse_id)
                    }
                    _ => true,
                });
                source_is_valid && costs_are_valid && rewards_are_valid
            }
            Self::Corpse { corpse_id, .. } => valid_corpse_id(corpse_id),
            Self::Resurrection { corpse_id, .. } => corpse_id.as_ref().is_none_or(valid_corpse_id),
            _ => true,
        };
        if !sequence_ids_are_valid {
            return Err(ProtocolError::new(
                "feedback sequence identity is not canonical",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEvent {
    ActorMoved {
        actor_id: ActorId,
        from: Position,
        to: Position,
        navigation: NavigationKind,
    },
    Inspected {
        location: Position,
        tile: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        tile_move_cost: Option<i32>,
        exits: Vec<ObserverInspectExit>,
        nearby_actors: Vec<ObserverInspectActor>,
        ground_items: Vec<ObserverInspectGroundItem>,
    },
    GroupChanged {
        group_id: DecimalU64,
    },
    GroupInvitationChanged {
        invitation_id: DecimalU64,
    },
    GroupPresenceChanged {
        group_id: DecimalU64,
        character_id: CharacterId,
        connected: bool,
    },
    PlayerFollowChanged {
        follower_character_id: CharacterId,
        target_character_id: Option<CharacterId>,
    },
    CommunicationPreferencesChanged,
    ItemOfferChanged {
        item_instance_id: ItemInstanceId,
    },
    DefeatRewardShare {
        character_id: CharacterId,
        amount: i32,
    },
    Feedback {
        cue: FeedbackCue,
    },
}

impl ObservedEvent {
    fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Feedback { cue } => cue.validate(),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostilityAuthorization {
    Safe,
    ConfirmedUnsafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalAttackMode {
    Fight,
    Kick,
    Jumpkick,
    Poke,
    Shoot,
    Throw,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellTarget {
    None,
    SelfTarget,
    Actor {
        actor_id: ActorId,
    },
    Path {
        directions: Vec<Direction>,
    },
    Coordinate {
        position: Position,
    },
    Area {
        center: Position,
    },
    Direction {
        direction: Direction,
    },
    Door {
        direction: Direction,
    },
    Item {
        item_instance_id: ItemInstanceId,
        location: SpellItemLocation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellItemLocation {
    Sack,
    ActiveEquipment,
    GroundHere,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemMoveDestination {
    GroundHere,
    Carried { position: CarriedPosition },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoldMoveSource {
    Carried { position: CarriedGoldPosition },
    Ground { gold_pile_id: WireLabel },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoldMoveDestination {
    Carried { position: CarriedGoldPosition },
    GroundHere,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoldMoveQuantity {
    All,
    Exact { amount: DecimalI64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementPace {
    Walk,
    Run,
    Sprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurdenTier {
    LightlyLoaded,
    ModeratelyLoaded,
    HeavilyLoaded,
    VeryHeavilyLoaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementExertion {
    None,
    Normal,
    Rapid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementStopReason {
    FullPathAccepted,
    Blocked,
    Transitioned,
    ZeroStaminaLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPreviewBlockedReason {
    SuppressedByStatus,
    OutOfBounds,
    BlockedTerrain,
    InsufficientMovementPoints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathPreviewStepOutcome {
    Moved {
        navigation: NavigationKind,
    },
    Transitioned {
        navigation: NavigationKind,
        to: Position,
    },
    Blocked {
        reason: PathPreviewBlockedReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathPreviewStep {
    pub index: DecimalU64,
    pub direction: Direction,
    pub from: Position,
    pub attempted: Position,
    pub opens_door: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub terrain_name: Option<WireLabel>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub cost: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub remaining_points_after: Option<i32>,
    pub outcome: PathPreviewStepOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Burden {
    pub item_burden: DecimalU64,
    pub coin_burden: DecimalU64,
    pub total_burden: DecimalU64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub lightly_loaded_limit: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub moderately_loaded_limit: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub heavily_loaded_limit: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub tier: Option<BurdenTier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathPreview {
    pub contract_version: u32,
    pub actor_id: ActorId,
    pub start: Position,
    pub pace: MovementPace,
    pub requested_path: Vec<Direction>,
    pub available_path_points: i32,
    pub accepted_steps: DecimalU64,
    pub steps: Vec<PathPreviewStep>,
    pub stop_reason: MovementStopReason,
    pub final_position: Position,
    pub remaining_path_points: i32,
    pub burden: Burden,
    pub movement_exertion: MovementExertion,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_before: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_cost: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_after: Option<i32>,
}

impl PathPreview {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.contract_version != 8 {
            return Err(ProtocolError::new(
                "path preview contract version is not current",
            ));
        }
        if self.requested_path.is_empty() || self.requested_path.len() > MAX_MOVE_PATH_STEPS {
            return Err(ProtocolError::new(
                "path preview must contain 1-3 requested steps",
            ));
        }
        if self.accepted_steps.get() > self.requested_path.len() as u64
            || self.steps.len() > self.requested_path.len()
            || self.accepted_steps.get() > self.steps.len() as u64
        {
            return Err(ProtocolError::new("path preview step counts are invalid"));
        }
        let expected_pace = match self.requested_path.len() {
            1 => MovementPace::Walk,
            2 => MovementPace::Run,
            3 => MovementPace::Sprint,
            _ => unreachable!("requested path length was validated"),
        };
        if self.pace != expected_pace {
            return Err(ProtocolError::new("path preview pace is invalid"));
        }
        if self
            .burden
            .item_burden
            .get()
            .checked_add(self.burden.coin_burden.get())
            != Some(self.burden.total_burden.get())
        {
            return Err(ProtocolError::new("path preview burden total is invalid"));
        }
        let burden_shape = [
            self.burden.lightly_loaded_limit.is_some(),
            self.burden.moderately_loaded_limit.is_some(),
            self.burden.heavily_loaded_limit.is_some(),
            self.burden.tier.is_some(),
        ];
        if !burden_shape.iter().all(|present| *present)
            && burden_shape.iter().any(|present| *present)
        {
            return Err(ProtocolError::new(
                "path preview burden classification is incomplete",
            ));
        }
        let stamina_shape = [
            self.stamina_before.is_some(),
            self.stamina_cost.is_some(),
            self.stamina_after.is_some(),
        ];
        if !stamina_shape.iter().all(|present| *present)
            && stamina_shape.iter().any(|present| *present)
        {
            return Err(ProtocolError::new(
                "path preview stamina classification is incomplete",
            ));
        }

        let mut current = &self.start;
        let mut accepted = 0_u64;
        let mut last_remaining = self.available_path_points;
        for (index, step) in self.steps.iter().enumerate() {
            if step.index.get() != index as u64
                || step.direction != self.requested_path[index]
                || &step.from != current
            {
                return Err(ProtocolError::new("path preview step ordering is invalid"));
            }
            match &step.outcome {
                PathPreviewStepOutcome::Moved { .. } => {
                    if step.terrain_name.is_none()
                        || step.cost.is_none()
                        || step.remaining_points_after.is_none()
                        || step.opens_door
                    {
                        return Err(ProtocolError::new(
                            "path preview moved-step facts are invalid",
                        ));
                    }
                    accepted += 1;
                    current = &step.attempted;
                    last_remaining = step.remaining_points_after.unwrap();
                }
                PathPreviewStepOutcome::Transitioned { navigation, to } => {
                    if step.terrain_name.is_none()
                        || step.cost.is_none()
                        || step.remaining_points_after.is_none()
                        || (step.opens_door && *navigation != NavigationKind::Door)
                    {
                        return Err(ProtocolError::new(
                            "path preview transition-step facts are invalid",
                        ));
                    }
                    accepted += 1;
                    current = to;
                    last_remaining = step.remaining_points_after.unwrap();
                }
                PathPreviewStepOutcome::Blocked { .. } => {
                    if step.terrain_name.is_some()
                        || step.cost.is_some()
                        || step.remaining_points_after.is_some()
                        || step.opens_door
                    {
                        return Err(ProtocolError::new(
                            "path preview blocked-step facts are invalid",
                        ));
                    }
                }
            }
        }
        if self.accepted_steps.get() != accepted
            || &self.final_position != current
            || self.remaining_path_points != last_remaining
        {
            return Err(ProtocolError::new(
                "path preview accepted prefix is inconsistent",
            ));
        }
        let stop_is_valid = match self.stop_reason {
            MovementStopReason::FullPathAccepted => {
                self.steps.len() == self.requested_path.len()
                    && self
                        .steps
                        .iter()
                        .all(|step| !matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
            }
            MovementStopReason::Blocked => {
                self.steps.last().is_some_and(|step| {
                    matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. })
                }) && self
                    .steps
                    .iter()
                    .filter(|step| matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
                    .count()
                    == 1
            }
            MovementStopReason::Transitioned => {
                self.steps.last().is_some_and(|step| {
                    matches!(step.outcome, PathPreviewStepOutcome::Transitioned { .. })
                }) && self
                    .steps
                    .iter()
                    .all(|step| !matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
            }
            MovementStopReason::ZeroStaminaLimit => {
                self.steps.len() < self.requested_path.len()
                    && self
                        .steps
                        .iter()
                        .all(|step| !matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
            }
        };
        if !stop_is_valid {
            return Err(ProtocolError::new("path preview stop reason is invalid"));
        }
        Ok(())
    }
}

fn deserialize_required_nullable_spell_target<'de, D>(
    deserializer: D,
) -> Result<Option<SpellTarget>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<SpellTarget>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    MovePath {
        path: Vec<Direction>,
    },
    Traverse {
        traversal: ExplicitTraversalKind,
    },
    Open {
        direction: Direction,
    },
    Close {
        direction: Direction,
    },
    Inspect,
    Hide,
    ShowSack,
    Wait,
    Rest,
    PhysicalAttack {
        mode: PhysicalAttackMode,
        target_actor_id: ActorId,
        authorization: HostilityAuthorization,
    },
    Nock,
    UnloadBow,
    WarmSpell {
        spell_id: WireLabel,
    },
    CastSpell {
        spell_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable_spell_target")]
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    CastWarmedSpell {
        #[serde(deserialize_with = "deserialize_required_nullable_spell_target")]
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    FizzleWarmedSpell,
    SearchCorpse {
        corpse_id: CorpseId,
    },
    MoveItem {
        item_instance_id: ItemInstanceId,
        destination: ItemMoveDestination,
    },
    MoveGold {
        source: GoldMoveSource,
        destination: GoldMoveDestination,
        quantity: GoldMoveQuantity,
    },
    DepositBankGold {
        service_id: WireLabel,
        capability_id: WireLabel,
        gold_pile_id: WireLabel,
    },
    WithdrawBankGold {
        service_id: WireLabel,
        capability_id: WireLabel,
        amount: DecimalI64,
    },
    DepositLockerItem {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
    },
    WithdrawLockerItem {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
        destination: CarriedPosition,
    },
    DrinkItem {
        item_instance_id: ItemInstanceId,
    },
    Train {
        service_id: WireLabel,
        offered_gold: DecimalI64,
    },
    Critique {
        service_id: WireLabel,
        track_id: WireLabel,
    },
    PromoteClass {
        target_class_id: WireLabel,
    },
    LearnSpell {
        spell_id: WireLabel,
    },
    CommitServiceTransaction {
        service_id: WireLabel,
        capability_id: WireLabel,
        transaction_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        item_instance_id: Option<ItemInstanceId>,
    },
    BuyFromMerchant {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_ids: Vec<ItemInstanceId>,
    },
    SellToMerchant {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
    },
    UseItemService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation: ItemServiceOperationKind,
        item_instance_id: ItemInstanceId,
    },
    UseRestorationService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        item_instance_id: Option<ItemInstanceId>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        corpse_id: Option<CorpseId>,
    },
    InteractWithNpc {
        npc_actor_id: ActorId,
        interaction_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        item_instance_id: Option<ItemInstanceId>,
    },
    ClearSelfDefense {
        attacker_character_id: CharacterId,
    },
    Invite {
        target_character_id: CharacterId,
    },
    AcceptInvite {
        invitation_id: DecimalU64,
    },
    DeclineInvite {
        invitation_id: DecimalU64,
    },
    CancelInvite {
        invitation_id: DecimalU64,
    },
    LeaveGroup,
    RemoveMember {
        member_character_id: CharacterId,
    },
    DisbandGroup,
    TransferLeadership {
        member_character_id: CharacterId,
    },
    BeginFollow {
        target_character_id: CharacterId,
    },
    EndFollow,
    SetPagesEnabled {
        enabled: bool,
    },
    Block {
        target_character_id: CharacterId,
    },
    Unblock {
        target_character_id: CharacterId,
    },
    OfferItem {
        recipient_character_id: CharacterId,
        item_instance_id: ItemInstanceId,
    },
    AcceptItemOffer {
        item_instance_id: ItemInstanceId,
        destination: CarriedPosition,
    },
    RefuseItemOffer {
        item_instance_id: ItemInstanceId,
    },
    WithdrawItemOffer {
        item_instance_id: ItemInstanceId,
    },
}

impl Intent {
    fn validate(&self) -> Result<(), ProtocolError> {
        if let Self::MovePath { path } = self
            && (path.is_empty() || path.len() > MAX_MOVE_PATH_STEPS)
        {
            return Err(ProtocolError::new("move path must contain 1-3 steps"));
        }
        match self {
            Self::CastSpell {
                target: Some(SpellTarget::Path { directions }),
                ..
            }
            | Self::CastWarmedSpell {
                target: Some(SpellTarget::Path { directions }),
                ..
            } if directions.is_empty() || directions.len() > MAX_MOVE_PATH_STEPS => {
                return Err(ProtocolError::new("spell path must contain 1-3 steps"));
            }
            Self::SearchCorpse { corpse_id }
                if !is_canonical_sequence_id(corpse_id.as_str(), "corpse:") =>
            {
                return Err(ProtocolError::new("corpse ID is not canonical"));
            }
            Self::MoveGold {
                source: GoldMoveSource::Ground { gold_pile_id },
                ..
            } if !is_canonical_sequence_id(gold_pile_id.as_str(), "gold:") => {
                return Err(ProtocolError::new("gold pile ID is not canonical"));
            }
            Self::MoveGold {
                quantity: GoldMoveQuantity::Exact { amount },
                ..
            } if amount.get() <= 0 => {
                return Err(ProtocolError::new("gold amount must be positive"));
            }
            Self::WithdrawBankGold { amount, .. } if amount.get() <= 0 => {
                return Err(ProtocolError::new(
                    "bank withdrawal amount must be positive",
                ));
            }
            Self::Train { offered_gold, .. } if offered_gold.get() <= 0 => {
                return Err(ProtocolError::new("training offer must be positive"));
            }
            Self::BuyFromMerchant {
                item_instance_ids, ..
            } => {
                if item_instance_ids.len() > MAX_MERCHANT_PURCHASE_ITEMS {
                    return Err(ProtocolError::new(
                        "merchant purchase contains too many items",
                    ));
                }
                let mut unique = BTreeSet::new();
                if item_instance_ids
                    .iter()
                    .any(|item_instance_id| !unique.insert(item_instance_id.as_str()))
                {
                    return Err(ProtocolError::new(
                        "merchant purchase contains duplicate items",
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientHelloEnvelope {
    ClientHello {
        ticket: AdmissionTicket,
        supported_minors: Vec<u16>,
    },
}

impl ClientHelloEnvelope {
    pub fn into_parts(self) -> Result<(AdmissionTicket, Vec<u16>), ProtocolError> {
        let Self::ClientHello {
            ticket,
            supported_minors,
        } = self;
        if supported_minors.is_empty() || supported_minors.len() > MAX_SUPPORTED_MINORS {
            return Err(ProtocolError::new(
                "supported_minors must contain 1-8 values",
            ));
        }
        let unique = supported_minors.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != supported_minors.len() {
            return Err(ProtocolError::new("supported_minors must be unique"));
        }
        Ok((ticket, supported_minors))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientCommandEnvelope {
    Command {
        command_id: CommandId,
        control_epoch: DecimalU64,
        client_sequence: DecimalU64,
        observed_world_revision: DecimalU64,
        actor_id: ActorId,
        intent: Intent,
    },
    PathPreview {
        preview_id: PreviewId,
        control_epoch: DecimalU64,
        observed_world_revision: DecimalU64,
        actor_id: ActorId,
        path: Vec<Direction>,
    },
    SocialMessage {
        message_id: MessageId,
        control_epoch: DecimalU64,
        actor_id: ActorId,
        scope: SocialScope,
        body: SocialBody,
    },
}

impl ClientCommandEnvelope {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Command { intent, .. } => intent.validate(),
            Self::PathPreview { path, .. }
                if path.is_empty() || path.len() > MAX_MOVE_PATH_STEPS =>
            {
                Err(ProtocolError::new(
                    "path preview request must contain 1-3 steps",
                ))
            }
            Self::PathPreview { .. } => Ok(()),
            Self::SocialMessage { .. } => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SocialScope {
    Say,
    Shout,
    Group,
    Page { target_character_id: CharacterId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDisposition {
    Accepted,
    Unavailable,
    NotGrouped,
    RateLimited,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayStatus {
    New,
    Replayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionCode {
    WrongActor,
    StaleControlEpoch,
    FutureWorldRevision,
    OutOfOrderClientSequence,
    RulesRejected,
    ProjectionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandDisposition {
    Accepted,
    Rejected { code: RejectionCode },
    CommandResultExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPreviewRejectionCode {
    WrongActor,
    StaleControlEpoch,
    FutureWorldRevision,
    RulesRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathPreviewDisposition {
    Previewed,
    Rejected { code: PathPreviewRejectionCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrainingReason {
    Shutdown,
    ControlReplaced,
    SessionEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    MalformedProtocol,
    BinaryMessage,
    OversizedProtocol,
    UnsupportedVersion,
    InvalidTicket,
    ExpiredTicket,
    ConsumedTicket,
    OriginRejected,
    HostRejected,
    HelloTimeout,
    Capacity,
    RateLimited,
    QueuePressure,
    CommandInProgress,
    GameplayMarkLocked,
    Unavailable,
    Draining,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerEnvelope {
    ServerWelcome {
        selected_major: u16,
        selected_minor: u16,
        connection_id: ConnectionId,
        actor_id: ActorId,
        control_epoch: DecimalU64,
        server_sequence: DecimalU64,
        world_revision: DecimalU64,
        static_scene_context: StaticSceneContext,
        frame: ObserverFrame,
    },
    CommandResult {
        command_id: CommandId,
        disposition: CommandDisposition,
        replay_status: ReplayStatus,
        server_sequence: Option<DecimalU64>,
        before_revision: Option<DecimalU64>,
        after_revision: Option<DecimalU64>,
        events: Vec<ObservedEvent>,
        events_truncated: bool,
    },
    PathPreviewResult {
        preview_id: PreviewId,
        disposition: PathPreviewDisposition,
        control_epoch: DecimalU64,
        actor_id: ActorId,
        world_revision: DecimalU64,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        preview: Option<PathPreview>,
    },
    StateUpdate {
        server_sequence: DecimalU64,
        world_revision: DecimalU64,
        events: Vec<ObservedEvent>,
        events_truncated: bool,
        static_scene_context: StaticSceneContext,
        frame: ObserverFrame,
    },
    SocialMessage {
        message_id: MessageId,
        scope: SocialScope,
        sender_character_id: CharacterId,
        sender_name: DisplayName,
        body: SocialBody,
    },
    MessageResult {
        message_id: MessageId,
        disposition: MessageDisposition,
    },
    ServerDraining {
        reason: DrainingReason,
        reconnect_hint: bool,
    },
    Error {
        code: ErrorCode,
    },
}

impl ServerEnvelope {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::ServerWelcome {
                selected_major,
                selected_minor,
                static_scene_context,
                frame,
                ..
            } => {
                if *selected_major != PROTOCOL_MAJOR || *selected_minor != PROTOCOL_MINOR {
                    return Err(ProtocolError::new(
                        "welcome selected an unsupported version",
                    ));
                }
                static_scene_context.validate()?;
                validate_static_scene_frame_pair(static_scene_context, frame)
            }
            Self::CommandResult { events, .. } => validate_observed_events(events),
            Self::StateUpdate {
                static_scene_context,
                frame,
                events,
                ..
            } => {
                static_scene_context.validate()?;
                validate_static_scene_frame_pair(static_scene_context, frame)?;
                validate_observed_events(events)
            }
            Self::PathPreviewResult {
                disposition,
                preview,
                ..
            } => match (disposition, preview) {
                (PathPreviewDisposition::Previewed, Some(preview)) => preview.validate(),
                (PathPreviewDisposition::Rejected { .. }, None) => Ok(()),
                _ => Err(ProtocolError::new(
                    "path preview disposition and payload disagree",
                )),
            },
            _ => Ok(()),
        }
    }
}

fn validate_static_scene_frame_pair(
    context: &StaticSceneContext,
    frame: &ObserverFrame,
) -> Result<(), ProtocolError> {
    frame.validate()?;
    if context.site.realm != frame.observation_center.realm
        || context.site.level != frame.observation_center.level
    {
        return Err(ProtocolError::new(
            "static scene context site differs from observer frame",
        ));
    }
    Ok(())
}

fn validate_observed_events(events: &[ObservedEvent]) -> Result<(), ProtocolError> {
    if events.len() > MAX_OBSERVED_EVENTS {
        return Err(ProtocolError::new("observed event vector exceeds bound"));
    }
    events.iter().try_for_each(ObservedEvent::validate)
}

pub fn decode_client_hello(input: &[u8]) -> Result<ClientHelloEnvelope, ProtocolError> {
    let envelope: ClientHelloEnvelope = decode_strict(input)?;
    envelope.clone().into_parts()?;
    Ok(envelope)
}

pub fn decode_login_request(input: &[u8]) -> Result<LoginRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_logout_request(input: &[u8]) -> Result<LogoutRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_character_select_request(
    input: &[u8],
) -> Result<CharacterSelectRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_socket_ticket_request(input: &[u8]) -> Result<SocketTicketRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_forgive_player_kill_mark_request(
    input: &[u8],
) -> Result<ForgivePlayerKillMarkRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_client_command(
    input: &[u8],
    selected_minor: u16,
) -> Result<ClientCommandEnvelope, ProtocolError> {
    if selected_minor != PROTOCOL_MINOR {
        return Err(ProtocolError::new("unsupported selected protocol minor"));
    }
    let envelope: ClientCommandEnvelope = decode_strict(input)?;
    envelope.validate()?;
    Ok(envelope)
}

pub fn encode_server_envelope(envelope: &ServerEnvelope) -> Result<Vec<u8>, ProtocolError> {
    envelope.validate()?;
    let encoded =
        serde_json::to_vec(envelope).map_err(|error| ProtocolError::new(error.to_string()))?;
    if encoded.len() > MAX_SERVER_ENVELOPE_BYTES {
        return Err(ProtocolError::new(
            "encoded server envelope exceeds 256 KiB",
        ));
    }
    Ok(encoded)
}

fn deserialize_required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn decode_strict<T: DeserializeOwned>(input: &[u8]) -> Result<T, ProtocolError> {
    decode_strict_with_limits(input, MAX_INPUT_BYTES, MAX_JSON_NESTING)
}

fn decode_control<T: DeserializeOwned>(input: &[u8]) -> Result<T, ProtocolError> {
    decode_strict_with_limits(input, MAX_CONTROL_INPUT_BYTES, MAX_CONTROL_JSON_NESTING)
}

fn decode_strict_with_limits<T: DeserializeOwned>(
    input: &[u8],
    max_bytes: usize,
    max_depth: usize,
) -> Result<T, ProtocolError> {
    if input.len() > max_bytes {
        return Err(ProtocolError::new("JSON input exceeds its byte bound"));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = StrictValueSeed {
        depth: 1,
        max_depth,
    }
    .deserialize(&mut deserializer)
    .map_err(|error| ProtocolError::new(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| ProtocolError::new(error.to_string()))?;
    serde_json::from_value(value).map_err(|error| ProtocolError::new(error.to_string()))
}

struct StrictValueSeed {
    depth: usize,
    max_depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.depth > self.max_depth {
            return Err(serde::de::Error::custom("JSON nesting exceeds its bound"));
        }
        deserializer.deserialize_any(StrictValueVisitor {
            depth: self.depth,
            max_depth: self.max_depth,
        })
    }
}

struct StrictValueVisitor {
    depth: usize,
    max_depth: usize,
}

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a strict JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictValueSeed {
            depth: self.depth + 1,
            max_depth: self.max_depth,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate object key {key:?}"
                )));
            }
            let value = map.next_value_seed(StrictValueSeed {
                depth: self.depth + 1,
                max_depth: self.max_depth,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}
