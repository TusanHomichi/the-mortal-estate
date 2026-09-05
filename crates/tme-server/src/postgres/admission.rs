use super::*;

impl PostgresState {
    pub async fn admit(
        &self,
        ticket: &wire::AdmissionTicket,
        supported_minors: &[u16],
        origin: &str,
        host: &str,
        outbound: mpsc::Sender<wire::ServerEnvelope>,
        terminal: watch::Sender<Option<wire::DrainingReason>>,
    ) -> Result<(AdmissionGrant, FacetWelcome), AdmissionError> {
        let _transition = self.coordinator.transition().await;
        if !self.gameplay_ready() {
            return Err(AdmissionError::Unavailable);
        }
        let mut tx = serializable(self.store.pool())
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        let row = sqlx::query(
            "SELECT t.session_id,t.account_id,t.character_id,t.actor_id, \
                    t.expected_control_epoch,t.origin,t.host, \
                    t.expires_at <= statement_timestamp() AS expired,t.consumed_at IS NOT NULL AS consumed \
             FROM tme.socket_tickets t WHERE ticket_digest=$1 FOR UPDATE",
        )
        .bind(digest(ticket.expose_for_admission()).as_slice())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AdmissionError::Unavailable)?
        .ok_or(AdmissionError::InvalidTicket)?;
        if row
            .try_get::<bool, _>("consumed")
            .map_err(|_| AdmissionError::Unavailable)?
        {
            return Err(AdmissionError::ConsumedTicket);
        }
        sqlx::query("UPDATE tme.socket_tickets SET consumed_at=statement_timestamp() WHERE ticket_digest=$1")
            .bind(digest(ticket.expose_for_admission()).as_slice())
            .execute(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        if !supported_minors.contains(&wire::PROTOCOL_MINOR) {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::UnsupportedVersion);
        }
        if row
            .try_get::<bool, _>("expired")
            .map_err(|_| AdmissionError::Unavailable)?
        {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::ExpiredTicket);
        }
        if row
            .try_get::<String, _>("origin")
            .map_err(|_| AdmissionError::Unavailable)?
            != origin
        {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::OriginRejected);
        }
        if row
            .try_get::<String, _>("host")
            .map_err(|_| AdmissionError::Unavailable)?
            != host
        {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::HostRejected);
        }
        let session_id = wire::SessionId::new(
            row.try_get("session_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        let account_id = wire::AccountId::new(
            row.try_get("account_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        let character_id = wire::CharacterId::new(
            row.try_get("character_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        // D4: one world. The admitted grant binds to the world this process
        // hosts; the ticket never named one.
        let facet_id = self.world.facet_id;
        let actor_id = ActorId::new(
            row.try_get::<String, _>("actor_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        );
        let expected_epoch = checked_u64(
            row.try_get("expected_control_epoch")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        let session_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE session_id=$1 AND account_id=$2 \
             AND selected_character_id=$3 AND revoked_at IS NULL \
             AND idle_expires_at>statement_timestamp() AND absolute_expires_at>statement_timestamp())",
        )
        .bind(session_id.as_uuid())
        .bind(account_id.as_uuid())
        .bind(character_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| AdmissionError::Unavailable)?;
        if !session_ok {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::Unavailable);
        }
        sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
            .bind(account_id.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        crate::store::reschedule_player_kill_marks_raw(&mut tx, account_id.as_uuid(), false)
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        let active_marks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
             AND forgiven_at IS NULL AND expired_at IS NULL",
        )
        .bind(account_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| AdmissionError::Unavailable)?;
        if active_marks >= 4 {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::GameplayMarkLocked);
        }
        // Ticket consumption and static request validation finish before the
        // facet is quiesced. The control transaction begins afterward so a
        // scheduler commit cannot invalidate an old serializable snapshot.
        tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
        let next_epoch = expected_epoch
            .checked_add(1)
            .ok_or(AdmissionError::Unavailable)?;
        if facet_id != self.world.facet_id {
            return Err(AdmissionError::Unavailable);
        }
        let facet = self.world.handle.clone();
        {
            let mut live = self.live.lock().map_err(|_| AdmissionError::Unavailable)?;
            if !live.transitioning.insert(character_id) {
                return Err(AdmissionError::Unavailable);
            }
        }
        if facet.prepare_control(character_id).await.is_err() {
            self.clear_transition(character_id);
            return Err(AdmissionError::Unavailable);
        }
        let grant = ControlGrant::new(
            account_id,
            session_id,
            wire::ConnectionId::new(Uuid::now_v7()).map_err(|_| AdmissionError::Unavailable)?,
            character_id,
            facet_id,
            actor_id.clone(),
            next_epoch,
        );
        // Owner ruling 2026-08-20 (#3): a killer who logged off before a delayed
        // kill landed still owes the karma. Pay it here, before they see the
        // world, and clear it in the same transaction that makes the applied
        // sheet durable. The candidate is prepared BEFORE the control
        // transaction opens, for the same reason forgiveness does it: the facet
        // task must never be waiting on a SQL row this transaction holds.
        let pending = match crate::store::pending_kill_consequences(
            self.store.pool(),
            account_id.as_uuid(),
            character_id.as_uuid(),
        )
        .await
        {
            Ok(pending) => pending,
            Err(_) => {
                self.clear_transition(character_id);
                let _ = facet.resume_control(character_id).await;
                return Err(AdmissionError::Unavailable);
            }
        };
        let prepared_pending = if pending.is_empty() {
            None
        } else {
            match self.prepare_pending_consequences(&facet, &pending).await {
                Ok(prepared) => Some(prepared),
                Err(_) => {
                    self.clear_transition(character_id);
                    let _ = facet.resume_control(character_id).await;
                    return Err(AdmissionError::Unavailable);
                }
            }
        };
        let mut control_tx = match serializable(self.store.pool()).await {
            Ok(tx) => tx,
            Err(_) => {
                if let Some((epoch, _, _)) = &prepared_pending {
                    let _ = facet.rollback_transfer(*epoch).await;
                }
                self.clear_transition(character_id);
                let _ = facet.resume_control(character_id).await;
                return Err(AdmissionError::Unavailable);
            }
        };
        let durable = async {
            sqlx::query("SELECT facet_id FROM tme.facets WHERE facet_id=$1 FOR UPDATE")
                .bind(facet_id.as_uuid())
                .fetch_one(&mut *control_tx)
                .await
                .map_err(|error| error.to_string())?;
            sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
                .bind(account_id.as_uuid())
                .fetch_one(&mut *control_tx)
                .await
                .map_err(|error| error.to_string())?;
            if let Some((_, checkpoint, linked)) = &prepared_pending {
                Self::persist_prepared_checkpoint(&mut control_tx, checkpoint).await.map_err(|_| "admission checkpoint commit failed".to_string())?;
                // Clearing rides the same transaction as the checkpoint above.
                // Crash before it commits and the rows survive to be applied at
                // the next admission; crash after and they are gone for good.
                for (owed, linked_karma_added) in pending.iter().zip(linked) {
                    crate::store::clear_pending_kill_consequence_raw(
                        &mut control_tx,
                        owed.facet_kill_sequence,
                        *linked_karma_added,
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                }
            }
            crate::store::reschedule_player_kill_marks_raw(
                &mut control_tx,
                account_id.as_uuid(),
                false,
            )
            .await
            .map_err(|error| error.to_string())?;
            let active_marks: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL",
            )
            .bind(account_id.as_uuid())
            .fetch_one(&mut *control_tx)
            .await
            .map_err(|error| error.to_string())?;
            if active_marks >= 4 {
                return Err("account became gameplay-mark locked".to_string());
            }
            let session_ok: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE session_id=$1 \
                 AND account_id=$2 AND selected_character_id=$3 AND revoked_at IS NULL \
                 AND idle_expires_at>statement_timestamp() \
                 AND absolute_expires_at>statement_timestamp())",
            )
            .bind(session_id.as_uuid())
            .bind(account_id.as_uuid())
            .bind(character_id.as_uuid())
            .fetch_one(&mut *control_tx)
            .await
            .map_err(|error| error.to_string())?;
            if !session_ok {
                return Err("session authority changed during admission".to_string());
            }
            let updated = sqlx::query(
                "UPDATE tme.characters SET control_epoch=$2 WHERE character_id=$1 \
                 AND account_id=$3 AND actor_id=$4 AND control_epoch=$5",
            )
            .bind(character_id.as_uuid())
            .bind(checked_i64(next_epoch)?)
            .bind(account_id.as_uuid())
            .bind(actor_id.as_str())
            .bind(checked_i64(expected_epoch)?)
            .execute(&mut *control_tx)
            .await
            .map_err(|error| error.to_string())?;
            if updated.rows_affected() != 1 {
                return Err("character control epoch changed".to_string());
            }
            sqlx::query("UPDATE tme.sessions SET last_seen_at=statement_timestamp(), idle_expires_at=LEAST(absolute_expires_at,statement_timestamp()+make_interval(secs=>$2)) WHERE session_id=$1")
                .bind(session_id.as_uuid())
                .bind(checked_i64(SESSION_IDLE.as_secs())?)
                .execute(&mut *control_tx)
                .await
                .map_err(|error| error.to_string())?;
            audit(
                &mut control_tx,
                AuditEvent {
                    account_id: Some(account_id.as_uuid()),
                    session_id: Some(session_id.as_uuid()),
                    character_id: Some(character_id.as_uuid()),
                    command_id: None,
                    actor: "runtime",
                    action: "admit",
                    result: "success",
                },
            )
            .await?;
            self.commit_gameplay_transaction(control_tx)
                .await
                .map_err(|error| error.to_string())
        }
        .await;
        if durable.is_err() {
            if let Some((epoch, _, _)) = &prepared_pending {
                let _ = facet.rollback_transfer(*epoch).await;
            }
            self.clear_transition(character_id);
            let _ = facet.resume_control(character_id).await;
            return Err(AdmissionError::Unavailable);
        }
        if let Some((epoch, _, _)) = &prepared_pending
            && (facet.commit_transfer(*epoch).await.is_err()
                || facet.publish_transfer(*epoch).await.is_err())
        {
            self.ready.fail();
            return Err(AdmissionError::Unavailable);
        }
        let welcome = match facet.install_grant(grant.clone(), outbound, terminal).await {
            Ok(value) => value,
            Err(_) => {
                self.ready.fail();
                return Err(AdmissionError::Unavailable);
            }
        };
        {
            let mut live = self.live.lock().map_err(|_| AdmissionError::Unavailable)?;
            live.transitioning.remove(&character_id);
            live.active_grants.insert(character_id, grant.clone());
        }
        Ok((
            AdmissionGrant {
                control: grant,
                facet,
            },
            welcome,
        ))
    }

    pub async fn authorize_grant(&self, grant: &ControlGrant) -> bool {
        if !self.gameplay_ready()
            || !self.live.lock().ok().is_some_and(|live| {
                !live.transitioning.contains(&grant.character_id)
                    && live.active_grants.get(&grant.character_id) == Some(grant)
            })
        {
            return false;
        }
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tme.sessions s JOIN tme.characters c \
             ON c.character_id=$3 WHERE s.session_id=$1 AND s.account_id=$2 \
             AND s.revoked_at IS NULL AND s.idle_expires_at>statement_timestamp() \
             AND s.absolute_expires_at>statement_timestamp() AND c.control_epoch=$4 \
             AND c.account_id=$2 AND c.actor_id=$5)",
        )
        .bind(grant.session_id.as_uuid())
        .bind(grant.account_id.as_uuid())
        .bind(grant.character_id.as_uuid())
        .bind(checked_i64(grant.control_epoch).unwrap_or(-1))
        .bind(grant.actor_id.as_str())
        .fetch_one(self.store.pool())
        .await
        .unwrap_or(false)
    }

    pub(crate) async fn deliver_page(
        &self,
        target_character_id: wire::CharacterId,
        message_id: wire::MessageId,
        sender_character_id: wire::CharacterId,
        sender_name: wire::DisplayName,
        body: wire::SocialBody,
    ) -> bool {
        if !self.gameplay_ready() {
            return false;
        }
        let target = self.live.lock().ok().and_then(|live| {
            (!live.transitioning.contains(&target_character_id))
                .then(|| live.active_grants.get(&target_character_id).cloned())
                .flatten()
        });
        let Some(target) = target else {
            return false;
        };
        if target.facet_id != self.world.facet_id {
            return false;
        }
        let facet = self.world.handle.clone();
        facet
            .deliver_page(target, message_id, sender_character_id, sender_name, body)
            .await
            .unwrap_or(false)
    }
}
