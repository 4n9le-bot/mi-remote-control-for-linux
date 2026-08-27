use std::{
    fs::{self, OpenOptions},
    io,
    path::Path,
    process,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use crate::{
    AtvvEvent, AtvvTransport, Clock, Command, CommandOutput, OperationalEvent, OperationalEvents,
    ProcessExecutor, Storage,
};

static PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default)]
pub struct SystemBoundaries;

impl AtvvTransport for SystemBoundaries {
    fn next_event(&mut self) -> io::Result<AtvvEvent> {
        loop {
            thread::park();
        }
    }
}

impl ProcessExecutor for SystemBoundaries {
    fn execute(&mut self, command: &Command) -> io::Result<CommandOutput> {
        let output = process::Command::new(&command.program)
            .args(&command.args)
            .output()?;
        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

impl Storage for SystemBoundaries {
    fn read_optional_config(&mut self, path: &Path) -> io::Result<Option<String>> {
        match fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn prepare_wav_dir(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)?;
        if !fs::metadata(path)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path is not a directory",
            ));
        }

        let probe = path.join(format!(
            ".atvv-bridge-write-probe-{}-{}",
            process::id(),
            PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(&probe)?;
        drop(file);
        fs::remove_file(probe)
    }
}

impl Clock for SystemBoundaries {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl OperationalEvents for SystemBoundaries {
    fn emit(&mut self, event: OperationalEvent) {
        match event {
            OperationalEvent::DaemonStarted {
                at,
                max_duration_secs,
                wav_dir,
                keep_wav,
            } => eprintln!(
                "event=daemon_started at_unix_ms={} max_duration_secs={} wav_dir={:?} keep_wav={}",
                unix_millis(at),
                max_duration_secs,
                wav_dir,
                keep_wav
            ),
            OperationalEvent::WaitingForRemote { at } => {
                eprintln!("event=waiting_for_remote at_unix_ms={}", unix_millis(at))
            }
            OperationalEvent::RemoteReady { at, address } => eprintln!(
                "event=remote_ready at_unix_ms={} address={:?}",
                unix_millis(at),
                address
            ),
            OperationalEvent::DaemonStopped { at } => {
                eprintln!("event=daemon_stopped at_unix_ms={}", unix_millis(at))
            }
        }
    }
}

fn unix_millis(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
