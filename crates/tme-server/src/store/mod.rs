mod checked;
pub mod migrations;
pub mod receipt;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicU8, Ordering};

use sqlx::postgres::PgPool;
use sqlx::{Postgres, Row, Transaction};
use tme_protocol as wire;
use tme_rules::{DurableGameplayEffectV1, FacetCheckpointV4};
use uuid::Uuid;

use self::receipt::ReceiptOutcomeV3;

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
    #[cfg(test)]
    ev_fault: Arc<AtomicU8>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum EvStoreFault {
    ReceiptSqlAcquire = 1,
    CommandSqlAcquire = 2,
    CommandRowLock = 3,
    CommandReceiptInsert = 4,
    CommandDurableEffects = 5,
    CommandCompareAndSwap = 6,
    CommandAudit = 7,
    CommandCommit = 8,
    SystemSqlAcquire = 9,
    SystemCompareAndSwap = 10,
    SystemAudit = 11,
    SystemCommit = 12,
    AuthorityRejectionInsert = 13,
    AuthorityRejectionCommit = 14,
    AuthorityRejectionOutcomeUnknown = 15,
    CommandCommitOutcomeUnknown = 16,
}

#[derive(Debug)]
pub(crate) enum CommandCommitError {
    Definite,
    OutcomeUnknown,
}

impl From<String> for CommandCommitError {
    fn from(_error: String) -> Self {
        Self::Definite
    }
}

pub struct CommandCommit<'a> {
    pub account_id: wire::AccountId,
    pub session_id: wire::SessionId,
    pub character_id: wire::CharacterId,
    pub command_id: wire::CommandId,
    pub request_digest: [u8; 32],
    pub facet_id: wire::FacetId,
    pub actor_id: &'a str,
    pub control_epoch: u64,
    pub client_sequence: u64,
    pub expected_server_sequence: u64,
    pub expected_revision: u64,
    pub next_server_sequence: u64,
    pub next_revision: u64,
    pub checkpoint: &'a FacetCheckpointV4,
    pub outcome: &'a ReceiptOutcomeV3,
    pub durable_effects: &'a [DurableGameplayEffectV1],
}

pub struct SystemCommit<'a> {
    pub facet_id: wire::FacetId,
    pub expected_server_sequence: u64,
    pub expected_revision: u64,
    pub next_server_sequence: u64,
    pub next_revision: u64,
    pub checkpoint: &'a FacetCheckpointV4,
    pub action: &'static str,
    pub durable_effects: &'a [DurableGameplayEffectV1],
}

