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
