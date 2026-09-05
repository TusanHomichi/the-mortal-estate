use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tme_protocol as wire;
use tokio::sync::{mpsc, watch};

use crate::admission::{AdmissionError, AdmissionGrant};
use crate::config::{
    MAX_CONNECTIONS_PER_SOURCE, MAX_GLOBAL_CONNECTIONS, MAX_PREAUTH_CONNECTIONS,
    MAX_PREAUTH_PER_SOURCE, ServerConfig,
};
use crate::coordinator::Coordinator;
use crate::facet::FacetWelcome;
use crate::postgres::PostgresState;

#[derive(Clone)]
pub struct AppState {
    pub(crate) inner: Arc<AppStateInner>,
}

pub(crate) struct AppStateInner {
    pub config: ServerConfig,
    pub backend: Option<Arc<PostgresState>>,
    pub coordinator: Arc<Coordinator>,
    pub(crate) open: AtomicUsize,
    pub(crate) preauth: AtomicUsize,
    per_source: Mutex<BTreeMap<IpAddr, SourceCounts>>,
    draining: AtomicBool,
    draining_send: watch::Sender<bool>,
}

#[derive(Default)]
struct SourceCounts {
    open: usize,
    preauth: usize,
}

impl AppState {
    pub fn disabled(config: ServerConfig) -> Self {
        Self::build(config, None)
    }

    pub fn postgres(config: ServerConfig, backend: Arc<PostgresState>) -> Self {
        Self::build(config, Some(backend))
    }

    fn build(config: ServerConfig, backend: Option<Arc<PostgresState>>) -> Self {
        let (draining_send, _) = watch::channel(false);
        let coordinator = backend
            .as_ref()
            .map(|backend| backend.coordinator())
            .unwrap_or_else(|| Arc::new(Coordinator::default()));
        Self {
            inner: Arc::new(AppStateInner {
                config,
                backend,
                coordinator,
                open: AtomicUsize::new(0),
                preauth: AtomicUsize::new(0),
                per_source: Mutex::new(BTreeMap::new()),
                draining: AtomicBool::new(false),
                draining_send,
            }),
        }
    }

    pub fn begin_draining(&self) {
        self.inner.draining.store(true, Ordering::Release);
        let _ = self.inner.draining_send.send(true);
    }

    pub fn is_draining(&self) -> bool {
        self.inner.draining.load(Ordering::Acquire)
    }

    pub(crate) fn subscribe_draining(&self) -> watch::Receiver<bool> {
        self.inner.draining_send.subscribe()
    }

    pub(crate) async fn admit(
        &self,
        ticket: &wire::AdmissionTicket,
        supported_minors: &[u16],
        origin: &str,
        host: &str,
        outbound: mpsc::Sender<wire::ServerEnvelope>,
        terminal: watch::Sender<Option<wire::DrainingReason>>,
    ) -> Result<(AdmissionGrant, FacetWelcome), AdmissionError> {
        let backend = self
            .inner
            .backend
            .as_ref()
            .ok_or(AdmissionError::Unavailable)?;
        backend
            .admit(ticket, supported_minors, origin, host, outbound, terminal)
            .await
    }

    pub(crate) async fn authorize_grant(&self, grant: &crate::admission::ControlGrant) -> bool {
        match self.inner.backend.as_ref() {
            Some(backend) => backend.authorize_grant(grant).await,
            None => false,
        }
    }

    pub(crate) async fn deliver_page(
        &self,
        target_character_id: wire::CharacterId,
        message_id: wire::MessageId,
        sender_character_id: wire::CharacterId,
        sender_name: wire::DisplayName,
        body: wire::SocialBody,
    ) -> bool {
        match self.inner.backend.as_ref() {
            Some(backend) => {
                backend
                    .deliver_page(
                        target_character_id,
                        message_id,
                        sender_character_id,
                        sender_name,
                        body,
                    )
                    .await
            }
            None => false,
        }
    }

    fn try_open(&self, source: IpAddr) -> Option<ConnectionGuard> {
        if self.is_draining()
            || self.inner.open.load(Ordering::Acquire) >= MAX_GLOBAL_CONNECTIONS
            || self.inner.preauth.load(Ordering::Acquire) >= MAX_PREAUTH_CONNECTIONS
        {
            return None;
        }
        let mut sources = self.inner.per_source.lock().ok()?;
        if self.is_draining()
            || self.inner.open.load(Ordering::Acquire) >= MAX_GLOBAL_CONNECTIONS
            || self.inner.preauth.load(Ordering::Acquire) >= MAX_PREAUTH_CONNECTIONS
        {
            return None;
        }
        let counts = sources.entry(source).or_default();
        if counts.open >= MAX_CONNECTIONS_PER_SOURCE || counts.preauth >= MAX_PREAUTH_PER_SOURCE {
            return None;
        }
        counts.open += 1;
        counts.preauth += 1;
        self.inner.open.fetch_add(1, Ordering::AcqRel);
        self.inner.preauth.fetch_add(1, Ordering::AcqRel);
        Some(ConnectionGuard {
            state: self.inner.clone(),
            source,
            authenticated: false,
        })
    }
}

pub(crate) struct ConnectionGuard {
    state: Arc<AppStateInner>,
    source: IpAddr,
    authenticated: bool,
}

