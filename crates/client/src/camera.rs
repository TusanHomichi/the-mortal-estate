//! The camera: high above the player and to the south, looking down at an
//! angle with a long lens, so the land reads as a diorama on a table. North
//! is up the screen. The tilt-shift blur is a depth of field focused on the
//! player, opened just enough to keep a band of the screen sharp.

use crate::predict::Me;
use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use bevy::transform::TransformSystems;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn).add_systems(
        PostUpdate,
        (follow, focus).chain().before(TransformSystems::Propagate),
    );
}

/// How much of the screen's height the tilt-shift keeps sharp, from 0 to
/// 1, and whether it's on at all.
#[derive(Resource)]
pub struct Band {
    pub sharp: f32,
    pub on: bool,
}

/// How far the camera stands from what it looks at, in metres.
const DISTANCE: f32 = 32.0;
/// How steeply it looks down.
const PITCH_DEGREES: f32 = 52.0;
/// Its field of view, top to bottom: narrow, as a long lens is.
const FOV_DEGREES: f32 = 24.0;
/// How high above the player's feet it aims.
const AIM_HEIGHT: f32 = 0.8;
/// How wide a blur may get at the edges, in pixels.
const MOST_BLUR: f32 = 12.0;
/// How wide a blur counts as sharp, in pixels.
const SHARP_BLUR: f32 = 1.5;

/// Where the camera stands, from what it looks at.
fn offset() -> Vec3 {
    let pitch = PITCH_DEGREES.to_radians();
    Vec3::new(0.0, pitch.sin(), pitch.cos()) * DISTANCE
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_DEGREES.to_radians(),
            near: 1.0,
            far: 200.0,
            ..default()
        }),
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        ColorGrading::default(),
        DistanceFog::default(),
        AmbientLight::default(),
        DepthOfField {
            mode: DepthOfFieldMode::Gaussian,
            focal_distance: DISTANCE,
            max_circle_of_confusion_diameter: MOST_BLUR,
            ..default()
        },
        Transform::from_translation(offset()).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Keep the player in the middle of the view.
fn follow(me: Res<Me>, time: Res<Time>, mut camera: Single<&mut Transform, With<Camera3d>>) {
    if !me.known {
        return;
    }
    let aim = me.shown + Vec3::Y * AIM_HEIGHT;
    let want = aim + offset();
    // Close enough to be steady, loose enough not to jitter with the body.
    let keep = (-time.delta_secs() * 10.0).exp();
    let at = if camera.translation.distance(want) > 8.0 {
        want
    } else {
        want.lerp(camera.translation, keep)
    };
    **camera = Transform::from_translation(at).looking_at(at - offset(), Vec3::Y);
}

/// Focus on the player, and open the lens just enough that the band of the
/// screen around them stays sharp.
fn focus(
    band: Res<Band>,
    window: Single<&Window>,
    camera: Single<(&mut DepthOfField, &Projection), With<Camera3d>>,
) {
    let (mut dof, projection) = camera.into_inner();
    let Projection::Perspective(lens) = projection else {
        return;
    };
    dof.focal_distance = DISTANCE;
    dof.aperture_f_stops = if band.on && band.sharp < 1.0 {
        f_stops(
            band.sharp.max(0.05),
            lens.fov,
            dof.sensor_height,
            window.physical_height() as f32,
        )
    } else {
        // So narrow nothing blurs.
        f32::MAX
    };
}

/// The aperture that blurs the ground at the edges of a band `sharp` of
/// the screen high by exactly [`SHARP_BLUR`], and more beyond it: the
/// circle of confusion the depth of field draws, solved for the f-stops.
fn f_stops(sharp: f32, fov: f32, sensor: f32, pixels: f32) -> f32 {
    let pitch = PITCH_DEGREES.to_radians();
    let height = DISTANCE * pitch.sin();
    let edge = (sharp * (fov / 2.0).tan()).atan();
    let depth = |down: f32| height * edge.cos() / down.sin();
    let (near, far) = (depth(pitch + edge), depth(pitch - edge));
    let focal_length = 0.5 * sensor / (fov / 2.0).tan();
    let spread = ((DISTANCE - near) / near).max((far - DISTANCE) / far);
    focal_length * focal_length * pixels * spread
        / (sensor * (DISTANCE - focal_length) * SHARP_BLUR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wider_band_takes_a_narrower_aperture() {
        let at = |sharp| f_stops(sharp, FOV_DEGREES.to_radians(), 0.01866, 900.0);
        assert!(at(0.8) > at(0.5));
        assert!(at(0.5) > at(0.2));
        assert!(at(0.2) > 0.0);
    }
}
