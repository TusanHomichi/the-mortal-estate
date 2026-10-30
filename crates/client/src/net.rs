//! The connection to the world, on a thread of its own: intents go out as
//! the player makes them, and whatever the world says comes back to be read
//! each frame.

use bevy::prelude::*;
use std::sync::Mutex;
use std::sync::mpsc;
use tme_link::{Joined, LinkError};
use tme_sim::Intent;
use tme_wire::{Identity, Invite, ServerMsg};
use tokio::sync::mpsc as async_mpsc;

/// What arrives from the world.
pub enum FromWorld {
    Said(ServerMsg),
    /// The connection is over, and why.
    Closed(String),
}

#[derive(Resource)]
pub struct Net {
    pub joined: Joined,
    intents: async_mpsc::UnboundedSender<Intent>,
    inbox: Mutex<mpsc::Receiver<FromWorld>>,
    /// How many intents have gone out: the last one's number.
    pub sent: u32,
}

impl Net {
    /// Send an intent: its number, which observations acknowledge.
    pub fn act(&mut self, intent: Intent) -> u32 {
        // A closed connection shows in the inbox; nothing more to do here.
        let _ = self.intents.send(intent);
        self.sent += 1;
        self.sent
    }

    /// Everything the world has said since the last look.
    pub fn drain(&self) -> Vec<FromWorld> {
        let inbox = self.inbox.lock().expect("the inbox isn't poisoned");
        inbox.try_iter().collect()
    }
}

/// Join the world `invite` names, and keep talking to it on its own thread.
pub fn connect(invite: &Invite, me: Identity, name: &str) -> Result<Net, LinkError> {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (intents, mut intent_rx) = async_mpsc::unbounded_channel();
    let (inbox_tx, inbox) = mpsc::channel();
    let (invite, name) = (invite.clone(), name.to_owned());
    std::thread::Builder::new()
        .name("tme-link".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime for the connection");
            runtime.block_on(async move {
                let (joined, mut tx, mut rx) = match tme_link::join(&invite, &me, &name).await {
                    Ok(j) => j,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(joined));
                loop {
                    tokio::select! {
                        intent = intent_rx.recv() => match intent {
                            Some(intent) => {
                                if let Err(e) = tx.act(intent).await {
                                    let _ = inbox_tx.send(FromWorld::Closed(e.to_string()));
                                    return;
                                }
                            }
                            // The game is closing: leave the world.
                            None => {
                                tx.close().await;
                                return;
                            }
                        },
                        msg = rx.next() => {
                            let out = match msg {
                                Some(Ok(msg)) => FromWorld::Said(msg),
                                Some(Err(e)) => FromWorld::Closed(e.to_string()),
                                None => FromWorld::Closed("the world closed the connection".into()),
                            };
                            let closed = matches!(out, FromWorld::Closed(_));
                            if inbox_tx.send(out).is_err() || closed {
                                return;
                            }
                        }
                    }
                }
            });
        })
        .map_err(|e| LinkError::Connection(e.to_string()))?;
    let joined = ready_rx
        .recv()
        .map_err(|_| LinkError::Connection("the connection's thread stopped".into()))??;
    Ok(Net {
        joined,
        intents,
        inbox: Mutex::new(inbox),
        sent: 0,
    })
}
