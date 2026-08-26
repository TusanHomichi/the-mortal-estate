use std::net::SocketAddr;
use std::time::Duration;

pub const MAX_GLOBAL_CONNECTIONS: usize = 64;
pub const MAX_CONNECTIONS_PER_SOURCE: usize = 16;
pub const MAX_PREAUTH_CONNECTIONS: usize = 16;
pub const MAX_PREAUTH_PER_SOURCE: usize = 8;
pub const HELLO_DEADLINE: Duration = Duration::from_secs(10);
pub const TICKET_LIFETIME: Duration = Duration::from_secs(30);
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
pub const PING_INTERVAL: Duration = Duration::from_secs(20);
pub const PONG_GRACE: Duration = Duration::from_secs(10);
pub const COMMAND_RATE_BURST: u32 = 20;
pub const COMMAND_RATE_PER_SECOND: u32 = 20;
pub const SOCIAL_RATE_BURST: u32 = 5;
pub const SOCIAL_RATE_PER_SECOND: f64 = 0.5;
pub const SOCIAL_DEDUPE_CAPACITY: usize = 4_096;
pub const FACET_MAILBOX_CAPACITY: usize = 64;
pub const OUTBOUND_QUEUE_CAPACITY: usize = 32;
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub allowed_host: String,
    pub allowed_origin: String,
}

impl ServerConfig {
    pub fn new(
        listen: SocketAddr,
        allowed_host: impl Into<String>,
        allowed_origin: impl Into<String>,
    ) -> Result<Self, String> {
        if !listen.ip().is_loopback() {
            return Err(
                "The Mortal Estate server listener must use a loopback address".to_string(),
            );
        }
        let allowed_host = allowed_host.into();
        let allowed_origin = allowed_origin.into();
        if allowed_host.is_empty() || allowed_origin.is_empty() {
            return Err("Host and Origin allowlists must be non-empty".to_string());
        }
        Ok(Self {
            listen,
            allowed_host,
            allowed_origin,
        })
    }

    pub fn loopback_default() -> Self {
        Self::new(
            "127.0.0.1:3000".parse().expect("static loopback address"),
            "127.0.0.1:3000",
            "http://127.0.0.1:3000",
        )
        .expect("static loopback config")
    }
}
