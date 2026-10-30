//! Restoring from the database: the latest snapshot, the journal since and
//! every step completed after, played on the content and settings they
//! were played with; then the world moves onto the host's content and
//! settings.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tme_server::sim::{Restored, restore};
use tme_server::store::{ContentId, Store};
use tme_sim::world::{Input, Intent, PersonId, Presence, Settings};
use tme_sim::{Catalogue, Dir, Id, World};

const A: PersonId = PersonId(1);
const B: PersonId = PersonId(2);

fn catalogue() -> Arc<Catalogue> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    Arc::new(tme_content::load(&dir).unwrap().catalogue)
}

fn settings(harm: bool) -> Settings {
    Settings {
        harm_between_players: harm,
    }
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tme-journal-{}-{name}.db", std::process::id()));
    for end in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{end}", path.display()));
    }
    path
}

/// A world restored from `store` on the catalogue and settings given.
fn start(store: &mut Store, catalogue: &Arc<Catalogue>, harm: bool) -> Restored {
    restore(store, Arc::clone(catalogue), 9, settings(harm)).unwrap()
}

/// Play into `world` as the server does: each accepted input journalled
/// with the steps completed, a snapshot after `snapshot_at` inputs, if
/// given, and then a while with no inputs at all. Nothing saves at the
/// end, as if the server crashed.
fn play(world: &mut World, store: &mut Store, content: ContentId, snapshot_at: Option<usize>) {
    let mut inputs = vec![
        Input::Join {
            person: A,
            name: "A".into(),
        },
        Input::Join {
            person: B,
            name: "B".into(),
        },
    ];
    for i in 0..40 {
        let who = if i % 2 == 0 { A } else { B };
        let dir = Dir::new(400 + i * 13 % 300, 500 - i * 29 % 400);
        let intent = if i % 4 == 0 {
            Intent::Attack(dir)
        } else {
            Intent::Move(dir)
        };
        inputs.push(Input::Act {
            person: who,
            intent,
        });
    }
    inputs.push(Input::Leave { person: B });
    for (n, input) in inputs.into_iter().enumerate() {
        for _ in 0..7 {
            world.step();
            store.journal(&[], world.step_count()).unwrap();
        }
        let step = world.step_count();
        if world.apply(&input).is_ok() {
            store.journal(&[(step, input)], step).unwrap();
        }
        if snapshot_at == Some(n) {
            store.snapshot(world.state(), content).unwrap();
        }
    }
    // A walks on, with nobody saying anything, for half a minute.
    for _ in 0..600 {
        world.step();
        store.journal(&[], world.step_count()).unwrap();
    }
}

/// The world as a restore leaves it: whoever was here has left.
fn everyone_gone(world: &mut World) {
    let here: Vec<PersonId> = world
        .state()
        .people
        .iter()
        .filter(|(_, p)| p.presence == Presence::Here)
        .map(|(id, _)| *id)
        .collect();
    for person in here {
        world.apply(&Input::Leave { person }).unwrap();
    }
}

#[test]
fn every_step_saved_is_restored_after_a_crash() {
    let catalogue = catalogue();
    for (name, snapshot_at) in [("after-a-snapshot", Some(20)), ("before-any", None)] {
        let path = scratch(name);
        let mut store = Store::open(&path).unwrap();
        let Restored {
            world: mut original,
            content,
        } = start(&mut store, &catalogue, false);
        play(&mut original, &mut store, content, snapshot_at);
        everyone_gone(&mut original);
        drop(store);
        let mut store = Store::open(&path).unwrap();
        let restored = start(&mut store, &catalogue, false).world;
        assert_eq!(restored.state(), original.state(), "{name}");
        // The restore's leaving is saved: a second restore agrees.
        let again = start(&mut store, &catalogue, false).world;
        assert_eq!(again.state(), original.state(), "{name}, twice");
    }
}

