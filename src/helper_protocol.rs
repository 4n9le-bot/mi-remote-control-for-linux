use crate::button_mapping::{ButtonId, CATALOG_VERSION, LOGICAL_KEYS, Mapping, MappingTarget};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Inspect,
    Apply {
        expected_revision: String,
        mapping: Mapping,
    },
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidRequest,
    UnsupportedProtocol,
    UnsupportedCatalog,
    InvalidMapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StableErrorCode {
    InvalidRequest,
    UnsupportedProtocol,
    UnsupportedCatalog,
    InvalidMapping,
    RevisionConflict,
    Busy,
    InconsistentState,
    UnsupportedSystem,
    OperationFailed,
    RollbackFailed,
    InternalError,
}

impl StableErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::UnsupportedProtocol => "unsupported_protocol",
            Self::UnsupportedCatalog => "unsupported_catalog",
            Self::InvalidMapping => "invalid_mapping",
            Self::RevisionConflict => "revision_conflict",
            Self::Busy => "busy",
            Self::InconsistentState => "inconsistent_state",
            Self::UnsupportedSystem => "unsupported_system",
            Self::OperationFailed => "operation_failed",
            Self::RollbackFailed => "rollback_failed",
            Self::InternalError => "internal_error",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", deny_unknown_fields)]
enum WireRequest {
    #[serde(rename = "inspect")]
    Inspect {
        protocol_version: u32,
        catalog_version: u32,
    },
    #[serde(rename = "apply")]
    Apply {
        protocol_version: u32,
        catalog_version: u32,
        expected_revision: String,
        mapping: Vec<WireEntry>,
    },
    #[serde(rename = "reset")]
    Reset {
        protocol_version: u32,
        catalog_version: u32,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireEntry {
    button: String,
    target: WireTarget,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum WireTarget {
    #[serde(rename = "original")]
    Original,
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "key")]
    Key { key: String },
}

pub fn decode_request(input: &[u8]) -> Result<Request, ProtocolError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(ProtocolError::InvalidRequest);
    }
    let mut de = serde_json::Deserializer::from_slice(input);
    let wire = WireRequest::deserialize(&mut de).map_err(|_| ProtocolError::InvalidRequest)?;
    de.end().map_err(|_| ProtocolError::InvalidRequest)?;
    let (protocol, catalog) = match &wire {
        WireRequest::Inspect {
            protocol_version,
            catalog_version,
        }
        | WireRequest::Reset {
            protocol_version,
            catalog_version,
        }
        | WireRequest::Apply {
            protocol_version,
            catalog_version,
            ..
        } => (*protocol_version, *catalog_version),
    };
    if protocol != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedProtocol);
    }
    if catalog != CATALOG_VERSION {
        return Err(ProtocolError::UnsupportedCatalog);
    }
    match wire {
        WireRequest::Inspect { .. } => Ok(Request::Inspect),
        WireRequest::Reset { .. } => Ok(Request::Reset),
        WireRequest::Apply {
            expected_revision,
            mapping,
            ..
        } => Ok(Request::Apply {
            expected_revision,
            mapping: mapping_from_wire(mapping)?,
        }),
    }
}

fn mapping_from_wire(mapping: Vec<WireEntry>) -> Result<Mapping, ProtocolError> {
    let mut entries = Vec::with_capacity(mapping.len());
    for entry in mapping {
        let button =
            ButtonId::try_from(entry.button.as_str()).map_err(|_| ProtocolError::InvalidMapping)?;
        let target = match entry.target {
            WireTarget::Original => MappingTarget::Original,
            WireTarget::Disabled => MappingTarget::Disabled,
            WireTarget::Key { key } => MappingTarget::Key(
                *LOGICAL_KEYS
                    .iter()
                    .find(|candidate| candidate.symbol() == key)
                    .ok_or(ProtocolError::InvalidMapping)?,
            ),
        };
        if entries.iter().any(|(id, _)| *id == button) {
            return Err(ProtocolError::InvalidMapping);
        }
        entries.push((button, target));
    }
    Mapping::from_entries(entries).map_err(|_| ProtocolError::InvalidMapping)
}

#[derive(Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum WireRequestOut<'a> {
    Inspect {
        protocol_version: u32,
        catalog_version: u32,
    },
    Apply {
        protocol_version: u32,
        catalog_version: u32,
        expected_revision: &'a str,
        mapping: Vec<WireEntryOut>,
    },
    Reset {
        protocol_version: u32,
        catalog_version: u32,
    },
}

