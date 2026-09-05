use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tme_protocol as wire;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::Message;

use super::WallClockWatchdog;

pub const PASSWORD: &str = "correct horse ev battery";

pub type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub struct Client {
    pub token: String,
    pub csrf: wire::CsrfToken,
    pub socket: Socket,
    pub actor_id: wire::ActorId,
    pub control_epoch: u64,
    pub next_client_sequence: u64,
    pub facet_revision: u64,
    pub logical_time: u64,
    pub can_act: bool,
    pub pages_enabled: bool,
    pub observed_facet_ids: BTreeSet<String>,
    pub observed_actor_ids: BTreeSet<String>,
    pub observed_character_ids: BTreeSet<String>,
    pub observed_item_ids: BTreeSet<String>,
}

impl Client {
    pub fn apply(&mut self, envelope: &wire::ServerEnvelope) {
        let encoded = serde_json::to_value(envelope).expect("server envelope serializes");
        collect_observed_identities(
            &encoded,
            None,
            &mut self.observed_facet_ids,
            &mut self.observed_actor_ids,
            &mut self.observed_character_ids,
            &mut self.observed_item_ids,
        );
        match envelope {
            wire::ServerEnvelope::ServerWelcome {
                actor_id,
                control_epoch,
                world_revision,
                frame,
                ..
            } => {
                self.actor_id = actor_id.clone();
                self.control_epoch = control_epoch.get();
                self.facet_revision = world_revision.get();
                self.logical_time = frame.logical_time.get();
                self.can_act = frame.can_act;
                self.pages_enabled = frame.social.pages_enabled;
            }
            wire::ServerEnvelope::StateUpdate {
                world_revision,
                frame,
                ..
            } => {
                self.facet_revision = world_revision.get();
                self.logical_time = frame.logical_time.get();
                self.can_act = frame.can_act;
                self.pages_enabled = frame.social.pages_enabled;
            }
            _ => {}
        }
    }
}

pub async fn login_select_connect(
    address: SocketAddr,
    host: &str,
    origin: &str,
    username: &str,
    character_id: wire::CharacterId,
) -> Client {
    let login = http_request(
        address,
        host,
        origin,
        "POST",
        "/v4/login",
        None,
        Some(
            &serde_json::to_string(&wire::LoginRequestV1 {
                username: wire::Username::new(username).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(login.status, 200, "login failed for {username}");
    assert!(!login.headers.contains_key("set-cookie"));
    let login: wire::LoginResponseV1 = serde_json::from_slice(&login.body).unwrap();
    let token = login.session_token.expose_for_validation().to_string();
    let bootstrap = login.bootstrap;
    let csrf = bootstrap.csrf_token;
    let selection = post_json(
        address,
        host,
        origin,
        "/v4/characters/select",
        &token,
        &wire::CharacterSelectRequestV1 {
            csrf_token: csrf.clone(),
            character_id,
        },
    )
    .await;
    assert_eq!(selection.status, 200, "character selection failed");
    let ticket = issue_ticket(address, host, origin, &token, &csrf).await;
    let (socket, welcome) = connect_ticket(address, host, origin, &ticket).await;
    let mut client = Client {
        token,
        csrf,
        socket,
        actor_id: wire::ActorId::new("temporary").unwrap(),
        control_epoch: 0,
        next_client_sequence: 1,
        facet_revision: 0,
        logical_time: 0,
        can_act: false,
        pages_enabled: false,
        observed_facet_ids: BTreeSet::new(),
        observed_actor_ids: BTreeSet::new(),
        observed_character_ids: BTreeSet::new(),
        observed_item_ids: BTreeSet::new(),
    };
    client.apply(&welcome);
    client
}

fn collect_observed_identities(
    value: &serde_json::Value,
    key: Option<&str>,
    facets: &mut BTreeSet<String>,
    actors: &mut BTreeSet<String>,
    characters: &mut BTreeSet<String>,
    items: &mut BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                collect_observed_identities(value, Some(key), facets, actors, characters, items);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_observed_identities(value, key, facets, actors, characters, items);
            }
        }
        serde_json::Value::String(value) => match key {
            Some("facet_id") => {
                facets.insert(value.clone());
            }
            Some(key) if key == "actor_id" || key.ends_with("_actor_id") => {
                actors.insert(value.clone());
            }
            Some(key) if key == "character_id" || key.ends_with("_character_id") => {
                characters.insert(value.clone());
            }
            Some("item_instance_id") => {
                items.insert(value.clone());
            }
            _ => {}
        },
        _ => {}
    }
}

pub async fn replacement_connection(
    address: SocketAddr,
    host: &str,
    origin: &str,
    client: &Client,
) -> (Socket, wire::ServerEnvelope) {
    let ticket = issue_ticket(address, host, origin, &client.token, &client.csrf).await;
    connect_ticket(address, host, origin, &ticket).await
}

pub fn command(
    client: &Client,
    command_id: wire::CommandId,
    observed_facet_revision: u64,
    intent: wire::Intent,
) -> wire::ClientCommandEnvelope {
    wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(client.control_epoch),
        client_sequence: wire::DecimalU64::new(client.next_client_sequence),
        observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
        actor_id: client.actor_id.clone(),
        intent,
    }
}

