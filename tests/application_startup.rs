use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use atvv_bridge::{
    Application, AtvvEvent, AtvvTransport, Clock, Command, CommandOutput, ConfigSelection,
    OperationalEvent, OperationalEvents, ProcessExecutor, Storage,
};

#[derive(Default)]
struct ControlledBoundaries {
    config: Option<String>,
    prepared_wav_dir: Option<String>,
    prepare_error: Option<io::ErrorKind>,
    atvv_events: VecDeque<AtvvEvent>,
    process_results: VecDeque<io::Result<CommandOutput>>,
    commands: Vec<Command>,
    events: Vec<OperationalEvent>,
    wav: Option<(PathBuf, Vec<u8>)>,
    removed_files: Vec<PathBuf>,
}

impl AtvvTransport for ControlledBoundaries {
    fn next_event(&mut self) -> io::Result<AtvvEvent> {
        self.atvv_events
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "script exhausted"))
    }
}

impl ProcessExecutor for ControlledBoundaries {
    fn execute(&mut self, command: &Command) -> io::Result<CommandOutput> {
        self.commands.push(command.clone());
        self.process_results
            .pop_front()
            .unwrap_or_else(|| Err(io::Error::new(io::ErrorKind::NotFound, "not scripted")))
    }
}

impl Storage for ControlledBoundaries {
    fn read_optional_config(&mut self, _path: &Path) -> io::Result<Option<String>> {
        Ok(self.config.clone())
    }

    fn prepare_wav_dir(&mut self, path: &Path) -> io::Result<()> {
        if let Some(kind) = self.prepare_error {
            return Err(io::Error::new(kind, "controlled storage failure"));
        }
        self.prepared_wav_dir = Some(path.display().to_string());
        Ok(())
    }

    fn create_private_wav(&mut self, _directory: &Path, contents: &[u8]) -> io::Result<PathBuf> {
        let path = PathBuf::from("/tmp/atvv-bridge/capture-test.wav");
        self.wav = Some((path.clone(), contents.to_vec()));
        Ok(path)
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        self.removed_files.push(path.to_owned());
        Ok(())
    }
}

impl Clock for ControlledBoundaries {
    fn now(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH
    }
}

impl OperationalEvents for ControlledBoundaries {
    fn emit(&mut self, event: OperationalEvent) {
        self.events.push(event);
    }
}

#[test]
fn missing_configuration_starts_with_safe_defaults() {
    let mut boundaries = ControlledBoundaries::default();

    let running = Application::start(
        ConfigSelection::DefaultPath("/home/test/.config/atvv-bridge/config.toml".into()),
        &mut boundaries,
    )
    .expect("missing optional configuration should start the daemon");

    assert_eq!(running.config().max_duration_secs, 60);
    assert_eq!(running.config().wav_dir, Path::new("/tmp/atvv-bridge"));
    assert!(!running.config().keep_wav);
    assert_eq!(
        boundaries.prepared_wav_dir.as_deref(),
        Some("/tmp/atvv-bridge")
    );
    assert_eq!(
        boundaries.events,
        vec![OperationalEvent::DaemonStarted {
            at: SystemTime::UNIX_EPOCH,
            max_duration_secs: 60,
            wav_dir: "/tmp/atvv-bridge".into(),
            keep_wav: false,
        }]
    );
}

#[test]
fn valid_configuration_overrides_each_default() {
    let mut boundaries = ControlledBoundaries {
        config: Some(
            "max_duration_secs = 15\nwav_dir = '/var/tmp/voice'\nkeep_wav = true\n".into(),
        ),
        ..Default::default()
    };

    let running = Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .expect("valid configuration should start the daemon");

    assert_eq!(running.config().max_duration_secs, 15);
    assert_eq!(running.config().wav_dir, Path::new("/var/tmp/voice"));
    assert!(running.config().keep_wav);
}

#[test]
fn zero_duration_is_rejected_before_startup_side_effects() {
    let mut boundaries = ControlledBoundaries {
        config: Some("max_duration_secs = 0\n".into()),
        ..Default::default()
    };

    let error = Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .err()
    .expect("zero duration must be rejected");

    assert_eq!(
        error.to_string(),
        "max_duration_secs must be between 1 and 3600 seconds, got 0"
    );
    assert_eq!(boundaries.prepared_wav_dir, None);
    assert!(boundaries.commands.is_empty());
    assert!(boundaries.events.is_empty());
}

#[test]
fn excessive_duration_is_rejected() {
    let mut boundaries = ControlledBoundaries {
        config: Some("max_duration_secs = 3601\n".into()),
        ..Default::default()
    };

    let error = Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .err()
    .expect("unsafe duration must be rejected");

    assert_eq!(
        error.to_string(),
        "max_duration_secs must be between 1 and 3600 seconds, got 3601"
    );
}

