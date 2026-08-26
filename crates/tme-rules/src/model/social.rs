use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{ActorId, ActorKind, CharacterAlignment, CharacterId, LogicalTime};

pub const MAX_GROUP_MEMBERS: usize = 6;
pub const MAX_INCOMING_GROUP_INVITATIONS: usize = 4;
pub const MAX_OUTGOING_GROUP_INVITATIONS: usize = 8;
pub const MAX_BLOCKED_CHARACTERS: usize = 256;
pub const GROUP_INVITATION_LIFETIME_UNITS: u64 = 60;
pub const GROUP_DISCONNECT_GRACE_UNITS: u64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupId(u64);

impl GroupId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupInviteId(u64);

impl GroupInviteId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMembershipKey {
    pub character_id: CharacterId,
    pub membership_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMemberState {
    pub character_id: CharacterId,
    pub joined_order: u64,
    pub membership_epoch: u64,
}

impl GroupMemberState {
    pub fn membership_key(&self) -> GroupMembershipKey {
        GroupMembershipKey {
            character_id: self.character_id.clone(),
            membership_epoch: self.membership_epoch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterPresenceState {
    pub connected: bool,
    pub control_epoch: u64,
    pub absent_since: Option<LogicalTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupState {
    pub id: GroupId,
    pub leader_character_id: CharacterId,
    pub members: Vec<GroupMemberState>,
    pub next_join_order: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupInvitationState {
    pub id: GroupInviteId,
    pub issuer_character_id: CharacterId,
    pub issuer_membership_epoch: Option<u64>,
    pub group_id: Option<GroupId>,
    pub target_character_id: CharacterId,
    pub expires_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommunicationPreferences {
    pub pages_enabled: bool,
    pub blocked_character_ids: BTreeSet<CharacterId>,
}

impl Default for CommunicationPreferences {
    fn default() -> Self {
        Self {
            pages_enabled: true,
            blocked_character_ids: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefeatRewardClass {
    Physical,
    DirectedSpell,
    AreaOrIllusionSpell,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DefeatRewardUnitId {
    Solo { character_id: CharacterId },
    Group { group_id: GroupId },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefeatContributionKey {
    pub contributor_character_id: CharacterId,
    pub reward_class: DefeatRewardClass,
    pub eligible_memberships: Vec<GroupMembershipKey>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefeatRewardUnitContribution {
    /// Positive applied damage grouped by contributor, reward class, and the
    /// exact membership cohort present when that damage landed.
    pub slices: BTreeMap<DefeatContributionKey, u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefeatContributionLedger {
    pub total_actual_damage: u64,
    pub reward_units: BTreeMap<DefeatRewardUnitId, DefeatRewardUnitContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialIntent {
    Invite { target_character_id: CharacterId },
    AcceptInvite { invitation_id: GroupInviteId },
    DeclineInvite { invitation_id: GroupInviteId },
    CancelInvite { invitation_id: GroupInviteId },
    LeaveGroup,
    RemoveMember { member_character_id: CharacterId },
    DisbandGroup,
    TransferLeadership { member_character_id: CharacterId },
    BeginFollow { target_character_id: CharacterId },
    EndFollow,
    SetPagesEnabled { enabled: bool },
    Block { target_character_id: CharacterId },
    Unblock { target_character_id: CharacterId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialBroadcastScope {
    Say,
    Shout,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialNature {
    Human,
    Animal,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialBehavior {
    Adventurer,
    Civilian,
    TownEnforcer,
    AlignmentCreature,
    Passive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialOwnerRelation {
    None,
    Summoner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SocialAlignmentSource {
    Character {},
    Inherent { alignment: CharacterAlignment },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialProfile {
    pub alignment_source: SocialAlignmentSource,
    pub nature: SocialNature,
    pub behavior: SocialBehavior,
    pub owner_relation: SocialOwnerRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LawZone {
    None,
    Town,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TownLawClassification {
    Permitted,
    TerrainAlignmentViolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellSocialProfile {
    pub hostile_act: bool,
    pub town_law: TownLawClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfDefenseRightV1 {
    pub victim_character_id: CharacterId,
    pub attacker_character_id: CharacterId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcGrudgeRelation {
    pub npc_actor_id: ActorId,
    pub attacker_actor_id: ActorId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SocialRelationLedger {
    pub self_defense: BTreeMap<CharacterId, SelfDefenseRightV1>,
    pub npc_grudges: BTreeSet<NpcGrudgeRelation>,
}

pub type SelfDefenseRights = BTreeMap<CharacterId, SelfDefenseRightV1>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerceivedSocialIdentity {
    pub actor_id: ActorId,
    pub alignment: CharacterAlignment,
    pub nature: SocialNature,
    pub behavior: SocialBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostilityReason {
    SameActor,
    Owner,
    Passive,
    NpcGrudge,
    SelfDefense,
    LawfulHumanResponse,
    ChaoticOpposition,
    EvilOpposition,
    NoHostility,
}

impl HostilityReason {
    pub const fn target_priority(self) -> u8 {
        match self {
            Self::NpcGrudge => 0,
            Self::SelfDefense
            | Self::LawfulHumanResponse
            | Self::ChaoticOpposition
            | Self::EvilOpposition => 1,
            Self::SameActor | Self::Owner | Self::Passive | Self::NoHostility => u8::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostilityAssessment {
    pub observer_actor_id: ActorId,
    pub target_actor_id: ActorId,
    pub target_identity: PerceivedSocialIdentity,
    pub hostile: bool,
    pub reason: HostilityReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostilityAuthorization {
    Safe,
    ConfirmedUnsafe,
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

impl AttackSafety {
    pub const fn permits(self, authorization: HostilityAuthorization) -> bool {
        !matches!(self, Self::Invalid)
            && (!matches!(self, Self::Protected)
                || matches!(authorization, HostilityAuthorization::ConfirmedUnsafe))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackSafetyAssessment {
    pub attacker_actor_id: ActorId,
    pub target_actor_id: ActorId,
    pub safety: AttackSafety,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialContactKind {
    PhysicalAttack,
    HostileSpellContact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfDefenseRelationPlan {
    pub before: Option<SelfDefenseRightV1>,
    pub after: SelfDefenseRightV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcGrudgeRelationPlan {
    pub relation: NpcGrudgeRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackRelationPlan {
    pub attacker_actor_id: ActorId,
    pub target_actor_id: ActorId,
    pub contact_kind: SocialContactKind,
    pub self_defense: Option<SelfDefenseRelationPlan>,
    pub npc_grudge: Option<NpcGrudgeRelationPlan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentConsequenceReason {
    UnjustLawfulHumanKill,
    UnjustLawfulAnimalKill,
    KarmaThreshold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountMarkAssessmentReason {
    AddForPlayerKill,
    ExemptSelfDefense,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMarkAssessment {
    pub killer_actor_id: ActorId,
    pub killer_character_id: CharacterId,
    pub victim_actor_id: ActorId,
    pub victim_character_id: CharacterId,
    pub credited_source_actor_id: ActorId,
    pub assessed: bool,
    pub reason: AccountMarkAssessmentReason,
}

/// D4: there is one world. `AppliedHere` covers the ordinary case where the
/// credited killer is resident and the consequence lands on their sheet in the
/// same tick. `RequiresAbsentKiller` covers a delayed hostile effect whose
/// credited killer has already departed the world — their sheet is not loaded,
/// so the karma/alignment change has no live target yet. The server records it
/// durably alongside the mark and applies it at the killer's next admission
/// (owner ruling 2026-08-20); see docs/server-notes.md.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerKillConsequenceV1 {
    AppliedHere {
        linked_karma_added: bool,
    },
    RequiresAbsentKiller {
        victim_alignment: CharacterAlignment,
        victim_nature: SocialNature,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerKillAssessmentV1 {
    pub facet_kill_sequence: u64,
    pub killer_character_id: CharacterId,
    pub victim_character_id: CharacterId,
    pub exempt_self_defense: bool,
    pub consequence: PlayerKillConsequenceV1,
    pub logical_time: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkedPlayerKillKarmaV1 {
    pub facet_kill_sequence: u64,
    pub killer_character_id: CharacterId,
    pub victim_character_id: CharacterId,
    pub logical_time: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DurableGameplayEffectV1 {
    PlayerKillAssessed(PlayerKillAssessmentV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostileEffectAuthority {
    pub credited_actor_id: ActorId,
    pub credited_character_id: CharacterId,
    pub authorization: HostilityAuthorization,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LethalSocialConsequencePlan {
    pub killer_actor_id: ActorId,
    pub killer_character_id: CharacterId,
    pub credited_source_actor_id: ActorId,
    pub victim_actor_id: ActorId,
    pub victim_character_id: Option<CharacterId>,
    pub victim_kind: ActorKind,
    pub victim_nature: SocialNature,
    pub victim_alignment: CharacterAlignment,
    pub self_defense: Option<SelfDefenseRightV1>,
    pub before_alignment: CharacterAlignment,
    pub after_alignment: CharacterAlignment,
    pub alignment_reason: Option<AlignmentConsequenceReason>,
    pub before_karma: u32,
    pub after_karma: u32,
    pub account_mark: Option<AccountMarkAssessment>,
    pub requires_knight_demotion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TownLawConsequencePlan {
    pub actor_id: ActorId,
    pub character_id: CharacterId,
    pub spell_id: String,
    pub site: super::WorldSite,
    pub before_alignment: CharacterAlignment,
    pub after_alignment: CharacterAlignment,
}
