//! Restoring from the database: a snapshot and the journal since, or the
//! journal alone after a crash that left no last snapshot.

use std::path::Path;
use std::sync::Arc;
use tme_server::sim::restore;
use tme_server::store::Store;
use tme_sim::world::{Input, Intent, PersonId, Presence, Settings};
use tme_sim::{Catalogue, Dir, World};

fn catalogue() -> Arc<Catalogue> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    Arc::new(tme_content::load(&dir).unwrap().catalogue)
}

/// Play inputs into a world as the server does, journalling the accepted
/// ones; snapshot after `snapshot_at` of them, if given.
fn play(world: &mut World, store: &mut Store, snapshot_at: Option<usize>) {
    let (a, b) = (PersonId(1), PersonId(2));
    let mut inputs = vec![
        Input::Join {
            person: a,
            name: "A".into(),
        },
        Input::Join {
            person: b,
            name: "B".into(),
        },
    ];
    for i in 0..40 {
        let who = if i % 2 == 0 { a } else { b };
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
    inputs.push(Input::Leave { person: b });
    for (n, input) in inputs.into_iter().enumerate() {
        for _ in 0..7 {
            world.step();
        }
        let step = world.step_count();
        if world.apply(&input).is_ok() {
            store.journal(&[(step, input)]).unwrap();
        }
        if snapshot_at == Some(n) {
            store.snapshot(world.state()).unwrap();
        }
    }
}

fn scratch(name: &str) -> Store {
    let path = std::env::temp_dir().join(format!("tme-journal-{}-{name}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    Store::open(&path).unwrap()
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
fn a_snapshot_and_the_journal_since_restore_the_world() {
    for (name, snapshot_at) in [("snapshot", Some(20)), ("crash-before-any-snapshot", None)] {
        let mut store = scratch(name);
        let mut original = World::new(catalogue(), 9, Settings::default());
        play(&mut original, &mut store, snapshot_at);
        everyone_gone(&mut original);
        let restored = restore(
            &mut store,
            || World::new(catalogue(), 9, Settings::default()),
            |state| World::load(catalogue(), state).map_err(|e| e.to_string()),
        )
        .unwrap();
        assert_eq!(restored.state(), original.state(), "{name}");
        // The restore's leaving is journalled: a second restore agrees.
        let again = restore(
            &mut store,
            || World::new(catalogue(), 9, Settings::default()),
            |state| World::load(catalogue(), state).map_err(|e| e.to_string()),
        )
        .unwrap();
        assert_eq!(again.state(), original.state(), "{name}, twice");
    }
}

#[test]
fn a_database_from_another_layout_is_refused() {
    let path = std::env::temp_dir().join(format!("tme-journal-{}-layout.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    drop(Store::open(&path).unwrap());
    // Say it was saved in layout 0, behind the store's back.
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("UPDATE meta SET value = 0 WHERE key = 'format'", [])
        .unwrap();
    drop(conn);
    let refused = Store::open(&path).err().expect("refused").to_string();
    assert!(refused.contains("layout 0"), "{refused}");
}
