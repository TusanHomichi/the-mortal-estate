use std::net::{IpAddr, SocketAddr};

use axum::body::Bytes;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::rejection::BytesRejection;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tme_protocol as wire;

use crate::http::{AppState, single_header};
use crate::postgres::{LoginError, SESSION_ABSOLUTE, SessionError};

const SESSION_COOKIE_NAME: &str = "__Host-tme_session";

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/v3/login", post(login))
        .route("/v3/session", get(session_bootstrap))
        .route("/v3/logout", post(logout))
        .route("/v3/characters/select", post(select_character))
        .route("/v3/socket-tickets", post(issue_socket_ticket))
        .route(
            "/v3/player-kill-marks/{mark_id}/forgive",
            post(forgive_player_kill_mark),
        )
        .layer(DefaultBodyLimit::max(wire::MAX_CONTROL_INPUT_BYTES))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let context = match control_context(&state, peer, &headers, true) {
        Ok(context) => context,
        Err(failure) => return failure.into_response(),
    };
    let body = match control_body(body) {
        Ok(body) => body,
        Err(failure) => return failure.into_response(),
    };
    let request = match wire::decode_login_request(&body) {
        Ok(request) => request,
        Err(_) => return malformed(StatusCode::BAD_REQUEST),
    };
    let Some(backend) = &state.inner.backend else {
        return control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        );
    };
    match backend.login(context.source, request).await {
        Ok(success) => {
            let cookie = format!(
                "{SESSION_COOKIE_NAME}={}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age={}",
                success.session_cookie.expose(),
                SESSION_ABSOLUTE.as_secs()
            );
            let mut response = (StatusCode::OK, Json(success.bootstrap)).into_response();
            response.headers_mut().insert(
                SET_COOKIE,
                HeaderValue::from_str(&cookie).expect("generated cookie is a valid header"),
            );
            response
        }
        Err(LoginError::InvalidCredentials) => control_error(
            StatusCode::UNAUTHORIZED,
            wire::ControlErrorCode::InvalidCredentials,
        ),
        Err(LoginError::RateLimited) => control_error(
            StatusCode::TOO_MANY_REQUESTS,
            wire::ControlErrorCode::RateLimited,
        ),
        Err(LoginError::Unavailable) => control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        ),
    }
}

async fn session_bootstrap(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(failure) = control_context(&state, peer, &headers, false) {
        return failure.into_response();
    }
    let cookie = match session_cookie(&headers) {
        Ok(cookie) => cookie,
        Err(failure) => return failure.into_response(),
    };
    let Some(backend) = &state.inner.backend else {
        return control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        );
    };
    match backend.session_bootstrap(&cookie).await {
        Ok(bootstrap) => (StatusCode::OK, Json(bootstrap)).into_response(),
        Err(error) => session_error(error),
    }
}

async fn logout(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    if let Err(failure) = control_context(&state, peer, &headers, true) {
        return failure.into_response();
    }
    let cookie = match session_cookie(&headers) {
        Ok(cookie) => cookie,
        Err(failure) => return failure.into_response(),
    };
    let body = match control_body(body) {
        Ok(body) => body,
        Err(failure) => return failure.into_response(),
    };
    let request = match wire::decode_logout_request(&body) {
        Ok(request) => request,
        Err(_) => return malformed(StatusCode::BAD_REQUEST),
    };
    let Some(backend) = &state.inner.backend else {
        return control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        );
    };
    match backend.logout(&cookie, request).await {
        Ok(()) => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            response.headers_mut().insert(
                SET_COOKIE,
                HeaderValue::from_static(
                    "__Host-tme_session=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0",
                ),
            );
            response
        }
        Err(error) => session_error(error),
    }
}

async fn select_character(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    if let Err(failure) = control_context(&state, peer, &headers, true) {
        return failure.into_response();
    }
    let cookie = match session_cookie(&headers) {
        Ok(cookie) => cookie,
        Err(failure) => return failure.into_response(),
    };
    let body = match control_body(body) {
        Ok(body) => body,
        Err(failure) => return failure.into_response(),
    };
    let request = match wire::decode_character_select_request(&body) {
        Ok(request) => request,
        Err(_) => return malformed(StatusCode::BAD_REQUEST),
    };
    let Some(backend) = &state.inner.backend else {
        return control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        );
    };
    match backend.select_character(&cookie, request).await {
        Ok(selection) => (StatusCode::OK, Json(selection)).into_response(),
        Err(error) => session_error(error),
    }
}

