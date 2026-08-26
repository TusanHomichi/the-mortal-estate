use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::*;

const DECODERS: [&str; 15] = [
    "decimal_u64",
    "decimal_i64",
    "login_request_v1",
    "logout_request_v1",
    "character_select_request_v1",
    "socket_ticket_request_v1",
    "forgive_player_kill_mark_request_v1",
    "session_bootstrap_v1",
    "character_selection_v1",
    "socket_ticket_v1",
    "forgive_player_kill_mark_result_v1",
    "control_error_v1",
    "client_hello_envelope",
    "client_command_envelope",
    "server_envelope",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureFile {
    schema_version: u32,
    decoder: String,
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    case_id: String,
    expect: String,
    reason: String,
    input_utf8: Option<String>,
    input_hex: Option<String>,
}

#[test]
fn client_wire_fixture_conformance() {
    // The shared wire corpus lives at the repository root and is read from
    // there by both sides of the wire: this test and the client's own codec
    // suite, which resolves the same absolute directory rather than keeping a
    // copy. The inventory assertion below stays exact, so a fixture added,
    // removed, or renamed on either side fails this test.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/wire");
    let expected = DECODERS
        .iter()
        .map(|decoder| format!("{decoder}.json"))
        .collect::<BTreeSet<_>>();
    let actual = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("cannot read fixture directory {}: {error}", root.display()))
        .map(|entry| {
            entry
                .expect("fixture directory entry must be readable")
                .file_name()
        })
        .map(|name| name.to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "shared fixture file inventory drifted");

    let mut assertion_count = 0usize;
    for decoder in DECODERS {
        assertion_count += check_fixture_file(&root.join(format!("{decoder}.json")), decoder);
    }
    assert!(
        assertion_count > 0,
        "fixture corpus must execute at least one case"
    );
}

fn check_fixture_file(path: &PathBuf, expected_decoder: &str) -> usize {
    let metadata_bytes = fs::read(path)
        .unwrap_or_else(|error| panic!("cannot read fixture {}: {error}", path.display()));
    let fixture: FixtureFile = decode_strict_with_limits(&metadata_bytes, 4 * 1024 * 1024, 64)
        .unwrap_or_else(|error| panic!("invalid fixture metadata {}: {error}", path.display()));
    assert_eq!(fixture.schema_version, 1, "fixture schema version drifted");
    assert_eq!(
        fixture.decoder, expected_decoder,
        "fixture decoder must equal filename stem"
    );
    assert!(
        !fixture.cases.is_empty(),
        "fixture case list must not be empty"
    );

    let mut case_ids = BTreeSet::new();
    let mut reasons = BTreeSet::new();
    for case in &fixture.cases {
        assert!(
            is_lower_snake(&case.case_id),
            "invalid case_id {}",
            case.case_id
        );
        assert!(
            is_lower_snake(&case.reason),
            "invalid reason {}",
            case.reason
        );
        assert!(
            case_ids.insert(&case.case_id),
            "duplicate case_id {}",
            case.case_id
        );
        assert!(
            reasons.insert(&case.reason),
            "duplicate reason {}",
            case.reason
        );
        assert!(matches!(case.expect.as_str(), "accept" | "reject"));

        let input = match (&case.input_utf8, &case.input_hex) {
            (Some(text), None) => text.as_bytes().to_vec(),
            (None, Some(text)) => decode_hex(text)
                .unwrap_or_else(|| panic!("invalid lowercase input_hex for {}", case.case_id)),
            _ => panic!("{} must contain exactly one input source", case.case_id),
        };
        let accepted = dispatch(expected_decoder, &input).is_ok();
        assert_eq!(
            accepted,
            case.expect == "accept",
            "fixture verdict drift for {expected_decoder}/{} ({})",
            case.case_id,
            case.reason
        );
    }
    fixture.cases.len()
}

fn dispatch(decoder: &str, input: &[u8]) -> Result<(), ProtocolError> {
    match decoder {
        "decimal_u64" => decode_strict::<DecimalU64>(input).map(drop),
        "decimal_i64" => decode_strict::<DecimalI64>(input).map(drop),
        "login_request_v1" => decode_login_request(input).map(drop),
        "logout_request_v1" => decode_logout_request(input).map(drop),
        "character_select_request_v1" => decode_character_select_request(input).map(drop),
        "socket_ticket_request_v1" => decode_socket_ticket_request(input).map(drop),
        "forgive_player_kill_mark_request_v1" => {
            decode_forgive_player_kill_mark_request(input).map(drop)
        }
        "session_bootstrap_v1" => {
            decode_control::<SessionBootstrapV1>(input).and_then(validate_bootstrap)
        }
        "character_selection_v1" => decode_control::<CharacterSelectionV1>(input)
            .and_then(|value| require_control_version(value.control_api_version)),
        "socket_ticket_v1" => decode_control::<SocketTicketV1>(input).and_then(validate_ticket),
        "forgive_player_kill_mark_result_v1" => {
            decode_control::<ForgivePlayerKillMarkResultV1>(input)
                .and_then(|value| require_control_version(value.control_api_version))
        }
        "control_error_v1" => decode_control::<ControlErrorV1>(input).map(drop),
        "client_hello_envelope" => decode_client_hello(input).and_then(validate_hello),
        "client_command_envelope" => decode_client_command(input, PROTOCOL_MINOR).map(drop),
        "server_envelope" => decode_strict_with_limits::<ServerEnvelope>(
            input,
            MAX_SERVER_ENVELOPE_BYTES,
            MAX_JSON_NESTING,
        )
        .and_then(validate_server_envelope),
        _ => Err(ProtocolError::new("unrecognized fixture decoder")),
    }
}

fn require_control_version(version: u16) -> Result<(), ProtocolError> {
    if version == CONTROL_API_VERSION {
        Ok(())
    } else {
        Err(ProtocolError::new("unsupported control API version"))
    }
}

fn validate_bootstrap(value: SessionBootstrapV1) -> Result<(), ProtocolError> {
    require_control_version(value.control_api_version)?;
    let character_ids = value
        .characters
        .iter()
        .map(|character| character.character_id)
        .collect::<BTreeSet<_>>();
    if character_ids.len() != value.characters.len() {
        return Err(ProtocolError::new("bootstrap identities must be unique"));
    }
    Ok(())
}

fn validate_ticket(value: SocketTicketV1) -> Result<(), ProtocolError> {
    if value.protocol_major != PROTOCOL_MAJOR || value.supported_minors != [PROTOCOL_MINOR] {
        return Err(ProtocolError::new("ticket selects an unsupported protocol"));
    }
    Ok(())
}

fn validate_hello(value: ClientHelloEnvelope) -> Result<(), ProtocolError> {
    let ClientHelloEnvelope::ClientHello {
        supported_minors, ..
    } = value;
    if supported_minors == [PROTOCOL_MINOR] {
        Ok(())
    } else {
        Err(ProtocolError::new(
            "hello must offer only the current minor",
        ))
    }
}

fn validate_server_envelope(value: ServerEnvelope) -> Result<(), ProtocolError> {
    value.validate()
}

fn is_lower_snake(value: &str) -> bool {
    !value.is_empty()
        && value.split('_').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.is_empty()
        || !value.len().is_multiple_of(2)
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Some(high * 16 + low)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
