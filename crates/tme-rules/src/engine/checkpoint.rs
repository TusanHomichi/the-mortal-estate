use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::content::{
    SeedWorldPositionStatus, SelectedCatalog, ValidationError, WorldSeedValidationContext,
    WorldTemplateV3,
};
use crate::events::Event;
use crate::model::*;
use crate::rng::DeterministicRng;

use super::{Engine, GameDefinition};

pub const FACET_CHECKPOINT_SCHEMA_VERSION: u32 = 4;
const FACET_CHECKPOINT_KIND: &str = "facet_checkpoint";
pub const MAX_FACET_CHECKPOINT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentIdentityV1 {
    pub catalog_id: String,
    pub catalog_profile: String,
    pub world_template_id: String,
    pub definition_sha256: String,
}

impl ContentIdentityV1 {
    pub(crate) fn from_selected(
        selected: &SelectedCatalog,
        template: &WorldTemplateV3,
    ) -> Result<Self, ValidationError> {
        let bytes = serde_json::to_vec(&(selected, template)).map_err(|error| {
            ValidationError::new(vec![format!(
                "selected content could not be serialized for identity: {error}"
            )])
        })?;
        Ok(Self {
            catalog_id: selected.catalog_id.clone(),
            catalog_profile: selected.profile_key.as_str().to_string(),
            world_template_id: template.id.clone(),
            definition_sha256: hex_lower(&Sha256::digest(bytes)),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetCheckpointV4 {
    bytes: Vec<u8>,
}

impl FacetCheckpointV4 {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, CheckpointError> {
        if bytes.is_empty() || bytes.len() > MAX_FACET_CHECKPOINT_BYTES {
            return Err(CheckpointError::new(
                "checkpoint byte count is out of bounds",
            ));
        }
        let payload: FacetCheckpointPayloadV1 =
            serde_json::from_slice(&bytes).map_err(CheckpointError::json)?;
        payload.validate_header()?;
        let canonical = serde_json::to_vec(&payload).map_err(CheckpointError::json)?;
        if canonical != bytes {
            return Err(CheckpointError::new(
                "checkpoint bytes are not canonical JSON",
            ));
        }
        Ok(Self { bytes })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(&self.bytes).into()
    }

    fn decode(&self) -> Result<FacetCheckpointPayloadV1, CheckpointError> {
        serde_json::from_slice(&self.bytes).map_err(CheckpointError::json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointError(String);

impl CheckpointError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    fn json(error: serde_json::Error) -> Self {
        Self::new(format!("invalid checkpoint JSON: {error}"))
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CheckpointError {}

impl Engine {
    pub fn export_checkpoint(&self) -> Result<FacetCheckpointV4, CheckpointError> {
        let payload = FacetCheckpointPayloadV1::from_engine(self);
        let bytes = serde_json::to_vec(&payload).map_err(CheckpointError::json)?;
        FacetCheckpointV4::from_bytes(bytes)
    }

    pub fn hydrate_checkpoint(
        definition: Arc<GameDefinition>,
        checkpoint: &FacetCheckpointV4,
    ) -> Result<Self, CheckpointError> {
        let payload = checkpoint.decode()?;
        payload.validate_header()?;
        if &payload.content != definition.content_identity() {
            return Err(CheckpointError::new("checkpoint content identity mismatch"));
        }
        let engine = payload.into_engine(definition)?;
        engine
            .validate_world_item_locations()
            .map_err(|error| CheckpointError::new(error.to_string()))?;
        engine
            .validate_bow_readiness_invariants()
            .map_err(|error| CheckpointError::new(error.to_string()))?;
        engine
            .validate_world_item_burden()
            .map_err(|error| CheckpointError::new(error.to_string()))?;
        validate_character_ownership(&engine)?;
        validate_checkpoint_references(&engine)?;
        validate_social_checkpoint_state(&engine)?;
        let reencoded = engine.export_checkpoint()?;
        if reencoded.as_bytes() != checkpoint.as_bytes() {
            return Err(CheckpointError::new(
                "hydrated checkpoint does not re-export byte-identically",
            ));
        }
        Ok(engine)
    }
}

fn validate_checkpoint_references(engine: &Engine) -> Result<(), CheckpointError> {
    let definition = engine.definition.as_ref();
    let actor_ids = engine
        .world
        .actors
        .iter()
        .map(|actor| actor.id.clone())
        .collect::<BTreeSet<_>>();
    let character_ids = engine
        .world
        .actors
        .iter()
        .filter_map(|actor| actor.character_id.clone())
        .collect::<BTreeSet<_>>();

    for actor in &engine.world.actors {
        let authored = definition
            .catalog
            .actor_definitions
            .get(&actor.definition_id)
            .ok_or_else(|| CheckpointError::new("checkpoint actor definition is unknown"))?;
        if authored.kind != actor.kind {
            return Err(CheckpointError::new(
                "checkpoint actor kind differs from its definition",
            ));
        }
        validate_position(definition, &actor.location)?;
        validate_position(definition, &actor.home_location)?;
        if let Some(origin) = &actor.ecology_origin {
            let site = engine
                .world
                .ecology_sites
                .get(&origin.site_id)
                .ok_or_else(|| CheckpointError::new("checkpoint ecology origin site is unknown"))?;
            let slot = site.member_slots.get(&origin.member_id).ok_or_else(|| {
                CheckpointError::new("checkpoint ecology origin member slot is unknown")
            })?;
            if origin.generation > site.generation
                || (actor.is_alive() && slot.actor_id.as_ref() != Some(&actor.id))
            {
                return Err(CheckpointError::new(
                    "checkpoint ecology actor origin disagrees with its site",
                ));
            }
        }
    }

    for (site_id, site) in &engine.world.ecology_sites {
        if site_id != &site.id
            || !definition
                .catalog
                .spawn_groups
                .contains_key(&site.spawn_group_id)
        {
            return Err(CheckpointError::new(
                "checkpoint ecology site identity is invalid",
            ));
        }
        let group = &definition.catalog.spawn_groups[&site.spawn_group_id];
        let expected_members = group
            .members
            .iter()
            .map(|member| member.member_id.as_str())
            .collect::<BTreeSet<_>>();
        let actual_members = site
            .member_slots
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if actual_members != expected_members {
            return Err(CheckpointError::new(
                "checkpoint ecology member slots disagree with the spawn group",
            ));
        }
        if site.full_clear_due_at.is_some()
            && site
                .member_slots
                .values()
                .any(|slot| slot.actor_id.is_some())
        {
            return Err(CheckpointError::new(
                "checkpoint ecology full-clear due state has a living slot",
            ));
        }
        for (member_id, slot) in &site.member_slots {
            if member_id != &slot.member_id || (slot.actor_id.is_some() && slot.due_at.is_some()) {
                return Err(CheckpointError::new(
                    "checkpoint ecology member slot state is invalid",
                ));
            }
            validate_position(definition, &slot.location)?;
            let Some(actor_id) = &slot.actor_id else {
                continue;
            };
            let actor = engine
                .world
                .actor(actor_id)
                .ok_or_else(|| CheckpointError::new("checkpoint ecology actor is unknown"))?;
            if !actor.is_alive()
                || actor.ecology_origin.as_ref().is_none_or(|origin| {
                    origin.site_id != *site_id
                        || origin.member_id != *member_id
                        || origin.generation > site.generation
                })
            {
                return Err(CheckpointError::new(
                    "checkpoint ecology site disagrees with its actor origin",
                ));
            }
        }
    }

    let mut service_ids = BTreeSet::new();
    for service in &engine.world.service_instances {
        if !service_ids.insert(&service.id)
            || !definition
                .catalog
                .service_definitions
                .iter()
                .any(|candidate| candidate.id == service.definition_id)
        {
            return Err(CheckpointError::new(
                "checkpoint service identity is invalid",
            ));
        }
        validate_position(definition, &service.position)?;
    }

    for (instance_id, item) in &engine.world.item_instances {
        if instance_id.is_empty()
            || item.quantity == 0
            || !definition
                .catalog
                .item_catalog
                .contains_key(&item.definition_id)
        {
            return Err(CheckpointError::new("checkpoint item instance is invalid"));
        }
    }
    for item in &engine.world.ground_items {
        validate_position(definition, &item.location)?;
    }

    for (bank_id, bank) in &engine.world.banks {
        if !definition.catalog.bank_definitions.contains_key(bank_id)
            || bank.balances.iter().any(|(character_id, balance)| {
                !character_ids.contains(character_id) || *balance < 0
            })
        {
            return Err(CheckpointError::new("checkpoint bank state is invalid"));
        }
    }
    for (vault_id, vault) in &engine.world.locker_vaults {
        let definition = definition
            .catalog
            .locker_vault_definitions
            .get(vault_id)
            .ok_or_else(|| CheckpointError::new("checkpoint locker definition is unknown"))?;
        if vault.lockers.iter().any(|(character_id, items)| {
            !character_ids.contains(character_id)
                || items.len() > definition.capacity as usize
                || items
                    .iter()
                    .any(|item_id| !engine.world.item_instances.contains_key(item_id))
        }) {
            return Err(CheckpointError::new("checkpoint locker state is invalid"));
        }
    }
    for (item_id, offer) in &engine.world.item_offers {
        if !engine.world.item_instances.contains_key(item_id)
            || !character_ids.contains(&offer.sender_character_id)
            || !character_ids.contains(&offer.recipient_character_id)
            || offer.sender_character_id == offer.recipient_character_id
        {
            return Err(CheckpointError::new("checkpoint item offer is invalid"));
        }
    }
    for (character_id, quests) in &engine.world.quest_states {
        if !character_ids.contains(character_id) {
            return Err(CheckpointError::new("checkpoint quest owner is unknown"));
        }
        for (quest_id, stage_id) in quests {
            let quest = definition
                .catalog
                .quests
                .get(quest_id)
                .ok_or_else(|| CheckpointError::new("checkpoint quest is unknown"))?;
            if !quest.stages.contains_key(stage_id) {
                return Err(CheckpointError::new("checkpoint quest stage is unknown"));
            }
        }
    }

    let mut max_corpse_sequence = 0;
    for (corpse_id, corpse) in &engine.world.corpses {
        if corpse_id != &corpse.id {
            return Err(CheckpointError::new(
                "checkpoint corpse key differs from its identity",
            ));
        }
        validate_position(definition, &corpse.location)?;
        max_corpse_sequence = max_corpse_sequence.max(corpse.sequence);
    }
    if engine.world.next_corpse_sequence == 0
        || engine.world.next_corpse_sequence <= max_corpse_sequence
    {
        return Err(CheckpointError::new(
            "checkpoint corpse allocation sequence is invalid",
        ));
    }
    let mut max_gold_sequence = 0;
    for (gold_id, pile) in &engine.world.ground_gold {
        if gold_id != &pile.id || pile.amount <= 0 {
            return Err(CheckpointError::new(
                "checkpoint ground-gold state is invalid",
            ));
        }
        validate_position(definition, &pile.location)?;
        max_gold_sequence = max_gold_sequence.max(sequence_suffix(gold_id.as_str(), "gold:")?);
    }
    if engine.world.next_gold_sequence == 0 || engine.world.next_gold_sequence <= max_gold_sequence
    {
        return Err(CheckpointError::new(
            "checkpoint gold allocation sequence is invalid",
        ));
    }

    for effect in &engine.world.tile_effects {
        validate_position(definition, &effect.location)?;
    }
    for enchantment in &engine.world.item_enchantments {
        if !engine
            .world
            .item_instances
            .contains_key(&enchantment.item_instance_id)
        {
            return Err(CheckpointError::new(
                "checkpoint enchantment item is unknown",
            ));
        }
    }
    for portal in &engine.world.portal_transitions {
        validate_position(definition, &portal.location)?;
        validate_position(definition, &portal.target)?;
    }
    for concealed in &engine.world.concealed_transitions {
        validate_position(definition, &concealed.location)?;
    }
    for position in engine
        .world
        .hidden_transition_revealed
        .keys()
        .chain(engine.world.door_states.keys())
    {
        validate_position(definition, position)?;
    }

    for (victim, relation) in &engine.world.social_relations.self_defense {
        if victim != &relation.victim_character_id
            || !character_ids.contains(victim)
            || relation.victim_character_id == relation.attacker_character_id
        {
            return Err(CheckpointError::new(
                "checkpoint self-defense relation is invalid",
            ));
        }
    }
    let mut previous_link = None;
    for link in &engine.world.linked_player_kill_karma {
        if link.facet_kill_sequence == 0
            || link.killer_character_id == link.victim_character_id
            || !character_ids.contains(&link.killer_character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint linked player-kill karma is invalid",
            ));
        }
        if previous_link
            .as_ref()
            .is_some_and(|previous| previous >= link)
        {
            return Err(CheckpointError::new(
                "checkpoint linked player-kill karma is not strictly ordered",
            ));
        }
        previous_link = Some(link.clone());
    }
    if engine
        .world
        .social_relations
        .npc_grudges
        .iter()
        .any(|relation| {
            !actor_ids.contains(&relation.npc_actor_id)
                || !actor_ids.contains(&relation.attacker_actor_id)
        })
    {
        return Err(CheckpointError::new(
            "checkpoint NPC grudge relation is invalid",
        ));
    }
    Ok(())
}

fn validate_position(
    definition: &GameDefinition,
    position: &WorldPosition,
) -> Result<(), CheckpointError> {
    match definition.world_position_status(position) {
        Some(SeedWorldPositionStatus::Passable | SeedWorldPositionStatus::Blocked) => Ok(()),
        Some(SeedWorldPositionStatus::OutOfBounds) | None => Err(CheckpointError::new(
            "checkpoint world position is outside immutable content",
        )),
    }
}

fn sequence_suffix(value: &str, prefix: &str) -> Result<u64, CheckpointError> {
    value
        .strip_prefix(prefix)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| CheckpointError::new("checkpoint sequence identity is invalid"))
}

fn validate_social_checkpoint_state(engine: &Engine) -> Result<(), CheckpointError> {
    let character_ids = engine
        .world
        .actors
        .iter()
        .filter_map(|actor| actor.character_id.clone())
        .collect::<BTreeSet<_>>();
    let preference_ids = engine
        .world
        .communication_preferences
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let presence_ids = engine
        .world
        .character_presence
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if preference_ids != character_ids || presence_ids != character_ids {
        return Err(CheckpointError::new(
            "checkpoint communication/presence ownership differs from local characters",
        ));
    }
    for (character_id, preferences) in &engine.world.communication_preferences {
        if preferences.blocked_character_ids.len() > MAX_BLOCKED_CHARACTERS
            || preferences.blocked_character_ids.contains(character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint communication preferences are invalid",
            ));
        }
    }
    for presence in engine.world.character_presence.values() {
        if presence.connected == presence.absent_since.is_some() {
            return Err(CheckpointError::new(
                "checkpoint character presence is internally inconsistent",
            ));
        }
    }
    if engine.world.next_group_sequence == 0
        || engine.world.next_group_invite_sequence == 0
        || engine.world.next_membership_epoch == 0
    {
        return Err(CheckpointError::new(
            "checkpoint social allocation sequence is zero",
        ));
    }

    let mut grouped_characters = BTreeSet::new();
    let mut membership_epochs = BTreeSet::new();
    let mut greatest_group_id = 0;
    for (group_id, group) in &engine.world.groups {
        greatest_group_id = greatest_group_id.max(group_id.value());
        if group.id != *group_id
            || group.members.len() < 2
            || group.members.len() > MAX_GROUP_MEMBERS
            || !group
                .members
                .iter()
                .any(|member| member.character_id == group.leader_character_id)
        {
            return Err(CheckpointError::new("checkpoint group shape is invalid"));
        }
        let mut joined_orders = BTreeSet::new();
        let mut greatest_join_order = 0;
        for member in &group.members {
            greatest_join_order = greatest_join_order.max(member.joined_order);
            if !character_ids.contains(&member.character_id)
                || !grouped_characters.insert(member.character_id.clone())
                || !joined_orders.insert(member.joined_order)
                || !membership_epochs.insert(member.membership_epoch)
                || member.membership_epoch >= engine.world.next_membership_epoch
            {
                return Err(CheckpointError::new(
                    "checkpoint group membership is invalid",
                ));
            }
        }
        if group.next_join_order <= greatest_join_order {
            return Err(CheckpointError::new(
                "checkpoint group join sequence is invalid",
            ));
        }
    }
    if engine.world.next_group_sequence <= greatest_group_id {
        return Err(CheckpointError::new(
            "checkpoint group allocation sequence is invalid",
        ));
    }

    let mut incoming = BTreeMap::<CharacterId, usize>::new();
    let mut outgoing = BTreeMap::<(Option<GroupId>, Option<CharacterId>), usize>::new();
    let mut greatest_invitation_id = 0;
    for (invitation_id, invitation) in &engine.world.group_invitations {
        greatest_invitation_id = greatest_invitation_id.max(invitation_id.value());
        if invitation.id != *invitation_id
            || invitation.issuer_character_id == invitation.target_character_id
            || !character_ids.contains(&invitation.issuer_character_id)
            || !character_ids.contains(&invitation.target_character_id)
            || grouped_characters.contains(&invitation.target_character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint group invitation shape is invalid",
            ));
        }
        if let Some(group_id) = invitation.group_id {
            let group =
                engine.world.groups.get(&group_id).ok_or_else(|| {
                    CheckpointError::new("checkpoint invitation group is unknown")
                })?;
            if !group.members.iter().any(|member| {
                member.character_id == invitation.issuer_character_id
                    && Some(member.membership_epoch) == invitation.issuer_membership_epoch
            }) {
                return Err(CheckpointError::new(
                    "checkpoint invitation issuer membership is stale",
                ));
            }
        } else if invitation.issuer_membership_epoch.is_some()
            || grouped_characters.contains(&invitation.issuer_character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint solo invitation issuer is grouped",
            ));
        }
        *incoming
            .entry(invitation.target_character_id.clone())
            .or_default() += 1;
        let outgoing_key = match invitation.group_id {
            Some(group_id) => (Some(group_id), None),
            None => (None, Some(invitation.issuer_character_id.clone())),
        };
        *outgoing.entry(outgoing_key).or_default() += 1;
    }
    if incoming
        .values()
        .any(|count| *count > MAX_INCOMING_GROUP_INVITATIONS)
        || outgoing
            .values()
            .any(|count| *count > MAX_OUTGOING_GROUP_INVITATIONS)
        || engine.world.next_group_invite_sequence <= greatest_invitation_id
    {
        return Err(CheckpointError::new(
            "checkpoint group invitation bounds or sequence are invalid",
        ));
    }

    for (follower, target) in &engine.world.player_follow_targets {
        if follower == target
            || !character_ids.contains(follower)
            || !character_ids.contains(target)
            || engine.group_id_for_character(follower) != engine.group_id_for_character(target)
            || engine.group_id_for_character(follower).is_none()
        {
            return Err(CheckpointError::new(
                "checkpoint player follow edge is invalid",
            ));
        }
        let mut cursor = target;
        let mut visited = BTreeSet::new();
        while let Some(next) = engine.world.player_follow_targets.get(cursor) {
            if next == follower || !visited.insert(cursor.clone()) {
                return Err(CheckpointError::new(
                    "checkpoint player follow graph contains a cycle",
                ));
            }
            cursor = next;
        }
    }

    for (target_actor_id, ledger) in &engine.world.defeat_contributions {
        if !engine
            .world
            .actors
            .iter()
            .any(|actor| &actor.id == target_actor_id)
            || ledger.total_actual_damage == 0
        {
            return Err(CheckpointError::new(
                "checkpoint defeat contribution target is invalid",
            ));
        }
        let mut recorded_damage = 0_u64;
        for (unit_id, unit) in &ledger.reward_units {
            if unit.slices.is_empty() {
                return Err(CheckpointError::new(
                    "checkpoint defeat reward unit has no contribution slices",
                ));
            }
            for (slice, amount) in &unit.slices {
                if *amount == 0 {
                    return Err(CheckpointError::new(
                        "checkpoint defeat contribution slice is invalid",
                    ));
                }
                recorded_damage = recorded_damage.checked_add(*amount).ok_or_else(|| {
                    CheckpointError::new("checkpoint defeat contribution sum overflow")
                })?;
                if slice
                    .eligible_memberships
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                    || slice
                        .eligible_memberships
                        .iter()
                        .any(|membership| membership.membership_epoch == 0)
                {
                    return Err(CheckpointError::new(
                        "checkpoint defeat membership cohort is invalid",
                    ));
                }
            }
            match unit_id {
                DefeatRewardUnitId::Solo { character_id } => {
                    if unit.slices.keys().any(|slice| {
                        &slice.contributor_character_id != character_id
                            || !slice.eligible_memberships.is_empty()
                    }) {
                        return Err(CheckpointError::new(
                            "checkpoint solo defeat reward unit is invalid",
                        ));
                    }
                }
                DefeatRewardUnitId::Group { group_id } => {
                    engine.world.groups.get(group_id).ok_or_else(|| {
                        CheckpointError::new("checkpoint contribution group is unknown")
                    })?;
                    if unit
                        .slices
                        .keys()
                        .any(|slice| slice.eligible_memberships.is_empty())
                    {
                        return Err(CheckpointError::new(
                            "checkpoint group reward membership is invalid",
                        ));
                    }
                }
            }
        }
        if recorded_damage > ledger.total_actual_damage {
            return Err(CheckpointError::new(
                "checkpoint rewarded damage exceeds total actual damage",
            ));
        }
    }
    Ok(())
}

