# Secure system-wide hwdb updates from a desktop application

## Question

What is the smallest safe and supportable privileged boundary for a GTK application that installs, validates, compiles, conflict-checks, resets, and removes a device-specific udev hwdb Button Mapping?

## Decision

Ship a root-owned, non-interactive helper and invoke it through one project-specific Polkit action. The unprivileged GUI stages mappings and sends only a versioned, structured mapping request plus the revision it originally read. The helper owns every privileged choice: fixed paths, the certified ATVV Remote match, the supported Physical Button scan codes, the Logical Key allowlist, serialization, locking, validation, compilation, rollback, reset, and removal.

Do not let the GUI submit a path, executable, shell command, match expression, scan code, or arbitrary hwdb text. Polkit explicitly models the privileged mechanism as treating its unprivileged subject as untrusted and checking authorization for every request; `pkexec` also explicitly does not validate arguments passed to the program.[^polkit-architecture][^pkexec]

Use a dedicated action with `auth_admin` for active local sessions and no retained authorization. Polkit describes `auth_admin` as administrative authentication, warns that `auth_self` is generally insufficient on multi-user systems, and describes the `*_keep` variants as retaining authorization for a period.[^polkit-actions] A cancelled or failed authorization makes no privileged call and leaves both the installed configuration and the GUI's staged edits unchanged.

## File ownership and precedence

Keep package defaults in `/usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb`: the existing voice-button suppression and, once its scan code is verified, the power Physical Button's default `Disabled` mapping. Store only user overrides in a separately managed `/etc/udev/hwdb.d/99-atvv-bridge-button-mapping.hwdb`. `Reset All` then removes the override file and recompiles, naturally exposing the package defaults.

This naming is significant. hwdb reads `/usr/lib/udev/hwdb.d` and `/etc/udev/hwdb.d`, sorts all files lexically regardless of directory, gives later records higher priority, and only gives `/etc` special precedence when filenames are identical.[^hwdb-files] A late, uniquely named file makes the override explicit without replacing the package file wholesale.

The managed source file must be root-owned, non-user-writable, and ordinarily `0644`; its parent is the system-owned `/etc/udev/hwdb.d`. Temporary files must be created in that directory by the helper with a restrictive umask, opened without following symlinks, written completely, synced, set to the final ownership/mode, and atomically renamed over the managed filename. No user-controlled directory or filename enters this operation.

`Disabled` must serialize to the hwdb key name `reserved`, which maps to Linux `KEY_RESERVED` (numeric value zero).[^linux-key-reserved] Every other target is serialized from the helper's compiled Logical Key allowlist. systemd's keyboard hwdb format is `KEYBOARD_KEY_<lowercase hexadecimal scan code>=<key identifier>`; identifiers correspond to Linux input `KEY_*` names and are written lowercase, usually without the `key_` prefix.[^keyboard-hwdb]

## Apply transaction

The helper should perform one bounded operation under a fixed, root-owned advisory lock:

1. Parse a size-limited, versioned request from a pipe or standard input. Reject unknown fields, duplicate Physical Buttons, unknown scan codes, non-allowlisted Logical Keys, and all free-form strings except the expected revision token.
2. Acquire an exclusive lock such as `/run/lock/atvv-bridge-button-mapping.lock`.
3. Open the fixed managed path safely, compute a canonical revision over its current bytes plus an explicit absent-file state, and compare it with the GUI's expected revision. Reject a mismatch as stale; never overwrite an external administrator's change.
4. Generate canonical hwdb bytes internally. Preserve the previous managed bytes in privileged memory or a root-only temporary file for rollback.
5. Validate in an isolated temporary root before touching live files: reproduce the effective `/usr/lib/udev/hwdb.d` and `/etc/udev/hwdb.d` source set there, replace only this helper's managed file with the candidate, run `/usr/bin/systemd-hwdb --root=<staging-root> --usr --strict update`, and query the certified device modalias from that root. Require the exact expected `KEYBOARD_KEY_*` set for every managed scan code. `--root` is the supported alternate-root option and `--strict` promises a non-zero status for parsing errors.[^systemd-hwdb]
6. Atomically replace the live managed source file, then run `/usr/bin/systemd-hwdb --usr --strict update` and verify the certified modalias with `/usr/bin/systemd-hwdb query`.
7. If compilation or verification fails, restore the old source atomically and compile again. Report failure only after attempting this rollback; retain actionable diagnostics without logging user identity or unrelated device data.
8. Sync durable state as appropriate, release the lock, and return the new revision. The GUI reports that the mapping was written and that reconnect is required.

