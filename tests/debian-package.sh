#!/usr/bin/env bash

set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
test_dir=$(mktemp -d)
trap 'rm -rf -- "$test_dir"' EXIT

package_dir="$test_dir/packages"
install_root="$test_dir/root"
admin_dir="$install_root/var/lib/dpkg"
mkdir -p -- "$package_dir" "$admin_dir" "$install_root/var/log"
touch "$admin_dir/status"

"$repo_dir/scripts/build-deb.sh" "$package_dir"
package=$(find "$package_dir" -maxdepth 1 -name 'atvv-bridge_*.deb' -print -quit)
test -n "$package"

contents=$(dpkg-deb --contents "$package")
grep -Eq '[[:space:]]\./usr/bin/atvv-bridge$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/share/applications/io.github.atvv_bridge.desktop$' <<<"$contents"
grep -Eq '[[:space:]]\./etc/xdg/autostart/io.github.atvv_bridge.desktop$' <<<"$contents"
! grep -Eq '[[:space:]]\./usr/lib/systemd/user/atvv-bridge.service$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/share/doc/atvv-bridge/README.md$' <<<"$contents"

dependencies=$(dpkg-deb --field "$package" Depends)
grep -Eq 'libgtk-4-1' <<<"$dependencies"
grep -Eq 'libadwaita-1-0' <<<"$dependencies"
! grep -Eqi 'voxtype|fcitx' <<<"$dependencies"

control_files=$(dpkg-deb --ctrl-tarfile "$package" | tar -tf -)
! grep -Eq '^\./(preinst|postinst|prerm|postrm)$' <<<"$control_files"

user_state="$install_root/home/test-user/.local/state/atvv-bridge"
user_config="$install_root/home/test-user/.config/atvv-bridge/config.toml"
mkdir -p -- "$user_state" "$(dirname "$user_config")"
touch "$user_state/retained.wav"
printf 'keep_wav = true\n' >"$user_config"

fakeroot dpkg --root="$install_root" --admindir="$admin_dir" --log=/dev/null \
    --force-bad-path --unpack "$package"

systemd-hwdb --root="$install_root" --strict update
remote_modalias='evdev:input:b0005v2717p32B8e00A4-e0,1,4,14,k71,72,73,74,75'
remote_properties=$(systemd-hwdb --root="$install_root" query "$remote_modalias")
grep -Fxq 'KEYBOARD_KEY_7003e=reserved' <<<"$remote_properties"

menu_entry="$install_root/usr/share/applications/io.github.atvv_bridge.desktop"
autostart_entry="$install_root/etc/xdg/autostart/io.github.atvv_bridge.desktop"
grep -Fxq 'Type=Application' "$menu_entry"
grep -Fxq 'Exec=atvv-bridge' "$menu_entry"
grep -Fxq 'Terminal=false' "$menu_entry"
grep -Fxq 'Type=Application' "$autostart_entry"
grep -Fxq 'Exec=atvv-bridge' "$autostart_entry"
grep -Fxq 'Terminal=false' "$autostart_entry"

fakeroot dpkg --root="$install_root" --admindir="$admin_dir" --log=/dev/null \
    --force-bad-path --remove atvv-bridge
test -f "$user_state/retained.wav"
test -f "$user_config"
test ! -e "$install_root/usr/bin/atvv-bridge"
test ! -e "$install_root/usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb"
test ! -e "$menu_entry"
test ! -e "$autostart_entry"
