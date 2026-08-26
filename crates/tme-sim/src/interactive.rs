use std::io::{BufRead, Write};

use crate::commands::{self, MetaCommand, ParsedLine};
use crate::session::{self, IntentAction, IntentSource, TranscriptWriter};

pub(crate) struct InteractiveIntentSource<R: BufRead> {
    input: R,
}

impl<R: BufRead> InteractiveIntentSource<R> {
    pub(crate) fn new(input: R) -> Self {
        Self { input }
    }
}

impl<R: BufRead> IntentSource for InteractiveIntentSource<R> {
    fn next_intent<W: Write>(
        &mut self,
        engine: &tme_rules::Engine,
        transcript: &mut TranscriptWriter<W>,
    ) -> Result<IntentAction, String> {
        transcript.write_raw("> ").map_err(|e| e.to_string())?;
        transcript.flush().map_err(|e| e.to_string())?;

        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) => {
                // EOF completes the prompt line in captured transcripts.
                transcript.write_raw("\n").map_err(|e| e.to_string())?;
                return Ok(IntentAction::Stop);
            }
            Ok(_) => {
                // Echo the line (read_line includes the trailing newline)
                transcript.write_raw(&line).map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(e.to_string()),
        }

        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        match commands::parse_line_for_engine(trimmed, engine) {
            Ok(ParsedLine::Intent(intent)) => Ok(IntentAction::Step(intent)),
            Ok(ParsedLine::CorpseSearch(_)) => {
                unreachable!("engine-aware parsing resolves corpse search")
            }
            Ok(ParsedLine::Meta(MetaCommand::Quit)) => Ok(IntentAction::Stop),
            Ok(ParsedLine::Meta(MetaCommand::Help)) => {
                for help_line in interactive_help_lines() {
                    transcript
                        .write_raw(&format!("{help_line}\n"))
                        .map_err(|e| e.to_string())?;
                }
                Ok(IntentAction::Continue)
            }
            Ok(ParsedLine::Empty) => Ok(IntentAction::Continue),
            Err(error) => {
                transcript
                    .write_raw(&format!("error: {error}\n"))
                    .map_err(|e| e.to_string())?;
                Ok(IntentAction::Continue)
            }
        }
    }
}

pub(crate) fn interactive_help_lines() -> Vec<String> {
    vec![
        "known commands:".to_string(),
        "  move <dir>         - walk one hex (n/ne/e/se/s/sw/w/nw)".to_string(),
        "  run <dir>          - run two hexes".to_string(),
        "  sprint <dir>       - sprint three hexes".to_string(),
        "  path <dir>...      - walk/run/sprint a 1-3 direction mixed path".to_string(),
        "  up|u / down|d      - use matching stairs under the actor".to_string(),
        "  fight|kick|jumpkick|poke|shoot|throw [--unsafe] <actor-id> - use a physical attack"
            .to_string(),
        "    --unsafe explicitly authorizes this one hostile action against a protected target"
            .to_string(),
        "  nock|load          - nock the bow in your right hand".to_string(),
        "  unload             - unload a nocked bow".to_string(),
        "  search corpse|N corpse|<corpse_id> - search one corpse here".to_string(),
        "  cast <spell> [target] - cast a direct spell".to_string(),
        "  cast warmed [target] - cast the ready warmed spell".to_string(),
        "  warm <spell>       - warm a warm-then-cast spell".to_string(),
        "  fizzle             - cancel the warmed spell".to_string(),
        "  learn_spell <spell> - learn a spell from a spell teacher".to_string(),
        "  train <service> <gold> - buy permanent learning rate".to_string(),
        "  critique <service> <track> - hear a skill critique".to_string(),
        "  move_item <item> to <position|ground_here> - relocate an item".to_string(),
        "  move_gold <source> to <position|ground_here> <all|amount> - relocate gold".to_string(),
        "  bank_deposit <service> <capability> <gold_pile> - deposit one ground pile".to_string(),
        "  bank_withdraw <service> <capability> <amount> - withdraw to the ground".to_string(),
        "  locker_deposit <service> <capability> <item> - store one carried item".to_string(),
        "  locker_withdraw <service> <capability> <item> <position> - retrieve one item"
            .to_string(),
        "  offer_item <character> <item> - offer a held item to a party member".to_string(),
        "  accept_offer <item> <position> - accept an incoming item offer".to_string(),
        "  refuse_offer <item> - refuse an incoming item offer".to_string(),
        "  withdraw_offer <item> - withdraw an outgoing item offer".to_string(),
        "  drink <item>       - drink a carried consumable".to_string(),
        "  open <dir>         - open a door in that direction".to_string(),
        "  close <dir>        - close a door in that direction".to_string(),
        "  show sack          - show carried items".to_string(),
        "  wait               - wait (skip turn)".to_string(),
        "  rest               - rest and fizzle any warmed spell".to_string(),
        "  inspect            - inspect your surroundings".to_string(),
        "  help               - show this help".to_string(),
        "  quit               - end the session".to_string(),
    ]
}

