use std::collections::BTreeMap;

use crate::content::{LootEntryDef, LootTableDef, SpawnResetDef};
use crate::events::{EcologyLifecyclePolicyV1, Event};
use crate::model::{
    ActorKind, BowReadiness, CarriedGold, CarriedLayout, CarriedPosition, EcologyActorOrigin,
    ItemBindingState, ItemInstanceState, ItemKnowledgeState, LogicalTime,
};

use super::setup::{ActorInstanceState, actor_state_from_definition};
use super::{Engine, StepError};

const MAX_SITE_MATERIALIZATIONS_PER_BOUNDARY: usize = 2;

#[derive(Debug)]
struct LootItemCandidate {
    identity_suffix: String,
    item_definition_id: String,
    quantity: u32,
    position: CarriedPosition,
}

impl Engine {
    pub(super) fn initialize_ecology(&mut self, events: &mut Vec<Event>) -> Result<(), StepError> {
        let site_ids = self.world.ecology_sites.keys().cloned().collect::<Vec<_>>();
        for site_id in site_ids {
            let site = self
                .world
                .ecology_sites
                .get(&site_id)
                .cloned()
                .ok_or_else(|| StepError::new("ecology site disappeared"))?;
            let member_ids = self
                .definition
                .catalog
                .spawn_groups
                .get(&site.spawn_group_id)
                .ok_or_else(|| StepError::new("ecology spawn group disappeared"))?
                .members
                .iter()
                .map(|member| member.member_id.clone())
                .collect::<Vec<_>>();
            self.spawn_ecology_members(
                &site_id,
                &member_ids,
                site.generation,
                self.current_time(),
                events,
            )?;
        }
        Ok(())
    }

    fn spawn_ecology_members(
        &mut self,
        site_id: &str,
        member_ids: &[String],
        generation: u32,
        ready_at: LogicalTime,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let site = self
            .world
            .ecology_sites
            .get(site_id)
            .cloned()
            .ok_or_else(|| StepError::new("ecology site disappeared"))?;
        let group = self
            .definition
            .catalog
            .spawn_groups
            .get(&site.spawn_group_id)
            .cloned()
            .ok_or_else(|| StepError::new("ecology spawn group disappeared"))?;

        for member_id in member_ids {
            let member = group
                .members
                .iter()
                .find(|candidate| candidate.member_id == *member_id)
                .ok_or_else(|| StepError::new("ecology spawn member disappeared"))?;
            let actor_definition = self
                .definition
                .catalog
                .actor_definitions
                .get(&member.actor_definition_id)
                .cloned()
                .ok_or_else(|| StepError::new("ecology actor definition disappeared"))?;
            let location = site
                .member_slots
                .get(member_id)
                .map(|slot| slot.location.clone())
                .ok_or_else(|| StepError::new("ecology member slot disappeared"))?;
            let actor_id = crate::model::ActorId::new(format!(
                "ecology:{}:{}:{}",
                site.id, member.member_id, generation
            ));
            if self.world.actors.iter().any(|actor| actor.id == actor_id) {
                return Err(StepError::new("ecology actor ID already exists"));
            }

            let carried = if let Some(loot_table_id) = &member.loot_table_id {
                let table = self
                    .definition
                    .catalog
                    .loot_tables
                    .get(loot_table_id)
                    .cloned()
                    .ok_or_else(|| StepError::new("ecology loot table disappeared"))?;
                self.roll_ecology_loot(&actor_id, &table)?
            } else {
                CarriedLayout {
                    items: BTreeMap::new(),
                    gold: CarriedGold::default(),
                }
            };

            let timing = self.allocate_actor_timing(ready_at);
            let actor = actor_state_from_definition(
                &actor_definition,
                ActorInstanceState {
                    id: actor_id.clone(),
                    location: location.clone(),
                    hp: actor_definition.stats.hp,
                    mp: 0,
                    stamina: 10,
                    timing,
                    attack_ready_at: ready_at,
                    carried,
                    npc: None,
                    character_id: None,
                    character: None,
                    active_effects: Vec::new(),
                    summoned: None,
                    ecology_origin: Some(EcologyActorOrigin {
                        site_id: site.id.clone(),
                        member_id: member.member_id.clone(),
                        generation,
                    }),
                },
            );
            self.world.actors.push(actor);
            let slot = self
                .world
                .ecology_sites
                .get_mut(site_id)
                .and_then(|state| state.member_slots.get_mut(member_id))
                .ok_or_else(|| StepError::new("ecology member slot disappeared during spawn"))?;
            slot.actor_id = Some(actor_id.clone());
            slot.due_at = None;
            events.push(Event::EcologyActorSpawned {
                site_id: site.id.clone(),
                member_id: member.member_id.clone(),
                generation,
                actor_id,
                actor_definition_id: actor_definition.id,
                location,
            });
        }
        Ok(())
    }

