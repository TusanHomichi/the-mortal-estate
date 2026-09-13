//! Canonical conversion from model intent to its typed command payload.
use super::*;

impl Engine {
    /// Convert a `PlayerIntent` into its typed payload representation.
    /// This is the canonical conversion; use it instead of duplicating the match.
    pub fn player_intent_to_payload(intent: &PlayerIntent) -> PlayerIntentPayloadV1 {
        match intent {
            PlayerIntent::MovePath(p) => PlayerIntentPayloadV1::MovePath { path: p.clone() },
            PlayerIntent::Traverse(kind) => PlayerIntentPayloadV1::Traverse { kind: *kind },
            PlayerIntent::Hide => PlayerIntentPayloadV1::Hide,
            PlayerIntent::Nock => PlayerIntentPayloadV1::Nock,
            PlayerIntent::UnloadBow => PlayerIntentPayloadV1::UnloadBow,
            PlayerIntent::PhysicalAttack {
                mode,
                target_actor_id,
                authorization,
            } => PlayerIntentPayloadV1::PhysicalAttack {
                mode: *mode,
                target_actor_id: target_actor_id.clone(),
                authorization: *authorization,
            },
            PlayerIntent::SearchCorpse(corpse_id) => PlayerIntentPayloadV1::SearchCorpse {
                corpse_id: corpse_id.clone(),
            },
            PlayerIntent::MoveItem {
                item_instance_id,
                destination,
            } => PlayerIntentPayloadV1::MoveItem {
                item_instance_id: item_instance_id.clone(),
                destination: destination.clone(),
            },
            PlayerIntent::MoveGold {
                source,
                destination,
                quantity,
            } => PlayerIntentPayloadV1::MoveGold {
                source: source.clone(),
                destination: destination.clone(),
                quantity: quantity.clone(),
            },
            PlayerIntent::DepositBankGold {
                service_id,
                capability_id,
                gold_pile_id,
            } => PlayerIntentPayloadV1::DepositBankGold {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                gold_pile_id: gold_pile_id.clone(),
            },
            PlayerIntent::WithdrawBankGold {
                service_id,
                capability_id,
                amount,
            } => PlayerIntentPayloadV1::WithdrawBankGold {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                amount: *amount,
            },
            PlayerIntent::DepositLockerItem {
                service_id,
                capability_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::DepositLockerItem {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::WithdrawLockerItem {
                service_id,
                capability_id,
                item_instance_id,
                destination,
            } => PlayerIntentPayloadV1::WithdrawLockerItem {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
                destination: *destination,
            },
            PlayerIntent::OfferItem {
                recipient_character_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::OfferItem {
                recipient_character_id: recipient_character_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::AcceptItemOffer {
                item_instance_id,
                destination,
            } => PlayerIntentPayloadV1::AcceptItemOffer {
                item_instance_id: item_instance_id.clone(),
                destination: *destination,
            },
            PlayerIntent::RefuseItemOffer { item_instance_id } => {
                PlayerIntentPayloadV1::RefuseItemOffer {
                    item_instance_id: item_instance_id.clone(),
                }
            }
            PlayerIntent::WithdrawItemOffer { item_instance_id } => {
                PlayerIntentPayloadV1::WithdrawItemOffer {
                    item_instance_id: item_instance_id.clone(),
                }
            }
            PlayerIntent::Drink(id) => PlayerIntentPayloadV1::Drink {
                item_instance_id: id.clone(),
            },
            PlayerIntent::Open(d) => PlayerIntentPayloadV1::Open { direction: *d },
            PlayerIntent::Close(d) => PlayerIntentPayloadV1::Close { direction: *d },
            PlayerIntent::ShowSack => PlayerIntentPayloadV1::ShowSack,
            PlayerIntent::RequestResurrection => PlayerIntentPayloadV1::RequestResurrection,
            PlayerIntent::Wait => PlayerIntentPayloadV1::Wait,
            PlayerIntent::Inspect => PlayerIntentPayloadV1::Inspect,
            PlayerIntent::Train {
                service_id,
                offered_gold,
            } => PlayerIntentPayloadV1::Train {
                service_id: service_id.clone(),
                offered_gold: *offered_gold,
            },
            PlayerIntent::Critique {
                service_id,
                track_id,
            } => PlayerIntentPayloadV1::Critique {
                service_id: service_id.clone(),
                track_id: track_id.clone(),
            },
            PlayerIntent::PromoteClass(target) => PlayerIntentPayloadV1::PromoteClass {
                target_class_id: target.clone(),
            },
            PlayerIntent::LearnSpell(spell_id) => PlayerIntentPayloadV1::LearnSpell {
                spell_id: spell_id.clone(),
            },
            PlayerIntent::CommitServiceTransaction {
                service_id,
                capability_id,
                transaction_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::CommitServiceTransaction {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                transaction_id: transaction_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::BuyFromMerchant {
                service_id,
                capability_id,
                item_instance_ids,
            } => PlayerIntentPayloadV1::BuyFromMerchant {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_ids: item_instance_ids.clone(),
            },
            PlayerIntent::SellToMerchant {
                service_id,
                capability_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::SellToMerchant {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::UseItemService {
                service_id,
                capability_id,
                operation,
                item_instance_id,
            } => PlayerIntentPayloadV1::UseItemService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation: *operation,
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::UseRestorationService {
                service_id,
                capability_id,
                operation_id,
                item_instance_id,
                corpse_id,
            } => PlayerIntentPayloadV1::UseRestorationService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation_id: operation_id.clone(),
                item_instance_id: item_instance_id.clone(),
                corpse_id: corpse_id.clone(),
            },
            PlayerIntent::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::InteractWithNpc {
                npc_actor_id: npc_actor_id.clone(),
                interaction_id: interaction_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::CastSpell {
                spell_id,
                target,
                authorization,
            } => PlayerIntentPayloadV1::CastSpell {
                spell_id: spell_id.clone(),
                target: target.clone(),
                authorization: *authorization,
            },
            PlayerIntent::WarmSpell { spell_id } => PlayerIntentPayloadV1::WarmSpell {
                spell_id: spell_id.clone(),
            },
            PlayerIntent::CastWarmedSpell {
                target,
                authorization,
            } => PlayerIntentPayloadV1::CastWarmedSpell {
                target: target.clone(),
                authorization: *authorization,
            },
            PlayerIntent::ClearSelfDefense {
                attacker_character_id,
            } => PlayerIntentPayloadV1::ClearSelfDefense {
                attacker_character_id: attacker_character_id.clone(),
            },
            PlayerIntent::FizzleWarmedSpell => PlayerIntentPayloadV1::FizzleWarmedSpell,
            PlayerIntent::Rest => PlayerIntentPayloadV1::Rest,
        }
    }

    pub fn actor_command_for_intent(
        &self,
        actor_id: &crate::model::ActorId,
        intent: &PlayerIntent,
    ) -> Result<PlayerCommandV1, StepError> {
        self.player_actor_index(actor_id)?;

        Ok(PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent: Self::player_intent_to_payload(intent),
        })
    }
}
