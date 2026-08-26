use std::collections::BTreeMap;

use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::model::{
    CarriedGoldPosition, CharacterAlignment, ItemBindingState, LootClaim, LootClaimBasis,
    LootOwnerId, SocialAlignmentSource, SocialBehavior,
};
use tme_rules::{
    ActorId, ActorKind, AutomaticActorDecisionV1, AutomaticMovementPurposeV1, CarriedPosition,
    CharacterId, Coord, CorpseId, CorpseState, Event, GoldPileId, GroundGoldPile, GroundItem,
    HostilityAuthorization, ItemRelocationReason, LogicalTime, PhysicalAttackMode, PlayerIntent,
    WeaponFumbleReason,
};

const SCAVENGER: &str = "road_scavenger";
const PROFILE: &str = "scavenging/original_provisional";

fn parts_with_profile(mut profile: Value) -> ContentParts {
    let mut parts = ContentParts::tracked(
        "town_adventure_loop_gallery",
        "profile/town_adventure_loop_gallery",
    );
    profile["search_radius"] = profile
        .get("search_radius")
        .cloned()
        .unwrap_or_else(|| json!(6));
    parts.catalog["scavenging_profiles"][PROFILE] = profile;
    let definition = parts.actor_definition_by_actor_id_mut(SCAVENGER);
    definition["social"]["alignment_source"] = json!({"kind": "inherent", "alignment": "neutral"});
    definition["social"]["behavior"] = json!("passive");
    definition["stats"] = json!({"hp": 10, "attack": 10, "defense": 0});
    parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .retain(|actor| actor["id"] == "player" || actor["id"] == SCAVENGER);
    parts
}

fn profile() -> Value {
    json!({
        "searches_corpses": true,
        "collects_ground_items": true,
        "collects_gold": true,
        "equips_items": true,
        "uses_healing_balm": true,
        "search_radius": 6,
        "balm_below_hp_percent": 50,
        "balm_chance_numerator": 1,
        "balm_chance_denominator": 4
    })
}

fn engine_with_profile(profile: Value) -> tme_rules::Engine {
    parts_with_profile(profile)
        .engine(7)
        .expect("scavenging fixture starts")
}

fn actor_index(engine: &tme_rules::Engine, actor_id: &str) -> usize {
    engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == actor_id)
        .unwrap_or_else(|| panic!("missing actor {actor_id}"))
}

fn actor_location(engine: &tme_rules::Engine, actor_id: &str) -> tme_rules::WorldPosition {
    engine.world().actors[actor_index(engine, actor_id)]
        .location
        .clone()
}

fn wait(engine: &mut tme_rules::Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("player wait drives one automatic opportunity")
        .events
}