    fn roll_ecology_loot(
        &mut self,
        actor_id: &crate::model::ActorId,
        table: &LootTableDef,
    ) -> Result<CarriedLayout, StepError> {
        let mut candidates = Vec::new();
        let mut gold = CarriedGold::default();

        for entry in table.entries() {
            let (chance_numerator, chance_denominator) = entry.chance();
            let selected = if chance_numerator == chance_denominator {
                true
            } else {
                self.rng
                    .roll_bounded(chance_denominator)
                    .map_err(StepError::new)?
                    <= chance_numerator
            };
            if !selected {
                continue;
            }

            match entry {
                LootEntryDef::Item {
                    item_definition_id,
                    quantity,
                    position,
                    ..
                } => candidates.push(LootItemCandidate {
                    identity_suffix: entry.id().to_string(),
                    item_definition_id: item_definition_id.clone(),
                    quantity: *quantity,
                    position: *position,
                }),
                LootEntryDef::ItemChoice { members, .. } => {
                    let member_roll = self
                        .rng
                        .roll_bounded(
                            u32::try_from(members.len())
                                .map_err(|_| StepError::new("loot choice is too large"))?,
                        )
                        .map_err(StepError::new)?;
                    let member = &members[usize::try_from(member_roll - 1)
                        .map_err(|_| StepError::new("loot choice index is invalid"))?];
                    candidates.push(LootItemCandidate {
                        identity_suffix: format!("{}:{}", entry.id(), member.member_id),
                        item_definition_id: member.item_definition_id.clone(),
                        quantity: member.quantity,
                        position: member.position,
                    });
                }
                LootEntryDef::Gold {
                    minimum_amount,
                    maximum_amount,
                    position,
                    ..
                } => {
                    let span = maximum_amount
                        .checked_sub(*minimum_amount)
                        .and_then(|difference| difference.checked_add(1))
                        .and_then(|value| u32::try_from(value).ok())
                        .ok_or_else(|| StepError::new("ecology gold range is invalid"))?;
                    let amount_roll = self.rng.roll_bounded(span).map_err(StepError::new)?;
                    let amount = minimum_amount
                        .checked_add(i64::from(amount_roll - 1))
                        .ok_or_else(|| StepError::new("ecology gold amount overflow"))?;
                    *gold.amount_mut(*position) = amount;
                }
            }
        }

        let keep = table
            .maximum_non_gold_drops()
            .map_or(candidates.len(), usize::from);
        let mut items = BTreeMap::new();
        for candidate in candidates.into_iter().take(keep) {
            let item_id = format!(
                "{}:loot:{}:{}",
                actor_id,
                table.id(),
                candidate.identity_suffix
            );
            let bow_readiness = self
                .definition
                .catalog
                .item_catalog
                .get(&candidate.item_definition_id)
                .and_then(|item| item.weapon.as_ref())
                .and_then(|weapon| {
                    (weapon.handedness == crate::model::WeaponHandedness::Bow)
                        .then_some(BowReadiness::Unnocked)
                });
            if self
                .world
                .item_instances
                .insert(
                    item_id.clone(),
                    ItemInstanceState {
                        definition_id: candidate.item_definition_id,
                        quantity: candidate.quantity,
                        knowledge: ItemKnowledgeState {
                            identified: false,
                            appraised: false,
                        },
                        binding: ItemBindingState::Unrestricted,
                        bow_readiness,
                    },
                )
                .is_some()
            {
                return Err(StepError::new("ecology item ID already exists"));
            }
            if items.insert(candidate.position, item_id).is_some() {
                return Err(StepError::new(
                    "ecology loot occupied one carried position twice",
                ));
            }
        }

        Ok(CarriedLayout { items, gold })
    }

