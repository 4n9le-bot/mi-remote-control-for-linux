use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use serde::Deserialize;
use thiserror::Error;

pub mod system;

pub(crate) const ATVV_SERVICE_UUID: &str = "ab5e0001-5a21-4f05-bc7d-af01f617b664";
pub(crate) const ATVV_CHARACTERISTIC_UUIDS: [&str; 3] = [
    "ab5e0002-5a21-4f05-bc7d-af01f617b664",
    "ab5e0003-5a21-4f05-bc7d-af01f617b664",
    "ab5e0004-5a21-4f05-bc7d-af01f617b664",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtvvVersion {
    V1_0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtvvInteractionModel {
    HoldToTalk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtvvCodec {
    ImaDviAdpcm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtvvProfile {
    version: AtvvVersion,
    interaction_model: AtvvInteractionModel,
    codec: AtvvCodec,
    sample_rate_hz: u32,
    frame_bytes: usize,
    headerless_frames: bool,
}

impl AtvvProfile {
    pub const XIAOMI_V1_HTT_16KHZ_120: Self = Self {
        version: AtvvVersion::V1_0,
        interaction_model: AtvvInteractionModel::HoldToTalk,
        codec: AtvvCodec::ImaDviAdpcm,
        sample_rate_hz: 16_000,
        frame_bytes: 120,
        headerless_frames: true,
    };

    pub fn version(self) -> AtvvVersion {
        self.version
    }

    pub fn interaction_model(self) -> AtvvInteractionModel {
        self.interaction_model
    }

    pub fn codec(self) -> AtvvCodec {
        self.codec
    }

    pub fn sample_rate_hz(self) -> u32 {
        self.sample_rate_hz
    }

    pub fn frame_bytes(self) -> usize {
        self.frame_bytes
    }

    pub fn frames_are_headerless(self) -> bool {
        self.headerless_frames
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    #[error("malformed ATVV capability response")]
    MalformedCapabilities,
    #[error("unsupported ATVV protocol version")]
    UnsupportedVersion,
    #[error("unsupported ATVV codec")]
    UnsupportedCodec,
    #[error("unsupported ATVV interaction model")]
    UnsupportedInteractionModel,
    #[error("unsupported ATVV audio frame shape")]
    UnsupportedFrameShape,
}

pub fn select_profile(capabilities: &[u8]) -> Result<AtvvProfile, ProfileError> {
    if capabilities.len() != 9 || capabilities.first() != Some(&0x0B) {
        return Err(ProfileError::MalformedCapabilities);
    }
    if capabilities[1..3] != [0x01, 0x00] {
        return Err(ProfileError::UnsupportedVersion);
    }
    if capabilities[3] != 0x02 {
        return Err(ProfileError::UnsupportedCodec);
    }
    if capabilities[4] != 0x03 {
        return Err(ProfileError::UnsupportedInteractionModel);
    }
    if capabilities[5..7] != [0x00, 0x78] {
        return Err(ProfileError::UnsupportedFrameShape);
    }
    if capabilities[7..9] != [0x00, 0x00] {
        return Err(ProfileError::MalformedCapabilities);
    }
    Ok(AtvvProfile::XIAOMI_V1_HTT_16KHZ_120)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BluezSnapshot {
    pub devices: Vec<Device>,
    pub services: Vec<GattService>,
    pub characteristics: Vec<GattCharacteristic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub path: String,
    pub address: String,
    pub connected: bool,
    pub services_resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GattService {
    pub path: String,
    pub device_path: String,
    pub uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GattCharacteristic {
    pub path: String,
    pub service_path: String,
    pub uuid: String,
}

pub trait AtvvGatt {
    fn snapshot(&mut self) -> io::Result<BluezSnapshot>;
    fn watch_connection(&mut self, device_path: &str) -> io::Result<()>;
    fn subscribe(&mut self, characteristic_path: &str) -> io::Result<()>;
    fn get_capabilities(&mut self, tx_path: &str, control_path: &str) -> io::Result<Vec<u8>>;
    fn wait_for_change_until(
        &mut self,
        deadline: Option<SystemTime>,
    ) -> io::Result<Option<AtvvChange>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtvvChange {
    TopologyChanged,
    ConnectionChanged,
    ControlNotification(ControlNotification),
    AudioNotification(Vec<u8>),
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlNotification {
    pub received_at: SystemTime,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedRemote {
    pub address: String,
    pub profile: AtvvProfile,
    endpoints: AtvvEndpoints,
}

#[derive(Debug, Error)]
pub enum AttachmentError {
    #[error("could not inspect BlueZ objects: {0}")]
    Inspect(#[source] io::Error),
    #[error("could not subscribe to ATVV control notifications: {0}")]
    SubscribeControl(#[source] io::Error),
    #[error("could not monitor ATVV Remote connection state: {0}")]
    WatchConnection(#[source] io::Error),
    #[error("could not subscribe to ATVV audio notifications: {0}")]
    SubscribeAudio(#[source] io::Error),
    #[error("ATVV capability exchange failed: {0}")]
    CapabilityExchange(#[source] io::Error),
    #[error("ATVV capability negotiation failed: {0}")]
    Profile(#[from] ProfileError),
    #[error("could not wait for an online ATVV Remote: {0}")]
    Wait(#[source] io::Error),
}

pub fn attach_online_remote(
    gatt: &mut impl AtvvGatt,
) -> Result<Option<AttachedRemote>, AttachmentError> {
    let snapshot = gatt.snapshot().map_err(AttachmentError::Inspect)?;
    let Some(remote) = snapshot.online_remote() else {
        return Ok(None);
    };
    gatt.watch_connection(&remote.device_path)
        .map_err(AttachmentError::WatchConnection)?;
    gatt.subscribe(&remote.endpoints.control_path)
        .map_err(AttachmentError::SubscribeControl)?;
    gatt.subscribe(&remote.endpoints.audio_path)
        .map_err(AttachmentError::SubscribeAudio)?;
    let capabilities = gatt
        .get_capabilities(&remote.endpoints.tx_path, &remote.endpoints.control_path)
        .map_err(AttachmentError::CapabilityExchange)?;
    let profile = select_profile(&capabilities)?;
    Ok(Some(AttachedRemote {
        address: remote.address,
        profile,
        endpoints: remote.endpoints,
    }))
}

#[derive(Debug, Default)]
pub struct AttachmentMonitor {
    ready_remote: Option<AttachedIdentity>,
    waiting_reported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttachedIdentity {
    address: String,
    endpoints: AtvvEndpoints,
}

impl AttachmentMonitor {
    pub fn next_event(
        &mut self,
        gatt: &mut impl AtvvGatt,
        deadline: Option<SystemTime>,
    ) -> Result<Option<AtvvEvent>, AttachmentError> {
        loop {
            if let Some(ready) = self.ready_remote.as_ref() {
                let snapshot = gatt.snapshot().map_err(AttachmentError::Inspect)?;
                let remains_online = snapshot.online_remote().is_some_and(|remote| {
                    remote.address == ready.address && remote.endpoints == ready.endpoints
                });
                if remains_online {
                    let Some(change) = gatt
                        .wait_for_change_until(deadline)
                        .map_err(AttachmentError::Wait)?
                    else {
                        return Ok(None);
                    };
                    match change {
                        AtvvChange::ConnectionChanged => {
                            self.ready_remote = None;
                            self.waiting_reported = true;
                            return Ok(Some(AtvvEvent::WaitingForRemote));
                        }
                        AtvvChange::ControlNotification(payload) => {
                            return Ok(Some(AtvvEvent::ControlNotification(payload)));
                        }
                        AtvvChange::AudioNotification(payload) => {
                            return Ok(Some(AtvvEvent::AudioNotification(payload)));
                        }
                        AtvvChange::Stopped => return Ok(Some(AtvvEvent::Stopped)),
                        AtvvChange::TopologyChanged => continue,
                    }
                }
                self.ready_remote = None;
                self.waiting_reported = true;
                return Ok(Some(AtvvEvent::WaitingForRemote));
            }

            match attach_online_remote(gatt)? {
                Some(attached) => {
                    self.ready_remote = Some(AttachedIdentity {
                        address: attached.address.clone(),
                        endpoints: attached.endpoints,
                    });
                    self.waiting_reported = false;
                    return Ok(Some(AtvvEvent::RemoteReady {
                        address: attached.address,
                        profile: attached.profile,
                    }));
                }
                None if !self.waiting_reported => {
                    self.waiting_reported = true;
                    return Ok(Some(AtvvEvent::WaitingForRemote));
                }
                None => {
                    let Some(change) = gatt
                        .wait_for_change_until(deadline)
                        .map_err(AttachmentError::Wait)?
                    else {
                        return Ok(None);
                    };
                    if change == AtvvChange::Stopped {
                        return Ok(Some(AtvvEvent::Stopped));
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AtvvEndpoints {
    tx_path: String,
    audio_path: String,
    control_path: String,
}

struct OnlineRemote {
    address: String,
    device_path: String,
    endpoints: AtvvEndpoints,
}

pub trait BluezClient {
    fn managed_objects(&mut self) -> io::Result<BluezSnapshot>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotReady {
    NoAtvvRemote,
    Disconnected,
    ServicesUnresolved,
    MissingCharacteristics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    Ready {
        address: String,
    },
    NotReady {
        address: Option<String>,
        reason: NotReady,
    },
}

pub fn check_readiness(bluez: &mut impl BluezClient) -> io::Result<Readiness> {
    Ok(bluez.managed_objects()?.readiness())
}

impl BluezSnapshot {
    fn atvv_endpoints(&self, device: &Device) -> Option<AtvvEndpoints> {
        self.services
            .iter()
            .filter(|service| {
                service.device_path == device.path
                    && service.uuid.eq_ignore_ascii_case(ATVV_SERVICE_UUID)
            })
            .find_map(|service| {
                let path_for = |uuid: &str| {
                    self.characteristics
                        .iter()
                        .find(|characteristic| {
                            characteristic.service_path == service.path
                                && characteristic.uuid.eq_ignore_ascii_case(uuid)
                        })
                        .map(|characteristic| characteristic.path.clone())
                };
                Some(AtvvEndpoints {
                    tx_path: path_for(ATVV_CHARACTERISTIC_UUIDS[0])?,
                    audio_path: path_for(ATVV_CHARACTERISTIC_UUIDS[1])?,
                    control_path: path_for(ATVV_CHARACTERISTIC_UUIDS[2])?,
                })
            })
    }

    fn online_remote(&self) -> Option<OnlineRemote> {
        let mut devices: Vec<_> = self
            .devices
            .iter()
            .filter(|device| device.connected && device.services_resolved)
            .collect();
        devices.sort_by(|left, right| left.address.cmp(&right.address));

        devices.into_iter().find_map(|device| {
            Some(OnlineRemote {
                address: device.address.clone(),
                device_path: device.path.clone(),
                endpoints: self.atvv_endpoints(device)?,
            })
        })
    }

    fn readiness(&self) -> Readiness {
        let mut candidates: Vec<_> = self
            .devices
            .iter()
            .filter(|device| {
                self.services.iter().any(|service| {
                    service.device_path == device.path
                        && service.uuid.eq_ignore_ascii_case(ATVV_SERVICE_UUID)
                })
            })
            .collect();
        candidates.sort_by(|left, right| {
            right
                .connected
                .cmp(&left.connected)
                .then_with(|| left.address.cmp(&right.address))
        });
        let Some(device) = candidates.first() else {
            return Readiness::NotReady {
                address: None,
                reason: NotReady::NoAtvvRemote,
            };
        };
        let not_ready = |reason| Readiness::NotReady {
            address: Some(device.address.clone()),
            reason,
        };
        if !device.connected {
            return not_ready(NotReady::Disconnected);
        }
        if !device.services_resolved {
            return not_ready(NotReady::ServicesUnresolved);
        }
        if self.atvv_endpoints(device).is_none() {
            return not_ready(NotReady::MissingCharacteristics);
        }
        Readiness::Ready {
            address: device.address.clone(),
        }
    }
}

const DEFAULT_MAX_DURATION_SECS: u64 = 60;
const DEFAULT_WAV_DIR: &str = "/tmp/atvv-bridge";
const MAX_DURATION_SECS: u64 = 3_600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtvvEvent {
    WaitingForRemote,
    RemoteReady {
        address: String,
        profile: AtvvProfile,
    },
    ControlNotification(ControlNotification),
    AudioNotification(Vec<u8>),
    Stopped,
}

pub trait AtvvTransport {
    fn next_event(&mut self, deadline: Option<SystemTime>) -> io::Result<Option<AtvvEvent>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait ProcessExecutor {
    fn execute(&mut self, command: &Command) -> io::Result<CommandOutput>;
}

pub trait Storage {
    fn read_optional_config(&mut self, path: &Path) -> io::Result<Option<String>>;
    fn prepare_wav_dir(&mut self, path: &Path) -> io::Result<()>;
    fn create_private_wav(&mut self, directory: &Path, contents: &[u8]) -> io::Result<PathBuf>;
    fn remove_file(&mut self, path: &Path) -> io::Result<()>;
}

pub trait Clock {
    fn now(&self) -> SystemTime;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationalEvent {
    DaemonStarted {
        at: SystemTime,
        max_duration_secs: u64,
        wav_dir: PathBuf,
        keep_wav: bool,
    },
    WaitingForRemote {
        at: SystemTime,
    },
    RemoteReady {
        at: SystemTime,
        address: String,
    },
    CaptureCompleted {
        at: SystemTime,
        stream_id: u8,
        samples: usize,
    },
    ControlNotificationIgnored {
        at: SystemTime,
        issue: ControlNotificationIssue,
    },
    AudioNotificationIgnored {
        at: SystemTime,
        issue: AudioNotificationIssue,
    },
    DecoderSynchronized {
        at: SystemTime,
    },
    WavCleanupFailed {
        at: SystemTime,
        path: PathBuf,
    },
    DaemonStopped {
        at: SystemTime,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlNotificationIssue {
    WavHandoffBusy,
    DuplicateStart,
    OutOfOrder,
    Unknown,
    Malformed,
    InvalidSynchronization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioNotificationIssue {
    Malformed {
        expected_bytes: usize,
        actual_bytes: usize,
    },
}

enum ControlMessage {
    Start { stream_id: u8 },
    Stop,
    Synchronize { predictor: i16, step_index: u8 },
    InvalidSynchronization,
    Malformed,
    Unknown,
}

impl ControlMessage {
    fn parse(payload: &[u8]) -> Self {
        match payload {
            [0x04, 0x03, 0x02, stream_id] => Self::Start {
                stream_id: *stream_id,
            },
            [0x00, 0x02] => Self::Stop,
            [0x0A, 0x02, _, _, predictor_high, predictor_low, step_index] => Self::Synchronize {
                predictor: i16::from_be_bytes([*predictor_high, *predictor_low]),
                step_index: *step_index,
            },
            [0x0A, ..] => Self::InvalidSynchronization,
            [0x04, ..] | [0x00, ..] | [] => Self::Malformed,
            _ => Self::Unknown,
        }
    }
}

pub trait OperationalEvents {
    fn emit(&mut self, event: OperationalEvent);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSelection {
    DefaultPath(PathBuf),
    Explicit(PathBuf),
    DefaultsOnly,
}

impl ConfigSelection {
    pub fn resolve(
        explicit: Option<PathBuf>,
        xdg_config_home: Option<&OsStr>,
        home: Option<&OsStr>,
    ) -> Self {
        if let Some(path) = explicit {
            return Self::Explicit(path);
        }
        if let Some(base) = absolute_nonempty_path(xdg_config_home) {
            return Self::DefaultPath(base.join("atvv-bridge/config.toml"));
        }
        if let Some(base) = absolute_nonempty_path(home) {
            return Self::DefaultPath(base.join(".config/atvv-bridge/config.toml"));
        }
        Self::DefaultsOnly
    }
}

fn absolute_nonempty_path(value: Option<&OsStr>) -> Option<PathBuf> {
    let path = PathBuf::from(value?);
    (!path.as_os_str().is_empty() && path.is_absolute()).then_some(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub max_duration_secs: u64,
    pub wav_dir: PathBuf,
    pub keep_wav: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_duration_secs: DEFAULT_MAX_DURATION_SECS,
            wav_dir: DEFAULT_WAV_DIR.into(),
            keep_wav: false,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ConfigFile {
    max_duration_secs: Option<u64>,
    wav_dir: Option<PathBuf>,
    keep_wav: Option<bool>,
}

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("could not read configuration {path}: {source}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("configuration file does not exist: {0}")]
    MissingExplicitConfig(PathBuf),
    #[error("invalid configuration in {path}: {source}")]
    ParseConfig {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("max_duration_secs must be between 1 and {MAX_DURATION_SECS} seconds, got {0}")]
    InvalidMaxDuration(u64),
    #[error("WAV directory {path} is unusable: {source}")]
    PrepareWavDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Error)]
#[error("ATVV transport failed: {0}")]
pub struct RunError(#[source] io::Error);

pub struct Application;

impl Application {
    pub fn start<'a, B>(
        selection: ConfigSelection,
        boundaries: &'a mut B,
    ) -> Result<RunningApplication<'a, B>, StartupError>
    where
        B: AtvvTransport + ProcessExecutor + Storage + Clock + OperationalEvents,
    {
        let config = load_config(selection, boundaries)?;
        boundaries
            .prepare_wav_dir(&config.wav_dir)
            .map_err(|source| StartupError::PrepareWavDir {
                path: config.wav_dir.clone(),
                source,
            })?;
        boundaries.emit(OperationalEvent::DaemonStarted {
            at: boundaries.now(),
            max_duration_secs: config.max_duration_secs,
            wav_dir: config.wav_dir.clone(),
            keep_wav: config.keep_wav,
        });

        Ok(RunningApplication { config, boundaries })
    }
}

pub struct RunningApplication<'a, B> {
    config: Config,
    boundaries: &'a mut B,
}

#[derive(Debug, Default)]
pub struct ImaAdpcmDecoder {
    predictor: i32,
    step_index: i32,
}

impl ImaAdpcmDecoder {
    pub fn reset(&mut self, predictor: i16, step_index: u8) {
        self.predictor = i32::from(predictor);
        self.step_index = i32::from(step_index.min(88));
    }

    pub fn decode(&mut self, encoded: &[u8]) -> Vec<i16> {
        let mut samples = Vec::with_capacity(encoded.len() * 2);
        for byte in encoded {
            samples.push(self.decode_nibble(byte >> 4));
            samples.push(self.decode_nibble(byte & 0x0f));
        }
        samples
    }

    fn decode_nibble(&mut self, nibble: u8) -> i16 {
        const STEP_TABLE: [i32; 89] = [
            7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55,
            60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307,
            337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411,
            1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358,
            5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500,
            20350, 22385, 24623, 27086, 29794, 32767,
        ];
        const INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

        let step = STEP_TABLE[self.step_index as usize];
        let mut difference = step >> 3;
        if nibble & 1 != 0 {
            difference += step >> 2;
        }
        if nibble & 2 != 0 {
            difference += step >> 1;
        }
        if nibble & 4 != 0 {
            difference += step;
        }
        if nibble & 8 != 0 {
            self.predictor -= difference;
        } else {
            self.predictor += difference;
        }
        self.predictor = self.predictor.clamp(i16::MIN as i32, i16::MAX as i32);
        self.step_index = (self.step_index + INDEX_TABLE[nibble as usize]).clamp(0, 88);
        self.predictor as i16
    }
}

struct Capture {
    stream_id: u8,
    profile: AtvvProfile,
    decoder: ImaAdpcmDecoder,
    samples: Vec<i16>,
    started_at: SystemTime,
}

impl<B> RunningApplication<'_, B>
where
    B: AtvvTransport + ProcessExecutor + Storage + Clock + OperationalEvents,
{
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn run(mut self) -> Result<(), RunError> {
        let mut capture: Option<Capture> = None;
        let mut profile: Option<AtvvProfile> = None;
        let mut reject_starts_received_through: Option<SystemTime> = None;
        loop {
            let deadline = capture.as_ref().map(|capture| {
                capture.started_at + Duration::from_secs(self.config.max_duration_secs)
            });
            if deadline.is_some_and(|deadline| self.boundaries.now() >= deadline) {
                reject_starts_received_through = self.process_pending_capture(
                    &mut capture,
                    deadline.expect("Capture has a deadline"),
                );
                continue;
            }
            let Some(event) = self.boundaries.next_event(deadline).map_err(RunError)? else {
                reject_starts_received_through =
                    self.process_pending_capture(&mut capture, self.boundaries.now());
                continue;
            };
            let at = self.boundaries.now();
            let operational_event = match event {
                AtvvEvent::WaitingForRemote => {
                    capture = None;
                    profile = None;
                    OperationalEvent::WaitingForRemote { at }
                }
                AtvvEvent::RemoteReady {
                    address,
                    profile: negotiated_profile,
                } => {
                    profile = Some(negotiated_profile);
                    OperationalEvent::RemoteReady { at, address }
                }
                AtvvEvent::ControlNotification(notification) => {
                    let issue = match ControlMessage::parse(&notification.payload) {
                        ControlMessage::Start { .. } if capture.is_some() => {
                            Some(ControlNotificationIssue::DuplicateStart)
                        }
                        ControlMessage::Start { .. }
                            if reject_starts_received_through
                                .is_some_and(|cutoff| notification.received_at <= cutoff) =>
                        {
                            Some(ControlNotificationIssue::WavHandoffBusy)
                        }
                        ControlMessage::Start { stream_id } if profile.is_some() => {
                            capture = Some(Capture {
                                stream_id,
                                profile: profile.expect("profile checked before Capture"),
                                decoder: ImaAdpcmDecoder::default(),
                                samples: Vec::new(),
                                started_at: notification.received_at,
                            });
                            None
                        }
                        ControlMessage::Stop => {
                            if let Some(completed) = capture.take() {
                                self.process_completed_capture(completed, at);
                                reject_starts_received_through = Some(self.boundaries.now());
                                None
                            } else {
                                Some(ControlNotificationIssue::OutOfOrder)
                            }
                        }
                        ControlMessage::Synchronize {
                            predictor,
                            step_index,
                        } => {
                            if let Some(active) = capture.as_mut() {
                                active.decoder.reset(predictor, step_index);
                                self.boundaries
                                    .emit(OperationalEvent::DecoderSynchronized { at });
                                None
                            } else {
                                Some(ControlNotificationIssue::OutOfOrder)
                            }
                        }
                        ControlMessage::Start { .. } => Some(ControlNotificationIssue::OutOfOrder),
                        ControlMessage::InvalidSynchronization => {
                            Some(ControlNotificationIssue::InvalidSynchronization)
                        }
                        ControlMessage::Malformed => Some(ControlNotificationIssue::Malformed),
                        ControlMessage::Unknown => Some(ControlNotificationIssue::Unknown),
                    };
                    if let Some(issue) = issue {
                        self.boundaries
                            .emit(OperationalEvent::ControlNotificationIgnored { at, issue });
                    }
                    continue;
                }
                AtvvEvent::AudioNotification(frame) => {
                    if let Some(expected_bytes) = profile.map(AtvvProfile::frame_bytes) {
                        if frame.len() != expected_bytes {
                            self.boundaries
                                .emit(OperationalEvent::AudioNotificationIgnored {
                                    at,
                                    issue: AudioNotificationIssue::Malformed {
                                        expected_bytes,
                                        actual_bytes: frame.len(),
                                    },
                                });
                        } else if let Some(active) = capture.as_mut() {
                            active.samples.extend(active.decoder.decode(&frame));
                        }
                    }
                    continue;
                }
                AtvvEvent::Stopped => {
                    self.boundaries.emit(OperationalEvent::DaemonStopped { at });
                    return Ok(());
                }
            };
            self.boundaries.emit(operational_event);
        }
    }

    fn process_pending_capture(
        &mut self,
        capture: &mut Option<Capture>,
        at: SystemTime,
    ) -> Option<SystemTime> {
        let completed = capture.take()?;
        self.process_completed_capture(completed, at);
        Some(self.boundaries.now())
    }

    fn process_completed_capture(&mut self, completed: Capture, at: SystemTime) {
        let samples = completed.samples.len();
        if samples == 0 {
            return;
        }
        self.perform_wav_handoff_and_text_commit(
            completed.profile.sample_rate_hz(),
            &completed.samples,
        );
        self.boundaries.emit(OperationalEvent::CaptureCompleted {
            at,
            stream_id: completed.stream_id,
            samples,
        });
    }

    fn perform_wav_handoff_and_text_commit(&mut self, sample_rate_hz: u32, samples: &[i16]) {
        let wav = pcm16_wav(sample_rate_hz, samples);
        let Ok(path) = self
            .boundaries
            .create_private_wav(&self.config.wav_dir, &wav)
        else {
            return;
        };
        let transcription = Command {
            program: "voxtype".into(),
            args: vec!["transcribe".into(), path.display().to_string()],
        };
        let Ok(output) = self.boundaries.execute(&transcription) else {
            return;
        };
        if output.status != 0 {
            return;
        }
        let Ok(stdout) = String::from_utf8(output.stdout) else {
            return;
        };
        let transcript = stdout.trim();
        if !transcript.is_empty() {
            let commit = Command {
                program: "fcitx5-commit".into(),
                args: vec!["--text".into(), transcript.into()],
            };
            let Ok(output) = self.boundaries.execute(&commit) else {
                return;
            };
            if output.status != 0 {
                return;
            }
        }
        if !self.config.keep_wav && self.boundaries.remove_file(&path).is_err() {
            let at = self.boundaries.now();
            self.boundaries
                .emit(OperationalEvent::WavCleanupFailed { at, path });
        }
    }
}

fn pcm16_wav(sample_rate_hz: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = u32::try_from(samples.len().saturating_mul(2)).unwrap_or(u32::MAX);
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36_u32.saturating_add(data_len)).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate_hz.to_le_bytes());
    wav.extend_from_slice(&sample_rate_hz.saturating_mul(2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

fn load_config<B>(selection: ConfigSelection, storage: &mut B) -> Result<Config, StartupError>
where
    B: Storage,
{
    let (path, required) = match selection {
        ConfigSelection::DefaultPath(path) => (path, false),
        ConfigSelection::Explicit(path) => (path, true),
        ConfigSelection::DefaultsOnly => return Ok(Config::default()),
    };
    let contents =
        storage
            .read_optional_config(&path)
            .map_err(|source| StartupError::ReadConfig {
                path: path.clone(),
                source,
            })?;
    let Some(contents) = contents else {
        return if required {
            Err(StartupError::MissingExplicitConfig(path))
        } else {
            Ok(Config::default())
        };
    };
    let file =
        toml::from_str::<ConfigFile>(&contents).map_err(|source| StartupError::ParseConfig {
            path: path.clone(),
            source,
        })?;
    let defaults = Config::default();
    let config = Config {
        max_duration_secs: file.max_duration_secs.unwrap_or(defaults.max_duration_secs),
        wav_dir: file.wav_dir.unwrap_or(defaults.wav_dir),
        keep_wav: file.keep_wav.unwrap_or(defaults.keep_wav),
    };
    if !(1..=MAX_DURATION_SECS).contains(&config.max_duration_secs) {
        return Err(StartupError::InvalidMaxDuration(config.max_duration_secs));
    }
    Ok(config)
}
