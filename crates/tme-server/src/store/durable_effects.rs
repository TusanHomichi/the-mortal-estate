use super::*;

pub(super) async fn persist_durable_effects(
    tx: &mut Transaction<'_, Postgres>,
    issuer: Option<(Uuid, Uuid, Uuid)>,
    effects: &[DurableGameplayEffectV1],
) -> Result<(), String> {
    persist_durable_effects_raw(tx, issuer, effects)
        .await
        .map_err(store_error)
}

pub(super) async fn persist_durable_effects_raw(
    tx: &mut Transaction<'_, Postgres>,
    issuer: Option<(Uuid, Uuid, Uuid)>,
    effects: &[DurableGameplayEffectV1],
) -> Result<(), sqlx::Error> {
    let mut reschedule_accounts = BTreeSet::new();
    for effect in effects {
        let DurableGameplayEffectV1::PlayerKillAssessed(assessment) = effect;
        let killer_character_id = Uuid::parse_str(assessment.killer_character_id.as_str())
            .map_err(|_| protocol_store_error("player-kill killer character is not a UUID"))?;
        let victim_character_id = Uuid::parse_str(assessment.victim_character_id.as_str())
            .map_err(|_| protocol_store_error("player-kill victim character is not a UUID"))?;

        let character_rows = sqlx::query(
            "SELECT character_id,account_id FROM tme.characters \
             WHERE character_id = ANY($1) ORDER BY character_id",
        )
        .bind(vec![killer_character_id, victim_character_id])
        .fetch_all(&mut **tx)
        .await?;
        if character_rows.len() != 2 {
            return Err(protocol_store_error(
                "player-kill assessment references missing durable character authority",
            ));
        }
        let mut killer_account_id = None;
        let mut victim_account_id = None;
        for row in character_rows {
            let character_id: Uuid = row.try_get("character_id")?;
            let account_id: Uuid = row.try_get("account_id")?;
            if character_id == killer_character_id {
                killer_account_id = Some(account_id);
            }
            if character_id == victim_character_id {
                victim_account_id = Some(account_id);
            }
        }
        let killer_account_id = killer_account_id
            .ok_or_else(|| protocol_store_error("player-kill killer mapping disappeared"))?;
        let victim_account_id = victim_account_id
            .ok_or_else(|| protocol_store_error("player-kill victim mapping disappeared"))?;
        let mut account_ids = vec![killer_account_id, victim_account_id];
        account_ids.sort_unstable();
        account_ids.dedup();
        let locked_accounts: Vec<Uuid> = sqlx::query_scalar(
            "SELECT account_id FROM tme.accounts WHERE account_id = ANY($1) \
             ORDER BY account_id FOR UPDATE",
        )
        .bind(account_ids.clone())
        .fetch_all(&mut **tx)
        .await?;
        if locked_accounts != account_ids {
            return Err(protocol_store_error(
                "player-kill account authority disappeared",
            ));
        }

        let victim_sessions: Vec<Uuid> = sqlx::query_scalar(
            "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
             AND selected_character_id=$2 AND revoked_at IS NULL \
             AND idle_expires_at>statement_timestamp() \
             AND absolute_expires_at>statement_timestamp() \
             ORDER BY session_id FOR UPDATE",
        )
        .bind(victim_account_id)
        .bind(victim_character_id)
        .fetch_all(&mut **tx)
        .await?;
        if victim_sessions.len() != 1 {
            return Err(protocol_store_error(
                "player-kill victim does not have one exact live session authority",
            ));
        }
        let victim_session_id = victim_sessions[0];
        let issuer_session_id = issuer.and_then(|(account_id, session_id, character_id)| {
            (account_id == killer_account_id && character_id == killer_character_id)
                .then_some(session_id)
        });
        let killer_sessions: Vec<Uuid> = sqlx::query_scalar(
            "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
             AND selected_character_id=$2 AND revoked_at IS NULL \
             AND idle_expires_at>statement_timestamp() \
             AND absolute_expires_at>statement_timestamp() \
             ORDER BY session_id FOR UPDATE",
        )
        .bind(killer_account_id)
        .bind(killer_character_id)
        .fetch_all(&mut **tx)
        .await?;
        if killer_sessions.len() > 1 {
            return Err(protocol_store_error(
                "player-kill killer has ambiguous live session authority",
            ));
        }
        if issuer_session_id.is_some_and(|session_id| !killer_sessions.contains(&session_id)) {
            return Err(protocol_store_error(
                "player-kill issuer session authority changed",
            ));
        }
        let killer_session_id = issuer_session_id.or_else(|| killer_sessions.first().copied());

        audit_raw(
            tx,
            AuditEvent {
                account_id: Some(killer_account_id),
                session_id: killer_session_id,
                character_id: Some(killer_character_id),
                command_id: None,
                actor: "runtime",
                action: "mark_assess",
                result: "success",
            },
        )
        .await?;
        if assessment.exempt_self_defense {
            continue;
        }
        // Owner ruling 2026-08-20 (#3): an absent killer's karma is deferred, not
        // waived. Nothing has been added to their sheet yet, so the mark records
        // false and the pending row below carries the consequence. Whichever
        // admission applies it updates this column to the value the rules
        // actually produced, in that same transaction.
        let absent_killer = matches!(
            assessment.consequence,
            tme_rules::PlayerKillConsequenceV1::RequiresAbsentKiller { .. }
        );
        let linked_karma_added = match assessment.consequence {
            tme_rules::PlayerKillConsequenceV1::AppliedHere { linked_karma_added } => {
                linked_karma_added
            }
            tme_rules::PlayerKillConsequenceV1::RequiresAbsentKiller { .. } => false,
        };
        let facet_kill_sequence = i64::try_from(assessment.facet_kill_sequence)
            .map_err(|_| protocol_store_error("player-kill sequence exceeds bigint"))?;
        // D4: one world, so the kill sequence alone identifies the kill. Leaving
        // the world id out also keeps a mark's identity stable when a checkpoint
        // is restored into a differently-identified database.
        let name = format!(
            "https://tme.invalid/ids/player-kill-mark/v1/{}",
            assessment.facet_kill_sequence
        );
        let mark_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes());
        let logical_time = assessment.logical_time.as_millis().to_string();
        let karma_forgiveness_eligible = linked_karma_added && killer_session_id.is_some();
        let inserted = sqlx::query(
            "INSERT INTO tme.player_kill_marks \
             (mark_id,facet_kill_sequence,killer_account_id,killer_character_id, \
              victim_account_id,victim_character_id,killer_session_id,victim_session_id, \
              assessed_at,assessed_logical_millis,linked_karma_added, \
              karma_forgiveness_eligible,expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,tme.mark_now(), \
                     CAST($9 AS numeric),$10,$11,NULL) \
             ON CONFLICT (facet_kill_sequence) DO NOTHING",
        )
        .bind(mark_id)
        .bind(facet_kill_sequence)
        .bind(killer_account_id)
        .bind(killer_character_id)
        .bind(victim_account_id)
        .bind(victim_character_id)
        .bind(killer_session_id)
        .bind(victim_session_id)
        .bind(&logical_time)
        .bind(linked_karma_added)
        .bind(karma_forgiveness_eligible)
        .execute(&mut **tx)
        .await?;
        if inserted.rows_affected() == 0 {
            let agrees: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tme.player_kill_marks WHERE \
                 facet_kill_sequence=$1 AND mark_id=$2 \
                 AND killer_account_id=$3 AND killer_character_id=$4 \
                 AND victim_account_id=$5 AND victim_character_id=$6 \
                 AND killer_session_id IS NOT DISTINCT FROM $7 \
                 AND victim_session_id=$8 AND assessed_logical_millis=CAST($9 AS numeric) \
                 AND linked_karma_added=$10 AND karma_forgiveness_eligible=$11)",
            )
            .bind(facet_kill_sequence)
            .bind(mark_id)
            .bind(killer_account_id)
            .bind(killer_character_id)
            .bind(victim_account_id)
            .bind(victim_character_id)
            .bind(killer_session_id)
            .bind(victim_session_id)
            .bind(&logical_time)
            .bind(linked_karma_added)
            .bind(karma_forgiveness_eligible)
            .fetch_one(&mut **tx)
            .await?;
            if !agrees {
                return Err(protocol_store_error(
                    "player-kill sequence conflicts with different durable facts",
                ));
            }
        }
        if let tme_rules::PlayerKillConsequenceV1::RequiresAbsentKiller {
            victim_alignment,
            victim_nature,
        } = assessment.consequence
        {
            debug_assert!(absent_killer);
            // Same transaction as the mark, by construction: there is no point
            // at which the mark exists without the consequence it defers.
            sqlx::query(
                "INSERT INTO tme.pending_player_kill_consequences \
                 (facet_kill_sequence,killer_account_id,killer_character_id, \
                  victim_character_id,victim_alignment,victim_nature, \
                  assessed_logical_millis) \
                 VALUES ($1,$2,$3,$4,$5,$6,CAST($7 AS numeric)) \
                 ON CONFLICT (facet_kill_sequence) DO NOTHING",
            )
            .bind(facet_kill_sequence)
            .bind(killer_account_id)
            .bind(killer_character_id)
            .bind(victim_character_id)
            .bind(alignment_label(victim_alignment))
            .bind(nature_label(victim_nature))
            .bind(&logical_time)
            .execute(&mut **tx)
            .await?;
        }
        reschedule_accounts.insert(killer_account_id);
    }
    for account_id in reschedule_accounts {
        reschedule_player_kill_marks_raw(tx, account_id, true).await?;
    }
    Ok(())
}

