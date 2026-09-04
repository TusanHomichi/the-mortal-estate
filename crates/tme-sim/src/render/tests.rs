use super::*;
use tme_rules::{
    CharacterAlignment, Coord, Event, ItemConsumptionReason, SocialAlignmentSource, SocialBehavior,
    SocialNature, SocialOwnerRelation, SocialProfile,
};

fn item_view(item_instance_id: &str, name: &str) -> tme_rules::ItemInstanceViewV1 {
    tme_rules::ItemInstanceViewV1 {
        item_instance_id: item_instance_id.to_string(),
        item_definition_id: item_instance_id.to_string(),
        name: name.to_string(),
        quantity: 1,
        identified: false,
        appraised: false,
        known_unit_value_gold: None,
        known_stack_value_gold: None,
        unit_burden: 1,
        stack_burden: 1,
        binding: tme_rules::ItemBindingViewV1::Unrestricted,
        bow_readiness: None,
    }
}

fn positioned_item(
    item_instance_id: &str,
    name: &str,
    position: tme_rules::CarriedPosition,
) -> tme_rules::PositionedItemViewV1 {
    tme_rules::PositionedItemViewV1 {
        item: item_view(item_instance_id, name),
        position,
        category: None,
        valid_placements: vec![tme_rules::ItemPlacementKind::Sack],
        capability: None,
        armor: None,
    }
}

#[test]
fn renders_exact_item_relocation_and_sack_events() {
    let lines = render_events(&[
        Event::ItemRelocated {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            item_instance_id: "hemp_rope".to_string(),
            item_definition_id: "hemp_rope".to_string(),
            item: "Hemp Rope".to_string(),
            quantity: 1,
            from: tme_rules::ItemLocationViewV1::Ground {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 1, y: 1 },
                ),
            },
            to: tme_rules::ItemLocationViewV1::Carried {
                actor_id: "player0".into(),
                position: tme_rules::CarriedPosition::SackItem1,
            },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            loot_claim: None,
        },
        Event::ItemRelocated {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            item_instance_id: "hemp_rope".to_string(),
            item_definition_id: "hemp_rope".to_string(),
            item: "Hemp Rope".to_string(),
            quantity: 1,
            from: tme_rules::ItemLocationViewV1::Carried {
                actor_id: "player0".into(),
                position: tme_rules::CarriedPosition::SackItem1,
            },
            to: tme_rules::ItemLocationViewV1::Ground {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
            },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            loot_claim: None,
        },
        Event::SackShown {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            items: vec![
                positioned_item(
                    "hemp_rope",
                    "Hemp Rope",
                    tme_rules::CarriedPosition::SackItem1,
                ),
                positioned_item(
                    "waterskin",
                    "Waterskin",
                    tme_rules::CarriedPosition::SackItem2,
                ),
            ],
            gold: 3,
        },
        Event::SackShown {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            items: vec![],
            gold: 0,
        },
    ]);

    assert_eq!(
        lines,
        vec![
            "Delver moved Hemp Rope from (1,1) to sack_item_1".to_string(),
            "Delver moved Hemp Rope from sack_item_1 to (2,1)".to_string(),
            "Delver's sack:".to_string(),
            "  sack_item_1: Hemp Rope".to_string(),
            "  sack_item_2: Waterskin".to_string(),
            "  gold: 3".to_string(),
            "Delver's sack is empty".to_string(),
        ]
    );
}

