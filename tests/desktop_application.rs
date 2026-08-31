use std::{
    collections::VecDeque,
    io,
    time::{Duration, SystemTime},
};

use atvv_bridge::{
    ActionableFailureKind, AtvvProfile, AtvvProfileReadiness, BatteryStatus, CaptureStatus,
    ConfigRecoveryDebounce, DesktopApplication, DesktopShell, DesktopStatus, InProcessVoiceBridge,
    IntegrationStage, OperationalEvent, RecentWavHandoff, RecoveryStatus, RemoteStatus,
    StartupError, StatusWindowChrome, VoiceBridge, WavHandoffActivity,
};

#[derive(Default)]
struct FakeBridge {
    starts: usize,
    statuses: VecDeque<DesktopStatus>,
}

impl VoiceBridge for FakeBridge {
    fn start(&mut self) -> io::Result<()> {
        self.starts += 1;
        Ok(())
    }

    fn take_latest_status(&mut self) -> Option<DesktopStatus> {
        let latest = self.statuses.pop_back();
        self.statuses.clear();
        latest
    }
}

#[derive(Default)]
struct FakeDesktopShell {
    windows_created: usize,
    window_chrome: Option<StatusWindowChrome>,
    windows_presented: usize,
    windows_hidden: usize,
    close_confirmations: usize,
    quits: usize,
    tray_available: bool,
    statuses: Vec<DesktopStatus>,
}

impl DesktopShell for FakeDesktopShell {
    fn create_status_window(&mut self, chrome: StatusWindowChrome) {
        self.windows_created += 1;
        self.window_chrome = Some(chrome);
    }

    fn present_status_window(&mut self) {
        self.windows_presented += 1;
    }

    fn display_status(&mut self, status: &DesktopStatus) {
        self.statuses.push(status.clone());
    }

    fn tray_available(&self) -> bool {
        self.tray_available
    }

    fn hide_status_window(&mut self) {
        self.windows_hidden += 1;
    }

    fn confirm_close_quits_bridge(&mut self) {
        self.close_confirmations += 1;
    }

    fn quit(&mut self) {
        self.quits += 1;
    }
}

#[test]
fn refresh_displays_only_the_latest_complete_status_snapshot() {
    let waiting = DesktopStatus::default();
    let ready = waiting
        .clone()
        .transitioned_by(&OperationalEvent::RemoteReady {
            at: SystemTime::UNIX_EPOCH,
            address: "AA:BB:CC:DD:EE:FF".into(),
            profile: AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
        });
    let active = ready
        .clone()
        .transitioned_by(&OperationalEvent::CaptureStarted {
            at: SystemTime::UNIX_EPOCH,
            stream_id: 3,
        });
    let bridge = FakeBridge {
        statuses: VecDeque::from([waiting, ready, active.clone()]),
        ..FakeBridge::default()
    };
    let mut application = DesktopApplication::new(bridge);
    let mut shell = FakeDesktopShell::default();

    application
        .activate(&mut shell)
        .expect("desktop activation should start the bridge");
    application.refresh_status(&mut shell);

    assert_eq!(shell.statuses, [active]);
}

#[test]
fn repeated_activation_reuses_the_bridge_and_status_window() {
    let mut application = DesktopApplication::new(FakeBridge::default());
    let mut shell = FakeDesktopShell::default();

    application
        .activate(&mut shell)
        .expect("the first desktop activation should start the bridge");
    application
        .activate(&mut shell)
        .expect("the second desktop activation should reuse the running application");

    assert_eq!(application.bridge().starts, 1);
    assert_eq!(shell.windows_created, 1);
    assert_eq!(shell.windows_presented, 2);
}

#[test]
fn status_window_exposes_a_visible_close_action() {
    let mut application = DesktopApplication::new(FakeBridge::default());
    let mut shell = FakeDesktopShell::default();

    application
        .activate(&mut shell)
        .expect("desktop activation should create the status window");

    assert_eq!(shell.window_chrome, Some(StatusWindowChrome::CloseAction));
}

