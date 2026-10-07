//! 异步设置操作的持久化边界；配置仅含非敏感选项。
use crate::{
    ConfigSelection, load_config,
    system::SystemBoundaries,
    transcription::{CredentialStore, Credentials, Provider, TranscriptionSettings},
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub type SharedSettings = Arc<RwLock<Option<TranscriptionSettings>>>;
#[derive(Clone)]
pub enum SettingsRequest {
    Inspect,
    Save {
        settings: TranscriptionSettings,
        credentials: Option<Credentials>,
    },
    Delete(Provider),
}
#[derive(Debug, Clone)]
pub struct SettingsPresentation {
    pub settings: TranscriptionSettings,
    pub configured: bool,
    pub busy: bool,
    pub message: String,
}

pub struct SettingsService<K> {
    keys: K,
    selection: ConfigSelection,
    shared: SharedSettings,
}
impl<K: CredentialStore> SettingsService<K> {
    pub fn new(keys: K, selection: ConfigSelection, shared: SharedSettings) -> Self {
        Self {
            keys,
            selection,
            shared,
        }
    }
    pub fn perform(&mut self, request: SettingsRequest) -> Result<SettingsPresentation, String> {
        let mut storage = SystemBoundaries::default();
        let mut config = load_config(self.selection.clone(), &mut storage)
            .map_err(|_| "无法读取配置，请先修复配置文件")?;
        let mut message = String::new();
        let mut saved = false;
        match request {
            SettingsRequest::Inspect => {}
            SettingsRequest::Save {
                settings,
                credentials,
            } => {
                settings.validate()?;
                let provider = settings.provider;
                let mut previous_credentials = None;
                let mut credentials_changed = false;
                if provider != Provider::Voxtype {
                    previous_credentials = self.keys.read(provider)?;
                    let (credentials, changed) = match credentials {
                        Some(value) => (value, true),
                        None => (
                            previous_credentials
                                .clone()
                                .ok_or("未找到凭据或密钥环未解锁，请解锁后配置并保存")?,
                            false,
                        ),
                    };
                    credentials.validate(provider)?;
                    if changed {
                        self.keys.write(provider, &credentials)?;
                        credentials_changed = true;
                    }
                }
                config.transcription = settings;
                if let Err(error) = self.persist(&config) {
                    if credentials_changed {
                        let rollback = match previous_credentials {
                            Some(keys) => self.keys.write(provider, &keys),
                            None => self.keys.delete(provider),
                        };
                        if rollback.is_err() {
                            return Err(
                                "配置保存失败，凭据回滚也失败；当前服务选择未变更，请重新配置凭据"
                                    .into(),
                            );
                        }
                    }
                    return Err(error);
                }
                *self.shared.write().map_err(|_| "无法更新生效设置")? =
                    Some(config.transcription.clone());
                message = "已保存；下一次 Capture 使用新设置".into();
                saved = true;
            }
            SettingsRequest::Delete(provider) => {
                if provider == Provider::Voxtype {
                    return Err("Voxtype 不使用在线凭据".into());
                }
                // 先持久化切回本地，避免删除后继续启用无凭据在线服务。
                if config.transcription.provider == provider {
                    config.transcription.provider = Provider::Voxtype;
                    self.persist(&config)?;
                    *self.shared.write().map_err(|_| "无法更新生效设置")? =
                        Some(config.transcription.clone());
                }
                self.keys.delete(provider)?;
                message = "凭据已删除；可重新配置".into();
            }
        }
        let configured = if saved || config.transcription.provider == Provider::Voxtype {
            true
        } else {
            match self.keys.read(config.transcription.provider) {
                Ok(keys) => {
                    keys.is_some_and(|keys| keys.validate(config.transcription.provider).is_ok())
                }
                Err(error) => {
                    message = format!("{message} {error}");
                    false
                }
            }
        };
        Ok(SettingsPresentation {
            settings: config.transcription,
            configured,
            busy: false,
            message,
        })
    }
    fn persist(&self, config: &crate::Config) -> Result<(), String> {
        let path = match &self.selection {
            ConfigSelection::DefaultPath(path) | ConfigSelection::Explicit(path) => path,
            ConfigSelection::DefaultsOnly => {
                return Err("无法确定配置目录，请设置 HOME 或 XDG_CONFIG_HOME".into());
            }
        };
        let directory = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(directory).map_err(|_| "无法创建配置目录")?;
        let content = toml::to_string_pretty(config).map_err(|_| "无法序列化配置")?;
        let mut temp =
            tempfile::NamedTempFile::new_in(directory).map_err(|_| "无法创建配置临时文件")?;
        temp.write_all(content.as_bytes())
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|_| "无法写入配置")?;
        temp.persist(path)
            .map_err(|_| "无法保存配置，当前选择未变更")?;
        Ok(())
    }
}

