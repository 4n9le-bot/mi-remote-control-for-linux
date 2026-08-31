use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use atvv_bridge::{
    button_mapping::Mapping,
    button_mapping_backend::{
        BackendFailure, BackendOperation, BackendStartError, ButtonMappingBackend,
        FakeButtonMappingBackend, ProcessButtonMappingBackend,
    },
    helper_protocol::{DecodedResponse, StableErrorCode},
};

struct FakeProcesses {
    _directory: tempfile::TempDir,
    helper: PathBuf,
    pkexec: PathBuf,
    log: PathBuf,
    marker: PathBuf,
}

impl FakeProcesses {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let helper = directory.path().join("atvv-button-mapping-helper");
        let pkexec = directory.path().join("pkexec");
        let log = directory.path().join("invocations");
        let marker = directory.path().join("marker");
        fs::write(
            &helper,
            r##"#!/bin/sh
printf 'helper:%s\n' "$1" >>"$FAKE_LOG"
case "$FAKE_MODE" in
  malformed-once)
    if [ ! -e "$FAKE_MARKER" ]; then : >"$FAKE_MARKER"; printf 'not json'; exit 0; fi ;;
  abnormal-once)
    if [ ! -e "$FAKE_MARKER" ]; then : >"$FAKE_MARKER"; exit 9; fi ;;
  timeout-once)
    if [ ! -e "$FAKE_MARKER" ]; then : >"$FAKE_MARKER"; sleep 2; exit 0; fi ;;
  wrong-operation-once)
    if [ ! -e "$FAKE_MARKER" ]; then : >"$FAKE_MARKER"; printf '%s' '{"protocol_version":1,"catalog_version":1,"ok":true,"result":{"kind":"success","operation":"apply","revision":"wrong","mapping":null}}'; exit 0; fi ;;
  error)
    printf '%s' '{"protocol_version":1,"catalog_version":1,"ok":false,"result":{"kind":"error","code":"busy"}}'; exit 0 ;;
  recovery)
    printf '%s' '{"protocol_version":1,"catalog_version":1,"ok":false,"result":{"kind":"recovery_required"}}'; exit 0 ;;
esac
case "$1" in
  inspect) printf '%s' '{"protocol_version":1,"catalog_version":1,"ok":true,"result":{"kind":"success","operation":"inspect","revision":"r1","mapping":[{"button":"power","target":{"type":"disabled"}},{"button":"confirm","target":{"type":"original"}},{"button":"up","target":{"type":"original"}},{"button":"down","target":{"type":"original"}},{"button":"left","target":{"type":"original"}},{"button":"right","target":{"type":"original"}},{"button":"back","target":{"type":"original"}},{"button":"volume_up","target":{"type":"original"}},{"button":"volume_down","target":{"type":"original"}},{"button":"menu","target":{"type":"original"}},{"button":"live","target":{"type":"original"}}]}}' ;;
  apply) printf '%s' '{"protocol_version":1,"catalog_version":1,"ok":true,"result":{"kind":"success","operation":"apply","revision":"r2","mapping":null}}' ;;
  reset) printf '%s' '{"protocol_version":1,"catalog_version":1,"ok":true,"result":{"kind":"success","operation":"reset","revision":"r3","mapping":null}}' ;;
esac
"##,
        )
        .unwrap();
        fs::write(
            &pkexec,
            r##"#!/bin/sh
printf 'pkexec:%s:%s\n' "$1" "$2" >>"$FAKE_LOG"
if [ "${FAKE_PKEXEC_CODE:-0}" -ne 0 ]; then exit "$FAKE_PKEXEC_CODE"; fi
exec "$@"
"##,
        )
        .unwrap();
        for executable in [&helper, &pkexec] {
            fs::set_permissions(executable, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self {
            _directory: directory,
            helper,
            pkexec,
            log,
            marker,
        }
    }

    fn backend(
        &self,
        mode: &str,
        pkexec_code: i32,
        timeout: Duration,
    ) -> ProcessButtonMappingBackend {
        ProcessButtonMappingBackend::with_processes(
            self.helper.clone(),
            self.pkexec.clone(),
            timeout,
            [
                ("FAKE_MODE".into(), mode.into()),
                ("FAKE_LOG".into(), self.log.as_os_str().into()),
                ("FAKE_MARKER".into(), self.marker.as_os_str().into()),
                ("FAKE_PKEXEC_CODE".into(), pkexec_code.to_string().into()),
            ],
        )
    }

    fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }
}

