use super::*;

#[tokio::test]
async fn prepared_mutation_reserves_commits_and_publishes_one_fresh_sequence() {
    let engine = tme_sim::load_engine_from_scenario(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-corpus/world_topology_gallery.json"),
        Some(7),
    )
    .unwrap();
    let character_id = wire::CharacterId::new(uuid::Uuid::now_v7()).unwrap();
    let rules_id = tme_rules::CharacterId::new(character_id.to_string());
    let actor_id = tme_rules::ActorId::new("player");
    let engine = engine
        .prepare_character_id_rekey(&actor_id, rules_id.clone())
        .unwrap();
    let facet_id = wire::FacetId::new(uuid::Uuid::now_v7()).unwrap();
    let connection_id = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let handle = FacetHandle::spawn_with_id(facet_id, engine);
    let (outbound, mut receive) = mpsc::channel(16);
    let (terminal, _terminal) = watch::channel(None);
    let welcome = handle
        .install_grant(
            ControlGrant {
                account_id: wire::AccountId::new(uuid::Uuid::now_v7()).unwrap(),
                session_id: wire::SessionId::new(uuid::Uuid::now_v7()).unwrap(),
                character_id,
                connection_id,
                facet_id,
                actor_id,
                control_epoch: 1,
            },
            outbound,
            terminal,
        )
        .await
        .unwrap();
    handle
        .prepare_character_exit(1, rules_id.clone())
        .await
        .unwrap();
    let candidate = handle.prepared_checkpoint(1).await.unwrap();
    assert_eq!(candidate.before_sequence, welcome.server_sequence);
    assert_eq!(candidate.after_sequence, welcome.server_sequence + 1);
    handle.rollback_transfer(1).await.unwrap();
    assert!(receive.try_recv().is_err());
    handle.prepare_character_exit(2, rules_id).await.unwrap();
    let candidate = handle.prepared_checkpoint(2).await.unwrap();
    assert_eq!(candidate.before_sequence, welcome.server_sequence);
    handle.commit_transfer(2).await.unwrap();
    handle.publish_transfer(2).await.unwrap();
    let wire::ServerEnvelope::StateUpdate {
        server_sequence,
        world_revision,
        ..
    } = receive.recv().await.unwrap()
    else {
        panic!("full update required")
    };
    assert_eq!(server_sequence.get(), candidate.after_sequence);
    assert_eq!(world_revision.get(), candidate.after_revision);
}
