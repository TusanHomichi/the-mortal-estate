use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FacetCheckpointPayloadV1 {
    pub(super) schema_version: u32,
    pub(super) kind: String,
    pub(super) content: ContentIdentityV1,
    pub(super) rng_state: DecimalU32,
    pub(super) world: WorldCheckpointV3,
    pub(super) initial_events: Vec<Event>,
}

impl FacetCheckpointPayloadV1 {
    pub(super) fn from_engine(engine: &Engine) -> Self {
        Self {
            schema_version: FACET_CHECKPOINT_SCHEMA_VERSION,
            kind: FACET_CHECKPOINT_KIND.to_string(),
            content: engine.definition.content_identity().clone(),
            rng_state: DecimalU32(engine.rng.checkpoint_state()),
            world: WorldCheckpointV3::from(&engine.world),
            initial_events: engine.initial_events.clone(),
        }
    }

    pub(super) fn validate_header(&self) -> Result<(), CheckpointError> {
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

    pub(super) fn into_engine(
        self,
        definition: Arc<GameDefinition>,
    ) -> Result<Engine, CheckpointError> {
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
pub(super) struct DecimalU32(u32);

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
pub(super) enum EcologyCheckpointV3 {
    SlotLifecycle {
        sites: BTreeMap<String, EcologySiteCheckpointV3>,
    },
}

impl EcologyCheckpointV3 {
    pub(super) fn into_sites(self) -> BTreeMap<String, EcologySiteCheckpointV3> {
        match self {
            Self::SlotLifecycle { sites } => sites,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EcologyMemberSlotCheckpointV3 {
    pub(super) member_id: String,
    pub(super) location: WorldPosition,
    pub(super) actor_id: Option<ActorId>,
    pub(super) due_at: Option<LogicalTime>,
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
pub(super) struct EcologySiteCheckpointV3 {
    pub(super) id: String,
    pub(super) spawn_group_id: String,
    pub(super) generation: u32,
    pub(super) member_slots: BTreeMap<String, EcologyMemberSlotCheckpointV3>,
    pub(super) full_clear_due_at: Option<LogicalTime>,
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
pub(super) struct WorldCheckpointV3 {
    pub(super) timing: WorldTimingCheckpointV2,
    pub(super) actors: Vec<ActorCheckpointV2>,
    pub(super) ecology: EcologyCheckpointV3,
    pub(super) social_relations: SocialRelationsCheckpointV3,
    pub(super) groups: Vec<GroupState>,
    pub(super) group_invitations: Vec<GroupInvitationState>,
    pub(super) player_follow_targets: Vec<CharacterFollowCheckpointV2>,
    pub(super) communication_preferences: BTreeMap<CharacterId, CommunicationPreferences>,
    pub(super) character_presence: BTreeMap<CharacterId, CharacterPresenceState>,
    pub(super) defeat_contributions: Vec<DefeatContributionCheckpointV2>,
    pub(super) item_instances: BTreeMap<String, ItemInstanceCheckpointV2>,
    pub(super) service_instances: Vec<ServiceInstanceCheckpointV2>,
    pub(super) merchant_inventories: Vec<MerchantInventoryCheckpointV2>,
    pub(super) banks: BTreeMap<BankId, BankCheckpointV2>,
    pub(super) locker_vaults: BTreeMap<LockerVaultId, LockerCheckpointV2>,
    pub(super) item_offers: BTreeMap<String, ItemOfferCheckpointV2>,
    pub(super) quest_states: QuestStateLedger,
    pub(super) ground_items: Vec<GroundItemCheckpointV2>,
    pub(super) corpses: BTreeMap<CorpseId, CorpseCheckpointV2>,
    pub(super) ground_gold: BTreeMap<GoldPileId, GroundGoldCheckpointV2>,
    pub(super) next_corpse_sequence: u64,
    pub(super) next_gold_sequence: u64,
    pub(super) next_summon_sequence: u32,
    pub(super) next_group_sequence: u64,
    pub(super) next_group_invite_sequence: u64,
    pub(super) next_membership_epoch: u64,
    pub(super) next_player_kill_sequence: u64,
    pub(super) linked_player_kill_karma: Vec<LinkedPlayerKillKarmaV1>,
    pub(super) tile_effects: Vec<TileEffectCheckpointV3>,
    pub(super) item_enchantments: Vec<ItemEnchantmentCheckpointV2>,
    pub(super) portal_transitions: Vec<PortalCheckpointV2>,
    pub(super) concealed_transitions: Vec<ConcealedCheckpointV2>,
    pub(super) hidden_transition_revealed: Vec<PositionBoolCheckpointV2>,
    pub(super) door_states: Vec<PositionBoolCheckpointV2>,
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
pub(super) struct CharacterFollowCheckpointV2 {
    pub(super) follower_character_id: CharacterId,
    pub(super) target_character_id: CharacterId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DefeatRewardUnitCheckpointV2 {
    pub(super) reward_unit_id: DefeatRewardUnitId,
    pub(super) slices: Vec<DefeatContributionSliceCheckpointV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DefeatContributionSliceCheckpointV2 {
    pub(super) key: DefeatContributionKey,
    pub(super) damage: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DefeatContributionCheckpointV2 {
    pub(super) target_actor_id: ActorId,
    pub(super) total_actual_damage: u64,
    pub(super) reward_units: Vec<DefeatRewardUnitCheckpointV2>,
}

impl DefeatContributionCheckpointV2 {
    pub(super) fn new(target_actor_id: &ActorId, ledger: &DefeatContributionLedger) -> Self {
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

    pub(super) fn into_pair(self) -> Result<(ActorId, DefeatContributionLedger), CheckpointError> {
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
pub(super) struct WorldTimingCheckpointV2 {
    pub(super) now: LogicalTime,
    pub(super) next_tie_break_order: u64,
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
pub(super) struct ActorCheckpointV2 {
    pub(super) id: ActorId,
    pub(super) definition_id: String,
    pub(super) kind: ActorKind,
    pub(super) creature_traits: Vec<CreatureTrait>,
    pub(super) social: SocialProfile,
    pub(super) name: String,
    pub(super) location: WorldPosition,
    pub(super) home_location: WorldPosition,
    pub(super) stats: Stats,
    pub(super) magic_resistance: MagicResistanceCheckpointV2,
    pub(super) physical_damage_affinity_profile_id: String,
    pub(super) physical_damage_affinity: PhysicalAffinityCheckpointV2,
    pub(super) hp: i32,
    pub(super) mp: i32,
    pub(super) stamina: i32,
    pub(super) life_state: ActorLifeState,
    pub(super) corpse_disposition: CorpseDisposition,
    pub(super) resource_activity: ResourceActivityCheckpointV2,
    pub(super) timing: ActorTimingCheckpointV2,
    pub(super) attack_ready_at: LogicalTime,
    pub(super) carried: CarriedCheckpointV2,
    pub(super) ai: Option<AiCheckpointV3>,
    pub(super) npc: Option<NpcCheckpointV2>,
    pub(super) xp_value: i32,
    pub(super) character_id: Option<CharacterId>,
    pub(super) character: Option<CharacterSheetV1>,
    pub(super) active_effects: Vec<ActiveEffectState>,
    pub(super) balm_effect: Option<BalmCheckpointV2>,
    pub(super) warmed_spell: Option<WarmedSpellState>,
    pub(super) monster_abilities: Vec<MonsterAbilityCheckpointV2>,
    pub(super) summoned: Option<SummonedActorState>,
    pub(super) ecology_origin: Option<EcologyOriginCheckpointV2>,
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
