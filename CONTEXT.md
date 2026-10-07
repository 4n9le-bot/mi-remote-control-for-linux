# ATVV Voice Bridge

本上下文描述蓝牙遥控器在 Linux 上提供语音 Capture，经所选识别服务转为文字并输入当前应用。

## Language

**ATVV Remote**:
A paired Bluetooth HID device that exposes the ATVV GATT service and supplies encoded voice audio.
_Avoid_: Microphone, generic remote

**ATVV Profile**:
The negotiated ATVV protocol and codec variant that defines control messages, audio framing, sample rate, and decoder state rules.
_Avoid_: Device model, fixed ATVV format

**Capture**:
A bounded voice utterance beginning with an ATVV `AUDIO_START` control event and ending with `AUDIO_STOP` or a safety stop.
_Avoid_: Recording session, stream

**WAV Handoff**:
Delivery of a completed Capture to Voxtype as a temporary WAV file for transcription.
_Avoid_: WAV output, live input

**Transcription Provider**:
将完整 Capture 转为文字的识别服务，包括默认的 Voxtype 和用户选择的在线服务。
_Avoid_: Text Commit, 输入法

**Online Mode**:
用户主动选择在线 Transcription Provider 并提供其所需凭据后，将完整 Capture 交给该服务识别的模式。
_Avoid_: 自动回退, 流式识别

**Text Commit**:
Insertion of a successful Voxtype transcript into the application currently focused through Fcitx 5.
_Avoid_: Transcription, paste

**Bridge Status**:
The current read-only operational snapshot of the ATVV Voice Bridge, including ATVV Remote connection, Capture, battery, recovery, and actionable-failure state.
_Avoid_: Log stream, daemon state

**Physical Button**:
A non-voice control exposed by the certified ATVV Remote through its Bluetooth HID input device, including power, confirm, direction, back, volume, menu, and live controls.
_Avoid_: Voice button, logical action

**Logical Key**:
A standard Linux input key code emitted for a Physical Button and interpreted by the desktop or focused application.
_Avoid_: Command, application action

**Button Mapping**:
An explicit system-wide override from a Physical Button to either a Logical Key or Disabled. Unconfigured Physical Buttons retain their existing Linux input behavior.
_Avoid_: Shortcut, command binding

**Installed Mapping**:
The complete Button Mapping durably stored in the managed hwdb source. It may not govern input until the ATVV Remote reconnects.
_Avoid_: Current Mapping, Applied Mapping

**Draft Mapping**:
The complete editable Button Mapping held by the graphical interface but not yet installed.
_Avoid_: Pending Mapping, unsaved settings

**Mapping Revision**:
An opaque identity for an Installed Mapping used to detect concurrent changes.
_Avoid_: File timestamp, version number

**Disabled**:
A Button Mapping result that suppresses input from one Physical Button. It is distinct from leaving the Physical Button unconfigured.
_Avoid_: Unconfigured, no mapping
