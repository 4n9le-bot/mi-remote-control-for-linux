//! 完整 Capture 的在线识别边界。厂商错误正文从不进入诊断。
use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    #[default]
    Voxtype,
    Openai,
    Volcengine,
    Aliyun,
}
impl Provider {
    pub fn key(self) -> &'static str {
        match self {
            Self::Voxtype => "voxtype",
            Self::Openai => "openai",
            Self::Volcengine => "volcengine",
            Self::Aliyun => "aliyun",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TranscriptionSettings {
    pub provider: Provider,
    pub openai_model: String,
    pub volcengine_resource: String,
    pub aliyun_region: String,
}
impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            provider: Provider::Voxtype,
            openai_model: "gpt-transcribe".into(),
            volcengine_resource: "volc.bigasr.auc_turbo".into(),
            aliyun_region: "cn-shanghai".into(),
        }
    }
}
impl TranscriptionSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.openai_model.is_empty()
            || self.openai_model.len() > 100
            || !self
                .openai_model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err("OpenAI 模型名称无效".into());
        }
        if self.volcengine_resource != "volc.bigasr.auc_turbo" {
            return Err("火山资源 ID 必须为 volc.bigasr.auc_turbo".into());
        }
        if !["cn-shanghai", "cn-beijing", "cn-shenzhen"].contains(&self.aliyun_region.as_str()) {
            return Err("阿里云区域必须为 cn-shanghai、cn-beijing 或 cn-shenzhen".into());
        }
        Ok(())
    }
}
// 不派生 Debug，避免秘密进入事件、panic 和日志。
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Credentials {
    pub api_key: String,
    pub app_key: String,
    pub access_key_id: String,
    pub access_key_secret: String,
}
impl Credentials {
    pub fn validate(&self, provider: Provider) -> Result<(), String> {
        let fields: &[(&str, &str)] = match provider {
            Provider::Voxtype => &[],
            Provider::Openai => &[("API Key", &self.api_key)],
            Provider::Volcengine => &[("API Key", &self.api_key)],
            Provider::Aliyun => &[
                ("AppKey", &self.app_key),
                ("AccessKey ID", &self.access_key_id),
                ("AccessKey Secret", &self.access_key_secret),
            ],
        };
        for (name, value) in fields {
            if value.trim().is_empty() {
                return Err(format!("缺少 {name}，请配置凭据"));
            }
            if value.len() > 4096 || value.chars().any(char::is_control) {
                return Err(format!("{name} 格式无效"));
            }
        }
        Ok(())
    }
}
pub trait CredentialStore {
    fn read(&mut self, provider: Provider) -> Result<Option<Credentials>, String>;
    fn write(&mut self, provider: Provider, credentials: &Credentials) -> Result<(), String>;
    fn delete(&mut self, provider: Provider) -> Result<(), String>;
}
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub timeout: Duration,
}
pub struct HttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
pub trait HttpTransport {
    fn send(&mut self, request: HttpRequest) -> Result<HttpResponse, String>;
}

