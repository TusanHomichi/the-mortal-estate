//! The player's hands: keys and mouse, or a gamepad, turned into intents.
//!
//! Move with WASD, the arrows or the left stick. Swing with the left mouse
//! button, J, or the gamepad's south button, toward the cursor or the right
//! stick. Dodge with space, the right mouse button or the east button, the
//! way you're moving. Use what's here with E or the west button. T turns
//! the tilt-shift blur off and on; Escape quits.

use crate::camera::Band;
use crate::net::Net;
use crate::predict::{Me, Predicted};
use crate::seen::Seen;
use bevy::prelude::*;
use tme_sim::{Dir, Intent, Vec2};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (steer, act, settings).before(Predicted));
}

/// How far a stick has to be pushed to count.
const DEAD_ZONE: f32 = 0.2;

/// A direction as the world takes it, from x east and y north, each from
/// -1 to 1.
fn dir(v: bevy::math::Vec2) -> Dir {
    let unit = f32::from(Dir::UNIT);
    Dir::new((v.x * unit).round() as i32, (v.y * unit).round() as i32)
}

/// Where the keys or the stick are steering: keys always all the way, a
/// stick as far as it's pushed.
fn held(keys: &ButtonInput<KeyCode>, pads: &Query<&Gamepad>) -> Dir {
    let pressed = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let mut v = bevy::math::Vec2::ZERO;
    if pressed(KeyCode::KeyW, KeyCode::ArrowUp) {
        v.y += 1.0;
    }
    if pressed(KeyCode::KeyS, KeyCode::ArrowDown) {
        v.y -= 1.0;
    }
    if pressed(KeyCode::KeyD, KeyCode::ArrowRight) {
        v.x += 1.0;
    }
    if pressed(KeyCode::KeyA, KeyCode::ArrowLeft) {
        v.x -= 1.0;
    }
    if v != bevy::math::Vec2::ZERO {
        return dir(v.normalize());
    }
    for pad in pads {
        let stick = pad.left_stick();
        if stick.length() > DEAD_ZONE {
            return dir(stick.clamp_length_max(1.0));
        }
    }
    Dir::NONE
}

fn steer(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    seen: Res<Seen>,
    mut me: ResMut<Me>,
    mut net: ResMut<Net>,
    time: Res<Time>,
) {
    if seen.you.is_none() || seen.closed.is_some() {
        return;
    }
    let dir = held(&keys, &pads);
    me.steer(dir, &mut net, time.elapsed_secs_f64());
}

/// Where the cursor points on the ground, in the world's millimetres.
fn cursor_on_ground(window: &Window, camera: &Camera, at: &GlobalTransform) -> Option<Vec2> {
    let ray = camera
        .viewport_to_world(at, window.cursor_position()?)
        .ok()?;
    let along = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
    let p = ray.get_point(along);
    Some(Vec2::new(
        (p.x * 1000.0).round() as i32,
        (-p.z * 1000.0).round() as i32,
    ))
}

#[allow(clippy::too_many_arguments)]
fn act(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    seen: Res<Seen>,
    mut me: ResMut<Me>,
    mut net: ResMut<Net>,
    time: Res<Time>,
) {
    if seen.you.is_none() || seen.closed.is_some() {
        return;
    }
    let now = time.elapsed_secs_f64();
    let pad = |b: GamepadButton| pads.iter().any(|p| p.just_pressed(b));
    let aim = || {
        for p in &pads {
            let stick = p.right_stick();
            if stick.length() > DEAD_ZONE {
                return dir(stick.normalize());
            }
        }
        let (camera, at) = *camera;
        cursor_on_ground(&window, camera, at)
            .filter(|_| mouse.just_pressed(MouseButton::Left))
            .map_or(Dir::NONE, |target| Dir::toward(me.pos, target))
    };
    if mouse.just_pressed(MouseButton::Left)
        || keys.just_pressed(KeyCode::KeyJ)
        || pad(GamepadButton::South)
        || pad(GamepadButton::RightTrigger2)
    {
        let aim = aim();
        me.commit(Intent::Attack(aim), &mut net, now);
    } else if mouse.just_pressed(MouseButton::Right)
        || keys.just_pressed(KeyCode::Space)
        || pad(GamepadButton::East)
    {
        let way = held(&keys, &pads);
        me.commit(Intent::Dodge(way), &mut net, now);
    } else if keys.just_pressed(KeyCode::KeyE) || pad(GamepadButton::West) {
        me.commit(Intent::Use, &mut net, now);
    }
}

fn settings(
    keys: Res<ButtonInput<KeyCode>>,
    mut band: ResMut<Band>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        band.on = !band.on;
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
