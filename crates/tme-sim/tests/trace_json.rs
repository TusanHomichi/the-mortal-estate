use serde_json::{Value, json};
use std::path::PathBuf;
use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, COMMAND_CONTRACT_VERSION, EVENT_CONTRACT_VERSION,
    OBSERVED_SNAPSHOT_CONTRACT_VERSION, SNAPSHOT_CONTRACT_VERSION, TRACE_V2_CONTRACT_VERSION,
    TraceV1, TraceV2,
};

fn scenario_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/test-corpus")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn run_trace(scenario: &str, seed: u64) -> TraceV1 {
    let args = vec![
        "tme-sim".to_string(),
        "--scenario".to_string(),
        scenario_path(scenario),
        "--seed".to_string(),
        seed.to_string(),
        "--trace-json".to_string(),
    ];
    let output = tme_sim::run_from_args(args).expect("trace run should succeed");
    serde_json::from_str(&output).expect("trace output must be valid JSON")
}

// ---------------------------------------------------------------------------
// Trace V2 tests
// ---------------------------------------------------------------------------

fn run_trace_v2(scenario: &str, seed: u64) -> TraceV2 {
    let args = vec![
        "tme-sim".to_string(),
        "--scenario".to_string(),
        scenario_path(scenario),
        "--seed".to_string(),
        seed.to_string(),
        "--trace-json-v2".to_string(),
    ];
    let output = tme_sim::run_from_args(args).expect("trace v2 run should succeed");
    serde_json::from_str(&output).expect("trace v2 output must be valid JSON")
}

fn add_unknown_field(value: &Value, pointer: &str) -> Value {
    let mut mutated = value.clone();
    mutated
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("missing JSON pointer {pointer}"))
        .as_object_mut()
        .unwrap_or_else(|| panic!("JSON pointer {pointer} is not an object"))
        .insert("private_fact".to_string(), json!({"quarantined": true}));
    mutated
}

#[path = "trace_json/creature_ecology_gallery_trace_v1_and_v2_match_goldens.rs"]
mod creature_ecology_gallery_trace_v1_and_v2_match_goldens;

#[path = "trace_json/trace_v2_utility_door_secret_item_fixture_exposes_bu_events.rs"]
mod trace_v2_utility_door_secret_item_fixture_exposes_bu_events;

#[path = "trace_json/trace_v2_remaining_spell_effect_families_exposes_all_dy_routes.rs"]
mod trace_v2_remaining_spell_effect_families_exposes_all_dy_routes;

#[path = "trace_json/trace_v2_town_adventure_loop_gallery_closes_end_to_end_state.rs"]
mod trace_v2_town_adventure_loop_gallery_closes_end_to_end_state;