#[test]
fn visible_close_action_hides_the_window_when_a_tray_is_available() {
    let mut application = DesktopApplication::new(FakeBridge::default());
    let mut shell = FakeDesktopShell {
        tray_available: true,
        ..FakeDesktopShell::default()
    };

    application.close_requested(&mut shell);

    assert_eq!(shell.windows_hidden, 1);
    assert_eq!(shell.close_confirmations, 0);
    assert_eq!(shell.quits, 0);
}

#[test]
fn visible_close_action_without_a_tray_requires_confirmation() {
    let mut application = DesktopApplication::new(FakeBridge::default());
    let mut shell = FakeDesktopShell::default();

    application.close_requested(&mut shell);
    application.close_confirmed(false, &mut shell);

    assert_eq!(shell.windows_hidden, 0);
    assert_eq!(shell.close_confirmations, 1);
    assert_eq!(shell.quits, 0);
}

#[test]
fn confirmed_no_tray_close_stops_the_application() {
    let mut application = DesktopApplication::new(FakeBridge::default());
    let mut shell = FakeDesktopShell::default();

    application.close_confirmed(true, &mut shell);
    assert_eq!(shell.quits, 1);
}

#[test]
fn local_configuration_failure_is_actionable_without_hiding_the_desktop() {
    let error =
        StartupError::MissingExplicitConfig("/home/test/.config/atvv-bridge/config.toml".into());
    let failure = DesktopStatus::from_startup_error(&error);
    let bridge = FakeBridge {
        statuses: VecDeque::from([failure.clone()]),
        ..FakeBridge::default()
    };
    let mut application = DesktopApplication::new(bridge);
    let mut shell = FakeDesktopShell::default();

    application
        .activate(&mut shell)
        .expect("local failures should not prevent desktop activation");
    application.refresh_status(&mut shell);

    assert_eq!(shell.windows_created, 1);
    assert_eq!(shell.windows_presented, 1);
    assert_eq!(shell.statuses.as_slice(), std::slice::from_ref(&failure));
    let actionable = failure
        .actionable_failure
        .expect("the local failure should be actionable");
    assert_eq!(actionable.kind, ActionableFailureKind::Configuration);
    assert_eq!(actionable.summary, "Configuration needs attention");
    assert!(actionable.diagnostics.contains("config.toml"));
}

#[test]
fn production_bridge_keeps_the_window_available_when_configuration_is_missing() {
    let config_directory = tempfile::tempdir().expect("a temporary config directory should exist");
    let bridge = InProcessVoiceBridge::new(atvv_bridge::ConfigSelection::Explicit(
        config_directory.path().join("missing.toml"),
    ));
    let mut application = DesktopApplication::new(bridge);
    let mut shell = FakeDesktopShell::default();

    application
        .activate(&mut shell)
        .expect("a local configuration failure should stay inside Bridge Status");

    assert_eq!(shell.windows_created, 1);
    assert_eq!(shell.windows_presented, 1);
}

#[test]
fn local_storage_failure_has_a_concise_summary_and_detailed_diagnostics() {
    let error = StartupError::PrepareWavDir {
        path: "/tmp/atvv-bridge/capture-secret.wav".into(),
        source: io::Error::new(
            io::ErrorKind::PermissionDenied,
            "controlled storage failure",
        ),
    };

    let status = DesktopStatus::from_startup_error(&error);
    let actionable = status.actionable_failure.unwrap();

    assert_eq!(actionable.kind, ActionableFailureKind::LocalStorage);
    assert_eq!(actionable.summary, "Local storage is unavailable");
    assert!(!actionable.summary.contains("/tmp/"));
    assert!(actionable.diagnostics.contains("capture-secret.wav"));
}