pub async fn send_command(socket: &mut Socket, command: &wire::ClientCommandEnvelope) {
    socket
        .send(Message::Text(
            serde_json::to_string(command).unwrap().into(),
        ))
        .await
        .unwrap();
}

pub async fn receive_result(
    client: &mut Client,
    command_id: wire::CommandId,
) -> wire::ServerEnvelope {
    loop {
        let envelope = receive_envelope(&mut client.socket).await;
        client.apply(&envelope);
        match &envelope {
            wire::ServerEnvelope::CommandResult {
                command_id: received,
                ..
            } if *received == command_id => return envelope,
            wire::ServerEnvelope::CommandResult {
                command_id: received,
                ..
            } => panic!(
                "received terminal result for unexpected command {received} while awaiting {command_id}"
            ),
            wire::ServerEnvelope::StateUpdate { .. } => {}
            other => panic!(
                "received unexpected envelope while awaiting command {command_id} at client sequence {}: {other:?}",
                client.next_client_sequence,
            ),
        }
    }
}

pub async fn assert_no_terminal_result(client: &mut Client) {
    let mut watchdog = WallClockWatchdog::start(Duration::from_millis(250));
    loop {
        tokio::select! {
            biased;
            message = client.socket.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    let envelope: wire::ServerEnvelope =
                        serde_json::from_str(&text).expect("valid server envelope");
                    client.apply(&envelope);
                    match envelope {
                        wire::ServerEnvelope::StateUpdate { .. } => {}
                        wire::ServerEnvelope::CommandResult { command_id, .. } => {
                            panic!("received extra terminal result for command {command_id}")
                        }
                        other => panic!(
                            "received unexpected envelope while checking terminal count: {other:?}"
                        ),
                    }
                }
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                Some(Ok(message)) => {
                    panic!("unexpected WebSocket message while checking terminal count: {message:?}")
                }
                Some(Err(error)) => {
                    panic!("WebSocket failed while checking terminal count: {error}")
                }
                None => panic!("WebSocket closed while checking terminal count"),
            },
            expired = &mut watchdog.expired => {
                expired.expect("terminal-count watchdog sender");
                return;
            }
        }
    }
}

pub async fn wait_for_state_sequence(client: &mut Client, expected: u64) {
    loop {
        let envelope = receive_envelope(&mut client.socket).await;
        client.apply(&envelope);
        match envelope {
            wire::ServerEnvelope::StateUpdate {
                server_sequence, ..
            } if server_sequence.get() == expected => return,
            wire::ServerEnvelope::StateUpdate { .. } => {}
            other => panic!(
                "received unexpected envelope while awaiting state sequence {expected}: {other:?}"
            ),
        }
    }
}

pub async fn wait_until_ready_after(client: &mut Client, logical_time: u64) {
    while !client.can_act || client.logical_time <= logical_time {
        let envelope = receive_envelope(&mut client.socket).await;
        client.apply(&envelope);
        if !matches!(envelope, wire::ServerEnvelope::StateUpdate { .. }) {
            panic!(
                "received unexpected envelope while awaiting ready state after {logical_time}: {envelope:?}"
            );
        }
    }
}

pub async fn wait_for_draining(client: &mut Client) -> wire::DrainingReason {
    loop {
        let envelope = receive_envelope(&mut client.socket).await;
        client.apply(&envelope);
        if let wire::ServerEnvelope::ServerDraining { reason, .. } = envelope {
            return reason;
        }
    }
}