#[test]
fn renders_offer_creation_refusal_and_reserved_hand_return_truthfully() {
    let sender_character_id: tme_rules::CharacterId =
        serde_json::from_value(serde_json::json!("character:sender")).unwrap();
    let recipient_character_id: tme_rules::CharacterId =
        serde_json::from_value(serde_json::json!("character:recipient")).unwrap();
    let lines = render_events(&[
        Event::ItemOfferCreated {
            actor_id: "sender_actor".into(),
            actor: "Sender".to_string(),
            item_instance_id: "field_case".to_string(),
            item_definition_id: "field_case".to_string(),
            item: "Field Case".to_string(),
            sender_character_id: sender_character_id.clone(),
            recipient_character_id: recipient_character_id.clone(),
            source_position: tme_rules::CarriedPosition::RightHand,
        },
        Event::ItemRelocated {
            actor_id: "recipient_actor".into(),
            actor: "Recipient".to_string(),
            item_instance_id: "field_case".to_string(),
            item_definition_id: "field_case".to_string(),
            item: "Field Case".to_string(),
            quantity: 1,
            from: tme_rules::ItemLocationViewV1::Offered {
                sender_character_id: sender_character_id.clone(),
                recipient_character_id: recipient_character_id.clone(),
                source_position: tme_rules::CarriedPosition::RightHand,
            },
            to: tme_rules::ItemLocationViewV1::Carried {
                actor_id: "sender_actor".into(),
                position: tme_rules::CarriedPosition::RightHand,
            },
            reason: tme_rules::ItemRelocationReason::OfferReturned,
            loot_claim: None,
        },
        Event::ItemOfferCompleted {
            actor_id: "recipient_actor".into(),
            actor: "Recipient".to_string(),
            item_instance_id: "field_case".to_string(),
            item_definition_id: "field_case".to_string(),
            item: "Field Case".to_string(),
            sender_character_id,
            recipient_character_id,
            destination: tme_rules::CarriedPosition::RightHand,
            reason: tme_rules::ItemOfferCompletionReasonV1::Refused,
        },
    ]);
    assert_eq!(
        lines,
        [
            "Sender offered Field Case from right_hand to character character:recipient",
            "Field Case returned to sender_actor's right_hand",
            "Recipient completed the Field Case offer: refused to right_hand",
        ]
    );
}

#[test]
fn renders_npc_transaction_receipts_in_exact_order() {
    let character_id: tme_rules::CharacterId =
        serde_json::from_value(serde_json::json!("character:harbor:primary")).unwrap();
    let lines = render_events(&[Event::TransactionCommitted {
        actor_id: "player".into(),
        actor: "Delver".to_string(),
        source: tme_rules::TransactionSourceV1::NpcInteraction {
            npc_actor_id: "wayfinder".into(),
            interaction_id: "offer_signal_token".to_string(),
        },
        costs: vec![tme_rules::TransactionCostReceiptV1::SelectedCarriedItem {
            item_instance_id: "player_signal_token".to_string(),
            item_definition_id: "signal_token".to_string(),
            consumed_quantity: 1,
            remaining_quantity: 0,
        }],
        rewards: vec![
            tme_rules::TransactionRewardReceiptV1::NpcInteraction {
                npc_actor_id: "wayfinder".into(),
                interaction_id: "offer_signal_token".to_string(),
                outcome: tme_rules::NpcInteractionOutcome::BeginFollow,
            },
            tme_rules::TransactionRewardReceiptV1::QuestStage {
                character_id,
                quest_id: "harbor_escort".to_string(),
                before_stage_id: Some("awaiting_token".to_string()),
                after_stage_id: "escorting_guide".to_string(),
            },
        ],
    }]);

    assert_eq!(
        lines,
        [
            "Delver completes NPC interaction offer_signal_token with wayfinder.",
            "  costs: [{\"kind\":\"selected_carried_item\",\"item_instance_id\":\"player_signal_token\",\"item_definition_id\":\"signal_token\",\"consumed_quantity\":1,\"remaining_quantity\":0}]",
            "  rewards: [{\"kind\":\"npc_interaction\",\"npc_actor_id\":\"wayfinder\",\"interaction_id\":\"offer_signal_token\",\"outcome\":{\"kind\":\"begin_follow\"}},{\"kind\":\"quest_stage\",\"character_id\":\"character:harbor:primary\",\"quest_id\":\"harbor_escort\",\"before_stage_id\":\"awaiting_token\",\"after_stage_id\":\"escorting_guide\"}]",
        ]
    );
}

