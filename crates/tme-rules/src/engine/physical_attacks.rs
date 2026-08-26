use crate::model::{
    ActorId, ArmorProtectionPlan, AttackRelationPlan, AttackSafetyAssessment, BowReadiness,
    CarriedPosition, CharacterAlignment, CharacterId, HostilityAuthorization,
    PerceivedSocialIdentity, PhysicalAttackMode, PhysicalDamageKind, SocialContactKind, WoundState,
};

use super::weapons::PhysicalWeaponSelection;
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalAttackPlan {
    pub attacker_index: usize,
    pub defender_index: usize,
    pub mode: PhysicalAttackMode,
    pub selection: PhysicalWeaponSelection,
    pub skill_track_id: String,
    pub skill_level: u8,
    pub maximum_range: i32,
    pub distance: i32,
    pub damage_kind: PhysicalDamageKind,
    pub cooldown_units: u32,
    pub effective_combat_add_rating: i32,
    pub jumpkick_stamina_cost: i32,
    pub barefoot_full_effect: bool,
    pub consumes_bow_nock: bool,
    pub releases_weapon: bool,
    pub defender_armor: ArmorProtectionPlan,
    pub attacker_wound_before: WoundState,
    pub defender_wound_before: WoundState,
    pub social: CapturedPhysicalAttackSocialPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PhysicalAttackAuthority {
    Player {
        authorization: HostilityAuthorization,
    },
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CapturedPhysicalAttackAuthority {
    Player {
        authorization: HostilityAuthorization,
        safety: AttackSafetyAssessment,
    },
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CapturedPhysicalAttackSocialPlan {
    pub attacker_actor_id: ActorId,
    pub attacker_character_id: Option<CharacterId>,
    pub attacker_true_alignment: CharacterAlignment,
    pub defender_actor_id: ActorId,
    pub defender_character_id: Option<CharacterId>,
    pub defender_true_alignment: CharacterAlignment,
    pub perceived_defender: PerceivedSocialIdentity,
    pub authority: CapturedPhysicalAttackAuthority,
    pub relations: AttackRelationPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalAttackModePlan {
    pub selection: PhysicalWeaponSelection,
    pub skill_track_id: String,
    pub skill_level: u8,
    pub maximum_range: i32,
    pub damage_kind: PhysicalDamageKind,
    pub cooldown_units: u32,
    pub effective_combat_add_rating: i32,
    pub jumpkick_stamina_cost: i32,
    pub barefoot_full_effect: bool,
}

impl PhysicalAttackPlan {
    pub const fn is_weapon_backed(&self) -> bool {
        self.selection.item_instance_id.is_some()
    }
}

impl Engine {
    pub(super) fn physical_attack_plan(
        &self,
        attacker_index: usize,
        defender_index: usize,
        mode: PhysicalAttackMode,
        authorization: HostilityAuthorization,
    ) -> Result<PhysicalAttackPlan, StepError> {
        self.physical_attack_plan_with_nock_requirement(
            attacker_index,
            defender_index,
            mode,
            true,
            None,
            PhysicalAttackAuthority::Player { authorization },
        )
    }

    pub(super) fn automatic_physical_attack_plan(
        &self,
        attacker_index: usize,
        defender_index: usize,
        mode: PhysicalAttackMode,
    ) -> Result<PhysicalAttackPlan, StepError> {
        self.physical_attack_plan_with_nock_requirement(
            attacker_index,
            defender_index,
            mode,
            true,
            None,
            PhysicalAttackAuthority::Automatic,
        )
    }

    pub(super) fn automatic_physical_attack_opportunity_plan(
        &self,
        attacker_index: usize,
        defender_index: usize,
        mode: PhysicalAttackMode,
    ) -> Result<PhysicalAttackPlan, StepError> {
        self.physical_attack_plan_with_nock_requirement(
            attacker_index,
            defender_index,
            mode,
            false,
            None,
            PhysicalAttackAuthority::Automatic,
        )
    }

    pub(super) fn physical_attack_option_plan(
        &self,
        attacker_index: usize,
        defender_index: usize,
        mode: PhysicalAttackMode,
    ) -> (
        Option<PhysicalAttackModePlan>,
        Result<PhysicalAttackPlan, StepError>,
    ) {
        match self.physical_attack_mode_plan(attacker_index, mode) {
            Ok(mode_plan) => {
                let authorization = self
                    .attack_safety_assessment(attacker_index, defender_index)
                    .map(|assessment| {
                        if matches!(assessment.safety, crate::model::AttackSafety::Protected) {
                            HostilityAuthorization::ConfirmedUnsafe
                        } else {
                            HostilityAuthorization::Safe
                        }
                    });
                let result = authorization.and_then(|authorization| {
                    self.physical_attack_plan_with_nock_requirement(
                        attacker_index,
                        defender_index,
                        mode,
                        true,
                        Some(mode_plan.clone()),
                        PhysicalAttackAuthority::Player { authorization },
                    )
                });
                (Some(mode_plan), result)
            }
            Err(error) => (None, Err(error)),
        }
    }

    fn physical_attack_mode_plan(
        &self,
        attacker_index: usize,
        mode: PhysicalAttackMode,
    ) -> Result<PhysicalAttackModePlan, StepError> {
        self.world
            .actors
            .get(attacker_index)
            .ok_or_else(|| StepError::new("unknown attacker"))?;
        let (selection, maximum_range, damage_kind, cooldown_units, jumpkick_stamina_cost) =
            match mode {
                PhysicalAttackMode::Kick => {
                    let rules = &self.definition.catalog.rules.combat.attack_modes.kick;
                    (
                        self.martial_attack_selection(attacker_index)?,
                        rules.maximum_range,
                        rules.damage_kind,
                        rules.cooldown_units,
                        0,
                    )
                }
                PhysicalAttackMode::Jumpkick => {
                    let selection = self.martial_attack_selection(attacker_index)?;
                    let rules = &self.definition.catalog.rules.combat.attack_modes.jumpkick;
                    let skill_range = 1_i32
                        .checked_add(
                            i32::from(selection.skill_level)
                                / i32::try_from(rules.skill_levels_per_extra_hex)
                                    .map_err(|_| StepError::new("jumpkick divisor overflow"))?,
                        )
                        .ok_or_else(|| StepError::new("jumpkick range overflow"))?
                        .min(rules.maximum_range_cap);
                    (
                        selection,
                        skill_range,
                        rules.damage_kind,
                        rules.cooldown_units,
                        rules.stamina_cost,
                    )
                }
                _ => {
                    let selection = self.physical_weapon_selection(attacker_index)?;
                    let row = selection.attack_mode(mode).ok_or_else(|| {
                        StepError::new(format!(
                            "selected right-hand weapon does not support {}",
                            mode.label()
                        ))
                    })?;
                    (
                        selection.clone(),
                        row.maximum_range,
                        row.damage_kind,
                        selection.cooldown_units,
                        0,
                    )
                }
            };
        let barefoot_full_effect = mode != PhysicalAttackMode::Jumpkick
            || self
                .item_at_position(attacker_index, CarriedPosition::Boots)?
                .is_none();
        let effective_combat_add_rating = self.effective_combat_add_rating(&selection)?;
        Ok(PhysicalAttackModePlan {
            skill_track_id: selection.skill_track_id.clone(),
            skill_level: selection.skill_level,
            selection,
            maximum_range,
            damage_kind,
            cooldown_units,
            effective_combat_add_rating,
            jumpkick_stamina_cost,
            barefoot_full_effect,
        })
    }

    fn physical_attack_plan_with_nock_requirement(
        &self,
        attacker_index: usize,
        defender_index: usize,
        mode: PhysicalAttackMode,
        require_nocked_bow: bool,
        mode_plan: Option<PhysicalAttackModePlan>,
        authority: PhysicalAttackAuthority,
    ) -> Result<PhysicalAttackPlan, StepError> {
        if attacker_index == defender_index {
            return Err(StepError::new("actor cannot attack itself"));
        }
        let attacker = self
            .world
            .actors
            .get(attacker_index)
            .ok_or_else(|| StepError::new("unknown attacker"))?;
        let defender = self
            .world
            .actors
            .get(defender_index)
            .ok_or_else(|| StepError::new("unknown defender"))?;
        if !attacker.is_alive() {
            return Err(StepError::new("attacker is not alive"));
        }
        if !defender.is_alive() {
            return Err(StepError::new("target is not alive"));
        }
        let social =
            self.capture_physical_attack_social_plan(attacker_index, defender_index, authority)?;
        if attacker.location.level != defender.location.level {
            return Err(StepError::new(
                "physical attack target is not in the same room",
            ));
        }

        let distance = attacker
            .location
            .position
            .chebyshev_distance(defender.location.position);
        let mode_plan = match mode_plan {
            Some(mode_plan) => mode_plan,
            None => self.physical_attack_mode_plan(attacker_index, mode)?,
        };
        let PhysicalAttackModePlan {
            selection,
            skill_track_id,
            skill_level,
            maximum_range,
            damage_kind,
            cooldown_units,
            effective_combat_add_rating,
            jumpkick_stamina_cost,
            barefoot_full_effect,
        } = mode_plan;

        let legal_range = match mode {
            PhysicalAttackMode::Fight | PhysicalAttackMode::Kick => distance == 0,
            PhysicalAttackMode::Jumpkick => (1..=maximum_range).contains(&distance),
            PhysicalAttackMode::Poke => (0..=maximum_range).contains(&distance),
            PhysicalAttackMode::Shoot | PhysicalAttackMode::Throw => {
                (1..=maximum_range).contains(&distance)
            }
        };
        if !legal_range {
            return Err(StepError::new(format!(
                "{} target is out of range",
                mode.label()
            )));
        }

        if matches!(
            mode,
            PhysicalAttackMode::Jumpkick | PhysicalAttackMode::Shoot | PhysicalAttackMode::Throw
        ) {
            let target = defender.location.clone();
            if !self.actor_can_see(attacker_index, &target) {
                return Err(StepError::new(format!(
                    "{} target is not visible",
                    mode.label()
                )));
            }
        }
        if require_nocked_bow
            && mode == PhysicalAttackMode::Shoot
            && selection.bow_readiness != Some(BowReadiness::Nocked)
        {
            return Err(StepError::new("bow is not nocked"));
        }
        if jumpkick_stamina_cost > attacker.stamina {
            return Err(StepError::new("not enough stamina to jumpkick"));
        }

        Ok(PhysicalAttackPlan {
            attacker_index,
            defender_index,
            mode,
            skill_track_id,
            skill_level,
            selection,
            maximum_range,
            distance,
            damage_kind,
            cooldown_units,
            effective_combat_add_rating,
            jumpkick_stamina_cost,
            barefoot_full_effect,
            consumes_bow_nock: mode == PhysicalAttackMode::Shoot,
            releases_weapon: mode == PhysicalAttackMode::Throw,
            defender_armor: self.armor_protection_plan(defender_index)?,
            attacker_wound_before: self.wound_state(attacker_index),
            defender_wound_before: self.wound_state(defender_index),
            social,
        })
    }

    fn capture_physical_attack_social_plan(
        &self,
        attacker_index: usize,
        defender_index: usize,
        authority: PhysicalAttackAuthority,
    ) -> Result<CapturedPhysicalAttackSocialPlan, StepError> {
        let attacker = self
            .world
            .actors
            .get(attacker_index)
            .ok_or_else(|| StepError::new("physical social attacker disappeared"))?;
        let defender = self
            .world
            .actors
            .get(defender_index)
            .ok_or_else(|| StepError::new("physical social defender disappeared"))?;
        let perceived_defender = self.perceived_social_identity(attacker_index, defender_index)?;
        let authority = match authority {
            PhysicalAttackAuthority::Player { authorization } => {
                let safety = self.attack_safety_assessment(attacker_index, defender_index)?;
                if !safety.safety.permits(authorization) {
                    return Err(StepError::new(match safety.safety {
                        crate::model::AttackSafety::Protected => {
                            "protected_target_requires_confirmation"
                        }
                        _ => "invalid_hostile_target",
                    }));
                }
                CapturedPhysicalAttackAuthority::Player {
                    authorization,
                    safety,
                }
            }
            PhysicalAttackAuthority::Automatic => CapturedPhysicalAttackAuthority::Automatic,
        };
        Ok(CapturedPhysicalAttackSocialPlan {
            attacker_actor_id: attacker.id.clone(),
            attacker_character_id: attacker.character_id.clone(),
            attacker_true_alignment: self.true_actor_alignment(attacker_index)?,
            defender_actor_id: defender.id.clone(),
            defender_character_id: defender.character_id.clone(),
            defender_true_alignment: self.true_actor_alignment(defender_index)?,
            perceived_defender,
            authority,
            relations: self.plan_attack_relations(
                attacker_index,
                defender_index,
                SocialContactKind::PhysicalAttack,
            )?,
        })
    }

    pub(super) fn commit_physical_attack_social_plan(
        &mut self,
        plan: &PhysicalAttackPlan,
        events: &mut Vec<crate::events::Event>,
    ) -> Result<(), StepError> {
        let authority = match &plan.social.authority {
            CapturedPhysicalAttackAuthority::Player { authorization, .. } => {
                PhysicalAttackAuthority::Player {
                    authorization: *authorization,
                }
            }
            CapturedPhysicalAttackAuthority::Automatic => PhysicalAttackAuthority::Automatic,
        };
        let current = self.capture_physical_attack_social_plan(
            plan.attacker_index,
            plan.defender_index,
            authority,
        )?;
        if current != plan.social {
            return Err(StepError::new(
                "physical attack social facts changed before commit",
            ));
        }
        self.commit_attack_relations(&plan.social.relations, events)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{
        ActorKind, CharacterAlignment, PhysicalAttackMode, SocialAlignmentSource, SocialBehavior,
        SocialNature,
    };

    use super::{CapturedPhysicalAttackAuthority, Engine};

    fn same_tile_engine() -> Engine {
        let mut engine = crate::engine::setup::test_engine("character_sheet");
        engine.world.actors[0].location.position = engine.world.actors[1].location.position;
        engine
    }

    #[test]
    fn captured_physical_social_identity_is_rechecked_before_relation_mutation() {
        let mut engine = same_tile_engine();
        let plan = engine
            .physical_attack_plan(
                0,
                1,
                PhysicalAttackMode::Fight,
                crate::model::HostilityAuthorization::Safe,
            )
            .expect("chaotic target should accept non-forced fight plan");
        assert!(matches!(
            plan.social.authority,
            CapturedPhysicalAttackAuthority::Player {
                authorization: crate::model::HostilityAuthorization::Safe,
                ..
            }
        ));

        engine.world.actors[1].social.alignment_source = SocialAlignmentSource::Inherent {
            alignment: CharacterAlignment::Evil,
        };
        let ledger_before = engine.world.social_relations.clone();
        let mut events = Vec::new();
        let error = engine
            .commit_physical_attack_social_plan(&plan, &mut events)
            .expect_err("changed true identity must stale the physical social plan");

        assert_eq!(
            error.message(),
            "physical attack social facts changed before commit"
        );
        assert_eq!(engine.world.social_relations, ledger_before);
        assert!(events.is_empty());
    }

    #[test]
    fn automatic_physical_plan_has_no_force_path_and_captures_relation_state() {
        let mut engine = same_tile_engine();
        engine.world.actors[1].kind = ActorKind::Npc;
        engine.world.actors[1].social.alignment_source = SocialAlignmentSource::Inherent {
            alignment: CharacterAlignment::Lawful,
        };
        engine.world.actors[1].social.nature = SocialNature::Human;
        engine.world.actors[1].social.behavior = SocialBehavior::Civilian;

        let plan = engine
            .automatic_physical_attack_plan(0, 1, PhysicalAttackMode::Fight)
            .expect("automatic plan should not require player confirmation");
        assert_eq!(
            plan.social.authority,
            CapturedPhysicalAttackAuthority::Automatic
        );
        let relation = plan
            .social
            .relations
            .npc_grudge
            .as_ref()
            .expect("lawful human NPC should capture a grudge change")
            .relation
            .clone();
        engine.world.social_relations.npc_grudges.insert(relation);
        let ledger_before = engine.world.social_relations.clone();
        let mut events = Vec::new();
        let error = engine
            .commit_physical_attack_social_plan(&plan, &mut events)
            .expect_err("changed relation state must stale the physical social plan");

        assert_eq!(
            error.message(),
            "physical attack social facts changed before commit"
        );
        assert_eq!(engine.world.social_relations, ledger_before);
        assert!(events.is_empty());
    }
}
