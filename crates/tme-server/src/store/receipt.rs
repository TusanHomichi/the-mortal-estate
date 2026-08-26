use serde::{Deserialize, Serialize};
use sqlx::Row;
use tme_protocol as wire;
use tme_rules::ObservedEventV1;

pub const RECEIPT_OUTCOME_SCHEMA_VERSION: u16 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptOutcomeV3 {
    schema_version: u16,
    disposition: ReceiptDispositionV3,
    server_sequence: Option<u64>,
    before_revision: Option<u64>,
    after_revision: Option<u64>,
    events: Vec<ObservedEventV1>,
    events_truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ReceiptDispositionV3 {
    Accepted {},
    Rejected { code: ReceiptRejectionV3 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReceiptRejectionV3 {
    WrongActor,
    StaleControlEpoch,
    FutureWorldRevision,
    OutOfOrderClientSequence,
    RulesRejected,
    ProjectionFailed,
}

impl ReceiptOutcomeV3 {
    pub fn accepted(
        server_sequence: u64,
        before_revision: u64,
        after_revision: u64,
        events: Vec<ObservedEventV1>,
        events_truncated: bool,
    ) -> Self {
        Self {
            schema_version: RECEIPT_OUTCOME_SCHEMA_VERSION,
            disposition: ReceiptDispositionV3::Accepted {},
            server_sequence: Some(server_sequence),
            before_revision: Some(before_revision),
            after_revision: Some(after_revision),
            events,
            events_truncated,
        }
    }

    pub fn accepted_control() -> Self {
        Self {
            schema_version: RECEIPT_OUTCOME_SCHEMA_VERSION,
            disposition: ReceiptDispositionV3::Accepted {},
            server_sequence: None,
            before_revision: None,
            after_revision: None,
            events: Vec::new(),
            events_truncated: false,
        }
    }

    pub fn rejected(
        code: wire::RejectionCode,
        server_sequence: Option<u64>,
        revision: Option<u64>,
    ) -> Self {
        Self {
            schema_version: RECEIPT_OUTCOME_SCHEMA_VERSION,
            disposition: ReceiptDispositionV3::Rejected { code: code.into() },
            server_sequence,
            before_revision: revision,
            after_revision: revision,
            events: Vec::new(),
            events_truncated: false,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        let decoded: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if &decoded != self {
            return Err("receipt outcome failed canonical self-check".to_string());
        }
        Ok(bytes)
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if value.schema_version != RECEIPT_OUTCOME_SCHEMA_VERSION
            || value.encode()?.as_slice() != bytes
        {
            return Err("receipt outcome is noncanonical or unsupported".to_string());
        }
        Ok(value)
    }

    pub fn disposition_name(&self) -> &'static str {
        match self.disposition {
            ReceiptDispositionV3::Accepted {} => "accepted",
            ReceiptDispositionV3::Rejected { .. } => "rejected",
        }
    }

    pub fn audit_result(&self) -> &'static str {
        match self.disposition {
            ReceiptDispositionV3::Accepted {} => "success",
            ReceiptDispositionV3::Rejected { .. } => "rejected",
        }
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self.disposition, ReceiptDispositionV3::Accepted {})
    }

    pub fn to_envelope(
        &self,
        command_id: wire::CommandId,
        replay_status: wire::ReplayStatus,
    ) -> Result<wire::ServerEnvelope, String> {
        let disposition = match self.disposition {
            ReceiptDispositionV3::Accepted {} => wire::CommandDisposition::Accepted,
            ReceiptDispositionV3::Rejected { code } => {
                wire::CommandDisposition::Rejected { code: code.into() }
            }
        };
        Ok(wire::ServerEnvelope::CommandResult {
            command_id,
            disposition,
            replay_status,
            server_sequence: self.server_sequence.map(wire::DecimalU64::new),
            before_revision: self.before_revision.map(wire::DecimalU64::new),
            after_revision: self.after_revision.map(wire::DecimalU64::new),
            events: crate::protocol_v1::events(&self.events).map_err(|error| error.to_string())?,
            events_truncated: self.events_truncated,
        })
    }
}

impl From<wire::RejectionCode> for ReceiptRejectionV3 {
    fn from(value: wire::RejectionCode) -> Self {
        match value {
            wire::RejectionCode::WrongActor => Self::WrongActor,
            wire::RejectionCode::StaleControlEpoch => Self::StaleControlEpoch,
            wire::RejectionCode::FutureWorldRevision => Self::FutureWorldRevision,
            wire::RejectionCode::OutOfOrderClientSequence => Self::OutOfOrderClientSequence,
            wire::RejectionCode::RulesRejected => Self::RulesRejected,
            wire::RejectionCode::ProjectionFailed => Self::ProjectionFailed,
        }
    }
}

impl From<ReceiptRejectionV3> for wire::RejectionCode {
    fn from(value: ReceiptRejectionV3) -> Self {
        match value {
            ReceiptRejectionV3::WrongActor => Self::WrongActor,
            ReceiptRejectionV3::StaleControlEpoch => Self::StaleControlEpoch,
            ReceiptRejectionV3::FutureWorldRevision => Self::FutureWorldRevision,
            ReceiptRejectionV3::OutOfOrderClientSequence => Self::OutOfOrderClientSequence,
            ReceiptRejectionV3::RulesRejected => Self::RulesRejected,
            ReceiptRejectionV3::ProjectionFailed => Self::ProjectionFailed,
        }
    }
}

pub struct StoredReceipt {
    pub request_digest: [u8; 32],
    pub outcome: Option<ReceiptOutcomeV3>,
}

impl StoredReceipt {
    pub(crate) fn decode(row: sqlx::postgres::PgRow) -> Result<Self, String> {
        let digest: Vec<u8> = row
            .try_get("request_digest")
            .map_err(|error| error.to_string())?;
        let disposition: String = row
            .try_get("disposition")
            .map_err(|error| error.to_string())?;
        let bytes: Option<Vec<u8>> = row
            .try_get("outcome_bytes")
            .map_err(|error| error.to_string())?;
        Self::decode_parts(digest, disposition, bytes)
    }

    pub(crate) fn decode_parts(
        digest: Vec<u8>,
        disposition: String,
        bytes: Option<Vec<u8>>,
    ) -> Result<Self, String> {
        let request_digest: [u8; 32] = digest
            .try_into()
            .map_err(|_| "stored request digest has the wrong length".to_string())?;
        let outcome = match bytes {
            Some(bytes) => Some(ReceiptOutcomeV3::decode(&bytes)?),
            None if disposition == "expired" => None,
            None => return Err("non-expired receipt lacks outcome".to_string()),
        };
        Ok(Self {
            request_digest,
            outcome,
        })
    }
}

pub fn expired_envelope(command_id: wire::CommandId) -> wire::ServerEnvelope {
    wire::ServerEnvelope::CommandResult {
        command_id,
        disposition: wire::CommandDisposition::CommandResultExpired,
        replay_status: wire::ReplayStatus::Replayed,
        server_sequence: None,
        before_revision: None,
        after_revision: None,
        events: Vec::new(),
        events_truncated: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_is_protocol_neutral_and_strict() {
        let outcome = ReceiptOutcomeV3::accepted(4, 2, 3, Vec::new(), false);
        let bytes = outcome.encode().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(!text.contains("command_id"));
        assert!(!text.contains("replay_status"));
        assert_eq!(ReceiptOutcomeV3::decode(&bytes).unwrap(), outcome);
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["extra"] = serde_json::json!(true);
        assert!(ReceiptOutcomeV3::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