async fn issue_socket_ticket(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let context = match control_context(&state, peer, &headers, true) {
        Ok(context) => context,
        Err(failure) => return failure.into_response(),
    };
    let cookie = match session_cookie(&headers) {
        Ok(cookie) => cookie,
        Err(failure) => return failure.into_response(),
    };
    let body = match control_body(body) {
        Ok(body) => body,
        Err(failure) => return failure.into_response(),
    };
    let request = match wire::decode_socket_ticket_request(&body) {
        Ok(request) => request,
        Err(_) => return malformed(StatusCode::BAD_REQUEST),
    };
    let Some(backend) = &state.inner.backend else {
        return control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        );
    };
    match backend
        .issue_ticket(&cookie, request, &context.origin, &context.host)
        .await
    {
        Ok(ticket) => (StatusCode::OK, Json(ticket)).into_response(),
        Err(error) => session_error(error),
    }
}

async fn forgive_player_kill_mark(
    State(state): State<AppState>,
    Path(mark_id): Path<String>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    if let Err(failure) = control_context(&state, peer, &headers, true) {
        return failure.into_response();
    }
    let cookie = match session_cookie(&headers) {
        Ok(cookie) => cookie,
        Err(failure) => return failure.into_response(),
    };
    let csrf_token = match single_header(&headers, "x-tme-csrf")
        .and_then(|value| wire::CsrfToken::new(value).ok())
    {
        Some(token) => token,
        None => {
            return control_error(StatusCode::FORBIDDEN, wire::ControlErrorCode::CsrfRejected);
        }
    };
    let mark_uuid = match uuid::Uuid::parse_str(&mark_id) {
        Ok(value) if value.hyphenated().to_string() == mark_id => value,
        _ => {
            return control_error(
                StatusCode::CONFLICT,
                wire::ControlErrorCode::ForgivenessUnavailable,
            );
        }
    };
    let mark_id = match wire::PlayerKillMarkId::new(mark_uuid) {
        Ok(mark_id) => mark_id,
        Err(_) => {
            return control_error(
                StatusCode::CONFLICT,
                wire::ControlErrorCode::ForgivenessUnavailable,
            );
        }
    };
    let body = match control_body(body) {
        Ok(body) => body,
        Err(failure) => return failure.into_response(),
    };
    let request = match wire::decode_forgive_player_kill_mark_request(&body) {
        Ok(request) => request,
        Err(_) => return malformed(StatusCode::BAD_REQUEST),
    };
    let Some(backend) = &state.inner.backend else {
        return control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        );
    };
    match backend
        .forgive_player_kill_mark(&cookie, &csrf_token, mark_id, request)
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(error) => session_error(error),
    }
}

struct ControlContext {
    source: IpAddr,
    host: String,
    origin: String,
}

#[derive(Clone, Copy)]
struct ControlFailure {
    status: StatusCode,
    code: wire::ControlErrorCode,
}

impl ControlFailure {
    fn into_response(self) -> Response {
        control_error(self.status, self.code)
    }
}

fn malformed(status: StatusCode) -> Response {
    control_error(status, wire::ControlErrorCode::MalformedRequest)
}

fn control_body(body: Result<Bytes, BytesRejection>) -> Result<Bytes, ControlFailure> {
    body.map_err(|_| ControlFailure {
        status: StatusCode::PAYLOAD_TOO_LARGE,
        code: wire::ControlErrorCode::MalformedRequest,
    })
}

fn control_context(
    state: &AppState,
    peer: SocketAddr,
    headers: &HeaderMap,
    require_json: bool,
) -> Result<ControlContext, ControlFailure> {
    let host = single_header(headers, "host").ok_or_else(forbidden)?;
    let origin = single_header(headers, "origin").ok_or_else(forbidden)?;
    if host != state.inner.config.allowed_host || origin != state.inner.config.allowed_origin {
        return Err(forbidden());
    }
    if require_json
        && single_header(headers, CONTENT_TYPE.as_str()).as_deref() != Some("application/json")
    {
        return Err(ControlFailure {
            status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
            code: wire::ControlErrorCode::MalformedRequest,
        });
    }
    let forwarded_for = single_header(headers, "x-forwarded-for");
    let forwarded_proto = single_header(headers, "x-forwarded-proto");
    let forwarded_host = single_header(headers, "x-forwarded-host");
    let source = match (forwarded_for, forwarded_proto, forwarded_host) {
        (None, None, None) => peer.ip(),
        (Some(source), Some(proto), Some(forwarded_host))
            if peer.ip().is_loopback()
                && !source.contains(',')
                && proto == "https"
                && forwarded_host == state.inner.config.allowed_host =>
        {
            source.parse::<IpAddr>().map_err(|_| forbidden())?
        }
        _ => return Err(forbidden()),
    };
    Ok(ControlContext {
        source,
        host,
        origin,
    })
}

fn session_cookie(headers: &HeaderMap) -> Result<String, ControlFailure> {
    let header = single_header(headers, COOKIE.as_str()).ok_or(ControlFailure {
        status: StatusCode::UNAUTHORIZED,
        code: wire::ControlErrorCode::AuthenticationRequired,
    })?;
    let values = header
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .filter_map(|(name, value)| (name == SESSION_COOKIE_NAME).then_some(value))
        .collect::<Vec<_>>();
    if values.len() != 1 || wire::AdmissionTicket::new(values[0]).is_err() {
        return Err(ControlFailure {
            status: StatusCode::UNAUTHORIZED,
            code: wire::ControlErrorCode::AuthenticationRequired,
        });
    }
    Ok(values[0].to_string())
}

