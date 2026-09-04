use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientHelloEnvelope {
    ClientHello {
        ticket: AdmissionTicket,
        supported_minors: Vec<u16>,
    },
}

impl ClientHelloEnvelope {
    pub fn into_parts(self) -> Result<(AdmissionTicket, Vec<u16>), ProtocolError> {
        let Self::ClientHello {
            ticket,
            supported_minors,
        } = self;
        if supported_minors.is_empty() || supported_minors.len() > MAX_SUPPORTED_MINORS {
            return Err(ProtocolError::new(
                "supported_minors must contain 1-8 values",
            ));
        }
        let unique = supported_minors.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != supported_minors.len() {
            return Err(ProtocolError::new("supported_minors must be unique"));
        }
        Ok((ticket, supported_minors))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientCommandEnvelope {
    Command {
        command_id: CommandId,
        control_epoch: DecimalU64,
        client_sequence: DecimalU64,
        observed_world_revision: DecimalU64,
        actor_id: ActorId,
        intent: Intent,
    },
    PathPreview {
        preview_id: PreviewId,
        control_epoch: DecimalU64,
        observed_world_revision: DecimalU64,
        actor_id: ActorId,
        path: Vec<Direction>,
    },
    SocialMessage {
        message_id: MessageId,
        control_epoch: DecimalU64,
        actor_id: ActorId,
        scope: SocialScope,
        body: SocialBody,
    },
}

impl ClientCommandEnvelope {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Command { intent, .. } => intent.validate(),
            Self::PathPreview { path, .. }
                if path.is_empty() || path.len() > MAX_MOVE_PATH_STEPS =>
            {
                Err(ProtocolError::new(
                    "path preview request must contain 1-3 steps",
                ))
            }
            Self::PathPreview { .. } => Ok(()),
            Self::SocialMessage { .. } => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SocialScope {
    Say,
    Shout,
    Group,
    Page { target_character_id: CharacterId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDisposition {
    Accepted,
    Unavailable,
    NotGrouped,
    RateLimited,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayStatus {
    New,
    Replayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionCode {
    WrongActor,
    StaleControlEpoch,
    FutureWorldRevision,
    OutOfOrderClientSequence,
    RulesRejected,
    ProjectionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandDisposition {
    Accepted,
    Rejected { code: RejectionCode },
    CommandResultExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPreviewRejectionCode {
    WrongActor,
    StaleControlEpoch,
    FutureWorldRevision,
    RulesRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathPreviewDisposition {
    Previewed,
    Rejected { code: PathPreviewRejectionCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrainingReason {
    Shutdown,
    ControlReplaced,
    SessionEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    MalformedProtocol,
    BinaryMessage,
    OversizedProtocol,
    UnsupportedVersion,
    InvalidTicket,
    ExpiredTicket,
    ConsumedTicket,
    OriginRejected,
    HostRejected,
    HelloTimeout,
    Capacity,
    RateLimited,
    QueuePressure,
    CommandInProgress,
    GameplayMarkLocked,
    Unavailable,
    Draining,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerEnvelope {
    ServerWelcome {
        selected_major: u16,
        selected_minor: u16,
        connection_id: ConnectionId,
        actor_id: ActorId,
        control_epoch: DecimalU64,
        server_sequence: DecimalU64,
        world_revision: DecimalU64,
        static_scene_context: StaticSceneContext,
        frame: ObserverFrame,
    },
    CommandResult {
        command_id: CommandId,
        disposition: CommandDisposition,
        replay_status: ReplayStatus,
        server_sequence: Option<DecimalU64>,
        before_revision: Option<DecimalU64>,
        after_revision: Option<DecimalU64>,
        events: Vec<ObservedEvent>,
        events_truncated: bool,
    },
    PathPreviewResult {
        preview_id: PreviewId,
        disposition: PathPreviewDisposition,
        control_epoch: DecimalU64,
        actor_id: ActorId,
        world_revision: DecimalU64,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        preview: Option<PathPreview>,
    },
    StateUpdate {
        server_sequence: DecimalU64,
        world_revision: DecimalU64,
        events: Vec<ObservedEvent>,
        events_truncated: bool,
        static_scene_context: StaticSceneContext,
        frame: ObserverFrame,
    },
    SocialMessage {
        message_id: MessageId,
        scope: SocialScope,
        sender_character_id: CharacterId,
        sender_name: DisplayName,
        body: SocialBody,
    },
    MessageResult {
        message_id: MessageId,
        disposition: MessageDisposition,
    },
    ServerDraining {
        reason: DrainingReason,
        reconnect_hint: bool,
    },
    Error {
        code: ErrorCode,
    },
}

impl ServerEnvelope {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::ServerWelcome {
                selected_major,
                selected_minor,
                static_scene_context,
                frame,
                ..
            } => {
                if *selected_major != PROTOCOL_MAJOR || *selected_minor != PROTOCOL_MINOR {
                    return Err(ProtocolError::new(
                        "welcome selected an unsupported version",
                    ));
                }
                static_scene_context.validate()?;
                validate_static_scene_frame_pair(static_scene_context, frame)
            }
            Self::CommandResult { events, .. } => validate_observed_events(events),
            Self::StateUpdate {
                static_scene_context,
                frame,
                events,
                ..
            } => {
                static_scene_context.validate()?;
                validate_static_scene_frame_pair(static_scene_context, frame)?;
                validate_observed_events(events)
            }
            Self::PathPreviewResult {
                disposition,
                preview,
                ..
            } => match (disposition, preview) {
                (PathPreviewDisposition::Previewed, Some(preview)) => preview.validate(),
                (PathPreviewDisposition::Rejected { .. }, None) => Ok(()),
                _ => Err(ProtocolError::new(
                    "path preview disposition and payload disagree",
                )),
            },
            _ => Ok(()),
        }
    }
}

pub(super) fn validate_static_scene_frame_pair(
    context: &StaticSceneContext,
    frame: &ObserverFrame,
) -> Result<(), ProtocolError> {
    frame.validate()?;
    if context.site.realm != frame.observation_center.realm
        || context.site.level != frame.observation_center.level
    {
        return Err(ProtocolError::new(
            "static scene context site differs from observer frame",
        ));
    }
    Ok(())
}

pub(super) fn validate_observed_events(events: &[ObservedEvent]) -> Result<(), ProtocolError> {
    if events.len() > MAX_OBSERVED_EVENTS {
        return Err(ProtocolError::new("observed event vector exceeds bound"));
    }
    events.iter().try_for_each(ObservedEvent::validate)
}

pub fn decode_client_hello(input: &[u8]) -> Result<ClientHelloEnvelope, ProtocolError> {
    let envelope: ClientHelloEnvelope = decode_strict(input)?;
    envelope.clone().into_parts()?;
    Ok(envelope)
}

pub fn decode_login_request(input: &[u8]) -> Result<LoginRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_logout_request(input: &[u8]) -> Result<LogoutRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_character_select_request(
    input: &[u8],
) -> Result<CharacterSelectRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_socket_ticket_request(input: &[u8]) -> Result<SocketTicketRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_forgive_player_kill_mark_request(
    input: &[u8],
) -> Result<ForgivePlayerKillMarkRequestV1, ProtocolError> {
    decode_control(input)
}

pub fn decode_client_command(
    input: &[u8],
    selected_minor: u16,
) -> Result<ClientCommandEnvelope, ProtocolError> {
    if selected_minor != PROTOCOL_MINOR {
        return Err(ProtocolError::new("unsupported selected protocol minor"));
    }
    let envelope: ClientCommandEnvelope = decode_strict(input)?;
    envelope.validate()?;
    Ok(envelope)
}

pub fn encode_server_envelope(envelope: &ServerEnvelope) -> Result<Vec<u8>, ProtocolError> {
    envelope.validate()?;
    let encoded =
        serde_json::to_vec(envelope).map_err(|error| ProtocolError::new(error.to_string()))?;
    if encoded.len() > MAX_SERVER_ENVELOPE_BYTES {
        return Err(ProtocolError::new(
            "encoded server envelope exceeds 256 KiB",
        ));
    }
    Ok(encoded)
}

pub(super) fn deserialize_required_nullable<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

pub(super) fn decode_strict<T: DeserializeOwned>(input: &[u8]) -> Result<T, ProtocolError> {
    decode_strict_with_limits(input, MAX_INPUT_BYTES, MAX_JSON_NESTING)
}

pub(super) fn decode_control<T: DeserializeOwned>(input: &[u8]) -> Result<T, ProtocolError> {
    decode_strict_with_limits(input, MAX_CONTROL_INPUT_BYTES, MAX_CONTROL_JSON_NESTING)
}

pub(super) fn decode_strict_with_limits<T: DeserializeOwned>(
    input: &[u8],
    max_bytes: usize,
    max_depth: usize,
) -> Result<T, ProtocolError> {
    if input.len() > max_bytes {
        return Err(ProtocolError::new("JSON input exceeds its byte bound"));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = StrictValueSeed {
        depth: 1,
        max_depth,
    }
    .deserialize(&mut deserializer)
    .map_err(|error| ProtocolError::new(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| ProtocolError::new(error.to_string()))?;
    serde_json::from_value(value).map_err(|error| ProtocolError::new(error.to_string()))
}

struct StrictValueSeed {
    depth: usize,
    max_depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.depth > self.max_depth {
            return Err(serde::de::Error::custom("JSON nesting exceeds its bound"));
        }
        deserializer.deserialize_any(StrictValueVisitor {
            depth: self.depth,
            max_depth: self.max_depth,
        })
    }
}

struct StrictValueVisitor {
    depth: usize,
    max_depth: usize,
}

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a strict JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictValueSeed {
            depth: self.depth + 1,
            max_depth: self.max_depth,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate object key {key:?}"
                )));
            }
            let value = map.next_value_seed(StrictValueSeed {
                depth: self.depth + 1,
                max_depth: self.max_depth,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}
