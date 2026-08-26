use serde_json::json;

use crate::events::{
    AccountMarkAssessmentReasonV1, AlignmentChangeReasonV1, Event, NpcGrudgeReasonV1,
    SelfDefenseChangeReasonV1,
};
use crate::model::{
    ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, ActorKind,
    CharacterAlignment, CharacterId, Coord, HostilityReason, LogicalTime, NpcGrudgeRelation,
    NpcState, SocialAlignmentSource, SocialBehavior, SocialContactKind, SocialNature,
    SocialOwnerRelation, SummonedActorState,
};

use super::super::Engine;

fn social_engine() -> Engine {
    crate::engine::setup::test_engine("character_sheet")
}

fn character_id(value: &str) -> CharacterId {
    serde_json::from_value(json!(value)).expect("character id should deserialize")
}

fn clone_character_actor(engine: &mut Engine, id: &str, stable_id: &str) -> usize {
    let mut actor = engine.world.actors[0].clone();
    actor.id = id.into();
    actor.name = id.to_string();
    actor.character_id = Some(character_id(stable_id));
    actor.location.position.x +=
        i32::try_from(engine.world.actors.len()).expect("small test actor count");
    actor.home_location.position = actor.location.position;
    actor.timing.tie_break_order = engine.world.timing.next_tie_break_order;
    engine.world.timing.next_tie_break_order += 1;
    engine.world.actors.push(actor);
    engine.world.actors.len() - 1
}

fn set_character_alignment(
    engine: &mut Engine,
    actor_index: usize,
    alignment: CharacterAlignment,
    karma_points: u32,
) {
    let character = engine.world.actors[actor_index]
        .character
        .as_mut()
        .expect("test actor should be character-backed");
    character.alignment_state.alignment = alignment;
    character.alignment_state.karma_points = karma_points;
}

#[test]
fn neutral_thief_disguise_is_observer_specific_and_has_only_locked_detectors() {
    let mut engine = social_engine();
    set_character_alignment(&mut engine, 0, CharacterAlignment::Neutral, 0);
    engine.world.actors[0]
        .character
        .as_mut()
        .expect("player character")
        .identity
        .current_class_id = "thief".to_string();
    let observer = clone_character_actor(&mut engine, "observer", "character:observer");

    assert_eq!(
        engine
            .perceived_social_identity(observer, 0)
            .expect("foreign observer assessment")
            .alignment,
        CharacterAlignment::Lawful
    );
    assert_eq!(
        engine
            .perceived_social_identity(0, 0)
            .expect("self assessment")
            .alignment,
        CharacterAlignment::Neutral
    );

    engine.world.actors[observer]
        .character
        .as_mut()
        .expect("observer character")
        .identity
        .current_class_id = "knight".to_string();
    assert_eq!(
        engine
            .perceived_social_identity(observer, 0)
            .expect("knight observer assessment")
            .alignment,
        CharacterAlignment::Neutral
    );

    engine.world.actors[observer]
        .character
        .as_mut()
        .expect("observer character")
        .identity
        .current_class_id = "fighter".to_string();
    engine.world.actors[observer].social.behavior = SocialBehavior::TownEnforcer;
    assert_eq!(
        engine
            .perceived_social_identity(observer, 0)
            .expect("town enforcer assessment")
            .alignment,
        CharacterAlignment::Neutral
    );
}

#[test]
fn hostility_uses_directional_alignment_behavior_and_passive_override() {
    let mut engine = social_engine();
    let assessment = engine
        .hostility_assessment(1, 0)
        .expect("chaotic creature assessment");
    assert!(assessment.hostile);
    assert_eq!(assessment.reason, HostilityReason::ChaoticOpposition);

    let reverse = engine
        .hostility_assessment(0, 1)
        .expect("adventurer assessment");
    assert!(!reverse.hostile);
    assert_eq!(reverse.reason, HostilityReason::NoHostility);

    engine.world.actors[1].social.behavior = SocialBehavior::Passive;
    let passive = engine
        .hostility_assessment(1, 0)
        .expect("passive assessment");
    assert!(!passive.hostile);
    assert_eq!(passive.reason, HostilityReason::Passive);
}

