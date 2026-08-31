use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, SystemTime},
};

use atvv_bridge::{
    AtvvProfile, AtvvProfileReadiness, BatteryStatus, CaptureStatus, DesktopStatus,
    IntegrationStage, LatestDesktopStatus, OperationalEvent, OperationalEvents, RecentWavHandoff,
    RecoveryStatus, RemoteStatus, Storage, WavHandoffActivity, system::SystemBoundaries,
};

#[test]
fn completed_wavs_are_private_and_use_distinct_paths() {
    let directory = tempfile::tempdir().expect("a temporary WAV directory should be available");
    let paths = (0..32)
        .map(|index| {
            SystemBoundaries::default()
                .create_private_wav(directory.path(), index.to_string().as_bytes())
                .expect("a WAV should survive fresh boundary instances")
        })
        .collect::<Vec<_>>();

    let mut distinct = paths.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), paths.len());
    for (index, path) in paths.into_iter().enumerate() {
        assert_eq!(fs::read(&path).unwrap(), index.to_string().as_bytes());
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn operational_events_publish_complete_status_snapshots() {
    let latest_status = LatestDesktopStatus::default();
    let mut boundaries = SystemBoundaries::with_status_updates(latest_status.clone());
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(10);

    boundaries.emit(OperationalEvent::RemoteReady {
        at,
        address: "AA:BB:CC:DD:EE:FF".into(),
        profile: AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
    });
    boundaries.emit(OperationalEvent::WavHandoffFailed {
        at,
        address: "AA:BB:CC:DD:EE:FF".into(),
        duration: Duration::from_secs(1),
        audio_bytes: 120,
        stage: IntegrationStage::Transcription,
        error: "voxtype failed".into(),
        retained_wav: None,
    });

    assert_eq!(
        latest_status.take_latest(),
        Some(DesktopStatus {
            remote: RemoteStatus::Connected {
                address: "AA:BB:CC:DD:EE:FF".into(),
            },
            profile: AtvvProfileReadiness::Ready {
                profile: AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            capture: CaptureStatus::Idle,
            wav_handoff: WavHandoffActivity::Idle,
            recent_wav_handoff: RecentWavHandoff::Failed {
                stage: IntegrationStage::Transcription,
                error: "voxtype failed".into(),
            },
            recovery: RecoveryStatus::Idle,
            battery: BatteryStatus::Unknown,
            actionable_failure: None,
        })
    );
}

#[test]
fn status_publication_coalesces_to_the_latest_snapshot() {
    let latest_status = LatestDesktopStatus::default();
    let mut boundaries = SystemBoundaries::with_status_updates(latest_status.clone());

    for stream_id in 0..=u8::MAX {
        boundaries.emit(OperationalEvent::CaptureStarted {
            at: SystemTime::UNIX_EPOCH,
            stream_id,
        });
    }

    assert_eq!(
        latest_status.take_latest().map(|status| status.capture),
        Some(CaptureStatus::Active)
    );
    assert_eq!(latest_status.take_latest(), None);
}
