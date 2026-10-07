# 在线语音识别 Provider 调研

日期：2026-10-07。用于 #41 的实现 brief；只查文档，没有调用付费 API。

## 已确认的产品行为

- 默认继续使用本地 Voxtype。
- 用户在图形界面主动开启在线模式，并选择 OpenAI、火山引擎或阿里云。
- 凭据保存在系统密钥环；配置文件只保存非敏感选项。
- 录音结束后识别整段音频；结果沿用 Fcitx 5 输入流程。
- 在线识别失败时报错并保留 WAV，不自动回退 Voxtype。用户可以手动切回 Voxtype。
- 输入为现有捕获流程产生的 16 kHz、单声道、16 位 PCM WAV。

## 核验状态

已实际读取 OpenAI 文件识别指南、阿里云一句话识别 RESTful API、Token 签名说明及火山极速版/标准版官方正文。火山动态网页正文通过其公开客户端使用的 `getDocDetail` 接口取得。未调用真实语音 API。

## OpenAI：Audio Transcriptions

已核验来源：[官方文件识别指南](https://developers.openai.com/api/docs/guides/speech-to-text/)。

- `POST https://api.openai.com/v1/audio/transcriptions`。
- 鉴权为 `Authorization: Bearer <API Key>`；首版只要求用户提供 API Key。
- multipart/form-data 提交本地 `file` 和 `model`，无需公开 URL。
- 官方指南当前推荐模型为 `gpt-transcribe`，适用于录音原语言转写。首版采用该值；不要按过时印象把 `gpt-4o-mini-transcribe` 当成当前推荐默认。
- 官方列出的输入格式包含 WAV；上传文件上限 25 MB。
- 返回转写文本；示例读取 `transcription.text`。首版使用非流式 JSON 结果。
- 指南未在所读位置给出所有 WAV 编码限制；实施时以现有 PCM16 WAV 作为实际输入做 mock 请求格式验证，使用者提供凭据后的联调再确认服务接受条件。

## 阿里云：NLS 一句话识别 RESTful API

已核验来源：[一句话识别 RESTful API](https://help.aliyun.com/zh/isi/developer-reference/restful-api-2)、[获取 Token](https://help.aliyun.com/zh/isi/user-guide/obtain-an-access-token-1/)。

- 上海公网地址 `https://nls-gateway-cn-shanghai.aliyuncs.com/stream/v1/asr`；官方另列北京、深圳公网地址。
- `POST`，`X-NLS-Token` 请求头，查询参数 `appkey`、`format=wav`、`sample_rate=16000`；请求体为完整 WAV 二进制，`Content-Type: application/octet-stream`。无需 OSS 或公开 URL。
- 支持单声道、16 bit，8000/16000 Hz，PCM 编码 WAV，符合本项目音频格式。
- **单次音频不得超过 1 分钟**；处理完成后一次性返回 JSON。不得静默截断更长录音。
- 凭据字段为 AppKey、AccessKey ID、AccessKey Secret。用 AccessKey 获取临时 NLS Token，按响应 `ExpireTime` 刷新；官方明确不提供永久 Token。
- AppKey 与获取 Token 的账号须匹配，RAM 身份需相应 NLS 权限。Token 请求签名和返回字段见下文。

工程选择：首版可采用上述已核验接口，并明确在阿里模式对超过 60 秒的录音报错、保留 WAV；如实施目标要求长录音，则在录音完成后按实时识别 WebSocket 协议发送已录 PCM，并等待最终结果。后一方案无需增加边录边显示文本，但其协议、分帧节奏和限制尚未核验。不得将未经核验的 WebSocket 路线宣称为已实现或已验证。

## 阿里云 Token 签名详细契约

已核验来源：[通过 HTTP 或 HTTPS 获取 Token](https://help.aliyun.com/zh/isi/user-guide/use-http-or-https-to-obtain-an-access-token)。

`GET https://nls-meta.cn-shanghai.aliyuncs.com/`，参数为 `AccessKeyId`、`Action=CreateToken`、`Version=2019-02-28`、`RegionId=cn-shanghai`、`Format=JSON`、`SignatureMethod=HMAC-SHA1`、`SignatureVersion=1.0`、随机唯一 `SignatureNonce`、UTC `Timestamp`（`YYYY-MM-DDTHH:MM:SSZ`）。

1. 不包括 `Signature`，按参数名区分大小写字典序排序。
2. 分别对参数名和值做 UTF-8 百分号编码：仅 `A-Z a-z 0-9 - _ . ~` 不编码；空格 `%20`，不能使用表单编码 `+`。以 `=` 和 `&` 连接得到 canonical query。
3. `string_to_sign = "GET&%2F&" + percent_encode(canonical_query)`，即对 query 再编码一次。
4. `signature = Base64(HMAC-SHA1(UTF8(AccessKeySecret + "&"), UTF8(string_to_sign)))`。
5. 将 `&Signature=` 加上百分号编码后的 signature，追加到 query；不应把签名、密钥或完整鉴权 URL 打印到日志。

官方固定离线示例（全部为虚拟凭据，不可用于请求）：

```text
AccessKeyId=my_access_key_id
AccessKeySecret=my_access_key_secret
SignatureNonce=b924c8c3-6d03-4c5d-ad36-d984d3116788
Timestamp=2019-04-18T08:32:31Z
CanonicalQuery=AccessKeyId=my_access_key_id&Action=CreateToken&Format=JSON&RegionId=cn-shanghai&SignatureMethod=HMAC-SHA1&SignatureNonce=b924c8c3-6d03-4c5d-ad36-d984d3116788&SignatureVersion=1.0&Timestamp=2019-04-18T08%3A32%3A31Z&Version=2019-02-28
Signature=hHq4yNsPitlfDJ2L0nQPdugdEzM=
EncodedSignature=hHq4yNsPitlfDJ2L0nQPdugdEzM%3D
```

成功 HTTP 200 JSON 返回 `Token.Id` 和 `Token.ExpireTime`。`ExpireTime` 是 Unix 秒绝对时间，不能当成 TTL 秒数。失败 HTTP 非 200，读取脱敏后的 `Code` 和 `Message`。可用官方固定签名示例写离线测试，无需真实密钥。

## 火山引擎：录音文件识别极速版

已核验来源：[极速版 HTTP 文档](https://www.volcengine.com/docs/6561/1631584)、[标准版 HTTP 文档](https://www.volcengine.com/docs/6561/1354868)。公开正文取得地址：[getDocDetail 极速版](https://www.volcengine.com/api/doc/getDocDetail?LibraryID=6561&DocumentID=1631584)、[getDocDetail 标准版](https://www.volcengine.com/api/doc/getDocDetail?LibraryID=6561&DocumentID=1354868)。

- `POST https://openspeech.bytedance.com/api/v3/auc/bigmodel/recognize/flash`，JSON 请求，一次返回结果，无需 submit/query 轮询。
- **新版控制台首版只需 APP Key/API Key**，请求头为 `X-Api-Key`。旧版控制台才使用 `X-Api-App-Key`（App ID）与 `X-Api-Access-Key`（Access Token）组合。既然本项目是新适配器，推荐只实现新版 API Key，避免无需求的旧版兼容。
- 共同请求头：`X-Api-Resource-Id: volc.bigasr.auc_turbo`、`X-Api-Request-Id: <random UUID>`、`X-Api-Sequence: -1`，`Content-Type: application/json`。账户须开通资源权限。
- `audio.data` 提交完整本地 WAV 的标准 Base64，不能同时设置 `audio.url`；不需要公开 URL 或 OSS。
- JSON `request.model_name` 为 `bigmodel`。可设置 `audio.format=wav`、`audio.codec=raw`、`audio.rate=16000`、`audio.bits=16`、`audio.channel=1`，字段规格见极速版引用的标准版请求表。示例的 `user.uid` 可采用应用自己的非敏感随机标识，无需发送遥控器 MAC。
- 限制：音频时长不超过 2 小时、音频大小不超过 100 MB，支持 WAV/MP3/OGG OPUS；官方建议二进制上传尽量在 20M 以内，取决于出口带宽。Base64 会扩大请求体，不能把编码后字符串长度错误当作原始音频大小。
- HTTP 成功仍须检查业务响应头 `X-Api-Status-Code`。`20000000` 为成功，文本位于 JSON `result.text`。
- `20000003` 为静音音频：映射为“未识别到语音”，不能当普通转写成功后执行空文本提交。`45000002` 为空音频，`45000151` 为格式错误，`45000001` 为参数错误，`55000031` 为服务器繁忙；`550XXXX` 为内部错误。
- `X-Api-Message` 为服务消息，`X-Tt-Logid` 可用于脱敏诊断；不要输出原始音频或凭据。

可用请求骨架：

```json
{
  "user": { "uid": "<application-session-id>" },
  "audio": {
    "data": "<base64-of-complete-wav>",
    "format": "wav", "codec": "raw", "rate": 16000, "bits": 16, "channel": 1
  },
  "request": { "model_name": "bigmodel" }
}
```

## 实施边界和验证

Provider 接收完整 WAV/PCM 与非敏感配置，读取对应密钥环凭据，返回识别文本或结构化错误。捕获、解码与 Fcitx 5 输入不感知厂商协议。

配置缺省与迁移默认 Voxtype。启用在线模式前检查必需凭据；缺失、密钥环锁定或不可用时显示可操作错误，不能写入明文配置。不得记录 Authorization、Access Token、签名或音频 Base64。

本地 mock 覆盖上传格式、鉴权失败、限流、超时、无效响应、空文本及失败保留 WAV。API 产品自身的时长/体积限制必须在请求前验证并提示，不能静默截断。真实付费联调留作使用者提供凭据后的独立验证。

阿里首版须对超过 60 秒的录音明确报错并保留 WAV；若后续要求长录音，再核验其他产品或 WebSocket 方案。三家协议文档已足以实现短录音 Provider；真实账户权限、配额和服务接受情况仍需独立联调。
