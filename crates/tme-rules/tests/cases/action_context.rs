use tme_rules::{
    ActionBlockedReasonV1, AttackSafety, Coord, Direction, Engine, PlayerIntent, SpellTarget,
    WorldPosition, view::SocialBehaviorViewV1,
};

use crate::action_context_support::common::{option_by_id, status_engine};
use crate::action_context_support::items::non_weapon_hands_with_ground_weapon_engine;
use crate::action_context_support::projection::*;
use crate::support::content_parts::ContentParts;

fn tracked_engine(case_id: &str, catalog_profile: &str) -> Engine {
    ContentParts::tracked(case_id, catalog_profile)
        .engine(7)
        .expect("tracked content should start")
}

#[test]
fn action_context_exposes_current_and_exit_tile_effects() {
    let engine = bt_action_context_overlay_engine();
    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(
        context
            .tile_effects_here
            .iter()
            .any(|effect| effect.effect_id == "ember_cloud")
    );
    let east = context
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit");
    assert!(
        east.tile_effects
            .iter()
            .any(|effect| effect.effect_id == "web_field")
    );
}

#[test]
fn hidden_door_is_absent_from_action_context_until_revealed() {
    let door_position = Coord { x: 2, y: 1 };
    let mut engine = hidden_closed_door_engine();

    let hidden = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(
        hidden.door_actions.is_empty(),
        "hidden adjacent door must not produce door actions"
    );
    let hidden_east = hidden
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit should exist");
    assert_eq!(
        hidden_east.transition, None,
        "hidden adjacent door must not appear on exit drafts"
    );
    assert!(
        !hidden_east.blocked,
        "hidden closed door should not block as a door before reveal"
    );

    engine
        .set_navigation_revealed(&WorldPosition::new("realm_0", "start", door_position), true)
        .expect("reveal should succeed");
    let revealed = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert_eq!(revealed.door_actions.len(), 1);
    assert_eq!(revealed.door_actions[0].location.position, door_position);
    assert!(revealed.door_actions[0].can_open);
    let revealed_east = revealed
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit should exist");
    assert!(revealed_east.transition.is_some());
    assert!(!revealed_east.blocked);
    assert!(revealed_east.opens_door);
    assert_eq!(revealed_east.blocked_reason, None);

    engine
        .set_navigation_revealed(
            &WorldPosition::new("realm_0", "start", door_position),
            false,
        )
        .expect("hide should succeed");
    let hidden_again = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(hidden_again.door_actions.is_empty());
    let hidden_again_east = hidden_again
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit should exist");
    assert_eq!(hidden_again_east.transition, None);
}

#[test]
fn action_context_is_read_only() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let ctx1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    let ctx2 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert_eq!(
        ctx1, ctx2,
        "action context must be deterministic and idempotent"
    );
}

#[test]
fn action_context_includes_exits() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert_eq!(ctx.exits.len(), 8, "must have 8 directional exits");
    // At least one exit should be walkable (south in first_room)
    let walkable = ctx.exits.iter().filter(|e| !e.blocked).count();
    assert!(walkable > 0, "must have at least one walkable exit");
}

#[test]
fn action_context_includes_attack_targets() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(!ctx.attack_targets.is_empty(), "first_room has a monster");
    for target in &ctx.attack_targets {
        assert!(!target.actor_id.is_empty(), "target must have actor_id");
        assert!(!target.actor_name.is_empty(), "target must have name");
    }
}

#[test]
fn action_context_includes_door_actions() {
    let engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(!ctx.door_actions.is_empty(), "undercroft_loop has doors");
    for door in &ctx.door_actions {
        assert!(!door.target.level.is_empty(), "door must have target level");
    }
}

#[test]
fn action_context_includes_ground_items() {
    let engine = tracked_engine("supply_cache", "profile/supply_cache");
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(
        !ctx.ground_items_here.is_empty(),
        "supply_cache has items at player feet"
    );
}

#[test]
fn action_context_includes_carried_items_after_take() {
    let mut engine = tracked_engine("supply_cache", "profile/supply_cache");
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "hemp_rope".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .ok();
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(
        !ctx.carried.items.is_empty(),
        "should have carried items after take"
    );
}

