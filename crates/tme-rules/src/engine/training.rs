//! Service-addressed permanent skill training and read-only critique.

use std::collections::BTreeSet;

use crate::engine::StepError;
use crate::events::Event;
use crate::model::{
    CarriedPosition, SkillTrainingCapability, TrainingOffer, Transaction, TransactionCost,
    TransactionRequirement,
};
use crate::view::ActionBlockedReasonV1;

use super::Engine;
use super::transactions::{PlannedReward, TransactionPlan, TransactionSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TrainingContractError {
    reason: ActionBlockedReasonV1,
    message: String,
}

impl TrainingContractError {
    fn new(reason: ActionBlockedReasonV1, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }

    pub(super) const fn reason(&self) -> ActionBlockedReasonV1 {
        self.reason
    }
}

impl From<TrainingContractError> for StepError {
    fn from(error: TrainingContractError) -> Self {
        StepError::new(error.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TrainingPlan {
    actor_id: crate::model::ActorId,
    actor_name: String,
    service_id: String,
    track_id: String,
    offered_gold: i64,
    spent_gold: i64,
    unspent_gold: i64,
    previous_learning_rate: u64,
    new_learning_rate: u64,
    transaction: TransactionPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CritiquePlan {
    actor_id: crate::model::ActorId,
    actor_name: String,
    service_id: String,
    track_id: String,
    track_display: Option<String>,
    level: u8,
    critique_rank: Option<u8>,
    level_title: Option<String>,
}

impl Engine {
    fn training_service(
        &self,
        actor_index: usize,
        service_id: &str,
    ) -> Result<(String, SkillTrainingCapability), TrainingContractError> {
        let service = self.service_by_id(service_id).ok_or_else(|| {
            TrainingContractError::new(
                ActionBlockedReasonV1::NoService,
                format!("trainer service {service_id:?} was not found"),
            )
        })?;
        let training = self
            .skill_training_capability(service)
            .cloned()
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::NoService,
                    format!("trainer service {service_id:?} has no skill-training capability"),
                )
            })?;
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TrainingContractError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        if service.position().level != actor.location.level
            || service.position().position != actor.location.position
        {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::ServiceNotHere,
                format!("trainer service {service_id:?} is not at the actor coordinate"),
            ));
        }
        Ok((service.id().to_string(), training))
    }

    fn critique_service(
        &self,
        actor_index: usize,
        service_id: &str,
    ) -> Result<String, TrainingContractError> {
        let service = self.service_by_id(service_id).ok_or_else(|| {
            TrainingContractError::new(
                ActionBlockedReasonV1::NoService,
                format!("trainer service {service_id:?} was not found"),
            )
        })?;
        if self.skill_critique_capability(service).is_none() {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::NoService,
                format!("trainer service {service_id:?} has no skill-critique capability"),
            ));
        }
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TrainingContractError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        if service.position().level != actor.location.level
            || service.position().position != actor.location.position
        {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::ServiceNotHere,
                format!("trainer service {service_id:?} is not at the actor coordinate"),
            ));
        }
        Ok(service.id().to_string())
    }

    fn actor_training_class_id(&self, actor_index: usize) -> Result<&str, TrainingContractError> {
        self.world
            .actors
            .get(actor_index)
            .and_then(|actor| actor.character.as_ref())
            .map(|character| character.identity.current_class_id.as_str())
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::WrongClass,
                    "actor has no character sheet",
                )
            })
    }

    fn offer_for_track_and_class(
        &self,
        actor_index: usize,
        service_id: &str,
        training: &SkillTrainingCapability,
        track_id: &str,
    ) -> Result<TrainingOffer, TrainingContractError> {
        let class_id = self.actor_training_class_id(actor_index)?;
        let mut matching = training
            .offers
            .iter()
            .filter(|offer| offer.track_id == track_id);
        let Some(first) = matching.next() else {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::MissingTrainingFocus,
                format!(
                    "trainer service {:?} does not offer track {track_id:?}",
                    service_id
                ),
            ));
        };
        let eligible = std::iter::once(first)
            .chain(matching)
            .filter(|offer| {
                offer
                    .eligible_class_ids
                    .iter()
                    .any(|eligible| eligible == class_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        match eligible.as_slice() {
            [offer] => Ok(offer.clone()),
            [] => Err(TrainingContractError::new(
                ActionBlockedReasonV1::WrongClass,
                format!("class {class_id:?} is not eligible for track {track_id:?}"),
            )),
            _ => Err(TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                format!(
                    "trainer service {:?} has ambiguous offers for track {track_id:?}",
                    service_id
                ),
            )),
        }
    }

    fn focus_track_for_service(
        &self,
        actor_index: usize,
        training: &SkillTrainingCapability,
    ) -> Result<String, TrainingContractError> {
        let class_id = self.actor_training_class_id(actor_index)?;
        let magic_track = super::skills::magic_skill_track_for_class(class_id).filter(|track_id| {
            training.offers.iter().any(|offer| {
                offer.track_id == *track_id
                    && offer
                        .eligible_class_ids
                        .iter()
                        .any(|eligible| eligible == class_id)
            })
        });
        let require_magic_book = |track_id: &'static str| {
            self.right_hand_spell_book(actor_index, track_id)
                .map_err(|reason| {
                    TrainingContractError::new(
                        reason,
                        "magic training requires the character's bound Spell Book in the right hand",
                    )
                })?;
            Ok(track_id.to_string())
        };
        let right_hand = self
            .item_at_position(actor_index, CarriedPosition::RightHand)
            .map_err(|error| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    error.message(),
                )
            })?;

        let Some(item_instance_id) = right_hand else {
            let track_id = self
                .physical_weapon_selection(actor_index)
                .map_err(|error| {
                    TrainingContractError::new(
                        ActionBlockedReasonV1::InvalidTrainingOffer,
                        error.message(),
                    )
                })?
                .skill_track_id;
            if training
                .offers
                .iter()
                .any(|offer| offer.track_id == track_id)
            {
                return Ok(track_id);
            }
            if let Some(magic_track) = magic_track {
                return require_magic_book(magic_track);
            }
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::MissingTrainingFocus,
                "empty right hand does not select an offered Hand track",
            ));
        };
        let definition = self.item_definition(item_instance_id).map_err(|error| {
            TrainingContractError::new(ActionBlockedReasonV1::InvalidTrainingOffer, error.message())
        })?;
        if let Some(magic_track) = magic_track
            && definition
                .capability
                .as_ref()
                .and_then(|capability| capability.spell_book_for.as_ref())
                .is_some_and(|tracks| tracks.iter().any(|track_id| track_id == magic_track))
        {
            return require_magic_book(magic_track);
        }
        if definition.weapon.is_some() {
            let track_id = self
                .physical_weapon_selection(actor_index)
                .map(|selection| selection.skill_track_id)
                .map_err(|error| {
                    TrainingContractError::new(
                        ActionBlockedReasonV1::InvalidTrainingOffer,
                        error.message(),
                    )
                })?;
            if training
                .offers
                .iter()
                .any(|offer| offer.track_id == track_id)
            {
                return Ok(track_id);
            }
            if let Some(magic_track) = magic_track {
                return require_magic_book(magic_track);
            }
            return Ok(track_id);
        }
        let Some(capability) = definition.capability.as_ref() else {
            if let Some(magic_track) = magic_track {
                return require_magic_book(magic_track);
            }
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::MissingTrainingFocus,
                "right-hand item has no training focus",
            ));
        };

        let Some(focus_tracks) = capability.training_focus_for.as_ref() else {
            if let Some(magic_track) = magic_track {
                return require_magic_book(magic_track);
            }
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::MissingTrainingFocus,
                "right-hand item has no training focus",
            ));
        };
        let offered = focus_tracks
            .iter()
            .filter(|track_id| {
                training
                    .offers
                    .iter()
                    .any(|offer| offer.track_id == track_id.as_str())
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        if offered.is_empty()
            && let Some(magic_track) = magic_track
        {
            return require_magic_book(magic_track);
        }
        let eligible = offered
            .iter()
            .filter(|track_id| {
                training.offers.iter().any(|offer| {
                    offer.track_id == track_id.as_str()
                        && offer
                            .eligible_class_ids
                            .iter()
                            .any(|eligible| eligible == class_id)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        match eligible.as_slice() {
            [track_id] => Ok(track_id.clone()),
            [] if offered.len() == 1 => Ok(offered
                .first()
                .expect("one offered focus was checked")
                .clone()),
            [] if offered.is_empty() => Err(TrainingContractError::new(
                ActionBlockedReasonV1::MissingTrainingFocus,
                "right-hand item does not focus an offered track",
            )),
            [] => Err(TrainingContractError::new(
                ActionBlockedReasonV1::WrongClass,
                "right-hand training focuses do not match the actor class",
            )),
            _ => Err(TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                "right-hand item ambiguously selects multiple training tracks",
            )),
        }
    }

    pub(super) fn training_focus_track_for_service_id(
        &self,
        actor_index: usize,
        service_id: &str,
    ) -> Result<String, TrainingContractError> {
        let (_service, training) = self.training_service(actor_index, service_id)?;
        self.focus_track_for_service(actor_index, &training)
    }

    pub(super) fn training_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        offered_gold: i64,
    ) -> Result<TrainingPlan, TrainingContractError> {
        // The locked validation order establishes character-sheet presence
        // before resolving any service or held focus.
        self.actor_training_class_id(actor_index)?;
        let (service, training) = self.training_service(actor_index, service_id)?;
        let track_id = self.focus_track_for_service(actor_index, &training)?;
        let offer = self.offer_for_track_and_class(actor_index, &service, &training, &track_id)?;
        if !self.skill_track_is_allowed_for_actor(actor_index, &track_id) {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::WrongClass,
                format!("track {track_id:?} is not available to this character"),
            ));
        }
        let category_level = self
            .highest_skill_level_in_category(actor_index, &track_id)
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    format!("track {track_id:?} has no training category"),
                )
            })?;
        if category_level < offer.minimum_category_level
            || category_level > offer.maximum_category_level
        {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::OutsideTrainerWindow,
                format!(
                    "category level {category_level} is outside trainer window {}..={}",
                    offer.minimum_category_level, offer.maximum_category_level
                ),
            ));
        }

        let current = self.skill_entry_for_actor(actor_index, &track_id);
        if current.is_some_and(crate::model::SkillEntry::is_maximum) {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::TrainingCapReached,
                format!("skill {track_id:?} is already at maximum"),
            ));
        }
        let level = current.map_or(0, |entry| entry.level);
        let previous_learning_rate = current.map_or(
            self.definition.catalog.rules.skills.base_learning_rate,
            |entry| entry.learning_rate,
        );
        let maximum_learning_rate = self
            .definition
            .catalog
            .rules
            .skills
            .training
            .maximum_learning_rates
            .get(usize::from(level))
            .copied()
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    format!("missing learning-rate cap for level {level}"),
                )
            })?;
        let available_rate = maximum_learning_rate
            .checked_sub(previous_learning_rate)
            .filter(|available| *available > 0)
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::TrainingCapReached,
                    format!("current-level training cap reached for skill {track_id:?}"),
                )
            })?;

        if offered_gold <= 0 {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                "offered gold must be positive",
            ));
        }
        let carried_gold = self
            .carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)
            .map_err(|error| {
                TrainingContractError::new(ActionBlockedReasonV1::InsufficientGold, error.message())
            })?;
        if offered_gold > carried_gold {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::InsufficientGold,
                format!("offered gold {offered_gold} exceeds carried gold {carried_gold}"),
            ));
        }
        let gold_per_rate = self
            .definition
            .catalog
            .rules
            .skills
            .training
            .gold_per_learning_rate;
        if gold_per_rate <= 0 {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                "gold per learning-rate unit must be positive",
            ));
        }
        let offered_units = u64::try_from(offered_gold / gold_per_rate).map_err(|_| {
            TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                "offered learning-rate units do not fit u64",
            )
        })?;
        if offered_units == 0 {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::InsufficientGold,
                format!("offer must contain at least {gold_per_rate} gold"),
            ));
        }
        let purchased_units = available_rate.min(offered_units);
        let spent_gold_u64 = purchased_units
            .checked_mul(u64::try_from(gold_per_rate).map_err(|_| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    "gold per learning-rate unit does not fit u64",
                )
            })?)
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    "training gold multiplication overflow",
                )
            })?;
        let spent_gold = i64::try_from(spent_gold_u64).map_err(|_| {
            TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                "spent training gold does not fit i64",
            )
        })?;
        let unspent_gold = offered_gold.checked_sub(spent_gold).ok_or_else(|| {
            TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                "training offer remainder overflow",
            )
        })?;
        let new_learning_rate = previous_learning_rate
            .checked_add(purchased_units)
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    "learning-rate addition overflow",
                )
            })?;
        let purchased_units_i32 = i32::try_from(purchased_units).map_err(|_| {
            TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                "purchased learning-rate units do not fit i32",
            )
        })?;
        let training_xp = purchased_units_i32
            .checked_mul(
                self.definition
                    .catalog
                    .rules
                    .skills
                    .training
                    .experience_per_learning_rate,
            )
            .filter(|xp| *xp > 0)
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    "training experience multiplication overflow",
                )
            })?;
        let actor = &self.world.actors[actor_index];
        let shared = Transaction {
            id: "skill_training".to_string(),
            label: "Skill training".to_string(),
            requirements: vec![TransactionRequirement::MinimumCarriedGold {
                amount: offered_gold,
            }],
            costs: vec![TransactionCost::CarriedGold { amount: spent_gold }],
            rewards: Vec::new(),
        };
        let transaction = self
            .plan_transaction(
                actor_index,
                TransactionSource::SkillTraining {
                    service_id: service.clone(),
                    capability_id: training.id.clone(),
                    track_id: track_id.clone(),
                },
                &shared,
                None,
                vec![
                    PlannedReward::LearningRate {
                        track_id: track_id.clone(),
                        before: previous_learning_rate,
                        after: new_learning_rate,
                    },
                    PlannedReward::Experience {
                        amount: training_xp,
                    },
                ],
            )
            .map_err(|error| TrainingContractError::new(error.reason(), error.message()))?;
        Ok(TrainingPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            service_id: service,
            track_id,
            offered_gold,
            spent_gold,
            unspent_gold,
            previous_learning_rate,
            new_learning_rate,
            transaction,
        })
    }

    pub(super) fn critique_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        track_id: &str,
    ) -> Result<CritiquePlan, TrainingContractError> {
        let service = self.critique_service(actor_index, service_id)?;
        let catalog = self
            .definition
            .catalog
            .skill_catalog
            .as_ref()
            .ok_or_else(|| {
                TrainingContractError::new(
                    ActionBlockedReasonV1::InvalidTrainingOffer,
                    "critique requires a skill catalog",
                )
            })?;
        if catalog.track(track_id).is_none() {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::InvalidTrainingOffer,
                format!("unknown critique track {track_id:?}"),
            ));
        }
        if !self.skill_track_is_allowed_for_actor(actor_index, track_id) {
            return Err(TrainingContractError::new(
                ActionBlockedReasonV1::WrongClass,
                format!("track {track_id:?} is not available to this character"),
            ));
        }
        let entry = self.skill_entry_for_actor(actor_index, track_id);
        let level = entry.map_or(0, |entry| entry.level);
        let critique_rank = entry
            .map(|entry| entry.critique_rank)
            .filter(|rank| *rank > 0);
        let actor = &self.world.actors[actor_index];
        Ok(CritiquePlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            service_id: service,
            track_id: track_id.to_string(),
            track_display: self.skill_track_display(track_id),
            level,
            critique_rank,
            level_title: self.skill_level_title(track_id, level),
        })
    }

    pub(super) fn apply_player_train(
        &mut self,
        player_index: usize,
        service_id: &str,
        offered_gold: i64,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.training_plan(player_index, service_id, offered_gold)?;
        let mut receipt = self.commit_transaction(player_index, plan.transaction.clone())?;
        let experience_events = receipt
            .delegated_events
            .split_off(1.min(receipt.delegated_events.len()));
        events.append(&mut receipt.delegated_events);
        events.push(Event::TrainingPurchased {
            actor_id: plan.actor_id.clone(),
            actor: plan.actor_name.clone(),
            service_id: plan.service_id.clone(),
            track_id: plan.track_id.clone(),
            offered_gold: plan.offered_gold,
            spent_gold: plan.spent_gold,
            unspent_gold: plan.unspent_gold,
            previous_learning_rate: plan.previous_learning_rate,
            new_learning_rate: plan.new_learning_rate,
        });
        events.extend(experience_events);
        events.push(receipt.committed_event(plan.actor_id, plan.actor_name));
        Ok(())
    }

    pub(super) fn apply_player_critique(
        &mut self,
        player_index: usize,
        service_id: &str,
        track_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.critique_plan(player_index, service_id, track_id)?;
        events.push(Event::SkillCritiqued {
            actor_id: plan.actor_id,
            actor: plan.actor_name,
            service_id: plan.service_id,
            track_id: plan.track_id,
            track_display: plan.track_display,
            level: plan.level,
            critique_rank: plan.critique_rank,
            level_title: plan.level_title,
        });
        Ok(())
    }
}