Validation must precede the live source change. Runtime udev uses only the compiled binary database, not the `.hwdb` sources.[^hwdb-runtime] Moreover, systemd's current implementation parses all sources, stores a replacement binary through a linkable temporary file, and only then returns a remembered strict-mode parse error; therefore a live `--strict update` is not a harmless validator.[^systemd-source-update][^systemd-source-store] Canonical generation, isolated-root compilation, and exact query verification are all required.

The managed source and compiled database are two distinct files, so the overall operation cannot be a single filesystem rename. The design gives atomic source replacement and rollback for ordinary failures, but a process or machine crash between source replacement and binary compilation can leave new source with the prior binary. Every later helper operation and package-maintainer update must therefore recompile and verify, repairing that state. The revision check is optimistic conflict detection; the helper lock cannot serialize an external root process that ignores it.

## Debian compilation boundary

On Debian, compile to `/usr/lib/udev/hwdb.bin` with `systemd-hwdb --usr update`, even though the override source lives in `/etc`. Debian's `udev` package declares a dpkg trigger for `/usr/lib/udev/hwdb.d` and its maintainer script invokes `systemd-hwdb --usr update`.[^debian-trigger][^debian-postinst] Using the default `/etc/udev/hwdb.bin` would create a second compiled database outside that packaging convention and risk a stale local binary surviving later package-triggered `/usr` recompilation. The systemd manual confirms that `--usr` selects `/usr/lib/udev` rather than `/etc/udev` for generated output.[^systemd-hwdb]

The helper must invoke fixed absolute binaries directly, with a minimal environment and no shell. It should check required systemd capabilities/version during installation or startup and fail closed when `--root`, `--usr`, or `--strict` is unavailable.

## Reload and reconnect

