use std::path::PathBuf;
use std::process::Command;

use tme_sim::{RunMode, RunOptions, run_with_options};

fn assert_golden(scenario: &str, seed: u64, golden: &str) {
    let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(scenario);
    let output = run_with_options(RunOptions {
        scenario_path,
        seed: Some(seed),
        mode: RunMode::Transcript,
    })
    .expect("simulation should run");

    let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(golden);
    let expected = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|error| panic!("{}: {error}", golden_path.display()))
        .replace("\r\n", "\n");

    assert_eq!(output, expected);
}

#[test]
fn first_room_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/first_room.json",
        7,
        "golden/first_room_seed_7.txt",
    );
}

#[test]
fn first_land_structure_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/first_land_structure.json",
        7,
        "golden/first_land_structure_seed_7.txt",
    );
}

#[test]
fn combat_labels_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/combat_labels.json",
        7,
        "golden/combat_labels_seed_7.txt",
    );
}

#[test]
fn inspect_room_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/inspect_room.json",
        7,
        "golden/inspect_room_seed_7.txt",
    );
}

#[test]
fn terrain_movement_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/terrain_movement.json",
        7,
        "golden/terrain_movement_seed_7.txt",
    );
}

#[test]
fn resource_movement_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/resource_movement.json",
        7,
        "golden/resource_movement_seed_7.txt",
    );
}

#[test]
fn starter_circuit_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/starter_circuit.json",
        7,
        "golden/starter_circuit_seed_7.txt",
    );
}

#[test]
fn reach_attack_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/reach_attack.json",
        7,
        "golden/reach_attack_seed_7.txt",
    );
}

#[test]
fn ranged_attack_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/ranged_attack.json",
        7,
        "golden/ranged_attack_seed_7.txt",
    );
}

#[test]
fn thrown_attack_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/thrown_attack.json",
        7,
        "golden/thrown_attack_seed_7.txt",
    );
}

#[test]
fn kobold_warren_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/kobold_warren.json",
        7,
        "golden/kobold_warren_seed_7.txt",
    );
}

#[test]
fn spider_gallery_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/spider_gallery.json",
        7,
        "golden/spider_gallery_seed_7.txt",
    );
}

#[test]
fn troll_track_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/troll_track.json",
        7,
        "golden/troll_track_seed_7.txt",
    );
}

#[test]
fn gargoyle_threshold_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/gargoyle_threshold.json",
        7,
        "golden/gargoyle_threshold_seed_7.txt",
    );
}

#[test]
fn death_corpse_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/death_corpse.json",
        7,
        "golden/death_corpse_seed_7.txt",
    );
}

#[test]
fn supply_cache_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/supply_cache.json",
        7,
        "golden/supply_cache_seed_7.txt",
    );
}

#[test]
fn item_instance_contract_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/item_instance_contract.json",
        7,
        "golden/item_instance_contract_seed_7.txt",
    );
}

#[test]
fn resting_hollow_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/resting_hollow.json",
        7,
        "golden/resting_hollow_seed_7.txt",
    );
}

#[test]
fn balm_cache_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/balm_cache.json",
        7,
        "golden/balm_cache_seed_7.txt",
    );
}

#[test]
fn undercroft_loop_seed_7_matches_golden() {
    assert_golden(
        "../../content/test-corpus/undercroft_loop.json",
        7,
        "golden/undercroft_loop_seed_7.txt",
    );
}

#[test]
fn status_effects_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/status_effects.json",
        7,
        "golden/status_effects_seed_7.txt",
    );
}

#[test]
fn spell_effects_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/spell_effects.json",
        7,
        "golden/spell_effects_seed_7.txt",
    );
}

#[test]
fn control_poison_protection_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/control_poison_protection.json",
        7,
        "golden/control_poison_protection_seed_7.txt",
    );
}

#[test]
fn area_path_terrain_spells_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/area_path_terrain_spells.json",
        7,
        "golden/area_path_terrain_spells_seed_7.txt",
    );
}

#[test]
fn utility_door_secret_item_spells_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/utility_door_secret_item_spells.json",
        7,
        "golden/utility_door_secret_item_spells_seed_7.txt",
    );
}

#[test]
fn spell_learning_purchase_casting_xp_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/spell_learning_purchase_casting_xp.json",
        7,
        "golden/spell_learning_purchase_casting_xp_seed_7.txt",
    );
}

