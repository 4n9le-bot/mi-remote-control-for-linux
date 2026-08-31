# ATVV Voice Bridge

This context describes how a Bluetooth voice remote supplies an utterance to Voxtype on Linux.

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
