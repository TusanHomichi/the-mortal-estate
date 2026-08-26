use super::{
    ActorInstanceState, active_effect_from_def, actor_state_from_definition,
    carried_layout_from_def, character_sheet_from_actor, npc_state_from_def,
};
use crate::content::WorldSeedDef;
use crate::engine::{GameDefinition, StepError};
use crate::model::{
    ActorTimingState, BankState, BowReadiness, DoorState, GroundItem, ItemInstanceState,
    ItemKnowledgeState, LockerVaultState, LogicalTime, MerchantInventoryId, MerchantInventoryState,
    MerchantListingOrigin, MerchantListingState, ServiceInstanceState, SocialRelationLedger, World,
    WorldTimingState,
};

pub(super) fn seed(definition: &GameDefinition, source: &WorldSeedDef) -> Result<World, StepError> {
    let actors = source
        .actors
        .iter()
        .enumerate()
        .map(|(tie_break_order, actor)| {
            let actor_definition = definition
                .catalog
                .actor_definitions
                .get(&actor.actor_definition_id)
                .expect("validated actor definition");
            let character = character_sheet_from_actor(actor);
            let is_ready_automatic = actor_definition.kind != crate::model::ActorKind::Npc
                || actor_definition.ai.is_some();
            actor_state_from_definition(
                actor_definition,
                ActorInstanceState {
                    id: actor.id.clone(),
                    location: actor.location.clone(),
                    hp: character
                        .as_ref()
                        .map(|character| character.resources.hp)
                        .unwrap_or(actor_definition.stats.hp),
                    mp: character
                        .as_ref()
                        .map(|character| character.resources.mp)
                        .unwrap_or(0),
                    stamina: character
                        .as_ref()
                        .map(|character| character.resources.stamina)
                        .unwrap_or(10),
                    timing: ActorTimingState {
                        ready_at: if is_ready_automatic {
                            LogicalTime::FIRST
                        } else {
                            LogicalTime::new(u64::MAX)
                        },
                        tie_break_order: u64::try_from(tie_break_order)
                            .expect("actor index should fit u64"),
                    },
                    attack_ready_at: if is_ready_automatic {
                        LogicalTime::FIRST
                    } else {
                        LogicalTime::new(u64::MAX)
                    },
                    carried: carried_layout_from_def(&actor.carried),
                    npc: actor.npc.as_ref().map(npc_state_from_def),
                    character_id: actor.character_id.clone(),
                    character,
                    active_effects: actor
                        .active_effects
                        .iter()
                        .map(active_effect_from_def)
                        .collect(),
                    summoned: None,
                    ecology_origin: None,
                },
            )
        })
        .collect::<Vec<_>>();

    let item_instances = source
        .item_instances
        .iter()
        .map(|(instance_id, instance)| {
            (
                instance_id.clone(),
                ItemInstanceState {
                    definition_id: instance.definition_id.clone(),
                    quantity: instance.quantity,
                    knowledge: ItemKnowledgeState {
                        identified: instance.knowledge.identified,
                        appraised: instance.knowledge.appraised,
                    },
                    binding: instance.binding.clone(),
                    bow_readiness: definition
                        .catalog
                        .item_catalog
                        .get(&instance.definition_id)
                        .and_then(|item| item.weapon.as_ref())
                        .and_then(|weapon| {
                            (weapon.handedness == crate::model::WeaponHandedness::Bow)
                                .then_some(BowReadiness::Unnocked)
                        }),
                },
            )
        })
        .collect();
    let ground_items = source
        .ground_items
        .iter()
        .map(|ground| GroundItem {
            item_instance_id: ground.item_instance_id.clone(),
            location: ground.location.clone(),
            loot_claim: None,
        })
        .collect();
    let service_instances = source
        .service_instances
        .iter()
        .map(|instance| ServiceInstanceState {
            id: instance.id.clone(),
            definition_id: instance.service_definition_id.clone(),
            position: instance.location.clone(),
        })
        .collect();
    let merchant_inventories = source
        .merchant_inventories
        .iter()
        .map(|inventory| {
            (
                MerchantInventoryId::new(&inventory.service_instance_id, &inventory.capability_id),
                MerchantInventoryState {
                    listings: inventory
                        .stock
                        .iter()
                        .map(|listing| MerchantListingState {
                            item_instance_id: listing.item_instance_id.clone(),
                            origin: MerchantListingOrigin::AuthoredStock,
                            price_gold: listing.price_gold,
                        })
                        .collect(),
                },
            )
        })
        .collect();
    let banks = definition
        .catalog
        .bank_definitions
        .keys()
        .cloned()
        .map(|id| {
            (
                id,
                BankState {
                    balances: std::collections::BTreeMap::new(),
                },
            )
        })
        .collect();
    let locker_vaults = definition
        .catalog
        .locker_vault_definitions
        .keys()
        .cloned()
        .map(|id| {
            (
                id,
                LockerVaultState {
                    lockers: std::collections::BTreeMap::new(),
                },
            )
        })
        .collect();

    let door_states = definition
        .world_template
        .navigation
        .iter()
        .filter_map(|(location, edges)| {
            edges
                .iter()
                .find(|edge| edge.kind == crate::model::NavigationKind::Door)?
                .initial_state
                .map(|state| (location.clone(), matches!(state, DoorState::Open)))
        })
        .collect();
    let hidden_transition_revealed = definition
        .world_template
        .navigation
        .iter()
        .filter(|(_, edges)| edges.iter().any(|edge| edge.hidden))
        .map(|(location, _)| (location.clone(), false))
        .collect();

    let ecology_sites = source
        .ecology_sites
        .iter()
        .map(|site| {
            let spawn_group_id = match &site.source {
                crate::content::EcologySiteSourceDef::SpawnGroup { spawn_group_id } => {
                    spawn_group_id.clone()
                }
                crate::content::EcologySiteSourceDef::Lair { lair_definition_id } => definition
                    .catalog
                    .lair_definitions
                    .get(lair_definition_id)
                    .expect("validated lair definition")
                    .spawn_group_id
                    .clone(),
            };
            (
                site.id.clone(),
                crate::model::EcologySiteState {
                    id: site.id.clone(),
                    spawn_group_id,
                    generation: 0,
                    member_slots: site
                        .member_locations
                        .iter()
                        .map(|(member_id, location)| {
                            (
                                member_id.clone(),
                                crate::model::EcologyMemberSlotState {
                                    member_id: member_id.clone(),
                                    location: location.clone(),
                                    actor_id: None,
                                    due_at: None,
                                },
                            )
                        })
                        .collect(),
                    full_clear_due_at: None,
                },
            )
        })
        .collect();

    let communication_preferences = actors
        .iter()
        .filter_map(|actor| {
            actor.character_id.clone().map(|character_id| {
                (
                    character_id,
                    crate::model::CommunicationPreferences::default(),
                )
            })
        })
        .collect();
    let character_presence = actors
        .iter()
        .filter_map(|actor| {
            actor.character_id.clone().map(|character_id| {
                (
                    character_id,
                    crate::model::CharacterPresenceState {
                        connected: true,
                        control_epoch: 0,
                        absent_since: None,
                    },
                )
            })
        })
        .collect();

    Ok(World {
        timing: WorldTimingState {
            now: LogicalTime::FIRST,
            next_tie_break_order: u64::try_from(actors.len()).expect("actor count should fit u64"),
        },
        actors,
        ecology_sites,
        social_relations: SocialRelationLedger::default(),
        groups: std::collections::BTreeMap::new(),
        group_invitations: std::collections::BTreeMap::new(),
        player_follow_targets: std::collections::BTreeMap::new(),
        communication_preferences,
        character_presence,
        defeat_contributions: std::collections::BTreeMap::new(),
        item_instances,
        service_instances,
        merchant_inventories,
        banks,
        locker_vaults,
        item_offers: std::collections::BTreeMap::new(),
        quest_states: std::collections::BTreeMap::new(),
        ground_items,
        corpses: std::collections::BTreeMap::new(),
        ground_gold: std::collections::BTreeMap::new(),
        next_corpse_sequence: 1,
        next_gold_sequence: 1,
        next_summon_sequence: 0,
        next_group_sequence: 1,
        next_group_invite_sequence: 1,
        next_membership_epoch: 1,
        next_player_kill_sequence: 1,
        linked_player_kill_karma: Vec::new(),
        tile_effects: Vec::new(),
        item_enchantments: Vec::new(),
        portal_transitions: Vec::new(),
        concealed_transitions: Vec::new(),
        hidden_transition_revealed,
        door_states,
    })
}
