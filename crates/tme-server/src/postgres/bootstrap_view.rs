use super::*;

impl PostgresState {
    /// Control handover marks a character in transition while its grant moves
    /// between connections. It is not world selection: there is one world.
    pub(super) fn clear_transition(&self, character_id: wire::CharacterId) {
        if let Ok(mut live) = self.live.lock() {
            live.transitioning.remove(&character_id);
        }
    }

    pub(super) async fn bootstrap_for(
        &self,
        session_id: wire::SessionId,
        account_id: wire::AccountId,
        csrf_token: wire::CsrfToken,
        selected_character_id: Option<wire::CharacterId>,
    ) -> Result<wire::SessionBootstrapV1, SessionError> {
        self.store
            .reconcile_player_kill_marks(account_id.as_uuid())
            .await
            .map_err(unavailable)?;
        let row = sqlx::query(
            "SELECT display_name FROM tme.accounts WHERE account_id=$1 AND status='active'",
        )
        .bind(account_id.as_uuid())
        .fetch_optional(self.store.pool())
        .await
        .map_err(unavailable)?
        .ok_or(SessionError::Unavailable)?;
        let display_name = wire::DisplayName::new(
            row.try_get::<String, _>("display_name")
                .map_err(unavailable)?,
        )
        .map_err(|_| SessionError::Unavailable)?;
        let rows = sqlx::query("SELECT character_id,account_id,slot,display_name,actor_id,control_epoch FROM tme.characters WHERE account_id=$1 ORDER BY slot,character_id")
            .bind(account_id.as_uuid()).fetch_all(self.store.pool()).await.map_err(unavailable)?;
        let characters = rows
            .into_iter()
            .map(decode_character)
            .collect::<Result<Vec<_>, _>>()?;
        let mark_rows = sqlx::query(
            "SELECT m.mark_id,m.victim_character_id,c.display_name AS victim_display_name, \
                    to_char(m.assessed_at AT TIME ZONE 'UTC', \
                            'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS assessed_at, \
                    CASE WHEN m.expires_at IS NULL THEN NULL ELSE \
                         to_char(m.expires_at AT TIME ZONE 'UTC', \
                                 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') END AS expires_at \
             FROM tme.player_kill_marks m \
             JOIN tme.characters c ON c.character_id=m.victim_character_id \
             WHERE m.killer_account_id=$1 AND m.forgiven_at IS NULL AND m.expired_at IS NULL \
             ORDER BY m.assessed_at,m.mark_id",
        )
        .bind(account_id.as_uuid())
        .fetch_all(self.store.pool())
        .await
        .map_err(unavailable)?;
        let active_marks = mark_rows
            .into_iter()
            .map(|row| {
                Ok(wire::PlayerKillMarkSummaryV1 {
                    mark_id: wire::PlayerKillMarkId::new(
                        row.try_get("mark_id").map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    victim_character_id: wire::CharacterId::new(
                        row.try_get("victim_character_id").map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    victim_display_name: wire::DisplayName::new(
                        row.try_get::<String, _>("victim_display_name")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    assessed_at: wire::WireLabel::new(
                        row.try_get::<String, _>("assessed_at")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    expires_at: row
                        .try_get::<Option<String>, _>("expires_at")
                        .map_err(unavailable)?
                        .map(wire::WireLabel::new)
                        .transpose()
                        .map_err(|_| SessionError::Unavailable)?,
                })
            })
            .collect::<Result<Vec<_>, SessionError>>()?;
        let forgivable_rows = sqlx::query(
            "SELECT m.mark_id,m.killer_account_id,m.killer_character_id,c.display_name AS killer_display_name, \
                    to_char(m.assessed_at AT TIME ZONE 'UTC', \
                            'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS assessed_at, \
                    (SELECT count(*) FROM tme.player_kill_marks active \
                     WHERE active.killer_account_id=m.killer_account_id \
                     AND active.forgiven_at IS NULL AND active.expired_at IS NULL) AS killer_active_count \
             FROM tme.player_kill_marks m \
             JOIN tme.characters c ON c.character_id=m.killer_character_id \
             WHERE m.victim_account_id=$1 AND m.forgiven_at IS NULL AND m.expired_at IS NULL \
             ORDER BY m.assessed_at,m.mark_id",
        )
        .bind(account_id.as_uuid())
        .fetch_all(self.store.pool())
        .await
        .map_err(unavailable)?;
        let mut forgivable_marks = Vec::new();
        for row in forgivable_rows {
            let killer_account_id: Uuid = row.try_get("killer_account_id").map_err(unavailable)?;
            let killer_character_id =
                wire::CharacterId::new(row.try_get("killer_character_id").map_err(unavailable)?)
                    .map_err(|_| SessionError::Unavailable)?;
            let same_facet = self.live.lock().ok().is_some_and(|live| {
                let killer = live.active_grants.get(&killer_character_id);
                let victim = live
                    .active_grants
                    .values()
                    .find(|grant| grant.account_id == account_id);
                killer.zip(victim).is_some_and(|(killer, victim)| {
                    killer.facet_id == victim.facet_id
                        && killer.account_id.as_uuid() == killer_account_id
                })
            });
            let killer_locked = row
                .try_get::<i64, _>("killer_active_count")
                .map_err(unavailable)?
                >= 4;
            let lobby_eligible = if killer_locked {
                let no_gameplay = self.live.lock().ok().is_some_and(|live| {
                    !live.active_grants.values().any(|grant| {
                        grant.account_id == account_id
                            || grant.account_id.as_uuid() == killer_account_id
                    })
                });
                let killer_session: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE account_id=$1 \
                     AND revoked_at IS NULL AND idle_expires_at>statement_timestamp() \
                     AND absolute_expires_at>statement_timestamp())",
                )
                .bind(killer_account_id)
                .fetch_one(self.store.pool())
                .await
                .map_err(unavailable)?;
                no_gameplay && killer_session
            } else {
                false
            };
            if same_facet || lobby_eligible {
                forgivable_marks.push(wire::ForgivablePlayerKillMarkV1 {
                    mark_id: wire::PlayerKillMarkId::new(
                        row.try_get("mark_id").map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    killer_character_id,
                    killer_display_name: wire::DisplayName::new(
                        row.try_get::<String, _>("killer_display_name")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    assessed_at: wire::WireLabel::new(
                        row.try_get::<String, _>("assessed_at")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                });
            }
        }
        let active_count = u32::try_from(active_marks.len()).map_err(unavailable)?;
        Ok(wire::SessionBootstrapV1 {
            control_api_version: wire::CONTROL_API_VERSION,
            account: wire::AccountSummaryV1 {
                account_id,
                display_name,
            },
            session: wire::SessionSummaryV1 {
                session_id,
                idle_timeout_seconds: wire::DecimalU64::new(SESSION_IDLE.as_secs()),
                absolute_timeout_seconds: wire::DecimalU64::new(SESSION_ABSOLUTE.as_secs()),
            },
            csrf_token,
            characters: characters.iter().map(character_summary).collect(),
            selected_character_id,
            player_kill_marks: wire::PlayerKillMarkStateV1 {
                active_count,
                gameplay_locked: active_count >= 4,
                active_marks,
                forgivable_marks,
            },
        })
    }
}
