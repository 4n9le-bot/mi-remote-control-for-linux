use atvv_bridge::transcription::{
    CredentialStore, Credentials, HttpRequest, HttpResponse, HttpTransport, OnlineTranscriber,
    Provider, TranscriptionSettings,
};
use std::sync::{Arc, atomic::AtomicBool};

struct Keys;
impl CredentialStore for Keys {
    fn read(&mut self, _: Provider) -> Result<Option<Credentials>, String> {
        Ok(Some(Credentials {
            api_key: "test-secret".into(),
            app_key: "test-app".into(),
            access_key_id: "test-id".into(),
            access_key_secret: "test-key-secret".into(),
        }))
    }
    fn write(&mut self, _: Provider, _: &Credentials) -> Result<(), String> {
        Ok(())
    }
    fn delete(&mut self, _: Provider) -> Result<(), String> {
        Ok(())
    }
}
#[derive(Default)]
struct Http {
    requests: Vec<HttpRequest>,
    responses: std::collections::VecDeque<Result<HttpResponse, String>>,
}
impl HttpTransport for Http {
    fn send(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
        self.requests.push(request);
        if let Some(response) = self.responses.pop_front() {
            return response;
        }
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: r#"{"text":"你好\n世界"}"#.as_bytes().to_vec(),
        })
    }
}

fn wav() -> Vec<u8> {
    let mut wav = vec![0; 46];
    wav[..4].copy_from_slice(b"RIFF");
    wav[8..12].copy_from_slice(b"WAVE");
    wav
}
fn response(body: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: Default::default(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}
fn token() -> HttpResponse {
    response(
        serde_json::json!({"Token":{"Id":"token-marker", "ExpireTime":chrono::Utc::now().timestamp()+3600}}),
    )
}

#[test]
fn volcengine_uploads_base64_wav_with_new_api_key_authentication() {
    let mut reply = response(serde_json::json!({"result":{"text":"火山文本"}}));
    reply
        .headers
        .insert("x-api-status-code".into(), "20000000".into());
    let mut service = OnlineTranscriber::new(
        Http {
            responses: [Ok(reply)].into(),
            ..Default::default()
        },
        Keys,
        Arc::new(AtomicBool::new(false)),
    );
    let settings = TranscriptionSettings {
        provider: Provider::Volcengine,
        ..Default::default()
    };
    assert_eq!(
        service.transcribe(&settings, &wav()).unwrap(),
        Some("火山文本".into())
    );
    let request = &service.http().requests[0];
    assert_eq!(request.headers["X-Api-Key"], "test-secret");
    assert_eq!(
        request.headers["X-Api-Resource-Id"],
        "volc.bigasr.auc_turbo"
    );
    assert_eq!(request.headers["X-Api-Sequence"], "-1");
    assert!(!request.headers.contains_key("X-Api-App-Key"));
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    use base64::Engine;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(body["audio"]["data"].as_str().unwrap())
            .unwrap(),
        wav()
    );
}

#[test]
fn aliyun_gets_a_signed_token_then_uploads_wav_and_reuses_unexpired_token() {
    let reply = || response(serde_json::json!({"status":20000000, "result":"阿里文本"}));
    let mut service = OnlineTranscriber::new(
        Http {
            responses: [Ok(token()), Ok(reply()), Ok(reply())].into(),
            ..Default::default()
        },
        Keys,
        Arc::new(AtomicBool::new(false)),
    );
    let settings = TranscriptionSettings {
        provider: Provider::Aliyun,
        ..Default::default()
    };
    for _ in 0..2 {
        assert_eq!(
            service.transcribe(&settings, &wav()).unwrap(),
            Some("阿里文本".into())
        );
    }
    assert_eq!(service.http().requests.len(), 3);
    let token_request = &service.http().requests[0];
    assert_eq!(token_request.method, "GET");
    let url = reqwest::Url::parse(&token_request.url).unwrap();
    let params = url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(params["Action"], "CreateToken");
    assert_eq!(params["SignatureMethod"], "HMAC-SHA1");
    assert!(!url.as_str().contains("test-key-secret"));
    let upload = &service.http().requests[1];
    assert_eq!(upload.method, "POST");
    assert_eq!(upload.headers["X-NLS-Token"], "token-marker");
    assert_eq!(upload.body, wav());
    assert!(upload.url.contains("format=wav"));
}