#[test]
fn wav_storage_failure_is_an_actionable_local_failure() {
    let status = DesktopStatus::default().transitioned_by(&OperationalEvent::WavHandoffFailed {
        at: SystemTime::UNIX_EPOCH,
        address: "AA:BB:CC:DD:EE:FF".into(),
        duration: Duration::from_secs(1),
        audio_bytes: 120,
        stage: IntegrationStage::WavCreation,
        error: "could not create /tmp/atvv-bridge/capture-secret.wav".into(),
        retained_wav: None,
    });

    let actionable = status.actionable_failure.unwrap();
    assert_eq!(actionable.kind, ActionableFailureKind::LocalStorage);
    assert_eq!(actionable.summary, "Local storage is unavailable");
    assert!(!actionable.summary.contains("/tmp/"));
    assert!(actionable.diagnostics.contains("capture-secret.wav"));
}

#[test]
fn configuration_events_are_debounced_for_three_hundred_milliseconds() {
    let started_at = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    let mut debounce = ConfigRecoveryDebounce::default();

    debounce.record_event(started_at);
    debounce.record_event(started_at + Duration::from_millis(200));

    assert!(!debounce.take_retry_due(started_at + Duration::from_millis(499)));
    assert!(debounce.take_retry_due(started_at + Duration::from_millis(500)));
    assert!(!debounce.take_retry_due(started_at + Duration::from_secs(1)));
}

#[test]
fn valid_reinitialization_clears_the_actionable_local_failure() {
    let failed = DesktopStatus::from_startup_error(&StartupError::MissingExplicitConfig(
        "/home/test/.config/atvv-bridge/config.toml".into(),
    ));
    assert!(failed.actionable_failure.is_some());

    let recovered = failed.transitioned_by(&OperationalEvent::DaemonStarted {
        at: SystemTime::UNIX_EPOCH,
        max_duration_secs: 60,
        wav_dir: "/tmp/atvv-bridge".into(),
        keep_wav: false,
    });

    assert_eq!(recovered, DesktopStatus::default());
}