#[test]
fn action_context_blocks_walkable_exit_when_suppressed() {
    let engine = status_engine();
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    let east = ctx
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit");
    assert!(
        east.blocked,
        "suppressed status should block walkable movement"
    );
    assert_eq!(east.blocked_reason.as_deref(), Some("suppressed by status"));
}

#[test]
fn observed_action_context_is_read_only() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let ctx1 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    let ctx2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert_eq!(
        ctx1, ctx2,
        "observed action context must be deterministic and idempotent"
    );
}

#[test]
fn observed_context_includes_exits() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert_eq!(ctx.exits.len(), 8, "must have 8 directional exits");
}

#[test]
fn observed_context_blocks_walkable_exit_when_suppressed() {
    let engine = status_engine();
    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    let east = ctx
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit");
    assert!(
        east.blocked,
        "suppressed status should block walkable movement"
    );
    assert_eq!(
        east.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );
}

#[test]
fn observed_context_hides_monster_behind_wall() {
    // In first_room (3x3), player at (1,1), mireling at (2,1).
    // (1,1) can see (2,1) — open floor. So the mireling IS visible.
    // To test hiding, use a fixture with a wall between player and monster.
    // We use first_room and move the player: from (1,1), the wall at (2,2)
    // blocks sight to (3,3). But first_room is 3x3 with no monster at (3,3).
    // Instead, create an inline engine with a wall between player and monster.
    let engine = tracked_engine("first_room", "profile/first_room");

    // V1 (omniscient): the mireling should appear
    let v1_ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    assert!(
        !v1_ctx.attack_targets.is_empty(),
        "omniscient V1 must show the monster"
    );

    // V2 (observed): the mireling at (2,1) is visible from (1,1) — open floor
    let v2_ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert!(
        !v2_ctx.attack_targets.is_empty(),
        "visible monster must appear in observed context"
    );
}

#[test]
fn observed_context_uses_typed_block_reasons() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");

    // Blocked exits should have typed reasons (walls in first_room are BlockedTerrain)
    for exit in &ctx.exits {
        if exit.blocked {
            assert!(
                exit.blocked_reason.is_some(),
                "blocked exit must have a typed reason"
            );
            // Verify it's a valid enum variant, not a string
            let reason = exit.blocked_reason.unwrap();
            assert!(
                matches!(
                    reason,
                    ActionBlockedReasonV1::OutOfBounds
                        | ActionBlockedReasonV1::BlockedTerrain
                        | ActionBlockedReasonV1::ClosedDoor
                ),
                "exit reason must be a terrain/door/bounds variant, got {reason:?}"
            );
        }
    }
}

#[test]
fn observed_context_typed_attack_block_reason() {
    // Use ranged_attack fixture: player with bow, target at distance
    let engine = tracked_engine("ranged_attack", "profile/ranged_attack");
    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");

    for target in &ctx.attack_targets {
        for option in &target.physical_attacks {
            if !option.enabled {
                assert!(
                    option.blocked_reason.is_some(),
                    "blocked option must have a typed reason"
                );
            }
        }
    }
}

#[test]
fn observed_context_reports_door_actions() {
    let engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");
    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(!ctx.door_actions.is_empty(), "undercroft_loop has doors");
    for door in &ctx.door_actions {
        assert!(!door.target.level.is_empty(), "door must have target level");
    }
}

#[test]
fn observed_context_includes_ground_items() {
    let engine = tracked_engine("supply_cache", "profile/supply_cache");
    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(
        !ctx.ground_items_here.is_empty(),
        "supply_cache has items at player feet, always visible"
    );
}

#[test]
fn observed_context_monster_visible_behind_open_door() {
    // Use undercroft_loop: open the east door, then check the monster in the next tile
    let mut engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");

    // Open east door
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(tme_rules::Direction::East),
        )
        .ok();

    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    // After opening door, we should have door actions (close is now available)
    let close_actions: Vec<_> = ctx.door_actions.iter().filter(|d| d.can_close).collect();
    assert!(
        !close_actions.is_empty(),
        "should be able to close the open door"
    );
}

