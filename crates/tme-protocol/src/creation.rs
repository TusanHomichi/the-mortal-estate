use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterCreationDraftV1 {
    pub profile_id: WireLabel,
    pub display_name: DisplayName,
    pub attributes: CharacterAttributes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterCreateRequestV1 {
    pub csrf_token: CsrfToken,
    pub request_id: CommandId,
    pub draft: CharacterCreationDraftV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterCreationOptionV1 {
    pub profile_id: WireLabel,
    pub class_name: WireLabel,
    pub nationality: WireLabel,
    pub minimum: CharacterAttributes,
    pub maximum: CharacterAttributes,
    pub suggested: CharacterAttributes,
    pub attribute_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterCreationOptionsV1 {
    pub control_api_version: u16,
    pub options: Vec<CharacterCreationOptionV1>,
    pub maximum_characters: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterCreatedV1 {
    pub control_api_version: u16,
    pub character: CharacterSummaryV1,
    pub replay_status: ReplayStatus,
}

pub fn decode_character_create_request(
    input: &[u8],
) -> Result<CharacterCreateRequestV1, ProtocolError> {
    decode_control(input)
}
