//! Fighter-to-Knight promotion domain planning over the shared transaction owner.

use crate::events::{ClassDemotionReasonV1, Event, PromotionSpellGrantViewV1};
use crate::model::{
    CharacterId, ClassPromotionCapability, PromotionEntry, TransactionRequirement,
    TransactionReward,
};
use crate::view::ActionBlockedReasonV1;

use super::transactions::{TransactionPlan, TransactionSource};
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PromotionContractError {
    reason: ActionBlockedReasonV1,
    message: String,
}

impl PromotionContractError {
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

impl From<PromotionContractError> for StepError {
    fn from(error: PromotionContractError) -> Self {
        StepError::new(error.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedSpellGrant {
    spell_id: String,
    spell_name: String,
    lane: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PromotionPlan {
    actor_id: crate::model::ActorId,
    actor_name: String,
    from_class_display: String,
    to_class_display: String,
    granted_item_instance_id: String,
    granted_item_definition_id: String,
    granted_item_name: String,
    granted_item_position: crate::model::CarriedPosition,
    granted_spells: Vec<PlannedSpellGrant>,
    transaction: TransactionPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClassDemotionPlan {
    actor_id: crate::model::ActorId,
    character_id: CharacterId,
    expected_promotion_history_len: usize,
    victim_actor_id: crate::model::ActorId,
}

impl Engine {
    pub(super) fn class_demotion_plan(
        &self,
        actor_index: usize,
        victim_actor_id: &crate::model::ActorId,
    ) -> Result<Option<ClassDemotionPlan>, StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("class demotion actor disappeared"))?;
        let Some(character) = actor.character.as_ref() else {
            return Ok(None);
        };
        if character.identity.current_class_id != "knight" {
            return Ok(None);
        }
        let character_id = actor
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("class demotion character identity disappeared"))?;
        Ok(Some(ClassDemotionPlan {
            actor_id: actor.id.clone(),
            character_id,
            expected_promotion_history_len: character.promotion_history.len(),
            victim_actor_id: victim_actor_id.clone(),
        }))
    }

    pub(super) fn commit_class_demotion(
        &mut self,
        plan: &ClassDemotionPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = self
            .world
            .actors
            .iter_mut()
            .find(|actor| actor.id == plan.actor_id)
            .ok_or_else(|| StepError::new("class demotion actor changed before commit"))?;
        if actor.character_id.as_ref() != Some(&plan.character_id) {
            return Err(StepError::new(
                "class demotion character identity changed before commit",
            ));
        }
        let character = actor
            .character
            .as_mut()
            .ok_or_else(|| StepError::new("class demotion character sheet disappeared"))?;
        if character.identity.current_class_id != "knight"
            || character.promotion_history.len() != plan.expected_promotion_history_len
        {
            return Err(StepError::new("class demotion facts changed before commit"));
        }
        character.identity.current_class_id = "fighter".to_string();
        character.identity.display_class = "Fighter".to_string();
        events.push(Event::ClassDemoted {
            actor_id: plan.actor_id.clone(),
            character_id: plan.character_id.clone(),
            from_class_id: "knight".to_string(),
            to_class_id: "fighter".to_string(),
            reason: ClassDemotionReasonV1::UnjustLawfulHumanKill,
            victim_actor_id: plan.victim_actor_id.clone(),
        });
        Ok(())
    }

    pub(super) fn apply_transaction_class_reward(
        &mut self,
        actor_index: usize,
        from_class_id: &str,
        to_class_id: &str,
        to_class_display: &str,
        level: i32,
    ) -> Result<(), StepError> {
        let character = self.world.actors[actor_index]
            .character
            .as_mut()
            .ok_or_else(|| StepError::new("class reward requires character sheet"))?;
        character.identity.current_class_id = to_class_id.to_string();
        character.identity.display_class = to_class_display.to_string();
        character.promotion_history.push(PromotionEntry {
            from_class_id: from_class_id.to_string(),
            to_class_id: to_class_id.to_string(),
            level,
        });
        Ok(())
    }

    fn promotion_service_at_actor(
        &self,
        actor_index: usize,
        target_class_id: &str,
    ) -> Result<(String, ClassPromotionCapability), PromotionContractError> {
        self.world.actors.get(actor_index).ok_or_else(|| {
            PromotionContractError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let matches = self.promotion_capabilities_at_actor(actor_index, target_class_id);
        match matches.as_slice() {
            [(service, promotion)] => Ok((service.id().to_string(), (*promotion).clone())),
            [] => Err(PromotionContractError::new(
                ActionBlockedReasonV1::NoService,
                format!(
                    "no promotion service for target {target_class_id:?} at the actor coordinate"
                ),
            )),
            _ => Err(PromotionContractError::new(
                ActionBlockedReasonV1::NoService,
                format!(
                    "ambiguous promotion services for target {target_class_id:?} at the actor coordinate"
                ),
            )),
        }
    }

    pub(super) fn promotion_plan(
        &self,
        actor_index: usize,
        target_class_id: &str,
    ) -> Result<PromotionPlan, PromotionContractError> {
        let (service, promotion) = self.promotion_service_at_actor(actor_index, target_class_id)?;
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            PromotionContractError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let character = actor.character.as_ref().ok_or_else(|| {
            PromotionContractError::new(
                ActionBlockedReasonV1::NotReady,
                "player has no character sheet",
            )
        })?;

        let _from_class_id = promotion
            .transaction
            .requirements
            .iter()
            .find_map(|requirement| match requirement {
                TransactionRequirement::CurrentClass { class_id } => Some(class_id.as_str()),
                _ => None,
            })
            .ok_or_else(|| {
                PromotionContractError::new(
                    ActionBlockedReasonV1::NotReady,
                    "promotion transaction has no source class",
                )
            })?;
        let (to_class_id, to_class_display) = promotion
            .transaction
            .rewards
            .iter()
            .find_map(|reward| match reward {
                TransactionReward::Class {
                    to_class_id,
                    to_class_display,
                } => Some((to_class_id.as_str(), to_class_display.as_str())),
                _ => None,
            })
            .ok_or_else(|| {
                PromotionContractError::new(
                    ActionBlockedReasonV1::NotReady,
                    "promotion transaction has no class reward",
                )
            })?;
        if to_class_id != target_class_id {
            return Err(PromotionContractError::new(
                ActionBlockedReasonV1::WrongClass,
                "promotion target does not match the addressed transaction",
            ));
        }
        let (item_instance_id, item_definition_id, item_position) = promotion
            .transaction
            .rewards
            .iter()
            .find_map(|reward| match reward {
                TransactionReward::Item {
                    item_instance_id,
                    item_definition_id,
                    position,
                } => Some((item_instance_id, item_definition_id, *position)),
                _ => None,
            })
            .ok_or_else(|| {
                PromotionContractError::new(
                    ActionBlockedReasonV1::NotReady,
                    "promotion transaction has no item reward",
                )
            })?;
        let item = self
            .definition
            .catalog
            .item_catalog
            .get(item_definition_id)
            .ok_or_else(|| {
                PromotionContractError::new(
                    ActionBlockedReasonV1::NotReady,
                    "promotion grant definition is missing",
                )
            })?;
        let mut granted_spells = Vec::new();
        for spell_id in promotion.transaction.rewards.iter().filter_map(|reward| {
            if let TransactionReward::Spell { spell_id } = reward {
                Some(spell_id)
            } else {
                None
            }
        }) {
            let spell = self
                .definition
                .catalog
                .spells
                .get(spell_id)
                .ok_or_else(|| {
                    PromotionContractError::new(
                        ActionBlockedReasonV1::NotReady,
                        format!("promotion spell {spell_id:?} is missing"),
                    )
                })?;
            granted_spells.push(PlannedSpellGrant {
                spell_id: spell.id.clone(),
                spell_name: spell.name.clone(),
                lane: spell.lane.clone().unwrap_or_default(),
            });
        }

        let source = TransactionSource::ClassPromotion {
            service_id: service,
            capability_id: promotion.id.clone(),
            transaction_id: promotion.transaction.id.clone(),
            target_class_id: to_class_id.to_string(),
        };
        let transaction = self
            .plan_transaction(
                actor_index,
                source,
                &promotion.transaction,
                None,
                Vec::new(),
            )
            .map_err(|error| PromotionContractError::new(error.reason(), error.message()))?;

        Ok(PromotionPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            from_class_display: character.identity.display_class.clone(),
            to_class_display: to_class_display.to_string(),
            granted_item_instance_id: item_instance_id.clone(),
            granted_item_definition_id: item_definition_id.clone(),
            granted_item_name: item.name.clone(),
            granted_item_position: item_position,
            granted_spells,
            transaction,
        })
    }

    pub(super) fn apply_player_promotion(
        &mut self,
        actor_index: usize,
        target_class_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.promotion_plan(actor_index, target_class_id)?;
        let mut receipt = self.commit_transaction(actor_index, plan.transaction)?;
        events.append(&mut receipt.delegated_events);
        events.push(Event::ClassPromoted {
            actor_id: plan.actor_id.clone(),
            actor: plan.actor_name.clone(),
            from_class: plan.from_class_display,
            to_class: plan.to_class_display,
            granted_item_instance_id: plan.granted_item_instance_id,
            granted_item_definition_id: plan.granted_item_definition_id,
            granted_item: plan.granted_item_name,
            granted_item_position: plan.granted_item_position,
            granted_spells: plan
                .granted_spells
                .into_iter()
                .map(|spell| PromotionSpellGrantViewV1 {
                    spell_id: spell.spell_id,
                    spell_name: spell.spell_name,
                    lane: spell.lane,
                })
                .collect(),
        });
        events.push(receipt.committed_event(plan.actor_id, plan.actor_name));
        Ok(())
    }
}
