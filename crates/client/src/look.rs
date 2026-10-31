//! Lantern: the land at dusk, a low warm sun throwing long shadows, cool
//! shade, glowing windows, haze at the edges. Lately is the same scene lit
//! from below in cold teal, its colour paler.

use crate::scene::Glow;
use crate::seen::Seen;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use tme_sim::Side;

pub fn plugin(app: &mut App) {
    app.insert_resource(ClearColor(LANTERN.haze))
        .add_systems(Startup, setup)
        .add_systems(Update, follow_side);
}

/// The sun, low in the south-west.
#[derive(Component)]
struct Sun;

/// Light from below, which only Lately has.
#[derive(Component)]
struct Under;

/// One side's lighting.
struct Lighting {
    sun: Color,
    sun_lux: f32,
    under: Color,
    under_lux: f32,
    shade: Color,
    shade_brightness: f32,
    glow: Color,
    glow_lumens: f32,
    haze: Color,
    saturation: f32,
    temperature: f32,
    exposure: f32,
}

const LANTERN: Lighting = Lighting {
    sun: Color::srgb(1.0, 0.82, 0.62),
    sun_lux: 8_000.0,
    under: Color::BLACK,
    under_lux: 0.0,
    shade: Color::srgb(0.48, 0.55, 0.92),
    shade_brightness: 900.0,
    glow: Color::srgb(1.0, 0.62, 0.3),
    glow_lumens: 90_000.0,
    haze: Color::srgb(0.2, 0.15, 0.2),
    saturation: 0.9,
    temperature: 0.0,
    exposure: -0.2,
};

const LATELY: Lighting = Lighting {
    sun: Color::srgb(0.55, 0.68, 0.75),
    sun_lux: 1_800.0,
    under: Color::srgb(0.25, 0.95, 0.85),
    under_lux: 4_000.0,
    shade: Color::srgb(0.35, 0.62, 0.66),
    shade_brightness: 450.0,
    glow: Color::srgb(0.5, 0.95, 0.92),
    glow_lumens: 50_000.0,
    haze: Color::srgb(0.07, 0.17, 0.19),
    saturation: 0.6,
    temperature: -0.12,
    exposure: 0.0,
};

/// Haze starts a little past the player and hides the far edge of the
/// view, metres from the camera.
const HAZE_FROM: f32 = 36.0;
const HAZE_TO: f32 = 70.0;

fn setup(mut commands: Commands) {
    commands.spawn((
        Sun,
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 2,
            minimum_distance: 1.0,
            first_cascade_far_bound: 45.0,
            maximum_distance: 90.0,
            ..default()
        }
        .build(),
        // From the south-west, under thirty degrees up the sky.
        Transform::from_xyz(-10.0, 6.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Under,
        DirectionalLight {
            shadow_maps_enabled: false,
            ..default()
        },
        // From below and to the south, so it catches what faces the camera.
        Transform::from_xyz(2.0, -3.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Light the scene for the side the player is on, when that changes.
#[allow(clippy::type_complexity)]
fn follow_side(
    seen: Res<Seen>,
    mut lit: Local<Option<Side>>,
    mut clear: ResMut<ClearColor>,
    camera: Single<(&mut DistanceFog, &mut ColorGrading, &mut AmbientLight)>,
    mut lights: ParamSet<(
        Single<&mut DirectionalLight, With<Sun>>,
        Single<&mut DirectionalLight, With<Under>>,
        Query<&mut PointLight, With<Glow>>,
    )>,
    added: Query<(), Added<Glow>>,
) {
    let Some(you) = &seen.you else {
        return;
    };
    let side = you.body.side;
    if *lit == Some(side) && added.is_empty() {
        return;
    }
    *lit = Some(side);
    let l = match side {
        Side::Living => &LANTERN,
        Side::Lately => &LATELY,
    };
    clear.0 = l.haze;
    let (mut fog, mut grading, mut ambient) = camera.into_inner();
    *fog = DistanceFog {
        color: l.haze,
        directional_light_color: l.sun.with_alpha(0.25),
        directional_light_exponent: 12.0,
        falloff: FogFalloff::Linear {
            start: HAZE_FROM,
            end: HAZE_TO,
        },
    };
    grading.global.post_saturation = l.saturation;
    grading.global.temperature = l.temperature;
    grading.global.exposure = l.exposure;
    ambient.color = l.shade;
    ambient.brightness = l.shade_brightness;
    {
        let mut sun = lights.p0();
        sun.color = l.sun;
        sun.illuminance = l.sun_lux;
    }
    {
        let mut under = lights.p1();
        under.color = l.under;
        under.illuminance = l.under_lux;
    }
    for mut glow in &mut lights.p2() {
        glow.color = l.glow;
        glow.intensity = l.glow_lumens;
    }
}
