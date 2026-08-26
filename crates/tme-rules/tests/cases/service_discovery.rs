use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    ActionBlockedReasonV1, ActionOptionV1, Engine, PlayerActionContextV1, PlayerActionContextV2,
    PlayerIntentPayloadV1, ServiceCapabilityViewV1, ServiceViewV1,
};

fn content_parts(name: &str) -> ContentParts {
    let profile = match name {
        "magic_profession_gallery" => "profile/magic_profession_gallery",
        "gold_training" => "profile/gold_training",
        "knight_promotion" => "profile/knight_promotion",
        _ => panic!("unknown fixture"),
    };
    ContentParts::tracked(name, profile)
}

fn engine_from_parts(parts: ContentParts) -> Engine {
    parts.engine(7).expect("current content graph starts")
}

fn controlled_actor_id(engine: &Engine) -> tme_rules::ActorId {
    engine
        .world()
        .controlled_actors()
        .next()
        .expect("fixture player")
        .id
        .clone()
}

fn service_actions_in_flat_order(context: &PlayerActionContextV2) -> Vec<ActionOptionV1> {
    let mut actions = Vec::new();
    for service in &context.services_here {
        for capability in &service.capabilities {
            match capability {
                ServiceCapabilityViewV1::SkillTraining {
                    actions: grouped, ..
                }
                | ServiceCapabilityViewV1::SkillCritique {
                    actions: grouped, ..
                } => actions.extend(grouped.iter().cloned()),
                ServiceCapabilityViewV1::SpellTeaching { .. }
                | ServiceCapabilityViewV1::ClassPromotion { .. }
                | ServiceCapabilityViewV1::ServiceTransaction { .. }
                | ServiceCapabilityViewV1::Merchant { .. }
                | ServiceCapabilityViewV1::ItemService { .. }
                | ServiceCapabilityViewV1::Restoration { .. }
                | ServiceCapabilityViewV1::Bank { .. }
                | ServiceCapabilityViewV1::Locker { .. } => {}
            }
        }
    }
    for service in &context.services_here {
        for capability in &service.capabilities {
            if let ServiceCapabilityViewV1::ClassPromotion {
                actions: grouped, ..
            } = capability
            {
                actions.extend(grouped.iter().cloned());
            }
        }
    }
    for service in &context.services_here {
        for capability in &service.capabilities {
            if let ServiceCapabilityViewV1::SpellTeaching {
                actions: grouped, ..
            } = capability
            {
                actions.extend(grouped.iter().cloned());
            }
        }
    }
    actions
}

fn assert_flat_service_tail(engine: &Engine, context: &PlayerActionContextV2) {
    let grouped = service_actions_in_flat_order(context);
    assert!(!grouped.is_empty(), "fixture must project service actions");
    let flat = engine
        .actor_action_options(&controlled_actor_id(engine))
        .expect("flat options");
    assert_eq!(&flat[flat.len() - grouped.len()..], grouped.as_slice());
}

