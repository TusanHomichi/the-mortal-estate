use super::*;

impl PostgresState {
    pub async fn logout(
        &self,
        session_token: &str,
        request: wire::LogoutRequestV1,
    ) -> Result<(), SessionError> {
        let _transition = self.coordinator.transition().await;
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, session_token, false)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        // Authentication must not pin a snapshot across a queued world mutation.
        // Prepare freezes the world; only then open the durable transaction.
        tx.rollback().await.map_err(unavailable)?;
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
            let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
            Self::revalidate_exit_session(
                &mut tx,
                session_token,
                &session,
                &request.csrf_token,
                false,
            )
            .await?;
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
        tx.rollback().await.map_err(|error| error.to_string())?;
        let prepared = match session.selected_character_id {
            Some(character_id) => Some(
                self.prepare_character_exit_candidate(character_id)
                    .await
                    .map_err(|error| format!("expired-session preparation failed: {error:?}"))?,
            ),
            None => None,
        };
        let durable = async {
            let mut tx = serializable(self.store.pool()).await?;
            let still_expired: bool = sqlx::query_scalar(
                "SELECT revoked_at IS NULL AND \
                 (idle_expires_at<=statement_timestamp() OR absolute_expires_at<=statement_timestamp()) \
                 FROM tme.sessions WHERE session_id=$1 FOR UPDATE",
            ).bind(session.session_id.as_uuid()).fetch_one(&mut *tx).await
                .map_err(|error| error.to_string())?;
            if !still_expired {
                return Err("expired session changed during world preparation".to_string());
            }
            if let Some(prepared) = &prepared {
                Self::persist_prepared_facets(&mut tx, &prepared.facets).await
                    .map_err(|error| format!("expired-session persistence failed: {error:?}"))?;
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
            if let Some(prepared) = &prepared {
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
                        .map_err(|error| {
                            format!("expired-session grant revoke failed: {error:?}")
                        })?,
                );
            }
        }
        if let Some(prepared) = &prepared {
            self.publish_character_exit(prepared)
                .await
                .map_err(|error| format!("expired-session publication failed: {error:?}"))?;
        }
        for revocation in revocations {
            revocation
                .await
                .map_err(|_| "expired-session grant revoke failed: unavailable".to_string())?;
        }
        Ok(())
    }
}
