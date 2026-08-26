use std::env;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use crate::{run_interactive_with_io, run_with_options};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOptions {
    pub scenario_path: PathBuf,
    pub seed: Option<u64>,
    pub mode: RunMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Transcript,
    TraceJson,
    TraceJsonV2,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            scenario_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/test-corpus/first_room.json"),
            seed: None,
            mode: RunMode::Transcript,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Scripted(RunOptions),
    Interactive(RunOptions),
    Help,
}

pub fn run() -> Result<String, String> {
    run_from_args(env::args())
}

pub fn run_from_args<I, S>(args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match parse_args(args)? {
        CliAction::Scripted(options) => run_with_options(options),
        CliAction::Interactive(_) => Err(
            "--interactive requires run_cli_with_io so stdin/stdout can be streamed".to_string(),
        ),
        CliAction::Help => Ok(help_text()),
    }
}

pub fn parse_args<I, S>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut options = RunOptions::default();
    let mut interactive = false;
    let mut args = args.into_iter().map(Into::into);
    let _program = args.next();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scenario" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--scenario requires a path".to_string())?;
                options.scenario_path = PathBuf::from(value);
            }
            "--seed" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--seed requires a value".to_string())?;
                options.seed = Some(
                    value
                        .parse()
                        .map_err(|error| format!("invalid --seed value {value:?}: {error}"))?,
                );
            }
            "--trace-json" => options.mode = RunMode::TraceJson,
            "--trace-json-v2" => options.mode = RunMode::TraceJsonV2,
            "--interactive" => interactive = true,
            "--help" | "-h" => return Ok(CliAction::Help),
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if interactive {
        Ok(CliAction::Interactive(options))
    } else {
        Ok(CliAction::Scripted(options))
    }
}

pub fn run_cli_with_io<I, S, R, W>(args: I, input: R, mut output: W) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    R: BufRead,
    W: Write,
{
    match parse_args(args)? {
        CliAction::Scripted(options) => {
            let transcript = run_with_options(options)?;
            output
                .write_all(transcript.as_bytes())
                .map_err(|error| error.to_string())
        }
        CliAction::Interactive(options) => {
            let _output = run_interactive_with_io(options, input, output)?;
            Ok(())
        }
        CliAction::Help => output
            .write_all(help_text().as_bytes())
            .map_err(|error| error.to_string()),
    }
}

fn help_text() -> String {
    concat!(
        "usage: tme-sim [--scenario <path>] [--seed <u64>] [--interactive] [--trace-json]\n",
        "\n",
        "options:\n",
        "  --scenario <path>  simulation scenario to load\n",
        "  --seed <u64>       override the scenario's deterministic seed\n",
        "  --interactive      read line commands from stdin\n",
        "  --trace-json       output trace V1 as JSON instead of text transcript\n",
        "  --trace-json-v2    output trace V2 as JSON (includes observed snapshots and action context)\n",
        "  -h, --help         show this help\n",
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_is_an_optional_override() {
        let default = parse_args(["tme-sim"]).unwrap();
        let CliAction::Scripted(default) = default else {
            panic!("expected scripted defaults")
        };
        assert_eq!(default.seed, None);

        let overridden = parse_args(["tme-sim", "--seed", "19"]).unwrap();
        let CliAction::Scripted(overridden) = overridden else {
            panic!("expected scripted options")
        };
        assert_eq!(overridden.seed, Some(19));
    }
}
