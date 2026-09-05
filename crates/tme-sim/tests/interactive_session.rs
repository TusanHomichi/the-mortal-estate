use std::io::Cursor;
use std::path::PathBuf;

use tme_sim::{RunMode, RunOptions, run_interactive_with_io};

fn scenario_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn run_session(scenario: &str, seed: u64, commands: &str) -> String {
    let output = run_interactive_with_io(
        RunOptions {
            scenario_path: scenario_path(scenario),
            seed: Some(seed),
            mode: RunMode::Transcript,
        },
        Cursor::new(commands.as_bytes()),
        Vec::new(),
    )
    .expect("interactive session should run");

    String::from_utf8(output).expect("output should be utf8")
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("golden")
            .join(name),
    )
    .expect("golden should read")
    .replace("\r\n", "\n")
}

fn insert_mode(scripted: &str) -> String {
    scripted.replacen("seed: 7\n\n", "seed: 7\nmode: interactive\n\n", 1)
}

fn insert_prompt_before_commits(mut transcript: String, commands: &[&str]) -> String {
    let mut cursor = 0;
    for command in commands {
        let marker = "\n\nplayer ready:";
        let relative = transcript[cursor..]
            .find(marker)
            .expect("scripted transcript should have one player-ready marker per command");
        let start = cursor + relative;
        let replacement = format!("\n> {command}{marker}");
        transcript.replace_range(start..start + marker.len(), &replacement);
        cursor = start + replacement.len();
    }
    transcript
}

fn insert_prompt_before_final(transcript: String, command: &str) -> String {
    transcript.replacen(
        "\n\nfinal state",
        &format!("\n> {command}\n\nfinal state"),
        1,
    )
}

#[test]
fn interactive_first_room_matches_scripted_events_with_prompts() {
    let output = run_session(
        "../../content/test-corpus/first_room.json",
        7,
        "path east east\nfight mireling\nquit\n",
    );
    let expected = insert_prompt_before_final(
        insert_prompt_before_commits(
            insert_mode(&golden("first_room_seed_7.txt")),
            &["path east east", "fight mireling"],
        ),
        "quit",
    );

    assert_eq!(output, expected);
}

#[test]
fn interactive_quit_mid_session_prints_final_state() {
    let output = run_session(
        "../../content/test-corpus/first_room.json",
        7,
        "wait\nquit\n",
    );

    assert!(output.contains("mode: interactive\n"));
    assert!(output.contains("> wait\n\nplayer ready: Delver at 3000ms\n"));
    assert!(output.contains("> quit\n\nfinal state\n"));
    assert!(!output.contains("player ready: Delver at 6000ms\n"));
    assert!(output.ends_with("Mireling at realm_0/room_0:2,1 hp=7 alive\n"));
}

#[test]
fn interactive_eof_after_prompt_prints_final_state() {
    let output = run_session("../../content/test-corpus/first_room.json", 7, "");

    assert_eq!(
        output,
        concat!(
            "The Mortal Estate local simulation\n",
            "scenario: first_room\n",
            "seed: 7\n",
            "mode: interactive\n",
            "\n",
            "loaded \"First Room\" realms=[realm_0] levels=[realm_0/room_0]\n",
            "player Delver at realm_0/room_0:1,1 hp=12\n",
            "monster Mireling at realm_0/room_0:3,1 hp=7\n",
            "> \n",
            "\n",
            "final state\n",
            "Delver at realm_0/room_0:1,1 hp=12 alive\n",
            "Mireling at realm_0/room_0:3,1 hp=7 alive\n",
        )
    );
}

#[test]
fn parse_error_reprompts_without_costing_a_round() {
    let output = run_session(
        "../../content/test-corpus/first_room.json",
        7,
        "frobnicate\nwait\nquit\n",
    );

    assert!(output.contains("> frobnicate\nerror: unknown command: frobnicate\n"));
    assert!(output.contains("> wait\n\nplayer ready: Delver at 3000ms\n"));
    assert!(!output.contains("player ready: Delver at 6000ms\n"));
}

