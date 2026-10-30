//! A hosted world, end to end over real sockets: joining by invite from two
//! clients, who's turned away, and the world surviving a restart. The world
//! steps every millisecond here, so a test's minute of play is a moment.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tme_link::{LinkError, Receiver, Sender, join};
use tme_server::folder::{Folder, NewWorld};
use tme_server::{Options, Server};
use tme_sim::{Dir, Side, Vec2};
use tme_wire::{Identity, Invite, Observation, ServerMsg, Turned};

fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

/// A new world in a scratch folder, listening on a free port.
fn new_world(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tme-hosting-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Folder::create(
        &dir,
        NewWorld {
            name: "Lattermead",
            content: &content(),
            bind: "127.0.0.1:0",
            address: "unused:0",
        },
    )
    .expect("a new world");
    dir
}

fn fast() -> Options {
    Options {
        step: Duration::from_millis(1),
    }
}

/// The server's invite, pointed at where it's actually listening.
fn invite(server: &Server) -> Invite {
    Invite {
        address: server.local.to_string(),
        ..server.invite().clone()
    }
}

/// The next observation, skipping anything else.
async fn observe(rx: &mut Receiver) -> Observation {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(10), rx.next())
            .await
            .expect("the world keeps talking")
            .expect("the world is still there")
            .expect("readable");
        if let ServerMsg::Observe(o) = msg {
            return o;
        }
    }
}

/// Steer toward each waypoint in turn, by what the world says we observe.
async fn walk(tx: &mut Sender, rx: &mut Receiver, waypoints: &[Vec2]) -> Observation {
    let mut o = observe(rx).await;
    for &to in waypoints {
        for _ in 0..20_000 {
            let at = o.you.body.pos;
            if at.distance_sq(to) < 500 * 500 {
                break;
            }
            tx.act(tme_sim::Intent::Move(Dir::toward(at, to)))
                .await
                .unwrap();
            o = observe(rx).await;
        }
        assert!(
            o.you.body.pos.distance_sq(to) < 500 * 500,
            "stuck at {:?} on the way to {to:?}",
            o.you.body.pos
        );
    }
    stop(tx, rx).await
}

/// Stand still: the first observation after the world has applied it.
async fn stop(tx: &mut Sender, rx: &mut Receiver) -> Observation {
    let seq = tx.act(tme_sim::Intent::Move(Dir::NONE)).await.unwrap();
    applied(rx, seq).await
}

/// The first observation that acknowledges intent `seq`.
async fn applied(rx: &mut Receiver, seq: u32) -> Observation {
    loop {
        let o = observe(rx).await;
        if o.ack >= seq {
            return o;
        }
    }
}

fn m(x: f64, y: f64) -> Vec2 {
    Vec2::new((x * 1000.0) as i32, (y * 1000.0) as i32)
}