impl PostgresStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            #[cfg(test)]
            ev_fault: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[cfg(test)]
    pub(crate) fn ev_arm_fault(&self, fault: EvStoreFault) {
        assert_eq!(
            self.ev_fault.swap(fault as u8, Ordering::AcqRel),
            0,
            "EV store fault control was already armed"
        );
    }

    #[cfg(test)]
    pub(crate) fn ev_assert_fault_consumed(&self) {
        assert_eq!(
            self.ev_fault.load(Ordering::Acquire),
            0,
            "armed EV store fault was not reached"
        );
    }

    #[cfg(test)]
    fn ev_fail_if(&self, fault: EvStoreFault) -> Result<(), String> {
        if self
            .ev_fault
            .compare_exchange(fault as u8, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Err(format!("EV injected store failure at {fault:?}"))
        } else {
            Ok(())
        }
    }

    pub async fn reconcile_player_kill_marks(&self, account_id: Uuid) -> Result<(), String> {
        let mut tx = serializable(&self.pool).await?;
        sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(store_error)?
            .ok_or_else(|| "mark reconciliation account is absent".to_string())?;
        reschedule_player_kill_marks_raw(&mut tx, account_id, false)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)
    }

    pub async fn reconcile_all_player_kill_marks(&self) -> Result<(), String> {
        let account_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT killer_account_id FROM tme.player_kill_marks \
             WHERE forgiven_at IS NULL AND expired_at IS NULL ORDER BY killer_account_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;
        for account_id in account_ids {
            self.reconcile_player_kill_marks(account_id).await?;
        }
        Ok(())
    }

    pub async fn verify_player_kill_marks(&self) -> Result<(), String> {
        const TWO_WEEKS_MICROSECONDS: i64 = 14 * 24 * 60 * 60 * 1_000_000;

        let rows = sqlx::query(
            "SELECT mark_id,facet_kill_sequence,killer_account_id, \
                    killer_character_id,victim_account_id,victim_character_id, \
                    killer_session_id,victim_session_id,linked_karma_added, \
                    karma_forgiveness_eligible,(forgiven_at IS NOT NULL) AS forgiven, \
                    (expired_at IS NOT NULL) AS expired, \
                    CASE WHEN expires_at IS NULL THEN NULL ELSE \
                      (EXTRACT(EPOCH FROM expires_at) * 1000000)::bigint END AS expires_at_us \
             FROM tme.player_kill_marks \
             ORDER BY killer_account_id,assessed_at,mark_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;
        let mut active = BTreeMap::<Uuid, Vec<(Uuid, Option<i64>)>>::new();
        for row in rows {
            let mark_id: Uuid = row.try_get("mark_id").map_err(store_error)?;
            let facet_kill_sequence: i64 =
                row.try_get("facet_kill_sequence").map_err(store_error)?;
            let killer_account_id: Uuid = row.try_get("killer_account_id").map_err(store_error)?;
            let killer_character_id: Uuid =
                row.try_get("killer_character_id").map_err(store_error)?;
            let victim_account_id: Uuid = row.try_get("victim_account_id").map_err(store_error)?;
            let victim_character_id: Uuid =
                row.try_get("victim_character_id").map_err(store_error)?;
            let killer_session_id: Option<Uuid> =
                row.try_get("killer_session_id").map_err(store_error)?;
            let victim_session_id: Uuid = row.try_get("victim_session_id").map_err(store_error)?;
            let linked_karma_added: bool =
                row.try_get("linked_karma_added").map_err(store_error)?;
            let karma_forgiveness_eligible: bool = row
                .try_get("karma_forgiveness_eligible")
                .map_err(store_error)?;
            let forgiven: bool = row.try_get("forgiven").map_err(store_error)?;
            let expired: bool = row.try_get("expired").map_err(store_error)?;
            let expires_at_us: Option<i64> = row.try_get("expires_at_us").map_err(store_error)?;

            let expected_name =
                format!("https://tme.invalid/ids/player-kill-mark/v1/{facet_kill_sequence}");
            if mark_id != Uuid::new_v5(&Uuid::NAMESPACE_URL, expected_name.as_bytes()) {
                return Err("player-kill mark ID differs from its deterministic identity".into());
            }
            if killer_account_id == victim_account_id
                || killer_character_id == victim_character_id
                || killer_session_id == Some(victim_session_id)
            {
                return Err("player-kill mark records contradictory killer/victim identity".into());
            }
            if karma_forgiveness_eligible && (!linked_karma_added || killer_session_id.is_none()) {
                return Err("player-kill mark has impossible forgiveness eligibility".into());
            }
            if forgiven || expired {
                if expires_at_us.is_some() {
                    return Err("historical player-kill mark retains an active expiry".into());
                }
            } else {
                active
                    .entry(killer_account_id)
                    .or_default()
                    .push((mark_id, expires_at_us));
            }
        }
        for marks in active.values() {
            if marks.len() >= 4 {
                if marks.iter().any(|(_, expires_at)| expires_at.is_some()) {
                    return Err("locked player-kill mark schedule is not paused".into());
                }
                continue;
            }
            if marks.iter().any(|(_, expires_at)| expires_at.is_none()) {
                return Err("active player-kill mark schedule is missing an expiry".into());
            }
            for pair in marks.windows(2) {
                let older = pair[0].1.expect("active schedule expiry checked");
                let newer = pair[1].1.expect("active schedule expiry checked");
                if older.checked_sub(newer) != Some(TWO_WEEKS_MICROSECONDS) {
                    return Err("active player-kill mark schedule spacing is corrupt".into());
                }
            }
        }
        Ok(())
    }

    pub async fn receipt(
        &self,
        account_id: wire::AccountId,
        command_id: wire::CommandId,
    ) -> Result<Option<receipt::StoredReceipt>, String> {
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::ReceiptSqlAcquire)?;
        let mut tx = serializable(&self.pool).await?;
        let mut row = checked::receipt(&mut tx, account_id.as_uuid(), command_id.as_uuid())
            .await
            .map_err(store_error)?;
        if row
            .as_ref()
            .is_some_and(|value| value.full_expired && value.outcome_bytes.is_some())
        {
            let updated = sqlx::query(
                "UPDATE tme.command_receipts SET disposition='expired',outcome_bytes=NULL, \
                 session_id=NULL,actor_id=NULL,control_epoch=NULL, \
                 client_sequence=NULL,server_sequence=NULL,before_revision=NULL,after_revision=NULL \
                 WHERE account_id=$1 AND command_id=$2 AND outcome_bytes IS NOT NULL",
            )
            .bind(account_id.as_uuid())
            .bind(command_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(store_error)?;
            if updated.rows_affected() != 1 {
                return Err("receipt tombstone compare-and-swap failed".to_string());
            }
            if let Some(value) = row.as_mut() {
                value.disposition = "expired".to_string();
                value.outcome_bytes = None;
            }
        }
        tx.commit().await.map_err(store_error)?;
        row.map(|row| {
            receipt::StoredReceipt::decode_parts(
                row.request_digest,
                row.disposition,
                row.outcome_bytes,
            )
        })
        .transpose()
    }

    pub async fn insert_authority_rejection(
        &self,
        account_id: wire::AccountId,
        session_id: wire::SessionId,
        command_id: wire::CommandId,
        request_digest: [u8; 32],
        outcome: &ReceiptOutcomeV3,
    ) -> Result<(), String> {
        let bytes = outcome.encode()?;
        let mut tx = serializable(&self.pool).await?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::AuthorityRejectionInsert)?;
        let result = sqlx::query(
            "INSERT INTO tme.command_receipts \
             (account_id, command_id, request_digest, session_id, outcome_schema, disposition, \
              outcome_bytes, full_expires_at) \
             VALUES ($1, $2, $3, $4, 3, $5, $6, statement_timestamp() + interval '90 days') \
             ON CONFLICT DO NOTHING",
        )
        .bind(account_id.as_uuid())
        .bind(command_id.as_uuid())
        .bind(request_digest.as_slice())
        .bind(session_id.as_uuid())
        .bind(outcome.disposition_name())
        .bind(bytes)
        .execute(&mut *tx)
        .await
        .map_err(store_error)?;
        if result.rows_affected() != 1 {
            return Err("command receipt already exists".to_string());
        }
        audit(
            &mut tx,
            AuditEvent {
                account_id: Some(account_id.as_uuid()),
                session_id: Some(session_id.as_uuid()),
                character_id: None,
                command_id: Some(command_id.as_uuid()),
                actor: "runtime",
                action: "command",
                result: outcome.audit_result(),
            },
        )
        .await?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::AuthorityRejectionCommit)?;
        tx.commit().await.map_err(store_error)?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::AuthorityRejectionOutcomeUnknown)?;
        Ok(())
    }

    pub(crate) async fn commit_command(
        &self,
        value: CommandCommit<'_>,
    ) -> Result<(), CommandCommitError> {
        let expected_sequence = checked_i64(value.expected_server_sequence)?;
        let expected_revision = checked_i64(value.expected_revision)?;
        let next_sequence = checked_i64(value.next_server_sequence)?;
        let next_revision = checked_i64(value.next_revision)?;
        let outcome_bytes = value.outcome.encode()?;
        let checkpoint_sha = value.checkpoint.sha256();

        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandSqlAcquire)?;
        // The facet row lock is the command/control cutover order for ordinary
        // single-facet mutations. READ COMMITTED avoids false serialization
        // failures when independent facets append receipts and audit rows at
        // the same time. Cross-account durable effects retain SERIALIZABLE.
        let mut tx = if value.durable_effects.is_empty() {
            self.pool.begin().await.map_err(store_error)?
        } else {
            serializable(&self.pool).await?
        };
        let row = sqlx::query(
            "SELECT last_server_sequence, facet_revision FROM tme.facets \
             WHERE facet_id = $1 FOR UPDATE",
        )
        .bind(value.facet_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(store_error)?
        .ok_or_else(|| "facet is absent".to_string())?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandRowLock)?;
        let stored_sequence: i64 = row.try_get("last_server_sequence").map_err(store_error)?;
        let stored_revision: i64 = row.try_get("facet_revision").map_err(store_error)?;
        if stored_sequence != expected_sequence || stored_revision != expected_revision {
            return Err("facet durable revision or sequence changed"
                .to_string()
                .into());
        }
        if value.outcome.is_accepted() {
            let authority: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tme.characters c JOIN tme.sessions s \
                 ON s.session_id=$2 AND s.account_id=c.account_id \
                 WHERE c.character_id=$1 AND c.account_id=$3 \
                 AND c.actor_id=$4 AND c.control_epoch=$5 AND s.revoked_at IS NULL \
                 AND s.idle_expires_at>statement_timestamp() \
                 AND s.absolute_expires_at>statement_timestamp())",
            )
            .bind(value.character_id.as_uuid())
            .bind(value.session_id.as_uuid())
            .bind(value.account_id.as_uuid())
            .bind(value.actor_id)
            .bind(checked_i64(value.control_epoch)?)
            .fetch_one(&mut *tx)
            .await
            .map_err(store_error)?;
            if !authority {
                return Err("durable command authority changed".to_string().into());
            }
        }

        let inserted = sqlx::query(
            "INSERT INTO tme.command_receipts \
             (account_id, command_id, request_digest, session_id, actor_id, \
              control_epoch, client_sequence, server_sequence, before_revision, after_revision, \
              outcome_schema, disposition, outcome_bytes, full_expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,3,$11,$12, \
                     statement_timestamp() + interval '90 days') \
             ON CONFLICT DO NOTHING",
        )
        .bind(value.account_id.as_uuid())
        .bind(value.command_id.as_uuid())
        .bind(value.request_digest.as_slice())
        .bind(value.session_id.as_uuid())
        .bind(value.actor_id)
        .bind(checked_i64(value.control_epoch)?)
        .bind(checked_i64(value.client_sequence)?)
        .bind(next_sequence)
        .bind(expected_revision)
        .bind(next_revision)
        .bind(value.outcome.disposition_name())
        .bind(outcome_bytes)
        .execute(&mut *tx)
        .await
        .map_err(store_error)?;
        if inserted.rows_affected() != 1 {
            return Err("command receipt already exists".to_string().into());
        }
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandReceiptInsert)?;
        persist_durable_effects(
            &mut tx,
            Some((
                value.account_id.as_uuid(),
                value.session_id.as_uuid(),
                value.character_id.as_uuid(),
            )),
            value.durable_effects,
        )
        .await?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandDurableEffects)?;

        let updated = sqlx::query(
            "UPDATE tme.facets SET checkpoint_bytes=$2, checkpoint_sha256=$3, \
             facet_revision=$4, last_server_sequence=$5, updated_at=statement_timestamp() \
             WHERE facet_id=$1 AND facet_revision=$6 AND last_server_sequence=$7",
        )
        .bind(value.facet_id.as_uuid())
        .bind(value.checkpoint.as_bytes())
        .bind(checkpoint_sha.as_slice())
        .bind(next_revision)
        .bind(next_sequence)
        .bind(expected_revision)
        .bind(expected_sequence)
        .execute(&mut *tx)
        .await
        .map_err(store_error)?;
        if updated.rows_affected() != 1 {
            return Err("facet compare-and-swap failed".to_string().into());
        }
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandCompareAndSwap)?;
        audit(
            &mut tx,
            AuditEvent {
                account_id: Some(value.account_id.as_uuid()),
                session_id: Some(value.session_id.as_uuid()),
                character_id: Some(value.character_id.as_uuid()),
                command_id: Some(value.command_id.as_uuid()),
                actor: "runtime",
                action: "command",
                result: value.outcome.audit_result(),
            },
        )
        .await?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandAudit)?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandCommit)?;
        tx.commit()
            .await
            .map_err(|_| CommandCommitError::OutcomeUnknown)?;
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::CommandCommitOutcomeUnknown)
            .map_err(|_| CommandCommitError::OutcomeUnknown)?;
        Ok(())
    }

    pub async fn commit_system(&self, value: SystemCommit<'_>) -> Result<(), String> {
        let expected_sequence = checked_i64(value.expected_server_sequence)?;
        let expected_revision = checked_i64(value.expected_revision)?;
        let next_sequence = checked_i64(value.next_server_sequence)?;
        let next_revision = checked_i64(value.next_revision)?;
        let checkpoint_sha = value.checkpoint.sha256();
        #[cfg(test)]
        self.ev_fail_if(EvStoreFault::SystemSqlAcquire)?;
        match commit_system_once(
            &self.pool,
            value.facet_id.as_uuid(),
            value.checkpoint.as_bytes(),
            checkpoint_sha.as_slice(),
            expected_revision,
            expected_sequence,
            next_revision,
            next_sequence,
            value.action,
            value.durable_effects,
            #[cfg(test)]
            Some(self),
        )
        .await
        {
            Ok(true) => Ok(()),
            Ok(false) => Err("system facet compare-and-swap failed".to_string()),
            Err(error) => Err(store_error(error)),
        }
    }
}

