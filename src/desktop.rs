use std::{
    io,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, SystemTime},
};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::{
    Application, BatteryPercentage, ConfigSelection, IntegrationStage, OperationalEvent,
    StartupError, WavHandoffOutcome, system::SystemBoundaries,
};
use crate::{
    button_mapping::{ButtonId, Mapping, MappingTarget},
    button_mapping_backend::{BackendFailure, BackendOperation, ButtonMappingBackend},
    helper_protocol::{DecodedResponse, StableErrorCode},
};

const CONFIG_RECOVERY_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionableFailureKind {
    Configuration,
    LocalStorage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionableFailure {
    pub kind: ActionableFailureKind,
    pub summary: String,
    pub action: String,
    pub diagnostics: String,
}

#[derive(Debug, Default)]
pub struct ConfigRecoveryDebounce {
    retry_at: Option<std::time::SystemTime>,
}

impl ConfigRecoveryDebounce {
    pub fn record_event(&mut self, at: std::time::SystemTime) {
        self.retry_at = at.checked_add(CONFIG_RECOVERY_DEBOUNCE);
    }

    pub fn take_retry_due(&mut self, now: std::time::SystemTime) -> bool {
        if self.retry_at.is_some_and(|retry_at| now >= retry_at) {
            self.retry_at = None;
            true
        } else {
            false
        }
    }

    fn remaining(&self, now: SystemTime) -> Option<Duration> {
        self.retry_at
            .map(|retry_at| retry_at.duration_since(now).unwrap_or(Duration::ZERO))
    }
}

struct ConfigDirectoryWatcher {
    _watcher: RecommendedWatcher,
    events: mpsc::Receiver<notify::Result<notify::Event>>,
    config_path: PathBuf,
    config_directory: PathBuf,
}

impl ConfigDirectoryWatcher {
    fn new(selection: &ConfigSelection) -> io::Result<Option<Self>> {
        let config_path = match selection {
            ConfigSelection::DefaultPath(path) | ConfigSelection::Explicit(path) => path.clone(),
            ConfigSelection::DefaultsOnly => return Ok(None),
        };
        let config_directory = config_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let config_directory = if config_directory.is_absolute() {
            config_directory
        } else {
            std::env::current_dir()?.join(config_directory)
        };
        let config_path = if config_path.is_absolute() {
            config_path
        } else {
            std::env::current_dir()?.join(config_path)
        };
        let watch_root = nearest_existing_directory(&config_directory)?;
        let (sender, events) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        })
        .map_err(io::Error::other)?;
        watcher
            .watch(&watch_root, RecursiveMode::Recursive)
            .map_err(io::Error::other)?;
        Ok(Some(Self {
            _watcher: watcher,
            events,
            config_path,
            config_directory,
        }))
    }

    fn wait_for_debounced_change(&self) -> io::Result<()> {
        let mut debounce = ConfigRecoveryDebounce::default();
        loop {
            let event = self
                .events
                .recv()
                .map_err(|_| io::Error::other("configuration directory watcher stopped"))?
                .map_err(io::Error::other)?;
            if !self.is_relevant(&event) {
                continue;
            }
            debounce.record_event(SystemTime::now());
            loop {
                let remaining = debounce
                    .remaining(SystemTime::now())
                    .expect("a configuration event set the retry deadline");
                match self.events.recv_timeout(remaining) {
                    Ok(Ok(event)) if self.is_relevant(&event) => {
                        debounce.record_event(SystemTime::now());
                    }
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => return Err(io::Error::other(error)),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if debounce.take_retry_due(SystemTime::now()) {
                            return Ok(());
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(io::Error::other("configuration directory watcher stopped"));
                    }
                }
            }
        }
    }

    fn is_relevant(&self, event: &notify::Event) -> bool {
        event.paths.iter().any(|path| {
            path == &self.config_path
                || path == &self.config_directory
                || path.parent() == Some(self.config_directory.as_path())
        })
    }
}