No `udevadm control --reload` is needed for a hwdb-only change: the runtime data is the newly compiled binary database. Applying `KEYBOARD_KEY_*` to a live input device requires running the udev keyboard builtin for that device (systemd's documented example uses `udevadm trigger /dev/input/eventXX`) or recreating the device.[^keyboard-hwdb] The project decision is not to trigger or control Bluetooth, so Apply and Reset stop after writing and compiling, clearly instruct the user to reconnect the ATVV Remote, and do not claim an Active state.

The reason is observable in systemd's implementation: the keyboard builtin reads `KEYBOARD_KEY_*` device properties and performs `EVIOCSKEYCODE` ioctls on the input device.[^systemd-keyboard-source] Updating the database alone cannot retroactively redo those ioctls on an already-created device.

## Reset and package removal

`Reset All` is the same authorized transaction with an absent override as its candidate. It conflict-checks, safely removes only `/etc/udev/hwdb.d/99-atvv-bridge-button-mapping.hwdb`, recompiles, verifies that package defaults are exposed, and requests reconnect. It never deletes or edits the package-owned `90-atvv-bridge.hwdb`.

The generated `/etc` override must not be a Debian conffile, because the product decision requires `apt remove`—not only purge—to delete it. Debian normally preserves conffiles through remove and removes them during purge.[^debian-removal] Add an idempotent maintainer-script remove path that deletes only the fixed generated override, then runs the Debian-consistent `systemd-hwdb --usr --strict update`. By the time `postrm remove` runs, dpkg has removed ordinary package files (including the package default hwdb), while `postrm` itself remains available.[^debian-removal] Thus removal drops both package defaults and GUI overrides and restores native behavior after the user reconnects the ATVV Remote.

Because maintainer scripts may run without a controlling terminal, removal must never prompt.[^debian-maintainer-scripts] A cleanup/recompile failure should be surfaced as a package error rather than silently claiming restoration.

## Acceptance boundaries for the later specification

- One authorization covers one Apply or Reset request; there is no cached project authorization.
- The helper's accepted request language cannot express arbitrary files, programs, hwdb matches, scan codes, or key names.
- A stale expected revision causes no writes and asks the GUI to reload.
- Any validation, compilation, or verification failure preserves or restores the previously installed source and reports failure.
- Query verification covers the exact certified modalias and every known Physical Button property, including `reserved` for Disabled.
- Success means source and binary were durably updated; it does not mean the current input device is Active.
- Reconnect is the only supported activation path.
- Reset exposes package defaults; package remove removes defaults and overrides and restores native behavior after reconnect.
- Crash recovery is explicit: the next helper or maintainer-script operation recompiles and verifies the source of truth.

## Sources

[^polkit-architecture]: polkit project, [`polkit(8)`, “Description”](https://polkit.pages.freedesktop.org/polkit/polkit.8.html#description).
[^polkit-actions]: polkit project, [`polkit(8)`, “Declaring Actions”](https://polkit.pages.freedesktop.org/polkit/polkit.8.html#polkit-declaring-actions).
[^pkexec]: polkit project, [`pkexec(1)`, “Security Notes”](https://polkit.pages.freedesktop.org/polkit/pkexec.1.html#security-notes).
[^hwdb-files]: systemd project, [`hwdb(7)`, “Hardware Database Files”](https://www.freedesktop.org/software/systemd/man/257/hwdb.html#Hardware%20Database%20Files).
[^hwdb-runtime]: systemd project, [`hwdb(7)`, compiled database and runtime behavior](https://www.freedesktop.org/software/systemd/man/257/hwdb.html#Hardware%20Database%20Files).
[^keyboard-hwdb]: systemd project, [`60-keyboard.hwdb`](https://github.com/systemd/systemd/blob/v257.13/hwdb.d/60-keyboard.hwdb).
[^linux-key-reserved]: Linux kernel, [`input-event-codes.h`](https://github.com/torvalds/linux/blob/v6.16/include/uapi/linux/input-event-codes.h#L75).
[^systemd-hwdb]: systemd project, [`systemd-hwdb(8)`](https://www.freedesktop.org/software/systemd/man/257/systemd-hwdb.html).
[^systemd-source-update]: systemd project, [`hwdb_update()` in `hwdb-util.c` v257.13](https://github.com/systemd/systemd/blob/v257.13/src/shared/hwdb-util.c#L567-L647).
[^systemd-source-store]: systemd project, [`trie_store()` in `hwdb-util.c` v257.13](https://github.com/systemd/systemd/blob/v257.13/src/shared/hwdb-util.c#L359-L416).
[^systemd-keyboard-source]: systemd project, [`udev-builtin-keyboard.c` v257.13](https://github.com/systemd/systemd/blob/v257.13/src/udev/udev-builtin-keyboard.c#L55-L87).
[^debian-trigger]: Debian systemd maintainers, [`debian/udev.triggers`](https://salsa.debian.org/systemd-team/systemd/-/blob/debian/master/debian/udev.triggers).
[^debian-postinst]: Debian systemd maintainers, [`debian/udev.postinst`](https://salsa.debian.org/systemd-team/systemd/-/blob/debian/master/debian/udev.postinst).
[^debian-removal]: Debian Policy, [§6.8, “Details of removal and/or configuration purging”](https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html#details-of-removal-and-or-configuration-purging).
[^debian-maintainer-scripts]: Debian Policy, [§6.3, “Controlling terminal for maintainer scripts”](https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html#controlling-terminal-for-maintainer-scripts).