pub(crate) async fn serializable(pool: &PgPool) -> Result<Transaction<'_, Postgres>, String> {
    serializable_raw(pool).await.map_err(store_error)
}

async fn serializable_raw(pool: &PgPool) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

#[allow(clippy::too_many_arguments)]
async fn commit_system_once(
    pool: &PgPool,
    facet_id: Uuid,
    checkpoint_bytes: &[u8],
    checkpoint_sha: &[u8],
    expected_revision: i64,
    expected_sequence: i64,
    next_revision: i64,
    next_sequence: i64,
    action: &'static str,
    durable_effects: &[DurableGameplayEffectV1],
    #[cfg(test)] fault_store: Option<&PostgresStore>,
) -> Result<bool, sqlx::Error> {
    // A facet task is the sole writer for its Engine, and this update retains
    // the exact durable revision/sequence compare-and-swap. READ COMMITTED
    // keeps independent one-second facet ticks from forming serialization
    // pivots through their shared append-only audit table; the facet row and
    // audit receipt still commit atomically in this one transaction.
    let mut tx = if durable_effects.is_empty() {
        pool.begin().await?
    } else {
        serializable_raw(pool).await?
    };
    persist_durable_effects_raw(&mut tx, None, durable_effects).await?;
    let updated = sqlx::query(
        "UPDATE tme.facets SET checkpoint_bytes=$2, checkpoint_sha256=$3, \
         facet_revision=$4, last_server_sequence=$5, updated_at=statement_timestamp() \
         WHERE facet_id=$1 AND facet_revision=$6 AND last_server_sequence=$7",
    )
    .bind(facet_id)
    .bind(checkpoint_bytes)
    .bind(checkpoint_sha)
    .bind(next_revision)
    .bind(next_sequence)
    .bind(expected_revision)
    .bind(expected_sequence)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Ok(false);
    }
    #[cfg(test)]
    if let Some(store) = fault_store {
        store
            .ev_fail_if(EvStoreFault::SystemCompareAndSwap)
            .map_err(sqlx::Error::Protocol)?;
    }
    audit_raw(
        &mut tx,
        AuditEvent {
            account_id: None,
            session_id: None,
            character_id: None,
            command_id: None,
            actor: "runtime",
            action,
            result: "success",
        },
    )
    .await?;
    #[cfg(test)]
    if let Some(store) = fault_store {
        store
            .ev_fail_if(EvStoreFault::SystemAudit)
            .map_err(sqlx::Error::Protocol)?;
        store
            .ev_fail_if(EvStoreFault::SystemCommit)
            .map_err(sqlx::Error::Protocol)?;
    }
    tx.commit().await?;
    Ok(true)
}

