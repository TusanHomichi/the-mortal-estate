use tme_rules::events::{
    AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1,
    BanishResultReasonV1, RaiseDeadResultReasonV1, SpellCastFailure, SpellFizzleCause,
    SpellPathFailureReason, TransitionConcealmentRemovalReasonV1,
};
use tme_rules::{ActorKind, Event, InspectExitStatus, NavigationKind};
pub(crate) fn render_events(events: &[Event]) -> Vec<String> {
    events.iter().flat_map(render_event).collect()
}

fn option_label<T: std::fmt::Display>(value: &Option<T>) -> String {
    value
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string())
}

fn mitigation_mode_label(mode: Option<tme_rules::SpellResistanceMitigationMode>) -> &'static str {
    match mode {
        Some(tme_rules::SpellResistanceMitigationMode::Negate) => "negate",
        Some(tme_rules::SpellResistanceMitigationMode::HalfDamage) => "half_damage",
        Some(tme_rules::SpellResistanceMitigationMode::MinimumDamage) => "minimum_damage",
        None => "none",
    }
}

fn death_cause_label(cause: tme_rules::DeathCause) -> &'static str {
    match cause {
        tme_rules::DeathCause::Physical => "physical",
        tme_rules::DeathCause::Poison => "poison",
        tme_rules::DeathCause::Fire => "fire",
        tme_rules::DeathCause::OtherMagic => "other_magic",
        tme_rules::DeathCause::Hazard => "hazard",
    }
}

fn life_state_label(state: &tme_rules::ActorLifeState) -> &'static str {
    match state {
        tme_rules::ActorLifeState::Alive => "alive",
        tme_rules::ActorLifeState::Ghost { .. } => "ghost",
        tme_rules::ActorLifeState::AwaitingResurrection { .. } => "awaiting_resurrection",
        tme_rules::ActorLifeState::Dead => "dead",
    }
}

fn life_state_view_label(state: &tme_rules::ActorLifeStateViewV1) -> &'static str {
    match state {
        tme_rules::ActorLifeStateViewV1::Alive => "alive",
        tme_rules::ActorLifeStateViewV1::Ghost { .. } => "ghost",
        tme_rules::ActorLifeStateViewV1::AwaitingResurrection { .. } => "awaiting_resurrection",
        tme_rules::ActorLifeStateViewV1::Dead => "dead",
    }
}

fn loot_claim_label(claim: &tme_rules::LootClaim) -> String {
    let owner = match &claim.owner {
        tme_rules::LootOwnerId::Character(id) => id.as_str().to_string(),
        tme_rules::LootOwnerId::TransientActor(id) => format!("actor:{id}"),
    };
    let basis = match claim.basis {
        tme_rules::LootClaimBasis::KillingBlow => "killing_blow",
        tme_rules::LootClaimBasis::CharacterDeathPile => "character_death_pile",
    };
    format!("{owner}/{basis}")
}

fn claim_suffix(claim: &Option<tme_rules::LootClaim>) -> String {
    claim
        .as_ref()
        .map(|claim| format!(" claim={}", loot_claim_label(claim)))
        .unwrap_or_default()
}

fn gold_location_label(location: &tme_rules::GoldLocationViewV1) -> String {
    match location {
        tme_rules::GoldLocationViewV1::Carried { actor_id, position } => {
            format!("carried:{actor_id}:{}", position.label())
        }
        tme_rules::GoldLocationViewV1::Corpse { corpse_id } => corpse_id.to_string(),
        tme_rules::GoldLocationViewV1::Ground {
            gold_pile_id,
            location,
        } => format!("ground:{gold_pile_id}@{}", location.label()),
        tme_rules::GoldLocationViewV1::Bank {
            bank_id,
            character_id,
        } => format!("bank:{bank_id}:{}", character_id.as_str()),
    }
}

fn resurrection_method_label(method: tme_rules::ResurrectionMethod) -> &'static str {
    match method {
        tme_rules::ResurrectionMethod::Gods => "gods",
        tme_rules::ResurrectionMethod::Priest => "priest",
        tme_rules::ResurrectionMethod::Thaumaturge => "thaumaturge",
    }
}

fn gold_reason_label(reason: tme_rules::GoldRelocationReason) -> &'static str {
    match reason {
        tme_rules::GoldRelocationReason::PlayerMove => "player_move",
        tme_rules::GoldRelocationReason::Scavenging => "scavenging",
        tme_rules::GoldRelocationReason::BankDeposit => "bank_deposit",
        tme_rules::GoldRelocationReason::BankWithdrawal => "bank_withdrawal",
        tme_rules::GoldRelocationReason::DeathDrop => "death_drop",
        tme_rules::GoldRelocationReason::CorpseRetention => "corpse_retention",
        tme_rules::GoldRelocationReason::CorpseSearch => "corpse_search",
        tme_rules::GoldRelocationReason::ResurrectionReturn => "resurrection_return",
    }
}

fn alignment_label(alignment: tme_rules::CharacterAlignment) -> &'static str {
    match alignment {
        tme_rules::CharacterAlignment::Lawful => "lawful",
        tme_rules::CharacterAlignment::Neutral => "neutral",
        tme_rules::CharacterAlignment::Chaotic => "chaotic",
        tme_rules::CharacterAlignment::Evil => "evil",
    }
}

mod action_events;
mod inventory_events;
mod magic_events;