fn decision<'a>(events: &'a [Event], actor_id: &str) -> &'a AutomaticActorDecisionV1 {
    events
        .iter()
        .find_map(|event| match event {
            Event::AutomaticActorDecision {
                actor_id: candidate,
                decision,
                ..
            } if candidate == actor_id => Some(decision),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing automatic decision for {actor_id}: {events:#?}"))
}

fn clone_item(
    engine: &mut tme_rules::Engine,
    source: &str,
    instance_id: &str,
    binding: ItemBindingState,
) {
    let mut item = engine.world().item_instances[source].clone();
    item.binding = binding;
    assert!(
        engine
            .world_mut()
            .item_instances
            .insert(instance_id.to_string(), item)
            .is_none()
    );
}

fn carry_item(
    engine: &mut tme_rules::Engine,
    actor_id: &str,
    instance_id: &str,
    position: CarriedPosition,
) {
    let index = actor_index(engine, actor_id);
    assert!(
        engine.world_mut().actors[index]
            .carried
            .items
            .insert(position, instance_id.to_string())
            .is_none()
    );
}

fn rng_state(engine: &tme_rules::Engine) -> String {
    let checkpoint = engine.export_checkpoint().expect("checkpoint");
    let value: Value = serde_json::from_slice(checkpoint.as_bytes()).expect("checkpoint JSON");
    value["rng_state"]
        .as_str()
        .expect("decimal RNG state")
        .to_string()
}

#[test]
fn scavenger_searches_before_collecting_preserves_claim_and_uses_shared_tied_fumble() {
    let mut engine = engine_with_profile(profile());
    let location = actor_location(&engine, SCAVENGER);
    let corpse_id = CorpseId::parse("corpse:900").unwrap();
    let owner = CharacterId::new("character:absent:owner");
    let claim = LootClaim {
        owner: LootOwnerId::Character(CharacterId::new("character:claim:owner")),
        basis: LootClaimBasis::KillingBlow,
    };
    clone_item(
        &mut engine,
        "weathered_staff",
        "bound_staff",
        ItemBindingState::Bound {
            character_id: owner.clone(),
        },
    );
    engine.world_mut().corpses.insert(
        corpse_id.clone(),
        CorpseState {
            id: corpse_id.clone(),
            origin_actor_id: ActorId::from("fallen"),
            origin_character_id: None,
            origin_kind: ActorKind::Monster,
            origin_name: "Fallen".to_string(),
            location: location.clone(),
            created_at: LogicalTime::FIRST,
            sequence: 900,
            searched: false,
            loot_claim: Some(claim.clone()),
            contents: BTreeMap::from([(CarriedPosition::RightHand, "bound_staff".to_string())]),
            gold: 7,
        },
    );
    engine.world_mut().next_corpse_sequence = 900;

    let searched = wait(&mut engine);
    assert!(matches!(
        decision(&searched, SCAVENGER),
        AutomaticActorDecisionV1::SearchCorpse { corpse_id: found } if found == &corpse_id
    ));
    assert!(!searched.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            decision: AutomaticActorDecisionV1::CollectItem { .. },
            ..
        }
    )));
    assert!(engine.world().corpses[&corpse_id].searched);
    let released = engine
        .world()
        .ground_items
        .iter()
        .find(|item| item.item_instance_id == "bound_staff")
        .expect("searched corpse releases the exact item");
    assert_eq!(released.loot_claim.as_ref(), Some(&claim));
    assert_eq!(engine.world().ground_gold.len(), 1);
    assert_eq!(
        engine
            .world()
            .ground_gold
            .values()
            .next()
            .and_then(|pile| pile.loot_claim.as_ref()),
        Some(&claim)
    );

    let collected = wait(&mut engine);
    assert!(matches!(
        decision(&collected, SCAVENGER),
        AutomaticActorDecisionV1::CollectItem {
            item_instance_id,
            destination: CarriedPosition::RightHand,
        } if item_instance_id == "bound_staff"
    ));
    let scavenger = &engine.world().actors[actor_index(&engine, SCAVENGER)];
    assert_eq!(
        scavenger.carried.items.get(&CarriedPosition::RightHand),
        Some(&"bound_staff".to_string())
    );
    assert!(matches!(
        &engine.world().item_instances["bound_staff"].binding,
        ItemBindingState::Bound { character_id } if character_id == &owner
    ));
    let observed = engine
        .observer_projection(&ActorId::from("player"), &collected)
        .expect("observer projection");
    assert!(
        !serde_json::to_string(&observed)
            .unwrap()
            .contains(owner.as_str())
    );

    let scavenger_index = actor_index(&engine, SCAVENGER);
    engine.world_mut().actors[scavenger_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Chaotic,
    };
    engine.world_mut().actors[scavenger_index].social.behavior = SocialBehavior::AlignmentCreature;
    let fumbled = wait(&mut engine);
    assert!(
        fumbled.iter().any(|event| matches!(
            event,
            Event::WeaponFumbled {
                attacker_id,
                reason: WeaponFumbleReason::TiedToOtherCharacter,
                ..
            } if attacker_id == SCAVENGER
        )),
        "{fumbled:#?}"
    );
    assert!(fumbled.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            actor_id,
            item_instance_id,
            reason: ItemRelocationReason::WeaponFumble,
            ..
        } if actor_id == SCAVENGER && item_instance_id == "bound_staff"
    )));
}

