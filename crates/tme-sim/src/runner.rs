use tme_rules::Engine;

use crate::fixture::script::ScriptedIntentSource;
use crate::loading::load_simulation;
use crate::session;
use crate::trace;
use crate::{RunMode, RunOptions};

pub fn load_engine_from_scenario(
    path: &std::path::Path,
    override_seed: Option<u64>,
) -> Result<Engine, String> {
    let loaded = load_simulation(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let effective_seed = loaded.scenario.effective_rng_seed(override_seed);
    Engine::new(loaded.world_seed, effective_seed).map_err(|error| error.to_string())
}

pub fn run_with_options(options: RunOptions) -> Result<String, String> {
    let loaded = load_simulation(&options.scenario_path)
        .map_err(|error| format!("{}: {error}", options.scenario_path.display()))?;
    let effective_seed = loaded.scenario.effective_rng_seed(options.seed);
    let scenario_id = loaded.scenario.id.clone();
    let script = loaded.scenario.script;
    let engine =
        Engine::new(loaded.world_seed, effective_seed).map_err(|error| error.to_string())?;

    match options.mode {
        RunMode::Transcript => {
            let output = session::run_simulation_loop(
                engine,
                session::SessionHeader {
                    scenario_id,
                    seed: effective_seed,
                    scenario_loaded_event: loaded.scenario_loaded_event,
                    mode: None,
                },
                ScriptedIntentSource::new(script),
                Vec::new(),
                session::StepErrorPolicy::Fatal,
            )?;
            String::from_utf8(output).map_err(|error| error.to_string())
        }
        RunMode::TraceJson => {
            let trace = trace::run_trace_json(
                engine,
                scenario_id,
                effective_seed,
                ScriptedIntentSource::new(script),
                session::StepErrorPolicy::Fatal,
            )?;
            serde_json::to_string_pretty(&trace).map_err(|error| error.to_string())
        }
        RunMode::TraceJsonV2 => {
            let trace = trace::run_trace_json_v2(
                engine,
                scenario_id,
                effective_seed,
                ScriptedIntentSource::new(script),
                session::StepErrorPolicy::Fatal,
            )?;
            serde_json::to_string_pretty(&trace).map_err(|error| error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_room() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-corpus/first_room.json")
    }

    #[test]
    fn sim_runner_prepends_scenario_loaded_exactly_once() {
        let transcript = run_with_options(RunOptions {
            scenario_path: first_room(),
            seed: None,
            mode: RunMode::Transcript,
        })
        .unwrap();
        let loaded = transcript.find("loaded \"First Room\"").unwrap();
        let actor = transcript.find("player Delver").unwrap();
        assert!(loaded < actor);
        assert_eq!(transcript.matches("loaded \"First Room\"").count(), 1);

        let engine = load_engine_from_scenario(&first_room(), None).unwrap();
        assert!(
            engine
                .initial_events()
                .iter()
                .all(|event| !matches!(event, tme_rules::Event::ScenarioLoaded { .. }))
        );
    }
}
