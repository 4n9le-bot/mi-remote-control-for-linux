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
  session's `PATH`.
- `fcitx5-commit` available in the user session's `PATH`.

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