fn nearest_existing_directory(path: &Path) -> io::Result<PathBuf> {
    path.ancestors()
        .find(|candidate| candidate.is_dir())
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("configuration directory has no existing ancestor"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteStatus {
    Waiting,
    Connected { address: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtvvProfileReadiness {
    Waiting,
    Ready { profile: crate::AtvvProfile },
    Unsupported { reason: crate::ProfileError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureStatus {
    Idle,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavHandoffActivity {
    Idle,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStatus {
    Idle,
    Retrying {
        next_attempt_at: std::time::SystemTime,
        failure: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Unknown,
    Percentage(BatteryPercentage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecentWavHandoff {
    NoOutcome,
    Succeeded {
        outcome: WavHandoffOutcome,
    },
    Failed {
        stage: IntegrationStage,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopStatus {
    pub remote: RemoteStatus,
    pub profile: AtvvProfileReadiness,
    pub capture: CaptureStatus,
    pub wav_handoff: WavHandoffActivity,
    pub recent_wav_handoff: RecentWavHandoff,
    pub recovery: RecoveryStatus,
    pub battery: BatteryStatus,
    pub actionable_failure: Option<ActionableFailure>,
}

impl Default for DesktopStatus {
    fn default() -> Self {
        Self {
            remote: RemoteStatus::Waiting,
            profile: AtvvProfileReadiness::Waiting,
            capture: CaptureStatus::Idle,
            wav_handoff: WavHandoffActivity::Idle,
            recent_wav_handoff: RecentWavHandoff::NoOutcome,
            recovery: RecoveryStatus::Idle,
            battery: BatteryStatus::Unknown,
            actionable_failure: None,
        }
    }
}

impl DesktopStatus {
    fn local_storage_failure(diagnostics: impl Into<String>) -> ActionableFailure {
        ActionableFailure {
            kind: ActionableFailureKind::LocalStorage,
            summary: "Local storage is unavailable".into(),
            action: "Check the WAV directory; the next Capture will retry.".into(),
            diagnostics: diagnostics.into(),
        }
    }

    pub fn from_startup_error(error: &StartupError) -> Self {
        let (kind, summary, action) = match error {
            StartupError::ReadConfig { .. }
            | StartupError::MissingExplicitConfig(_)
            | StartupError::ParseConfig { .. }
            | StartupError::InvalidMaxDuration(_) => (
                ActionableFailureKind::Configuration,
                "Configuration needs attention",
                "Save a valid replacement configuration to retry.",
            ),
            StartupError::PrepareWavDir { .. } => (
                ActionableFailureKind::LocalStorage,
                "Local storage is unavailable",
                "Check the WAV directory, then save the configuration to retry.",
            ),
        };
        Self {
            actionable_failure: Some(ActionableFailure {
                kind,
                summary: summary.into(),
                action: action.into(),
                diagnostics: error.to_string(),
            }),
            ..Self::default()
        }
    }

    fn from_config_watcher_error(error: &io::Error) -> Self {
        Self {
            actionable_failure: Some(ActionableFailure {
                kind: ActionableFailureKind::Configuration,
                summary: "Configuration recovery is unavailable".into(),
                action: "Restart the application after correcting the configuration.".into(),
                diagnostics: format!("could not watch the configuration directory: {error}"),
            }),
            ..Self::default()
        }
    }

    pub fn transitioned_by(mut self, event: &OperationalEvent) -> Self {
        match event {
            OperationalEvent::DaemonStarted { .. } => self = Self::default(),
            OperationalEvent::WaitingForRemote { .. } | OperationalEvent::DaemonStopped { .. } => {
                self.remote = RemoteStatus::Waiting;
                self.profile = AtvvProfileReadiness::Waiting;
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recovery = RecoveryStatus::Idle;
                self.battery = BatteryStatus::Unknown;
            }
            OperationalEvent::RemoteConnected { address, .. } => {
                self.remote = RemoteStatus::Connected {
                    address: address.clone(),
                };
                self.profile = AtvvProfileReadiness::Waiting;
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recovery = RecoveryStatus::Idle;
                self.battery = BatteryStatus::Unknown;
            }
            OperationalEvent::RemoteReady {
                address, profile, ..
            } => {
                self.remote = RemoteStatus::Connected {
                    address: address.clone(),
                };
                self.profile = AtvvProfileReadiness::Ready { profile: *profile };
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recovery = RecoveryStatus::Idle;
            }
            OperationalEvent::AtvvProfileUnsupported {
                address, reason, ..
            } => {
                self.remote = RemoteStatus::Connected {
                    address: address.clone(),
                };
                self.profile = AtvvProfileReadiness::Unsupported { reason: *reason };
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recovery = RecoveryStatus::Idle;
            }
            OperationalEvent::CaptureStarted { .. } => {
                self.capture = CaptureStatus::Active;
            }
            OperationalEvent::CaptureStopped { .. } => {
                self.capture = CaptureStatus::Idle;
            }
            OperationalEvent::WavHandoffStarted { .. } => {
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Active;
            }
            OperationalEvent::WavHandoffFailed { stage, error, .. } => {
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recent_wav_handoff = RecentWavHandoff::Failed {
                    stage: *stage,
                    error: error.clone(),
                };
                if matches!(
                    stage,
                    IntegrationStage::WavCreation | IntegrationStage::WavCleanup
                ) {
                    self.actionable_failure = Some(Self::local_storage_failure(error));
                }
            }
            OperationalEvent::WavHandoffSucceeded { outcome, .. } => {
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recent_wav_handoff = RecentWavHandoff::Succeeded { outcome: *outcome };
                if self
                    .actionable_failure
                    .as_ref()
                    .is_some_and(|failure| failure.kind == ActionableFailureKind::LocalStorage)
                {
                    self.actionable_failure = None;
                }
            }
            OperationalEvent::AtvvRemoteRetryScheduled {
                next_attempt_at,
                failure,
                ..
            } => {
                self.profile = AtvvProfileReadiness::Waiting;
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recovery = RecoveryStatus::Retrying {
                    next_attempt_at: *next_attempt_at,
                    failure: failure.clone(),
                };
            }
            OperationalEvent::BatteryUpdated { percentage, .. } => {
                self.battery = percentage
                    .map(BatteryStatus::Percentage)
                    .unwrap_or(BatteryStatus::Unknown);
            }
            OperationalEvent::CaptureCompleted { .. }
            | OperationalEvent::ControlNotificationIgnored { .. }
            | OperationalEvent::AudioNotificationIgnored { .. }
            | OperationalEvent::DecoderSynchronized { .. } => {}
        }
        self
    }
}

#[derive(Debug, Clone)]
pub struct LatestDesktopStatus {
    sender: async_channel::Sender<DesktopStatus>,
    receiver: async_channel::Receiver<DesktopStatus>,
}

impl Default for LatestDesktopStatus {
    fn default() -> Self {
        let (sender, receiver) = async_channel::bounded(1);
        Self { sender, receiver }
    }
}

impl LatestDesktopStatus {
    pub fn publish(&self, status: DesktopStatus) {
        if let Err(async_channel::TrySendError::Full(status)) = self.sender.try_send(status) {
            let _ = self.receiver.try_recv();
            let _ = self.sender.try_send(status);
        }
    }

    pub fn take_latest(&self) -> Option<DesktopStatus> {
        self.receiver.try_recv().ok()
    }
}

/// The desktop-facing lifecycle of the ATVV Voice Bridge.
pub trait VoiceBridge {
    fn start(&mut self) -> io::Result<()>;
    fn take_latest_status(&mut self) -> Option<DesktopStatus>;
}

/// The desktop operations the application needs, independent of GTK widgets.
pub trait DesktopShell {
    fn create_status_window_with_close_action(&mut self);
    fn present_status_window(&mut self);
    fn display_status(&mut self, status: &DesktopStatus);
    fn tray_available(&self) -> bool;
    fn hide_status_window(&mut self);
    fn confirm_close_quits_bridge(&mut self);
    fn quit(&mut self);
    fn render_button_mapping(&mut self, _presentation: &ButtonMappingPresentation) {}
    fn perform_button_mapping_effect(&mut self, _effect: ButtonMappingEffect) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ButtonMappingState {
    Unloaded,
    Inspecting,
    Ready {
        installed: Mapping,
        draft: Mapping,
        revision: String,
    },
    Applying,
    Resetting,
    Conflict {
        installed: Mapping,
        draft: Mapping,
        revision: String,
    },
    RecoveryRequired,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonMappingPresentation {
    pub state: ButtonMappingState,
    pub dirty: bool,
    pub can_apply: bool,
    pub can_reset: bool,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonMappingEffect {
    Open,
    Render,
    AuthorizationRequired,
    ConfirmReset,
    ConfirmReload,
    Hide,
    ConfirmQuit,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ButtonMappingEvent {
    Open,
    Edit(ButtonId, MappingTarget),
    Apply,
    Reset,
    ConfirmReset,
    Cancel,
    Reload,
    ConfirmReload,
    Retry,
    AuthorizationCancelled,
}

pub struct ButtonMappingController<B> {
    backend: B,
    state: ButtonMappingState,
    notice: Option<String>,
    reset_confirmation: bool,
    reload_confirmation: bool,
    pending_mapping: Option<Mapping>,
    pending_installed: Option<Mapping>,
    pending_revision: Option<String>,
    preserved_draft: Option<Mapping>,
}

impl<B: ButtonMappingBackend> ButtonMappingController<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            state: ButtonMappingState::Unloaded,
            notice: None,
            reset_confirmation: false,
            reload_confirmation: false,
            pending_mapping: None,
            pending_installed: None,
            pending_revision: None,
            preserved_draft: None,
        }
    }
    pub fn state(&self) -> &ButtonMappingState {
        &self.state
    }
    pub fn backend(&self) -> &B {
        &self.backend
    }
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
    pub fn presentation(&self) -> ButtonMappingPresentation {
        let (dirty, can_apply) = match &self.state {
            ButtonMappingState::Ready {
                installed, draft, ..
            } => (installed != draft, installed != draft),
            ButtonMappingState::Conflict { .. } => (true, false),
            _ => (false, false),
        };
        ButtonMappingPresentation {
            state: self.state.clone(),
            dirty,
            can_apply,
            can_reset: !matches!(
                self.state,
                ButtonMappingState::Inspecting
                    | ButtonMappingState::Applying
                    | ButtonMappingState::Resetting
                    | ButtonMappingState::Unavailable
            ),
            notice: self.notice.clone(),
        }
    }
    pub fn dispatch(&mut self, event: ButtonMappingEvent) -> Option<ButtonMappingEffect> {
        self.notice = None;
        if matches!(
            self.state,
            ButtonMappingState::Applying | ButtonMappingState::Resetting
        ) && !matches!(event, ButtonMappingEvent::AuthorizationCancelled)
        {
            return None;
        }
        match event {
            ButtonMappingEvent::Open | ButtonMappingEvent::Retry
                if matches!(
                    self.state,
                    ButtonMappingState::Unloaded | ButtonMappingState::Unavailable
                ) =>
            {
                self.backend.start(BackendOperation::Inspect).ok()?;
                self.state = ButtonMappingState::Inspecting;
                Some(ButtonMappingEffect::Render)
            }
            ButtonMappingEvent::Open => Some(ButtonMappingEffect::Render),
            ButtonMappingEvent::Edit(id, target) => {
                if let ButtonMappingState::Ready {
                    installed,
                    mut draft,
                    revision,
                } = self.state.clone()
                {
                    draft = Mapping::from_entries(
                        draft
                            .iter()
                            .map(|(b, t)| (b, if b == id { target } else { t })),
                    )
                    .ok()?;
                    self.state = ButtonMappingState::Ready {
                        installed,
                        draft,
                        revision,
                    };
                    Some(ButtonMappingEffect::Render)
                } else {
                    None
                }
            }
            ButtonMappingEvent::Apply => {
                if let ButtonMappingState::Ready {
                    installed,
                    draft,
                    revision,
                } = &self.state
                {
                    if installed == draft {
                        return None;
                    }
                    self.pending_mapping = Some(draft.clone());
                    self.pending_installed = Some(installed.clone());
                    self.pending_revision = Some(revision.clone());
                    self.backend
                        .start(BackendOperation::Apply {
                            expected_revision: revision.clone(),
                            mapping: draft.clone(),
                        })
                        .ok()?;
                    self.state = ButtonMappingState::Applying;
                    Some(ButtonMappingEffect::AuthorizationRequired)
                } else {
                    None
                }
            }
            ButtonMappingEvent::Reset => {
                if matches!(self.state, ButtonMappingState::RecoveryRequired) {
                    self.backend.start(BackendOperation::Reset).ok()?;
                    self.state = ButtonMappingState::Resetting;
                    Some(ButtonMappingEffect::AuthorizationRequired)
                } else if matches!(self.state, ButtonMappingState::Ready { .. }) {
                    self.reset_confirmation = true;
                    Some(ButtonMappingEffect::ConfirmReset)
                } else {
                    None
                }
            }
            ButtonMappingEvent::ConfirmReset if self.reset_confirmation => {
                self.reset_confirmation = false;
                if let ButtonMappingState::Ready {
                    installed,
                    draft,
                    revision,
                } = &self.state
                {
                    self.pending_installed = Some(installed.clone());
                    self.pending_mapping = Some(draft.clone());
                    self.pending_revision = Some(revision.clone());
                }
                self.backend.start(BackendOperation::Reset).ok()?;
                self.state = ButtonMappingState::Resetting;
                Some(ButtonMappingEffect::AuthorizationRequired)
            }
            ButtonMappingEvent::Reload
                if matches!(self.state, ButtonMappingState::Conflict { .. }) =>
            {
                self.reload_confirmation = true;
                Some(ButtonMappingEffect::ConfirmReload)
            }
            ButtonMappingEvent::ConfirmReload if self.reload_confirmation => {
                self.reload_confirmation = false;
                self.backend.start(BackendOperation::Inspect).ok()?;
                self.state = ButtonMappingState::Inspecting;
                Some(ButtonMappingEffect::Render)
            }
            ButtonMappingEvent::AuthorizationCancelled => {
                self.restore_pending_or_unavailable();
                self.notice =
                    Some("Authorization was cancelled; staged edits were preserved.".into());
                Some(ButtonMappingEffect::Render)
            }
            ButtonMappingEvent::Cancel => {
                self.reset_confirmation = false;
                self.reload_confirmation = false;
                self.notice = Some("Operation cancelled; staged edits were preserved.".into());
                Some(ButtonMappingEffect::Render)
            }
            _ => None,
        }
    }
    pub fn poll(&mut self) -> bool {
        let Some(result) = self.backend.try_take_result() else {
            return false;
        };
        match result {
            Ok(DecodedResponse::Inspect { revision, mapping }) => {
                let draft = self
                    .preserved_draft
                    .take()
                    .unwrap_or_else(|| mapping.clone());
                self.state = ButtonMappingState::Ready {
                    installed: mapping,
                    draft,
                    revision,
                };
            }
            Ok(DecodedResponse::Apply { revision }) => {
                if let ButtonMappingState::Applying = self.state {
                    let mapping = self
                        .pending_mapping
                        .take()
                        .unwrap_or_else(Mapping::defaults);
                    self.pending_installed = None;
                    self.pending_revision = None;
                    self.notice = Some(format!(
                        "Mapping written (revision {revision}); reconnect the remote to activate it."
                    ));
                    self.state = ButtonMappingState::Ready {
                        installed: mapping.clone(),
                        draft: mapping,
                        revision,
                    };
                }
            }
            Ok(DecodedResponse::Reset { revision }) => {
                self.pending_mapping = None;
                self.pending_installed = None;
                self.notice = Some(format!(
                    "Defaults restored (revision {revision}); reconnect the remote to activate them."
                ));
                let mapping = Mapping::defaults();
                self.state = ButtonMappingState::Ready {
                    installed: mapping.clone(),
                    draft: mapping,
                    revision,
                };
            }
            Ok(DecodedResponse::RecoveryRequired) => {
                self.state = ButtonMappingState::RecoveryRequired
            }
            Ok(DecodedResponse::Error(code)) => self.handle_code(code),
            Err(error) => self.handle_failure(error),
        }
        true
    }
    fn handle_code(&mut self, code: StableErrorCode) {
        match code {
            StableErrorCode::RevisionConflict => {
                if let ButtonMappingState::Applying = self.state {
                    let installed = self
                        .pending_installed
                        .take()
                        .unwrap_or_else(Mapping::defaults);
                    let draft = self
                        .pending_mapping
                        .take()
                        .unwrap_or_else(Mapping::defaults);
                    self.pending_revision = None;
                    self.state = ButtonMappingState::Conflict {
                        installed,
                        draft,
                        revision: String::new(),
                    };
                }
            }
            StableErrorCode::InconsistentState | StableErrorCode::RollbackFailed => {
                self.preserved_draft = self.pending_mapping.take();
                self.pending_installed = None;
                self.pending_revision = None;
                self.state = ButtonMappingState::RecoveryRequired
            }
            StableErrorCode::UnsupportedSystem | StableErrorCode::UnsupportedCatalog => {
                self.state = ButtonMappingState::Unavailable
            }
            _ => {
                self.notice = Some(format!(
                    "Button Mapping operation failed: {}",
                    code.as_str()
                ));
                self.restore_pending_or_unavailable();
            }
        }
    }
    fn handle_failure(&mut self, error: BackendFailure) {
        self.notice = Some(match error {
            BackendFailure::AuthorizationNotGranted => {
                "Authorization was cancelled; staged edits were preserved.".into()
            }
            BackendFailure::AuthorizationUnavailable => "Authorization is unavailable.".into(),
            _ => "Button Mapping operation failed; staged edits were preserved.".into(),
        });
        if matches!(self.state, ButtonMappingState::Inspecting) {
            self.state = ButtonMappingState::Unavailable;
        } else if matches!(
            self.state,
            ButtonMappingState::Applying | ButtonMappingState::Resetting
        ) {
            self.restore_pending_or_unavailable();
        }
    }

    fn restore_pending_or_unavailable(&mut self) {
        if let (Some(installed), Some(draft), Some(revision)) = (
            self.pending_installed.take(),
            self.pending_mapping.take(),
            self.pending_revision.take(),
        ) {
            self.preserved_draft = Some(draft.clone());
            self.state = ButtonMappingState::Ready {
                installed,
                draft,
                revision,
            };
        } else {
            self.state = ButtonMappingState::Unavailable;
        }
    }
}

/// A single desktop application that owns one ATVV Voice Bridge and status window.
pub struct DesktopApplication<B, M = crate::button_mapping_backend::ProcessButtonMappingBackend> {
    bridge: B,
    button_mapping: ButtonMappingController<M>,
    started: bool,
}

impl<B> DesktopApplication<B, crate::button_mapping_backend::ProcessButtonMappingBackend>
where
    B: VoiceBridge,
{
    pub fn new(bridge: B) -> Self {
        Self::new_with_button_mapping(
            bridge,
            crate::button_mapping_backend::ProcessButtonMappingBackend::new(),
        )
    }
}

impl<B, M> DesktopApplication<B, M>
where
    B: VoiceBridge,
    M: ButtonMappingBackend,
{
    pub fn new_with_button_mapping(bridge: B, button_mapping_backend: M) -> Self {
        Self {
            bridge,
            button_mapping: ButtonMappingController::new(button_mapping_backend),
            started: false,
        }
    }

    pub fn activate(&mut self, shell: &mut impl DesktopShell) -> io::Result<()> {
        if !self.started {
            self.bridge.start()?;
            self.started = true;
            shell.create_status_window_with_close_action();
        }
        shell.present_status_window();
        Ok(())
    }

    pub fn bridge(&self) -> &B {
        &self.bridge
    }

    pub fn button_mapping(&self) -> &ButtonMappingController<M> {
        &self.button_mapping
    }

    pub fn button_mapping_event(
        &mut self,
        event: ButtonMappingEvent,
        shell: &mut impl DesktopShell,
    ) {
        if let Some(effect) = self.button_mapping.dispatch(event) {
            shell.perform_button_mapping_effect(effect);
        }
        shell.render_button_mapping(&self.button_mapping.presentation());
    }

    pub fn refresh_button_mapping(&mut self, shell: &mut impl DesktopShell) {
        if self.button_mapping.poll() {
            shell.render_button_mapping(&self.button_mapping.presentation());
        }
    }

    pub fn refresh_status(&mut self, shell: &mut impl DesktopShell) {
        if let Some(status) = self.bridge.take_latest_status() {
            shell.display_status(&status);
        }
    }

    pub fn close_requested(&mut self, shell: &mut impl DesktopShell) {
        if matches!(
            self.button_mapping.state(),
            ButtonMappingState::Applying | ButtonMappingState::Resetting
        ) {
            shell.perform_button_mapping_effect(ButtonMappingEffect::Render);
            return;
        }
        if shell.tray_available() {
            shell.hide_status_window();
            shell.perform_button_mapping_effect(ButtonMappingEffect::Hide);
        } else {
            shell.confirm_close_quits_bridge();
            shell.perform_button_mapping_effect(ButtonMappingEffect::ConfirmQuit);
        }
    }

    pub fn close_confirmed(&mut self, confirmed: bool, shell: &mut impl DesktopShell) {
        if confirmed
            && !matches!(
                self.button_mapping.state(),
                ButtonMappingState::Applying | ButtonMappingState::Resetting
            )
        {
            shell.quit();
            shell.perform_button_mapping_effect(ButtonMappingEffect::Quit);
        }
    }

    pub fn quit_requested(&mut self, shell: &mut impl DesktopShell) {
        shell.quit();
    }
}

/// Starts the production bridge on its own in-process thread.
pub struct InProcessVoiceBridge {
    selection: Option<ConfigSelection>,
    latest_status: LatestDesktopStatus,
}

impl InProcessVoiceBridge {
    pub fn new(selection: ConfigSelection) -> Self {
        Self {
            selection: Some(selection),
            latest_status: LatestDesktopStatus::default(),
        }
    }
}

impl VoiceBridge for InProcessVoiceBridge {
    fn start(&mut self) -> io::Result<()> {
        let selection = self.selection.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::AlreadyExists, "ATVV Voice Bridge started")
        })?;
        let latest_status = self.latest_status.clone();
        thread::Builder::new()
            .name("atvv-voice-bridge".into())
            .spawn(move || {
                run_with_config_recovery(selection, latest_status);
            })?;
        Ok(())
    }

    fn take_latest_status(&mut self) -> Option<DesktopStatus> {
        self.latest_status.take_latest()
    }
}

fn run_with_config_recovery(selection: ConfigSelection, latest_status: LatestDesktopStatus) {
    let watcher = match ConfigDirectoryWatcher::new(&selection) {
        Ok(watcher) => watcher,
        Err(error) => {
            latest_status.publish(DesktopStatus::from_config_watcher_error(&error));
            return;
        }
    };
    loop {
        let mut boundaries = SystemBoundaries::with_status_updates(latest_status.clone());
        match Application::start(selection.clone(), &mut boundaries) {
            Ok(application) => {
                let _ = application.run();
                return;
            }
            Err(error) => latest_status.publish(DesktopStatus::from_startup_error(&error)),
        }

        let Some(watcher) = watcher.as_ref() else {
            return;
        };
        if let Err(error) = watcher.wait_for_debounced_change() {
            latest_status.publish(DesktopStatus::from_config_watcher_error(&error));
            return;
        }
    }
}
