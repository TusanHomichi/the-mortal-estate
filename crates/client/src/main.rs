//! `tme`: join a world by the invite its host gave you, and play it.

mod bodies;
mod camera;
mod hud;
mod input;
mod look;
mod net;
mod predict;
mod scene;
mod seen;
mod shot;

use bevy::prelude::*;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tme_wire::{Identity, Invite};

#[derive(Parser)]
#[command(about = "Play The Mortal Estate")]
struct Cli {
    /// The invite link a host gave you: tme://HOST:PORT/WORLD?invite=CODE.
    invite: Invite,
    /// What you're called, the first time you join a world.
    #[arg(long, default_value = "Stranger")]
    name: String,
    /// Your key: who you are, in every world. Made the first time.
    #[arg(long)]
    key: Option<PathBuf>,
    /// The content the world runs (until joining fetches it).
    #[arg(long, default_value = "content")]
    content: PathBuf,
    /// The models and the rest the content draws with.
    #[arg(long, default_value = "assets")]
    assets: PathBuf,
    /// How much of the screen's height the tilt-shift blur keeps sharp,
    /// from 0 to 1: 1 is no blur at all. T turns it off and on in play.
    #[arg(long, default_value_t = 0.7, value_parser = band)]
    band: f32,
    /// Save a picture of the window here once the world has loaded, then
    /// quit: for seeing the look without a person at the keys.
    #[arg(long)]
    screenshot: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("tme: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let content = tme_content::load(&cli.content)?;
    let me = identity(cli.key.as_deref())?;
    let net = net::connect(&cli.invite, me, &cli.name)?;
    let assets = std::fs::canonicalize(&cli.assets)
        .map_err(|e| format!("the assets folder {}: {e}", cli.assets.display()))?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("The Mortal Estate: {}", net.joined.world),
                    resolution: (1600, 900).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: assets.to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .insert_resource(seen::Content(std::sync::Arc::new(content)))
    .insert_resource(net)
    .insert_resource(camera::Band {
        sharp: cli.band,
        on: cli.band < 1.0,
    })
    .add_plugins((
        seen::plugin,
        predict::plugin,
        scene::plugin,
        look::plugin,
        camera::plugin,
        bodies::plugin,
        input::plugin,
        hud::plugin,
    ));
    if let Some(path) = cli.screenshot {
        app.add_plugins(shot::plugin(path));
    }
    app.run();
    Ok(())
}

fn band(s: &str) -> Result<f32, String> {
    match s.parse::<f32>() {
        Ok(b) if (0.0..=1.0).contains(&b) => Ok(b),
        _ => Err("a share of the screen, from 0 to 1".into()),
    }
}

/// The player's identity: the key at `path`, or the usual place for it,
/// made the first time.
fn identity(path: Option<&Path>) -> Result<Identity, String> {
    let path = match path {
        Some(p) => p.to_owned(),
        None => data_dir()?.join("key"),
    };
    if path.exists() {
        return Identity::load(&path).map_err(|e| format!("your key {}: {e}", path.display()));
    }
    if let Some(dir) = path.parent() {
        tme_wire::create_private_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Identity::create(&path).map_err(|e| format!("can't make your key {}: {e}", path.display()))
}

/// Where this game keeps what's yours on this machine.
fn data_dir() -> Result<PathBuf, String> {
    let var = |name: &str| {
        std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let base = if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    base.map(|b| b.join("the-mortal-estate"))
        .ok_or_else(|| "no home folder to keep your key in; give one with --key".to_owned())
}
