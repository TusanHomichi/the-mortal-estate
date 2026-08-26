use std::collections::{BTreeMap, VecDeque};
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket};
use sha2::{Digest, Sha256};
use tme_protocol as wire;
use tokio::sync::{mpsc, watch};

use crate::admission::{AdmissionError, AdmissionGrant};
use crate::config::{
    COMMAND_RATE_BURST, COMMAND_RATE_PER_SECOND, HELLO_DEADLINE, IDLE_TIMEOUT,
    OUTBOUND_QUEUE_CAPACITY, PING_INTERVAL, PONG_GRACE, SOCIAL_DEDUPE_CAPACITY, SOCIAL_RATE_BURST,
    SOCIAL_RATE_PER_SECOND,
};
use crate::coordinator::Reservation;
use crate::facet::{FacetCommand, FacetError};
use crate::http::{AppState, ConnectionGuard};

pub(crate) async fn serve(
    mut socket: WebSocket,
    state: AppState,
    mut guard: ConnectionGuard,
    host: String,
    origin: String,
) {
    let hello_message = match tokio::time::timeout(HELLO_DEADLINE, socket.recv()).await {
        Ok(Some(Ok(Message::Text(text)))) => text,
        Ok(Some(Ok(Message::Binary(_)))) => {
            send_error(&mut socket, wire::ErrorCode::BinaryMessage).await;
            return;
        }
        Ok(_) => {
            send_error(&mut socket, wire::ErrorCode::MalformedProtocol).await;
            return;
        }
        Err(_) => {
            send_error(&mut socket, wire::ErrorCode::HelloTimeout).await;
            return;
        }
    };
    let hello = match wire::decode_client_hello(hello_message.as_bytes()) {
        Ok(hello) => hello,
        Err(_) => {
            send_error(&mut socket, wire::ErrorCode::MalformedProtocol).await;
            return;
        }
    };
    let (ticket, supported_minors) = match hello.into_parts() {
        Ok(parts) => parts,
        Err(_) => {
            send_error(&mut socket, wire::ErrorCode::MalformedProtocol).await;
            return;
        }
    };
    let (outbound, mut outbound_receive) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
    let (terminal, mut terminal_receive) = watch::channel(None);
    let (grant, welcome) = match state
        .admit(
            &ticket,
            &supported_minors,
            &origin,
            &host,
            outbound,
            terminal,
        )
        .await
    {
        Ok(admission) => admission,
        Err(error) => {
            send_error(&mut socket, admission_error(error)).await;
            return;
        }
    };
    guard.mark_authenticated();
    let wire_actor_id = match crate::protocol_v1::actor_id(&grant.control.actor_id) {
        Ok(actor_id) => actor_id,
        Err(_) => {
            send_error(&mut socket, wire::ErrorCode::Unavailable).await;
            let _ = grant.facet.detach(grant.control.connection_id).await;
            return;
        }
    };
    let welcome = wire::ServerEnvelope::ServerWelcome {
        selected_major: wire::PROTOCOL_MAJOR,
        selected_minor: wire::PROTOCOL_MINOR,
        connection_id: grant.control.connection_id,
        actor_id: wire_actor_id,
        control_epoch: wire::DecimalU64::new(grant.control.control_epoch),
        server_sequence: wire::DecimalU64::new(welcome.server_sequence),
        world_revision: wire::DecimalU64::new(welcome.facet_revision),
        static_scene_context: welcome.static_scene_context,
        frame: welcome.frame,
    };
    if let Err(error) = send_envelope(&mut socket, &welcome).await {
        tracing::error!(%error, "failed to encode or send initial server welcome");
        let _ = grant.facet.detach(grant.control.connection_id).await;
        return;
    }

    let mut command_limiter =
        RateLimiter::new(COMMAND_RATE_BURST, f64::from(COMMAND_RATE_PER_SECOND));
    let mut social_limiter = RateLimiter::new(SOCIAL_RATE_BURST, SOCIAL_RATE_PER_SECOND);
    let mut social_dedupe = SocialDedupe::default();
    let mut last_input = Instant::now();
    let mut last_ping = Instant::now();
    let mut waiting_for_pong = None;
    let mut draining = state.subscribe_draining();
    loop {
        if state.is_draining() {
            let _ = send_envelope(
                &mut socket,
                &wire::ServerEnvelope::ServerDraining {
                    reason: wire::DrainingReason::Shutdown,
                    reconnect_hint: false,
                },
            )
            .await;
            break;
        }
        let idle_remaining = IDLE_TIMEOUT.saturating_sub(last_input.elapsed());
        let ping_remaining = PING_INTERVAL.saturating_sub(last_ping.elapsed());
        let pong_remaining = waiting_for_pong
            .map(|sent: Instant| PONG_GRACE.saturating_sub(sent.elapsed()))
            .unwrap_or(PONG_GRACE);
        tokio::select! {
            biased;
            changed = draining.changed() => {
                if changed.is_err() || *draining.borrow() {
                    let _ = send_envelope(
                        &mut socket,
                        &wire::ServerEnvelope::ServerDraining {
                            reason: wire::DrainingReason::Shutdown,
                            reconnect_hint: false,
                        },
                    )
                    .await;
                    break;
                }
            }
            changed = terminal_receive.changed() => {
                let reason = terminal_receive.borrow_and_update().to_owned();
                if let Some(reason) = reason {
                    let reconnect_hint = matches!(
                        reason,
                        wire::DrainingReason::ControlReplaced
                    );
                    let _ = send_envelope(
                        &mut socket,
                        &wire::ServerEnvelope::ServerDraining {
                            reason,
                            reconnect_hint,
                        },
                    )
                    .await;
                }
                if changed.is_err() || reason.is_some() {
                    break;
                }
            }
            _ = tokio::time::sleep(idle_remaining) => {
                tracing::error!("closing gameplay socket after idle timeout");
                break;
            },
            _ = tokio::time::sleep(ping_remaining), if waiting_for_pong.is_none() => {
                if socket.send(Message::Ping(Bytes::new())).await.is_err() { break; }
                let now = Instant::now();
                last_ping = now;
                waiting_for_pong = Some(now);
            }
            _ = tokio::time::sleep(pong_remaining), if waiting_for_pong.is_some() => {
                tracing::error!("closing gameplay socket after pong timeout");
                break;
            },
            envelope = outbound_receive.recv() => {
                let Some(envelope) = envelope else {
                    let terminal_reason = terminal_receive.borrow_and_update().to_owned();
                    if let Some(reason) = terminal_reason {
                        let reconnect_hint = matches!(
                            reason,
                            wire::DrainingReason::ControlReplaced
                        );
                        let _ = send_envelope(
                            &mut socket,
                            &wire::ServerEnvelope::ServerDraining { reason, reconnect_hint },
                        )
                        .await;
                    }
                    tracing::error!("closing gameplay socket after outbound channel ended");
                    break;
                };
                if send_envelope(&mut socket, &envelope).await.is_err() { break; }
            }
            message = socket.recv() => {
                let Some(Ok(message)) = message else {
                    tracing::error!("closing gameplay socket after receive failure");
                    break;
                };
                last_input = Instant::now();
                // Any authenticated inbound frame proves the peer is live even when its
                // WebSocket implementation does not surface an automatic Pong.
                waiting_for_pong = None;
                match message {
                    Message::Pong(_) => {}
                    Message::Ping(payload) => {
                        if socket.send(Message::Pong(payload)).await.is_err() { break; }
                    }
                    Message::Binary(_) => {
                        send_error(&mut socket, wire::ErrorCode::BinaryMessage).await;
                        break;
                    }
                    Message::Text(text) => {
                        let envelope = match wire::decode_client_command(text.as_bytes(), wire::PROTOCOL_MINOR) {
                            Ok(envelope) => envelope,
                            Err(_) => {
                                send_error(&mut socket, wire::ErrorCode::MalformedProtocol).await;
                                break;
                            }
                        };
                        match envelope {
                            command @ wire::ClientCommandEnvelope::Command { .. } => {
                                if !command_limiter.allow() {
                                    send_error(&mut socket, wire::ErrorCode::RateLimited).await;
                                    continue;
                                }
                                if handle_command(&mut socket, &state, &grant, command).await.is_err() {
                                    break;
                                }
                            }
                            preview @ wire::ClientCommandEnvelope::PathPreview { .. } => {
                                if !command_limiter.allow() {
                                    send_error(&mut socket, wire::ErrorCode::RateLimited).await;
                                    continue;
                                }
                                if handle_path_preview(&mut socket, &state, &grant, preview).await.is_err() {
                                    break;
                                }
                            }
                            message @ wire::ClientCommandEnvelope::SocialMessage { .. } => {
                                if handle_social_message(
                                    &mut socket,
                                    &state,
                                    &grant,
                                    message,
                                    &mut social_limiter,
                                    &mut social_dedupe,
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    Message::Close(_) => break,
                }
            }
        }
    }
    let _ = grant.facet.detach(grant.control.connection_id).await;
}

async fn handle_path_preview(
    socket: &mut WebSocket,
    state: &AppState,
    grant: &AdmissionGrant,
    preview: wire::ClientCommandEnvelope,
) -> Result<(), ()> {
    let wire::ClientCommandEnvelope::PathPreview {
        preview_id,
        control_epoch,
        observed_world_revision,
        actor_id,
        path,
    } = preview
    else {
        return Err(());
    };
    if !state.authorize_grant(&grant.control).await {
        send_error(socket, wire::ErrorCode::Unavailable).await;
        return Ok(());
    }
    let receive = match grant
        .facet
        .try_path_preview(crate::facet::FacetPathPreview {
            grant: grant.control.clone(),
            preview_id,
            control_epoch: control_epoch.get(),
            observed_facet_revision: observed_world_revision.get(),
            actor_id,
            path,
        }) {
        Ok(receive) => receive,
        Err(FacetError::QueueFull) => {
            send_error(socket, wire::ErrorCode::QueuePressure).await;
            return Ok(());
        }
        Err(_) => {
            send_error(socket, wire::ErrorCode::Unavailable).await;
            return Ok(());
        }
    };
    let reply = match receive.await {
        Ok(reply) => reply,
        Err(_) => {
            send_error(socket, wire::ErrorCode::Unavailable).await;
            return Ok(());
        }
    };
    send_envelope(socket, &reply.envelope).await.map_err(|_| ())
}

async fn handle_social_message(
    socket: &mut WebSocket,
    state: &AppState,
    grant: &AdmissionGrant,
    message: wire::ClientCommandEnvelope,
    limiter: &mut RateLimiter,
    dedupe: &mut SocialDedupe,
) -> Result<(), ()> {
    let wire::ClientCommandEnvelope::SocialMessage {
        message_id,
        control_epoch,
        actor_id,
        scope,
        body,
    } = message
    else {
        return Err(());
    };
    let digest = social_digest(message_id, control_epoch, &actor_id, &scope, &body);
    match dedupe.lookup(message_id, digest) {
        DedupeLookup::Replay(disposition) => {
            return send_envelope(
                socket,
                &wire::ServerEnvelope::MessageResult {
                    message_id,
                    disposition,
                },
            )
            .await
            .map_err(|_| ());
        }
        DedupeLookup::Mismatch => {
            let _ = send_envelope(
                socket,
                &wire::ServerEnvelope::MessageResult {
                    message_id,
                    disposition: wire::MessageDisposition::Malformed,
                },
            )
            .await;
            return Err(());
        }
        DedupeLookup::New => {}
    }
    let disposition = if !limiter.allow() {
        wire::MessageDisposition::RateLimited
    } else if control_epoch.get() != grant.control.control_epoch
        || actor_id.as_str() != grant.control.actor_id.as_str()
        || !state.authorize_grant(&grant.control).await
    {
        wire::MessageDisposition::Unavailable
    } else {
        match grant
            .facet
            .social_message(grant.control.clone(), message_id, scope, body.clone())
            .await
        {
            Ok(crate::facet::FacetSocialOutcome::Complete(disposition)) => disposition,
            Ok(crate::facet::FacetSocialOutcome::PagePrepared {
                target_character_id,
                sender_name,
            }) => {
                if state
                    .deliver_page(
                        target_character_id,
                        message_id,
                        grant.control.character_id,
                        sender_name,
                        body,
                    )
                    .await
                {
                    wire::MessageDisposition::Accepted
                } else {
                    wire::MessageDisposition::Unavailable
                }
            }
            Err(_) => wire::MessageDisposition::Unavailable,
        }
    };
    dedupe.insert(message_id, digest, disposition);
    send_envelope(
        socket,
        &wire::ServerEnvelope::MessageResult {
            message_id,
            disposition,
        },
    )
    .await
    .map_err(|_| ())
}

fn social_digest(
    message_id: wire::MessageId,
    control_epoch: wire::DecimalU64,
    actor_id: &wire::ActorId,
    scope: &wire::SocialScope,
    body: &wire::SocialBody,
) -> [u8; 32] {
    let bytes = serde_json::to_vec(&(message_id, control_epoch, actor_id, scope, body))
        .expect("bounded social message fields serialize");
    Sha256::digest(bytes).into()
}

#[derive(Default)]
struct SocialDedupe {
    insertion_order: VecDeque<wire::MessageId>,
    entries: BTreeMap<wire::MessageId, ([u8; 32], wire::MessageDisposition)>,
}

enum DedupeLookup {
    New,
    Replay(wire::MessageDisposition),
    Mismatch,
}

impl SocialDedupe {
    fn lookup(&self, message_id: wire::MessageId, digest: [u8; 32]) -> DedupeLookup {
        match self.entries.get(&message_id) {
            Some((stored, disposition)) if stored == &digest => DedupeLookup::Replay(*disposition),
            Some(_) => DedupeLookup::Mismatch,
            None => DedupeLookup::New,
        }
    }

    fn insert(
        &mut self,
        message_id: wire::MessageId,
        digest: [u8; 32],
        disposition: wire::MessageDisposition,
    ) {
        if self.entries.contains_key(&message_id) {
            return;
        }
        if self.entries.len() == SOCIAL_DEDUPE_CAPACITY
            && let Some(expired) = self.insertion_order.pop_front()
        {
            self.entries.remove(&expired);
        }
        self.insertion_order.push_back(message_id);
        self.entries.insert(message_id, (digest, disposition));
    }
}

async fn handle_command(
    socket: &mut WebSocket,
    state: &AppState,
    grant: &AdmissionGrant,
    command: wire::ClientCommandEnvelope,
) -> Result<(), ()> {
    let command_id = match &command {
        wire::ClientCommandEnvelope::Command { command_id, .. } => *command_id,
        wire::ClientCommandEnvelope::PathPreview { .. }
        | wire::ClientCommandEnvelope::SocialMessage { .. } => return Err(()),
    };
    match state
        .inner
        .coordinator
        .reserve(grant.control.account_id, command_id, &command)
        .await
    {
        Reservation::InProgress => {
            send_error(socket, wire::ErrorCode::CommandInProgress).await;
            Ok(())
        }
        Reservation::DigestMismatch => {
            send_error(socket, wire::ErrorCode::MalformedProtocol).await;
            Err(())
        }
        Reservation::Unavailable => {
            send_error(socket, wire::ErrorCode::Unavailable).await;
            Ok(())
        }
        Reservation::Replay(result) => {
            send_envelope(socket, &result).await.map_err(|_| ())?;
            let current = grant
                .facet
                .try_current_state(grant.control.connection_id)
                .map_err(|_| ())?
                .await
                .map_err(|_| ())?
                .map_err(|_| ())?;
            send_envelope(socket, &current).await.map_err(|_| ())?;
            Ok(())
        }
        Reservation::New { digest } => {
            if !state.authorize_grant(&grant.control).await {
                let result = state
                    .inner
                    .coordinator
                    .complete_authority_rejection(
                        grant.control.account_id,
                        grant.control.session_id,
                        command_id,
                        digest,
                        wire::RejectionCode::StaleControlEpoch,
                    )
                    .await
                    .map_err(|_| ())?;
                send_envelope(socket, &result).await.map_err(|_| ())?;
                return Ok(());
            }

            let facet_command = match &command {
                wire::ClientCommandEnvelope::Command {
                    command_id,
                    control_epoch,
                    client_sequence,
                    observed_world_revision,
                    actor_id,
                    intent,
                } => FacetCommand {
                    connection_id: grant.control.connection_id,
                    account_id: grant.control.account_id,
                    session_id: grant.control.session_id,
                    character_id: grant.control.character_id,
                    command_id: *command_id,
                    control_epoch: control_epoch.get(),
                    client_sequence: client_sequence.get(),
                    observed_facet_revision: observed_world_revision.get(),
                    actor_id: actor_id.clone(),
                    intent: intent.clone(),
                    request_digest: digest,
                    #[cfg(test)]
                    certification_trace: None,
                    #[cfg(test)]
                    ev_fail_checkpoint_export: false,
                    #[cfg(test)]
                    ev_fail_after_store_commit: false,
                },
                wire::ClientCommandEnvelope::PathPreview { .. }
                | wire::ClientCommandEnvelope::SocialMessage { .. } => return Err(()),
            };
            let receive = match grant.facet.try_command(facet_command) {
                Ok(receive) => receive,
                Err(FacetError::QueueFull) => {
                    state
                        .inner
                        .coordinator
                        .release(grant.control.account_id, command_id, digest);
                    send_error(socket, wire::ErrorCode::QueuePressure).await;
                    return Ok(());
                }
                Err(_) => {
                    state
                        .inner
                        .coordinator
                        .release(grant.control.account_id, command_id, digest);
                    send_error(socket, wire::ErrorCode::Unavailable).await;
                    return Ok(());
                }
            };
            let reply = match receive.await {
                Ok(reply) => reply,
                Err(_) => {
                    state
                        .inner
                        .coordinator
                        .release(grant.control.account_id, command_id, digest);
                    send_error(socket, wire::ErrorCode::Unavailable).await;
                    return Ok(());
                }
            };
            state
                .inner
                .coordinator
                .finish(grant.control.account_id, command_id, digest);
            send_envelope(socket, &reply.envelope)
                .await
                .map_err(|_| ())?;
            Ok(())
        }
    }
}

async fn send_envelope(
    socket: &mut WebSocket,
    envelope: &wire::ServerEnvelope,
) -> Result<(), wire::ProtocolError> {
    let encoded = wire::encode_server_envelope(envelope).map_err(|error| {
        tracing::error!(%error, "failed to encode outbound server envelope");
        error
    })?;
    let text = String::from_utf8(encoded).expect("JSON is UTF-8");
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to send outbound server envelope");
            wire::ProtocolError::new("socket send failed")
        })
}

async fn send_error(socket: &mut WebSocket, code: wire::ErrorCode) {
    let _ = send_envelope(socket, &wire::ServerEnvelope::Error { code }).await;
}

fn admission_error(error: AdmissionError) -> wire::ErrorCode {
    match error {
        AdmissionError::InvalidTicket => wire::ErrorCode::InvalidTicket,
        AdmissionError::ExpiredTicket => wire::ErrorCode::ExpiredTicket,
        AdmissionError::ConsumedTicket => wire::ErrorCode::ConsumedTicket,
        AdmissionError::UnsupportedVersion => wire::ErrorCode::UnsupportedVersion,
        AdmissionError::OriginRejected => wire::ErrorCode::OriginRejected,
        AdmissionError::HostRejected => wire::ErrorCode::HostRejected,
        AdmissionError::GameplayMarkLocked => wire::ErrorCode::GameplayMarkLocked,
        AdmissionError::Unavailable => wire::ErrorCode::Unavailable,
    }
}

struct RateLimiter {
    tokens: f64,
    last: Instant,
    burst: f64,
    per_second: f64,
}

impl RateLimiter {
    fn new(burst: u32, per_second: f64) -> Self {
        Self {
            tokens: f64::from(burst),
            last: Instant::now(),
            burst: f64::from(burst),
            per_second,
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();
        self.tokens = (self.tokens + now.duration_since(self.last).as_secs_f64() * self.per_second)
            .min(self.burst);
        self.last = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn message_id(sequence: u128) -> wire::MessageId {
        wire::MessageId::new(uuid::Uuid::from_u128(sequence)).expect("non-nil message ID")
    }

    #[test]
    fn social_dedupe_replays_exact_payload_and_rejects_changed_payload() {
        let id = message_id(1);
        let mut dedupe = SocialDedupe::default();
        dedupe.insert(id, [7; 32], wire::MessageDisposition::Accepted);
        assert!(matches!(
            dedupe.lookup(id, [7; 32]),
            DedupeLookup::Replay(wire::MessageDisposition::Accepted)
        ));
        assert!(matches!(dedupe.lookup(id, [8; 32]), DedupeLookup::Mismatch));
    }

    #[test]
    fn social_dedupe_retains_the_latest_bounded_window() {
        let mut dedupe = SocialDedupe::default();
        for sequence in 1..=u128::try_from(SOCIAL_DEDUPE_CAPACITY + 1).unwrap() {
            dedupe.insert(
                message_id(sequence),
                [sequence as u8; 32],
                wire::MessageDisposition::Unavailable,
            );
        }
        assert!(matches!(
            dedupe.lookup(message_id(1), [1; 32]),
            DedupeLookup::New
        ));
        assert_eq!(dedupe.entries.len(), SOCIAL_DEDUPE_CAPACITY);
    }

    #[test]
    fn social_rate_limit_is_burst_five_and_refills_one_per_two_seconds() {
        let mut limiter = RateLimiter::new(SOCIAL_RATE_BURST, SOCIAL_RATE_PER_SECOND);
        for _ in 0..SOCIAL_RATE_BURST {
            assert!(limiter.allow());
        }
        assert!(!limiter.allow());
        limiter.last = limiter.last.checked_sub(Duration::from_secs(2)).unwrap();
        assert!(limiter.allow());
        assert!(!limiter.allow());
    }

    #[test]
    fn command_rate_limit_enforces_burst_and_twenty_per_second_refill() {
        let mut limiter = RateLimiter::new(COMMAND_RATE_BURST, f64::from(COMMAND_RATE_PER_SECOND));
        for _ in 0..COMMAND_RATE_BURST {
            assert!(limiter.allow(), "the configured command burst is admitted");
        }
        assert!(
            !limiter.allow(),
            "the first command beyond the burst is denied"
        );

        limiter.last = limiter
            .last
            .checked_sub(Duration::from_millis(500))
            .unwrap();
        for _ in 0..(COMMAND_RATE_PER_SECOND / 2) {
            assert!(limiter.allow(), "half a second refills half the rate");
        }
        assert!(
            !limiter.allow(),
            "half a second does not refill an eleventh command"
        );

        limiter.last = limiter.last.checked_sub(Duration::from_millis(50)).unwrap();
        assert!(limiter.allow(), "fifty milliseconds refills one command");
        assert!(!limiter.allow(), "one token cannot admit two commands");
    }
}
