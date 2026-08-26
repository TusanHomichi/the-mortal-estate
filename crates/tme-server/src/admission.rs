use tme_protocol as wire;
use tme_rules::ActorId;

use crate::facet::FacetHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlGrant {
    pub account_id: wire::AccountId,
    pub session_id: wire::SessionId,
    pub connection_id: wire::ConnectionId,
    pub character_id: wire::CharacterId,
    pub facet_id: wire::FacetId,
    pub actor_id: ActorId,
    pub control_epoch: u64,
}

impl ControlGrant {
    pub fn new(
        account_id: wire::AccountId,
        session_id: wire::SessionId,
        connection_id: wire::ConnectionId,
        character_id: wire::CharacterId,
        facet_id: wire::FacetId,
        actor_id: ActorId,
        control_epoch: u64,
    ) -> Self {
        Self {
            account_id,
            session_id,
            connection_id,
            character_id,
            facet_id,
            actor_id,
            control_epoch,
        }
    }
}

#[derive(Clone)]
pub struct AdmissionGrant {
    pub control: ControlGrant,
    pub facet: FacetHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    InvalidTicket,
    ExpiredTicket,
    ConsumedTicket,
    UnsupportedVersion,
    OriginRejected,
    HostRejected,
    GameplayMarkLocked,
    Unavailable,
}
