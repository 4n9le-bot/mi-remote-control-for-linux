# Certified Remote Button Matrix

This matrix records the non-voice input events emitted by the certified Xiaomi
Bluetooth Remote Control 2 Pro (`2717:32B8`). It is derived from a sanitized
interactive `evtest` capture performed on 2026-08-30. No Bluetooth address or
raw HID payload is retained.

## Verified Physical Buttons

| Physical Button | HID scan code | Native Linux key | Press | Release | Hold |
| --- | --- | --- | --- | --- | --- |
| Power | `70066` | `KEY_POWER` (`116`) | `1` | `0` | Repeats with value `2` |
| Confirm | `70028` | `KEY_ENTER` (`28`) | `1` | `0` | Repeats with value `2` |
| Direction Up | `70052` | `KEY_UP` (`103`) | `1` | `0` | Repeats with value `2` |
| Direction Down | `70051` | `KEY_DOWN` (`108`) | `1` | `0` | Repeats with value `2` |
| Direction Left | `70050` | `KEY_LEFT` (`105`) | `1` | `0` | Repeats with value `2` |
| Direction Right | `7004f` | `KEY_RIGHT` (`106`) | `1` | `0` | Repeats with value `2` |
| Back | `700f1` | `KEY_BACK` (`158`) | `1` | `0` | Repeats with value `2` |
| Volume Up | `70080` | `KEY_VOLUMEUP` (`115`) | `1` | `0` | Repeats with value `2` |
| Volume Down | `70081` | `KEY_VOLUMEDOWN` (`114`) | `1` | `0` | Repeats with value `2` |
| Menu | `70065` | `KEY_COMPOSE` (`127`) | `1` | `0` | Repeats with value `2` |
| Live | `70035` | `KEY_GRAVE` (`41`) | `1` | `0` | Repeats with value `2` |

Every verified Physical Button has its own scan code. Holds begin repeating
after approximately 255–272 ms and then repeat approximately every 40–44 ms. A
Button Mapping can therefore preserve the device's native press, release, and
repeat timing by changing only the scan-code-to-key-code association.

## Power Validation Safety

Pressing Power under the test host's native configuration immediately suspends
the PC. Before Power was captured, GNOME's power-button action was temporarily
set to `nothing` and a blocking systemd inhibitor covered both `sleep` and
`handle-power-key`. The active settings and inhibitor were independently
verified, and the same input node was proven with Confirm before Power was
pressed. The capture used ordinary, non-grabbing `evtest`, so it retained both
the press and release events without switching readers mid-press.

Power is independently remappable as scan code `70066`. The package can safely
map it to Disabled by default without affecting the other Physical Buttons.
