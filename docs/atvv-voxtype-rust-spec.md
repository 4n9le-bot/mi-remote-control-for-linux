# ATVV → Voxtype Rust 实现规格

## 1. 目标

实现一个 Linux 用户态 Rust 桥接程序，使支持 ATVV 私有 BLE 语音协议的遥控器能够作为 `voxtype` 的语音输入设备。

首版目标是：接收 HID 语音键事件、通过 BlueZ GATT 采集 ATVV 音频、解码 IMA/DVI ADPCM 为 16 kHz 单声道 PCM16，并输出 WAV 或虚拟 PipeWire 音频源。

## 2. 已验证协议事实

- BLE 私有服务：`AB5E0001`
- 语音特征范围：`AB5E0002` 至 `AB5E0004`
- `GET_CAPS` 响应：`0B 01 00 02 03 00 78 00 00`
- 采集序列：`0x04` → 音频通知 → `0x00`
- `0x08`、`MIC_OPEN`、`MIC_CLOSE` 对该设备不是必需的
- 每个音频帧 120 字节
- 编码：IMA/DVI ADPCM，高半字节优先
- 解码输出：16 kHz、单声道、PCM16
- HOGP/HID 连接与 ATVV 音频通知可并行存在
- 已观察到一次语音键按下/释放、161 个非空通知、19,320 字节音频和一个 `0x00`

## 3. 非目标

- 不实现通用 ATVV 设备兼容层
- 不以 root 运行
- 不持久化原始音频，除非用户明确启用 WAV 调试输出
- 不模拟键盘输入；文字注入由 `voxtype` 完成

## 4. 总体架构

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

## 5. 模块设计

### 5.1 Bluetooth/GATT

负责发现目标设备、连接、服务发现、读取能力、写入 `0x04`/`0x00` 控制消息和订阅音频通知。

建议 API：

```rust
trait AtvvTransport {
    async fn connect(&mut self) -> Result<()>;
    async fn get_caps(&mut self) -> Result<Vec<u8>>;
    async fn start_capture(&mut self) -> Result<()>;
    async fn stop_capture(&mut self) -> Result<()>;
    async fn next_audio_notification(&mut self) -> Result<Option<Vec<u8>>>;
}
```

优先使用 BlueZ D-Bus；`btleplug` 可作为实现选择，但必须验证其对目标设备通知和重连的支持。

### 5.2 HID/evdev

通过 `/dev/input/event*` 识别目标 HID 设备和语音键的按下/释放边沿。

- 使用 VID/PID、蓝牙地址或稳定的设备名匹配；不得写死 `eventX`。
- 使用 `EVIOCGRAB` 时必须在所有退出路径释放。
- 语音键按下开始采集，释放停止采集。
- 若无法独占设备，应提供不 grab 的兼容模式。

### 5.3 ADPCM 解码

实现 IMA/DVI ADPCM 状态机：

- 4-bit nibble；
- 高半字节优先；
- 每帧独立或连续状态必须通过测试确认；
- 输出有符号 PCM16；
- 对 predictor 和 index 做范围钳制。

必须提供纯函数测试，覆盖静音、最大正负值、帧边界、损坏数据和连续帧拼接。

### 5.4 音频输出

首版实现 WAV：

- 使用 `tempfile` 创建 `/tmp` 下用户拥有的临时文件；
- 权限 `0600`；
- 固定 60 秒上限；
- 不接受用户指定输出路径；
- 完成转写后立即删除。

后续实现 PipeWire：创建 16 kHz mono PCM16 虚拟 source，使 `voxtype` 将其作为普通音频输入读取。

## 6. 状态机

```text
Disconnected
  → Connecting
  → Discovering
  → ReadingCaps
  → Ready
  → Capturing
  → Stopping
  → Ready
```

任何错误均进入 `Cleanup`，完成以下操作后才返回：停止通知、发送 `0x00`（如连接仍有效）、释放 `EVIOCGRAB`、关闭音频输出、删除临时文件。

## 7. CLI

建议命令：

```text
atvv-bridge devices
atvv-bridge info --mac AA:BB:CC:DD:EE:FF
atvv-bridge capture-wav
atvv-bridge run --voxtype
atvv-bridge self-test
```

`capture-wav` 不允许 root，默认 60 秒；`run --voxtype` 使用 HID 按键控制实时采集。

## 8. 配置

配置文件：`~/.config/atvv-bridge/config.toml`

```toml
device_mac = "C0:5D:39:C2:CE:26"
voice_service = "AB5E0001"
sample_rate = 16000
max_duration_secs = 60
output = "pipewire"
evdev_grab = true
```

## 9. 与 voxtype 的集成

### MVP

```text
HID 按键 → capture-wav → voxtype transcribe file.wav → 删除 WAV
```

### 实时方案

```text
HID 按键 → ATVV → ADPCM → PipeWire virtual source → voxtype
```

不建议通过 `output.post_process` 把实时音频伪装成文本后处理；该接口适合文本流，不适合音频采集。

## 10. 安全与隐私

- 拒绝 root 运行；
- 日志只记录设备状态、帧数、字节数和错误，不记录音频内容；
- 临时 WAV 使用 `0600`，成功或失败均删除；
- 断线、Ctrl-C、超时、panic unwind 前均执行清理；
- 不将 BLE 音频上传到网络。

## 11. 测试验收标准

### 协议

- 能发现 `AB5E0001`；
- 能读取已知 `GET_CAPS` 响应；
- 能完成 `0x04` → frames → `0x00`；
- 能处理连接断开并自动清理。

### 音频

- 120 字节帧正确解码；
- 输出为 16 kHz mono PCM16；
- 人声可理解；
- 连续 161 帧统计为 19,320 字节。

### HID/并发

- 一个按下/释放对应一次采集；
- `EVIOCGRAB` 必须释放；
- 所有通知订阅必须停止；
- 不产生重复采集任务。

### 安全

- root 启动失败；
- WAV 权限为 `0600`；
- 退出后不残留音频文件。

## 12. 待确认事项

1. `AB5E0002`、`AB5E0003`、`AB5E0004` 的确切角色；
2. ADPCM predictor/index 是每帧重置还是跨帧连续；
3. `0x04` 和 `0x00` 分别写入哪个 characteristic；
4. 多设备选择和重连策略；
5. PipeWire 虚拟 source 的具体 Rust API；
6. `voxtype` 实时输入所需的采集格式和生命周期。

## 13. 推荐实现顺序

1. 纯 Rust ADPCM 解码器和测试；
2. GATT 服务发现与固定设备采集；
3. WAV 输出和 `capture-wav`；
4. HID 按键事件与清理状态机；
5. `voxtype transcribe` 调用；
6. PipeWire 实时 source；
7. 自动重连、多设备和打包。