    pub(super) fn observe_ecology_defeat(
        &mut self,
        actor_id: &crate::model::ActorId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let Some(origin) = self
            .world
            .actors
            .iter()
            .find(|actor| &actor.id == actor_id)
            .and_then(|actor| actor.ecology_origin.clone())
        else {
            return Ok(());
        };
        let site = self
            .world
            .ecology_sites
            .get(&origin.site_id)
            .cloned()
            .ok_or_else(|| StepError::new("defeated ecology actor has no site"))?;
        let group = self
            .definition
            .catalog
            .spawn_groups
            .get(&site.spawn_group_id)
            .cloned()
            .ok_or_else(|| StepError::new("ecology spawn group disappeared"))?;

        {
            let slot = self
                .world
                .ecology_sites
                .get_mut(&origin.site_id)
                .and_then(|state| state.member_slots.get_mut(&origin.member_id))
                .ok_or_else(|| StepError::new("defeated ecology actor has no member slot"))?;
            if slot.actor_id.as_ref() != Some(actor_id) {
                return Err(StepError::new(
                    "defeated ecology actor disagrees with its member slot",
                ));
            }
            slot.actor_id = None;
            slot.due_at = None;
        }

        let all_vacant = self.world.ecology_sites[&origin.site_id]
            .member_slots
            .values()
            .all(|slot| slot.actor_id.is_none());
        match group.reset {
            SpawnResetDef::FullSite { delay_units } => {
                if !all_vacant
                    || self.world.ecology_sites[&origin.site_id]
                        .full_clear_due_at
                        .is_some()
                {
                    return Ok(());
                }
                let due_at = self.logical_time_after(delay_units);
                self.world
                    .ecology_sites
                    .get_mut(&origin.site_id)
                    .expect("checked ecology site")
                    .full_clear_due_at = Some(due_at);
                events.push(Event::EcologyResetScheduled {
                    site_id: origin.site_id,
                    generation: origin.generation,
                    member_ids: group
                        .members
                        .iter()
                        .map(|member| member.member_id.clone())
                        .collect(),
                    due_at,
                    policy: EcologyLifecyclePolicyV1::FullSite,
                });
            }
            SpawnResetDef::SlotReplenishment {
                slot_delay_units,
                full_clear_delay_units,
            } => {
                let (due_at, member_ids) = if all_vacant {
                    let due_at = self.logical_time_after(full_clear_delay_units);
                    let state = self
                        .world
                        .ecology_sites
                        .get_mut(&origin.site_id)
                        .expect("checked ecology site");
                    state.full_clear_due_at = Some(due_at);
                    for slot in state.member_slots.values_mut() {
                        slot.due_at = None;
                    }
                    (
                        due_at,
                        group
                            .members
                            .iter()
                            .map(|member| member.member_id.clone())
                            .collect(),
                    )
                } else {
                    let due_at = self.logical_time_after(slot_delay_units);
                    self.world
                        .ecology_sites
                        .get_mut(&origin.site_id)
                        .and_then(|state| state.member_slots.get_mut(&origin.member_id))
                        .expect("checked ecology member slot")
                        .due_at = Some(due_at);
                    (due_at, vec![origin.member_id.clone()])
                };
                events.push(Event::EcologyResetScheduled {
                    site_id: origin.site_id,
                    generation: origin.generation,
                    member_ids,
                    due_at,
                    policy: EcologyLifecyclePolicyV1::SlotReplenishment,
                });
            }
        }
        Ok(())
    }