fn protocol_store_error(message: impl Into<String>) -> sqlx::Error {
    sqlx::Error::Protocol(message.into())
}

async fn persist_durable_effects(
    tx: &mut Transaction<'_, Postgres>,
    issuer: Option<(Uuid, Uuid, Uuid)>,
    effects: &[DurableGameplayEffectV1],
) -> Result<(), String> {
    persist_durable_effects_raw(tx, issuer, effects)
        .await
        .map_err(store_error)
}

async fn persist_durable_effects_raw(
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
        let logical_time = assessment.logical_time.value().to_string();
        let karma_forgiveness_eligible = linked_karma_added && killer_session_id.is_some();
        let inserted = sqlx::query(
            "INSERT INTO tme.player_kill_marks \
             (mark_id,facet_kill_sequence,killer_account_id,killer_character_id, \
              victim_account_id,victim_character_id,killer_session_id,victim_session_id, \
              assessed_at,assessed_logical_time,linked_karma_added, \
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
                 AND victim_session_id=$8 AND assessed_logical_time=CAST($9 AS numeric) \
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
                  assessed_logical_time) \
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

fn alignment_label(value: tme_rules::CharacterAlignment) -> &'static str {
    match value {
        tme_rules::CharacterAlignment::Lawful => "lawful",
        tme_rules::CharacterAlignment::Neutral => "neutral",
        tme_rules::CharacterAlignment::Chaotic => "chaotic",
        tme_rules::CharacterAlignment::Evil => "evil",
    }
}