#[test]
fn lawful_npc_response_and_grudge_acquisition_require_same_level_visibility() {
    let mut engine = social_engine();
    let npc_index = 1;
    set_character_alignment(&mut engine, 0, CharacterAlignment::Neutral, 0);
    engine.world.actors[npc_index].kind = ActorKind::Npc;
    engine.world.actors[npc_index].social.alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Lawful,
    };
    engine.world.actors[npc_index].social.nature = SocialNature::Human;
    engine.world.actors[npc_index].social.behavior = SocialBehavior::Civilian;
    engine.world.actors[npc_index].npc = Some(NpcState {
        follow_cadence_units: 1,
        interactions: Vec::new(),
        following_character_id: None,
    });

    let visible = engine
        .hostility_assessment(npc_index, 0)
        .expect("visible lawful-NPC assessment");
    assert!(visible.hostile);
    assert_eq!(visible.reason, HostilityReason::LawfulHumanResponse);
    assert!(
        engine
            .has_automatic_combat_priority(npc_index)
            .expect("visible response priority")
    );

    engine.world.actors[0].location.position = Coord { x: 1, y: 3 };
    let behind_wall = engine
        .hostility_assessment(npc_index, 0)
        .expect("wall-blocked lawful-NPC assessment");
    assert!(!behind_wall.hostile);
    assert_eq!(behind_wall.reason, HostilityReason::NoHostility);
    assert!(
        !engine
            .has_automatic_combat_priority(npc_index)
            .expect("wall-blocked response priority")
    );

    engine.world.actors[0].location.position = Coord { x: 2, y: 1 };
    engine.world.actors[0]
        .active_effects
        .push(ActiveEffectState {
            spell_damage_credit: None,
            instance_id: "hidden:player".to_string(),
            effect_id: "hidden".to_string(),
            source: ActiveEffectSource {
                kind: "fixture".to_string(),
                id: "social_visibility_test".to_string(),
            },
            source_actor_id: None,
            hostile_authority: None,
            kind: "control_status".to_string(),
            tags: vec!["hidden".to_string()],
            potency: 0,
            remaining_rounds: Some(20),
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::RefreshDuration,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: Vec::new(),
            last_ticked_at: LogicalTime::ZERO,
        });
    let hidden = engine
        .hostility_assessment(npc_index, 0)
        .expect("hidden lawful-NPC assessment");
    assert!(!hidden.hostile);
    assert_eq!(hidden.reason, HostilityReason::NoHostility);
    assert!(
        !engine
            .has_automatic_combat_priority(npc_index)
            .expect("hidden response priority")
    );
    engine.world.actors[0].active_effects.clear();

    engine.world.actors[0].location.level = "elsewhere".to_string();
    let cross_room = engine
        .hostility_assessment(npc_index, 0)
        .expect("cross-room lawful-NPC assessment");
    assert!(!cross_room.hostile);
    assert_eq!(cross_room.reason, HostilityReason::NoHostility);
    assert!(
        !engine
            .has_automatic_combat_priority(npc_index)
            .expect("cross-room response priority")
    );

    engine
        .world
        .social_relations
        .npc_grudges
        .insert(NpcGrudgeRelation {
            npc_actor_id: engine.world.actors[npc_index].id.clone(),
            attacker_actor_id: engine.world.actors[0].id.clone(),
        });
    let grudge = engine
        .hostility_assessment(npc_index, 0)
        .expect("cross-room grudge assessment");
    assert!(grudge.hostile);
    assert_eq!(grudge.reason, HostilityReason::NpcGrudge);
    assert!(
        !engine
            .has_automatic_combat_priority(npc_index)
            .expect("cross-room grudge priority")
    );
}