pub struct OnlineTranscriber<H, K> {
    http: H,
    keys: K,
    cancel: Arc<AtomicBool>,
    token: Option<(String, String, String, u64)>,
}
impl<H: HttpTransport, K: CredentialStore> OnlineTranscriber<H, K> {
    pub fn new(http: H, keys: K, cancel: Arc<AtomicBool>) -> Self {
        Self {
            http,
            keys,
            cancel,
            token: None,
        }
    }
    pub fn http(&self) -> &H {
        &self.http
    }
    pub fn transcribe(
        &mut self,
        settings: &TranscriptionSettings,
        wav: &[u8],
    ) -> Result<Option<String>, String> {
        settings.validate()?;
        if self.cancel.load(Ordering::Relaxed) {
            return Err("识别已取消".into());
        }
        if wav.len() < 44 || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
            return Err("Capture WAV 格式无效".into());
        }
        if settings.provider == Provider::Openai && wav.len() > 25_000_000 {
            return Err("OpenAI 音频超过 25 MB，已保留 WAV".into());
        }
        if settings.provider == Provider::Volcengine && wav.len() > 100_000_000 {
            return Err("火山音频超过 100 MB，已保留 WAV".into());
        }
        if settings.provider == Provider::Aliyun && wav.len() - 44 > 60 * 16_000 * 2 {
            return Err("阿里云一句话识别最多支持 60 秒，已保留 WAV".into());
        }
        let deadline = Instant::now() + Duration::from_secs(60);
        let keys = self
            .keys
            .read(settings.provider)?
            .ok_or("未找到凭据或密钥环未解锁，请解锁后打开语音识别设置")?;
        keys.validate(settings.provider)?;
        let mut headers = BTreeMap::new();
        let (url, body) = match settings.provider {
            Provider::Voxtype => return Err("Voxtype 由本地识别路径执行".into()),
            Provider::Openai => {
                let boundary = format!("atvv-{}", uuid::Uuid::new_v4());
                headers.insert("Authorization".into(), format!("Bearer {}", keys.api_key));
                headers.insert(
                    "Content-Type".into(),
                    format!("multipart/form-data; boundary={boundary}"),
                );
                let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n{}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"capture.wav\"\r\nContent-Type: audio/wav\r\n\r\n", settings.openai_model).into_bytes();
                body.extend_from_slice(wav);
                body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
                (
                    "https://api.openai.com/v1/audio/transcriptions".to_owned(),
                    body,
                )
            }
            Provider::Volcengine => {
                headers.insert("Content-Type".into(), "application/json".into());
                headers.insert("X-Api-Key".into(), keys.api_key);
                headers.insert(
                    "X-Api-Resource-Id".into(),
                    settings.volcengine_resource.clone(),
                );
                headers.insert("X-Api-Request-Id".into(), uuid::Uuid::new_v4().to_string());
                headers.insert("X-Api-Sequence".into(), "-1".into());
                let body = serde_json::to_vec(&json!({
                    "user": {"uid": "atvv-bridge"},
                    "audio": {"data": STANDARD.encode(wav), "format": "wav", "codec": "raw", "rate": 16000, "bits": 16, "channel": 1},
                    "request": {"model_name": "bigmodel", "enable_itn": true, "enable_punc": true}
                })).expect("JSON serialization");
                (
                    "https://openspeech.bytedance.com/api/v3/auc/bigmodel/recognize/flash".into(),
                    body,
                )
            }
            Provider::Aliyun => {
                let token = self.aliyun_token(&keys, deadline)?;
                headers.insert("X-NLS-Token".into(), token);
                headers.insert("Content-Type".into(), "application/octet-stream".into());
                let mut url = reqwest::Url::parse(&format!(
                    "https://nls-gateway-{}.aliyuncs.com/stream/v1/asr",
                    settings.aliyun_region
                ))
                .expect("fixed URL");
                url.query_pairs_mut()
                    .append_pair("appkey", &keys.app_key)
                    .append_pair("format", "wav")
                    .append_pair("sample_rate", "16000")
                    .append_pair("enable_punctuation_prediction", "true");
                (url.to_string(), wav.to_vec())
            }
        };
        let response = self.send(HttpRequest {
            method: "POST",
            url,
            headers,
            body,
            timeout: deadline.saturating_duration_since(Instant::now()),
        })?;
        if settings.provider == Provider::Volcengine {
            let code = response
                .headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("x-api-status-code"))
                .map(|(_, value)| value.as_str());
            match code {
                Some("20000000") => {}
                Some("20000003") => return Ok(None),
                Some(_) => return Err("火山识别失败，请检查语音服务权限、资源配额与凭据".into()),
                None => return Err("火山响应缺少状态码".into()),
            }
        }
        let value: Value =
            serde_json::from_slice(&response.body).map_err(|_| "服务返回无效 JSON，请稍后重试")?;
        let text = match settings.provider {
            Provider::Openai => value.get("text"),
            Provider::Volcengine => value.pointer("/result/text"),
            Provider::Aliyun => {
                if value.get("status").and_then(Value::as_i64) != Some(20000000) {
                    return Err("阿里识别失败，请检查 AppKey、权限与音频限制".into());
                }
                value.get("result")
            }
            Provider::Voxtype => unreachable!(),
        }
        .and_then(Value::as_str)
        .ok_or("服务响应缺少识别文本")?;
        Ok((!text.trim().is_empty()).then(|| text.to_owned()))
    }
    fn send(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err("识别已取消".into());
        }
        if request.timeout.is_zero() {
            return Err("在线识别超时，请稍后重试".into());
        }
        let result = self.http.send(request)?;
        if self.cancel.load(Ordering::Relaxed) {
            return Err("识别已取消".into());
        }
        match result.status {
            200..=299 => Ok(result),
            401 | 403 => Err("在线服务鉴权失败，请检查凭据与权限".into()),
            429 => Err("在线服务限流，请稍后重试".into()),
            _ => Err("在线服务请求失败，请检查服务状态".into()),
        }
    }
    fn aliyun_token(&mut self, keys: &Credentials, deadline: Instant) -> Result<String, String> {
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        if let Some((id, secret, token, expires)) = &self.token {
            if id == &keys.access_key_id && secret == &keys.access_key_secret && *expires > now + 60
            {
                return Ok(token.clone());
            }
        }
        let request = aliyun_token_request(
            keys,
            &chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            &uuid::Uuid::new_v4().to_string(),
            deadline.saturating_duration_since(Instant::now()),
        );
        let response = self.send(request)?;
        let value: Value =
            serde_json::from_slice(&response.body).map_err(|_| "阿里 Token 响应无效")?;
        let token = value
            .pointer("/Token/Id")
            .and_then(Value::as_str)
            .ok_or("无法获取阿里 Token，请检查 AccessKey 权限")?
            .to_owned();
        let expires = value
            .pointer("/Token/ExpireTime")
            .and_then(Value::as_u64)
            .ok_or("阿里 Token 缺少有效期")?;
        if expires <= now + 60 {
            return Err("阿里 Token 已过期，请检查系统时间".into());
        }
        self.token = Some((
            keys.access_key_id.clone(),
            keys.access_key_secret.clone(),
            token.clone(),
            expires,
        ));
        Ok(token)
    }
}
/// 构造阿里 Token 请求；时间与 nonce 由调用者提供，便于协议固定向量验证。
pub fn aliyun_token_request(
    keys: &Credentials,
    timestamp: &str,
    nonce: &str,
    timeout: Duration,
) -> HttpRequest {
    let mut params = BTreeMap::from([
        ("AccessKeyId", keys.access_key_id.clone()),
        ("Action", "CreateToken".into()),
        ("Version", "2019-02-28".into()),
        ("Format", "JSON".into()),
        ("RegionId", "cn-shanghai".into()),
        ("SignatureMethod", "HMAC-SHA1".into()),
        ("SignatureVersion", "1.0".into()),
        ("SignatureNonce", nonce.to_owned()),
        ("Timestamp", timestamp.to_owned()),
    ]);
    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let sign = format!("GET&%2F&{}", percent_encode(&query));
    let mut mac =
        Hmac::<sha1::Sha1>::new_from_slice(format!("{}&", keys.access_key_secret).as_bytes())
            .expect("HMAC key");
    mac.update(sign.as_bytes());
    params.insert("Signature", STANDARD.encode(mac.finalize().into_bytes()));
    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    HttpRequest {
        method: "GET",
        url: format!("https://nls-meta.cn-shanghai.aliyuncs.com/?{query}"),
        headers: BTreeMap::new(),
        body: Vec::new(),
        timeout,
    }
}
fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

