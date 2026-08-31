use std::{io, sync::mpsc, thread};

use crate::{
    Application, BatteryPercentage, ConfigSelection, IntegrationStage, OperationalEvent,
    StartupError, WavHandoffOutcome, system::SystemBoundaries,
};

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
        }
    }
}

impl DesktopStatus {
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
            }
            OperationalEvent::WavHandoffSucceeded { outcome, .. } => {
                self.capture = CaptureStatus::Idle;
                self.wav_handoff = WavHandoffActivity::Idle;
                self.recent_wav_handoff = RecentWavHandoff::Succeeded { outcome: *outcome };
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
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let latest_status = self.latest_status.clone();
        thread::Builder::new()
            .name("atvv-voice-bridge".into())
            .spawn(move || {
                let mut boundaries = SystemBoundaries::with_status_updates(latest_status);
                let result = Application::start(selection, &mut boundaries);
                match result {
                    Ok(application) => {
                        let _ = started_tx.send(Ok(()));
                        let _ = application.run();
                    }
                    Err(error) => {
                        let _ = started_tx.send(Err(startup_error_to_io(error)));
                    }
                }
            })?;
        started_rx
            .recv()
            .map_err(|_| io::Error::other("ATVV Voice Bridge stopped during startup"))?
    }

    fn take_latest_status(&mut self) -> Option<DesktopStatus> {
        self.latest_status.take_latest()
    }
}

fn startup_error_to_io(error: StartupError) -> io::Error {
    io::Error::other(error)
}
