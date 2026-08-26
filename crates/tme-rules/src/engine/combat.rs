use super::death::DefeatContext;
use super::physical_attacks::{PhysicalAttackAuthority, PhysicalAttackPlan};
use super::{Engine, StepError};
use crate::combat::DamageLabel;
use crate::events::{ArmorProtectionSourceEventV1, Event, ItemRelocationReason};
use crate::model::{
    BlockSourceKind, BowReadiness, BowReadinessChangeReason, CarriedPosition, Coord, DeathCause,
    ItemLocation, PhysicalAttackMode, PhysicalAttackOutcome, WeaponFumbleReason, WorldPosition,
    WoundState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PhysicalAttackResolution {
    MartialHandBlock,
    ShieldBlock,
    ArmorBlock,
    Miss,
    DamagingHit { defeated: bool },
    Fumble { practice_eligible: bool },
}

impl PhysicalAttackResolution {
    pub(super) const fn outcome(self) -> PhysicalAttackOutcome {
        match self {
            Self::MartialHandBlock | Self::ShieldBlock => PhysicalAttackOutcome::HandBlocked,
            Self::ArmorBlock => PhysicalAttackOutcome::ArmorBlocked,
            Self::Miss => PhysicalAttackOutcome::Miss,
            Self::DamagingHit { defeated: true } => PhysicalAttackOutcome::FatalBlow,
            Self::DamagingHit { defeated: false } => PhysicalAttackOutcome::DamagingHit,
            Self::Fumble { .. } => PhysicalAttackOutcome::Fumble,
        }
    }

    pub(super) const fn practice_eligible(self) -> bool {
        !matches!(
            self,
            Self::Fumble {
                practice_eligible: false
            }
        )
    }
}

fn armor_event_sources(plan: &PhysicalAttackPlan) -> Vec<ArmorProtectionSourceEventV1> {
    plan.defender_armor
        .sources
        .iter()
        .map(|source| ArmorProtectionSourceEventV1 {
            carried_position: source.carried_position,
            item_instance_id: source.item_instance_id.clone(),
            item_definition_id: source.item_definition_id.clone(),
            block_rating: source.block_rating,
            encumbrance: source.encumbrance,
            cutting_reduction: source.cutting_reduction,
            piercing_reduction: source.piercing_reduction,
            crushing_reduction: source.crushing_reduction,
        })
        .collect()
}

fn wound_state_for_hp(hp: i32, max_hp: i32, rules: &crate::combat::CombatWoundRules) -> WoundState {
    if hp <= 0 {
        return WoundState::Dead;
    }
    let percent = i64::from(hp.max(0)).saturating_mul(100) / i64::from(max_hp.max(1));
    if percent <= i64::from(rules.near_death_max_percent) {
        WoundState::NearDeath
    } else if percent <= i64::from(rules.badly_wounded_max_percent) {
        WoundState::BadlyWounded
    } else if percent <= i64::from(rules.wounded_max_percent) {
        WoundState::Wounded
    } else {
        WoundState::Unhurt
    }
}

impl Engine {
    pub(super) fn apply_player_physical_attack(
        &mut self,
        player_index: usize,
        mode: PhysicalAttackMode,
        target_id: &crate::model::ActorId,
        authorization: crate::model::HostilityAuthorization,
        events: &mut Vec<Event>,
    ) -> Result<bool, StepError> {
        let target_index = self.live_actor_by_id(target_id).ok_or_else(|| {
            StepError::new(format!(
                "physical attack target {target_id:?} was not found"
            ))
        })?;
        match self.physical_attack_plan(player_index, target_index, mode, authorization) {
            Ok(_) => {}
            Err(error) if error.message().contains("not visible") => {
                events.push(Event::AttackBlockedNoSight {
                    attacker_id: self.world.actors[player_index].id.clone(),
                    attacker: self.world.actors[player_index].name.clone(),
                    attacker_site: self.world.actors[player_index].location.site(),
                    defender_id: self.world.actors[target_index].id.clone(),
                    defender: self.world.actors[target_index].name.clone(),
                    mode,
                });
                return Ok(false);
            }
            Err(error) => return Err(error),
        }

        let event_start = events.len();
        self.attack_if_ready(
            player_index,
            target_index,
            mode,
            PhysicalAttackAuthority::Player { authorization },
            events,
        )?;
        Ok(events[event_start..].iter().any(|event| {
            matches!(
                event,
                Event::Attacked { .. }
                    | Event::AttackMissed { .. }
                    | Event::AttackBlocked { .. }
                    | Event::WeaponFumbled { .. }
            )
        }))
    }

    pub(super) fn resolve_physical_attack(
        &mut self,
        plan: &PhysicalAttackPlan,
        events: &mut Vec<Event>,
    ) -> Result<Option<PhysicalAttackResolution>, StepError> {
        let attacker_index = plan.attacker_index;
        let defender_index = plan.defender_index;
        if !self.world.actors[attacker_index].is_alive()
            || !self.world.actors[defender_index].is_alive()
        {
            return Ok(None);
        }

        let attacker_name = self.world.actors[attacker_index].name.clone();
        let defender_name = self.world.actors[defender_index].name.clone();
        let attack = self.world.actors[attacker_index].stats.attack;
        let defense = self.world.actors[defender_index].stats.defense;
        let strength_adds = self.world.actors[attacker_index]
            .character
            .as_ref()
            .map(|character| character.physical_attribute_adds.strength_adds)
            .unwrap_or(0);
        let defender_location = self.world.actors[defender_index].location.clone();

        let block_candidates = self.block_candidates(
            defender_index,
            plan.defender_armor.encumbrance,
            plan.effective_combat_add_rating,
        )?;
        if let Some(candidate) = self.choose_block_candidate(&block_candidates)? {
            let block_roll = self.rng.roll_d20();
            if block_roll.saturating_mul(5) < candidate.chance_percent {
                let source = candidate.source;
                events.push(Event::AttackBlocked {
                    attacker_id: self.world.actors[attacker_index].id.clone(),
                    attacker: attacker_name.clone(),
                    defender_id: self.world.actors[defender_index].id.clone(),
                    defender: defender_name.clone(),
                    defender_location: defender_location.clone(),
                    mode: plan.mode,
                    damage_kind: plan.damage_kind,
                    effective_combat_add_rating: plan.effective_combat_add_rating,
                    armor_encumbrance: plan.defender_armor.encumbrance,
                    source,
                    carried_position: candidate.carried_position,
                    item_instance_id: candidate.item_instance_id,
                    block_value: candidate.block_value,
                    skill_track_id: candidate.skill_track_id,
                    skill_level: candidate.skill_level,
                    roll: block_roll,
                    chance_percent: candidate.chance_percent,
                    armor_sources: armor_event_sources(plan),
                });
                return Ok(Some(if source == BlockSourceKind::RightMartialHand {
                    PhysicalAttackResolution::MartialHandBlock
                } else {
                    PhysicalAttackResolution::ShieldBlock
                }));
            }
        }

        if plan.defender_armor.block_rating > 0 {
            let block_rules = &self.definition.catalog.rules.combat.block;
            let base_block = u32::try_from(plan.defender_armor.block_rating)
                .unwrap_or(0)
                .checked_mul(block_rules.armor_percent_per_point)
                .ok_or_else(|| StepError::new("armor block chance overflow"))?
                .min(block_rules.armor_percent_cap);
            let strength_penetration = u32::try_from(strength_adds)
                .unwrap_or(0)
                .checked_mul(block_rules.strength_penetration_percent_per_add)
                .ok_or_else(|| StepError::new("strength block penetration overflow"))?;
            let combat_add_penetration = u32::try_from(plan.effective_combat_add_rating)
                .unwrap_or(0)
                .checked_mul(block_rules.combat_add_penetration_percent_per_rating)
                .ok_or_else(|| StepError::new("combat-add armor penetration overflow"))?;
            let penetration = strength_penetration
                .checked_add(combat_add_penetration)
                .ok_or_else(|| StepError::new("armor penetration overflow"))?;
            let chance_percent = base_block.saturating_sub(penetration);
            let roll = self.rng.roll_d20();
            if roll.saturating_mul(5) < chance_percent {
                events.push(Event::AttackBlocked {
                    attacker_id: self.world.actors[attacker_index].id.clone(),
                    attacker: attacker_name.clone(),
                    defender_id: self.world.actors[defender_index].id.clone(),
                    defender: defender_name.clone(),
                    defender_location: defender_location.clone(),
                    mode: plan.mode,
                    damage_kind: plan.damage_kind,
                    effective_combat_add_rating: plan.effective_combat_add_rating,
                    armor_encumbrance: plan.defender_armor.encumbrance,
                    source: BlockSourceKind::Armor,
                    carried_position: None,
                    item_instance_id: None,
                    block_value: plan.defender_armor.block_rating,
                    skill_track_id: None,
                    skill_level: None,
                    roll,
                    chance_percent,
                    armor_sources: armor_event_sources(plan),
                });
                return Ok(Some(PhysicalAttackResolution::ArmorBlock));
            }
        }

        let hit_roll = self.rng.roll_d20();
        let dexterity_adds = self.world.actors[attacker_index]
            .character
            .as_ref()
            .map(|character| character.physical_attribute_adds.dexterity_adds)
            .unwrap_or(0);
        let attacker_score = attack
            / self
                .definition
                .catalog
                .rules
                .combat
                .hit
                .attacker_attack_stat_divisor
            + plan.effective_combat_add_rating
            + i32::from(plan.skill_level)
                / self
                    .definition
                    .catalog
                    .rules
                    .combat
                    .hit
                    .attacker_skill_level_divisor
            + dexterity_adds;
        let defender_dexterity = self.world.actors[defender_index]
            .character
            .as_ref()
            .map(|character| character.attributes.dexterity)
            .unwrap_or(
                self.definition
                    .catalog
                    .rules
                    .combat
                    .hit
                    .non_character_defender_dexterity,
            );
        let defender_score = self.definition.catalog.rules.combat.hit.base_defender_score
            + defense
                / self
                    .definition
                    .catalog
                    .rules
                    .combat
                    .hit
                    .defender_defense_stat_divisor
            + defender_dexterity
                / self
                    .definition
                    .catalog
                    .rules
                    .combat
                    .hit
                    .defender_dexterity_divisor;
        if i32::try_from(hit_roll).unwrap_or(i32::MAX) + attacker_score <= defender_score {
            events.push(Event::AttackMissed {
                attacker_id: self.world.actors[attacker_index].id.clone(),
                attacker: attacker_name,
                defender_id: self.world.actors[defender_index].id.clone(),
                defender: defender_name,
                defender_location: defender_location.clone(),
                mode: plan.mode,
                damage_kind: plan.damage_kind,
                effective_combat_add_rating: plan.effective_combat_add_rating,
                attacker_score,
                defender_score,
                roll: i32::try_from(hit_roll).unwrap_or(i32::MAX),
            });
            return Ok(Some(PhysicalAttackResolution::Miss));
        }

        let damage_before_armor = (attack + plan.effective_combat_add_rating + strength_adds
            - defense
            + i32::try_from(
                hit_roll
                    % self
                        .definition
                        .catalog
                        .rules
                        .combat
                        .damage
                        .roll_variation_modulus,
            )
            .unwrap_or(0))
        .max(self.definition.catalog.rules.combat.damage.minimum_damage);
        let armor_reduction = plan
            .defender_armor
            .reduction_for(plan.damage_kind)
            .min(damage_before_armor - self.definition.catalog.rules.combat.damage.minimum_damage)
            .max(0);
        let damage_after_armor = damage_before_armor - armor_reduction;
        if armor_reduction > 0 {
            events.push(Event::ProtectionApplied {
                attacker_id: self.world.actors[attacker_index].id.clone(),
                attacker: attacker_name.clone(),
                defender_id: self.world.actors[defender_index].id.clone(),
                defender: defender_name.clone(),
                damage_kind: plan.damage_kind,
                amount: armor_reduction,
                armor_sources: armor_event_sources(plan),
            });
        }

        let (affinity_numerator, affinity_denominator) = self.world.actors[defender_index]
            .physical_damage_affinity
            .response(plan.damage_kind);
        let adjusted = u64::try_from(damage_after_armor)
            .map_err(|_| StepError::new("physical affinity input damage must be non-negative"))?
            .checked_mul(u64::from(affinity_numerator))
            .ok_or_else(|| StepError::new("physical affinity damage multiplication overflow"))?
            / u64::from(affinity_denominator);
        let damage = i32::try_from(adjusted)
            .map_err(|_| StepError::new("physical affinity adjusted damage exceeds i32"))?;
        if (affinity_numerator, affinity_denominator) != (1, 1) {
            events.push(Event::PhysicalDamageAffinityApplied {
                defender_id: self.world.actors[defender_index].id.clone(),
                defender: defender_name.clone(),
                damage_kind: plan.damage_kind,
                input_damage: damage_after_armor,
                numerator: affinity_numerator,
                denominator: affinity_denominator,
                adjusted_damage: damage,
            });
        }

        let attacker_id = self.world.actors[attacker_index].id.clone();
        let defender_id = self.world.actors[defender_index].id.clone();
        let defender_max_hp = self.world.actors[defender_index].max_hp();
        let damage_rules = self.definition.catalog.rules.combat.damage.clone();
        let wound_rules = self.definition.catalog.rules.combat.wounds.clone();
        let mode = plan.mode;
        let damage_kind = plan.damage_kind;
        let effective_combat_add_rating = plan.effective_combat_add_rating;
        let wound_before = plan.defender_wound_before;
        let outcome = self.apply_damage_and_resolve_defeat(
            defender_index,
            damage,
            DefeatContext {
                cause: DeathCause::Physical,
                credited_actor_id: Some(attacker_id.clone()),
                direct_social_actor_id: Some(attacker_id.clone()),
                spell_damage_credit: None,
                hostile_authority: None,
            },
            events,
            move |outcome| Event::Attacked {
                attacker_id,
                attacker: attacker_name,
                defender_id,
                defender: defender_name,
                defender_location,
                mode,
                damage_kind,
                effective_combat_add_rating,
                roll: hit_roll,
                damage: outcome.requested,
                armor_reduction,
                label: DamageLabel::for_hit(
                    outcome.requested,
                    outcome.hp_before,
                    outcome.hp_after,
                    damage_rules.moderate_label_min_percent,
                    damage_rules.heavy_label_min_percent,
                    damage_rules.severe_label_min_percent,
                ),
                wound_before,
                wound_after: wound_state_for_hp(outcome.hp_after, defender_max_hp, &wound_rules),
                defender_hp: outcome.hp_after,
            },
            |_| {},
        )?;
        Ok(Some(PhysicalAttackResolution::DamagingHit {
            defeated: outcome.defeated,
        }))
    }

    pub(super) fn attack_if_ready(
        &mut self,
        attacker_index: usize,
        defender_index: usize,
        mode: PhysicalAttackMode,
        authority: PhysicalAttackAuthority,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let current_time = self.current_time();
        let ready_at = self.world.actors[attacker_index].attack_ready_at;
        if current_time < ready_at {
            events.push(Event::AttackNotReady {
                actor_id: self.world.actors[attacker_index].id.clone(),
                actor: self.world.actors[attacker_index].name.clone(),
                target_id: self.world.actors[defender_index].id.clone(),
                target: self.world.actors[defender_index].name.clone(),
                current_time,
                ready_at,
                mode,
            });
            return Ok(());
        }

        let plan = match authority {
            PhysicalAttackAuthority::Player { authorization } => {
                self.physical_attack_plan(attacker_index, defender_index, mode, authorization)?
            }
            PhysicalAttackAuthority::Automatic => {
                self.automatic_physical_attack_plan(attacker_index, defender_index, mode)?
            }
        };

        if plan.is_weapon_backed() {
            if let Some(reason) = self.selected_weapon_restriction(attacker_index, &plan.selection)
            {
                self.resolve_weapon_fumble(attacker_index, &plan.selection, mode, reason, events)?;
                let resolution = PhysicalAttackResolution::Fumble {
                    practice_eligible: false,
                };
                self.award_physical_attack_practice(&plan, resolution, events)?;
                self.world.actors[attacker_index].attack_ready_at =
                    self.logical_time_after(plan.cooldown_units);
                return Ok(());
            }
            let chance = self.general_fumble_percent(attacker_index, &plan.selection);
            if self.rng.roll_percent() <= chance {
                self.resolve_weapon_fumble(
                    attacker_index,
                    &plan.selection,
                    mode,
                    WeaponFumbleReason::General,
                    events,
                )?;
                let resolution = PhysicalAttackResolution::Fumble {
                    practice_eligible: true,
                };
                self.award_physical_attack_practice(&plan, resolution, events)?;
                self.world.actors[attacker_index].attack_ready_at =
                    self.logical_time_after(plan.cooldown_units);
                return Ok(());
            }
        }

        self.commit_physical_attack_social_plan(&plan, events)?;
        self.commit_physical_stamina(attacker_index, mode, plan.jumpkick_stamina_cost, events)?;
        if plan.consumes_bow_nock {
            let item_instance_id = plan
                .selection
                .item_instance_id
                .as_deref()
                .expect("shoot plan has a selected bow");
            self.change_bow_readiness(
                attacker_index,
                item_instance_id,
                BowReadiness::Unnocked,
                BowReadinessChangeReason::Shot,
                events,
            )?;
        }

        let landing_position = self.world.actors[defender_index].location.position;
        let resolution = self.resolve_physical_attack(&plan, events)?;
        if let Some(resolution) = resolution {
            self.award_physical_attack_practice(&plan, resolution, events)?;
        }
        self.world.actors[attacker_index].attack_ready_at =
            self.logical_time_after(plan.cooldown_units);
        if plan.releases_weapon {
            self.release_thrown_weapon(&plan, landing_position, events)?;
        }
        Ok(())
    }

    pub(super) fn release_thrown_weapon(
        &mut self,
        plan: &PhysicalAttackPlan,
        landing_position: Coord,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let weapon_instance_id = plan
            .selection
            .item_instance_id
            .as_deref()
            .ok_or_else(|| StepError::new("throw plan has no selected weapon"))?;
        let attacker_index = plan.attacker_index;
        let location = self.world.actors[attacker_index].location.clone();
        let holder = self.item_holder_for_actor_index(attacker_index)?;
        self.relocate_item_with_event(
            attacker_index,
            weapon_instance_id,
            ItemLocation::Carried {
                holder,
                position: CarriedPosition::RightHand,
            },
            ItemLocation::Ground {
                position: WorldPosition::new(&location.realm, &location.level, landing_position),
            },
            ItemRelocationReason::Thrown,
            events,
        )
        .map_err(|_| StepError::new("thrown weapon is not in the right hand"))
    }
}
