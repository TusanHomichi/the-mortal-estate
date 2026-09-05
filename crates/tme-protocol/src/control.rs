use super::*;

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
