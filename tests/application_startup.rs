use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use atvv_bridge::{
    Application, AtvvEvent, AtvvTransport, AudioNotificationIssue, Clock, Command, CommandOutput,
    ConfigSelection, ControlNotification, ControlNotificationIssue, IntegrationStage,
    OperationalEvent, OperationalEvents, ProcessExecutor, Storage, WavHandoffOutcome,
};

#[derive(Default)]
struct ControlledBoundaries {
    config: Option<String>,
    prepared_wav_dir: Option<String>,
    prepare_error: Option<io::ErrorKind>,
    create_error: Option<io::ErrorKind>,
    remove_error: Option<io::ErrorKind>,
    atvv_events: VecDeque<AtvvEvent>,
    process_results: VecDeque<io::Result<CommandOutput>>,
    commands: Vec<Command>,
    events: Vec<OperationalEvent>,
    wav: Option<(PathBuf, Vec<u8>)>,
    created_wavs: Vec<(PathBuf, Vec<u8>)>,
    removed_files: Vec<PathBuf>,
    now: Duration,
    observed_deadlines: Vec<Option<SystemTime>>,
    timeout_after_events: Option<usize>,
    process_durations: VecDeque<Duration>,
    wav_creation_duration: Duration,
}

impl AtvvTransport for ControlledBoundaries {
    fn next_event(&mut self, deadline: Option<SystemTime>) -> io::Result<Option<AtvvEvent>> {
        self.observed_deadlines.push(deadline);
        if let Some(deadline) = deadline {
            match self.timeout_after_events {
                Some(0) => {
                    self.timeout_after_events = None;
                    self.now = deadline
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .expect("controlled deadlines follow the Unix epoch");
                    return Ok(None);
                }
                Some(remaining) => self.timeout_after_events = Some(remaining - 1),
                None => {}
            }
        }
        let event = self
            .atvv_events
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "script exhausted"))?;
        if let AtvvEvent::ControlNotification(notification) = &event {
            self.now = self.now.max(
                notification
                    .received_at
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("controlled notifications follow the Unix epoch"),
            );
        }
        Ok(Some(event))
    }
}

impl ProcessExecutor for ControlledBoundaries {
    fn execute(&mut self, command: &Command) -> io::Result<CommandOutput> {
        self.commands.push(command.clone());
        self.now += self.process_durations.pop_front().unwrap_or_default();
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
        if let Some(kind) = self.create_error {
            return Err(io::Error::new(kind, "controlled WAV creation failure"));
        }
        self.now += self.wav_creation_duration;
        self.wav_creation_duration = Duration::ZERO;
        let path = PathBuf::from(format!(
            "/tmp/atvv-bridge/capture-test-{}.wav",
            self.created_wavs.len()
        ));
        self.wav = Some((path.clone(), contents.to_vec()));
        self.created_wavs.push((path.clone(), contents.to_vec()));
        Ok(path)
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        if let Some(kind) = self.remove_error {
            return Err(io::Error::new(kind, "controlled WAV cleanup failure"));
        }
        self.removed_files.push(path.to_owned());
        Ok(())
    }
}