fn render_event(event: &Event) -> Vec<String> {
    match event {
        Event::ScenarioLoaded { .. }
        | Event::ActorStatus { .. }
        | Event::ActorReady { .. }
        | Event::PlayerIntent { .. }
        | Event::ActorReadinessScheduled { .. }
        | Event::GroupInvitationCreated { .. }
        | Event::GroupInvitationResolved { .. }
        | Event::GroupChanged { .. }
        | Event::GroupPresenceChanged { .. }
        | Event::PlayerFollowChanged { .. }
        | Event::CommunicationPreferenceChanged { .. }
        | Event::CharacterBlockChanged { .. }
        | Event::LogicalTimeAdvanced { .. }
        | Event::Inspected { .. }
        | Event::AutomaticActorDecision { .. }
        | Event::Moved { .. }
        | Event::MovementStarted { .. }
        | Event::MovementCostPaid { .. }
        | Event::MovementBlocked { .. }
        | Event::AttackBlockedNoSight { .. }
        | Event::AttackNotReady { .. }
        | Event::Attacked { .. }
        | Event::AttackBlocked { .. }
        | Event::BowReadinessChanged { .. }
        | Event::WeaponFumbled { .. }
        | Event::AttackMissed { .. }
        | Event::ProtectionApplied { .. }
        | Event::PhysicalDamageAffinityApplied { .. }
        | Event::EcologyResetScheduled { .. }
        | Event::EcologyReset { .. }
        | Event::EcologyActorSpawned { .. }
        | Event::PhysicalStaminaSpent { .. }
        | Event::PhysicalPracticeEvaluated { .. }
        | Event::DefeatContributionRecorded { .. }
        | Event::DefeatRewardEvaluated { .. }
        | Event::DefeatRewardShareAwarded { .. }
        | Event::ThaumAboveSkillEvaluated { .. }
        | Event::MagicPracticeEvaluated { .. } => action_events::render(event),
        Event::ItemRelocated { .. }
        | Event::ItemBound { .. }
        | Event::ActorDefeated { .. }
        | Event::CorpseCreated { .. }
        | Event::CorpseSearched { .. }
        | Event::CorpseRemoved { .. }
        | Event::ActorLifeStateChanged { .. }
        | Event::ResurrectionRequested { .. }
        | Event::ActorResurrected { .. }
        | Event::GoldRelocated { .. }
        | Event::BankBalanceChanged { .. }
        | Event::ItemOfferCreated { .. }
        | Event::ItemOfferCompleted { .. }
        | Event::ResourceRegenerated { .. }
        | Event::ResourceRestored { .. }
        | Event::ItemConsumed { .. }
        | Event::BalmHealed { .. }
        | Event::DoorOpened { .. }
        | Event::DoorClosed { .. }
        | Event::SecretTransitionRevealed { .. }
        | Event::SecretTransitionHidden { .. }
        | Event::TransitionConcealed { .. }
        | Event::TransitionConcealmentRemoved { .. }
        | Event::WorldTransition { .. }
        | Event::SackShown { .. }
        | Event::ItemIdentified { .. }
        | Event::ItemAppraised { .. }
        | Event::ItemEnchanted { .. }
        | Event::ItemEnchantmentExpired { .. }
        | Event::ItemTransformed { .. }
        | Event::Located { .. }
        | Event::PortalCreated { .. }
        | Event::PortalExpired { .. }
        | Event::ExperienceAwarded { .. }
        | Event::LevelGained { .. }
        | Event::PhysicalAttributeAddsChanged { .. }
        | Event::MovementStaminaSpent { .. }
        | Event::SkillPracticeAwarded { .. }
        | Event::SkillPositionChanged { .. }
        | Event::GoldChanged { .. }
        | Event::TrainingPurchased { .. } => inventory_events::render(event),
        Event::SkillCritiqued { .. }
        | Event::ClassPromoted { .. }
        | Event::SpellLearned { .. }
        | Event::SpellCastStubbed { .. }
        | Event::SpellCastCommitted { .. }
        | Event::ActorSummoned { .. }
        | Event::SummonExpired { .. }
        | Event::BanishEvaluated { .. }
        | Event::ActorBanished { .. }
        | Event::TurnUndeadResolved { .. }
        | Event::RaiseDeadEvaluated { .. }
        | Event::SpellDamaged { .. }
        | Event::SpellHealed { .. }
        | Event::SpellWarmed { .. }
        | Event::WarmedSpellReady { .. }
        | Event::WarmedSpellCast { .. }
        | Event::SpellFizzled { .. }
        | Event::SpellCastFailed { .. }
        | Event::ActorHidden { .. }
        | Event::HideBroken { .. }
        | Event::EffectApplied { .. }
        | Event::TileEffectApplied { .. }
        | Event::SpellSaveResolved { .. }
        | Event::EffectTicked { .. }
        | Event::TileEffectTicked { .. }
        | Event::EffectDamaged { .. }
        | Event::TileEffectDamaged { .. }
        | Event::EffectExpired { .. }
        | Event::TileEffectExpired { .. }
        | Event::EffectRemoved { .. }
        | Event::TileEffectRemoved { .. }
        | Event::ActionSuppressedByStatus { .. }
        | Event::NpcSpoke { .. }
        | Event::NpcFollowChanged { .. }
        | Event::NpcFollowDecision { .. }
        | Event::SelfDefenseChanged(..)
        | Event::NpcGrudgeEstablished { .. }
        | Event::AlignmentChanged { .. }
        | Event::KarmaChanged { .. }
        | Event::AccountMarkAssessed { .. }
        | Event::ClassDemoted { .. }
        | Event::QuestStateChanged { .. }
        | Event::TransactionCommitted { .. }
        | Event::FinalState { .. } => magic_events::render(event),
    }
}

fn role_label(kind: ActorKind) -> &'static str {
    match kind {
        ActorKind::Player => "player",
        ActorKind::Monster => "monster",
        ActorKind::Npc => "npc",
    }
}

#[cfg(test)]
mod tests;