pub fn run_interactive_with_io<R: BufRead, W: Write>(
    options: crate::RunOptions,
    input: R,
    output: W,
) -> Result<W, String> {
    let loaded = crate::loading::load_simulation(&options.scenario_path)
        .map_err(|error| format!("{}: {error}", options.scenario_path.display()))?;
    let scenario_id = loaded.scenario.id.clone();
    let effective_seed = loaded.scenario.effective_rng_seed(options.seed);
    let replay_seed = loaded.world_seed.clone();
    let engine = tme_rules::Engine::new(loaded.world_seed, effective_seed)
        .map_err(|error| error.to_string())?;
    // Interactive mode ignores the scenario script; the player provides intent live.
    let source = InteractiveIntentSource::new(input);
    session::run_simulation_loop(
        engine,
        session::SessionHeader {
            scenario_id,
            seed: effective_seed,
            scenario_loaded_event: loaded.scenario_loaded_event,
            mode: Some("interactive"),
        },
        source,
        output,
        session::StepErrorPolicy::RePrompt {
            world_seed: replay_seed,
            seed: effective_seed,
            accepted_intents: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::interactive_help_lines;

    #[test]
    fn help_text_lists_only_current_spell_item_and_training_commands() {
        let help = interactive_help_lines();
        assert!(
            help.iter().any(|line| {
                line.contains("fight|kick|jumpkick|poke|shoot|throw [--unsafe] <actor-id>")
            }),
            "help text should expose the exact unsafe-attack grammar"
        );
        assert!(
            help.iter().any(
                |line| line.contains("one hostile action") && line.contains("protected target")
            ),
            "help text should explain per-action unsafe authorization"
        );

        assert!(
            !help.iter().any(|line| line.contains("learn <spell>      ")),
            "help text must not list the removed learn alias"
        );
        assert!(
            help.iter().any(|line| line.contains("learn_spell <spell>")),
            "help text should mention the canonical learn_spell command"
        );
        assert!(
            help.iter().any(|line| line.contains("move_item <item>")),
            "help text should mention the canonical move_item command"
        );
        for current in [
            "move_gold <source>",
            "bank_deposit <service>",
            "bank_withdraw <service>",
            "locker_deposit <service>",
            "locker_withdraw <service>",
            "offer_item <character>",
            "accept_offer <item>",
            "refuse_offer <item>",
            "withdraw_offer <item>",
        ] {
            assert!(
                help.iter().any(|line| line.contains(current)),
                "help text should mention current ED command {current}"
            );
        }
        assert!(
            help.iter()
                .any(|line| line.contains("train <service> <gold>")),
            "help text should mention the canonical train command"
        );
        assert!(
            help.iter()
                .any(|line| line.contains("critique <service> <track>")),
            "help text should mention the canonical critique command"
        );
        for removed in [
            "take <item>",
            "retrieve <item>",
            "drop <item>",
            "equip <item>",
            "unequip <slot>",
        ] {
            assert!(
                !help.iter().any(|line| line.contains(removed)),
                "help text must not list removed command {removed}"
            );
        }
    }
}