impl Clock for ControlledBoundaries {
    fn now(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + self.now
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
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
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
    let transcript = "quotes ' \"; $(still-data)\nsecond line\n\nfinal paragraph";
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 0x65]),
            AtvvEvent::AudioNotification(frame),
            control_at(0, vec![0x00, 0x02]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([
            Ok(CommandOutput {
                status: 0,
                stdout: format!(
                    "Loading audio file: capture.wav\nAudio format: 16000 Hz, 1 channel(s), Int\nProcessing 240 samples (0.02s)...\n\n  {transcript}\t\n"
                )
                .into_bytes(),
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
    let handoff_event = boundaries
        .events
        .iter()
        .find(|event| matches!(event, OperationalEvent::WavHandoffSucceeded { .. }))
        .expect("successful handoff should emit diagnostics");
    assert!(matches!(
        handoff_event,
        OperationalEvent::WavHandoffSucceeded {
            address,
            audio_bytes: 120,
            outcome: WavHandoffOutcome::TextCommitted,
            retained_wav: None,
            ..
        } if address == "AA:BB:CC:DD:EE:FF"
    ));
    assert!(!format!("{handoff_event:?}").contains(transcript));
    let event_position = |matches: fn(&OperationalEvent) -> bool| {
        boundaries
            .events
            .iter()
            .position(matches)
            .expect("Capture lifecycle event should be observable")
    };
    let capture_started = event_position(|event| {
        matches!(
            event,
            OperationalEvent::CaptureStarted {
                stream_id: 0x65,
                ..
            }
        )
    });
    let capture_stopped =
        event_position(|event| matches!(event, OperationalEvent::CaptureStopped { .. }));
    let handoff_started =
        event_position(|event| matches!(event, OperationalEvent::WavHandoffStarted { .. }));
    let handoff_succeeded =
        event_position(|event| matches!(event, OperationalEvent::WavHandoffSucceeded { .. }));
    assert!(capture_started < capture_stopped);
    assert!(capture_stopped < handoff_started);
    assert!(handoff_started < handoff_succeeded);
}

#[test]
fn missing_voxtype_retains_the_wav_and_reports_a_diagnostic_path() {
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results = VecDeque::from([Err(io::Error::new(
        io::ErrorKind::NotFound,
        "voxtype is not installed",
    ))]);

    run_capture(&mut boundaries);

    let path = boundaries
        .wav
        .as_ref()
        .expect("failed transcription should retain its WAV")
        .0
        .clone();
    assert!(boundaries.removed_files.is_empty());
    assert!(boundaries.events.iter().any(|event| matches!(
        event,
        OperationalEvent::WavHandoffFailed {
            at: SystemTime::UNIX_EPOCH,
            address,
            duration: Duration::ZERO,
            audio_bytes: 120,
            stage: IntegrationStage::Transcription,
            retained_wav: Some(retained),
            ..
        } if address == "AA:BB:CC:DD:EE:FF" && retained == &path
    )));
}

#[test]
fn failed_transcription_retains_the_wav_without_logging_process_output() {
    let transcript_marker = "PRIVATE TRANSCRIPT MARKER";
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results = VecDeque::from([Ok(CommandOutput {
        status: 23,
        stdout: transcript_marker.as_bytes().to_vec(),
        stderr: transcript_marker.as_bytes().to_vec(),
    })]);

    run_capture(&mut boundaries);

    let event = boundaries
        .events
        .iter()
        .find(|event| matches!(event, OperationalEvent::WavHandoffFailed { .. }))
        .expect("failed transcription should be diagnostic");
    assert!(format!("{event:?}").contains("status 23"));
    assert!(!format!("{event:?}").contains(transcript_marker));
    assert!(boundaries.removed_files.is_empty());
}

#[test]
fn failed_text_commit_is_not_retried_and_retains_the_wav() {
    let transcript = "do not log this transcript";
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results = VecDeque::from([
        successful_process(transcript),
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "fcitx5-commit is not installed",
        )),
    ]);

    run_capture(&mut boundaries);

    assert_eq!(boundaries.commands.len(), 2, "Text Commit must not retry");
    assert!(boundaries.removed_files.is_empty());
    let event = boundaries
        .events
        .iter()
        .find(|event| matches!(event, OperationalEvent::WavHandoffFailed { .. }))
        .expect("failed Text Commit should be diagnostic");
    assert!(matches!(
        event,
        OperationalEvent::WavHandoffFailed {
            stage: IntegrationStage::TextCommit,
            error,
            retained_wav: Some(_),
            ..
        } if error.contains("fcitx5-commit")
    ));
    assert!(!format!("{event:?}").contains(transcript));
}

#[test]
fn voxtype_no_speech_result_is_a_successful_no_op() {
    let stdout = "Loading audio file: \"capture.wav\"\n\
Audio format: 16000 Hz, 1 channel(s), Int\n\
Processing 240 samples (0.02s)...\n\
VAD: 0.00s speech (0.0% of audio)\n\
No speech detected, skipping transcription.\n";
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results = VecDeque::from([successful_process(stdout)]);

    run_capture(&mut boundaries);

    assert_eq!(boundaries.commands.len(), 1);
    assert_eq!(boundaries.removed_files.len(), 1);
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::WavHandoffSucceeded {
                at: SystemTime::UNIX_EPOCH,
                address: "AA:BB:CC:DD:EE:FF".into(),
                duration: Duration::ZERO,
                audio_bytes: 120,
                outcome: WavHandoffOutcome::NoSpeech,
                retained_wav: None,
            })
    );
}

