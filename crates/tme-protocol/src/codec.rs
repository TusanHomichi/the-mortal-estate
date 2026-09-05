//! The browser and native corpus consume the same decoding and validation.
use super::*;

fn encoded<T: Serialize>(value: T) -> Result<Vec<u8>, ProtocolError> {
    serde_json::to_vec(&value).map_err(|_| ProtocolError::new("encoding failed"))
}

fn control_version(version: u16) -> Result<(), ProtocolError> {
    if version != CONTROL_API_VERSION {
        return Err(ProtocolError::new("unsupported control API version"));
    }
    Ok(())
}

pub fn decode_document(decoder: &str, input: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    match decoder {
        "decimal_u64" => encoded(decode_strict::<DecimalU64>(input)?),
        "decimal_i64" => encoded(decode_strict::<DecimalI64>(input)?),
        "session_bootstrap_request_v1" => {
            encoded(decode_control::<SessionBootstrapRequestV1>(input)?)
        }
        "login_request_v1" => encoded(decode_login_request(input)?),
        "logout_request_v1" => encoded(decode_logout_request(input)?),
        "character_select_request_v1" => encoded(decode_character_select_request(input)?),
        "socket_ticket_request_v1" => encoded(decode_socket_ticket_request(input)?),
        "forgive_player_kill_mark_request_v1" => {
            encoded(decode_forgive_player_kill_mark_request(input)?)
        }
        "login_response_v1" => {
            let value = decode_control::<LoginResponseV1>(input)?;
            validate_bootstrap(&value.bootstrap)?;
            encoded(value)
        }
        "session_bootstrap_v1" => {
            let value = decode_control::<SessionBootstrapV1>(input)?;
            validate_bootstrap(&value)?;
            encoded(value)
        }
        "character_selection_v1" => {
            let value = decode_control::<CharacterSelectionV1>(input)?;
            control_version(value.control_api_version)?;
            encoded(value)
        }
        "socket_ticket_v1" => {
            let value = decode_control::<SocketTicketV1>(input)?;
            if value.protocol_major != PROTOCOL_MAJOR || value.supported_minors != [PROTOCOL_MINOR]
            {
                return Err(ProtocolError::new("ticket selects an unsupported protocol"));
            }
            encoded(value)
        }
        "forgive_player_kill_mark_result_v1" => {
            let value = decode_control::<ForgivePlayerKillMarkResultV1>(input)?;
            control_version(value.control_api_version)?;
            encoded(value)
        }
        "control_error_v1" => encoded(decode_control::<ControlErrorV1>(input)?),
        "client_hello_envelope" => {
            let value = decode_client_hello(input)?;
            let ClientHelloEnvelope::ClientHello {
                supported_minors, ..
            } = &value;
            if supported_minors != &[PROTOCOL_MINOR] {
                return Err(ProtocolError::new(
                    "hello must offer only the current minor",
                ));
            }
            encoded(value)
        }
        "client_command_envelope" => encoded(decode_client_command(input, PROTOCOL_MINOR)?),
        "server_envelope" => {
            let value = decode_strict_with_limits::<ServerEnvelope>(
                input,
                MAX_SERVER_ENVELOPE_BYTES,
                MAX_JSON_NESTING,
            )?;
            value.validate()?;
            encoded(value)
        }
        _ => Err(ProtocolError::new("unrecognized decoder")),
    }
}

fn validate_bootstrap(value: &SessionBootstrapV1) -> Result<(), ProtocolError> {
    control_version(value.control_api_version)?;
    let ids = value
        .characters
        .iter()
        .map(|row| row.character_id)
        .collect::<BTreeSet<_>>();
    if ids.len() != value.characters.len() {
        return Err(ProtocolError::new("bootstrap identities must be unique"));
    }
    Ok(())
}