pub(super) fn alignment_label(value: tme_rules::CharacterAlignment) -> &'static str {
    match value {
        tme_rules::CharacterAlignment::Lawful => "lawful",
        tme_rules::CharacterAlignment::Neutral => "neutral",
        tme_rules::CharacterAlignment::Chaotic => "chaotic",
        tme_rules::CharacterAlignment::Evil => "evil",
    }
}

pub(super) fn nature_label(value: tme_rules::SocialNature) -> &'static str {
    match value {
        tme_rules::SocialNature::Human => "human",
        tme_rules::SocialNature::Animal => "animal",
        tme_rules::SocialNature::Other => "other",
    }
}

pub(crate) fn alignment_from_label(value: &str) -> Option<tme_rules::CharacterAlignment> {
    Some(match value {
        "lawful" => tme_rules::CharacterAlignment::Lawful,
        "neutral" => tme_rules::CharacterAlignment::Neutral,
        "chaotic" => tme_rules::CharacterAlignment::Chaotic,
        "evil" => tme_rules::CharacterAlignment::Evil,
        _ => return None,
    })
}

pub(crate) fn nature_from_label(value: &str) -> Option<tme_rules::SocialNature> {
    Some(match value {
        "human" => tme_rules::SocialNature::Human,
        "animal" => tme_rules::SocialNature::Animal,
        "other" => tme_rules::SocialNature::Other,
        _ => return None,
    })
}