#[test]
fn unknown_configuration_field_is_rejected() {
    let mut boundaries = ControlledBoundaries {
        config: Some("keep_audio = true\n".into()),
        ..Default::default()
    };

    let error = Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .err()
    .expect("unknown fields must be rejected");

    let message = error.to_string();
    assert!(message.contains("invalid configuration in /etc/atvv-bridge.toml"));
    assert!(message.contains("unknown field `keep_audio`"));
}

#[test]
fn invalid_configuration_type_is_rejected() {
    let mut boundaries = ControlledBoundaries {
        config: Some("keep_wav = 'yes'\n".into()),
        ..Default::default()
    };

    let error = Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .err()
    .expect("invalid field types must be rejected");

    let message = error.to_string();
    assert!(message.contains("invalid configuration in /etc/atvv-bridge.toml"));
    assert!(message.contains("keep_wav"));
    assert!(message.contains("boolean"));
}

#[test]
fn missing_explicit_configuration_is_an_error() {
    let mut boundaries = ControlledBoundaries::default();

    let error = Application::start(
        ConfigSelection::Explicit("/missing/config.toml".into()),
        &mut boundaries,
    )
    .err()
    .expect("an explicitly selected file is required");

    assert_eq!(
        error.to_string(),
        "configuration file does not exist: /missing/config.toml"
    );
}

#[test]
fn unusable_wav_directory_prevents_daemon_start() {
    let mut boundaries = ControlledBoundaries {
        prepare_error: Some(io::ErrorKind::PermissionDenied),
        ..Default::default()
    };

    let error = Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .err()
        .expect("an unusable WAV directory must fail startup");

    assert_eq!(
        error.to_string(),
        "WAV directory /tmp/atvv-bridge is unusable: controlled storage failure"
    );
    assert!(boundaries.events.is_empty());
}

#[test]
fn scripted_atvv_transport_drives_observable_daemon_behavior() {
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::WaitingForRemote,
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            AtvvEvent::Stopped,
        ]),
        ..Default::default()
    };

    Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .expect("startup should succeed")
        .run()
        .expect("scripted transport should stop cleanly");

    assert_eq!(
        &boundaries.events[1..],
        &[
            OperationalEvent::WaitingForRemote {
                at: SystemTime::UNIX_EPOCH,
            },
            OperationalEvent::RemoteReady {
                at: SystemTime::UNIX_EPOCH,
                address: "AA:BB:CC:DD:EE:FF".into(),
            },
            OperationalEvent::DaemonStopped {
                at: SystemTime::UNIX_EPOCH,
            },
        ]
    );
}

#[test]
fn completed_capture_is_transcribed_committed_and_deleted() {
    let frame = vec![0x11; 120];
    let transcript = "quotes ' \"; $(still-data)\nsecond line";
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            AtvvEvent::ControlNotification(vec![0x04, 0x03, 0x02, 0x65]),
            AtvvEvent::AudioNotification(frame),
            AtvvEvent::ControlNotification(vec![0x00, 0x02]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([
            Ok(CommandOutput {
                status: 0,
                stdout: format!("\n  {transcript}\t\n").into_bytes(),
                stderr: Vec::new(),
            }),
            Ok(CommandOutput {
                status: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
            }),
        ]),
        ..Default::default()
    };

    Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .expect("startup should succeed")
        .run()
        .expect("a valid Capture should complete");

    let (path, wav) = boundaries.wav.as_ref().expect("a WAV should be created");
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1);
    assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
    assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16);
    assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 480);
    assert_eq!(wav.len(), 44 + 240 * 2);
    assert_eq!(i16::from_le_bytes([wav[44], wav[45]]), 1);
    assert_eq!(i16::from_le_bytes([wav[46], wav[47]]), 2);
    assert_eq!(i16::from_le_bytes([wav[522], wav[523]]), 240);
    assert_eq!(
        boundaries.commands,
        [
            Command {
                program: "voxtype".into(),
                args: vec!["transcribe".into(), path.display().to_string()],
            },
            Command {
                program: "fcitx5-commit".into(),
                args: vec!["--text".into(), transcript.into()],
            },
        ]
    );
    assert_eq!(
        boundaries.removed_files.as_slice(),
        std::slice::from_ref(path)
    );
    assert_eq!(
        boundaries
            .events
            .iter()
            .filter(|event| matches!(event, OperationalEvent::CaptureCompleted { .. }))
            .collect::<Vec<_>>(),
        [&OperationalEvent::CaptureCompleted {
            at: SystemTime::UNIX_EPOCH,
            stream_id: 0x65,
            samples: 240,
        }]
    );
}
