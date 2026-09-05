use super::*;

impl PostgresState {
    pub async fn forgive_player_kill_mark(
        &self,
        session_cookie: &str,
        csrf_token: &wire::CsrfToken,
        mark_id: wire::PlayerKillMarkId,
        request: wire::ForgivePlayerKillMarkRequestV1,
    ) -> Result<wire::ForgivePlayerKillMarkResultV1, SessionError> {
        let _transition = self.coordinator.transition().await;
        let request_digest: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&serde_json::json!({
                "mark_id": mark_id,
                "request": request,
            }))
            .map_err(unavailable)?,
        )
        .into();

        // Authenticate and discover immutable routing facts without retaining
        // SQL row locks while a facet candidate is prepared.
        let mut discovery = serializable(self.store.pool()).await.map_err(unavailable)?;
        let discovered_session = active_session(&mut discovery, session_cookie, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(discovered_session.csrf_digest, csrf_token)?;
        if let Some(row) = sqlx::query(
            "SELECT request_digest,disposition,outcome_schema,outcome_bytes \
             FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2 FOR UPDATE",
        )
        .bind(discovered_session.account_id.as_uuid())
        .bind(request.request_id.as_uuid())
        .fetch_optional(&mut *discovery)
        .await
        .map_err(unavailable)?
        {
            let stored_digest: Vec<u8> = row.try_get("request_digest").map_err(unavailable)?;
            let disposition: String = row.try_get("disposition").map_err(unavailable)?;
            let outcome_schema: i16 = row.try_get("outcome_schema").map_err(unavailable)?;
            let outcome_bytes: Option<Vec<u8>> =
                row.try_get("outcome_bytes").map_err(unavailable)?;
            if stored_digest.as_slice() != request_digest
                || disposition != "accepted"
                || outcome_schema != 3
                || outcome_bytes.is_none()
            {
                return Err(SessionError::ForgivenessUnavailable);
            }
            discovery.commit().await.map_err(unavailable)?;
            return Ok(wire::ForgivePlayerKillMarkResultV1 {
                control_api_version: wire::CONTROL_API_VERSION,
                mark_id,
                replay_status: wire::ReplayStatus::Replayed,
            });
        }

        let discovered_mark = sqlx::query(
            "SELECT facet_kill_sequence,assessed_logical_millis::text AS assessed_logical_millis, \
                    killer_account_id,killer_character_id,victim_account_id,victim_character_id, \
                    killer_session_id,victim_session_id,linked_karma_added, \
                    karma_forgiveness_eligible,(forgiven_at IS NOT NULL) AS forgiven, \
                    (expired_at IS NOT NULL) AS expired \
             FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(mark_id.as_uuid())
        .fetch_optional(&mut *discovery)
        .await
        .map_err(unavailable)?
        .ok_or(SessionError::ForgivenessUnavailable)?;
        let origin_sequence_i64: i64 = discovered_mark
            .try_get("facet_kill_sequence")
            .map_err(unavailable)?;
        let origin_sequence = checked_u64(origin_sequence_i64).map_err(unavailable)?;
        let assessed_logical_millis: String = discovered_mark
            .try_get("assessed_logical_millis")
            .map_err(unavailable)?;
        let logical_time = assessed_logical_millis
            .parse::<u64>()
            .map_err(unavailable)?;
        let killer_account_id: Uuid = discovered_mark
            .try_get("killer_account_id")
            .map_err(unavailable)?;
        let killer_character_uuid: Uuid = discovered_mark
            .try_get("killer_character_id")
            .map_err(unavailable)?;
        let killer_character_id =
            wire::CharacterId::new(killer_character_uuid).map_err(unavailable)?;
        let victim_account_id: Uuid = discovered_mark
            .try_get("victim_account_id")
            .map_err(unavailable)?;
        let victim_character_uuid: Uuid = discovered_mark
            .try_get("victim_character_id")
            .map_err(unavailable)?;
        let victim_character_id =
            wire::CharacterId::new(victim_character_uuid).map_err(unavailable)?;
        let killer_session_id: Option<Uuid> = discovered_mark
            .try_get("killer_session_id")
            .map_err(unavailable)?;
        let victim_session_id: Uuid = discovered_mark
            .try_get("victim_session_id")
            .map_err(unavailable)?;
        let linked_karma_added: bool = discovered_mark
            .try_get("linked_karma_added")
            .map_err(unavailable)?;
        let karma_forgiveness_eligible: bool = discovered_mark
            .try_get("karma_forgiveness_eligible")
            .map_err(unavailable)?;
        if victim_account_id != discovered_session.account_id.as_uuid()
            || discovered_mark
                .try_get::<bool, _>("forgiven")
                .map_err(unavailable)?
            || discovered_mark
                .try_get::<bool, _>("expired")
                .map_err(unavailable)?
        {
            return Err(SessionError::ForgivenessUnavailable);
        }

        let prepared_forgiveness = if linked_karma_added && karma_forgiveness_eligible {
            // One world: the killer's character is hosted here or nowhere.
            let _: Uuid = sqlx::query_scalar(
                "SELECT character_id FROM tme.characters WHERE character_id=$1 AND account_id=$2",
            )
            .bind(killer_character_uuid)
            .bind(killer_account_id)
            .fetch_optional(&mut *discovery)
            .await
            .map_err(unavailable)?
            .ok_or(SessionError::Unavailable)?;
            let handle = self.world.handle.clone();
            let epoch = self
                .next_transfer_epoch
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| SessionError::Unavailable)?;
            let assessment = tme_rules::PlayerKillAssessmentV1 {
                facet_kill_sequence: origin_sequence,
                killer_character_id: CharacterId::new(killer_character_id.to_string()),
                victim_character_id: CharacterId::new(victim_character_id.to_string()),
                exempt_self_defense: false,
                consequence: tme_rules::PlayerKillConsequenceV1::AppliedHere {
                    linked_karma_added: true,
                },
                logical_time: tme_rules::LogicalTime::from_millis(logical_time),
            };
            discovery.commit().await.map_err(unavailable)?;
            if handle
                .prepare_player_kill_forgiveness(epoch, assessment)
                .await
                .is_err()
            {
                return Err(SessionError::Unavailable);
            }
            let checkpoint = match handle.prepared_checkpoint(epoch).await {
                Ok(checkpoint) => checkpoint,
                Err(_) => {
                    let _ = handle.rollback_transfer(epoch).await;
                    return Err(SessionError::Unavailable);
                }
            };
            if checkpoint.facet_id != self.world.facet_id {
                let _ = handle.rollback_transfer(epoch).await;
                return Err(SessionError::Unavailable);
            }
            Some((handle, checkpoint, epoch))
        } else {
            discovery.commit().await.map_err(unavailable)?;
            None
        };

        let durable = async {
            let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;

            // All gameplay transactions take durable facet rows before
            // accounts and marks. Preparing the in-memory candidate first
            // quiesces ordinary commands without holding a SQL account lock.
            if let Some((_, checkpoint, _)) = &prepared_forgiveness {
                let row = sqlx::query(
                    "SELECT facet_revision,last_server_sequence FROM tme.facets \
                     WHERE facet_id=$1 FOR UPDATE",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?;
                if checked_u64(row.try_get("facet_revision").map_err(unavailable)?)
                    .map_err(unavailable)?
                    != checkpoint.before_revision
                    || checked_u64(
                        row.try_get("last_server_sequence")
                            .map_err(unavailable)?,
                    )
                    .map_err(unavailable)?
                        != checkpoint.server_sequence
                {
                    return Err(SessionError::Unavailable);
                }
            }

            let session = active_session(&mut tx, session_cookie, false)
                .await?
                .ok_or(SessionError::AuthenticationRequired)?;
            validate_csrf(session.csrf_digest, csrf_token)?;
            if session.account_id != discovered_session.account_id {
                return Err(SessionError::ForgivenessUnavailable);
            }

            if let Some(row) = sqlx::query(
                "SELECT request_digest,disposition,outcome_schema,outcome_bytes \
                 FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2 FOR UPDATE",
            )
            .bind(session.account_id.as_uuid())
            .bind(request.request_id.as_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            {
                let stored_digest: Vec<u8> =
                    row.try_get("request_digest").map_err(unavailable)?;
                let disposition: String = row.try_get("disposition").map_err(unavailable)?;
                let outcome_schema: i16 =
                    row.try_get("outcome_schema").map_err(unavailable)?;
                let outcome_bytes: Option<Vec<u8>> =
                    row.try_get("outcome_bytes").map_err(unavailable)?;
                if stored_digest.as_slice() != request_digest
                    || disposition != "accepted"
                    || outcome_schema != 3
                    || outcome_bytes.is_none()
                {
                    return Err(SessionError::ForgivenessUnavailable);
                }
                tx.commit().await.map_err(unavailable)?;
                return Ok(true);
            }

            let mut account_ids = vec![killer_account_id, victim_account_id];
            account_ids.sort_unstable();
            account_ids.dedup();
            let locked: Vec<Uuid> = sqlx::query_scalar(
                "SELECT account_id FROM tme.accounts WHERE account_id=ANY($1) \
                 ORDER BY account_id FOR UPDATE",
            )
            .bind(account_ids.clone())
            .fetch_all(&mut *tx)
            .await
            .map_err(unavailable)?;
            if locked != account_ids {
                return Err(SessionError::Unavailable);
            }

            let mark = sqlx::query(
                "SELECT facet_kill_sequence,assessed_logical_millis::text AS assessed_logical_millis, \
                        killer_account_id,killer_character_id,victim_account_id,victim_character_id, \
                        killer_session_id,victim_session_id,linked_karma_added, \
                        karma_forgiveness_eligible,(forgiven_at IS NOT NULL) AS forgiven, \
                        (expired_at IS NOT NULL) AS expired \
                 FROM tme.player_kill_marks WHERE mark_id=$1 FOR UPDATE",
            )
            .bind(mark_id.as_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            .ok_or(SessionError::ForgivenessUnavailable)?;
            if mark
                    .try_get::<i64, _>("facet_kill_sequence")
                    .map_err(unavailable)?
                    != origin_sequence_i64
                || mark
                    .try_get::<String, _>("assessed_logical_millis")
                    .map_err(unavailable)?
                    != assessed_logical_millis
                || mark
                    .try_get::<Uuid, _>("killer_account_id")
                    .map_err(unavailable)?
                    != killer_account_id
                || mark
                    .try_get::<Uuid, _>("killer_character_id")
                    .map_err(unavailable)?
                    != killer_character_uuid
                || mark
                    .try_get::<Uuid, _>("victim_account_id")
                    .map_err(unavailable)?
                    != victim_account_id
                || mark
                    .try_get::<Uuid, _>("victim_character_id")
                    .map_err(unavailable)?
                    != victim_character_uuid
                || mark
                    .try_get::<Option<Uuid>, _>("killer_session_id")
                    .map_err(unavailable)?
                    != killer_session_id
                || mark
                    .try_get::<Uuid, _>("victim_session_id")
                    .map_err(unavailable)?
                    != victim_session_id
                || mark
                    .try_get::<bool, _>("linked_karma_added")
                    .map_err(unavailable)?
                    != linked_karma_added
                || mark
                    .try_get::<bool, _>("karma_forgiveness_eligible")
                    .map_err(unavailable)?
                    != karma_forgiveness_eligible
                || mark.try_get::<bool, _>("forgiven").map_err(unavailable)?
                || mark.try_get::<bool, _>("expired").map_err(unavailable)?
                || victim_account_id != session.account_id.as_uuid()
            {
                return Err(SessionError::ForgivenessUnavailable);
            }

            crate::store::reschedule_player_kill_marks_raw(
                &mut tx,
                killer_account_id,
                false,
            )
            .await
            .map_err(unavailable)?;
            let still_active: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tme.player_kill_marks WHERE mark_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL)",
            )
            .bind(mark_id.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
            if !still_active {
                return Err(SessionError::ForgivenessUnavailable);
            }

            let active_count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL",
            )
            .bind(killer_account_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
            let (same_facet, no_gameplay) = self
                .live
                .lock()
                .map(|live| {
                    let killer = live.active_grants.get(&killer_character_id);
                    let victim = live.active_grants.get(&victim_character_id);
                    let same_facet = killer.zip(victim).is_some_and(|(killer, victim)| {
                        killer.account_id.as_uuid() == killer_account_id
                            && victim.account_id == session.account_id
                            && Some(killer.session_id.as_uuid()) == killer_session_id
                            && victim.session_id.as_uuid() == victim_session_id
                            && killer.facet_id == victim.facet_id
                    });
                    let no_gameplay = !live.active_grants.values().any(|grant| {
                        grant.account_id.as_uuid() == killer_account_id
                            || grant.account_id == session.account_id
                    });
                    (same_facet, no_gameplay)
                })
                .map_err(|_| SessionError::Unavailable)?;
            let killer_causal_session_active = match killer_session_id {
                Some(killer_session_id) => sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE session_id=$1 \
                     AND account_id=$2 AND selected_character_id=$3 AND revoked_at IS NULL \
                     AND idle_expires_at>statement_timestamp() \
                     AND absolute_expires_at>statement_timestamp())",
                )
                .bind(killer_session_id)
                .bind(killer_account_id)
                .bind(killer_character_uuid)
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?,
                None => false,
            };
            if !(same_facet
                || (active_count >= 4 && no_gameplay && killer_causal_session_active))
            {
                return Err(SessionError::ForgivenessUnavailable);
            }

            if let Some((_, checkpoint, _)) = &prepared_forgiveness {
                let updated = sqlx::query(
                    "UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3, \
                     facet_revision=$4,updated_at=statement_timestamp() WHERE facet_id=$1 \
                     AND facet_revision=$5 AND last_server_sequence=$6",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .bind(checkpoint.checkpoint.as_bytes())
                .bind(checkpoint.checkpoint.sha256().as_slice())
                .bind(checked_i64(checkpoint.after_revision).map_err(unavailable)?)
                .bind(checked_i64(checkpoint.before_revision).map_err(unavailable)?)
                .bind(checked_i64(checkpoint.server_sequence).map_err(unavailable)?)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
                if updated.rows_affected() != 1 {
                    return Err(SessionError::Unavailable);
                }
            }

            let updated = sqlx::query(
                "UPDATE tme.player_kill_marks SET forgiven_at=tme.mark_now(), \
                 forgiven_by_account_id=$2,expires_at=NULL WHERE mark_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL",
            )
            .bind(mark_id.as_uuid())
            .bind(session.account_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            if updated.rows_affected() != 1 {
                return Err(SessionError::ForgivenessUnavailable);
            }
            crate::store::reschedule_player_kill_marks_raw(
                &mut tx,
                killer_account_id,
                true,
            )
            .await
            .map_err(unavailable)?;
            let outcome = ReceiptOutcomeV3::accepted_control()
                .encode()
                .map_err(unavailable)?;
            let inserted = sqlx::query(
                "INSERT INTO tme.command_receipts \
                 (account_id,command_id,request_digest,session_id,outcome_schema,disposition, \
                  outcome_bytes,full_expires_at) \
                 VALUES ($1,$2,$3,$4,3,'accepted',$5,statement_timestamp()+interval '90 days') \
                 ON CONFLICT DO NOTHING",
            )
            .bind(session.account_id.as_uuid())
            .bind(request.request_id.as_uuid())
            .bind(request_digest.as_slice())
            .bind(session.session_id.as_uuid())
            .bind(outcome)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            if inserted.rows_affected() != 1 {
                return Err(SessionError::ForgivenessUnavailable);
            }
            audit(
                &mut tx,
                AuditEvent {
                    account_id: Some(session.account_id.as_uuid()),
                    session_id: Some(session.session_id.as_uuid()),
                    character_id: Some(victim_character_uuid),
                    command_id: Some(request.request_id.as_uuid()),
                    actor: "runtime",
                    action: "mark_forgive",
                    result: "success",
                },
            )
            .await
            .map_err(unavailable)?;
            self.commit_gameplay_transaction(tx)
                .await
                .map_err(unavailable)?;
            Ok(false)
        }
        .await;

        let replayed = match durable {
            Ok(replayed) => replayed,
            Err(error) => {
                if let Some((handle, _, epoch)) = &prepared_forgiveness {
                    let _ = handle.rollback_transfer(*epoch).await;
                }
                return Err(error);
            }
        };
        if replayed {
            if let Some((handle, _, epoch)) = &prepared_forgiveness {
                let _ = handle.rollback_transfer(*epoch).await;
            }
            return Ok(wire::ForgivePlayerKillMarkResultV1 {
                control_api_version: wire::CONTROL_API_VERSION,
                mark_id,
                replay_status: wire::ReplayStatus::Replayed,
            });
        }
        if let Some((handle, _, epoch)) = prepared_forgiveness
            && (handle.commit_transfer(epoch).await.is_err()
                || handle.publish_transfer(epoch).await.is_err())
        {
            self.ready.fail();
            return Err(SessionError::Unavailable);
        }
        Ok(wire::ForgivePlayerKillMarkResultV1 {
            control_api_version: wire::CONTROL_API_VERSION,
            mark_id,
            replay_status: wire::ReplayStatus::New,
        })
    }
}