#[test]
fn summoned_alignment_creature_never_targets_its_current_owner() {
    let mut engine = social_engine();
    let owner_id = engine.world.actors[0].id.clone();
    engine.world.actors[1].social.owner_relation = SocialOwnerRelation::Summoner;
    engine.world.actors[1].summoned = Some(SummonedActorState {
        instance_id: "summon:test".into(),
        owner_id,
        source_spell_id: "test_spell".to_string(),
        template_id: "test_template".to_string(),
        remaining_rounds: None,
        last_ticked_at: LogicalTime::ZERO,
    });
    let owner = engine
        .hostility_assessment(1, 0)
        .expect("summon owner assessment");
    assert!(!owner.hostile);
    assert_eq!(owner.reason, HostilityReason::Owner);

    let other = clone_character_actor(&mut engine, "other", "character:other");
    let non_owner = engine
        .hostility_assessment(1, other)
        .expect("summon non-owner assessment");
    assert!(non_owner.hostile);
    assert_eq!(non_owner.reason, HostilityReason::ChaoticOpposition);
}

#[test]
fn physical_contact_establishes_and_replaces_exact_self_defense() {
    let mut engine = social_engine();
    let first_attacker =
        clone_character_actor(&mut engine, "first_attacker", "character:first-attacker");
    let mut events = Vec::new();
    let first = engine
        .plan_attack_relations(first_attacker, 0, SocialContactKind::PhysicalAttack)
        .expect("first relation plan");
    engine
        .commit_attack_relations(&first, &mut events)
        .expect("first relation commit");
    assert!(matches!(
        events.as_slice(),
        [Event::SelfDefenseChanged(change)]
            if change.reason == SelfDefenseChangeReasonV1::Established
    ));
    let retaliation = engine
        .attack_safety_assessment(0, first_attacker)
        .expect("retaliation confirmation");
    assert_eq!(
        retaliation.safety,
        crate::model::AttackSafety::OpenSelfDefense
    );
    assert_eq!(
        engine
            .hostility_assessment(0, first_attacker)
            .expect("retaliation hostility")
            .reason,
        HostilityReason::SelfDefense
    );

    let second_attacker =
        clone_character_actor(&mut engine, "second_attacker", "character:second-attacker");
    events.clear();
    let replacement = engine
        .plan_attack_relations(second_attacker, 0, SocialContactKind::PhysicalAttack)
        .expect("replacement relation plan");
    engine
        .commit_attack_relations(&replacement, &mut events)
        .expect("replacement relation commit");
    assert!(matches!(
        events.as_slice(),
        [Event::SelfDefenseChanged(change)]
            if change.reason == SelfDefenseChangeReasonV1::Replaced
    ));
}

#[test]
fn remote_player_kill_karma_has_one_exact_forgiveness_link() {
    let mut engine = social_engine();
    let killer_character_id = engine.world.actors[0]
        .character_id
        .clone()
        .expect("player character ID");
    set_character_alignment(&mut engine, 0, CharacterAlignment::Lawful, 0);
    let character = engine.world.actors[0]
        .character
        .as_mut()
        .expect("player character");
    character.identity.current_class_id = "knight".to_string();
    character.identity.display_class = "Knight".to_string();
    let assessment = crate::model::PlayerKillAssessmentV1 {
        facet_kill_sequence: 19,
        killer_character_id,
        victim_character_id: character_id("character:remote-victim"),
        exempt_self_defense: false,
        consequence: crate::model::PlayerKillConsequenceV1::RequiresAbsentKiller {
            victim_alignment: CharacterAlignment::Lawful,
            victim_nature: SocialNature::Human,
        },
        logical_time: LogicalTime::new(41),
    };

    let (outcome, linked) = engine
        .apply_absent_killer_player_kill_consequence(&assessment)
        .expect("remote consequence");
    assert!(outcome.state_changed);
    assert!(linked);
    let character = engine.world.actors[0]
        .character
        .as_ref()
        .expect("player character");
    assert_eq!(character.alignment_state.karma_points, 1);
    assert_eq!(
        character.alignment_state.alignment,
        CharacterAlignment::Neutral
    );
    assert_eq!(character.identity.current_class_id, "fighter");
    assert_eq!(engine.world.linked_player_kill_karma.len(), 1);

    let forgiven = engine
        .apply_player_kill_karma_forgiveness(&assessment)
        .expect("linked forgiveness");
    assert!(forgiven.state_changed);
    let character = engine.world.actors[0]
        .character
        .as_ref()
        .expect("player character");
    assert_eq!(character.alignment_state.karma_points, 0);
    assert_eq!(
        character.alignment_state.alignment,
        CharacterAlignment::Neutral,
        "forgiveness never reverses alignment"
    );
    assert_eq!(character.identity.current_class_id, "fighter");
    assert!(engine.world.linked_player_kill_karma.is_empty());
    assert!(
        engine
            .apply_player_kill_karma_forgiveness(&assessment)
            .is_err(),
        "the exact point can be reversed at most once"
    );
}

