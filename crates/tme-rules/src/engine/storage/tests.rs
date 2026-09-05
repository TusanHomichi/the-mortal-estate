use super::*;
use crate::engine::death::DefeatContext;
use crate::events::{BankBalanceChangeReasonV1, TransactionSourceV1};
use crate::model::{
    ActorTimingState, CarriedGold, DeathCause, GoldMoveDestination, GoldMoveQuantity,
    GoldMoveSource, ItemBindingState, LogicalTime, LootClaim, LootClaimBasis, LootOwnerId,
};
use crate::view::{PlayerIntentPayloadV1, ServiceCapabilityViewV1};
use serde_json::json;

fn character_id(value: &str) -> CharacterId {
    serde_json::from_value(json!(value)).expect("character ID")
}

fn storage_engine() -> Engine {
    use crate::content::{
        BankDef, CatalogRegistryKey, LockerVaultDef, ServiceCapabilityDef, ServiceDefinitionDef,
        ServiceInstanceSeedDef,
    };

    let (mut catalog, profile, template, mut seed) =
        crate::engine::setup::test_parts("gold_training");
    let profile_def = catalog.profiles.get_mut(&profile).expect("test profile");
    for (key, bank) in [
        (
            "bank/test/shared",
            BankDef {
                id: "shared_bank".to_string(),
                transaction_cap_gold: 200,
            },
        ),
        (
            "bank/test/isolated",
            BankDef {
                id: "isolated_bank".to_string(),
                transaction_cap_gold: 75,
            },
        ),
    ] {
        let key = CatalogRegistryKey::from(key);
        catalog.banks.insert(key.clone(), bank);
        profile_def.banks.push(key);
    }
    for (key, vault) in [
        (
            "vault/test/shared",
            LockerVaultDef {
                id: "shared_vault".to_string(),
                capacity: 2,
            },
        ),
        (
            "vault/test/isolated",
            LockerVaultDef {
                id: "isolated_vault".to_string(),
                capacity: 1,
            },
        ),
    ] {
        let key = CatalogRegistryKey::from(key);
        catalog.locker_vaults.insert(key.clone(), vault);
        profile_def.locker_vaults.push(key);
    }
    for (suffix, name, bank_id, vault_id) in [
        ("a", "Storage Counter A", "shared_bank", "shared_vault"),
        ("b", "Storage Counter B", "shared_bank", "shared_vault"),
        ("c", "Storage Counter C", "isolated_bank", "isolated_vault"),
    ] {
        let definition_id = format!("storage_counter_{suffix}");
        let key = CatalogRegistryKey::from(format!("service/test/{suffix}"));
        catalog.service_definitions.insert(
            key.clone(),
            ServiceDefinitionDef {
                id: definition_id.clone(),
                name: name.to_string(),
                capabilities: vec![
                    ServiceCapabilityDef::Bank {
                        id: format!("bank_{suffix}"),
                        bank_id: bank_id.to_string(),
                    },
                    ServiceCapabilityDef::Locker {
                        id: format!("locker_{suffix}"),
                        vault_id: vault_id.to_string(),
                    },
                ],
            },
        );
        profile_def.service_definitions.push(key);
        seed.service_instances.push(ServiceInstanceSeedDef {
            id: definition_id.clone(),
            service_definition_id: definition_id,
            location: crate::model::WorldPosition::new(
                "realm_0",
                "room_0",
                crate::model::Coord { x: 1, y: 1 },
            ),
        });
    }
    crate::engine::setup::test_engine_from_parts(catalog, profile, template, seed)
}

fn add_recipient(engine: &mut Engine) -> (usize, CharacterId) {
    let recipient_character_id = character_id("character:storage:recipient");
    let mut recipient = engine.world.actors[0].clone();
    recipient.id = "recipient".into();
    recipient.name = "Recipient".to_string();
    recipient.character_id = Some(recipient_character_id.clone());
    recipient.carried.items.clear();
    recipient.carried.gold = CarriedGold::default();
    recipient.timing = ActorTimingState {
        ready_at: LogicalTime::FIRST,
        tie_break_order: engine.world.timing.next_tie_break_order,
    };
    engine.world.timing.next_tie_break_order += 1;
    engine.world.actors.push(recipient);
    (engine.world.actors.len() - 1, recipient_character_id)
}

fn create_offer(engine: &mut Engine) -> (usize, CharacterId, Vec<Event>) {
    let (recipient_index, recipient_character_id) = add_recipient(engine);
    let mut events = Vec::new();
    engine
        .apply_item_offer(0, &recipient_character_id, "training_sword", &mut events)
        .expect("offer creation");
    (recipient_index, recipient_character_id, events)
}

mod positioned_gold_split_collect_and_hand_collision_are_atomic;

mod offers_preserve_binding_unwind_for_death_and_reject_missing_parties;
