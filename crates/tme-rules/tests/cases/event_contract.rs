use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    BanishResultReasonV1, COMMAND_CONTRACT_VERSION, CarriedGoldPosition, CarriedPosition, Coord,
    CorpseId, CreatureTrait, Direction, EVENT_CONTRACT_VERSION, EcologyLifecyclePolicyV1, Engine,
    Event, GoldLocationViewV1, ItemConsumptionReason, ItemLocationViewV1, LogicalTime,
    MagicArithmeticRounding, MagicPrimaryAttribute, NavigationKind, PlayerCommandV1, PlayerIntent,
    RaiseDeadResultReasonV1, ResistanceBoostSourceKind, ResourceActivity, ResourceKind,
    RestorationStatusKind, ResurrectionMethod, SpellCastClass, SpellCastFailure,
    SpellCastingMethod, SpellFizzleCause, SpellPathFailureReason, SpellResistanceMitigationMode,
    SpellTarget, TransactionCostReceiptV1, TransactionRewardReceiptV1, TransactionSourceV1,
    TransitionConcealmentRemovalReasonV1, VerticalDirection, WorldPosition,
};

fn item_contract_parts() -> ContentParts {
    ContentParts::tracked("item_instance_contract", "profile/item_instance_contract")
}

fn item_contract_engine() -> Engine {
    item_contract_parts()
        .engine(7)
        .expect("item instance contract engine should start")
}

fn event_payload(events: &[Event], event_name: &str) -> serde_json::Value {
    let serialized = serde_json::to_value(events).expect("events should serialize");
    let values = serialized.as_array().expect("events serialize as an array");
    values
        .iter()
        .find_map(|event| event.get(event_name))
        .cloned()
        .unwrap_or_else(|| panic!("missing serialized {event_name} event"))
}

#[path = "event_contract/event_40_ecology_lifecycle_payloads_are_exact_and_strict.rs"]
mod event_40_ecology_lifecycle_payloads_are_exact_and_strict;

#[path = "event_contract/event_34_spell_effect_family_shapes_are_exact_and_strict.rs"]
mod event_34_spell_effect_family_shapes_are_exact_and_strict;

#[path = "event_contract/hide_events_serialize_with_contract_fields.rs"]
mod hide_events_serialize_with_contract_fields;
