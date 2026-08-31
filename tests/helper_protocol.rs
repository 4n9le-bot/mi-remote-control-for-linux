use atvv_bridge::button_mapping::{ButtonId, MappingTarget};
use atvv_bridge::helper_protocol::{ProtocolError, Request, decode_request};

fn envelope(operation: &str) -> String {
    format!(r#"{{"operation":"{operation}","protocol_version":1,"catalog_version":1}}"#)
}

#[test]
fn decodes_each_operation() {
    assert_eq!(
        decode_request(envelope("inspect").as_bytes()),
        Ok(Request::Inspect)
    );
    assert_eq!(
        decode_request(envelope("reset").as_bytes()),
        Ok(Request::Reset)
    );
}

#[test]
fn apply_requires_complete_mapping_and_accepts_duplicate_targets() {
    let body = r#"{"operation":"apply","protocol_version":1,"catalog_version":1,"expected_revision":"r1","mapping":[{"button":"power","target":{"type":"disabled"}},{"button":"confirm","target":{"type":"key","key":"KEY_ENTER"}},{"button":"up","target":{"type":"key","key":"KEY_ENTER"}},{"button":"down","target":{"type":"original"}},{"button":"left","target":{"type":"original"}},{"button":"right","target":{"type":"original"}},{"button":"back","target":{"type":"original"}},{"button":"volume_up","target":{"type":"original"}},{"button":"volume_down","target":{"type":"original"}},{"button":"menu","target":{"type":"original"}},{"button":"live","target":{"type":"original"}}]}"#;
    let Request::Apply { mapping, .. } = decode_request(body.as_bytes()).unwrap() else {
        panic!()
    };
    assert_eq!(
        mapping.get(ButtonId::Up),
        MappingTarget::Key(
            atvv_bridge::button_mapping::LOGICAL_KEYS
                .iter()
                .find(|k| k.symbol == "KEY_ENTER")
                .copied()
                .unwrap()
        )
    );
}

#[test]
fn rejects_trailing_unknown_duplicate_and_oversize_input() {
    assert_eq!(
        decode_request(format!("{}x", envelope("inspect")).as_bytes()),
        Err(ProtocolError::InvalidRequest)
    );
    assert_eq!(
        decode_request(
            br#"{"operation":"inspect","protocol_version":1,"catalog_version":1,"extra":true}"#
        ),
        Err(ProtocolError::InvalidRequest)
    );
    assert_eq!(decode_request(br#"{"operation":"inspect","protocol_version":1,"protocol_version":1,"catalog_version":1}"#), Err(ProtocolError::InvalidRequest));
    assert_eq!(
        decode_request(&vec![b' '; 64 * 1024 + 1]),
        Err(ProtocolError::InvalidRequest)
    );
}
