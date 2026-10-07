# ATVV Voice Bridge

`atvv-bridge` 通过 BlueZ 接收遥控器语音，将完整 Capture 解码为 WAV，
默认交给 Voxtype 识别，也可选择 OpenAI、火山引擎或阿里云在线识别。
识别结果通过 Fcitx 5 输入当前应用。

The currently supported remote is the Xiaomi Bluetooth Remote Control 2 Pro
(`2717:32b8`) using the certified ATVV v1.0 hold-to-talk profile.

## Requirements

- A Linux desktop using BlueZ and Fcitx 5.
- A paired Xiaomi Bluetooth Remote Control 2 Pro.
- 默认本地后端需要 [`voxtype`](https://github.com/peteonrails/voxtype) 位于用户会话的 `PATH`；在线后端不依赖 Voxtype。
- `fcitx5-commit` available in the user session's `PATH`.
- 在线模式需要所选云服务凭据，以及当前用户的 Secret Service 密钥环
  （例如 GNOME Keyring 或支持 Secret Service 的 KWallet）和 `secret-tool`。
  Debian 包依赖 `libsecret-tools`；无需在线识别时不会访问密钥环。

Voxtype and `fcitx5-commit` are runtime integrations rather than Debian package
dependencies because their installation and configuration are desktop-specific.

## Build the Debian package

Install Rust 1.85 or newer and the Debian build tools:

```sh
sudo apt install bubblewrap cargo dpkg-dev jq systemd
```

Run the package integration test, then build the package:

```sh
tests/debian-package.sh
scripts/build-deb.sh
```

The package is written to `target/debian/` by default. To select another output
directory, pass it as the only argument:

```sh
scripts/build-deb.sh /tmp/atvv-packages
```

The build uses the locked Cargo dependency versions and the architecture
reported by `dpkg`. The package test uses Bubblewrap for an unprivileged root
namespace and verifies desktop and autostart entries, runtime dependencies,
hardware database compilation, upgrades, and removal behavior.

To build only the release binary:

```sh
cargo build --release --locked
```

## Install

Install the generated package explicitly:

```sh
sudo apt install ./target/debian/atvv-bridge_*.deb
```

The package installs an application-menu entry and a system-wide XDG autostart
entry. ATVV Voice Bridge starts with each graphical desktop session and uses
that user's XDG configuration. To start it immediately after installation,
select **ATVV Voice Bridge** from the application menu.

Where a StatusNotifier tray is available, closing the status window keeps voice
input running and the tray menu provides **Show Status** and **Quit** actions.
Without a tray, closing the window explains that voice input will stop and asks
for confirmation.

The package also installs and compiles a hardware database entry for the
supported remote. Voice and Power default to Disabled; every other certified
Physical Button retains its native behavior. Button Mapping overrides survive
package upgrades. Disconnect and reconnect the remote after changing a mapping,
installing, or removing the package so the input device uses the compiled
mapping.

## Configure

The bridge reads the first applicable standard path:

1. `$XDG_CONFIG_HOME/atvv-bridge/config.toml`, when `XDG_CONFIG_HOME` is an
   absolute path.
2. `$HOME/.config/atvv-bridge/config.toml`.
3. Built-in defaults when neither location is available or the selected
   standard file does not exist.

Create the standard configuration directory and file:

```sh
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/atvv-bridge"
${EDITOR:-vi} "${XDG_CONFIG_HOME:-$HOME/.config}/atvv-bridge/config.toml"
```

Example configuration showing all available settings:

```toml
max_duration_secs = 60
wav_dir = "/tmp/atvv-bridge"
keep_wav = false

[transcription]
provider = "voxtype"
openai_model = "gpt-transcribe"
volcengine_resource = "volc.bigasr.auc_turbo"
aliyun_region = "cn-shanghai"
```

| Setting | Default | Description |
| --- | --- | --- |
| `max_duration_secs` | `60` | Capture safety limit in seconds; must be between `1` and `3600`. |
| `wav_dir` | `"/tmp/atvv-bridge"` | Writable directory for temporary and retained Capture WAV files; created at startup when absent. |
| `keep_wav` | `false` | Retain WAV files after successful Text Commit. Failures retain their WAV regardless so it can be diagnosed or recovered. |
| `transcription.provider` | `"voxtype"` | 可选 `voxtype`、`openai`、`volcengine`、`aliyun`。旧配置继续默认 Voxtype。 |
| `transcription.openai_model` | `"gpt-transcribe"` | OpenAI 转写模型名称。 |
| `transcription.volcengine_resource` | `"volc.bigasr.auc_turbo"` | 火山录音文件识别极速版固定资源 ID。 |
| `transcription.aliyun_region` | `"cn-shanghai"` | 阿里语音网关区域：上海、北京或深圳，对应 `cn-shanghai`、`cn-beijing`、`cn-shenzhen`。 |

## 在线语音识别

打开应用的「语音识别」页面，选择服务，填写凭据，再点击「保存并使用」。
保存前不会改变当前识别服务；新设置作用于下一次 Capture。
修改已有凭据时填写该服务的全部字段；全部留空则保留现有凭据。
应用只显示是否已配置，不回显秘密。凭据通过 `secret-tool` 存入当前用户系统密钥环，
配置文件不接受 API Key、Token 或 AccessKey Secret，也没有明文兜底。

| 服务 | 产品及所需凭据 | 音频限制 |
| --- | --- | --- |
| OpenAI | [Audio Transcriptions](https://developers.openai.com/api/docs/guides/speech-to-text/)，在 OpenAI 项目创建 API Key，确认项目可使用所选模型。 | WAV 最大 25 MB。 |
| 火山引擎 | [豆包语音大模型录音文件识别极速版](https://www.volcengine.com/docs/6561/1631584)，使用新版控制台创建的 API Key，并开通 `volc.bigasr.auc_turbo` 资源。 | WAV 最大 100 MB、最长 2 小时；桥接自身 Capture 上限为 1 小时。 |
| 阿里云 | [NLS 一句话识别](https://help.aliyun.com/zh/isi/developer-reference/restful-api-2)，在智能语音交互控制台创建项目取得 AppKey，再使用同账号具备语音服务权限的 RAM AccessKey ID/Secret。桥接按[官方协议](https://help.aliyun.com/zh/isi/user-guide/use-http-or-https-to-obtain-an-access-token)获取并缓存临时 Token。 | 单声道 PCM16、16 kHz WAV，最长 60 秒，超过时明确报错并保留 WAV。 |

在线模式会将完整录音提交所选云服务，并可能产生账户费用。松开语音键后整段识别，
无需公开音频 URL 或对象存储。一次识别（含凭据读取和阿里 Token 获取）总预算 60 秒，
密钥环单次操作最多 30 秒，网络连接最多 10 秒；退出应用会取消待处理在线请求，避免延迟输入文字。

凭据缺失或密钥环锁定时，解锁密钥环后重试配置；密钥环服务缺失时先安装并启动 Secret Service。
鉴权失败检查凭据、产品权限和区域；限流或网络错误可稍后重新录音。
识别失败保留私有 WAV，可在状态页诊断信息中查看其路径。
不自动回退或重试；需要本地识别时，在设置页选择 Voxtype 并保存。
删除当前在线服务凭据会先切回 Voxtype；删除其他服务的凭据不会更改当前选择。

新在线设置可直接通过图形界面保存；手工编辑配置文件后重启应用使其生效。
真实云服务联调需要用户自己的账户、开通权限与凭据；自动化测试使用离线协议 fake 和本地 HTTP 服务，
不会调用付费 API。接口核验记录见 [Provider 调研](docs/research/online-transcription-providers.md)。

Unknown settings, invalid TOML types, unsafe duration values, and unusable WAV
directories cause startup to fail with an actionable error. Capture WAV files
are created privately and may contain dictated speech; protect any custom
`wav_dir` accordingly.

Invalid configuration is shown as an actionable failure in the status window.
After saving a valid replacement, the running desktop application detects it
and reinitializes the bridge automatically.

## Map remote buttons

Open **Button Mapping** from the application header or tray menu. The supported
Physical Buttons are Power, Confirm, Up, Down, Left, Right, Back, Volume Up,
Volume Down, Menu, and Live. Voice is deliberately excluded: it remains reserved
for ATVV Capture and cannot be remapped.

Each Physical Button can **Keep Original**, become **Disabled**, or emit any
Logical Key from the complete searchable Linux key catalog. Changes remain a
Draft Mapping until **Apply** is selected. A Button Mapping is system-wide, so
it affects every user after the ATVV Remote reconnects. Applying or restoring
defaults uses a one-shot administrator authorization; permission is not retained.
Canceling authorization preserves the Draft Mapping. A successful write changes
the Installed Mapping but does not affect the already-connected input device:
disconnect and reconnect the remote to activate it.

Power defaults to Disabled because its native `KEY_POWER` action may suspend or
shut down the computer. Enabling **Keep Original** for Power shows an explicit
warning. **Reset All** and **Restore Defaults** return Power to Disabled.

### Button Mapping recovery

- **Busy**: another mapping operation owns the system lock. Wait for it to
  finish, then retry; staged edits are preserved.
- **Authorization not granted**: canceling or denying the system prompt makes no
  change. Apply again when administrator authorization is available.
- **Unsupported system**: Button Mapping is unavailable when the required hwdb
  catalog or runtime tools are unsupported. Voice input remains available;
  install the supported package/runtime before retrying.
- **Revision conflict**: the Installed Mapping changed after this Draft Mapping
  was loaded. Choose **Reload**, confirm loss of staged edits, then recreate and
  apply the desired changes.
- **RecoveryRequired**: the managed source and compiled hwdb cannot be trusted as
  a consistent pair. Editing and Apply remain disabled; use **Restore Defaults**.
- **Rollback failure**: recovery could not restore the previous Installed
  Mapping. Treat the state as RecoveryRequired and use **Restore Defaults**. If
  that also fails, retain the diagnostics and repair the system hwdb tooling
  before retrying.

The release-time certified-device procedure is documented in the
[Button Mapping hardware acceptance checklist](docs/hardware/button-mapping-acceptance-checklist.md).

## Remove

```sh
sudo apt remove atvv-bridge
```

`apt remove` deletes the Installed Mapping. Package removal preserves user
configuration and retained WAV files; reconnect the remote afterward to restore
native non-voice button behavior. Removal and purge delete only the Button
Mapping override managed by ATVV Voice Bridge; unrelated hwdb files are
preserved.