#[test]
fn renders_actor_defeat_corpse_and_ghost_events() {
    let corpse_id = tme_rules::CorpseId::parse("corpse:1").unwrap();
    let lines = render_events(&[
        Event::ActorDefeated {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            kind: tme_rules::ActorKind::Player,
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 0, y: 0 },
            ),
            cause: tme_rules::DeathCause::Physical,
            credited_actor_id: None,
            loot_claim: None,
        },
        Event::CorpseCreated {
            corpse_id: corpse_id.clone(),
            origin_actor_id: "player0".into(),
            origin_character_id: None,
            origin_kind: tme_rules::ActorKind::Player,
            origin_name: "Delver".to_string(),
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 0, y: 0 },
            ),
            created_at: tme_rules::LogicalTime::FIRST,
            sequence: 1,
            loot_claim: None,
        },
        Event::ActorLifeStateChanged {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            from: tme_rules::ActorLifeState::Alive,
            to: tme_rules::ActorLifeState::Ghost {
                corpse_id,
                defeated_at: tme_rules::LogicalTime::FIRST,
            },
        },
    ]);

    assert_eq!(
        lines,
        vec![
            "Delver was defeated: cause=physical".to_string(),
            "corpse corpse:1 created for Delver at realm_0/room_0:0,0".to_string(),
            "Delver life state: alive -> ghost".to_string(),
        ]
    );
}

#[test]
fn renders_defeat_drop_events() {
    let lines = render_events(&[
        Event::ItemRelocated {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            item_instance_id: "hemp_rope".to_string(),
            item_definition_id: "hemp_rope".to_string(),
            item: "Hemp Rope".to_string(),
            quantity: 1,
            from: tme_rules::ItemLocationViewV1::Carried {
                actor_id: "player0".into(),
                position: tme_rules::CarriedPosition::SackItem1,
            },
            to: tme_rules::ItemLocationViewV1::Ground {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 1, y: 1 },
                ),
            },
            reason: tme_rules::ItemRelocationReason::DeathDrop,
            loot_claim: None,
        },
        Event::ItemRelocated {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            item_instance_id: "iron_dirk".to_string(),
            item_definition_id: "iron_dirk".to_string(),
            item: "Iron Dirk".to_string(),
            quantity: 1,
            from: tme_rules::ItemLocationViewV1::Carried {
                actor_id: "player0".into(),
                position: tme_rules::CarriedPosition::RightHand,
            },
            to: tme_rules::ItemLocationViewV1::Ground {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 1, y: 1 },
                ),
            },
            reason: tme_rules::ItemRelocationReason::DeathDrop,
            loot_claim: None,
        },
    ]);

    assert_eq!(
        lines,
        vec![
            "Delver's Hemp Rope fell to the ground at (1,1)".to_string(),
            "Delver's Iron Dirk fell to the ground at (1,1)".to_string(),
        ]
    );
}

#[test]
fn renders_resource_recovery_events() {
    let lines = render_events(&[Event::ResourceRegenerated {
        actor_id: "player0".into(),
        actor: "Delver".to_string(),
        resource: tme_rules::ResourceKind::Hp,
        activity: tme_rules::ResourceActivity::Inactive,
        boundary_at: tme_rules::LogicalTime::new(2),
        base_amount: 1,
        multiplier_numerator: 1,
        multiplier_denominator: 1,
        rounding: tme_rules::MagicArithmeticRounding::Down,
        modifier_item_instance_id: None,
        modifier_item_definition_id: None,
        modifier_item: None,
        modifier_item_position: None,
        amount: 1,
        current: 9,
        maximum: 12,
    }]);

    assert_eq!(
        lines,
        vec!["Delver regenerated 1 Hp (9/12, Inactive, time 6000ms)".to_string()]
    );
}

#[test]
fn renders_door_and_transition_events() {
    let lines = render_events(&[
        Event::DoorOpened {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "entrance_hall",
                Coord { x: 4, y: 1 },
            ),
        },
        Event::WorldTransition {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            from: tme_rules::WorldPosition::new("realm_0", "entrance_hall", Coord { x: 4, y: 1 }),
            to: tme_rules::WorldPosition::new("realm_0", "guard_post", Coord { x: 1, y: 1 }),
            navigation: NavigationKind::Door,
        },
        Event::DoorClosed {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            location: tme_rules::WorldPosition::new("realm_0", "guard_post", Coord { x: 0, y: 1 }),
        },
    ]);

    assert_eq!(
            lines,
            vec![
                "--- Door opened: realm_0/entrance_hall:4,1 ---".to_string(),
                "--- Delver transitions via Door: realm_0/entrance_hall:4,1 -> realm_0/guard_post:1,1 ---".to_string(),
                "--- Door closed: realm_0/guard_post:0,1 ---".to_string(),
            ]
        );
}

