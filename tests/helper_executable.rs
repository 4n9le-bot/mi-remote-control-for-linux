use std::{
    io::Write,
    process::{Command, Stdio},
};

use atvv_bridge::{
    button_mapping::Mapping,
    helper_protocol::{DecodedResponse, Request, StableErrorCode, decode_response, encode_request},
};

fn invoke(argument: &str, request: &Request) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_atvv-button-mapping-helper"))
        .arg(argument)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&encode_request(request).unwrap())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn inspect_always_returns_at_most_one_trustworthy_response() {
    let output = invoke("inspect", &Request::Inspect);
    assert!(output.status.success());
    assert!(decode_response(&output.stdout).is_ok());
}

#[test]
fn direct_mutation_is_rejected_for_an_unprivileged_caller() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let output = invoke(
        "apply",
        &Request::Apply {
            expected_revision: "sha256:old".into(),
            mapping: Mapping::defaults(),
        },
    );
    assert!(output.status.success());
    assert_eq!(
        decode_response(&output.stdout),
        Ok(DecodedResponse::Error(StableErrorCode::OperationFailed))
    );
}

#[test]
fn argument_and_json_operation_must_agree() {
    let output = invoke("reset", &Request::Inspect);
    assert!(output.status.success());
    assert_eq!(
        decode_response(&output.stdout),
        Ok(DecodedResponse::Error(StableErrorCode::InvalidRequest))
    );
}
