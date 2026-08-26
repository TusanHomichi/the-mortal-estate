use std::collections::BTreeMap;
use std::sync::Mutex;

use sha2::{Digest, Sha256};
use tme_protocol as wire;

use crate::postgres::MAX_RECEIPTS_PER_ACCOUNT;
use crate::store::receipt::{ReceiptOutcomeV3, expired_envelope};
use crate::store::{SharedStore, SystemCommit};

pub struct Coordinator {
    pending: Mutex<BTreeMap<(wire::AccountId, wire::CommandId), [u8; 32]>>,
    store: Option<SharedStore>,
    transition: tokio::sync::Mutex<()>,
}

impl Default for Coordinator {
    fn default() -> Self {
        Self {
            pending: Mutex::new(BTreeMap::new()),
            store: None,
            transition: tokio::sync::Mutex::new(()),
        }
    }
}

pub enum Reservation {
    New { digest: [u8; 32] },
    InProgress,
    Replay(Box<wire::ServerEnvelope>),
    DigestMismatch,
    Unavailable,
}

impl Coordinator {
    pub(crate) fn new(store: SharedStore) -> Self {
        Self {
            store: Some(store),
            ..Self::default()
        }
    }

    pub async fn transition(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.transition.lock().await
    }

    /// D4: one canonical world. A player-kill consequence whose credited killer
    /// is not resident here cannot be applied to a live engine, because there is
    /// no second world hosting them — they are simply absent. Owner ruling
    /// 2026-08-20: it is deferred, not waived. The mark and a durable pending
    /// consequence commit together, and the killer's next admission applies it.
    /// See docs/server-notes.md.
    pub(crate) async fn commit_system(&self, value: SystemCommit<'_>) -> Result<(), String> {
        self.store
            .as_ref()
            .ok_or_else(|| "durable coordinator store is absent".to_string())?
            .commit_system(value)
            .await
    }

    pub async fn reserve(
        &self,
        account_id: wire::AccountId,
        command_id: wire::CommandId,
        command: &wire::ClientCommandEnvelope,
    ) -> Reservation {
        let digest = command_digest(command);
        let Some(store) = &self.store else {
            return Reservation::Unavailable;
        };
        match store.receipt(account_id, command_id).await {
            Ok(Some(stored)) if stored.request_digest != digest => {
                return Reservation::DigestMismatch;
            }
            Ok(Some(stored)) => {
                let envelope = match stored.outcome {
                    Some(outcome) => {
                        match outcome.to_envelope(command_id, wire::ReplayStatus::Replayed) {
                            Ok(value) => value,
                            Err(_) => return Reservation::Unavailable,
                        }
                    }
                    None => expired_envelope(command_id),
                };
                return Reservation::Replay(Box::new(envelope));
            }
            Ok(None) => {}
            Err(_) => return Reservation::Unavailable,
        }
        let Ok(mut pending) = self.pending.lock() else {
            return Reservation::Unavailable;
        };
        let key = (account_id, command_id);
        match pending.get(&key) {
            None => {
                if pending
                    .keys()
                    .filter(|(owner, _)| owner == &account_id)
                    .count()
                    >= MAX_RECEIPTS_PER_ACCOUNT
                {
                    return Reservation::Unavailable;
                }
                pending.insert(key, digest);
                Reservation::New { digest }
            }
            Some(existing) if existing == &digest => Reservation::InProgress,
            Some(_) => Reservation::DigestMismatch,
        }
    }

    pub async fn complete_authority_rejection(
        &self,
        account_id: wire::AccountId,
        session_id: wire::SessionId,
        command_id: wire::CommandId,
        digest: [u8; 32],
        code: wire::RejectionCode,
    ) -> Result<wire::ServerEnvelope, ()> {
        let outcome = ReceiptOutcomeV3::rejected(code, None, None);
        let persisted = self
            .store
            .as_ref()
            .ok_or(())?
            .insert_authority_rejection(account_id, session_id, command_id, digest, &outcome)
            .await
            .map_err(|_| ());
        self.release(account_id, command_id, digest);
        persisted?;
        outcome
            .to_envelope(command_id, wire::ReplayStatus::New)
            .map_err(|_| ())
    }

    pub fn finish(
        &self,
        account_id: wire::AccountId,
        command_id: wire::CommandId,
        digest: [u8; 32],
    ) {
        self.release(account_id, command_id, digest);
    }

    pub fn release(
        &self,
        account_id: wire::AccountId,
        command_id: wire::CommandId,
        digest: [u8; 32],
    ) {
        if let Ok(mut pending) = self.pending.lock() {
            let key = (account_id, command_id);
            if matches!(pending.get(&key), Some(value) if value == &digest) {
                pending.remove(&key);
            }
        }
    }
}

pub(crate) fn command_digest(command: &wire::ClientCommandEnvelope) -> [u8; 32] {
    let encoded = serde_json::to_vec(command).expect("typed protocol command serializes");
    Sha256::digest(encoded).into()
}