#[test]
fn empty_transcript_after_voxtype_wrapper_output_is_a_successful_no_op() {
    let stdout = "Loading audio file: capture.wav\nAudio format: 16000 Hz, 1 channel(s), Int\nProcessing 240 samples (0.02s)...\n\n\n";
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results = VecDeque::from([successful_process(stdout)]);

    run_capture(&mut boundaries);

    assert_eq!(boundaries.commands.len(), 1);
    assert_eq!(boundaries.removed_files.len(), 1);
    assert!(boundaries.events.iter().any(|event| matches!(
        event,
        OperationalEvent::WavHandoffSucceeded {
            outcome: WavHandoffOutcome::NoSpeech,
            ..
        }
    )));
}

#[test]
fn unwrapped_paragraphs_preserve_all_internal_transcript_content() {
    let transcript = "first paragraph\n\nsecond paragraph";
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results =
        VecDeque::from([successful_process(transcript), successful_process("")]);

    run_capture(&mut boundaries);

    assert_eq!(
        boundaries.commands[1].args,
        ["--text", transcript],
        "internal paragraph separators are transcript content"
    );
}

#[test]
fn transcript_that_starts_like_one_wrapper_line_is_not_rewritten() {
    let transcript = "Loading audio file: notes\n\nsecond paragraph";
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results =
        VecDeque::from([successful_process(transcript), successful_process("")]);

    run_capture(&mut boundaries);

    assert_eq!(boundaries.commands[1].args, ["--text", transcript]);
}

#[test]
fn wrapped_no_speech_result_is_a_successful_no_op() {
    let stdout = "Loading audio file: capture.wav\nProcessing 240 samples (0.02s)...\n\nNo speech detected, skipping transcription.\n";
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results = VecDeque::from([successful_process(stdout)]);

    run_capture(&mut boundaries);

    assert_eq!(boundaries.commands.len(), 1);
    assert_eq!(boundaries.removed_files.len(), 1);
}

#[test]
fn missing_integration_is_resolved_again_for_the_next_capture() {
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 1]),
            AtvvEvent::AudioNotification(vec![0x11; 120]),
            control_at(0, vec![0x00, 0x02]),
            control_at(1, vec![0x04, 0x03, 0x02, 2]),
            AtvvEvent::AudioNotification(vec![0x22; 120]),
            control_at(1, vec![0x00, 0x02]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([
            Err(io::Error::new(io::ErrorKind::NotFound, "voxtype missing")),
            successful_process(""),
        ]),
        ..Default::default()
    };

    run_capture(&mut boundaries);

    assert_eq!(
        boundaries
            .commands
            .iter()
            .map(|command| command.program.as_str())
            .collect::<Vec<_>>(),
        ["voxtype", "voxtype"]
    );
    assert_eq!(boundaries.created_wavs.len(), 2);
    assert_eq!(
        boundaries.removed_files,
        [boundaries.created_wavs[1].0.clone()]
    );
    assert_ne!(boundaries.removed_files[0], boundaries.created_wavs[0].0);
}

