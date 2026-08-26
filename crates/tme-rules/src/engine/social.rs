use crate::events::{
    AccountMarkAssessmentReasonV1, AlignmentChangeReasonV1, Event, KarmaChangeReasonV1,
    NpcGrudgeReasonV1, SelfDefenseChangeReasonV1, SelfDefenseChangedEventV1,
    SocialConsequenceSourceV1,
};
use crate::model::{
    AccountMarkAssessment, AccountMarkAssessmentReason, AlignmentConsequenceReason,
    AttackRelationPlan, AttackSafety, AttackSafetyAssessment, CharacterAlignment,
    DurableGameplayEffectV1, HostilityAssessment, HostilityReason, LawZone,
    LethalSocialConsequencePlan, NpcGrudgeRelation, NpcGrudgeRelationPlan, PerceivedSocialIdentity,
    PlayerKillAssessmentV1, PlayerKillConsequenceV1, SelfDefenseRelationPlan, SelfDefenseRightV1,
    SocialAlignmentSource, SocialBehavior, SocialContactKind, SocialNature, SocialOwnerRelation,
    TownLawConsequencePlan,
};

use super::promotion::ClassDemotionPlan;
use super::{Engine, RulesOutcomeV1, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CapturedLethalSocialConsequencePlan {
    facts: LethalSocialConsequencePlan,
    class_demotion: Option<ClassDemotionPlan>,
}

impl Engine {
    pub fn apply_absent_killer_player_kill_consequence(
        &mut self,
        assessment: &PlayerKillAssessmentV1,
    ) -> Result<(RulesOutcomeV1, bool), StepError> {
        let before = self.clone();
        let result = self.apply_absent_killer_player_kill_consequence_inner(assessment);
        if result.is_err() {
            *self = before;
        }
        result
    }

    fn apply_absent_killer_player_kill_consequence_inner(
        &mut self,
        assessment: &PlayerKillAssessmentV1,
    ) -> Result<(RulesOutcomeV1, bool), StepError> {
        let PlayerKillConsequenceV1::RequiresAbsentKiller {
            victim_alignment,
            victim_nature,
        } = assessment.consequence
        else {
            return Err(StepError::new(
                "remote player-kill consequence is not remotely owned",
            ));
        };
        if assessment.exempt_self_defense || assessment.facet_kill_sequence == 0 {
            return Err(StepError::new(
                "remote player-kill consequence has invalid assessment facts",
            ));
        }
        let matching = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, actor)| {
                actor.character_id.as_ref() == Some(&assessment.killer_character_id)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [killer_index] = matching.as_slice() else {
            return Err(StepError::new(
                "remote player-kill killer is not uniquely present",
            ));
        };
        let killer_index = *killer_index;
        let actor = &self.world.actors[killer_index];
        if actor.kind != crate::model::ActorKind::Player
            || !matches!(
                actor.social.alignment_source,
                SocialAlignmentSource::Character {}
            )
        {
            return Err(StepError::new(
                "remote player-kill killer is not character-authoritative",
            ));
        }
        let exact_link = crate::model::LinkedPlayerKillKarmaV1 {
            facet_kill_sequence: assessment.facet_kill_sequence,
            killer_character_id: assessment.killer_character_id.clone(),
            victim_character_id: assessment.victim_character_id.clone(),
            logical_time: assessment.logical_time,
        };
        if self.world.linked_player_kill_karma.contains(&exact_link) {
            return Err(StepError::new(
                "remote player-kill consequence was already applied",
            ));
        }
        let character = self.world.actors[killer_index]
            .character
            .as_mut()
            .ok_or_else(|| StepError::new("remote player-kill character sheet is absent"))?;
        let before_alignment = character.alignment_state.alignment;
        let linked_karma_added =
            victim_alignment == CharacterAlignment::Lawful && victim_nature == SocialNature::Human;
        if linked_karma_added {
            character.alignment_state.karma_points = character
                .alignment_state
                .karma_points
                .checked_add(1)
                .ok_or_else(|| StepError::new("remote player-kill karma overflow"))?;
            if character.alignment_state.karma_points >= 4 {
                character.alignment_state.alignment = CharacterAlignment::Evil;
            } else if before_alignment == CharacterAlignment::Lawful {
                character.alignment_state.alignment = CharacterAlignment::Neutral;
            }
            if character.identity.current_class_id == "knight" {
                character.identity.current_class_id = "fighter".to_string();
                character.identity.display_class = "Fighter".to_string();
            }
            self.world.linked_player_kill_karma.push(exact_link);
            self.world.linked_player_kill_karma.sort();
        } else if victim_alignment == CharacterAlignment::Lawful
            && victim_nature == SocialNature::Animal
            && before_alignment == CharacterAlignment::Lawful
        {
            character.alignment_state.alignment = CharacterAlignment::Neutral;
        }
        Ok((
            RulesOutcomeV1 {
                events: Vec::new(),
                state_changed: linked_karma_added
                    || character.alignment_state.alignment != before_alignment,
                durable_effects: Vec::new(),
            },
            linked_karma_added,
        ))
    }

    pub fn apply_player_kill_karma_forgiveness(
        &mut self,
        assessment: &PlayerKillAssessmentV1,
    ) -> Result<RulesOutcomeV1, StepError> {
        let before = self.clone();
        let link = crate::model::LinkedPlayerKillKarmaV1 {
            facet_kill_sequence: assessment.facet_kill_sequence,
            killer_character_id: assessment.killer_character_id.clone(),
            victim_character_id: assessment.victim_character_id.clone(),
            logical_time: assessment.logical_time,
        };
        let result = (|| {
            let link_index = self
                .world
                .linked_player_kill_karma
                .binary_search(&link)
                .map_err(|_| StepError::new("linked player-kill karma assessment is absent"))?;
            let matching = self
                .world
                .actors
                .iter()
                .enumerate()
                .filter(|(_, actor)| {
                    actor.character_id.as_ref() == Some(&assessment.killer_character_id)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let [killer_index] = matching.as_slice() else {
                return Err(StepError::new(
                    "forgiven player-kill killer is not uniquely present",
                ));
            };
            let character = self.world.actors[*killer_index]
                .character
                .as_mut()
                .ok_or_else(|| StepError::new("forgiven killer character sheet is absent"))?;
            character.alignment_state.karma_points = character
                .alignment_state
                .karma_points
                .checked_sub(1)
                .ok_or_else(|| StepError::new("linked player-kill karma is already zero"))?;
            self.world.linked_player_kill_karma.remove(link_index);
            Ok(RulesOutcomeV1 {
                events: Vec::new(),
                state_changed: true,
                durable_effects: Vec::new(),
            })
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    pub fn apply_character_session_exit(
        &mut self,
        character_id: &crate::model::CharacterId,
    ) -> RulesOutcomeV1 {
        let before = self.world.social_relations.self_defense.len();
        self.world
            .social_relations
            .self_defense
            .retain(|victim, relation| {
                victim != character_id && &relation.attacker_character_id != character_id
            });
        RulesOutcomeV1 {
            events: Vec::new(),
            state_changed: before != self.world.social_relations.self_defense.len(),
            durable_effects: Vec::new(),
        }
    }

    pub(super) fn true_actor_alignment(
        &self,
        actor_index: usize,
    ) -> Result<CharacterAlignment, StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("social actor disappeared"))?;
        match actor.social.alignment_source {
            SocialAlignmentSource::Character {} => {
                if actor.character_id.is_none() {
                    return Err(StepError::new(
                        "character-owned social alignment has no stable character identity",
                    ));
                }
                actor
                    .character
                    .as_ref()
                    .map(|character| character.alignment_state.alignment)
                    .ok_or_else(|| {
                        StepError::new("character-owned social alignment has no character sheet")
                    })
            }
            SocialAlignmentSource::Inherent { alignment } => {
                if actor.character_id.is_some() || actor.character.is_some() {
                    return Err(StepError::new(
                        "inherent social alignment carries character-owned facts",
                    ));
                }
                Ok(alignment)
            }
        }
    }

    pub(super) fn perceived_social_identity(
        &self,
        observer_index: usize,
        target_index: usize,
    ) -> Result<PerceivedSocialIdentity, StepError> {
        let observer = self
            .world
            .actors
            .get(observer_index)
            .ok_or_else(|| StepError::new("social observer disappeared"))?;
        let target = self
            .world
            .actors
            .get(target_index)
            .ok_or_else(|| StepError::new("social target disappeared"))?;
        let true_alignment = self.true_actor_alignment(target_index)?;
        let neutral_thief_disguise = observer_index != target_index
            && true_alignment == CharacterAlignment::Neutral
            && target
                .character
                .as_ref()
                .is_some_and(|character| character.identity.current_class_id == "thief");
        let observer_detects_disguise = observer.social.behavior == SocialBehavior::TownEnforcer
            || observer
                .character
                .as_ref()
                .is_some_and(|character| character.identity.current_class_id == "knight");
        let alignment = if neutral_thief_disguise && !observer_detects_disguise {
            CharacterAlignment::Lawful
        } else {
            true_alignment
        };
        Ok(PerceivedSocialIdentity {
            actor_id: target.id.clone(),
            alignment,
            nature: target.social.nature,
            behavior: target.social.behavior,
        })
    }

    fn exact_self_defense_relation(
        &self,
        victim_index: usize,
        attacker_index: usize,
    ) -> Option<&SelfDefenseRightV1> {
        let victim = self.world.actors.get(victim_index)?;
        let attacker = self.world.actors.get(attacker_index)?;
        let victim_character_id = victim.character_id.as_ref()?;
        let attacker_character_id = attacker.character_id.as_ref()?;
        self.world
            .social_relations
            .self_defense
            .get(victim_character_id)
            .filter(|relation| {
                relation.victim_character_id == *victim_character_id
                    && relation.attacker_character_id == *attacker_character_id
            })
    }

    fn has_npc_grudge(&self, npc_index: usize, attacker_index: usize) -> bool {
        let Some(npc) = self.world.actors.get(npc_index) else {
            return false;
        };
        let Some(attacker) = self.world.actors.get(attacker_index) else {
            return false;
        };
        self.world
            .social_relations
            .npc_grudges
            .contains(&NpcGrudgeRelation {
                npc_actor_id: npc.id.clone(),
                attacker_actor_id: attacker.id.clone(),
            })
    }

    pub(super) fn hostility_assessment(
        &self,
        observer_index: usize,
        target_index: usize,
    ) -> Result<HostilityAssessment, StepError> {
        let observer = self
            .world
            .actors
            .get(observer_index)
            .ok_or_else(|| StepError::new("hostility observer disappeared"))?;
        let target = self
            .world
            .actors
            .get(target_index)
            .ok_or_else(|| StepError::new("hostility target disappeared"))?;
        let target_identity = self.perceived_social_identity(observer_index, target_index)?;

        let assessment = |hostile, reason| HostilityAssessment {
            observer_actor_id: observer.id.clone(),
            target_actor_id: target.id.clone(),
            target_identity: target_identity.clone(),
            hostile,
            reason,
        };

        if observer_index == target_index {
            return Ok(assessment(false, HostilityReason::SameActor));
        }
        if observer.social.owner_relation == SocialOwnerRelation::Summoner {
            let owner_id = observer
                .summoned
                .as_ref()
                .map(|summoned| summoned.owner_id.as_str())
                .ok_or_else(|| {
                    StepError::new("summoner-owned social actor has no summoned state")
                })?;
            if owner_id == target.id.as_str() {
                return Ok(assessment(false, HostilityReason::Owner));
            }
        }
        if observer.social.behavior == SocialBehavior::Passive {
            return Ok(assessment(false, HostilityReason::Passive));
        }
        if self.has_npc_grudge(observer_index, target_index) {
            return Ok(assessment(true, HostilityReason::NpcGrudge));
        }
        if self
            .exact_self_defense_relation(observer_index, target_index)
            .is_some()
        {
            return Ok(assessment(true, HostilityReason::SelfDefense));
        }

        let observer_alignment = self.true_actor_alignment(observer_index)?;
        if observer_alignment == CharacterAlignment::Lawful
            && observer.social.nature == SocialNature::Human
            && matches!(
                observer.social.behavior,
                SocialBehavior::Civilian | SocialBehavior::TownEnforcer
            )
            && target_identity.nature == SocialNature::Human
            && matches!(
                target_identity.alignment,
                CharacterAlignment::Neutral | CharacterAlignment::Evil
            )
            && (!self.actor_is_hidden(target_index)
                || (observer.location.level == target.location.level
                    && observer.location.position == target.location.position))
            && self.actor_can_see(observer_index, &target.location.clone())
        {
            return Ok(assessment(true, HostilityReason::LawfulHumanResponse));
        }
        if observer.social.behavior == SocialBehavior::AlignmentCreature {
            if observer_alignment == CharacterAlignment::Chaotic
                && target_identity.alignment != CharacterAlignment::Chaotic
            {
                return Ok(assessment(true, HostilityReason::ChaoticOpposition));
            }
            if observer_alignment == CharacterAlignment::Evil
                && target_identity.alignment != CharacterAlignment::Evil
            {
                return Ok(assessment(true, HostilityReason::EvilOpposition));
            }
        }
        Ok(assessment(false, HostilityReason::NoHostility))
    }

    pub fn attack_safety_assessment(
        &self,
        attacker_index: usize,
        target_index: usize,
    ) -> Result<AttackSafetyAssessment, StepError> {
        let attacker = self
            .world
            .actors
            .get(attacker_index)
            .ok_or_else(|| StepError::new("attack confirmation actor disappeared"))?;
        let target = self
            .world
            .actors
            .get(target_index)
            .ok_or_else(|| StepError::new("attack confirmation target disappeared"))?;
        if attacker_index == target_index {
            return Ok(AttackSafetyAssessment {
                attacker_actor_id: attacker.id.clone(),
                target_actor_id: target.id.clone(),
                safety: AttackSafety::Invalid,
            });
        }
        if !target.is_alive() {
            return Err(StepError::new("attack confirmation target is not alive"));
        }
        let self_defense = self
            .exact_self_defense_relation(attacker_index, target_index)
            .is_some();
        let owned = target
            .summoned
            .as_ref()
            .is_some_and(|summoned| summoned.owner_id == attacker.id);
        let safety = if owned {
            AttackSafety::Invalid
        } else if target.kind == crate::model::ActorKind::Player {
            let target_alignment = self.true_actor_alignment(target_index)?;
            if self_defense {
                AttackSafety::OpenSelfDefense
            } else if target_alignment == CharacterAlignment::Evil {
                AttackSafety::OpenEvilPlayer
            } else {
                AttackSafety::Protected
            }
        } else {
            let target_identity = self.perceived_social_identity(attacker_index, target_index)?;
            if target_identity.alignment == CharacterAlignment::Lawful {
                AttackSafety::Protected
            } else if target.social.behavior != SocialBehavior::Passive
                && self
                    .hostility_assessment(target_index, attacker_index)?
                    .hostile
            {
                AttackSafety::OpenHostile
            } else {
                AttackSafety::Invalid
            }
        };
        Ok(AttackSafetyAssessment {
            attacker_actor_id: attacker.id.clone(),
            target_actor_id: target.id.clone(),
            safety,
        })
    }

    pub(super) fn delayed_hostile_contact_allowed(
        &self,
        authority: &crate::model::HostileEffectAuthority,
        target_index: usize,
    ) -> Result<bool, StepError> {
        let target = self
            .world
            .actors
            .get(target_index)
            .ok_or_else(|| StepError::new("delayed hostile target disappeared"))?;
        if !target.is_alive() {
            return Ok(false);
        }
        if let Some(source_index) = self.world.actors.iter().position(|actor| {
            actor.id == authority.credited_actor_id
                && actor.character_id.as_ref() == Some(&authority.credited_character_id)
        }) {
            return Ok(self
                .attack_safety_assessment(source_index, target_index)?
                .safety
                .permits(authority.authorization));
        }
        if target.character_id.as_ref() == Some(&authority.credited_character_id)
            || target
                .summoned
                .as_ref()
                .is_some_and(|summoned| summoned.owner_id == authority.credited_actor_id)
        {
            return Ok(false);
        }
        let safety = if target.kind == crate::model::ActorKind::Player {
            let target_character_id = target.character_id.as_ref().ok_or_else(|| {
                StepError::new("delayed hostile player target has no stable character")
            })?;
            if self
                .world
                .social_relations
                .self_defense
                .get(&authority.credited_character_id)
                .is_some_and(|right| {
                    right.victim_character_id == authority.credited_character_id
                        && right.attacker_character_id == *target_character_id
                })
            {
                AttackSafety::OpenSelfDefense
            } else if self.true_actor_alignment(target_index)? == CharacterAlignment::Evil {
                AttackSafety::OpenEvilPlayer
            } else {
                AttackSafety::Protected
            }
        } else if self.true_actor_alignment(target_index)? == CharacterAlignment::Lawful {
            AttackSafety::Protected
        } else if target.social.behavior != SocialBehavior::Passive {
            AttackSafety::OpenHostile
        } else {
            AttackSafety::Invalid
        };
        Ok(safety.permits(authority.authorization))
    }

    pub(super) fn plan_attack_relations(
        &self,
        attacker_index: usize,
        target_index: usize,
        contact_kind: SocialContactKind,
    ) -> Result<AttackRelationPlan, StepError> {
        let attacker = self
            .world
            .actors
            .get(attacker_index)
            .ok_or_else(|| StepError::new("social contact attacker disappeared"))?;
        let target = self
            .world
            .actors
            .get(target_index)
            .ok_or_else(|| StepError::new("social contact target disappeared"))?;
        if attacker_index == target_index || !attacker.is_alive() || !target.is_alive() {
            return Err(StepError::new(
                "social contact requires distinct living actors",
            ));
        }

        let self_defense = if attacker.kind == crate::model::ActorKind::Player
            && target.kind == crate::model::ActorKind::Player
            && self.true_actor_alignment(attacker_index)? != CharacterAlignment::Evil
            && self.true_actor_alignment(target_index)? != CharacterAlignment::Evil
            && self
                .exact_self_defense_relation(attacker_index, target_index)
                .is_none()
        {
            let attacker_character_id = attacker.character_id.clone().ok_or_else(|| {
                StepError::new("lawful player attacker has no stable character identity")
            })?;
            let victim_character_id = target.character_id.clone().ok_or_else(|| {
                StepError::new("lawful player victim has no stable character identity")
            })?;
            let after = SelfDefenseRightV1 {
                victim_character_id: victim_character_id.clone(),
                attacker_character_id,
            };
            let before = self
                .world
                .social_relations
                .self_defense
                .get(&victim_character_id)
                .cloned();
            (before.as_ref() != Some(&after)).then_some(SelfDefenseRelationPlan { before, after })
        } else {
            None
        };

        let inherent_lawful_human_npc = target.kind == crate::model::ActorKind::Npc
            && target.social.nature == SocialNature::Human
            && matches!(
                target.social.alignment_source,
                SocialAlignmentSource::Inherent {
                    alignment: CharacterAlignment::Lawful
                }
            );
        let npc_grudge = if inherent_lawful_human_npc {
            if target.ai.is_none() {
                return Err(StepError::new(
                    "lawful human NPC grudge target has no automatic AI state",
                ));
            }
            let relation = NpcGrudgeRelation {
                npc_actor_id: target.id.clone(),
                attacker_actor_id: attacker.id.clone(),
            };
            (!self.world.social_relations.npc_grudges.contains(&relation))
                .then_some(NpcGrudgeRelationPlan { relation })
        } else {
            None
        };

        Ok(AttackRelationPlan {
            attacker_actor_id: attacker.id.clone(),
            target_actor_id: target.id.clone(),
            contact_kind,
            self_defense,
            npc_grudge,
        })
    }

    pub(super) fn commit_attack_relations(
        &mut self,
        plan: &AttackRelationPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let attacker_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.attacker_actor_id)
            .ok_or_else(|| StepError::new("social contact attacker changed before commit"))?;
        let target_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.target_actor_id)
            .ok_or_else(|| StepError::new("social contact target changed before commit"))?;
        let current =
            self.plan_attack_relations(attacker_index, target_index, plan.contact_kind)?;
        if current != *plan {
            return Err(StepError::new("social contact facts changed before commit"));
        }

        if let Some(change) = &plan.self_defense {
            self.world.social_relations.self_defense.insert(
                change.after.victim_character_id.clone(),
                change.after.clone(),
            );
            events.push(Event::SelfDefenseChanged(SelfDefenseChangedEventV1 {
                victim_actor_id: plan.target_actor_id.clone(),
                victim_character_id: change.after.victim_character_id.clone(),
                before_attacker_character_id: change
                    .before
                    .as_ref()
                    .map(|relation| relation.attacker_character_id.clone()),
                after_attacker_character_id: Some(change.after.attacker_character_id.clone()),
                reason: if change.before.is_some() {
                    SelfDefenseChangeReasonV1::Replaced
                } else {
                    SelfDefenseChangeReasonV1::Established
                },
            }));
        }
        if let Some(grudge) = &plan.npc_grudge {
            if !self
                .world
                .social_relations
                .npc_grudges
                .insert(grudge.relation.clone())
            {
                return Err(StepError::new("NPC grudge was already established"));
            }
            self.make_npc_ready_now(target_index)?;
            events.push(Event::NpcGrudgeEstablished {
                npc_actor_id: grudge.relation.npc_actor_id.clone(),
                attacker_actor_id: grudge.relation.attacker_actor_id.clone(),
                reason: match plan.contact_kind {
                    SocialContactKind::PhysicalAttack => NpcGrudgeReasonV1::PhysicalAttack,
                    SocialContactKind::HostileSpellContact => {
                        NpcGrudgeReasonV1::HostileSpellContact
                    }
                },
            });
        }
        Ok(())
    }

    pub(super) fn clear_self_defense(
        &mut self,
        victim_index: usize,
        attacker_character_id: &crate::model::CharacterId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let victim = self
            .world
            .actors
            .get(victim_index)
            .ok_or_else(|| StepError::new("self-defense victim disappeared"))?;
        let victim_character_id = victim
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("self-defense clear requires a stable character"))?;
        let attacker_index = self.character_actor_index(attacker_character_id)?;
        let attacker = &self.world.actors[attacker_index];
        if !victim.is_alive() || !attacker.is_alive() {
            return Err(StepError::new(
                "self-defense clear requires two living characters",
            ));
        }
        if victim.location != attacker.location
            || !self.actor_can_see(victim_index, &attacker.location)
        {
            return Err(StepError::new(
                "self-defense clear requires a visible co-located attacker",
            ));
        }
        let exact = self
            .world
            .social_relations
            .self_defense
            .get(&victim_character_id)
            .is_some_and(|right| right.attacker_character_id == *attacker_character_id);
        if !exact {
            return Ok(());
        }
        self.world
            .social_relations
            .self_defense
            .remove(&victim_character_id);
        events.push(Event::SelfDefenseChanged(SelfDefenseChangedEventV1 {
            victim_actor_id: victim.id.clone(),
            victim_character_id,
            before_attacker_character_id: Some(attacker_character_id.clone()),
            after_attacker_character_id: None,
            reason: SelfDefenseChangeReasonV1::Cleared,
        }));
        Ok(())
    }

    pub(super) fn lethal_social_consequence_plan(
        &self,
        killer_index: usize,
        victim_index: usize,
        credited_source_actor_id: &crate::model::ActorId,
    ) -> Result<Option<CapturedLethalSocialConsequencePlan>, StepError> {
        let killer = self
            .world
            .actors
            .get(killer_index)
            .ok_or_else(|| StepError::new("lethal social killer disappeared"))?;
        let victim = self
            .world
            .actors
            .get(victim_index)
            .ok_or_else(|| StepError::new("lethal social victim disappeared"))?;
        if &killer.id != credited_source_actor_id {
            return Err(StepError::new(
                "lethal social credit is not the direct killer actor",
            ));
        }
        if !matches!(
            killer.social.alignment_source,
            SocialAlignmentSource::Character {}
        ) {
            if killer.character_id.is_some() || killer.character.is_some() {
                return Err(StepError::new(
                    "direct character killer does not use character-owned alignment",
                ));
            }
            return Ok(None);
        }
        let (Some(killer_character_id), Some(killer_character)) =
            (killer.character_id.clone(), killer.character.as_ref())
        else {
            if killer.character_id.is_some() || killer.character.is_some() {
                return Err(StepError::new(
                    "lethal social killer character facts are incomplete",
                ));
            }
            return Ok(None);
        };
        let victim_alignment = self.true_actor_alignment(victim_index)?;
        let self_defense = self
            .exact_self_defense_relation(killer_index, victim_index)
            .cloned();
        let exempt_self_defense = self_defense.is_some();
        let before_alignment = killer_character.alignment_state.alignment;
        let before_karma = killer_character.alignment_state.karma_points;
        let mut after_alignment = before_alignment;
        let mut after_karma = before_karma;
        let mut alignment_reason = None;
        let unjust_lawful_human = !exempt_self_defense
            && victim_alignment == CharacterAlignment::Lawful
            && victim.social.nature == SocialNature::Human;
        let unjust_lawful_animal = !exempt_self_defense
            && victim_alignment == CharacterAlignment::Lawful
            && victim.social.nature == SocialNature::Animal;

        if unjust_lawful_human {
            after_karma = before_karma
                .checked_add(1)
                .ok_or_else(|| StepError::new("karma overflow"))?;
            if after_karma >= 4 && before_alignment != CharacterAlignment::Evil {
                after_alignment = CharacterAlignment::Evil;
                alignment_reason = Some(AlignmentConsequenceReason::KarmaThreshold);
            } else if before_alignment == CharacterAlignment::Lawful {
                after_alignment = CharacterAlignment::Neutral;
                alignment_reason = Some(AlignmentConsequenceReason::UnjustLawfulHumanKill);
            }
        } else if unjust_lawful_animal && before_alignment == CharacterAlignment::Lawful {
            after_alignment = CharacterAlignment::Neutral;
            alignment_reason = Some(AlignmentConsequenceReason::UnjustLawfulAnimalKill);
        }

        let account_mark = if victim.kind == crate::model::ActorKind::Player {
            let victim_character_id = victim
                .character_id
                .clone()
                .ok_or_else(|| StepError::new("player victim has no stable character identity"))?;
            Some(AccountMarkAssessment {
                killer_actor_id: killer.id.clone(),
                killer_character_id: killer_character_id.clone(),
                victim_actor_id: victim.id.clone(),
                victim_character_id,
                credited_source_actor_id: credited_source_actor_id.clone(),
                assessed: !exempt_self_defense,
                reason: if exempt_self_defense {
                    AccountMarkAssessmentReason::ExemptSelfDefense
                } else {
                    AccountMarkAssessmentReason::AddForPlayerKill
                },
            })
        } else {
            None
        };
        let class_demotion = if unjust_lawful_human {
            self.class_demotion_plan(killer_index, &victim.id)?
        } else {
            None
        };
        if after_alignment == before_alignment
            && after_karma == before_karma
            && account_mark.is_none()
            && class_demotion.is_none()
        {
            return Ok(None);
        }
        let facts = LethalSocialConsequencePlan {
            killer_actor_id: killer.id.clone(),
            killer_character_id,
            credited_source_actor_id: credited_source_actor_id.clone(),
            victim_actor_id: victim.id.clone(),
            victim_character_id: victim.character_id.clone(),
            victim_kind: victim.kind,
            victim_nature: victim.social.nature,
            victim_alignment,
            self_defense,
            before_alignment,
            after_alignment,
            alignment_reason,
            before_karma,
            after_karma,
            account_mark,
            requires_knight_demotion: class_demotion.is_some(),
        };
        Ok(Some(CapturedLethalSocialConsequencePlan {
            facts,
            class_demotion,
        }))
    }

    pub(super) fn commit_lethal_social_consequence(
        &mut self,
        plan: &CapturedLethalSocialConsequencePlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let killer_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.facts.killer_actor_id)
            .ok_or_else(|| StepError::new("lethal social killer changed before commit"))?;
        let victim_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.facts.victim_actor_id)
            .ok_or_else(|| StepError::new("lethal social victim changed before commit"))?;
        let current = self.lethal_social_consequence_plan(
            killer_index,
            victim_index,
            &plan.facts.credited_source_actor_id,
        )?;
        if current.as_ref() != Some(plan) {
            return Err(StepError::new("lethal social facts changed before commit"));
        }

        let character = self.world.actors[killer_index]
            .character
            .as_mut()
            .ok_or_else(|| StepError::new("lethal social character disappeared"))?;
        character.alignment_state.alignment = plan.facts.after_alignment;
        character.alignment_state.karma_points = plan.facts.after_karma;

        if let Some(reason) = plan.facts.alignment_reason {
            events.push(Event::AlignmentChanged {
                actor_id: plan.facts.killer_actor_id.clone(),
                character_id: plan.facts.killer_character_id.clone(),
                before: plan.facts.before_alignment,
                after: plan.facts.after_alignment,
                reason: match reason {
                    AlignmentConsequenceReason::UnjustLawfulHumanKill => {
                        AlignmentChangeReasonV1::UnjustLawfulHumanKill
                    }
                    AlignmentConsequenceReason::UnjustLawfulAnimalKill => {
                        AlignmentChangeReasonV1::UnjustLawfulAnimalKill
                    }
                    AlignmentConsequenceReason::KarmaThreshold => {
                        AlignmentChangeReasonV1::KarmaThreshold
                    }
                },
                source: SocialConsequenceSourceV1::LawfulVictimDeath {
                    victim_actor_id: plan.facts.victim_actor_id.clone(),
                },
            });
        }
        if plan.facts.after_karma != plan.facts.before_karma {
            events.push(Event::KarmaChanged {
                actor_id: plan.facts.killer_actor_id.clone(),
                character_id: plan.facts.killer_character_id.clone(),
                before: plan.facts.before_karma,
                after: plan.facts.after_karma,
                delta: i32::try_from(
                    i64::from(plan.facts.after_karma) - i64::from(plan.facts.before_karma),
                )
                .map_err(|_| StepError::new("karma event delta does not fit signed 32-bit"))?,
                reason: KarmaChangeReasonV1::UnjustLawfulHumanKill,
                victim_actor_id: plan.facts.victim_actor_id.clone(),
            });
        }
        if let Some(mark) = &plan.facts.account_mark {
            events.push(Event::AccountMarkAssessed {
                killer_actor_id: mark.killer_actor_id.clone(),
                killer_character_id: mark.killer_character_id.clone(),
                victim_actor_id: mark.victim_actor_id.clone(),
                victim_character_id: mark.victim_character_id.clone(),
                credited_source_actor_id: mark.credited_source_actor_id.clone(),
                assessed: mark.assessed,
                reason: match mark.reason {
                    AccountMarkAssessmentReason::AddForPlayerKill => {
                        AccountMarkAssessmentReasonV1::AddForPlayerKill
                    }
                    AccountMarkAssessmentReason::ExemptSelfDefense => {
                        AccountMarkAssessmentReasonV1::ExemptSelfDefense
                    }
                },
            });

            let sequence = self.world.next_player_kill_sequence;
            self.world.next_player_kill_sequence = sequence
                .checked_add(1)
                .ok_or_else(|| StepError::new("player kill sequence overflow"))?;
            let linked_karma_added = plan.facts.after_karma > plan.facts.before_karma;
            if linked_karma_added {
                self.world
                    .linked_player_kill_karma
                    .push(crate::model::LinkedPlayerKillKarmaV1 {
                        facet_kill_sequence: sequence,
                        killer_character_id: mark.killer_character_id.clone(),
                        victim_character_id: mark.victim_character_id.clone(),
                        logical_time: self.world.timing.now,
                    });
                self.world.linked_player_kill_karma.sort();
            }
            self.pending_durable_effects
                .push(DurableGameplayEffectV1::PlayerKillAssessed(
                    PlayerKillAssessmentV1 {
                        facet_kill_sequence: sequence,
                        killer_character_id: mark.killer_character_id.clone(),
                        victim_character_id: mark.victim_character_id.clone(),
                        exempt_self_defense: !mark.assessed,
                        consequence: PlayerKillConsequenceV1::AppliedHere { linked_karma_added },
                        logical_time: self.world.timing.now,
                    },
                ));
        }
        if let Some(class_demotion) = &plan.class_demotion {
            self.commit_class_demotion(class_demotion, events)?;
        }
        Ok(())
    }

    pub(super) fn commit_absent_killer_player_kill_assessment(
        &mut self,
        authority: &crate::model::HostileEffectAuthority,
        victim_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let victim = self
            .world
            .actors
            .get(victim_index)
            .ok_or_else(|| StepError::new("remote player-kill victim disappeared"))?;
        if victim.kind != crate::model::ActorKind::Player {
            return Ok(());
        }
        let victim_character_id = victim
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("remote player-kill victim has no character ID"))?;
        if victim_character_id == authority.credited_character_id {
            return Err(StepError::new("remote player-kill credits the victim"));
        }
        let victim_actor_id = victim.id.clone();
        let victim_alignment = self.true_actor_alignment(victim_index)?;
        let victim_nature = victim.social.nature;
        let exempt_self_defense = self
            .world
            .social_relations
            .self_defense
            .get(&authority.credited_character_id)
            .is_some_and(|right| {
                right.victim_character_id == authority.credited_character_id
                    && right.attacker_character_id == victim_character_id
            });
        events.push(Event::AccountMarkAssessed {
            killer_actor_id: authority.credited_actor_id.clone(),
            killer_character_id: authority.credited_character_id.clone(),
            victim_actor_id: victim_actor_id.clone(),
            victim_character_id: victim_character_id.clone(),
            credited_source_actor_id: authority.credited_actor_id.clone(),
            assessed: !exempt_self_defense,
            reason: if exempt_self_defense {
                AccountMarkAssessmentReasonV1::ExemptSelfDefense
            } else {
                AccountMarkAssessmentReasonV1::AddForPlayerKill
            },
        });
        let sequence = self.world.next_player_kill_sequence;
        self.world.next_player_kill_sequence = sequence
            .checked_add(1)
            .ok_or_else(|| StepError::new("player kill sequence overflow"))?;
        self.pending_durable_effects
            .push(DurableGameplayEffectV1::PlayerKillAssessed(
                PlayerKillAssessmentV1 {
                    facet_kill_sequence: sequence,
                    killer_character_id: authority.credited_character_id.clone(),
                    victim_character_id,
                    exempt_self_defense,
                    consequence: PlayerKillConsequenceV1::RequiresAbsentKiller {
                        victim_alignment,
                        victim_nature,
                    },
                    logical_time: self.world.timing.now,
                },
            ));
        Ok(())
    }

    pub(super) fn town_law_consequence_plan(
        &self,
        actor_index: usize,
        spell_id: &str,
        site: &crate::model::WorldSite,
    ) -> Result<Option<TownLawConsequencePlan>, StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("town-law actor disappeared"))?;
        if actor.location.site() != *site {
            return Err(StepError::new("town-law actor site changed"));
        }
        let level = self
            .definition
            .world_template
            .realms
            .get(&site.realm)
            .and_then(|realm| realm.levels.get(&site.level))
            .ok_or_else(|| StepError::new("town-law level disappeared"))?;
        let spell = self
            .definition
            .catalog
            .spells
            .get(spell_id)
            .ok_or_else(|| StepError::new("town-law spell disappeared"))?;
        if level.law_zone != LawZone::Town
            || spell.social.town_law
                != crate::content::TownLawClassificationDef::TerrainAlignmentViolation
        {
            return Ok(None);
        }
        let Some(character) = actor.character.as_ref() else {
            return Ok(None);
        };
        if !matches!(
            actor.social.alignment_source,
            SocialAlignmentSource::Character {}
        ) {
            return Err(StepError::new(
                "town-law character does not use character-owned alignment",
            ));
        }
        let character_id = actor
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("town-law character identity disappeared"))?;
        if self.true_actor_alignment(actor_index)? != CharacterAlignment::Lawful {
            return Ok(None);
        }
        Ok(Some(TownLawConsequencePlan {
            actor_id: actor.id.clone(),
            character_id,
            spell_id: spell_id.to_string(),
            site: site.clone(),
            before_alignment: character.alignment_state.alignment,
            after_alignment: CharacterAlignment::Neutral,
        }))
    }

    pub(super) fn commit_town_law_consequence(
        &mut self,
        plan: &TownLawConsequencePlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.actor_id)
            .ok_or_else(|| StepError::new("town-law actor changed before commit"))?;
        let current = self.town_law_consequence_plan(actor_index, &plan.spell_id, &plan.site)?;
        if current.as_ref() != Some(plan) {
            return Err(StepError::new("town-law facts changed before commit"));
        }
        self.world.actors[actor_index]
            .character
            .as_mut()
            .ok_or_else(|| StepError::new("town-law character disappeared"))?
            .alignment_state
            .alignment = plan.after_alignment;
        events.push(Event::AlignmentChanged {
            actor_id: plan.actor_id.clone(),
            character_id: plan.character_id.clone(),
            before: plan.before_alignment,
            after: plan.after_alignment,
            reason: AlignmentChangeReasonV1::TownTerrainCast,
            source: SocialConsequenceSourceV1::TownTerrainCast {
                spell_id: plan.spell_id.clone(),
                site: plan.site.clone(),
            },
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests;