fn validate_character_ownership(engine: &Engine) -> Result<(), CheckpointError> {
    let mut actors = BTreeSet::new();
    let mut characters = BTreeSet::new();
    for actor in &engine.world.actors {
        if !actors.insert(actor.id.clone()) {
            return Err(CheckpointError::new(
                "checkpoint contains duplicate actor IDs",
            ));
        }
        if let Some(character_id) = &actor.character_id
            && !characters.insert(character_id.clone())
        {
            return Err(CheckpointError::new(
                "checkpoint contains duplicate character ownership",
            ));
        }
        if actor.character_id.is_some() != actor.character.is_some() {
            return Err(CheckpointError::new(
                "checkpoint character ID/sheet presence disagrees",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FacetCheckpointPayloadV1 {
    schema_version: u32,
    kind: String,
    content: ContentIdentityV1,
    rng_state: DecimalU32,
    world: WorldCheckpointV3,
    initial_events: Vec<Event>,
}

impl FacetCheckpointPayloadV1 {
    fn from_engine(engine: &Engine) -> Self {
        Self {
            schema_version: FACET_CHECKPOINT_SCHEMA_VERSION,
            kind: FACET_CHECKPOINT_KIND.to_string(),
            content: engine.definition.content_identity().clone(),
            rng_state: DecimalU32(engine.rng.checkpoint_state()),
            world: WorldCheckpointV3::from(&engine.world),
            initial_events: engine.initial_events.clone(),
        }
    }

    fn validate_header(&self) -> Result<(), CheckpointError> {
        if self.schema_version != FACET_CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::new(
                "unsupported checkpoint schema version",
            ));
        }
        if self.kind != FACET_CHECKPOINT_KIND {
            return Err(CheckpointError::new("checkpoint kind mismatch"));
        }
        if self.content.catalog_id.is_empty()
            || self.content.catalog_profile.is_empty()
            || self.content.world_template_id.is_empty()
            || self.content.definition_sha256.len() != 64
            || !self
                .content
                .definition_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(CheckpointError::new(
                "checkpoint content identity is invalid",
            ));
        }
        Ok(())
    }

    fn into_engine(self, definition: Arc<GameDefinition>) -> Result<Engine, CheckpointError> {
        Ok(Engine {
            definition,
            world: self.world.try_into()?,
            rng: DeterministicRng::from_checkpoint_state(self.rng_state.0),
            initial_events: self.initial_events,
            pending_durable_effects: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecimalU32(u32);

impl Serialize for DecimalU32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalU32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(serde::de::Error::custom("expected canonical decimal u32"));
        }
        value
            .parse::<u32>()
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum EcologyCheckpointV3 {
    SlotLifecycle {
        sites: BTreeMap<String, EcologySiteCheckpointV3>,
    },
}

impl EcologyCheckpointV3 {
    fn into_sites(self) -> BTreeMap<String, EcologySiteCheckpointV3> {
        match self {
            Self::SlotLifecycle { sites } => sites,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EcologyMemberSlotCheckpointV3 {
    member_id: String,
    location: WorldPosition,
    actor_id: Option<ActorId>,
    due_at: Option<LogicalTime>,
}

impl From<&EcologyMemberSlotState> for EcologyMemberSlotCheckpointV3 {
    fn from(value: &EcologyMemberSlotState) -> Self {
        Self {
            member_id: value.member_id.clone(),
            location: value.location.clone(),
            actor_id: value.actor_id.clone(),
            due_at: value.due_at,
        }
    }
}

impl From<EcologyMemberSlotCheckpointV3> for EcologyMemberSlotState {
    fn from(value: EcologyMemberSlotCheckpointV3) -> Self {
        Self {
            member_id: value.member_id,
            location: value.location,
            actor_id: value.actor_id,
            due_at: value.due_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EcologySiteCheckpointV3 {
    id: String,
    spawn_group_id: String,
    generation: u32,
    member_slots: BTreeMap<String, EcologyMemberSlotCheckpointV3>,
    full_clear_due_at: Option<LogicalTime>,
}

impl From<&EcologySiteState> for EcologySiteCheckpointV3 {
    fn from(value: &EcologySiteState) -> Self {
        Self {
            id: value.id.clone(),
            spawn_group_id: value.spawn_group_id.clone(),
            generation: value.generation,
            member_slots: value
                .member_slots
                .iter()
                .map(|(key, slot)| (key.clone(), slot.into()))
                .collect(),
            full_clear_due_at: value.full_clear_due_at,
        }
    }
}

impl From<EcologySiteCheckpointV3> for EcologySiteState {
    fn from(value: EcologySiteCheckpointV3) -> Self {
        Self {
            id: value.id,
            spawn_group_id: value.spawn_group_id,
            generation: value.generation,
            member_slots: value
                .member_slots
                .into_iter()
                .map(|(key, slot)| (key, slot.into()))
                .collect(),
            full_clear_due_at: value.full_clear_due_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldCheckpointV3 {
    timing: WorldTimingCheckpointV2,
    actors: Vec<ActorCheckpointV2>,
    ecology: EcologyCheckpointV3,
    social_relations: SocialRelationsCheckpointV3,
    groups: Vec<GroupState>,
    group_invitations: Vec<GroupInvitationState>,
    player_follow_targets: Vec<CharacterFollowCheckpointV2>,
    communication_preferences: BTreeMap<CharacterId, CommunicationPreferences>,
    character_presence: BTreeMap<CharacterId, CharacterPresenceState>,
    defeat_contributions: Vec<DefeatContributionCheckpointV2>,
    item_instances: BTreeMap<String, ItemInstanceCheckpointV2>,
    service_instances: Vec<ServiceInstanceCheckpointV2>,
    merchant_inventories: Vec<MerchantInventoryCheckpointV2>,
    banks: BTreeMap<BankId, BankCheckpointV2>,
    locker_vaults: BTreeMap<LockerVaultId, LockerCheckpointV2>,
    item_offers: BTreeMap<String, ItemOfferCheckpointV2>,
    quest_states: QuestStateLedger,
    ground_items: Vec<GroundItemCheckpointV2>,
    corpses: BTreeMap<CorpseId, CorpseCheckpointV2>,
    ground_gold: BTreeMap<GoldPileId, GroundGoldCheckpointV2>,
    next_corpse_sequence: u64,
    next_gold_sequence: u64,
    next_summon_sequence: u32,
    next_group_sequence: u64,
    next_group_invite_sequence: u64,
    next_membership_epoch: u64,
    next_player_kill_sequence: u64,
    linked_player_kill_karma: Vec<LinkedPlayerKillKarmaV1>,
    tile_effects: Vec<TileEffectCheckpointV3>,
    item_enchantments: Vec<ItemEnchantmentCheckpointV2>,
    portal_transitions: Vec<PortalCheckpointV2>,
    concealed_transitions: Vec<ConcealedCheckpointV2>,
    hidden_transition_revealed: Vec<PositionBoolCheckpointV2>,
    door_states: Vec<PositionBoolCheckpointV2>,
}

impl From<&World> for WorldCheckpointV3 {
    fn from(world: &World) -> Self {
        Self {
            timing: (&world.timing).into(),
            actors: world.actors.iter().map(Into::into).collect(),
            ecology: EcologyCheckpointV3::SlotLifecycle {
                sites: world
                    .ecology_sites
                    .iter()
                    .map(|(key, value)| (key.clone(), value.into()))
                    .collect(),
            },
            social_relations: (&world.social_relations).into(),
            groups: world.groups.values().cloned().collect(),
            group_invitations: world.group_invitations.values().cloned().collect(),
            player_follow_targets: world
                .player_follow_targets
                .iter()
                .map(
                    |(follower_character_id, target_character_id)| CharacterFollowCheckpointV2 {
                        follower_character_id: follower_character_id.clone(),
                        target_character_id: target_character_id.clone(),
                    },
                )
                .collect(),
            communication_preferences: world.communication_preferences.clone(),
            character_presence: world.character_presence.clone(),
            defeat_contributions: world
                .defeat_contributions
                .iter()
                .map(|(target_actor_id, ledger)| {
                    DefeatContributionCheckpointV2::new(target_actor_id, ledger)
                })
                .collect(),
            item_instances: world
                .item_instances
                .iter()
                .map(|(key, value)| (key.clone(), value.into()))
                .collect(),
            service_instances: world.service_instances.iter().map(Into::into).collect(),
            merchant_inventories: world
                .merchant_inventories
                .iter()
                .map(|(id, state)| MerchantInventoryCheckpointV2::new(id, state))
                .collect(),
            banks: world
                .banks
                .iter()
                .map(|(key, value)| (key.clone(), value.into()))
                .collect(),
            locker_vaults: world
                .locker_vaults
                .iter()
                .map(|(key, value)| (key.clone(), value.into()))
                .collect(),
            item_offers: world
                .item_offers
                .iter()
                .map(|(key, value)| (key.clone(), value.into()))
                .collect(),
            quest_states: world.quest_states.clone(),
            ground_items: world.ground_items.iter().map(Into::into).collect(),
            corpses: world
                .corpses
                .iter()
                .map(|(key, value)| (key.clone(), value.into()))
                .collect(),
            ground_gold: world
                .ground_gold
                .iter()
                .map(|(key, value)| (key.clone(), value.into()))
                .collect(),
            next_corpse_sequence: world.next_corpse_sequence,
            next_gold_sequence: world.next_gold_sequence,
            next_summon_sequence: world.next_summon_sequence,
            next_group_sequence: world.next_group_sequence,
            next_group_invite_sequence: world.next_group_invite_sequence,
            next_membership_epoch: world.next_membership_epoch,
            next_player_kill_sequence: world.next_player_kill_sequence,
            linked_player_kill_karma: world.linked_player_kill_karma.clone(),
            tile_effects: world.tile_effects.iter().map(Into::into).collect(),
            item_enchantments: world.item_enchantments.iter().map(Into::into).collect(),
            portal_transitions: world.portal_transitions.iter().map(Into::into).collect(),
            concealed_transitions: world.concealed_transitions.iter().map(Into::into).collect(),
            hidden_transition_revealed: sorted_position_bools(&world.hidden_transition_revealed),
            door_states: sorted_position_bools(&world.door_states),
        }
    }
}

impl TryFrom<WorldCheckpointV3> for World {
    type Error = CheckpointError;

    fn try_from(value: WorldCheckpointV3) -> Result<Self, Self::Error> {
        let merchant_inventories = value
            .merchant_inventories
            .into_iter()
            .map(MerchantInventoryCheckpointV2::into_pair)
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let groups = value
            .groups
            .into_iter()
            .map(|group| (group.id, group))
            .collect::<BTreeMap<_, _>>();
        let group_invitations = value
            .group_invitations
            .into_iter()
            .map(|invitation| (invitation.id, invitation))
            .collect::<BTreeMap<_, _>>();
        let player_follow_targets = value
            .player_follow_targets
            .into_iter()
            .map(|follow| (follow.follower_character_id, follow.target_character_id))
            .collect::<BTreeMap<_, _>>();
        let defeat_contributions = value
            .defeat_contributions
            .into_iter()
            .map(DefeatContributionCheckpointV2::into_pair)
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self {
            timing: value.timing.into(),
            actors: value.actors.into_iter().map(Into::into).collect(),
            ecology_sites: value
                .ecology
                .into_sites()
                .into_iter()
                .map(|(key, item)| (key, item.into()))
                .collect(),
            social_relations: value.social_relations.into(),
            groups,
            group_invitations,
            player_follow_targets,
            communication_preferences: value.communication_preferences,
            character_presence: value.character_presence,
            defeat_contributions,
            item_instances: value
                .item_instances
                .into_iter()
                .map(|(key, item)| (key, item.into()))
                .collect(),
            service_instances: value
                .service_instances
                .into_iter()
                .map(Into::into)
                .collect(),
            merchant_inventories,
            banks: value
                .banks
                .into_iter()
                .map(|(key, item)| (key, item.into()))
                .collect(),
            locker_vaults: value
                .locker_vaults
                .into_iter()
                .map(|(key, item)| (key, item.into()))
                .collect(),
            item_offers: value
                .item_offers
                .into_iter()
                .map(|(key, item)| (key, item.into()))
                .collect(),
            quest_states: value.quest_states,
            ground_items: value.ground_items.into_iter().map(Into::into).collect(),
            corpses: value
                .corpses
                .into_iter()
                .map(|(key, item)| (key, item.into()))
                .collect(),
            ground_gold: value
                .ground_gold
                .into_iter()
                .map(|(key, item)| (key, item.into()))
                .collect(),
            next_corpse_sequence: value.next_corpse_sequence,
            next_gold_sequence: value.next_gold_sequence,
            next_summon_sequence: value.next_summon_sequence,
            next_group_sequence: value.next_group_sequence,
            next_group_invite_sequence: value.next_group_invite_sequence,
            next_membership_epoch: value.next_membership_epoch,
            next_player_kill_sequence: value.next_player_kill_sequence,
            linked_player_kill_karma: value.linked_player_kill_karma,
            tile_effects: value.tile_effects.into_iter().map(Into::into).collect(),
            item_enchantments: value
                .item_enchantments
                .into_iter()
                .map(Into::into)
                .collect(),
            portal_transitions: value
                .portal_transitions
                .into_iter()
                .map(Into::into)
                .collect(),
            concealed_transitions: value
                .concealed_transitions
                .into_iter()
                .map(Into::into)
                .collect(),
            hidden_transition_revealed: position_bools(value.hidden_transition_revealed)?,
            door_states: position_bools(value.door_states)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CharacterFollowCheckpointV2 {
    follower_character_id: CharacterId,
    target_character_id: CharacterId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefeatRewardUnitCheckpointV2 {
    reward_unit_id: DefeatRewardUnitId,
    slices: Vec<DefeatContributionSliceCheckpointV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefeatContributionSliceCheckpointV2 {
    key: DefeatContributionKey,
    damage: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefeatContributionCheckpointV2 {
    target_actor_id: ActorId,
    total_actual_damage: u64,
    reward_units: Vec<DefeatRewardUnitCheckpointV2>,
}

impl DefeatContributionCheckpointV2 {
    fn new(target_actor_id: &ActorId, ledger: &DefeatContributionLedger) -> Self {
        Self {
            target_actor_id: target_actor_id.clone(),
            total_actual_damage: ledger.total_actual_damage,
            reward_units: ledger
                .reward_units
                .iter()
                .map(
                    |(reward_unit_id, contribution)| DefeatRewardUnitCheckpointV2 {
                        reward_unit_id: reward_unit_id.clone(),
                        slices: contribution
                            .slices
                            .iter()
                            .map(|(key, damage)| DefeatContributionSliceCheckpointV2 {
                                key: key.clone(),
                                damage: *damage,
                            })
                            .collect(),
                    },
                )
                .collect(),
        }
    }

    fn into_pair(self) -> Result<(ActorId, DefeatContributionLedger), CheckpointError> {
        let expected_len = self.reward_units.len();
        let reward_units = self
            .reward_units
            .into_iter()
            .map(|unit| {
                let expected_slice_len = unit.slices.len();
                let slices = unit
                    .slices
                    .into_iter()
                    .map(|slice| (slice.key, slice.damage))
                    .collect::<BTreeMap<_, _>>();
                if slices.len() != expected_slice_len {
                    return Err(CheckpointError::new(
                        "checkpoint contains duplicate defeat contribution slices",
                    ));
                }
                Ok((unit.reward_unit_id, DefeatRewardUnitContribution { slices }))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        if reward_units.len() != expected_len {
            return Err(CheckpointError::new(
                "checkpoint contains duplicate defeat reward units",
            ));
        }
        Ok((
            self.target_actor_id,
            DefeatContributionLedger {
                total_actual_damage: self.total_actual_damage,
                reward_units,
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldTimingCheckpointV2 {
    now: LogicalTime,
    next_tie_break_order: u64,
}

impl From<&WorldTimingState> for WorldTimingCheckpointV2 {
    fn from(value: &WorldTimingState) -> Self {
        Self {
            now: value.now,
            next_tie_break_order: value.next_tie_break_order,
        }
    }
}

impl From<WorldTimingCheckpointV2> for WorldTimingState {
    fn from(value: WorldTimingCheckpointV2) -> Self {
        Self {
            now: value.now,
            next_tie_break_order: value.next_tie_break_order,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorCheckpointV2 {
    id: ActorId,
    definition_id: String,
    kind: ActorKind,
    creature_traits: Vec<CreatureTrait>,
    social: SocialProfile,
    name: String,
    location: WorldPosition,
    home_location: WorldPosition,
    stats: Stats,
    magic_resistance: MagicResistanceCheckpointV2,
    physical_damage_affinity_profile_id: String,
    physical_damage_affinity: PhysicalAffinityCheckpointV2,
    hp: i32,
    mp: i32,
    stamina: i32,
    life_state: ActorLifeState,
    corpse_disposition: CorpseDisposition,
    resource_activity: ResourceActivityCheckpointV2,
    timing: ActorTimingCheckpointV2,
    attack_ready_at: LogicalTime,
    carried: CarriedCheckpointV2,
    ai: Option<AiCheckpointV3>,
    npc: Option<NpcCheckpointV2>,
    xp_value: i32,
    character_id: Option<CharacterId>,
    character: Option<CharacterSheetV1>,
    active_effects: Vec<ActiveEffectState>,
    balm_effect: Option<BalmCheckpointV2>,
    warmed_spell: Option<WarmedSpellState>,
    monster_abilities: Vec<MonsterAbilityCheckpointV2>,
    summoned: Option<SummonedActorState>,
    ecology_origin: Option<EcologyOriginCheckpointV2>,
}

impl From<&ActorState> for ActorCheckpointV2 {
    fn from(value: &ActorState) -> Self {
        Self {
            id: value.id.clone(),
            definition_id: value.definition_id.clone(),
            kind: value.kind,
            creature_traits: value.creature_traits.clone(),
            social: value.social.clone(),
            name: value.name.clone(),
            location: value.location.clone(),
            home_location: value.home_location.clone(),
            stats: value.stats.clone(),
            magic_resistance: (&value.magic_resistance).into(),
            physical_damage_affinity_profile_id: value.physical_damage_affinity_profile_id.clone(),
            physical_damage_affinity: (&value.physical_damage_affinity).into(),
            hp: value.hp,
            mp: value.mp,
            stamina: value.stamina,
            life_state: value.life_state.clone(),
            corpse_disposition: value.corpse_disposition,
            resource_activity: (&value.resource_activity).into(),
            timing: (&value.timing).into(),
            attack_ready_at: value.attack_ready_at,
            carried: (&value.carried).into(),
            ai: value.ai.as_ref().map(Into::into),
            npc: value.npc.as_ref().map(Into::into),
            xp_value: value.xp_value,
            character_id: value.character_id.clone(),
            character: value.character.clone(),
            active_effects: value.active_effects.clone(),
            balm_effect: value.balm_effect.as_ref().map(Into::into),
            warmed_spell: value.warmed_spell.clone(),
            monster_abilities: value.monster_abilities.iter().map(Into::into).collect(),
            summoned: value.summoned.clone(),
            ecology_origin: value.ecology_origin.as_ref().map(Into::into),
        }
    }
}

impl From<ActorCheckpointV2> for ActorState {
    fn from(value: ActorCheckpointV2) -> Self {
        Self {
            id: value.id,
            definition_id: value.definition_id,
            kind: value.kind,
            creature_traits: value.creature_traits,
            social: value.social,
            name: value.name,
            location: value.location,
            home_location: value.home_location,
            stats: value.stats,
            magic_resistance: value.magic_resistance.into(),
            physical_damage_affinity_profile_id: value.physical_damage_affinity_profile_id,
            physical_damage_affinity: value.physical_damage_affinity.into(),
            hp: value.hp,
            mp: value.mp,
            stamina: value.stamina,
            life_state: value.life_state,
            corpse_disposition: value.corpse_disposition,
            resource_activity: value.resource_activity.into(),
            timing: value.timing.into(),
            attack_ready_at: value.attack_ready_at,
            carried: value.carried.into(),
            ai: value.ai.map(Into::into),
            npc: value.npc.map(Into::into),
            xp_value: value.xp_value,
            character_id: value.character_id,
            character: value.character,
            active_effects: value.active_effects,
            balm_effect: value.balm_effect.map(Into::into),
            warmed_spell: value.warmed_spell,
            monster_abilities: value
                .monster_abilities
                .into_iter()
                .map(Into::into)
                .collect(),
            summoned: value.summoned,
            ecology_origin: value.ecology_origin.map(Into::into),
        }
    }
}

macro_rules! copy_checkpoint {
    ($checkpoint:ident, $runtime:ty, { $($field:ident : $kind:ty),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct $checkpoint { $( $field: $kind, )+ }

        impl From<&$runtime> for $checkpoint {
            fn from(value: &$runtime) -> Self {
                Self { $( $field: value.$field.clone(), )+ }
            }
        }

        impl From<$checkpoint> for $runtime {
            fn from(value: $checkpoint) -> Self {
                Self { $( $field: value.$field, )+ }
            }
        }
    };
}

copy_checkpoint!(MagicResistanceCheckpointV2, ActorMagicResistanceState, {
    natural_save_twentieths: u32,
    evidence_state: MagicRuleEvidenceState,
});
copy_checkpoint!(PhysicalAffinityCheckpointV2, PhysicalDamageAffinity, {
    cutting_numerator: u32,
    cutting_denominator: u32,
    piercing_numerator: u32,
    piercing_denominator: u32,
    crushing_numerator: u32,
    crushing_denominator: u32,
});
copy_checkpoint!(ResourceActivityCheckpointV2, ActorResourceActivity, {
    last_active_at: Option<LogicalTime>,
});
copy_checkpoint!(ActorTimingCheckpointV2, ActorTimingState, {
    ready_at: LogicalTime,
    tie_break_order: u64,
});
copy_checkpoint!(EcologyOriginCheckpointV2, EcologyActorOrigin, {
    site_id: String,
    member_id: String,
    generation: u32,
});
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemKnowledgeCheckpointV2 {
    identified: bool,
    appraised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemInstanceCheckpointV2 {
    definition_id: String,
    quantity: u32,
    knowledge: ItemKnowledgeCheckpointV2,
    binding: ItemBindingState,
    bow_readiness: Option<BowReadiness>,
}

impl From<&ItemInstanceState> for ItemInstanceCheckpointV2 {
    fn from(value: &ItemInstanceState) -> Self {
        Self {
            definition_id: value.definition_id.clone(),
            quantity: value.quantity,
            knowledge: ItemKnowledgeCheckpointV2 {
                identified: value.knowledge.identified,
                appraised: value.knowledge.appraised,
            },
            binding: value.binding.clone(),
            bow_readiness: value.bow_readiness,
        }
    }
}

impl From<ItemInstanceCheckpointV2> for ItemInstanceState {
    fn from(value: ItemInstanceCheckpointV2) -> Self {
        Self {
            definition_id: value.definition_id,
            quantity: value.quantity,
            knowledge: ItemKnowledgeState {
                identified: value.knowledge.identified,
                appraised: value.knowledge.appraised,
            },
            binding: value.binding,
            bow_readiness: value.bow_readiness,
        }
    }
}
copy_checkpoint!(ServiceInstanceCheckpointV2, ServiceInstanceState, {
    id: String,
    definition_id: String,
    position: WorldPosition,
});
copy_checkpoint!(BankCheckpointV2, BankState, {
    balances: BTreeMap<CharacterId, i64>,
});
copy_checkpoint!(LockerCheckpointV2, LockerVaultState, {
    lockers: BTreeMap<CharacterId, Vec<String>>,
});
copy_checkpoint!(ItemOfferCheckpointV2, ItemOfferState, {
    sender_character_id: CharacterId,
    recipient_character_id: CharacterId,
    source_position: CarriedPosition,
});
copy_checkpoint!(GroundItemCheckpointV2, GroundItem, {
    item_instance_id: String,
    location: WorldPosition,
    loot_claim: Option<LootClaim>,
});
copy_checkpoint!(CorpseCheckpointV2, CorpseState, {
    id: CorpseId,
    origin_actor_id: ActorId,
    origin_character_id: Option<CharacterId>,
    origin_kind: ActorKind,
    origin_name: String,
    location: WorldPosition,
    created_at: LogicalTime,
    sequence: u64,
    searched: bool,
    loot_claim: Option<LootClaim>,
    contents: BTreeMap<CarriedPosition, String>,
    gold: i64,
});
copy_checkpoint!(GroundGoldCheckpointV2, GroundGoldPile, {
    id: GoldPileId,
    amount: i64,
    location: WorldPosition,
    loot_claim: Option<LootClaim>,
});
copy_checkpoint!(TileEffectCheckpointV3, TileEffectState, {
    instance_id: String,
    effect_id: String,
    source: ActiveEffectSource,
    source_actor_id: Option<ActorId>,
    hostile_authority: Option<HostileEffectAuthority>,
    location: WorldPosition,
    kind: String,
    tags: Vec<String>,
    potency: i32,
    remaining_rounds: Option<u32>,
    passability: Option<String>,
    sight: Option<String>,
    hazard: Option<String>,
    move_cost: Option<i32>,
    tick_interval_rounds: u32,
    last_ticked_at: LogicalTime,
});
copy_checkpoint!(ItemEnchantmentCheckpointV2, ItemEnchantmentState, {
    enchantment_instance_id: String,
    source: ItemOperationSource,
    item_instance_id: String,
    combat_add_rating_bonus: i32,
    tags: Vec<String>,
    remaining_rounds: Option<u32>,
    last_ticked_at: LogicalTime,
});
copy_checkpoint!(PortalCheckpointV2, PortalTransitionState, {
    instance_id: String,
    source_spell_id: String,
    source_actor_id: ActorId,
    location: WorldPosition,
    target: WorldPosition,
    two_way: bool,
    remaining_rounds: Option<u32>,
    last_ticked_at: LogicalTime,
});
copy_checkpoint!(ConcealedCheckpointV2, ConcealedTransitionState, {
    instance_id: String,
    source_spell_id: String,
    source_actor_id: ActorId,
    location: WorldPosition,
    remaining_rounds: u32,
    last_ticked_at: LogicalTime,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BalmCheckpointV2 {
    heal_per_round: i32,
    restored: i32,
    budget: i32,
    last_tick_at: LogicalTime,
}

impl From<&BalmEffectState> for BalmCheckpointV2 {
    fn from(value: &BalmEffectState) -> Self {
        Self {
            heal_per_round: value.heal_per_round,
            restored: value.restored,
            budget: value.budget,
            last_tick_at: value.last_tick_at,
        }
    }
}

impl From<BalmCheckpointV2> for BalmEffectState {
    fn from(value: BalmCheckpointV2) -> Self {
        Self {
            heal_per_round: value.heal_per_round,
            restored: value.restored,
            budget: value.budget,
            last_tick_at: value.last_tick_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CarriedCheckpointV2 {
    items: BTreeMap<CarriedPosition, String>,
    gold: CarriedGold,
}

impl From<&CarriedLayout> for CarriedCheckpointV2 {
    fn from(value: &CarriedLayout) -> Self {
        Self {
            items: value.items.clone(),
            gold: value.gold,
        }
    }
}

impl From<CarriedCheckpointV2> for CarriedLayout {
    fn from(value: CarriedCheckpointV2) -> Self {
        Self {
            items: value.items,
            gold: value.gold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum AwarenessPolicyCheckpointV2 {
    Unrestricted,
    LineOfSightMemory { memory_opportunities: u32 },
}

impl From<ActorAwarenessPolicy> for AwarenessPolicyCheckpointV2 {
    fn from(value: ActorAwarenessPolicy) -> Self {
        match value {
            ActorAwarenessPolicy::Unrestricted => Self::Unrestricted,
            ActorAwarenessPolicy::LineOfSightMemory {
                memory_opportunities,
            } => Self::LineOfSightMemory {
                memory_opportunities,
            },
        }
    }
}

impl From<AwarenessPolicyCheckpointV2> for ActorAwarenessPolicy {
    fn from(value: AwarenessPolicyCheckpointV2) -> Self {
        match value {
            AwarenessPolicyCheckpointV2::Unrestricted => Self::Unrestricted,
            AwarenessPolicyCheckpointV2::LineOfSightMemory {
                memory_opportunities,
            } => Self::LineOfSightMemory {
                memory_opportunities,
            },
        }
    }
}

copy_checkpoint!(RememberedCheckpointV2, RememberedHostile, {
    actor_id: ActorId,
    last_seen: WorldPosition,
    remaining_opportunities: u32,
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AiCheckpointV3 {
    behavior: ActorAiBehavior,
    cadence_units: u32,
    aggro_radius: u32,
    leash_range: u32,
    policy: AwarenessPolicyCheckpointV2,
    remembered: Option<RememberedCheckpointV2>,
    physical_attack_modes: Vec<PhysicalAttackMode>,
    returning_home: bool,
}

impl From<&ActorAiState> for AiCheckpointV3 {
    fn from(value: &ActorAiState) -> Self {
        Self {
            behavior: value.behavior,
            cadence_units: value.cadence_units,
            aggro_radius: value.aggro_radius,
            leash_range: value.leash_range,
            policy: value.awareness.policy.into(),
            remembered: value.awareness.remembered.as_ref().map(Into::into),
            physical_attack_modes: value.physical_attack_modes.clone(),
            returning_home: value.returning_home,
        }
    }
}

impl From<AiCheckpointV3> for ActorAiState {
    fn from(value: AiCheckpointV3) -> Self {
        Self {
            behavior: value.behavior,
            cadence_units: value.cadence_units,
            aggro_radius: value.aggro_radius,
            leash_range: value.leash_range,
            awareness: ActorAwarenessState {
                policy: value.policy.into(),
                remembered: value.remembered.map(Into::into),
            },
            physical_attack_modes: value.physical_attack_modes,
            returning_home: value.returning_home,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcCheckpointV2 {
    follow_cadence_units: u32,
    interactions: Vec<NpcInteractionCheckpointV2>,
    following_character_id: Option<CharacterId>,
}

impl From<&NpcState> for NpcCheckpointV2 {
    fn from(value: &NpcState) -> Self {
        Self {
            follow_cadence_units: value.follow_cadence_units,
            interactions: value.interactions.iter().map(Into::into).collect(),
            following_character_id: value.following_character_id.clone(),
        }
    }
}

impl From<NpcCheckpointV2> for NpcState {
    fn from(value: NpcCheckpointV2) -> Self {
        Self {
            follow_cadence_units: value.follow_cadence_units,
            interactions: value.interactions.into_iter().map(Into::into).collect(),
            following_character_id: value.following_character_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcInteractionCheckpointV2 {
    transaction: TransactionCheckpointV2,
    response: String,
    outcome: NpcInteractionOutcome,
}

impl From<&NpcInteraction> for NpcInteractionCheckpointV2 {
    fn from(value: &NpcInteraction) -> Self {
        Self {
            transaction: (&value.transaction).into(),
            response: value.response.clone(),
            outcome: value.outcome.clone(),
        }
    }
}

impl From<NpcInteractionCheckpointV2> for NpcInteraction {
    fn from(value: NpcInteractionCheckpointV2) -> Self {
        Self {
            transaction: value.transaction.into(),
            response: value.response,
            outcome: value.outcome,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RequirementCheckpointV2 {
    CurrentClass {
        class_id: String,
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
        track_id: String,
        level: u8,
    },
    MinimumCarriedGold {
        amount: i64,
    },
    CarriedItem {
        item_definition_id: String,
        quantity: u32,
    },
    CarriedPositionEmpty {
        position: CarriedPosition,
    },
    SpellUnknown {
        spell_id: String,
    },
    QuestUnstarted {
        quest_id: QuestId,
    },
    QuestAtStage {
        quest_id: QuestId,
        stage_id: QuestStageId,
    },
    NpcAccompanying {
        npc_actor_id: ActorId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CostCheckpointV2 {
    CarriedGold { amount: i64 },
    SelectedCarriedItem { quantity: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RewardCheckpointV2 {
    Experience {
        amount: i32,
    },
    Item {
        item_instance_id: String,
        item_definition_id: String,
        position: CarriedPosition,
    },
    Class {
        to_class_id: String,
        to_class_display: String,
    },
    Spell {
        spell_id: String,
    },
    QuestStage {
        quest_id: QuestId,
        stage_id: QuestStageId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionCheckpointV2 {
    id: String,
    label: String,
    requirements: Vec<RequirementCheckpointV2>,
    costs: Vec<CostCheckpointV2>,
    rewards: Vec<RewardCheckpointV2>,
}

impl From<&Transaction> for TransactionCheckpointV2 {
    fn from(value: &Transaction) -> Self {
        Self {
            id: value.id.clone(),
            label: value.label.clone(),
            requirements: value
                .requirements
                .iter()
                .map(requirement_to_checkpoint)
                .collect(),
            costs: value.costs.iter().map(cost_to_checkpoint).collect(),
            rewards: value.rewards.iter().map(reward_to_checkpoint).collect(),
        }
    }
}

impl From<TransactionCheckpointV2> for Transaction {
    fn from(value: TransactionCheckpointV2) -> Self {
        Self {
            id: value.id,
            label: value.label,
            requirements: value.requirements.into_iter().map(Into::into).collect(),
            costs: value.costs.into_iter().map(Into::into).collect(),
            rewards: value.rewards.into_iter().map(Into::into).collect(),
        }
    }
}

fn requirement_to_checkpoint(value: &TransactionRequirement) -> RequirementCheckpointV2 {
    match value {
        TransactionRequirement::CurrentClass { class_id } => {
            RequirementCheckpointV2::CurrentClass {
                class_id: class_id.clone(),
            }
        }
        TransactionRequirement::MinimumLevel { level } => {
            RequirementCheckpointV2::MinimumLevel { level: *level }
        }
        TransactionRequirement::ExactKarma { karma_points } => {
            RequirementCheckpointV2::ExactKarma {
                karma_points: *karma_points,
            }
        }
        TransactionRequirement::ExactAlignment { alignment } => {
            RequirementCheckpointV2::ExactAlignment {
                alignment: *alignment,
            }
        }
        TransactionRequirement::MinimumSkillLevel { track_id, level } => {
            RequirementCheckpointV2::MinimumSkillLevel {
                track_id: track_id.clone(),
                level: *level,
            }
        }
        TransactionRequirement::MinimumCarriedGold { amount } => {
            RequirementCheckpointV2::MinimumCarriedGold { amount: *amount }
        }
        TransactionRequirement::CarriedItem {
            item_definition_id,
            quantity,
        } => RequirementCheckpointV2::CarriedItem {
            item_definition_id: item_definition_id.clone(),
            quantity: *quantity,
        },
        TransactionRequirement::CarriedPositionEmpty { position } => {
            RequirementCheckpointV2::CarriedPositionEmpty {
                position: *position,
            }
        }
        TransactionRequirement::SpellUnknown { spell_id } => {
            RequirementCheckpointV2::SpellUnknown {
                spell_id: spell_id.clone(),
            }
        }
        TransactionRequirement::QuestUnstarted { quest_id } => {
            RequirementCheckpointV2::QuestUnstarted {
                quest_id: quest_id.clone(),
            }
        }
        TransactionRequirement::QuestAtStage { quest_id, stage_id } => {
            RequirementCheckpointV2::QuestAtStage {
                quest_id: quest_id.clone(),
                stage_id: stage_id.clone(),
            }
        }
        TransactionRequirement::NpcAccompanying { npc_actor_id } => {
            RequirementCheckpointV2::NpcAccompanying {
                npc_actor_id: npc_actor_id.clone(),
            }
        }
    }
}

impl From<RequirementCheckpointV2> for TransactionRequirement {
    fn from(value: RequirementCheckpointV2) -> Self {
        match value {
            RequirementCheckpointV2::CurrentClass { class_id } => Self::CurrentClass { class_id },
            RequirementCheckpointV2::MinimumLevel { level } => Self::MinimumLevel { level },
            RequirementCheckpointV2::ExactKarma { karma_points } => {
                Self::ExactKarma { karma_points }
            }
            RequirementCheckpointV2::ExactAlignment { alignment } => {
                Self::ExactAlignment { alignment }
            }
            RequirementCheckpointV2::MinimumSkillLevel { track_id, level } => {
                Self::MinimumSkillLevel { track_id, level }
            }
            RequirementCheckpointV2::MinimumCarriedGold { amount } => {
                Self::MinimumCarriedGold { amount }
            }
            RequirementCheckpointV2::CarriedItem {
                item_definition_id,
                quantity,
            } => Self::CarriedItem {
                item_definition_id,
                quantity,
            },
            RequirementCheckpointV2::CarriedPositionEmpty { position } => {
                Self::CarriedPositionEmpty { position }
            }
            RequirementCheckpointV2::SpellUnknown { spell_id } => Self::SpellUnknown { spell_id },
            RequirementCheckpointV2::QuestUnstarted { quest_id } => {
                Self::QuestUnstarted { quest_id }
            }
            RequirementCheckpointV2::QuestAtStage { quest_id, stage_id } => {
                Self::QuestAtStage { quest_id, stage_id }
            }
            RequirementCheckpointV2::NpcAccompanying { npc_actor_id } => {
                Self::NpcAccompanying { npc_actor_id }
            }
        }
    }
}

fn cost_to_checkpoint(value: &TransactionCost) -> CostCheckpointV2 {
    match value {
        TransactionCost::CarriedGold { amount } => {
            CostCheckpointV2::CarriedGold { amount: *amount }
        }
        TransactionCost::SelectedCarriedItem { quantity } => {
            CostCheckpointV2::SelectedCarriedItem {
                quantity: *quantity,
            }
        }
    }
}

impl From<CostCheckpointV2> for TransactionCost {
    fn from(value: CostCheckpointV2) -> Self {
        match value {
            CostCheckpointV2::CarriedGold { amount } => Self::CarriedGold { amount },
            CostCheckpointV2::SelectedCarriedItem { quantity } => {
                Self::SelectedCarriedItem { quantity }
            }
        }
    }
}

fn reward_to_checkpoint(value: &TransactionReward) -> RewardCheckpointV2 {
    match value {
        TransactionReward::Experience { amount } => {
            RewardCheckpointV2::Experience { amount: *amount }
        }
        TransactionReward::Item {
            item_instance_id,
            item_definition_id,
            position,
        } => RewardCheckpointV2::Item {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            position: *position,
        },
        TransactionReward::Class {
            to_class_id,
            to_class_display,
        } => RewardCheckpointV2::Class {
            to_class_id: to_class_id.clone(),
            to_class_display: to_class_display.clone(),
        },
        TransactionReward::Spell { spell_id } => RewardCheckpointV2::Spell {
            spell_id: spell_id.clone(),
        },
        TransactionReward::QuestStage { quest_id, stage_id } => RewardCheckpointV2::QuestStage {
            quest_id: quest_id.clone(),
            stage_id: stage_id.clone(),
        },
    }
}

impl From<RewardCheckpointV2> for TransactionReward {
    fn from(value: RewardCheckpointV2) -> Self {
        match value {
            RewardCheckpointV2::Experience { amount } => Self::Experience { amount },
            RewardCheckpointV2::Item {
                item_instance_id,
                item_definition_id,
                position,
            } => Self::Item {
                item_instance_id,
                item_definition_id,
                position,
            },
            RewardCheckpointV2::Class {
                to_class_id,
                to_class_display,
            } => Self::Class {
                to_class_id,
                to_class_display,
            },
            RewardCheckpointV2::Spell { spell_id } => Self::Spell { spell_id },
            RewardCheckpointV2::QuestStage { quest_id, stage_id } => {
                Self::QuestStage { quest_id, stage_id }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AbilityKindCheckpointV2 {
    Spell,
    SpecialAttack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AbilityTargetCheckpointV2 {
    NearestHostile,
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MonsterAbilityCheckpointV2 {
    id: String,
    kind: AbilityKindCheckpointV2,
    spell_id: String,
    cooldown_rounds: u32,
    target_policy: AbilityTargetCheckpointV2,
    ready_at: LogicalTime,
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
struct SocialRelationsCheckpointV3 {
    self_defense: BTreeMap<CharacterId, SelfDefenseRightV1>,
    npc_grudges: BTreeSet<NpcGrudgeRelation>,
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
struct MerchantInventoryCheckpointV2 {
    service_id: String,
    capability_id: String,
    listings: Vec<MerchantListingCheckpointV2>,
}

impl MerchantInventoryCheckpointV2 {
    fn new(id: &MerchantInventoryId, state: &MerchantInventoryState) -> Self {
        Self {
            service_id: id.service_id.clone(),
            capability_id: id.capability_id.clone(),
            listings: state.listings.iter().map(Into::into).collect(),
        }
    }

    fn into_pair(self) -> Result<(MerchantInventoryId, MerchantInventoryState), CheckpointError> {
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
enum ListingOriginCheckpointV2 {
    AuthoredStock,
    PawnPool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MerchantListingCheckpointV2 {
    item_instance_id: String,
    origin: ListingOriginCheckpointV2,
    price_gold: i64,
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
struct PositionBoolCheckpointV2 {
    position: WorldPosition,
    value: bool,
}

fn sorted_position_bools(
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

fn position_bools(
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

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::setup::test_engine;

    #[test]
    fn checkpoint_round_trip_is_byte_exact_and_preserves_rng() {
        let mut engine = test_engine("first_room");
        let actor_id = engine
            .world()
            .controlled_actors()
            .next()
            .unwrap()
            .id
            .clone();
        let _ = engine
            .apply_actor_intent(&actor_id, PlayerIntent::Wait)
            .unwrap();
        let checkpoint = engine.export_checkpoint().unwrap();
        let hydrated =
            Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint).unwrap();
        assert_eq!(checkpoint, hydrated.export_checkpoint().unwrap());

        let mut expected = engine.clone();
        let mut actual = hydrated;
        assert_eq!(
            expected.apply_actor_intent(&actor_id, PlayerIntent::Wait),
            actual.apply_actor_intent(&actor_id, PlayerIntent::Wait)
        );
        assert_eq!(
            expected.export_checkpoint().unwrap(),
            actual.export_checkpoint().unwrap()
        );

        for case_id in [
            "world_topology_gallery",
            "town_adventure_loop_gallery",
            "magic_profession_gallery",
        ] {
            let representative = test_engine(case_id);
            let checkpoint = representative.export_checkpoint().unwrap();
            let hydrated =
                Engine::hydrate_checkpoint(representative.definition().clone(), &checkpoint)
                    .unwrap();
            assert_eq!(
                checkpoint,
                hydrated.export_checkpoint().unwrap(),
                "{case_id}"
            );
        }
    }

    #[test]
    fn checkpoint_rejects_noncanonical_and_content_mismatch() {
        let engine = test_engine("first_room");
        let checkpoint = engine.export_checkpoint().unwrap();
        let mut spaced = checkpoint.as_bytes().to_vec();
        spaced.insert(1, b' ');
        assert!(FacetCheckpointV4::from_bytes(spaced).is_err());

        let other = test_engine("ranged_attack");
        assert!(Engine::hydrate_checkpoint(other.definition().clone(), &checkpoint).is_err());
    }

    #[test]
    fn checkpoint_rejects_size_corruption_unknown_missing_and_wrong_schema() {
        assert!(FacetCheckpointV4::from_bytes(Vec::new()).is_err());
        assert!(FacetCheckpointV4::from_bytes(vec![b'x'; MAX_FACET_CHECKPOINT_BYTES + 1]).is_err());
        assert!(FacetCheckpointV4::from_bytes(b"not-json".to_vec()).is_err());

        let checkpoint = test_engine("first_room").export_checkpoint().unwrap();
        let mut unknown: serde_json::Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
        unknown["unknown"] = serde_json::json!(true);
        assert!(FacetCheckpointV4::from_bytes(serde_json::to_vec(&unknown).unwrap()).is_err());

        let mut missing: serde_json::Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
        missing.as_object_mut().unwrap().remove("rng_state");
        assert!(FacetCheckpointV4::from_bytes(serde_json::to_vec(&missing).unwrap()).is_err());

        let mut schema: serde_json::Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
        schema["schema_version"] = serde_json::json!(2);
        assert!(FacetCheckpointV4::from_bytes(serde_json::to_vec(&schema).unwrap()).is_err());
    }

    #[test]
    fn checkpoint_three_rejects_both_empty_and_nonempty_pre_slot_ecology_maps() {
        let checkpoint = test_engine("creature_ecology_gallery")
            .export_checkpoint()
            .unwrap();
        let current: serde_json::Value =
            serde_json::from_slice(checkpoint.as_bytes()).expect("checkpoint JSON");
        assert_eq!(current["world"]["ecology"]["kind"], "slot_lifecycle");
        assert!(
            current["world"]["ecology"]["sites"]
                .as_object()
                .expect("ecology sites")
                .contains_key("gallery_pack")
        );

        for old_sites in [
            current["world"]["ecology"]["sites"].clone(),
            serde_json::json!({}),
        ] {
            let mut old = current.clone();
            old["world"]
                .as_object_mut()
                .expect("checkpoint world")
                .remove("ecology");
            old["world"]["ecology_sites"] = old_sites;
            assert!(
                FacetCheckpointV4::from_bytes(serde_json::to_vec(&old).unwrap()).is_err(),
                "Checkpoint 3 must reject the retired bare ecology_sites shape"
            );
        }
    }

    #[test]
    fn checkpoint_hydration_rejects_broken_content_references_and_sequences() {
        let engine = test_engine("world_topology_gallery");
        let checkpoint = engine.export_checkpoint().unwrap();
        let payload: FacetCheckpointPayloadV1 =
            serde_json::from_slice(checkpoint.as_bytes()).unwrap();

        let mut unknown_location = payload.clone();
        unknown_location.world.actors[0].location.realm = "missing_realm".to_string();
        let unknown_location =
            FacetCheckpointV4::from_bytes(serde_json::to_vec(&unknown_location).unwrap()).unwrap();
        assert!(
            Engine::hydrate_checkpoint(engine.definition().clone(), &unknown_location).is_err()
        );

        let item_engine = test_engine("first_room");
        let mut unknown_item: FacetCheckpointPayloadV1 =
            serde_json::from_slice(item_engine.export_checkpoint().unwrap().as_bytes()).unwrap();
        unknown_item
            .world
            .item_instances
            .first_entry()
            .unwrap()
            .get_mut()
            .definition_id = "missing_item_definition".to_string();
        let unknown_item =
            FacetCheckpointV4::from_bytes(serde_json::to_vec(&unknown_item).unwrap()).unwrap();
        assert!(
            Engine::hydrate_checkpoint(item_engine.definition().clone(), &unknown_item).is_err()
        );

        let mut invalid_sequence = payload;
        invalid_sequence.world.next_gold_sequence = 0;
        let invalid_sequence =
            FacetCheckpointV4::from_bytes(serde_json::to_vec(&invalid_sequence).unwrap()).unwrap();
        assert!(
            Engine::hydrate_checkpoint(engine.definition().clone(), &invalid_sequence).is_err()
        );
    }
}
