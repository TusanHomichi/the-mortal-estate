pub mod admission;
pub mod auth;
pub mod config;
mod connection;
mod control_api;
mod coordinator;
pub mod facet;
pub mod http;
pub mod operations;
pub mod operator;
pub mod postgres;
pub mod production;
pub mod protocol_v1;
pub mod runtime;
pub mod scheduler;
pub mod store;
pub mod telemetry;

pub use config::ServerConfig;
pub use http::AppState;
pub use postgres::{
    PostgresBootstrap, PostgresCharacterBootstrap, PostgresState, PostgresWorldBootstrap,
};

#[cfg(test)]
mod tests {
    use tme_protocol as wire;

    use super::*;

    #[test]
    fn config_and_protocol_bounds_are_locked() {
        assert!(
            ServerConfig::new(
                "0.0.0.0:3000".parse().unwrap(),
                "localhost:3000",
                "http://localhost:3000"
            )
            .is_err()
        );
        assert_eq!(config::MAX_GLOBAL_CONNECTIONS, 64);
        assert_eq!(config::MAX_CONNECTIONS_PER_SOURCE, 16);
        assert_eq!(config::MAX_PREAUTH_CONNECTIONS, 16);
        assert_eq!(config::MAX_PREAUTH_PER_SOURCE, 8);
        assert_eq!(config::FACET_MAILBOX_CAPACITY, 64);
        assert_eq!(config::OUTBOUND_QUEUE_CAPACITY, 32);
        assert_eq!(config::COMMAND_RATE_BURST, 20);
        assert_eq!(config::COMMAND_RATE_PER_SECOND, 20);
        assert_eq!(config::DRAIN_TIMEOUT, std::time::Duration::from_secs(5));
        assert_eq!(wire::PROTOCOL_MINOR, 8);
        assert_eq!(wire::CONTROL_API_VERSION, 3);
        assert_eq!(wire::MAX_CONTROL_INPUT_BYTES, 16 * 1024);
        assert_eq!(wire::MAX_CONTROL_JSON_NESTING, 16);
        assert_eq!(postgres::MAX_ACCOUNTS, 64);
        assert_eq!(postgres::MAX_CHARACTERS_PER_ACCOUNT, 8);
        assert_eq!(postgres::MAX_SESSIONS_PER_ACCOUNT, 8);
        assert_eq!(postgres::MAX_TICKETS_PER_SESSION, 16);
        assert_eq!(postgres::MAX_DURABLE_TICKET_RECORDS, 8_192);
        assert_eq!(postgres::MAX_RECEIPTS_PER_ACCOUNT, 65_536);
        assert_eq!(postgres::ARGON2_CONCURRENCY, 2);
        assert!(!AppState::disabled(ServerConfig::loopback_default()).is_draining());
    }

    #[test]
    fn protocol_conversion_is_exhaustive_for_every_direction() {
        let directions = [
            (wire::Direction::North, tme_rules::Direction::North),
            (wire::Direction::Northeast, tme_rules::Direction::Northeast),
            (wire::Direction::East, tme_rules::Direction::East),
            (wire::Direction::Southeast, tme_rules::Direction::Southeast),
            (wire::Direction::South, tme_rules::Direction::South),
            (wire::Direction::Southwest, tme_rules::Direction::Southwest),
            (wire::Direction::West, tme_rules::Direction::West),
            (wire::Direction::Northwest, tme_rules::Direction::Northwest),
        ];
        for (wire_direction, rules_direction) in directions {
            assert_eq!(
                protocol_v1::intent(&wire::Intent::MovePath {
                    path: vec![wire_direction]
                }),
                protocol_v1::RulesIntent::Gameplay(tme_rules::PlayerIntent::MovePath(vec![
                    rules_direction
                ]))
            );
        }
        assert_eq!(
            protocol_v1::intent(&wire::Intent::Wait),
            protocol_v1::RulesIntent::Gameplay(tme_rules::PlayerIntent::Wait)
        );
    }
}
