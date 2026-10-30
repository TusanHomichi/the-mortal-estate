//! The place the player is in: its ground as a slab on a plinth, the way a
//! diorama stands on a table, and what stands on it. Each side has its own
//! ground and its own scenery; only the player's side shows.

use crate::seen::{Content, Seen, ground, model};
use bevy::prelude::*;
use tme_content::Sides;
use tme_sim::Side;
use tme_sim::catalogue::{Id, Place};

pub fn plugin(app: &mut App) {
    app.init_resource::<Drawn>()
        .add_systems(Update, (build, show_side).chain());
}

/// The place drawn now, and the side it's drawn for.
#[derive(Resource, Default)]
struct Drawn {
    place: Option<Id<Place>>,
    side: Option<Side>,
}

/// Everything drawn for a place, so it can all go when the player leaves.
#[derive(Component)]
struct Scenery;

/// Which sides something shows on.
#[derive(Component)]
pub struct OnSide(pub Sides);

impl OnSide {
    fn shows_on(&self, side: Side) -> bool {
        match self.0 {
            Sides::Both => true,
            Sides::Living => side == Side::Living,
            Sides::Lately => side == Side::Lately,
        }
    }
}

/// A light something gives off: windows and lanterns at dusk. The look
/// colours it for the side.
#[derive(Component)]
pub struct Glow;

/// How high above its foot a glowing thing's light hangs, in metres.
const GLOW_HEIGHT: f32 = 1.6;
/// How thick the ground is, and the plinth under it.
const GROUND: f32 = 0.4;
const PLINTH: f32 = 1.2;

#[allow(clippy::too_many_arguments)]
fn build(
    mut commands: Commands,
    mut drawn: ResMut<Drawn>,
    seen: Res<Seen>,
    content: Res<Content>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    old: Query<Entity, With<Scenery>>,
) {
    let Some(you) = &seen.you else {
        return;
    };
    if drawn.place.as_ref() == Some(&you.body.place) {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    drawn.place = Some(you.body.place.clone());
    drawn.side = None;
    let Some(scenery) = content.0.scenery.get(&you.body.place) else {
        return;
    };
    let place = content.0.catalogue.place(&you.body.place);

    let (min, max) = (ground(scenery.bounds.min), ground(scenery.bounds.max));
    let middle = (min + max) / 2.0;
    let size = (max - min).abs();
    let slab = |height: f32, margin: f32| Cuboid::new(size.x + margin, height, size.z + margin);
    commands.spawn((
        Scenery,
        Mesh3d(meshes.add(slab(PLINTH, 1.6))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.11, 0.08),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_translation(middle.with_y(-GROUND - PLINTH / 2.0)),
    ));
    for (sides, colour) in [
        (Sides::Living, scenery.ground_living),
        (Sides::Lately, scenery.ground_lately),
    ] {
        let [r, g, b] = colour;
        commands.spawn((
            Scenery,
            OnSide(sides),
            Mesh3d(meshes.add(slab(GROUND, 0.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(r, g, b),
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_translation(middle.with_y(-GROUND / 2.0)),
        ));
    }

    for prop in &scenery.props {
        let at = ground(prop.at);
        commands.spawn((
            Scenery,
            OnSide(prop.sides),
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(model(&prop.model)))),
            Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_y(prop.turn.to_radians()))
                .with_scale(Vec3::splat(prop.scale)),
        ));
        if prop.glow {
            commands.spawn((
                Scenery,
                OnSide(prop.sides),
                Glow,
                PointLight {
                    range: 10.0,
                    radius: 0.3,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(at.with_y(GLOW_HEIGHT)),
            ));
        }
    }

    // Where the two sides touch: a ring the dead can see, and step into.
    let ring = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.95, 0.9),
        emissive: LinearRgba::rgb(0.6, 3.0, 2.6),
        ..default()
    });
    for threshold in &place.thresholds {
        let r = threshold.radius as f32 / 1000.0;
        let at = ground(threshold.at);
        commands.spawn((
            Scenery,
            OnSide(Sides::Lately),
            Mesh3d(meshes.add(Torus::new(r - 0.08, r))),
            MeshMaterial3d(ring.clone()),
            Transform::from_translation(at.with_y(0.03)),
        ));
        commands.spawn((
            Scenery,
            OnSide(Sides::Lately),
            PointLight {
                color: Color::srgb(0.5, 1.0, 0.92),
                intensity: 120_000.0,
                range: 8.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(at.with_y(1.0)),
        ));
    }
}

/// Show what belongs to the player's side, and hide the rest.
fn show_side(
    mut drawn: ResMut<Drawn>,
    seen: Res<Seen>,
    mut things: Query<(&OnSide, &mut Visibility)>,
    added: Query<(), Added<OnSide>>,
) {
    let Some(you) = &seen.you else {
        return;
    };
    let side = you.body.side;
    if drawn.side == Some(side) && added.is_empty() {
        return;
    }
    drawn.side = Some(side);
    for (on, mut visibility) in &mut things {
        *visibility = if on.shows_on(side) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