#[test]
fn provider_failures_are_redacted_and_never_retried() {
    for provider in [Provider::Openai, Provider::Volcengine, Provider::Aliyun] {
        for status in [401, 403, 429, 503] {
            let mut responses = std::collections::VecDeque::new();
            if provider == Provider::Aliyun {
                responses.push_back(Ok(token()));
            }
            responses.push_back(Ok(HttpResponse {
                status,
                headers: Default::default(),
                body: b"test-secret audio-marker transcript-marker".to_vec(),
            }));
            let mut service = OnlineTranscriber::new(
                Http {
                    responses,
                    ..Default::default()
                },
                Keys,
                Arc::new(AtomicBool::new(false)),
            );
            let error = service
                .transcribe(
                    &TranscriptionSettings {
                        provider,
                        ..Default::default()
                    },
                    &wav(),
                )
                .unwrap_err();
            assert!(!error.contains("marker") && !error.contains("test-secret"));
            assert_eq!(
                service.http().requests.len(),
                if provider == Provider::Aliyun { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn no_speech_and_malformed_responses_are_handled_for_all_providers() {
    for provider in [Provider::Openai, Provider::Volcengine, Provider::Aliyun] {
        for malformed in [false, true] {
            let body = match provider {
                Provider::Openai => serde_json::json!({"text":"  "}),
                Provider::Volcengine => serde_json::json!({"result":{"text":""}}),
                _ => serde_json::json!({"status":20000000,"result":""}),
            };
            let mut reply = response(body);
            if provider == Provider::Volcengine {
                reply
                    .headers
                    .insert("x-api-status-code".into(), "20000000".into());
            }
            if malformed {
                reply.body = b"not-json secret-marker".to_vec();
            }
            let mut responses = std::collections::VecDeque::new();
            if provider == Provider::Aliyun {
                responses.push_back(Ok(token()));
            }
            responses.push_back(Ok(reply));
            let mut service = OnlineTranscriber::new(
                Http {
                    responses,
                    ..Default::default()
                },
                Keys,
                Arc::new(AtomicBool::new(false)),
            );
            let result = service.transcribe(
                &TranscriptionSettings {
                    provider,
                    ..Default::default()
                },
                &wav(),
            );
            if malformed {
                assert!(result.is_err());
                assert!(!result.unwrap_err().contains("secret-marker"));
            } else {
                assert_eq!(result.unwrap(), None);
            }
        }
    }
}

#[test]
fn cancelled_requests_and_aliyun_overlong_audio_do_not_upload() {
    let mut service =
        OnlineTranscriber::new(Http::default(), Keys, Arc::new(AtomicBool::new(true)));
    assert!(
        service
            .transcribe(
                &TranscriptionSettings {
                    provider: Provider::Openai,
                    ..Default::default()
                },
                &wav()
            )
            .is_err()
    );
    assert!(service.http().requests.is_empty());
    let mut service =
        OnlineTranscriber::new(Http::default(), Keys, Arc::new(AtomicBool::new(false)));
    let mut long = wav();
    long.resize(44 + 60 * 16000 * 2 + 2, 0);
    assert!(
        service
            .transcribe(
                &TranscriptionSettings {
                    provider: Provider::Aliyun,
                    ..Default::default()
                },
                &long
            )
            .unwrap_err()
            .contains("60 秒")
    );
    assert!(service.http().requests.is_empty());
}

#[test]
fn aliyun_token_request_matches_the_official_offline_signature_vector() {
    let request = atvv_bridge::transcription::aliyun_token_request(
        &Credentials {
            access_key_id: "my_access_key_id".into(),
            access_key_secret: "my_access_key_secret".into(),
            ..Default::default()
        },
        "2019-04-18T08:32:31Z",
        "b924c8c3-6d03-4c5d-ad36-d984d3116788",
        std::time::Duration::from_secs(60),
    );
    let url = reqwest::Url::parse(&request.url).unwrap();
    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "Signature")
            .unwrap()
            .1,
        "hHq4yNsPitlfDJ2L0nQPdugdEzM="
    );
}

#[test]
fn timeout_or_network_failure_is_not_retried_for_any_provider() {
    for provider in [Provider::Openai, Provider::Volcengine, Provider::Aliyun] {
        let mut replies = std::collections::VecDeque::new();
        if provider == Provider::Aliyun {
            replies.push_back(Ok(token()));
        }
        replies.push_back(Err("在线请求超时，请稍后重试".into()));
        let mut service = OnlineTranscriber::new(
            Http {
                responses: replies,
                ..Default::default()
            },
            Keys,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(
            service
                .transcribe(
                    &TranscriptionSettings {
                        provider,
                        ..Default::default()
                    },
                    &wav()
                )
                .unwrap_err()
                .contains("超时")
        );
        assert_eq!(
            service.http().requests.len(),
            if provider == Provider::Aliyun { 2 } else { 1 }
        );
    }
}

struct UnavailableKeys;
impl CredentialStore for UnavailableKeys {
    fn read(&mut self, _: Provider) -> Result<Option<Credentials>, String> {
        Err("密钥环不可用或锁定，请解锁后重试".into())
    }
    fn write(&mut self, _: Provider, _: &Credentials) -> Result<(), String> {
        Err("密钥环不可用".into())
    }
    fn delete(&mut self, _: Provider) -> Result<(), String> {
        Err("密钥环不可用".into())
    }
}
#[test]
fn locked_keyring_never_uploads_audio() {
    for provider in [Provider::Openai, Provider::Volcengine, Provider::Aliyun] {
        let mut service = OnlineTranscriber::new(
            Http::default(),
            UnavailableKeys,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(
            service
                .transcribe(
                    &TranscriptionSettings {
                        provider,
                        ..Default::default()
                    },
                    &wav()
                )
                .unwrap_err()
                .contains("解锁")
        );
        assert!(service.http().requests.is_empty());
    }
}

#[test]
fn system_http_cancels_an_inflight_request_without_waiting_for_server_response() {
    use std::{
        io::Read,
        net::TcpListener,
        sync::{atomic::Ordering, mpsc},
        thread,
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let (received, request_arrived) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut data = [0; 4096];
        assert!(stream.read(&mut data).unwrap() > 0);
        received.send(()).unwrap();
        // 没有响应；等待客户端取消并关闭连接。
        while let Ok(count) = stream.read(&mut data) {
            if count == 0 {
                break;
            }
        }
    });
    let (completed, result) = mpsc::channel();
    let worker_cancel = cancel.clone();
    let worker = thread::spawn(move || {
        let mut http = atvv_bridge::transcription::SystemHttp::new(worker_cancel);
        let reply = http.send(HttpRequest {
            method: "POST",
            url: format!("http://{address}/"),
            headers: Default::default(),
            body: vec![],
            timeout: Duration::from_secs(30),
        });
        completed.send(reply.map(|_| ())).unwrap();
    });
    request_arrived
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert!(
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap_err()
            .contains("取消")
    );
    worker.join().unwrap();
    server.join().unwrap();
}
#[test]
fn openai_uploads_completed_wav_and_preserves_transcript_content() {
    let mut service =
        OnlineTranscriber::new(Http::default(), Keys, Arc::new(AtomicBool::new(false)));
    let settings = TranscriptionSettings {
        provider: Provider::Openai,
        ..Default::default()
    };
    let mut wav = vec![0; 46];
    wav[..4].copy_from_slice(b"RIFF");
    wav[8..12].copy_from_slice(b"WAVE");
    assert_eq!(
        service.transcribe(&settings, &wav).unwrap(),
        Some("你好\n世界".into())
    );
    let request = &service.http().requests[0];
    assert_eq!(
        request.url,
        "https://api.openai.com/v1/audio/transcriptions"
    );
    assert_eq!(request.headers["Authorization"], "Bearer test-secret");
    assert!(request.body.windows(wav.len()).any(|bytes| bytes == wav));
}
