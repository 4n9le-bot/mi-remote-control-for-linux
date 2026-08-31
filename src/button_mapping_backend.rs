use std::{
    collections::VecDeque,
    ffi::OsString,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use crate::{
    button_mapping::Mapping,
    helper_protocol::{
        DecodedResponse, MAX_REQUEST_BYTES, Request, decode_response, encode_request,
    },
};

pub const BUTTON_MAPPING_HELPER_PATH: &str = "/usr/libexec/atvv-bridge/atvv-button-mapping-helper";
pub const PKEXEC_PATH: &str = "/usr/bin/pkexec";
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendOperation {
    Inspect,
    Apply {
        expected_revision: String,
        mapping: Mapping,
    },
    Reset,
}

impl BackendOperation {
    fn request(&self) -> Request {
        match self {
            Self::Inspect => Request::Inspect,
            Self::Apply {
                expected_revision,
                mapping,
            } => Request::Apply {
                expected_revision: expected_revision.clone(),
                mapping: mapping.clone(),
            },
            Self::Reset => Request::Reset,
        }
    }

    fn argument(&self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Apply { .. } => "apply",
            Self::Reset => "reset",
        }
    }

    fn privileged(&self) -> bool {
        !matches!(self, Self::Inspect)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendStartError {
    AlreadyBusy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendFailure {
    AuthorizationNotGranted,
    AuthorizationUnavailable,
    HelperFailed,
    InternalError,
}

pub type BackendResult = Result<DecodedResponse, BackendFailure>;

pub trait ButtonMappingBackend {
    fn start(&mut self, operation: BackendOperation) -> Result<(), BackendStartError>;
    fn try_take_result(&mut self) -> Option<BackendResult>;
}

pub struct ProcessButtonMappingBackend {
    helper: PathBuf,
    pkexec: PathBuf,
    timeout: Duration,
    environment: Vec<(OsString, OsString)>,
    active: Option<ActiveOperation>,
}

struct ActiveOperation {
    receiver: Receiver<ProcessCompletion>,
    may_recover: bool,
}

struct ProcessCompletion {
    result: BackendResult,
    inspect_afterward: bool,
}

impl Default for ProcessButtonMappingBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessButtonMappingBackend {
    pub fn new() -> Self {
        Self {
            helper: PathBuf::from(BUTTON_MAPPING_HELPER_PATH),
            pkexec: PathBuf::from(PKEXEC_PATH),
            timeout: PROCESS_TIMEOUT,
            environment: Vec::new(),
            active: None,
        }
    }

    #[doc(hidden)]
    pub fn with_processes(
        helper: PathBuf,
        pkexec: PathBuf,
        timeout: Duration,
        environment: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Self {
        Self {
            helper,
            pkexec,
            timeout,
            environment: environment.into_iter().collect(),
            active: None,
        }
    }

    fn launch(&self, operation: BackendOperation, may_recover: bool) -> ActiveOperation {
        let (sender, receiver) = mpsc::sync_channel(1);
        let helper = self.helper.clone();
        let pkexec = self.pkexec.clone();
        let timeout = self.timeout;
        let environment = self.environment.clone();
        let fallback = sender.clone();
        if thread::Builder::new()
            .name("button-mapping".into())
            .spawn(move || {
                let _ = sender.send(run_process(operation, helper, pkexec, timeout, environment));
            })
            .is_err()
        {
            let _ = fallback.send(ProcessCompletion {
                result: Err(BackendFailure::HelperFailed),
                inspect_afterward: false,
            });
        }
        ActiveOperation {
            receiver,
            may_recover,
        }
    }
}

impl ButtonMappingBackend for ProcessButtonMappingBackend {
    fn start(&mut self, operation: BackendOperation) -> Result<(), BackendStartError> {
        if self.active.is_some() {
            return Err(BackendStartError::AlreadyBusy);
        }
        self.active = Some(self.launch(operation, true));
        Ok(())
    }

    fn try_take_result(&mut self) -> Option<BackendResult> {
        let active = self.active.as_ref()?;
        let completion = match active.receiver.try_recv() {
            Ok(completion) => completion,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => ProcessCompletion {
                result: Err(BackendFailure::InternalError),
                inspect_afterward: active.may_recover,
            },
        };
        let should_inspect = completion.inspect_afterward && active.may_recover;
        self.active = if should_inspect {
            Some(self.launch(BackendOperation::Inspect, false))
        } else {
            None
        };
        Some(completion.result)
    }
}

fn run_process(
    operation: BackendOperation,
    helper: PathBuf,
    pkexec: PathBuf,
    timeout: Duration,
    environment: Vec<(OsString, OsString)>,
) -> ProcessCompletion {
    let privileged = operation.privileged();
    let request = match encode_request(&operation.request()) {
        Ok(request) => request,
        Err(_) => return untrustworthy(),
    };
    let mut input = match tempfile::tempfile() {
        Ok(input) => input,
        Err(_) => return failed(BackendFailure::HelperFailed),
    };
    if input.write_all(&request).is_err() || input.seek(SeekFrom::Start(0)).is_err() {
        return failed(BackendFailure::HelperFailed);
    }
    let mut output = match tempfile::tempfile() {
        Ok(output) => output,
        Err(_) => return failed(BackendFailure::HelperFailed),
    };
    let stdout = match output.try_clone() {
        Ok(stdout) => stdout,
        Err(_) => return failed(BackendFailure::HelperFailed),
    };
    let mut command = if privileged {
        let mut command = Command::new(&pkexec);
        command.arg(&helper).arg(operation.argument());
        command
    } else {
        let mut command = Command::new(&helper);
        command.arg(operation.argument());
        command
    };
    command
        .envs(environment)
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::inherit());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            return failed(if privileged {
                BackendFailure::AuthorizationUnavailable
            } else {
                BackendFailure::HelperFailed
            });
        }
    };
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return untrustworthy();
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return untrustworthy();
            }
        }
    };
    interpret_status(status, &operation, &mut output)
}

