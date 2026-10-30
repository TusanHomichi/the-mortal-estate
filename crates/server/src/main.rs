//! `tme-server`: make a world, print its invite, run it.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;
use tme_server::Server;
use tme_server::folder::{Folder, NewWorld};

#[derive(Parser)]
#[command(about = "Host a world of The Mortal Estate")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Make a new world in a folder.
    Init {
        /// The folder to make it in.
        world: PathBuf,
        /// The world's name, as players see it.
        #[arg(long)]
        name: String,
        /// The content folder it runs.
        #[arg(long, default_value = "content")]
        content: PathBuf,
        /// Where to listen.
        #[arg(long, default_value = "0.0.0.0:7420")]
        bind: String,
        /// Where players reach it, as its invites say.
        #[arg(long, default_value = "127.0.0.1:7420")]
        address: String,
    },
    /// Print the world's invite link: the one thing that prints its code.
    Invite { world: PathBuf },
    /// Run the world until interrupted.
    Run { world: PathBuf },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("tme-server: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Init {
            world,
            name,
            content,
            bind,
            address,
        } => {
            let folder = Folder::create(
                &world,
                NewWorld {
                    name: &name,
                    content: &content,
                    bind: &bind,
                    address: &address,
                },
            )?;
            println!("made {name} in {}", folder.path.display());
            println!(
                "`tme-server invite {}` prints the link that brings someone in",
                folder.path.display()
            );
        }
        Command::Invite { world } => println!("{}", Folder::open(&world)?.invite()),
        Command::Run { world } => {
            let server = Server::start(&world)?;
            println!("listening on {}", server.local);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            // Asked to stop, or the world stopped on its own because it
            // couldn't save: either way, stop says how it went.
            let asked = rt.block_on(async {
                tokio::select! {
                    asked = asked_to_stop() => asked.map(|()| true),
                    () = server.stopped() => Ok(false),
                }
            })?;
            if asked {
                println!("saving the world");
            }
            server.stop()?;
            if !asked {
                return Err("the world stopped".into());
            }
        }
    }
    Ok(())
}

/// Until the host asks the world to stop: Ctrl-C, or the terminate signal a
/// service manager sends.
async fn asked_to_stop() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate())?;
        tokio::select! {
            r = tokio::signal::ctrl_c() => r,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}
