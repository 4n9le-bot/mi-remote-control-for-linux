#!/usr/bin/env bash

set -euo pipefail

command -v bwrap >/dev/null

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
test_dir=$(mktemp -d)
trap 'rm -rf -- "$test_dir"' EXIT

package_dir="$test_dir/packages"
install_root="$test_dir/root"
mkdir -p -- "$package_dir" "$install_root"

"$repo_dir/scripts/build-deb.sh" "$package_dir"
package=$(find "$package_dir" -maxdepth 1 -name 'atvv-bridge_*.deb' -print -quit)
test -n "$package"

contents=$(dpkg-deb --contents "$package")
grep -Eq '[[:space:]]\./usr/bin/atvv-bridge$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/libexec/atvv-bridge/atvv-button-mapping-helper$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/share/polkit-1/actions/io.github.atvv_bridge.button-mapping.policy$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/share/applications/io.github.atvv_bridge.desktop$' <<<"$contents"
grep -Eq '[[:space:]]\./etc/xdg/autostart/io.github.atvv_bridge.desktop$' <<<"$contents"
! grep -Eq '[[:space:]]\./usr/lib/systemd/user/atvv-bridge.service$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/share/doc/atvv-bridge/README.md$' <<<"$contents"

dependencies=$(dpkg-deb --field "$package" Depends)
grep -Eq 'libgtk-4-1' <<<"$dependencies"
grep -Eq 'libadwaita-1-0' <<<"$dependencies"
grep -Eq '(^|, )pkexec([ (]|,|$)' <<<"$dependencies"
grep -Eq '(^|, )udev([ (]|,|$)' <<<"$dependencies"
! grep -Eqi 'voxtype|fcitx' <<<"$dependencies"

control_files=$(dpkg-deb --ctrl-tarfile "$package" | tar -tf -)
grep -Eq '^\./postinst$' <<<"$control_files"
grep -Eq '^\./postrm$' <<<"$control_files"
control_listing=$(dpkg-deb --ctrl-tarfile "$package" | tar -tvf -)
grep -Eq '^-rwxr-xr-x root/root .* \./postinst$' <<<"$control_listing"
grep -Eq '^-rwxr-xr-x root/root .* \./postrm$' <<<"$control_listing"

archive_listing=$(dpkg-deb --contents "$package")
grep -Eq '^-rwxr-xr-x root/root .* \./usr/bin/atvv-bridge$' <<<"$archive_listing"
grep -Eq '^-rwxr-xr-x root/root .* \./usr/libexec/atvv-bridge/atvv-button-mapping-helper$' <<<"$archive_listing"
grep -Eq '^-rw-r--r-- root/root .* \./usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb$' <<<"$archive_listing"
grep -Eq '^-rw-r--r-- root/root .* \./usr/share/polkit-1/actions/io.github.atvv_bridge.button-mapping.policy$' <<<"$archive_listing"
grep -Eq '^-rw-r--r-- root/root .* \./usr/share/applications/io.github.atvv_bridge.desktop$' <<<"$archive_listing"
grep -Eq '^-rw-r--r-- root/root .* \./etc/xdg/autostart/io.github.atvv_bridge.desktop$' <<<"$archive_listing"
grep -Eq '^-rw-r--r-- root/root .* \./usr/share/doc/atvv-bridge/README.md$' <<<"$archive_listing"

control_dir="$test_dir/control"
dpkg-deb --control "$package" "$control_dir"
test -x "$control_dir/postinst"
test -x "$control_dir/postrm"

user_state="$install_root/home/test-user/.local/state/atvv-bridge"
user_config="$install_root/home/test-user/.config/atvv-bridge/config.toml"
mkdir -p -- "$user_state" "$(dirname "$user_config")"
touch "$user_state/retained.wav"
printf 'keep_wav = true\n' >"$user_config"

dpkg-deb --extract "$package" "$install_root"

fake_bin="$test_dir/bin"
hwdb_log="$test_dir/systemd-hwdb.log"
mkdir -p "$fake_bin"
cat >"$fake_bin/systemd-hwdb" <<'EOF'
#!/bin/sh
set -eu
printf '%s\n' "$*" >>"$HWDB_LOG"
if test "${HWDB_FAIL:-0}" = 1; then
    exit 1
fi
exec /usr/bin/systemd-hwdb "$@"
EOF
chmod 0755 "$fake_bin/systemd-hwdb"
export DPKG_ROOT="$install_root" HWDB_LOG="$hwdb_log"
export PATH="$fake_bin:$PATH"

"$control_dir/postinst" configure
grep -Fxq -- "--root=$install_root --strict update" "$hwdb_log"
remote_modalias='evdev:input:b0005v2717p32B8e00A4-e0,1,4,14,k71,72,73,74,75'
remote_properties=$(systemd-hwdb --root="$install_root" query "$remote_modalias")
grep -Fxq 'KEYBOARD_KEY_7003e=reserved' <<<"$remote_properties"
grep -Fxq 'KEYBOARD_KEY_70066=reserved' <<<"$remote_properties"
for native_scan_code in 70028 70052 70051 70050 7004f 700f1 70080 70081 70065 70035; do
    ! grep -Fq "KEYBOARD_KEY_${native_scan_code}=" <<<"$remote_properties"
