use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActorId, ActorKind, ActorLifeState, CarriedGoldPosition, CharacterId, Coord, CorpseId,
    DamageLabel, DeathCause, Event, LogicalTime, ObservedEventV1, ObserverFeedbackCueV1,
    PhysicalAttackMode, PhysicalDamageKind, ResourceActivity, ResourceKind, ResurrectionMethod,
    TransactionCostReceiptV1, TransactionSourceV1, WeaponFumbleResult, WorldPosition, WoundState,
};

fn engine() -> tme_rules::Engine {
    let mut character_parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    let character = character_parts.actors_mut()[0]["character"].clone();
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actor_definition_by_actor_id_mut("player")["social"]["alignment_source"] =
        serde_json::json!({"kind": "character"});
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:observer:player");
    parts.actors_mut()[0]["character"] = character;
    parts.engine(7).expect("first-room engine")
}

fn here() -> WorldPosition {
    WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 })
}

fn cue_name(cue: &ObserverFeedbackCueV1) -> &'static str {
    match cue {
        ObserverFeedbackCueV1::PhysicalCombat { .. } => "physical_combat",
        ObserverFeedbackCueV1::WeaponFumbled { .. } => "weapon_fumbled",
        ObserverFeedbackCueV1::SpellLifecycle { .. } => "spell_lifecycle",
        ObserverFeedbackCueV1::SpellImpact { .. } => "spell_impact",
        ObserverFeedbackCueV1::ActorEffect { .. } => "actor_effect",
        ObserverFeedbackCueV1::TileEffect { .. } => "tile_effect",
        ObserverFeedbackCueV1::EffectDamage { .. } => "effect_damage",
        ObserverFeedbackCueV1::Resource { .. } => "resource",
        ObserverFeedbackCueV1::Transaction { .. } => "transaction",
        ObserverFeedbackCueV1::Quest { .. } => "quest",
        ObserverFeedbackCueV1::NpcMessage { .. } => "npc_message",
        ObserverFeedbackCueV1::Defeat { .. } => "defeat",
        ObserverFeedbackCueV1::Corpse { .. } => "corpse",
        ObserverFeedbackCueV1::LifeState { .. } => "life_state",
        ObserverFeedbackCueV1::Resurrection { .. } => "resurrection",
    }
}