#[test]
fn failed_wav_handoff_is_history_and_does_not_make_the_remote_unready() {
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    let status = DesktopStatus::default()
        .transitioned_by(&OperationalEvent::RemoteReady {
            at,
            address: "AA:BB:CC:DD:EE:FF".into(),
            profile: AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
        })
        .transitioned_by(&OperationalEvent::CaptureStarted { at, stream_id: 7 })
        .transitioned_by(&OperationalEvent::WavHandoffStarted { at })
        .transitioned_by(&OperationalEvent::WavHandoffFailed {
            at,
            address: "AA:BB:CC:DD:EE:FF".into(),
            duration: Duration::from_secs(1),
            audio_bytes: 120,
            stage: IntegrationStage::Transcription,
            error: "voxtype failed".into(),
            retained_wav: None,
        })
        .transitioned_by(&OperationalEvent::CaptureStarted { at, stream_id: 8 })
        .transitioned_by(&OperationalEvent::WavHandoffStarted { at });

    assert_eq!(
        status,
        DesktopStatus {
            remote: RemoteStatus::Connected {
                address: "AA:BB:CC:DD:EE:FF".into(),
            },
            profile: AtvvProfileReadiness::Ready {
                profile: AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            capture: CaptureStatus::Idle,
            wav_handoff: WavHandoffActivity::Active,
            recent_wav_handoff: RecentWavHandoff::Failed {
                stage: IntegrationStage::Transcription,
                error: "voxtype failed".into(),
            },
            recovery: RecoveryStatus::Idle,
            battery: BatteryStatus::Unknown,
            actionable_failure: None,
        }
    );
}

#[test]
fn connected_remote_is_distinct_from_profile_readiness() {
    let status = DesktopStatus::default().transitioned_by(&OperationalEvent::RemoteConnected {
        at: SystemTime::UNIX_EPOCH,
        address: "AA:BB:CC:DD:EE:FF".into(),
    });

    assert!(matches!(status.remote, RemoteStatus::Connected { .. }));
    assert_eq!(status.profile, AtvvProfileReadiness::Waiting);
}

#[test]
fn capture_returns_to_idle_when_no_wav_handoff_is_needed() {
    let at = SystemTime::UNIX_EPOCH;
    let status = DesktopStatus::default()
        .transitioned_by(&OperationalEvent::CaptureStarted { at, stream_id: 7 })
        .transitioned_by(&OperationalEvent::CaptureStopped { at });

    assert_eq!(status.capture, CaptureStatus::Idle);
    assert_eq!(status.recent_wav_handoff, RecentWavHandoff::NoOutcome);
}

#[test]
fn retry_status_shows_the_next_attempt_and_failure_summary() {
    let at = SystemTime::UNIX_EPOCH;
    let next_attempt_at = at + Duration::from_secs(4);
    let retrying =
        DesktopStatus::default().transitioned_by(&OperationalEvent::AtvvRemoteRetryScheduled {
            at,
            next_attempt_at,
            failure: "ATVV capability response timed out".into(),
        });

    assert_eq!(
        retrying.recovery,
        RecoveryStatus::Retrying {
            next_attempt_at,
            failure: "ATVV capability response timed out".into(),
        }
    );

    let connected = retrying.transitioned_by(&OperationalEvent::RemoteConnected {
        at: next_attempt_at,
        address: "AA:BB:CC:DD:EE:FF".into(),
    });
    assert_eq!(connected.recovery, RecoveryStatus::Idle);
}

#[test]
fn proven_unsupported_profile_is_displayed_without_recovery() {
    let status =
        DesktopStatus::default().transitioned_by(&OperationalEvent::AtvvProfileUnsupported {
            at: SystemTime::UNIX_EPOCH,
            address: "AA:BB:CC:DD:EE:FF".into(),
            reason: atvv_bridge::ProfileError::UnsupportedFrameShape,
        });

    assert!(matches!(status.remote, RemoteStatus::Connected { .. }));
    assert_eq!(
        status.profile,
        AtvvProfileReadiness::Unsupported {
            reason: atvv_bridge::ProfileError::UnsupportedFrameShape,
        }
    );
    assert_eq!(status.recovery, RecoveryStatus::Idle);
}

#[test]
fn battery_becomes_unknown_immediately_after_disconnection() {
    let connected = DesktopStatus::default()
        .transitioned_by(&OperationalEvent::RemoteConnected {
            at: SystemTime::UNIX_EPOCH,
            address: "AA:BB:CC:DD:EE:FF".into(),
        })
        .transitioned_by(&OperationalEvent::BatteryUpdated {
            at: SystemTime::UNIX_EPOCH,
            percentage: atvv_bridge::BatteryPercentage::new(87),
        });
    assert_eq!(
        connected.battery,
        BatteryStatus::Percentage(atvv_bridge::BatteryPercentage::new(87).unwrap())
    );

    let disconnected = connected.transitioned_by(&OperationalEvent::WaitingForRemote {
        at: SystemTime::UNIX_EPOCH,
    });
    assert_eq!(disconnected.battery, BatteryStatus::Unknown);
}

#[test]
fn profile_recovery_preserves_connected_atvv_remote_and_current_battery() {
    let at = SystemTime::UNIX_EPOCH;
    let connected = DesktopStatus::default()
        .transitioned_by(&OperationalEvent::RemoteConnected {
            at,
            address: "AA:BB:CC:DD:EE:FF".into(),
        })
        .transitioned_by(&OperationalEvent::BatteryUpdated {
            at,
            percentage: atvv_bridge::BatteryPercentage::new(87),
        });
    let retrying = connected.transitioned_by(&OperationalEvent::AtvvRemoteRetryScheduled {
        at,
        next_attempt_at: at + Duration::from_secs(2),
        failure: "ATVV capability response timed out".into(),
    });

    assert!(matches!(retrying.remote, RemoteStatus::Connected { .. }));
    assert_eq!(
        retrying.battery,
        BatteryStatus::Percentage(atvv_bridge::BatteryPercentage::new(87).unwrap())
    );
    assert!(matches!(retrying.recovery, RecoveryStatus::Retrying { .. }));
}

#[test]
fn battery_percentage_rejects_values_above_one_hundred() {
    assert_eq!(atvv_bridge::BatteryPercentage::new(100).unwrap().get(), 100);
    assert_eq!(atvv_bridge::BatteryPercentage::new(101), None);
}
