//! What's written on the screen: your health and side, your tardies, how a
//! rite is going, what just happened, and names and wounds over the bodies
//! around you.

use crate::seen::{Content, Seen};
use bevy::prelude::*;
use std::collections::HashMap;
use tme_sim::body::Doing;
use tme_sim::catalogue::Place;
use tme_sim::{BodyId, Side};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, (status, rite, news, hint, labels));
}

/// How long a line of news stays, in seconds.
const NEWS_FOR: f64 = 8.0;
/// How high over a body's feet its label floats, in metres.
const LABEL_HEIGHT: f32 = 2.3;

#[derive(Component)]
struct Health;
#[derive(Component)]
struct HealthText;
#[derive(Component)]
struct Status;
#[derive(Component)]
struct Rite;
#[derive(Component)]
struct RiteFill;
#[derive(Component)]
struct News;
#[derive(Component)]
struct Hint;

/// A name and health bar floating over someone in view.
#[derive(Component)]
struct Label {
    id: BodyId,
    fill: Entity,
}

fn text(size: f32, colour: Color) -> (TextFont, TextColor) {
    (TextFont::from_font_size(size), TextColor(colour))
}

const INK: Color = Color::srgb(0.96, 0.92, 0.84);
const DIM: Color = Color::srgba(0.96, 0.92, 0.84, 0.75);
const PANEL: Color = Color::srgba(0.06, 0.05, 0.07, 0.55);

fn setup(mut commands: Commands) {
    // Bottom left: health, side, tardies.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(PANEL),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(14.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                ))
                .with_child((
                    Health,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.75, 0.2, 0.15)),
                ));
            panel.spawn((HealthText, Text::new(""), text(16.0, INK)));
            panel.spawn((Status, Text::new(""), text(15.0, DIM)));
        });
    // Bottom middle: a rite's progress, while one's under way.
    commands
        .spawn((
            Rite,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(90.0),
                left: Val::Percent(35.0),
                width: Val::Percent(30.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|rite| {
            rite.spawn((Text::new("Turning up"), text(18.0, INK)));
            rite.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ))
            .with_child((
                RiteFill,
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.45, 0.95, 0.88)),
            ));
        });
    // Top left: what just happened.
    commands.spawn((
        News,
        Text::new(""),
        text(16.0, INK),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            max_width: Val::Px(520.0),
            ..default()
        },
    ));
    // Top middle: what can be done here.
    commands.spawn((
        Hint,
        Text::new(""),
        text(18.0, INK),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Percent(25.0),
            width: Val::Percent(50.0),
            ..default()
        },
    ));
}

#[allow(clippy::type_complexity)]
fn status(
    seen: Res<Seen>,
    content: Res<Content>,
    mut bar: Single<&mut Node, With<Health>>,
    mut texts: ParamSet<(
        Single<&mut Text, With<HealthText>>,
        Single<&mut Text, With<Status>>,
    )>,
) {
    let Some(you) = &seen.you else {
        return;
    };
    let b = &you.body;
    let share = b.health.max(0) as f32 / b.max_health.max(1) as f32;
    bar.width = Val::Percent(share * 100.0);
    texts.p0().0 = format!("{} / {}", b.health.max(0), b.max_health);
    let side = match b.side {
        Side::Living => "Living",
        Side::Lately => "In Lately",
    };
    let marks = &content.0.catalogue.marks;
    let tardies: Vec<&str> = you
        .tardies
        .iter()
        .filter_map(|t| marks.get(t).map(|m| m.name.as_str()))
        .collect();
    let mut line = format!(
        "{side}, {}",
        match you.deaths {
            0 => "never died".to_owned(),
            1 => "died once".to_owned(),
            n => format!("died {n} times"),
        }
    );
    if !tardies.is_empty() {
        line.push_str(&format!("\nTardies: {}", tardies.join(", ")));
    }
    texts.p1().0 = line;
}