/// One deferred karma/alignment consequence owed to a killer who was absent
/// when the kill landed (owner ruling 2026-08-20, successor issue #3).
pub(crate) struct PendingKillConsequence {
    pub facet_kill_sequence: i64,
    pub assessment: tme_rules::PlayerKillAssessmentV1,
}

/// Reads what this character owes, locking the rows so the transaction that
/// applies them is the only one that can clear them.
pub(crate) async fn pending_kill_consequences(
    pool: &PgPool,
    killer_account_id: Uuid,
    killer_character_id: Uuid,
) -> Result<Vec<PendingKillConsequence>, sqlx::Error> {
    let rows = sqlx::query(PENDING_KILL_CONSEQUENCE_SELECT)
        .bind(killer_account_id)
        .bind(killer_character_id)
        .fetch_all(pool)
        .await?;
    decode_pending_kill_consequences(rows, killer_character_id)
}

const PENDING_KILL_CONSEQUENCE_SELECT: &str = "SELECT facet_kill_sequence,victim_character_id,victim_alignment,victim_nature, \
            assessed_logical_millis::text AS assessed_logical_millis \
     FROM tme.pending_player_kill_consequences \
     WHERE killer_account_id=$1 AND killer_character_id=$2 \
     ORDER BY facet_kill_sequence";

pub(super) fn decode_pending_kill_consequences(
    rows: Vec<sqlx::postgres::PgRow>,
    killer_character_id: Uuid,
) -> Result<Vec<PendingKillConsequence>, sqlx::Error> {
    let mut pending = Vec::with_capacity(rows.len());
    for row in rows {
        let facet_kill_sequence: i64 = row.try_get("facet_kill_sequence")?;
        let sequence = u64::try_from(facet_kill_sequence)
            .map_err(|_| protocol_store_error("pending kill sequence is out of range"))?;
        let victim_character_id: Uuid = row.try_get("victim_character_id")?;
        let alignment: String = row.try_get("victim_alignment")?;
        let nature: String = row.try_get("victim_nature")?;
        let logical_time: String = row.try_get("assessed_logical_millis")?;
        let victim_alignment = alignment_from_label(&alignment)
            .ok_or_else(|| protocol_store_error("pending consequence alignment is unknown"))?;
        let victim_nature = nature_from_label(&nature)
            .ok_or_else(|| protocol_store_error("pending consequence nature is unknown"))?;
        let logical_time = logical_time
            .parse::<u64>()
            .map_err(|_| protocol_store_error("pending consequence logical time is invalid"))?;
        pending.push(PendingKillConsequence {
            facet_kill_sequence,
            assessment: tme_rules::PlayerKillAssessmentV1 {
                facet_kill_sequence: sequence,
                killer_character_id: tme_rules::CharacterId::new(killer_character_id.to_string()),
                victim_character_id: tme_rules::CharacterId::new(victim_character_id.to_string()),
                exempt_self_defense: false,
                consequence: tme_rules::PlayerKillConsequenceV1::RequiresAbsentKiller {
                    victim_alignment,
                    victim_nature,
                },
                logical_time: tme_rules::LogicalTime::from_millis(logical_time),
            },
        });
    }
    Ok(pending)
}

