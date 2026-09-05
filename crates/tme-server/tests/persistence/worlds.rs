// Private worlds proof for the persistence integration target.
fn social_bootstrap(
    first_account: wire::AccountId,
    first_character: wire::CharacterId,
    replacement_character: wire::CharacterId,
    second_account: wire::AccountId,
    second_character: wire::CharacterId,
    shared_facet: wire::FacetId,
) -> PostgresBootstrap {
    let mut shared = scenario_engine();
    let original_character = shared.world().actors[0]
        .character_id
        .clone()
        .expect("scenario player character");
    let temporary_character = tme_rules::CharacterId::new("prototype:social:second");
    let temporary_replacement = tme_rules::CharacterId::new("prototype:social:replacement");
    shared.world_mut().actors[0].stats.attack = 100;
    let mut second = shared.world().actors[0].clone();
    second.id = tme_rules::ActorId::new("player2");
    second.name = "Companion".to_string();
    second.character_id = Some(temporary_character.clone());
    second.hp = 1;
    second
        .character
        .as_mut()
        .expect("second social player character sheet")
        .resources
        .hp = 1;
    second.timing.tie_break_order += 100;
    second.carried.items.clear();
    second.carried.gold = Default::default();
    let preferences = shared
        .world()
        .communication_preferences
        .get(&original_character)
        .cloned()
        .unwrap_or_default();
    let presence = shared
        .world()
        .character_presence
        .get(&original_character)
        .copied()
        .expect("scenario character presence");
    let quest_state = shared
        .world()
        .quest_states
        .get(&original_character)
        .cloned();
    let mut replacement = shared.world().actors[0].clone();
    replacement.id = tme_rules::ActorId::new("player3");
    replacement.name = "Replacement".to_string();
    replacement.character_id = Some(temporary_replacement.clone());
    replacement.timing.tie_break_order += 200;
    shared.world_mut().actors.push(second);
    shared.world_mut().actors.push(replacement);
    shared
        .world_mut()
        .communication_preferences
        .insert(temporary_character.clone(), preferences);
    shared
        .world_mut()
        .communication_preferences
        .insert(temporary_replacement.clone(), Default::default());
    shared
        .world_mut()
        .character_presence
        .insert(temporary_character.clone(), presence);
    shared
        .world_mut()
        .character_presence
        .insert(temporary_replacement.clone(), presence);
    if let Some(quest_state) = quest_state {
        shared
            .world_mut()
            .quest_states
            .insert(temporary_character, quest_state.clone());
        shared
            .world_mut()
            .quest_states
            .insert(temporary_replacement, quest_state);
    }
    let arrival_id = shared
        .definition()
        .world_template()
        .arrivals()
        .keys()
        .min()
        .cloned()
        .expect("social scenario arrival");
    shared
        .clone()
        .advance_action_interval()
        .expect("two-player social facet advances one boundary");

    let _ = arrival_id;
    PostgresBootstrap {
        world: PostgresWorldBootstrap {
            facet_id: shared_facet,
            key: "social-world".to_string(),
            engine: shared,
        },
        characters: vec![
            PostgresCharacterBootstrap {
                account_id: first_account,
                character_id: first_character,
                slot: 1,
                display_name: wire::DisplayName::new("Social One").unwrap(),
                actor_id: tme_rules::ActorId::new("player"),
            },
            PostgresCharacterBootstrap {
                account_id: second_account,
                character_id: second_character,
                slot: 1,
                display_name: wire::DisplayName::new("Social Two").unwrap(),
                actor_id: tme_rules::ActorId::new("player2"),
            },
            PostgresCharacterBootstrap {
                account_id: first_account,
                character_id: replacement_character,
                slot: 2,
                display_name: wire::DisplayName::new("Replacement").unwrap(),
                actor_id: tme_rules::ActorId::new("player3"),
            },
        ],
    }
}

fn scenario_engine() -> tme_rules::Engine {
    let mut scenario = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend([
        "..",
        "..",
        "content",
        "test-corpus",
        "world_topology_gallery.json",
    ]);
    tme_sim::load_engine_from_scenario(&scenario, Some(7)).unwrap()
}