fn rite(
    seen: Res<Seen>,
    time: Res<Time>,
    mut panel: Single<&mut Visibility, With<Rite>>,
    mut fill: Single<&mut Node, With<RiteFill>>,
) {
    let turning = seen.you.as_ref().and_then(|y| {
        let action = y.action.as_ref()?;
        let Doing::TurnUp { length, .. } = action.doing else {
            return None;
        };
        let into = seen.now(time.elapsed_secs_f64()) - action.started as f64;
        Some((into / f64::from(length.max(1))).clamp(0.0, 1.0) as f32)
    });
    **panel = if turning.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    fill.width = Val::Percent(turning.unwrap_or(0.0) * 100.0);
}

fn news(seen: Res<Seen>, time: Res<Time>, mut text: Single<&mut Text, With<News>>) {
    let now = time.elapsed_secs_f64();
    let lines: Vec<&str> = seen
        .news
        .iter()
        .filter(|(at, _)| now - at < NEWS_FOR)
        .map(|(_, l)| l.as_str())
        .collect();
    text.0 = lines.join("\n");
}

/// What the player might do next, where it isn't obvious.
fn hint(seen: Res<Seen>, content: Res<Content>, mut text: Single<&mut Text, With<Hint>>) {
    text.0 = match (&seen.closed, &seen.you) {
        (Some(why), _) => format!("The world let go of you: {why}"),
        (None, None) => "Arriving".into(),
        (None, Some(you)) if you.body.side == Side::Lately && you.action.is_none() => {
            let place: &Place = content.0.catalogue.place(&you.body.place);
            let near = place.thresholds.iter().any(|t| {
                you.body.pos.distance_sq(t.at) <= i64::from(t.radius) * i64::from(t.radius)
            });
            if near {
                "Press E to turn up.".into()
            } else {
                "You're in Lately. Find where the sides touch to turn up again.".into()
            }
        }
        _ => String::new(),
    };
}

/// Names over people, and a bar over anyone wounded.
#[allow(clippy::too_many_arguments)]
fn labels(
    mut commands: Commands,
    seen: Res<Seen>,
    time: Res<Time>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut labels: Query<(Entity, &Label, &mut Node, &mut Visibility)>,
    mut fills: Query<&mut Node, Without<Label>>,
) {
    let (camera, at) = *camera;
    let t = seen.now(time.elapsed_secs_f64()) - 2.0;
    let mut have: HashMap<BodyId, Entity> = HashMap::new();
    for (e, label, ..) in &labels {
        if seen.bodies.contains_key(&label.id) {
            have.insert(label.id, e);
        } else {
            commands.entity(e).despawn();
        }
    }
    for (id, tracked) in &seen.bodies {
        if have.contains_key(id) {
            continue;
        }
        let name = tracked.view.name.clone().unwrap_or_default();
        let fill = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.75, 0.2, 0.15)),
            ))
            .id();
        let bar = commands
            .spawn((
                Node {
                    width: Val::Px(60.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ))
            .add_child(fill)
            .id();
        let name = commands.spawn((Text::new(name), text(14.0, INK))).id();
        commands
            .spawn((
                Label { id: *id, fill },
                Node {
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    width: Val::Px(120.0),
                    ..default()
                },
                Visibility::Hidden,
            ))
            .add_children(&[name, bar]);
    }
    for (_, label, mut node, mut visibility) in &mut labels {
        let Some(tracked) = seen.bodies.get(&label.id) else {
            continue;
        };
        let v = &tracked.view;
        let wounded = v.health < v.max_health;
        let over = tracked.at(t) + Vec3::Y * LABEL_HEIGHT;
        match camera.world_to_viewport(at, over) {
            Ok(p) if v.name.is_some() || wounded => {
                node.left = Val::Px(p.x - 60.0);
                node.top = Val::Px(p.y - 24.0);
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
        if let Ok(mut fill) = fills.get_mut(label.fill) {
            fill.width = Val::Percent(v.health.max(0) as f32 / v.max_health.max(1) as f32 * 100.0);
        }
    }
}
