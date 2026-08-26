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

#[test]
fn credential_and_ticket_debug_output_is_redacted() {
    let password = wire::Password::new(PASSWORD).unwrap();
    let csrf = wire::CsrfToken::new("A".repeat(43)).unwrap();
    let ticket = wire::AdmissionTicket::new("E".repeat(43)).unwrap();
    assert_eq!(format!("{password:?}"), "Password([REDACTED])");
    assert_eq!(format!("{csrf:?}"), "CsrfToken([REDACTED])");
    assert_eq!(format!("{ticket:?}"), "AdmissionTicket([REDACTED])");
}

#[tokio::test]
#[ignore = "requires an EV runner-owned PostgreSQL database"]
async fn postgres_bootstrap_command_and_restart_are_durable() {
    let database_url = std::env::var("TME_TEST_DATABASE_URL")
        .expect("the exact PostgreSQL runner must provide TME_TEST_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    sqlx::raw_sql(
        "TRUNCATE tme.audit_events,tme.command_receipts,tme.socket_tickets,\
         tme.sessions,tme.characters,tme.facets,tme.account_credentials,\
         tme.accounts RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();

    two_client_social_socket_smoke(&database_url, &pool).await;
    sqlx::raw_sql(
        "TRUNCATE tme.audit_events,tme.command_receipts,tme.socket_tickets,\
         tme.sessions,tme.characters,tme.facets,tme.account_credentials,\
         tme.accounts RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();

    let account_id = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let character_id = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let world_id = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account(&pool, account_id).await;

    let first = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    assert!(first.gameplay_ready());
    let login = first
        .login(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            wire::LoginRequestV1 {
                username: wire::Username::new(USERNAME).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            },
        )
        .await
        .unwrap();
    let cookie = login.session_cookie.expose().to_string();
    assert_eq!(login.bootstrap.control_api_version, 3);
    let selection = first
        .select_character(
            &cookie,
            wire::CharacterSelectRequestV1 {
                csrf_token: login.bootstrap.csrf_token,
                character_id,
            },
        )
        .await
        .unwrap();
    assert_eq!(selection.control_api_version, 3);
    let session_bootstrap = first.session_bootstrap(&cookie).await.unwrap();
    assert_eq!(session_bootstrap.control_api_version, 3);
    let ticket = first
        .issue_ticket(
            &cookie,
            wire::SocketTicketRequestV1 {
                csrf_token: session_bootstrap.csrf_token,
            },
            "https://localhost:3000",
            "localhost:3000",
        )
        .await
        .unwrap();
    assert_eq!(ticket.protocol_major, 1);
    assert_eq!(ticket.supported_minors, vec![8]);
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (grant, welcome) = first
        .admit(
            &ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
        .unwrap();
    assert!(welcome.server_sequence > 0);
    let observation_center_before_restart = welcome.frame.observation_center.clone();
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let actor_id = wire::ActorId::new("player").unwrap();
    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(grant.control.control_epoch),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(0),
        actor_id: actor_id.clone(),
        intent: wire::Intent::Wait,
    };
    let request_digest: [u8; 32] = Sha256::digest(serde_json::to_vec(&command).unwrap()).into();
    let reply = grant
        .facet
        .try_command(tme_server::facet::FacetCommand {
            connection_id: grant.control.connection_id,
            account_id,
            session_id: grant.control.session_id,
            character_id,
            command_id,
            control_epoch: grant.control.control_epoch,
            client_sequence: 1,
            observed_facet_revision: 0,
            actor_id,
            intent: wire::Intent::Wait,
            request_digest,
        })
        .unwrap()
        .await
        .unwrap();
    let (command_sequence, command_revision) = match reply.envelope {
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            server_sequence: Some(server_sequence),
            after_revision: Some(after_revision),
            ..
        } => (server_sequence.get(), after_revision.get()),
        envelope => panic!("unexpected command reply: {envelope:?}"),
    };
    let row = sqlx::query(
        "SELECT facet_revision,last_server_sequence,checkpoint_bytes,checkpoint_sha256 \
         FROM tme.facets WHERE facet_id=$1",
    )
    .bind(world_id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.get::<i64, _>("facet_revision") >= i64::try_from(command_revision).unwrap());
    assert!(row.get::<i64, _>("last_server_sequence") >= i64::try_from(command_sequence).unwrap());
    let checkpoint: Vec<u8> = row.get("checkpoint_bytes");
    let checkpoint_sha: Vec<u8> = row.get("checkpoint_sha256");
    assert_eq!(Sha256::digest(&checkpoint).as_slice(), checkpoint_sha);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
        )
        .bind(account_id.as_uuid())
        .bind(command_id.as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );

    drop(grant);
    drop(first);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let second = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    let restarted = second.session_bootstrap(&cookie).await.unwrap();
    assert_eq!(restarted.selected_character_id, Some(character_id));
    assert_eq!(restarted.control_api_version, 3);
    drop(second);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let third = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    assert!(third.gameplay_ready());
    let restarted_session = third.session_bootstrap(&cookie).await.unwrap();
    let restarted_ticket = third
        .issue_ticket(
            &cookie,
            wire::SocketTicketRequestV1 {
                csrf_token: restarted_session.csrf_token,
            },
            "https://localhost:3000",
            "localhost:3000",
        )
        .await
        .unwrap();
    assert_eq!(restarted_ticket.protocol_major, 1);
    assert_eq!(restarted_ticket.supported_minors, vec![8]);
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (restarted_grant, restarted_welcome) = third
        .admit(
            &restarted_ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
        .unwrap();
    // D4: there is no arrival to transfer into. A restart must resume the
    // character exactly where the durable checkpoint left it.
    assert_eq!(
        restarted_welcome.frame.observation_center,
        observation_center_before_restart
    );
    drop(restarted_grant);

    http_reconnect_replay_and_logout(third, character_id).await;

    sqlx::query(
        "UPDATE tme.command_receipts SET full_expires_at=created_at+interval '1 microsecond' \
         WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .execute(&pool)
    .await
    .unwrap();
    let tombstone = PostgresStore::new(pool.clone())
        .receipt(account_id, command_id)
        .await
        .unwrap()
        .unwrap();
    assert!(tombstone.outcome.is_none());
    let tombstone_row = sqlx::query(
        "SELECT disposition,outcome_bytes,session_id,actor_id,control_epoch, \
                client_sequence,server_sequence,before_revision,after_revision \
         FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(tombstone_row.get::<String, _>("disposition"), "expired");
    for column in [
        "outcome_bytes",
        "session_id",
        "actor_id",
        "control_epoch",
        "client_sequence",
        "server_sequence",
        "before_revision",
        "after_revision",
    ] {
        assert!(tombstone_row.try_get_raw(column).unwrap().is_null());
    }

    let durable = sqlx::query(
        "SELECT checkpoint_bytes,checkpoint_sha256,content_digest \
         FROM tme.facets WHERE facet_id=$1",
    )
    .bind(world_id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    let checkpoint_bytes: Vec<u8> = durable.get("checkpoint_bytes");
    let checkpoint_sha: Vec<u8> = durable.get("checkpoint_sha256");
    let content_digest: Vec<u8> = durable.get("content_digest");

    sqlx::query("UPDATE tme.facets SET checkpoint_sha256=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind([0_u8; 32].as_slice())
        .execute(&pool)
        .await
        .unwrap();
    assert!(tme_server::operator::verify_store(&pool).await.is_err());
    sqlx::query("UPDATE tme.facets SET checkpoint_sha256=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(&checkpoint_sha)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("UPDATE tme.facets SET content_digest=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind([0_u8; 32].as_slice())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id),)
            .await
            .is_err()
    );
    sqlx::query("UPDATE tme.facets SET content_digest=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(&content_digest)
        .execute(&pool)
        .await
        .unwrap();

    let invalid_checkpoint = b"{}";
    sqlx::query("UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(invalid_checkpoint.as_slice())
        .bind(Sha256::digest(invalid_checkpoint).as_slice())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id),)
            .await
            .is_err()
    );
    sqlx::query("UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(&checkpoint_bytes)
        .bind(&checkpoint_sha)
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        sqlx::query("UPDATE tme.facets SET checkpoint_schema=1 WHERE facet_id=$1")
            .bind(world_id.as_uuid())
            .execute(&pool)
            .await
            .is_err()
    );

    let session_id: Uuid =
        sqlx::query_scalar("SELECT session_id FROM tme.sessions WHERE account_id=$1 LIMIT 1")
            .bind(account_id.as_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        sqlx::query(
            "INSERT INTO tme.socket_tickets (
                 ticket_digest,session_id,account_id,character_id,facet_id,actor_id,
                 expected_control_epoch,origin,host,selected_major,expires_at
             ) VALUES ($1,$2,$3,$4,$5,'player',0,'https://localhost:3000',
                       'localhost:3000',1,statement_timestamp()+interval '30 seconds')",
        )
        .bind([9_u8; 32].as_slice())
        .bind(session_id)
        .bind(account_id.as_uuid())
        .bind(character_id.as_uuid())
        .bind(Uuid::now_v7())
        .execute(&pool)
        .await
        .is_err()
    );

    sqlx::query("UPDATE tme.characters SET actor_id='wrong_actor' WHERE character_id=$1")
        .bind(character_id.as_uuid())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id),)
            .await
            .is_err()
    );
    sqlx::query("UPDATE tme.characters SET actor_id='player' WHERE character_id=$1")
        .bind(character_id.as_uuid())
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a fenced disposable restore database"]
async fn fenced_restore_hydrates_and_commits_fresh_authenticated_command() {
    tme_server::telemetry::init();
    let database_url = std::env::var("TME_RESTORE_DATABASE_URL")
        .expect("the restore proof must provide TME_RESTORE_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let account_uuid: Uuid =
        sqlx::query_scalar("SELECT account_id FROM tme.accounts WHERE username=$1")
            .bind(USERNAME)
            .fetch_one(&pool)
            .await
            .unwrap();
    let row = sqlx::query("SELECT character_id FROM tme.characters WHERE account_id=$1")
        .bind(account_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    let account_id = wire::AccountId::new(account_uuid).unwrap();
    let character_id = wire::CharacterId::new(row.get("character_id")).unwrap();
    let world_id = wire::FacetId::new(
        sqlx::query_scalar("SELECT facet_id FROM tme.facets WHERE facet_key='world'")
            .fetch_one(&pool)
            .await
            .unwrap(),
    )
    .unwrap();
    let state = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    let login = state
        .login(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            wire::LoginRequestV1 {
                username: wire::Username::new(USERNAME).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            },
        )
        .await
        .unwrap();
    let cookie = login.session_cookie.expose().to_string();
    state
        .select_character(
            &cookie,
            wire::CharacterSelectRequestV1 {
                csrf_token: login.bootstrap.csrf_token,
                character_id,
            },
        )
        .await
        .unwrap();
    let session = state.session_bootstrap(&cookie).await.unwrap();
    let ticket = state
        .issue_ticket(
            &cookie,
            wire::SocketTicketRequestV1 {
                csrf_token: session.csrf_token,
            },
            "https://localhost:3000",
            "localhost:3000",
        )
        .await
        .unwrap();
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (grant, welcome) = state
        .admit(
            &ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
        .unwrap();
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let actor_id = wire::ActorId::new("player").unwrap();
    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(grant.control.control_epoch),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(welcome.facet_revision),
        actor_id: actor_id.clone(),
        intent: wire::Intent::Wait,
    };
    let digest: [u8; 32] = Sha256::digest(serde_json::to_vec(&command).unwrap()).into();
    let reply = grant
        .facet
        .try_command(tme_server::facet::FacetCommand {
            connection_id: grant.control.connection_id,
            account_id,
            session_id: grant.control.session_id,
            character_id,
            command_id,
            control_epoch: grant.control.control_epoch,
            client_sequence: 1,
            observed_facet_revision: welcome.facet_revision,
            actor_id,
            intent: wire::Intent::Wait,
            request_digest: digest,
        })
        .unwrap()
        .await
        .unwrap();
    assert!(matches!(
        reply.envelope,
        wire::ServerEnvelope::CommandResult { .. }
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
        )
        .bind(account_id.as_uuid())
        .bind(command_id.as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    http_reconnect_replay_and_logout(state, character_id).await;
}

async fn two_client_social_socket_smoke(database_url: &str, pool: &sqlx::PgPool) {
    let first_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let second_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let first_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let replacement_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let second_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let shared_facet = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account_named(pool, first_account, "social_one", "Social One", 11).await;
    insert_account_named(pool, second_account, "social_two", "Social Two", 12).await;

    let backend = PostgresState::open(
        database_url,
        social_bootstrap(
            first_account,
            first_character,
            replacement_character,
            second_account,
            second_character,
            shared_facet,
        ),
    )
    .await
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let host = address.to_string();
    let origin = format!("http://{host}");
    let config = ServerConfig::new(address, host.clone(), origin.clone()).unwrap();
    let state = AppState::postgres(config, backend.clone());
    let (stop_send, stop_receive) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(tme_server::runtime::serve(listener, state, async {
        let _ = stop_receive.await;
    }));

    let (mut first, first_welcome, first_cookie, first_csrf) =
        login_select_connect_with_session(address, &host, &origin, "social_one", first_character)
            .await;
    let (mut second, second_welcome, second_cookie, second_csrf) =
        login_select_connect_with_session(address, &host, &origin, "social_two", second_character)
            .await;
    let (first_actor, first_epoch, first_revision) = welcome_parts(&first_welcome);
    let (second_actor, second_epoch, second_revision) = welcome_parts(&second_welcome);

    let preview_id = wire::PreviewId::new(Uuid::now_v7()).unwrap();
    let preview = send_socket_path_preview(
        &mut first,
        preview_id,
        first_actor.clone(),
        first_epoch,
        first_revision,
        vec![wire::Direction::North],
    )
    .await;
    assert!(matches!(
        preview,
        wire::ServerEnvelope::PathPreviewResult {
            disposition: wire::PathPreviewDisposition::Previewed,
            control_epoch,
            actor_id,
            preview: Some(_),
            ..
        } if control_epoch.get() == first_epoch && actor_id == first_actor
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
        )
        .bind(first_account.as_uuid())
        .bind(preview_id.as_uuid())
        .fetch_one(pool)
        .await
        .unwrap(),
        0,
        "path preview must not create a durable receipt"
    );

    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        1,
        first_revision,
        wire::Intent::Invite {
            target_character_id: second_character,
        },
    )
    .await;
    let invitation_id = wait_for_invitation(&mut second).await;
    send_durable_socket_command(
        &mut second,
        second_actor.clone(),
        second_epoch,
        1,
        second_revision,
        wire::Intent::AcceptInvite { invitation_id },
    )
    .await;

    let group_message_id = wire::MessageId::new(Uuid::now_v7()).unwrap();
    let group_message = wire::ClientCommandEnvelope::SocialMessage {
        message_id: group_message_id,
        control_epoch: wire::DecimalU64::new(first_epoch),
        actor_id: first_actor.clone(),
        scope: wire::SocialScope::Group,
        body: wire::SocialBody::new("meet by the fountain").unwrap(),
    };
    first
        .send(Message::Text(
            serde_json::to_string(&group_message).unwrap().into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        wait_for_message_result(&mut first, group_message_id).await,
        wire::MessageDisposition::Accepted
    );
    let delivered = wait_for_social_message(&mut second, group_message_id).await;
    assert!(matches!(delivered, wire::SocialScope::Group));

    first
        .send(Message::Text(
            serde_json::to_string(&group_message).unwrap().into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        wait_for_message_result(&mut first, group_message_id).await,
        wire::MessageDisposition::Accepted
    );
    assert_no_social_replay(&mut second, group_message_id).await;

    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        2,
        first_revision,
        wire::Intent::TransferLeadership {
            member_character_id: second_character,
        },
    )
    .await;
    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        3,
        first_revision,
        wire::Intent::BeginFollow {
            target_character_id: second_character,
        },
    )
    .await;
    send_durable_socket_command(
        &mut second,
        second_actor.clone(),
        second_epoch,
        2,
        second_revision,
        wire::Intent::DisbandGroup,
    )
    .await;
    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        4,
        first_revision,
        wire::Intent::Block {
            target_character_id: second_character,
        },
    )
    .await;

    let page_id = wire::MessageId::new(Uuid::now_v7()).unwrap();
    second
        .send(Message::Text(
            serde_json::to_string(&wire::ClientCommandEnvelope::SocialMessage {
                message_id: page_id,
                control_epoch: wire::DecimalU64::new(second_epoch),
                actor_id: second_actor,
                scope: wire::SocialScope::Page {
                    target_character_id: first_character,
                },
                body: wire::SocialBody::new("can you hear me?").unwrap(),
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        wait_for_message_result(&mut second, page_id).await,
        wire::MessageDisposition::Unavailable
    );

    let safe = send_socket_command_result(
        &mut first,
        first_actor.clone(),
        first_epoch,
        5,
        first_revision,
        wire::Intent::PhysicalAttack {
            mode: wire::PhysicalAttackMode::Fight,
            target_actor_id: wire::ActorId::new("player2").unwrap(),
            authorization: wire::HostilityAuthorization::Safe,
        },
    )
    .await;
    assert!(matches!(
        safe,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Rejected {
                code: wire::RejectionCode::RulesRejected
            },
            ..
        }
    ));
    let unsafe_result = send_socket_command_result(
        &mut first,
        first_actor.clone(),
        first_epoch,
        6,
        first_revision,
        wire::Intent::PhysicalAttack {
            mode: wire::PhysicalAttackMode::Fight,
            target_actor_id: wire::ActorId::new("player2").unwrap(),
            authorization: wire::HostilityAuthorization::ConfirmedUnsafe,
        },
    )
    .await;
    assert!(
        matches!(
            &unsafe_result,
            wire::ServerEnvelope::CommandResult {
                disposition: wire::CommandDisposition::Accepted,
                ..
            }
        ),
        "unexpected unsafe PvP result: {unsafe_result:?}"
    );
    let mark_uuid: Uuid = sqlx::query_scalar(
        "SELECT mark_id FROM tme.player_kill_marks WHERE killer_account_id=$1 \
         AND victim_account_id=$2 AND forgiven_at IS NULL AND expired_at IS NULL",
    )
    .bind(first_account.as_uuid())
    .bind(second_account.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let linked: bool = sqlx::query_scalar(
        "SELECT linked_karma_added AND karma_forgiveness_eligible \
         FROM tme.player_kill_marks WHERE mark_id=$1",
    )
    .bind(mark_uuid)
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(linked);
    let request = wire::ForgivePlayerKillMarkRequestV1 {
        request_id: wire::CommandId::new(Uuid::now_v7()).unwrap(),
    };
    let forgiveness = post_json_with_csrf(
        address,
        &host,
        &origin,
        &format!("/v3/player-kill-marks/{mark_uuid}/forgive"),
        &second_cookie,
        &second_csrf,
        &request,
    )
    .await;
    assert_eq!(
        forgiveness.status,
        200,
        "{}",
        String::from_utf8_lossy(&forgiveness.body)
    );
    let forgiven: wire::ForgivePlayerKillMarkResultV1 =
        serde_json::from_slice(&forgiveness.body).unwrap();
    assert_eq!(forgiven.control_api_version, 3);
    assert_eq!(forgiven.replay_status, wire::ReplayStatus::New);
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT forgiven_at IS NOT NULL AND expired_at IS NULL \
             FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(mark_uuid)
        .fetch_one(pool)
        .await
        .unwrap()
    );

    let exit_mark = exercise_player_kill_mark_schedule(
        pool,
        address,
        &host,
        &origin,
        first_account,
        first_character,
        &first_cookie,
        &first_csrf,
        second_account,
        second_character,
        &second_cookie,
        &second_csrf,
    )
    .await;

    // D4: the durable store holds exactly one world, so social scope can never
    // be a question of which copy a player happens to be looking at.
    let worlds: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.facets")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(worlds, 1);

    let replacement = post_json(
        address,
        &host,
        &origin,
        "/v3/characters/select",
        &first_cookie,
        &wire::CharacterSelectRequestV1 {
            csrf_token: first_csrf.clone(),
            character_id: replacement_character,
        },
    )
    .await;
    assert_eq!(
        replacement.status,
        200,
        "{}",
        String::from_utf8_lossy(&replacement.body)
    );
    let replacement: wire::CharacterSelectionV1 =
        serde_json::from_slice(&replacement.body).unwrap();
    assert_eq!(replacement.control_api_version, 3);
    assert_eq!(replacement.character.character_id, replacement_character);
    assert_eq!(
        wait_for_draining(&mut first).await,
        (wire::DrainingReason::SessionEnded, false)
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT karma_forgiveness_eligible FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(exit_mark)
        .fetch_one(pool)
        .await
        .unwrap(),
        "character replacement closes the linked-karma forgiveness window"
    );
    tme_server::operator::verify_store(pool).await.unwrap();
    let corrupt_mark_id = Uuid::now_v7();
    sqlx::query("UPDATE tme.player_kill_marks SET mark_id=$2 WHERE mark_id=$1")
        .bind(exit_mark)
        .bind(corrupt_mark_id)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        tme_server::operator::verify_store(pool).await.is_err(),
        "store verification accepted a mark whose ID contradicted its durable identity"
    );
    sqlx::query("UPDATE tme.player_kill_marks SET mark_id=$2 WHERE mark_id=$1")
        .bind(corrupt_mark_id)
        .bind(exit_mark)
        .execute(pool)
        .await
        .unwrap();
    tme_server::operator::verify_store(pool).await.unwrap();
    second.close(None).await.unwrap();
    let _ = stop_send.send(());
    server.await.unwrap().unwrap();
    drop(backend);
    tokio::time::sleep(Duration::from_millis(250)).await;
}

#[allow(clippy::too_many_arguments)]
async fn exercise_player_kill_mark_schedule(
    pool: &sqlx::PgPool,
    address: SocketAddr,
    host: &str,
    origin: &str,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    killer_cookie: &str,
    killer_csrf: &wire::CsrfToken,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
    victim_cookie: &str,
    victim_csrf: &wire::CsrfToken,
) -> Uuid {
    let killer_session: Uuid = sqlx::query_scalar(
        "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
         AND selected_character_id=$2 AND revoked_at IS NULL",
    )
    .bind(killer_account.as_uuid())
    .bind(killer_character.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let victim_session: Uuid = sqlx::query_scalar(
        "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
         AND selected_character_id=$2 AND revoked_at IS NULL",
    )
    .bind(victim_account.as_uuid())
    .bind(victim_character.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let store = PostgresStore::new(pool.clone());

    let oldest = insert_synthetic_player_kill_mark(
        pool,
        100,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        40,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        200,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(pool, killer_account, &[(oldest, 2)]).await;

    let middle = insert_synthetic_player_kill_mark(
        pool,
        101,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        30,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        201,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(pool, killer_account, &[(oldest, 4), (middle, 2)]).await;

    let newest = insert_synthetic_player_kill_mark(
        pool,
        102,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        20,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        202,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(
        pool,
        killer_account,
        &[(oldest, 6), (middle, 4), (newest, 2)],
    )
    .await;

    let lockout = insert_synthetic_player_kill_mark(
        pool,
        103,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        10,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        203,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule_paused(pool, killer_account, &[oldest, middle, newest, lockout]).await;
    let locked = post_json(
        address,
        host,
        origin,
        "/v3/socket-tickets",
        killer_cookie,
        &wire::SocketTicketRequestV1 {
            csrf_token: killer_csrf.clone(),
        },
    )
    .await;
    assert_eq!(locked.status, 423);
    assert_eq!(
        serde_json::from_slice::<wire::ControlErrorV1>(&locked.body).unwrap(),
        wire::ControlErrorV1 {
            code: wire::ControlErrorCode::GameplayMarkLocked,
        }
    );

    let forgiveness = post_json_with_csrf(
        address,
        host,
        origin,
        &format!("/v3/player-kill-marks/{lockout}/forgive"),
        victim_cookie,
        victim_csrf,
        &wire::ForgivePlayerKillMarkRequestV1 {
            request_id: wire::CommandId::new(Uuid::now_v7()).unwrap(),
        },
    )
    .await;
    assert_eq!(
        forgiveness.status,
        200,
        "{}",
        String::from_utf8_lossy(&forgiveness.body)
    );
    let forgiveness: wire::ForgivePlayerKillMarkResultV1 =
        serde_json::from_slice(&forgiveness.body).unwrap();
    assert_eq!(forgiveness.control_api_version, 3);
    assert_mark_schedule(
        pool,
        killer_account,
        &[(oldest, 6), (middle, 4), (newest, 2)],
    )
    .await;

    sqlx::query(
        "UPDATE tme.player_kill_marks \
         SET expires_at=statement_timestamp()-interval '1 second' WHERE mark_id=$1",
    )
    .bind(newest)
    .execute(pool)
    .await
    .unwrap();
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(pool, killer_account, &[(oldest, 4), (middle, 2)]).await;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT expired_at IS NOT NULL AND expires_at IS NULL \
             FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(newest)
        .fetch_one(pool)
        .await
        .unwrap()
    );

    let exit_mark = insert_synthetic_player_kill_mark(
        pool,
        104,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        5,
        false,
    )
    .await;
    sqlx::query(
        "UPDATE tme.player_kill_marks SET linked_karma_added=true, \
         karma_forgiveness_eligible=true WHERE mark_id=$1",
    )
    .bind(exit_mark)
    .execute(pool)
    .await
    .unwrap();
    insert_expired_schedule_sentinel(
        pool,
        204,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(
        pool,
        killer_account,
        &[(oldest, 6), (middle, 4), (exit_mark, 2)],
    )
    .await;
    prove_serializable_mark_reconciliation_retry(pool, killer_account).await;
    exit_mark
}

async fn prove_serializable_mark_reconciliation_retry(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
) {
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("UPDATE tme.accounts SET display_name=display_name WHERE account_id=$1")
        .bind(killer_account.as_uuid())
        .execute(&mut *blocker)
        .await
        .unwrap();

    let first_store = PostgresStore::new(pool.clone());
    let second_store = first_store.clone();
    let account_id = killer_account.as_uuid();
    let first =
        tokio::spawn(async move { first_store.reconcile_player_kill_marks(account_id).await });
    let second =
        tokio::spawn(async move { second_store.reconcile_player_kill_marks(account_id).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    blocker.commit().await.unwrap();
    let outcomes = [first.await.unwrap(), second.await.unwrap()];
    assert!(
        outcomes.iter().any(Result::is_err),
        "concurrent serializable reconciliation did not exercise a retryable conflict"
    );
    assert!(
        outcomes
            .iter()
            .filter_map(|result| result.as_ref().err())
            .all(|error| error.contains("could not serialize access due to concurrent update"))
    );

    let retry = PostgresStore::new(pool.clone());
    retry
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    retry.verify_player_kill_marks().await.unwrap();
}

#[allow(clippy::too_many_arguments)]
async fn insert_synthetic_player_kill_mark(
    pool: &sqlx::PgPool,
    sequence: i64,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    killer_session: Uuid,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
    victim_session: Uuid,
    assessed_seconds_ago: i64,
    expires_in_past: bool,
) -> Uuid {
    let name = format!("https://tme.invalid/ids/player-kill-mark/v1/{sequence}");
    let mark_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes());
    sqlx::query(
        "INSERT INTO tme.player_kill_marks \
         (mark_id,facet_kill_sequence,killer_account_id,killer_character_id, \
          victim_account_id,victim_character_id,killer_session_id,victim_session_id, \
          assessed_at,assessed_logical_time,linked_karma_added, \
          karma_forgiveness_eligible,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8, \
                 statement_timestamp()-make_interval(secs=>$9),$2,false,false, \
                 CASE WHEN $10 THEN statement_timestamp()-interval '1 second' ELSE NULL END)",
    )
    .bind(mark_id)
    .bind(sequence)
    .bind(killer_account.as_uuid())
    .bind(killer_character.as_uuid())
    .bind(victim_account.as_uuid())
    .bind(victim_character.as_uuid())
    .bind(killer_session)
    .bind(victim_session)
    .bind(assessed_seconds_ago)
    .bind(expires_in_past)
    .execute(pool)
    .await
    .unwrap();
    mark_id
}

#[allow(clippy::too_many_arguments)]
async fn insert_expired_schedule_sentinel(
    pool: &sqlx::PgPool,
    sequence: i64,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    killer_session: Uuid,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
    victim_session: Uuid,
) {
    insert_synthetic_player_kill_mark(
        pool,
        sequence,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        60,
        true,
    )
    .await;
}

async fn assert_mark_schedule(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
    expected: &[(Uuid, i64)],
) {
    let rows = sqlx::query(
        "SELECT mark_id,EXTRACT(EPOCH FROM (expires_at-statement_timestamp()))::double precision AS seconds \
         FROM tme.player_kill_marks WHERE killer_account_id=$1 \
         AND forgiven_at IS NULL AND expired_at IS NULL ORDER BY assessed_at,mark_id",
    )
    .bind(killer_account.as_uuid())
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), expected.len());
    for (row, (expected_id, weeks)) in rows.iter().zip(expected) {
        assert_eq!(row.get::<Uuid, _>("mark_id"), *expected_id);
        let actual: f64 = row.get("seconds");
        let wanted = (*weeks as f64) * 7.0 * 24.0 * 60.0 * 60.0;
        assert!(
            (actual - wanted).abs() < 10.0,
            "mark {expected_id} has {actual} seconds, expected {wanted}"
        );
    }
}

async fn assert_mark_schedule_paused(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
    expected_ids: &[Uuid],
) {
    let rows = sqlx::query(
        "SELECT mark_id,expires_at IS NULL AS paused FROM tme.player_kill_marks \
         WHERE killer_account_id=$1 AND forgiven_at IS NULL AND expired_at IS NULL \
         ORDER BY assessed_at,mark_id",
    )
    .bind(killer_account.as_uuid())
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), expected_ids.len());
    for (row, expected_id) in rows.iter().zip(expected_ids) {
        assert_eq!(row.get::<Uuid, _>("mark_id"), *expected_id);
        assert!(row.get::<bool, _>("paused"));
    }
}

async fn login_select_connect_with_session(
    address: SocketAddr,
    host: &str,
    origin: &str,
    username: &str,
    character_id: wire::CharacterId,
) -> (Socket, wire::ServerEnvelope, String, wire::CsrfToken) {
    let login = http_request(
        address,
        host,
        origin,
        "POST",
        "/v3/login",
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
    assert_eq!(login.status, 200);
    let cookie = login.headers["set-cookie"]
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let bootstrap: wire::SessionBootstrapV1 = serde_json::from_slice(&login.body).unwrap();
    assert_eq!(bootstrap.control_api_version, 3);
    let selection = post_json(
        address,
        host,
        origin,
        "/v3/characters/select",
        &cookie,
        &wire::CharacterSelectRequestV1 {
            csrf_token: bootstrap.csrf_token.clone(),
            character_id,
        },
    )
    .await;
    assert_eq!(selection.status, 200);
    let selection: wire::CharacterSelectionV1 = serde_json::from_slice(&selection.body).unwrap();
    assert_eq!(selection.control_api_version, 3);
    let ticket = issue_ticket(address, host, origin, &cookie, &bootstrap.csrf_token).await;
    let (socket, welcome) = connect_character(address, host, origin, &ticket.ticket).await;
    (socket, welcome, cookie, bootstrap.csrf_token)
}

#[allow(clippy::too_many_arguments)]
async fn send_durable_socket_command(
    socket: &mut Socket,
    actor_id: wire::ActorId,
    control_epoch: u64,
    client_sequence: u64,
    observed_facet_revision: u64,
    intent: wire::Intent,
) {
    let result = send_socket_command_result(
        socket,
        actor_id,
        control_epoch,
        client_sequence,
        observed_facet_revision,
        intent,
    )
    .await;
    assert!(matches!(
        result,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
}

#[allow(clippy::too_many_arguments)]
async fn send_socket_command_result(
    socket: &mut Socket,
    actor_id: wire::ActorId,
    control_epoch: u64,
    client_sequence: u64,
    observed_facet_revision: u64,
    intent: wire::Intent,
) -> wire::ServerEnvelope {
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    socket
        .send(Message::Text(
            serde_json::to_string(&wire::ClientCommandEnvelope::Command {
                command_id,
                control_epoch: wire::DecimalU64::new(control_epoch),
                client_sequence: wire::DecimalU64::new(client_sequence),
                observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
                actor_id,
                intent,
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    loop {
        let envelope = receive_envelope(socket).await;
        if matches!(
            &envelope,
            wire::ServerEnvelope::CommandResult {
                command_id: received,
                ..
            } if *received == command_id
        ) {
            return envelope;
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_socket_path_preview(
    socket: &mut Socket,
    preview_id: wire::PreviewId,
    actor_id: wire::ActorId,
    control_epoch: u64,
    observed_facet_revision: u64,
    path: Vec<wire::Direction>,
) -> wire::ServerEnvelope {
    socket
        .send(Message::Text(
            serde_json::to_string(&wire::ClientCommandEnvelope::PathPreview {
                preview_id,
                control_epoch: wire::DecimalU64::new(control_epoch),
                observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
                actor_id,
                path,
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    loop {
        let envelope = receive_envelope(socket).await;
        if matches!(
            &envelope,
            wire::ServerEnvelope::PathPreviewResult {
                preview_id: received,
                ..
            } if *received == preview_id
        ) {
            return envelope;
        }
    }
}

async fn wait_for_invitation(socket: &mut Socket) -> wire::DecimalU64 {
    loop {
        if let wire::ServerEnvelope::StateUpdate { frame, .. } = receive_envelope(socket).await
            && let Some(invitation) = frame.social.incoming_invitations.first()
        {
            return invitation.invitation_id;
        }
    }
}

async fn wait_for_message_result(
    socket: &mut Socket,
    expected: wire::MessageId,
) -> wire::MessageDisposition {
    loop {
        if let wire::ServerEnvelope::MessageResult {
            message_id,
            disposition,
        } = receive_envelope(socket).await
            && message_id == expected
        {
            return disposition;
        }
    }
}

async fn wait_for_social_message(
    socket: &mut Socket,
    expected: wire::MessageId,
) -> wire::SocialScope {
    loop {
        if let wire::ServerEnvelope::SocialMessage {
            message_id, scope, ..
        } = receive_envelope(socket).await
            && message_id == expected
        {
            return scope;
        }
    }
}

async fn assert_no_social_replay(socket: &mut Socket, message_id: wire::MessageId) {
    let result = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            if let wire::ServerEnvelope::SocialMessage {
                message_id: received,
                ..
            } = receive_envelope(socket).await
                && received == message_id
            {
                return;
            }
        }
    })
    .await;
    assert!(result.is_err(), "exact message retry was redelivered");
}

async fn insert_account(pool: &sqlx::PgPool, account_id: wire::AccountId) {
    insert_account_named(pool, account_id, USERNAME, "Durable Tester", 7).await;
}

async fn insert_account_named(
    pool: &sqlx::PgPool,
    account_id: wire::AccountId,
    username: &str,
    display_name: &str,
    salt_byte: u8,
) {
    let params = Params::new(65_536, 3, 4, Some(32)).unwrap();
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(&[salt_byte; 16]).unwrap();
    let phc = argon
        .hash_password(PASSWORD.as_bytes(), &salt)
        .unwrap()
        .to_string();
    sqlx::query("INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,$3)")
        .bind(account_id.as_uuid())
        .bind(username)
        .bind(display_name)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tme.account_credentials(account_id,password_phc) VALUES($1,$2)")
        .bind(account_id.as_uuid())
        .bind(phc)
        .execute(pool)
        .await
        .unwrap();
}

fn bootstrap(
    account_id: wire::AccountId,
    character_id: wire::CharacterId,
    world_id: wire::FacetId,
) -> PostgresBootstrap {
    let engine = scenario_engine();
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .realm,
        "realm_0"
    );
    PostgresBootstrap {
        world: PostgresWorldBootstrap {
            facet_id: world_id,
            key: "world".to_string(),
            engine,
        },
        characters: vec![PostgresCharacterBootstrap {
            account_id,
            character_id,
            slot: 1,
            display_name: wire::DisplayName::new("Wayfarer").unwrap(),
            actor_id: tme_rules::ActorId::from("player"),
        }],
    }
}

/// Owner ruling 2026-08-20 (successor issue #3): logging off is not a karma
/// escape. Covers the four properties the ruling needs — the debt survives a
/// process restart, it applies exactly once, a rolled-back admission leaves it
/// owed rather than silently paid, and a present killer is untouched.
#[tokio::test]
#[ignore = "requires an EV runner-owned PostgreSQL database"]
async fn absent_killer_karma_is_deferred_and_applied_exactly_once() {
    let database_url = std::env::var("TME_TEST_DATABASE_URL")
        .expect("the exact PostgreSQL runner must provide TME_TEST_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    truncate_all(&pool).await;

    let killer_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let killer_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let replacement_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let victim_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let victim_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let world_id = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account_named(
        &pool,
        killer_account,
        "pending_killer",
        "Pending Killer",
        11,
    )
    .await;
    insert_account_named(
        &pool,
        victim_account,
        "pending_victim",
        "Pending Victim",
        12,
    )
    .await;

    let build = || {
        social_bootstrap(
            killer_account,
            killer_character,
            replacement_character,
            victim_account,
            victim_character,
            world_id,
        )
    };

    // A kill lands while the killer is away. The mark and the deferred
    // consequence are written together, exactly as the runtime writes them.
    let first = PostgresState::open(&database_url, build()).await.unwrap();
    let (killer_session, victim_session) = open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    let karma_before = admit_and_read_karma(&first, &pool, killer_account, killer_character).await;
    drop(first);

    insert_synthetic_player_kill_mark(
        &pool,
        7,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        10,
        false,
    )
    .await;
    insert_pending_consequence(&pool, 7, killer_account, killer_character, victim_character).await;
    schedule_marks(&pool).await;
    assert_eq!(1, pending_count(&pool, killer_account).await);
    assert!(
        !forgivable(&pool, 7).await,
        "a consequence that has not landed yet is not forgivable"
    );

    // PROPERTY 1 — the debt survives the process. This is a brand new
    // PostgresState over the same database, which is what a restart is.
    let second = PostgresState::open(&database_url, build()).await.unwrap();
    let karma_after = admit_and_read_karma(&second, &pool, killer_account, killer_character).await;
    assert_eq!(
        0,
        pending_count(&pool, killer_account).await,
        "an applied consequence must be cleared"
    );
    // One kill, one karma point — asserted concretely so this cannot pass on
    // some unrelated drift in the sheet.
    assert_eq!(
        karma_before + 1,
        karma_after,
        "the deferred consequence never reached the killer's sheet"
    );
    let linked: bool = sqlx::query_scalar(
        "SELECT linked_karma_added FROM tme.player_kill_marks WHERE facet_kill_sequence=7",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        linked,
        "the mark still claims no karma was linked after the consequence landed"
    );
    // Owner ruling 2026-08-20: forgiveness follows the karma. Once it lands the
    // victim can forgive, exactly as if the killer had never left.
    assert!(
        forgivable(&pool, 7).await,
        "karma landed but the victim still cannot forgive it"
    );

    // PROPERTY 2 — exactly once. Re-admitting must not charge them twice.
    let karma_replayed =
        admit_and_read_karma(&second, &pool, killer_account, killer_character).await;
    assert_eq!(
        karma_after, karma_replayed,
        "a second admission applied the same consequence again"
    );
    // Admission on its own never moves karma, which is what makes the
    // assertion above evidence of cause rather than coincidence.
    assert_eq!(karma_before + 1, karma_replayed);
    assert_eq!(0, pending_count(&pool, killer_account).await);
    drop(second);

    // PROPERTY 3 — a rolled-back admission leaves the debt owed. Four active
    // marks lock the account, which aborts the admission transaction AFTER the
    // clear would have run. If the clear were in its own transaction, the
    // consequence would vanish unpaid.
    // A second, distinct kill — the rules refuse to apply one kill's
    // consequence twice, which is a property of the engine, not of this table.
    insert_synthetic_player_kill_mark(
        &pool,
        8,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        9,
        false,
    )
    .await;
    insert_pending_consequence(&pool, 8, killer_account, killer_character, victim_character).await;
    for sequence in [11_i64, 12, 13, 14] {
        insert_synthetic_player_kill_mark(
            &pool,
            sequence,
            killer_account,
            killer_character,
            killer_session,
            victim_account,
            victim_character,
            victim_session,
            5,
            false,
        )
        .await;
    }
    schedule_marks(&pool).await;
    let third = PostgresState::open(&database_url, build()).await.unwrap();
    let checkpoint_before = world_checkpoint_sha(&pool).await;
    assert!(
        admit_character(&third, &pool, killer_account, killer_character)
            .await
            .is_err(),
        "a gameplay-mark-locked account must not be admitted"
    );
    assert_eq!(
        1,
        pending_count(&pool, killer_account).await,
        "a rolled-back admission silently discharged the debt"
    );
    assert_eq!(
        checkpoint_before,
        world_checkpoint_sha(&pool).await,
        "a rolled-back admission still moved the durable world"
    );
    drop(third);

    // PROPERTY 5 — a deferred consequence survives disaster recovery. A
    // restored database is a different database as far as the store is
    // concerned, so the restore fence must clear before anything runs, and the
    // debt must still be owed on the far side of it. The row is the very one
    // PROPERTY 3 preserved through its rolled-back admission; the marks that
    // locked the account come out so admission can proceed this time.
    for sequence in [11_i64, 12, 13, 14] {
        sqlx::query("DELETE FROM tme.player_kill_marks WHERE facet_kill_sequence=$1")
            .bind(sequence)
            .execute(&pool)
            .await
            .unwrap();
    }
    schedule_marks(&pool).await;
    sqlx::query("UPDATE tme.store_state SET database_oid='424242' WHERE singleton")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, build()).await.is_err(),
        "a restored database must refuse to open before it is fenced"
    );
    assert_eq!(
        1,
        pending_count(&pool, killer_account).await,
        "the refused open discarded the debt"
    );
    let epoch_before: i64 =
        sqlx::query_scalar("SELECT restore_fence_epoch FROM tme.store_state WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    unsafe { std::env::set_var("DATABASE_URL", &database_url) };
    tme_server::operator::run(&[
        "store".to_string(),
        "restore-fence".to_string(),
        "--confirm-restored-database".to_string(),
    ])
    .await
    .expect("the operator fences the restored database");
    let epoch_after: i64 =
        sqlx::query_scalar("SELECT restore_fence_epoch FROM tme.store_state WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(epoch_before + 1, epoch_after, "the fence did not advance");
    // The fence invalidates every session it inherited, so the killer comes
    // back through a fresh one — which is exactly the path the debt must
    // survive.
    open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    let fifth = PostgresState::open(&database_url, build()).await.unwrap();
    let karma_restored =
        admit_and_read_karma(&fifth, &pool, killer_account, killer_character).await;
    assert_eq!(
        karma_replayed + 1,
        karma_restored,
        "the debt did not survive the restore"
    );
    assert_eq!(
        0,
        pending_count(&pool, killer_account).await,
        "the restored consequence was applied but not cleared"
    );
    drop(fifth);

    // PROPERTY 4 — the present-killer path writes no pending row at all.
    truncate_all(&pool).await;
    insert_account_named(
        &pool,
        killer_account,
        "pending_killer",
        "Pending Killer",
        11,
    )
    .await;
    insert_account_named(
        &pool,
        victim_account,
        "pending_victim",
        "Pending Victim",
        12,
    )
    .await;
    let fourth = PostgresState::open(&database_url, build()).await.unwrap();
    open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    let _ = admit_and_read_karma(&fourth, &pool, killer_account, killer_character).await;
    let total: i64 =
        sqlx::query_scalar("SELECT count(*) FROM tme.pending_player_kill_consequences")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        0, total,
        "an ordinary admission invented a deferred consequence"
    );
}

/// Synthetic marks are inserted with raw SQL, so they miss the anchoring the
/// real durable-effects path performs on insert. This reproduces the
/// scheduler's contract: active marks carry an expiry until an account holds
/// four of them, at which point the account is locked and they carry none.
/// The duplicate-sequence branch of the durable player-kill write was dark to
/// every test, which is how a stale `facet_id` predicate survived the D4 schema
/// cut inside it. A replay must agree with what is already stored; a replay
/// carrying different facts must be refused.
#[tokio::test]
#[ignore = "requires an EV runner-owned PostgreSQL database"]
async fn replayed_player_kill_assessment_agrees_and_a_contradicting_one_is_refused() {
    let database_url = std::env::var("TME_TEST_DATABASE_URL")
        .expect("the exact PostgreSQL runner must provide TME_TEST_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    truncate_all(&pool).await;

    let killer_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let killer_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let replacement_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let victim_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let victim_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let world_id = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account_named(&pool, killer_account, "replay_killer", "Replay Killer", 21).await;
    insert_account_named(&pool, victim_account, "replay_victim", "Replay Victim", 22).await;
    let state = PostgresState::open(
        &database_url,
        social_bootstrap(
            killer_account,
            killer_character,
            replacement_character,
            victim_account,
            victim_character,
            world_id,
        ),
    )
    .await
    .unwrap();
    open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    drop(state);

    let store = PostgresStore::new(pool.clone());
    let assessment = |logical_time: u64| tme_rules::PlayerKillAssessmentV1 {
        facet_kill_sequence: 42,
        killer_character_id: tme_rules::CharacterId::new(killer_character.to_string()),
        victim_character_id: tme_rules::CharacterId::new(victim_character.to_string()),
        exempt_self_defense: false,
        consequence: tme_rules::PlayerKillConsequenceV1::AppliedHere {
            linked_karma_added: true,
        },
        logical_time: tme_rules::LogicalTime::new(logical_time),
    };

    commit_kill(&store, &pool, world_id, &assessment(9))
        .await
        .expect("the first durable write of a kill succeeds");
    let marks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.player_kill_marks WHERE facet_kill_sequence=42",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(1, marks);
    // A present killer holds a live session when their kill is assessed, so the
    // victim can forgive immediately. This slice must not have changed that.
    assert!(
        forgivable(&pool, 42).await,
        "a present killer's mark stopped being forgivable"
    );

    // The replay. Before the fix this raised a SQL error on a column the D4 cut
    // had dropped, instead of comparing the stored facts.
    commit_kill(&store, &pool, world_id, &assessment(9))
        .await
        .expect("replaying an identical kill agrees with what is stored");
    let marks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.player_kill_marks WHERE facet_kill_sequence=42",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(1, marks, "a replay must not create a second mark");

    // Same sequence, different assessed time: that is not a replay, it is a
    // contradiction, and it must be refused rather than silently ignored.
    let error = commit_kill(&store, &pool, world_id, &assessment(10))
        .await
        .expect_err("a contradicting replay must be refused");
    assert!(
        error.contains("conflicts with different durable facts"),
        "unexpected refusal: {error}"
    );
}

async fn commit_kill(
    store: &PostgresStore,
    pool: &sqlx::PgPool,
    world_id: wire::FacetId,
    assessment: &tme_rules::PlayerKillAssessmentV1,
) -> Result<(), String> {
    let row =
        sqlx::query("SELECT facet_revision,last_server_sequence,checkpoint_bytes FROM tme.facets")
            .fetch_one(pool)
            .await
            .unwrap();
    let revision: i64 = row.get("facet_revision");
    let sequence: i64 = row.get("last_server_sequence");
    let bytes: Vec<u8> = row.get("checkpoint_bytes");
    let checkpoint = tme_rules::FacetCheckpointV4::from_bytes(bytes).unwrap();
    store
        .commit_system(tme_server::store::SystemCommit {
            facet_id: world_id,
            expected_server_sequence: u64::try_from(sequence).unwrap(),
            expected_revision: u64::try_from(revision).unwrap(),
            next_server_sequence: u64::try_from(sequence).unwrap() + 1,
            next_revision: u64::try_from(revision).unwrap() + 1,
            checkpoint: &checkpoint,
            action: "facet_tick",
            durable_effects: std::slice::from_ref(
                &tme_rules::DurableGameplayEffectV1::PlayerKillAssessed(assessment.clone()),
            ),
        })
        .await
}

async fn schedule_marks(pool: &sqlx::PgPool) {
    sqlx::raw_sql(
        "WITH ordered AS ( \
             SELECT mark_id, \
                    row_number() OVER (PARTITION BY killer_account_id \
                                       ORDER BY assessed_at, mark_id) AS position, \
                    count(*) OVER (PARTITION BY killer_account_id) AS total \
             FROM tme.player_kill_marks \
             WHERE forgiven_at IS NULL AND expired_at IS NULL) \
         UPDATE tme.player_kill_marks m \
         SET expires_at = CASE WHEN o.total >= 4 THEN NULL ELSE \
             statement_timestamp() + \
             ((((o.total - o.position) + 1) * 2) * interval '1 week') END \
         FROM ordered o WHERE m.mark_id = o.mark_id",
    )
    .execute(pool)
    .await
    .unwrap();
    PostgresStore::new(pool.clone())
        .verify_player_kill_marks()
        .await
        .expect("synthetic marks form a valid schedule");
}

async fn forgivable(pool: &sqlx::PgPool, sequence: i64) -> bool {
    sqlx::query_scalar(
        "SELECT linked_karma_added AND karma_forgiveness_eligible \
         FROM tme.player_kill_marks WHERE facet_kill_sequence=$1",
    )
    .bind(sequence)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn truncate_all(pool: &sqlx::PgPool) {
    sqlx::raw_sql(
        "TRUNCATE tme.audit_events,tme.command_receipts,tme.socket_tickets,\
         tme.sessions,tme.characters,tme.facets,tme.account_credentials,\
         tme.accounts RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn open_sessions(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
) -> (Uuid, Uuid) {
    let mut sessions = Vec::new();
    for (account, character) in [
        (killer_account, killer_character),
        (victim_account, victim_character),
    ] {
        let session_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tme.sessions \
             (session_id,account_id,token_digest,csrf_digest,selected_character_id, \
              idle_expires_at,absolute_expires_at) \
             VALUES ($1,$2,$3,$4,$5,statement_timestamp()+interval '1 day', \
                     statement_timestamp()+interval '2 days')",
        )
        .bind(session_id)
        .bind(account.as_uuid())
        .bind(Sha256::digest(session_id.as_bytes()).to_vec())
        .bind(Sha256::digest(account.as_uuid().as_bytes()).to_vec())
        .bind(character.as_uuid())
        .execute(pool)
        .await
        .unwrap();
        sessions.push(session_id);
    }
    (sessions[0], sessions[1])
}

async fn insert_pending_consequence(
    pool: &sqlx::PgPool,
    sequence: i64,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    victim_character: wire::CharacterId,
) {
    sqlx::query(
        "INSERT INTO tme.pending_player_kill_consequences \
         (facet_kill_sequence,killer_account_id,killer_character_id,victim_character_id, \
          victim_alignment,victim_nature,assessed_logical_time) \
         VALUES ($1,$2,$3,$4,'lawful','human',1)",
    )
    .bind(sequence)
    .bind(killer_account.as_uuid())
    .bind(killer_character.as_uuid())
    .bind(victim_character.as_uuid())
    .execute(pool)
    .await
    .unwrap();
}

async fn pending_count(pool: &sqlx::PgPool, killer_account: wire::AccountId) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM tme.pending_player_kill_consequences WHERE killer_account_id=$1",
    )
    .bind(killer_account.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn world_checkpoint_sha(pool: &sqlx::PgPool) -> Vec<u8> {
    sqlx::query_scalar("SELECT checkpoint_sha256 FROM tme.facets")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn admit_character(
    state: &PostgresState,
    pool: &sqlx::PgPool,
    account: wire::AccountId,
    character: wire::CharacterId,
) -> Result<wire::ObserverFrame, String> {
    let session_id: Uuid = sqlx::query_scalar(
        "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
         AND selected_character_id=$2 AND revoked_at IS NULL ORDER BY session_id LIMIT 1",
    )
    .bind(account.as_uuid())
    .bind(character.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    // One ticket at a time: the digest is the primary key, and each admission
    // in this test issues a fresh one.
    sqlx::query("DELETE FROM tme.socket_tickets")
        .execute(pool)
        .await
        .unwrap();
    let ticket = wire::AdmissionTicket::new("A".repeat(43)).unwrap();
    let epoch: i64 =
        sqlx::query_scalar("SELECT control_epoch FROM tme.characters WHERE character_id=$1")
            .bind(character.as_uuid())
            .fetch_one(pool)
            .await
            .unwrap();
    let actor_id: String =
        sqlx::query_scalar("SELECT actor_id FROM tme.characters WHERE character_id=$1")
            .bind(character.as_uuid())
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query(
        "INSERT INTO tme.socket_tickets \
         (ticket_digest,session_id,account_id,character_id,actor_id, \
          expected_control_epoch,origin,host,selected_major,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,'https://localhost:3000','localhost:3000',1, \
                 statement_timestamp()+interval '1 hour')",
    )
    .bind(Sha256::digest(ticket.expose_for_admission().as_bytes()).to_vec())
    .bind(session_id)
    .bind(account.as_uuid())
    .bind(character.as_uuid())
    .bind(&actor_id)
    .bind(epoch)
    .execute(pool)
    .await
    .unwrap();
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    match state
        .admit(
            &ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
    {
        Ok((_grant, welcome)) => Ok(welcome.frame),
        Err(error) => Err(format!("{error:?}")),
    }
}

async fn admit_and_read_karma(
    state: &PostgresState,
    pool: &sqlx::PgPool,
    account: wire::AccountId,
    character: wire::CharacterId,
) -> i64 {
    let frame = admit_character(state, pool, account, character)
        .await
        .unwrap_or_else(|error| panic!("admission failed: {error}"));
    i64::from(frame.character.karma_points)
}

fn social_bootstrap(
    first_account: wire::AccountId,
    first_character: wire::CharacterId,
    replacement_character: wire::CharacterId,
    second_account: wire::AccountId,
    second_character: wire::CharacterId,
    shared_facet: wire::FacetId,
) -> PostgresBootstrap {
    let mut shared = scenario_engine();
    let original_character = shared.world().actors[0]
        .character_id
        .clone()
        .expect("scenario player character");
    let temporary_character = tme_rules::CharacterId::new("prototype:social:second");
    let temporary_replacement = tme_rules::CharacterId::new("prototype:social:replacement");
    shared.world_mut().actors[0].stats.attack = 100;
    let mut second = shared.world().actors[0].clone();
    second.id = tme_rules::ActorId::new("player2");
    second.name = "Companion".to_string();
    second.character_id = Some(temporary_character.clone());
    second.hp = 1;
    second
        .character
        .as_mut()
        .expect("second social player character sheet")
        .resources
        .hp = 1;
    second.timing.tie_break_order += 100;
    second.carried.items.clear();
    second.carried.gold = Default::default();
    let preferences = shared
        .world()
        .communication_preferences
        .get(&original_character)
        .cloned()
        .unwrap_or_default();
    let presence = shared
        .world()
        .character_presence
        .get(&original_character)
        .copied()
        .expect("scenario character presence");
    let quest_state = shared
        .world()
        .quest_states
        .get(&original_character)
        .cloned();
    let mut replacement = shared.world().actors[0].clone();
    replacement.id = tme_rules::ActorId::new("player3");
    replacement.name = "Replacement".to_string();
    replacement.character_id = Some(temporary_replacement.clone());
    replacement.timing.tie_break_order += 200;
    shared.world_mut().actors.push(second);
    shared.world_mut().actors.push(replacement);
    shared
        .world_mut()
        .communication_preferences
        .insert(temporary_character.clone(), preferences);
    shared
        .world_mut()
        .communication_preferences
        .insert(temporary_replacement.clone(), Default::default());
    shared
        .world_mut()
        .character_presence
        .insert(temporary_character.clone(), presence);
    shared
        .world_mut()
        .character_presence
        .insert(temporary_replacement.clone(), presence);
    if let Some(quest_state) = quest_state {
        shared
            .world_mut()
            .quest_states
            .insert(temporary_character, quest_state.clone());
        shared
            .world_mut()
            .quest_states
            .insert(temporary_replacement, quest_state);
    }
    let arrival_id = shared
        .definition()
        .world_template()
        .arrivals()
        .keys()
        .min()
        .cloned()
        .expect("social scenario arrival");
    shared
        .clone()
        .advance_realtime_boundary()
        .expect("two-player social facet advances one boundary");

    let _ = arrival_id;
    PostgresBootstrap {
        world: PostgresWorldBootstrap {
            facet_id: shared_facet,
            key: "social-world".to_string(),
            engine: shared,
        },
        characters: vec![
            PostgresCharacterBootstrap {
                account_id: first_account,
                character_id: first_character,
                slot: 1,
                display_name: wire::DisplayName::new("Social One").unwrap(),
                actor_id: tme_rules::ActorId::new("player"),
            },
            PostgresCharacterBootstrap {
                account_id: second_account,
                character_id: second_character,
                slot: 1,
                display_name: wire::DisplayName::new("Social Two").unwrap(),
                actor_id: tme_rules::ActorId::new("player2"),
            },
            PostgresCharacterBootstrap {
                account_id: first_account,
                character_id: replacement_character,
                slot: 2,
                display_name: wire::DisplayName::new("Replacement").unwrap(),
                actor_id: tme_rules::ActorId::new("player3"),
            },
        ],
    }
}

fn scenario_engine() -> tme_rules::Engine {
    let mut scenario = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend([
        "..",
        "..",
        "content",
        "test-corpus",
        "world_topology_gallery.json",
    ]);
    tme_sim::load_engine_from_scenario(&scenario, Some(7)).unwrap()
}

async fn http_reconnect_replay_and_logout(
    backend: std::sync::Arc<PostgresState>,
    character_id: wire::CharacterId,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind HTTP integration listener");
    let address = listener.local_addr().expect("HTTP integration address");
    let host = address.to_string();
    let origin = format!("http://{host}");
    let config = ServerConfig::new(address, host.clone(), origin.clone()).expect("server config");
    let state = AppState::postgres(config, backend);
    let (stop_send, stop_receive) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(tme_server::runtime::serve(listener, state, async {
        let _ = stop_receive.await;
    }));

    let ready = http_request(address, &host, &origin, "GET", "/health/ready", None, None).await;
    assert_eq!(ready.status, 200);

    let old_root = concat!("/v", "2");
    let removed_routes = [
        ("POST", format!("{old_root}/login")),
        ("GET", format!("{old_root}/session")),
        ("POST", format!("{old_root}/logout")),
        ("POST", format!("{old_root}/characters/select")),
        ("POST", format!("{old_root}/socket-tickets")),
        ("POST", format!("{old_root}/characters/switch-facet")),
        // D4: the world selector is retired at the current root too, not just
        // behind the predecessor's version prefix.
        ("POST", "/v3/characters/switch-facet".to_string()),
        ("GET", "/v3/facets".to_string()),
        (
            "POST",
            format!("{old_root}/player-kill-marks/{}/forgive", Uuid::now_v7()),
        ),
        ("GET", format!("{old_root}/socket")),
    ];
    for (method, path) in removed_routes {
        let response = http_request(address, &host, &origin, method, &path, None, None).await;
        assert_eq!(response.status, 404, "removed route {method} {path}");
    }

    let login = http_request(
        address,
        &host,
        &origin,
        "POST",
        "/v3/login",
        None,
        Some(
            &serde_json::to_string(&wire::LoginRequestV1 {
                username: wire::Username::new(USERNAME).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(
        login.status,
        200,
        "{}",
        String::from_utf8_lossy(&login.body)
    );
    let set_cookie = login.headers.get("set-cookie").expect("session cookie");
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Strict"));
    let cookie = set_cookie.split(';').next().unwrap().to_string();
    let bootstrap: wire::SessionBootstrapV1 = serde_json::from_slice(&login.body).unwrap();
    assert_eq!(bootstrap.control_api_version, 3);

    let session = http_request(
        address,
        &host,
        &origin,
        "GET",
        "/v3/session",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(session.status, 200);
    let session_bootstrap: wire::SessionBootstrapV1 =
        serde_json::from_slice(&session.body).unwrap();
    assert_eq!(session_bootstrap.control_api_version, 3);

    let selection = post_json(
        address,
        &host,
        &origin,
        "/v3/characters/select",
        &cookie,
        &wire::CharacterSelectRequestV1 {
            csrf_token: session_bootstrap.csrf_token.clone(),
            character_id,
        },
    )
    .await;
    assert_eq!(selection.status, 200);
    let selection_body: wire::CharacterSelectionV1 =
        serde_json::from_slice(&selection.body).unwrap();
    assert_eq!(selection_body.control_api_version, 3);

    let first_ticket = issue_ticket(
        address,
        &host,
        &origin,
        &cookie,
        &session_bootstrap.csrf_token,
    )
    .await;
    let (mut first, first_welcome) =
        connect_character(address, &host, &origin, &first_ticket.ticket).await;
    let (actor_id, control_epoch, facet_revision) = welcome_parts(&first_welcome);
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(control_epoch),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(facet_revision),
        actor_id,
        intent: wire::Intent::Wait,
    };
    first
        .send(Message::Text(
            serde_json::to_string(&command).unwrap().into(),
        ))
        .await
        .unwrap();
    assert!(matches!(
        receive_envelope(&mut first).await,
        wire::ServerEnvelope::CommandResult {
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    assert!(matches!(
        receive_envelope(&mut first).await,
        wire::ServerEnvelope::StateUpdate { .. }
    ));

    let second_ticket = issue_ticket(
        address,
        &host,
        &origin,
        &cookie,
        &session_bootstrap.csrf_token,
    )
    .await;
    let (mut second, second_welcome) =
        connect_character(address, &host, &origin, &second_ticket.ticket).await;
    assert_eq!(
        wait_for_draining(&mut first).await,
        (wire::DrainingReason::ControlReplaced, true)
    );
    let (_, second_epoch, _) = welcome_parts(&second_welcome);
    assert_eq!(second_epoch, control_epoch + 1);
    second
        .send(Message::Text(
            serde_json::to_string(&command).unwrap().into(),
        ))
        .await
        .unwrap();
    assert!(matches!(
        receive_envelope(&mut second).await,
        wire::ServerEnvelope::CommandResult {
            replay_status: wire::ReplayStatus::Replayed,
            ..
        }
    ));
    assert!(matches!(
        receive_envelope(&mut second).await,
        wire::ServerEnvelope::StateUpdate { .. }
    ));

    let logout = post_json(
        address,
        &host,
        &origin,
        "/v3/logout",
        &cookie,
        &wire::LogoutRequestV1 {
            csrf_token: session_bootstrap.csrf_token,
        },
    )
    .await;
    assert_eq!(logout.status, 204);
    assert_eq!(
        wait_for_draining(&mut second).await,
        (wire::DrainingReason::SessionEnded, false)
    );

    let _ = stop_send.send(());
    server.await.unwrap().unwrap();
}

async fn issue_ticket(
    address: SocketAddr,
    host: &str,
    origin: &str,
    cookie: &str,
    csrf_token: &wire::CsrfToken,
) -> wire::SocketTicketV1 {
    let response = post_json(
        address,
        host,
        origin,
        "/v3/socket-tickets",
        cookie,
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
    cookie: &str,
    value: &T,
) -> HttpResponse {
    let body = serde_json::to_string(value).unwrap();
    http_request(
        address,
        host,
        origin,
        "POST",
        path,
        Some(cookie),
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
    cookie: &str,
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
        Some(cookie),
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
    cookie: Option<&str>,
    body: Option<&str>,
) -> HttpResponse {
    http_request_with_csrf(address, host, origin, method, path, cookie, None, body).await
}

#[allow(clippy::too_many_arguments)]
async fn http_request_with_csrf(
    address: SocketAddr,
    host: &str,
    origin: &str,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    csrf_token: Option<&str>,
    body: Option<&str>,
) -> HttpResponse {
    let host = host.to_string();
    let origin = origin.to_string();
    let method = method.to_string();
    let path = path.to_string();
    let cookie = cookie.map(str::to_string);
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
        if let Some(cookie) = cookie {
            request.push_str(&format!("Cookie: {cookie}\r\n"));
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
    let mut request = format!("ws://{address}/v3/socket")
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
