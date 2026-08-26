//! Player intent dispatch: the one place a `PlayerIntent` becomes engine work.
//!
//! Every arm delegates to the subsystem that owns the rule. What this module
//! owns is the surrounding contract each intent shares — status suppression,
//! ending a follow, breaking hiding, and marking the actor active — so those
//! four concerns are stated once instead of once per intent.

use crate::events::{Event, ItemLocationViewV1};
use crate::model::{HideBreakTrigger, PlayerIntent};

use super::{CommittedActivity, Engine, StepError, restoration};

fn item_location_view_is_active(location: &ItemLocationViewV1) -> bool {
    matches!(
        location,
        ItemLocationViewV1::Carried { position, .. } if position.is_active_equipment()
    )
}
impl Engine {
    pub(in crate::engine) fn apply_player_intent(
        &mut self,
        player_index: usize,
        intent: PlayerIntent,
        events: &mut Vec<Event>,
    ) -> Result<CommittedActivity, StepError> {
        if !matches!(
            intent,
            PlayerIntent::Wait | PlayerIntent::Inspect | PlayerIntent::ShowSack
        ) && let Some(effect) = self.suppressing_effect_for_actor(player_index)
        {
            let actor = &self.world.actors[player_index];
            events.push(Event::ActionSuppressedByStatus {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                location: actor.location.clone(),
                intent: intent.label(),
                instance_id: effect.instance_id.clone(),
                effect_id: effect.effect_id.clone(),
                kind: effect.kind.clone(),
            });
            return Ok(CommittedActivity::Inactive);
        }
        let player_actor_id = self.world.actors[player_index].id.clone();
        if Self::gameplay_intent_ends_player_follow(&intent)
            && let Some(character_id) = self.world.actors[player_index].character_id.clone()
        {
            self.remove_follow_edges_for_character(
                &character_id,
                crate::events::PlayerFollowChangeReasonV1::ManualAction,
                events,
            );
        }
        let hide_break_trigger = Self::hide_break_trigger_for_intent(&intent);
        let intent_event_start = events.len();
        let result: Result<bool, StepError> = match intent {
            PlayerIntent::Wait => Ok(false),
            PlayerIntent::Inspect => self.inspect_actor(player_index, events).map(|()| false),
            PlayerIntent::MovePath(path) => self.resolve_player_path(player_index, &path, events),
            PlayerIntent::Traverse(kind) => self
                .apply_explicit_traversal(player_index, kind, events)
                .map(|()| true),
            PlayerIntent::Hide => self.apply_player_hide(player_index, events).map(|()| true),
            PlayerIntent::Nock => self.apply_actor_nock(player_index, events).map(|()| true),
            PlayerIntent::UnloadBow => self
                .apply_actor_unload_bow(player_index, events)
                .map(|()| true),
            PlayerIntent::PhysicalAttack {
                mode,
                target_actor_id,
                authorization,
            } => self.apply_player_physical_attack(
                player_index,
                mode,
                &target_actor_id,
                authorization,
                events,
            ),
            PlayerIntent::SearchCorpse(corpse_id) => self
                .apply_corpse_search(player_index, &corpse_id, events)
                .map(|()| true),
            PlayerIntent::MoveItem {
                item_instance_id,
                destination,
            } => self
                .apply_player_move_item(player_index, &item_instance_id, &destination, events)
                .map(|()| true),
            PlayerIntent::MoveGold {
                source,
                destination,
                quantity,
            } => self
                .apply_player_move_gold(player_index, &source, &destination, &quantity, events)
                .map(|()| true),
            PlayerIntent::DepositBankGold {
                service_id,
                capability_id,
                gold_pile_id,
            } => self
                .apply_bank_deposit(
                    player_index,
                    &service_id,
                    &capability_id,
                    &gold_pile_id,
                    events,
                )
                .map(|()| true),
            PlayerIntent::WithdrawBankGold {
                service_id,
                capability_id,
                amount,
            } => self
                .apply_bank_withdrawal(player_index, &service_id, &capability_id, amount, events)
                .map(|()| true),
            PlayerIntent::DepositLockerItem {
                service_id,
                capability_id,
                item_instance_id,
            } => self
                .apply_locker_deposit(
                    player_index,
                    &service_id,
                    &capability_id,
                    &item_instance_id,
                    events,
                )
                .map(|()| true),
            PlayerIntent::WithdrawLockerItem {
                service_id,
                capability_id,
                item_instance_id,
                destination,
            } => self
                .apply_locker_withdrawal(
                    player_index,
                    &service_id,
                    &capability_id,
                    &item_instance_id,
                    destination,
                    events,
                )
                .map(|()| true),
            PlayerIntent::OfferItem {
                recipient_character_id,
                item_instance_id,
            } => self
                .apply_item_offer(
                    player_index,
                    &recipient_character_id,
                    &item_instance_id,
                    events,
                )
                .map(|()| true),
            PlayerIntent::AcceptItemOffer {
                item_instance_id,
                destination,
            } => self
                .apply_accept_item_offer(player_index, &item_instance_id, destination, events)
                .map(|()| true),
            PlayerIntent::RefuseItemOffer { item_instance_id } => self
                .apply_refuse_item_offer(player_index, &item_instance_id, events)
                .map(|()| true),
            PlayerIntent::WithdrawItemOffer { item_instance_id } => self
                .apply_withdraw_item_offer(player_index, &item_instance_id, events)
                .map(|()| true),
            PlayerIntent::Drink(item_instance_id) => self
                .apply_player_drink(player_index, &item_instance_id, events)
                .map(|()| true),
            PlayerIntent::Open(direction) => self
                .apply_door_open(player_index, direction, events)
                .map(|()| true),
            PlayerIntent::Close(direction) => self
                .apply_door_close(player_index, direction, events)
                .map(|()| true),
            PlayerIntent::ShowSack => self
                .apply_player_show_sack(player_index, events)
                .map(|()| false),
            PlayerIntent::Train {
                service_id,
                offered_gold,
            } => self
                .apply_player_train(player_index, &service_id, offered_gold, events)
                .map(|()| true),
            PlayerIntent::Critique {
                service_id,
                track_id,
            } => self
                .apply_player_critique(player_index, &service_id, &track_id, events)
                .map(|()| true),
            PlayerIntent::PromoteClass(target) => self
                .apply_player_promotion(player_index, &target, events)
                .map(|()| true),
            PlayerIntent::CastSpell {
                spell_id,
                target,
                authorization,
            } => self
                .apply_player_direct_cast(
                    player_index,
                    &spell_id,
                    target.as_ref(),
                    authorization,
                    events,
                )
                .map(|()| true),
            PlayerIntent::WarmSpell { spell_id } => self
                .apply_player_warm_spell(player_index, &spell_id, events)
                .map(|()| true),
            PlayerIntent::CastWarmedSpell {
                target,
                authorization,
            } => self
                .apply_player_cast_warmed_spell(
                    player_index,
                    target.as_ref(),
                    authorization,
                    events,
                )
                .map(|()| true),
            PlayerIntent::ClearSelfDefense {
                attacker_character_id,
            } => self
                .clear_self_defense(player_index, &attacker_character_id, events)
                .map(|()| false),
            PlayerIntent::FizzleWarmedSpell => self
                .apply_player_fizzle_warmed_spell(player_index, events)
                .map(|()| false),
            PlayerIntent::Rest => self.apply_player_rest(player_index, events).map(|()| false),
            PlayerIntent::LearnSpell(spell_id) => self
                .apply_player_learn_spell(player_index, &spell_id, events)
                .map(|()| true),
            PlayerIntent::CommitServiceTransaction {
                service_id,
                capability_id,
                transaction_id,
                item_instance_id,
            } => self
                .apply_player_service_transaction(
                    player_index,
                    &service_id,
                    &capability_id,
                    &transaction_id,
                    item_instance_id.as_deref(),
                    events,
                )
                .map(|()| true),
            PlayerIntent::BuyFromMerchant {
                service_id,
                capability_id,
                item_instance_ids,
            } => self
                .apply_player_merchant_purchase(
                    player_index,
                    &service_id,
                    &capability_id,
                    &item_instance_ids,
                    events,
                )
                .map(|()| true),
            PlayerIntent::SellToMerchant {
                service_id,
                capability_id,
                item_instance_id,
            } => self
                .apply_player_merchant_sale(
                    player_index,
                    &service_id,
                    &capability_id,
                    &item_instance_id,
                    events,
                )
                .map(|()| true),
            PlayerIntent::UseItemService {
                service_id,
                capability_id,
                operation,
                item_instance_id,
            } => self
                .apply_player_item_service(
                    player_index,
                    &service_id,
                    &capability_id,
                    operation,
                    &item_instance_id,
                    events,
                )
                .map(|()| true),
            PlayerIntent::UseRestorationService {
                service_id,
                capability_id,
                operation_id,
                item_instance_id,
                corpse_id,
            } => self
                .apply_player_restoration_service(
                    player_index,
                    restoration::RestorationServiceRequest {
                        service_id,
                        capability_id,
                        operation_id,
                        item_instance_id,
                        corpse_id,
                    },
                    events,
                )
                .map(|()| true),
            PlayerIntent::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id,
            } => self
                .apply_player_npc_interaction(
                    player_index,
                    &npc_actor_id,
                    &interaction_id,
                    item_instance_id.as_deref(),
                    events,
                )
                .map(|()| true),
        };
        if result.is_ok()
            && let Some(trigger) = hide_break_trigger
            && Self::hide_break_trigger_succeeded(
                trigger,
                &player_actor_id,
                &events[intent_event_start..],
            )
        {
            let player_index = self
                .world
                .actors
                .iter()
                .position(|actor| actor.id == player_actor_id)
                .ok_or_else(|| StepError::new("controlled actor disappeared during intent"))?;
            self.break_hidden_if_needed(player_index, trigger, events);
        }
        let active = result?;
        if active {
            let player_index = self
                .world
                .actors
                .iter()
                .position(|actor| actor.id == player_actor_id)
                .ok_or_else(|| StepError::new("controlled actor disappeared during intent"))?;
            self.mark_actor_resource_active(player_index)?;
            Ok(CommittedActivity::Active)
        } else {
            Ok(CommittedActivity::Inactive)
        }
    }

    fn gameplay_intent_ends_player_follow(intent: &PlayerIntent) -> bool {
        matches!(
            intent,
            PlayerIntent::MovePath(_)
                | PlayerIntent::Traverse(_)
                | PlayerIntent::PhysicalAttack { .. }
                | PlayerIntent::CastSpell { .. }
                | PlayerIntent::MoveItem { .. }
                | PlayerIntent::MoveGold { .. }
        )
    }

    fn hide_break_trigger_succeeded(
        trigger: HideBreakTrigger,
        actor_id: &crate::model::ActorId,
        action_events: &[Event],
    ) -> bool {
        match trigger {
            HideBreakTrigger::Move => action_events.iter().any(|event| {
                matches!(
                    event,
                    Event::Moved { actor_id: moved_actor_id, .. }
                        | Event::WorldTransition { actor_id: moved_actor_id, .. }
                        if moved_actor_id == actor_id
                )
            }),
            HideBreakTrigger::Attack => action_events.iter().any(|event| {
                matches!(
                    event,
                    Event::Attacked {
                        attacker_id,
                        ..
                    } | Event::AttackMissed {
                        attacker_id,
                        ..
                    } if attacker_id == actor_id
                ) || matches!(
                    event,
                    Event::AttackBlocked {
                        attacker_id,
                        ..
                    } | Event::WeaponFumbled {
                        attacker_id,
                        ..
                    } if attacker_id == actor_id
                )
            }),
            HideBreakTrigger::ActiveItemMove => action_events.iter().any(|event| {
                matches!(
                    event,
                    Event::ItemRelocated {
                        actor_id: move_actor_id,
                        from,
                        to,
                        ..
                    } if move_actor_id == actor_id
                        && (item_location_view_is_active(from) || item_location_view_is_active(to))
                )
            }),
            HideBreakTrigger::Cast => true,
            HideBreakTrigger::Warm => action_events.iter().any(|event| {
                matches!(event, Event::SpellWarmed { actor_id: warm_actor_id, .. } if warm_actor_id == actor_id)
            }),
        }
    }

    fn hide_break_trigger_for_intent(intent: &PlayerIntent) -> Option<HideBreakTrigger> {
        match intent {
            PlayerIntent::MovePath(_) | PlayerIntent::Traverse(_) => Some(HideBreakTrigger::Move),
            PlayerIntent::PhysicalAttack { .. } => Some(HideBreakTrigger::Attack),
            PlayerIntent::MoveItem { .. }
            | PlayerIntent::DepositLockerItem { .. }
            | PlayerIntent::WithdrawLockerItem { .. }
            | PlayerIntent::OfferItem { .. }
            | PlayerIntent::AcceptItemOffer { .. }
            | PlayerIntent::RefuseItemOffer { .. }
            | PlayerIntent::WithdrawItemOffer { .. } => Some(HideBreakTrigger::ActiveItemMove),
            PlayerIntent::CastSpell { .. } | PlayerIntent::CastWarmedSpell { .. } => {
                Some(HideBreakTrigger::Cast)
            }
            PlayerIntent::WarmSpell { .. } => Some(HideBreakTrigger::Warm),
            _ => None,
        }
    }
}
