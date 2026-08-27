# ATVV Voice Bridge

`atvv-bridge` connects to a paired ATVV Remote through BlueZ, hands each
completed Capture to Voxtype as a WAV file, and commits successful text through
Fcitx 5.

## Runtime integrations

The bridge expects these commands to be available in the service environment:

- [`voxtype`](https://github.com/peteonrails/voxtype) for the WAV Handoff.
- `fcitx5-commit` for the Text Commit.

They are runtime integrations, not Debian package dependencies, because their
installation and configuration are specific to the user's desktop.

## Build the Debian package

On a Debian system, install Rust 1.85 or newer plus `dpkg-dev`, `fakeroot`,
`jq`, and `systemd`, then run:

```sh
tests/debian-package.sh
scripts/build-deb.sh
```

The package is written to `target/debian/` by default. The package test checks
its contents, dependencies, systemd unit, install behavior, and removal
behavior.

## Install and enable

Install the generated package explicitly, replacing the filename with the
artifact produced for the current version and architecture:

```sh
sudo apt install ./target/debian/atvv-bridge_*.deb
atvv-bridge --check
systemctl --user enable --now atvv-bridge.service
```

Package installation deliberately does not enable or start the service. The
unit runs as the current user and becomes part of that user's `default.target`
only after the explicit `systemctl --user enable` command.

Inspect the service with:

```sh
systemctl --user status atvv-bridge.service
journalctl --user -u atvv-bridge.service
```

Removing the package does not remove configuration under
`$XDG_CONFIG_HOME/atvv-bridge` or retained WAV files under the configured
`wav_dir`.
