use super::*;

impl PostgresState {
    pub async fn logout(
        &self,
        session_cookie: &str,
        request: wire::LogoutRequestV1,
    ) -> Result<(), SessionError> {
        let _transition = self.coordinator.transition().await;
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, session_cookie, false)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        let grants = {
            let live = self.live.lock().map_err(|_| SessionError::Unavailable)?;
            live.active_grants
                .values()
                .filter(|grant| grant.session_id == session.session_id)
                .cloned()
                .collect::<Vec<_>>()
        };
        let prepared_exit = match session.selected_character_id {
            Some(character_id) => Some(self.prepare_character_exit_candidate(character_id).await?),
            None => None,
        };
        let durable = async {
            if let Some(prepared) = &prepared_exit {
                Self::persist_prepared_facets(&mut tx, &prepared.facets).await?;
            }
            sqlx::query(
                "UPDATE tme.sessions SET revoked_at=statement_timestamp() WHERE session_id=$1",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            sqlx::query(
                "UPDATE tme.player_kill_marks SET karma_forgiveness_eligible=false \
                 WHERE forgiven_at IS NULL AND expired_at IS NULL \
                 AND karma_forgiveness_eligible \
                 AND (killer_session_id=$1 OR victim_session_id=$1)",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            sqlx::query(
                "DELETE FROM tme.socket_tickets WHERE session_id=$1 AND consumed_at IS NULL",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            if let Some(character_id) = session.selected_character_id {
                sqlx::query(
                    "UPDATE tme.characters SET control_epoch=control_epoch+1 \
                     WHERE character_id=$1 AND account_id=$2",
                )
                .bind(character_id.as_uuid())
                .bind(session.account_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            }
            audit(
                &mut tx,
                AuditEvent {
                    account_id: Some(session.account_id.as_uuid()),
                    session_id: Some(session.session_id.as_uuid()),
                    character_id: session
                        .selected_character_id
                        .map(|character_id| character_id.as_uuid()),
                    command_id: None,
                    actor: "runtime",
                    action: "logout",
                    result: "success",
                },
            )
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
        let mut revocations = Vec::new();
        for grant in grants {
            if let Ok(mut live) = self.live.lock() {
                live.active_grants.remove(&grant.character_id);
            }
            if grant.facet_id == self.world.facet_id {
                let facet = &self.world;
                revocations.push(
                    facet
                        .handle
                        .begin_revoke_grant(grant.connection_id, wire::DrainingReason::SessionEnded)
                        .await
                        .map_err(|_| SessionError::Unavailable)?,
                );
            }
        }
        if let Some(prepared) = &prepared_exit {
            self.publish_character_exit(prepared).await?;
        }
        for revocation in revocations {
            revocation.await.map_err(|_| SessionError::Unavailable)?;
        }
        Ok(())
    }

    pub(super) async fn reconcile_expired_sessions(&self) -> Result<(), String> {
        let session_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT session_id FROM tme.sessions WHERE revoked_at IS NULL \
             AND (idle_expires_at<=statement_timestamp() OR \
                  absolute_expires_at<=statement_timestamp()) \
             ORDER BY session_id LIMIT 64",
        )
        .fetch_all(self.store.pool())
        .await
        .map_err(|error| error.to_string())?;
        for session_id in session_ids {
            self.expire_session(session_id).await?;
        }
        Ok(())
    }

    pub(super) async fn expire_session(&self, session_id: Uuid) -> Result<(), String> {
        let _transition = self.coordinator.transition().await;
        let mut tx = serializable(self.store.pool()).await?;
        let row = sqlx::query(
            "SELECT session_id,account_id,csrf_digest,selected_character_id \
             FROM tme.sessions WHERE session_id=$1 AND revoked_at IS NULL \
             AND (idle_expires_at<=statement_timestamp() OR \
                  absolute_expires_at<=statement_timestamp()) FOR UPDATE",
        )
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        let Some(row) = row else {
            tx.rollback().await.map_err(|error| error.to_string())?;
            return Ok(());
        };
        let session = decode_session(row).map_err(|error| format!("{error:?}"))?;
        let grants = self
            .live
            .lock()
            .map_err(|_| "live grant registry is unavailable".to_string())?
            .active_grants
            .values()
            .filter(|grant| grant.session_id == session.session_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut prepared = Vec::<(FacetHandle, crate::facet::PreparedFacetCheckpoint)>::new();
        let mut exit_epoch = None;
        if let Some(character_id) = session.selected_character_id {
            let epoch = self
                .next_transfer_epoch
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| "character-exit epoch overflow".to_string())?;
            exit_epoch = Some(epoch);
            let rules_character_id = CharacterId::new(character_id.to_string());
            {
                let handle = self.world.handle.clone();
                if let Err(error) = handle
                    .prepare_character_exit(epoch, rules_character_id.clone())
                    .await
                {
                    for (prepared_handle, _) in &prepared {
                        let _ = prepared_handle.rollback_transfer(epoch).await;
                    }
                    return Err(format!(
                        "expired-session facet preparation failed: {error:?}"
                    ));
                }
                match handle.prepared_checkpoint(epoch).await {
                    Ok(checkpoint) => prepared.push((handle, checkpoint)),
                    Err(error) => {
                        let _ = handle.rollback_transfer(epoch).await;
                        for (prepared_handle, _) in &prepared {
                            let _ = prepared_handle.rollback_transfer(epoch).await;
                        }
                        return Err(format!(
                            "expired-session checkpoint preparation failed: {error:?}"
                        ));
                    }
                }
            }
            prepared.sort_by_key(|(_, checkpoint)| checkpoint.facet_id);
        }

        let durable = async {
            for (_, checkpoint) in &prepared {
                let row = sqlx::query(
                    "SELECT facet_revision,last_server_sequence FROM tme.facets \
                     WHERE facet_id=$1 FOR UPDATE",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .fetch_one(&mut *tx)
                .await
                .map_err(|error| error.to_string())?;
                if checked_u64(
                    row.try_get("facet_revision")
                        .map_err(|error| error.to_string())?,
                )? != checkpoint.before_revision
                    || checked_u64(
                        row.try_get("last_server_sequence")
                            .map_err(|error| error.to_string())?,
                    )? != checkpoint.server_sequence
                {
                    return Err("expired-session facet revision changed".to_string());
                }
                let updated = sqlx::query(
                    "UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3, \
                     facet_revision=$4,updated_at=statement_timestamp() WHERE facet_id=$1 \
                     AND facet_revision=$5 AND last_server_sequence=$6",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .bind(checkpoint.checkpoint.as_bytes())
                .bind(checkpoint.checkpoint.sha256().as_slice())
                .bind(checked_i64(checkpoint.after_revision)?)
                .bind(checked_i64(checkpoint.before_revision)?)
                .bind(checked_i64(checkpoint.server_sequence)?)
                .execute(&mut *tx)
                .await
                .map_err(|error| error.to_string())?;
                if updated.rows_affected() != 1 {
                    return Err("expired-session facet update lost its fence".to_string());
                }
            }
            sqlx::query(
                "UPDATE tme.sessions SET revoked_at=statement_timestamp() WHERE session_id=$1",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            sqlx::query(
                "UPDATE tme.player_kill_marks SET karma_forgiveness_eligible=false \
                 WHERE forgiven_at IS NULL AND expired_at IS NULL \
                 AND karma_forgiveness_eligible \
                 AND (killer_session_id=$1 OR victim_session_id=$1)",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            sqlx::query(
                "DELETE FROM tme.socket_tickets WHERE session_id=$1 AND consumed_at IS NULL",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            if let Some(character_id) = session.selected_character_id {
                sqlx::query(
                    "UPDATE tme.characters SET control_epoch=control_epoch+1 \
                     WHERE character_id=$1 AND account_id=$2",
                )
                .bind(character_id.as_uuid())
                .bind(session.account_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(|error| error.to_string())?;
            }
            audit(
                &mut tx,
                AuditEvent {
                    account_id: Some(session.account_id.as_uuid()),
                    session_id: Some(session.session_id.as_uuid()),
                    character_id: session
                        .selected_character_id
                        .map(|character_id| character_id.as_uuid()),
                    command_id: None,
                    actor: "runtime",
                    action: "session_expire",
                    result: "success",
                },
            )
            .await?;
            self.commit_gameplay_transaction(tx)
                .await
                .map_err(|error| error.to_string())
        }
        .await;
        if let Err(error) = durable {
            if let Some(epoch) = exit_epoch {
                for (handle, _) in &prepared {
                    let _ = handle.rollback_transfer(epoch).await;
                }
            }
            return Err(error);
        }
        let mut revocations = Vec::new();
        for grant in grants {
            if let Ok(mut live) = self.live.lock() {
                live.active_grants.remove(&grant.character_id);
            }
            if grant.facet_id == self.world.facet_id {
                let facet = &self.world;
                revocations.push(
                    facet
                        .handle
                        .begin_revoke_grant(grant.connection_id, wire::DrainingReason::SessionEnded)
                        .await
                        .map_err(|error| {
                            format!("expired-session grant revoke failed: {error:?}")
                        })?,
                );
            }
        }
        if let Some(epoch) = exit_epoch {
            for (handle, _) in &prepared {
                handle
                    .commit_transfer(epoch)
                    .await
                    .map_err(|error| format!("expired-session commit failed: {error:?}"))?;
            }
            for (handle, _) in &prepared {
                handle
                    .publish_transfer(epoch)
                    .await
                    .map_err(|error| format!("expired-session publish failed: {error:?}"))?;
            }
        }
        for revocation in revocations {
            revocation
                .await
                .map_err(|_| "expired-session grant revoke failed: unavailable".to_string())?;
        }
        Ok(())
    }
}