fn nature_label(value: tme_rules::SocialNature) -> &'static str {
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
            assessed_logical_time::text AS assessed_logical_time \
     FROM tme.pending_player_kill_consequences \
     WHERE killer_account_id=$1 AND killer_character_id=$2 \
     ORDER BY facet_kill_sequence";

fn decode_pending_kill_consequences(
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
        let logical_time: String = row.try_get("assessed_logical_time")?;
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
                logical_time: tme_rules::LogicalTime::new(logical_time),
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

pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    event: AuditEvent<'_>,
) -> Result<(), String> {
    audit_raw(tx, event).await.map_err(store_error)
}

async fn audit_raw(
    tx: &mut Transaction<'_, Postgres>,
    event: AuditEvent<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tme.audit_events \
         (account_id,session_id,character_id,command_id,actor,action,result,correlation_id) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(event.account_id)
    .bind(event.session_id)
    .bind(event.character_id)
    .bind(event.command_id)
    .bind(event.actor)
    .bind(event.action)
    .bind(event.result)
    .bind(Uuid::now_v7())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) fn checked_i64(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| "counter exceeds PostgreSQL bigint".to_string())
}

pub(crate) fn checked_u64(value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "negative durable counter".to_string())
}

pub(crate) fn store_error(error: impl std::fmt::Display) -> String {
    format!("durable store operation failed: {error}")
}

pub type SharedStore = Arc<PostgresStore>;