done

managed_override="$install_root/etc/udev/hwdb.d/99-atvv-bridge-button-mapping.hwdb"
unrelated_hwdb="$install_root/etc/udev/hwdb.d/98-unrelated.hwdb"
install -Dm644 /dev/null "$managed_override"
cat >"$managed_override" <<'EOF'
evdev:input:b0005v2717p32B8*
 KEYBOARD_KEY_70065=home
EOF
install -Dm644 /dev/null "$unrelated_hwdb"
cat >"$unrelated_hwdb" <<'EOF'
evdev:input:b9999v9999p9999*
 KEYBOARD_KEY_1=reserved
EOF

updates_before_upgrade=$(wc -l <"$hwdb_log")
"$control_dir/postrm" upgrade 0.2.0
test -f "$managed_override"
test "$(wc -l <"$hwdb_log")" -eq "$updates_before_upgrade"
"$control_dir/postinst" configure 0.1.0
test -f "$managed_override"
remote_properties=$(systemd-hwdb --root="$install_root" query "$remote_modalias")
grep -Fxq 'KEYBOARD_KEY_70065=home' <<<"$remote_properties"

if HWDB_FAIL=1 "$control_dir/postinst" configure; then
    echo 'postinst silently succeeded when hwdb compilation failed' >&2
    exit 1
fi

rm -f -- "$install_root/usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb"
"$control_dir/postrm" remove
test ! -e "$managed_override"
test -f "$unrelated_hwdb"
remote_properties=$(systemd-hwdb --root="$install_root" query "$remote_modalias")
! grep -Fq 'KEYBOARD_KEY_7003e=' <<<"$remote_properties"
! grep -Fq 'KEYBOARD_KEY_70066=' <<<"$remote_properties"
for native_scan_code in 70028 70052 70051 70050 7004f 700f1 70080 70081 70065 70035; do
    ! grep -Fq "KEYBOARD_KEY_${native_scan_code}=" <<<"$remote_properties"
done

install -Dm644 /dev/null "$managed_override"
updates_before_purge=$(wc -l <"$hwdb_log")
"$control_dir/postrm" purge
test ! -e "$managed_override"
test -f "$unrelated_hwdb"
test "$(wc -l <"$hwdb_log")" -eq "$((updates_before_purge + 1))"
remote_properties=$(systemd-hwdb --root="$install_root" query "$remote_modalias")
! grep -Fq 'KEYBOARD_KEY_70065=' <<<"$remote_properties"

install -Dm644 /dev/null "$managed_override"
if HWDB_FAIL=1 "$control_dir/postrm" purge; then
    echo 'postrm silently succeeded when hwdb compilation failed' >&2
    exit 1
fi
test ! -e "$managed_override"
test -f "$unrelated_hwdb"

menu_entry="$install_root/usr/share/applications/io.github.atvv_bridge.desktop"
autostart_entry="$install_root/etc/xdg/autostart/io.github.atvv_bridge.desktop"
grep -Fxq 'Type=Application' "$menu_entry"
grep -Fxq 'Exec=atvv-bridge' "$menu_entry"
grep -Fxq 'Terminal=false' "$menu_entry"
grep -Fxq 'Type=Application' "$autostart_entry"
grep -Fxq 'Exec=atvv-bridge' "$autostart_entry"
grep -Fxq 'Terminal=false' "$autostart_entry"

test -f "$user_state/retained.wav"
test -f "$user_config"

lifecycle_root="$test_dir/lifecycle-root"
lifecycle_admin="$lifecycle_root/var/lib/dpkg"
mkdir -p "$lifecycle_admin" "$lifecycle_root/var/log"
touch "$lifecycle_admin/status" "$lifecycle_root/var/log/dpkg.log"
lifecycle_state="$lifecycle_root/home/test-user/.local/state/atvv-bridge/retained.wav"
lifecycle_config="$lifecycle_root/home/test-user/.config/atvv-bridge/config.toml"
install -Dm600 /dev/null "$lifecycle_state"
install -Dm600 /dev/null "$lifecycle_config"
bwrap --unshare-user --uid 0 --gid 0 --bind / / \
    dpkg --root="$lifecycle_root" --admindir="$lifecycle_admin" \
    --log="$lifecycle_root/var/log/dpkg.log" --force-bad-path \
    --force-script-chrootless --unpack "$package"
bwrap --unshare-user --uid 0 --gid 0 --bind / / \
    dpkg --root="$lifecycle_root" --admindir="$lifecycle_admin" \
    --log="$lifecycle_root/var/log/dpkg.log" --force-bad-path \
    --force-script-chrootless --remove atvv-bridge
test -f "$lifecycle_state"
test -f "$lifecycle_config"
test ! -e "$lifecycle_root/usr/bin/atvv-bridge"
test ! -e "$lifecycle_root/usr/libexec/atvv-bridge/atvv-button-mapping-helper"
test ! -e "$lifecycle_root/usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb"
test ! -e "$lifecycle_root/usr/share/polkit-1/actions/io.github.atvv_bridge.button-mapping.policy"
test ! -e "$lifecycle_root/usr/share/applications/io.github.atvv_bridge.desktop"
test ! -e "$lifecycle_root/etc/xdg/autostart/io.github.atvv_bridge.desktop"