#[test]
fn service_capability_variants_have_exact_strict_json() {
    let variants = [
        json!({
            "kind": "skill_training",
            "capability_id": "training",
            "offered_track_ids": ["sword"],
            "selected_track_id": null,
            "actions": []
        }),
        json!({
            "kind": "skill_critique",
            "capability_id": "critique",
            "actions": []
        }),
        json!({
            "kind": "spell_teaching",
            "capability_id": "spell_teaching",
            "spell_ids": ["spark"],
            "actions": []
        }),
        json!({
            "kind": "class_promotion",
            "capability_id": "class_promotion",
            "target_class_id": "knight",
            "actions": []
        }),
        json!({
            "kind": "bank",
            "capability_id": "bank",
            "bank_id": "shared_bank",
            "balance_gold": 125,
            "transaction_cap_gold": 200,
            "deposit_actions": [],
            "withdrawal_actions": []
        }),
        json!({
            "kind": "locker",
            "capability_id": "locker",
            "vault_id": "shared_vault",
            "capacity": 48,
            "item_count": 0,
            "items": [],
            "deposit_actions": [],
            "withdrawal_actions": []
        }),
    ];

    for value in &variants {
        let parsed: ServiceCapabilityViewV1 =
            serde_json::from_value(value.clone()).expect("current variant parses");
        assert_eq!(
            serde_json::to_value(parsed).expect("variant serializes"),
            value.clone()
        );
    }

    let mut missing_selected = variants[0].clone();
    missing_selected
        .as_object_mut()
        .expect("training object")
        .remove("selected_track_id");
    assert!(serde_json::from_value::<ServiceCapabilityViewV1>(missing_selected).is_err());

    let mut selected = variants[0].clone();
    selected["selected_track_id"] = json!("sword");
    serde_json::from_value::<ServiceCapabilityViewV1>(selected)
        .expect("required nullable string accepts a string");

    for variant in variants {
        let mut unknown = variant;
        unknown["private_fact"] = json!(1);
        assert!(serde_json::from_value::<ServiceCapabilityViewV1>(unknown).is_err());
    }
}

#[test]
fn action_context_26_requires_services_here_in_both_current_views() {
    let engine = engine_from_parts(content_parts("magic_profession_gallery"));
    let mut v1 = serde_json::to_value(
        engine
            .actor_action_context(&controlled_actor_id(&engine))
            .expect("v1 context"),
    )
    .expect("v1 serializes");
    let mut v2 = serde_json::to_value(
        engine
            .actor_observed_action_context(&controlled_actor_id(&engine))
            .expect("v2 context"),
    )
    .expect("v2 serializes");
    assert_eq!(v1["contract_version"], 31);
    assert_eq!(v2["contract_version"], 31);
    v1.as_object_mut().unwrap().remove("services_here");
    v2.as_object_mut().unwrap().remove("services_here");
    assert!(serde_json::from_value::<PlayerActionContextV1>(v1).is_err());
    assert!(serde_json::from_value::<PlayerActionContextV2>(v2).is_err());

    let mut null_v1 = serde_json::to_value(
        engine
            .actor_action_context(&controlled_actor_id(&engine))
            .expect("v1 context"),
    )
    .expect("v1 serializes");
    let mut null_v2 = serde_json::to_value(
        engine
            .actor_observed_action_context(&controlled_actor_id(&engine))
            .expect("v2 context"),
    )
    .expect("v2 serializes");
    null_v1["services_here"] = Value::Null;
    null_v2["services_here"] = Value::Null;
    assert!(serde_json::from_value::<PlayerActionContextV1>(null_v1).is_err());
    assert!(serde_json::from_value::<PlayerActionContextV2>(null_v2).is_err());
}

#[test]
fn serialized_discovery_excludes_authored_and_private_service_facts() {
    for fixture in ["magic_profession_gallery", "knight_promotion"] {
        let engine = engine_from_parts(content_parts(fixture));
        let services = engine
            .actor_observed_action_context(&controlled_actor_id(&engine))
            .expect("context")
            .services_here;
        let encoded = serde_json::to_value(&services).expect("services serialize");
        let decoded: Vec<ServiceViewV1> =
            serde_json::from_value(encoded.clone()).expect("services round trip");
        assert_eq!(decoded, services);
        let wire = encoded.to_string();
        for forbidden in [
            "eligible_class_ids",
            "minimum_category_level",
            "maximum_category_level",
            "offers",
            "training_capability_id",
            "from_class_id",
            "minimum_level",
            "required_karma_points",
            "granted_item_instance_id",
            "granted_item_definition_id",
            "granted_spell_ids",
            "gold_cost",
            "price",
            "provenance",
            "research_boundary",
        ] {
            assert!(!wire.contains(forbidden), "wire leaked {forbidden}: {wire}");
        }
    }
}

