use crate::events::Event;
use crate::model::{
    BurdenTier, LogicalTime, MovementExertion, MovementPace, PhysicalAttackMode, ResourceActivity,
    ResourceKind,
};
use crate::view::BurdenViewV1;

use super::inventory::MpRecoverySelection;
use super::{Engine, StepError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ResourceDelta {
    pub(super) before: i32,
    pub(super) current: i32,
    pub(super) maximum: i32,
    pub(super) actual: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryEventReceipt {
    resource: ResourceKind,
    activity: ResourceActivity,
    boundary_at: LogicalTime,
    base_amount: i32,
    modifier: Option<MpRecoverySelection>,
    delta: ResourceDelta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LevelGrowthReceipt {
    pub(super) hp: i32,
    pub(super) max_hp: i32,
    pub(super) peak_hp: i32,
    pub(super) mp: i32,
    pub(super) max_mp: i32,
    pub(super) stamina: i32,
    pub(super) max_stamina: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MovementResourcePlan {
    pub(super) burden: BurdenViewV1,
    pub(super) exertion: MovementExertion,
    pub(super) stamina_before: Option<i32>,
    pub(super) stamina_cost: Option<i32>,
    pub(super) stamina_after: Option<i32>,
}

impl Engine {
    pub(super) fn commit_physical_stamina(
        &mut self,
        actor_index: usize,
        mode: PhysicalAttackMode,
        cost: i32,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        if cost <= 0 {
            return Ok(());
        }
        if self.world.actors[actor_index].stamina < cost {
            return Err(StepError::new("not enough stamina for physical attack"));
        }
        let delta = self.change_stamina(actor_index, -cost)?;
        let actor = &self.world.actors[actor_index];
        events.push(Event::PhysicalStaminaSpent {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            mode,
            amount: -delta.actual,
            stamina: delta.current,
            max_stamina: delta.maximum,
        });
        Ok(())
    }

    fn actor_mut_for_resource(
        &mut self,
        actor_index: usize,
    ) -> Result<&mut crate::model::ActorState, StepError> {
        self.world
            .actors
            .get_mut(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))
    }

    pub(super) fn set_hp(
        &mut self,
        actor_index: usize,
        value: i32,
    ) -> Result<ResourceDelta, StepError> {
        let actor = self.actor_mut_for_resource(actor_index)?;
        let maximum = actor.max_hp();
        let before = actor.hp;
        actor.hp = value.clamp(0, maximum);
        if let Some(character) = &mut actor.character {
            character.resources.hp = actor.hp;
        }
        Ok(ResourceDelta {
            before,
            current: actor.hp,
            maximum,
            actual: actor.hp - before,
        })
    }

    pub(super) fn change_hp(
        &mut self,
        actor_index: usize,
        amount: i32,
    ) -> Result<ResourceDelta, StepError> {
        let current = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?
            .hp;
        self.set_hp(actor_index, current.saturating_add(amount))
    }

    pub(super) fn set_mp(
        &mut self,
        actor_index: usize,
        value: i32,
    ) -> Result<ResourceDelta, StepError> {
        let actor = self.actor_mut_for_resource(actor_index)?;
        let maximum = actor
            .character
            .as_ref()
            .map_or(0, |character| character.resources.max_mp);
        let before = actor.mp;
        actor.mp = value.clamp(0, maximum);
        if let Some(character) = &mut actor.character {
            character.resources.mp = actor.mp;
        }
        Ok(ResourceDelta {
            before,
            current: actor.mp,
            maximum,
            actual: actor.mp - before,
        })
    }

    pub(super) fn change_mp(
        &mut self,
        actor_index: usize,
        amount: i32,
    ) -> Result<ResourceDelta, StepError> {
        let current = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?
            .mp;
        self.set_mp(actor_index, current.saturating_add(amount))
    }

    pub(super) fn set_stamina(
        &mut self,
        actor_index: usize,
        value: i32,
    ) -> Result<ResourceDelta, StepError> {
        let actor = self.actor_mut_for_resource(actor_index)?;
        let maximum = actor.max_stamina();
        let before = actor.stamina;
        actor.stamina = value.clamp(0, maximum);
        if let Some(character) = &mut actor.character {
            character.resources.stamina = actor.stamina;
        }
        Ok(ResourceDelta {
            before,
            current: actor.stamina,
            maximum,
            actual: actor.stamina - before,
        })
    }

    pub(super) fn change_stamina(
        &mut self,
        actor_index: usize,
        amount: i32,
    ) -> Result<ResourceDelta, StepError> {
        let current = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?
            .stamina;
        self.set_stamina(actor_index, current.saturating_add(amount))
    }

    pub(super) fn apply_level_growth(
        &mut self,
        actor_index: usize,
        hp_growth: i32,
        mp_growth: i32,
        stamina_growth: i32,
    ) -> Result<LevelGrowthReceipt, StepError> {
        if hp_growth <= 0 || mp_growth < 0 || stamina_growth <= 0 {
            return Err(StepError::new(
                "level growth requires positive HP/stamina and non-negative MP deltas",
            ));
        }
        let actor = self.actor_mut_for_resource(actor_index)?;
        let character = actor
            .character
            .as_mut()
            .ok_or_else(|| StepError::new("actor has no character resources"))?;
        let resources = &character.resources;
        if actor.hp != resources.hp
            || actor.mp != resources.mp
            || actor.stamina != resources.stamina
        {
            return Err(StepError::new(
                "actor and character resource mirrors must agree before level growth",
            ));
        }
        let receipt = LevelGrowthReceipt {
            hp: resources
                .hp
                .checked_add(hp_growth)
                .ok_or_else(|| StepError::new("level HP growth overflow"))?,
            max_hp: resources
                .max_hp
                .checked_add(hp_growth)
                .ok_or_else(|| StepError::new("level maximum HP growth overflow"))?,
            peak_hp: resources
                .peak_hp
                .checked_add(hp_growth)
                .ok_or_else(|| StepError::new("level peak HP growth overflow"))?,
            mp: resources
                .mp
                .checked_add(mp_growth)
                .ok_or_else(|| StepError::new("level MP growth overflow"))?,
            max_mp: resources
                .max_mp
                .checked_add(mp_growth)
                .ok_or_else(|| StepError::new("level maximum MP growth overflow"))?,
            stamina: resources
                .stamina
                .checked_add(stamina_growth)
                .ok_or_else(|| StepError::new("level stamina growth overflow"))?,
            max_stamina: resources
                .max_stamina
                .checked_add(stamina_growth)
                .ok_or_else(|| StepError::new("level maximum stamina growth overflow"))?,
        };
        if receipt.hp > receipt.max_hp
            || receipt.max_hp > receipt.peak_hp
            || receipt.mp > receipt.max_mp
            || receipt.stamina > receipt.max_stamina
        {
            return Err(StepError::new(
                "level growth would violate resource invariants",
            ));
        }

        character.resources.hp = receipt.hp;
        character.resources.max_hp = receipt.max_hp;
        character.resources.peak_hp = receipt.peak_hp;
        character.resources.mp = receipt.mp;
        character.resources.max_mp = receipt.max_mp;
        character.resources.stamina = receipt.stamina;
        character.resources.max_stamina = receipt.max_stamina;
        actor.hp = receipt.hp;
        actor.mp = receipt.mp;
        actor.stamina = receipt.stamina;
        Ok(receipt)
    }

    pub(super) fn classify_burden(&self, actor_index: usize) -> Result<BurdenViewV1, StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        let raw = self.actor_burden(actor_index)?;
        let Some(character) = &actor.character else {
            return Ok(BurdenViewV1 {
                item_burden: raw.item_burden,
                coin_burden: raw.coin_burden,
                total_burden: raw.total_burden,
                lightly_loaded_limit: None,
                moderately_loaded_limit: None,
                heavily_loaded_limit: None,
                tier: None,
            });
        };
        let strength = u64::try_from(character.attributes.strength)
            .map_err(|_| StepError::new("character strength cannot be negative"))?;
        let rules = &self.definition.catalog.rules.burden;
        let lightly_loaded_limit = rules
            .lightly_loaded_max_per_strength
            .checked_mul(strength)
            .ok_or_else(|| StepError::new("lightly loaded burden threshold overflow"))?;
        let moderately_loaded_limit = rules
            .moderately_loaded_max_per_strength
            .checked_mul(strength)
            .ok_or_else(|| StepError::new("moderately loaded burden threshold overflow"))?;
        let heavily_loaded_limit = rules
            .heavily_loaded_max_per_strength
            .checked_mul(strength)
            .ok_or_else(|| StepError::new("heavily loaded burden threshold overflow"))?;
        let tier = if raw.total_burden <= lightly_loaded_limit {
            BurdenTier::LightlyLoaded
        } else if raw.total_burden <= moderately_loaded_limit {
            BurdenTier::ModeratelyLoaded
        } else if raw.total_burden <= heavily_loaded_limit {
            BurdenTier::HeavilyLoaded
        } else {
            BurdenTier::VeryHeavilyLoaded
        };
        Ok(BurdenViewV1 {
            item_burden: raw.item_burden,
            coin_burden: raw.coin_burden,
            total_burden: raw.total_burden,
            lightly_loaded_limit: Some(lightly_loaded_limit),
            moderately_loaded_limit: Some(moderately_loaded_limit),
            heavily_loaded_limit: Some(heavily_loaded_limit),
            tier: Some(tier),
        })
    }

    pub(super) fn movement_resource_plan(
        &self,
        actor_index: usize,
        pace: MovementPace,
        accepted_steps: usize,
        difficult_terrain_committed: bool,
    ) -> Result<MovementResourcePlan, StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        let burden = self.classify_burden(actor_index)?;
        let Some(tier) = burden.tier else {
            return Ok(MovementResourcePlan {
                burden,
                exertion: MovementExertion::None,
                stamina_before: None,
                stamina_cost: None,
                stamina_after: None,
            });
        };

        let rapid = accepted_steps > 0
            && matches!(pace, MovementPace::Run | MovementPace::Sprint)
            && (difficult_terrain_committed || actor.hp.saturating_mul(2) <= actor.max_hp());
        let normal = accepted_steps > 0
            && (matches!(
                (pace, tier),
                (MovementPace::Walk, BurdenTier::VeryHeavilyLoaded)
            ) || (matches!(pace, MovementPace::Run | MovementPace::Sprint)
                && matches!(
                    tier,
                    BurdenTier::ModeratelyLoaded
                        | BurdenTier::HeavilyLoaded
                        | BurdenTier::VeryHeavilyLoaded
                )));
        let exertion = if rapid {
            MovementExertion::Rapid
        } else if normal {
            MovementExertion::Normal
        } else {
            MovementExertion::None
        };
        let selected_cost = match exertion {
            MovementExertion::None => 0,
            MovementExertion::Normal => {
                self.definition
                    .catalog
                    .rules
                    .resources
                    .normal_movement_stamina_cost
            }
            MovementExertion::Rapid => {
                self.definition
                    .catalog
                    .rules
                    .resources
                    .rapid_movement_stamina_cost
            }
        };
        let stamina_before = actor.stamina;
        Ok(MovementResourcePlan {
            burden,
            exertion,
            stamina_before: Some(stamina_before),
            stamina_cost: Some(selected_cost),
            stamina_after: Some(stamina_before.saturating_sub(selected_cost).max(0)),
        })
    }

    pub(super) fn commit_movement_stamina(
        &mut self,
        actor_index: usize,
        pace: MovementPace,
        plan: &MovementResourcePlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let Some(selected_cost) = plan.stamina_cost else {
            return Ok(());
        };
        if selected_cost == 0 {
            return Ok(());
        }
        let delta = self.change_stamina(actor_index, -selected_cost)?;
        if delta.actual == 0 {
            return Ok(());
        }
        let actor = &self.world.actors[actor_index];
        events.push(Event::MovementStaminaSpent {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            pace,
            exertion: plan.exertion,
            amount: -delta.actual,
            stamina: delta.current,
            max_stamina: delta.maximum,
        });
        Ok(())
    }

    pub(super) fn mark_actor_resource_active(
        &mut self,
        actor_index: usize,
    ) -> Result<(), StepError> {
        let current_time = self.current_time();
        self.actor_mut_for_resource(actor_index)?
            .resource_activity
            .last_active_at = Some(current_time);
        Ok(())
    }

    pub(super) fn apply_actor_resource_recovery(
        &mut self,
        actor_index: usize,
        boundary_at: LogicalTime,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        if !actor.is_alive() || actor.character.is_none() {
            return Ok(());
        }
        let interval = self
            .definition
            .catalog
            .rules
            .resources
            .recovery_interval_units;
        let previous = actor.resource_activity.last_recovered_at;
        if boundary_at.elapsed_rounds_since(previous) < u64::from(interval) {
            return Ok(());
        }
        let activity = if actor
            .resource_activity
            .last_active_at
            .is_some_and(|at| at >= previous)
        {
            ResourceActivity::Active
        } else {
            ResourceActivity::Inactive
        };
        self.world.actors[actor_index]
            .resource_activity
            .last_recovered_at = boundary_at;
        let hp_amount = match activity {
            ResourceActivity::Active => self.definition.catalog.rules.resources.active_hp_recovery,
            ResourceActivity::Inactive => {
                self.definition.catalog.rules.resources.inactive_hp_recovery
            }
        };

        let hp = self.change_hp(actor_index, hp_amount)?;
        self.push_recovery_event(
            actor_index,
            RecoveryEventReceipt {
                resource: ResourceKind::Hp,
                activity,
                boundary_at,
                base_amount: hp_amount,
                modifier: None,
                delta: hp,
            },
            events,
        );

        let mp_base_amount = self.definition.catalog.rules.resources.mp_recovery;
        let mp_modifier = self.highest_worn_mp_recovery_multiplier(actor_index)?;
        let (numerator, denominator) = mp_modifier.as_ref().map_or((1, 1), |modifier| {
            (modifier.numerator, modifier.denominator)
        });
        let mp_amount_i64 = i64::from(mp_base_amount)
            .checked_mul(i64::from(numerator))
            .ok_or_else(|| StepError::new("MP recovery multiplier overflow"))?
            / i64::from(denominator);
        let mp_amount = i32::try_from(mp_amount_i64)
            .map_err(|_| StepError::new("MP recovery exceeds supported range"))?;
        if mp_amount < mp_base_amount {
            return Err(StepError::new(
                "MP recovery modifier must not reduce recovery",
            ));
        }
        let mp = self.change_mp(actor_index, mp_amount)?;
        self.push_recovery_event(
            actor_index,
            RecoveryEventReceipt {
                resource: ResourceKind::Mp,
                activity,
                boundary_at,
                base_amount: mp_base_amount,
                modifier: mp_modifier,
                delta: mp,
            },
            events,
        );

        let full_hp = self.world.actors[actor_index].hp == self.world.actors[actor_index].max_hp();
        if activity == ResourceActivity::Inactive && full_hp {
            let stamina_amount = self
                .definition
                .catalog
                .rules
                .resources
                .inactive_stamina_recovery;
            let stamina = self.change_stamina(actor_index, stamina_amount)?;
            self.push_recovery_event(
                actor_index,
                RecoveryEventReceipt {
                    resource: ResourceKind::Stamina,
                    activity,
                    boundary_at,
                    base_amount: stamina_amount,
                    modifier: None,
                    delta: stamina,
                },
                events,
            );
        }
        Ok(())
    }

    fn push_recovery_event(
        &self,
        actor_index: usize,
        receipt: RecoveryEventReceipt,
        events: &mut Vec<Event>,
    ) {
        if receipt.delta.actual <= 0 {
            return;
        }
        let actor = &self.world.actors[actor_index];
        let modifier = receipt.modifier;
        events.push(Event::ResourceRegenerated {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            resource: receipt.resource,
            activity: receipt.activity,
            boundary_at: receipt.boundary_at,
            base_amount: receipt.base_amount,
            multiplier_numerator: modifier.as_ref().map_or(1, |item| item.numerator),
            multiplier_denominator: modifier.as_ref().map_or(1, |item| item.denominator),
            rounding: crate::model::MagicArithmeticRounding::Down,
            modifier_item_instance_id: modifier.as_ref().map(|item| item.item_instance_id.clone()),
            modifier_item_definition_id: modifier
                .as_ref()
                .map(|item| item.item_definition_id.clone()),
            modifier_item: modifier.as_ref().map(|item| item.item_name.clone()),
            modifier_item_position: modifier.as_ref().map(|item| item.position),
            amount: receipt.delta.actual,
            current: receipt.delta.current,
            maximum: receipt.delta.maximum,
        });
    }
}
