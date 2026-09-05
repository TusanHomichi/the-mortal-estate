// Private transport proof for the persistence integration target.
async fn issue_ticket(
    address: SocketAddr,
    host: &str,
    origin: &str,
    token: &str,
    csrf_token: &wire::CsrfToken,
) -> wire::SocketTicketV1 {
    let response = post_json(
        address,
        host,
        origin,
        "/v4/socket-tickets",
        token,
        &wire::SocketTicketRequestV1 {
            csrf_token: csrf_token.clone(),
        },
    )
    .await;
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let ticket: wire::SocketTicketV1 = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(ticket.protocol_major, 1);
    assert_eq!(ticket.supported_minors, vec![8]);
    ticket
}

async fn post_json<T: serde::Serialize>(
    address: SocketAddr,
    host: &str,
    origin: &str,
    path: &str,
    token: &str,
    value: &T,
) -> HttpResponse {
    let body = serde_json::to_string(value).unwrap();
    http_request(
        address,
        host,
        origin,
        "POST",
        path,
        Some(token),
        Some(&body),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn post_json_with_csrf<T: serde::Serialize>(
    address: SocketAddr,
    host: &str,
    origin: &str,
    path: &str,
    token: &str,
    csrf_token: &wire::CsrfToken,
    value: &T,
) -> HttpResponse {
    let body = serde_json::to_string(value).unwrap();
    http_request_with_csrf(
        address,
        host,
        origin,
        "POST",
        path,
        Some(token),
        Some(csrf_token.expose_for_validation()),
        Some(&body),
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
    http_request_with_csrf(address, host, origin, method, path, token, None, body).await
}

#[allow(clippy::too_many_arguments)]
async fn http_request_with_csrf(
    address: SocketAddr,
    host: &str,
    origin: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    csrf_token: Option<&str>,
    body: Option<&str>,
) -> HttpResponse {
    let host = host.to_string();
    let origin = origin.to_string();
    let method = method.to_string();
    let path = path.to_string();
    let token = token.map(str::to_string);
    let csrf_token = csrf_token.map(str::to_string);
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
        if let Some(csrf_token) = csrf_token {
            request.push_str(&format!("X-Tme-Csrf: {csrf_token}\r\n"));
        }
        request.push_str("\r\n");
        let mut stream = TcpStream::connect(address).expect("HTTP connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
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

async fn connect_character(
    address: SocketAddr,
    host: &str,
    origin: &str,
    ticket: &wire::AdmissionTicket,
) -> (Socket, wire::ServerEnvelope) {
    let mut socket = open_socket(address, host, origin).await;
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

async fn open_socket(address: SocketAddr, host: &str, origin: &str) -> Socket {
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
    let (socket, response) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(
        response.headers().get("sec-websocket-protocol").unwrap(),
        wire::WEBSOCKET_SUBPROTOCOL
    );
    socket
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

fn welcome_parts(welcome: &wire::ServerEnvelope) -> (wire::ActorId, u64, u64) {
    match welcome {
        wire::ServerEnvelope::ServerWelcome {
            selected_minor,
            actor_id,
            control_epoch,
            world_revision,
            ..
        } => {
            assert_eq!(*selected_minor, wire::PROTOCOL_MINOR);
            (actor_id.clone(), control_epoch.get(), world_revision.get())
        }
        other => panic!("expected welcome, got {other:?}"),
    }
}

async fn receive_envelope(socket: &mut Socket) -> wire::ServerEnvelope {
    // The server sends keepalive pings on its own schedule. They are transport
    // frames, not envelopes, and reading one lets the client library answer it.
    // Treating a ping as a protocol violation made this helper fail whenever a
    // test happened to wait across the ping interval.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let message = tokio::time::timeout_at(deadline, socket.next())
            .await
            .expect("message deadline")
            .expect("socket open")
            .expect("message");
        match message {
            Message::Text(text) => {
                return serde_json::from_str(&text).expect("server envelope");
            }
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("expected text message, got {other:?}"),
        }
    }
}

async fn wait_for_draining(socket: &mut Socket) -> (wire::DrainingReason, bool) {
    loop {
        if let wire::ServerEnvelope::ServerDraining {
            reason,
            reconnect_hint,
        } = receive_envelope(socket).await
        {
            return (reason, reconnect_hint);
        }
    }
}