#[test]
fn balm_uses_stable_lowest_instance_before_scavenging_and_draws_once_per_opportunity() {
    let mut guaranteed = profile();
    guaranteed["balm_chance_numerator"] = json!(1);
    guaranteed["balm_chance_denominator"] = json!(1);
    let mut engine = engine_with_profile(guaranteed);
    clone_item(
        &mut engine,
        "trade_charm",
        "z_balm",
        ItemBindingState::Unrestricted,
    );
    clone_item(
        &mut engine,
        "trade_charm",
        "a_balm",
        ItemBindingState::Unrestricted,
    );
    engine
        .world_mut()
        .item_instances
        .get_mut("z_balm")
        .unwrap()
        .definition_id = "healing_balm".to_string();
    engine
        .world_mut()
        .item_instances
        .get_mut("a_balm")
        .unwrap()
        .definition_id = "healing_balm".to_string();
    carry_item(&mut engine, SCAVENGER, "a_balm", CarriedPosition::SackItem2);
    carry_item(&mut engine, SCAVENGER, "z_balm", CarriedPosition::SackItem3);
    clone_item(
        &mut engine,
        "trade_charm",
        "ground_charm",
        ItemBindingState::Unrestricted,
    );
    let ground_location = actor_location(&engine, SCAVENGER);
    engine.world_mut().ground_items.push(GroundItem {
        item_instance_id: "ground_charm".to_string(),
        location: ground_location,
        loot_claim: None,
    });
    let index = actor_index(&engine, SCAVENGER);
    engine.world_mut().actors[index].hp = 4;

    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, SCAVENGER),
        AutomaticActorDecisionV1::DrinkBalm { item_instance_id } if item_instance_id == "a_balm"
    ));
    assert!(!engine.world().item_instances.contains_key("a_balm"));
    assert!(engine.world().item_instances.contains_key("z_balm"));
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "ground_charm")
    );

    let mut failing_profile = profile();
    failing_profile["balm_chance_numerator"] = json!(0);
    let mut one_balm = engine_with_profile(failing_profile.clone());
    clone_item(
        &mut one_balm,
        "trade_charm",
        "only_balm",
        ItemBindingState::Unrestricted,
    );
    one_balm
        .world_mut()
        .item_instances
        .get_mut("only_balm")
        .unwrap()
        .definition_id = "healing_balm".to_string();
    carry_item(
        &mut one_balm,
        SCAVENGER,
        "only_balm",
        CarriedPosition::SackItem2,
    );
    let index = actor_index(&one_balm, SCAVENGER);
    one_balm.world_mut().actors[index].hp = 4;
    let before = rng_state(&one_balm);
    let failed = wait(&mut one_balm);
    assert!(!failed.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            decision: AutomaticActorDecisionV1::DrinkBalm { .. },
            ..
        }
    )));
    let after_one = rng_state(&one_balm);
    assert_ne!(after_one, before, "a failed balm attempt consumes one draw");

    let mut two_balms = engine_with_profile(failing_profile);
    for (id, position) in [
        ("a_balm", CarriedPosition::SackItem2),
        ("z_balm", CarriedPosition::SackItem3),
    ] {
        clone_item(
            &mut two_balms,
            "trade_charm",
            id,
            ItemBindingState::Unrestricted,
        );
        two_balms
            .world_mut()
            .item_instances
            .get_mut(id)
            .unwrap()
            .definition_id = "healing_balm".to_string();
        carry_item(&mut two_balms, SCAVENGER, id, position);
    }
    let index = actor_index(&two_balms, SCAVENGER);
    two_balms.world_mut().actors[index].hp = 4;
    wait(&mut two_balms);
    assert_eq!(
        rng_state(&two_balms),
        after_one,
        "the balm chance is rolled once per opportunity, not once per item"
    );

    let mut no_balm = engine_with_profile(profile());
    let before_no_balm = rng_state(&no_balm);
    wait(&mut no_balm);
    assert_eq!(rng_state(&no_balm), before_no_balm);
}