#[test]
fn typed_discovery_is_deterministic_read_only_and_reuses_flat_actions() {
    let mut value = content_parts("magic_profession_gallery");
    value.skill_catalog_mut().expect("skill catalog")["tracks"]
        .as_array_mut()
        .expect("track list")
        .push(json!({
            "id": "mace",
            "display": "Mace",
            "kind": "weapon",
            "ladder_id": "magic_measure"
        }));
    value.skill_catalog_mut().expect("skill catalog")["tracks"]
        .as_array_mut()
        .expect("track list")
        .push(json!({
            "id": "axe",
            "display": "Axe",
            "kind": "weapon",
            "ladder_id": "magic_measure",
            "eligible_class_ids": ["wizard"]
        }));
    let engine = engine_from_parts(value);
    let before_world = engine.world().clone();
    let v1 = engine
        .actor_action_context(&controlled_actor_id(&engine))
        .expect("v1 context");
    let v2 = engine
        .actor_observed_action_context(&controlled_actor_id(&engine))
        .expect("v2 context");
    let flat = engine
        .actor_action_options(&controlled_actor_id(&engine))
        .expect("flat options");

    assert_eq!(v1.services_here, v2.services_here);
    assert_eq!(v2.services_here.len(), 1);
    let service = &v2.services_here[0];
    assert_eq!(service.service_id, "thief_trainer");
    assert_eq!(service.position, v2.position);
    assert_eq!(service.capabilities.len(), 3);

    match &service.capabilities[0] {
        ServiceCapabilityViewV1::SkillTraining {
            capability_id,
            offered_track_ids,
            selected_track_id,
            actions,
        } => {
            assert_eq!(capability_id, "training");
            assert_eq!(offered_track_ids.len(), 1);
            assert_eq!(offered_track_ids[0], "thief_magic");
            assert_eq!(selected_track_id, &None);
            assert!(
                actions.is_empty(),
                "the missing Spell Book leaves no focus action"
            );
        }
        other => panic!("expected training first, got {other:?}"),
    }
    match &service.capabilities[1] {
        ServiceCapabilityViewV1::SkillCritique {
            capability_id,
            actions,
        } => {
            assert_eq!(capability_id, "critique");
            let tracks = actions
                .iter()
                .map(
                    |action| match &action.command.as_ref().expect("command").intent {
                        PlayerIntentPayloadV1::Critique { track_id, .. } => track_id.as_str(),
                        other => panic!("unexpected critique action {other:?}"),
                    },
                )
                .collect::<Vec<_>>();
            assert_eq!(tracks.len(), 2);
            assert_eq!(tracks[0], "mace");
            assert_eq!(tracks[1], "thief_magic");
            assert!(!tracks.contains(&"axe"));
        }
        other => panic!("expected critique second, got {other:?}"),
    }
    match &service.capabilities[2] {
        ServiceCapabilityViewV1::SpellTeaching {
            capability_id,
            spell_ids,
            actions,
        } => {
            assert_eq!(capability_id, "spell_teaching");
            assert_eq!(spell_ids.len(), 1);
            assert_eq!(spell_ids[0], "shadow_sting");
            assert_eq!(actions.len(), 1);
            assert!(!actions[0].enabled);
            assert_eq!(
                actions[0].blocked_reason,
                Some(ActionBlockedReasonV1::SpellBookRequired)
            );
        }
        other => panic!("expected teaching last, got {other:?}"),
    }

    for action in service
        .capabilities
        .iter()
        .flat_map(|capability| match capability {
            ServiceCapabilityViewV1::SkillTraining { actions, .. }
            | ServiceCapabilityViewV1::SkillCritique { actions, .. }
            | ServiceCapabilityViewV1::SpellTeaching { actions, .. }
            | ServiceCapabilityViewV1::ClassPromotion { actions, .. } => actions.as_slice(),
            ServiceCapabilityViewV1::ServiceTransaction { .. }
            | ServiceCapabilityViewV1::Merchant { .. }
            | ServiceCapabilityViewV1::ItemService { .. }
            | ServiceCapabilityViewV1::Restoration { .. }
            | ServiceCapabilityViewV1::Bank { .. }
            | ServiceCapabilityViewV1::Locker { .. } => &[] as &[ActionOptionV1],
        })
    {
        let status = engine
            .validate_actor_command(action.command.as_ref().expect("service command"))
            .expect("command validates structurally");
        assert_eq!(action.enabled, status.accepted);
        assert_eq!(action.blocked_reason, status.blocked_reason);
    }

    assert_flat_service_tail(&engine, &v2);
    let serialized_services =
        serde_json::to_string(&v2.services_here).expect("discovery serializes");
    let serialized_options = serde_json::to_string(&flat).expect("options serialize");
    for _ in 0..3 {
        let repeated_context = engine
            .actor_observed_action_context(&controlled_actor_id(&engine))
            .expect("repeat context");
        let repeated_options = engine
            .actor_action_options(&controlled_actor_id(&engine))
            .expect("repeat options");
        assert_eq!(repeated_context, v2);
        assert_eq!(repeated_options, flat);
        assert_eq!(
            serde_json::to_string(&repeated_context.services_here).expect("serialize repeat"),
            serialized_services
        );
        assert_eq!(
            serde_json::to_string(&repeated_options).expect("serialize repeat"),
            serialized_options
        );
    }
    assert_eq!(engine.world(), &before_world);
}