fn forbidden() -> ControlFailure {
    ControlFailure {
        status: StatusCode::FORBIDDEN,
        code: wire::ControlErrorCode::MalformedRequest,
    }
}

fn control_error(status: StatusCode, code: wire::ControlErrorCode) -> Response {
    (status, Json(wire::ControlErrorV1 { code })).into_response()
}

fn session_error(error: SessionError) -> Response {
    match error {
        SessionError::AuthenticationRequired => control_error(
            StatusCode::UNAUTHORIZED,
            wire::ControlErrorCode::AuthenticationRequired,
        ),
        SessionError::CsrfRejected => {
            control_error(StatusCode::FORBIDDEN, wire::ControlErrorCode::CsrfRejected)
        }
        SessionError::CharacterNotOwned => control_error(
            StatusCode::FORBIDDEN,
            wire::ControlErrorCode::CharacterNotOwned,
        ),
        SessionError::CharacterNotSelected => control_error(
            StatusCode::CONFLICT,
            wire::ControlErrorCode::CharacterNotSelected,
        ),
        SessionError::GameplayMarkLocked => control_error(
            StatusCode::LOCKED,
            wire::ControlErrorCode::GameplayMarkLocked,
        ),
        SessionError::ForgivenessUnavailable => control_error(
            StatusCode::CONFLICT,
            wire::ControlErrorCode::ForgivenessUnavailable,
        ),
        SessionError::Unavailable => control_error(
            StatusCode::SERVICE_UNAVAILABLE,
            wire::ControlErrorCode::Unavailable,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerConfig;

    fn state() -> AppState {
        AppState::disabled(
            ServerConfig::new(
                "127.0.0.1:3000".parse().unwrap(),
                "tme.test",
                "https://tme.test",
            )
            .unwrap(),
        )
    }

    fn headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("tme.test"));
        headers.insert("origin", HeaderValue::from_static("https://tme.test"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    #[test]
    fn the_control_surface_offers_no_world_selection_route() {
        // D4: one canonical world. No route may let a player list or choose
        // among divergent copies of it.
        let routes = format!("{:?}", router());
        for retired in ["switch-facet", "facets", "switch_facet"] {
            assert!(
                !routes.contains(retired),
                "control router still exposes {retired}"
            );
        }
    }

    #[test]
    fn direct_and_complete_loopback_proxy_sources_are_exact() {
        let state = state();
        let Ok(direct) =
            control_context(&state, "127.0.0.1:4000".parse().unwrap(), &headers(), true)
        else {
            panic!("direct control context should be accepted");
        };
        assert_eq!(direct.source, IpAddr::from([127, 0, 0, 1]));

        let mut proxied = headers();
        proxied.insert("x-forwarded-for", HeaderValue::from_static("192.0.2.40"));
        proxied.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        proxied.insert("x-forwarded-host", HeaderValue::from_static("tme.test"));
        let Ok(context) =
            control_context(&state, "127.0.0.1:4000".parse().unwrap(), &proxied, true)
        else {
            panic!("complete trusted proxy context should be accepted");
        };
        assert_eq!(context.source, IpAddr::from([192, 0, 2, 40]));
    }

    #[test]
    fn partial_spoofed_comma_or_duplicate_forwarding_fails_closed() {
        let state = state();
        let mut partial = headers();
        partial.insert("x-forwarded-for", HeaderValue::from_static("192.0.2.40"));
        assert!(
            control_context(&state, "127.0.0.1:4000".parse().unwrap(), &partial, true).is_err()
        );

        let mut comma = headers();
        comma.insert(
            "x-forwarded-for",
            HeaderValue::from_static("192.0.2.40, 192.0.2.41"),
        );
        comma.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        comma.insert("x-forwarded-host", HeaderValue::from_static("tme.test"));
        assert!(control_context(&state, "127.0.0.1:4000".parse().unwrap(), &comma, true).is_err());

        let mut duplicate = headers();
        duplicate.append("origin", HeaderValue::from_static("https://tme.test"));
        assert!(
            control_context(&state, "127.0.0.1:4000".parse().unwrap(), &duplicate, true).is_err()
        );

        let mut spoofed = headers();
        spoofed.insert("x-forwarded-for", HeaderValue::from_static("192.0.2.40"));
        spoofed.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        spoofed.insert("x-forwarded-host", HeaderValue::from_static("tme.test"));
        assert!(
            control_context(&state, "198.51.100.9:4000".parse().unwrap(), &spoofed, true).is_err()
        );
    }
}
