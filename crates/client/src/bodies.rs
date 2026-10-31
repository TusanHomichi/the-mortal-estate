//! Bodies: the player's own where prediction puts it, everyone else's a
//! couple of steps behind the world so they move smoothly between
//! observations. Each is its kind's model, animated by what it's doing, and
//! faint when it's on the other side from the player.

use crate::predict::Me;
use crate::seen::{Content, Seen, facing, model};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::HashMap;
use std::time::Duration;
use tme_sim::body::Phase;
use tme_sim::catalogue::{BodyKind, Id};
use tme_sim::{BodyId, Event, STEP_MS, Side};
use tme_wire::BodyView;

pub fn plugin(app: &mut App) {
    app.init_resource::<Rigs>()
        .init_resource::<Faint>()
        .add_systems(Startup, load)
        .add_systems(
            Update,
            (
                rig,
                (spawn, flinch).chain(),
                (place, animate, wobble, fade, telegraph),
            )
                .chain()
                .after(crate::predict::Predicted),
        )
        .add_observer(dressed);
}

/// How far behind the world everyone else is drawn, in steps: enough that
/// the next observation is almost always in hand.
const BEHIND: f64 = 2.0;
/// How long a struck body flinches, in seconds.
const FLINCH: f32 = 0.35;

/// The clip each pose plays, by its name in the model.
const CLIPS: [(Pose, &str); 6] = [
    (Pose::Idle, "Idle"),
    (Pose::Run, "Running_A"),
    (Pose::Attack, "1H_Melee_Attack_Chop"),
    (Pose::Dodge, "Dodge_Forward"),
    (Pose::Rite, "Spellcasting"),
    (Pose::Hit, "Hit_A"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pose {
    Idle,
    Run,
    Attack,
    Dodge,
    Rite,
    Hit,
}

/// Each kind's model, and its animations once they've loaded.
#[derive(Resource, Default)]
struct Rigs {
    models: HashMap<Id<BodyKind>, Handle<Gltf>>,
    /// `None` for a model with no animations: it's moved by hand.
    built: HashMap<Id<BodyKind>, Option<Rig>>,
}

#[derive(Clone)]
struct Rig {
    graph: Handle<AnimationGraph>,
    /// Each pose's node and how long its clip is, in seconds.
    clips: HashMap<Pose, (AnimationNodeIndex, f32)>,
}

/// Faint copies of materials, by the original and the side the body is on.
#[derive(Resource, Default)]
struct Faint(HashMap<(AssetId<StandardMaterial>, Side), Handle<StandardMaterial>>);

/// A body drawn in the scene.
#[derive(Component)]
struct Drawn {
    id: BodyId,
    kind: Id<BodyKind>,
    /// The model under it.
    model: Entity,
    /// What its model's scale is, before any wobble.
    scale: f32,
}

/// The model's animation player, whether its kind's animations are hooked
/// up to it yet, and what it's playing.
#[derive(Component)]
struct Animated {
    player: Entity,
    hooked: bool,
    playing: Option<(Pose, u64)>,
}

/// The model's meshes and their own materials, and whether they're
/// faint now (and for which side).
#[derive(Component)]
struct Skin {
    meshes: Vec<(Entity, Handle<StandardMaterial>)>,
    faint: Option<Option<Side>>,
}

/// Struck: flinching until then, in seconds of the app's clock.
#[derive(Component)]
struct Flinch(f32);

fn load(mut rigs: ResMut<Rigs>, content: Res<Content>, assets: Res<AssetServer>) {
    for (kind, look) in &content.0.looks {
        rigs.models
            .insert(kind.clone(), assets.load(model(&look.model)));
    }
}

/// Build each model's animation graph once it's loaded.
fn rig(
    mut rigs: ResMut<Rigs>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let Rigs { models, built } = &mut *rigs;
    for (kind, handle) in models.iter() {
        if built.contains_key(kind) {
            continue;
        }
        let Some(gltf) = gltfs.get(handle) else {
            continue;
        };
        let found: Vec<(Pose, Handle<AnimationClip>)> = CLIPS
            .iter()
            .filter_map(|(pose, name)| Some((*pose, gltf.named_animations.get(*name)?.clone())))
            .collect();
        if !found.iter().any(|(p, _)| *p == Pose::Idle) {
            built.insert(kind.clone(), None);
            continue;
        }
        let Some(lengths) = found
            .iter()
            .map(|(_, h)| clips.get(h).map(AnimationClip::duration))
            .collect::<Option<Vec<f32>>>()
        else {
            continue;
        };
        let (graph, nodes) = AnimationGraph::from_clips(found.iter().map(|(_, h)| h.clone()));
        let clips = found
            .iter()
            .zip(nodes)
            .zip(lengths)
            .map(|(((pose, _), node), length)| (*pose, (node, length)))
            .collect();
        built.insert(
            kind.clone(),
            Some(Rig {
                graph: graphs.add(graph),
                clips,
            }),
        );
    }
}

/// Everyone in view has a body drawn, and nobody else does.
fn spawn(
    mut commands: Commands,
    seen: Res<Seen>,
    content: Res<Content>,
    assets: Res<AssetServer>,
    drawn: Query<(Entity, &Drawn)>,
) {
    let Some(you) = &seen.you else {
        return;
    };
    let wanted = |id: &BodyId| *id == you.body.id || seen.bodies.contains_key(id);
    let mut have = HashMap::new();
    for (e, d) in &drawn {
        if wanted(&d.id) {
            have.insert(d.id, e);
        } else {
            commands.entity(e).despawn();
        }
    }
    let views = std::iter::once(&you.body).chain(seen.bodies.values().map(|t| &t.view));
    for view in views {
        if have.contains_key(&view.id) {
            continue;
        }
        let Some(look) = content.0.looks.get(&view.kind) else {
            continue;
        };
        let model = commands
            .spawn((
                WorldAssetRoot(
                    assets.load(GltfAssetLabel::Scene(0).from_asset(model(&look.model))),
                ),
                Transform::from_scale(Vec3::splat(look.scale)),
            ))
            .id();
        commands
            .spawn((Transform::default(), Visibility::default()))
            .add_child(model)
            .insert(Drawn {
                id: view.id,
                kind: view.kind.clone(),
                model,
                scale: look.scale,
            });
    }
}

/// A model's scene is in: find its animation player and its meshes.
fn dressed(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    parents: Query<&ChildOf>,
    drawn: Query<(), With<Drawn>>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    let model = ready.entity;
    let Ok(body) = parents.get(model).map(ChildOf::parent) else {
        return;
    };
    if !drawn.contains(body) {
        return;
    }
    let below: Vec<Entity> = children.iter_descendants(model).collect();
    commands.entity(body).insert(Skin {
        meshes: below
            .iter()
            .filter_map(|e| Some((*e, meshes.get(*e).ok()?.0.clone())))
            .collect(),
        faint: None,
    });
    if let Some(player) = below.iter().copied().find(|e| players.contains(*e)) {
        commands.entity(body).insert(Animated {
            player,
            hooked: false,
            playing: None,
        });
    }
}

/// Bodies struck in view flinch.
fn flinch(
    mut commands: Commands,
    seen: Res<Seen>,
    time: Res<Time>,
    drawn: Query<(Entity, &Drawn)>,
) {
    for event in &seen.events {
        let Event::Struck { target, .. } = event else {
            continue;
        };
        for (e, d) in &drawn {
            if d.id == *target {
                commands
                    .entity(e)
                    .insert(Flinch(time.elapsed_secs() + FLINCH));
            }
        }
    }
}

/// Put each body where it is: the player's where prediction says, everyone
/// else's between the observations either side of a moment just past.
fn place(
    seen: Res<Seen>,
    me: Res<Me>,
    time: Res<Time>,
    mut drawn: Query<(&Drawn, &mut Transform)>,
) {
    let Some(you) = &seen.you else {
        return;
    };
    let t = seen.now(time.elapsed_secs_f64()) - BEHIND;
    let turn = 1.0 - (-time.delta_secs() * 18.0).exp();
    for (d, mut transform) in &mut drawn {
        let (at, face) = if d.id == you.body.id {
            (me.shown, me.facing)
        } else if let Some(tracked) = seen.bodies.get(&d.id) {
            (tracked.at(t), tracked.view.facing)
        } else {
            continue;
        };
        transform.translation = at;
        let want = facing(face);
        transform.rotation = transform.rotation.slerp(want, turn);
    }
}

/// What a body's view says it's doing, as a pose, the step that began on,
/// and how long it lasts in seconds (for poses that play once).
fn pose(
    view: &BodyView,
    walking: bool,
    kind: &BodyKind,
    playing: Option<(Pose, u64)>,
    flinching: bool,
) -> (Pose, u64, f32) {
    let secs = |steps: u32| steps as f32 * STEP_MS as f32 / 1000.0;
    match view.phase {
        None if flinching => (Pose::Hit, 0, FLINCH),
        None if walking => (Pose::Run, 0, 0.0),
        None => (Pose::Idle, 0, 0.0),
        Some(Phase::Rite) => (Pose::Rite, view.since, 0.0),
        Some(Phase::Dashing) => (Pose::Dodge, view.since, dodge_length(kind)),
        // A recovery is from whatever began on that step.
        Some(Phase::Recover) if playing == Some((Pose::Dodge, view.since)) => {
            (Pose::Dodge, view.since, dodge_length(kind))
        }
        Some(Phase::WindUp | Phase::Recover) => {
            (Pose::Attack, view.since, secs(kind.attack.length()))
        }
    }
}

fn dodge_length(kind: &BodyKind) -> f32 {
    kind.dodge.as_ref().map_or(0.0, |d| {
        (d.duration + d.recover) as f32 * STEP_MS as f32 / 1000.0
    })
}

/// The view of a drawn body, and whether it's walking.
fn view_of<'a>(seen: &'a Seen, me: &Me, id: BodyId) -> Option<(&'a BodyView, bool)> {
    let you = seen.you.as_ref()?;
    if id == you.body.id {
        return Some((&you.body, me.walking()));
    }
    seen.bodies.get(&id).map(|t| (&t.view, t.view.moving))
}

/// Play what each animated body is doing.
#[allow(clippy::too_many_arguments)]
fn animate(
    mut commands: Commands,
    seen: Res<Seen>,
    me: Res<Me>,
    rigs: Res<Rigs>,
    content: Res<Content>,
    time: Res<Time>,
    mut bodies: Query<(&Drawn, &mut Animated, Option<&Flinch>)>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (d, mut animated, flinch) in &mut bodies {
        let Some((view, walking)) = view_of(&seen, &me, d.id) else {
            continue;
        };
        let Some(Some(rig)) = rigs.built.get(&d.kind) else {
            continue;
        };
        if !animated.hooked {
            // Played from next frame, once these are in.
            commands.entity(animated.player).insert((
                AnimationGraphHandle(rig.graph.clone()),
                AnimationTransitions::new(),
            ));
            animated.hooked = true;
            continue;
        }
        let kind = content.0.catalogue.body(&d.kind);
        let flinching = flinch.is_some_and(|f| f.0 > time.elapsed_secs());
        let (pose, since, length) = pose(view, walking, kind, animated.playing, flinching);
        if animated.playing == Some((pose, since)) {
            continue;
        }
        let Ok((mut player, mut transitions)) = players.get_mut(animated.player) else {
            continue;
        };
        let Some(&(node, clip)) = rig.clips.get(&pose).or_else(|| rig.clips.get(&Pose::Idle))
        else {
            continue;
        };
        animated.playing = Some((pose, since));
        let active = transitions.play(&mut player, node, Duration::from_millis(120));
        if length > 0.0 {
            active.set_speed(clip / length);
        } else {
            active.set_speed(1.0).repeat();
        }
    }
}

/// Bodies with no animations of their own are moved by hand: a bob when
/// they walk, a rearing wind-up, a lunge, a flinch.
fn wobble(
    seen: Res<Seen>,
    me: Res<Me>,
    rigs: Res<Rigs>,
    content: Res<Content>,
    time: Res<Time>,
    bodies: Query<(&Drawn, Option<&Flinch>)>,
    mut models: Query<&mut Transform, Without<Drawn>>,
) {
    let now = time.elapsed_secs();
    let step = seen.now(time.elapsed_secs_f64()) - BEHIND;
    for (d, flinch) in &bodies {
        if !matches!(rigs.built.get(&d.kind), Some(None)) {
            continue;
        }
        let Some((view, walking)) = view_of(&seen, &me, d.id) else {
            continue;
        };
        let Ok(mut model) = models.get_mut(d.model) else {
            continue;
        };
        let kind = content.0.catalogue.body(&d.kind);
        let into = (step - view.since as f64).max(0.0) as f32;
        let (mut rise, mut lean, mut forward, mut stretch) = (0.0, 0.0, 0.0, 1.0);
        match view.phase {
            Some(Phase::WindUp) => {
                let p = (into / kind.attack.wind_up.max(1) as f32).min(1.0);
                lean = -0.35 * p;
                stretch = 1.0 + 0.18 * p;
                rise = 0.03 * (now * 40.0).sin() * p;
            }
            Some(Phase::Recover) => {
                let p = ((into - kind.attack.wind_up as f32) / 3.0).clamp(0.0, 1.0);
                forward = 0.45 * (1.0 - p);
                lean = 0.2 * (1.0 - p);
            }
            _ if walking => rise = 0.05 * (now * 14.0).sin().abs(),
            _ => {}
        }
        if let Some(f) = flinch
            && f.0 > now
        {
            stretch *= 0.85;
        }
        *model = Transform::from_xyz(0.0, rise, forward)
            .with_rotation(Quat::from_rotation_x(lean))
            .with_scale(Vec3::new(1.0, stretch, 1.0) * d.scale);
    }
}

/// Bodies on the other side from the player are faint: the dead to the
/// living (unless their life sees them clearly), and the living to the dead.
#[allow(clippy::too_many_arguments)]
fn fade(
    mut commands: Commands,
    seen: Res<Seen>,
    mut faint: ResMut<Faint>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bodies: Query<(&Drawn, &mut Skin)>,
) {
    let Some(you) = &seen.you else {
        return;
    };
    let mine = you.body.side;
    for (d, mut skin) in &mut bodies {
        let Some(side) = (if d.id == you.body.id {
            Some(mine)
        } else {
            seen.bodies.get(&d.id).map(|t| t.view.side)
        }) else {
            continue;
        };
        let shows_faint = match (mine, side) {
            (Side::Living, Side::Lately) => !you.sees_lately,
            (Side::Lately, Side::Living) => true,
            _ => false,
        };
        let want = shows_faint.then_some(side);
        if skin.faint == Some(want) {
            continue;
        }
        let mut done = true;
        for (mesh, original) in &skin.meshes {
            let handle = match want {
                None => original.clone(),
                Some(side) => match faint.0.get(&(original.id(), side)) {
                    Some(h) => h.clone(),
                    None => {
                        let Some(m) = materials.get(original).map(|m| faded(m, side)) else {
                            done = false;
                            continue;
                        };
                        let h = materials.add(m);
                        faint.0.insert((original.id(), side), h.clone());
                        h
                    }
                },
            };
            let mut e = commands.entity(*mesh);
            e.insert(MeshMaterial3d(handle));
            if want.is_some() {
                e.insert(NotShadowCaster);
            } else {
                e.remove::<NotShadowCaster>();
            }
        }
        if done {
            skin.faint = Some(want);
        }
    }
}

/// A material as a body on `side` looks from the other side: see-through,
/// and lit from within in that side's colour.
fn faded(m: &StandardMaterial, side: Side) -> StandardMaterial {
    let (tint, glow) = match side {
        Side::Lately => (
            Color::srgb(0.55, 1.0, 0.95),
            LinearRgba::rgb(0.05, 0.35, 0.32),
        ),
        Side::Living => (Color::srgb(1.0, 0.85, 0.6), LinearRgba::rgb(0.3, 0.2, 0.08)),
    };
    let base = m.base_color.to_linear();
    let t = tint.to_linear();
    StandardMaterial {
        base_color: Color::LinearRgba(LinearRgba::new(
            base.red * t.red,
            base.green * t.green,
            base.blue * t.blue,
            0.35,
        )),
        emissive: glow,
        alpha_mode: AlphaMode::Blend,
        ..m.clone()
    }
}

/// A swing being wound up shows where it will land, on the ground, so it
/// can be seen coming and stepped out of.
fn telegraph(
    seen: Res<Seen>,
    content: Res<Content>,
    time: Res<Time>,
    drawn: Query<(&Drawn, &Transform)>,
    mut gizmos: Gizmos,
) {
    let Some(you) = &seen.you else {
        return;
    };
    let step = seen.now(time.elapsed_secs_f64()) - BEHIND;
    for (d, transform) in &drawn {
        if d.id == you.body.id {
            continue;
        }
        let Some(tracked) = seen.bodies.get(&d.id) else {
            continue;
        };
        let view = &tracked.view;
        if view.phase != Some(Phase::WindUp) || view.side != you.body.side {
            continue;
        }
        let kind = content.0.catalogue.body(&d.kind);
        let progress =
            ((step - view.since as f64) as f32 / kind.attack.wind_up.max(1) as f32).clamp(0.0, 1.0);
        let reach = (kind.radius + kind.attack.reach) as f32 / 1000.0;
        let half = (kind.attack.arc_cos as f32 / 1024.0)
            .clamp(-1.0, 1.0)
            .acos();
        let centre = transform.translation.with_y(0.05);
        let ahead = facing(view.facing) * Vec3::Z;
        let edge = |a: f32| centre + Quat::from_rotation_y(a) * ahead * reach;
        let colour = Color::srgba(1.0, 0.35, 0.15, 0.35 + 0.6 * progress);
        gizmos.short_arc_3d_between(centre, edge(-half), edge(half), colour);
        gizmos.line(centre, edge(-half), colour);
        gizmos.line(centre, edge(half), colour);
        // The arc fills in as the swing nears.
        gizmos.short_arc_3d_between(
            centre,
            centre + (edge(-half) - centre) * progress,
            centre + (edge(half) - centre) * progress,
            colour,
        );
    }
}
