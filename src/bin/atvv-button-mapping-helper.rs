use std::io::{Read, Write};

use atvv_bridge::{
    helper_protocol::{
        MAX_REQUEST_BYTES, ProtocolError, Request, Response, StableErrorCode, decode_request,
        encode_response,
    },
    hwdb_mapping::{HwdbMappingError, HwdbMappingHelper, InspectOutcome, MappingRevision},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("button mapping helper failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), &'static str> {
    let argument = single_argument();
    let mut input = Vec::new();
    std::io::stdin()
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| "could not read request")?;
    let response = match decode_request(&input) {
        Ok(request) if argument_matches(argument.as_deref(), &request) => execute(request),
        Ok(_) => Response::error(StableErrorCode::InvalidRequest),
        Err(error) => Response::error(protocol_error_code(error)),
    };
    let encoded = encode_response(&response).map_err(|_| "could not encode response")?;
    std::io::stdout()
        .write_all(&encoded)
        .map_err(|_| "could not write response")
}

fn single_argument() -> Option<String> {
    let mut arguments = std::env::args().skip(1);
    let argument = arguments.next()?;
    arguments.next().is_none().then_some(argument)
}

fn argument_matches(argument: Option<&str>, request: &Request) -> bool {
    matches!(
        (argument, request),
        (Some("inspect"), Request::Inspect)
            | (Some("apply"), Request::Apply { .. })
            | (Some("reset"), Request::Reset)
    )
}

fn execute(request: Request) -> Response {
    if !matches!(request, Request::Inspect) && unsafe { libc::geteuid() } != 0 {
        return Response::error(StableErrorCode::OperationFailed);
    }
    let mut helper = HwdbMappingHelper::production();
    match request {
        Request::Inspect => match helper.inspect() {
            Ok(InspectOutcome::Ready { mapping, revision }) => {
                Response::inspect_success(revision.as_str(), &mapping)
            }
            Ok(InspectOutcome::RecoveryRequired) => Response::recovery_required(),
            Err(error) => Response::error(hwdb_error_code(error)),
        },
        Request::Apply {
            expected_revision,
            mapping,
        } => match helper.apply(&MappingRevision::from_opaque(expected_revision), &mapping) {
            Ok(revision) => Response::apply_success(revision.as_str()),
            Err(error) => Response::error(hwdb_error_code(error)),
        },
        Request::Reset => match helper.reset() {
            Ok(revision) => Response::reset_success(revision.as_str()),
            Err(error) => Response::error(hwdb_error_code(error)),
        },
    }
}

fn protocol_error_code(error: ProtocolError) -> StableErrorCode {
    match error {
        ProtocolError::InvalidRequest => StableErrorCode::InvalidRequest,
        ProtocolError::UnsupportedProtocol => StableErrorCode::UnsupportedProtocol,
        ProtocolError::UnsupportedCatalog => StableErrorCode::UnsupportedCatalog,
        ProtocolError::InvalidMapping => StableErrorCode::InvalidMapping,
    }
}

fn hwdb_error_code(error: HwdbMappingError) -> StableErrorCode {
    match error {
        HwdbMappingError::Busy => StableErrorCode::Busy,
        HwdbMappingError::RevisionConflict => StableErrorCode::RevisionConflict,
        HwdbMappingError::InconsistentState => StableErrorCode::InconsistentState,
        HwdbMappingError::UnsupportedSystem => StableErrorCode::UnsupportedSystem,
        HwdbMappingError::OperationFailed => StableErrorCode::OperationFailed,
        HwdbMappingError::RollbackFailed => StableErrorCode::RollbackFailed,
    }
}