#[test]
fn linked_karma_forgiveness_fails_closed_at_zero_without_consuming_the_link() {
    let mut engine = social_engine();
    let killer_character_id = engine.world.actors[0]
        .character_id
        .clone()
        .expect("player character ID");
    set_character_alignment(&mut engine, 0, CharacterAlignment::Evil, 0);
    let assessment = crate::model::PlayerKillAssessmentV1 {
        facet_kill_sequence: 7,
        killer_character_id: killer_character_id.clone(),
        victim_character_id: character_id("character:remote-victim"),
        exempt_self_defense: false,
        consequence: crate::model::PlayerKillConsequenceV1::RequiresAbsentKiller {
            victim_alignment: CharacterAlignment::Lawful,
            victim_nature: SocialNature::Human,
        },
        logical_time: LogicalTime::new(9),
    };
    engine
        .world
        .linked_player_kill_karma
        .push(crate::model::LinkedPlayerKillKarmaV1 {
            facet_kill_sequence: assessment.facet_kill_sequence,
            killer_character_id,
            victim_character_id: assessment.victim_character_id.clone(),
            logical_time: assessment.logical_time,
        });
    let before_ledger = engine.world.linked_player_kill_karma.clone();
    let before_character = engine.world.actors[0]
        .character
        .clone()
        .expect("player character");
    assert!(
        engine
            .apply_player_kill_karma_forgiveness(&assessment)
            .is_err()
    );
    assert_eq!(engine.world.linked_player_kill_karma, before_ledger);
    assert_eq!(
        engine.world.actors[0]
            .character
            .as_ref()
            .expect("player character"),
        &before_character,
        "failed reversal must be atomic"
    );
}

#[test]
fn true_character_exit_clears_rights_owned_by_or_against_that_character() {
    let mut engine = social_engine();
    let exiting_character_id = engine.world.actors[0]
        .character_id
        .clone()
        .expect("player character ID");
    let attacker = clone_character_actor(&mut engine, "attacker", "character:attacker");
    let other_victim = clone_character_actor(&mut engine, "other_victim", "character:other-victim");
    let mut events = Vec::new();
    let against_exiting = engine
        .plan_attack_relations(attacker, 0, SocialContactKind::PhysicalAttack)
        .expect("attacker-to-exiting relation");
    engine
        .commit_attack_relations(&against_exiting, &mut events)
        .expect("attacker-to-exiting commit");
    let by_exiting = engine
        .plan_attack_relations(0, other_victim, SocialContactKind::PhysicalAttack)
        .expect("exiting-to-victim relation");
    engine
        .commit_attack_relations(&by_exiting, &mut events)
        .expect("exiting-to-victim commit");
    assert_eq!(engine.world.social_relations.self_defense.len(), 2);

    let outcome = engine.apply_character_session_exit(&exiting_character_id);
    assert!(outcome.state_changed);
    assert!(engine.world.social_relations.self_defense.is_empty());
    assert!(
        !engine
            .apply_character_session_exit(&exiting_character_id)
            .state_changed
    );
}

