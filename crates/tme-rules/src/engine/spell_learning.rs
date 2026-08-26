use crate::events::Event;
use crate::model::{KnownSpell, Transaction, TransactionCost, TransactionRequirement};
use crate::view::ActionBlockedReasonV1;

use super::inventory::SpellBookReceipt;
use super::transactions::{PlannedReward, TransactionPlan, TransactionSource};
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpellLearningPlan {
    pub spell_id: String,
    pub spell_name: String,
    pub lane: String,
    pub skill_requirement: i32,
    pub learned_at_level: i32,
    pub gold_cost: i64,
    pub trainer_service_id: String,
    pub trainer_name: String,
    pub spell_book: SpellBookReceipt,
    pub transaction: TransactionPlan,
}

impl Engine {
    pub(super) fn apply_transaction_spell_reward(
        &mut self,
        actor_index: usize,
        spell_id: &str,
        lane: &str,
        learned_at_level: i32,
    ) -> Result<(), StepError> {
        let character = self.world.actors[actor_index]
            .character
            .as_mut()
            .ok_or_else(|| StepError::new("spell reward requires character sheet"))?;
        character.known_spells.push(KnownSpell {
            spell_id: spell_id.to_string(),
            lane: lane.to_string(),
            learned_at_level,
        });
        Ok(())
    }

    pub(super) fn validate_learn_spell_command(
        &self,
        player_index: usize,
        spell_id: &str,
    ) -> Result<SpellLearningPlan, ActionBlockedReasonV1> {
        let spell = self
            .definition
            .catalog
            .spells
            .get(spell_id)
            .ok_or(ActionBlockedReasonV1::NoSuchSpell)?;
        let player = self
            .world
            .actors
            .get(player_index)
            .ok_or(ActionBlockedReasonV1::NoSuchTarget)?;
        let character = player
            .character
            .as_ref()
            .ok_or(ActionBlockedReasonV1::SpellNotKnown)?;

        if character
            .known_spells
            .iter()
            .any(|known_spell| known_spell.spell_id == spell_id)
        {
            return Err(ActionBlockedReasonV1::SpellAlreadyKnown);
        }

        let lane = spell
            .lane
            .as_deref()
            .ok_or(ActionBlockedReasonV1::WrongClass)?;
        if lane == "knight_magic"
            || !super::action_context::class_spell_lanes(
                character.identity.current_class_id.as_str(),
            )
            .contains(&lane)
        {
            return Err(ActionBlockedReasonV1::WrongClass);
        }

        let authored_trainers = self.spell_teachers_for(spell_id);
        if authored_trainers.is_empty() {
            return Err(ActionBlockedReasonV1::NoService);
        }
        let (trainer, teaching) = authored_trainers
            .into_iter()
            .find(|(service, _)| {
                service.position().level == player.location.level
                    && service.position().position == player.location.position
            })
            .ok_or(ActionBlockedReasonV1::ServiceNotHere)?;
        let training = self
            .referenced_training_capability(trainer, teaching)
            .ok_or(ActionBlockedReasonV1::NoService)?;
        if !training.offers.iter().any(|offer| {
            offer.track_id == lane
                && offer
                    .eligible_class_ids
                    .iter()
                    .any(|class_id| class_id == &character.identity.current_class_id)
        }) {
            return Err(ActionBlockedReasonV1::WrongClass);
        }

        let skill_requirement = spell
            .skill_requirement
            .filter(|requirement| *requirement > 0)
            .ok_or(ActionBlockedReasonV1::SkillLevelTooLow)?;
        let current_level = character
            .skill_ledger
            .iter()
            .find(|entry| entry.track_id == lane)
            .map(|entry| i32::from(entry.level))
            .unwrap_or(0);
        if current_level < skill_requirement {
            return Err(ActionBlockedReasonV1::SkillLevelTooLow);
        }
        if spell.mp_cost.is_none_or(|cost| cost <= 0) {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        }

        let spell_book = self.right_hand_spell_book(player_index, lane)?;
        let gold_cost = i64::from(
            spell
                .acquisition
                .as_ref()
                .ok_or(ActionBlockedReasonV1::NoService)?
                .gold_cost,
        );
        if self
            .carried_gold_at(player_index, crate::model::CarriedGoldPosition::Sack)
            .unwrap_or(0)
            < gold_cost
        {
            return Err(ActionBlockedReasonV1::InsufficientGold);
        }

        let shared = Transaction {
            id: "spell_learning".to_string(),
            label: "Spell learning".to_string(),
            requirements: vec![
                TransactionRequirement::CurrentClass {
                    class_id: character.identity.current_class_id.clone(),
                },
                TransactionRequirement::MinimumSkillLevel {
                    track_id: lane.to_string(),
                    level: u8::try_from(skill_requirement)
                        .map_err(|_| ActionBlockedReasonV1::SkillLevelTooLow)?,
                },
                TransactionRequirement::MinimumCarriedGold { amount: gold_cost },
                TransactionRequirement::CarriedItem {
                    item_definition_id: spell_book.item_definition_id.clone(),
                    quantity: 1,
                },
                TransactionRequirement::SpellUnknown {
                    spell_id: spell.id.clone(),
                },
            ],
            costs: vec![TransactionCost::CarriedGold { amount: gold_cost }],
            rewards: Vec::new(),
        };
        let transaction = self
            .plan_transaction(
                player_index,
                TransactionSource::SpellLearning {
                    service_id: trainer.id().to_string(),
                    capability_id: teaching.id.clone(),
                    spell_id: spell.id.clone(),
                },
                &shared,
                Some(&spell_book.item_instance_id),
                vec![PlannedReward::Spell {
                    spell_id: spell.id.clone(),
                    lane: lane.to_string(),
                    learned_at_level: character.progression.level,
                }],
            )
            .map_err(|error| error.reason())?;

        Ok(SpellLearningPlan {
            spell_id: spell.id.clone(),
            spell_name: spell.name.clone(),
            lane: lane.to_string(),
            skill_requirement,
            learned_at_level: character.progression.level,
            gold_cost,
            trainer_service_id: trainer.id().to_string(),
            trainer_name: trainer.name().to_string(),
            spell_book,
            transaction,
        })
    }

    pub(super) fn apply_player_learn_spell(
        &mut self,
        player_index: usize,
        spell_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self
            .validate_learn_spell_command(player_index, spell_id)
            .map_err(Self::spell_command_error)?;

        let actor_id = self.world.actors[player_index].id.clone();
        let actor_name = self.world.actors[player_index].name.clone();
        let mut receipt = self.commit_transaction(player_index, plan.transaction.clone())?;
        events.append(&mut receipt.delegated_events);
        events.push(Event::SpellLearned {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            lane: plan.lane.clone(),
            skill_requirement: plan.skill_requirement,
            learned_at_level: plan.learned_at_level,
            gold_cost: plan.gold_cost,
            trainer_service_id: plan.trainer_service_id.clone(),
            trainer: plan.trainer_name.clone(),
            spell_book_item_instance_id: plan.spell_book.item_instance_id.clone(),
            spell_book_item_definition_id: plan.spell_book.item_definition_id.clone(),
            spell_book: plan.spell_book.item_name.clone(),
            spell_book_character_id: plan.spell_book.character_id.clone(),
        });
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }
}
