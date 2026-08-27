use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::Deserialize;
use thiserror::Error;

pub mod system;

const DEFAULT_MAX_DURATION_SECS: u64 = 60;
const DEFAULT_WAV_DIR: &str = "/tmp/atvv-bridge";
const MAX_DURATION_SECS: u64 = 3_600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtvvEvent {
    WaitingForRemote,
    RemoteReady { address: String },
    Stopped,
}

pub trait AtvvTransport {
    fn next_event(&mut self) -> io::Result<AtvvEvent>;
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
    DaemonStopped {
        at: SystemTime,
    },
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

impl<B> RunningApplication<'_, B>
where
    B: AtvvTransport + ProcessExecutor + Storage + Clock + OperationalEvents,
{
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn run(self) -> Result<(), RunError> {
        loop {
            let event = self.boundaries.next_event().map_err(RunError)?;
            let at = self.boundaries.now();
            let operational_event = match event {
                AtvvEvent::WaitingForRemote => OperationalEvent::WaitingForRemote { at },
                AtvvEvent::RemoteReady { address } => OperationalEvent::RemoteReady { at, address },
                AtvvEvent::Stopped => {
                    self.boundaries.emit(OperationalEvent::DaemonStopped { at });
                    return Ok(());
                }
            };
            self.boundaries.emit(operational_event);
        }
    }
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