#[test]
fn invalid_voxtype_output_retains_the_wav_as_a_transcription_failure() {
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.process_results = VecDeque::from([Ok(CommandOutput {
        status: 0,
        stdout: vec![0xff],
        stderr: Vec::new(),
    })]);

    run_capture(&mut boundaries);

    assert!(boundaries.removed_files.is_empty());
    assert!(boundaries.events.iter().any(|event| matches!(
        event,
        OperationalEvent::WavHandoffFailed {
            stage: IntegrationStage::Transcription,
            error,
            retained_wav: Some(_),
            ..
        } if error.contains("non-UTF-8")
    )));
}

#[test]
fn keep_wav_retains_a_successful_completed_capture() {
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.config = Some("keep_wav = true\n".into());

    Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .expect("startup should succeed")
    .run()
    .expect("the Capture should leave the daemon usable");

    let retained_path = boundaries.created_wavs[0].0.clone();
    assert!(boundaries.removed_files.is_empty());
    assert!(boundaries.events.iter().any(|event| matches!(
        event,
        OperationalEvent::WavHandoffSucceeded {
            retained_wav: Some(path),
            ..
        } if path == &retained_path
    )));
}

#[test]
fn wav_creation_failure_is_reported_without_running_integrations() {
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.create_error = Some(io::ErrorKind::PermissionDenied);

    run_capture(&mut boundaries);

    assert!(boundaries.commands.is_empty());
    assert!(boundaries.events.iter().any(|event| matches!(
        event,
        OperationalEvent::WavHandoffFailed {
            stage: IntegrationStage::WavCreation,
            error,
            retained_wav: None,
            ..
        } if error.contains("WAV")
    )));
}

#[test]
fn wav_cleanup_failure_reports_the_retained_diagnostic_path() {
    let mut boundaries = capture_boundaries([AtvvEvent::AudioNotification(vec![0x11; 120])]);
    boundaries.remove_error = Some(io::ErrorKind::PermissionDenied);

    run_capture(&mut boundaries);

    let retained_path = boundaries.created_wavs[0].0.clone();
    assert!(boundaries.events.iter().any(|event| matches!(
        event,
        OperationalEvent::WavHandoffFailed {
            stage: IntegrationStage::WavCleanup,
            error,
            retained_wav: Some(path),
            ..
        } if error.contains("delete")
            && path == &retained_path
    )));
}

#[test]
fn disconnect_discards_capture_before_reattachment() {
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 1]),
            AtvvEvent::AudioNotification(vec![0x11; 120]),
            AtvvEvent::WaitingForRemote,
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(1, vec![0x04, 0x03, 0x02, 2]),
            AtvvEvent::AudioNotification(vec![0x22; 120]),
            control_at(2, vec![0x00, 0x02]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([successful_process("")]),
        ..Default::default()
    };

    Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .expect("startup should succeed")
        .run()
        .expect("reattachment should leave the daemon usable");

    assert_eq!(boundaries.commands.len(), 1);
    assert_eq!(wav_samples(&boundaries).len(), 240);
    assert_eq!(
        boundaries
            .events
            .iter()
            .filter_map(|event| match event {
                OperationalEvent::CaptureCompleted { stream_id, .. } => Some(*stream_id),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [2]
    );
}

#[test]
fn shutdown_discards_an_unfinished_capture() {
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 1]),
            AtvvEvent::AudioNotification(vec![0x11; 120]),
            AtvvEvent::Stopped,
        ]),
        ..Default::default()
    };

    Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .expect("startup should succeed")
        .run()
        .expect("shutdown should stop cleanly");

    assert!(boundaries.wav.is_none());
    assert!(boundaries.commands.is_empty());
    assert!(boundaries.removed_files.is_empty());
    assert!(
        !boundaries
            .events
            .iter()
            .any(|event| matches!(event, OperationalEvent::CaptureCompleted { .. }))
    );
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::DaemonStopped {
                at: SystemTime::UNIX_EPOCH,
            })
    );
}