pub fn encode_request(request: &Request) -> Result<Vec<u8>, serde_json::Error> {
    let wire = match request {
        Request::Inspect => WireRequestOut::Inspect {
            protocol_version: PROTOCOL_VERSION,
            catalog_version: CATALOG_VERSION,
        },
        Request::Apply {
            expected_revision,
            mapping,
        } => WireRequestOut::Apply {
            protocol_version: PROTOCOL_VERSION,
            catalog_version: CATALOG_VERSION,
            expected_revision,
            mapping: mapping
                .iter()
                .map(|(button, target)| WireEntryOut {
                    button: button.as_str(),
                    target: wire_target_out(target),
                })
                .collect(),
        },
        Request::Reset => WireRequestOut::Reset {
            protocol_version: PROTOCOL_VERSION,
            catalog_version: CATALOG_VERSION,
        },
    };
    serde_json::to_vec(&wire)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Response {
    protocol_version: u32,
    catalog_version: u32,
    ok: bool,
    result: ResponseResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ResponseResult {
    Success {
        operation: Operation,
        revision: Option<String>,
        mapping: Option<Vec<WireEntryOut>>,
    },
    Error {
        code: StableErrorCode,
    },
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WireEntryOut {
    button: &'static str,
    target: WireTargetOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Inspect,
    Apply,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireTargetOut {
    Original,
    Disabled,
    Key { key: &'static str },
}

fn wire_target_out(target: MappingTarget) -> WireTargetOut {
    match target {
        MappingTarget::Original => WireTargetOut::Original,
        MappingTarget::Disabled => WireTargetOut::Disabled,
        MappingTarget::Key(key) => WireTargetOut::Key { key: key.symbol() },
    }
}

impl Response {
    pub fn inspect_success(revision: impl Into<String>, mapping: &Mapping) -> Self {
        Self::success(Operation::Inspect, Some(revision.into()), Some(mapping))
    }

    pub fn apply_success(revision: impl Into<String>) -> Self {
        Self::success(Operation::Apply, Some(revision.into()), None)
    }

    pub fn reset_success(revision: impl Into<String>) -> Self {
        Self::success(Operation::Reset, Some(revision.into()), None)
    }

    pub fn error(code: StableErrorCode) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            catalog_version: CATALOG_VERSION,
            ok: false,
            result: ResponseResult::Error { code },
        }
    }

    pub fn recovery_required() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            catalog_version: CATALOG_VERSION,
            ok: false,
            result: ResponseResult::RecoveryRequired,
        }
    }

    fn success(operation: Operation, revision: Option<String>, mapping: Option<&Mapping>) -> Self {
        let mapping = mapping.map(|mapping| {
            mapping
                .iter()
                .map(|(button, target)| WireEntryOut {
                    button: button.as_str(),
                    target: wire_target_out(target),
                })
                .collect()
        });
        Self {
            protocol_version: PROTOCOL_VERSION,
            catalog_version: CATALOG_VERSION,
            ok: true,
            result: ResponseResult::Success {
                operation,
                revision,
                mapping,
            },
        }
    }
}

pub fn encode_response(response: &Response) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(response)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedResponse {
    Inspect { revision: String, mapping: Mapping },
    Apply { revision: String },
    Reset { revision: String },
    Error(StableErrorCode),
    RecoveryRequired,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResponse {
    protocol_version: u32,
    catalog_version: u32,
    ok: bool,
    result: WireResponseResult,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WireResponseResult {
    Success {
        operation: WireOperation,
        revision: Option<String>,
        mapping: Option<Vec<WireEntry>>,
    },
    Error {
        code: StableErrorCode,
    },
    RecoveryRequired,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireOperation {
    Inspect,
    Apply,
    Reset,
}

pub fn decode_response(input: &[u8]) -> Result<DecodedResponse, ProtocolError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(ProtocolError::InvalidRequest);
    }
    let mut de = serde_json::Deserializer::from_slice(input);
    let wire = WireResponse::deserialize(&mut de).map_err(|_| ProtocolError::InvalidRequest)?;
    de.end().map_err(|_| ProtocolError::InvalidRequest)?;
    if wire.protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedProtocol);
    }
    if wire.catalog_version != CATALOG_VERSION {
        return Err(ProtocolError::UnsupportedCatalog);
    }
    match (wire.ok, wire.result) {
        (false, WireResponseResult::Error { code }) => Ok(DecodedResponse::Error(code)),
        (false, WireResponseResult::RecoveryRequired) => Ok(DecodedResponse::RecoveryRequired),
        (
            true,
            WireResponseResult::Success {
                operation: WireOperation::Inspect,
                revision: Some(revision),
                mapping: Some(mapping),
            },
        ) => Ok(DecodedResponse::Inspect {
            revision,
            mapping: mapping_from_wire(mapping)?,
        }),
        (
            true,
            WireResponseResult::Success {
                operation: WireOperation::Apply,
                revision: Some(revision),
                mapping: None,
            },
        ) => Ok(DecodedResponse::Apply { revision }),
        (
            true,
            WireResponseResult::Success {
                operation: WireOperation::Reset,
                revision: Some(revision),
                mapping: None,
            },
        ) => Ok(DecodedResponse::Reset { revision }),
        _ => Err(ProtocolError::InvalidRequest),
    }
}