#[test]
fn npc_grudge_preempts_follow_and_follow_resumes_without_combat_priority() {
    let mut engine = social_engine();
    let npc_index = 1;
    let followed_character_id = engine.world.actors[0]
        .character_id
        .clone()
        .expect("player character id");
    engine.world.actors[npc_index].kind = ActorKind::Npc;
    engine.world.actors[npc_index].social.alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Lawful,
    };
    engine.world.actors[npc_index].social.nature = SocialNature::Human;
    engine.world.actors[npc_index].social.behavior = SocialBehavior::Civilian;
    engine.world.actors[npc_index].social.owner_relation = SocialOwnerRelation::None;
    engine.world.actors[npc_index].npc = Some(NpcState {
        follow_cadence_units: 1,
        interactions: Vec::new(),
        following_character_id: Some(followed_character_id),
    });

    let mut events = Vec::new();
    let plan = engine
        .plan_attack_relations(0, npc_index, SocialContactKind::HostileSpellContact)
        .expect("NPC grudge plan");
    engine
        .commit_attack_relations(&plan, &mut events)
        .expect("NPC grudge commit");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::NpcGrudgeEstablished {
            reason: NpcGrudgeReasonV1::HostileSpellContact,
            ..
        }
    )));
    assert_eq!(
        engine
            .hostility_assessment(npc_index, 0)
            .expect("NPC grudge hostility")
            .reason,
        HostilityReason::NpcGrudge
    );

    events.clear();
    engine
        .apply_ready_npc_action(npc_index, &mut events)
        .expect("grudge combat arbitration");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::AutomaticActorDecision { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NpcFollowDecision { .. }))
    );

    engine.world.social_relations.npc_grudges.clear();
    engine.world.actors[npc_index]
        .ai
        .as_mut()
        .expect("NPC AI")
        .awareness
        .remembered = None;
    events.clear();
    engine
        .apply_ready_npc_action(npc_index, &mut events)
        .expect("follow arbitration");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::NpcFollowDecision { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::AutomaticActorDecision { .. }))
    );
}

#[test]
fn fourth_karma_point_turns_neutral_thief_evil_before_mark_assessment() {
    let mut engine = social_engine();
    set_character_alignment(&mut engine, 0, CharacterAlignment::Neutral, 3);
    engine.world.actors[0]
        .character
        .as_mut()
        .expect("killer character")
        .identity
        .current_class_id = "thief".to_string();
    let victim = clone_character_actor(&mut engine, "victim", "character:victim");
    set_character_alignment(&mut engine, victim, CharacterAlignment::Lawful, 0);

    let plan = engine
        .lethal_social_consequence_plan(0, victim, &"player".into())
        .expect("lethal consequence planning")
        .expect("lawful human kill has consequences");
    let mut events = Vec::new();
    engine
        .commit_lethal_social_consequence(&plan, &mut events)
        .expect("lethal consequence commit");
    let alignment_state = &engine.world.actors[0]
        .character
        .as_ref()
        .expect("killer character")
        .alignment_state;
    assert_eq!(alignment_state.alignment, CharacterAlignment::Evil);
    assert_eq!(alignment_state.karma_points, 4);
    assert!(matches!(
        events.as_slice(),
        [
            Event::AlignmentChanged {
                reason: AlignmentChangeReasonV1::KarmaThreshold,
                ..
            },
            Event::KarmaChanged { .. },
            Event::AccountMarkAssessed {
                assessed: true,
                reason: AccountMarkAssessmentReasonV1::AddForPlayerKill,
                ..
            }
        ]
    ));
}

#[test]
fn exact_self_defense_exempts_player_kill_without_mutating_alignment_or_karma() {
    let mut engine = social_engine();
    let attacker = clone_character_actor(&mut engine, "attacker", "character:attacker");
    let relation = engine
        .plan_attack_relations(attacker, 0, SocialContactKind::PhysicalAttack)
        .expect("self-defense relation plan");
    engine
        .commit_attack_relations(&relation, &mut Vec::new())
        .expect("self-defense relation commit");
    let plan = engine
        .lethal_social_consequence_plan(0, attacker, &"player".into())
        .expect("lethal consequence planning")
        .expect("player kill produces mark assessment");
    let mut events = Vec::new();
    engine
        .commit_lethal_social_consequence(&plan, &mut events)
        .expect("lethal consequence commit");
    let alignment_state = &engine.world.actors[0]
        .character
        .as_ref()
        .expect("killer character")
        .alignment_state;
    assert_eq!(alignment_state.alignment, CharacterAlignment::Lawful);
    assert_eq!(alignment_state.karma_points, 0);
    assert!(matches!(
        events.as_slice(),
        [Event::AccountMarkAssessed {
            assessed: false,
            reason: AccountMarkAssessmentReasonV1::ExemptSelfDefense,
            ..
        }]
    ));
}