#[tokio::test(flavor = "multi_thread")]
async fn two_clients_join_by_invite_and_see_each_other() {
    let dir = new_world("two");
    let server = Server::start_with(&dir, fast()).unwrap();
    let link = invite(&server);
    let (alice, bob) = (Identity::generate(), Identity::generate());
    let (a, mut a_tx, mut a_rx) = join(&link, &alice, "Alice").await.unwrap();
    let (b, _b_tx, mut b_rx) = join(&link, &bob, "Bob").await.unwrap();
    assert_eq!(a.world, "Lattermead");
    assert_ne!(a.you, b.you);
    // Alice walks; Bob sees her come into view, moving, by name.
    a_tx.act(tme_sim::Intent::Move(Dir::NORTH)).await.unwrap();
    let mut saw = None;
    for _ in 0..200 {
        let o = observe(&mut b_rx).await;
        if let Some(v) = o
            .seen
            .iter()
            .find(|v| v.id == tme_sim::BodyId::Person(a.you))
        {
            saw = Some(v.clone());
            if v.moving {
                break;
            }
        }
    }
    let saw = saw.expect("Bob sees Alice");
    assert_eq!(saw.name.as_deref(), Some("Alice"));
    // And Alice's own observations acknowledge her intent, and show it.
    let mine = applied(&mut a_rx, 1).await;
    assert!(mine.you.body.moving);
    server.stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn someone_new_needs_the_invite_and_someone_returning_does_not() {
    let dir = new_world("invite");
    let server = Server::start_with(&dir, fast()).unwrap();
    let link = invite(&server);
    let me = Identity::generate();
    let uninvited = Invite {
        code: None,
        ..link.clone()
    };
    let wrong = Invite {
        code: Some("nope".into()),
        ..link.clone()
    };
    for bad in [&uninvited, &wrong] {
        let refused = join(bad, &me, "Me").await.err().expect("turned away");
        assert!(
            matches!(refused, LinkError::Turned(Turned::NeedInvite)),
            "{refused}"
        );
    }
    let (first, tx, _rx) = join(&link, &me, "Me").await.unwrap();
    tx.close().await;
    // Back again with only the address and key: the key is who we are.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let (again, _tx, _rx) = join(&uninvited, &me, "Someone Else").await.unwrap();
    assert_eq!(again.you, first.you);
    server.stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_world_that_is_not_the_one_invited_to_is_refused() {
    let dir = new_world("impostor");
    let server = Server::start_with(&dir, fast()).unwrap();
    let forged = Invite {
        world: Identity::generate().public(),
        ..invite(&server)
    };
    let refused = join(&forged, &Identity::generate(), "Me")
        .await
        .err()
        .expect("refused");
    assert!(matches!(refused, LinkError::WrongWorld), "{refused}");
    server.stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_world_both_sides_survives_a_restart() {
    let dir = new_world("restart");
    let (alice, bob) = (Identity::generate(), Identity::generate());
    let (alice_was, bob_was, step_was) = {
        let server = Server::start_with(&dir, fast()).unwrap();
        let link = invite(&server);
        let (_, mut a_tx, mut a_rx) = join(&link, &alice, "Alice").await.unwrap();
        let (_, mut b_tx, mut b_rx) = join(&link, &bob, "Bob").await.unwrap();
        // Bob takes a walk by the well; Alice goes into the hall and stays
        // until the dun has killed her.
        let bob_was = walk(&mut b_tx, &mut b_rx, &[m(-10.0, 2.0)]).await.you;
        walk(
            &mut a_tx,
            &mut a_rx,
            &[m(-6.0, 3.0), m(6.0, 2.0), m(15.0, 2.0), m(15.0, 6.0)],
        )
        .await;
        let mut o = observe(&mut a_rx).await;
        // Three blows kill her: a few seconds of world time.
        for _ in 0..5000 {
            if o.you.body.side == Side::Lately {
                break;
            }
            o = observe(&mut a_rx).await;
        }
        assert_eq!(o.you.body.side, Side::Lately, "the dun never killed her");
        let alice_was = o.you;
        let step = o.step;
        server.stop().unwrap();
        (alice_was, bob_was, step)
    };
    let server = Server::start_with(&dir, fast()).unwrap();
    let link = invite(&server);
    let (a, _a_tx, mut a_rx) = join(&link, &alice, "Alice").await.unwrap();
    let (_, _b_tx, mut b_rx) = join(&link, &bob, "Bob").await.unwrap();
    assert!(
        a.step >= step_was,
        "the world's time carries on from where it stopped"
    );
    let alice_is = observe(&mut a_rx).await.you;
    assert_eq!(alice_is.body.side, Side::Lately);
    assert_eq!(alice_is.body.pos, alice_was.body.pos);
    assert_eq!(alice_is.deaths, 1);
    let bob_is = observe(&mut b_rx).await.you;
    assert_eq!(bob_is.body.side, Side::Living);
    assert_eq!(bob_is.body.pos, bob_was.body.pos);
    server.stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_world_runs_on_one_server_at_a_time() {
    let dir = new_world("one-at-a-time");
    let server = Server::start_with(&dir, fast()).unwrap();
    let (_, mut tx, mut rx) = join(&invite(&server), &Identity::generate(), "Me")
        .await
        .unwrap();
    // A second server for the same world, by another way to its folder and
    // on another port, is refused before it touches the world.
    let refused = Server::start_with(&dir.join("."), fast())
        .err()
        .expect("refused");
    assert!(refused.to_string().contains("another server"), "{refused}");
    let seq = tx.act(tme_sim::Intent::Move(Dir::NORTH)).await.unwrap();
    applied(&mut rx, seq).await;
    server.stop().unwrap();
    // Once it has stopped, the world can be run again.
    Server::start_with(&dir, fast()).unwrap().stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_world_that_cannot_save_stops_serving() {
    let dir = new_world("unsaved");
    let server = Server::start_with(&dir, fast()).unwrap();
    let (_, _tx, mut rx) = join(&invite(&server), &Identity::generate(), "Me")
        .await
        .unwrap();
    // Something else takes the world's database and keeps it.
    let db = rusqlite::Connection::open(dir.join("world.db")).unwrap();
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    tokio::time::timeout(Duration::from_secs(10), server.stopped())
        .await
        .expect("the world stops");
    let let_go = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(_)) = rx.next().await {}
    })
    .await;
    assert!(let_go.is_ok(), "its players are let go");
    let why = server.stop().expect_err("it says why").to_string();
    assert!(why.contains("saving the world"), "{why}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_loses_no_more_than_the_last_moment() {
    let dir = new_world("crash");
    let me = Identity::generate();
    // The real server, so it can be killed outright.
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tme-server"))
        .arg("run")
        .arg(&dir)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut said = String::new();
    std::io::BufRead::read_line(
        &mut std::io::BufReader::new(child.stdout.take().unwrap()),
        &mut said,
    )
    .unwrap();
    let address = said.trim().strip_prefix("listening on ").expect(&said);
    let link = Invite {
        address: address.to_owned(),
        ..Folder::open(&dir).unwrap().invite()
    };
    let (_, mut tx, mut rx) = join(&link, &me, "Me").await.unwrap();
    let from = observe(&mut rx).await.you.body.pos;
    // West, and keep going: nothing more is said after this.
    tx.act(tme_sim::Intent::Move(Dir::new(-i32::from(Dir::UNIT), 0)))
        .await
        .unwrap();
    let mut o = observe(&mut rx).await;
    while o.you.body.pos.x > from.x - 2000 {
        o = observe(&mut rx).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    child.kill().unwrap();
    child.wait().unwrap();

    let server = Server::start_with(&dir, fast()).unwrap();
    let (back, _tx, mut rx) = join(&invite(&server), &me, "Me").await.unwrap();
    assert!(back.step >= o.step, "{} steps, then {}", o.step, back.step);
    let now = observe(&mut rx).await.you.body.pos;
    assert!(
        now.x <= o.you.body.pos.x,
        "walked to {:?}, back at {now:?}",
        o.you.body.pos
    );
    server.stop().unwrap();
}
