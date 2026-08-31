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
    fn create_status_window(&mut self);
    fn present_status_window(&mut self);
    fn display_status(&mut self, status: &DesktopStatus);
    fn tray_available(&self) -> bool;
    fn hide_status_window(&mut self);
    fn confirm_close_quits_bridge(&mut self);
    fn quit(&mut self);
}

/// A single desktop application that owns one ATVV Voice Bridge and status window.
pub struct DesktopApplication<B> {
    bridge: B,
    started: bool,
}

impl<B> DesktopApplication<B>
where
    B: VoiceBridge,
{
    pub fn new(bridge: B) -> Self {
        Self {
            bridge,
            started: false,
        }
    }

    pub fn activate(&mut self, shell: &mut impl DesktopShell) -> io::Result<()> {
        if !self.started {
            self.bridge.start()?;
            self.started = true;
            shell.create_status_window();
        }
        shell.present_status_window();
        Ok(())
    }

    pub fn bridge(&self) -> &B {
        &self.bridge
    }

    pub fn refresh_status(&mut self, shell: &mut impl DesktopShell) {
        if let Some(status) = self.bridge.take_latest_status() {
            shell.display_status(&status);
        }
    }

    pub fn close_requested(&mut self, shell: &mut impl DesktopShell) {
        if shell.tray_available() {
            shell.hide_status_window();
        } else {
            shell.confirm_close_quits_bridge();
        }
    }

    pub fn close_confirmed(&mut self, confirmed: bool, shell: &mut impl DesktopShell) {
        if confirmed {
            shell.quit();
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