#[test]
fn item_and_gold_collection_survive_checkpoint_and_death_rereleases_exact_instance() {
    let mut engine = engine_with_profile(profile());
    let location = actor_location(&engine, SCAVENGER);
    let owner = CharacterId::new("character:absent:owner");
    clone_item(
        &mut engine,
        "trade_charm",
        "found_charm",
        ItemBindingState::Bound {
            character_id: owner.clone(),
        },
    );
    engine.world_mut().ground_items.push(GroundItem {
        item_instance_id: "found_charm".to_string(),
        location: location.clone(),
        loot_claim: None,
    });
    let gold_id = GoldPileId::parse("gold:800").unwrap();
    engine.world_mut().ground_gold.insert(
        gold_id.clone(),
        GroundGoldPile {
            id: gold_id.clone(),
            amount: 9,
            location,
            loot_claim: None,
        },
    );
    engine.world_mut().next_gold_sequence = 800;

    let item_events = wait(&mut engine);
    assert!(matches!(
        decision(&item_events, SCAVENGER),
        AutomaticActorDecisionV1::CollectItem {
            item_instance_id,
            destination: CarriedPosition::LeftHand,
        } if item_instance_id == "found_charm"
    ));
    let gold_events = wait(&mut engine);
    assert!(matches!(
        decision(&gold_events, SCAVENGER),
        AutomaticActorDecisionV1::CollectGold {
            gold_pile_id,
            amount: 9,
        } if gold_pile_id == &gold_id
    ));
    let scavenger_index = actor_index(&engine, SCAVENGER);
    assert_eq!(
        engine.world().actors[scavenger_index]
            .carried
            .gold
            .amount(CarriedGoldPosition::Sack),
        29
    );

    let checkpoint = engine.export_checkpoint().expect("checkpoint");
    let mut restored =
        tme_rules::Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint)
            .expect("scavenged state hydrates");
    assert_eq!(restored.world(), engine.world());
    assert!(matches!(
        &restored.world().item_instances["found_charm"].binding,
        ItemBindingState::Bound { character_id } if character_id == &owner
    ));

    let scavenger_index = actor_index(&restored, SCAVENGER);
    restored.world_mut().actors[scavenger_index].hp = 1;
    restored.world_mut().actors[scavenger_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Chaotic,
    };
    restored.world_mut().actors[scavenger_index].social.behavior =
        SocialBehavior::AlignmentCreature;
    let player_index = actor_index(&restored, "player");
    restored.world_mut().actors[player_index].stats.attack = 100;
    let defeated = restored
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: ActorId::from(SCAVENGER),
            },
        )
        .expect("open hostile scavenger can be defeated")
        .events;
    assert!(defeated.iter().any(|event| matches!(
        event,
        Event::ActorDefeated { actor_id, .. } if actor_id == SCAVENGER
    )));
    assert!(
        restored
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "found_charm")
    );
    assert!(matches!(
        &restored.world().item_instances["found_charm"].binding,
        ItemBindingState::Bound { character_id } if character_id == &owner
    ));
}

#[test]
fn scavenger_leaves_item_untouched_without_a_compatible_destination() {
    let mut engine = engine_with_profile(profile());
    let source = engine.world().item_instances["trade_charm"].clone();
    let positions = CarriedPosition::ALL
        .into_iter()
        .filter(|position| {
            matches!(
                position,
                CarriedPosition::LeftHand | CarriedPosition::RightHand
            ) || position.is_sack_item()
        })
        .collect::<Vec<_>>();
    for (sequence, position) in positions.into_iter().enumerate() {
        let scavenger_index = actor_index(&engine, SCAVENGER);
        if engine.world().actors[scavenger_index]
            .carried
            .items
            .contains_key(&position)
        {
            continue;
        }
        let id = format!("filler_{sequence:02}");
        engine
            .world_mut()
            .item_instances
            .insert(id.clone(), source.clone());
        engine.world_mut().actors[scavenger_index]
            .carried
            .items
            .insert(position, id);
    }
    clone_item(
        &mut engine,
        "trade_charm",
        "blocked_charm",
        ItemBindingState::Unrestricted,
    );
    let ground_location = actor_location(&engine, SCAVENGER);
    engine.world_mut().ground_items.push(GroundItem {
        item_instance_id: "blocked_charm".to_string(),
        location: ground_location,
        loot_claim: None,
    });
    let before = engine.world().clone();
    let events = wait(&mut engine);
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            item_instance_id,
            ..
        } if item_instance_id == "blocked_charm"
    )));
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "blocked_charm")
    );
    assert_eq!(
        engine.world().item_instances["blocked_charm"],
        before.item_instances["blocked_charm"]
    );
}