#[test]
fn summons_created_creature_lifecycle_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/summons_created_creature_lifecycle.json",
        7,
        "golden/summons_created_creature_lifecycle_seed_7.txt",
    );
}

#[test]
fn profession_specific_actions_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/profession_specific_actions.json",
        7,
        "golden/profession_specific_actions_seed_7.txt",
    );
}

#[test]
fn martial_hand_block_actions_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/martial_hand_block_actions.json",
        7,
        "golden/martial_hand_block_actions_seed_7.txt",
    );
}

#[test]
fn knight_support_actions_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/knight_support_actions.json",
        7,
        "golden/knight_support_actions_seed_7.txt",
    );
}

#[test]
fn monster_spellcasting_special_attacks_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/monster_spellcasting_special_attacks.json",
        7,
        "golden/monster_spellcasting_special_attacks_seed_7.txt",
    );
}

#[test]
fn magic_profession_gallery_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/magic_profession_gallery.json",
        7,
        "golden/magic_profession_gallery_seed_7.txt",
    );
}

#[test]
fn remaining_spell_effect_families_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/remaining_spell_effect_families.json",
        7,
        "golden/remaining_spell_effect_families_seed_7.txt",
    );
}

#[test]
fn default_cli_scenario_path_works_from_crate_directory() {
    let binary = env!("CARGO_BIN_EXE_tme-sim");
    let output = Command::new(binary)
        .arg("--seed")
        .arg("7")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("The Mortal Estate local simulation"));
}

#[test]
fn character_sheet_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/character_sheet.json",
        7,
        "golden/character_sheet_seed_7.txt",
    );
}

#[test]
fn equipment_slots_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/equipment_slots.json",
        7,
        "golden/equipment_slots_seed_7.txt",
    );
}

#[test]
fn xp_progression_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/xp_progression.json",
        7,
        "golden/xp_progression_seed_7.txt",
    );
}

#[test]
fn skill_progression_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/skill_progression.json",
        7,
        "golden/skill_progression_seed_7.txt",
    );
}

#[test]
fn gold_training_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/gold_training.json",
        7,
        "golden/gold_training_seed_7.txt",
    );
}

#[test]
fn gold_bank_locker_storage_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/gold_bank_locker_storage.json",
        7,
        "golden/gold_bank_locker_storage_seed_7.txt",
    );
}

#[test]
fn service_transactions_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/service_transactions.json",
        7,
        "golden/service_transactions_seed_7.txt",
    );
}

#[test]
fn merchant_item_services_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/merchant_item_services.json",
        7,
        "golden/merchant_item_services_seed_7.txt",
    );
}

#[test]
fn restoration_services_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/restoration_services.json",
        7,
        "golden/restoration_services_seed_7.txt",
    );
}

#[test]
fn npc_quest_interactions_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/npc_quest_interactions.json",
        7,
        "golden/npc_quest_interactions_seed_7.txt",
    );
}

#[test]
fn fidelity_gallery_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/fidelity_gallery.json",
        7,
        "golden/fidelity_gallery_seed_7.txt",
    );
}

#[test]
fn knight_promotion_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/knight_promotion.json",
        7,
        "golden/knight_promotion_seed_7.txt",
    );
}

#[test]
fn progression_gallery_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/progression_gallery.json",
        7,
        "golden/progression_gallery_seed_7.txt",
    );
}

#[test]
fn alignment_social_law_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/alignment_social_law.json",
        7,
        "golden/alignment_social_law_seed_7.txt",
    );
}

#[test]
fn knight_social_consequence_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/knight_social_consequence.json",
        7,
        "golden/knight_social_consequence_seed_7.txt",
    );
}

#[test]
fn town_adventure_loop_gallery_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/town_adventure_loop_gallery.json",
        7,
        "golden/town_adventure_loop_gallery_seed_7.txt",
    );
}

#[test]
fn world_topology_gallery_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/world_topology_gallery.json",
        7,
        "golden/world_topology_gallery_seed_7.txt",
    );
}

#[test]
fn creature_ecology_gallery_seed_7_matches_golden_transcript() {
    assert_golden(
        "../../content/test-corpus/creature_ecology_gallery.json",
        7,
        "golden/creature_ecology_gallery_seed_7.txt",
    );
}