fn take_result(backend: &mut dyn ButtonMappingBackend) -> Result<DecodedResponse, BackendFailure> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = backend.try_take_result() {
            return result;
        }
        assert!(Instant::now() < deadline, "backend did not finish");
        thread::yield_now();
    }
}

#[test]
fn inspect_runs_directly_while_mutations_use_pkexec_and_the_helper_path() {
    let processes = FakeProcesses::new();
    let mut backend = processes.backend("success", 0, Duration::from_secs(1));
    backend.start(BackendOperation::Inspect).unwrap();
    assert!(matches!(
        take_result(&mut backend),
        Ok(DecodedResponse::Inspect { .. })
    ));
    backend
        .start(BackendOperation::Apply {
            expected_revision: "r1".into(),
            mapping: Mapping::defaults(),
        })
        .unwrap();
    assert_eq!(
        take_result(&mut backend),
        Ok(DecodedResponse::Apply {
            revision: "r2".into()
        })
    );
    assert_eq!(
        processes.log(),
        format!(
            "helper:inspect\npkexec:{}:apply\nhelper:apply\n",
            processes.helper.display()
        )
    );
}

#[test]
fn valid_helper_errors_remain_trustworthy_results() {
    let processes = FakeProcesses::new();
    let mut backend = processes.backend("error", 0, Duration::from_secs(1));
    backend.start(BackendOperation::Inspect).unwrap();
    assert_eq!(
        take_result(&mut backend),
        Ok(DecodedResponse::Error(StableErrorCode::Busy))
    );
}

#[test]
fn recovery_required_is_a_trustworthy_inspect_result() {
    let processes = FakeProcesses::new();
    let mut backend = processes.backend("recovery", 0, Duration::from_secs(1));
    backend.start(BackendOperation::Inspect).unwrap();
    assert_eq!(
        take_result(&mut backend),
        Ok(DecodedResponse::RecoveryRequired)
    );
}

#[test]
fn malformed_abnormal_and_timed_out_helpers_report_internal_error_then_inspect() {
    for mode in [
        "malformed-once",
        "abnormal-once",
        "timeout-once",
        "wrong-operation-once",
    ] {
        let processes = FakeProcesses::new();
        let mut backend = processes.backend(mode, 0, Duration::from_millis(50));
        backend.start(BackendOperation::Inspect).unwrap();
        assert_eq!(
            take_result(&mut backend),
            Err(BackendFailure::InternalError)
        );
        assert!(matches!(
            take_result(&mut backend),
            Ok(DecodedResponse::Inspect { .. })
        ));
        assert_eq!(processes.log(), "helper:inspect\nhelper:inspect\n");
    }
}

#[test]
fn privileged_abnormal_exit_also_triggers_recovery_inspect() {
    let processes = FakeProcesses::new();
    let mut backend = processes.backend("abnormal-once", 0, Duration::from_millis(50));
    backend
        .start(BackendOperation::Apply {
            expected_revision: "r1".into(),
            mapping: Mapping::defaults(),
        })
        .unwrap();
    assert_eq!(
        take_result(&mut backend),
        Err(BackendFailure::InternalError)
    );
    assert!(matches!(
        take_result(&mut backend),
        Ok(DecodedResponse::Inspect { .. })
    ));
}

#[test]
fn pkexec_exit_codes_have_stable_authorization_results() {
    for (code, expected) in [
        (126, BackendFailure::AuthorizationNotGranted),
        (127, BackendFailure::AuthorizationUnavailable),
    ] {
        let processes = FakeProcesses::new();
        let mut backend = processes.backend("success", code, Duration::from_secs(1));
        backend.start(BackendOperation::Reset).unwrap();
        assert_eq!(take_result(&mut backend), Err(expected));
    }
}

#[test]
fn a_second_operation_is_rejected_instead_of_queued() {
    let processes = FakeProcesses::new();
    let mut backend = processes.backend("timeout-once", 0, Duration::from_millis(50));
    backend.start(BackendOperation::Inspect).unwrap();
    assert_eq!(
        backend.start(BackendOperation::Reset),
        Err(BackendStartError::AlreadyBusy)
    );
}

#[test]
fn deterministic_fake_implements_the_same_start_and_poll_seam() {
    let expected = Ok(DecodedResponse::Reset {
        revision: "r1".into(),
    });
    let mut backend = FakeButtonMappingBackend::new([expected.clone()]);
    backend.start(BackendOperation::Reset).unwrap();
    assert_eq!(
        backend.start(BackendOperation::Inspect),
        Err(BackendStartError::AlreadyBusy)
    );
    assert_eq!(backend.try_take_result(), Some(expected));
    assert_eq!(backend.started(), &[BackendOperation::Reset]);
}