fn interpret_status(
    status: ExitStatus,
    operation: &BackendOperation,
    output: &mut File,
) -> ProcessCompletion {
    let privileged = operation.privileged();
    if !status.success() {
        return match (privileged, status.code()) {
            (true, Some(126)) => ProcessCompletion {
                result: Err(BackendFailure::AuthorizationNotGranted),
                inspect_afterward: false,
            },
            (true, Some(127)) => ProcessCompletion {
                result: Err(BackendFailure::AuthorizationUnavailable),
                inspect_afterward: false,
            },
            (true, _) => untrustworthy(),
            (false, _) => untrustworthy(),
        };
    }
    let oversized = output
        .metadata()
        .map_or(true, |metadata| metadata.len() > MAX_REQUEST_BYTES as u64);
    if oversized || output.seek(SeekFrom::Start(0)).is_err() {
        return untrustworthy();
    }
    let mut bytes = Vec::new();
    if output.read_to_end(&mut bytes).is_err() {
        return untrustworthy();
    }
    match decode_response(&bytes) {
        Ok(response) if response_matches(operation, &response) => ProcessCompletion {
            result: Ok(response),
            inspect_afterward: false,
        },
        _ => untrustworthy(),
    }
}

fn response_matches(operation: &BackendOperation, response: &DecodedResponse) -> bool {
    matches!(response, DecodedResponse::Error(_))
        || matches!(
            (operation, response),
            (BackendOperation::Inspect, DecodedResponse::Inspect { .. })
                | (BackendOperation::Inspect, DecodedResponse::RecoveryRequired)
                | (
                    BackendOperation::Apply { .. },
                    DecodedResponse::Apply { .. }
                )
                | (BackendOperation::Reset, DecodedResponse::Reset { .. })
        )
}

fn failed(failure: BackendFailure) -> ProcessCompletion {
    ProcessCompletion {
        result: Err(failure),
        inspect_afterward: false,
    }
}

fn untrustworthy() -> ProcessCompletion {
    ProcessCompletion {
        result: Err(BackendFailure::InternalError),
        inspect_afterward: true,
    }
}

pub struct FakeButtonMappingBackend {
    planned: VecDeque<BackendResult>,
    active: Option<BackendResult>,
    started: Vec<BackendOperation>,
}

impl FakeButtonMappingBackend {
    pub fn new(planned: impl IntoIterator<Item = BackendResult>) -> Self {
        Self {
            planned: planned.into_iter().collect(),
            active: None,
            started: Vec::new(),
        }
    }

    pub fn started(&self) -> &[BackendOperation] {
        &self.started
    }
}

impl ButtonMappingBackend for FakeButtonMappingBackend {
    fn start(&mut self, operation: BackendOperation) -> Result<(), BackendStartError> {
        if self.active.is_some() {
            return Err(BackendStartError::AlreadyBusy);
        }
        self.started.push(operation);
        self.active = self
            .planned
            .pop_front()
            .or(Some(Err(BackendFailure::InternalError)));
        Ok(())
    }

    fn try_take_result(&mut self) -> Option<BackendResult> {
        self.active.take()
    }
}
