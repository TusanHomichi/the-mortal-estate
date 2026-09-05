use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AbilityKindCheckpointV2 {
    Spell,
    SpecialAttack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AbilityTargetCheckpointV2 {
    NearestHostile,
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MonsterAbilityCheckpointV2 {
    pub(super) id: String,
    pub(super) kind: AbilityKindCheckpointV2,
    pub(super) spell_id: String,
    pub(super) cooldown_rounds: u32,
    pub(super) target_policy: AbilityTargetCheckpointV2,
    pub(super) ready_at: LogicalTime,
}

impl From<&MonsterAbilityState> for MonsterAbilityCheckpointV2 {
    fn from(value: &MonsterAbilityState) -> Self {
        Self {
            id: value.id.clone(),
            kind: match value.kind {
                MonsterAbilityKind::Spell => AbilityKindCheckpointV2::Spell,
                MonsterAbilityKind::SpecialAttack => AbilityKindCheckpointV2::SpecialAttack,
            },
            spell_id: value.spell_id.clone(),
            cooldown_rounds: value.cooldown_rounds,
            target_policy: match value.target_policy {
                MonsterAbilityTargetPolicy::NearestHostile => {
                    AbilityTargetCheckpointV2::NearestHostile
                }
                MonsterAbilityTargetPolicy::SelfTarget => AbilityTargetCheckpointV2::SelfTarget,
            },
            ready_at: value.ready_at,
        }
    }
}

impl From<MonsterAbilityCheckpointV2> for MonsterAbilityState {
    fn from(value: MonsterAbilityCheckpointV2) -> Self {
        Self {
            id: value.id,
            kind: match value.kind {
                AbilityKindCheckpointV2::Spell => MonsterAbilityKind::Spell,
                AbilityKindCheckpointV2::SpecialAttack => MonsterAbilityKind::SpecialAttack,
            },
            spell_id: value.spell_id,
            cooldown_rounds: value.cooldown_rounds,
            target_policy: match value.target_policy {
                AbilityTargetCheckpointV2::NearestHostile => {
                    MonsterAbilityTargetPolicy::NearestHostile
                }
                AbilityTargetCheckpointV2::SelfTarget => MonsterAbilityTargetPolicy::SelfTarget,
            },
            ready_at: value.ready_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SocialRelationsCheckpointV3 {
    pub(super) self_defense: BTreeMap<CharacterId, SelfDefenseRightV1>,
    pub(super) npc_grudges: BTreeSet<NpcGrudgeRelation>,
}

impl From<&SocialRelationLedger> for SocialRelationsCheckpointV3 {
    fn from(value: &SocialRelationLedger) -> Self {
        Self {
            self_defense: value.self_defense.clone(),
            npc_grudges: value.npc_grudges.clone(),
        }
    }
}

impl From<SocialRelationsCheckpointV3> for SocialRelationLedger {
    fn from(value: SocialRelationsCheckpointV3) -> Self {
        Self {
            self_defense: value.self_defense,
            npc_grudges: value.npc_grudges,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MerchantInventoryCheckpointV2 {
    pub(super) service_id: String,
    pub(super) capability_id: String,
    pub(super) listings: Vec<MerchantListingCheckpointV2>,
}

impl MerchantInventoryCheckpointV2 {
    pub(super) fn new(id: &MerchantInventoryId, state: &MerchantInventoryState) -> Self {
        Self {
            service_id: id.service_id.clone(),
            capability_id: id.capability_id.clone(),
            listings: state.listings.iter().map(Into::into).collect(),
        }
    }

    pub(super) fn into_pair(
        self,
    ) -> Result<(MerchantInventoryId, MerchantInventoryState), CheckpointError> {
        if self.service_id.is_empty() || self.capability_id.is_empty() {
            return Err(CheckpointError::new("merchant inventory identity is empty"));
        }
        Ok((
            MerchantInventoryId::new(self.service_id, self.capability_id),
            MerchantInventoryState {
                listings: self.listings.into_iter().map(Into::into).collect(),
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ListingOriginCheckpointV2 {
    AuthoredStock,
    PawnPool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MerchantListingCheckpointV2 {
    pub(super) item_instance_id: String,
    pub(super) origin: ListingOriginCheckpointV2,
    pub(super) price_gold: i64,
}

impl From<&MerchantListingState> for MerchantListingCheckpointV2 {
    fn from(value: &MerchantListingState) -> Self {
        Self {
            item_instance_id: value.item_instance_id.clone(),
            origin: match value.origin {
                MerchantListingOrigin::AuthoredStock => ListingOriginCheckpointV2::AuthoredStock,
                MerchantListingOrigin::PawnPool => ListingOriginCheckpointV2::PawnPool,
            },
            price_gold: value.price_gold,
        }
    }
}

impl From<MerchantListingCheckpointV2> for MerchantListingState {
    fn from(value: MerchantListingCheckpointV2) -> Self {
        Self {
            item_instance_id: value.item_instance_id,
            origin: match value.origin {
                ListingOriginCheckpointV2::AuthoredStock => MerchantListingOrigin::AuthoredStock,
                ListingOriginCheckpointV2::PawnPool => MerchantListingOrigin::PawnPool,
            },
            price_gold: value.price_gold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PositionBoolCheckpointV2 {
    pub(super) position: WorldPosition,
    pub(super) value: bool,
}

pub(super) fn sorted_position_bools(
    values: &std::collections::HashMap<WorldPosition, bool>,
) -> Vec<PositionBoolCheckpointV2> {
    let mut entries = values
        .iter()
        .map(|(position, value)| PositionBoolCheckpointV2 {
            position: position.clone(),
            value: *value,
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

pub(super) fn position_bools(
    values: Vec<PositionBoolCheckpointV2>,
) -> Result<std::collections::HashMap<WorldPosition, bool>, CheckpointError> {
    let mut result = std::collections::HashMap::new();
    for value in values {
        if result.insert(value.position, value.value).is_some() {
            return Err(CheckpointError::new(
                "checkpoint contains duplicate world-position Boolean",
            ));
        }
    }
    Ok(result)
}

pub(super) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}
