# ATVV Voice Bridge

`atvv-bridge` connects a paired ATVV Remote to Linux through BlueZ, decodes
each completed voice Capture, hands it to Voxtype as a WAV file, and commits
the resulting text through Fcitx 5.

The currently supported remote is the Xiaomi Bluetooth Remote Control 2 Pro
(`2717:32b8`) using the certified ATVV v1.0 hold-to-talk profile.

## Requirements

- A Linux desktop using BlueZ and Fcitx 5.
- A paired Xiaomi Bluetooth Remote Control 2 Pro.
- [`voxtype`](https://github.com/peteonrails/voxtype) available in the user
  service's `PATH`.
- `fcitx5-commit` available in the user service's `PATH`.

Voxtype and `fcitx5-commit` are runtime integrations rather than Debian package
dependencies because their installation and configuration are desktop-specific.

## Build the Debian package

Install Rust 1.85 or newer and the Debian build tools:

```sh
sudo apt install cargo dpkg-dev fakeroot jq systemd
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
reported by `dpkg`. The package test verifies its payload, dependencies,
systemd unit, hardware database entry, installation, and removal behavior.

To build only the release binary:

```sh
cargo build --release --locked
```

## Install

Install the generated package explicitly:

```sh
sudo apt install ./target/debian/atvv-bridge_*.deb
```

Installation deliberately does not enable or start the user service. The
package also installs a hardware database entry for the supported remote. It
maps only the voice button's repeating F5 scan code to `KEY_RESERVED`, while
leaving the other remote buttons unchanged. Disconnect and reconnect the remote
after installing or removing the package so the mapping is applied to the new
input device.

Check that BlueZ can see a connected remote with the complete ATVV service:

```sh
atvv-bridge --check
```

Then enable and start the bridge for the current user:

```sh
systemctl --user enable --now atvv-bridge.service
```

Inspect its status and logs with:

```sh
systemctl --user status atvv-bridge.service
journalctl --user -u atvv-bridge.service
```

## Configure

Without `--config`, the bridge reads the first applicable standard path:

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
```

| Setting | Default | Description |
| --- | --- | --- |
| `max_duration_secs` | `60` | Capture safety limit in seconds; must be between `1` and `3600`. |
| `wav_dir` | `"/tmp/atvv-bridge"` | Writable directory for temporary and retained Capture WAV files; created at startup when absent. |
| `keep_wav` | `false` | Retain WAV files after successful Text Commit. Failures retain their WAV regardless so it can be diagnosed or recovered. |

Unknown settings, invalid TOML types, unsafe duration values, and unusable WAV
directories cause startup to fail with an actionable error. Capture WAV files
are created privately and may contain dictated speech; protect any custom
`wav_dir` accordingly.

To use a nonstandard file, start the binary with an explicit path:

```sh
atvv-bridge --config /absolute/path/to/config.toml
```

Unlike a missing standard configuration file, a missing explicit file is an
error. After changing configuration for the installed service, restart it:

```sh
systemctl --user restart atvv-bridge.service
```

## Remove

Disable the user service before removing the package:

```sh
systemctl --user disable --now atvv-bridge.service
sudo apt remove atvv-bridge
```

Package removal preserves user configuration and retained WAV files. Reconnect
the remote after removal to restore its original voice-button key mapping.