#[test]
fn maximum_duration_hands_off_collected_audio_without_a_stop_notification() {
    let deadline = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
    let mut boundaries = ControlledBoundaries {
        config: Some("max_duration_secs = 2\n".into()),
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 0x65]),
            AtvvEvent::AudioNotification(vec![0x11; 120]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([Ok(CommandOutput {
            status: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
        })]),
        timeout_after_events: Some(1),
        ..Default::default()
    };

    let running = Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .expect("startup should succeed");
    running
        .run()
        .expect("an expired Capture should complete and remain usable");

    assert_eq!(boundaries.commands.len(), 1);
    assert!(boundaries.wav.is_some());
    assert!(boundaries.observed_deadlines.contains(&Some(deadline)));
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::CaptureCompleted {
                at: deadline,
                stream_id: 0x65,
                samples: 240,
            })
    );
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::WavHandoffSucceeded {
                at: deadline,
                address: "AA:BB:CC:DD:EE:FF".into(),
                duration: Duration::from_secs(2),
                audio_bytes: 120,
                outcome: WavHandoffOutcome::NoSpeech,
                retained_wav: None,
            })
    );
}

#[test]
fn queued_notifications_cannot_extend_capture_beyond_the_duration_limit() {
    let mut boundaries = ControlledBoundaries {
        config: Some("max_duration_secs = 2\n".into()),
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 1]),
            AtvvEvent::AudioNotification(vec![0x11; 120]),
            control_at(2, vec![0x99]),
            AtvvEvent::AudioNotification(vec![0x11; 120]),
            control_at(3, vec![0x04, 0x03, 0x02, 2]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([successful_process("")]),
        ..Default::default()
    };

    Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .expect("startup should succeed")
    .run()
    .expect("queued notifications should not defeat the duration limit");

    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::CaptureCompleted {
                at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                stream_id: 1,
                samples: 240,
            })
    );
    assert_eq!(boundaries.wav.as_ref().unwrap().1.len(), 44 + 240 * 2);
}

#[test]
fn start_received_during_handoff_is_rejected_without_queueing_or_overlap() {
    let frame = vec![0x11; 120];
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 1]),
            AtvvEvent::AudioNotification(frame.clone()),
            control_at(1, vec![0x00, 0x02]),
            control_at_millis(1_500, vec![0x04, 0x03, 0x02, 2]),
            control_at_millis(2_500, vec![0x04, 0x03, 0x02, 3]),
            control_at_millis(3_500, vec![0x04, 0x03, 0x02, 4]),
            control_at(5, vec![0x04, 0x03, 0x02, 5]),
            AtvvEvent::AudioNotification(frame),
            control_at(6, vec![0x00, 0x02]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([
            successful_process("first"),
            successful_process(""),
            successful_process("second"),
            successful_process(""),
        ]),
        process_durations: VecDeque::from([Duration::from_secs(1), Duration::from_secs(1)]),
        wav_creation_duration: Duration::from_secs(1),
        ..Default::default()
    };

    Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .expect("startup should succeed")
        .run()
        .expect("busy starts should not disrupt the daemon");

    let completed_streams = boundaries
        .events
        .iter()
        .filter_map(|event| match event {
            OperationalEvent::CaptureCompleted { stream_id, .. } => Some(*stream_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(completed_streams, [1, 5]);
    assert_eq!(
        boundaries
            .events
            .iter()
            .filter(|event| matches!(
                event,
                OperationalEvent::ControlNotificationIgnored {
                    issue: ControlNotificationIssue::WavHandoffBusy,
                    ..
                }
            ))
            .count(),
        3
    );
}

#[test]
fn invalid_control_notifications_are_warned_and_do_not_corrupt_capture_state() {
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x00, 0x02]),
            control_at(1, vec![0x99, 0x01]),
            control_at(2, vec![0x04, 0x01]),
            control_at(3, vec![0x04, 0x03, 0x02, 7]),
            control_at(4, vec![0x04, 0x03, 0x02, 8]),
            AtvvEvent::AudioNotification(vec![0x11; 120]),
            control_at(5, vec![0x00, 0x02]),
            AtvvEvent::Stopped,
        ]),
        process_results: VecDeque::from([successful_process("")]),
        ..Default::default()
    };

    Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .expect("startup should succeed")
        .run()
        .expect("invalid controls should not terminate the daemon");

    let issues = boundaries
        .events
        .iter()
        .filter_map(|event| match event {
            OperationalEvent::ControlNotificationIgnored { issue, .. } => Some(*issue),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        issues,
        [
            ControlNotificationIssue::OutOfOrder,
            ControlNotificationIssue::Unknown,
            ControlNotificationIssue::Malformed,
            ControlNotificationIssue::DuplicateStart,
        ]
    );
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::CaptureCompleted {
                at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
                stream_id: 7,
                samples: 240,
            })
    );
}