impl ConnectionGuard {
    pub fn mark_authenticated(&mut self) {
        if self.authenticated {
            return;
        }
        self.authenticated = true;
        self.state.preauth.fetch_sub(1, Ordering::AcqRel);
        if let Ok(mut sources) = self.state.per_source.lock()
            && let Some(counts) = sources.get_mut(&self.source)
        {
            counts.preauth = counts.preauth.saturating_sub(1);
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.state.open.fetch_sub(1, Ordering::AcqRel);
        if !self.authenticated {
            self.state.preauth.fetch_sub(1, Ordering::AcqRel);
        }
        if let Ok(mut sources) = self.state.per_source.lock()
            && let Some(counts) = sources.get_mut(&self.source)
        {
            counts.open = counts.open.saturating_sub(1);
            if !self.authenticated {
                counts.preauth = counts.preauth.saturating_sub(1);
            }
            if counts.open == 0 {
                sources.remove(&self.source);
            }
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v4/socket", get(socket))
        .merge(crate::control_api::router())
        .with_state(state)
}

#[derive(Serialize)]
struct Health<'a> {
    status: &'a str,
    gameplay_ready: bool,
}

async fn health_live() -> Json<Health<'static>> {
    Json(Health {
        status: "live",
        gameplay_ready: false,
    })
}

async fn health_ready(State(state): State<AppState>) -> Response {
    let ready = state
        .inner
        .backend
        .as_ref()
        .is_some_and(|backend| backend.gameplay_ready())
        && !state.is_draining();
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(Health {
            status: if ready { "ready" } else { "unavailable" },
            gameplay_ready: ready,
        }),
    )
        .into_response()
}

async fn socket(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if state.is_draining() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if headers.contains_key("cookie") || headers.contains_key("authorization") {
        return StatusCode::FORBIDDEN.into_response();
    }
    let host = single_header(&headers, "host");
    let origin = single_header(&headers, "origin");
    if host.as_deref() != Some(state.inner.config.allowed_host.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if origin.as_deref() != Some(state.inner.config.allowed_origin.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let requested_protocol =
        single_header(&headers, "sec-websocket-protocol").is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim() == wire::WEBSOCKET_SUBPROTOCOL)
        });
    if !requested_protocol {
        return StatusCode::UPGRADE_REQUIRED.into_response();
    }
    let Some(guard) = state.try_open(peer.ip()) else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    ws.protocols([wire::WEBSOCKET_SUBPROTOCOL])
        .max_frame_size(wire::MAX_INPUT_BYTES)
        .max_message_size(wire::MAX_INPUT_BYTES)
        .on_upgrade(move |socket| {
            crate::connection::serve(socket, state, guard, host.unwrap(), origin.unwrap())
        })
}

pub(crate) fn single_header(headers: &HeaderMap, name: &str) -> Option<String> {
    let name = HeaderName::from_bytes(name.as_bytes()).ok()?;
    let mut values = headers.get_all(name).iter();
    let first = values.next()?.to_str().ok()?.to_string();
    values.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    fn state() -> AppState {
        AppState::disabled(ServerConfig::loopback_default())
    }

    fn source(octet: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, octet))
    }

    #[test]
    fn global_preauth_cap_rejects_then_reopens_after_authentication() {
        let state = state();
        let mut guards = (1..=MAX_PREAUTH_CONNECTIONS)
            .map(|index| {
                state
                    .try_open(source(u8::try_from(index).unwrap()))
                    .expect("every slot through the global preauth cap is admitted")
            })
            .collect::<Vec<_>>();
        assert!(
            state.try_open(source(200)).is_none(),
            "the first connection beyond the global preauth cap is rejected"
        );

        guards[0].mark_authenticated();
        guards.push(
            state
                .try_open(source(200))
                .expect("authentication releases one global preauth slot"),
        );
    }

    #[test]
    fn per_source_preauth_cap_rejects_then_reopens_after_authentication() {
        let state = state();
        let source = source(1);
        let mut guards = (0..MAX_PREAUTH_PER_SOURCE)
            .map(|_| {
                state
                    .try_open(source)
                    .expect("every slot through the source preauth cap is admitted")
            })
            .collect::<Vec<_>>();
        assert!(
            state.try_open(source).is_none(),
            "the first connection beyond the source preauth cap is rejected"
        );

        guards[0].mark_authenticated();
        guards.push(
            state
                .try_open(source)
                .expect("authentication releases one source preauth slot"),
        );
    }

    #[test]
    fn per_source_open_cap_rejects_then_reopens_after_drop() {
        let state = state();
        let source = source(1);
        let mut guards = Vec::new();
        for _ in 0..MAX_CONNECTIONS_PER_SOURCE {
            let mut guard = state
                .try_open(source)
                .expect("every slot through the source open cap is admitted");
            guard.mark_authenticated();
            guards.push(guard);
        }
        assert!(
            state.try_open(source).is_none(),
            "the first connection beyond the source open cap is rejected"
        );

        drop(guards.pop());
        let mut replacement = state
            .try_open(source)
            .expect("dropping a connection releases one source open slot");
        replacement.mark_authenticated();
    }

    #[test]
    fn global_open_cap_rejects_then_reopens_after_drop() {
        let state = state();
        let mut guards = Vec::new();
        for index in 1..=MAX_GLOBAL_CONNECTIONS {
            let mut guard = state
                .try_open(source(u8::try_from(index).unwrap()))
                .expect("every slot through the global open cap is admitted");
            guard.mark_authenticated();
            guards.push(guard);
        }
        assert!(
            state.try_open(source(200)).is_none(),
            "the first connection beyond the global open cap is rejected"
        );

        drop(guards.pop());
        let mut replacement = state
            .try_open(source(200))
            .expect("dropping a connection releases one global open slot");
        replacement.mark_authenticated();
    }
}
