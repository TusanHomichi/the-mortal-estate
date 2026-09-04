use super::*;

#[cfg(test)]
pub(super) async fn ev_certify_command_pipeline(
    pool: &PgPool,
    state: &Arc<PostgresState>,
    fixture: EvDatabaseFixture,
    cookie: &str,
) {
    use crate::coordinator::Reservation;
    use crate::store::EvStoreFault;

    eprintln!("EV source-fault stage: command pipeline");

    let (grant, welcome, outbound, mut outbound_receive) = ev_admit_character(state, cookie).await;
    assert_eq!(grant.control.facet_id, fixture.world_id);
    let initial = ev_current_state(&grant).await;
    assert_eq!(ev_state_revision(&initial), welcome.facet_revision);

    let reservation_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let reservation_command = ev_wire_command(
        &grant,
        reservation_id,
        1,
        ev_state_revision(&initial),
        false,
    );
    state.store.ev_arm_fault(EvStoreFault::ReceiptSqlAcquire);
    assert!(matches!(
        state
            .coordinator
            .reserve(fixture.account_id, reservation_id, &reservation_command)
            .await,
        Reservation::Unavailable
    ));
    state.store.ev_assert_fault_consumed();
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, reservation_id).await,
        (0, 0)
    );
    let digest = ev_reserve_new(state, fixture, reservation_id, &reservation_command).await;
    state
        .coordinator
        .release(fixture.account_id, reservation_id, digest);

    for fault in [
        EvStoreFault::AuthorityRejectionInsert,
        EvStoreFault::AuthorityRejectionCommit,
    ] {
        let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
        let command = ev_wire_command(&grant, command_id, 1, ev_state_revision(&initial), false);
        let digest = ev_reserve_new(state, fixture, command_id, &command).await;
        state.store.ev_arm_fault(fault);
        assert!(
            state
                .coordinator
                .complete_authority_rejection(
                    fixture.account_id,
                    fixture.session_id,
                    command_id,
                    digest,
                    wire::RejectionCode::StaleControlEpoch,
                )
                .await
                .is_err(),
            "{fault:?} must fail authority-rejection persistence"
        );
        state.store.ev_assert_fault_consumed();
        assert_eq!(
            ev_command_artifacts(pool, fixture.account_id, command_id).await,
            (0, 0)
        );
        let retry_digest = ev_reserve_new(state, fixture, command_id, &command).await;
        assert_eq!(retry_digest, digest);
        state
            .coordinator
            .release(fixture.account_id, command_id, retry_digest);
    }

    let ambiguous_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let ambiguous_command =
        ev_wire_command(&grant, ambiguous_id, 1, ev_state_revision(&initial), false);
    let ambiguous_digest = ev_reserve_new(state, fixture, ambiguous_id, &ambiguous_command).await;
    state
        .store
        .ev_arm_fault(EvStoreFault::AuthorityRejectionOutcomeUnknown);
    assert!(
        state
            .coordinator
            .complete_authority_rejection(
                fixture.account_id,
                fixture.session_id,
                ambiguous_id,
                ambiguous_digest,
                wire::RejectionCode::StaleControlEpoch,
            )
            .await
            .is_err(),
        "lost authority-rejection commit result must not report success"
    );
    state.store.ev_assert_fault_consumed();
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, ambiguous_id).await,
        (1, 1)
    );
    assert!(matches!(
        state
            .coordinator
            .reserve(fixture.account_id, ambiguous_id, &ambiguous_command)
            .await,
        Reservation::Replay(envelope)
            if matches!(
                *envelope,
                wire::ServerEnvelope::CommandResult {
                    disposition: wire::CommandDisposition::Rejected {
                        code: wire::RejectionCode::StaleControlEpoch,
                    },
                    replay_status: wire::ReplayStatus::Replayed,
                    ..
                }
            )
    ));

    let queue_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let queue_command = ev_wire_command(&grant, queue_id, 1, ev_state_revision(&initial), false);
    let queue_digest = ev_reserve_new(state, fixture, queue_id, &queue_command).await;
    let (release, entered) = grant.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let rules_character_id = CharacterId::new(fixture.character_id.to_string());
    let mut queued = 0;
    while grant
        .facet
        .ev_try_inspect(rules_character_id.clone())
        .is_ok()
    {
        queued += 1;
    }
    assert_eq!(queued, crate::config::FACET_MAILBOX_CAPACITY);
    assert!(matches!(
        grant.facet.try_command(ev_facet_command(
            &grant,
            queue_id,
            1,
            ev_state_revision(&initial),
            false,
            queue_digest,
            EvCommandFault::None,
        )),
        Err(crate::facet::FacetError::QueueFull)
    ));
    state
        .coordinator
        .release(fixture.account_id, queue_id, queue_digest);
    release.send(()).unwrap();
    let drained = ev_wait_for_mailbox_state(&grant).await;
    assert_eq!(drained, initial);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, queue_id).await,
        (0, 0)
    );

    let failures = [
        ("checkpoint_export", None),
        ("sql_acquire", Some(EvStoreFault::CommandSqlAcquire)),
        ("row_lock", Some(EvStoreFault::CommandRowLock)),
        ("receipt_insert", Some(EvStoreFault::CommandReceiptInsert)),
        ("durable_effects", Some(EvStoreFault::CommandDurableEffects)),
        (
            "compare_and_swap",
            Some(EvStoreFault::CommandCompareAndSwap),
        ),
        ("audit", Some(EvStoreFault::CommandAudit)),
        ("commit", Some(EvStoreFault::CommandCommit)),
    ];
    for (label, fault) in failures {
        let before_state = ev_current_state(&grant).await;
        let before_store = ev_facet_row(pool, fixture.world_id).await;
        let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
        let command = ev_wire_command(
            &grant,
            command_id,
            1,
            ev_state_revision(&before_state),
            false,
        );
        let digest = ev_reserve_new(state, fixture, command_id, &command).await;
        if let Some(fault) = fault {
            state.store.ev_arm_fault(fault);
        }
        let receive = grant
            .facet
            .try_command(ev_facet_command(
                &grant,
                command_id,
                1,
                ev_state_revision(&before_state),
                false,
                digest,
                if label == "checkpoint_export" {
                    EvCommandFault::CheckpointExport
                } else {
                    EvCommandFault::None
                },
            ))
            .unwrap();
        assert!(receive.await.is_err(), "{label} emitted a success reply");
        state
            .coordinator
            .release(fixture.account_id, command_id, digest);
        if fault.is_some() {
            state.store.ev_assert_fault_consumed();
        }
        let after_state = ev_current_state(&grant).await;
        assert_eq!(after_state, before_state, "{label} swapped in-memory state");
        assert_eq!(
            ev_facet_row(pool, fixture.world_id).await,
            before_store,
            "{label} changed the durable checkpoint"
        );
        assert_eq!(
            ev_command_artifacts(pool, fixture.account_id, command_id).await,
            (0, 0),
            "{label} leaked receipt or audit"
        );
        assert!(outbound_receive.try_recv().is_err(), "{label} fanned out");
    }

    let accepted_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let before = ev_current_state(&grant).await;
    let before_revision = ev_state_revision(&before);
    let accepted_wire = ev_wire_command(&grant, accepted_id, 1, before_revision, false);
    let (left, right) = tokio::join!(
        state
            .coordinator
            .reserve(fixture.account_id, accepted_id, &accepted_wire),
        state
            .coordinator
            .reserve(fixture.account_id, accepted_id, &accepted_wire),
    );
    let accepted_digest = match (left, right) {
        (Reservation::New { digest }, Reservation::InProgress)
        | (Reservation::InProgress, Reservation::New { digest }) => digest,
        _ => panic!("same-ID gameplay reservation race must be new/in-progress"),
    };
    let (release, entered) = grant.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let accepted_receive = grant
        .facet
        .try_command(ev_facet_command(
            &grant,
            accepted_id,
            1,
            before_revision,
            false,
            accepted_digest,
            EvCommandFault::None,
        ))
        .unwrap();
    let tick_receive = grant
        .facet
        .ev_try_tick(grant.control.actor_id.clone())
        .unwrap();
    release.send(()).unwrap();
    let accepted = accepted_receive.await.unwrap();
    let tick = tick_receive.await.unwrap();
    state
        .coordinator
        .finish(fixture.account_id, accepted_id, accepted_digest);
    assert!(matches!(
        accepted.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    let after = ev_current_state(&grant).await;
    assert_eq!(ev_state_revision(&after), tick.facet_revision);
    assert_eq!(tick.facet_revision, before_revision + 2);
    assert!(tick.outcome.state_changed);
    let wire::ServerEnvelope::StateUpdate { frame, .. } = &after else {
        unreachable!()
    };
    assert!(
        !frame.social.pages_enabled,
        "in-memory swap is authoritative"
    );
    assert!(matches!(
        outbound_receive.try_recv(),
        Ok(wire::ServerEnvelope::StateUpdate { .. })
    ));
    assert!(matches!(
        outbound_receive.try_recv(),
        Ok(wire::ServerEnvelope::StateUpdate { .. })
    ));
    let durable_after_tick = ev_facet_row(pool, fixture.world_id).await;
    assert_eq!(
        durable_after_tick.0,
        i64::try_from(tick.facet_revision).unwrap()
    );
    assert_eq!(
        durable_after_tick.1,
        i64::try_from(tick.server_sequence).unwrap()
    );
    assert_eq!(durable_after_tick.2, tick.checkpoint.as_bytes());
    assert_eq!(durable_after_tick.3, tick.checkpoint.sha256());
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, accepted_id).await,
        (1, 1)
    );
    let replay_before = ev_facet_row(pool, fixture.world_id).await;
    assert!(matches!(
        state
            .coordinator
            .reserve(fixture.account_id, accepted_id, &accepted_wire)
            .await,
        Reservation::Replay(_)
    ));
    assert_eq!(
        ev_facet_row(pool, fixture.world_id).await,
        replay_before,
        "same-ID gameplay replay executed a second mutation"
    );

    while outbound_receive.try_recv().is_ok() {}
    for _ in 0..crate::config::OUTBOUND_QUEUE_CAPACITY {
        outbound
            .try_send(wire::ServerEnvelope::ServerDraining {
                reason: wire::DrainingReason::Shutdown,
                reconnect_hint: false,
            })
            .unwrap();
    }
    let publication_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let publication_wire = ev_wire_command(&grant, publication_id, 2, tick.facet_revision, true);
    let publication_digest =
        ev_reserve_new(state, fixture, publication_id, &publication_wire).await;
    let publication_reply = grant
        .facet
        .try_command(ev_facet_command(
            &grant,
            publication_id,
            2,
            tick.facet_revision,
            true,
            publication_digest,
            EvCommandFault::None,
        ))
        .unwrap()
        .await
        .unwrap();
    state
        .coordinator
        .finish(fixture.account_id, publication_id, publication_digest);
    assert!(matches!(
        publication_reply.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Rejected {
                code: wire::RejectionCode::ProjectionFailed,
            },
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, publication_id).await,
        (1, 1)
    );
    let detached = grant
        .facet
        .try_current_state(grant.control.connection_id)
        .unwrap()
        .await
        .unwrap();
    assert!(
        detached.is_err(),
        "failed publication must detach the observer"
    );
    for _ in 0..crate::config::OUTBOUND_QUEUE_CAPACITY {
        assert!(matches!(
            outbound_receive.try_recv(),
            Ok(wire::ServerEnvelope::ServerDraining { .. })
        ));
    }
    assert!(
        outbound_receive.try_recv().is_err(),
        "projection failure published a state update into the saturated queue"
    );

    let (presence_grant, presence_welcome, _presence_outbound, _presence_receive) =
        ev_admit_character(state, cookie).await;
    let presence_command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let presence_wire = ev_wire_command(
        &presence_grant,
        presence_command_id,
        1,
        presence_welcome.facet_revision,
        true,
    );
    let presence_digest = ev_reserve_new(state, fixture, presence_command_id, &presence_wire).await;
    let (release, entered) = presence_grant.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let presence_command = presence_grant
        .facet
        .try_command(ev_facet_command(
            &presence_grant,
            presence_command_id,
            1,
            presence_welcome.facet_revision,
            true,
            presence_digest,
            EvCommandFault::None,
        ))
        .unwrap();
    let presence_detach = presence_grant
        .facet
        .ev_try_detach(presence_grant.control.connection_id)
        .unwrap();
    release.send(()).unwrap();
    let presence_reply = presence_command.await.unwrap();
    let presence_step = presence_detach.await.unwrap();
    state
        .coordinator
        .finish(fixture.account_id, presence_command_id, presence_digest);
    assert!(matches!(
        presence_reply.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            ..
        }
    ));
    assert_eq!(
        presence_step.facet_revision,
        presence_welcome.facet_revision + 2,
        "command and presence commits must occupy consecutive mailbox revisions"
    );
    let durable_presence = ev_facet_row(pool, fixture.world_id).await;
    assert_eq!(
        durable_presence.0,
        i64::try_from(presence_step.facet_revision).unwrap()
    );
    assert_eq!(
        durable_presence.1,
        i64::try_from(presence_step.server_sequence).unwrap()
    );
    assert_eq!(
        durable_presence.2.as_slice(),
        presence_step.checkpoint.as_bytes()
    );

    let (replacement, replacement_welcome, _replacement_outbound, _replacement_receive) =
        ev_admit_character(state, cookie).await;
    let reply_loss_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let reply_loss_wire = ev_wire_command(
        &replacement,
        reply_loss_id,
        1,
        replacement_welcome.facet_revision,
        false,
    );
    let reply_loss_digest = ev_reserve_new(state, fixture, reply_loss_id, &reply_loss_wire).await;
    let (release, entered) = replacement.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let reply_loss = replacement
        .facet
        .try_command(ev_facet_command(
            &replacement,
            reply_loss_id,
            1,
            replacement_welcome.facet_revision,
            false,
            reply_loss_digest,
            EvCommandFault::None,
        ))
        .unwrap();
    drop(reply_loss);
    release.send(()).unwrap();
    loop {
        if state
            .store
            .receipt(fixture.account_id, reply_loss_id)
            .await
            .unwrap()
            .is_some()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    state
        .coordinator
        .release(fixture.account_id, reply_loss_id, reply_loss_digest);
    let expected = state
        .store
        .receipt(fixture.account_id, reply_loss_id)
        .await
        .unwrap()
        .unwrap()
        .outcome
        .unwrap()
        .to_envelope(reply_loss_id, wire::ReplayStatus::Replayed)
        .unwrap();
    let replay = match state
        .coordinator
        .reserve(fixture.account_id, reply_loss_id, &reply_loss_wire)
        .await
    {
        Reservation::Replay(envelope) => *envelope,
        _ => panic!("committed command with lost reply must replay"),
    };
    assert_eq!(replay, expected);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, reply_loss_id).await,
        (1, 1)
    );

    let rows = sqlx::query(
        "SELECT command_id,before_revision,after_revision,server_sequence \
         FROM tme.command_receipts WHERE account_id=$1 AND command_id=ANY($2) \
         ORDER BY server_sequence",
    )
    .bind(fixture.account_id.as_uuid())
    .bind(vec![
        accepted_id.as_uuid(),
        publication_id.as_uuid(),
        reply_loss_id.as_uuid(),
    ])
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 3);
    let mut previous_sequence = None;
    for row in rows {
        let command_id: Uuid = row.try_get("command_id").unwrap();
        let before: i64 = row.try_get("before_revision").unwrap();
        let after: i64 = row.try_get("after_revision").unwrap();
        let sequence: i64 = row.try_get("server_sequence").unwrap();
        if command_id == publication_id.as_uuid() {
            assert_eq!(after, before, "projection rejection changed the revision");
        } else {
            assert_eq!(after, before + 1);
        }
        if let Some(previous) = previous_sequence {
            assert!(
                sequence > previous,
                "mailbox sequences remain strictly ordered"
            );
        }
        previous_sequence = Some(sequence);
    }
    eprintln!("EV source-fault stage complete: command pipeline");
}