#[test]
fn malformed_audio_is_warned_and_does_not_change_decoder_state() {
    let mut boundaries = capture_boundaries([
        AtvvEvent::AudioNotification(vec![0x77; 120]),
        AtvvEvent::AudioNotification(vec![0x00; 119]),
        AtvvEvent::AudioNotification(vec![0x00; 120]),
    ]);

    run_capture(&mut boundaries);

    assert_eq!(wav_samples(&boundaries).len(), 480);
    assert_eq!(wav_samples(&boundaries)[240], i16::MAX);
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::AudioNotificationIgnored {
                at: SystemTime::UNIX_EPOCH,
                issue: AudioNotificationIssue::Malformed {
                    expected_bytes: 120,
                    actual_bytes: 119,
                },
            })
    );
}

#[test]
fn malformed_audio_outside_a_capture_is_warned_without_stopping_the_daemon() {
    let mut boundaries = ControlledBoundaries {
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            AtvvEvent::AudioNotification(vec![0; 119]),
            AtvvEvent::Stopped,
        ]),
        ..Default::default()
    };

    Application::start(ConfigSelection::DefaultsOnly, &mut boundaries)
        .expect("startup should succeed")
        .run()
        .expect("malformed audio should not stop the daemon");

    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::AudioNotificationIgnored {
                at: SystemTime::UNIX_EPOCH,
                issue: AudioNotificationIssue::Malformed {
                    expected_bytes: 120,
                    actual_bytes: 119,
                },
            })
    );
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::DaemonStopped {
                at: SystemTime::UNIX_EPOCH,
            })
    );
}

#[test]
fn omitted_audio_notification_does_not_discard_capture_or_invent_loss_detection() {
    let mut boundaries = capture_boundaries([
        AtvvEvent::AudioNotification(vec![0x11; 120]),
        AtvvEvent::AudioNotification(vec![0x11; 120]),
    ]);

    run_capture(&mut boundaries);

    assert_eq!(wav_samples(&boundaries).len(), 480);
    assert!(!boundaries.events.iter().any(|event| matches!(
        event,
        OperationalEvent::AudioNotificationIgnored { .. }
            | OperationalEvent::ControlNotificationIgnored { .. }
    )));
}

#[test]
fn valid_audio_sync_resets_decoder_state_for_later_headerless_audio() {
    let mut boundaries = capture_boundaries([
        AtvvEvent::AudioNotification(vec![0x77; 120]),
        control_at(0, vec![0x0A, 0x02, 0x12, 0x34, 0x03, 0xE8, 20]),
        AtvvEvent::AudioNotification({
            let mut frame = vec![0; 120];
            frame[0] = 0x35;
            frame
        }),
    ]);

    run_capture(&mut boundaries);

    assert_eq!(&wav_samples(&boundaries)[240..242], [1_043, 1_104]);
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::DecoderSynchronized {
                at: SystemTime::UNIX_EPOCH,
            })
    );
}

