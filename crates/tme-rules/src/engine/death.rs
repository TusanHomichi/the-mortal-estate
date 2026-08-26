use crate::events::{
    Event, GoldLocationViewV1, GoldRelocationReason, ItemRelocationReason, SpellFizzleCause,
};
use crate::model::{
    ActorId, ActorKind, ActorLifeState, CarriedPosition, CorpseDisposition, CorpseId, CorpseState,
    DeathCause, GroundGoldPile, ItemLocation, LootClaim, LootClaimBasis, LootOwnerId,
    ResurrectionMethod, ResurrectionRequest, SpellDamageCredit,
};
use crate::view::ActionBlockedReasonV1;

use super::inventory::ItemRelocation;
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DefeatContext {
    pub(super) cause: DeathCause,
    pub(super) credited_actor_id: Option<ActorId>,
    pub(super) direct_social_actor_id: Option<ActorId>,
    pub(super) spell_damage_credit: Option<SpellDamageCredit>,
    pub(super) hostile_authority: Option<crate::model::HostileEffectAuthority>,
}

impl DefeatContext {
    pub(super) fn credited_actor_id(&self) -> Option<&ActorId> {
        self.spell_damage_credit
            .as_ref()
            .map(|credit| &credit.caster_actor_id)
            .or(self.credited_actor_id.as_ref())
    }
}

impl Engine {
    fn allocate_corpse_id(&self) -> Result<(CorpseId, u64), StepError> {
        let sequence = self.world.next_corpse_sequence;
        let next = sequence
            .checked_add(1)
            .ok_or_else(|| StepError::new("corpse sequence overflow"))?;
        let corpse_id = CorpseId::from_sequence(sequence);
        if self.world.corpses.contains_key(&corpse_id) {
            return Err(StepError::new(format!("duplicate corpse ID {corpse_id}")));
        }
        Ok((corpse_id, next))
    }

    fn loot_claim_for_defeat(
        &self,
        actor_index: usize,
        context: &DefeatContext,
    ) -> Option<LootClaim> {
        let defeated = &self.world.actors[actor_index];
        if defeated.kind == ActorKind::Player {
            let owner = defeated
                .character_id
                .clone()
                .map(LootOwnerId::Character)
                .unwrap_or_else(|| LootOwnerId::TransientActor(defeated.id.clone()));
            return Some(LootClaim {
                owner,
                basis: LootClaimBasis::CharacterDeathPile,
            });
        }
        context.credited_actor_id().map(|credited_actor_id| {
            let owner = self
                .world
                .actors
                .iter()
                .find(|actor| actor.id == *credited_actor_id)
                .and_then(|actor| actor.character_id.clone())
                .map(LootOwnerId::Character)
                .unwrap_or_else(|| LootOwnerId::TransientActor(credited_actor_id.clone()));
            LootClaim {
                owner,
                basis: LootClaimBasis::KillingBlow,
            }
        })
    }

