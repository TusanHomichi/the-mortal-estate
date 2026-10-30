//! A picture of the window once the world has loaded, then quit: for
//! seeing the look without anyone at the keys.

use crate::predict::Me;
use bevy::asset::RecursiveDependencyLoadState;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use std::path::PathBuf;

/// How many frames to let settle after loading: the blur, the bloom and
/// the shadows all take a few.
const SETTLE: u32 = 30;

pub fn plugin(path: PathBuf) -> impl Plugin {
    move |app: &mut App| {
        app.insert_resource(Shot {
            path: path.clone(),
            settled: 0,
            taken: false,
        })
        .add_systems(Last, take);
    }
}

#[derive(Resource)]
struct Shot {
    path: PathBuf,
    settled: u32,
    taken: bool,
}

fn take(
    mut commands: Commands,
    mut shot: ResMut<Shot>,
    me: Res<Me>,
    assets: Res<AssetServer>,
    roots: Query<&WorldAssetRoot>,
) {
    if shot.taken || !me.known {
        return;
    }
    // Loaded, or failed: either way it's as loaded as it gets.
    let loading = roots.iter().any(|r| {
        matches!(
            assets.recursive_dependency_load_state(r.id()),
            RecursiveDependencyLoadState::NotLoaded | RecursiveDependencyLoadState::Loading
        )
    });
    if loading {
        shot.settled = 0;
        return;
    }
    shot.settled += 1;
    if shot.settled < SETTLE {
        return;
    }
    shot.taken = true;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(shot.path.clone()))
        .observe(
            |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                exit.write(AppExit::Success);
            },
        );
}