#[test]
fn renders_balm_events() {
    let lines = render_events(&[
        Event::ItemConsumed {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            item_instance_id: "healing_balm".to_string(),
            item_definition_id: "healing_balm".to_string(),
            item: "Healing Balm".to_string(),
            quantity_consumed: 1,
            remaining_quantity: 0,
            reason: ItemConsumptionReason::Drink,
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 0, y: 0 },
            ),
        },
        Event::BalmHealed {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 0, y: 0 },
            ),
            amount: 2,
            hp: 6,
        },
    ]);

    assert_eq!(
        lines,
        vec![
            "Delver drinks the Healing Balm and the empty bottle shatters".to_string(),
            "The balm knits Delver's wounds: regained 2 hp (6 hp)".to_string(),
        ]
    );
}

#[test]
fn renders_item_consumption_reasons() {
    let lines = render_events(&[Event::ItemConsumed {
        actor_id: "player0".into(),
        actor: "Delver".to_string(),
        item_instance_id: "healing_balm".to_string(),
        item_definition_id: "healing_balm".to_string(),
        item: "Healing Balm".to_string(),
        quantity_consumed: 1,
        remaining_quantity: 0,
        reason: ItemConsumptionReason::Drink,
        location: tme_rules::WorldPosition::new(
            "realm_0",
            "room_0",
            tme_rules::Coord { x: 0, y: 0 },
        ),
    }]);

    assert_eq!(
        lines,
        vec!["Delver drinks the Healing Balm and the empty bottle shatters".to_string(),]
    );
}

#[test]
fn renders_spell_learned_events() {
    let lines = render_events(&[Event::SpellLearned {
        actor_id: "player0".into(),
        actor: "Wiz".to_string(),
        spell_id: "spark".to_string(),
        spell_name: "Spark".to_string(),
        lane: "wizard_magic".to_string(),
        skill_requirement: 1,
        learned_at_level: 2,
        gold_cost: 25,
        trainer_service_id: "wizard_trainer".to_string(),
        trainer: "Wizard Trainer".to_string(),
        spell_book_item_instance_id: "spell_book".to_string(),
        spell_book_item_definition_id: "spell_book".to_string(),
        spell_book: "Spell Book".to_string(),
        spell_book_character_id: "character:player0".to_string(),
    }]);

    assert_eq!(
        lines,
        vec![
            "Wizard Trainer records Spark (wizard_magic) in Wiz's retained Spell Book for 25 gold"
                .to_string()
        ]
    );
}

#[test]
fn renders_dx_magic_attempt_practice_and_reward_receipts() {
    let lines = render_events(&[
        Event::ThaumAboveSkillEvaluated {
            actor_id: "player0".into(),
            actor: "Thaum".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            track_id: "thaumaturge_magic".to_string(),
            current_skill_level: 1,
            skill_requirement: 3,
            gap: 2,
            roll_denominator: 20,
            success_threshold: 18,
            roll: 17,
            success: true,
        },
        Event::MagicPracticeEvaluated {
            actor_id: "player0".into(),
            actor: "Wiz".to_string(),
            current_class_id: "wizard".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            track_id: "wizard_magic".to_string(),
            mp_cost: 3,
            cast_class: tme_rules::SpellCastClass::Character,
            primary_attribute: Some(tme_rules::MagicPrimaryAttribute::Intelligence),
            primary_attribute_value: Some(14),
            base_raw_points: 3,
            primary_attribute_bonus_raw_points: 1,
            total_raw_points: 4,
            risk_applied: false,
            reason: "eligible_successful_cast".to_string(),
        },
        Event::DefeatRewardEvaluated {
            target_id: "target".into(),
            target: "Mireling".to_string(),
            authored_experience: 13,
            actual_damage: 3,
            weighted_damage_numerator: 6,
            weighted_damage_denominator: 5,
            available_experience: 5,
            awarded_experience: 5,
            reason: "contribution_shared".to_string(),
        },
    ]);

    assert_eq!(
        lines,
        vec![
            "Thaum's above-skill Spark attempt: gap=2 roll=17 threshold=18 success=true"
                .to_string(),
            "Wiz's Spark practice on wizard_magic: raw=4 reason=eligible_successful_cast"
                .to_string(),
            "shared defeat reward for Mireling: available=5 awarded=5 reason=contribution_shared"
                .to_string(),
        ]
    );
}

