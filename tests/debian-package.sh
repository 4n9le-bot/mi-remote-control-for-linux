#!/usr/bin/env bash

set -euo pipefail

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
grep -Eq '[[:space:]]\./usr/lib/systemd/user/atvv-bridge.service$' <<<"$contents"
grep -Eq '[[:space:]]\./usr/share/doc/atvv-bridge/README.md$' <<<"$contents"

dependencies=$(dpkg-deb --field "$package" Depends)
! grep -Eqi 'voxtype|fcitx' <<<"$dependencies"

control_files=$(dpkg-deb --ctrl-tarfile "$package" | tar -tf -)
! grep -Eq '^\./(preinst|postinst|prerm|postrm)$' <<<"$control_files"

user_state="$install_root/home/test-user/.local/state/atvv-bridge"
user_config="$install_root/home/test-user/.config/atvv-bridge/config.toml"
mkdir -p -- "$user_state" "$(dirname "$user_config")"
touch "$user_state/retained.wav"
printf 'keep_wav = true\n' >"$user_config"

payload_files=$(dpkg-deb --fsys-tarfile "$package" | tar -tf -)
! grep -Ev '^\./($|usr(/|$))' <<<"$payload_files"
dpkg-deb --extract "$package" "$install_root"
test ! -e "$install_root/etc/systemd/user/default.target.wants/atvv-bridge.service"
test -f "$user_state/retained.wav"
test -f "$user_config"

unit="$install_root/usr/lib/systemd/user/atvv-bridge.service"
grep -Fxq 'ExecStart=/usr/bin/atvv-bridge' "$unit"
grep -Fxq 'Restart=on-failure' "$unit"
grep -Fxq 'WantedBy=default.target' "$unit"
sed "s|^ExecStart=/usr/bin/atvv-bridge$|ExecStart=$install_root/usr/bin/atvv-bridge|" \
    "$unit" >"$test_dir/atvv-bridge.service"
systemd-analyze verify --user --generators=no --man=no \
    "$test_dir/atvv-bridge.service"
