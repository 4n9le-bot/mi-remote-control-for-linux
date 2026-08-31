# Button Mapping Certified-Device Acceptance Checklist

Use this checklist for release-time validation with the certified Xiaomi
Bluetooth Remote Control 2 Pro. Ordinary CI runs the isolated helper and package
tests and does not require physical hardware. This checklist is deliberately
manual because it verifies behavior after Bluetooth input-device recreation.

## Result record

Complete every field and retain this section with the release evidence.

| Field | Observed result |
| --- | --- |
| Date and tester | |
| Package version (`dpkg-query -W -f='${Version}' atvv-bridge`) | |
| Package architecture (`dpkg-query -W -f='${Architecture}' atvv-bridge`) | |
| Remote identity | `2717:32B8` (must be independently observed) |
| Linux/systemd version | |
| Desktop session | |
| Default Power suppression | Pass / Fail — notes: |
| Voice unaffected | Pass / Fail — notes: |
| Original mapping | Pass / Fail — notes: |
| Disabled mapping | Pass / Fail — notes: |
| Custom Logical Key | Pass / Fail — notes: |
| Press/release/repeat timing | Pass / Fail — notes: |
| Offline Apply | Pass / Fail — notes: |
| Reconnect-only activation | Pass / Fail — notes: |
| Revision conflict recovery | Pass / Fail — notes: |
| Restore Defaults | Pass / Fail — notes: |
| Package removal | Pass / Fail — notes: |

Do not mark the checklist complete without the tested package version, the
observed `2717:32B8` identity, and a Pass/Fail plus notes for every result row.

## Preconditions

1. Install the release candidate Debian package and confirm the version recorded
   above is the package under test.
2. Pair the remote normally. Confirm vendor/product identity `2717:32B8` from
   the recreated input device; do not record its Bluetooth address. Inspect the
   selected node and independently match `ID_VENDOR_ID=2717` and
   `ID_MODEL_ID=32b8`:

   ```sh
   udevadm info --query=property --name=/dev/input/eventX
   ```
3. Keep a terminal open with `evtest` available. Identify the remote input node
   again after every reconnect because its node number can change.
4. Do not automate disconnect or reconnect. Perform both through the normal
   Bluetooth controls or by turning the remote off and on.

## Mandatory native-Power safety

Native Power emits `KEY_POWER` and can suspend or shut down the test host. Before
any step that enables or presses native Power, all of the following are required:

1. Temporarily configure the desktop power-button action to do nothing.
2. Start a blocking inhibitor that covers both sleep and power-key handling, for
   example in a dedicated terminal:

   ```sh
   systemd-inhibit --what=sleep:handle-power-key \
     --why='ATVV Button Mapping Power acceptance test' \
     --mode=block sleep infinity
   ```

3. Verify the inhibitor with `systemd-inhibit --list` and independently verify
   the desktop power-button setting.
4. Prove the selected `evtest` node with Confirm before pressing Power.
5. Keep the inhibitor active until Power is returned to Disabled and the remote
   has reconnected. Restore the desktop setting afterward.

If any safeguard cannot be independently verified, skip native Power testing and
record the checklist as incomplete. Never rely on an unverified settings change.

## Acceptance procedure

### 1. Package defaults and Voice exclusion

1. Remove any prior Installed Mapping with **Reset All**, then reinstall or
   configure the release package and reconnect the remote.
2. Confirm Power emits no key event.
3. Hold Voice, dictate a short phrase, and release it. Confirm normal ATVV Capture
   and Text Commit. Confirm Voice does not appear in the Button Mapping page.
4. Confirm all eleven supported rows are present: Power, Confirm, Up, Down, Left,
   Right, Back, Volume Up, Volume Down, Menu, and Live.

### 2. Original, Disabled, and custom mappings

1. Set Confirm to **Keep Original**, Apply, and verify it remains unchanged until
   reconnect. After reconnect, verify `KEY_ENTER` press, release, and hold repeat.
2. Set Menu to **Disabled**, Apply, reconnect, and verify press and hold produce no
   key events while Voice still works.
3. Select a custom Logical Key for Live from the complete searchable catalog
   (use `KEY_HOME` for a repeatable result), Apply, reconnect, and verify its
   press, release, and hold repeat events.
4. Compare representative press/release/repeat timing with the
   [certified remote button matrix](certified-remote-button-matrix.md). Mapping
   must not synthesize extra events or change the remote's repeat cadence.

### 3. Offline Apply and reconnect-only activation

1. Disconnect the remote without closing the application.
2. Stage and Apply a different Menu mapping. Confirm the Installed Mapping write
   succeeds while the remote is offline.
3. Reconnect and verify the new mapping. Stage another mapping while connected,
   Apply it, and prove the old behavior remains until the next reconnect.

### 4. Conflict and recovery

1. Start with a non-default Installed Mapping, open Button Mapping, and stage an
   edit without applying it.
2. Reset the Installed Mapping outside that loaded Draft Mapping with the same
   packaged helper protocol (review the JSON response before continuing):

   ```sh
   printf '%s' '{"protocol_version":1,"catalog_version":1,"operation":"reset"}' \
     | pkexec /usr/libexec/atvv-bridge/atvv-button-mapping-helper reset
   ```

3. Apply the now-stale Draft Mapping. Confirm a revision conflict, preserved
   staged edits, and disabled Apply.
4. Choose **Reload**, confirm that the stale Draft Mapping will be lost, recreate
   the intended change, and Apply successfully.
5. After recording the current managed source for diagnostics, deliberately make
   only the helper-managed source inconsistent, then quit and restart the
   application before reopening Button Mapping:

   ```sh
   sudo install -m 0644 /dev/null \
     /etc/udev/hwdb.d/99-atvv-bridge-button-mapping.hwdb
   ```

   Confirm **RecoveryRequired** disables editing and offers **Restore Defaults**.
   Restore, reconnect, and verify package defaults. Do not modify unrelated hwdb
   files.
6. Confirm Power is Disabled before leaving this section. Native-Power checks, if
   performed, must use the mandatory inhibition procedure above.

### 5. Package removal

1. Install a visible custom mapping and reconnect to prove it is active.
2. Run `sudo apt remove atvv-bridge`. Confirm unrelated user configuration,
   retained WAV files, and unrelated hwdb sources remain.
3. Confirm the current input device still exhibits its already-loaded behavior.
4. Reconnect the remote and verify native non-voice behavior is restored. Voice
   bridge behavior is unavailable after package removal and is not interpreted as
   a button-mapping failure.

Record every observed result in the table, including deviations and diagnostic
paths. A release fails certified-device acceptance if any required row is Fail or
left blank.