#[test]
fn renders_summon_lifecycle_events() {
    let lines = render_events(&[
        Event::ActorSummoned {
            caster_id: "player0".into(),
            caster: "Wiz".to_string(),
            spell_id: "call_echo".to_string(),
            spell_name: "Call Echo".to_string(),
            actor_id: "summon:call_echo:1:echo_guardian".into(),
            actor: "Echo Guardian".to_string(),
            template_id: "echo_guardian".to_string(),
            owner_id: "player0".into(),
            social: SocialProfile {
                alignment_source: SocialAlignmentSource::Inherent {
                    alignment: CharacterAlignment::Lawful,
                },
                nature: SocialNature::Other,
                behavior: SocialBehavior::AlignmentCreature,
                owner_relation: SocialOwnerRelation::Summoner,
            },
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "start",
                tme_rules::Coord { x: 2, y: 1 },
            ),
            remaining_rounds: Some(2),
        },
        Event::SummonExpired {
            actor_id: "summon:call_echo:1:echo_guardian".into(),
            actor: "Echo Guardian".to_string(),
            instance_id: "summon:call_echo:1:echo_guardian".into(),
            owner_id: "player0".into(),
            source_spell_id: "call_echo".to_string(),
            template_id: "echo_guardian".to_string(),
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "start",
                tme_rules::Coord { x: 2, y: 1 },
            ),
        },
    ]);

    assert_eq!(
        lines,
        vec![
            "--- Wiz summoned Echo Guardian at realm_0/start:2,1 for 2 rounds ---".to_string(),
            "--- Echo Guardian faded from realm_0/start:2,1 ---".to_string(),
        ]
    );
}

#[test]
fn renders_tile_effect_events() {
    let lines = render_events(&[
        Event::TileEffectApplied {
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 2, y: 1 },
            ),
            instance_id: "tile:web:1".to_string(),
            effect_id: "web_field".to_string(),
            source_kind: "spell".to_string(),
            source_id: "web_field".to_string(),
            kind: "terrain_overlay".to_string(),
            tags: vec!["web".to_string()],
            potency: 0,
            remaining_rounds: Some(2),
            passability: Some("hindered".to_string()),
            sight: Some("obscured".to_string()),
            hazard: None,
            move_cost: Some(2),
        },
        Event::TileEffectTicked {
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 2, y: 1 },
            ),
            instance_id: "tile:web:1".to_string(),
            effect_id: "web_field".to_string(),
            kind: "terrain_overlay".to_string(),
            tags: vec!["web".to_string()],
            potency: 0,
            remaining_rounds: Some(1),
        },
        Event::TileEffectDamaged {
            actor_id: "target".into(),
            actor: "Target".to_string(),
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 2, y: 1 },
            ),
            instance_id: "tile:ember:1".to_string(),
            effect_id: "ember_cloud".to_string(),
            kind: "terrain_overlay".to_string(),
            tags: vec!["fire".to_string()],
            damage: 2,
            hp: 6,
        },
        Event::TileEffectExpired {
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 2, y: 1 },
            ),
            instance_id: "tile:web:1".to_string(),
            effect_id: "web_field".to_string(),
            kind: "terrain_overlay".to_string(),
        },
        Event::TileEffectRemoved {
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 2, y: 1 },
            ),
            instance_id: "tile:web:1".to_string(),
            effect_id: "web_field".to_string(),
            kind: "terrain_overlay".to_string(),
            reason: "clear_sight".to_string(),
        },
    ]);

    assert_eq!(
            lines,
            vec![
                "tile effect applied: web_field at realm_0/room_0:2,1 passability=hindered sight=obscured hazard=none cost=2 remaining=2".to_string(),
                "tile effect ticked: web_field at realm_0/room_0:2,1 remaining=1".to_string(),
                "tile effect damaged: Target by ember_cloud damage=2 hp=6".to_string(),
                "tile effect expired: web_field at realm_0/room_0:2,1".to_string(),
                "tile effect removed: web_field at realm_0/room_0:2,1 reason=clear_sight".to_string(),
            ]
        );
}
