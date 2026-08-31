# ATVV → Voxtype Rust 实现参考

## 1. 目标

实现一个 Linux 用户态 Rust 桥接程序，使支持 ATVV 私有 BLE 语音协议的遥控器能够作为 `voxtype` 的语音输入设备。

目标是：接收 HID 语音键事件、通过 BlueZ GATT 采集 ATVV 音频、解码 IMA/DVI ADPCM 为 16 kHz 单声道 PCM16，并输出 WAV 或虚拟 PipeWire 音频源。

## 2. 已验证协议事实

- BLE 私有服务：`AB5E0001`
- 语音特征范围：`AB5E0002` 至 `AB5E0004`
- `GET_CAPS` 响应：`0B 01 00 02 03 00 78 00 00`
- 采集序列：`0x04` → 音频通知 → `0x00`
- 每个音频帧 120 字节
- 编码：IMA/DVI ADPCM，高半字节优先
- 解码输出：16 kHz、单声道、PCM16
- HOGP/HID 连接与 ATVV 音频通知可并行存在
- 已观察到一次语音键按下/释放、161 个非空通知、19,320 字节音频和一个 `0x00`


## 3. 总体架构

```text
BLE HID 语音键 ──→ HID 事件模块 ──→ Capture Controller
                                      │
BLE GATT AB5E ──→ ATVV 会话 ──→ ADPCM 解码器
                                      │
                         ┌────────────┴────────────┐
                         │                         │
                    WAV 输出                 PipeWire 输出
                         │                         │
                 voxtype transcribe       voxtype 音频输入
```

## 4. 模块设计

### 4.1 Bluetooth/GATT
负责定位 BlueZ 中已配对的目标设备，复用现有 BLE 连接，等待 GATT 服务解析完成，读取能力、写入 0x04/0x00 控制消息并订阅音频通知。若设备断开，可根据配置请求 BlueZ 重连，但不负责首次配对。

建议 API：

```rust
trait AtvvTransport {
    async fn attach(&mut self) -> Result<()>;
    async fn wait_ready(&mut self) -> Result<()>;
    async fn get_caps(&mut self) -> Result<Vec<u8>>;
    async fn start_capture(&mut self) -> Result<()>;
    async fn stop_capture(&mut self) -> Result<()>;
    async fn next_audio_notification(&mut self) -> Result<Option<Vec<u8>>>;
}
```


### 4.2 HID/evdev

通过 evdev 监听目标遥控器的 HID 语音键事件。使用 udev 属性、输入设备标识及能力识别设备，不依赖固定的 eventX。确认并配置实际语音键的 event code；按下开始采集，释放停止采集，忽略自动重复。

默认采用非独占监听，仅在明确需要时启用可选的 EVIOCGRAB。设备移除、事件丢失、BLE 断线、超时或程序退出时必须停止采集并清理状态；蓝牙重连后应重新定位输入设备。

### 4.3 ADPCM 解码

实现 IMA/DVI ADPCM 解码，使用高半字节优先顺序，输出 PCM16。解码器状态通过输入和返回值显式传递，predictor 钳制到 PCM16 范围，step index 钳制到 IMA 标准表范围。实现前必须通过抓包或参考实现。

确认每个通知的 payload 布局、初始 predictor/index、跨通知状态规则及丢包处理策略。确认后使用标准向量和真实采集数据建立测试。

### 4.4 音频输出

音频输出: WAV和Pipewire，接口接收 16 kHz mono PCM16 sample 流。
WAV方式文件保存到用户指定的新路径； voxtype transcribe recording.wav。

PipeWire 虚拟 source，需先验证 voxtype 的设备选择、格式和生命周期要求。

## 5. Runtime and deployment

- The GTK4/Libadwaita desktop application owns the long-running bridge.
- A normal application-menu entry starts it on demand.
- A system-wide XDG autostart entry starts it with each graphical session.
- A StatusNotifier tray keeps it discoverable after closing the window where
  supported; without a tray, closing requires confirmation and quits.
- Configuration errors remain visible and recover after a valid replacement is
  saved.
- The application runs as the desktop user, never as root or a system service.


## 6. 配置

配置文件：`~/.config/atvv-bridge/config.toml`

```toml
device = "C0:5D:39:C2:CE:26"
max_duration_secs = 60
output = "pipewire"         # wav or pipewire
wav_dir = "/tmp"
```

## 7. 与 voxtype 的集成

### WAV

```text
HID 按键 → capture-wav → voxtype transcribe file.wav 
```

### 实时方案

```text
HID 按键 → ATVV → ADPCM → PipeWire virtual source → voxtype
```
