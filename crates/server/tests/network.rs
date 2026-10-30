//! What a client can do to a world over the network, and what it can't:
//! read or change the session, flood the world with intents, make it hold
//! messages it won't read, or hold connections open without joining.

use futures_util::{SinkExt, StreamExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tme_link::{Receiver, join};
use tme_server::folder::{Folder, NewWorld};
use tme_server::{Options, Server};
use tme_sim::{BodyId, Dir, Event, Intent};
use tme_wire::session::{self, Session};
use tme_wire::{
    ClientMsg, Greeting, Identity, Invite, Join, PROTOCOL, ServerMsg, Turned, decode, encode,
};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpSocket, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn new_world(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tme-network-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Folder::create(
        &dir,
        NewWorld {
            name: "Lattermead",
            content: &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
            bind: "127.0.0.1:0",
            address: "unused:0",
        },
    )
    .expect("a new world");
    dir
}

fn start(name: &str) -> (Server, Invite) {
    let server = Server::start_with(
        &new_world(name),
        Options {
            step: Duration::from_millis(1),
            handshake: Duration::from_millis(500),
        },
    )
    .unwrap();
    let invite = Invite {
        address: server.local.to_string(),
        ..server.invite().clone()
    };
    (server, invite)
}

/// The next binary frame, or `None` once the world has closed.
async fn frame(ws: &mut Ws) -> Option<Vec<u8>> {
    loop {
        match tokio::time::timeout(Duration::from_secs(10), ws.next())
            .await
            .expect("the world answers or hangs up")
        {
            Some(Ok(Message::Binary(b))) => return Some(b.to_vec()),
            Some(Ok(Message::Close(_)) | Err(_)) | None => return None,
            Some(Ok(_)) => continue,
        }
    }
}

/// Join by hand over `ws`: every frame the client sent, and the session.
async fn handshake(mut ws: Ws, invite: &Invite) -> (Ws, Vec<Vec<u8>>, Session) {
    let me = Identity::generate();
    let (dialing, hello) = session::dial(&me, &invite.world, PROTOCOL);
    let mut sent = vec![encode(&hello)];
    ws.send(Message::binary(sent[0].clone())).await.unwrap();
    let Ok(Greeting::Noise(answer)) = decode(&frame(&mut ws).await.unwrap()) else {
        panic!("no answer");
    };
    let join = Join {
        name: "Me".into(),
        invite: invite.code.clone(),
    };
    let finished = dialing.finish(&answer, &encode(&join)).unwrap();
    sent.push(finished.third.clone());
    ws.send(Message::binary(finished.third)).await.unwrap();
    let mut session = finished.session;
    let welcome = session.open.open(&frame(&mut ws).await.unwrap()).unwrap();
    assert!(matches!(decode(&welcome), Ok(ServerMsg::Welcome { .. })));
    (ws, sent, session)
}

async fn connect(invite: &Invite) -> Ws {
    let url = format!("ws://{}/", invite.address);
    tokio_tungstenite::connect_async(url).await.unwrap().0
}

/// Whether the world hangs up within a few seconds, reading what it sends
/// until then.
async fn hangs_up(ws: &mut Ws) -> bool {
    tokio::time::timeout(Duration::from_secs(5), async {
        while frame(ws).await.is_some() {}
    })
    .await
    .is_ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_invite_code_never_travels_in_the_clear() {
    let (server, invite) = start("clear");
    let (_, sent, _) = handshake(connect(&invite).await, &invite).await;
    let code = invite.code.clone().unwrap();
    for frame in sent {
        assert!(
            !frame.windows(code.len()).any(|w| w == code.as_bytes()),
            "a frame carried the code"
        );
    }
    server.stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_frame_replayed_or_slipped_in_ends_the_session() {
    let (server, invite) = start("tamper");
    let act = encode(&ClientMsg::Act {
        seq: 1,
        intent: Intent::Move(Dir::NORTH),
    });
    // The same sealed frame twice.
    let (mut ws, _, mut session) = handshake(connect(&invite).await, &invite).await;
    let sealed = session.seal.seal(&act);
    ws.send(Message::binary(sealed.clone())).await.unwrap();
    ws.send(Message::binary(sealed)).await.unwrap();
    assert!(hangs_up(&mut ws).await, "a replay is refused");
    // A frame that isn't sealed, as anyone between could send.
    let (mut ws, _, _) = handshake(connect(&invite).await, &invite).await;
    ws.send(Message::binary(act)).await.unwrap();
    assert!(hangs_up(&mut ws).await, "an unsealed frame is refused");
    server.stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_that_floods_the_world_is_turned_away() {
    let (server, invite) = start("flood");
    let (mut ws, _, mut session) = handshake(connect(&invite).await, &invite).await;
    for seq in 1..=500 {
        let act = ClientMsg::Act {
            seq,
            intent: Intent::Move(Dir::NORTH),
        };
        if ws
            .send(Message::binary(session.seal.seal(&encode(&act))))
            .await
            .is_err()
        {
            break;
        }
    }
    let turned = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(f) = frame(&mut ws).await {
            if let Ok(ServerMsg::Turned(t)) = decode(&session.open.open(&f).unwrap()) {
                return Some(t);
            }
        }
        None
    })
    .await;
    assert_eq!(turned, Ok(Some(Turned::TooFast)));
    server.stop().unwrap();
}

/// What `rx` observes until someone with `id` leaves; whether they did
/// within `within`.
async fn sees_leave(rx: &mut Receiver, id: BodyId, within: Duration) -> bool {
    tokio::time::timeout(within, async {
        loop {
            let Some(Ok(ServerMsg::Observe(o))) = rx.next().await else {
                continue;
            };
            let left = o.events.iter().any(|e| match (e, id) {
                (Event::Left { person }, BodyId::Person(p)) => *person == p,
                _ => false,
            });
            if left {
                return;
            }
        }
    })
    .await
    .is_ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_that_stops_reading_is_let_go_and_the_world_goes_on() {
    let (server, invite) = start("slow");
    let (_, _b_tx, mut b_rx) = join(&invite, &Identity::generate(), "Watcher")
        .await
        .unwrap();
    // A client with almost no room to receive, which never reads.
    let socket = TcpSocket::new_v4().unwrap();
    socket.set_recv_buffer_size(2048).unwrap();
    let stream = socket.connect(server.local).await.unwrap();
    let url = format!("ws://{}/", invite.address);
    let (ws, _) = tokio_tungstenite::client_async(url, MaybeTlsStream::Plain(stream))
        .await
        .unwrap();
    let (_ws, _, _) = handshake(ws, &invite).await;
    // It's person 2: the watcher came first. It's let go once the world
    // has held a few seconds of what it hasn't read, sooner than a frame
    // to it is given up on.
    let not_reading = BodyId::Person(tme_sim::PersonId(2));
    assert!(
        sees_leave(&mut b_rx, not_reading, Duration::from_secs(8)).await,
        "the one not reading is let go"
    );
    server.stop().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn connections_that_never_join_are_closed_and_bounded() {
    let (server, invite) = start("idle");
    // One that never even upgrades to a WebSocket is closed at the deadline.
    let mut idle = TcpStream::connect(server.local).await.unwrap();
    let closed = tokio::time::timeout(Duration::from_secs(5), idle.read(&mut [0; 1])).await;
    assert!(matches!(closed, Ok(Ok(0) | Err(_))), "closed: {closed:?}");
    // While as many as can be are joining, one more waits its turn: it's
    // let in once they've had their time, not before and not turned away.
    let mut joining = Vec::new();
    for _ in 0..64 {
        joining.push(TcpStream::connect(server.local).await.unwrap());
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    let asked = std::time::Instant::now();
    join(&invite, &Identity::generate(), "Me").await.unwrap();
    let waited = asked.elapsed();
    assert!(waited >= Duration::from_millis(300), "waited {waited:?}");
    drop(joining);
    server.stop().unwrap();
}
