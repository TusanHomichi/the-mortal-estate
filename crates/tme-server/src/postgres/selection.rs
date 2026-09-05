use super::*;

impl PostgresState {
    /// Builds the candidate engine carrying every consequence this returning
    /// killer owes, and hands back the checkpoint plus the per-kill
    /// `linked_karma_added` the rules produced. Nothing is durable yet; the
    /// caller's transaction decides whether this lands.
    pub(super) async fn prepare_pending_consequences(
        &self,
        facet: &FacetHandle,
        pending: &[crate::store::PendingKillConsequence],
    ) -> Result<(u64, crate::facet::PreparedFacetCheckpoint, Vec<bool>), SessionError> {
        let epoch = self
            .next_transfer_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| SessionError::Unavailable)?;
        let assessments = pending
            .iter()
            .map(|owed| owed.assessment.clone())
            .collect::<Vec<_>>();
        let linked = facet
            .prepare_pending_kill_consequences(epoch, assessments)
            .await
            .map_err(|_| SessionError::Unavailable)?;
        let checkpoint = match facet.prepared_checkpoint(epoch).await {
            Ok(checkpoint) => checkpoint,
            Err(_) => {
                let _ = facet.rollback_transfer(epoch).await;
                return Err(SessionError::Unavailable);
            }
        };
        if checkpoint.facet_id != self.world.facet_id || linked.len() != pending.len() {
            let _ = facet.rollback_transfer(epoch).await;
            return Err(SessionError::Unavailable);
        }
        Ok((epoch, checkpoint, linked))
    }

    pub(super) async fn prepare_character_exit_candidate(
        &self,
        character_id: wire::CharacterId,
    ) -> Result<PreparedCharacterExit, SessionError> {
        let epoch = self
            .next_transfer_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| SessionError::Unavailable)?;
        let rules_character_id = CharacterId::new(character_id.to_string());
        let mut facets = Vec::new();
        {
            let handle = self.world.handle.clone();
            if handle
                .prepare_character_exit(epoch, rules_character_id.clone())
                .await
                .is_err()
            {
                Self::rollback_character_exit(epoch, &facets).await;
                return Err(SessionError::Unavailable);
            }
            let checkpoint = match handle.prepared_checkpoint(epoch).await {
                Ok(checkpoint) => checkpoint,
                Err(_) => {
                    let _ = handle.rollback_transfer(epoch).await;
                    Self::rollback_character_exit(epoch, &facets).await;
                    return Err(SessionError::Unavailable);
                }
            };
            facets.push((handle, checkpoint));
        }
        facets.sort_by_key(|(_, checkpoint)| checkpoint.facet_id);
        Ok(PreparedCharacterExit { epoch, facets })
    }

    pub(super) async fn persist_prepared_facets(
        tx: &mut Transaction<'_, Postgres>,
        facets: &[(FacetHandle, crate::facet::PreparedFacetCheckpoint)],
    ) -> Result<(), SessionError> {
        for (_, checkpoint) in facets {
            let row = sqlx::query(
                "SELECT facet_revision,last_server_sequence FROM tme.facets \
                 WHERE facet_id=$1 FOR UPDATE",
            )
            .bind(checkpoint.facet_id.as_uuid())
            .fetch_one(&mut **tx)
            .await
            .map_err(unavailable)?;
            if checked_u64(row.try_get("facet_revision").map_err(unavailable)?)
                .map_err(unavailable)?
                != checkpoint.before_revision
                || checked_u64(row.try_get("last_server_sequence").map_err(unavailable)?)
                    .map_err(unavailable)?
                    != checkpoint.server_sequence
            {
                return Err(SessionError::Unavailable);
            }
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
            .execute(&mut **tx)
            .await
            .map_err(unavailable)?;
            if updated.rows_affected() != 1 {
                return Err(SessionError::Unavailable);
            }
        }
        Ok(())
    }

    pub(super) async fn rollback_character_exit(
        epoch: u64,
        facets: &[(FacetHandle, crate::facet::PreparedFacetCheckpoint)],
    ) {
        for (handle, _) in facets {
            let _ = handle.rollback_transfer(epoch).await;
        }
    }

    pub(super) async fn publish_character_exit(
        &self,
        prepared: &PreparedCharacterExit,
    ) -> Result<(), SessionError> {
        for (handle, _) in &prepared.facets {
            if handle.commit_transfer(prepared.epoch).await.is_err() {
                self.ready.fail();
                return Err(SessionError::Unavailable);
            }
        }
        for (handle, _) in &prepared.facets {
            if handle.publish_transfer(prepared.epoch).await.is_err() {
                self.ready.fail();
                return Err(SessionError::Unavailable);
            }
        }
        Ok(())
    }

    pub async fn select_character(
        &self,
        session_cookie: &str,
        request: wire::CharacterSelectRequestV1,
    ) -> Result<wire::CharacterSelectionV1, SessionError> {
        let _transition = self.coordinator.transition().await;
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, session_cookie, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        let character = character_for_account(&mut tx, request.character_id, session.account_id)
            .await?
            .ok_or(SessionError::CharacterNotOwned)?;
        let replacing_character = session
            .selected_character_id
            .filter(|selected| *selected != character.character_id);
        let replaced_grant = replacing_character.and_then(|replaced| {
            self.live
                .lock()
                .ok()
                .and_then(|live| live.active_grants.get(&replaced).cloned())
        });
        let prepared_exit = match replacing_character {
            Some(replaced) => Some(self.prepare_character_exit_candidate(replaced).await?),
            None => None,
        };
        let durable = async {
            if let (Some(replaced), Some(prepared)) = (replacing_character, &prepared_exit) {
                Self::persist_prepared_facets(&mut tx, &prepared.facets).await?;
                sqlx::query(
                    "UPDATE tme.player_kill_marks SET karma_forgiveness_eligible=false \
                     WHERE forgiven_at IS NULL AND expired_at IS NULL \
                     AND karma_forgiveness_eligible AND ( \
                        (killer_character_id=$1 AND killer_session_id=$2) OR \
                        (victim_character_id=$1 AND victim_session_id=$2))",
                )
                .bind(replaced.as_uuid())
                .bind(session.session_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
                sqlx::query(
                    "UPDATE tme.characters SET control_epoch=control_epoch+1 \
                     WHERE character_id=$1 AND account_id=$2",
                )
                .bind(replaced.as_uuid())
                .bind(session.account_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
                sqlx::query(
                    "DELETE FROM tme.socket_tickets WHERE session_id=$1 AND character_id=$2 \
                     AND consumed_at IS NULL",
                )
                .bind(session.session_id.as_uuid())
                .bind(replaced.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            }
            sqlx::query("UPDATE tme.sessions SET selected_character_id=$2 WHERE session_id=$1")
                .bind(session.session_id.as_uuid())
                .bind(character.character_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            self.commit_gameplay_transaction(tx)
                .await
                .map_err(unavailable)
        }
        .await;
        if let Err(error) = durable {
            if let Some(prepared) = &prepared_exit {
                Self::rollback_character_exit(prepared.epoch, &prepared.facets).await;
            }
            return Err(error);
        }
        let revocation = if let Some(grant) = replaced_grant {
            if let Ok(mut live) = self.live.lock() {
                live.active_grants.remove(&grant.character_id);
            }
            if grant.facet_id == self.world.facet_id {
                let facet = &self.world;
                Some(
                    facet
                        .handle
                        .begin_revoke_grant(grant.connection_id, wire::DrainingReason::SessionEnded)
                        .await
                        .map_err(|_| SessionError::Unavailable)?,
                )
            } else {
                None
            }
        } else {
            None
        };
        if let Some(prepared) = &prepared_exit {
            self.publish_character_exit(prepared).await?;
        }
        if let Some(revocation) = revocation {
            revocation.await.map_err(|_| SessionError::Unavailable)?;
        }
        Ok(selection(&character))
    }

    pub async fn issue_ticket(
        &self,
        session_cookie: &str,
        request: wire::SocketTicketRequestV1,
        origin: &str,
        host: &str,
    ) -> Result<wire::SocketTicketV1, SessionError> {
        let _transition = self.coordinator.transition().await;
        let ticket = random_ticket().map_err(|_| SessionError::Unavailable)?;
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, session_cookie, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
            .bind(session.account_id.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
        crate::store::reschedule_player_kill_marks_raw(
            &mut tx,
            session.account_id.as_uuid(),
            false,
        )
        .await
        .map_err(unavailable)?;
        let active_marks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
             AND forgiven_at IS NULL AND expired_at IS NULL",
        )
        .bind(session.account_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if active_marks >= 4 {
            return Err(SessionError::GameplayMarkLocked);
        }
        let character_id = session
            .selected_character_id
            .ok_or(SessionError::CharacterNotSelected)?;
        let character = character_for_account(&mut tx, character_id, session.account_id)
            .await?
            .ok_or(SessionError::Unavailable)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.socket_tickets WHERE session_id=$1 AND \
             (consumed_at IS NULL OR consumed_at > statement_timestamp()-interval '30 seconds')",
        )
        .bind(session.session_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if count >= MAX_TICKETS_PER_SESSION as i64 {
            return Err(SessionError::Unavailable);
        }
        sqlx::query(
            "INSERT INTO tme.socket_tickets \
             (ticket_digest,session_id,account_id,character_id,actor_id, \
              expected_control_epoch,origin,host,selected_major,expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,1, \
                     statement_timestamp()+make_interval(secs=>$9))",
        )
        .bind(digest(ticket.expose_for_admission()).as_slice())
        .bind(session.session_id.as_uuid())
        .bind(session.account_id.as_uuid())
        .bind(character.character_id.as_uuid())
        .bind(character.actor_id.as_str())
        .bind(checked_i64(character.control_epoch).map_err(unavailable)?)
        .bind(origin)
        .bind(host)
        .bind(checked_i64(TICKET_LIFETIME.as_secs()).map_err(unavailable)?)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(wire::SocketTicketV1 {
            ticket,
            protocol_major: wire::PROTOCOL_MAJOR,
            supported_minors: vec![wire::PROTOCOL_MINOR],
            expires_in_seconds: wire::DecimalU64::new(TICKET_LIFETIME.as_secs()),
        })
    }
}
