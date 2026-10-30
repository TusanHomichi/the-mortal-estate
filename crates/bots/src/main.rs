//! `tme-bots`: host a world for a while, fill it with bots, and say what a
//! step cost and how long input took.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use tme_bots::{Plan, measure};

#[derive(Parser)]
#[command(about = "Measure a world of The Mortal Estate under headless players")]
struct Cli {
    /// How many bots.
    #[arg(long, default_value_t = 20)]
    bots: usize,
    /// How long they play, in seconds.
    #[arg(long, default_value_t = 30)]
    seconds: u64,
    /// The content folder the world runs.
    #[arg(long, default_value = "content")]
    content: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let plan = Plan {
        bots: cli.bots,
        length: Duration::from_secs(cli.seconds),
        step: Duration::from_millis(u64::from(tme_sim::STEP_MS)),
    };
    match measure(&cli.content, plan).await {
        Ok(report) => {
            print!("{report}");
            if report.failed.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