#[test]
fn invalid_audio_sync_is_warned_and_leaves_decoder_state_safe() {
    let mut boundaries = capture_boundaries([
        AtvvEvent::AudioNotification(vec![0x77; 120]),
        control_at(0, vec![0x0A, 0x01, 0x00, 0x01, 0xFC, 0x18, 0]),
        AtvvEvent::AudioNotification(vec![0x00; 120]),
    ]);

    run_capture(&mut boundaries);

    assert_eq!(wav_samples(&boundaries)[240], i16::MAX);
    assert!(
        boundaries
            .events
            .contains(&OperationalEvent::ControlNotificationIgnored {
                at: SystemTime::UNIX_EPOCH,
                issue: ControlNotificationIssue::InvalidSynchronization,
            })
    );
}

#[test]
fn empty_expired_and_normally_stopped_captures_run_no_external_commands() {
    let mut boundaries = ControlledBoundaries {
        config: Some("max_duration_secs = 2\n".into()),
        atvv_events: VecDeque::from([
            AtvvEvent::RemoteReady {
                address: "AA:BB:CC:DD:EE:FF".into(),
                profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
            },
            control_at(0, vec![0x04, 0x03, 0x02, 1]),
            control_at(3, vec![0x04, 0x03, 0x02, 2]),
            control_at(4, vec![0x00, 0x02]),
            AtvvEvent::Stopped,
        ]),
        timeout_after_events: Some(0),
        ..Default::default()
    };

    Application::start(
        ConfigSelection::Explicit("/etc/atvv-bridge.toml".into()),
        &mut boundaries,
    )
    .expect("startup should succeed")
    .run()
    .expect("empty Captures should leave the daemon usable");

    assert!(boundaries.commands.is_empty());
    assert!(boundaries.wav.is_none());
    assert!(
        !boundaries
            .events
            .iter()
            .any(|event| matches!(event, OperationalEvent::CaptureCompleted { .. }))
    );
}

fn control_at(seconds: u64, payload: Vec<u8>) -> AtvvEvent {
    control_at_millis(seconds * 1_000, payload)
}

fn control_at_millis(milliseconds: u64, payload: Vec<u8>) -> AtvvEvent {
    AtvvEvent::ControlNotification(ControlNotification {
        received_at: SystemTime::UNIX_EPOCH + Duration::from_millis(milliseconds),
        payload,
    })
}

fn successful_process(stdout: &str) -> io::Result<CommandOutput> {
    Ok(CommandOutput {
        status: 0,
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    })
}

fn capture_boundaries<const N: usize>(audio_events: [AtvvEvent; N]) -> ControlledBoundaries {
    let mut atvv_events = VecDeque::from([
        AtvvEvent::RemoteReady {
            address: "AA:BB:CC:DD:EE:FF".into(),
            profile: atvv_bridge::AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
        },
        control_at(0, vec![0x04, 0x03, 0x02, 1]),
    ]);
    atvv_events.extend(audio_events);
    atvv_events.extend([control_at(0, vec![0x00, 0x02]), AtvvEvent::Stopped]);
    ControlledBoundaries {
        atvv_events,
        process_results: VecDeque::from([successful_process("")]),
        ..Default::default()
    }
}

fn run_capture(boundaries: &mut ControlledBoundaries) {
    Application::start(ConfigSelection::DefaultsOnly, boundaries)
        .expect("startup should succeed")
        .run()
        .expect("the Capture should leave the daemon usable");
}

fn wav_samples(boundaries: &ControlledBoundaries) -> Vec<i16> {
    boundaries.wav.as_ref().expect("a WAV should be created").1[44..]
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
        .collect()
}
