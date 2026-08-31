use atvv_bridge::button_mapping::{ButtonId, MappingTarget};
use atvv_bridge::helper_protocol::{
    ProtocolError, Request, Response, StableErrorCode, decode_request, encode_response,
};

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
                .find(|k| k.symbol() == "KEY_ENTER")
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

#[test]
fn rejects_missing_mistyped_tagged_unknown_and_duplicate_mapping_values() {
    for body in [
        r#"{"operation":"inspect","protocol_version":1}"#,
        r#"{"operation":"inspect","protocol_version":"1","catalog_version":1}"#,
        r#"{"operation":"wat","protocol_version":1,"catalog_version":1}"#,
        r#"{"operation":"apply","protocol_version":1,"catalog_version":1,"expected_revision":"r","mapping":[{"button":"voice","target":{"type":"disabled"}}]}"#,
        r#"{"operation":"apply","protocol_version":1,"catalog_version":1,"expected_revision":"r","mapping":[{"button":"power","target":{"type":"key","key":"KEY_NOT_REAL"}}]}"#,
        r#"{"operation":"apply","protocol_version":1,"catalog_version":1,"expected_revision":"r","mapping":[{"button":"power","target":{"type":"disabled"}},{"button":"power","target":{"type":"original"}}]}"#,
    ] {
        assert!(matches!(
            decode_request(body.as_bytes()),
            Err(ProtocolError::InvalidRequest | ProtocolError::InvalidMapping)
        ));
    }
}

#[test]
fn golden_success_envelopes_cover_every_operation() {
    let inspect = String::from_utf8(
        encode_response(&Response::inspect_success(
            "r1",
            &atvv_bridge::button_mapping::Mapping::defaults(),
        ))
        .unwrap(),
    )
    .unwrap();
    assert!(inspect.starts_with(r#"{"protocol_version":1,"catalog_version":1,"ok":true,"result":{"kind":"success","operation":"inspect","revision":"r1","mapping":[{"button":"power","target":{"type":"disabled"}}"#));
    assert_eq!(
        String::from_utf8(encode_response(&Response::apply_success("r2")).unwrap()).unwrap(),
        r#"{"protocol_version":1,"catalog_version":1,"ok":true,"result":{"kind":"success","operation":"apply","revision":"r2","mapping":null}}"#
    );
    assert_eq!(
        String::from_utf8(encode_response(&Response::reset_success("r3")).unwrap()).unwrap(),
        r#"{"protocol_version":1,"catalog_version":1,"ok":true,"result":{"kind":"success","operation":"reset","revision":"r3","mapping":null}}"#
    );
}

#[test]
fn golden_error_envelopes_cover_every_stable_code() {
    let codes = [
        StableErrorCode::InvalidRequest,
        StableErrorCode::UnsupportedProtocol,
        StableErrorCode::UnsupportedCatalog,
        StableErrorCode::InvalidMapping,
        StableErrorCode::RevisionConflict,
        StableErrorCode::Busy,
        StableErrorCode::InconsistentState,
        StableErrorCode::UnsupportedSystem,
        StableErrorCode::OperationFailed,
        StableErrorCode::RollbackFailed,
        StableErrorCode::InternalError,
    ];
    for code in codes {
        assert_eq!(
            String::from_utf8(encode_response(&Response::error(code)).unwrap()).unwrap(),
            format!(
                r#"{{"protocol_version":1,"catalog_version":1,"ok":false,"result":{{"kind":"error","code":"{}"}}}}"#,
                code.as_str()
            )
        );
    }
}