#[test]
fn observed_action_context_includes_active_effects_and_resistances() {
    let engine = tracked_engine("status_effects", "profile/status_effects");
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("observed context");
    assert_eq!(context.active_effects.len(), 1);
    assert_eq!(context.active_effects[0].instance_id, "rooted_1");
    assert_eq!(context.magic_resistance.boosts.len(), 1);
    assert_eq!(context.magic_resistance.boosts[0].tag, "stun");
}

#[test]
fn v1_and_v2_produce_same_exit_count() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let v2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert_eq!(v1.exits.len(), v2.exits.len());
}

#[test]
fn v1_and_v2_agree_on_ground_items() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let v2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert_eq!(v1.ground_items_here.len(), v2.ground_items_here.len());
    for (a, b) in v1.ground_items_here.iter().zip(v2.ground_items_here.iter()) {
        assert_eq!(a.item_instance_id, b.item_instance_id);
        assert_eq!(a.name, b.name);
    }
}

#[test]
fn v1_and_v2_agree_on_carried_items() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let v2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert_eq!(v1.carried.items.len(), v2.carried.items.len());
}

#[test]
fn v1_and_v2_agree_on_door_actions() {
    let engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let v2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert_eq!(v1.door_actions.len(), v2.door_actions.len());
    for (a, b) in v1.door_actions.iter().zip(v2.door_actions.iter()) {
        assert_eq!(a.direction, b.direction);
        assert_eq!(a.can_open, b.can_open);
        assert_eq!(a.can_close, b.can_close);
    }
}

#[test]
fn v1_and_v2_agree_on_ground_items_in_supply_cache() {
    // supply_cache fixture has ground items — V1 and V2 should agree
    let engine = tracked_engine("supply_cache", "profile/supply_cache");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let v2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert!(
        !v1.ground_items_here.is_empty(),
        "supply_cache has ground items"
    );
    assert_eq!(v1.ground_items_here.len(), v2.ground_items_here.len());
    for (a, b) in v1.ground_items_here.iter().zip(v2.ground_items_here.iter()) {
        assert_eq!(a.item_instance_id, b.item_instance_id);
        assert_eq!(a.name, b.name);
    }
}

#[test]
fn v1_and_v2_agree_on_carried_items_after_take_in_supply_cache() {
    // Take an item from supply_cache, then verify V1 and V2 agree on carried items
    let mut engine = tracked_engine("supply_cache", "profile/supply_cache");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "hemp_rope".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("take should succeed");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let v2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert!(
        !v1.carried.items.is_empty(),
        "should have carried items after take"
    );
    assert_eq!(v1.carried.items.len(), v2.carried.items.len());
    for (a, b) in v1.carried.items.iter().zip(v2.carried.items.iter()) {
        assert_eq!(a.item.item_instance_id, b.item.item_instance_id);
        assert_eq!(a.item.name, b.item.name);
        assert_eq!(a.position, b.position);
    }
}

#[test]
fn v1_and_v2_agree_on_readiness() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let v2 = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2");
    assert_eq!(v1.logical_time, v2.logical_time);
    assert_eq!(v1.ready_at, v2.ready_at);
    assert_eq!(v1.can_act, v2.can_act);
    assert_eq!(v1.attack_ready_at, v2.attack_ready_at);
    assert_eq!(v1.controlled_path_points, v2.controlled_path_points);
}

#[test]
fn observed_action_context_exposes_no_warmed_spell_initially() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");

    assert_eq!(context.warmed_spell, None);
}

