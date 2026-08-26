use crate::content::ScavengingProfileDef;
use crate::events::{
    AutomaticActorDecisionV1, AutomaticMovementPurposeV1, Event, GoldRelocationReason,
    ItemRelocationReason,
};
use crate::model::{
    CarriedGoldPosition, CarriedPosition, GoldMoveDestination, GoldMoveQuantity, GoldMoveSource,
    ItemMoveDestination, WorldPosition,
};

use super::super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScavengingTarget {
    Corpse(crate::model::CorpseId, WorldPosition),
    Item(String, WorldPosition),
    Gold(crate::model::GoldPileId, WorldPosition, i64),
}

impl ScavengingTarget {
    fn location(&self) -> &WorldPosition {
        match self {
            Self::Corpse(_, location) | Self::Item(_, location) | Self::Gold(_, location, _) => {
                location
            }
        }
    }
}

impl Engine {
    fn scavenging_profile(&self, actor_index: usize) -> Option<ScavengingProfileDef> {
        let actor = self.world.actors.get(actor_index)?;
        self.definition
            .catalog
            .actor_definitions
            .get(&actor.definition_id)?
            .scavenging_profile
    }

    pub(super) fn try_automatic_balm(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<bool, StepError> {
        let Some(profile) = self.scavenging_profile(actor_index) else {
            return Ok(false);
        };
        let actor = &self.world.actors[actor_index];
        if !profile.uses_healing_balm
            || i64::from(actor.hp) * 100
                >= i64::from(actor.max_hp()) * i64::from(profile.balm_below_hp_percent)
        {
            return Ok(false);
        }
        let mut balm_ids = self
            .carried_item_ids(actor_index)?
            .into_iter()
            .filter(|id| {
                self.world
                    .item_instances
                    .get(id)
                    .is_some_and(|item| item.definition_id == "healing_balm")
            })
            .collect::<Vec<_>>();
        balm_ids.sort();
        let Some(item_instance_id) = balm_ids.first() else {
            return Ok(false);
        };
        let roll = self
            .rng
            .roll_bounded(u32::from(profile.balm_chance_denominator))
            .map_err(StepError::new)?;
        if roll > u32::from(profile.balm_chance_numerator) {
            return Ok(false);
        }
        self.emit_automatic_decision(
            actor_index,
            AutomaticActorDecisionV1::DrinkBalm {
                item_instance_id: item_instance_id.clone(),
            },
            events,
        );
        self.apply_actor_drink(actor_index, item_instance_id, events)?;
        Ok(true)
    }

    fn visible_scavenging_location(
        &self,
        actor_index: usize,
        location: &WorldPosition,
        profile: ScavengingProfileDef,
    ) -> bool {
        let actor = &self.world.actors[actor_index];
        actor.location.same_site(location)
            && actor
                .location
                .position
                .chebyshev_distance(location.position)
                <= i32::from(profile.search_radius)
            && actor.home_location.same_site(location)
            && actor
                .home_location
                .position
                .chebyshev_distance(location.position)
                <= actor
                    .ai
                    .as_ref()
                    .expect("automatic actor has AI")
                    .leash_range as i32
            && self.actor_can_see(actor_index, location)
    }

    fn scavenging_key(
        &self,
        actor_index: usize,
        location: &WorldPosition,
        stable_id: &str,
    ) -> (i32, String, i32, i32, String) {
        (
            self.world.actors[actor_index]
                .location
                .position
                .chebyshev_distance(location.position),
            location.level.clone(),
            location.position.y,
            location.position.x,
            stable_id.to_string(),
        )
    }

    fn select_scavenging_target(
        &self,
        actor_index: usize,
        profile: ScavengingProfileDef,
    ) -> Option<ScavengingTarget> {
        if profile.searches_corpses {
            let mut corpses = self
                .world
                .corpses
                .values()
                .filter(|corpse| {
                    !corpse.searched
                        && self.visible_scavenging_location(actor_index, &corpse.location, profile)
                })
                .collect::<Vec<_>>();
            corpses.sort_by_key(|corpse| {
                self.scavenging_key(actor_index, &corpse.location, corpse.id.as_str())
            });
            if let Some(corpse) = corpses.first() {
                return Some(ScavengingTarget::Corpse(
                    corpse.id.clone(),
                    corpse.location.clone(),
                ));
            }
        }
        if profile.collects_ground_items {
            let mut items = self
                .world
                .ground_items
                .iter()
                .filter(|item| {
                    self.visible_scavenging_location(actor_index, &item.location, profile)
                })
                .collect::<Vec<_>>();
            items.sort_by_key(|item| {
                self.scavenging_key(actor_index, &item.location, &item.item_instance_id)
            });
            if let Some(item) = items.first() {
                return Some(ScavengingTarget::Item(
                    item.item_instance_id.clone(),
                    item.location.clone(),
                ));
            }
        }
        if profile.collects_gold {
            let mut piles = self
                .world
                .ground_gold
                .values()
                .filter(|pile| {
                    self.visible_scavenging_location(actor_index, &pile.location, profile)
                })
                .collect::<Vec<_>>();
            piles.sort_by_key(|pile| {
                self.scavenging_key(actor_index, &pile.location, pile.id.as_str())
            });
            if let Some(pile) = piles.first() {
                return Some(ScavengingTarget::Gold(
                    pile.id.clone(),
                    pile.location.clone(),
                    pile.amount,
                ));
            }
        }
        None
    }

    fn scavenged_item_destination(
        &self,
        actor_index: usize,
        item_instance_id: &str,
        equips_items: bool,
    ) -> Option<CarriedPosition> {
        let definition = self.item_definition(item_instance_id).ok()?;
        let open_and_compatible = |position: &CarriedPosition| {
            !self.world.actors[actor_index]
                .carried
                .items
                .contains_key(position)
                && definition
                    .valid_placements
                    .contains(&position.placement_kind())
        };
        if equips_items {
            if definition.weapon.is_some() && open_and_compatible(&CarriedPosition::RightHand) {
                return Some(CarriedPosition::RightHand);
            }
            if definition.weapon.is_none()
                && let Some(position) = CarriedPosition::ALL.into_iter().find(|position| {
                    position.is_active_equipment() && open_and_compatible(position)
                })
            {
                return Some(position);
            }
        }
        CarriedPosition::ALL
            .into_iter()
            .find(|position| position.is_sack_item() && open_and_compatible(position))
    }

    pub(super) fn try_automatic_scavenging(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<bool, StepError> {
        let Some(profile) = self.scavenging_profile(actor_index) else {
            return Ok(false);
        };
        let Some(target) = self.select_scavenging_target(actor_index, profile) else {
            return Ok(false);
        };
        if self.world.actors[actor_index].location != *target.location() {
            if let Some(direction) = self.step_toward(actor_index, target.location().position) {
                self.commit_automatic_move(
                    actor_index,
                    direction,
                    AutomaticMovementPurposeV1::Scavenge,
                    events,
                )?;
                return Ok(true);
            }
            return Ok(false);
        }
        match target {
            ScavengingTarget::Corpse(corpse_id, _) => {
                self.emit_automatic_decision(
                    actor_index,
                    AutomaticActorDecisionV1::SearchCorpse {
                        corpse_id: corpse_id.clone(),
                    },
                    events,
                );
                self.apply_corpse_search(actor_index, &corpse_id, events)?;
            }
            ScavengingTarget::Item(item_instance_id, _) => {
                let Some(position) = self.scavenged_item_destination(
                    actor_index,
                    &item_instance_id,
                    profile.equips_items,
                ) else {
                    return Ok(false);
                };
                let destination = ItemMoveDestination::Carried { position };
                if self
                    .validate_item_move(actor_index, &item_instance_id, &destination)
                    .is_err()
                {
                    return Ok(false);
                }
                self.emit_automatic_decision(
                    actor_index,
                    AutomaticActorDecisionV1::CollectItem {
                        item_instance_id: item_instance_id.clone(),
                        destination: position,
                    },
                    events,
                );
                self.apply_actor_move_item(
                    actor_index,
                    &item_instance_id,
                    &destination,
                    ItemRelocationReason::Scavenging,
                    events,
                )?;
            }
            ScavengingTarget::Gold(gold_pile_id, _, amount) => {
                self.emit_automatic_decision(
                    actor_index,
                    AutomaticActorDecisionV1::CollectGold {
                        gold_pile_id: gold_pile_id.clone(),
                        amount,
                    },
                    events,
                );
                self.apply_actor_move_gold(
                    actor_index,
                    &GoldMoveSource::Ground { gold_pile_id },
                    &GoldMoveDestination::Carried {
                        position: CarriedGoldPosition::Sack,
                    },
                    &GoldMoveQuantity::All,
                    GoldRelocationReason::Scavenging,
                    events,
                )?;
            }
        }
        Ok(true)
    }
}