#[test]
fn discovery_is_exact_coordinate_only_and_preserves_authored_provider_order() {
    let mut other_coordinate = content_parts("magic_profession_gallery");
    other_coordinate.service_instances_mut()[0]["location"]["position"]["x"] = json!(2);
    let engine = engine_from_parts(other_coordinate);
    assert!(
        engine
            .actor_observed_action_context(&controlled_actor_id(&engine))
            .expect("context")
            .services_here
            .is_empty()
    );

    let mut other_room = content_parts("magic_profession_gallery");
    let room = other_room.template_levels_source_mut()["room_0"].clone();
    other_room.template_levels_source_mut()["other"] = room;
    other_room.service_instances_mut()[0]["location"]["level"] = json!("other");
    let engine = engine_from_parts(other_room);
    assert!(
        engine
            .actor_observed_action_context(&controlled_actor_id(&engine))
            .expect("context")
            .services_here
            .is_empty()
    );

    let mut colocated = content_parts("magic_profession_gallery");
    let mut second = colocated.selected_mut("service_definitions", 0).clone();
    second["id"] = json!("second_trainer");
    second["name"] = json!("Second Trainer");
    second["capabilities"]
        .as_array_mut()
        .expect("capability list")
        .pop();
    colocated.push_selected("service_definitions", "service/second_trainer/test", second);
    let mut second_instance = colocated.service_instances_mut()[0].clone();
    second_instance["id"] = json!("second_trainer");
    second_instance["service_definition_id"] = json!("second_trainer");
    colocated
        .service_instances_mut()
        .as_array_mut()
        .expect("service instances")
        .push(second_instance);
    let engine = engine_from_parts(colocated);
    let ids = engine
        .actor_observed_action_context(&controlled_actor_id(&engine))
        .expect("context")
        .services_here
        .into_iter()
        .map(|service| service.service_id)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], "thief_trainer");
    assert_eq!(ids[1], "second_trainer");
}