    fn build_death_item_relocations(
        &self,
        actor_index: usize,
        corpse_id: Option<&CorpseId>,
        loot_claim: Option<&LootClaim>,
    ) -> Result<(Vec<ItemRelocation>, Vec<ItemRelocation>), StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown defeated actor"))?;
        let holder = actor.item_holder_id();
        let ground = actor.location.clone();
        let mut drops = Vec::new();
        let mut retentions = Vec::new();
        for (position, item_instance_id) in &actor.carried.items {
            let expected = ItemLocation::Carried {
                holder: holder.clone(),
                position: *position,
            };
            let claim = loot_claim.cloned();
            if let Some(corpse_id) = corpse_id
                && !matches!(
                    position,
                    CarriedPosition::LeftHand | CarriedPosition::RightHand
                )
            {
                retentions.push(ItemRelocation {
                    item_instance_id: item_instance_id.clone(),
                    expected,
                    destination: ItemLocation::Corpse {
                        corpse_id: corpse_id.clone(),
                        position: *position,
                    },
                    loot_claim: claim,
                    merchant_listing: None,
                });
            } else {
                drops.push(ItemRelocation {
                    item_instance_id: item_instance_id.clone(),
                    expected,
                    destination: ItemLocation::Ground {
                        position: ground.clone(),
                    },
                    loot_claim: claim,
                    merchant_listing: None,
                });
            }
        }
        Ok((drops, retentions))
    }

    fn clear_death_invalid_state(&mut self, actor_index: usize, events: &mut Vec<Event>) {
        if self.world.actors[actor_index].kind == ActorKind::Player {
            self.world.actors[actor_index].balm_effect = None;
        }
        self.fizzle_warmed_spell(actor_index, SpellFizzleCause::Defeat, events);
        self.remove_active_effects_from_actor(actor_index, "defeat", events);
    }

    pub(super) fn resolve_actor_defeat(
        &mut self,
        actor_index: usize,
        context: DefeatContext,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let defeated = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown defeated actor"))?;
        if !defeated.is_alive() {
            return Ok(());
        }

        let actor_id = defeated.id.clone();
        let actor_name = defeated.name.clone();
        let actor_kind = defeated.kind;
        let origin_character_id = defeated.character_id.clone();
        let location = defeated.location.clone();
        let previous_life_state = defeated.life_state.clone();
        let loot_claim = self.loot_claim_for_defeat(actor_index, &context);
        let creates_corpse = context.cause != DeathCause::Fire
            && defeated.corpse_disposition == CorpseDisposition::SearchableCorpse;

        let social_consequence = context
            .direct_social_actor_id
            .as_ref()
            .and_then(|credited_actor_id| {
                self.world
                    .actors
                    .iter()
                    .position(|actor| actor.id == *credited_actor_id)
                    .map(|killer_index| (killer_index, credited_actor_id.clone()))
            })
            .map(|(killer_index, credited_actor_id)| {
                self.lethal_social_consequence_plan(killer_index, actor_index, &credited_actor_id)
            })
            .transpose()?
            .flatten();

        let corpse = if creates_corpse {
            let (corpse_id, next_sequence) = self.allocate_corpse_id()?;
            Some((
                CorpseState {
                    id: corpse_id,
                    origin_actor_id: actor_id.clone(),
                    origin_character_id: origin_character_id.clone(),
                    origin_kind: actor_kind,
                    origin_name: actor_name.clone(),
                    location: location.clone(),
                    created_at: self.current_time(),
                    sequence: self.world.next_corpse_sequence,
                    searched: false,
                    loot_claim: loot_claim.clone(),
                    contents: Default::default(),
                    gold: 0,
                },
                next_sequence,
            ))
        } else {
            None
        };
        let corpse_id = corpse.as_ref().map(|(corpse, _)| corpse.id.clone());
        if let Some(social_consequence) = &social_consequence {
            self.commit_lethal_social_consequence(social_consequence, events)?;
        } else if let Some(authority) = context.hostile_authority.as_ref() {
            self.commit_absent_killer_player_kill_assessment(authority, actor_index, events)?;
        }
        self.unwind_item_offers_for_defeat(actor_index, events)?;
        let (drops, retentions) = self.build_death_item_relocations(
            actor_index,
            corpse_id.as_ref(),
            loot_claim.as_ref(),
        )?;

        self.clear_death_invalid_state(actor_index, events);
        events.push(Event::ActorDefeated {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            kind: actor_kind,
            location: location.clone(),
            cause: context.cause,
            credited_actor_id: context.credited_actor_id().cloned(),
            loot_claim: loot_claim.clone(),
        });

        if let Some((corpse, next_sequence)) = corpse {
            events.push(Event::CorpseCreated {
                corpse_id: corpse.id.clone(),
                origin_actor_id: corpse.origin_actor_id.clone(),
                origin_character_id: corpse.origin_character_id.clone(),
                origin_kind: corpse.origin_kind,
                origin_name: corpse.origin_name.clone(),
                location: corpse.location.clone(),
                created_at: corpse.created_at,
                sequence: corpse.sequence,
                loot_claim: corpse.loot_claim.clone(),
            });
            self.world.corpses.insert(corpse.id.clone(), corpse);
            self.world.next_corpse_sequence = next_sequence;
        }

        if !drops.is_empty() {
            self.relocate_items_with_events(
                actor_index,
                drops,
                ItemRelocationReason::DeathDrop,
                events,
            )?;
        }
        if !retentions.is_empty() {
            self.relocate_items_with_events(
                actor_index,
                retentions,
                ItemRelocationReason::CorpseRetention,
                events,
            )?;
        }

        let carried_gold = [
            crate::model::CarriedGoldPosition::LeftHand,
            crate::model::CarriedGoldPosition::RightHand,
            crate::model::CarriedGoldPosition::Sack,
        ]
        .into_iter()
        .filter_map(|position| {
            let amount = self.carried_gold_at(actor_index, position).ok()?;
            (amount > 0).then_some((position, amount))
        })
        .collect::<Vec<_>>();
        if let Some(corpse_id) = &corpse_id {
            let amount = self.move_actor_gold_to_corpse(actor_index, corpse_id)?;
            if amount > 0 {
                for (position, position_amount) in &carried_gold {
                    events.push(Event::GoldRelocated {
                        actor_id: actor_id.clone(),
                        actor: actor_name.clone(),
                        amount: *position_amount,
                        from: GoldLocationViewV1::Carried {
                            actor_id: actor_id.clone(),
                            position: *position,
                        },
                        to: GoldLocationViewV1::Corpse {
                            corpse_id: corpse_id.clone(),
                        },
                        reason: GoldRelocationReason::CorpseRetention,
                        loot_claim: loot_claim.clone(),
                    });
                }
            }
        } else if let Some(pile) =
            self.move_actor_gold_to_ground(actor_index, location.clone(), loot_claim.clone())?
        {
            for (position, amount) in carried_gold {
                events.push(Self::ground_gold_event(
                    &actor_id,
                    &actor_name,
                    GoldLocationViewV1::Carried {
                        actor_id: actor_id.clone(),
                        position,
                    },
                    &pile,
                    amount,
                    GoldRelocationReason::DeathDrop,
                ));
            }
        }

        let next_life_state = match actor_kind {
            ActorKind::Player => match corpse_id {
                Some(ref corpse_id) => ActorLifeState::Ghost {
                    corpse_id: corpse_id.clone(),
                    defeated_at: self.current_time(),
                },
                None => ActorLifeState::AwaitingResurrection {
                    cause: context.cause,
                    defeated_at: self.current_time(),
                },
            },
            ActorKind::Monster | ActorKind::Npc => ActorLifeState::Dead,
        };
        self.world.actors[actor_index].life_state = next_life_state.clone();
        events.push(Event::ActorLifeStateChanged {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            from: previous_life_state,
            to: next_life_state,
        });
        if let Some(character_id) = origin_character_id.as_ref() {
            self.clear_npc_followers_of_character(character_id, events)?;
            self.remove_follow_edges_for_character(
                character_id,
                crate::events::PlayerFollowChangeReasonV1::TargetLost,
                events,
            );
        }
        self.award_defeat_rewards(actor_index, events)?;
        if actor_kind == ActorKind::Player && corpse_id.is_none() {
            events.push(Event::ResurrectionRequested {
                actor_id: actor_id.clone(),
                actor: actor_name,
                cause: context.cause,
                method: ResurrectionMethod::Gods,
            });
        }
        self.observe_ecology_defeat(&actor_id, events)?;
        Ok(())
    }

    fn ground_gold_event(
        actor_id: &crate::model::ActorId,
        actor_name: &str,
        from: GoldLocationViewV1,
        pile: &GroundGoldPile,
        amount: i64,
        reason: GoldRelocationReason,
    ) -> Event {
        Event::GoldRelocated {
            actor_id: actor_id.clone(),
            actor: actor_name.to_string(),
            amount,
            from,
            to: GoldLocationViewV1::Ground {
                gold_pile_id: pile.id.clone(),
                location: pile.location.clone(),
            },
            reason,
            loot_claim: pile.loot_claim.clone(),
        }
    }

    pub(super) fn validate_corpse_search(
        &self,
        actor_index: usize,
        corpse_id: &CorpseId,
    ) -> Result<(), ActionBlockedReasonV1> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or(ActionBlockedReasonV1::ActorNotLiving)?;
        if !actor.is_alive() {
            return Err(ActionBlockedReasonV1::ActorNotLiving);
        }
        if !self.actor_can_act(actor_index) {
            return Err(ActionBlockedReasonV1::NotReady);
        }
        let corpse = self
            .world
            .corpses
            .get(corpse_id)
            .ok_or(ActionBlockedReasonV1::NoSuchCorpse)?;
        if corpse.location.level != actor.location.level
            || corpse.location.position != actor.location.position
        {
            return Err(ActionBlockedReasonV1::CorpseNotHere);
        }
        if corpse.searched {
            return Err(ActionBlockedReasonV1::CorpseAlreadySearched);
        }
        Ok(())
    }

    pub(super) fn apply_corpse_search(
        &mut self,
        actor_index: usize,
        corpse_id: &CorpseId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_corpse_search(actor_index, corpse_id)
            .map_err(|reason| StepError::new(reason.to_string()))?;
        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        let corpse = self.world.corpses[corpse_id].clone();
        let relocations = corpse
            .contents
            .iter()
            .map(|(position, item_instance_id)| ItemRelocation {
                item_instance_id: item_instance_id.clone(),
                expected: ItemLocation::Corpse {
                    corpse_id: corpse_id.clone(),
                    position: *position,
                },
                destination: ItemLocation::Ground {
                    position: corpse.location.clone(),
                },
                loot_claim: corpse.loot_claim.clone(),
                merchant_listing: None,
            })
            .collect::<Vec<_>>();
        let items_released = relocations.len();
        if !relocations.is_empty() {
            self.relocate_items_with_events(
                actor_index,
                relocations,
                ItemRelocationReason::CorpseSearch,
                events,
            )?;
        }
        let gold_released = if let Some(pile) = self.move_corpse_gold_to_ground(corpse_id)? {
            let amount = pile.amount;
            events.push(Self::ground_gold_event(
                &actor_id,
                &actor_name,
                GoldLocationViewV1::Corpse {
                    corpse_id: corpse_id.clone(),
                },
                &pile,
                amount,
                GoldRelocationReason::CorpseSearch,
            ));
            amount
        } else {
            0
        };
        self.world
            .corpses
            .get_mut(corpse_id)
            .expect("validated corpse must remain present")
            .searched = true;
        events.push(Event::CorpseSearched {
            corpse_id: corpse_id.clone(),
            actor_id,
            actor: actor_name,
            location: corpse.location,
            items_released,
            gold_released,
        });
        Ok(())
    }

    pub(super) fn validate_resurrection_request(
        &self,
        request: &ResurrectionRequest,
    ) -> Result<(usize, Option<CorpseState>), StepError> {
        let actor_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == request.actor_id)
            .ok_or_else(|| StepError::new("unknown resurrection actor"))?;
        let actor = &self.world.actors[actor_index];
        if actor.kind != ActorKind::Player {
            return Err(StepError::new("resurrection actor must be a player"));
        }
        if !self.in_bounds(&request.destination) || !self.is_walkable(&request.destination) {
            return Err(StepError::new("invalid resurrection destination"));
        }
        let max_hp = actor.max_hp();
        let max_stamina = actor.max_stamina();
        match request.method {
            ResurrectionMethod::Gods | ResurrectionMethod::Priest => {
                if request.current_hp <= 0 || request.current_hp >= max_hp {
                    return Err(StepError::new(
                        "gods and Priest resurrection HP must be below current maximum",
                    ));
                }
                if request.current_stamina < 0 || request.current_stamina >= max_stamina {
                    return Err(StepError::new(
                        "gods and Priest resurrection stamina must be below maximum",
                    ));
                }
            }
            ResurrectionMethod::Thaumaturge => {
                let expected_hp = max_hp
                    .checked_div(2)
                    .and_then(|half| half.checked_add(max_hp % 2))
                    .ok_or_else(|| StepError::new("Thaumaturge half-HP calculation overflow"))?;
                if request.current_hp != expected_hp || request.current_stamina != 0 {
                    return Err(StepError::new(
                        "Thaumaturge resurrection requires rounded-up half maximum HP and zero stamina",
                    ));
                }
            }
        }

        let corpse = match &actor.life_state {
            ActorLifeState::Ghost { corpse_id, .. } => {
                if request.corpse_id.as_ref() != Some(corpse_id) {
                    return Err(StepError::new("resurrection requires the matching corpse"));
                }
                let corpse = self
                    .world
                    .corpses
                    .get(corpse_id)
                    .ok_or_else(|| StepError::new("matching resurrection corpse is missing"))?
                    .clone();
                if corpse.origin_actor_id != actor.id || corpse.origin_kind != ActorKind::Player {
                    return Err(StepError::new("resurrection corpse has the wrong origin"));
                }
                Some(corpse)
            }
            ActorLifeState::AwaitingResurrection { .. } => {
                if request.method != ResurrectionMethod::Gods || request.corpse_id.is_some() {
                    return Err(StepError::new(
                        "no-corpse resurrection requires the gods method",
                    ));
                }
                None
            }
            ActorLifeState::Alive | ActorLifeState::Dead => {
                return Err(StepError::new("actor is not awaiting resurrection"));
            }
        };
        if matches!(
            request.method,
            ResurrectionMethod::Priest | ResurrectionMethod::Thaumaturge
        ) && corpse.is_none()
        {
            return Err(StepError::new("resurrection method requires a corpse"));
        }
        Ok((actor_index, corpse))
    }

    pub(super) fn apply_resurrection_request(
        &mut self,
        request: ResurrectionRequest,
    ) -> Result<Vec<Event>, StepError> {
        let (actor_index, corpse) = self.validate_resurrection_request(&request)?;
        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        let previous_life_state = self.world.actors[actor_index].life_state.clone();
        let holder = self.item_holder_for_actor_index(actor_index)?;
        let mut events = Vec::new();

        if let Some(corpse) = &corpse {
            let relocations = corpse
                .contents
                .iter()
                .map(|(position, item_instance_id)| ItemRelocation {
                    item_instance_id: item_instance_id.clone(),
                    expected: ItemLocation::Corpse {
                        corpse_id: corpse.id.clone(),
                        position: *position,
                    },
                    destination: ItemLocation::Carried {
                        holder: holder.clone(),
                        position: *position,
                    },
                    loot_claim: None,
                    merchant_listing: None,
                })
                .collect::<Vec<_>>();
            if !relocations.is_empty() {
                self.relocate_items_with_events(
                    actor_index,
                    relocations,
                    ItemRelocationReason::ResurrectionReturn,
                    &mut events,
                )?;
            }
            let gold = self.move_corpse_gold_to_actor(&corpse.id, actor_index)?;
            if gold > 0 {
                events.push(Event::GoldRelocated {
                    actor_id: actor_id.clone(),
                    actor: actor_name.clone(),
                    amount: gold,
                    from: GoldLocationViewV1::Corpse {
                        corpse_id: corpse.id.clone(),
                    },
                    to: GoldLocationViewV1::Carried {
                        actor_id: actor_id.clone(),
                        position: crate::model::CarriedGoldPosition::Sack,
                    },
                    reason: GoldRelocationReason::ResurrectionReturn,
                    loot_claim: None,
                });
            }
            self.world.corpses.remove(&corpse.id);
            events.push(Event::CorpseRemoved {
                corpse_id: corpse.id.clone(),
                origin_actor_id: corpse.origin_actor_id.clone(),
                location: corpse.location.clone(),
                method: request.method,
            });
        }

        self.world.actors[actor_index].location = request.destination.clone();
        self.set_hp(actor_index, request.current_hp)?;
        self.set_stamina(actor_index, request.current_stamina)?;
        self.world.actors[actor_index].life_state = ActorLifeState::Alive;
        events.push(Event::ActorLifeStateChanged {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            from: previous_life_state,
            to: ActorLifeState::Alive,
        });
        events.push(Event::ActorResurrected {
            actor_id,
            actor: actor_name,
            corpse_id: corpse.map(|corpse| corpse.id),
            method: request.method,
            destination: request.destination,
            current_hp: request.current_hp,
            current_stamina: request.current_stamina,
        });
        super::progression::apply_ready_level_advances(self, actor_index, &mut events)?;
        Ok(events)
    }

    pub fn resurrect(&mut self, request: ResurrectionRequest) -> Result<Vec<Event>, StepError> {
        let before = self.clone();
        match self.apply_resurrection_request(request) {
            Ok(events) => Ok(events),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }
}