#[test]
fn the_journal_replays_on_the_content_it_was_played_on() {
    let played_on = catalogue();
    let path = scratch("content");
    let mut store = Store::open(&path).unwrap();
    let Restored {
        world: mut original,
        content,
    } = start(&mut store, &played_on, false);
    play(&mut original, &mut store, content, None);
    everyone_gone(&mut original);

    // The host's content has changed since: people walk twice as fast.
    let mut faster = (*played_on).clone();
    faster
        .bodies
        .get_mut(&Id::new("person"))
        .expect("people")
        .speed *= 2;
    let faster = Arc::new(faster);
    let restored = start(&mut store, &faster, false).world;
    assert_eq!(
        restored.state(),
        original.state(),
        "what was played is replayed as it was played"
    );
    assert_eq!(**restored.catalogue(), *faster, "and runs on the new");
    // From now on, the new content is what's replayed.
    let mut moved = restored;
    moved
        .apply(&Input::Join {
            person: A,
            name: "A".into(),
        })
        .unwrap();
    let at = moved.step_count();
    moved
        .apply(&Input::Act {
            person: A,
            intent: Intent::Move(Dir::new(-1024, 0)),
        })
        .unwrap();
    store
        .journal(
            &[
                (
                    at,
                    Input::Join {
                        person: A,
                        name: "A".into(),
                    },
                ),
                (
                    at,
                    Input::Act {
                        person: A,
                        intent: Intent::Move(Dir::new(-1024, 0)),
                    },
                ),
            ],
            at,
        )
        .unwrap();
    for _ in 0..20 {
        moved.step();
    }
    store.journal(&[], moved.step_count()).unwrap();
    everyone_gone(&mut moved);
    assert_eq!(
        start(&mut store, &faster, false).world.state(),
        moved.state()
    );
}

#[test]
fn content_the_saved_world_does_not_fit_is_refused_before_anything_is_written() {
    let catalogue = catalogue();
    let path = scratch("misfit");
    let mut store = Store::open(&path).unwrap();
    let Restored { mut world, content } = start(&mut store, &catalogue, false);
    play(&mut world, &mut store, content, Some(10));
    let before = store.load().unwrap();

    // The hall's lair is gone from the host's content.
    let mut misfit = (*catalogue).clone();
    for place in misfit.places.values_mut() {
        place.lairs.clear();
    }
    let refused = restore(&mut store, Arc::new(misfit), 9, settings(false))
        .err()
        .expect("refused");
    assert!(refused.contains("lairs"), "{refused}");
    assert_eq!(store.load().unwrap(), before, "nothing was written");
}

#[test]
fn the_hosts_settings_apply_from_each_start_and_history_keeps_its_own() {
    let catalogue = catalogue();
    let path = scratch("settings");
    let mut store = Store::open(&path).unwrap();
    // Harm between players on: A swings at B, who stands beside A.
    let Restored { mut world, .. } = start(&mut store, &catalogue, true);
    for person in [A, B] {
        let join = Input::Join {
            person,
            name: format!("{person:?}"),
        };
        world.apply(&join).unwrap();
        store.journal(&[(0, join)], 0).unwrap();
    }
    let body = |w: &World, p: PersonId| w.person(p).unwrap().body.clone();
    let swing = Input::Act {
        person: A,
        intent: Intent::Attack(Dir::toward(body(&world, A).at.pos, body(&world, B).at.pos)),
    };
    world.apply(&swing).unwrap();
    store.journal(&[(0, swing)], 0).unwrap();
    for _ in 0..40 {
        world.step();
    }
    store.journal(&[], world.step_count()).unwrap();
    assert!(
        body(&world, B).health < catalogue.body(&Id::new("person")).health,
        "B was hurt"
    );
    everyone_gone(&mut world);

    // The host turns harm off: the swing that landed still landed.
    let restored = start(&mut store, &catalogue, false).world;
    assert!(!restored.state().settings.harm_between_players);
    let mut expected = world.state().clone();
    expected.settings = settings(false);
    assert_eq!(*restored.state(), expected);
    // And on again, through another start.
    let again = start(&mut store, &catalogue, true).world;
    assert!(again.state().settings.harm_between_players);
}

#[test]
fn a_database_from_another_layout_is_refused() {
    let path = scratch("layout");
    drop(Store::open(&path).unwrap());
    // Say it was saved in layout 0, behind the store's back.
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("UPDATE meta SET value = 0 WHERE key = 'format'", [])
        .unwrap();
    drop(conn);
    let refused = Store::open(&path).err().expect("refused").to_string();
    assert!(refused.contains("layout 0"), "{refused}");
}
