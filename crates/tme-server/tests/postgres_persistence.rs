use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use sqlx::{Row, ValueRef};
use tme_protocol as wire;
use tme_server::store::PostgresStore;
use tme_server::{
    AppState, PostgresBootstrap, PostgresCharacterBootstrap, PostgresState, PostgresWorldBootstrap,
    ServerConfig,
};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::Message;
use uuid::Uuid;

const USERNAME: &str = "durable_tester";
const PASSWORD: &str = "correct horse durable battery";

// Each file owns a bounded proof responsibility; all share this test-only fixture scope.
include!("persistence/durability.rs");
include!("persistence/social.rs");
include!("persistence/mark_schedule.rs");
include!("persistence/socket_commands.rs");
include!("persistence/fixtures.rs");
include!("persistence/karma.rs");
include!("persistence/consequences.rs");
include!("persistence/worlds.rs");
include!("persistence/http_lifecycle.rs");
include!("persistence/transport.rs");
