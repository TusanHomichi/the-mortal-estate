use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRole {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealmState {
    pub name: String,
    pub levels: std::collections::HashMap<String, LevelState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalmEffectState {
    pub(crate) heal_per_round: i32,
    pub(crate) restored: i32,
    pub(crate) budget: i32,
    pub(crate) last_tick_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct World {
    pub timing: WorldTimingState,
    pub actors: Vec<ActorState>,
    pub ecology_sites: std::collections::BTreeMap<String, EcologySiteState>,
    pub social_relations: SocialRelationLedger,
    pub groups: std::collections::BTreeMap<GroupId, GroupState>,
    pub group_invitations: std::collections::BTreeMap<GroupInviteId, GroupInvitationState>,
    pub player_follow_targets: std::collections::BTreeMap<CharacterId, CharacterId>,
    pub communication_preferences:
        std::collections::BTreeMap<CharacterId, CommunicationPreferences>,
    pub character_presence: std::collections::BTreeMap<CharacterId, CharacterPresenceState>,
    pub defeat_contributions: std::collections::BTreeMap<ActorId, DefeatContributionLedger>,
    pub item_instances: std::collections::BTreeMap<String, ItemInstanceState>,
    pub service_instances: Vec<ServiceInstanceState>,
    pub merchant_inventories:
        std::collections::BTreeMap<MerchantInventoryId, MerchantInventoryState>,
    pub banks: std::collections::BTreeMap<BankId, BankState>,
    pub locker_vaults: std::collections::BTreeMap<LockerVaultId, LockerVaultState>,
    pub item_offers: std::collections::BTreeMap<String, ItemOfferState>,
    pub quest_states: QuestStateLedger,
    pub ground_items: Vec<GroundItem>,
    pub corpses: std::collections::BTreeMap<CorpseId, CorpseState>,
    pub ground_gold: std::collections::BTreeMap<GoldPileId, GroundGoldPile>,
    pub next_corpse_sequence: u64,
    pub next_gold_sequence: u64,
    pub next_summon_sequence: u32,
    pub next_group_sequence: u64,
    pub next_group_invite_sequence: u64,
    pub next_membership_epoch: u64,
    pub next_player_kill_sequence: u64,
    pub linked_player_kill_karma: Vec<LinkedPlayerKillKarmaV1>,
    pub tile_effects: Vec<TileEffectState>,
    pub item_enchantments: Vec<ItemEnchantmentState>,
    pub portal_transitions: Vec<PortalTransitionState>,
    pub concealed_transitions: Vec<ConcealedTransitionState>,
    pub hidden_transition_revealed: std::collections::HashMap<WorldPosition, bool>,
    pub door_states: std::collections::HashMap<WorldPosition, bool>,
}

impl World {
    pub fn actor(&self, actor_id: &ActorId) -> Option<&ActorState> {
        self.actors.iter().find(|actor| &actor.id == actor_id)
    }

    pub fn controlled_actors(&self) -> impl Iterator<Item = &ActorState> {
        self.actors
            .iter()
            .filter(|actor| actor.kind == ActorKind::Player)
    }

    pub fn is_actor_alive(&self, actor_id: &ActorId) -> bool {
        self.actor(actor_id).is_some_and(ActorState::is_alive)
    }
}