#[test]
fn lawful_animal_kill_changes_lawful_alignment_without_karma() {
    let mut engine = social_engine();
    engine.world.actors[1].social.alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Lawful,
    };
    engine.world.actors[1].social.nature = SocialNature::Animal;
    engine.world.actors[1].social.behavior = SocialBehavior::Passive;
    let plan = engine
        .lethal_social_consequence_plan(0, 1, &"player".into())
        .expect("animal consequence planning")
        .expect("lawful animal kill has consequence");
    let mut events = Vec::new();
    engine
        .commit_lethal_social_consequence(&plan, &mut events)
        .expect("animal consequence commit");
    let state = &engine.world.actors[0]
        .character
        .as_ref()
        .expect("killer character")
        .alignment_state;
    assert_eq!(state.alignment, CharacterAlignment::Neutral);
    assert_eq!(state.karma_points, 0);
    assert!(matches!(
        events.as_slice(),
        [Event::AlignmentChanged {
            reason: AlignmentChangeReasonV1::UnjustLawfulAnimalKill,
            ..
        }]
    ));
}

#[test]
fn town_terrain_cast_changes_only_lawful_alignment() {
    let (mut catalog, profile, mut template, seed) =
        crate::engine::setup::test_parts("character_sheet");
    let terrain_profile =
        crate::content::CatalogProfileKey::from("profile/area_path_terrain_spells");
    let spell_key = catalog.profiles[&terrain_profile]
        .spells
        .iter()
        .find(|key| {
            catalog
                .spells
                .get(*key)
                .is_some_and(|spell| spell.id == "ember_cloud")
        })
        .cloned()
        .expect("tracked terrain spell should exist");
    catalog
        .profiles
        .get_mut(&profile)
        .unwrap()
        .spells
        .push(spell_key.clone());
    let spell = catalog
        .spells
        .get_mut(&spell_key)
        .expect("tracked spell should exist");
    spell.social.town_law = crate::content::TownLawClassificationDef::TerrainAlignmentViolation;
    let spell_id = spell.id.clone();
    template
        .realms
        .get_mut("realm_0")
        .expect("caster realm")
        .levels
        .get_mut("room_0")
        .expect("caster room")
        .law_zone = crate::content::LawZoneDef::Town;
    let mut engine = crate::engine::setup::test_engine_from_parts(catalog, profile, template, seed);
    engine.world.actors[0]
        .character
        .as_mut()
        .expect("caster character")
        .identity
        .current_class_id = "knight".to_string();
    let site = engine.world.actors[0].location.site();

    let plan = engine
        .town_law_consequence_plan(0, &spell_id, &site)
        .expect("town consequence planning")
        .expect("lawful town caster has consequence");
    let mut events = Vec::new();
    engine
        .commit_town_law_consequence(&plan, &mut events)
        .expect("town consequence commit");
    let character = engine.world.actors[0]
        .character
        .as_ref()
        .expect("caster character");
    assert_eq!(
        character.alignment_state.alignment,
        CharacterAlignment::Neutral
    );
    assert_eq!(character.alignment_state.karma_points, 0);
    assert_eq!(character.identity.current_class_id, "knight");
    assert!(matches!(
        events.as_slice(),
        [Event::AlignmentChanged {
            reason: AlignmentChangeReasonV1::TownTerrainCast,
            ..
        }]
    ));
}

#[test]
fn captured_relation_plan_rejects_changed_alignment_without_mutation() {
    let mut engine = social_engine();
    let attacker = clone_character_actor(&mut engine, "attacker", "character:attacker");
    let plan = engine
        .plan_attack_relations(attacker, 0, SocialContactKind::PhysicalAttack)
        .expect("relation plan");
    set_character_alignment(&mut engine, attacker, CharacterAlignment::Evil, 0);
    let error = engine
        .commit_attack_relations(&plan, &mut Vec::new())
        .expect_err("changed captured fact must fail");
    assert!(error.message().contains("changed before commit"));
    assert!(engine.world.social_relations.self_defense.is_empty());
}