    pub(super) fn process_ecology_resets(
        &mut self,
        boundary_at: LogicalTime,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let due_site_ids = self
            .world
            .ecology_sites
            .iter()
            .filter_map(|(site_id, site)| {
                let full_clear_due = site
                    .full_clear_due_at
                    .is_some_and(|due_at| due_at <= boundary_at);
                let slot_due = site
                    .member_slots
                    .values()
                    .any(|slot| slot.due_at.is_some_and(|due_at| due_at <= boundary_at));
                (full_clear_due || slot_due).then_some(site_id.clone())
            })
            .collect::<Vec<_>>();

        for site_id in due_site_ids {
            if self.ecology_site_is_observed(&site_id)? {
                continue;
            }
            let previous = self.world.ecology_sites[&site_id].clone();
            let group = self
                .definition
                .catalog
                .spawn_groups
                .get(&previous.spawn_group_id)
                .cloned()
                .ok_or_else(|| StepError::new("ecology spawn group disappeared"))?;
            let full_clear_due = previous
                .full_clear_due_at
                .is_some_and(|due_at| due_at <= boundary_at);

            let ordered_due_members = group
                .members
                .iter()
                .filter(|member| {
                    previous
                        .member_slots
                        .get(&member.member_id)
                        .is_some_and(|slot| {
                            slot.actor_id.is_none()
                                && (full_clear_due
                                    || slot.due_at.is_some_and(|due_at| due_at <= boundary_at))
                        })
                })
                .map(|member| member.member_id.clone())
                .collect::<Vec<_>>();
            if ordered_due_members.is_empty() {
                if full_clear_due {
                    self.world
                        .ecology_sites
                        .get_mut(&site_id)
                        .expect("checked ecology site")
                        .full_clear_due_at = None;
                }
                continue;
            }

            let materialized_members = ordered_due_members
                .iter()
                .take(MAX_SITE_MATERIALIZATIONS_PER_BOUNDARY)
                .cloned()
                .collect::<Vec<_>>();
            let from_generation = previous.generation;
            let advance_generation =
                full_clear_due || matches!(group.reset, SpawnResetDef::SlotReplenishment { .. });
            let to_generation = if advance_generation {
                from_generation
                    .checked_add(1)
                    .ok_or_else(|| StepError::new("ecology generation overflow"))?
            } else {
                from_generation
            };

            self.world.actors.retain(|actor| {
                actor.ecology_origin.as_ref().is_none_or(|origin| {
                    origin.site_id != site_id
                        || !materialized_members.contains(&origin.member_id)
                        || actor.is_alive()
                })
            });
            {
                let state = self
                    .world
                    .ecology_sites
                    .get_mut(&site_id)
                    .expect("checked ecology site");
                state.generation = to_generation;
                if full_clear_due {
                    state.full_clear_due_at = None;
                }
                for member_id in &materialized_members {
                    state
                        .member_slots
                        .get_mut(member_id)
                        .expect("validated ecology member")
                        .due_at = None;
                }
                for member_id in ordered_due_members
                    .iter()
                    .skip(MAX_SITE_MATERIALIZATIONS_PER_BOUNDARY)
                {
                    state
                        .member_slots
                        .get_mut(member_id)
                        .expect("validated ecology member")
                        .due_at = Some(boundary_at);
                }
            }

            let policy = match group.reset {
                SpawnResetDef::FullSite { .. } => EcologyLifecyclePolicyV1::FullSite,
                SpawnResetDef::SlotReplenishment { .. } => {
                    EcologyLifecyclePolicyV1::SlotReplenishment
                }
            };
            events.push(Event::EcologyReset {
                site_id: site_id.clone(),
                from_generation,
                to_generation,
                member_ids: materialized_members.clone(),
                policy,
            });
            self.spawn_ecology_members(
                &site_id,
                &materialized_members,
                to_generation,
                boundary_at,
                events,
            )?;
        }
        Ok(())
    }

    fn ecology_site_is_observed(&self, site_id: &str) -> Result<bool, StepError> {
        let site = self
            .world
            .ecology_sites
            .get(site_id)
            .ok_or_else(|| StepError::new("ecology site disappeared"))?;
        let living_player_indices = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter_map(|(index, actor)| {
                (actor.kind == ActorKind::Player && actor.is_alive()).then_some(index)
            })
            .collect::<Vec<_>>();
        Ok(living_player_indices.iter().any(|observer_index| {
            site.member_slots
                .values()
                .any(|slot| self.actor_can_see(*observer_index, &slot.location))
        }))
    }
}
