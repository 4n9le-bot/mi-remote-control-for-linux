use atvv_bridge::{
    ConfigSelection,
    transcription::{CredentialStore, Credentials, Provider, TranscriptionSettings},
    transcription_settings::{SettingsRequest, SettingsService},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};
#[derive(Default)]
struct Keys {
    values: Arc<Mutex<HashMap<String, Credentials>>>,
    fail_reads_after_write: bool,
    wrote: bool,
}
impl CredentialStore for Keys {
    fn read(&mut self, p: Provider) -> Result<Option<Credentials>, String> {
        if self.wrote && self.fail_reads_after_write {
            return Err("密钥环读取失败".into());
        }
        Ok(self.values.lock().unwrap().get(p.key()).cloned())
    }
    fn write(&mut self, p: Provider, c: &Credentials) -> Result<(), String> {
        self.wrote = true;
        self.values
            .lock()
            .unwrap()
            .insert(p.key().into(), c.clone());
        Ok(())
    }
    fn delete(&mut self, p: Provider) -> Result<(), String> {
        self.values.lock().unwrap().remove(p.key());
        Ok(())
    }
}
#[test]
fn saved_online_settings_do_not_depend_on_a_second_keyring_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let shared = Arc::new(RwLock::new(None));
    let mut service = SettingsService::new(
        Keys {
            fail_reads_after_write: true,
            ..Default::default()
        },
        ConfigSelection::DefaultPath(path.clone()),
        shared.clone(),
    );
    let settings = TranscriptionSettings {
        provider: Provider::Openai,
        ..Default::default()
    };
    let result = service.perform(SettingsRequest::Save {
        settings: settings.clone(),
        credentials: Some(Credentials {
            api_key: "secret-marker".into(),
            ..Default::default()
        }),
    });
    assert!(
        result.is_ok(),
        "success must reflect the committed configuration"
    );
    assert_eq!(*shared.read().unwrap(), Some(settings));
    let content = std::fs::read_to_string(path).unwrap();
    assert!(!content.contains("secret-marker"));
}

#[test]
fn missing_credentials_do_not_enable_online_or_create_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let shared = Arc::new(RwLock::new(None));
    let mut service = SettingsService::new(
        Keys::default(),
        ConfigSelection::DefaultPath(path.clone()),
        shared.clone(),
    );
    assert!(
        service
            .perform(SettingsRequest::Save {
                settings: TranscriptionSettings {
                    provider: Provider::Aliyun,
                    ..Default::default()
                },
                credentials: None
            })
            .is_err()
    );
    assert!(!path.exists());
    assert!(shared.read().unwrap().is_none());
}

#[test]
fn deleting_active_credentials_switches_to_voxtype_and_restart_preserves_selection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let shared = Arc::new(RwLock::new(None));
    let mut service = SettingsService::new(
        Keys::default(),
        ConfigSelection::DefaultPath(path.clone()),
        shared.clone(),
    );
    service
        .perform(SettingsRequest::Save {
            settings: TranscriptionSettings {
                provider: Provider::Openai,
                ..Default::default()
            },
            credentials: Some(Credentials {
                api_key: "secret-marker".into(),
                ..Default::default()
            }),
        })
        .unwrap();
    assert_eq!(
        service
            .perform(SettingsRequest::Inspect)
            .unwrap()
            .settings
            .provider,
        Provider::Openai
    );
    let result = service
        .perform(SettingsRequest::Delete(Provider::Openai))
        .unwrap();
    assert_eq!(result.settings.provider, Provider::Voxtype);
    let mut restart = SettingsService::new(
        Keys::default(),
        ConfigSelection::DefaultPath(path),
        Default::default(),
    );
    assert_eq!(
        restart
            .perform(SettingsRequest::Inspect)
            .unwrap()
            .settings
            .provider,
        Provider::Voxtype
    );
    assert_eq!(
        shared.read().unwrap().as_ref().unwrap().provider,
        Provider::Voxtype
    );
}

#[test]
fn failed_config_write_keeps_the_current_provider() {
    let dir = tempfile::tempdir().unwrap();
    let shared = Arc::new(RwLock::new(Some(TranscriptionSettings::default())));
    let mut service = SettingsService::new(
        Keys::default(),
        ConfigSelection::DefaultPath(dir.path().to_owned()),
        shared.clone(),
    );
    // 指向目录读取失败：设置不允许绕过现有配置错误。
    assert!(
        service
            .perform(SettingsRequest::Save {
                settings: TranscriptionSettings {
                    provider: Provider::Openai,
                    ..Default::default()
                },
                credentials: Some(Credentials {
                    api_key: "secret-marker".into(),
                    ..Default::default()
                })
            })
            .is_err()
    );
    assert_eq!(
        shared.read().unwrap().as_ref().unwrap().provider,
        Provider::Voxtype
    );
}

#[test]
fn failed_persistence_restores_the_previous_keyring_credentials() {
    let keys = Keys::default();
    keys.values.lock().unwrap().insert(
        "openai".into(),
        Credentials {
            api_key: "old-secret".into(),
            ..Default::default()
        },
    );
    let observed = keys.values.clone();
    let shared = Arc::new(RwLock::new(Some(TranscriptionSettings::default())));
    let mut service = SettingsService::new(keys, ConfigSelection::DefaultsOnly, shared.clone());
    let result = service.perform(SettingsRequest::Save {
        settings: TranscriptionSettings {
            provider: Provider::Openai,
            ..Default::default()
        },
        credentials: Some(Credentials {
            api_key: "new-secret".into(),
            ..Default::default()
        }),
    });
    assert!(result.is_err());
    assert_eq!(observed.lock().unwrap()["openai"].api_key, "old-secret");
    assert_eq!(
        shared.read().unwrap().as_ref().unwrap().provider,
        Provider::Voxtype
    );
}