#[test]
fn all_feedback_families_project_in_raw_order_without_mutation_or_private_math() {
    let engine = engine();
    let before = engine.snapshot();
    let location = here();
    let corpse_id = CorpseId::parse("corpse:1").expect("canonical corpse ID");
    let character_id: CharacterId =
        serde_json::from_str(r#""character:observer:player""#).expect("character ID");
    let events = vec![
        Event::Attacked {
            attacker_id: ActorId::from("mireling"),
            attacker: "Mireling".to_string(),
            defender_id: ActorId::from("player"),
            defender: "Wayfarer".to_string(),
            defender_location: location.clone(),
            mode: PhysicalAttackMode::Fight,
            damage_kind: PhysicalDamageKind::Cutting,
            effective_combat_add_rating: 900,
            roll: 19,
            damage: 3,
            armor_reduction: 1,
            label: DamageLabel::Light,
            wound_before: WoundState::Unhurt,
            wound_after: WoundState::Wounded,
            defender_hp: 7,
        },
        Event::WeaponFumbled {
            attacker_id: ActorId::from("mireling"),
            attacker: "Mireling".to_string(),
            item_instance_id: "private-weapon-instance".to_string(),
            mode: PhysicalAttackMode::Fight,
            reason: tme_rules::model::WeaponFumbleReason::AlignmentMismatch,
            result: WeaponFumbleResult::Dropped,
        },
        Event::SpellCastCommitted {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            target: None,
            casting_method: tme_rules::model::SpellCastingMethod::Direct,
            mp_cost: Some(2),
            stamina_cost: None,
        },
        Event::SpellDamaged {
            caster_id: ActorId::from("mireling"),
            caster: "Mireling".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            target_id: ActorId::from("player"),
            target: "Wayfarer".to_string(),
            location: location.clone(),
            damage_kind: Some("private-damage-kind".to_string()),
            damage: 2,
            hp: 5,
        },
        Event::EffectApplied {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            location: location.clone(),
            instance_id: "private-effect-instance".to_string(),
            effect_id: "poison".to_string(),
            source_kind: "private-source-kind".to_string(),
            source_id: "private-source-id".to_string(),
            kind: "damage_over_time".to_string(),
            tags: vec!["private-tag".to_string()],
            potency: 99,
            remaining_rounds: Some(3),
        },
        Event::TileEffectApplied {
            location: location.clone(),
            instance_id: "private-tile-instance".to_string(),
            effect_id: "fire".to_string(),
            source_kind: "private-source-kind".to_string(),
            source_id: "private-source-id".to_string(),
            kind: "hazard".to_string(),
            tags: vec!["private-tag".to_string()],
            potency: 99,
            remaining_rounds: None,
            passability: None,
            sight: None,
            hazard: Some("private-hazard".to_string()),
            move_cost: None,
        },
        Event::EffectDamaged {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            location: location.clone(),
            instance_id: "private-effect-instance".to_string(),
            effect_id: "poison".to_string(),
            kind: "damage_over_time".to_string(),
            tags: vec!["private-tag".to_string()],
            damage: 1,
            hp: 4,
        },
        Event::ResourceRegenerated {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            resource: ResourceKind::Stamina,
            activity: ResourceActivity::Active,
            boundary_at: LogicalTime::new(3),
            base_amount: 1,
            multiplier_numerator: 3,
            multiplier_denominator: 2,
            rounding: tme_rules::MagicArithmeticRounding::Down,
            modifier_item_instance_id: Some("private-item-instance".to_string()),
            modifier_item_definition_id: Some("private-item-definition".to_string()),
            modifier_item: Some("Private Item".to_string()),
            modifier_item_position: Some(tme_rules::CarriedPosition::LeftHand),
            amount: 1,
            current: 9,
            maximum: 10,
        },
        Event::TransactionCommitted {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            source: TransactionSourceV1::NpcInteraction {
                npc_actor_id: ActorId::from("mireling"),
                interaction_id: "speak".to_string(),
            },
            costs: vec![],
            rewards: vec![],
        },
        Event::NpcSpoke {
            npc_actor_id: ActorId::from("mireling"),
            npc: "Mireling".to_string(),
            recipient_character_id: character_id,
            interaction_id: "speak".to_string(),
            response: "A committed response.".to_string(),
        },
        Event::ActorDefeated {
            actor_id: ActorId::from("mireling"),
            actor: "Mireling".to_string(),
            kind: ActorKind::Monster,
            location: location.clone(),
            cause: DeathCause::Physical,
            credited_actor_id: Some(ActorId::from("player")),
            loot_claim: None,
        },
        Event::CorpseCreated {
            corpse_id: corpse_id.clone(),
            origin_actor_id: ActorId::from("mireling"),
            origin_character_id: None,
            origin_kind: ActorKind::Monster,
            origin_name: "Mireling".to_string(),
            location: location.clone(),
            created_at: LogicalTime::new(4),
            sequence: 1,
            loot_claim: None,
        },
        Event::ActorLifeStateChanged {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            from: ActorLifeState::Ghost {
                corpse_id: corpse_id.clone(),
                defeated_at: LogicalTime::new(4),
            },
            to: ActorLifeState::Alive,
        },
        Event::ActorResurrected {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            corpse_id: Some(corpse_id),
            method: ResurrectionMethod::Gods,
            destination: location,
            current_hp: 1,
            current_stamina: 1,
        },
    ];

    let projection = engine
        .observer_projection(&ActorId::from("player"), &events)
        .expect("feedback projection");
    assert_eq!(engine.snapshot(), before, "projection remains read-only");
    let cues = projection
        .events
        .iter()
        .filter_map(|event| match event {
            ObservedEventV1::Feedback { cue } => Some(cue),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cues.iter().map(|cue| cue_name(cue)).collect::<Vec<_>>(),
        [
            "physical_combat",
            "weapon_fumbled",
            "spell_lifecycle",
            "resource",
            "spell_impact",
            "actor_effect",
            "tile_effect",
            "effect_damage",
            "resource",
            "transaction",
            "npc_message",
            "defeat",
            "corpse",
            "life_state",
            "resurrection",
        ]
    );
    let encoded = serde_json::to_string(&projection.events).expect("serialize feedback");
    for private in [
        "private-weapon-instance",
        "private-effect-instance",
        "private-tile-instance",
        "private-source",
        "private-tag",
        "private-damage-kind",
        "private-item",
        "potency",
        "roll",
    ] {
        assert!(!encoded.contains(private), "feedback leaked {private}");
    }
    assert!(
        encoded.contains("poison"),
        "public effect ID remains available"
    );
}

#[test]
fn quest_feedback_uses_the_canonical_definition_and_private_character_route() {
    let engine = ContentParts::tracked("npc_quest_interactions", "profile/npc_quest_interactions")
        .engine(7)
        .expect("quest engine");
    let event = Event::QuestStateChanged {
        character_id: serde_json::from_str(r#""character:harbor:primary""#).expect("character ID"),
        quest_id: "harbor_escort".to_string(),
        before_stage_id: None,
        after_stage_id: "awaiting_token".to_string(),
    };
    let projection = engine
        .observer_projection(&ActorId::from("player"), &[event])
        .expect("quest feedback");
    assert!(matches!(
        projection.events.as_slice(),
        [ObservedEventV1::Feedback {
            cue: ObserverFeedbackCueV1::Quest {
                quest_title,
                after_stage_label,
                terminal: false,
                ..
            }
        }] if quest_title == "Harbor Escort" && after_stage_label == "Bring the signal token"
    ));
}

#[test]
fn nested_transaction_overflow_and_invalid_feedback_text_fail_projection() {
    let engine = engine();
    let excessive_costs = (0..=tme_rules::MAX_FEEDBACK_TRANSACTION_COSTS)
        .map(|_| TransactionCostReceiptV1::CarriedGold {
            amount: 1,
            position: CarriedGoldPosition::Sack,
            before: 2,
            after: 1,
        })
        .collect();
    let excessive = Event::TransactionCommitted {
        actor_id: ActorId::from("player"),
        actor: "Wayfarer".to_string(),
        source: TransactionSourceV1::NpcInteraction {
            npc_actor_id: ActorId::from("mireling"),
            interaction_id: "speak".to_string(),
        },
        costs: excessive_costs,
        rewards: vec![],
    };
    assert!(
        engine
            .observer_projection(&ActorId::from("player"), &[excessive])
            .is_err()
    );

    let invalid_text = Event::NpcSpoke {
        npc_actor_id: ActorId::from("mireling"),
        npc: "Mireling".to_string(),
        recipient_character_id: serde_json::from_str(r#""character:observer:player""#)
            .expect("character ID"),
        interaction_id: "speak".to_string(),
        response: "line\nbreak".to_string(),
    };
    assert!(
        engine
            .observer_projection(&ActorId::from("player"), &[invalid_text])
            .is_err()
    );
}