#[test]
fn engine_step_error_reprompts_without_costing_the_next_round() {
    let output = run_session(
        "../../content/test-corpus/first_room.json",
        7,
        "fight mireling\nwait\nquit\n",
    );

    assert!(output.contains("> fight mireling\nerror: fight target is out of range\n"));
    assert!(output.contains("> wait\n\nplayer ready: Delver at 3000ms\n"));
    assert!(!output.contains("player ready: Delver at 6000ms\n"));
    assert!(output.contains("> quit\n\nfinal state\n"));
    assert!(output.ends_with("Mireling at realm_0/room_0:2,1 hp=7 alive\n"));
}

#[test]
fn help_output_is_plain_ascii() {
    let output = run_session(
        "../../content/test-corpus/first_room.json",
        7,
        "help\nquit\n",
    );

    assert!(output.is_ascii());
    assert!(output.contains("known commands:\n"));
    assert!(output.contains("  path <dir>...      - walk/run/sprint a 1-3 direction mixed path\n"));
}

#[test]
fn interactive_living_player_resolves_and_searches_stacked_corpses() {
    let output = run_session(
        "../../content/test-corpus/death_corpse.json",
        7,
        "fight scavenger\nfight lookout\nsearch corpse\nsearch 2 corpse\nquit\n",
    );

    assert!(output.contains("> search corpse\n\nplayer ready: Wayfarer at 9000ms\n"));
    assert!(output.contains("Wayfarer searched corpse:2: items_released=0 gold_released=0\n"));
    assert!(output.contains("> search 2 corpse\n\nplayer ready: Wayfarer at 12000ms\n"));
    assert!(output.contains("Wayfarer searched corpse:1: items_released=1 gold_released=3\n"));
    assert!(output.contains("> quit\n\nfinal state\n"));
    assert!(output.contains("Wayfarer (Fighter) at realm_0/room_0:1,1 hp=6 alive\n"));
    assert!(!output.contains("Wayfarer life state: alive -> ghost"));
}

#[test]
fn help_lists_current_item_commands() {
    let output = run_session(
        "../../content/test-corpus/first_room.json",
        7,
        "help\nquit\n",
    );

    assert!(output.contains("  move_item <item> to <position|ground_here> - relocate an item\n"));
    assert!(output.contains("  drink <item>       - drink a carried consumable\n"));
    assert!(output.contains("  show sack          - show carried items\n"));
    assert!(output.contains("  search corpse|N corpse|<corpse_id> - search one corpse here\n"));
    for removed in [
        "take <item>",
        "retrieve <item>",
        "drop <item>",
        "equip <item>",
        "unequip <slot>",
    ] {
        assert!(!output.contains(removed));
    }
}

#[test]
fn interactive_balm_cache_matches_scripted_events_with_prompts() {
    let commands = [
        "move_item healing_balm to sack_item_1",
        "move_item spare_balm to sack_item_2",
        "path east east",
        "move east",
        "fight warden",
        "fight warden",
        "wait",
        "wait",
        "drink healing_balm",
        "fight warden",
        "fight warden",
        "fight warden",
        "drink spare_balm",
        "show sack",
    ];
    let output = run_session(
        "../../content/test-corpus/balm_cache.json",
        7,
        &format!("{}\nquit\n", commands.join("\n")),
    );
    let expected = insert_prompt_before_final(
        insert_prompt_before_commits(insert_mode(&golden("balm_cache_seed_7.txt")), &commands),
        "quit",
    );

    assert_eq!(output, expected);
    assert!(output.contains("Delver drinks the Healing Balm and the empty bottle shatters\n"));
}

#[test]
fn inventory_step_error_reprompts_without_costing_next_round() {
    let output = run_session(
        "../../content/test-corpus/first_room.json",
        7,
        "move_item missing_item to sack_item_1\nwait\nquit\n",
    );

    assert!(output.contains(
        "> move_item missing_item to sack_item_1\nerror: unknown item instance \"missing_item\"\n"
    ));
    assert!(output.contains("> wait\n\nplayer ready: Delver at 3000ms\n"));
    assert!(!output.contains("player ready: Delver at 6000ms\n"));
}

#[test]
fn interactive_cast_handles_warm_and_warmed_cast_flow() {
    let output = run_session(
        "../../content/test-corpus/spell_readiness.json",
        7,
        "warm charged_spark\nwait\ncast warmed watcher\nquit\n",
    );

    assert!(output.contains("Wiz warms Charged Spark"));
    assert!(output.contains("Wiz's Charged Spark is ready"));
    assert!(output.contains("Wiz casts warmed Charged Spark"));
}