pub async fn receive_envelope(socket: &mut Socket) -> wire::ServerEnvelope {
    let mut watchdog = WallClockWatchdog::start(Duration::from_secs(30));
    loop {
        let message = tokio::select! {
            message = socket.next() => message
                .expect("WebSocket remained open")
                .expect("WebSocket message"),
            expired = &mut watchdog.expired => {
                expired.expect("WebSocket wall-clock watchdog sender");
                panic!("WebSocket message exceeded its real wall-clock deadline");
            }
        };
        match message {
            Message::Text(text) => {
                return serde_json::from_str(&text).expect("valid server envelope");
            }
            Message::Ping(_) | Message::Pong(_) => {}
            other => panic!("expected a text WebSocket message, got {other:?}"),
        }
    }
}

pub async fn issue_ticket(
    address: SocketAddr,
    host: &str,
    origin: &str,
    token: &str,
    csrf: &wire::CsrfToken,
) -> wire::AdmissionTicket {
    let response = post_json(
        address,
        host,
        origin,
        "/v4/socket-tickets",
        token,
        &wire::SocketTicketRequestV1 {
            csrf_token: csrf.clone(),
        },
    )
    .await;
    assert_eq!(response.status, 200, "ticket issue failed");
    serde_json::from_slice::<wire::SocketTicketV1>(&response.body)
        .unwrap()
        .ticket
}

pub async fn connect_ticket(
    address: SocketAddr,
    host: &str,
    origin: &str,
    ticket: &wire::AdmissionTicket,
) -> (Socket, wire::ServerEnvelope) {
    let mut request = format!("ws://{address}/v4/socket")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("host", HeaderValue::from_str(host).unwrap());
    request
        .headers_mut()
        .insert("origin", HeaderValue::from_str(origin).unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_static(wire::WEBSOCKET_SUBPROTOCOL),
    );
    let (mut socket, response) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(
        response.headers().get("sec-websocket-protocol").unwrap(),
        wire::WEBSOCKET_SUBPROTOCOL
    );
    socket
        .send(Message::Text(
            serde_json::to_string(&wire::ClientHelloEnvelope::ClientHello {
                ticket: ticket.clone(),
                supported_minors: vec![wire::PROTOCOL_MINOR],
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    let welcome = receive_envelope(&mut socket).await;
    (socket, welcome)
}

pub async fn operations_status(address: SocketAddr) -> (u16, serde_json::Value) {
    let host = address.to_string();
    let response = http_request(
        address,
        &host,
        &format!("http://{host}"),
        "GET",
        "/internal/status",
        None,
        None,
    )
    .await;
    (
        response.status,
        serde_json::from_slice(&response.body).expect("valid operations status"),
    )
}

async fn post_json<T: serde::Serialize>(
    address: SocketAddr,
    host: &str,
    origin: &str,
    path: &str,
    token: &str,
    value: &T,
) -> HttpResponse {
    http_request(
        address,
        host,
        origin,
        "POST",
        path,
        Some(token),
        Some(&serde_json::to_string(value).unwrap()),
    )
    .await
}

struct HttpResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

async fn http_request(
    address: SocketAddr,
    host: &str,
    origin: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> HttpResponse {
    let host = host.to_string();
    let origin = origin.to_string();
    let method = method.to_string();
    let path = path.to_string();
    let token = token.map(str::to_string);
    let body = body.unwrap_or_default().as_bytes().to_vec();
    tokio::task::spawn_blocking(move || {
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\nConnection: close\r\n"
        );
        if !body.is_empty() {
            request.push_str("Content-Type: application/json\r\n");
            request.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        if let Some(token) = token {
            request.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        request.push_str("\r\n");
        let mut stream = TcpStream::connect(address).expect("HTTP connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        stream.write_all(&body).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        parse_http_response(response)
    })
    .await
    .unwrap()
}

fn parse_http_response(response: Vec<u8>) -> HttpResponse {
    let boundary = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("HTTP header boundary");
    let header = std::str::from_utf8(&response[..boundary]).unwrap();
    let mut lines = header.split("\r\n");
    let status = lines
        .next()
        .expect("status line")
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    HttpResponse {
        status,
        headers,
        body: response[(boundary + 4)..].to_vec(),
    }
}