/// secret-tool 通过当前用户 Secret Service 访问密钥环，秘密只经 stdin/stdout。
pub struct SystemKeyring {
    cancel: Arc<AtomicBool>,
}
impl SystemKeyring {
    pub fn new(cancel: Arc<AtomicBool>) -> Self {
        Self { cancel }
    }
    fn run(
        &self,
        operation: &str,
        provider: Provider,
        secret: Option<Vec<u8>>,
    ) -> Result<Option<Vec<u8>>, String> {
        let mut command = Command::new("secret-tool");
        command.arg(operation);
        if operation == "store" {
            command.arg("--label=ATVV Voice Bridge 语音识别凭据");
        }
        command.args([
            "application",
            "io.github.atvv_bridge",
            "provider",
            provider.key(),
        ]);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "密钥环工具不可用，请安装 libsecret-tools 并启动 Secret Service")?;
        let stdin = child.stdin.take().expect("piped stdin");
        let writer = thread::spawn(move || {
            if let Some(secret) = secret {
                let mut stdin = stdin;
                stdin.write_all(&secret)
            } else {
                drop(stdin);
                Ok(())
            }
        });
        let stdout = child.stdout.take().expect("piped stdout");
        let reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.take(32769).read_to_end(&mut bytes).map(|_| bytes)
        });
        let deadline = Instant::now() + Duration::from_secs(30);
        let result: Result<bool, String> = loop {
            if self.cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                break Err("密钥环访问已取消或超时，请解锁后重试".into());
            }
            match child.try_wait() {
                Ok(Some(status)) if status.success() => break Ok(true),
                Ok(Some(status)) if operation == "lookup" && status.code() == Some(1) => {
                    break Ok(false);
                }
                Ok(Some(_)) => break Err("密钥环操作失败，请解锁密钥环并重试".into()),
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err("无法完成密钥环操作".into());
                }
                Ok(None) => thread::sleep(Duration::from_millis(25)),
            }
        };
        let bytes = reader
            .join()
            .map_err(|_| "无法读取密钥环")?
            .map_err(|_| "无法读取密钥环")?;
        let written = writer.join().map_err(|_| "无法写入密钥环")?;
        let exists = result?;
        written.map_err(|_| "无法写入密钥环，请解锁后重试")?;
        if bytes.len() > 32768 {
            return Err("密钥环凭据大小无效".into());
        }
        Ok(exists.then_some(bytes))
    }
}
impl CredentialStore for SystemKeyring {
    fn read(&mut self, provider: Provider) -> Result<Option<Credentials>, String> {
        if provider == Provider::Voxtype {
            return Ok(None);
        }
        self.run("lookup", provider, None)?
            .map(|bytes| {
                serde_json::from_slice(&bytes).map_err(|_| "密钥环凭据格式无效，请重新配置".into())
            })
            .transpose()
    }
    fn write(&mut self, provider: Provider, credentials: &Credentials) -> Result<(), String> {
        credentials.validate(provider)?;
        let mut data = serde_json::to_vec(credentials).map_err(|_| "无法编码凭据")?;
        data.push(b'\n');
        self.run("store", provider, Some(data)).map(|_| ())
    }
    fn delete(&mut self, provider: Provider) -> Result<(), String> {
        self.run("clear", provider, None).map(|_| ())
    }
}

pub fn config_path(selection: &ConfigSelection) -> Option<PathBuf> {
    match selection {
        ConfigSelection::DefaultPath(path) | ConfigSelection::Explicit(path) => Some(path.clone()),
        ConfigSelection::DefaultsOnly => None,
    }
}