/// Clears one applied consequence and corrects the mark it deferred. This must
/// run in the same transaction that makes the applied sheet durable — that is
/// the whole of the exactly-once guarantee.
pub(crate) async fn clear_pending_kill_consequence_raw(
    tx: &mut Transaction<'_, Postgres>,
    facet_kill_sequence: i64,
    linked_karma_added: bool,
) -> Result<(), sqlx::Error> {
    let deleted = sqlx::query(
        "DELETE FROM tme.pending_player_kill_consequences WHERE facet_kill_sequence=$1",
    )
    .bind(facet_kill_sequence)
    .execute(&mut **tx)
    .await?;
    if deleted.rows_affected() != 1 {
        return Err(protocol_store_error(
            "pending player-kill consequence vanished before it was cleared",
        ));
    }
    // The mark recorded false when the kill landed because nothing had been
    // added yet. Now it has, so the durable record stops understating it.
    //
    // Forgiveness follows the karma, not the killer's session at kill time
    // (owner ruling 2026-08-20: "you should be able to forgive at any time
    // after"). A present killer always holds a live session when their kill is
    // assessed, so for them eligibility has always been exactly
    // `linked_karma_added`. Setting it the same way here is what makes a
    // returned absent killer indistinguishable from a present one, from the
    // victim's side.
    sqlx::query(
        "UPDATE tme.player_kill_marks \
         SET linked_karma_added=$2,karma_forgiveness_eligible=$2 \
         WHERE facet_kill_sequence=$1 AND forgiven_at IS NULL AND expired_at IS NULL",
    )
    .bind(facet_kill_sequence)
    .bind(linked_karma_added)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn reschedule_player_kill_marks_raw(
    tx: &mut Transaction<'_, Postgres>,
    killer_account_id: Uuid,
    force_reanchor: bool,
) -> Result<(), sqlx::Error> {
    let expired = sqlx::query(
        "UPDATE tme.player_kill_marks SET expired_at=tme.mark_now(),expires_at=NULL \
         WHERE killer_account_id=$1 AND forgiven_at IS NULL AND expired_at IS NULL \
         AND expires_at IS NOT NULL AND expires_at<=tme.mark_now()",
    )
    .bind(killer_account_id)
    .execute(&mut **tx)
    .await?;
    if expired.rows_affected() > 0 {
        audit_raw(
            tx,
            AuditEvent {
                account_id: Some(killer_account_id),
                session_id: None,
                character_id: None,
                command_id: None,
                actor: "runtime",
                action: "mark_expire",
                result: "success",
            },
        )
        .await?;
    }
    if expired.rows_affected() == 0 && !force_reanchor {
        return Ok(());
    }
    let mark_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT mark_id FROM tme.player_kill_marks WHERE killer_account_id=$1 \
         AND forgiven_at IS NULL AND expired_at IS NULL \
         ORDER BY assessed_at,mark_id FOR UPDATE",
    )
    .bind(killer_account_id)
    .fetch_all(&mut **tx)
    .await?;
    if mark_ids.len() >= 4 {
        sqlx::query("UPDATE tme.player_kill_marks SET expires_at=NULL WHERE mark_id = ANY($1)")
            .bind(mark_ids)
            .execute(&mut **tx)
            .await?;
        return Ok(());
    }
    if !mark_ids.is_empty() {
        let count = i64::try_from(mark_ids.len())
            .map_err(|_| protocol_store_error("player-kill schedule overflow"))?;
        sqlx::query(
            "UPDATE tme.player_kill_marks AS marks \
             SET expires_at=tme.mark_now()+ \
                 (((($2-schedule.position::bigint)+1)*2)*interval '1 week') \
             FROM unnest($1::uuid[]) WITH ORDINALITY AS schedule(mark_id,position) \
             WHERE marks.mark_id=schedule.mark_id",
        )
        .bind(mark_ids)
        .bind(count)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub(crate) struct AuditEvent<'a> {
    pub account_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub character_id: Option<Uuid>,
    pub command_id: Option<Uuid>,
    pub actor: &'a str,
    pub action: &'a str,
    pub result: &'a str,
}