#[test]
fn scavenging_moves_toward_stable_nearest_target_but_never_preempts_combat() {
    let mut engine = engine_with_profile(profile());
    let scavenger_index = actor_index(&engine, SCAVENGER);
    engine.world_mut().actors[scavenger_index].location.position = Coord { x: 1, y: 1 };
    engine.world_mut().actors[scavenger_index]
        .home_location
        .position = Coord { x: 1, y: 1 };
    for (id, x) in [("z_near_charm", 2), ("a_near_charm", 2), ("far_charm", 3)] {
        clone_item(
            &mut engine,
            "trade_charm",
            id,
            ItemBindingState::Unrestricted,
        );
        let mut location = actor_location(&engine, SCAVENGER);
        location.position = Coord { x, y: 1 };
        engine.world_mut().ground_items.push(GroundItem {
            item_instance_id: id.to_string(),
            location,
            loot_claim: None,
        });
    }
    let moved = wait(&mut engine);
    assert!(matches!(
        decision(&moved, SCAVENGER),
        AutomaticActorDecisionV1::Move {
            purpose: AutomaticMovementPurposeV1::Scavenge,
            ..
        }
    ));
    assert_eq!(restored_position(&engine, SCAVENGER), Coord { x: 2, y: 1 });
    let collected = wait(&mut engine);
    assert!(matches!(
        decision(&collected, SCAVENGER),
        AutomaticActorDecisionV1::CollectItem {
            item_instance_id,
            ..
        } if item_instance_id == "a_near_charm"
    ));

    let scavenger_index = actor_index(&engine, SCAVENGER);
    engine.world_mut().actors[scavenger_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Chaotic,
    };
    engine.world_mut().actors[scavenger_index].social.behavior = SocialBehavior::AlignmentCreature;
    let player_index = actor_index(&engine, "player");
    engine.world_mut().actors[player_index].location =
        engine.world().actors[scavenger_index].location.clone();
    let combat = wait(&mut engine);
    assert!(matches!(
        decision(&combat, SCAVENGER),
        AutomaticActorDecisionV1::PhysicalAttack { .. }
    ));
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "z_near_charm")
    );
}

#[test]
fn scavenging_respects_search_radius_and_home_leash() {
    let mut narrow = profile();
    narrow["search_radius"] = json!(1);
    let mut outside_radius = engine_with_profile(narrow);
    let scavenger_index = actor_index(&outside_radius, SCAVENGER);
    outside_radius.world_mut().actors[scavenger_index]
        .location
        .position = Coord { x: 1, y: 1 };
    outside_radius.world_mut().actors[scavenger_index]
        .home_location
        .position = Coord { x: 1, y: 1 };
    clone_item(
        &mut outside_radius,
        "trade_charm",
        "outside_radius",
        ItemBindingState::Unrestricted,
    );
    let mut location = actor_location(&outside_radius, SCAVENGER);
    location.position = Coord { x: 3, y: 1 };
    outside_radius.world_mut().ground_items.push(GroundItem {
        item_instance_id: "outside_radius".to_string(),
        location,
        loot_claim: None,
    });
    assert!(matches!(
        decision(&wait(&mut outside_radius), SCAVENGER),
        AutomaticActorDecisionV1::Wait { .. }
    ));

    let mut outside_leash = engine_with_profile(profile());
    let scavenger_index = actor_index(&outside_leash, SCAVENGER);
    outside_leash.world_mut().actors[scavenger_index]
        .location
        .position = Coord { x: 1, y: 1 };
    outside_leash.world_mut().actors[scavenger_index]
        .home_location
        .position = Coord { x: 1, y: 1 };
    outside_leash.world_mut().actors[scavenger_index]
        .ai
        .as_mut()
        .expect("automatic actor")
        .leash_range = 1;
    clone_item(
        &mut outside_leash,
        "trade_charm",
        "outside_leash",
        ItemBindingState::Unrestricted,
    );
    let mut location = actor_location(&outside_leash, SCAVENGER);
    location.position = Coord { x: 3, y: 1 };
    outside_leash.world_mut().ground_items.push(GroundItem {
        item_instance_id: "outside_leash".to_string(),
        location,
        loot_claim: None,
    });
    assert!(matches!(
        decision(&wait(&mut outside_leash), SCAVENGER),
        AutomaticActorDecisionV1::Wait { .. }
    ));
}

fn restored_position(engine: &tme_rules::Engine, actor_id: &str) -> Coord {
    engine.world().actors[actor_index(engine, actor_id)]
        .location
        .position
}