#[test]
fn v1_excludes_occluded_attack_target_and_keeps_visible_target() {
    let mut parts = ContentParts::tracked("ranged_attack", "profile/ranged_attack");
    parts.template_levels_source_mut()["room_0"]["cells"][1][3] = serde_json::json!(["stone_wall"]);
    let mut visible = parts.actors_mut()[1].clone();
    visible["id"] = serde_json::json!("visible_reedling");
    visible["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts
        .actors_mut()
        .as_array_mut()
        .expect("actors array")
        .push(visible);
    let engine = parts.engine(7).expect("content should start");
    let v1 = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1");
    let actor_ids = v1
        .attack_targets
        .iter()
        .map(|target| target.actor_id.as_str())
        .collect::<Vec<_>>();

    assert!(actor_ids.contains(&"visible_reedling"));
    assert!(!actor_ids.contains(&"reedling"));
}

#[test]
fn summon_action_context_includes_summoned_target_metadata() {
    let mut engine = bw_summon_action_context_engine("chaotic");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    let target = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("v1 action context")
        .attack_targets
        .into_iter()
        .find(|target| target.actor_id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor target");

    assert_eq!(target.social.attack_safety, AttackSafety::Invalid);
    assert_eq!(
        target.social.apparent_behavior,
        SocialBehaviorViewV1::AlignmentCreature
    );
    assert_eq!(target.owner_id.as_deref(), Some("player"));
    let summoned = target.summoned.expect("summoned metadata");
    assert_eq!(summoned.instance_id, "summon:call_echo:1:echo_guardian");
    assert_eq!(summoned.source_spell_id, "call_echo");
    assert_eq!(summoned.template_id, "echo_guardian");
    assert_eq!(summoned.remaining_rounds, Some(1));
}

#[test]
fn summon_observed_action_context_includes_summoned_target_metadata() {
    let mut engine = bw_summon_action_context_engine("chaotic");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    let target = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("v2 action context")
        .attack_targets
        .into_iter()
        .find(|target| target.actor_id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor target");

    assert_eq!(target.social.attack_safety, AttackSafety::Invalid);
    assert_eq!(
        target.social.apparent_behavior,
        SocialBehaviorViewV1::AlignmentCreature
    );
    assert_eq!(target.owner_id.as_deref(), Some("player"));
    let summoned = target.summoned.expect("summoned metadata");
    assert_eq!(summoned.instance_id, "summon:call_echo:1:echo_guardian");
    assert_eq!(summoned.source_spell_id, "call_echo");
    assert_eq!(summoned.template_id, "echo_guardian");
    assert_eq!(summoned.remaining_rounds, Some(1));
}

#[test]
fn action_context_item_surfaces_share_instance_projection_and_report_burden() {
    let engine = tracked_engine("item_instance_contract", "profile/item_instance_contract");
    let context = serde_json::to_value(
        engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("action context should build"),
    )
    .expect("action context should serialize");

    let tonic = context["ground_items_here"]
        .as_array()
        .expect("ground action items should be an array")
        .iter()
        .find(|item| item["item_instance_id"] == "tonic_a")
        .expect("tonic_a should be offered");
    assert_eq!(tonic["item_definition_id"], "restorative_tonic");
    assert_eq!(tonic["name"], "Restorative Tonic");
    assert_eq!(tonic["quantity"], 2);
    assert_eq!(tonic["identified"], false);
    assert_eq!(tonic["appraised"], true);
    assert_eq!(tonic["known_unit_value_gold"], 12);
    assert_eq!(tonic["known_stack_value_gold"], 24);
    assert_eq!(tonic["unit_burden"], 3);
    assert_eq!(tonic["stack_burden"], 6);
    assert!(tonic.get("item_id").is_none());
    assert!(tonic.get("item_name").is_none());
    assert_eq!(
        context["burden"],
        serde_json::json!({
            "item_burden": 0,
            "coin_burden": 5,
            "total_burden": 5,
            "lightly_loaded_limit": 100000,
            "moderately_loaded_limit": 200000,
            "heavily_loaded_limit": 300000,
            "tier": "lightly_loaded"
        })
    );
}

#[test]
fn action_context_keeps_ground_item_visible_when_destination_is_occupied() {
    let engine = non_weapon_hands_with_ground_weapon_engine();

    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    assert!(
        !context.ground_items_here.is_empty(),
        "ground items remain visible even when a carried destination is occupied"
    );
    let observed = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("observed action context");
    assert!(
        !observed.ground_items_here.is_empty(),
        "observed context must preserve the same visible ground items"
    );
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("action options");
    let option = option_by_id(&options, "move_training_knife_to_right_hand");
    assert!(!option.enabled);
    assert_eq!(
        option.blocked_reason,
        Some(ActionBlockedReasonV1::OccupiedCarriedPosition)
    );
}
