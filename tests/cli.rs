use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[test]
fn version_flag_reports_the_executable_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_atvv-bridge"))
        .arg("--version")
        .output()
        .expect("version command should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "atvv-bridge 0.1.0\n"
    );
}

#[test]
fn explicit_invalid_configuration_fails_with_an_actionable_error() {
    let temp = tempfile::tempdir().expect("temporary directory should be created");
    let config = temp.path().join("config.toml");
    fs::write(&config, "unexpected = true\n").expect("configuration should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_atvv-bridge"))
        .args(["--config", config.to_str().expect("UTF-8 temporary path")])
        .output()
        .expect("daemon should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid configuration"));
    assert!(stderr.contains("unknown field `unexpected`"));
}

#[test]
fn xdg_standard_configuration_is_selected_by_public_startup() {
    let temp = tempfile::tempdir().expect("temporary directory should be created");
    let config_dir = temp.path().join("atvv-bridge");
    fs::create_dir(&config_dir).expect("configuration directory should be created");
    fs::write(config_dir.join("config.toml"), "keep_wav = 'yes'\n")
        .expect("configuration should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_atvv-bridge"))
        .env("XDG_CONFIG_HOME", temp.path())
        .env_remove("HOME")
        .output()
        .expect("daemon should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(config_dir.join("config.toml").to_str().unwrap()));
    assert!(stderr.contains("keep_wav"));
    assert!(stderr.contains("boolean"));
}

#[test]
fn normal_startup_enters_the_daemon_run_loop() {
    let temp = tempfile::tempdir().expect("temporary directory should be created");
    let config = temp.path().join("config.toml");
    let wav_dir = temp.path().join("wav");
    fs::write(
        &config,
        format!("max_duration_secs = 5\nwav_dir = {:?}\n", wav_dir),
    )
    .expect("configuration should be written");
    let log_path = temp.path().join("daemon.log");
    let log = fs::File::create(&log_path).expect("log file should be created");
    let mut child = Command::new(env!("CARGO_BIN_EXE_atvv-bridge"))
        .args(["--config", config.to_str().expect("UTF-8 temporary path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log))
        .spawn()
        .expect("daemon should start");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let log = fs::read_to_string(&log_path).expect("daemon log should be readable");
        if log.contains("event=daemon_started") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "daemon did not report startup: {log}"
        );
        assert!(
            child
                .try_wait()
                .expect("daemon status should be readable")
                .is_none(),
            "daemon exited during startup: {log}"
        );
        thread::sleep(Duration::from_millis(10));
    }

    assert!(
        child
            .try_wait()
            .expect("daemon status should be readable")
            .is_none(),
        "daemon must remain running"
    );
    child
        .kill()
        .expect("daemon should be terminated after the test");
    child.wait().expect("terminated daemon should be reaped");
    assert!(wav_dir.is_dir());
}