#[test]
fn teaching_projection_preserves_authored_nonlexicographic_order() {
    let mut value = content_parts("magic_profession_gallery");
    let spell_index = (0..value.selected_len("spells"))
        .find(|index| value.selected_mut("spells", *index)["id"] == "shadow_sting")
        .expect("teachable spell");
    let mut second_spell = value.selected_mut("spells", spell_index).clone();
    second_spell["id"] = json!("amber_sting");
    second_spell["name"] = json!("Amber Sting");
    value.push_selected("spells", "spell/amber_sting/test", second_spell);
    value.selected_mut("service_definitions", 0)["capabilities"][2]["teachings"]
        .as_array_mut()
        .expect("teaching list")
        .push(json!({"spell_id": "amber_sting"}));

    let engine = engine_from_parts(value);
    let context = engine
        .actor_observed_action_context(&controlled_actor_id(&engine))
        .expect("teaching context");
    let teaching = context.services_here[0]
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            ServiceCapabilityViewV1::SpellTeaching {
                spell_ids, actions, ..
            } => Some((spell_ids, actions)),
            _ => None,
        })
        .expect("teaching capability");
    assert_eq!(teaching.0.len(), 2);
    assert_eq!(teaching.0[0], "shadow_sting");
    assert_eq!(teaching.0[1], "amber_sting");
    let action_spells = teaching
        .1
        .iter()
        .map(
            |action| match &action.command.as_ref().expect("command").intent {
                PlayerIntentPayloadV1::LearnSpell { spell_id } => spell_id.as_str(),
                other => panic!("unexpected teaching action {other:?}"),
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(action_spells.len(), 2);
    assert_eq!(action_spells[0], "shadow_sting");
    assert_eq!(action_spells[1], "amber_sting");
}

#[test]
fn training_and_promotion_discovery_expose_current_typed_commands() {
    let training_engine = engine_from_parts(content_parts("gold_training"));
    let training_context = training_engine
        .actor_observed_action_context(&controlled_actor_id(&training_engine))
        .expect("training context");
    let training = training_context.services_here.iter().find_map(|service| {
        service.capabilities.iter().find_map(|capability| {
            let ServiceCapabilityViewV1::SkillTraining {
                selected_track_id,
                actions,
                ..
            } = capability
            else {
                return None;
            };
            Some((service, selected_track_id, actions))
        })
    });
    let (service, selected_track_id, actions) = training.expect("training capability");
    assert!(selected_track_id.is_some());
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0].command.as_ref().expect("train command").intent,
        PlayerIntentPayloadV1::Train { service_id, .. } if service_id == &service.service_id
    ));
    assert_flat_service_tail(&training_engine, &training_context);

    let mut promotion_value = content_parts("knight_promotion");
    promotion_value.actors_mut()[0]["character"]["alignment_state"]["karma_points"] = json!(3);
    let promotion_engine = engine_from_parts(promotion_value);
    let promotion_context = promotion_engine
        .actor_observed_action_context(&controlled_actor_id(&promotion_engine))
        .expect("promotion context");
    let promotion = promotion_context
        .services_here
        .iter()
        .flat_map(|service| &service.capabilities)
        .find_map(|capability| match capability {
            ServiceCapabilityViewV1::ClassPromotion {
                target_class_id,
                actions,
                ..
            } => Some((target_class_id, actions)),
            _ => None,
        })
        .expect("promotion capability");
    assert_eq!(promotion.0, "knight");
    assert_eq!(promotion.1.len(), 1);
    assert!(!promotion.1[0].enabled);
    assert_eq!(
        promotion.1[0].blocked_reason,
        Some(ActionBlockedReasonV1::NotReady)
    );
    assert!(matches!(
        &promotion.1[0]
            .command
            .as_ref()
            .expect("promotion command")
            .intent,
        PlayerIntentPayloadV1::PromoteClass { target_class_id } if target_class_id == "knight"
    ));
    assert_flat_service_tail(&promotion_engine, &promotion_context);
}

#[test]
fn zero_gold_training_is_visible_but_has_no_invalid_command() {
    let mut value = content_parts("gold_training");
    value.actors_mut()[0]["carried"]["gold"]["sack"] = json!(0);
    let engine = engine_from_parts(value);
    let context = engine
        .actor_observed_action_context(&controlled_actor_id(&engine))
        .expect("zero-gold training context");
    let action = context
        .services_here
        .iter()
        .flat_map(|service| &service.capabilities)
        .find_map(|capability| match capability {
            ServiceCapabilityViewV1::SkillTraining { actions, .. } => actions.first(),
            _ => None,
        })
        .expect("visible training action");
    assert!(!action.enabled);
    assert!(action.blocked_reason.is_some());
    assert!(action.command.is_none());
}