pub struct SystemHttp {
    cancel: Arc<AtomicBool>,
}
impl SystemHttp {
    pub fn new(cancel: Arc<AtomicBool>) -> Self {
        Self { cancel }
    }
}
impl HttpTransport for SystemHttp {
    fn send(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "无法启动网络请求")?;
        runtime.block_on(async {
            let operation = async {
                let client = reqwest::Client::builder()
                    .retry(reqwest::retry::never())
                    .redirect(reqwest::redirect::Policy::none())
                    .connect_timeout(Duration::from_secs(10))
                    .timeout(request.timeout)
                    .build().map_err(|_| "无法创建安全 HTTP 客户端")?;
                let method = reqwest::Method::from_bytes(request.method.as_bytes()).map_err(|_| "请求方法无效")?;
                let mut builder = client.request(method, request.url);
                for (key, value) in request.headers { builder = builder.header(key, value); }
                let mut response = builder.body(request.body).send().await.map_err(|error| {
                    if error.is_timeout() { "在线请求超时，请稍后重试" } else { "在线请求失败，请检查网络与服务连接" }
                })?;
                let status = response.status().as_u16();
                let headers = response.headers().iter().filter_map(|(name, value)| value.to_str().ok().map(|v| (name.to_string(), v.to_owned()))).collect();
                let mut body = Vec::new();
                while let Some(chunk) = response.chunk().await.map_err(|_| "读取在线响应失败")? {
                    if body.len() + chunk.len() > 2_000_000 { return Err("在线响应超过安全大小限制".to_owned()); }
                    body.extend_from_slice(&chunk);
                }
                Ok(HttpResponse { status, headers, body })
            };
            let cancelled = async { loop { if self.cancel.load(Ordering::Relaxed) { break; } tokio::time::sleep(Duration::from_millis(25)).await; } };
            tokio::select! { result = tokio::time::timeout(request.timeout, operation) => result.map_err(|_| "在线请求超时，请稍后重试".to_owned())?, _ = cancelled => Err("识别已取消".into()) }
        })
    }
}
