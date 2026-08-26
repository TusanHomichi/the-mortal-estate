use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_GRAPH: AtomicU64 = AtomicU64::new(0);

fn validator() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tme-validate-content"))
}

fn trace_validator() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tme-validate-trace"))
}

fn valid_scenario() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/test-corpus/first_room.json")
        .display()
        .to_string()
}

fn graph_with_unknown_actor_room() -> (PathBuf, PathBuf) {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/test-corpus");
    let source_scenario = source_root.join("first_room.json");
    let scenario: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&source_scenario).unwrap()).unwrap();
    let root = std::env::temp_dir().join(format!(
        "tme-validator-cli-{}-{}",
        std::process::id(),
        NEXT_TEMP_GRAPH.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let scenario_path = root.join("first_room.json");
    std::fs::copy(&source_scenario, &scenario_path).unwrap();
    for field in ["catalog", "world_template", "simulation_seed"] {
        let reference = scenario[field].as_str().unwrap();
        let destination = root.join(reference);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(source_root.join(reference), &destination).unwrap();
    }
    let seed_path = root.join(scenario["simulation_seed"].as_str().unwrap());
    let mut seed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&seed_path).unwrap()).unwrap();
    seed["actors"][0]["room"] = serde_json::json!("missing_room");
    std::fs::write(seed_path, serde_json::to_vec(&seed).unwrap()).unwrap();
    (root, scenario_path)
}

#[test]
fn validator_exit_zero_emits_one_compact_success_document() {
    let input = valid_scenario();
    let output = validator().arg(&input).output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches('\n').count(), 1);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["kind"], "tme_content_validation_result");
    assert_eq!(report["results"][0]["input"], input);
    assert_eq!(report["results"][0]["valid"], true);
    assert_eq!(report["results"][0]["diagnostics"], serde_json::json!([]));
}

#[test]
fn validator_exit_one_preserves_batch_order_and_reports_path_failure() {
    let valid = valid_scenario();
    let missing = "definitely/missing/simulation.json";
    let output = validator().args([&valid, missing]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["results"][0]["input"], valid);
    assert_eq!(report["results"][0]["valid"], true);
    assert_eq!(report["results"][1]["input"], missing);
    assert_eq!(report["results"][1]["valid"], false);
    assert_eq!(
        report["results"][1]["diagnostics"][0]["component"],
        "scenario"
    );
    assert_eq!(report["results"][1]["diagnostics"][0]["pointer"], "");
}

#[test]
fn validator_protocol_preserves_removed_room_field_pointer_and_message() {
    let (root, scenario) = graph_with_unknown_actor_room();
    let input = scenario.display().to_string();
    let output = validator().arg(&input).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["results"][0]["input"], input);
    assert_eq!(report["results"][0]["valid"], false);
    assert_eq!(
        report["results"][0]["diagnostics"][0],
        serde_json::json!({
            "component": "simulation_seed",
            "pointer": "/actors/0/room",
            "message": "invalid Simulation Seed 3: unknown field `room`, expected one of `id`, `actor_definition_id`, `location`, `npc`, `character_id`, `character`, `starter_character`, `carried`, `active_effects`"
        })
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn validator_exit_two_has_no_json_protocol_on_usage_error() {
    let output = validator().output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "usage: tme-validate-content <simulation-scenario>...\n"
    );
}

#[test]
fn trace_validator_accepts_a_tracked_path_and_stdin() {
    let trace_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/trace_v2_first_room_seed_7.json");
    let output = trace_validator().arg(&trace_path).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "OK: 2 steps, trace V2 consistent\n"
    );
    assert!(output.stderr.is_empty());

    let mut child = trace_validator()
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(
        child.stdin.as_mut().unwrap(),
        &std::fs::read(trace_path).unwrap(),
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

#[test]
fn trace_validator_rejects_invalid_input_and_uses_exit_two_for_usage() {
    let output = trace_validator()
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("invalid JSON")
    );

    let output = trace_validator().output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "usage: tme-validate-trace <trace.json|->\n"
    );
}
